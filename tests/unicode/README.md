# Unicode conformance

This package checks the public `i/string` API against the pinned Unicode
17.0.0 data in `stdlib/unicode/data`. Expected grapheme boundaries come directly
from Unicode's `auxiliary/GraphemeBreakTest.txt`. Default lowercase and uppercase
expectations combine `UnicodeData.txt` with the unconditional `SpecialCasing.txt`
mappings. Separate examples exercise final sigma in the original input context,
combining marks, default locale behavior, expansion mappings, NUL, and empty text.

Regenerate with `python3 tests/unicode/generate.py`, or check that the corpus is
current with `python3 tests/unicode/generate.py --check`. The latter reads files
only. `corpus.json` records input SHA-256 checksums and exact case counts.

Run this directory as an iLang test project. The generated test functions group
32 cases per function to keep Wasm instance setup costs bounded. Every assertion
remains explicit so failures retain a specific source location.

The oracle files are published by the Unicode Consortium in the
[Unicode 17.0.0 database](https://www.unicode.org/Public/17.0.0/ucd/) and retain their copyright and license
headers in the vendored data directory. The corpus generator does not use
iLang's Unicode implementation or the host Python Unicode database to derive
expected results.
