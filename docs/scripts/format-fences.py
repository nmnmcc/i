#!/usr/bin/env python3
"""Check or fix only module-header blank lines in Markdown source fences.

Default scope: authored docs/content pages and docs/spec.md. Full-module i
and scheme file=*.is fences use i fmt as the header oracle. Bare i fragments
are included when they parse as modules. Generated pages are regenerated from
their sources. Other layout, code, prose, fence markers and indentation stay
unchanged. Formatter-output text fences follow the same rule.

--write also maps line-leading file:line:column locations in error/types transcripts
through the inserted/removed blank lines, keeping columns and messages.
Unknown locations stay unchanged; ambiguous local locations are refused.
Invalid fragments and declared syntax-error examples stay unchanged, just as
i fmt leaves them. Other syntax errors, malformed fences, inline header
content requiring nonblank edits, and ambiguous transcript edits are refused.
All pages are prepared before any write, so a refusal writes nothing.
"""
import argparse
import difflib
import importlib.util
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

from example_fences import fences, generated, i_info

ROOT = Path(__file__).resolve().parents[2]
LOCATION = re.compile(r'(?m)^(?P<prefix>[ \t]*)(?P<file>[\w./-]+\.is?):(?P<line>\d+):(?P<col>\d+)')
ANY_LOCATION = re.compile(r'(?P<file>[\w./-]+\.is?):\d+:\d+')
RUNTIME_LOCATION = re.compile(r'[\w./-]+:\d+:\d+')
CODE = re.compile(r'error\[(E\d+)\]')
SHORT_LOCATION = re.compile(r'(?<![\w./:-])\d+:\d+(?!\d)')


class Refusal(ValueError):
    pass


def skip(source, pos):
    while True:
        before = pos
        while pos < len(source) and source[pos].isspace():
            pos += 1
        if source.startswith('//', pos):
            end = source.find('\n', pos)
            pos = len(source) if end < 0 else end + 1
        if pos == before:
            return pos


def close(source, pos):
    if source[pos:pos + 1] != '(':
        raise Refusal('formatter header does not match source parentheses')
    depth = 0
    while pos < len(source):
        if source.startswith('//', pos):
            pos = source.find('\n', pos)
            if pos < 0:
                break
        elif source[pos] == '"':
            raise Refusal('literal in a purported header; cannot map it safely')
        elif source[pos] == '(':
            depth += 1
        elif source[pos] == ')':
            depth -= 1
            if depth == 0:
                return pos + 1
        pos += 1
    raise Refusal('unclosed header')


def header_count(pretty, lisp):
    count, pos = 0, skip(pretty, 0)
    while pos < len(pretty):
        begins = re.match(r'\(,(?=\s|\))', pretty[pos:]) if lisp else pretty.startswith('(\n', pos)
        if not begins:
            break
        end = close(pretty, pos)
        # A broken function/data pattern has =, ->, <- or { after its
        # parenthesis; the normal header printer closes on its own line.
        if end < len(pretty) and pretty[end] != '\n':
            break
        count += 1
        pos = skip(pretty, end)
    return count


def header_gaps(source, pretty, lisp):
    ends, pos = [], 0
    for _ in range(header_count(pretty, lisp)):
        pos = close(source, skip(source, pos))
        ends.append(pos)
    for end in reversed(ends):
        following = end
        while following < len(source) and source[following].isspace():
            following += 1
        gap = source[end:following]
        needed = 2 if following < len(source) else 1
        if gap.count('\n') == needed:
            continue
        newline = gap.find('\n')
        if newline < 0:
            raise Refusal('inline content after a header needs nonblank-line edits')
        # Keep the closing line's whitespace and the next content's indent.
        first_line = gap[:newline + 1]
        indent = gap.rsplit('\n', 1)[-1] if following < len(source) else ''
        replacement = first_line + ('\n' if needed == 2 else '') + indent
        source = source[:end] + replacement + source[following:]
    return source


def blank_only(old, new):
    return [s for s in old.splitlines(True) if s.strip()] == [
        s for s in new.splitlines(True) if s.strip()]


def raw_replacement(raw, old, new):
    """Transfer empty-line edits to the original, possibly indented fence."""
    a, b = old.splitlines(True), new.splitlines(True)
    result = list(raw)
    if len(result) != len(a) or not blank_only(old, new):
        raise Refusal('edit would change a nonblank source line')
    for tag, i, j, k, m in reversed(list(difflib.SequenceMatcher(
            a=a, b=b, autojunk=False).get_opcodes())):
        if tag == 'equal':
            continue
        if any(s.strip() for s in a[i:j] + b[k:m]):
            raise Refusal('cannot transfer the header edits without changing code')
        result[i:j] = b[k:m]
    return result


def line_map(old, new):
    a = [(n, s) for n, s in enumerate(old.splitlines(), 1) if s.strip()]
    b = [(n, s) for n, s in enumerate(new.splitlines(), 1) if s.strip()]
    if [s for _, s in a] != [s for _, s in b]:
        raise Refusal('cannot map locations across changed source lines')
    return {i: j for (i, _), (j, _) in zip(a, b)}


def load_runner():
    spec = importlib.util.spec_from_file_location('fence_runner', ROOT / 'docs/scripts/run-examples.py')
    runner = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = runner
    spec.loader.exec_module(runner)
    return runner


def compiler():
    configured = os.environ.get('I_BIN')
    found = shutil.which(configured) if configured else ROOT / 'target/release/i'
    if not found or not Path(found).is_file():
        raise Refusal('build the compiler first, or set I_BIN to an existing compiler')
    return str(Path(found).resolve())


def paths(arguments):
    selected = arguments or [ROOT / 'docs/content', ROOT / 'docs/spec.md']
    result = set()
    for path in selected:
        if not path.exists():
            raise Refusal(f'{path}: does not exist')
        candidates = path.rglob('*') if path.is_dir() else [path]
        result.update(p.resolve() for p in candidates
                      if p.suffix in ('.md', '.mdx') and not generated(p))
    return sorted(result)


def prepare(pages, binary):
    documents, requests, changes, positions = {}, [], [], []
    for page in pages:
        text = page.read_bytes().decode('utf-8')
        if '\r' in text:
            raise Refusal(f'{page}: only LF Markdown is supported')
        blocks = list(fences(text))
        documents[page] = (text.splitlines(True), blocks, {})
        for n, block in enumerate(blocks):
            try:
                info = i_info(block.info)
            except ValueError as error:
                if page == ROOT / 'docs/spec.md' and block.info.split()[:1] == ['i']:
                    info = ('fragment', None, frozenset())
                else:
                    raise Refusal(f'{page}:{block.line}: {error}') from error
            language = block.info.split()[:1]
            if info is None or language not in (['i'], ['scheme']):
                continue
            kind, filename, codes = info
            if not block.closed:
                raise Refusal(f'{page}:{block.line}: unclosed source fence')
            requests.append((page, n, block, language == ['scheme'], kind, codes))
            if kind == 'format':
                if n + 1 == len(blocks) or blocks[n + 1].info != 'text' or not blocks[n + 1].closed:
                    raise Refusal(f'{page}:{block.line}: format needs a closed text output fence')
                requests.append((page, n + 1, blocks[n + 1], False, 'format-output', frozenset()))
    with tempfile.TemporaryDirectory(prefix='format-fences-') as scratch:
        files = []
        for index, (_, _, block, lisp, _, _) in enumerate(requests):
            file = Path(scratch) / f'{index}.{"is" if lisp else "i"}'
            file.write_text(block.source)
            files.append(file)
        if files:
            result = subprocess.run([binary, 'fmt', scratch], capture_output=True, text=True)
            if result.returncode not in (0, 1):
                raise Refusal(f'i fmt failed: {result.stderr}')
        for request, file in zip(requests, files):
            page, n, block, lisp, kind, codes = request
            diagnostics = '\n'.join(s for s in result.stderr.splitlines() if s.startswith(str(file) + ':'))
            if diagnostics:
                actual = frozenset(CODE.findall(diagnostics))
                if kind == 'fragment' or kind == 'error' and actual and actual <= codes:
                    continue  # i fmt deliberately leaves this source untouched.
                raise Refusal(f'{page}:{block.line}: i fmt rejected source:\n{diagnostics}')
            new = header_gaps(block.source, file.read_text(), lisp)
            if new != block.source:
                documents[page][2][n] = new
                changes.append(f'{page}:{block.line}: header blank lines')
    runner = load_runner()
    for page, (_, blocks, edits) in documents.items():
        changed_fences = {blocks[n].line: new for n, new in edits.items()}
        # Bare spec fragments have no execution metadata and no transcripts.
        if not changed_fences or page == ROOT / 'docs/spec.md':
            continue
        examples, failures, _ = runner.read_page(page)
        if failures:
            raise Refusal('\n'.join(map(str, failures)))
        by_line = {block.line: n for n, block in enumerate(blocks)}
        for example in examples:
            testing = example.kind in ('test', 'test-fails')
            filename = example.filename or (f'test/{runner.page_name(page)}_test.i'
                                             if testing else 'src/main.i')
            sources = {**example.files, filename: example.source}
            origins = {**example.file_lines, filename: example.line}
            maps = {name: line_map(source, changed_fences[origins[name]])
                    for name, source in sources.items()
                    if origins.get(name) in changed_fences}
            n = by_line[example.line]
            if not maps or n + 1 == len(blocks) or blocks[n + 1].info != 'text':
                continue
            output = blocks[n + 1]
            if example.kind not in ('error', 'types'):
                if example.kind != 'format' and RUNTIME_LOCATION.search(output.source):
                    raise Refusal(f'{page}:{output.line}: runtime transcript locations need explicit review')
                continue
            def remap(match):
                name, old = match['file'], int(match['line'])
                if name not in maps:
                    return match[0]
                if old not in maps[name]:
                    raise Refusal(f'{page}:{output.line}: cannot map {match[0]} to an unchanged source line')
                new = maps[name][old]
                if new == old:
                    return match[0]
                positions.append(f'{page}:{output.line}: {name}:{old}:{match["col"]} -> {name}:{new}:{match["col"]}')
                return f'{match["prefix"]}{name}:{new}:{match["col"]}'
            leading = {match.start('file') for match in LOCATION.finditer(output.source)}
            if any(match['file'] in maps and match.start('file') not in leading
                   for match in ANY_LOCATION.finditer(output.source)):
                raise Refusal(f'{page}:{output.line}: embedded local locations need explicit review')
            if SHORT_LOCATION.search(output.source):
                raise Refusal(f'{page}:{output.line}: unqualified transcript positions need explicit review')
            new = LOCATION.sub(remap, output.source)
            if new != output.source:
                edits[n + 1] = new
    rewritten = {}
    for page, (lines, blocks, edits) in documents.items():
        for n, new in sorted(edits.items(), reverse=True):
            block = blocks[n]
            start, count = block.line, len(block.source.splitlines(True))
            raw = lines[start:start + count]
            if block.info == 'text' and not blank_only(block.source, new):
                # Only remapped diagnostic locations may change text bytes.
                old_lines, new_lines = block.source.splitlines(True), new.splitlines(True)
                if len(new_lines) != count:
                    raise Refusal('a transcript location edit changed its line count')
                replacement = []
                for raw_line, old_line, new_line in zip(raw, old_lines, new_lines):
                    if not raw_line.endswith(old_line):
                        raise Refusal('cannot preserve transcript indentation')
                    replacement.append(raw_line[:len(raw_line) - len(old_line)] + new_line)
                lines[start:start + count] = replacement
            else:
                lines[start:start + count] = raw_replacement(raw, block.source, new)
        rewritten[page] = ''.join(lines)
    return rewritten, changes, positions


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--write', action='store_true', help='apply blank-line fixes and safe transcript line mappings')
    parser.add_argument('paths', nargs='*', type=Path, help='Markdown files/directories (default: authored docs and spec)')
    args = parser.parse_args()
    try:
        pages = paths(args.paths)
        rewritten, changes, positions = prepare(pages, compiler())
        changed = [p for p in pages if p.read_bytes().decode('utf-8') != rewritten[p]]
        if args.write:
            for page in changed:
                page.write_bytes(rewritten[page].encode('utf-8'))
        for message in changes + positions:
            print(message)
        print(f'{len(pages)} Markdown files checked; {len(changed)} {"fixed" if args.write else "need header-gap fixes"}; {len(positions)} recorded locations mapped.')
        return bool(changed) and not args.write
    except (OSError, ValueError) as error:
        print(f'format-fences: {error}', file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
