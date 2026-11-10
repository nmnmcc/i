"""Move the former K4 runtime corpus into the permanent review registry.

The pinned source is this repository's reviewed test revision, not a compiler
observation. Existing complete programs are reused after formatting. Runtime
variants with an added entry remain distinct programs. No tests run here.
"""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

from corpus import ROOT, CORPUS, dump, sha

REVISION = '9695b32a2a4f180511202c0dbe5812bf1b778586'


def source(path):
    return subprocess.check_output(['git', 'show', REVISION + ':' + path], cwd=ROOT, text=True)


class Fixture:
    def __init__(self, path='crates/ilang-cli/tests/k4'):
        self.path = path

    def __truediv__(self, name):
        return Fixture(self.path + '/' + name)

    def read_text(self):
        return source(self.path)


def programs():
    # This pinned, locally authored module only defines source-building and
    # runner functions. Its main guard is false; only cases() is called.
    namespace = {'__file__': str(ROOT / 'tests/k4/review.py'), '__name__': 'k4_migration_template'}
    exec(compile(source('tests/k4/review.py'), 'historical-k4-cases', 'exec'), namespace)
    namespace['FIXTURES'] = Fixture()
    for name, text, reject, _ in namespace['cases']():
        yield name, text, 'E0522' if name in ['b4', 'b7', 'r2', 'r3'] else 'E0515' if reject else None

    rust = source('crates/ilang-cli/tests/k4.rs')
    decode = lambda s: json.loads('"' + s.replace('\n', '\\n') + '"')
    exact = decode(re.search(r'let exact = "((?:\\.|[^"\\])*)";', rust).group(1))
    for inferred in [False, True]:
        text = exact.replace('local: () -> Nil ! N | S(N)', 'local') if inferred else exact
        yield f'entry-{inferred}', text, 'E0522'
        yield f'unused-{inferred}', text.replace('local() }', 'Nil }'), None
    array = rust.split('let sources = [', 1)[1].split('\n    ];', 1)[0]
    for i, match in enumerate(re.finditer(r'"((?:\\.|[^"\\])*)"', array)):
        for inferred in [False, True]:
            text = decode(match[1])
            if inferred:
                for old in ['local: () -> Nil ! N | S(N)', '-> Nil ! N | S(N)',
                            '-> A ? E ! N | Outer(A, E, N)', '-> Int ! N | Outer(N)',
                            '-> Nil ? E ! N | Sink(E, N)']:
                    text = text.replace(old, 'local' if old.startswith('local:') else '')
                text = text.replace('() -> Nil ? E ! N | S(E, N) {', '() {')
            yield f'previous-{i}-{inferred}', text + '\n@main\nmain = () { example() }\n', 'E0522'
    chain = ('S(N) { S(() -> Nil ! N, (N) -> Nil) }\n'
             'use(N) = (s ! S(N)) { S(f, _) = s f() }\n'
             'run(N) = (body: () -> Nil ! N) { body() }\n'
             'forward_0 = () { run(() { use() }) }\n')
    for n in range(1, 64):
        chain += f'forward_{n} = () {{ run(() {{ forward_{n-1}() }}) }}\n'
    yield 'previous-chain-64', chain + '\n@main\nmain = () { forward_63() }\n', 'E0522'
    for inferred in [False, True]:
        for hole in [False, True]:
            text = (Fixture() / 'closed_handler_hole.i').read_text()
            if not hole:
                text = text.replace('! Svc(_) = svc', '! Svc(Int) = svc')
            if inferred:
                text = text.replace(') -> Int ! N {', ') {')
            yield f'handler-hole-{inferred}-{hole}', text, None


def migrate(binary):
    if os.environ.get('UPDATE') != '1':
        raise SystemExit('migrate-k4 requires UPDATE=1')
    manifest = json.loads((CORPUS / 'manifest.json').read_text())
    previous = [c for c in manifest['cases'] if c['origin'].startswith(f'git:{REVISION}:')]
    manifest['cases'] = [c for c in manifest['cases'] if c not in previous]
    lookup = {}
    records = []
    work = CORPUS / '.work'
    work.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='k4-migrate-', dir=work) as temporary:
        temp = Path(temporary)
        entries = list(programs())
        candidates = []
        # Prefer the reviewer's complete projects over loose-file copies.
        for case in sorted(manifest['cases'], key=lambda c: ('/proj/' not in c['id'], c['id'])):
            if case['files'] == ['src/main.i'] and not case.get('stored'):
                data = (ROOT / case['project'] / 'src/main.i').read_bytes()
                lookup.setdefault(sha(data), case)
                if len(data) <= 4 * max(len(t) for _, t, _ in entries):
                    path = temp / f'existing-{len(candidates)}.i'
                    path.write_bytes(data)
                    candidates.append((path, case))
        for name, text, _ in entries:
            (temp / (name + '.i')).write_text(text)
        subprocess.run([str(binary), 'fmt', str(temp)], cwd=ROOT, check=True, capture_output=True)
        for path, case in candidates:
            lookup.setdefault(sha(path.read_bytes()), case)
        for name, _, diagnostic in entries:
            data = (temp / (name + '.i')).read_bytes()
            digest = sha(data)
            case = lookup.get(digest)
            if case is None:
                identifier = 'review-k4-lane/' + name
                project = CORPUS / 'cases' / identifier
                project.joinpath('src').mkdir(parents=True, exist_ok=True)
                project.joinpath('src/main.i').write_bytes(data)
                project.joinpath('i.toml').write_text('name = "app"\n')
                case = {'id': identifier, 'project': str(project.relative_to(ROOT)),
                        'origin': f'git:{REVISION}:crates/ilang-cli/tests/k4.rs#{name}',
                        'source': 'docs/spec.md §§4.5, 4.7, 4.8, 9.2–9.3; K4 F1–F3 and first-review entry obligations',
                        'finding': 'K4 runtime variant ' + name, 'files': ['src/main.i']}
                if not any(c['id'] == identifier for c in manifest['cases']):
                    manifest['cases'].append(case)
                lookup[digest] = case
            record = {'name': name, 'case': case['id'], 'outcome': 'reject' if diagnostic else 'accept'}
            if diagnostic:
                record['diagnostic_codes'] = [diagnostic]
            if re.fullmatch(r'q\d+', name):
                record['types_contain'] = [f'app/main.{binding} : () -> Int ! N | S(N)' for binding in ['use', 'fw']]
            records.append(record)
    retained = {r['case'] for r in records}
    for case in previous:
        if case['id'] not in retained:
            shutil.rmtree(ROOT / case['project'])
    dump(CORPUS / 'manifest.json', manifest)
    dump(CORPUS / 'k4-migration.json', {'source_revision': REVISION, 'cases': records})
    print(f'{len(records)} K4 runtime assertions mapped to {len({r["case"] for r in records})} permanent projects')
