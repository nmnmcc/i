#!/usr/bin/env python3
"""Package a built `i` as the archive that `packages.i-bin` installs.

    nix/package.py --binary PATH --version 0.1.0 --platform x86_64-linux \
        --output dist

The archive is `i-<version>-<platform>.tar.gz` and contains `bin/i` at its
root. It is deterministic: sorted entries, mtime 0, uid/gid 0, and a gzip
header without a timestamp, so the same binary yields the same hash. The
archive is written to standard output as a line (`sha256-...`) that can be
copied into `nix/release.nix`.
"""

from __future__ import annotations

import argparse
import base64
import gzip
import hashlib
import io
import os
from pathlib import Path
import tarfile


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--platform", required=True)
    parser.add_argument("--output", type=Path, default=Path("dist"))
    args = parser.parse_args()

    root = Path(__file__).resolve().parent.parent
    name = f"i-{args.version}-{args.platform}.tar.gz"
    args.output.mkdir(parents=True, exist_ok=True)
    archive = args.output / name

    payload = [(args.binary, "bin/i")]
    for extra in ("LICENSE", "README.md"):
        path = root / extra
        if path.is_file():
            payload.append((path, extra))

    with open(archive, "wb") as raw:
        # mtime 0 removes the build clock from the gzip header.
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode="w") as tar:
                for source, destination in payload:
                    data = source.read_bytes()
                    info = tarfile.TarInfo(destination)
                    info.size = len(data)
                    info.mode = 0o755 if destination == "bin/i" else 0o644
                    info.mtime = 0
                    info.uid = info.gid = 0
                    info.uname = info.gname = ""
                    tar.addfile(info, io.BytesIO(data))

    digest = hashlib.sha256(archive.read_bytes()).digest()
    sri = "sha256-" + base64.b64encode(digest).decode()
    print(f"archive: {archive}")
    print(f"hash: {sri}")
    print(f"url-name: {name}")


if __name__ == "__main__":
    main()
