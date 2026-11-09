#!/usr/bin/env python3
"""Make the case tables of the runtime from UnicodeData.txt.

The functions `core.to_upper` and `core.to_lower` (spec §13) map each
codepoint by its simple uppercase or lowercase mapping: fields 12 and 13 of
crates/ilang-wasm/unicode/UnicodeData.txt. This script writes them as range
tables to crates/ilang-wasm/src/runtime/case_tables.rs.

A range is `(first, last, step, delta)`: every codepoint `c` from `first` to
`last` with `(c - first) % step == 0` maps to `c + delta`. `step` is 1 or 2;
2 covers the many blocks where upper and lower case letters take turns.
Ranges are sorted and do not overlap, also counting the codepoints that a
range with step 2 steps over, so a binary search on `first` finds the only
range that may hold a codepoint. A codepoint in no range maps to itself.

The output depends only on the input file, so running the script again gives
the same bytes. With `--check`, it writes nothing and fails when the file in
the repository is not what it would write.

Usage: scripts/gen-case-tables.py [--check]
"""

import pathlib
import sys

# The version of crates/ilang-wasm/unicode/UnicodeData.txt.
UNICODE_VERSION = "18.0.0"

ROOT = pathlib.Path(__file__).resolve().parent.parent
SOURCE = ROOT / "crates/ilang-wasm/unicode/UnicodeData.txt"
OUTPUT = ROOT / "crates/ilang-wasm/src/runtime/case_tables.rs"

UPPER_FIELD = 12
LOWER_FIELD = 13


def read_mappings(path):
    """Give two dicts, codepoint -> simple uppercase and -> simple lowercase.

    Ranges written as `<..., First>` and `<..., Last>` have no case mappings,
    so only single lines need to be read.
    """
    upper, lower = {}, {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line:
            continue
        fields = line.split(";")
        if len(fields) != 15:
            raise SystemExit(f"bad line in {path}: {line!r}")
        code = int(fields[0], 16)
        for field, table in ((UPPER_FIELD, upper), (LOWER_FIELD, lower)):
            if fields[field]:
                mapped = int(fields[field], 16)
                if mapped != code:
                    table[code] = mapped
    return upper, lower


def make_ranges(mapping):
    """Put the mappings into as few ranges as the greedy method finds.

    A range with step 2 may only step over codepoints with no mapping, so
    that ranges never overlap.
    """
    ranges = []
    for code in sorted(mapping):
        delta = mapping[code] - code
        if ranges:
            first, last, step, d = ranges[-1]
            if d == delta:
                if code == last + 1 and (step == 1 or first == last):
                    ranges[-1] = (first, code, 1, delta)
                    continue
                if code == last + 2 and (step == 2 or first == last) and last + 1 not in mapping:
                    ranges[-1] = (first, code, 2, delta)
                    continue
        ranges.append((code, code, 1, delta))
    return ranges


def check_ranges(ranges, mapping):
    """Check that the ranges give exactly the mapping."""
    covered = {}
    previous_last = -1
    for first, last, step, delta in ranges:
        assert first > previous_last, "ranges overlap"
        assert step in (1, 2)
        assert 0 <= first <= last <= 0x10FFFF
        for code in range(first, last + 1, step):
            covered[code] = code + delta
        previous_last = last
    assert covered == mapping, "ranges do not give the mapping"


def render(name, doc, ranges):
    lines = [f"/// {doc}", f"pub(super) const {name}: &[(u32, u32, u32, i32)] = &["]
    for first, last, step, delta in ranges:
        lines.append(f"    (0x{first:04X}, 0x{last:04X}, {step}, {delta}),")
    lines.append("];")
    return "\n".join(lines)


def generate():
    upper, lower = read_mappings(SOURCE)
    upper_ranges = make_ranges(upper)
    lower_ranges = make_ranges(lower)
    check_ranges(upper_ranges, upper)
    check_ranges(lower_ranges, lower)
    parts = [
        "//! The simple case mappings of Unicode, as range tables (spec §13).",
        "//!",
        "//! Made by `scripts/gen-case-tables.py` from",
        "//! `crates/ilang-wasm/unicode/UnicodeData.txt`. Do not change this file by",
        "//! hand; run the script again.",
        "//!",
        "//! A range `(first, last, step, delta)` maps every codepoint `c` from",
        "//! `first` to `last` with `(c - first) % step == 0` to `c + delta`. The",
        "//! ranges are sorted and do not overlap. Other codepoints map to themselves.",
        "",
        "/// The version of the Unicode Character Database the tables are made from.",
        f'pub(super) const UNICODE_VERSION: &str = "{UNICODE_VERSION}";',
        "",
        render(
            "TO_UPPER",
            f"Simple uppercase mappings: {len(upper)} codepoints in {len(upper_ranges)} ranges.",
            upper_ranges,
        ),
        "",
        render(
            "TO_LOWER",
            f"Simple lowercase mappings: {len(lower)} codepoints in {len(lower_ranges)} ranges.",
            lower_ranges,
        ),
        "",
    ]
    return "\n".join(parts)


def main(argv):
    text = generate()
    if argv[1:] == ["--check"]:
        if OUTPUT.read_text(encoding="utf-8") != text:
            raise SystemExit(f"{OUTPUT} is not up to date: run {sys.argv[0]}")
        return
    if argv[1:]:
        raise SystemExit(__doc__)
    OUTPUT.write_text(text, encoding="utf-8")


if __name__ == "__main__":
    main(sys.argv)
