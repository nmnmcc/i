# Conformance suites

Run from the pinned Devenv environment after building the compiler and runner:

```sh
python3 scripts/conformance.py \
  --compiler target/cargo/debug/i \
  --runner target/cargo/release/ilang-runner
```

`scripts/dev-verify` runs this command before browser tests. The harness requires
every selected tool and suite. Missing binaries, invalid Wasm, compiler crashes,
timeouts, unexpected diagnostics and incomplete test discovery fail verification.
There are no expected failures or skip-on-missing branches.

`cases.json` names ten executable programs, five compile-error programs, three
runtime-error programs and four complete test corpora. Runtime-error programs
must compile successfully, then fail with their expected message and exit code.
Compile-error programs must fail during checking. Returning success without
executing assertions cannot satisfy either contract.

The full corpora require these exact sets of public, zero-argument `*_test`
functions returning `Nil`:

| Corpus | Required tests | Inventory |
| --- | ---: | --- |
| Core language | 490 | `projects/upstream_language/migration.json` |
| Standard library | 1,520 | `../../stdlib/test/migration.json` |
| Unicode 17 official vectors | 120 | `../unicode/corpus.json` and generated sources |
| Additional standard library regressions | 250 | `cases.json` |

The runner's JSON report must contain every expected qualified name exactly once,
with every result passed and summary counts matching. The emitted ABI must list
the same tests. Test filename suffixes do not determine discovery: upstream core
tests include files such as `language/alternative_pattern.i`.

`host_abi` builds an actual iLang library with ten typed exports and seven host
imports. Its companion descriptor must match `projects/host_abi/expected-abi.json`.
The native Rust SDK example exercises the compiled module before it is staged
for the identical browser assertions. This step uses pinned Cargo and a separate
`target/cargo` directory; `--host-probe` can select an already-built
`verify_compiler_abi` executable. An empty list of language tests does not count
as successful host ABI coverage.

`upstream/` preserves the original pinned sources, including their original
platform support files. These are reference inputs, not iLang production
dependencies. `upstream-manifest.json` records SHA-256 checksums verified on every
harness invocation. Migration inventories explain all platform-specific changes
and preserve the original assertions.

i no longer runs these programs on Gleam/Erlang for comparison. i is developed
independently and its syntax diverges from Gleam, so the expected outputs in
`cases.json` and the test corpora are the reference.

Pass `--compare-compiler target/debug/i` to require both Cargo and Blaze
compiler builds to produce identical Wasm bytes and execution results for the
same programs and corpora. Source paths and build modes remain identical during
the comparison. Both compilers use `--no-cache`, so neither can reuse the other
compiler's generated artifact or IR. Timestamps are excluded from the comparison;
test names, captured output and expected-error diagnostics are compared. Compiler
and runner binary digests and the compiler/stdlib/test source fingerprint are
recorded; changing these inputs during a run fails it.

`--case NAME` selects a focused development run and can be repeated. Omit it for
acceptance. The report records the selection, so a focused pass is distinguishable
from a complete pass. `--timeout` controls individual commands and
`--corpus-timeout` allows longer stress suites. `--no-cache` forces regeneration
for focused compiler debugging and is recorded in the report.

Reports are written incrementally to `target/conformance/report.json`, including
the commands, captured output, durations and Wasm digests. They are execution
evidence for that run, not committed performance claims. Successfully validated
modules are copied to `target/conformance/modules/`.

`browser.json` is an array consumed by the browser tests. Each entry contains
`name`, an absolute `wasm` path and exact `stdout`; test corpora also contain
`tests` (qualified ABI export names) and `stderr`. The browser loads the very same
Wasm bytes already executed by Wasmtime. Missing or empty manifests fail browser
verification. A `hostAbi: true` entry runs the typed import/export assertions.
Chromium, Firefox and WebKit must each pass all staged modules.

## Supplemental verification after a build-only change

The full conformance command remains the normal acceptance and CI entry. When a
build-script change only corrects dependency tracking, an existing complete
successful execution report can support a narrower follow-up:

```sh
python3 scripts/conformance-verify-artifacts.py \
  --compiler target/verification/ilang-cargo-final \
  --compiler target/verification/ilang-blaze-final \
  --runner target/verification/ilang-runner \
  --reference-report target/conformance/report.json
```

This verifier deletes each previous Wasm and ABI output, rebuilds without caches,
validates the fresh Wasm and requires its complete SHA-256 to equal the module
already executed by the reference run. It also compares compile-error and
runtime-error diagnostics, ABI descriptors and test inventories. A missing or
changed artifact fails; changed Wasm requires complete runtime conformance.

For test corpora the CLI invokes an explicit artifact-verifier hook instead of a
runtime. The hook reports `mode: "artifact_equivalence"` and
`runtime_tests_executed: 0`; it never manufactures a passed test report. Corpus
execution coverage is inherited from the identical previously executed bytes.
The supplemental JSON records its reference report digest, compiler/runner
digests, source fingerprint and the verifier's own digest. It cannot use a
partial, failed or differently inventoried reference report.
