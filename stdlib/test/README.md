# Standard library conformance corpus

Run the migrated suite with the Wleam CLI:

```sh
wleam --project stdlib test
```

The source is `gleam_stdlib` v1.0.5, commit
`5b0642a77a38f984db5913548b84668652ca8b5c`. Original files are retained
byte-for-byte in `reference/` with `.reference` appended to their names, so the
compiler does not accidentally analyse a legacy platform fixture.
`migration.json` records original SHA-256 digests, active test names and each
platform-specific migration.

There are **1,545 original test declarations**, **1,538 unique test names**, and
**1,520 active Wasm tests**. Seven pairs of Erlang/JavaScript definitions are
consolidated into one assertion for the Wasm representation. Eighteen tests
inspect objects that exist only in a legacy host: Erlang process identifiers,
references, atoms and improper lists, or arbitrary JavaScript objects and
errors. Their original code is preserved, their disposition is listed
individually in the manifest, and `migration/*.gleam.invalid` exercises the
required rejection of the corresponding old-platform externals. These cases
are not counted as passing Wasm runtime tests.

All active functions explicitly return `Nil` for Wleam's test discovery. Their
original bodies still execute; the added final `Nil` also retains tail-call
stress tests whose upstream functions returned the produced list or value.
The tail-call stress depth remains one million elements. The CLI discovers
functions, runs each test and reports failures; the previous EUnit/JavaScript
runner and its reporting helpers are reference sources only.

WasmGC distinguishes tuples from constructors, floats from integers, strings
from bit arrays, and integer lists from host character lists. Inspection tests
assert these distinctions. The two dynamic-object decoder tests construct
values through `gleam/dynamic.properties`; the map variant reverses insertion
order to test that decoding does not depend on it.

The test count describes collected coverage, not verified execution. A suite
passes only when the runtime report contains all 1,520 active names, no failures
and no skipped or missing tests. The root conformance harness checks this
manifest and reports the actual outcome.
