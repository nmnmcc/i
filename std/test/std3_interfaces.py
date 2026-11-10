#!/usr/bin/env python3
"""Compare all 35 landable public bindings with the corrected accepted inventory.

direnv exec /home/agent/Programming/i python3 std/test/std3_interfaces.py

Expected signatures come from the note, independently of implementation bodies.
"""
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
note = (ROOT / 'docs/design/std-3.md').read_text()
result = subprocess.run([str(ROOT / 'target/release/i'), 'check', '--project', 'std', '--types'],
                        cwd=ROOT, text=True, capture_output=True, check=True)
actual = dict(re.findall(r'^([^ ]+) : (.+)$', result.stdout, re.M))
expected = {}
for section, module in [('3.2', 'property'), ('5.2', 'string')]:
    part = note.split('### ' + section + ' ', 1)[1].split('\n### ', 1)[0]
    part = part.split('```text\n', 1)[1].split('```', 1)[0]
    for name, signature in re.findall(r'^([a-z_][a-z_0-9]*)(?:\([A-Z][A-Za-z, ]*\))?: (.+)$', part, re.M):
        expected[f'std/{module}.{name}'] = signature
assert len(expected) == 35, len(expected)
for name, signature in expected.items():
    assert actual[name] == signature, (name, signature, actual[name])
    print(name + ' : ' + signature)
print('35/35 landable public signatures match the corrected note, including labels and effects')
