#!/usr/bin/env python3
"""Make library pages with checked value signatures and complete type definitions."""
from pathlib import Path
import io
import json
import os
import re
import subprocess
import tarfile
from spec_links import unlink_spec_references

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'docs/content/docs/library/generated'
INDEX = ROOT / 'docs/content/docs/library/index.mdx'
BINARY = Path(os.environ.get('I_BIN', ROOT / 'target/release/i')).resolve()
HEAVY = next((path for path in (ROOT.parent / 'cache/heavy', ROOT / '.rd/cache/heavy')
              if path.is_file()), None)


def newest_tag():
    out = subprocess.run(['git', '-C', str(ROOT), 'tag', '--list', 'stdlib-v*', '--sort=-v:refname'],
                         capture_output=True, text=True, check=True).stdout.split()
    return out[0] if out else None


def library_root():
    """The library the site documents: the newest tag, the one `i new` writes."""
    override = os.environ.get('LIBRARY_ROOT')
    if override:
        return Path(override).resolve()
    tag = os.environ.get('LIBRARY_TAG') or newest_tag()
    if not tag:
        raise SystemExit('no stdlib-v* tag to document')
    archive = subprocess.run(['git', '-C', str(ROOT), 'archive', f'{tag}:stdlib'],
                             capture_output=True, check=True).stdout
    destination = Path(os.environ.get('LIBRARY_EXPORT', '/tmp')) / ('i-' + tag)
    directory = destination / 'stdlib'
    if directory.is_dir():
        return directory
    directory.mkdir(parents=True)
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        tar.extractall(directory, filter='data')
    return directory


TAG = os.environ.get('LIBRARY_TAG') or newest_tag()
VERSION = TAG.removeprefix('stdlib-v') if TAG else None
LIBRARY = library_root()


def checked_types(project):
    cmd = [str(BINARY), 'check', '--project', str(project), '--types', '--offline']
    if HEAVY:
        cmd = [str(HEAVY)] + cmd
    result = subprocess.run(cmd, cwd=ROOT, text=True, capture_output=True)
    if result.returncode:
        raise SystemExit('i check --types failed:\n' + result.stderr)
    types = {}
    for line in result.stdout.splitlines():
        match = re.fullmatch(r'([\w/]+)\.([\w]+) : (.*)', line)
        if match:
            types[(match[1], match[2])] = match[3]
    return types


def inferred_types(modules):
    if not BINARY.is_file() or not os.access(BINARY, os.X_OK):
        raise SystemExit(f'Build target/release/i or set I_BIN to an executable compiler: {BINARY}')
    # The library is one package, the root of `stdlib/`, and --types prints
    # every exported value of the root package with its needs and Fail.
    types = checked_types(LIBRARY)
    missing = [(module, name) for module, source in modules
               for name in next(groups(source), [])
               if (name[0].islower() or name[0] == '_') and (module, name) not in types]
    if missing:
        raise SystemExit('Missing inferred signatures: ' + ', '.join(f'{m}.{n}' for m, n in missing))
    return types

def headers(text):
    # The initial groups are imports (containing a module path) and exports.
    rest = re.sub(r'\A(?:\s|//[^\n]*\n)*', '', text)
    for _ in range(2):
        if not rest.startswith('('):
            return
        depth = 0
        for pos, ch in enumerate(rest):
            depth += (ch == '(') - (ch == ')')
            if depth == 0:
                break
        else:
            return
        group, rest = rest[1:pos], rest[pos + 1:].lstrip()
        if rest.startswith('='):
            return
        yield group


def groups(text):
    for group in headers(text):
        group = re.sub(r'//[^\n]*', '', group)
        if '/' not in group:
            yield re.findall(r'\b([A-Za-z_]\w*)(?:\(\.\.\))?', group)
            return

def safe(text):
    text = unlink_spec_references(text)
    # MDX text can contain JSX metacharacters in source comments.
    output = []
    fenced = False
    for line in text.split('\n'):
        if line.startswith('```'):
            fenced = not fenced
        elif not fenced:
            chunks = re.split(r'(`+[^`]*`+)', line)
            for index in range(0, len(chunks), 2):
                chunks[index] = chunks[index].replace('&', '&amp;').replace('<', '&lt;').replace('{', '&#123;').replace('}', '&#125;')
            line = ''.join(chunks)
        output.append(line)
    return '\n'.join(output)


def declaration(source, name):
    """Read a whole declaration, including multiline data and function types."""
    match = re.search(r'(?m)^' + re.escape(name) + r'(?=\s*(?:[:=(]|$))[^\n]*', source)
    if not match:
        raise SystemExit(f'Source declaration not found: {name}')
    depth = 0
    end = len(source)
    # Ignore comment and string contents while balancing the declaration.
    tokens = re.finditer(r'//[^\n]*|"(?:\\.|[^"\\])*"|[()\[\]{}\n]', source[match.start():])
    for token in tokens:
        text = token.group()
        if text in ('(', '[', '{'):
            depth += 1
        elif text in (')', ']', '}'):
            depth -= 1
        elif text == '\n' and depth == 0:
            end = match.start() + token.start()
            break
    return match.start(), source[match.start():end].rstrip()


def generate():
    modules = [(prefix + '/' + file.stem, file.read_text())
               for prefix, directory in [('i', LIBRARY / 'src')]
               for file in sorted(directory.glob('*.i'))]
    types = inferred_types(modules)
    # Do not replace any generated pages unless every value has a checked type.
    OUT.mkdir(parents=True, exist_ok=True)
    for old in OUT.glob('*.mdx'):
        old.unlink()
    pages = []
    for module, source in modules:
        prefix, name = module.split('/')
        slug = prefix + '-' + name.replace('_', '-')
        pages.append(slug)
        opening = []
        for line in source.splitlines():
            if line.startswith('////'):
                opening.append(line[4:].lstrip())
            elif not line.strip() and not opening:
                continue
            else:
                break
        banner = (f'> Documents `{TAG}`, the version `i new` writes.'
                  if VERSION else '> Documents the working-tree standard library.')
        output = ['---', 'title: ' + json.dumps(module), '---', '', banner, '', *([safe('\n'.join(opening)), ''] if any(line.strip() for line in opening) else [])]
        exports = next(groups(source), [])
        for name in exports:
            # Attributes may stand between the documentation and the item.
            start, written = declaration(source, name)
            output += ['## ' + name, '']
            before = source[:start].splitlines()
            while before and before[-1].lstrip().startswith('@'):
                before.pop()
            comments = []
            while before and before[-1].startswith('///') and not before[-1].startswith('////'):
                comments.append(before.pop()[3:].lstrip())
            if any(line.strip() for line in comments):
                output += [safe('\n'.join(reversed(comments))), '']
            signature = types.get((module, name))
            if signature:
                output += ['```text', name + ' : ' + signature, '```', '']
            else:
                # --types lists values. Types retain their complete definition.
                output += ['```text', written, '```', '']
        (OUT / (slug + '.mdx')).write_text('\n'.join(output))
    (OUT / 'meta.json').write_text(json.dumps({'title':'Modules','pages':pages}, indent=2) + '\n')
    if VERSION:
        links = ' · '.join('[' + slug[2:] + '](/docs/library/generated/' + slug + ')' for slug in pages)
        text = INDEX.read_text()
        start, end = '{/* generated:start */}', '{/* generated:end */}'
        if start in text and end in text:
            text = re.sub('(?s)' + re.escape(start) + '.*?' + re.escape(end),
                          start + '\n\nDocuments `' + TAG + '`, the version `i new` writes. The source is [stdlib/src at `' + TAG + '`](https://github.com/nmnmcc/i/tree/' + TAG + '/stdlib/src).\n\n' + links + '\n\n' + end, text)
            INDEX.write_text(text)


if __name__ == '__main__':
    generate()
