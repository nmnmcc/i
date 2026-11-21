"""The one definition of the library the documentation describes.

Three things must name the same library: the pages the site builds, the
module check that accepts `i/x` claims, and the examples that run. They used
to read the baseline separately and drifted: the site documented the working
tree while the examples ran the newest `stdlib-v*` tag, which no released
compiler builds, so every example failed with E0522. So the baseline is one
value, defined here and read by all three.

`LIBRARY_TAG` names a released tag, whose `stdlib/` is exported from git
under `LIBRARY_EXPORT`; without it the baseline is the working tree. A tag is
a released library, built by the compiler released with it; the working tree
is what the unreleased compiler builds.
"""
import io
import os
import subprocess
import tarfile
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


@dataclass(frozen=True)
class Baseline:
    root: Path
    tag: str | None = None

    @property
    def label(self):
        return self.tag or 'the working tree'

    @property
    def version(self):
        return self.tag.removeprefix('stdlib-v') if self.tag else None


def current(root=None):
    """The library this checkout documents: the working tree, or a tag."""
    root = Path(root or ROOT).resolve()
    override = os.environ.get('LIBRARY_ROOT')
    if override:
        return Baseline(Path(override).resolve())
    tag = os.environ.get('LIBRARY_TAG') or None
    if not tag:
        return Baseline(root / 'stdlib')
    destination = Path(os.environ.get('LIBRARY_EXPORT', '/tmp')) / ('i-' + tag) / 'stdlib'
    if not destination.is_dir():
        archive = subprocess.run(['git', '-C', str(root), 'archive', f'{tag}:stdlib'],
                                 capture_output=True, check=True).stdout
        destination.mkdir(parents=True)
        with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
            tar.extractall(destination, filter='data')
    return Baseline(destination, tag)
