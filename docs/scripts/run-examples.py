#!/usr/bin/env python3
"""Execute the complete examples in authored documentation, without network access."""
import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, field
import fnmatch
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import tomllib

from example_fences import fences, generated, i_info, safe_file
from paths import ROOT

KINDS = ('run', 'test', 'check', 'error')
TIMEOUT = 60
# Match diagnostic headers, not codes mentioned in their messages or source.
DIAGNOSTIC = re.compile(r'(?m)^(?:[^\n]+:\d+:\d+: )?(?:error|warning|note)\[([A-Z]\d{4})\]:')


@dataclass
class Failure:
    page: Path
    line: int
    expected: str
    actual: str

    def __str__(self):
        try:
            page = self.page.relative_to(ROOT)
        except ValueError:
            page = self.page
        return f'{page}:{self.line}: expected {self.expected}; actual {self.actual}'


@dataclass
class Example:
    page: Path
    line: int
    kind: str
    source: str
    files: dict = field(default_factory=dict)
    output: str = ''
    codes: frozenset = frozenset()


def read_page(page):
    blocks = list(fences(page.read_text()))
    examples, failures, pending = [], [], {}
    counts = Counter()
    for index, block in enumerate(blocks):
        try:
            info = i_info(block.info)
            if info is None:
                continue
            kind, path, codes = info
            counts[kind] += 1
            if not block.closed:
                raise ValueError('unclosed i fence')
            if kind == 'fragment':
                continue
            if kind == 'file':
                safe_file(path)
                if path in pending:
                    raise ValueError(f'duplicate file={path} before the next example')
                pending[path] = block
                continue
            files = {name: fence.source for name, fence in pending.items()}
            pending.clear()
            output = ''
            if kind == 'run':
                if index + 1 >= len(blocks) or blocks[index + 1].info != 'text' or not blocks[index + 1].closed:
                    raise ValueError('run needs a closed text fence as the next fence')
                output = blocks[index + 1].source
            examples.append(Example(page, block.line, kind, block.source, files, output, codes))
        except ValueError as error:
            failures.append(Failure(page, block.line, 'a valid example fence', str(error)))
    for name, block in pending.items():
        failures.append(Failure(page, block.line, 'an example after file=' + name, 'end of page'))
    return examples, failures, counts


def page_name(page):
    name = re.sub(r'[^a-z0-9_]', '_', page.stem.lower())
    return name if re.match(r'^[a-z]', name) else 'page_' + name


def toml_value(value):
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False)
    if isinstance(value, bool):
        return str(value).lower()
    if isinstance(value, (int, float)):
        return repr(value)
    if isinstance(value, list):
        return '[' + ', '.join(toml_value(item) for item in value) + ']'
    if isinstance(value, dict):
        return '{ ' + ', '.join(json.dumps(key) + ' = ' + toml_value(item)
                                for key, item in value.items()) + ' }'
    raise ValueError(f'unsupported manifest value: {value!r}')


def prepare(example, project):
    name = page_name(example.page)
    main = f'test/{name}_test.i' if example.kind == 'test' else 'src/main.i'
    if main in example.files:
        raise ValueError(f'file={main} duplicates the example source')
    sources = {**example.files, main: example.source}
    manifest = tomllib.loads(sources.pop('i.toml', ''))
    manifest['name'] = name
    std = {'path': str(ROOT / 'std')}
    for table in ('dependencies', 'dev-dependencies', 'test'):
        if table in manifest and not isinstance(manifest[table], dict):
            raise ValueError(f'manifest {table} must be a table')
    if any(re.search(r'\bstd/', source) for path, source in sources.items()
           if Path(path).suffix in ('.i', '.is') and (example.kind != 'test' or path.startswith('src/'))):
        manifest.setdefault('dependencies', {})['std'] = std
    if example.kind == 'test':
        manifest.setdefault('test', {})['collection'] = 'std/test.tests'
        # The manifest forbids repeating a name across dependency tables.
        # Source modules need a regular dependency; test-only imports use dev.
        if 'std' in manifest.get('dependencies', {}):
            manifest['dependencies']['std'] = std
            manifest.get('dev-dependencies', {}).pop('std', None)
        else:
            manifest.setdefault('dev-dependencies', {})['std'] = std
    # Inline TOML tables preserve any supplementary manifest configuration.
    (project / 'i.toml').write_text(''.join(json.dumps(key) + ' = ' + toml_value(value) + '\n'
                                         for key, value in manifest.items()))
    for path, source in sources.items():
        target = project / safe_file(path)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(source)


def invoke(command, project, timeout):
    """Kill the compiler and any children together when the deadline expires."""
    env = {**os.environ, 'NO_COLOR': '1', 'RAYON_NUM_THREADS': '1', 'BINARYEN_CORES': '1'}
    with subprocess.Popen(command, cwd=project, env=env, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, start_new_session=True) as process:
        try:
            stdout, stderr = process.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            stdout, stderr = process.communicate()
            raise TimeoutError(f'timed out; stdout={stdout!r}; stderr={stderr!r}')
        return process.returncode, stdout.decode('utf-8', errors='replace'), stderr.decode('utf-8', errors='replace')


def output_matches(expected, actual):
    # Tolerate precisely one terminal LF, without trimming spaces or blank lines.
    return expected.removesuffix('\n') == actual.removesuffix('\n')


def run_example(example, binary, timeout=TIMEOUT):
    failures = []
    with tempfile.TemporaryDirectory(prefix=f'docs-{page_name(example.page)}-') as scratch:
        project = Path(scratch)
        try:
            prepare(example, project)
        except (OSError, ValueError, TypeError) as error:
            return [Failure(example.page, example.line, 'a valid temporary project', str(error))]
        levels = (0, 2) if example.kind in ('run', 'test') else (None,)
        deadline = time.monotonic() + timeout
        for opt in levels:
            command = [str(binary), 'check' if example.kind == 'error' else example.kind, '--offline']
            if opt is not None:
                command += ['--opt', str(opt)]
            label = example.kind + (f' --opt {opt}' if opt is not None else '')
            expected = {'run': f'exit 0 and stdout={example.output!r}',
                        'test': 'exit 0 (all tests pass)',
                        'check': 'exit 0 and no diagnostics',
                        'error': 'nonzero exit and codes=' + repr(sorted(example.codes))}[example.kind]
            try:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise TimeoutError('example deadline expired')
                status, stdout, stderr = invoke(command, project, remaining)
            except (OSError, TimeoutError) as error:
                failures.append(Failure(example.page, example.line, f'{label}: {expected}', str(error)))
                continue
            codes = frozenset(DIAGNOSTIC.findall(stdout + '\n' + stderr))
            if example.kind == 'run':
                passed = status == 0 and output_matches(example.output, stdout)
            elif example.kind == 'test':
                passed = status == 0
            elif example.kind == 'check':
                passed = status == 0 and not stdout and not stderr
            else:
                passed = status > 0 and codes == example.codes
            if not passed:
                failures.append(Failure(example.page, example.line, f'{label}: {expected}',
                                        f'exit {status}, codes={sorted(codes)!r}, stdout={stdout!r}, stderr={stderr!r}'))
    return failures


def compiler():
    configured = os.environ.get('I_BIN')
    if configured:
        found = shutil.which(configured)
        if not found:
            raise ValueError(f'I_BIN is not executable: {configured}')
        return Path(found).resolve()
    binary = ROOT / 'target/release/i'
    if not binary.is_file():
        command = ['cargo', 'build', '--release', '--locked', '--offline', '-p', 'ilang-cli']
        for heavy, shell in ((ROOT.parent / 'cache/heavy', ROOT.parent.parent),
                             (ROOT / '.rd/cache/heavy', ROOT)):
            if heavy.is_file():
                command = [str(heavy), 'direnv', 'exec', str(shell)] + command
                break
        subprocess.run(command, cwd=ROOT, check=True)
    return binary


def selected(page, patterns, content):
    return not patterns or any(fnmatch.fnmatchcase(str(path), pattern)
                               for pattern in patterns for path in
                               (page.relative_to(content), page.relative_to(ROOT),
                                page.relative_to(ROOT / 'docs'), page))


def check_content(content, patterns=(), no_placeholders=False):
    examples, failures, counts = [], [], Counter()
    for page in sorted(content.rglob('*')):
        if not page.is_file() or generated(page.relative_to(content)) or not selected(page, patterns, content):
            continue
        if no_placeholders:
            for line, source in enumerate(page.read_bytes().splitlines(), 1):
                if b'to be written' in source:
                    failures.append(Failure(page, line, 'no placeholder text', repr(source.decode(errors='replace'))))
        if page.suffix in ('.md', '.mdx'):
            found, errors, kinds = read_page(page)
            examples.extend(found)
            failures.extend(errors)
            counts.update(kinds)
    return examples, failures, counts


def positive_int(value):
    value = int(value)
    if value < 1:
        raise argparse.ArgumentTypeError('must be at least 1')
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--jobs', type=positive_int, default=4, help='parallel examples (default: 4)')
    parser.add_argument('--page', action='append', default=[], metavar='GLOB', help='page glob, relative to docs/content or the repository; repeatable')
    parser.add_argument('--no-placeholders', action='store_true', help='also reject "to be written" in authored content')
    args = parser.parse_args()
    examples, failures, counts = check_content(ROOT / 'docs/content', args.page, args.no_placeholders)
    if examples:
        try:
            binary = compiler()
        except (OSError, ValueError, subprocess.CalledProcessError) as error:
            for example in examples:
                failures.append(Failure(example.page, example.line, 'an available compiler', str(error)))
        else:
            with ThreadPoolExecutor(max_workers=args.jobs) as pool:
                for errors in pool.map(lambda example: run_example(example, binary), examples):
                    failures.extend(errors)
    else:
        print('No executable documentation examples found.')
    for failure in failures:
        print(failure, file=sys.stderr)
    for kind in (*KINDS, 'file', 'fragment'):
        print(f'{kind}: {counts[kind]} fences')
    print(f'{len(examples)} examples checked; {len(failures)} failures.')
    return bool(failures)


if __name__ == '__main__':
    sys.exit(main())
