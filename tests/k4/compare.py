#!/usr/bin/env python3
"""Compare deterministic generated projects, including both runtime levels."""
import argparse
import importlib.util
import json
from pathlib import Path
import re
import subprocess


def command(binary, args, project):
    try:
        p = subprocess.run([str(binary), *args, '--project', str(project)], capture_output=True, text=True, timeout=60)
        return {'status': p.returncode, 'stdout': p.stdout, 'stderr': p.stderr}
    except subprocess.TimeoutExpired:
        return {'status': 'timeout', 'stdout': '', 'stderr': ''}


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--before', required=True, type=Path)
    p.add_argument('--after', required=True, type=Path)
    p.add_argument('--out', required=True, type=Path)
    p.add_argument('--count', type=int, default=100)
    p.add_argument('--seed', type=int, default=44)
    a = p.parse_args()
    a.out.mkdir(parents=True, exist_ok=True)
    results = []
    for kind in ['general', 'mutate', 'plain']:
        path = Path(__file__).with_name('generators') / (kind + '.py')
        spec = importlib.util.spec_from_file_location(kind, path)
        gen = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(gen)
        for i, source in enumerate(gen.sources(a.seed, a.count)):
            name = f'{kind}-{a.seed}-{i}'
            project = a.out / name
            (project / 'src').mkdir(parents=True, exist_ok=True)
            (project / 'i.toml').write_text('name = "app"\n')
            (project / 'src/main.i').write_text(source)
            row = {'name': name}
            for version, binary in [('before', a.before), ('after', a.after)]:
                checked = command(binary, ['check', '--types'], project)
                row[version] = {'check': checked}
                if checked['status'] == 0:
                    row[version]['runs'] = [command(binary, ['run', '--opt', opt], project) for opt in ['0', '2']]
            results.append(row)
            (a.out / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
    summary = {}
    for row in results:
        key = 'before={} after={}'.format(row['before']['check']['status'], row['after']['check']['status'])
        summary[key] = summary.get(key, 0) + 1
    failures = [r['name'] for r in results if r['after']['check']['status'] == 'timeout' or 'panicked' in r['after']['check']['stderr'] or any(x['status'] != 0 for x in r['after'].get('runs', []))]
    print(json.dumps({'counts': summary, 'after_failures': failures}, indent=2))
    return bool(failures)


if __name__ == '__main__':
    raise SystemExit(main())
