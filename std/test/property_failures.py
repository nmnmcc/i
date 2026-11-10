#!/usr/bin/env python3
"""Check the ordinary failing-test boundary and three nonfinite stops.

direnv exec /home/agent/Programming/i python3 std/test/property_failures.py

The transient fixture is removed in finally. These expected failures are never
part of the passing std suite. F2 requires stops before a library call; P9
requires one ordinary failed test and a final report sent to the caller.
"""
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
fixture = ROOT / 'std/test/std3_failure_probe_test.i'
source = '''(i/core, std/property, std/float)
@test
adapter = () {
 property.test(property.constant(1), property.Config(42,1,0,0,0), (x) { assert(x == 0) }, core.inspect)
}
@test
positive_infinity = () { float.sin(core.float_from_bits(0x7ff0000000000000)) Nil }
@test
negative_infinity = () { float.sin(core.float_from_bits(0xfff0000000000000)) Nil }
@test
nan = () { float.sin(core.float_from_bits(0x7ff8000000000000)) Nil }
'''
assert not fixture.exists()
try:
    fixture.write_text(source)
    for opt in [0, 2]:
        result = subprocess.run([str(ROOT / 'target/release/i'), 'test', '--project', 'std',
                                 'std3_failure_probe_test', '--opt', str(opt)],
                                cwd=ROOT, text=True, capture_output=True)
        output = result.stdout + result.stderr
        print(output)
        assert result.returncode == 1
        assert '0 passed, 4 failed' in output
        assert output.count('std/property v1: falsified') == 1
        assert 'seed: 42' in output and 'original: 1' in output and 'smallest: 1' in output
        assert output.count('float: result is not finite') == 3
        assert 'std/float' not in output, 'nonfinite input reached the library'
finally:
    fixture.unlink(missing_ok=True)
