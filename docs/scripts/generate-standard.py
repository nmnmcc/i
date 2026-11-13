#!/usr/bin/env python3
"""Split the standard into searchable pages. SPEC_PATH is the only relocation point."""
from pathlib import Path
import json
import re

ROOT = Path(__file__).resolve().parents[2]
SPEC_PATH = ROOT / 'docs/spec.md'
OUT = ROOT / 'docs/content/docs/standard/generated'
text = SPEC_PATH.read_text().replace('(design/runtime.md)', '(https://github.com/nmnmcc/i/blob/main/docs/design/runtime.md)')
parts = re.split(r'(?m)^(## (?:\d+\.|Appendix [A-Z]\.).*)$', text)
OUT.mkdir(parents=True, exist_ok=True)
for old in OUT.glob('*.mdx'):
    old.unlink()
pages = []
for heading, body in zip(parts[1::2], parts[2::2]):
    title = heading[3:]
    number = re.match(r'(\d+|Appendix [A-Z])\.', title)[1]
    slug = 'chapter-' + number.lower().replace(' ', '-')
    pages.append(slug)
    # MDX treats literal braces and angle brackets as JSX. Escape them only
    # outside fenced and inline code, preserving the standard's prose.
    lines = []
    fenced = False
    for line in body.splitlines():
        if re.match(r'^\s*```', line):
            fenced = not fenced
        elif not fenced:
            chunks = re.split(r'(`+[^`]*`+)', line)
            for i in range(0, len(chunks), 2):
                chunks[i] = chunks[i].replace('{', '&#123;').replace('}', '&#125;').replace('<', '&lt;')
            line = ''.join(chunks)
            # Explicit IDs remain stable when a section title changes.
            m = re.match(r'^(#{2,6})\s+((?:\d+|[A-Z])(?:\.\d+)+)\b', line)
            if m:
                line = '<a id="section-' + m[2].lower().replace('.', '-') + '"></a>\n' + line
        lines.append(line)
    (OUT / (slug + '.mdx')).write_text('---\ntitle: ' + json.dumps(title) + '\n---\n\n' + '\n'.join(lines) + '\n')
(OUT / 'meta.json').write_text(json.dumps({'title':'Chapters','pages':pages}, indent=2) + '\n')
