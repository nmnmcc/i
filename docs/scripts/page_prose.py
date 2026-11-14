"""Count authored page prose and complete checked example fences."""
from html.parser import HTMLParser
from pathlib import Path
import re
from example_fences import fences, i_info

VOID = {'area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input', 'link', 'meta', 'param', 'source', 'track', 'wbr'}
BLOCK = {'p', 'div', 'article', 'section', 'main', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'li', 'td', 'th', 'br', 'pre', 'figure'}


class Prose(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.stack = []
        self.all_text = []
        self.page_text = []
        self.has_scope = False

    def boundary(self):
        self.all_text.append('\n')
        self.page_text.append('\n')

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        classes = attrs.get('class', '').split()
        parent_skip, parent_scope = self.stack[-1][1:] if self.stack else (False, False)
        scope = parent_scope or 'home-page' in classes or attrs.get('id') in ('nd-page', 'nd-home-layout') or 'data-page-content' in attrs
        self.has_scope |= scope
        skip = (parent_skip or tag in ('head', 'script', 'style', 'template', 'pre', 'button', 'nav')
                or 'hidden' in attrs or attrs.get('aria-hidden') == 'true' or 'sr-only' in classes
                or 'data-page-chrome' in attrs or (tag == 'figure' and 'shiki' in classes))
        if tag in BLOCK:
            self.boundary()
        if tag not in VOID:
            self.stack.append((tag, skip, scope))

    def handle_startendtag(self, tag, attrs):
        self.handle_starttag(tag, attrs)
        if tag not in VOID:
            self.handle_endtag(tag)

    def handle_endtag(self, tag):
        if tag in BLOCK:
            self.boundary()
        for index in range(len(self.stack)-1, -1, -1):
            if self.stack[index][0] == tag:
                del self.stack[index:]
                break

    def handle_data(self, data):
        skip, scope = self.stack[-1][1:] if self.stack else (False, False)
        if not skip:
            self.all_text.append(data)
            if scope:
                self.page_text.append(data)

    def words(self):
        text = ''.join(self.page_text if self.has_scope else self.all_text)
        return len(re.findall(r"\b\w+(?:[’'-]\w+)*\b", text))


def prose_words(html):
    parser = Prose()
    parser.feed(html)
    parser.close()
    return parser.words()


def source_for_page(path, content):
    if content is None:
        return None
    if str(path) == 'index.html':
        return content / 'home/index.mdx'
    stem = content / path.parent
    for candidate in (stem.with_suffix('.mdx'), stem / 'index.mdx', stem.with_suffix('.md'), stem / 'index.md'):
        if candidate.is_file():
            return candidate
    return None


def example_count(path, content):
    if 'generated' in path.parts:
        return 0  # Generated spec fragments include registered future syntax, not executable fences.
    source = source_for_page(path, content)
    if source is None or not source.is_file():
        return 0
    # Count each complete compiler action once, not one per optimization level.
    kinds = {'run', 'test', 'test-fails', 'check', 'error', 'types', 'format', 'expand'}
    return sum(bool(info and info[0] in kinds) for block in fences(source.read_text())
               if (info := i_info(block.info)) is not None)


def budget(path, area):
    if area == 'home':
        return 220, None, None
    if area == 'tour':
        return 450, 6, None
    if area == 'getting-started':
        return 300, None, None
    if area in ('guides', 'contributing'):
        return 500, None, None
    if str(path) in ('docs/library/index.html', 'docs/standard/index.html'):
        return 250, 3, 5
    return None, None, None
