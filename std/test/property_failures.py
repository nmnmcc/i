#!/usr/bin/env python3
"""Check the ordinary failing-test boundary.

direnv exec /home/agent/Programming/i python3 std/test/property_failures.py

The transient fixture is removed in finally. These expected failures are never
part of the passing std suite. P9 requires one ordinary failed test and a final report sent to the caller.
"""
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
fixture = ROOT / 'std/test/std3_failure_probe_test.i'
source = '''(i/core, std/property)
@test
adapter = () {
 property.test(property.constant(1), property.Config(42,1,0,0,0), (x) { assert(x == 0) }, core.inspect)
}
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
        assert '0 passed, 1 failed' in output
        assert output.count('std/property v1: falsified') == 1
        assert 'seed: 42' in output and 'original: 1' in output and 'smallest: 1' in output
finally:
    fixture.unlink(missing_ok=True)
