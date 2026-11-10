"""Permanent review fixtures, using the public CLI's existing test commands."""
import argparse
import collections
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import re
import runpy
import shutil
import signal
import subprocess
import tempfile
import time
import tomllib

ROOT = Path(__file__).resolve().parents[2]
CORPUS = ROOT / 'tests/review'
SKIP = {'.git', 'target', 'base-target', 'venv', 'node_modules', 'syntax-debug'}
SNAPSHOTS = {'base-src', 'redesign', 'mutation', 'baseline-check', 'after-check', 'before-check'}
TEXT = {'.py', '.mjs', '.js', '.sh', '.rs', '.c', '.cpp', '.wat', '.md', '.txt', '.json', '.jsonl', '.log', '.out', '.types', '.stderr', '.diff', '.toml', '.lock'}
SOURCES = {
    'review-a1': 'docs/design/syntax.md §§2.2, 4.8, 6.1, 6.12',
    'review-t1': 'docs/spec.md §§4.2, 4.8, 7.4: literal sets and coverage',
    'review-t2': 'docs/spec.md §§2.5, 4.3, 4.7: completed value types',
    'review-k4': 'docs/spec.md §§4.5 rule 4, 4.7, 9.2: equal service arguments and principal inference',
    'review-rec': 'docs/design/records.md; T3 is held in the ledger',
}


def dump(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n')


def sha(data):
    return hashlib.sha256(data).hexdigest()


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT, text=True).strip()


def source_for(review):
    return SOURCES.get(review, SOURCES['review-k4'] if review.startswith('review-k4') else 'docs/spec.md §§10–11; review criteria preserved under tools/')


def walk(root):
    for directory, dirs, files in os.walk(root):
        dirs[:] = sorted(d for d in dirs if d not in SKIP)
        for name in sorted(files):
            p = Path(directory) / name
            if not p.is_symlink():
                yield p


def import_reviews(args):
    """Preserve bytes first. Never execute historical scripts or copy build products."""
    if os.environ.get('UPDATE') != '1':
        raise SystemExit('import requires UPDATE=1')
    tracked = collections.defaultdict(list)
    for name in git('ls-files').splitlines():
        if name.startswith('tests/review/'):
            continue
        p = ROOT / name
        if p.is_file() and p.suffix in {'.i', '.py', '.mjs', '.rs'}:
            tracked[sha(p.read_bytes())].append(name)
    roots = sorted(Path(args.source).glob('review-*'))
    if not roots:
        raise SystemExit('no review directories')
    index = {'schema': 1, 'source_root': str(Path(args.source)), 'compiler_base': git('rev-parse', 'HEAD'), 'programs': [], 'tools': [], 'excluded': []}
    projects = {}
    for review in roots:
        for p in walk(review):
            rel = p.relative_to(review)
            if p.suffix != '.i':
                if p.suffix in TEXT and not any(x in SNAPSHOTS for x in rel.parts):
                    data = p.read_bytes()
                    if len(data) > 4_000_000 or b'\0' in data:
                        index['excluded'].append({'origin': str(p), 'reason': 'large log or binary; not a program'})
                        continue
                    # Historical host drivers are data, not active repository snippets.
                    target = CORPUS / 'tools' / review.name / (str(rel) + ('.source' if p.suffix in {'.rs', '.mjs', '.js', '.sh', '.md'} else ''))
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_bytes(data)
                    index['tools'].append({'origin': str(p), 'path': str(target.relative_to(ROOT)), 'sha256': sha(data)})
                continue
            data = p.read_bytes()
            entry = {'origin': str(p), 'sha256': sha(data), 'source': source_for(review.name)}
            matches = tracked.get(sha(data), [])
            if matches:
                entry['existing'] = matches
                index['programs'].append(entry)
                continue
            if any(x in SNAPSHOTS for x in rel.parts):
                # A copied checkout is not a new review program. Preserve its exact
                # old bytes, with a link to the maintained test at the same path.
                entry['snapshot'] = True
                position = next(i for i, part in enumerate(rel.parts) if part in SNAPSHOTS)
                repository_path = Path(*rel.parts[position + 1:])
                if (ROOT / repository_path).is_file():
                    target = CORPUS / 'tools' / review.name / (str(rel) + '.source')
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_bytes(data)
                    entry['existing'] = [str(repository_path)]
                    entry['historical_snapshot'] = str(target.relative_to(ROOT))
                    index['programs'].append(entry)
                    continue
            parent = p.parent
            while parent != review and not (parent / 'i.toml').exists():
                parent = parent.parent
            if not (parent / 'i.toml').exists():
                parent = None
            context = None
            if parent and p.relative_to(parent).parts[0] not in {'src', 'test'}:
                context = parent
                parent = None
            if parent:
                key = str(parent)
                module = str(p.relative_to(parent))
            else:
                key = str(p)
                module = 'src/main.i'
            project = projects.setdefault(key, {'origin': key, 'review': review.name, 'files': {}, 'origins': [], 'source': source_for(review.name)})
            if context:
                project['context'] = str(context)
                for directory in ['src', 'test']:
                    for helper in sorted((context / directory).rglob('*.i')):
                        project['files'][str(helper.relative_to(context))] = helper.read_bytes().decode()
            project['files'][module] = data.decode()
            project['origins'].append(entry)
            if parent:
                project['files']['i.toml'] = (parent / 'i.toml').read_text()
            elif context:
                project['files']['i.toml'] = (context / 'i.toml').read_text()
            else:
                project['files']['i.toml'] = 'name = "app"\n'
            index['programs'].append(entry)
    # Keep a complete mirrored package tree so relative cross-package paths survive.
    locations = {}
    for origin, project in projects.items():
        review = project['review']
        base = Path(args.source) / review
        rel = Path(origin).relative_to(base)
        if Path(origin).suffix == '.i':
            rel = Path('_loose') / rel.with_suffix('')
        locations[origin] = CORPUS / 'cases' / review / rel
    cases = []
    for origin, project in sorted(projects.items()):
        if Path(origin).is_dir():
            for directory in ['src', 'test']:
                for module_path in sorted((Path(origin) / directory).rglob('*.i')):
                    project['files'][str(module_path.relative_to(origin))] = module_path.read_bytes().decode()
        path = locations[origin]
        if path == CORPUS / 'cases' / project['review']:
            path = path / '_root'
            locations[origin] = path
        manifest = project['files']['i.toml']
        def dependency(match):
            old = match[2]
            dep = (Path(project.get('context', origin)) / old).resolve() if not Path(old).is_absolute() else Path(old)
            if str(dep) in locations:
                new = os.path.relpath(locations[str(dep)], path)
            elif dep.name == 'std' or str(dep).endswith('/std'):
                new = os.path.relpath(ROOT / 'std', path)
            elif dep.is_dir() and (dep / 'i.toml').is_file():
                # Installed review dependencies outside /tmp need an explicit escalation.
                new = old
                index['excluded'].append({'origin': origin, 'dependency': old, 'reason': 'external dependency; source not owned by this review'})
            else:
                new = old
            return match[1] + json.dumps(new)
        manifest = re.sub(r'(\bpath\s*=\s*)[\"\']([^\"\']+)[\"\']', dependency, manifest)
        project['files']['i.toml'] = manifest
        for name, data in project['files'].items():
            target = path / name
            target.parent.mkdir(parents=True, exist_ok=True)
            if not target.exists() or target.read_bytes() != data.encode():
                (path / 'expected.json').unlink(missing_ok=True)
            target.write_text(data)
        case_id = str(path.relative_to(CORPUS / 'cases'))
        cases.append({'id': case_id, 'project': str(path.relative_to(ROOT)), 'origin': origin, 'context': project.get('context'), 'source': project['source'], 'finding': Path(origin).stem, 'files': [x for x in project['files'] if x.endswith('.i')]})
        for entry in project['origins']:
            entry['case'] = case_id
    # Record models are review tools; read their committed versions, not another worktree.
    for name in git('ls-tree', '-r', '--name-only', 'lane/note-rec', '--', 'tests').splitlines():
        if name.endswith('.py') and '/t3-' in name:
            data = subprocess.check_output(['git', 'show', 'lane/note-rec:' + name], cwd=ROOT)
            dest = CORPUS / 'tools/record-models' / Path(name).name
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(data)
            index['tools'].append({'origin': 'lane/note-rec:' + name, 'path': str(dest.relative_to(ROOT)), 'sha256': sha(data)})
    dump(CORPUS / 'inventory.json', index)
    dump(CORPUS / 'manifest.json', {'schema': 1, 'cases': cases})
    retained = {c['project'] for c in cases}
    for old_manifest in sorted((CORPUS / 'cases').rglob('i.toml'), key=lambda p: len(p.parts), reverse=True):
        stale = old_manifest.parent
        relative = str(stale.relative_to(ROOT))
        if stale.exists() and relative not in retained and not any(p.startswith(relative + '/') for p in retained):
            shutil.rmtree(stale)
    print(f'imported {len(cases)} projects, {len(index["programs"])} source occurrences, {len(index["tools"])} tools/evidence files')


def invoke(binary, project, command, bound):
    argv = [str(binary), *command, '--offline', '--project', str(project)]
    started = time.monotonic()
    env = dict(os.environ, NO_COLOR='1', TERM='dumb')
    with subprocess.Popen(argv, cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True) as process:
        try:
            stdout, stderr = process.communicate(timeout=bound)
            code = process.returncode
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            stdout, stderr = process.communicate()
            code = 'timeout'
    def normalize(data):
        text = data.decode(errors='replace').replace(str(project), '<project>').replace(str(ROOT), '<repo>')
        text = re.sub(r'\x1b\[[0-9;]*m', '', text)
        return text
    return {'command': command, 'exit': code, 'stdout': normalize(stdout), 'stderr': normalize(stderr)}, time.monotonic() - started


def materialize(case, runtime):
    project = ROOT / case['project']
    manifest = (project / 'i.toml').read_text()
    def dependency(match):
        return match[1] + json.dumps(str((project / match[2]).resolve()))
    manifest = re.sub(r'(\bpath\s*=\s*)[\"\']([^\"\']+)[\"\']', dependency, manifest)
    (runtime / 'i.toml').write_text(manifest)
    for name in case['files']:
        target = runtime / name
        target.parent.mkdir(parents=True, exist_ok=True)
        source = project / case.get('stored', {}).get(name, name)
        target.write_bytes(source.read_bytes())
    return runtime


def observe(binary, case, bound):
    work = CORPUS / '.work'
    work.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='corpus-', dir=work) as temporary:
        return observe_in(binary, case, bound, Path(temporary))


def observe_in(binary, case, bound, runtime):
    project = ROOT / case['project']
    if case.get('stored'):
        project = materialize(case, runtime)
    check, elapsed = invoke(binary, project, ['check', '--types'], bound)
    result = {'check': check}
    if check['exit'] == 0:
        text = '\n'.join((project / f).read_text() for f in case['files'])
        has_main = bool(re.search(r'(?m)^\s*@main\b', text))
        # This guarantees startup is exercised, including for checker-only libraries.
        # The original check above still reports only the original top-level types.
        if project != runtime:
            project = materialize(case, runtime)
        name = 'qa_corpus_runtime_test'
        while any(f in case['files'] for f in [f'src/{name}.i', f'test/{name}.i']):
            name += '_again'
        entry = project / 'test' / (name + '.i')
        entry.parent.mkdir(parents=True, exist_ok=True)
        entry.write_text('@test\nqa_startup = () { Nil }\n')
        for opt in [0, 2]:
            test, _ = invoke(binary, project, ['test', '--opt', str(opt)], bound)
            result['test' + str(opt)] = test
            if has_main:
                run, _ = invoke(binary, project, ['run', '--opt', str(opt)], bound)
                result['run' + str(opt)] = run
    return result, elapsed


def select(args):
    cases = json.loads((CORPUS / 'manifest.json').read_text())['cases']
    cases = [c for c in cases if args.filter in c['id']]
    if not cases:
        raise SystemExit('no cases selected')
    return cases


def run_corpus(args):
    update = os.environ.get('UPDATE') == '1'
    failures = []
    summary = collections.Counter()
    cases = select(args)
    if args.missing_only:
        cases = [c for c in cases if not (ROOT / c['project'] / 'expected.json').exists()]
        if not cases:
            print('all selected observations already exist')
            return
    requirements = json.loads((CORPUS / 'requirements.json').read_text())
    binary_digest = sha(args.binary.read_bytes())
    revision = git('rev-parse', 'HEAD')
    def comparable(outcome):
        # A killed child has no complete diagnostic/output to compare. Preserve
        # its raw partial output, but compare only its censored outcome.
        return {k: ({'exit': 'timeout', 'command': v['command']} if v['exit'] == 'timeout' else v)
                for k, v in outcome.items()}
    def observe_case(case):
        bound = args.timeout
        saved = ROOT / case['project'] / 'expected.json'
        if not update and saved.exists():
            bound = json.loads(saved.read_text()).get('bound_seconds', 3)
        return case, observe(args.binary, case, bound)[0]
    pool = ThreadPoolExecutor(max_workers=args.jobs)
    for n, (case, result) in enumerate(pool.map(observe_case, cases)):
        applicable = [r for r in requirements['requirements'] if re.search(r['pattern'], case['id'])]
        expected_path = ROOT / case['project'] / 'expected.json'
        if update:
            dump(expected_path, {'compiler': revision, 'binary_sha256': binary_digest, 'kind': 'observation; semantic requirements are separate', 'bound_seconds': args.timeout, 'outcome': result})
        elif not expected_path.exists():
            failures.append(case['id'] + ': missing expected.json')
        else:
            old = json.loads(expected_path.read_text())['outcome']
            censored = any(r.get('censored') for r in applicable) and any(v['exit'] == 'timeout' for v in [*old.values(), *result.values()])
            if comparable(old) != comparable(result) and not censored:
                failures.append(case['id'] + ': full output changed')
                dump(ROOT / case['project'] / 'actual.json', result)
            elif censored:
                summary['censored-observation'] += 1
                dump(ROOT / case['project'] / 'actual.json', result)
            else:
                (ROOT / case['project'] / 'actual.json').unlink(missing_ok=True)
        summary[str(result['check']['exit'])] += 1
        if any('cast failure' in v['stderr'] or 'panicked at' in v['stderr'] or isinstance(v['exit'], int) and v['exit'] < 0 for v in result.values()) and not any(r.get('known_failure') for r in applicable):
            failures.append(case['id'] + ': unclassified cast failure or compiler panic')
        if result['check']['exit'] == 'timeout' and not any(r.get('known_failure') for r in applicable):
            failures.append(case['id'] + ': unclassified checker timeout')
        for requirement in requirements['requirements']:
            if not re.search(requirement['pattern'], case['id']):
                continue
            wanted = requirement['outcome']
            check = result['check']
            if wanted == 'reject':
                met = check['exit'] == 1 and 'error[' in check['stderr'] and 'panicked at' not in check['stderr']
                if requirement.get('phase') == 'entry':
                    met = any(v['exit'] == 1 and ('error[' in v['stderr'] or 'provider:' in v['stderr'])
                              and 'E0505' not in v['stderr'] and 'trap:' not in v['stderr']
                              for v in result.values())
            elif wanted == 'accept':
                met = check['exit'] == 0 and all(value['exit'] == 0 for value in result.values())
            else:
                met = False
            if requirement.get('diagnostic_codes'):
                diagnostic_text = '\n'.join(v['stderr'] for v in result.values()) if requirement.get('phase') == 'entry' else check['stderr']
                actual_codes = set(re.findall(r'error\[([^]]+)\]', diagnostic_text))
                met = met and bool(actual_codes.intersection(requirement['diagnostic_codes'])) and 'E0505' not in actual_codes
            for fragment in requirement.get('types_contain', []):
                met = met and fragment in check['stdout']
            if requirement.get('known_failure'):
                summary['completed-resource-case' if met and requirement.get('censored') else 'XPASS' if met else 'XFAIL'] += 1
                if met and not update and not requirement.get('censored'):
                    failures.append(case['id'] + ': XPASS; promote the requirement')
            elif not met:
                failures.append(case['id'] + ': required ' + wanted)
        if result['check']['exit'] == 'timeout' or (isinstance(result['check']['exit'], int) and result['check']['exit'] < 0) or 'panicked at' in result['check']['stderr']:
            summary['checker-crash-or-timeout'] += 1
        if (n + 1) % 50 == 0:
            print(f'{n+1}/{len(cases)} {dict(summary)}', flush=True)
    print(json.dumps({'selected': len(cases), 'checks': dict(summary), 'failures': failures}, indent=2))
    pool.shutdown()
    if failures:
        raise SystemExit(1)


def register_parses(args):
    if os.environ.get('UPDATE') != '1':
        raise SystemExit('parse-register requires UPDATE=1')
    scanner = runpy.run_path(str(ROOT / 'scripts/old-syntax-check'))
    manifest_path = CORPUS / 'manifest.json'
    manifest = json.loads(manifest_path.read_text())
    files = []
    owners = {}
    for case in manifest['cases']:
        for name in case['files']:
            path = ROOT / case['project'] / case.get('stored', {}).get(name, name)
            files.append(path)
            owners[path] = case, name
    files.sort()
    sources = [re.sub(r'(?m)^// old-syntax-check:.*\n?', '', p.read_bytes().decode()) for p in files]
    results = scanner['parse'](sources)
    config_path = ROOT / 'scripts/old-syntax-check.json'
    config = {k: v for k, v in json.loads(config_path.read_text()).items()
              if not k.startswith('tests/review/cases/')}
    invalid = 0
    for path, source, diagnostics in zip(files, sources, results):
        if not diagnostics:
            continue
        case, name = owners[path]
        project = ROOT / case['project']
        stored = Path('.invalid') / name
        target = project / stored
        target.parent.mkdir(parents=True, exist_ok=True)
        relative = str(target.relative_to(ROOT))
        key = scanner['identity'](relative, source)
        config[key] = {'reason': 'Historical review input; exact parser diagnostics retained. See tests/review/inventory.json for origin.', 'diagnostics': diagnostics}
        if '/review-rec/' in relative:
            config[key]['track'] = 'T3'
        else:
            marker = '// old-syntax-check: ' + ','.join(d.split(':', 1)[0] for d in diagnostics) + '\n'
            target.write_text(marker + source)
        if config[key].get('track'):
            target.write_text(source)
        if path != target:
            path.unlink()
        case.setdefault('stored', {})[name] = str(stored)
        invalid += 1
        # Prefix markers change diagnostic positions; recapture through the real CLI.
        (project / 'expected.json').unlink(missing_ok=True)
    config_path.write_text(json.dumps(config, indent=2) + '\n')
    dump(manifest_path, manifest)
    retained = {ROOT / c['project'] / c.get('stored', {}).get(name, name)
                for c in manifest['cases'] for name in c['files']}
    for stale in (CORPUS / 'cases').rglob('*.i'):
        if stale not in retained and '.work' not in stale.parts:
            stale.unlink()
    print(f'{len(files)} review modules parsed, {invalid} exact negative fixtures registered')


def report(args):
    inventory = json.loads((CORPUS / 'inventory.json').read_text())
    manifest = json.loads((CORPUS / 'manifest.json').read_text())
    counts = collections.defaultdict(collections.Counter)
    incidents = []
    for tool in inventory['tools']:
        if tool['origin'].startswith('/tmp/review-'):
            review = Path(tool['origin']).parts[2]
            counts[review]['tools_and_evidence'] += 1
    for entry in inventory['programs']:
        review = Path(entry['origin']).parts[2]
        counts[review]['source_occurrences'] += 1
        counts[review]['already_present' if 'existing' in entry else 'preserved'] += 1
    for case in manifest['cases']:
        p = ROOT / case['project'] / 'expected.json'
        review = case['id'].split('/')[0]
        counts[review]['projects'] += 1
        if not p.exists():
            counts[review]['not_captured'] += 1
            continue
        saved = json.loads(p.read_text())
        result = saved['outcome']
        check = result['check']
        if 'E0520' in check['stderr']:
            saved['transitions'] = ['W520: E0520 becomes a warning; review this observed rejection when W520 lands']
            dump(p, saved)
        elif 'W0503' in check['stderr']:
            saved['transitions'] = ['W520 applied: E0520 -> W0503; docs/spec.md §6.9 requires Bool, no widening or new narrowing fact']
            dump(p, saved)
        counts[review]['accepted' if check['exit'] == 0 else 'rejected' if check['exit'] == 1 else 'timeout_or_crash'] += 1
        if 'E0520' in check['stderr']:
            counts[review]['W520_transition'] += 1
        for phase, value in result.items():
            if value['exit'] == 'timeout' or isinstance(value['exit'], int) and value['exit'] < 0 or 'trap' in value['stderr'] or 'panicked at' in value['stderr']:
                incidents.append({'case': case['id'], 'phase': phase, 'exit': value['exit'], 'stderr': value['stderr'][:1000]})
    dump(CORPUS / 'summary.json', {'counts': counts, 'incidents': incidents, 'exclusions': inventory['excluded']})
    requirements = json.loads((CORPUS / 'requirements.json').read_text())
    lines = ['# Known review failures', '', 'Generated by `scripts/review-corpus report`. Required outcomes are independent of observed baselines.', '']
    for requirement in requirements['requirements']:
        if not requirement.get('known_failure'):
            continue
        lines += ['- ' + requirement['finding'], '  Required: ' + requirement['outcome'] + '.', '  Source: ' + requirement['source'], '  Cases: ' + ', '.join(c['id'] for c in manifest['cases'] if re.search(requirement['pattern'], c['id'])), '']
    (CORPUS / 'known-failures.md').write_text('\n'.join(lines) + '\n')
    if args.verification_log:
        log = args.verification_log.read_text()
        decoder = json.JSONDecoder()
        short_runs = []
        for match in re.finditer(r'\{\n  "compiler":', log):
            try:
                value, _ = decoder.raw_decode(log[match.start():])
            except json.JSONDecodeError:
                continue
            if value.get('mode') == 'short' and 'counts' in value and 'rules' in value:
                short_runs.append(value)
        if len(short_runs) != 1:
            raise SystemExit('verification log must contain exactly one short-generation report')
        dump(CORPUS / 'generation-short.json', short_runs[0])
    print(json.dumps({'counts': counts, 'incidents': len(incidents)}, indent=2))


def snapshot(args):
    if os.environ.get('UPDATE') != '1' or not re.fullmatch(r'[a-zA-Z0-9_-]+', args.label):
        raise SystemExit('snapshot requires UPDATE=1 and a plain label')
    cases = select(args)
    observations = {}
    for case in cases:
        path = ROOT / case['project'] / 'expected.json'
        if not path.exists():
            raise SystemExit('missing observation: ' + case['id'])
        observations[case['id']] = json.loads(path.read_text())
    dump(CORPUS / 'baselines' / (args.label + '.json'), {'label': args.label, 'observations': observations})
    print(f'preserved {len(observations)} observations as {args.label}')


def audit_w520(args):
    if os.environ.get('UPDATE') != '1':
        raise SystemExit('audit-w520 requires UPDATE=1')
    baseline = json.loads((CORPUS / 'baselines/pre-w520.json').read_text())['observations']
    changes, censored, startup, failures = [], [], [], []
    location = r'(?m)^(.+\.i):(\d+):(\d+): (?:error|warning)\[%s\]:'
    for case in select(args):
        path = ROOT / case['project'] / 'expected.json'
        saved = json.loads(path.read_text())
        old, new = baseline[case['id']]['outcome'], saved['outcome']
        before = re.findall(location % 'E0520', old['check']['stderr'])
        after = re.findall(location % 'W0503', new['check']['stderr'])
        if before:
            unchanged_types = old['check']['stdout'] == new['check']['stdout']
            excerpts = []
            for name, line, col in before:
                logical = name.removeprefix('<project>/')
                source = ROOT / case['project'] / case.get('stored', {}).get(logical, logical)
                text = source.read_text().splitlines()[int(line) - 1] if source.is_file() else 'dependency source: ' + name
                excerpts.append({'path': name, 'line': int(line), 'column': int(col), 'source': text})
            changes.append({'case': case['id'], 'old_exit': old['check']['exit'], 'new_exit': new['check']['exit'], 'old_locations': before, 'new_locations': after, 'shown_types_unchanged': unchanged_types, 'source_excerpts': excerpts})
            if before != after or not unchanged_types:
                failures.append(case['id'] + ': W520 changed locations or shown types')
        elif old != new:
            if any(value['exit'] == 'timeout' for value in [*old.values(), *new.values()]):
                censored.append(case['id'])
            elif old['check'] == new['check'] and any('qa_corpus_runtime_test.qa_startup' in value['stdout'] for key, value in new.items() if key.startswith('test')):
                startup.append(case['id'])
            else:
                failures.append(case['id'] + ': unrelated full outcome changed')
    dump(CORPUS / 'w520-audit.json', {'source': 'docs/spec.md §6.9: the compiler gives warning W0503, showing both types; the comparison is well defined and has type Bool; literal types are not widened; the warning adds no narrowing fact', 'changes': changes, 'censored_changes': censored, 'startup_harness_changes': startup, 'startup_source': 'docs/spec.md §2.5: Its value is worked out once, when the program starts, before any entry point runs. An added Nil test now exercises startup; this is a harness change, not a W520 runtime-equivalence assertion.', 'failures': failures})
    print(f'W520: {len(changes)} marked cases, {len(censored)} censored changes, {len(startup)} startup harness changes, {len(failures)} unexplained changes')
    if failures:
        raise SystemExit(1)


def main():
    parser = argparse.ArgumentParser(__doc__)
    sub = parser.add_subparsers(dest='mode', required=True)
    imp = sub.add_parser('import')
    imp.add_argument('--source', default='/tmp')
    run = sub.add_parser('run')
    run.add_argument('--binary', type=Path, default=ROOT / 'target/release/i')
    run.add_argument('--filter', default='')
    run.add_argument('--timeout', type=float, default=10)
    run.add_argument('--missing-only', action='store_true')
    run.add_argument('--jobs', type=int, default=1)
    sub.add_parser('parse-register')
    reporting = sub.add_parser('report')
    reporting.add_argument('--verification-log', type=Path)
    snap = sub.add_parser('snapshot')
    snap.add_argument('--label', required=True)
    snap.add_argument('--filter', default='')
    audit = sub.add_parser('audit-w520')
    audit.add_argument('--filter', default='')
    args = parser.parse_args()
    if args.mode == 'import':
        import_reviews(args)
    elif args.mode == 'run':
        args.binary = args.binary.resolve()
        run_corpus(args)
    elif args.mode == 'parse-register':
        register_parses(args)
    elif args.mode == 'snapshot':
        snapshot(args)
    elif args.mode == 'audit-w520':
        audit_w520(args)
    else:
        report(args)


if __name__ == '__main__':
    main()
