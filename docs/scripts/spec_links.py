"""Identify specification targets and measure reader-facing built references."""
from html.parser import HTMLParser
from pathlib import PurePosixPath
from urllib.parse import unquote, urljoin, urlsplit
import re


def is_spec_target(href, page='/'):
    url = urlsplit(urljoin(page, href))
    if url.scheme not in ('', 'http', 'https'):
        return False
    path = unquote(url.path).rstrip('/')
    return path == '/docs/standard' or path.startswith('/docs/standard/') or PurePosixPath(path).name == 'spec.md'


def unlink_spec_references(text):
    # Source comments may contain Markdown or HTML links. Keep their labels,
    # including literal section marks, without making them navigation links.
    text = re.sub(r'\[([^\]]+)\]\(([^\s)]+)(?:\s+"[^"]*")?\)',
                  lambda m: m[1] if is_spec_target(m[2]) else m[0], text)
    text = re.sub(r'<a\b[^>]*href=["\']([^"\']+)["\'][^>]*>(.*?)</a>',
                  lambda m: m[2] if is_spec_target(m[1]) else m[0], text, flags=re.I | re.S)
    # Code spans prevent automatic linking of bare or angle-bracket URLs.
    text = re.sub(r'(?<![`\w])<?https?://[^\s<>`]+>?',
                  lambda m: '`' + m[0].strip('<>') + '`' if is_spec_target(m[0].strip('<>')) else m[0], text)
    return text


class References(HTMLParser):
    def __init__(self, page='/'):
        super().__init__(convert_charrefs=True)
        self.page = page
        self.ignored = []
        self.sections = 0
        self.links = []

    def handle_starttag(self, tag, attrs):
        if tag in ('head', 'script', 'style', 'template'):
            self.ignored.append(tag)
        if not self.ignored and tag == 'a':
            href = dict(attrs).get('href', '')
            if is_spec_target(href, self.page):
                self.links.append(href)

    def handle_endtag(self, tag):
        if self.ignored and tag == self.ignored[-1]:
            self.ignored.pop()

    def handle_data(self, data):
        if not self.ignored:
            self.sections += data.count('§')


def area(path):
    if str(path) == 'index.html':
        return 'home'
    return path.parts[1] if len(path.parts) > 2 and path.parts[0] == 'docs' else 'other'


def measure(file, dist):
    path = file.relative_to(dist)
    parser = References('/' + str(path))
    parser.feed(file.read_text())
    parser.close()
    group = area(path)
    total = parser.sections + len(parser.links)
    limit = {'home': 1, 'tour': 1, 'getting-started': 1, 'guides': 2}.get(group)
    failed = total > limit if limit is not None else group == 'library' and bool(parser.links)
    return dict(page=str(path), area=group, sections=parser.sections, links=len(parser.links),
                total=total, limit=limit, failed=failed, targets=parser.links)
