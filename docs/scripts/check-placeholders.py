#!/usr/bin/env python3
"""Reject unfinished visible text in every built HTML page, including generated pages."""
import argparse
from bisect import bisect_right
from html.parser import HTMLParser
from pathlib import Path
import re
import sys

PLACEHOLDER = re.compile(r'(?i:to\s+be\s+written)|\b(?:TODO|TBD)\b')
IGNORED = {'head', 'script', 'style', 'template'}
BLOCKS = {'address', 'article', 'aside', 'blockquote', 'br', 'dd', 'div', 'dl', 'dt',
          'figcaption', 'figure', 'footer', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
          'header', 'hr', 'li', 'main', 'nav', 'ol', 'p', 'pre', 'section', 'table',
          'td', 'th', 'tr', 'ul'}


class VisibleText(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.ignored = []
        self.parts = []
        self.offsets = []
        self.lines = []
        self.length = 0

    def append(self, text):
        if text:
            self.offsets.append(self.length)
            self.lines.append(self.getpos()[0])
            self.parts.append(text)
            self.length += len(text)

    def handle_starttag(self, tag, attrs):
        if tag in IGNORED:
            self.ignored.append(tag)
        if not self.ignored and tag in BLOCKS:
            self.append('\n')

    def handle_endtag(self, tag):
        if self.ignored:
            if tag == self.ignored[-1]:
                self.ignored.pop()
        elif tag in BLOCKS:
            self.append('\n')

    def handle_data(self, data):
        if not self.ignored:
            self.append(data)

    def findings(self):
        text = ''.join(self.parts)
        for match in PLACEHOLDER.finditer(text):
            index = bisect_right(self.offsets, match.start()) - 1
            local = match.start() - self.offsets[index]
            line = self.lines[index] + self.parts[index][:local].count('\n')
            yield line, match.group(), ' '.join(text[max(0, match.start()-35):match.end()+35].split())


def check_file(file):
    parser = VisibleText()
    parser.feed(file.read_text())
    parser.close()
    return list(parser.findings())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dist', type=Path, default=Path(__file__).resolve().parents[1] / 'dist')
    args = parser.parse_args()
    pages = sorted(args.dist.rglob('*.html'))
    if not pages:
        parser.error('no HTML files found; build the site before checking placeholders')
    count = 0
    for page in pages:
        for line, word, context in check_file(page):
            print(f'{page}:{line}: unfinished visible text {word!r}: {context!r}', file=sys.stderr)
            count += 1
    print(f'Visible-text placeholders checked in {len(pages)} HTML files; {count} matches.')
    return bool(count)


if __name__ == '__main__':
    sys.exit(main())
