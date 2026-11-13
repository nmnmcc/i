#!/usr/bin/env python3
"""Make a page per library module from the source and, if available, checked types."""
from pathlib import Path
import json
import os
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'docs/content/docs/library/generated'
BINARY = ROOT / 'target/release/i'
HEAVY = ROOT.parent / 'cache/heavy'
types = {}
if BINARY.is_file() and os.access(BINARY, os.X_OK):
    cmd = [str(BINARY), 'check', '--project', 'std', '--types', '--offline']
    if HEAVY.is_file():
        cmd = [str(HEAVY)] + cmd
    result = subprocess.run(cmd, cwd=ROOT, text=True, capture_output=True)
    if result.returncode:
        print('i check --types failed; using source text instead:\n' + result.stderr, file=sys.stderr)
    else:
        for line in result.stdout.splitlines():
            m = re.fullmatch(r'([\w/]+)\.([\w]+) : (.*)', line)
            if m:
                types[(m[1], m[2])] = m[3]

def groups(text):
    # The initial groups are imports (containing a module path) and exports.
    rest = re.sub(r'\A(?:\s|//[^\n]*\n)*', '', text)
    for _ in range(2):
        if not rest.startswith('('):
            return
        depth = 0
        for pos, ch in enumerate(rest):
            depth += (ch == '(') - (ch == ')')
            if depth == 0:
                break
        else:
            return
        group, rest = rest[1:pos], rest[pos + 1:].lstrip()
        if rest.startswith('='):
            return
        if '/' not in group:
            yield re.findall(r'\b([A-Za-z_]\w*)(?:\(\.\.\))?', re.sub(r'//[^\n]*', '', group))
            return

def safe(text):
    # MDX text can contain JSX metacharacters in source comments.
    output = []
    fenced = False
    for line in text.split('\n'):
        if line.startswith('```'):
            fenced = not fenced
        elif not fenced:
            chunks = re.split(r'(`+[^`]*`+)', line)
            for index in range(0, len(chunks), 2):
                chunks[index] = chunks[index].replace('&', '&amp;').replace('<', '&lt;').replace('{', '&#123;').replace('}', '&#125;')
            line = ''.join(chunks)
        output.append(line)
    return '\n'.join(output)

OUT.mkdir(parents=True, exist_ok=True)
for old in OUT.glob('*.mdx'):
    old.unlink()
pages = []
for prefix, directory in [('i', ROOT / 'lib/i'), ('std', ROOT / 'std/src')]:
    for file in sorted(directory.glob('*.i')):
        module = prefix + '/' + file.stem
        source = file.read_text()
        slug = prefix + '-' + file.stem.replace('_', '-')
        pages.append(slug)
        opening = []
        for line in source.splitlines():
            if line.startswith('////'):
                opening.append(line[4:].lstrip())
            elif not line.strip() and not opening:
                continue
            else:
                break
        output = ['---', 'title: ' + json.dumps(module), '---', '', safe('\n'.join(opening)) if opening else 'to be written', '']
        exports = next(groups(source), [])
        for name in exports:
            # Attributes may stand between the documentation and the item.
            pattern = r'(?m)^' + re.escape(name) + r'(?=\s*(?:[:=(]|$))[^\n]*'
            match = re.search(pattern, source)
            output += ['## ' + name, '']
            if not match:
                output += ['Source declaration not found.', '']
                continue
            before = source[:match.start()].splitlines()
            while before and before[-1].lstrip().startswith('@'):
                before.pop()
            comments = []
            while before and before[-1].startswith('///') and not before[-1].startswith('////'):
                comments.append(before.pop()[3:].lstrip())
            if comments:
                output += [safe('\n'.join(reversed(comments))), '']
            signature = types.get((module, name))
            if signature:
                output += ['```text', name + ': ' + signature, '```', '']
            else:
                # Retain the written source line if inference is unavailable.
                output += ['```text', match.group(), '```', '']
        (OUT / (slug + '.mdx')).write_text('\n'.join(output))
(OUT / 'meta.json').write_text(json.dumps({'title':'Modules','pages':pages}, indent=2) + '\n')
