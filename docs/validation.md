# Validation record — 2026-10-29

This record distinguishes executed checks from the test infrastructure. Raw
reports are retained under `target/verification`, `target/conformance` and
`target/browser`; the commands needed to reproduce them are in the
[conformance guide](../tests/conformance/README.md).

## Fixed inputs

The language and stdlib baselines are Gleam 1.19.2 and gleam_stdlib 1.0.5.
The environment fixes Rust 1.99.1, Wasmtime 50.0.0, wasm-tools 1.262.0, Unicode
17.0.0 and Devenv 2.4.0. Blaze uses the pinned 2026-10-27 distribution and its
documented cache correctness backport; see [development](development.md).

The complete runtime acceptance used executables copied to independent
verification paths. `target/verification/frozen.json` records their hashes:

| Executable | SHA-256 |
| --- | --- |
| Cargo release compiler | `b229e6abc74abc13e6cb1e11f26a2924d5a3830ee33cebdd84647e4ba4404f22` |
| Blaze compiler | `17e2431a4d05a5a784e6b18d7a971caf5c325a0604d19bfafe4668509eb562cf` |
| Parallel release Wasmtime runner | `73689bc7779b2de0f46261f2de75c2c1f8f3b4f4fd346d11ee6ce4165b7a9356` |
| Native SDK integration probe | `249ddd0954b5d58574ef3d28e6d25d149d295761d766687851dee29091a13f7c` |

The native report's source fingerprint is
`48de976a9f6a1e8ad195252c67041efcd59d59faddd75f7f8272b4bf54ab0288`.
That initial fingerprint excluded all directories named `build`, including the
unchanged vendored Rust module `src/build`. The subsequent comparison uses the
corrected fingerprint
`99f233dfd88b26cfbf1dd67739ad2ae6400a2eb60b1573b07ac4cc4dab1a61cc`,
which excludes generated package-root build directories only. Compiler binaries
and language/runtime sources did not change between these two fingerprints;
the harness correction and scope are recorded explicitly in `frozen.json`.

## Executed checks

| Check | Result | Evidence |
| --- | --- | --- |
| Rust workspace tests | 147 passed, 0 failed, 0 ignored | `target/verification/cargo-checks.json` and its retained logs |
| Retained upstream Rust workspace | 4,513 passed, 0 failed, 0 ignored | Same verification record |
| Strict workspace Clippy and both workspace formatting checks | Passed | Same verification record |
| Complete release workspace build | Passed | Same verification record |
| Blaze default workspace tests | 123 passed, 0 failed, 0 ignored or filtered | `target/verification/environment-final-blaze-tests.json` |
| Native compiler conformance | **27/27 passed** | `target/conformance/report.json` |
| Cargo/Blaze uncached compiler comparison | **27/27 passed; identical Wasm, error diagnostics and behavior** | `target/conformance/blaze/report.json` |
| Final complete Chromium/Firefox/WebKit run | **72/72 groups passed; zero failed, skipped or flaky** | `target/browser/results.json` |
| Node SDK unit tests | 14/14 passed | Final Node test output |
| Release archive relocation, sibling runner and packaged SDK | Passed | `target/release/verification.json` |

The native conformance run executed exactly **2,380 named test functions**:
490 upstream core language tests, 1,520 stdlib tests, 120 Unicode vector groups
and 250 additional stdlib regressions. It also checked ten standalone programs
against the fixed Erlang oracle, eight intentional compile/runtime failures,
legacy-target migration diagnostics, generated Unicode data, and the production
Wasm value-runtime tests. Test names, per-test statuses and aggregate counts must
agree with the source inventories; an empty runner result fails the harness.

The comparison executed all 2,380 functions with each compiler build and required
byte-for-byte identical Wasm for every compiled fixture and corpus. Both builds
used `--no-cache`; one compiler could not satisfy the comparison by loading the
other compiler's cached artifact or IR. Expected-error diagnostics and captured
runtime output also matched. Source and binary fingerprint checks passed at the
end of the complete run.

| Corpus | Identical Wasm SHA-256 |
| --- | --- |
| Core language | `5fcee7c2d63f0d7e3a9c0fee0a4057213528aa45838e8a559cdc691072bab509` |
| Standard library | `f3d1abd4d598bf4e99d229cfb660f71607c9a1fc2858f2c95cb706bcbad94993` |
| Unicode 17 | `21911913cfd6c77050b489b5932f76f0dbc2aa4002d4c96dac49fbc33d7edb7e` |
| Additional regressions | `ab17efa2c44bc67f813a741fe1d9c4e7f58a8f2c9d4efda8a084c802f9486983` |

The actual compiler-generated host ABI library passed **44 native SDK checks**,
including seven typed imports, ten exports, large positive/negative integers,
Unicode/NUL strings, nine-bit nonzero-offset slices, opaque reference identity,
GC and recovery after guest and host exceptions. These checks exercise compiled
Gleam code in addition to the SDK's synthetic fixtures.

Each browser executed all **2,380 named corpus functions**, ten standalone
programs, the real compiled host ABI library and nine adapter regression groups.
The 72 Playwright groups passed in 53 seconds using Chromium 156.0.8078.4,
Firefox 157.0 and WebKit 27.2, with Playwright 1.64.1 and Node 24.21.0. Engine
versions are retained in `target/browser/versions.json`. Browser tests consumed
the copied native artifacts; they did not rebuild separate browser programs.

The 1,520-test native stdlib run completed in about **506 seconds**. Its largest
individual costs were the large randomized dictionary test (about 188 seconds)
and the million-element shuffle test (about 139 seconds). A separate 120-second
shuffle probe timed out before the full run demonstrated successful completion.
The full corpus deadline was not increased and the stress sizes were retained.
These durations describe this verification run, not a controlled performance
comparison. [Performance measurements](performance.md) use isolated snapshots
and separately recorded samples.

The randomized dictionary test also copies and traverses a reference property
list, and hosts receive different random inputs. These verification durations do
not isolate dictionary throughput or establish a GC bottleneck. No GC tuning was
applied on the basis of these timings.

The original upstream sources and licenses remain preserved, with 115 source
files checked against the pinned SHA-256 inventory. Platform-specific test
adaptations and the dictionary decoder's unspecified iteration order are
documented in the migration inventories linked from
[compatibility](compatibility.md).

## Build dependency tracking correction

The subsequent iteration benchmark exposed unnecessary Cargo rebuilds:
`build.rs` watched a nonexistent `vendor/gleam/licences/Cargo.toml` and the
stdlib's generated build directory. The correction watches existing manifests,
source directories and license files. Repeated unchanged builds and a write to
`stdlib/build` no longer trigger recompilation. Evidence is retained in
`target/verification/watcher-results.json`; measured iteration timings are in
[performance](performance.md).

After this build-only correction, the Rust workspace again passed all 147 tests,
strict Clippy, formatting and release building
(`target/verification/cargo-checks-final.json`). Blaze again passed its 123
default workspace tests
(`target/verification/environment-watchfix-blaze-tests.json`). The vendored Rust
sources did not change; their 4,513-test execution evidence remains applicable.

The corrected compiler snapshots in `target/verification/frozen-final.json` are:

| Executable | SHA-256 |
| --- | --- |
| Final Cargo release compiler | `6a080f4e337f2c90a408f6d90738b17667da3adb5c53c3ed1a3b1532392ca8c0` |
| Final Blaze compiler after its test build | `a84d18a380c303cb8c5117a34539ac6fbd24af3491b66ab8a3084acaf84aa661` |

Their verification source fingerprint is
`ce297391da24432ee77050af13b76a08f9ddce375a1004e30cb4854859aa79b4`.
The Wasmtime runner and native SDK probe remain unchanged.

Supplemental verification passed **47/47 checks**
(`target/verification/artifact-equivalence-final.json`). Each new compiler
rebuilt all 18 Wasm artifacts after deleting the old outputs and disabling
caches. Every byte matched the artifacts already executed by Wasmtime and the
three browsers. Error diagnostics and ABI/test inventories also matched; six
intentional runtime-error probes were executed again.

The 2,380 corpus test bodies were **not executed again** in this supplemental
step. Its report explicitly records zero corpus runtime executions and links the
complete successful execution report by digest. The proof is fresh compilation
to identical executed bytes, not a new runtime pass or a successful empty test
run. Any differing byte or diagnostic would have required complete conformance
again. The normal CI workflow still runs the full execution suites.
