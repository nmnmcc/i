#!/usr/bin/env python3
"""Check fenced i examples with the repository's snippet parser and registry."""
import importlib.machinery
import importlib.util
import json
from paths import ROOT, SPEC_PATH
import re
import subprocess
import sys

loader = importlib.machinery.SourceFileLoader('repo_snippets', str(ROOT / 'scripts/check-snippets'))
spec = importlib.util.spec_from_loader(loader.name, loader)
checker = importlib.util.module_from_spec(spec)
loader.exec_module(checker)
config = json.loads(checker.DATA.read_text())
local_config = json.loads((ROOT / 'docs/scripts/examples.json').read_text())
used = set()
items = []
for file in sorted((ROOT / 'docs/content').rglob('*')):
    if file.suffix not in ('.md', '.mdx'):
        continue
    text = file.read_text()
    for match in re.finditer(r'(?m)^(?P<indent> {0,3})(?P<fence>`{3,})i(?:[ \t][^\n]*)?\n(?P<source>.*?)^(?P=indent)(?P=fence)[ \t]*$', text, re.S):
        source = match['source']
        path = str(file.relative_to(ROOT))
        if '/standard/generated/' in path:
            path = str(SPEC_PATH.relative_to(ROOT))
        key = checker.identity(path, source)
        entry = config.get(key, local_config.get(key, {}))
        if key in local_config:
            used.add(key)
        if entry.get('not_source'):
            raise SystemExit(f'{file}: an i fence cannot be registered as non-source')
        code = entry.get('prefix', '') + source + entry.get('suffix', '')
        for old, new in entry.get('substitutions', []):
            code = code.replace(old, new)
        items.append((file, text[:match.start()].count('\n') + 1, entry, code, text[max(0, match.start()-100):match.end()+100]))
stale = local_config.keys() - used
if stale:
    raise SystemExit('Stale documentation example registrations: ' + ', '.join(sorted(stale)))
if not items:
    print('No i examples found.')
    sys.exit(0)
command = ['cargo', 'build', '-p', 'ilang-syntax', '--example', 'parse-snippets', '--locked', '--quiet']
heavy = ROOT.parent / 'cache/heavy'
if heavy.is_file():
    command = [str(heavy), 'direnv', 'exec', str(ROOT.parent.parent)] + command
subprocess.run(command, cwd=ROOT, check=True)
failed = False
for (file, line, entry, code, nearby), actual in zip(items, checker.parse([i[3] for i in items])):
    expected = entry.get('diagnostics', [])
    if entry.get('track') and not expected:
        print(f'{file.relative_to(ROOT)}:{line}: future-track entry must expect an error', file=sys.stderr)
        failed = True
    if actual != expected:
        print(f'{file.relative_to(ROOT)}:{line}: expected {expected!r}, got {actual!r}', file=sys.stderr)
        failed = True
    if expected and not entry.get('track'):
        marker = 'old-syntax-check: ' + ','.join(d.split(':', 1)[0] for d in expected)
        if marker not in nearby:
            print(f'{file.relative_to(ROOT)}:{line}: missing {marker}', file=sys.stderr)
            failed = True
if failed:
    sys.exit(1)
print(f'{len(items)} documentation examples match their parser expectations.')
