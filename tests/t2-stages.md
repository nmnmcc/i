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

## Final verification results

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
