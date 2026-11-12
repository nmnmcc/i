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
import math
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
                spec_copy = re.fullmatch(r'tests_spec_[0-9.]+_(.+)\.i', p.name)
                if spec_copy and b'// spec:' in data:
                    # Keep the existing spec runner's package/module convention.
                    module = 'test/' + spec_copy[1] + '.i'
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
                project['files']['i.toml'] = 'name = "spec"\n' if module.startswith('test/') else 'name = "app"\n'
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
    # The bound is CPU time of the process, so a busy machine does not turn a slow case into a
    # timeout. Wall time is only a backstop for a process that waits without using the CPU.
    argv = [str(binary), *command, '--offline', '--project', str(project)]
    started = time.monotonic()
    env = dict(os.environ, NO_COLOR='1', TERM='dumb', RAYON_NUM_THREADS='1')
    cpu = max(1, math.ceil(bound))
    # `prlimit` sets the limit in the child; `preexec_fn` is not safe with the runner's threads.
    argv = ['prlimit', f'--cpu={cpu}:{cpu + 1}', '--', *argv]
    with subprocess.Popen(argv, cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True) as process:
        try:
            stdout, stderr = process.communicate(timeout=max(30, 6 * bound))
            code = process.returncode
            if code == -signal.SIGXCPU:
                code = 'timeout'
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
    if check['exit'] == 1 and case.get('run_rejected'):
        for opt in [0, 2]:
            result['run' + str(opt)], _ = invoke(binary, project, ['run', '--opt', str(opt)], bound)
    if check['exit'] == 0:
        source = project / 'src/main.i'
        has_main = source.is_file() and bool(re.search(r'(?m)^\s*main(?:\([^\n]*\))?(?:\s*:[^=\n]+)?\s*=', source.read_text()))
        entries = tomllib.loads((project / 'i.toml').read_text()).get('entries', {})
        named_entry = next(iter(entries), None) if not has_main else None
        # This guarantees startup is exercised, including for checker-only libraries.
        # The original check above still reports only the original top-level types.
        if project != runtime:
            project = materialize(case, runtime)
        name = 'qa_corpus_runtime_test'
        while any(f in case['files'] for f in [f'src/{name}.i', f'test/{name}.i']):
            name += '_again'
        entry = project / 'test' / (name + '.i')
        entry.parent.mkdir(parents=True, exist_ok=True)
        entry.write_text('(testlib = (test) = std/test)\ntest("qa_startup", () { Nil })\n')
        manifest_path = project / 'i.toml'
        manifest = manifest_path.read_text()
        data = tomllib.loads(manifest)
        if data.get('name') != 'std' and 'std' not in data.get('dependencies', {}) and 'std' not in data.get('dev-dependencies', {}):
            dependency = 'std = { path = ' + json.dumps(str(ROOT / 'std')) + ' }\n'
            section = re.search(r'(?m)^\[dev-dependencies\]\s*\n', manifest)
            if section:
                manifest = manifest[:section.end()] + dependency + manifest[section.end():]
            else:
                manifest += '\n[dev-dependencies]\n' + dependency
        if 'collection' not in data.get('test', {}):
            section = re.search(r'(?m)^\[test\]\s*\n', manifest)
            if section:
                manifest = manifest[:section.end()] + 'collection = "std/test.tests"\n' + manifest[section.end():]
            else:
                manifest += '\n[test]\ncollection = "std/test.tests"\n'
        manifest_path.write_text(manifest)
        for opt in [0, 2]:
            test, _ = invoke(binary, project, ['test', '--opt', str(opt)], bound)
            result['test' + str(opt)] = test
            if has_main or named_entry:
                command = ['run', '--opt', str(opt)]
                if named_entry:
                    command += [named_entry]
                run, _ = invoke(binary, project, command, bound)
                result['run' + str(opt)] = run
    return result, elapsed


def select(args):
    cases = json.loads((CORPUS / 'manifest.json').read_text())['cases']
    if getattr(args, 'case_list', None):
        names = json.loads(args.case_list.read_text())
        if not isinstance(names, list) or not names or not all(isinstance(n, str) for n in names):
            raise SystemExit('case-list must be a nonempty JSON list of case IDs')
        missing = set(names) - {c['id'] for c in cases}
        if missing:
            raise SystemExit('unknown cases: ' + ', '.join(sorted(missing)))
        cases = [c for c in cases if c['id'] in names]
    cases = [c for c in cases if args.filter in c['id']]
    if getattr(args, 'shard', None):
        # Shard K of N takes every Nth case by ID, so the shards of one selection
        # are disjoint and together are the whole selection.
        index, count = args.shard
        cases = [c for n, c in enumerate(sorted(cases, key=lambda c: c['id'])) if n % count == index - 1]
    if not cases:
        raise SystemExit('no cases selected')
    return cases


def shard(text):
    match = re.fullmatch(r'([0-9]+)/([0-9]+)', text)
    if not match or not 1 <= int(match[1]) <= int(match[2]):
        raise argparse.ArgumentTypeError('expected K/N with 1 <= K <= N')
    return int(match[1]), int(match[2])


def run_corpus(args):
    update = os.environ.get('UPDATE') == '1'
    failures = []
    summary = collections.Counter()
    cases = select(args)
    if args.list:
        for case in cases:
            print(case['id'])
        return
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
    def completed_phases_match(before, after):
        checker_censored = before['check']['exit'] == 'timeout' or after['check']['exit'] == 'timeout'
        for phase in before.keys() | after.keys():
            left, right = before.get(phase), after.get(phase)
            if left is None or right is None:
                if checker_censored:
                    continue
                return False
            if left['exit'] != 'timeout' and right['exit'] != 'timeout' and left != right:
                return False
        return True
    def observe_case(case):
        if any(r.get('phase') == 'entry' and re.search(r['pattern'], case['id'])
               for r in requirements['requirements']):
            case = dict(case, run_rejected=True)
        bound = args.timeout
        saved = ROOT / case['project'] / 'expected.json'
        if (not update or args.keep_bounds) and saved.exists():
            bound = json.loads(saved.read_text()).get('bound_seconds', 3)
        return case, observe(args.binary, case, bound)[0], bound
    pool = ThreadPoolExecutor(max_workers=args.jobs)
    for n, (case, result, bound) in enumerate(pool.map(observe_case, cases)):
        applicable = [r for r in requirements['requirements'] if re.search(r['pattern'], case['id'])]
        expected_path = ROOT / case['project'] / 'expected.json'
        if update:
            dump(expected_path, {'compiler': revision, 'binary_sha256': binary_digest, 'kind': 'observation; semantic requirements are separate', 'bound_seconds': bound, 'outcome': result})
        elif not expected_path.exists():
            failures.append(case['id'] + ': missing expected.json')
        else:
            old = json.loads(expected_path.read_text())['outcome']
            censored = any(r.get('censored') for r in applicable) and any(v['exit'] == 'timeout' for v in [*old.values(), *result.values()])
            same = completed_phases_match(old, result) if censored else comparable(old) == comparable(result)
            if not same:
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
                    entry_checks = [check, *[result[k] for k in ['run0', 'run2'] if k in result]] if check['exit'] != 0 else [result['run0'], result['run2']] if 'run0' in result else [result['test0'], result['test2']]
                    met = all(v['exit'] == 1 and ('error[' in v['stderr'] or 'provider:' in v['stderr'])
                              and 'E0505' not in v['stderr'] and 'trap:' not in v['stderr']
                              for v in entry_checks)
            elif wanted == 'accept':
                met = check['exit'] == 0 and all(value['exit'] == 0 for value in result.values())
            elif wanted == 'diverge':
                met = check['exit'] == 0 and all(
                    result[phase]['exit'] == 0 for phase in ('test0', 'test2')
                ) and all(
                    result[phase]['exit'] == 'timeout' and not result[phase]['stdout']
                    for phase in ('run0', 'run2')
                )
            else:
                met = False
            if requirement.get('diagnostic_codes'):
                diagnostic_text = '\n'.join(v['stderr'] for v in result.values()) if requirement.get('phase') == 'entry' else check['stderr']
                actual_codes = set(re.findall(r'error\[([^]]+)\]', diagnostic_text))
                met = met and bool(actual_codes.intersection(requirement['diagnostic_codes'])) and 'E0505' not in actual_codes
            for fragment in requirement.get('types_contain', []):
                met = met and fragment in check['stdout']
            if 'run_stdout' in requirement:
                met = met and all(phase in result and result[phase]['stdout'] == requirement['run_stdout']
                                  for phase in ['run0', 'run2'])
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
    print(json.dumps({**({'shard': '%d/%d' % args.shard} if args.shard else {}), 'selected': len(cases), 'checks': dict(summary), 'failures': failures}, indent=2))
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
    pairs = 0
    mismatches = []
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
        for operation in ['test', 'run']:
            left, right = result.get(operation + '0'), result.get(operation + '2')
            if left and right and left['exit'] != 'timeout' and right['exit'] != 'timeout':
                pairs += 1
                if {k: v for k, v in left.items() if k != 'command'} != {k: v for k, v in right.items() if k != 'command'}:
                    mismatches.append({'case': case['id'], 'operation': operation})
        if check['exit'] == 0:
            counts[review]['main_entries' if 'run0' in result else 'library_or_test_entries'] += 1
            if 'qa_corpus_runtime_test.qa_startup' in result.get('test0', {}).get('stdout', ''):
                counts[review]['startup_entries_completed'] += 1
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
    dump(CORPUS / 'summary.json', {'counts': counts, 'optimization_pairs': pairs, 'optimization_mismatches': mismatches, 'incidents': incidents, 'exclusions': inventory['excluded']})
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


def snapshot_tools(args):
    if os.environ.get('UPDATE') != '1':
        raise SystemExit('snapshot-tools requires UPDATE=1')
    path = CORPUS / 'inventory.json'
    inventory = json.loads(path.read_text())
    present = {x['path'] for x in inventory['tools']}
    added = 0
    code = {'.py', '.mjs', '.js', '.sh', '.rs', '.c', '.cpp', '.wat', '.toml', '.lock'}
    for review in sorted(Path(args.source).glob('review-*')):
        for source in walk(review):
            rel = source.relative_to(review)
            if source.suffix not in code or 'binaryen' in rel.parts or not any(x in SNAPSHOTS for x in rel.parts):
                continue
            position = next(i for i, part in enumerate(rel.parts) if part in SNAPSHOTS)
            maintained = ROOT / Path(*rel.parts[position + 1:])
            data = source.read_bytes()
            if maintained.is_file() and maintained.read_bytes() == data:
                continue
            if b'\0' in data or len(data) > 4_000_000:
                continue
            target = CORPUS / 'tools' / review.name / (str(rel) + '.source')
            relative = str(target.relative_to(ROOT))
            if relative in present:
                continue
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
            inventory['tools'].append({'origin': str(source), 'path': relative, 'sha256': sha(data), 'kind': 'historical snapshot driver or changed source; not an active compiler file'})
            present.add(relative)
            added += 1
    dump(path, inventory)
    print(f'preserved {added} additional snapshot drivers/source variants; third-party Binaryen sources excluded')


def integrity(args):
    """Check original payload fingerprints without depending on /tmp surviving."""
    if os.environ.get('UPDATE') != '1':
        raise SystemExit('integrity requires UPDATE=1')
    inventory = json.loads((CORPUS / 'inventory.json').read_text())
    cases = {case['id']: case for case in select(args)}
    payloads, maintained = {}, {}
    counts = collections.Counter()
    failures = []
    for entry in inventory['programs']:
        if 'case' in entry:
            identifier = entry['case']
            if identifier not in payloads:
                case = cases[identifier]
                fingerprints = set()
                for logical in case['files']:
                    path = ROOT / case['project'] / case.get('stored', {}).get(logical, logical)
                    data = path.read_bytes()
                    fingerprints.add(sha(data))
                    if data.startswith(b'// old-syntax-check: '):
                        fingerprints.add(sha(data.split(b'\n', 1)[1]))
                payloads[identifier] = fingerprints
            matches = entry['sha256'] in payloads[identifier]
            counts['preserved'] += 1
        elif 'historical_snapshot' in entry:
            matches = sha((ROOT / entry['historical_snapshot']).read_bytes()) == entry['sha256']
            counts['mapped'] += 1
        else:
            path = entry['existing'][0]
            if path not in maintained:
                data = subprocess.check_output(['git', 'show', inventory['compiler_base'] + ':' + path], cwd=ROOT)
                maintained[path] = sha(data)
            matches = maintained[path] == entry['sha256']
            counts['mapped'] += 1
        if not matches:
            failures.append(entry['origin'])
    for entry in inventory['tools']:
        counts['tools'] += 1
        if sha((ROOT / entry['path']).read_bytes()) != entry['sha256']:
            failures.append(entry['origin'])
    dump(CORPUS / 'integrity.json', {'counts': dict(counts), 'failures': failures,
         'source': 'Original inventory fingerprints and recorded import revision; only the generated parser metadata prefix is excluded.'})
    print(f'Integrity: {dict(counts)}, {len(failures)} failures')
    if failures:
        raise SystemExit(1)


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
    run.add_argument('--case-list', type=Path)
    run.add_argument('--timeout', type=float, default=10)
    run.add_argument('--missing-only', action='store_true')
    run.add_argument('--keep-bounds', action='store_true', help='retain each saved observation budget during UPDATE=1')
    run.add_argument('--jobs', type=int, default=1)
    run.add_argument('--shard', type=shard, help='K/N: run only the cases whose index in ID order is K-1 modulo N')
    run.add_argument('--list', action='store_true', help='print the selected case IDs and run nothing')
    sub.add_parser('parse-register')
    migration = sub.add_parser('migrate-k4')
    migration.add_argument('--binary', type=Path, default=ROOT / 'target/release/i')
    sub.add_parser('audit-k4')
    reporting = sub.add_parser('report')
    reporting.add_argument('--verification-log', type=Path)
    snap = sub.add_parser('snapshot')
    snap.add_argument('--label', required=True)
    snap.add_argument('--filter', default='')
    audit = sub.add_parser('audit-w520')
    audit.add_argument('--filter', default='')
    audit = sub.add_parser('integrity')
    audit.add_argument('--filter', default='')
    salvage = sub.add_parser('snapshot-tools')
    salvage.add_argument('--source', default='/tmp')
    args = parser.parse_args()
    if args.mode == 'audit-k4':
        from k4_audit import audit
        audit()
    elif args.mode == 'migrate-k4':
        from k4_migrate import migrate
        migrate(args.binary)
    elif args.mode == 'import':
        import_reviews(args)
        snapshot_tools(args)
    elif args.mode == 'run':
        args.binary = args.binary.resolve()
        run_corpus(args)
    elif args.mode == 'parse-register':
        register_parses(args)
    elif args.mode == 'snapshot':
        snapshot(args)
    elif args.mode == 'audit-w520':
        audit_w520(args)
    elif args.mode == 'integrity':
        integrity(args)
    elif args.mode == 'snapshot-tools':
        snapshot_tools(args)
    else:
        report(args)


if __name__ == '__main__':
    main()
