"""Seeded semantic models and metamorphic tests. No compiler code is an oracle."""
import argparse
import collections
import itertools
import json
import os
from pathlib import Path
import random
import re
import tempfile

from corpus import ROOT, CORPUS, dump, git, invoke, sha

VALUES = ['a', 'ab', 'b', 'c', '']
# T1 review fuzz.py's finite domain and eight string predicates, represented as data.
PATTERNS = ['"a"', '"ab"', '"b"', '_: String - "a"', '_: String - "ab" - "b"', '"a" <> _', '_', '_: "a" | "b"']
BOOLS = ['True', 'False', '_']
RULES = {
    'coverage': 'docs/spec.md §7.4: A clause with a guard is not counted.',
    'runtime': 'docs/spec.md §6.12: it takes the first clause whose pattern matches the value and whose guard, if there is one, is True',
    'literal': 'docs/spec.md §4.2: Values are never changed, so a literal type is never made wider',
    'value': 'docs/spec.md §2.5: It only orders type inference',
    'annotation': 'docs/spec.md §4.7: For every binding, the compiler must find the principal type: the most general type, of which every other possible type is an instance.',
}


def matches(pattern, value):
    if pattern == 0:
        return value == 'a'
    if pattern == 1:
        return value == 'ab'
    if pattern == 2:
        return value == 'b'
    if pattern == 3:
        return value != 'a'
    if pattern == 4:
        return value not in {'ab', 'b'}
    if pattern == 5:
        return value.startswith('a')
    if pattern == 6:
        return True
    if pattern == 7:
        return value in {'a', 'b'}
    raise ValueError(pattern)


def evaluate(rows):
    # Guarded rows do not establish coverage. False guards never run their bodies.
    return {(v, b): next((i for i, (p, q, guard) in enumerate(rows)
                         if matches(p, v) and (q == 2 or (q == 0) == b) and not guard), None)
            for v in VALUES for b in [True, False]}


def literal_case(seed, *, value_types, rng=None):
    rng = rng if rng is not None else random.Random(seed)
    rows = [(rng.randrange(8), rng.randrange(3), rng.random() < .12) for _ in range(rng.randint(2, 9))]
    expected = evaluate(rows)
    covered = all(v is not None for v in expected.values())
    domain = '("a" | "ab" | "b" | "c" | "", Bool)'
    declarations = []
    if value_types:
        declarations += [f'domain: {domain} = ("a", True)', 'Input = domain']
        domain = 'Input'
    declarations.append(f'pass = (x: {domain}) -> {domain} {{ x }}')
    clauses = '\n'.join(f'({PATTERNS[p]}, {BOOLS[q]})' + (' ? False' if guard else '') + f' -> {i}' for i, (p, q, guard) in enumerate(rows))
    declarations.append(f'f = (x: {domain}) {{ x |> {{\n{clauses}\n}} }}')
    if covered:
        assertions = '\n'.join(f'  assert(f(pass(({json.dumps(v)}, {str(b)}))) == {i})' for (v, b), i in expected.items())
        declarations.append('main = () {\n' + assertions + '\n  Nil\n}')
    return {'name': ('literal-value-guard-' if value_types else 't1-finite-') + str(seed),
            'declarations': declarations, 'accept': covered, 'rows': rows,
            'model': [{'string': v, 'bool': b, 'result': i} for (v, b), i in expected.items()],
            'annotation': (f'pass = (x: {domain}) -> {domain} {{ x }}', 'pass = (x) { x }')}


def alias_case(seed):
    rng = random.Random(seed)
    n = rng.randint(3, 20)
    declarations = ['value: Int = 1', 'A0 = value']
    declarations += [f'A{i} = A{i-1}' for i in range(1, n)]
    declarations += [f'pass = (x: A{n-1}) -> Int {{ x }}', 'main = () { assert(pass(2) == 2) Nil }']
    return {'name': f't2-alias-{seed}', 'declarations': declarations, 'accept': True,
            'annotation': (f'pass = (x: A{n-1}) -> Int {{ x }}', 'pass = (x) { x }')}


def service_case(seed):
    # The positive service family in K4 reviews, with closed equal arguments.
    n = random.Random(seed).randint(1, 8)
    declarations = ['Svc(A) { Svc(f: () -> A, m: (A) -> Nil) }',
                    'ask(A) = (s ! Svc(A)) { s.f() }',
                    'run(A, N) = (body: () -> A ! N) { body() }',
                    'pass = (x: Int) -> Int { x }',
                    'h0 = () { ask() }']
    declarations += [f'h{i} = () {{ run(() {{ h{i-1}() }}) }}' for i in range(1, n)]
    declarations += [f'main = () {{\n ! Svc(Int) = Svc(() {{ 7 }}, (_: Int) {{ Nil }})\n assert(pass(h{n-1}()) == 7)\n Nil\n}}']
    return {'name': f'k4-services-{seed}', 'declarations': declarations, 'accept': True,
            'annotation': ('pass = (x: Int) -> Int { x }', 'pass = (x) { x }')}


def validate(binary, project, case, bound):
    project.joinpath('src/main.i').write_text('\n'.join(case['declarations']) + '\n')
    output, _ = invoke(binary, project, ['check', '--types'], bound)
    if output['exit'] not in [0, 1] or 'panicked at' in output['stderr']:
        return 'termination', output
    if (output['exit'] == 0) != case['accept']:
        return 'model-acceptance', output
    if output['exit'] == 1 and 'rows' in case and 'E0516' not in output['stderr']:
        return 'model-diagnostic', output
    if output['exit'] == 0:
        outcomes = []
        for opt in [0, 2]:
            result, _ = invoke(binary, project, ['run', '--opt', str(opt)], bound)
            outcomes.append(result)
            if result['exit'] != 0 or 'trap' in result['stderr'] or 'panicked at' in result['stderr']:
                return 'runtime', result
        # Warnings include stable source ranges; both levels must preserve them too.
        for result in outcomes:
            result.pop('command')
        if outcomes[0] != outcomes[1]:
            return 'optimization', outcomes
    return None, output


def minimize(case, property_fails):
    """Deletion-minimal for the generating grammar, with model recomputation."""
    current = dict(case)
    if 'rows' not in case:
        declarations = list(case['declarations'])
        changed = True
        while changed:
            changed = False
            for i in range(len(declarations)):
                provided = re.search(r'(?m)^([a-zA-Z][a-zA-Z_0-9]*)\b[^\n]*(?:=|\{)', declarations[i])
                if provided and any(re.search(r'\b' + provided[1] + r'\b', d) for j, d in enumerate(declarations) if j != i):
                    continue
                candidate = dict(current, declarations=declarations[:i] + declarations[i + 1:])
                if property_fails(candidate):
                    declarations = candidate['declarations']
                    current = candidate
                    changed = True
                    break
        return current, '1-minimal under top-level declaration deletion, preserving failure diagnostics'
    rows = list(case['rows'])
    changed = True
    while changed and len(rows) > 2:
        changed = False
        for i in range(len(rows)):
            candidate_rows = rows[:i] + rows[i + 1:]
            # Retain the same semantic domain and recompute every expected result.
            candidate = rebuild(current, candidate_rows)
            if property_fails(candidate):
                rows = candidate_rows
                current = candidate
                changed = True
                break
    return current, '1-minimal under clause deletion (at least two clauses)'


def rebuild(case, rows):
    candidate = dict(case, rows=rows)
    expected = evaluate(rows)
    candidate['accept'] = all(v is not None for v in expected.values())
    candidate['model'] = [{'string': v, 'bool': b, 'result': i} for (v, b), i in expected.items()]
    declarations = [d for d in case['declarations'] if not d.startswith(('f = ', 'main = '))]
    domain = 'Input' if any(d == 'Input = domain' for d in declarations) else '("a" | "ab" | "b" | "c" | "", Bool)'
    clauses = '\n'.join(f'({PATTERNS[p]}, {BOOLS[q]})' + (' ? False' if guard else '') + f' -> {i}' for i, (p, q, guard) in enumerate(rows))
    declarations.append(f'f = (x: {domain}) {{ x |> {{\n{clauses}\n}} }}')
    if candidate['accept']:
        declarations.append('main = () {\n' + '\n'.join(f' assert(f(pass(({json.dumps(v)}, {str(b)}))) == {i})' for (v, b), i in expected.items()) + '\n Nil\n}')
    candidate['declarations'] = declarations
    return candidate


def save_failure(args, case, reason, observation, project):
    primitive_reason = reason.rsplit(':', 1)[-1]
    def fingerprint(output):
        if isinstance(output, list):
            return [fingerprint(x) for x in output]
        errors = re.findall(r'error\[([^]]+)\]', output.get('stderr', ''))
        if errors:
            return errors
        # Do not replace a runtime regression with a missing-main or unrelated stop.
        lines = output.get('stderr', '').splitlines()
        relevant = [line for line in lines if 'trap:' in line or 'panicked at' in line]
        return [re.sub(r':\d+:\d+:', ':<position>:', line) for line in (relevant or lines)[:1]]
    signature = fingerprint(observation)
    def reproduces(candidate):
        actual_reason, output = validate(args.binary, project, candidate, args.timeout)
        return actual_reason == primitive_reason and fingerprint(output) == signature
    minimal, label = minimize(case, reproduces)
    source = '\n'.join(minimal['declarations']) + '\n'
    path = CORPUS / 'generated-failures' / (reason + '-' + sha(source.encode())[:12])
    path.joinpath('src').mkdir(parents=True, exist_ok=True)
    path.joinpath('i.toml').write_text('name = "generated"\n')
    path.joinpath('src/main.i').write_text(source)
    dump(path / 'requirement.json', {'origin': case['name'], 'reason': reason, 'sources': RULES, 'model': minimal.get('model'), 'accept': minimal['accept'], 'minimality': label})
    dump(path / 'actual.json', observation)
    return str(path.relative_to(ROOT))


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/i')
    parser.add_argument('--mode', choices=['short', 'long'], default='short')
    parser.add_argument('--seed', type=int, default=412)
    parser.add_argument('--count', type=int)
    parser.add_argument('--timeout', type=float, default=10)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--models', action='store_true')
    parser.add_argument('--mpfr-python', type=Path)
    parser.add_argument('--model-timeout', type=float, default=120)
    parser.add_argument('--model-filter', default='')
    args = parser.parse_args()
    args.binary = args.binary.resolve()
    count = args.count if args.count is not None else (8 if args.mode == 'short' else 180)
    if count < 1:
        parser.error('count must be positive')
    counts = collections.Counter()
    failures = []
    reports = []
    work = CORPUS / '.work'
    work.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='i-generation-', dir=work) as temp:
        project = Path(temp)
        project.joinpath('src').mkdir()
        project.joinpath('i.toml').write_text('name = "generated"\n')
        # Keep T1's original sequential random stream as well as independent new seeds.
        t1_rng = random.Random(args.seed)
        for i in range(count):
            seed = args.seed + i
            cases = [literal_case(seed, value_types=False, rng=t1_rng), literal_case(seed, value_types=True), alias_case(seed), service_case(seed)]
            for case in cases:
                counts['attempted'] += 1
                reason, result = validate(args.binary, project, case, args.timeout)
                if reason:
                    failures.append(save_failure(args, case, reason, result, project))
                    continue
                counts['accepted' if case['accept'] else 'rejected-by-model'] += 1
                original = list(case['declarations'])
                for order in [list(reversed(original)), random.Random(seed).sample(original, len(original))]:
                    variant = dict(case, declarations=order)
                    reason, result = validate(args.binary, project, variant, args.timeout)
                    counts['permutations'] += 1
                    if reason:
                        failures.append(save_failure(args, variant, 'declaration-order:' + reason, result, project))
                if case['annotation']:
                    old, new = case['annotation']
                    variant = dict(case, declarations=[d.replace(old, new) for d in original])
                    reason, result = validate(args.binary, project, variant, args.timeout)
                    counts['annotation-removals'] += 1
                    if reason:
                        # Only use report-only for the spec's explicit expected-type differences.
                        failures.append(save_failure(args, variant, 'annotation:' + reason, result, project))
            if (i + 1) % 10 == 0:
                print(f'{i+1}/{count}: {dict(counts)}, failures {len(failures)}', flush=True)
        # A source annotation can intentionally widen a permanent read (§4.7).
        outcomes = []
        for declaration in ['value: Int = 1', 'value = 1']:
            project.joinpath('src/main.i').write_text(declaration + '\nInput = value\nx: Input = 2\n')
            observation, _ = invoke(args.binary, project, ['check', '--types'], args.timeout)
            outcomes.append(observation)
        reports.append({'property': 'remove source annotation before permanent read',
                        'source': 'docs/spec.md §4.7: Write a wider annotation at the source when needed.',
                        'exits': [x['exit'] for x in outcomes], 'classification': 'permitted difference; report only',
                        'outcomes': outcomes})
    models = []
    if args.models:
        from models import run_models
        models = run_models(args.mpfr_python.absolute() if args.mpfr_python else None,
                            args.model_timeout, args.model_filter)
    report = {'compiler': git('rev-parse', 'HEAD'), 'binary_sha256': sha(args.binary.read_bytes()), 'seed': args.seed, 'count': count, 'mode': args.mode, 'counts': dict(counts), 'failures': failures, 'report_only': reports, 'rules': RULES, 'models': models}
    if args.output:
        dump(args.output, report)
    print(json.dumps(report, indent=2))
    if failures or any(not m['matched'] for m in models):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
