#!/usr/bin/env python3
"""Execute the ported corpus using the pinned upstream compiler and stdlib."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import tomllib


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gleam", required=True, help="Gleam v1.19.2 executable")
    parser.add_argument("--stdlib", required=True, type=Path, help="Unmodified gleam_stdlib v1.0.5 checkout")
    parser.add_argument("--erlang-bin", type=Path, help="Directory containing erl and escript")
    parser.add_argument("--timeout", type=float, default=120.0)
    args = parser.parse_args()
    gleam = shutil.which(args.gleam)
    if gleam is None:
        parser.error(f"Gleam executable does not exist: {args.gleam}")
    stdlib = args.stdlib.resolve()
    if not (stdlib / "gleam.toml").is_file():
        parser.error(f"Missing reference standard library: {stdlib}")
    if args.timeout <= 0:
        parser.error("--timeout must be positive")
    version = subprocess.run([gleam, "--version"], capture_output=True, text=True, check=True)
    if version.stdout.strip() != "gleam 1.19.2":
        parser.error(f"Expected gleam 1.19.2, got {version.stdout.strip()!r}")
    if tomllib.loads((stdlib / "gleam.toml").read_text()).get("version") != "1.0.5":
        parser.error("The reference standard library must be v1.0.5")

    corpus = Path(__file__).resolve().parent
    inventory = json.loads((corpus / "migration.json").read_text())
    expected = inventory["expected_tests"]
    if len(expected) != inventory["counts"]["ported_public_tests"] or len(set(expected)) != len(expected):
        raise ValueError("The corpus inventory contains a count mismatch or duplicate test")
    declared = set()
    for path in (corpus / "test").rglob("*.gleam"):
        module = path.relative_to(corpus / "test").with_suffix("").as_posix()
        for name in re.findall(r"^pub fn ([a-z][a-z0-9_]*_test)\(\)", path.read_text(), re.M):
            declared.add(f"{module}.{name}")
    if declared != set(expected):
        raise ValueError("The corpus test declarations differ from migration.json")
    modules = sorted({name.rsplit(".", 1)[0] for name in expected})
    aliases = {module: f"suite_{index}" for index, module in enumerate(modules)}
    marker = f"PASS: upstream_language {len(expected)} tests"
    entry = "import gleam/io\n"
    entry += "\n".join(f"import {module} as {alias}" for module, alias in aliases.items())
    entry += "\n\npub fn main() {\n"
    for name in expected:
        module, function = name.rsplit(".", 1)
        entry += f"  {aliases[module]}.{function}()\n"
    entry += f"  io.println({json.dumps(marker)})\n}}\n"

    env = dict(os.environ)
    if args.erlang_bin:
        env["PATH"] = str(args.erlang_bin.resolve()) + os.pathsep + env.get("PATH", "")
    with tempfile.TemporaryDirectory(prefix="wleam-language-oracle-") as directory:
        root = Path(directory)
        for name in ("src", "test"):
            shutil.copytree(corpus / name, root / name)
        (root / "gleam.toml").write_text(
            'name = "upstream_language"\nversion = "1.0.0"\ntarget = "erlang"\n\n'
            f"[dependencies]\ngleam_stdlib = {{ path = {json.dumps(str(stdlib))} }}\n"
        )
        (root / "test" / "oracle.gleam").write_text(entry)
        result = subprocess.run(
            [gleam, "run", "--module", "oracle"],
            cwd=root,
            env=env,
            capture_output=True,
            text=True,
            timeout=args.timeout,
        )
    passed = result.returncode == 0 and result.stdout.splitlines() == [marker]
    print(json.dumps({
        "corpus": "upstream_language",
        "reference": "Gleam v1.19.2 / Erlang",
        "expected_tests": len(expected),
        "passed": passed,
        "exit_code": result.returncode,
        "stdout": result.stdout,
        "stderr": result.stderr if not passed else "",
    }, indent=2))
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
