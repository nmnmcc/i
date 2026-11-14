#!/usr/bin/env python3
"""Quiet end-to-end R2 contention and shared-capture measurements.

Build the release compiler through heavy before invoking this script through
direnv. The worker runs only inside quiet; Cargo/heavy are never nested there.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import statistics
import subprocess
import time

ROOT = Path(__file__).resolve().parent.parent
CACHE = Path('/home/agent/Programming/i/.rd/cache')


def source(family, count):
    if family == 'dag':
        return f'''(i/task, (Io) = i/io)
Tree {{ Leaf Node(Tree, Tree) }}
make = (n: Int) -> Tree {{ n |> {{ 0 -> Leaf _ -> {{ x = make(n - 1) Node(x, x) }} }} }}
@main
main = (io ! Io) {{
 value = make({count})
 task.scope((s) {{ child = task.go(s, () {{ value |> {{ Leaf -> 0 Node(_, _) -> 1 }} }})
 assert(task.join(child) == 1) }})
 io.print("ok")
}}
'''
    if family == 'choose':
        declarations = "pick = (a, b, c, n : Int) { { n % 3 } |> { 0 -> a 1 -> b _ -> c } }\nspawn = (s : task.Scope(Never), a : task.Channel(Int), b : task.Channel(Int), c : task.Channel(Int), n : Int) -> Nil ! task.Scheduler | Assert {\n n |> { 0 -> Nil _ -> {\n task.go(s, () { assert(task.sync(task.choose([task.receive_event(a), task.receive_event(b), task.receive_event(c)])) == n) })\n spawn(s, a, b, c, n - 1)\n } }\n}\ndrain = (a : task.Channel(Int), b : task.Channel(Int), c : task.Channel(Int), n : Int) -> Nil ! task.Scheduler {\n n |> { 0 -> Nil _ -> { task.send(pick(a, b, c, n), n) drain(a, b, c, n - 1) } }\n}\n"
        body = f'''a: task.Channel(Int) = task.channel()
 b: task.Channel(Int) = task.channel()
 c: task.Channel(Int) = task.channel()
 spawn(s, a, b, c, {count})
 drain(a, b, c, {count})'''
    else:
        child = 'task.send(c, n)' if family == 'senders' else 'assert(task.receive(c) == n)'
        parent = 'assert(task.receive(c) == n)' if family == 'senders' else 'task.send(c, n)'
        declarations = f'''spawn = (s: task.Scope(Never), c: task.Channel(Int), n: Int) -> Nil ! task.Scheduler | Assert {{
 n |> {{ 0 -> Nil _ -> {{ task.go(s, () {{ {child} }}) spawn(s, c, n - 1) }} }}
}}
drain = (c: task.Channel(Int), n: Int) -> Nil ! task.Scheduler | Assert {{
 n |> {{ 0 -> Nil _ -> {{ {parent} drain(c, n - 1) }} }}
}}
'''
        body = f'c: task.Channel(Int) = task.channel() spawn(s, c, {count}) drain(c, {count})'
    return f'''(i/task, (Io) = i/io)
{declarations}
@main
main = (io ! Io) {{ task.scope((s) {{ {body} }}) io.print("ok") }}
'''


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/i')
    parser.add_argument('--runs', type=int, default=3)
    parser.add_argument('--sizes', type=int, nargs='+', default=[128, 256, 1000, 10000])
    parser.add_argument('--worker', action='store_true')
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    jobs = [(family, n) for family in ['senders', 'receivers', 'choose'] for n in args.sizes]
    jobs += [('dag', 28), ('dag', 60)]
    if not args.worker:
        for family, n in jobs:
            directory = out / f'{family}-{n}'
            (directory / 'src').mkdir(parents=True, exist_ok=True)
            (directory / 'i.toml').write_text('name = "scale"\n')
            (directory / 'src/main.i').write_text(source(family, n))
        subprocess.run([str(CACHE / 'quiet'), 'direnv', 'exec', '/home/agent/Programming/i',
                        'python3', str(Path(__file__).resolve()), str(out), '--worker',
                        '--binary', str(args.binary.resolve()), '--runs', str(args.runs),
                        '--sizes', *map(str, args.sizes)], check=True)
        return
    frozen = {str(p): hashlib.sha256(p.read_bytes()).hexdigest()
              for p in [args.binary, ROOT / 'lib/i/task.i']}
    results = []
    for family, n in jobs:
        directory = out / f'{family}-{n}'
        samples = []
        for iteration in range(args.runs):
            command = [str(args.binary), 'run', '--offline', '--project', str(directory), '--opt', '2']
            started = time.monotonic()
            with subprocess.Popen(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                  start_new_session=True) as process:
                try:
                    stdout, stderr = process.communicate(timeout=20)
                    status = process.returncode
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    stdout, stderr = process.communicate()
                    status = 'timeout'
            elapsed = time.monotonic() - started
            (directory / f'{iteration}.stdout').write_bytes(stdout)
            (directory / f'{iteration}.stderr').write_bytes(stderr)
            sample = dict(command=command, seconds=elapsed, status=status,
                          correct=status == 0 and stdout.decode().strip() == 'ok')
            samples.append(sample)
        result = dict(family=family, n=n, samples=samples,
                      median_seconds=statistics.median(s['seconds'] for s in samples),
                      correct=all(s['correct'] for s in samples))
        results.append(result)
        print(json.dumps(result), flush=True)
        (out / 'results.json').write_text(json.dumps(dict(source_hashes=frozen, results=results), indent=2) + '\n')
    assert all(hashlib.sha256(Path(p).read_bytes()).hexdigest() == h for p, h in frozen.items()), 'source/binary changed'
    assert all(r['correct'] for r in results), 'incorrect or timed-out scale sample'
    assert all(r['median_seconds'] < 10 for r in results if r['n'] == 10000), '10000-task latency gate'


if __name__ == '__main__':
    main()
