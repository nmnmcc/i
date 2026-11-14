# Effects landing verification, 2026-11-14/15

Worktree `.rd/e4`, branch `lane/e4`. Owner decisions of 2026-11-14 control;
main `9b8061137`, final-AST main `5e08939f0`, and documentation main
`05e39a6ae` are merged. This report is in progress; no LANDABLE claim yet.
Every cargo/check/runtime/corpus command below uses
`H=/home/agent/Programming/i/.rd/cache/heavy` and the shared environment
`direnv exec /home/agent/Programming/i` where tools require it. Only one H
command runs or waits for this worker at a time. Defaults:
`CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_RELEASE_DEBUG=0 CARGO_INCREMENTAL=0
RUST_TEST_THREADS=2 BINARYEN_CORES=1`.

## Diagnosed preparation runs

| Command (through H unless noted) | Result and action |
|---|---|
| `cargo check --workspace --all-targets --locked` | Before FA: three diagnosed merge-port failures, fourth passed. After FA: three diagnosed direct resolver/dev-oracle/metadata test ports, fourth passed. After the run/external repair: first failed on the new test's BuildOptions construction, second passed. |
| `cargo build --release --locked -p ilang-cli` | Passed after FA and after run/external changes; latest rebuild passed before ABI fixture generation. |
| `./target/release/i check --project std` | Initial 9 errors, then 2; restored main AnyHeaders, independent stream callback F and concrete sink needs. Third passed; passed again after run repair. |
| `python3 -u target/e4-ptest.py --workspace` | Gate-equivalent build/run of 118 Rust binaries, including the whole spec suite: 101 passed, 17 failed. All failures diagnosed; reruns below. This is not a green gate result. |
| `cargo test -p ilang-cli --test rest_services --test r2_runtime --locked --no-fail-fast -- --test-threads 2` | R2 9/9 passed, including 200 seeded replays at O0/O2 and cleanup/accounting. Existing rest 4/4 passed; new external-Fail test exposed missing ABI error slots. |
| `cargo test -p ilang-cli --test rest_services --locked -- external_host_errors_raise_through_the_callers_fail --exact` | Passed both O0/O2 after including external errors in ABI metadata. |
| `UPDATE=1 cargo test -p ilang-check --test projects --locked` | 3/3 passed; reviewed only external_types golden change against §2.10.12/§10.1. |
| `./target/release/i check --types --offline --project target/e4-run-contract-real` | Passed exact Runner-only run and Fail(ScheduleError)+Runner test contracts. Two expected unused-service warnings. |
| `E4_TEST_FILTER=... python3 -u target/e4-ptest.py --workspace` | Focused ten binaries: generic_services 10, coalescing 7, projects 3, query 24, syn3 39, syn3_lower 23, lisp_syntax 51 all passed. body_walk3, row_schemes and provider guard exposed the remaining ordinary-Fail visit order, exact hole witness and inhabited-meet expectations; fixed with R1/R8 explanations. |
| `python3 -u target/e4-provider-probe.py` | Checked all fourteen real manifests. All concrete provider-subtyping errors retained; b1 loses only false empty-method summary, m32 adds duplicate-key E0225. |
| `UPDATE=1 scripts/review-corpus run --keep-bounds --jobs 6 --case-list target/e4-provider-cases.json --binary ./target/release/i` | 14 selected, all rejected, no requirement failures. Bounds unchanged; unchanged fingerprint fields restored independently, never whole files. |
| `UPDATE=1 cargo test -p ilang-check --test types --locked` | 5/5 passed in 93.74s. Review found ordinary-difference display was affected; restricted fix to needs. Preserved Source empty-meet negative using a required value field, per §4.1. Focused rerun pending. |
| `I=$PWD/target/release/i bash scripts/build-abi-fixture.sh` | Passed; compiled, external-Fail O0/O2, R2 O0/O2 and all R2-law artifacts regenerated and validated as ABI 5. Hand-written sources retain ABI 3. |
| `python3 -I scripts/qa/corpus-test` (direct, no compiler invocation) | 5/5 passed. |
| `./target/release/i fmt --check bench std lib examples` (direct formatter) | Reported 30 files; formatted them without changing assertions, then passed. |
| `direnv exec /home/agent/Programming/i cargo fmt --all --check` (direct formatter) | Passed at preparation checkpoint; final rerun required. |
| `git diff --check` (direct) | Passed repeatedly. |

Test rewrites, diagnostic changes and quoted controlling paragraphs are in
[the corpus audit](effects-e3-e6-corpus.md). The authoritative current gate
results and final inventory will be appended below after their runs.

## Current runtime gates

`./target/release/i test --project std --opt 0` and `--opt 2`, each through
H with Binaryen limited to one core: **2,171 passed, 0 failed at each level**.
Logs: `target/e4-gate-std-o0.log`, `target/e4-gate-std-o2.log`.

The focused rerun passes all fourteen provider projects with exact messages,
all 19 macro-walker cases, all five type tests (91.37s), and 48/49 row-scheme
cases (211.11s). Only the exact local annotation-hole witness display differs;
that assertion is retained while tracing. The separate covariant-upper-bound
case and the new two-parameter inhabited-meet control both pass.

`E4_WITNESS_TRACE=1 cargo test -p ilang-check --test row_schemes --locked --
handler_witnesses_include_exact_bounds_of_annotation_holes --exact --nocapture`
confirmed equal Int bounds but a later polarity substitution in the mask;
the retained assertion failed as expected before the repair. Temporary trace
code is removed. `cargo test -p ilang-check --test row_schemes --locked --
handler_` now passes all 8 selected tests (38.46s), including the original exact
assertion, open covariant upper bound, and new incompatible-value control.
