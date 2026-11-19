#!/usr/bin/env python3
"""Make library pages with checked value signatures and complete type definitions."""
from pathlib import Path
import json
import os
import re
import subprocess
import tempfile
from spec_links import unlink_spec_references

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'docs/content/docs/library/generated'
BINARY = Path(os.environ.get('I_BIN', ROOT / 'target/release/i')).resolve()
HEAVY = next((path for path in (ROOT.parent / 'cache/heavy', ROOT / '.rd/cache/heavy')
              if path.is_file()), None)


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
    types = checked_types(ROOT / "stdlib")
    # --types prints only the root package. Return each exported i value from a
    # function in a temporary root to obtain its scheme, including needs and
    # Fail. The function defers compiler-owned syntax operations, which cannot
    # be read by an ordinary module's initialization.
    # Import the module's own exported types so those names stay unqualified,
    # just as they are in the original module's checked signatures.
    with tempfile.TemporaryDirectory(prefix='i-library-reference-') as directory:
        project = Path(directory)
        (project / 'src').mkdir()
        (project / 'i.toml').write_text('name = "library_reference"\n')
        for module, source in modules:
            if not module.startswith('i/'):
                continue
            names = next(groups(source), [])
            values = [name for name in names if name[0].islower() or name[0] == '_']
            if not values:
                continue
            own_types = [name for name in names if name[0].isupper()]
            # A fixed alias cannot collide with an exported binding or type.
            alias = 'library_module'
            while re.search(r'\b' + alias + r'\b', source):
                alias += '_'
            imports = f'{alias} = '
            if own_types:
                imports += '(' + ', '.join(own_types) + ') = '
            imports += module
            original_imports = next((group for group in headers(source)
                                     if '/' in re.sub(r'//[^\n]*', '', group)), '')
            text = '(\n' + original_imports + '\n' + imports + '\n)\n'
            text += '(' + ', '.join(values) + ')\n'
            text += ''.join(f'{name} = () {{ {alias}.{name} }}\n' for name in values)
            (project / 'src' / (module.split('/')[1] + '.i')).write_text(text)
        for (module, name), signature in checked_types(project).items():
            module = module.replace('library_reference/', 'i/', 1)
            # The target module's own exported types were imported by name;
            # any remaining qualified types belong to other library modules.
            if not signature.startswith('() -> '):
                raise SystemExit(f'Expected a signature getter for {module}.{name}: {signature}')
            types[(module, name)] = signature.removeprefix('() -> ')
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
               for prefix, directory in [('i', ROOT / 'lib/i'), ('std', ROOT / 'std/src')]
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
        output = ['---', 'title: ' + json.dumps(module), '---', '', *([safe('\n'.join(opening)), ''] if any(line.strip() for line in opening) else [])]
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


if __name__ == '__main__':
    generate()
