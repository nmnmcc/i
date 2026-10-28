#!/usr/bin/env python3
"""Execute the checked-in Unicode Gleam conformance tests on the reference VM.

Only the two UTF-8 scalar conversion intrinsics are replaced with Erlang shims;
segmentation, property lookup, full casing and all assertions execute the exact
Gleam sources shipped by Wleam. The Wasm conformance suite separately verifies
the production code generator and intrinsics.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[2]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gleam", default=os.environ.get("GLEAM_BIN", "gleam"))
    args = parser.parse_args()
    version = subprocess.check_output([args.gleam, "--version"], text=True).strip()
    if version != "gleam 1.19.2":
        raise SystemExit(f"Expected Gleam 1.19.2, got {version!r}")
    with tempfile.TemporaryDirectory(prefix="wleam-unicode-reference-") as temporary:
        project = Path(temporary)
        source = project / "src"
        (source / "gleam").mkdir(parents=True)
        (project / "gleam.toml").write_text('name = "unicode_reference"\nversion = "0.1.0"\ntarget = "erlang"\n')
        for module in ("wleam_unicode", "wleam_unicode_tables"):
            text = (ROOT / f"stdlib/src/gleam/{module}.gleam").read_text()
            text = text.replace('@external(wasm, "wleam:runtime/v1",', '@external(erlang, "unicode_reference_ffi",')
            (source / "gleam" / f"{module}.gleam").write_text(text)
        calls = []
        assertions = 0
        imports = []
        for test in sorted((ROOT / "tests/stdlib").glob("string_unicode_*_test.gleam")):
            text = test.read_text()
            (source / test.name).write_text(text)
            imports.append(f"import {test.stem}")
            for function in re.findall(r"pub fn (\w+_test)\(\)", text):
                calls.append(f"  {test.stem}.{function}()")
            assertions += len(re.findall(r"\bassert\b", text))
        (source / "unicode_reference.gleam").write_text("\n".join(imports) + "\n\npub fn main() {\n" + "\n".join(calls) + "\n}\n")
        (source / "unicode_reference_ffi.erl").write_text('''-module(unicode_reference_ffi).
-export([string_codepoints/1, string_from_codepoints/1]).
string_codepoints(Text) -> unicode:characters_to_list(Text, utf8).
string_from_codepoints(Codepoints) -> unicode:characters_to_binary(Codepoints, unicode, utf8).
''')
        subprocess.run([args.gleam, "run"], cwd=project, check=True)
        print(json.dumps({"unicode": "17.0.0", "reference": version, "test_functions": len(calls), "assertions": assertions, "status": "passed"}, sort_keys=True))


if __name__ == "__main__":
    main()
