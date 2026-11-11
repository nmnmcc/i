#!/usr/bin/env python3
from pathlib import Path

source = ['(testlib = (test, each) = std/test)', '']
for group in range(100):
    cases = ', '.join(f'("case {case}", {case})' for case in range(20))
    source.append(f'each("group {group}", [{cases}], (n) {{ assert(n >= 0) }})')
Path(__file__).with_name('test').joinpath('many_test.i').write_text('\n'.join(source) + '\n')
