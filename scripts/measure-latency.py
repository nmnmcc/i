#!/usr/bin/env python3
"""Measure `i check` and language-server latency on one machine.

This is a local, reproducible measurement method, not a CI gate. It times
`i check` over a project and the language server (`i lsp`, stdio) from a
didOpen to the first publishDiagnostics and from a didChange that
introduces an error to the next publishDiagnostics that reports it. Every
sample is kept; medians and p90 are summarised. One or more binaries may
be measured, and the first is the base of the ratios, so two builds or
two worktrees can be compared (A/B). Binaries are identified by path and
sha256.

Use release binaries and a quiet window; machine load before and after
each block is recorded, along with the background CPU fraction (busy CPU
outside the timed children, as in tests/t1/perf.py). A busy-machine run
is a busy-machine run: say so when reporting.

Examples:

    scripts/measure-latency.py all \\
        --binary target/release/i --label release \\
        --binary /path/to/other/target/release/i --label other \\
        --check-project examples/tour --check-project std \\
        --lsp-project examples/tour --lsp-file examples/tour/src/main.i \\
        --runs 10 --output target/perf/latency.json

    scripts/measure-latency.py check --binary target/release/i \\
        --check-project examples/tour --runs 11 --output target/perf/check.json

The language-server samples start a fresh server per sample: the open
latency is measured from sending didOpen after the initialize handshake,
so it excludes process start and the handshake; they are recorded
separately. The change latency includes the server's debounce.
"""
import argparse
import hashlib
import json
import math
import os
import queue
import resource
import statistics
import subprocess
import sys
import threading
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TIMEOUT = 120.0


def cpu_busy_seconds():
    # Linux accounts guest time in user/nice already; count the first
    # eight fields once, excluding idle and IO wait.
    fields = list(map(int, Path('/proc/stat').read_text().splitlines()[0].split()[1:9]))
    return (sum(fields) - fields[3] - fields[4]) / os.sysconf('SC_CLK_TCK')


def child_cpu_seconds():
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    return usage.ru_utime + usage.ru_stime


def p90(samples):
    ordered = sorted(samples)
    return ordered[max(0, math.ceil(0.9 * len(ordered)) - 1)]


def summary(samples):
    return {
        'ms': samples,
        'n': len(samples),
        'median_ms': statistics.median(samples),
        'p90_ms': p90(samples),
        'min_ms': min(samples),
        'max_ms': max(samples),
    }


def block_load():
    return {'loadavg': os.getloadavg()}


def run_check(binary, project, runs, timeout):
    busy_before = cpu_busy_seconds()
    child_before = child_cpu_seconds()
    wall_before = time.perf_counter()
    command = [str(binary), 'check', '--offline', '--project', str(project)]
    samples = []
    for i in range(runs + 1):
        start = time.perf_counter()
        p = subprocess.run(command, text=True, capture_output=True, timeout=timeout)
        elapsed = (time.perf_counter() - start) * 1000
        if p.returncode:
            raise RuntimeError(f'{command}: exit {p.returncode}\n{p.stdout}{p.stderr}')
        if i:
            samples.append(elapsed)
    wall = time.perf_counter() - wall_before
    background = max(0, cpu_busy_seconds() - busy_before - (child_cpu_seconds() - child_before))
    result = summary(samples)
    result['command'] = command
    result['warmups'] = 1
    result['background_cpu_fraction'] = background / (wall * os.cpu_count())
    return result


def write_message(stream, message):
    data = json.dumps(message).encode()
    stream.write(b'Content-Length: %d\r\n\r\n' % len(data))
    stream.write(data)
    stream.flush()


class Reader(threading.Thread):
    """Reads framed messages and queues them with the time they arrived."""

    def __init__(self, stream):
        super().__init__(daemon=True)
        self.stream = stream
        self.messages = queue.Queue()

    def run(self):
        try:
            while True:
                length = None
                while True:
                    line = self.stream.readline()
                    if not line:
                        raise EOFError('the server closed its output')
                    line = line.strip()
                    if not line:
                        break
                    name, _, value = line.partition(b':')
                    if name.lower() == b'content-length':
                        length = int(value.strip())
                if length is None:
                    raise ValueError('a message without Content-Length')
                body = self.stream.read(length)
                if len(body) != length:
                    raise EOFError('the server closed its output mid-message')
                self.messages.put((time.perf_counter(), json.loads(body)))
        except Exception as error:  # noqa: BLE001 - handed to the waiter
            self.messages.put((time.perf_counter(), error))


def receive(reader, deadline, what):
    remaining = deadline - time.perf_counter()
    if remaining <= 0:
        raise TimeoutError(f'timed out waiting for {what}')
    try:
        arrived, message = reader.messages.get(timeout=remaining)
    except queue.Empty:
        raise TimeoutError(f'timed out waiting for {what}') from None
    if isinstance(message, Exception):
        raise RuntimeError(f'the server failed while waiting for {what}: {message}')
    return arrived, message


def diagnostics_in(message, uri=None):
    if message.get('method') != 'textDocument/publishDiagnostics':
        return None
    params = message.get('params') or {}
    if uri is not None and params.get('uri') != uri:
        return None
    return params.get('diagnostics') or []


def await_diagnostics(reader, deadline, uri, require_reported, what):
    while True:
        arrived, message = receive(reader, deadline, what)
        diags = diagnostics_in(message, uri)
        if diags is None:
            continue
        if require_reported and not diags:
            # An empty publish may be the clear of an older check; the
            # error's diagnostics come in a later publish.
            continue
        return arrived, diags


def lsp_sample(binary, project, file, error_line, timeout):
    cwd = project.resolve()
    path = file.resolve()
    uri = path.as_uri()
    text = path.read_text()
    proc = subprocess.Popen(
        [str(binary), 'lsp'],
        cwd=cwd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    reader = Reader(proc.stdout)
    reader.start()
    errors = []
    stderr_reader = threading.Thread(target=lambda: errors.append(proc.stderr.read()), daemon=True)
    stderr_reader.start()
    deadline = time.perf_counter() + timeout
    try:
        started = time.perf_counter()
        write_message(proc.stdin, {
            'jsonrpc': '2.0',
            'id': 1,
            'method': 'initialize',
            'params': {
                'processId': None,
                'rootUri': cwd.as_uri(),
                'capabilities': {},
            },
        })
        while True:
            _, message = receive(reader, deadline, 'the initialize response')
            if message.get('id') == 1 and 'result' in message:
                break
        handshake_ms = (time.perf_counter() - started) * 1000
        write_message(proc.stdin, {'jsonrpc': '2.0', 'method': 'initialized', 'params': {}})

        opened = time.perf_counter()
        write_message(proc.stdin, {
            'jsonrpc': '2.0',
            'method': 'textDocument/didOpen',
            'params': {'textDocument': {'uri': uri, 'languageId': 'i', 'version': 1, 'text': text}},
        })
        arrived, diags = await_diagnostics(reader, deadline, uri, False, 'the first diagnostics')
        open_ms = (arrived - opened) * 1000
        open_diagnostics = len(diags)

        changed = time.perf_counter()
        write_message(proc.stdin, {
            'jsonrpc': '2.0',
            'method': 'textDocument/didChange',
            'params': {
                'textDocument': {'uri': uri, 'version': 2},
                'contentChanges': [{'text': text + '\n' + error_line + '\n'}],
            },
        })
        arrived, diags = await_diagnostics(reader, deadline, uri, True, 'the diagnostics of the error')
        change_ms = (arrived - changed) * 1000
        error_count = len(diags)
        return {
            'handshake_ms': handshake_ms,
            'open_ms': open_ms,
            'open_diagnostics': open_diagnostics,
            'change_error_ms': change_ms,
            'error_diagnostics': error_count,
        }
    except Exception as error:  # noqa: BLE001 - reported with the sample
        detail = ''.join(errors)[-2000:]
        raise RuntimeError(f'{error}\nserver stderr:\n{detail}') from None
    finally:
        proc.stdin.close()
        try:
            proc.terminate()
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()


def run_lsp(binary, project, file, error_line, runs, timeout):
    busy_before = cpu_busy_seconds()
    child_before = child_cpu_seconds()
    wall_before = time.perf_counter()
    samples = []
    for i in range(runs + 1):
        sample = lsp_sample(binary, project, file, error_line, timeout)
        if i:
            samples.append(sample)
    wall = time.perf_counter() - wall_before
    background = max(0, cpu_busy_seconds() - busy_before - (child_cpu_seconds() - child_before))
    result = {
        'warmups': 1,
        'open': summary([s['open_ms'] for s in samples]),
        'change_error': summary([s['change_error_ms'] for s in samples]),
        'handshake': summary([s['handshake_ms'] for s in samples]),
        'samples': samples,
        'background_cpu_fraction': background / (wall * os.cpu_count()),
    }
    if any(s['error_diagnostics'] == 0 for s in samples):
        raise RuntimeError('a change sample produced no reported diagnostics; the error line does not fail the check')
    return result


def binary_id(path, label):
    path = path.resolve()
    return {
        'label': label or str(path),
        'path': str(path),
        'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('mode', choices=['check', 'lsp', 'all'], help='what to measure')
    parser.add_argument('--binary', type=Path, action='append', required=True, help='an i binary; the first is the base of the ratios')
    parser.add_argument('--label', action='append', default=[], help='a name for each --binary, in order')
    parser.add_argument('--check-project', type=Path, action='append', default=[], help='a package directory for i check')
    parser.add_argument('--lsp-project', type=Path, action='append', default=[], help='a package directory for the language server')
    parser.add_argument('--lsp-file', type=Path, action='append', default=[], help='the open document; each is used with the lsp-project at the same position')
    parser.add_argument('--error-line', default='broken = missing_name', help='an appended line that makes the open file fail the check')
    parser.add_argument('--runs', type=int, default=10, help='timed samples per block (default 10)')
    parser.add_argument('--timeout', type=float, default=TIMEOUT, help='per-sample timeout in seconds')
    parser.add_argument('--cpu', type=int, help='run the timed children pinned to this CPU, as tests/t1/perf.py can')
    parser.add_argument('--note', help='free text kept in the report')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.runs < 1:
        parser.error('--runs must be positive')
    if args.cpu is not None:
        os.sched_setaffinity(0, {args.cpu})
    if len(args.label) > len(args.binary):
        parser.error('--label is given more than --binary')
    labels = args.label + [None] * (len(args.binary) - len(args.label))
    if args.mode in ('check', 'all') and not args.check_project:
        parser.error('give at least one --check-project')
    if args.mode in ('lsp', 'all'):
        if not args.lsp_project:
            parser.error('give at least one --lsp-project')
        if len(args.lsp_project) != len(args.lsp_file):
            parser.error('give one --lsp-file for each --lsp-project')
    binaries = [path.resolve() for path in args.binary]
    for path in binaries:
        if not path.is_file():
            parser.error(f'no such binary: {path}')
    for path in args.check_project:
        if not (path / 'i.toml').is_file():
            parser.error(f'no i.toml in {path}')
    for path in args.lsp_file:
        if not path.is_file():
            parser.error(f'no such file: {path}')

    report = {
        'kind': 'local measurement, not a CI gate',
        'mode': args.mode,
        'runs': args.runs,
        'warmups_per_block': 1,
        'note': args.note,
        'cwd': str(Path.cwd()),
        'git_head': subprocess.run(['git', 'rev-parse', 'HEAD'], text=True, capture_output=True).stdout.strip() or None,
        'machine': {
            'cpu_count': os.cpu_count(),
            'affinity': sorted(os.sched_getaffinity(0)),
            'load_before': os.getloadavg(),
        },
        'binaries': [binary_id(path, label) for path, label in zip(binaries, labels)],
        'check': [],
        'lsp': [],
    }
    try:
        if args.mode in ('check', 'all'):
            for project in args.check_project:
                rows = []
                for binary in binaries:
                    rows.append(run_check(binary, project.resolve(), args.runs, args.timeout * 4))
                base = rows[0]['median_ms']
                report['check'].append({
                    'project': str(project.resolve()),
                    'load_before': block_load(),
                    'rows': rows,
                    'median_ratio': [row['median_ms'] / base for row in rows[1:]],
                })
        if args.mode in ('lsp', 'all'):
            for project, file in zip(args.lsp_project, args.lsp_file):
                rows = []
                for binary in binaries:
                    rows.append(run_lsp(binary, project.resolve(), file.resolve(), args.error_line, args.runs, args.timeout))
                base_open = rows[0]['open']['median_ms']
                base_change = rows[0]['change_error']['median_ms']
                report['lsp'].append({
                    'project': str(project.resolve()),
                    'file': str(file.resolve()),
                    'error_line': args.error_line,
                    'load_before': block_load(),
                    'rows': rows,
                    'median_ratio': {
                        'open': [row['open']['median_ms'] / base_open for row in rows[1:]],
                        'change_error': [row['change_error']['median_ms'] / base_change for row in rows[1:]],
                    },
                })
    except (RuntimeError, TimeoutError) as error:
        report['error'] = str(error)
    report['machine']['load_after'] = os.getloadavg()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')

    # A short table for the terminal; the JSON has every sample.
    for block in report['check']:
        print(f'check {block["project"]}')
        for binary, row in zip(report['binaries'], block['rows']):
            print(f'  {binary["label"]}: median {row["median_ms"]:.1f} ms, p90 {row["p90_ms"]:.1f} ms, n {row["n"]}')
    for block in report['lsp']:
        print(f'lsp {block["file"]}')
        for binary, row in zip(report['binaries'], block['rows']):
            print(f'  {binary["label"]}: open median {row["open"]["median_ms"]:.1f} ms (p90 {row["open"]["p90_ms"]:.1f}), '
                  f'change->error median {row["change_error"]["median_ms"]:.1f} ms (p90 {row["change_error"]["p90_ms"]:.1f})')
    if 'error' in report:
        print(f'error: {report["error"]}', file=sys.stderr)
        print(f'report: {args.output}', file=sys.stderr)
        return 1
    print(f'report: {args.output}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
