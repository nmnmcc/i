"""End-to-end checker measurements. Raw per-child wait4 counters, no numeric gate."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import signal
import shutil
import statistics
import subprocess
import tempfile
import time

from corpus import ROOT, CORPUS, dump, git, sha


def workloads():
    yield 'startup', 'x = 1\n'
    for n in [64, 256, 1024, 4096]:
        for value in [False, True]:
            for reverse in [False, True]:
                declarations = ['A0 = value' if value else 'A0 = Int'] + [f'A{i} = A{i-1}' for i in range(1, n)]
                if reverse:
                    declarations.reverse()
                yield f'alias-{n}-{"value" if value else "ordinary"}-{"reverse" if reverse else "forward"}', 'value: Int = 1\n' + '\n'.join(declarations) + f'\nx: A{n-1} = 2\n'
    for n in [16, 64, 256, 1024]:
        union = ' | '.join(str(i) for i in range(n))
        yield f'literal-union-{n}', f'Input = {union}\nf = (x: Input) {{ x }}\n'
        clauses = '\n'.join(f'{i} -> {i}' for i in range(n))
        yield f'clauses-{n}', f'f = (x: Int) {{ x |> {{\n{clauses}\n_ -> -1\n}} }}\n'
        defs = '\n'.join(f'T{i} {{ T{i} }}' for i in range(n))
        union = ' | '.join(f'T{i}' for i in range(n))
        yield f'wide-union-{n}', defs + f'\nf = (x: {union}) {{ x }}\n'
    head = 'S(N) { S(f: () -> Int ! N, m: (N) -> Nil) }\nuse(N) = (s ! S(N)) { s.f() }\n'
    defs = '\n'.join(f'f{i} = (k: Int) {{ k |> {{ 0 -> use() _ -> f{(i+1)%100}(k - 1) }} }}' for i in range(100))
    yield 'service-ring-100', head + defs + '\n'


def measure(argv, timeout):
    # GNU time forks after exec, avoiding the Python parent's inherited RSS floor.
    timer = shutil.which('time')
    if not timer:
        raise RuntimeError('GNU time is required for per-checker peak RSS')
    with tempfile.TemporaryFile(dir=CORPUS / '.work') as stdout, tempfile.TemporaryFile(dir=CORPUS / '.work') as stderr, tempfile.NamedTemporaryFile(dir=CORPUS / '.work') as counters:
        measured = [timer, '-f', '%U %S %M', '-o', counters.name, '--', *argv]
        start = time.perf_counter()
        child = subprocess.Popen(measured, cwd=ROOT, stdout=stdout, stderr=stderr, start_new_session=True)
        timed_out = False
        while True:
            pid, status, usage = os.wait4(child.pid, os.WNOHANG)
            if pid:
                break
            if not timed_out and time.perf_counter() - start > timeout:
                timed_out = True
                # Keep GNU time alive to reap the checker and record censored CPU/RSS.
                children = Path(f'/proc/{child.pid}/task/{child.pid}/children').read_text().split()
                for pid_text in children:
                    try:
                        os.kill(int(pid_text), signal.SIGKILL)
                    except ProcessLookupError:
                        pass
            if timed_out and time.perf_counter() - start > timeout + 2:
                os.killpg(child.pid, signal.SIGKILL)
                _, status, usage = os.wait4(child.pid, 0)
                break
            time.sleep(.001)
        wall = time.perf_counter() - start
        child.returncode = os.waitstatus_to_exitcode(status)
        stdout.seek(0)
        stderr.seek(0)
        out, err = stdout.read(), stderr.read()
        raw_counters = Path(counters.name).read_text()
        fields = raw_counters.splitlines()[-1].split() if raw_counters else []
        if len(fields) == 3:
            user, system, peak_kib = map(float, fields)
        else:
            raise RuntimeError('GNU time produced no counters: ' + raw_counters)
    return {'command': argv, 'exit': child.returncode, 'timeout': timed_out,
            'measurement_tool': timer, 'raw_counters': raw_counters,
            'cpu_resolution_seconds': 0.01,
            'wall_seconds': wall, 'user_seconds': user, 'system_seconds': system,
            'cpu_seconds': user + system,
            'peak_rss_bytes': int(peak_kib * 1024),
            'instructions_retired': None, 'stdout_sha256': sha(out), 'stderr_sha256': sha(err),
            'stderr': err.decode(errors='replace'), 'load_average': list(os.getloadavg())}


def compare(paths):
    runs = [json.loads(p.read_text()) for p in paths]
    if len(runs) == 1:
        run = runs[0]
        labels = sorted({s['binary'] for s in run['samples']})
        if labels != ['baseline', 'candidate']:
            raise SystemExit('one input requires an interleaved baseline/candidate bundle')
        runs = [dict(run, samples=[s for s in run['samples'] if s['binary'] == label]) for label in labels]
    elif len(runs) != 2 or any(len({s['binary'] for s in r['samples']}) != 1 for r in runs):
        raise SystemExit('compare two single-binary bundles or one interleaved bundle')
    if runs[0]['catalogue'] != runs[1]['catalogue']:
        raise SystemExit('workload source inputs differ')
    groups = []
    for run in runs:
        by_name = {}
        for sample in run['samples']:
            by_name.setdefault(sample['workload'], []).append(sample)
        groups.append(by_name)
    if groups[0].keys() != groups[1].keys():
        raise SystemExit('workload sets differ')
    for name in groups[0]:
        censored = [any(s['timeout'] for s in group[name]) for group in groups]
        exits = [sorted({s['exit'] for s in group[name]}) for group in groups]
        valid = not any(censored) and exits[0] == exits[1]
        row = {'workload': name, 'censored': censored, 'exit_codes': exits, 'completion_comparison': valid}
        for metric in ['wall_seconds', 'cpu_seconds', 'peak_rss_bytes']:
            medians = []
            for group in groups:
                values = [s[metric] for s in group[name]]
                medians.append(statistics.median(values))
            row[metric] = {'baseline': medians[0], 'candidate': medians[1], 'ratio': medians[1] / medians[0] if valid and medians[0] else None}
        print(json.dumps(row))


def summarize(path):
    run = json.loads(path.read_text())
    groups = {}
    for sample in run['samples']:
        groups.setdefault(sample['workload'], []).append(sample)
    summary = {'compiler': run['compiler'], 'binaries': run['binaries'], 'samples': len(run['samples']),
               'workloads': len(groups), 'censored': [], 'metrics': {},
               'note': 'Censored values describe observations before the watchdog; they are not completed checker costs. GNU time CPU counters have 0.01-second resolution.'}
    if len(run['binaries']) != 1:
        raise SystemExit('summarize a single-binary bundle; use --compare for an interleaved pair')
    for name, samples in groups.items():
        if any(s['timeout'] for s in samples):
            summary['censored'].append(name)
        summary['metrics'][name] = {metric: {'min': min(s[metric] for s in samples),
                                           'median': statistics.median(s[metric] for s in samples),
                                           'max': max(s[metric] for s in samples)}
                                    for metric in ['wall_seconds', 'cpu_seconds', 'peak_rss_bytes']}
    return summary


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/i')
    parser.add_argument('--candidate', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--runs', type=int, default=5)
    parser.add_argument('--timeout', type=float, default=30)
    parser.add_argument('--filter', default='')
    parser.add_argument('--compare', nargs='+', type=Path)
    parser.add_argument('--summarize', type=Path)
    args = parser.parse_args()
    if args.compare:
        compare(args.compare)
        return
    if args.summarize:
        summary = summarize(args.summarize)
        if args.output:
            dump(args.output, summary)
        else:
            print(json.dumps(summary, indent=2))
        return
    if not args.output or args.runs < 2:
        parser.error('recording requires --output and at least two repetitions')
    binaries = [('baseline', args.binary.resolve())]
    if args.candidate:
        binaries.append(('candidate', args.candidate.resolve()))
    bundle = {'compiler': git('rev-parse', 'HEAD'), 'compiler_tree': git('rev-parse', 'HEAD:crates'), 'source_dirty': bool(git('status', '--porcelain', '--', 'crates', 'lib', 'std')),
              'platform': platform.platform(), 'machine': platform.machine(), 'cpu_count': os.cpu_count(),
              'cpu_model': next((l.split(':', 1)[1].strip() for l in Path('/proc/cpuinfo').read_text().splitlines() if l.startswith('model name')), None),
              'binaries': {name: {'path': str(path), 'sha256': sha(path.read_bytes())} for name, path in binaries},
              'instruction_counter': 'not collected; unavailable in this harness',
              'measurement': 'end-to-end i check --types through GNU time; parse/load/check/display/process startup and measurement wrapper included',
              'quiet_window': 'caller must acquire cache/quiet; load averages recorded, not proof of isolation',
              'deferred': [{'workload': 'record-growth', 'reason': 'T3 held; tests/t3-*.py are the reference models'}],
              'catalogue': [], 'samples': []}
    work = CORPUS / '.work'
    work.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='i-checker-bench-', dir=work) as temp:
        entries = []
        for name, source in workloads():
            if args.filter not in name:
                continue
            path = Path(temp) / name
            path.joinpath('src').mkdir(parents=True)
            path.joinpath('i.toml').write_text('name = "scale"\n')
            path.joinpath('src/main.i').write_text(source)
            entries.append((name, path))
            bundle['catalogue'].append({'name': name, 'source': source, 'source_sha256': sha(source.encode())})
        if args.filter in 'std':
            entries.append(('std', ROOT / 'std'))
            std_sources = [(str(p.relative_to(ROOT)), sha(p.read_bytes())) for p in sorted((ROOT / 'std').rglob('*.i'))]
            bundle['catalogue'].append({'name': 'std', 'sources': std_sources})
        if not entries:
            raise SystemExit('no workloads selected')
        for name, project in entries:
            for label, binary in binaries:
                warmup = measure([str(binary), 'check', '--types', '--offline', '--project', str(project)], args.timeout)
                if warmup['timeout'] or warmup['exit'] != 0:
                    bundle.setdefault('preflight_failures', []).append({'workload': name, 'binary': label, **warmup})
            for rep in range(args.runs):
                # Rotate baseline/candidate to control temporal drift.
                order = binaries[rep % len(binaries):] + binaries[:rep % len(binaries)]
                for label, binary in order:
                    sample = measure([str(binary), 'check', '--types', '--offline', '--project', str(project)], args.timeout)
                    bundle['samples'].append({'workload': name, 'binary': label, 'repetition': rep + 1, **sample})
                    dump(args.output, bundle)
            samples = [s for s in bundle['samples'] if s['workload'] == name]
            print(f'{name}: wall median {statistics.median(s["wall_seconds"] for s in samples):.6f}s', flush=True)
    dump(args.output, bundle)
    if bundle.get('preflight_failures') or any(s['timeout'] or s['exit'] != 0 for s in bundle['samples']):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
