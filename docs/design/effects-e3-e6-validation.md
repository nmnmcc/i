# Effects landing verification, 2026-11-15

Branch **lane/e4**, source checkpoint `867bd364157433bb8f6c2bf52a11ca2cbc5ec334`. Main through
`7c0652fad` is merged. **All final required gates are green**, with the
explicit bounded corpus censors listed below. The final report commit changes
only this verification record; production source has not changed since the
release build at `62af4eac4`. No test or assertion is deleted or weakened.

All compiler/build/test commands use
`H=/home/agent/Programming/i/.rd/cache/heavy`, with only one command running
or waiting for this worker. Cargo uses `direnv exec /home/agent/Programming/i`.
Defaults are `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_RELEASE_DEBUG=0
CARGO_INCREMENTAL=0 RUST_TEST_THREADS=2 BINARYEN_CORES=1`. The snippet script
runs directly because it acquires H internally. Static observation audits
and whitespace checks do not launch a compiler.

## Final gates

The workspace test driver performs `cargo test --workspace --locked --no-run
--message-format=json`, then executes all 118 binaries, six at a time with
two Rust test threads each. The value-types binary retains every test and
uses two CPUs to bound its own parallel worker pool; there is no filter or
new ignore. It passes all 31 tests, including the complete spec and std
reverse-order/recovery checks (3,099.47s). Whole spec passes **536/536 at
O0 and O2**. The default six existing Rust ignores and 29 pending spec cases
are unchanged from main.

Every command below passed in its final run. Logs are local worktree
artifacts; earlier diagnosed failures remain recorded in the history below.

| Command | Final result | Log |
|---|---|---|
| `$H direnv exec /home/agent/Programming/i cargo check --workspace --all-targets --locked` | Workspace/all targets passed | `target/e4-gate-workspace-check-final.log` |
| `E4_TEST_LOG_DIR=/home/agent/Programming/i/.rd/e4/target/e4-land-rust-logs $H direnv exec /home/agent/Programming/i python3 -u target/e4-ptest.py --workspace` | 118 binaries, 0 failed; value-types 31; spec 536/536 both opts | `target/e4-gate-workspace-tests-final.log` |
| `I_BIN=/home/agent/Programming/i/.rd/e4/target/release/i $H direnv exec /home/agent/Programming/i python3 docs/scripts/run-examples-test` | 14 contract tests passed | `target/e4-gate-docs-runner-contract-final.log` |
| `$H direnv exec /home/agent/Programming/i ./target/release/i test --project std --opt 0` | 2171 passed, 0 failed | `target/e4-gate-std-final-o0.log` |
| `$H direnv exec /home/agent/Programming/i ./target/release/i test --project std --opt 2` | 2171 passed, 0 failed | `target/e4-gate-std-final-o2.log` |
| `$H direnv exec /home/agent/Programming/i ./target/release/i test --project examples/macro-tests --opt 0` | 19 passed, 0 failed, 1 intentional skip | `target/e4-gate-examples-macro-tests-final-o0.log` |
| `$H direnv exec /home/agent/Programming/i ./target/release/i test --project examples/macro-tests --opt 2` | 19 passed, 0 failed, 1 intentional skip | `target/e4-gate-examples-macro-tests-final-o2.log` |
| `$H direnv exec /home/agent/Programming/i ./target/release/i test --project bench --opt 0` | 19 passed, 0 failed | `target/e4-gate-bench-final-o0.log` |
| `$H direnv exec /home/agent/Programming/i ./target/release/i test --project bench --opt 2` | 19 passed, 0 failed | `target/e4-gate-bench-final-o2.log` |
| `$H direnv exec /home/agent/Programming/i scripts/test-generation --mode short --binary ./target/release/i` | 32 attempts: 20 accepted/12 model rejections; 64 permutations/32 annotation removals; 0 failures | `target/e4-gate-generation-final.log` |
| `$H direnv exec /home/agent/Programming/i cargo clippy --workspace --all-targets --locked -- -D warnings` | Workspace/all targets, -D warnings passed | `target/e4-gate-clippy-final.log` |
| `$H direnv exec /home/agent/Programming/i cargo fmt --all --check` | All Rust formatting passed | `target/e4-gate-rustfmt-final.log` |
| `$H direnv exec /home/agent/Programming/i ./target/release/i fmt --check bench std lib examples` | bench/std/lib/examples formatting passed | `target/e4-gate-ifmt-final.log` |
| `direnv exec /home/agent/Programming/i scripts/check-snippets` | 5215 literals/blocks; exact diagnostic expectations; 48 .is/101 macro fragments; 4 existing future tracks | `target/e4-gate-snippets-final.log` |
| `$H direnv exec /home/agent/Programming/i python3 -I scripts/check-snippets-test` | 10 passed | `target/e4-gate-snippets-tests-final.log` |
| `$H direnv exec /home/agent/Programming/i python3 -I scripts/qa/corpus-test` | 5 passed | `target/e4-gate-corpus-runner-tests-final.log` |
| `$H direnv exec /home/agent/Programming/i python3 docs/scripts/check-placeholders-test` | 5 passed | `target/e4-gate-docs-placeholders-test-final.log` |
| `$H direnv exec /home/agent/Programming/i python3 docs/scripts/check-spec-links-test` | 5 passed | `target/e4-gate-docs-citation-test-final.log` |
| `I_BIN=/home/agent/Programming/i/.rd/e4/target/release/i $H direnv exec /home/agent/Programming/i python3 docs/scripts/run-examples.py --jobs 2 --no-placeholders` | 110 authored examples passed, including exact type/output/error assertions | `target/e4-gate-docs-examples-corrected-final.log` |

The first 110-example run passed 109 and differed only in the home page's
printed order of the same error union. §4.4 says “Order has no effect”.
The exact expected type now reads Fail(process.ProcessError | BadPort),
retaining both errors, every other service key, and all BadPort/runtime
assertions. The complete corrected 110-example run passes; no program or
compiler changes are used to repair that expectation.

## Runtime artifacts and additional verification

| Command through H | Result |
|---|---|
| `cargo build --release --locked -p ilang-cli` | Passed after the final runtime repair; current production source is byte-identical. |
| `I=$PWD/target/release/i bash scripts/build-abi-fixture.sh` | ABI5 compiled, external-Fail O0/O2, R2 O0/O2 and all law artifacts regenerated and Wasm-validated; hand-written ABI3 fixtures unchanged. |
| `DEV_VERIFY_SKIP_CORPUS=1 ILANG_BROWSER_PORT=4194 scripts/dev-verify js` | 123/123 Node; 69/69 browser checks across Chromium/Firefox/WebKit, no skips; fetch/hypot reuse/external-Fail and all runnable R2 laws at both opts. |
| `cargo test --doc -p ilang-check -p ilang-wasm -p ilang-host -p ilang-cli --locked` | Passed; Rust documentation source has not changed since. |
| `git diff --check` (direct static check) | Passed. |

The release binary SHA256 is
`9de164928abb87ff8e38802f3993c5b39b325723b38affc81c8f803e7dc95f3c`.
All 1,973 raw corpus observations carry this same digest. No production source
or binary changed between that run and the final gates. The test and report
commits do not require another artifact regeneration or JS run.

## Full corpus and exact preservation audit

| Command | Result |
|---|---|
| `UPDATE=1 BINARYEN_CORES=1 $H direnv exec /home/agent/Programming/i scripts/review-corpus run --binary ./target/release/i --keep-bounds --jobs 6 --case-list target/e4-full-corpus-cases.json` | All 1,973 selected: 554 rejected checks/1416 accepted/3 checker censors; 2 baseline XFAILs. Initial failures were withdrawn-rule requirements and resource censors, explicitly diagnosed below. |
| `$H direnv exec /home/agent/Programming/i python3 -u target/e4-retry-corpus-censors.py` | Nine isolated same-bound resource retries plus 19 exact ruling/type guards. Every guard passes; five resource cases clear, four remain censored. Each child uses UPDATE=1, jobs1, keep-bounds and an exact case-list/filter. |
| `python3 /home/agent/Programming/i/.rd/cache/splice.py` (direct observation-only operation) | Six timed phases restored in four records; four dependent unstarted runtime phases carried individually; no whole-file timeout restore. |
| `python3 target/e4-current-corpus-audit.py --base 5307a664e --restore-fingerprints` | Every bound unchanged; 977 identical observations restore only fingerprint fields, byte-exactly;972 display/24 semantic changes fully explained. |
| `python3 target/e4-audit-saved-requirements.py` | The unmodified current matcher AST rechecks all 395 applicable saved requirements: 0 failures, 2 existing XFAILs. This does not claim fresh completion of spliced phases. |
| `python3 target/e4-exact-semantic-guard.py` | 34/34 exact checks:all 24 semantic code/count/site sets, all 31 original-plus-m3 verdicts, and three Scheduler-leak programs. |
| `python3 target/e4-current-display-analysis.py` | All 972 current display cases classified; all 919 original display identities retained, plus 65 additional display cases; no unclassified prose/site changes. |
| Static production/binary and ignore/pending comparisons | Source unchanged since 62af4eac4; every raw observation matches the binary digest; no new Rust ignore and no pending-spec-manifest change. |

The controlling paragraphs, every non-timeout verdict, all 919 original
one-line display judgments and all 65 current additions are in
[the corpus audit](effects-e3-e6-corpus.md). There are **zero unexplained
semantic changes**. Concrete provider errors and their locations are retained;
only quoted-rule carrier/whole-inhabitance summaries disappear.

## Not freshly completed and default skips

The following bounded phases are not claimed as fresh passes. Every saved
bound is retained, and phase-level provenance is in the corpus audit:

| Case | Bound | Censored/unstarted phases |
|---|---:|---|
| review-k4/perf/wide_generic_1024 | 10s | test0,test2; isolated check completed |
| review-k4/perf/wide_generic_2048 | 60s | check; test0,test2 unstarted |
| review-k4c/cases/stress_wide_1024 | 10s | test0,test2; completed full-run check retained |
| review-k4c/cases/stress_wide_open_1024 | 10s | check; test0,test2 unstarted |

The pre-existing vendored-std and std3-bench test censors remain censors.
Registered intentional divergence is checked by its expected bounded timeout,
not recorded as a new failure. Six existing Rust ignores (four literal-stress
unit benchmarks and two performance routines), 29 pending spec cases, four
future-track snippet entries and the macro example's intentional skip remain.
No new skip/ignore/censor requirement is introduced. All other requested gates
are complete; no compiler or std work remains for this handoff.

## Recorded preparation history

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
