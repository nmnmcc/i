#!/usr/bin/env python3
"""Regenerate the independent review's 331 MPFR known answers.

UPDATE=1 direnv exec /home/agent/Programming/i /tmp/review-std3/venv/bin/python \
    std/test/float_oracle.py
Requires gmpy2 and MPFR (review environment: MPFR 4.2.2). This is a development
oracle, never a runtime dependency. MPFR evaluates the functions independently
of i's series and range reduction. Inputs and the midpoint search reproduce
math_review.py, extra_math.py, hypot_ties.py and underflow_review.py from the
independent review. No exact-rational model output is used as an expectation.
"""
import math
import os
from pathlib import Path
import random
import struct
import subprocess
import gmpy2 as g

SIGN = 1 << 63
MAX = 0x7fefffffffffffff


def bits(x):
    return struct.unpack('>Q', struct.pack('>d', x))[0]


def value(n):
    return struct.unpack('>d', struct.pack('>Q', n))[0]


def cases():
    tiny, normal, largest = value(1), value(1 << 52), value(MAX)
    xs = [0., -0., tiny, -tiny, normal, math.nextafter(normal, 0), 1., -1.,
          .5, math.nextafter(1., 0), math.nextafter(1., 2), math.pi / 2,
          math.nextafter(math.pi / 2, 0), math.nextafter(math.pi / 2, math.inf),
          math.pi, 1e20, 1e300, largest]
    rng = random.Random(3917)
    xs += [value(rng.randrange(MAX)) for _ in range(4)]
    rows = [(name, [x]) for name in
            ['sin', 'cos', 'tan', 'atan', 'asin', 'acos', 'radians', 'degrees']
            for x in xs]
    pairs = [(a, b) for a in [0., -0., tiny, -tiny, largest, -largest]
             for b in [0., -0., tiny, -tiny, largest, -largest]]
    pairs += [(3., 4.), (normal, tiny), (1., tiny)]
    rows += [(name, list(args)) for name in ['hypot', 'atan2'] for args in pairs]
    with g.context(precision=4096):
        centers = [float(g.const_pi() * g.mpfr(k) / 2)
                   for k in [2, 3, 127, 2**32 + 1, 2**100, 2**900]]
    rows += [(name, [x]) for center in centers
             for x in [math.nextafter(center, 0), center,
                       math.nextafter(center, math.inf)]
             for name in ['sin', 'cos', 'tan']]
    rng = random.Random(8041)
    best = []
    with g.context(precision=256):
        for _ in range(12000):
            x = value(rng.randrange(1, MAX))
            y = g.sin(g.mpfr(x))
            rounded = float(y)
            neighbor = math.nextafter(rounded, math.inf if y > g.mpfr(rounded)
                                      else -math.inf)
            midpoint = (g.mpfr(rounded) + g.mpfr(neighbor)) / 2
            distance = abs(y - midpoint) / abs(g.mpfr(rounded) - g.mpfr(neighbor))
            best.append((float(distance), x))
    rows += [('sin', [x]) for _, x in sorted(best)[:5]]
    for m, n, factor in [(2**26 + 1, 2**26, 1), (2**25 + 1, 2**25, 7)]:
        a, b = factor * 2 * m * n, factor * (m*m - n*n)
        for exponent in [0, -1074, 970]:
            rows.append(('hypot', [float(g.mpq(v) * g.mpq(2)**exponent)
                                   for v in [a, b]]))
    rows += [('atan2', [y, x]) for y in [tiny, -tiny]
             for x in [2., math.nextafter(2., 0), math.nextafter(2., math.inf), largest]]
    rows += [('radians', [x]) for x in [value(28), value(29), -value(28), -value(29)]]
    assert len(rows) == 331
    return rows


def oracle(name, args):
    if name in ['asin', 'acos'] and abs(args[0]) > 1:
        return None
    # A 4096-bit result alone is not a certificate. Require both directed
    # endpoints to round to the same binary64; increase precision otherwise.
    precision = 4096
    while True:
        endpoints = []
        for direction in [g.RoundDown, g.RoundUp]:
            with g.context(precision=precision, round=direction):
                aa = [g.mpfr(v) for v in args]
                if name in ['radians', 'degrees']:
                    negative = bool(bits(args[0]) & SIGN)
                    opposite = g.RoundUp if direction == g.RoundDown else g.RoundDown
                    magnitude_direction = opposite if negative else direction
                    with g.context(precision=precision, round=magnitude_direction):
                        if name == 'radians':
                            magnitude = abs(aa[0]) * g.const_pi() / 180
                        else:
                            pi_direction = (g.RoundUp if magnitude_direction == g.RoundDown
                                            else g.RoundDown)
                            with g.context(precision=precision, round=pi_direction):
                                pi = g.const_pi()
                            magnitude = abs(aa[0]) * 180 / pi
                    result = -magnitude if negative else magnitude
                else:
                    result = getattr(g, name)(*aa)
            with g.context(round=g.RoundToNearest):
                endpoints.append(bits(float(result)))
        if endpoints[0] == endpoints[1]:
            result = endpoints[0]
            return None if result & 0x7ff0000000000000 == 0x7ff0000000000000 else result
        precision *= 2


def generate():
    lines = ['// Generated by UPDATE=1 python std/test/float_oracle.py.',
             '// MPFR ' + g.mpfr_version() + ', directed intervals, initially 4096 bits.',
             '// F1–F9 require these real-function bits, including gradual underflow.',
             '(i/core, std/float)', '']
    checked = {'tan', 'asin', 'acos', 'hypot', 'degrees'}
    for index, (name, args) in enumerate(cases()):
        expected = oracle(name, args)
        call = 'float.' + name + '(' + ', '.join(
            f'core.float_from_bits(0x{bits(x):016x})' for x in args) + ')'
        lines += ['@test', f'mpfr_{index:03}_{name} = () {{']
        if name in checked:
            if expected is None:
                lines += [f' assert({call} == Error(Nil))']
            else:
                lines += [f' Ok(result) = {call}',
                          f' assert(core.float_to_bits(result) == 0x{expected:016x})']
        else:
            assert expected is not None
            lines += [f' assert(core.float_to_bits({call}) == 0x{expected:016x})']
        lines += ['}', '']
    output = '\n'.join(lines)
    path = Path(__file__).with_name('float_mpfr_test.i')
    root = Path(__file__).resolve().parents[2]
    temporary = root / 'target/std3-oracle.i'
    temporary.parent.mkdir(parents=True, exist_ok=True)
    temporary.write_text(output)
    subprocess.run([str(root / 'target/release/i'), 'fmt', str(temporary)], check=True)
    formatted = temporary.read_text()
    if os.environ.get('UPDATE') == '1':
        path.write_text(formatted)
    else:
        assert path.read_text() == formatted, 'oracle fixture differs; regenerate with UPDATE=1'
    print('331 directed MPFR certificates; MPFR', g.mpfr_version())


if __name__ == '__main__':
    generate()
