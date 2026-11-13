# Syntax-three stage 3: shard 1/10 landing audit

The November 14 follow-up at the end of this file supersedes the historical
blockers below: main's conversion repairs and the READY provider-alias fix
leave five explained display differences and no outcome differences.

The exact command was:

```sh
H=/home/agent/Programming/i/.rd/cache/heavy
$H python3 scripts/qa/corpus.py run --shard 1/10 --jobs 2
```

It selected 198 cases on `0f3053a0a`, using release binary SHA-256
`e0db61d40034648d64429b6361a70e324954999557b861d8263a1a8404cba719`.
The runner exited 1: 140 checks accepted, 58 rejected, 170 full-output
mismatches and two additional requirement failures (`proj/q35` and
`cancel_choose`). There were no checker crashes or checker timeouts.
One observation was censored and one known failure remained XFAIL.

The first-run audit changed no expected observation, requirement, bound, or
source fixture. The subsequent merge `62bc17079` changes only the captured-row
CLI fixture and a lowering test; its compiler is the same binary.

The classifications below are deliberately strict: changing a diagnostic
code or its count is an **outcome** even when both versions reject. Changed
runtime failure-site coordinates alone are a message change. Thus
`review-std3/failures` retains the same four failed tests and two passing
tests at both optimization levels; its changed failure sites do not change
its runtime outcome.

Initial classifications: **11 message only, 144 shown type, 15 outcome**. The other 28 cases match.

The unchanged-bound two-case retry completed with all phase exits and runtime
outputs matching the saved observations. Its only remaining differences are
check type notation. After that retry the classification is **11 message
only, 146 shown type, 13 outcome**, plus the unchanged 28 cases. The initial
timeouts remain preserved in the first-run artifacts.

## Reading the classifications

- **M (message only):** source coordinates after migration, updated old-syntax
  guidance, or diagnostic rendering; phase exits and error-code counts agree.
- **T (shown type):** check output differs. Syntax-three §2.4 says, “Function
  types drop the names and the body. The parameter part is a data type”;
  §2.3 writes data labels with `=`; §2.10 moves needs before the result and
  represents errors by `Fail`; §2.8 makes constructor results their own
  labelled types (`Token` → `Stop` in the two constructor-boundary cases).
  These rows classify what changed; they do not automatically approve every
  inferred scheme. In particular, bare open callback rows can introduce a
  `Fail(E)` variable absent from the legacy closed-error annotation (for
  example `generic_chain_64.run`). Those source contracts still need the
  conversion lane's review. No type observation was re-recorded here.
- **T! (`proj/q35`):** all phases return 0, but the existing principal-type
  requirement fails: `fw` is `() <- (S(N & A), ..N) -> Int`, not
  `() <- (S(N), ..N) -> Int`. The legacy functions explicitly declared the
  complete need `N | S(N)`; the converted literals retain their service
  patterns but lose that complete binding annotation. The requirement is
  retained. Spec §4.7 requires “the principal type: the most general type,
  of which every other possible type is an instance”. This is a bug to fix,
  not a reason to change the assertion.

## Outcomes requiring separate treatment

**O1 — permitted grammar change.** `runtime-new/03/original` retains E0115
(the late header) and adds E0111/E0101 for its preserved old clause-block
pipe. Syntax-three §2.5: “`|` may follow only the `}` of a function body,
and the thing after it must be a function literal (D3).” The new match uses
function literals, not `pattern -> expression` clauses in braces. This is
parser recovery over deliberately invalid input, not new acceptance.

**O2 — permitted grammar change.** `review-native3/field` retains rejection
and the import parse error, with additional E0001/E0101 on legacy `!`.
Syntax-three §2.10 replaces environment parameters with a service pattern;
§5.2.2 says, “The error part `^ E` and the environment part `! N` of a
function type are removed”. The preserved invalid source now reaches that
removed spelling. No baseline is updated.

**O3 — unresolved lost errors, classified as bugs.** In
`provision_closure_False` and `_loose/p/x8`, the generic hidden-provider
E0515 disappears; the concrete service mismatch remains. There is no
paragraph authorizing that loss. The conversion lane independently reports
a related hidden-provider failure (`p1` accepts instead of reporting E0515).
This audit does not claim that the root causes are proven identical.

**O4 — unresolved additional error, classified as a bug.** `_loose/p/b1`
keeps the two conflicting-service E0515 errors but adds another at `main`.
No language paragraph has been established for that extra diagnostic; it
was not re-recorded.

**O5/O6 — source-contract conversion bugs.** `missing_3_4_0_1` changes check
exit 1 (E0504) to 0. Its legacy callback has need `A | B`; conversion retains
only `A`. `mutual_recursive` loses the complete written needs on `first`
and `second` and changes two E0505s into acceptance. The newer conversion
head has committed source corrections for both cases, but that head is
not marked landable because of its separate `p1` blocker. Its exact texts
have not been independently rewritten or replaced with new observations.
Spec §4.7: “Annotations are used in inference, and the inferred type must
agree with them.” No note paragraph authorizes the acceptance changes.

**O7 — six source-contract conversion bugs.** For `perm/p0`, `p18`, `p27`,
`p36`, `p45`, and `p54`, the legacy source contains
`bad(N) = (s ! S(N)) -> Int ! N`. Conversion keeps the service pattern and
result, but not the complete written need `N`. Its E0505 disappears while
`loop = (x) { x(x) }` still fails. These fixtures must regain the original
binding contract under spec §4.7; the two-error observations remain intact.

**O8 — two resource observations, resolved by unchanged-bound retry.**
`cancel_choose.test2` and `deadlock.test2` time out after printing their
complete passing summaries. Check, O0 tests, and O0/O2 program runs retain
the expected results. The two-case retry used the original bounds and both
O2 tests completed successfully. Every program/test phase then matched the
baseline exactly (including the expected deadlock panic); only type notation
differs. The retry command still exits 1 for those two display differences,
with no requirement failures. No bound or observation was changed.

## Contract restorations after the first audit

Commit `022f42dc1` restores complete written needs in all 60 permutation
fixtures, takes conversion's exact corrections for `missing_3_4_0_1` and
`mutual_recursive`, and adopts its exact `proj/q35` forwarder. The old
`bad(N) ... -> Int ! N` head was independently checked against main in all
60 permutation files. The 63-case retry preserves every saved phase exit,
error-code count, and runtime stdout; every existing semantic requirement
passes, including the principal scheme in `q35`. The command still exits 1
for legacy type notation and source-coordinate differences. No observation,
requirement or bound changes.

The missing/mutual acceptance changes and all six sampled permutation
error losses (O5/O6/O7) are therefore fixed by source-contract restoration,
not by changing the compiler or approving weaker expectations. The other
11 forwarder backports were reviewed and deferred to conversion after their
optional verification waited 20 minutes without starting; those uncommitted
copies were deliberately discarded. The verified, committed `q35` remains.

Two isolated attempts to close callback error channels did not restore O3's
hidden E0515. Two isolated provider-nesting variants for O4 produced four
and three E0515s rather than the expected two. Neither experiment was
applied to the corpus; these investigations stopped after two unsuccessful
variants. O3/O4 remain bugs with no established language authority.

The final shard rerun completed on the restored inputs at `d402cf846`, using
that same release compiler and unchanged budgets. It selected 198 cases:
138 checks accepted and 60 rejected, with **no semantic requirement failures**.
All 198 cases preserve their baseline phase exits and runtime stdout (only
the property-failure source sites are normalized). The runner still exits 1
for 170 output differences: **11 message only, 154 shown type, five diagnostic
outcomes**. Two outcomes are the documented removed-grammar cases; the other
three are unresolved O3/O4 errors. No acceptance or runtime-output change
remains. No observation, requirement or bound was re-recorded.

The independent 63-case audit was repeated after this final run. It also
checks diagnostic counts and the original failing bindings, and passes.
CLI check passes 10/10 after the fixture merge, and the feature-enabled
lowering suite passes 19/19. This is not a landable result until the three
unexplained diagnostic differences are resolved.

## Every differing case

The table records all 170 final full-output mismatches. `T` rows can also have
source-coordinate or wording changes; those are included in the same row.

| Case | Classification | Review |
|---|---|---|
| `review-a1/_loose/capture/original` | message only | M; codes/counts and phase exits retained. |
| `review-a1/_loose/cases/20_old_alias_returns_fn/original` | message only | M; codes/counts and phase exits retained. |
| `review-a1/_loose/cases/30_param_type_collision/original` | message only | M; codes/counts and phase exits retained. |
| `review-a1/_loose/cases/40_guarded_choices/original` | shown type | T; no observation recorded. |
| `review-a1/_loose/cases/50_new_match_empty_call/original` | shown type | T; no observation recorded. |
| `review-a1/_loose/cases/60_mixed_capture_outer/original` | message only | M; codes/counts and phase exits retained. |
| `review-a1/_loose/confirmed/capture-keep-layout/original` | message only | M; codes/counts and phase exits retained. |
| `review-a1/_loose/runtime-new/03/original` | outcome | O1: removed clause-block grammar; rejection retained (§2.5). |
| `review-a1/_loose/runtime-new/40/original` | shown type | T; no observation recorded. |
| `review-a1/_loose/runtime/02/original` | message only | M; codes/counts and phase exits retained. |
| `review-a1/_loose/runtime/20/original` | message only | M; codes/counts and phase exits retained. |
| `review-a1/_loose/runtime/46/original` | message only | M; codes/counts and phase exits retained. |
| `review-a1/cases/22_new_tuple_returns_fn` | shown type | T; no observation recorded. |
| `review-a1/cases/32_call_constructor_boundary` | shown type | T; no observation recorded. |
| `review-a1/cases/42_old_plus_match` | shown type | T; no observation recorded. |
| `review-a1/cases/52_new_tuple_clauses_lines` | shown type | T; no observation recorded. |
| `review-a1/cases/62_new_record_reserved` | shown type | T; no observation recorded. |
| `review-a1/confirmed/idempotence-plain` | shown type | T; no observation recorded. |
| `review-a1/runtime-new/32` | shown type | T; no observation recorded. |
| `review-a1/runtime-new/64` | shown type | T; no observation recorded. |
| `review-a1/runtime/16` | shown type | T; no observation recorded. |
| `review-a1/runtime/43` | shown type | T; no observation recorded. |
| `review-a1/runtime/61` | shown type | T; no observation recorded. |
| `review-js3/opaque` | message only | M; codes/counts and phase exits retained. |
| `review-k4-lane/error_row_values` | shown type | T; no observation recorded. |
| `review-k4-lane/previous-2-True` | shown type | T; no observation recorded. |
| `review-k4-lane/q21` | shown type | T; no observation recorded. |
| `review-k4-lane/q31` | shown type | T; no observation recorded. |
| `review-k4-lane/row_alias` | shown type | T; no observation recorded. |
| `review-k4-lane/w12-Never` | shown type | T; no observation recorded. |
| `review-k4-lane/w2-Never` | shown type | T; no observation recorded. |
| `review-k4-lane/w7-Never` | shown type | T; no observation recorded. |
| `review-k4/cases/provision_closure_False` | outcome | O3 BUG: generic hidden-provider E0515 lost; concrete mismatch retained. |
| `review-k4/cases/wrapper_6_False_True` | shown type | T; no observation recorded. |
| `review-k4/cases/wrapper_8_True_True` | shown type | T; no observation recorded. |
| `review-k4/perf/generic_chain_64` | shown type | T; no observation recorded. |
| `review-k4/perf/wide_256` | shown type | T; no observation recorded. |
| `review-k4c/cases/annotation_1` | shown type | T; no observation recorded. |
| `review-k4c/cases/closure_statement` | shown type | T; no observation recorded. |
| `review-k4c/cases/contra_deep` | shown type | T; no observation recorded. |
| `review-k4c/cases/effects_co_0_0` | shown type | T; no observation recorded. |
| `review-k4c/cases/effects_co_5_0` | shown type | T; no observation recorded. |
| `review-k4c/cases/effects_inv_2_0` | shown type | T; no observation recorded. |
| `review-k4c/cases/effects_inv_7_0` | shown type | T; no observation recorded. |
| `review-k4c/cases/effects_two_4_0` | shown type | T; no observation recorded. |
| `review-k4c/cases/finite_field_2` | shown type | T; no observation recorded. |
| `review-k4c/cases/finite_pattern_2` | shown type | T; no observation recorded. |
| `review-k4c/cases/invariant` | shown type | T; no observation recorded. |
| `review-k4c/cases/missing_0_1_1_0` | shown type | T; no observation recorded. |
| `review-k4c/cases/missing_0_3_0_1` | shown type | T; no observation recorded. |
| `review-k4c/cases/missing_0_4_1_2` | shown type | T; no observation recorded. |
| `review-k4c/cases/missing_0_6_1_0` | shown type | T; no observation recorded. |
| `review-k4c/cases/missing_1_0_0_1` | shown type | T; no observation recorded. |
| `review-k4c/cases/missing_1_1_1_2` | shown type | T; no observation recorded. |
| `review-k4c/cases/missing_1_3_1_0` | shown type | T; no observation recorded. |
| `review-k4c/cases/missing_1_5_0_1` | shown type | T; no observation recorded. |
| `review-k4c/cases/missing_1_6_1_2` | shown type | T; no observation recorded. |
| `review-k4c/cases/missing_2_0_1_0` | shown type | T; no observation recorded. |
| `review-k4c/cases/missing_2_2_0_1` | shown type | T; no observation recorded. |
| `review-k4c/cases/missing_2_3_1_2` | shown type | T; no observation recorded. |
| `review-k4c/cases/missing_2_5_1_0` | shown type | T; no observation recorded. |
| `review-k4c/cases/missing_2_7_0_1` | shown type | T; no observation recorded. |
| `review-k4c/cases/missing_3_0_1_2` | shown type | T; no observation recorded. |
| `review-k4c/cases/missing_3_2_1_0` | shown type | T; no observation recorded. |
| `review-k4c/cases/missing_3_4_0_1` | shown type | Fixed O5: original A \| B callback need restored; E0504 retained; type/coordinate display only. |
| `review-k4c/cases/missing_3_5_1_2` | shown type | T; no observation recorded. |
| `review-k4c/cases/missing_3_7_1_0` | shown type | T; no observation recorded. |
| `review-k4c/cases/order_hidden_conflict_11` | shown type | T; no observation recorded. |
| `review-k4c/cases/order_hidden_conflict_20` | shown type | T; no observation recorded. |
| `review-k4c/cases/order_hidden_conflict_3` | shown type | T; no observation recorded. |
| `review-k4c/cases/order_multi_instance_1` | shown type | T; no observation recorded. |
| `review-k4c/cases/order_multi_instance_19` | shown type | T; no observation recorded. |
| `review-k4c/cases/order_multi_instance_28` | shown type | T; no observation recorded. |
| `review-k4c/cases/order_multi_instance_9` | shown type | T; no observation recorded. |
| `review-k4c/cases/order_mutual_17` | shown type | T; no observation recorded. |
| `review-k4c/cases/order_mutual_26` | shown type | T; no observation recorded. |
| `review-k4c/cases/order_mutual_7` | shown type | T; no observation recorded. |
| `review-k4c/cases/order_mutual_annot_15` | shown type | T; no observation recorded. |
| `review-k4c/cases/order_mutual_annot_24` | shown type | T; no observation recorded. |
| `review-k4c/cases/order_mutual_annot_5` | shown type | T; no observation recorded. |
| `review-k4c/cases/order_recursive_13` | shown type | T; no observation recorded. |
| `review-k4c/cases/order_recursive_22` | shown type | T; no observation recorded. |
| `review-k4c/cases/order_recursive_31` | shown type | T; no observation recorded. |
| `review-k4c/cases/plain_2` | shown type | T; no observation recorded. |
| `review-k4c/cases/recursive_co_4` | shown type | T; no observation recorded. |
| `review-k4c/cases/row_callback` | shown type | T; no observation recorded. |
| `review-k4c/cases/stress_plain_32` | shown type | T; no observation recorded. |
| `review-k4c/cases/stress_wide_open_1024` | shown type | T; no observation recorded. |
| `review-k4d/_loose/fuzz/e0505/21_135` | shown type | T; no observation recorded. |
| `review-k4d/_loose/fuzz/e0505/21_272` | shown type | T; no observation recorded. |
| `review-k4d/_loose/p/b1` | outcome | O4 BUG: third E0515 added at main; no authorization established. |
| `review-k4d/_loose/p/c1` | message only | M; codes/counts and phase exits retained. |
| `review-k4d/_loose/p/g_ann_SDep` | shown type | T; no observation recorded. |
| `review-k4d/_loose/p/q10` | shown type | T; no observation recorded. |
| `review-k4d/_loose/p/q2` | shown type | T; no observation recorded. |
| `review-k4d/_loose/p/q29` | shown type | T; no observation recorded. |
| `review-k4d/_loose/p/q38` | shown type | T; no observation recorded. |
| `review-k4d/_loose/p/r2` | shown type | T; no observation recorded. |
| `review-k4d/_loose/p/w12` | shown type | T; no observation recorded. |
| `review-k4d/_loose/p/w7` | shown type | T; no observation recorded. |
| `review-k4d/_loose/p/x8` | outcome | O3 BUG: hidden-provider E0515 lost; concrete mismatch retained. |
| `review-k4d/_loose/spec/tests_spec_4.3_a1_alias_pattern` | message only | M; codes/counts and phase exits retained. |
| `review-k4d/fuzz/m16` | shown type | T; no observation recorded. |
| `review-k4d/fuzz/pl42` | shown type | T; no observation recorded. |
| `review-k4d/perf/chain200` | shown type | T; no observation recorded. |
| `review-k4d/perf/mutplain25` | shown type | T; no observation recorded. |
| `review-k4d/perf/runchain4000` | shown type | T; no observation recorded. |
| `review-k4d/perm/p0` | shown type | Fixed O7: bad and loop both retain E0505; type/coordinate display only. |
| `review-k4d/perm/p18` | shown type | Fixed O7: bad and loop both retain E0505; type/coordinate display only. |
| `review-k4d/perm/p27` | shown type | Fixed O7: bad and loop both retain E0505; type/coordinate display only. |
| `review-k4d/perm/p36` | shown type | Fixed O7: bad and loop both retain E0505; type/coordinate display only. |
| `review-k4d/perm/p45` | shown type | Fixed O7: bad and loop both retain E0505; type/coordinate display only. |
| `review-k4d/perm/p54` | shown type | Fixed O7: bad and loop both retain E0505; type/coordinate display only. |
| `review-k4d/proj/a1` | shown type | T; no observation recorded. |
| `review-k4d/proj/b7` | shown type | T; no observation recorded. |
| `review-k4d/proj/f3` | shown type | T; no observation recorded. |
| `review-k4d/proj/m4` | shown type | T; no observation recorded. |
| `review-k4d/proj/q17` | shown type | T; no observation recorded. |
| `review-k4d/proj/q26` | shown type | T; no observation recorded. |
| `review-k4d/proj/q35` | shown type | Fixed T!: original principal-type assertion passes; type notation only. |
| `review-k4d/proj/q8` | shown type | T; no observation recorded. |
| `review-k4d/proj/w1` | shown type | T; no observation recorded. |
| `review-k4d/proj/w4` | shown type | T; no observation recorded. |
| `review-k4d/proj/x5` | shown type | T; no observation recorded. |
| `review-k4d/proj/z5` | shown type | T; no observation recorded. |
| `review-k4s/ann_install1` | shown type | T; no observation recorded. |
| `review-k4s/ann_twice1` | shown type | T; no observation recorded. |
| `review-k4s/co_0_17` | shown type | T; no observation recorded. |
| `review-k4s/extra_servicearg` | shown type | T; no observation recorded. |
| `review-k4s/min_main_0_0` | shown type | T; no observation recorded. |
| `review-k4s/min_main_5_0` | shown type | T; no observation recorded. |
| `review-k4s/mixed_install_3` | shown type | T; no observation recorded. |
| `review-k4s/mutual_recursive` | shown type | Fixed O6: written needs restored; first and second both retain E0505; type/coordinate display only. |
| `review-k4s/principal_missing_use` | shown type | T; no observation recorded. |
| `review-k4s/rec_local_0_17_4` | shown type | T; no observation recorded. |
| `review-k4s/rec_local_1_17_8` | shown type | T; no observation recorded. |
| `review-k4s/wide_128` | shown type | T; no observation recorded. |
| `review-native/fake` | shown type | T; no observation recorded. |
| `review-native3/field` | outcome | O2: removed ! service syntax in preserved invalid input (§2.10). |
| `review-r2/cancel_choose` | shown type | Initial O8 resolved: unchanged-bound retry matches all runtime phases; type notation only. |
| `review-r2/deadlock` | shown type | Initial O8 resolved: unchanged-bound retry matches all runtime phases; type notation only. |
| `review-r2/never_yields` | shown type | T; no observation recorded. |
| `review-r2/senders_256` | shown type | T; no observation recorded. |
| `review-std2/client_nil_error` | shown type | T; no observation recorded. |
| `review-std2/minimal` | shown type | T; no observation recorded. |
| `review-std3/failures` | shown type | T + M: type notation and reported source coordinates only; four failures retained. |
| `review-t1/equality` | shown type | T; no observation recorded. |
| `review-t1/fuzz-102` | shown type | T; no observation recorded. |
| `review-t1/fuzz-111` | shown type | T; no observation recorded. |
| `review-t1/fuzz-120` | shown type | T; no observation recorded. |
| `review-t1/fuzz-13` | shown type | T; no observation recorded. |
| `review-t1/fuzz-139` | shown type | T; no observation recorded. |
| `review-t1/fuzz-148` | shown type | T; no observation recorded. |
| `review-t1/fuzz-157` | shown type | T; no observation recorded. |
| `review-t1/fuzz-166` | shown type | T; no observation recorded. |
| `review-t1/fuzz-175` | shown type | T; no observation recorded. |
| `review-t1/fuzz-22` | shown type | T; no observation recorded. |
| `review-t1/fuzz-31` | shown type | T; no observation recorded. |
| `review-t1/fuzz-40` | shown type | T; no observation recorded. |
| `review-t1/fuzz-5` | shown type | T; no observation recorded. |
| `review-t1/fuzz-59` | shown type | T; no observation recorded. |
| `review-t1/fuzz-68` | shown type | T; no observation recorded. |
| `review-t1/fuzz-77` | shown type | T; no observation recorded. |
| `review-t1/fuzz-86` | shown type | T; no observation recorded. |
| `review-t1/fuzz-95` | shown type | T; no observation recorded. |
| `review-t1/host_export_input` | shown type | T; no observation recorded. |
| `review-t1/list_annotated` | shown type | T; no observation recorded. |
| `review-t1/original-stop_tail_service` | shown type | T; no observation recorded. |
| `review-t1/stress-alias-2000` | shown type | T; no observation recorded. |
| `review-t1/typed_function_acc` | shown type | T; no observation recorded. |

The original complete outcomes and unified diffs are preserved in the shared
coordinator cache as `s3-corpus-first-outcomes.json`,
`s3-corpus-first-diff.json`, and `s3-corpus-first-diff.txt`, alongside
`s3-corpus-1of10.log`. These files preserve the first run even when a
filtered retry replaces an ignored `actual.json`.

The final complete outcomes and unified diffs are preserved in the same cache
as `s3-corpus-final-outcomes.json`, `s3-corpus-final-diff.json`, and
`s3-corpus-final-diff.txt`; the final runner log is
`s3-corpus-final-1of10.log`. Every final difference is classified in the table
above. The three unexplained diagnostic changes remain blockers.

## Follow-up against landed conversion, 2026-11-14

The same wrapped shard command ran on `45ef0ced6`, after main `68b720e9`
and READY conversion fix `db743d5e` were merged. The fresh release binary's
SHA-256 was `9178624f3dcf58a20a78df3365242fad3c26dff5341b5688b186abc54fbc67d1`.
All 198 cases completed: 138 checks accepted, 60 rejected, one observation
censored as already registered, and one existing XFAIL. There were no new
requirement failures, crashes, or checker timeouts. The raw runner exits 1
for precisely these five full-output differences:

| Case | Classification | Complete difference and authority |
| --- | --- | --- |
| `review-a1/cases/32_call_constructor_boundary` | shown type | `f : () -> Token` becomes `() -> Stop`. Syntax-three §2.8: “A labelled type with data `D` has the constructor function `(D) -> T(…)`”; sums are union aliases. `Stop` is the constructor's own type. |
| `review-a1/runtime-new/32` | shown type | The same `Token` → `Stop` result; runtime phases and outputs are unchanged. Same §2.8 paragraph. |
| `review-r2/never_yields` | shown type | `Fail(task.ScheduleError)` becomes `Fail(task.NotRunnable)` in `main`'s need. `lib/i/task.i` defines `ScheduleError = NotRunnable`; this names the same type. §2.8: “A name is a transparent alias, as today.” All phase exits and runtime output match. |
| `review-t1/fuzz-111` | message only | E0516's uncovered witness becomes `("", True)` instead of `("", _)`. |
| `review-t1/fuzz-22` | message only | E0516's uncovered witness becomes `("b", True)` instead of `("b", _)`. |

Both witnesses are uncovered values of the original subject type; each case
still has exactly one E0516 at the same location. `coverage.rs::show_pat`
prints the single constructor of a labelled type, so the now independent
`True` type supplies a concrete witness. Syntax-three §2.8 says, “When `D`
is `()`, the constructor is a value: `None`, `Nil`, `True`”. These changes
do not change coverage or the errors reported.

The classification is **three shown type, two message only, zero outcome**;
the other 193 observations match, including the registered censored case.
No expectation, requirement, source, or bound was changed by this audit.
In particular, `provision_closure_False`, `_loose/p/x8`, and `_loose/p/b1`
match main's complete observations, with one, two, and three E0515 errors.
The historical O3/O4 blockers above are resolved against the landed main
baseline, also pinned by the new checker diagnostic regression.

The complete expected/actual snapshot is in the shared coordinator cache
as `s3-r3-corpus-outcomes.json`; the per-case machine classification and
unified diffs are `s3-r3-corpus-diff.json` and `s3-r3-corpus-diff.txt`.
The runner log is `s3-r3-corpus.log`.

### Final post-format verification

At `810df51bf`, after formatting the two stage-3 library lines and fixing
the snippet search-needle context, the release rebuild passes with the
same binary SHA-256. A second complete shard run again checks 198 cases,
with 138 accepted / 60 rejected, the same registered censored observation
and existing XFAIL, and unchanged checker diagnostics. It has seven
full-output differences: the five above, plus O2 `test2` timeouts in
`review-r2/cancel_choose` and `review-r2/deadlock`. Both print exactly the
saved `qa_startup` pass and `1 passed, 0 failed` summary before the timeout.
`cancel_choose` also fails its accept requirement in that bounded run.
These are classified as **two outcome differences**, not silently treated
as successful terminations. No language rule is used to permit them.

The two-case retry keeps each recorded bound of 10 seconds and `--jobs 2`.
It exits 0 with no observation or requirement differences. Both `test2`
exits are again 0. `deadlock`'s run phases intentionally exit 1, as saved;
those exits and messages remain unchanged. No bound, requirement, source,
or expectation is edited. The full first run, the first post-format run,
and both retry observations remain available for review.

After that retry, every one of the 198 complete observations is verified
identical to the first full run, not merely equal in checker acceptance.
The final classification remains **three shown type, two message only,
zero outcome**. No new diagnostic coordinate difference appears. The
three original provider cases match main completely in both runs.

The shared coordinator cache contains `s3-r3-corpus-final.log`,
`s3-r3-corpus-final-first-outcomes.json`, and
`s3-r3-corpus-final-first-diff.json` / `.txt` for the raw bounded run.
`s3-r3-runtime-retry.log` records the unchanged-bound retry. The reconciled
snapshot and classification are `s3-r3-corpus-final-outcomes.json` and
`s3-r3-corpus-final-diff.json` / `.txt`; `s3-r3-final-audit.json` records the
198-case equality check. No observation was re-recorded to obtain this
result.

### Required equality warnings on the landed-tip integration

The first complete `review-k4d` run checks 365 projects (213 accepted,
152 rejected); 364 observations match and one differs only in warning
messages. `review-k4d/_loose/spec/tests_spec_11_equality` gains W0503 at
`test/equality.i:57:5`, `59:5`, and `62:5`, in check, O0 and O2. The pairs
are `Some(1)` / `None`, `Ok(1)` / `Error(1)`, and `True` / `False`.
Syntax-three §2.8 makes each data definition its own nominal type and its
constructor return that type. These pairs are now known disjoint types;
spec §6.9 requires W0503 when both known types are disjoint (§4.5 rule 3).
Suppressing these warnings would violate that paragraph.

Only those nine warning lines are added to the saved observation; all
exits, stdout, commands, bounds, source programs and existing warnings are
unchanged. Each runtime phase still passes all five equality tests. The
record's compiler/hash provenance is refreshed for this observed binary.
The focused case passes after the update. This preserves and strengthens
the equality test's intent; no error is removed or weakened. All fourteen
named-provider records remain byte-identical to fetched main. The complete
first-run snapshot is `s3-r4-corpus-first-outcomes.json` in the shared
coordinator cache; its raw log is `s3-r4-corpus.log`, and the focused retry
is `s3-r4-equality.log`. A second complete filter run is required before
READY.
