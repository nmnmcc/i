# Syntax-three stage 3: shard 1/10 landing audit

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

No expected observation, requirement, bound, or source fixture was changed
by this audit. The subsequent merge `62bc17079` changes only the captured-row
CLI fixture and a lowering test; its compiler is the same binary.

The classifications below are deliberately strict: changing a diagnostic
code or its count is an **outcome** even when both versions reject. Changed
runtime failure-site coordinates alone are a message change. Thus
`review-std3/failures` retains the same four failed tests and two passing
tests at both optimization levels; its changed failure sites do not change
its runtime outcome.

Initial classifications: **11 message only, 144 shown type, 15 outcome**. The other 28 cases match.

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

**O8 — two resource observations, unresolved in the first run.**
`cancel_choose.test2` and `deadlock.test2` time out after printing their
complete passing summaries. Check, O0 tests, and O0/O2 program runs retain
the expected results. An unchanged-bound, two-case retry is recorded
separately. No timeout is approved as a language-outcome change.

## Every differing case

The table records all 170 full-output mismatches. `T` rows can also have
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
| `review-k4c/cases/missing_3_4_0_1` | outcome | O5 BUG: check rejects → accepts; callback A | B contract lost. |
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
| `review-k4d/perm/p0` | outcome | O7 BUG: bad loses its written complete need N; second E0505 disappears. |
| `review-k4d/perm/p18` | outcome | O7 BUG: bad loses its written complete need N; second E0505 disappears. |
| `review-k4d/perm/p27` | outcome | O7 BUG: bad loses its written complete need N; second E0505 disappears. |
| `review-k4d/perm/p36` | outcome | O7 BUG: bad loses its written complete need N; second E0505 disappears. |
| `review-k4d/perm/p45` | outcome | O7 BUG: bad loses its written complete need N; second E0505 disappears. |
| `review-k4d/perm/p54` | outcome | O7 BUG: bad loses its written complete need N; second E0505 disappears. |
| `review-k4d/proj/a1` | shown type | T; no observation recorded. |
| `review-k4d/proj/b7` | shown type | T; no observation recorded. |
| `review-k4d/proj/f3` | shown type | T; no observation recorded. |
| `review-k4d/proj/m4` | shown type | T; no observation recorded. |
| `review-k4d/proj/q17` | shown type | T; no observation recorded. |
| `review-k4d/proj/q26` | shown type | T; no observation recorded. |
| `review-k4d/proj/q35` | shown type | T! BUG: principal-type assertion fails; S(N & A) instead of S(N). |
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
| `review-k4s/mutual_recursive` | outcome | O6 BUG: two occurs errors → acceptance; complete written needs lost. |
| `review-k4s/principal_missing_use` | shown type | T; no observation recorded. |
| `review-k4s/rec_local_0_17_4` | shown type | T; no observation recorded. |
| `review-k4s/rec_local_1_17_8` | shown type | T; no observation recorded. |
| `review-k4s/wide_128` | shown type | T; no observation recorded. |
| `review-native/fake` | shown type | T; no observation recorded. |
| `review-native3/field` | outcome | O2: removed ! service syntax in preserved invalid input (§2.10). |
| `review-r2/cancel_choose` | outcome | O8: O2 test timed out after passing stdout; unchanged-bound retry pending. |
| `review-r2/deadlock` | outcome | O8: O2 test timed out after passing stdout; unchanged-bound retry pending. |
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
