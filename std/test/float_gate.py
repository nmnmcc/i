#!/usr/bin/env python3
"""Quiet-window O2 feasibility measurement, with native libm comparison.

/home/agent/Programming/i/.rd/cache/quiet direnv exec /home/agent/Programming/i \
    python3 std/test/float_gate.py

The CLI reports whole-test times to 1 ms. We amortize instance startup, loop,
checksum and timer quantization over a stated number of calls. Each reported
sample is a batch mean, NOT a measured worst single-call latency. No overhead
is subtracted. Run float_hosts.py first for slow-path classification. This substitute suffices to reject the gate by large margins;
it cannot certify a strict per-input upper bound or 20x throughput acceptance.
The transient test file is removed even after failure. Logs go under target/.
"""
import json
from pathlib import Path
import re
import statistics
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[2]
NAMES = ['sin', 'cos', 'tan', 'asin', 'acos', 'atan', 'atan2', 'hypot', 'radians', 'degrees']


def value(bits):
    return struct.unpack('>d', struct.pack('>Q', bits))[0]


def bits(x):
    return struct.unpack('>Q', struct.pack('>d', x))[0]


def cases():
    rows = []
    for name in NAMES:
        for x in [1e-6, .125, .5, .75, 1., 10., 1e6]:
            if name in ['asin', 'acos'] and x > 1:
                continue
            rows.append(dict(group='ordinary', fn=name, x=x, y=.75, count=32))
        if name not in ['asin', 'acos']:
            for x in [1e300, value(0x7fefffffffffffff)]:
                rows.append(dict(group='huge', fn=name, x=x, y=1., count=4))
        for x in [value(1), value(0x000fffffffffffff)]:
            rows.append(dict(group='subnormal', fn=name, x=x, y=2., count=2))
    # Five MPFR-searched sine midpoints and the exactly-half-subnormal atan2.
    for hexadecimal in ['0x1.38e17aa7782acp+664', '0x1.632c1c37771edp+887',
                        '0x1.3f9db6072ebe5p+898', '0x1.32a67c9220f98p+542',
                        '0x1.b23a321f88473p+395']:
        rows.append(dict(group='hard', fn='sin', x=float.fromhex(hexadecimal), y=0., count=4))
    rows.append(dict(group='hard', fn='atan2', x=value(1), y=2., count=1))
    # Exact realizable norm midpoint from the independent review.
    m, n = 2**26 + 1, 2**26
    rows.append(dict(group='hard', fn='hypot', x=float(2*m*n), y=float(m*m-n*n), count=2))
    for row in rows:
        if row['group'] == 'ordinary' and (
                row['fn'] in ['sin', 'cos'] and abs(row['x']) <= .75 or
                row['fn'] in ['asin', 'acos'] and abs(row['x']) == 1):
            row['count'] = 512
        if row['group'] == 'subnormal' and row['fn'] in ['sin', 'cos', 'tan', 'asin', 'atan']:
            row['count'] = 16384
    return rows


def source(rows):
    out = ['(i/core, std/float)', '']
    checked = {'tan', 'asin', 'acos', 'hypot', 'degrees'}
    for j, row in enumerate(rows):
        name = row['fn']
        args = [row['x'], row['y']] if name in ['atan2', 'hypot'] else [row['x']]
        call = f'float.{name}(' + ', '.join(f'core.float_from_bits(0x{bits(x):016x})' for x in args) + ')'
        if name in checked:
            call += ' |> { Ok(v) -> v Error(_) -> 0.0 }'
        out += ['@test', f'gate_{j:03} = () {{',
                ' loop = (left: Int, checksum: Int) -> Int {',
                '  { left == 0 } |> {', '   True -> checksum', '   False -> {',
                f'    result = {call}',
                '    loop(left - 1, core.bit_xor(checksum, core.float_to_bits(result)))',
                '   }', '  }', ' }',
                f' assert(loop({row["count"]}, 0) >= 0)', '}', '']
    return '\n'.join(out)


def run():
    rows = cases()
    work = ROOT / 'target/std3-gate'
    work.mkdir(parents=True, exist_ok=True)
    binary = work / 'native'
    subprocess.run(['cc', '-O2', '-fno-builtin', str(ROOT / 'std/test/float_native.c'),
                    '-lm', '-o', str(binary)], check=True)
    fixture = ROOT / 'std/test/float_gate_probe_test.i'
    assert not fixture.exists()
    try:
        fixture.write_text(source(rows))
        times = [[] for _ in rows]
        for repeat in range(5):
            result = subprocess.run([str(ROOT / 'target/release/i'), 'test', '--project', 'std',
                                     'float_gate_probe_test', '--opt', '2', '--time'],
                                    text=True, capture_output=True, check=True, cwd=ROOT)
            (work / f'run-{repeat}.log').write_text(result.stdout + result.stderr)
            matches = re.findall(r'gate_(\d+) \(([0-9.]+) s,', result.stdout)
            assert len(matches) == len(rows)
            for index, elapsed in matches:
                times[int(index)].append(float(elapsed) / rows[int(index)]['count'])
            print('completed repetition', repeat + 1, flush=True)
        for j, row in enumerate(rows):
            row['seconds'] = times[j]
            row['native_seconds'] = [float(subprocess.check_output(
                [str(binary), str(NAMES.index(row['fn'])), row['x'].hex(), row['y'].hex()],
                text=True)) for _ in range(5)]
        (work / 'measurements.json').write_text(json.dumps(rows, indent=2))
        classifications = json.loads((ROOT / 'target/std3-hosts/slow-share.json').read_text())
        assert len(classifications) == len(rows), 'run float_hosts.py first'
        for row, classified in zip(rows, classifications):
            assert (row['fn'], row['x'], row['y']) == (classified['fn'], classified['x'], classified['y'])
            row['slow'] = classified['slow']
        (work / 'measurements.json').write_text(json.dumps(rows, indent=2))
        for group in ['ordinary', 'huge', 'subnormal', 'hard']:
            selected = [r for r in rows if r['group'] == group]
            samples = [s for r in selected for s in r['seconds']]
            native = [s for r in selected for s in r['native_seconds']]
            ratios = [statistics.median(r['seconds']) / statistics.median(r['native_seconds'])
                      for r in selected]
            print(group, 'slow_calls', sum(r['count'] for r in selected if r['slow']), '/', sum(r['count'] for r in selected), 'slow_inputs', sum(r['slow'] for r in selected), '/', len(selected), 'inputs', len(selected), 'median_us', statistics.median(samples)*1e6,
                  'worst_us', max(samples)*1e6, 'native_median_ns', statistics.median(native)*1e9,
                  'median_ratio', statistics.median(ratios), flush=True)
    finally:
        fixture.unlink(missing_ok=True)


if __name__ == '__main__':
    run()
