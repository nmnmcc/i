"""Identify specification targets and measure reader-facing built references."""
from html.parser import HTMLParser
from pathlib import PurePosixPath
from urllib.parse import unquote, urljoin, urlsplit
import re
from page_prose import prose_words, example_count, budget, source_for_page
from example_fences import fences, generated, i_info, i_options, output_info


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
        self.hash_links = []

    def handle_starttag(self, tag, attrs):
        if dict(attrs).get('href', '').startswith('#'):
            self.hash_links.append(dict(attrs)['href'])
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


def checked_diagnostics(page):
    """Only stderr=exact transcripts are compared with the real compiler in CI."""
    if page is None or not page.is_file() or generated(page):
        return set()
    blocks = list(fences(page.read_text()))
    outputs = set()
    for block, following in zip(blocks, blocks[1:]):
        info = i_info(block.info)
        if (info and info[0] == 'error' and i_options(block.info)[1].get('stderr') == 'exact'
                and following.closed and output_info(following.info)):
            outputs.add(following.source.removesuffix('\n'))
    return outputs


class DiagnosticSections(HTMLParser):
    """Recognize whole rendered pre blocks, without exempting surrounding prose."""
    def __init__(self, outputs):
        super().__init__(convert_charrefs=True)
        self.outputs = outputs
        self.ignored = []
        self.pre = None
        self.sections = 0

    def handle_starttag(self, tag, attrs):
        if tag in ('head', 'script', 'style', 'template'):
            self.ignored.append(tag)
        elif tag == 'pre' and not self.ignored:
            self.pre = []

    def handle_endtag(self, tag):
        if self.ignored:
            if tag == self.ignored[-1]:
                self.ignored.pop()
        elif tag == 'pre' and self.pre is not None:
            text = ''.join(self.pre)
            if text.removesuffix('\n') in self.outputs:
                self.sections += text.count('§')
            self.pre = None

    def handle_data(self, data):
        if self.pre is not None and not self.ignored:
            self.pre.append(data)


def area(path):
    if str(path) == 'index.html':
        return 'home'
    return path.parts[1] if len(path.parts) > 2 and path.parts[0] == 'docs' else 'other'


def measure(file, dist, content=None, enforced_areas=()):
    path = file.relative_to(dist)
    html = file.read_text()
    parser = References('/' + str(path))
    parser.feed(html)
    parser.close()
    diagnostic = DiagnosticSections(checked_diagnostics(source_for_page(path, content)))
    diagnostic.feed(html)
    diagnostic.close()
    raw_sections = parser.sections
    parser.sections -= diagnostic.sections
    group = area(path)
    total = parser.sections + len(parser.links)
    limit = {'home': 0, 'tour': 1, 'getting-started': 1, 'guides': 2}.get(group)
    reference_failed = total > limit if limit is not None else group == 'library' and bool(parser.links)
    words, examples = prose_words(html), example_count(path, content)
    maximum, minimum_examples, maximum_examples = budget(path, group)
    issues = []
    if maximum is not None and words > maximum:
        issues.append(f'{words} prose words exceeds {maximum}')
    if minimum_examples is not None and examples < minimum_examples:
        issues.append(f'{examples} runnable examples is below {minimum_examples}')
    if maximum_examples is not None and examples > maximum_examples:
        issues.append(f'{examples} runnable examples exceeds {maximum_examples}')
    enforced = group == 'home' or group in enforced_areas
    failures = []
    if reference_failed:
        failures.append(f'{parser.sections} section marks and {len(parser.links)} specification links exceed the reference limit')
    if group == 'home' and parser.hash_links:
        failures.append(f'{len(parser.hash_links)} in-page href="#" links')
    if enforced:
        failures.extend(issues)
    return dict(page=str(path), area=group, sections=parser.sections, links=len(parser.links),
                raw_sections=raw_sections, diagnostic_sections=diagnostic.sections,
                total=total, limit=limit, failed=bool(failures), targets=parser.links,
                words=words, examples=examples, hash_links=parser.hash_links,
                failures=failures, warnings=[] if enforced else issues)
