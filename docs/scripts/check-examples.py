#!/usr/bin/env python3
"""Check normal and Lisp examples with the repository's snippet parser and registry."""
import importlib.machinery
import importlib.util
import json
import os
from paths import ROOT, SPEC_PATH
import subprocess
import sys
from example_fences import fences, generated, i_info
from pathlib import Path

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
    for block in fences(text):
        try:
            info = i_info(block.info)
        except ValueError as error:
            # Keep parsing legacy metadata in generated standard snippets.
            # The runner validates the info strings of authored pages.
            if not generated(file.relative_to(ROOT / 'docs/content')):
                raise SystemExit(f'{file.relative_to(ROOT)}:{block.line}: {error}')
            info = ('fragment', None, frozenset())
        if info is None:
            continue
        kind, filename, codes = info
        # toml/text file= fences are project inputs, not source. scheme file=
        # fences name .is modules and use the parser driver's Lisp surface.
        if filename and Path(filename).suffix not in ('.i', '.is'):
            continue
        source = block.source
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
        line = block.line
        nearby = '\n'.join(text.splitlines()[max(0, line - 5):line + source.count('\n') + 5])
        items.append((file, line, entry, code, nearby, filename and filename.endswith('.is'), codes))
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
if os.environ.get('I_SNIPPET_PARSER'):
    if not checker.parser_driver().is_file():
        raise SystemExit('I_SNIPPET_PARSER must name an existing parser driver')
else:
    subprocess.run(command, cwd=ROOT, check=True)
results = {}
for lisp in (False, True):
    group = [(index, item[3]) for index, item in enumerate(items) if bool(item[5]) == lisp]
    if not group:
        continue
    actuals = checker.parse([source for _, source in group], lisp=lisp)
    results.update((index, actual) for (index, _), actual in zip(group, actuals))
failed = False
for index, (file, line, entry, code, nearby, lisp, codes) in enumerate(items):
    actual = results[index]
    # An error example can deliberately fail during parsing. The executable
    # runner checks the complete code set; here require only that every parser
    # error is among the declared compiler errors.
    if codes and not entry:
        unexpected = {diagnostic.split(':', 1)[0] for diagnostic in actual} - codes
        if unexpected:
            print(f'{file.relative_to(ROOT)}:{line}: expected parser codes within {sorted(codes)!r}, got {actual!r}', file=sys.stderr)
            failed = True
        continue
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
