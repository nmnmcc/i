#!/usr/bin/env python3
"""Check that built web fonts are local, allowlisted DM Sans and DM Mono assets."""
import argparse
import hashlib
from html.parser import HTMLParser
from pathlib import Path
import re
import sys
from urllib.parse import unquote, urlsplit
from paths import ROOT

FONT_FILES = (
    '@fontsource-variable/dm-sans/files/dm-sans-latin-wght-normal.woff2',
    '@fontsource-variable/dm-sans/files/dm-sans-latin-wght-italic.woff2',
    '@fontsource/dm-mono/files/dm-mono-latin-400-normal.woff2',
    '@fontsource/dm-mono/files/dm-mono-latin-500-normal.woff2',
)
FONT_EXTENSIONS = {'.woff', '.woff2', '.ttf', '.otf', '.eot', '.ttc'}
URL = re.compile(r'url\(\s*["\']?([^\s)"\']+)["\']?\s*\)', re.I)
FACE = re.compile(r'@font-face\s*\{([^}]+)\}', re.I)
IMPORT = re.compile(r'@import\s+(?:url\(\s*)?["\']?([^\s;"\')]+)', re.I)


def font_hashes():
    return {hashlib.sha256((ROOT / 'docs/node_modules' / file).read_bytes()).hexdigest(): file for file in FONT_FILES}


def remote(value, site):
    url = urlsplit(value)
    return bool(url.netloc and url.netloc != urlsplit(site).netloc)


def font_url(value):
    return Path(unquote(urlsplit(value).path)).suffix.lower() in FONT_EXTENSIONS


def css_errors(text, site):
    errors = []
    for face in FACE.findall(text):
        family = re.search(r'font-family\s*:\s*([^;]+)', face, re.I)
        name = family[1].strip().strip('"\'') if family else ''
        if name not in ('DM Sans', 'DM Mono'):
            errors.append('web-font family is not DM Sans or DM Mono: ' + repr(name))
        if not re.search(r'font-display\s*:\s*swap\b', face, re.I):
            errors.append('web font must use font-display: swap')
        for value in URL.findall(face):
            if remote(value, site) or urlsplit(value).scheme not in ('', 'http', 'https'):
                errors.append('nonlocal font URL: ' + value)
    for value in URL.findall(text):
        if font_url(value) and remote(value, site):
            errors.append('external font URL: ' + value)
    for value in IMPORT.findall(text):
        if remote(value, site):
            errors.append('external stylesheet import: ' + value)
    return errors


class PageFonts(HTMLParser):
    def __init__(self, site):
        super().__init__()
        self.site, self.errors = site, []
        self.style = False

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'style':
            self.style = True
        if tag == 'link' and 'stylesheet' in attrs.get('rel', '').split() and remote(attrs.get('href', ''), self.site):
            self.errors.append('external stylesheet: ' + attrs['href'])
        if tag == 'link' and attrs.get('as', '').lower() == 'font' and remote(attrs.get('href', ''), self.site):
            self.errors.append('external font preload: ' + attrs['href'])
        for key in ('href', 'src'):
            value = attrs.get(key, '')
            if font_url(value) and remote(value, self.site):
                self.errors.append('external font URL: ' + value)
        if 'style' in attrs:
            self.errors.extend(css_errors(attrs['style'], self.site))

    def handle_endtag(self, tag):
        if tag == 'style':
            self.style = False

    def handle_data(self, data):
        if self.style:
            self.errors.extend(css_errors(data, self.site))


def audit(dist, site='https://i.nmnm.cc'):
    allowed = font_hashes()
    errors, fonts, found = [], [], set()
    for file in sorted(dist.rglob('*')):
        if not file.is_file():
            continue
        if file.suffix.lower() in FONT_EXTENSIONS:
            fonts.append(file)
            digest = hashlib.sha256(file.read_bytes()).hexdigest()
            if digest not in allowed:
                errors.append(f'{file}: web-font bytes are not an allowlisted DM font')
            else:
                found.add(digest)
        elif file.suffix == '.css':
            errors.extend(f'{file}: {error}' for error in css_errors(file.read_text(), site))
        elif file.suffix == '.html':
            parser = PageFonts(site)
            parser.feed(file.read_text())
            parser.close()
            errors.extend(f'{file}: {error}' for error in parser.errors)
    for digest, name in allowed.items():
        if digest not in found:
            errors.append('missing built font: ' + name)
    return errors, len(fonts), sum(file.stat().st_size for file in fonts)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dist', type=Path, default=ROOT / 'docs/dist')
    parser.add_argument('--site', default='https://i.nmnm.cc')
    args = parser.parse_args()
    if not list(args.dist.rglob('*.html')):
        parser.error('no HTML pages found; build the site first')
    errors, count, size = audit(args.dist, args.site)
    for error in errors:
        print(error, file=sys.stderr)
    print(f'Web fonts checked: {count} DM files, {size} bytes; {len(errors)} violations.')
    return bool(errors)


if __name__ == '__main__':
    sys.exit(main())
