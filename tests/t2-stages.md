# T2 compiler checkpoints and evidence

The accepted contracts are `docs/design/types-joint.md`, then
`docs/design/value-types.md`. The compiler implementation is in lane/t2.
The work present at the start of the compiler continuation was the committed
spec stage; there were no uncommitted compiler changes to discard.

## Interpretation checks

These checks use requirements from the accepted notes, separately from golden
regeneration. The semantic assertions are in
`crates/ilang-check/tests/value_types.rs` and the solver assertions are in
`crates/ilang-check/src/types/value_types.rs`.

| Decision | Governing sentence | Evidence |
| --- | --- | --- |
| Read schemes and share local monotypes | R1: “For a completed generalized binding, take a fresh instance of its scheme, exactly as for an expression use”; “For a parameter, clause binder or non-generalized local in the current top-level group, share its existing monotype.” | Annotation instance/sharing rejection tests and runtime tests |
| Use ordinary lookup | V2: “A module used alone gives E0205; an unknown name gives E0201.” V3: “For the qualifier of `m.x`, a pending value name gives E0207.” | Retired-reservation frontend/CLI expectations and lookup tests |
| Preserve lexical pending and reserved scopes | V3: “Inner binders that have entered scope cover that reserved frame.” | Inner identity shadowing accepted, entire own/header scope rejected |
| Classify before lookup | P1: “An item with an UpName head and `=` has an alias-pattern head exactly when it has arguments and its *written* right side, after stripping type groups, is a value type.” | Import-stable pattern binders, generic aliases, macro reservations |
| Keep alias formals and own holes separate from closed Of sites | R2: “The alias's own formal parameters remain formal parameters; the closure requirement concerns the value-derived part.” | Generic substitution and fresh written holes at separate uses |
| Fix only own-module demands | R3: “Only a demand written in the source definition's own module can fix that source's weak parts.” | Consumer isolation and earlier-open-variable tests |
| Trial the whole solver | Joint §2.3: “T2’s trial must include every field of solver state, not just failures or the initial variable.” | Complete Solver equality before/after failed trial; reverse edges, cycles, extrusion and environments |
| Retain original variable identities | R5: “Do **not** replace a variable at `constrain_in`, erase its old bounds, bypass `reach`, or forbid later bound insertion.” | Predecessor/successor, invariant constraints, shared skipped argument and skeleton tests |
| Reject foreign partial reads | R7: “Check this gate **before** matching P4, even if a pattern skips every open argument or only wants a previously fixed decode component.” | Unchanged §10.1 two-module fixture, exported/private hint variants, direct root and explicitly closed source variants |
| Suggest an available completed alias | R7: “Choose a named hint from the ordinary written alias items when applicable.” | Exported transitive aliases are preferred and qualified in the reader's scope |
| Read inferred types for known conversion | K1: “Of reads inferred schemes/monotypes, never a narrower flow fact.” | Fitted-string and list.map runtime assertions |
| Check surrounding places | K2: “Check the **whole expanded annotation**, including its surrounding union/intersection, with value types represented by their inferred shape before quantified variables are freshly instantiated.” | Distinct variable unions in surrounding input places; identities retained before holder allocation |
| Retain provenance at early uses | Joint J6: “T2’s context check precedes host/service shape checks.” | E0531 through aliases, fields, external/host declarations and environments |
| Mark invalid components before recovery | Joint §3: “A failed source group publishes Failed.” O4: “Mark definition sites, pattern binders and annotation sites in a bad dependency component Failed before inference.” | Type cycles in normal/reversed recovery; unrelated hover and navigation |
| Preserve nominal constructor privacy | X1: “Neither exports constructors of a nominal type that the alias happens to name.” | Both alias export forms accept opaque values, reject private constructors, and run at O0/O2 |
| Separate written query text and inferred hover | X4: “Query type printing shows the written value name in annotations.” “A pattern head targets its existing type; each binder targets its own TypeId.” | Source/head/binder navigation, failed hover and multi-binder outline |
| Publish structural alias results | Joint §2.2: “Full ShownType of each completed value-dependent alias/extraction binder, with formal identities.” | Queries, deferred private-type export checks, interface alpha normalization, own alias holes instantiated per use |
| Preserve runtime order and evaluation | O2: “Runtime needs, full needs and initialization continue to use only expression uses.” | Std/spec reverse-order tests and byte-equal WAT for type-only aliases/projections |

The `pick` place fixture retains its separate variables in the result tuple.
A simpler function can have a principal scheme with one shared variable; that
scheme contains no two-variable union and cannot substantiate an E0518 assertion.
Likewise, invariant Cell tests use an ordinary generic setter helper where the
pre-T1 known-field conversion would otherwise produce an unrelated baseline
E0501. No library or type name receives compiler treatment.

## Implementation checkpoints

1. Specification text: 888cc29, followed by the recorded upstream merge commits.
2. Representations, scopes, ordering and completion: 5859540.
3. Annotation/consumer hardening, semantic/recovery tests: 92c400b.
4. Upstream stream/URI checkpoint: 1ce1a7e (merged by 7e28c3f).
5. Native-design baseline: 681ad65ca053b44dbf4ea75840c38d1c98281993
   (merged by 9b3cc59). This adds native design documents, with no compiler
   changes. The detached checkout at target/t2-baseline-1ce1a7e was moved to
   exactly 681ad65; both its baseline build and WAT dump were rerun. Its
   Binaryen symlink resolves to the exact recorded submodule commit
   fba5005a132d85d578be5bd010e0487ed8e2ce3e.
6. R7 exported alias guidance: f264e72; O3 annotation/alias/path provenance:
   80a3b42. Parallel expression edges retain the original type-edge provenance.
7. Complete spec inventory checks: a4ca239.
8. Random-library baseline: f8ba9204bc352ad753d3751811f14d44bf7dc1cb,
   merged by a20e8d6. The same detached baseline checkout was moved to this
   exact commit; build and export were rerun again. This baseline includes the
   six random-library constructors and their numbering changes. Those upstream
   changes are not charged to T2.
9. Constructor privacy runtime checks: b1df111; field-privacy upstream baseline
   b9173ce, merged by ca9a38b. Its inference check is retained unchanged.
10. Complete R7 normal/reverse recovery variants and matched-holder/original-
    subject runtime regressions: 0740187.
11. Latest upstream baseline: 7b3cea3ed2602744bcbb8c747ebffe177864f1a2,
    merged by c7d7e3c. This includes opaque host nominal identity and the JS
    design. The detached baseline checkout was moved again to this exact SHA;
    both compiler builds and the WAT export were rerun.

The user's later WAT ruling supersedes the original
1df2fe0 reference. Baseline builds and dumps live under this lane's target/.

## Verification commands

Every cargo/dev-verify command uses
`/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i`.
`old-syntax-check` uses that wrapper internally for its parser build.

- `cargo test -p ilang-check --test value_types --lib --locked`
- `cargo test -p ilang-cli value_type_interfaces --locked`
- `cargo test -p ilang-cli --test value_types --locked`
- `cargo test -p ilang-cli --test spec --locked`
- `ILANG_BROWSER_PORT=4182 ./scripts/dev-verify`
- `./scripts/old-syntax-check`
- `WAT_DUMP=<dir> cargo test -p ilang-cli --test wat_dump --locked`, both at
  the exact upstream baseline (detached target/ worktree) and the lane head.
- `python3 crates/ilang-check/tests/value_types_time.py --baseline <baseline-release-i> --candidate <lane-release-i> --project <baseline-std>`

The time script runs one warm-up and 11 measurements for each compiler, in order,
matching literal-types.md's accepted measurement procedure. It reports raw
samples, medians and the +10% investigation threshold. Shared-machine load is
reported with the actual result, not assumed to be a quiet coordinator window.

## Failed checks and corrections

- Early compiler/test compilation found a duplicate last_word helper, a missing
  AST walker field, diagnostic/hover test API mismatches, and a missing fs import.
  Corrected before the first compiler checkpoint; subsequent compiler tests pass.
- Clippy found never_loop, collapsible_if and items_after_test_module. Corrected;
  the full warning-denying check is required again at the final head.
- Focused tests exposed missing written return-environment provenance and
  annotation place identities before allocation. Corrected with regressions.
- A proposed E0518 assertion used a scheme whose inferred variables had already
  coalesced. Corrected the program to contain the distinct union required by K2.
  A tuple after a case expression also needed a newline to avoid parsing a call.
- Runtime fixture setup initially omitted its std dependency and put a test in
  src / at a conflicting module path. Replaced with complete spec projects.
- Existing frontend and CLI tests expected retired reservation diagnostics.
  They now require the actual V2/P1 lookup and binder diagnostics.
- The checker golden suite reported kept.out and type_params.out changes.
  Regenerated only through UPDATE=1; the changed diagnostics are required by
  V2/V3 (E0201/E0207), early P3 binder collection (E0208), and generic parameter
  validation retained by P1 (E0231). No inferred type lines change.
- Old-syntax extraction found Rust assembly templates and a diagnostic search
  string. Replaced assembly templates with complete valid source pieces; the
  search string has the normal explicit non-source entry. Removed T2 ownership
  from shared future-record/literal entries; their remaining parser errors are
  still checked exactly.
- Reverse corpus recovery exposed a secondary E0501 after E0530 depending on
  group order. Affected source groups are now Failed before recovery inference.
- The corpus-count assertion initially guessed more than 200 resolvable cases.
  The run resolved 163 cases. Replaced that unsupported number with exhaustive
  traversal and comparisons for each resolved case, accounting for every
  discovered case. A parser/load rejection must be explicitly expected by its
  authored compile-error header; such cases have no inference graph to reorder.
  The added inventory assertion first exposed a missing rejected-case counter
  in its parser branch (163 compared, 170 discovered); the branch now validates
  and counts those seven authored parser rejections. Old-syntax extraction also
  rejected a capitalized inventory log message as source; the message is now
  ordinary lowercase prose, with no exemption added.
- Two verification retries overlapped in the same log. That log is not the
  final evidence; a separate serialized head verification log supersedes it.
- The first timing check measured 219.85 ms baseline / 243.62 ms candidate
  (+10.81%) during overlapping verification runs and failed its threshold.
  Retained raw samples: target/t2-time-std.json. Investigation found unnecessary
  provenance traversals without Of sites; those now return immediately, as does
  the second dependency graph when no value reads exist. The measurement
  was repeated after the verification jobs finished. At the intervening f8ba920
  baseline it measured +5.52%; that measurement is superseded by the final
  baseline result below.
- The opaque-alias runtime fixture first used two import groups and a
  constructor-list form in an import. Formatting reported E0101/E0115 and the
  strengthened corpus check rejected its unexpected parse failure. Corrected
  it to one imports group and a plain alias import. Its Alias(..) export stays
  on the source module, as X1 specifies.
- The new type-only WAT fixture initially omitted @main and failed the WASI
  entry check E0701. Added the same entry to both sides; all four comparisons
  now pass.

No checks were weakened to accept a compiler mismatch. No stash, config change,
push, dependency update or write to another lane was used. Detached baseline
worktrees, logs and build caches are intentionally retained under target/.

## Verification before the independent review

The compiler source at c7d7e3c contains all T2 changes and the upstream baseline
7b3cea3. The final documentation and derived-file commits change no compiler
behavior. The lane's initial committed spec stage was retained; there was no
uncommitted earlier worker code at the start of the continuation.

| Criterion | Result and command evidence |
| --- | --- |
| Semantic rules, recovery, immutable completion, ordering | Met: 24 integration families and 5 solver tests in the full Rust gate. All required review variants compare full alias results, schemes and diagnostics in normal/reversed recovery. |
| Complete corpus accounting | Met: the discovered spec inventory is fully accounted for; only authored compile-error cases may stop before inference. Every remaining case is checked in both orders, and std's canonical types, needs and init are unchanged. |
| Package/query/export consumers | Met: three package-interface unit tests, query/navigation/outline tests, hidden export rejections and opaque-alias constructor privacy checks. |
| Rust gate | Met: `./scripts/dev-verify rust`, included in `target/t2-7b3cea3-verify.log`: fmt, clippy -D warnings, workspace tests, locked dependencies. |
| Runtime gates | Met: `ILANG_BROWSER_PORT=4182 ./scripts/dev-verify`: std 1422, hello 2 and bench 12 tests pass at each of O0/O2; WASI smoke passes; Node tests and all 36 browser tests pass. |
| Spec suite | Met: `cargo test -p ilang-cli --test spec --locked`: 179/179 cases at both O0/O2 (`target/t2-7b3cea3-spec.log`). |
| Old-syntax gate | Met: `./scripts/old-syntax-check`: 1280 snippets, exact diagnostics, 19 remaining future-track entries and no T2 future entries (`target/t2-7b3cea3-old-syntax.log`). |
| Existing WAT | Met: baseline 636 files, candidate 670, missing 0, changed 0. All 34 additions belong to T2 cases. The comparison asserts byte equality for every baseline file (`target/t2-7b3cea3-wat-comparison.json`). |
| Type-only WAT | Met: completed aliases and projections add no evaluation; all four target/test-mode comparisons pass in `crates/ilang-cli/tests/value_types.rs`. |
| Standard-library timing | Met locally: one warm-up plus 11 release checks per compiler, baseline then lane, after this lane's heavy jobs finished. Medians 226.44 ms and 232.34 ms, +2.60%, below the accepted +10% threshold. Raw samples: `target/t2-7b3cea3-time-std.json`. This is a local shared-machine measurement, not a claim of a coordinator-controlled idle machine. |
| Combined T1 integration | Not run: T1 has not landed. The user's instruction to proceed without waiting governs this continuation. The later shared-region merge and literal-specific checks remain an integration prerequisite. |

The matched-holder and original-subject runtime cases pass at both levels.
Same-module patterns written after a use binding still produce Again = Int;
a whole-value annotation remains E0534 while the encode part is open. The
private/exported alias, wrapped/direct root, closed source and whole-value
variants are also checked under normal/reversed recovery.

No controlling source was overturned. The baseline change from 1df2fe0 is the
user's explicit later ruling, not an unexplained WAT exception. The conditional
pick example is checked with an actual distinct-variable union; flexible type
variables may otherwise coalesce under spec 4.7. The invariant setter helper
avoids the pre-T1 known-field bug while retaining the required ordinary solver
relationships. These are interpretation checks, not golden-derived requirements.

## Integration handoff

Closure, immutable completed reads, shape exposure and the single checked trial
boundary are together in types/value_types.rs. The HIR visitor separates type
children from Of references and pattern binders. value_dependent preserves alias
provenance; Resolver::reject_value_type is the reusable early-context gate.
Foreign readers have no partial-shape API.

T1 has not landed in the upstream baseline above. Per the user's instruction,
T2 proceeds without waiting. Its later merge must preserve ValueSet in every
new Con reconstruction in value_types.rs, add Lit leaves in the HIR visitors
and alias writer, and retain the shared exact display::tree writer. The new
exhaustive matches make omissions visible. Run T1's >12-literal interface case
and the T2/K1 suite after that mechanical integration. T3 owns Record/Without
closure and label checks; M4 owns expansion readiness tests (value-types M3).
These deferred features are not prematurely enabled by T2.


## Independent review corrections

The review was run against 3d10b4b. Its five findings are implementation defects;
the earlier coverage statements did not establish these missing cases. All
56 standalone review projects are now checked in `value_type_review/`, including
the three dependent packages. The 120 declaration permutations and every scale
size/order are generated by repository tests; no test reads the review scratch
directory. Four review runtime tests also run in the spec suite at O0 and O2.

| Finding | Cause, fix and test |
| --- | --- |
| 1 | Raw same-head bounds were counted before normalization. Normalize a shape view while retaining original argument variables, then check the batch against their original constraints. `independent_review_reproducers`, `normalized_shapes_preserve_skips_and_nested_heads`, `normalized_same_head_bounds_retain_each_original_argument`. Required by P4: “exactly one normalized member”. The explicit Int inputs require Item = Int independently of literal narrowing. |
| 2 | A zero-parameter alias expansion returned Exact, which the shape gate rejected. Expand Exact heads transparently and accept only named/tuple results. `independent_review_reproducers`, `qualified_exact_heads_expand_through_named_and_tuple_aliases`. Spec §4.3: “It may be a custom or built-in type, or a transparent alias expanding to one named type or tuple.” Function/union heads remain rejected. |
| 3 | Completion rescanned all definitions and recursively expanded completed bodies. Cache direct dependencies, value-read summaries and parameterized templates; wake aliases once their dependencies complete. `completed_alias_chains_are_linear_in_both_declaration_orders` covers 512/1024/2048/4096 in both orders with the user's exact 3x ordinary-alias gate. `cached_templates_preserve_hole_sharing_and_deferred_constraints` checks that caching preserves formal/hole identities and deferred invariant equations (R2). |
| 4 | One first demand supplied every fixed variable's diagnostic location. Record the demand origin per equation/shape, preserving an earlier fixation. `independent_review_reproducers` asserts Second=b at line 6 of the checked-in reproducer. Spec §4.7 requires “the place that fixed it”. |
| 5 | HIR/query traversal retained only the outer head span. Retain and visit every written head, including nested qualified heads and failed-source recovery. `nested_pattern_heads_have_hover_and_definition_targets` and the real LSP protocol test `t2_nested_pattern_heads_support_hover_and_definition_in_recovery`. X4: “A pattern head targets its existing type”. |

### Spec clarification at §4.3

User ruling: “say explicitly that demands are processed at the completion of
their source, and that a new read after completion follows the closed-source
rule.” The smallest text change distinguishes the completion batch from a new
closed-source observation. Existing R7 normal/reverse tests still reject
foreign partial extraction before matching; local unused patterns still join
the source's own completion batch.

### Spec clarification at §4.7

User ruling: “an annotation read is a value-type demand like any other and takes
part in fixation.” Basis: “a lowercase name in a type means the type of that
value everywhere; an annotation must not behave differently from an alias.”
The collected own-module annotation demand closes weak parts at the source
completion point. Quantified parts still instantiate freshly at each use,
local monotypes stay shared, and foreign reads cannot fix a source. The test
`annotation_demands_fix_weak_sources_and_keep_generalized_instances` accepts
`id=(x){x}; v=id(1); x:v=2` without Read=v, rejects a String use, and compares
normal/reverse group order. No counterexample to principal inference or order
independence was found in these checks; this is evidence for the tested cases,
not a general proof.

### Further members of the reviewed classes

- Closed completed forms are normalized after substituting fixed arguments;
  known conversion uses the same completed templates with its ordinary known
  normalization. Template bodies retain operators until instantiation, rather
  than prematurely solving formal parameters or cloning written holes.
- Exact aliases through another alias and to tuples are covered, with forbidden
  function and union heads retained as negative tests.
- Type ordering now uses dependency counts and reverse edges, failure
  propagation uses a queue, and read completion indexes demands by source.
  Diagnostic alias paths are expanded only when producing a cycle diagnostic.
- Pattern heads are resolved even when the source name is invalid, so query
  recovery does not lose nested navigation. Ordinary type/query walkers already
  recurse through their children. No other group-wide fixation origin remains.

The first extra skip fixture used an earlier weak Cell and correctly got R3's
E0534. Its replacement creates both shared parts in the demanded source group,
so it tests skipped-variable sharing rather than contradicting the ownership
rule. The first hole-sharing probe used covariant tuple members, whose union is
valid; the invariant Cell probe now tests the actual equality obligation.

### Review-check development failures and environment notes

The first focused test compile used an unavailable serde_json test dependency;
the fixture manifest now uses TSV without a dependency change. Query assertions
first used a private/wrong API and now use type_at/definition_at. An OfId sort
first assumed Ord; sorting explicitly by its numeric identity fixes that build.
A new diagnostic assertion first expected line 7, while the exact checked-in
review source has Second=b on line 6; the assertion now uses that source line.
A new diagnostic rendering assertion first used Display for Vec<String> and
was corrected to Debug. Initial normalized-shape trials still reported E0504:
the observed union of original arguments was sent through written-choice
routing. The checked trial now recognizes reflexive union/intersection
inclusions and substitutes only closed batch equations in its shape view; it
keeps ordinary constraints and all original solver identities. No ordinary
written-choice/place check is disabled.

The first extra skip and covariant-hole probes tested the wrong obligations,
as explained above; their replacements test ownership and invariant equality.
One queued performance invocation was cancelled before running because its
release build dependency had not finished; its later run uses the completed
release binary. A baseline command initially repeated a relative worktree path
from inside that same directory; checkout failed and was corrected to the
exact upstream SHA before the queued build began. Temporary baseline worktrees
and their exact Binaryen symlinks are retained under target/. No shared lock,
user checkout, dependency, shell configuration or another lane was changed.

The full post-merge Rust gate first stopped at kept.out: resolving heads before
an invalid source now also reports E0202 for the undefined Schema. Spec §4.3
requires the head to name an existing type; X4 retains resolved query references
through failure. UPDATE=1 cargo test -p ilang-check --test types --locked adds
only that diagnostic, without changing inferred type lines. The first
old-syntax run found an incomplete Rust source-assembly fragment in the new
negative Exact-head test. Replacing a complete valid declaration now builds
the two cases; old-syntax passes without another exemption.

The redesign merge at 94789e6 conflicted in the old alias-reservation paragraphs
of syntax.md and spec §4.3. Resolution retains the accepted T2 classification
and the upstream @js declaration wording (including the closed nominal host
annotation). No provider/runtime text was rewritten. No controlling rule was
overturned.

The chain generator also covers the review's earlier 64/128/256-size probes,
in addition to the four mandatory sizes. All 68 project reproducers are thus
retained directly or generated in full by repository tests.

## Verification after the independent review

Compiler checkpoint: 1804086; redesign merge: 245f518, containing exactly
94789e6141d9d2213d3b909b07bcbac9bbcdb02d as its upstream parent. The detached
baseline at target/t2-baseline-1ce1a7e was moved to that exact upstream SHA;
both its release compiler and WAT test compiler were rebuilt, and its WAT dump
was exported again. Binaryen still resolves to the recorded fba5005 commit.

| Criterion | Final command and result |
| --- | --- |
| Review semantics, transactional completion and query recovery | `cargo test -p ilang-check --test value_types --lib -p ilang-lsp --test protocol --locked`: 31 integration families, 6 solver checks and 14 LSP protocol checks pass. All 56 checked-in projects, 120 declaration permutations and the generated scale cases run from repository sources. The full Rust gate repeats these checks. |
| Rust, package and interface consumers | `ILANG_BROWSER_PORT=4182 ./scripts/dev-verify`: fmt, clippy -D warnings and locked workspace tests pass, including all three value-type interface tests (`target/t2-review-verify2.log`). |
| Runtime and hosts | The same full command passes std 1465, hello 2 and bench 12 tests at each of O0/O2, WASI smoke, 76 Node tests and 51 browser tests. |
| Spec | `cargo test -p ilang-cli --test spec --locked`: 180/180 cases pass at O0/O2, including the four review runtime tests (`target/t2-review-spec.log`). |
| Syntax | `./scripts/old-syntax-check`: 1409 snippets parsed, exact diagnostics match, 19 future-track entries and no T2 entry (`target/t2-review-old-syntax2.log`). No exemption was added. |
| Existing WAT | `WAT_DUMP=... cargo test -p ilang-cli --test wat_dump --locked` at exact upstream and candidate: 636 baseline files, 674 candidate files, missing 0, changed 0; all 38 additions are T2 (`target/t2-review-wat-comparison.json`). The comparison checks every baseline byte. |
| Alias-chain cost | `T2_PERFORMANCE_I=.../target/release/i cargo test -p ilang-cli --test value_type_performance --locked -- --nocapture`: all seven sizes in both orders pass the exact 3x gate. The normal debug-compiler run also passes (`target/t2-review-chain-debug-final.log`). |
| Ordinary corpus cost | `python3 crates/ilang-check/tests/value_types_time.py --baseline .../t2-baseline-build/release/i --candidate .../target/release/i --project .../t2-baseline-1ce1a7e/std`: one warm-up plus 11 samples per compiler, medians 286.04/300.10 ms, +4.91%, below +10% (`target/t2-review-time-std.json`). This is a shared-machine measurement, not a substitute for an idle coordinator measurement. |

Final release chain medians in milliseconds, with the same-count ordinary
control for each size (target/t2-review-chain-release-final.log):

| Alias count | Ordinary | Value forward | Value reverse |
| --- | --- | --- | --- |
| 512 | 7.61 | 9.11 | 10.06 |
| 1024 | 14.21 | 11.97 | 12.64 |
| 2048 | 19.09 | 19.11 | 17.72 |
| 4096 | 27.25 | 28.34 | 29.06 |

The largest required ratio is 1.33 (rounded upward); growth is near linear and
reversing declarations adds no chain rescans. The earlier partial release
measurement is superseded by these post-merge numbers. After adding the three
smaller historical probes, fmt and the complete debug/release performance tests
were rerun; compiler code did not change after the full gate.

The worktree was clean at the start of this review continuation. Existing
committed T2 work was retained and repaired; none was discarded. All failed or
interrupted development checks above were followed by successful checks. The
only derived change is the generated E0202 line in kept.out, committed last.
No new unresolved implementation issue or source conflict remains. T1 is not
in redesign at this baseline; its shared-region integration and literal-specific
checks remain the already assigned follow-up, per the instruction not to wait
for T1. Record builders and macro expansion remain T3/M4 scope.

## Combined T1 and T2 integration

The integration starts from clean 638aac1. All committed work is retained.
T1 landed at 490af9728fa9da1811013babdcc70e90f0e06418. Conflict resolution keeps
its literal sets, coverage, narrowing and deferred Assert checks, and T2's
value references, alias patterns, completion, fixation and query metadata.
Every T2 Con reconstruction copies the existing ValueSet; none substitutes All.
T1's scalar bound fusion calls T2's mutation observation before changing a
bound. The trial clones every Solver field, including the test-only bound
counter. Lit is an immutable leaf in the T2 visitors and alias templates.

### Joint interpretation and test matrix

| Contract and source sentence | Evidence |
| --- | --- |
| Joint §2.2: “No reconstruction defaults an existing Con’s set to All.” | `j1_value_reads_and_patterns_preserve_exact_literal_sets`: direct value annotations, nominal/tuple extraction, same-head unions, negative/cofinite sets and literal constructor fields. `closed_substitution_survives_cached_literal_algebra` covers dependent generic aliases. |
| J1: “Of reads inferred schemes/monotypes, never a narrower flow fact.” | `j1_j2_flow_facts_do_not_become_value_reads_through_access_or_calls`, the existing fitted/list-map T2 runtime checks, and the new nominal/tuple runtime access case. |
| J1: “Rule 12 narrows the new catch-all’s inferred type.” | `j1_j5_new_narrowed_binders_have_literal_types_original_subjects_do_not` checks the positive T-C scalar case's exact hover and a literal result read; the original subject still accepts String. The corresponding runtime test runs both branches. |
| J2: “The inferred field/tuple result comes from those same bound forms ... while its known result comes separately from the original known facts.” | Nominal/tuple access after a fitted literal permits q:p="b"; an actual sink accepting GET/POST rejects DELETE. Generic callback inference with a local value annotation is also checked. |
| J3: “Lower arguments in written evaluation order, using args only to place their results.” | `joint_literal_order.i` and `joint_literal_pipe_order.i` stop at the first written/pipe argument at O0/O2, with Of annotations on the callee's parameters. Label-generic key handling remains T3. |
| J4: “A failed source group publishes Failed.” “Trial all fixation and demanded-shape constraints together.” | Existing normal/reverse recovery families, `j4_literal_fixation_orders_completion_and_rejects_later_widening`, and `failed_literal_trial_restores_fused_bounds_and_ownership`. The latter proves rollback after scalar fusion and observes an earlier unfixed variable. |
| J5: “A type test of a closed T2 alias expands first ... of ... literal types it makes String/Only.” | `j5_completed_literal_aliases_produce_exact_runtime_tests` checks the structured test table and incomplete coverage; the cross-package runtime test checks both accepted strings, another string, and small/boxed Int values. T1's full coverage/narrowing regressions are retained. |
| J6: “T2’s context check precedes host/service shape checks.” | `j6_early_context_provenance_wins_over_literal_boundary_checks`: aliases of literal values still give E0531 in early contexts; an ordinary scheme with an existing environment remains readable. T1's directional and normalized-set host-boundary tests are retained. |
| J7: “T2 must instantiate the preserved scheme ... never generalize a weak variable by reading it.” | `j7_invariant_scheme_reads_keep_fresh_instances_and_weak_sharing`; permanent make aliases fail E0533; unequal invariant forms fail. K1's existing invariant, service, contra/bivariant and stream corpus regressions run in the full gate. |
| Joint §4: “Full type tree/text ... No truncation, private value spelling, solver id, provenance, or fixation site in type identity.” | `completed_literal_aliases_have_full_canonical_identity` checks private renames, canonical numeric/decoded string spelling, and a changed twentieth literal. `value_type_literals` derives a nominal type with literal fields in a dependency package and consumes it through an exported completed alias. |

J8's exact forgetting, the record portions of J1-J7, rigid labels and T-A/D/E/H/I
record programs remain T3's explicitly assigned implementation gate; no record
syntax, label semantics or Without rewrite is introduced here. Their future
syntax entries stay owned by T3. Their shared T1/T2 prerequisites are covered
by exact closed substitution, retained ValueSets, transaction rollback and the
existing scalar Codec/R7 cross-package cases.

### T1 changes to earlier T2 examples

T1 decision 4 says “No widening”; spec §4.7 says “Literal types are never
widened.” Thus id(1) now yields 1. The annotation-demand ruling still fixes it
exactly as Read=v does: x:v=1 passes, x:v=2 fails; an explicit v:Int permits 2.
The earlier pre-T1 example was Int-based and is superseded by the requested
combined literal semantics. This is not an exception to annotation fixation.

The 56 review source projects remain unchanged. Nine manifest expectations
now reflect the same no-widening rule: alias_constant_nested and exact_good
get E0535 because their constant Int subtree is unequal to 1 (P4 exact-subtree
equality); foreign_alias, nested, partial, shared_skip,
union_samehead_alias_only, weak_alias and weak_anno get E0501 for an out-of-set
value. The nested query acceptance fixture uses 1 instead of 2; its retained
review source still tests the original rejected program. Explicit Int-bound
normalization tests retain Item=Int. New literal tests require the narrower
results and reject incompatible values.

### Integration development checks

The first compile found four remaining exhaustive visitor/Con-shape sites;
all were completed without fallback semantics. The first focused T2 run found
the expected pre-literal assumptions above. New probes first omitted a std
dependency and treated one unguarded clause as an exhaustive match; the former
now uses an ordinary local callback combinator, and the latter uses a guarded
match. An invented negative probe tried to reject widening a current-group
weak catch-all variable through a monomorphic annotation. R1 shares that weak
monotype, so that rejection was not required: the retained positive T-C test
asserts the requested exact type before additional widening constraints, and
an explicitly literal result supplies the negative test. No weak local is
frozen to make the probe pass.

A CLI test invocation with --lib selected zero package-interface tests; it was
not counted as verification. The corrected --bin i invocation runs all four
interface tests. The first syntax-data merge restored two entries deliberately
removed by T1; a three-way merge now preserves both tracks' deletions and exact
diagnostics. No syntax check exemption was added. All these checks have passing
focused reruns; full-gate results are recorded below when complete.

### Final combined verification

This replaces the earlier waiting-for-T1 status. The T1 merge is ac8e1c6;
the subsequent upstream merge is b02ca81. Redesign moved to
f6f9cc3a2ea06df397e369b720ce5aed81fedbda during integration, adding only the
numeric-lowering design. The detached baseline checkout was moved to that
exact SHA; both baseline build commands and WAT export were rerun. Its
Binaryen symlink still resolves to the recorded fba5005 commit. The final gate rerun covers all compiler changes, including the alias-copy
optimization below. The final structured alias-test
assertion checks exact String/Only members instead of a diagnostic substring.

| Criterion | Command and result |
| --- | --- |
| Combined inference, exact reads, ordering, K1 and recovery | `cargo test --workspace --locked` in the full Rust gate passes 25 T1, 31 T2 and 9 joint integration families; 29 checker unit tests, all four completed-interface tests, LSP protocol and the existing invariant/service/contra/bivariant/stream fixtures pass. |
| Scalar-bound fusion | `cargo test -p ilang-check --lib --locked t1_stress_ -- --ignored --nocapture --test-threads=1`: all four separately gated tests pass. Alias 1000, calls 2500/5000, match 2000 and nested 50x50 retain peak concrete bounds/head/side=2 (`target/t2-t1-stress-counters-optimized.log`). These were initially ignored by the ordinary workspace command, then explicitly executed. |
| Whole gate | `ILANG_BROWSER_PORT=4182 ./scripts/dev-verify`: fmt, clippy -D warnings, workspace tests, std 1465/hello 2/bench 12 at both O0/O2, WASI smoke, 76 Node tests and 51 browser tests pass (`target/t2-t1-verify-optimized.log`). |
| Spec and runtime aliases | `cargo test -p ilang-cli --test spec --locked`: 239/239 cases pass at O0/O2, including the new cross-package literal-field program and labelled/pipe argument-order cases (`target/t2-t1-spec-optimized.log`). |
| Syntax inventory | `./scripts/old-syntax-check`: 1603 snippets parsed, exact diagnostics match, 17 future-track entries (`target/t2-t1-old-syntax-optimized.log`); the final documentation snippets are checked again before reporting. |
| WAT against exact merged upstream | `WAT_DUMP=... cargo test -p ilang-cli --test wat_dump --locked` at baseline and candidate: 848 baseline files, 898 candidate files, missing 0, changed 0; every baseline byte matches (`target/t2-t1-optimized-wat-comparison.json`). The 50 additions belong to T2/joint cases. |

The readback examples below are excerpts of the programs executed by
`types_joint.rs` and `tests/spec/4.3/value_type_literals` (the latter at O0/O2):

```i
v = "GET"
x: v = "GET"
```

The annotation reads the exact literal type of v. POST would be rejected.

```i
Box(A) = (A)
boxed = True |> (True) { Box(1) } | (False) { Box(2) }
Box(Number) = boxed
```

Number is 1 | 2; no alternative is discarded and no literal is widened.

```i
Config(M, P) = (method = M, port = P)
config = Config(method = "GET", port = 8080)
ConfigType = config
```

ConfigType is Config("GET", 8080). The dependency exports that completed alias;
a client constructs the same exact value through it. Constructor privacy is
unchanged. These are ordinary nominal constructor fields; record builders and
J8's record/Without tests remain T3's assigned gate. No library name receives
compiler treatment.

The first post-merge std timing failed the +10% investigation threshold:
335.53 ms baseline / 373.98 ms candidate, +11.46%, with raw samples retained in
target/t2-t1-time-std.json. Candidate samples ranged from 323.03 to 436.74 ms;
this noisy run is still a failed check, not discarded evidence. Investigation
found a completed template cloned before substitution, followed by a second
full copying traversal. Instantiation now borrows the template, normalization
consumes its owned result, and a cached structural flag avoids that traversal
only when neither the template nor its actual arguments contains an algebra
operator. With no union/intersection/difference, the previous traversal only
rebuilt the same constructors, sets, functions and variables. All operators
still normalize after substitution, so deferred invariant constraints and
written hole identities retain their rules. Literal ValueSets are moved or
preserved, never widened. Known conversion checks both template and actual
arguments for operators as well. All implementation gates are rerun after this
change; the earlier successful full run is not substituted for them.

### Final timing and handoff after the copy investigation

The final full rerun is target/t2-t1-verify-optimized.log (exit 0). The explicit
spec, WAT, syntax, stress-counter and release-chain reruns also pass. The normal
debug chain comparison passes in the workspace suite. Final release medians
from target/t2-t1-chain-optimized.log, milliseconds:

| Aliases | Ordinary | Value forward | Value reverse |
| --- | --- | --- | --- |
| 512 | 8.68 | 9.50 | 9.21 |
| 1024 | 12.12 | 13.19 | 12.12 |
| 2048 | 16.64 | 17.87 | 16.93 |
| 4096 | 27.91 | 27.94 | 31.16 |

All seven sizes including 64/128/256 pass the exact 3x control; completion still
scales near linearly in both declaration orders.

For std, the shared quiet helper reserved all three heavy-job slots and the
measurement process pinned itself to CPU 6. This coordinates cooperating
heavy jobs, not every process on the machine. The unchanged T2 time script ran
one warm-up plus 11 checks per compiler against the exact f6f9cc3 baseline and
its std tree. A diagnostic replay used the saved pre-optimization compiler;
the actual final gate used the rebuilt optimized compiler:

| Measurement | Baseline median | Candidate median | Change | Result |
| --- | --- | --- | --- | --- |
| Earlier unreserved run | 335.53 ms | 373.98 ms | +11.46% | Failed investigation threshold; retained |
| Reserved-window diagnostic replay before optimization | 308.43 ms | 315.53 ms | +2.30% | Below +10%; diagnostic, not final gate |
| Reserved-window final gate | 328.53 ms | 315.69 ms | -3.91% | Pass |

Raw samples and exit statuses are in target/t2-t1-quiet-comparison.json, with
individual before/after JSON files. The two candidate medians are effectively
equal and the baseline varied, so these measurements do not establish a
speedup caused by the copy optimization. They do establish the final gate's
measured result and show that the first failed measurement was noisy. No
threshold, test or source expectation was weakened. The earlier unreserved
measurement is not replaced by an unsupported claim of an idle machine.

All three documentation examples were also checked as independent projects by
the merged release compiler (target/t2-t1-examples.json): all exit 0, with
GET, Box(1|2) and Config(GET,8080) preserved as literal-bearing types. Their
runtime counterparts passed at O0/O2 in the cross-package spec project.

No new golden or derived repository file needed regeneration. Upstream's T1
goldens are retained by the merge; authored T2 expectation changes have the
source explanations above. No user checkout, other lane, shell configuration,
dependency lockfile or compiler check was changed. Temporary baseline trees,
compiler copies, logs and measurement programs remain under this lane's
target/. Scoped port, target-directory and performance variables ended with
their commands. CPU affinity was scoped to the timing process; shared quiet
locks were released on completion.

No applicable T1/T2 joint rule remains blocked. T3 extends Closure, alias
map_vars/has_algebra/normalization, the HIR visitor and shown-type traversal for
Rec/Without while preserving this exact ValueSet and transaction behavior. It
still owns J8 and the record/label-specific parts of the joint matrix. M4's
expansion readiness checks remain with M4. Neither requires an additional
T1/T2 implementation or another T1 merge before this lane can integrate.
