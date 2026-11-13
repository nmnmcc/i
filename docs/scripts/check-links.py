#!/usr/bin/env python3
"""Check local links and fragments in the built static site."""
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urljoin, urlsplit
import sys

DIST = Path(__file__).resolve().parents[1] / 'dist'
class Page(HTMLParser):
    def __init__(self):
        super().__init__()
        self.ids = set()
        self.links = []
    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if 'id' in attrs:
            self.ids.add(attrs['id'])
        if tag == 'a' and 'href' in attrs:
            self.links.append(attrs['href'])

pages = {}
for file in DIST.rglob('*.html'):
    page = Page()
    page.feed(file.read_text())
    pages[file] = page
if not pages:
    sys.exit('Build the site before checking links.')
errors = []
for file, page in pages.items():
    relative = file.relative_to(DIST)
    current = '/' + str(relative.parent) + '/' if relative.name == 'index.html' else '/' + str(relative)
    for href in page.links:
        url = urlsplit(urljoin(current, href))
        if url.scheme or url.netloc:
            continue
        path = unquote(url.path).lstrip('/')
        target = DIST / path
        if target.is_dir():
            target = target / 'index.html'
        if not target.exists():
            errors.append(f'{relative}: {href}: target does not exist')
        elif url.fragment and target in pages and unquote(url.fragment) not in pages[target].ids:
            errors.append(f'{relative}: {href}: fragment does not exist')
if errors:
    print('\n'.join(errors), file=sys.stderr)
    sys.exit(1)
print(f'Local links and fragments checked in {len(pages)} pages.')
