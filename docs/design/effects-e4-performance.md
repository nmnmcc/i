# E4 checker performance repair, 2026-11-15

Branch **lane/e4**. Production source is `7575ae5a525c703c0c117bb8ed8dd1e76f316406`, atop `e975c5e93`; main through `7c0652fad` is merged. All requested gates and exact corpus preservation checks pass. The earlier e975 LANDABLE claim and its four performance-censor justifications are withdrawn. No old observation phase is restored in this repair.

## Cause and unchanged behavior

In `crates/ilang-check/src/types/display.rs`, `Norm::steps` eliminated one independent service argument per pass. Every pass entered `Norm::meet_forms`, whose `.position` search scanned all preceding service forms for each member. Directed service variance made these arguments eligible for simplification. N passes each doing N squared comparisons caused cubic work: roughly 4.3 billion preceding-entry comparisons for one 2048-member simplification. Memory stayed about 118 MB.

`perf` is unavailable. Temporary timing spans first excluded solver operations on the 2048-entry program (under a second of its roughly 70 seconds). Targeted 512-entry spans measured 2165.974 ms in `simplify_steps`, including 994.923 ms in `meet_forms`, 150.577 ms in `steps`, and 2,992,529 recursive `norm` calls. Inclusive spans overlap. All instrumentation is removed from production.

A second profile sampled the supplied unmodified old e4 executable directly, by interrupting only the profiler’s own child and reading its instruction pointer. **5278 of 6503 samples (81.16%)** fall in `Norm::env_in`, which contains the inlined `meet_forms` lookup loop. The next solver symbols account for about 1% each. The profiled process completed in 72.274 seconds, with 70.327 user seconds; its stdout exactly equals the clean old-e4 run. This profiled time is separate from the clean benchmark table.

The fix indexes forms by constructor and equal value set, retaining the first output position and input merge order. It batches consecutive extremum substitutions only for variables with one singleton-group occurrence; naming substitutions, shared variables and nontrivial union/intersection members retain their barriers. Indexing alone reduced 2048 entries to 13.118 seconds, which was insufficient; batching removes the remaining repeated passes. The service index and independent batch are linear, apart from existing sorting. Remaining whole-checker growth on this family is at most quadratic, like main.

Spec §4.7 says: “A type is made simpler before it is shown: equal parts are put together, and type variables that have no effect on the meaning are taken out. This never changes which programs are correct or what they do.” This repair changes implementation work, with no new language rule, accepted/rejected result, diagnostic or runtime output.

The new `independent_service_arguments_are_simplified_in_one_step` regression asserts that 2048 arguments simplify in one recorded step containing 2048 substitutions, and checks the exact resulting type. It uses no elapsed-time threshold.

## Measurements

All measured compiler processes use `taskset -c 2`, `BINARYEN_CORES=1`, and the heavy wrapper. Main is compiler `7c0652fad` at `/home/agent/Programming/i/.rd/cache/docs-bin/i-5e08939f`; old e4 is `e975c5e93` at the sibling `i-e4`; fixed is this worktree’s `./target/release/i`. No executable is copied to run it. Entries below are wall/user seconds.

Scaling command: `taskset -c 2 BINARY check --types --offline --project tests/review/cases/review-k4/perf/wide_generic_N`.

| Entries | Main before | Old e4 | Fixed |
|---:|---:|---:|---:|
| 256 | 0.313/0.230 | 0.713/0.632 | 0.452/0.393 |
| 512 | 0.385/0.322 | 2.040/1.939 | 0.514/0.446 |
| 1024 | 0.666/0.583 | 10.319/10.085 | 0.752/0.676 |
| 2048 | 1.761/1.631 | 70.086/67.940 | 1.670/1.520 |

Before, the old e4 user-time doubling ratios are 3.07, 5.20 and 6.74, approaching cubic growth. After, this cubic term is removed and the measured residual follows main’s quadratic trend.

The three-run comparison uses a fresh process each time after one explicit warmup for **each** compiler. Generic cases use `check --types`; stress cases use `test --opt 0`. Every command adds `--offline --project tests/review/cases/CASE`.

| Case | Main, three runs | Fixed, three runs | Median wall ratio |
|---|---|---|---:|
| wide_generic_1024 | 0.688/0.615, 0.705/0.622, 0.678/0.583 | 0.752/0.676, 0.772/0.682, 0.759/0.675 | 1.10x |
| wide_generic_2048 | 1.755/1.612, 1.809/1.666, 1.734/1.584 | 1.670/1.520, 1.643/1.505, 1.621/1.496 | 0.94x |
| stress_wide_1024 | 0.712/0.620, 0.695/0.591, 0.760/0.647 | 0.709/0.618, 0.720/0.602, 0.713/0.629 | 1.00x |
| stress_wide_open_1024 | 0.944/0.811, 0.928/0.794, 0.931/0.822 | 0.846/0.751, 0.872/0.765, 0.845/0.740 | 0.91x |

Every measured fixed run is within 1.5x even the fastest corresponding main run. All four cases also pass fresh check/O0/O2 corpus phases within their original 10/60-second CPU bounds.

Earlier first-test fixed observations were 2.428/1.239 and 2.437/1.219 seconds; subsequent fresh processes took about 0.7 seconds. These observations are retained in the raw measurement logs. `ilang-host/src/lib.rs` keys compiled modules and functions by executable digest, so a rebuilt executable starts with different cache entries. Matched empty-store measurements below isolate this startup cost without deleting any shared cache or moving a binary; each process gets its own new `I_CACHE_DIR`.

| Case, cold store | Main, three runs | Fixed, three runs |
|---|---|---|
| stress_wide_1024-cold | 1.207/1.047, 1.248/1.082, 1.244/1.079 | 1.204/1.044, 1.214/1.041, 1.209/1.057 |
| stress_wide_open_1024-cold | 1.441/1.251, 1.454/1.290, 1.473/1.251 | 1.440/1.257, 1.417/1.254, 1.391/1.236 |

The clean old-e4 stress reproductions took 10.288/10.075 and 10.530/10.300 seconds. Raw records include wall/user/system/RSS and exact argv in `target/e4-perf/before-scaling.json`, `target/e4-perf/after-measurements.json`, `target/e4-perf-final/after-measurements.json`, and `target/e4-perf-warm/after-measurements.json`.

## Fresh corpus preservation

All **1973** cases run with `--jobs 6`, `BINARYEN_CORES=1`, UPDATE=1 and `--keep-bounds`: **554 rejected checks, 1419 accepted checks, no checker timeout, two existing XFAILs, no runner failures**. The stronger exact comparison with e975 catches one extra O2 timeout, `review-t1/stress-keys-2000`, at its saved 10-second bound. This is not hidden by the runner’s success.

The paired same-bound isolated run observes main O2 timing out (9.439 user + 1.859 system seconds), while fixed O2 completes in **5.773 wall / 5.506 user + 0.186 system seconds**. All fixed check/O0/O2 observations exactly equal e975. Only that failed test2 phase is replaced by the fresh same-binary observation; every completed full-run phase remains unchanged. No old passing phase is copied back.

The final exact preservation comparison is **1971 identical outcomes plus two corrected carried displays**, with no other difference from e975. Both corrected checks already had the new type in the unmodified old-e4 executable: `app/main.run` is `(() <- N -> ()) <- N -> ()`; the old copied main record instead showed an independent `(Fail(E), ..N)` carrier. They are D1 in [the corpus audit](effects-e3-e6-corpus.md), under R1’s single need row, and are not a new behavior of this optimization. Every bound remains unchanged.

The full raw snapshot has all 1973 observations from the same release digest; the extra phase retry has that same digest. Reconstructing the full snapshot with only that fresh phase exactly equals every final saved outcome. Only the two fingerprint metadata fields are restored for 1969 otherwise-identical files, with byte-exact serialization checks; all four wide-case files retain fresh producer metadata. Existing baseline resource observations remain exactly e975; no old phase is restored. The 395 saved requirements (two baseline XFAILs) and all 34 exact semantic/Scheduler guards pass. Relative to pinned main `5307a664e`, the existing integration audit is 975 identical / 974 display / 24 previously quoted semantic cases / zero unexplained; D1 has 847 cases.

Release SHA256: `795bb7f8e15dbbd9369774cef5e898d0dec1e7b8c1eb13862ff1b118fdbb96f8`. Production source stays byte-identical to `7575ae5a5` throughout all final gates.

## Checking commands and results

`H=/home/agent/Programming/i/.rd/cache/heavy`; Cargo/tooling uses `direnv exec /home/agent/Programming/i`. One heavy command runs or waits at a time. Defaults: `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_RELEASE_DEBUG=0 CARGO_INCREMENTAL=0 RUST_TEST_THREADS=2 BINARYEN_CORES=1`. The snippet script is direct because it acquires H internally. Static observation audits do not launch a compiler.

The full Rust driver builds all workspace tests and runs all **118 binaries**, six at a time, with two test threads; the value-types property’s internal pool is bounded to two CPUs. No test is filtered. This run sets DEV/TEST `OPT_LEVEL=1`, `DEBUG_ASSERTIONS=true`, and `OVERFLOW_CHECKS=true`, as recorded in its log; the complete checker unit suite also passed at default opt-level 0 (81 passed, four existing ignores). Whole spec passes 536/536 at both language opts; value-types 31/31 including reverse order/recovery; std passes 2171/2171 at both opts. The six existing Rust ignores, 29 pending spec cases, and one intentional macro-example skip are unchanged.

The orchestrator was held between steps while its running Rust suite completed, so the isolated retry could run next without overlapping another heavy command. No running test was paused. The command ledger is `target/e4-validation-events.jsonl`.

| Command | Result | Log |
|---|---|---|
| `/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo build --release --locked -p ilang-cli` | exit 0 | `target/e4-gate-perf-release-final.log` |
| `/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo test -p ilang-check --lib --locked` | exit 0 | `target/e4-gate-perf-checker-unit.log` |
| `/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i python3 -u target/e4-perf/final-measure.py` | exit 0 | `target/e4-gate-perf-compare-final.log` |
| `UPDATE=1 BINARYEN_CORES=1 /home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i scripts/review-corpus run --binary ./target/release/i --keep-bounds --jobs 6 --case-list target/e4-full-corpus-cases.json` | exit 0 | `target/e4-gate-perf-full-corpus.log` |
| `/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo check --workspace --all-targets --locked` | exit 0 | `target/e4-gate-perf-workspace-check-perf.log` |
| `E4_TEST_LOG_DIR=/home/agent/Programming/i/.rd/e4/target/e4-perf-rust-logs /home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i python3 -u target/e4-ptest.py --workspace` | exit 0 | `target/e4-gate-perf-workspace-tests-perf.log` |
| `I_BIN=/home/agent/Programming/i/.rd/e4/target/release/i /home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i python3 docs/scripts/run-examples-test` | exit 0 | `target/e4-gate-perf-docs-runner-contract-perf.log` |
| `I_BIN=/home/agent/Programming/i/.rd/e4/target/release/i /home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i python3 docs/scripts/run-examples.py --jobs 2 --no-placeholders` | exit 0 | `target/e4-gate-perf-docs-examples-perf.log` |
| `/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i ./target/release/i test --project std --opt 0` | exit 0 | `target/e4-gate-perf-std-perf-o0.log` |
| `/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i ./target/release/i test --project std --opt 2` | exit 0 | `target/e4-gate-perf-std-perf-o2.log` |
| `/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i ./target/release/i test --project examples/macro-tests --opt 0` | exit 0 | `target/e4-gate-perf-examples-macro-tests-perf-o0.log` |
| `/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i ./target/release/i test --project examples/macro-tests --opt 2` | exit 0 | `target/e4-gate-perf-examples-macro-tests-perf-o2.log` |
| `/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i ./target/release/i test --project bench --opt 0` | exit 0 | `target/e4-gate-perf-bench-perf-o0.log` |
| `/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i ./target/release/i test --project bench --opt 2` | exit 0 | `target/e4-gate-perf-bench-perf-o2.log` |
| `/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i scripts/test-generation --mode short --binary ./target/release/i` | exit 0 | `target/e4-gate-perf-generation-perf.log` |
| `/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 | `target/e4-gate-perf-clippy-perf.log` |
| `/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo fmt --all --check` | exit 0 | `target/e4-gate-perf-rustfmt-perf.log` |
| `/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i ./target/release/i fmt --check bench std lib examples` | exit 0 | `target/e4-gate-perf-ifmt-perf.log` |
| `direnv exec /home/agent/Programming/i scripts/check-snippets` | exit 0 | `target/e4-gate-perf-snippets-perf.log` |
| `/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i python3 -I scripts/check-snippets-test` | exit 0 | `target/e4-gate-perf-snippets-tests-perf-rerun.log` |
| `/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i python3 -I scripts/qa/corpus-test` | exit 0 | `target/e4-gate-perf-corpus-runner-tests-perf.log` |
| `/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i python3 docs/scripts/check-placeholders-test` | exit 0 | `target/e4-gate-perf-docs-placeholders-test-perf.log` |
| `/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i python3 docs/scripts/check-spec-links-test` | exit 0 | `target/e4-gate-perf-docs-citation-test-perf.log` |
| `I=/home/agent/Programming/i/.rd/e4/target/release/i /home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i bash scripts/build-abi-fixture.sh` | exit 0 | `target/e4-gate-perf-abi.log` |
| `CHECKS_SKIP_CORPUS=1 ILANG_BROWSER_PORT=4194 /home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i scripts/checks js` | exit 0 | `target/e4-gate-perf-js.log` |
| `/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo test --doc -p ilang-check -p ilang-wasm -p ilang-host -p ilang-cli --locked` | exit 0 | `target/e4-gate-perf-doctests.log` |
| `BINARYEN_CORES=1 /home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i python3 -u target/e4-perf/isolate-keys.py` | exit 0 | `target/e4-perf/keys-isolated.log` |

Additional executed preparation and preservation commands:

| Command | Result |
|---|---|
| H `python3 -u target/e4-perf/measure.py` | Eight baseline scaling runs, all exit 0. |
| H `cargo build --release --locked -p ilang-cli` for timing probes 1–5, followed by pinned E4_PERF=1 checks at 2048/512 | Builds/checks pass. First probe needed periodic output because the CLI exits without main-thread TLS destruction; subsequent probes correct this. All probes removed. |
| H indexed-lookup-only release build and pinned 2048 check | Pass, 13.118s. Initial `/usr/bin/time` launch fails because it is absent; Python wait4/resource counters replace it. |
| H `cargo test -p ilang-check --lib --locked independent_service_arguments_are_simplified_in_one_step` | 1 passed, 84 filtered, 0.02s; this focused run precedes full unfiltered checker/workspace tests. |
| H release build with both repairs; H `python3 -u target/e4-perf/after-measure.py` | Build passes; 30 measured processes exit 0, including the retained cold-start observations. |
| H `scripts/review-corpus run --binary ./target/release/i --keep-bounds --jobs 1 --case-list target/e4-perf-cases.json`, UPDATE=1 BINARYEN_CORES=1 | All four cases pass fresh check/test0/test2 at saved bounds. |
| `direnv exec /home/agent/Programming/i rustfmt --edition 2024 crates/ilang-check/src/types/display.rs` | Pass. |
| Parent/own-child PTRACE_SEIZE/GETREGS readiness check | Pass; only that temporary child is terminated. |
| H `python3 -u target/e4-perf/sample-old.py`, then H `python3 -u target/e4-perf/warm-measure.py` (sequential inside the Rust gate before build) | Old profile exits 0, stdout byte-identical; 48 warm/cold/scaling processes exit 0. Exact commands/cache dirs and resource usage retained. |
| `python3 target/e4-perf/compare-corpus.py` | Initial exit 1 identifies the extra O2 timeout; after its fresh retry, exit 0, 1971 exact/2 corrected displays. |
| `python3 target/e4-perf/compare-corpus.py --restore-fingerprints` | Restores metadata only for proven identical files: 1968 initially, one after the fresh retry; byte-exact outcome/bound preservation. |
| `python3 target/e4-audit-saved-requirements.py` | 395 applicable requirements, 0 failures, two existing XFAILs; matcher AST unchanged. |
| `python3 target/e4-exact-semantic-guard.py` | 34/34 exact semantic and Scheduler guards, 0 failures. |
| `python3 target/e4-current-corpus-audit.py --base 5307a664e` | 975 fingerprint-only, 714 check-display, 260 runtime wording, 24 previously explained semantic; no timeout or unexplained row after the retry. Read-only. |
| `python3 target/e4-current-display-analysis.py` | All 974 display cases classified, no REVIEW group; D1 847, other groups unchanged. |
| Initial direct `scripts/check-snippets` | Exit 1: the new Rust format string `S{i:04}` was parsed as an i constructor-list example. Add its precise `not_source` inventory entry, following existing rendered-name entries; compiler/test source and every existing inventory entry stay unchanged. |
| Direct `scripts/check-snippets`, then H `python3 -I scripts/check-snippets-test`, after that inventory classification | Both pass: 5216 extracted literals/blocks, 48 .is files/101 macro fragments, the same four future tracks; all 10 runner self-tests pass. |
| `git diff --check` and final raw-observation/source/binary proof | Pass. |

No requested gate is omitted. The full Rust suite is not additionally repeated at opt-level 0; all its assertions and overflow checks remain enabled at opt-level 1, and all checker unit tests also pass at opt-level 0. `perf` is unavailable, so direct own-child sampling and timing counters supply the profile. No timing test, new skip/ignore, removed assertion or speculative expectation update is introduced.
