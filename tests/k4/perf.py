#!/usr/bin/env python3
"""Interleave checker measurements and retain every sample.

Shared-host numbers are a substitute for the coordinator's quiet window.
"""
import argparse
import json
import os
from pathlib import Path
import resource
import statistics
import subprocess
import time


def measure(binary, project):
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    started = time.perf_counter()
    p = subprocess.run([str(binary), 'check', '--project', str(project)], capture_output=True, timeout=120)
    wall = time.perf_counter() - started
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    if p.returncode:
        raise RuntimeError(p.stderr.decode())
    return {'wall': wall, 'cpu': after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime}


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--before', required=True, type=Path)
    p.add_argument('--after', required=True, type=Path)
    p.add_argument('--out', required=True, type=Path)
    p.add_argument('--pairs', type=int, default=30)
    a = p.parse_args()
    a.out.mkdir(parents=True, exist_ok=True)
    if hasattr(os, 'sched_getaffinity'):
        os.sched_setaffinity(0, {min(os.sched_getaffinity(0))})
    results = {}
    projects = [('std', Path('std'), a.pairs)]
    for n in [25, 50, 100, 150]:
        project = a.out / f'ring{n}'
        (project / 'src').mkdir(parents=True, exist_ok=True)
        (project / 'i.toml').write_text('name = "app"\n')
        source = 'S(N) { S(f: () -> Int ! N, m: (N) -> Nil) }\nuse(N) = (s ! S(N)) { S(f, ..) = s f() }\n'
        for i in range(n):
            source += f'm{i} = (k: Int) {{ k |> {{ 0 -> use() _ -> m{(i+1)%n}(k - 1) }} }}\n'
        (project / 'src/main.i').write_text(source)
        projects.append((f'ring{n}', project, 3))
    for name, project, pairs in projects:
        values = {'before': [], 'after': []}
        for i in range(pairs):
            for label in (['before', 'after'] if i % 2 == 0 else ['after', 'before']):
                values[label].append(measure(getattr(a, label), project))
        results[name] = {'samples': values, 'medians': {label: {key: statistics.median(v[key] for v in samples) for key in ['wall', 'cpu']} for label, samples in values.items()}}
        print(name, results[name]['medians'], flush=True)
        (a.out / 'results.json').write_text(json.dumps(results, indent=2) + '\n')


if __name__ == '__main__':
    main()
