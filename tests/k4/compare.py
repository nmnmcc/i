#!/usr/bin/env python3
"""Compare deterministic generated projects, including both runtime levels."""
import argparse
import importlib.util
import json
import re
from pathlib import Path
import subprocess


def command(binary, args, project, timeout=60):
    try:
        p = subprocess.run([str(binary), *args, '--project', str(project)], capture_output=True, text=True, timeout=timeout)
        return {'status': p.returncode, 'stdout': p.stdout, 'stderr': p.stderr}
    except subprocess.TimeoutExpired:
        return {'status': 'timeout', 'stdout': '', 'stderr': ''}


def bound_recursion(source):
    """Replace only back edges in generated Int helper graphs.

    The reviewer generator can emit h0 = () { run(h0) }. That program
    correctly diverges. Keep its original source, but use a pure Int
    callback at recursive edges for the bounded runtime comparison.
    """
    pattern = re.compile(r'(?ms)^(h[0-9]+)\s*=(.*?)(?=^h[0-9]+\s*=|^@main)')
    parts = list(pattern.finditer(source))
    names = {m[1] for m in parts}
    graph = {m[1]: set(re.findall(r'\bh[0-9]+\b', m[2])) & names for m in parts}
    def reaches(start, end, seen):
        if start == end:
            return True
        if start in seen:
            return False
        seen.add(start)
        return any(reaches(n, end, seen) for n in graph[start])
    edges = {name: {n for n in deps if reaches(n, name, set())} for name, deps in graph.items()}
    if not any(edges.values()):
        return source, False
    def replace(m):
        body = re.sub(r'\bh[0-9]+\b', lambda n: 'k4_pure' if n[0] in edges[m[1]] else n[0], m[2])
        return m[1] + ' =' + body
    return 'k4_pure = () { 1 }\n' + pattern.sub(replace, source), True


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--before', required=True, type=Path)
    p.add_argument('--after', required=True, type=Path)
    p.add_argument('--out', required=True, type=Path)
    p.add_argument('--count', type=int, default=100)
    p.add_argument('--seed', type=int, default=44)
    p.add_argument('--bound-recursion', action='store_true')
    p.add_argument('--runtime-timeout', type=float, default=60)
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
            original = source
            bounded = False
            if a.bound_recursion:
                source, bounded = bound_recursion(source)
            (project / 'original.txt').write_text(original)
            (project / 'src/main.i').write_text(source)
            row = {'name': name, 'bounded': bounded}
            for version, binary in [('before', a.before), ('after', a.after)]:
                checked = command(binary, ['check', '--types'], project)
                row[version] = {'check': checked}
                if checked['status'] == 0:
                    row[version]['runs'] = [command(binary, ['run', '--opt', opt], project, a.runtime_timeout) for opt in ['0', '2']]
            results.append(row)
            (a.out / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
    summary = {}
    for row in results:
        key = 'before={} after={}'.format(row['before']['check']['status'], row['after']['check']['status'])
        summary[key] = summary.get(key, 0) + 1
    def diverges(row):
        source = (a.out / row['name'] / 'src/main.i').read_text()
        # Exact source-level recursion: each call immediately calls itself.
        return bool(re.search(r'(?m)^h0 = \(\) \{ run\(h0\) \}\s*$', source)) and 'run(h0)' in source.split('@main')[-1]
    expected_divergence = [r['name'] for r in results if any(x['status'] == 'timeout' for x in r['after'].get('runs', [])) and diverges(r)]
    failures = [r['name'] for r in results if r['after']['check']['status'] == 'timeout' or 'panicked' in r['after']['check']['stderr'] or any(x['status'] != 0 and not (x['status'] == 'timeout' and r['name'] in expected_divergence) for x in r['after'].get('runs', []))]
    changes = [r['name'] for r in results if r['before']['check']['status'] == 0 and r['after']['check']['status'] != 0]
    possible_regressions = [r['name'] for r in results if r['name'] in changes and all(x['status'] == 0 for x in r['before'].get('runs', []))]
    print(json.dumps({'counts': summary, 'after_failures': failures, 'expected_source_divergence': expected_divergence, 'accepted_then_rejected': changes, 'base_runs_fine_after_rejects': possible_regressions, 'bounded_programs': [r['name'] for r in results if r['bounded']]}, indent=2))
    return bool(failures)


if __name__ == '__main__':
    raise SystemExit(main())
