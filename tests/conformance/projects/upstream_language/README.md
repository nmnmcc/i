# Upstream language runtime conformance

This project ports Gleam v1.19.2's language runtime tests at commit
`ed41eace2dd89054eb2d744c33925261d1eff32e`. Original SPDX notices are retained.

Run it with:

```sh
wleam --project tests/conformance/projects/upstream_language test --json
```

A successful run must report **490 executed tests**, with zero failures. An empty
report or a successful compilation without execution is not a passing run.
`migration.json` records the upstream hashes, every adaptation and all expected
qualified test names.

All core assertions and statements are preserved. Each test ends in `Nil` so
that the new test discovery contract also includes tests whose original final
expression returned a boolean. The UTF-16/32 little-endian codepoint tests now
run unconditionally on Wasm. The two used FFI test helpers use portable standard
library/codepoint and bit-pattern operations.

The two target-exclusive definitions of the TypeScript output-copy check are
kept in `reference_only/`: their Erlang/JavaScript output directories do not
exist in Wleam. The outer conformance harness instead validates the Wasm artifact
header and ABI metadata and executes the resulting module. These definitions
are explicitly excluded from the 490 core runtime test count, not reported as
passing. The former `gleeunit.main` runner and its unused external helper
declarations are also preserved there; `wleam test` owns discovery and execution.

The 10-million-call tail recursion test is intentionally retained unchanged.
The corpus also exercises non-byte-aligned bit patterns, half/single/double
floats, UTF-8/16/32 encoding, arbitrary-width integer segments, record updates,
closures, generic data, structural equality and guarded string-prefix matching.

`validate_reference.py --gleam PATH --stdlib PATH --erlang-bin PATH` runs the
same 490 tests with Gleam 1.19.2 and an unmodified gleam_stdlib 1.0.5 checkout in
an isolated temporary project. It checks the pinned versions and declared test
inventory, calls every listed test, and requires the final execution marker.
The compiler executable and Erlang tools must already be installed; the script
does not download dependencies or change the supplied checkouts.
