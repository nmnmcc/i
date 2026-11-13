#!/usr/bin/env python3
"""Index design notes without assigning unsupported status to a draft."""
from pathlib import Path
import json

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'docs/content/docs/contributing/generated'
OUT.mkdir(parents=True, exist_ok=True)
lines = ['---', 'title: Design note index', '---', '',
         'The language standard takes precedence. Status labels below identify',
         'the controlling syntax-three note and historical syntax notes; other',
         'notes need a status review.', '']
for file in sorted((ROOT / 'docs/design').glob('*.md')):
    if file.name == 'syntax-3.md':
        status = 'in force'
    elif file.name in ('syntax.md', 'syntax-2.md') or file.stem.endswith('-report'):
        status = 'historical'
    else:
        status = 'status to be reviewed'
    url = 'https://github.com/nmnmcc/i/blob/main/docs/design/' + file.name
    lines.append(f'- [{file.name}]({url}) — {status}')
(OUT / 'design-index.mdx').write_text('\n'.join(lines) + '\n')
(OUT / 'meta.json').write_text(json.dumps({'pages':['design-index']}, indent=2) + '\n')
