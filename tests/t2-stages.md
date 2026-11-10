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
| Preserve lexical pending and reserved scopes | V3: “Inner binders that have entered scope cover that reserved frame.” | Inner identity shadowing accepted, entire own/header scope rejected |
| Classify before lookup | P1: “An item with an UpName head and `=` has an alias-pattern head exactly when it has arguments and its *written* right side, after stripping type groups, is a value type.” | Import-stable pattern binders, generic aliases, macro reservations |
| Fix only own-module demands | R3: “Only a demand written in the source definition's own module can fix that source's weak parts.” | Consumer isolation and earlier-open-variable tests |
| Trial the whole solver | Joint §2.3: “T2’s trial must include every field of solver state, not just failures or the initial variable.” | Complete Solver equality before/after failed trial; reverse edges, cycles, extrusion and environments |
| Retain original variable identities | R5: “Do **not** replace a variable at `constrain_in`, erase its old bounds, bypass `reach`, or forbid later bound insertion.” | Predecessor/successor, invariant constraints, shared skipped argument and skeleton tests |
| Reject foreign partial reads | R7: “Check this gate **before** matching P4, even if a pattern skips every open argument or only wants a previously fixed decode component.” | Unchanged §10.1 two-module fixture, exported/private hint variants, direct root and explicitly closed source variants |
| Read inferred types for known conversion | K1: “Of reads inferred schemes/monotypes, never a narrower flow fact.” | Fitted-string and list.map runtime assertions |
| Check surrounding places | K2: “Check the **whole expanded annotation**, including its surrounding union/intersection, with value types represented by their inferred shape before quantified variables are freshly instantiated.” | Distinct variable unions in surrounding input places; identities retained before holder allocation |
| Retain provenance at early uses | Joint J6: “T2’s context check precedes host/service shape checks.” | E0531 through aliases, fields, external/host declarations and environments |
| Mark invalid components before recovery | Joint §3: “A failed source group publishes Failed.” O4 requires affected sites and source components Failed before query inference. | Type cycles in normal/reversed recovery; unrelated hover and navigation |
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
4. Latest merged upstream baseline: 1ce1a7e15eb1bc39e238e2fb30fe9079df861c5b
   (merged by 7e28c3f). The user's later WAT ruling supersedes the original
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
  V2 (E0201/E0207), early P3 binder collection (E0208), and generic parameter
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
  traversal and comparisons for each resolved case; parser/load-rejected cases
  have no inference graph to reorder.
- The new type-only WAT fixture initially omitted @main and failed the WASI
  entry check E0701. Added the same entry to both sides; all four comparisons
  now pass.

No checks were weakened to accept a compiler mismatch. No stash, config change,
push, dependency update or write to another lane was used. Detached baseline
worktrees, logs and build caches are intentionally retained under target/.

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
