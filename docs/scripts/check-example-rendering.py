#!/usr/bin/env python3
"""Verify example fence titles, highlighted source, and hidden metadata in dist."""
from html.parser import HTMLParser
from pathlib import Path
import sys

from example_fences import fences, generated, i_info, i_options
from paths import ROOT


class CodeBlocks(HTMLParser):
    def __init__(self):
        super().__init__()
        self.blocks = []
        self.current = None
        self.caption = False
        self.code = False

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'figure' and 'shiki' in attrs.get('class', '').split():
            self.current = {'title': '', 'code': '', 'highlighted': False}
        elif self.current is not None:
            if tag == 'figcaption':
                self.caption = True
            elif tag == 'pre':
                self.code = True
            elif tag == 'span' and '--shiki-light:' in attrs.get('style', ''):
                self.current['highlighted'] = True

    def handle_endtag(self, tag):
        if self.current is None:
            return
        if tag == 'figcaption':
            self.caption = False
        elif tag == 'pre':
            self.code = False
        elif tag == 'figure':
            self.blocks.append(self.current)
            self.current = None

    def handle_data(self, data):
        if self.current is not None:
            if self.caption:
                self.current['title'] += data
            if self.code:
                self.current['code'] += data


def main():
    content = ROOT / 'docs/content'
    count = 0
    failures = []
    for page in sorted(content.rglob('*')):
        if page.suffix not in ('.md', '.mdx') or generated(page.relative_to(content)):
            continue
        blocks = [block for block in fences(page.read_text()) if i_info(block.info)]
        if not blocks:
            continue
        slug = page.relative_to(content).with_suffix('')
        if slug.name == 'index':
            slug = slug.parent
        html = ROOT / 'docs/dist' / slug / 'index.html'
        if not html.is_file():
            failures.append(f'{page.relative_to(ROOT)}:1: missing built page {html}')
            continue
        parsed = CodeBlocks()
        parsed.feed(html.read_text())
        for block in blocks:
            title = i_options(block.info)[1].get('title')
            matches = [rendered for rendered in parsed.blocks
                       if rendered['title'] == title and title
                       and rendered['code'].rstrip('\n') == block.source.rstrip('\n')
                       and (rendered['highlighted'] or block.info.split()[0] == 'text')]
            if not matches:
                failures.append(f'{page.relative_to(ROOT)}:{block.line}: expected source with title={title!r}, appropriate highlighting, and no metadata in its code')
            count += 1
    for failure in failures:
        print(failure, file=sys.stderr)
    print(f'{count} authored example fences checked in dist; {len(failures)} rendering failures.')
    return bool(failures)


if __name__ == '__main__':
    sys.exit(main())
