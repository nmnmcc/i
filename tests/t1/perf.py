#!/usr/bin/env python3
"""Measure the §9 budgets; failed checks and failed budgets return nonzero.

Use release binaries, sequentially, in a quiet window. An optional CPU
pins both children equally. The JSON retains every sample and environment;
a busy-machine run is not evidence for the prescribed quiet-window gate.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import statistics
import resource
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
import importlib.util
GEN_SPEC = importlib.util.spec_from_file_location('t1_gen_stress', Path(__file__).with_name('gen-stress.py'))
GEN = importlib.util.module_from_spec(GEN_SPEC)
GEN_SPEC.loader.exec_module(GEN)


def cpu_busy_seconds():
    # Linux accounts guest time in user/nice already; count the first
    # eight fields once, excluding idle and IO wait.
    fields = list(map(int, Path('/proc/stat').read_text().splitlines()[0].split()[1:9]))
    return (sum(fields) - fields[3] - fields[4]) / os.sysconf('SC_CLK_TCK')


def child_cpu_seconds():
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    return usage.ru_utime + usage.ru_stime


def measure(binary, project, runs):
    busy_before = cpu_busy_seconds()
    child_before = child_cpu_seconds()
    wall_before = time.perf_counter()
    command = [str(binary), 'check', '--offline', '--project', str(project)]
    samples = []
    for i in range(runs + 1):
        start = time.perf_counter()
        p = subprocess.run(command, text=True, capture_output=True, timeout=180)
        elapsed = (time.perf_counter() - start) * 1000
        if p.returncode:
            raise RuntimeError(f'{command}: exit {p.returncode}\n{p.stdout}{p.stderr}')
        if i:
            samples.append(elapsed)
    wall = time.perf_counter() - wall_before
    background = max(0, cpu_busy_seconds() - busy_before - (child_cpu_seconds() - child_before))
    return {'ms': samples, 'median_ms': statistics.median(samples), 'background_cpu_fraction': background / (wall * os.cpu_count())}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['std', 'stress'])
    parser.add_argument('before', type=Path)
    parser.add_argument('after', type=Path)
    parser.add_argument('--work', type=Path, default=ROOT / 'target/t1/perf')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--cpu', type=int)
    parser.add_argument('--require-quiet', action='store_true', help='fail if non-benchmark CPU use exceeds five percent of total capacity during either block')
    parser.add_argument('--runs', type=int, default=11)
    parser.add_argument('--project', type=Path, default=ROOT / 'std')
    parser.add_argument('--ignored-tests', action='store_true', help='also run the four debug-counter cases through the heavy-job wrapper')
    args = parser.parse_args()
    if args.runs < 11 or args.runs % 2 == 0:
        parser.error('use an odd number of runs, at least 11')
    if args.cpu is not None:
        os.sched_setaffinity(0, {args.cpu})
    binaries = [args.before.resolve(), args.after.resolve()]
    report = {'mode': args.mode, 'binaries': [{'path': str(b), 'sha256': hashlib.sha256(b.read_bytes()).hexdigest()} for b in binaries], 'load_before': os.getloadavg(), 'affinity': sorted(os.sched_getaffinity(0)), 'runs': args.runs, 'warmups': 1, 'measurements': [], 'gates': []}
    try:
        if args.mode == 'std':
            row = [measure(b, args.project.resolve(), args.runs) for b in binaries]
            ratio = row[1]['median_ms'] / row[0]['median_ms']
            report['measurements'].append({'part': 'std', 'samples': row, 'ratio': ratio})
            report['gates'].append({'name': 'std <= 1.10 before', 'met': ratio <= 1.10})
            report['gates'].append({'name': 'std hard ceiling <= 1.18 before', 'met': ratio <= 1.18})
        else:
            for part in ['keys', 'pick', 'calls']:
                rows = []
                sizes = [250, 500, 1000] if part != 'calls' else [2500, 5000]
                for n in sizes:
                    project = args.work.resolve() / f'{part}-{n}'
                    (project / 'src').mkdir(parents=True, exist_ok=True)
                    (project / 'i.toml').write_text('name = "stress"\n')
                    (project / 'src/main.i').write_text(GEN.source(part, n))
                    samples = [measure(b, project, args.runs) for b in binaries]
                    row = {'part': part, 'n': n, 'samples': samples, 'ratio': samples[1]['median_ms'] / samples[0]['median_ms']}
                    rows.append(row)
                    report['measurements'].append(row)
                growth = rows[-1]['samples'][1]['median_ms'] / rows[-2]['samples'][1]['median_ms']
                report['gates'].append({'name': f'{part} doubling <= 2.5', 'value': growth, 'met': growth <= 2.5})
                if part != 'calls':
                    report['gates'].append({'name': f'{part} at 1000 <= 2 before', 'value': rows[-1]['ratio'], 'met': rows[-1]['ratio'] <= 2})
            if args.ignored_tests:
                command = ['/home/agent/Programming/i/.rd/cache/heavy', 'direnv', 'exec', '/home/agent/Programming/i', 'cargo', 'test', '-p', 'ilang-check', '--lib', '--locked', 't1_stress_', '--', '--ignored', '--nocapture', '--test-threads=1']
                p = subprocess.run(command, cwd=ROOT)
                report['gates'].append({'name': 'four ignored debug-counter stresses', 'met': p.returncode == 0})
    except (RuntimeError, subprocess.TimeoutExpired) as error:
        report['error'] = str(error)
    if args.require_quiet:
        blocks = [sample for row in report['measurements'] for sample in row['samples']]
        peak_background = max((sample['background_cpu_fraction'] for sample in blocks), default=1)
        report['gates'].append({'name': 'background CPU <= 5% of machine capacity', 'value': peak_background, 'met': peak_background <= 0.05})
    report['load_after'] = os.getloadavg()
    report['passed'] = 'error' not in report and all(g['met'] for g in report['gates'])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'passed': report['passed'], 'gates': report['gates'], 'error': report.get('error'), 'report': str(args.output)}, indent=2))
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    sys.exit(main())
