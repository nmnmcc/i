#!/usr/bin/env python3
"""Incrementally preserve R2 review programs without reimporting other tracks.

UPDATE=1 direnv exec /home/agent/Programming/i python3 tests/r2-import-review.py
Then capture observations using UPDATE=1 scripts/review-corpus run --filter
review-r2. Required behavior is documented separately in r2-stages.md and the
runtime/spec tests; captured CLI output is never a semantic oracle.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil

ROOT = Path(__file__).resolve().parent.parent
CORPUS = ROOT / 'tests/review'
ORIGIN = Path('/tmp/review-r2')
SOURCE = 'docs/spec.md §§9.5, 9.9, 9.11; runtime.md §§5–6; independent review-r2/criteria.md'


def read(name):
    return json.loads((CORPUS / name).read_text())


def write(name, data):
    (CORPUS / name).write_text(json.dumps(data, indent=2, ensure_ascii=False) + '\n')


def main():
    if os.environ.get('UPDATE') != '1':
        raise SystemExit('import requires UPDATE=1')
    manifest = read('manifest.json')
    inventory = read('inventory.json')
    manifest['cases'] = [c for c in manifest['cases'] if not c['id'].startswith('review-r2/')]
    inventory['programs'] = [e for e in inventory['programs'] if not e['origin'].startswith(str(ORIGIN) + '/')]
    inventory['tools'] = [e for e in inventory['tools'] if not e['origin'].startswith(str(ORIGIN) + '/')]
    count = 0
    for package in sorted(ORIGIN.glob('*/i.toml')):
        original = package.parent
        identifier = 'review-r2/' + original.name
        project = CORPUS / 'cases' / identifier
        project.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(package, project / 'i.toml')
        files = []
        for directory in ['src', 'test']:
            for source in sorted((original / directory).rglob('*.i')):
                relative = source.relative_to(original)
                target = project / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                data = source.read_bytes()
                if target.exists() and target.read_bytes() != data:
                    (project / 'expected.json').unlink(missing_ok=True)
                target.write_bytes(data)
                files.append(str(relative))
                inventory['programs'].append(dict(origin=str(source), sha256=hashlib.sha256(data).hexdigest(),
                                                  source=SOURCE, case=identifier))
        manifest['cases'].append(dict(id=identifier, project=str(project.relative_to(ROOT)),
                                     origin=str(original), context=None, source=SOURCE,
                                     finding=original.name, files=files))
        count += 1
    # Preserve review criteria, the report, the measured old source and host
    # drivers as evidence, never as runnable repository code or current oracles.
    evidence = list(ORIGIN.glob('*.mjs')) + [ORIGIN / 'criteria.md', ORIGIN / 'report.md',
                ORIGIN / 'performance/task-source.i', ORIGIN / 'performance/summary.json',
                ORIGIN / 'performance/outer/allocation-attribution.json']
    for source in evidence:
        if not source.is_file():
            continue
        target = CORPUS / 'tools/review-r2' / (str(source.relative_to(ORIGIN)) + '.source')
        target.parent.mkdir(parents=True, exist_ok=True)
        data = source.read_bytes()
        target.write_bytes(data)
        inventory['tools'].append(dict(origin=str(source), path=str(target.relative_to(ROOT)),
                                       sha256=hashlib.sha256(data).hexdigest()))
    write('manifest.json', manifest)
    write('inventory.json', inventory)
    print(f'Preserved {count} complete R2 packages; other tracks unchanged.')


if __name__ == '__main__':
    main()
