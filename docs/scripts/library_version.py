#!/usr/bin/env python3
"""The standard-library version this compiler pairs with.

The docs, `i new` and the language are one release: a library tag is source,
compiled by the current compiler, so an unpaired tag can stop checking when
the language changes. The compiler owns the pair. Read it from
`i stdlib-version` when the compiler has that command, else from the
compiler's package version, which the release keeps equal.

`I_STDLIB_VERSION` overrides it for work on an unpublished release.
"""
from pathlib import Path
import os
import re
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[2]
VERSION = re.compile(r'\d+\.\d+\.\d+')


def compiler(root=ROOT):
    configured = os.environ.get('I_BIN')
    path = Path(configured) if configured else root / 'target/release/i'
    return path if path.is_file() and os.access(path, os.X_OK) else None


def paired(root=ROOT):
    override = os.environ.get('I_STDLIB_VERSION')
    if override:
        if not VERSION.fullmatch(override):
            raise SystemExit(f'I_STDLIB_VERSION is not X.Y.Z: {override}')
        return override
    binary = compiler(root)
    if binary:
        result = subprocess.run([str(binary), 'stdlib-version'],
                                capture_output=True, text=True)
        answer = result.stdout.strip()
        if result.returncode == 0 and VERSION.fullmatch(answer):
            return answer
    manifest = tomllib.loads((root / 'Cargo.toml').read_text())
    version = (manifest.get('workspace', {}).get('package') or manifest.get('package', {})).get('version', '')
    if not VERSION.fullmatch(version):
        raise SystemExit(f'no paired stdlib version: Cargo.toml has {version}')
    return version


def tag(version):
    return 'stdlib-v' + version


def tag_commit(root, version):
    result = subprocess.run(['git', '-C', str(root), 'rev-parse', '--verify', '--quiet',
                             'refs/tags/' + tag(version) + '^{commit}'],
                            capture_output=True, text=True)
    return result.stdout.strip() or None


def require_tag(root, version):
    if not tag_commit(root, version):
        raise SystemExit(
            f'the compiler pairs stdlib {version}, but tag {tag(version)} is not published; '
            f'publish the pair together')
    return tag(version)
