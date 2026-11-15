# Effects landing verification, 2026-11-15

Branch **lane/e4**. Production source is `7575ae5a5`, atop `e975c5e93`,
with main through `7c0652fad` merged. **All requested final gates pass.**
The earlier e975 LANDABLE claim and four performance-censor justifications
are withdrawn. The cubic checker defect is repaired; all four wide cases
have fresh passing check/O0/O2 observations at their saved bounds.

The final report commit changes documentation and observation metadata;
production source and release binary stay unchanged throughout verification.
Release SHA256:
`795bb7f8e15dbbd9369774cef5e898d0dec1e7b8c1eb13862ff1b118fdbb96f8`.
No test or assertion is deleted or weakened, and no skip/ignore is added.

## Final results

| Gate | Result |
|---|---|
| Workspace all-targets check and clippy `-D warnings` | Pass |
| Full workspace Rust tests | All 118 binaries pass, including all 31 value-types tests and the reverse-order/recovery properties |
| Whole spec | 536/536 at both O0 and O2; 29 existing pending cases unchanged |
| Std | 2171/2171 at O0 and O2 |
| JavaScript verification | 123 Node and 69 browser checks, no new skip; Chromium/Firefox/WebKit |
| ABI fixture generation | Built and Wasm-validated from this same release compiler; ABI5 generated artifacts, hand-written ABI3 fixtures retained |
| Short generation | 32 attempts, 20 accepted/12 model rejections; 64 permutations/32 annotation removals; no failures |
| Documentation examples | All 110 pass exact type/output/error assertions; runner contracts 14/14 |
| Macro examples and bench | Both language opts pass; one existing intentional macro-example skip remains |
| Rust and i formatting | Pass; i checks bench/std/lib/examples |
| Snippets | Direct script passes its exact diagnostic checks and existing future tracks |
| Doctests and verification-script self-tests | Pass |
| Full corpus | All 1973 selected, 554 rejected checks/1419 accepted checks, no checker timeout, same two registered XFAILs |
| Exact preservation against e975 | 1971 byte-identical outcomes plus only two corrected formerly carried check displays; no new acceptance/error/runtime difference |
| Semantic and requirement guards | 34 exact semantic/Scheduler cases; all 395 applicable requirements, no new failure |

All build/check/runtime/corpus commands use
`/home/agent/Programming/i/.rd/cache/heavy`, one command at a time. Cargo and
tooling use `direnv exec /home/agent/Programming/i`. The snippet script runs
directly because it acquires the wrapper internally. The full Rust run uses
opt-level 1 with debug assertions and overflow checks explicitly enabled,
retains all tests, and bounds the long property worker pool to two CPUs.
The entire checker unit suite also passes at default opt-level 0.
The six existing Rust ignores and all existing pending cases remain unchanged.

Every command, its result/log, before/after growth, and three-run pinned
main/fixed timing table is in
[the performance report](effects-e4-performance.md). The complete corpus
rulings and 919 original plus 65 additional display judgments remain in
[the corpus audit](effects-e3-e6-corpus.md).

## Fresh observations and the extra isolated retry

The four wide cases replace six previously spliced and four carried phases
with fresh passing observations at their original 10/60-second bounds.
The complete run and earlier isolated jobs-1 run both pass all twelve phases.

The stronger exact audit catches one additional full-run O2 timeout:
`review-t1/stress-keys-2000`, bound 10s. Its paired isolated run observes main
also timing out, while fixed O2 passes in 5.773 wall seconds / 5.692 CPU
seconds. All fixed outcomes exactly equal e975. Only the failed test2 phase
is replaced by that fresh same-binary observation; no old phase is restored.
Every completed full-run phase is retained.

All 1973 initial raw observations carry the release digest above, as does
the fresh retry. Reconstructing this actual run plus that fresh phase exactly
matches every final saved outcome. Only fingerprint metadata is restored in
1969 otherwise-identical files, with byte-exact checks; the four wide files
retain fresh producer metadata. All bounds are unchanged. The existing
baseline resource observations remain exactly e975 and are not claimed as
newly passing phases.

No requested gate is omitted. The full Rust suite is not additionally repeated
at opt-level 0; all assertions and overflow checks remain enabled at opt-level 1.
`perf` is unavailable; direct own-child instruction sampling finds 81.16% in
`Norm::env_in` (inlined `meet_forms`), supported by timing counters and the
2048-entry operation-count regression. No flaky timing assertion is added.

## Historical preparation (the e975 performance claim is withdrawn)

The dated checkpoints below preserve the earlier command history. Statements
about splicing or classifying the four wide cases as resource censors are
superseded by the fresh results above and were not sound landing evidence.
The old phases are not reused in this repair.


The following dated checkpoints preserve every earlier command and diagnosed
failure. Their in-progress statements are historical; the final results above
supersede them.


Worktree `.rd/e4`, branch `lane/e4`. Owner decisions of 2026-11-14 control;
main `9b8061137`, final-AST main `5e08939f0`, and documentation main
`7c0652fad` are merged. This report is in progress; no LANDABLE claim yet.
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

The final gate includes **all** value_types tests, including the reverse-order
and recovery property: this lane changes types and the latest brief requests
the full gate steps. That binary uses CPU affinity for two available CPUs, so
its own available_parallelism worker pool cannot multiply the six gate workers
by sixteen. Earlier preparation runs kept the older explicit skip; the final
run will supersede that limitation without adding an ignore or changing a test.

The complete Rust gate finished **118 binaries: 114 passed, 4 failed**.
Value-types passes all 31 tests, including the full reverse-order spec and
standard-library checks (2,081.32s, two CPUs). Whole spec passes 533/535 at both
levels with 29 existing pending cases: typed_final_fresh_scope reproduces D16,
and keep_env needs its actual Fail(Never) continuation annotation. The other
three failed binaries are the removed frame-kind-4 mask, the historical macro
oracle's deleted synthetic N%E carrier, and raw external metadata's newly
explicit empty errors list. Their exact assertions are retained and migrated
with the quoted E3/ABI paragraphs. These prepared fixes still need their reruns.
All four requested crates' doctests pass. examples/macro-tests and bench pass at
both levels. Short generation passes; its 8-seed counts and full report are in
`target/e4-gate-generation.log`. JS is Node122/123; browsers were not reached
because the Node law failed. Clippy's one fixture-initializer warning and the
obsolete E3/E6 snippet registrations are patched for rerun.

At 62af4eac4 the targeted native spec cases typed_final_fresh_scope,
task_run_error_after_resumes, keep_env and task_run_supplied_services all pass
at O0/O2. Exact macro-oracle, control-record and external-metadata tests pass.
Workspace/all-targets clippy passes with -D warnings. Direct snippets pass:
5,158 extracted blocks/literals, 4 existing future-track entries; Lisp check
covers 48 .is files and 101 macro fragments. Release rebuild and shipped std
check pass. The vendored check initially failed 15 cascading errors from its
stale callback-shaped Sink marker; restoring the specified ordinary (N)->Nil
marker makes the second check pass without a test change. ABI regeneration
and complete JS rerun follow; the full corpus has not run yet.

Final runtime ABI regeneration passes. Complete JS gate passes: Node 123/123,
browser 69/69 (Chromium, Firefox, WebKit), with no skips. All R2 laws run at
O0/O2, including the new Sample/Fail three-resume case, and all original final
and cancellation output assertions remain. The full 1,973-case corpus is next,
with jobs=6, BINARYEN_CORES=1 and --keep-bounds; no LANDABLE claim until reviewed.


## Completed full corpus and final workspace gate

The pinned record base is 5307a664e. Later main through 7c0652fad changes
only documentation; compiler/library/corpus inputs are identical. The release
production source is byte-identical to 62af4eac4; every one of the 1,973 raw
observations carries the SHA256 of the release binary still used by the gates.

The full corpus completes with 554 rejected checks, 1,416 accepted and three
checker censors, plus the same two registered known failures. The initial
failure list consists of withdrawn-rule requirements and resource censors.
All nineteen revised exact ruling/type requirements pass isolated reruns.
Nine new resource censors are retried alone without increasing a saved bound;
five recover completely. Four wide cases remain censored. The coordinator's
splice.py restores six timed phases. Four runtime phases unstarted after a
censored checker are retained individually; completed merged phases are never
discarded. Raw full/isolated observations and phase provenance remain in target.

After the specified phase splices, 977 observations equal main and restore only
the two fingerprint fields, byte-exactly. There are 972 display changes and 24
semantic changes, every one explained in the corpus audit. No bound changes.
The unmodified current requirement-loop AST rechecks all 395 applicable saved
requirements with zero failures and the same two existing XFAILs. These saved
checks do not claim fresh completion of the listed censored phases.

Workspace all-targets check passes (8.96s). The final gate-equivalent full Rust
run is in progress with all value-types properties and the whole spec suite;
no skip is added. Subsequent serialized gates cover current documentation,
std O0/O2, examples/macro-tests and bench O0/O2, short generation, workspace
clippy, both formatters, snippets and the gate self-tests. Final results follow.


The final whole spec suite passes **536/536 at O0 and O2**, with the unchanged
29 pending entries. The final row-scheme binary passes all 50 tests; typed
macros pass all 44, the macro oracle all 66, control-lazy all 17 and checker
goldens all 5. At this checkpoint 115 Rust binaries have completed with no
failures; the longest properties remain in progress.

`python3 target/e4-exact-semantic-guard.py` independently verifies all 24
semantic case code/count/site sets, all 31 original-plus-m3 verdicts and the
three original Scheduler-leak programs: **34/34 pass**. In particular it pins
all four b1 provider locations, both m3 provider sites, the surviving b10
cycle/provider errors, every z3/z5 provider error and each distinct S/T/Dep
entry diagnostic. No generic diagnostic-code matcher substitutes for those
exact preservation checks.


The completed final Rust gate is green: **118 binaries, zero failures**.
Value-types passes all 31 tests, including the full reverse-order/recovery
property and standard-library permutation checks (3,099.47s). The default
six existing ignores are four checker unit routines and two performance
routines; no ignore or spec-pending entry is added.

The new documentation runner's 14 contract tests pass. Its 110 authored
examples initially give 109 passes and one exact printed-type mismatch:
home read_port shows Fail(process.ProcessError | BadPort) while the expected
text lists those same members in the other order. Spec §4.4: “Order has no
effect”. Preserve the exact type assertion, every error and service key,
and every runtime output/BadPort assertion; change only that union member
ordering. The complete 110-example corrected rerun follows the serialized
gates; no compiler or program source changes for this correction.
