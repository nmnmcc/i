# Unicode data and algorithms

iLang's string functions use the Unicode **17.0.0** character database and the
extended grapheme-cluster rules in **UAX #29 revision 47**. They use the same
tables and Gleam implementation on every Wasm host.

`manifest.json` records the exact upstream URLs and SHA-256 hashes of every
vendored source. The Unicode data license is in `LICENSE.txt`. The generator
verifies these hashes before reading a source and does not access the network.

From the repository root:

```sh
python3 stdlib/unicode/generate.py
python3 stdlib/unicode/generate.py --check
python3 stdlib/unicode/test_reference.py
```

The generator uses only Python's standard library and the pinned reference
Gleam 1.19.2 formatter provided by the development environment. Set `GLEAM_BIN`
or pass `--formatter /path/to/gleam` when the reference compiler is not on PATH.

The generator writes `i/ilang_unicode_tables.i` and
`tests/stdlib/string_unicode_generated_test.i`. Properties and case mappings
are compact constant bit arrays with shared binary-search routines. This keeps
generated executable code small and lookup logarithmic, without thousands of
generated decision-tree branches.

`i/ilang_unicode.i` implements grapheme segmentation, including Indic
conjuncts (GB9c), emoji ZWJ sequences (GB11), and paired regional indicators
(GB12/GB13). Full default lowercase and uppercase mappings include one-to-many
expansions and the context-sensitive final sigma. Locale-specific Turkish,
Azeri, and Lithuanian casing is intentionally outside the locale-independent
string API. No normalization is performed.

Only UTF-8 decoding and encoding are runtime intrinsics:

- `ilang:runtime/v1.string_codepoints`: `String -> List(Int)` of Unicode scalars.
- `ilang:runtime/v1.string_from_codepoints`: `List(Int) -> String` of valid scalars.

Generated conformance cases cover every boundary vector from the Unicode 17
`GraphemeBreakTest.txt` and every nonidentity default full case mapping. The
additional handwritten tests cover contextual sigma, empty strings, whitespace
classification, scalar preservation, and expansion ordering.

`test_reference.py` runs these exact Gleam algorithms and assertions with the
pinned reference compiler on Erlang, using only an UTF-8 decode/encode shim.
This checks source-level conformance independently of the Wasm backend. The
production Wasm conformance tests additionally check Wasm code generation and
its scalar conversion intrinsics.
