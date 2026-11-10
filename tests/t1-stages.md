# T1 stage record

The baseline is `1df2fe0` on `lane/a`. The lane was clean on entry.
The implementation follows `docs/design/types-joint.md` first, then the
ordered commits in `docs/design/literal-types.md` §12.

## Stage 1: specification and baseline audit

The T1 paragraphs of the accepted replacement text are installed in
`docs/spec.md`. Literal type parsing remains stage 8. The new host example
has an explicit T1 future entry in `scripts/old-syntax-check.json`, as
literal-types §12 step 1 requires: “the blocks that write literal types
... are marked as not yet checked, and step 8 removes the marks.”
No current syntax reservation is removed at this stage.

Interpretations and sources:

- Only T1 grammar is installed now. Joint §6's section map assigns
  “Literals and TypeArg=Type” to T1, ValueType to T2, and RecordType to T3.
  The lowercase and macro reservations remain. Joint §5 says: “No import
  lookup changes this decision.”
- Rule 13 keeps distinct inferred and known results. Joint J2 says:
  “The inferred field/tuple result comes from those same bound forms ...
  while its known result comes separately from the original known facts.”
  The record dispatch and record traversal will be added by T3.
- Joint §10 corrects the old unconditional syntax-error example:
  “In `v |> { x: Int | y -> a  _ -> b }`, `Int | y` is a type; before T2
  y is reserved, and after T2 it reads a value type”. The spec retains
  the reservation for the current stage.
- K1 stays intact. Joint J7 says: “T1 must preserve that condition,
  variable identities, polarity, and weak levels while adding the set
  field; a set has no variables.” The condition in display's occurrence
  walk is `env || v == Variance::Inv`.
- No new dependency is needed: literal-types §4 explicitly permits “a
  small persistent B-tree written in lits.rs” when no dependency is
  approved. No Cargo manifest or lockfile change is planned.
- One-clause choices are required by joint §9 T1 step 7: “Implement
  fitted choices for one clause, replacing E0235”. The older conditional
  wording in literal-types §5.3/§12 does not control this work.
- Joint §10 corrects literal-types §5.7: “TypeShow::head prints a whole
  type definition and delegates its alias body to TypeShow::ty”. Literal
  alias bodies must retain their values; this is not a head conversion.

Current anchors (function names take precedence over line numbers):

| Area | Baseline anchor |
|---|---|
| AST/type parser | syntax `ast.rs` TypeKind 406, TypeArg 442; parser type_prim 1713 |
| HIR/resolution | HIR TypeKind 273; resolve ty 1303 |
| Written host checks | resolve host_fns 1477, host_fn 1512, host_part 1572, host_type 1655 |
| Grouped negatives | resolve is_direct 1819; coverage literal 672 |
| HIR visitors | resolve named_types 2753, for_each_child 2784 |
| Group lifecycle | infer group 233, binding decisions 269/1948/2076 |
| Inferred host checks | infer host_part 501, host_type 528 |
| Solver | constrain_in 245, add_bound 385, route 516, diff_left 544, union_in 698, inter_in 744, merge 801, diff_in 877 |
| Display/K1 | same_unmarked 225, occurrence condition 1034, Namer 1135, tree 1178/1282 |

Stage gate:

- `cargo test -p ilang-syntax --test parse --locked`: 13 passed, including
  `new_forms_and_joined_parentheses`, header/attribute reservations, and
  greedy type annotations. These fixtures predate this work and derive
  their expectations from accepted syntax §3 and §6.1, not new goldens.
- `scripts/old-syntax-check`: exact diagnostics must remain green with the
  explicitly pending T1 host example.

## Stage 2: pure literal algebra

`types/lits.rs` implements ValueSet, canonical integer text, and the public
LitValue/TestValues forms. It is compiled only for tests until the field
stage connects it to production types. This keeps the stage behavior
unchanged without suppressing compiler warnings.

The persistent B-tree uses path copying for unequal-size operations and
a sorted merge plus linear bulk construction for comparable-size sets.
Its cached hash is a wrapping sum of deterministic per-item hashes.
Equality checks contents after hashes; a hash collision cannot equate
different sets. These choices implement literal-types §4's sentences
“adding or removing one item is O(log n)” and “when the two sets are about
the same size, one sorted merge, O(n + m), is used instead.”

Interpretation checks cite literal-types §4's algebra table and §10's
exhaustive model. Four listed values plus an outside value represent the
infinite domain. Empty operation results are distinct from All. Tests
also check canonical form/equal hashes, decoded Unicode, large sets,
B-tree ordering/occupancy/depth, and preservation of earlier snapshots.

Stage gate:

- `cargo test -p ilang-check --lib --locked types::lits`: 5 passed.
- `cargo test -p ilang-check -p ilang-cli --test spec --test types --test query --lib --locked`:
  16 query tests, 5 type tests (including K1 fixtures), and 139 spec cases
  at both O0 and O2 passed, with no golden regeneration.
- `cargo clippy -p ilang-check --all-targets --locked -- -D warnings`: passed.

## Stage 3: Con field and runtime test table

The joint algebra shape is `Con(Head, ValueSet, Vec<Ty>)`. New named and
tuple heads have All values. Every child mapper and rebuilder carries the
existing set. No expression, annotation, or pattern constructs a restricted
set yet. This step used targeted temporary dead-code expectations for
staged helpers. Stage 4 removed them: unused constructors/accessors remain
test-only until their consumer stages. No lint exceptions remain.

The existing coverage integer conversion now calls the same canonical
decimal helper as the literal tests. Its output is unchanged, including
negative zero. The measured baseline Ty size is 40 bytes on this 64-bit
target; the enum and Head at `1865366` have the same fields as `1df2fe0`.
The compile-time limit is 48 bytes, literal-types §4's baseline plus 8.

`type_tests` now contains TestMember with TestValues. All existing tests
produce All. Lowering includes the future Only equality tests and the
Except head-and-negated-equality conjunction. Its source is literal-types
§5.6: “Not(Or(...)) alone is wrong ... The And(head_cond) is required.”

The eight wildcard Con matches and head comparisons were audited against
literal-types decision 2 and §12 step 3. Locations below refer to the
pre-field baseline so they stay useful after formatting:

| Location | Set rule / reason sets do not matter here |
|---|---|
| solve 298 | Concrete-member partition only; clones retain sets |
| solve 397 | Concrete bound collection only; stage 4 adds fusion |
| solve 442 | Bound reachability; equality/hash already include sets |
| solve 552 | diff_left set rule in stage 4 |
| solve 779 | inter_in set rule in stage 4 |
| solve 893 | diff_in set rule in stage 4 |
| display 871 | Norm difference set rule in stage 5 |
| known 333 | Matching shape/children; union subtraction changes in stage 7 |
| solve 320 | Difference overlap set rule in stage 4 |
| solve 393/453/473 | compare deduplicates argument/arity comparisons only; literal heads have no arguments |
| solve 519/521 | Route split set rule in stage 4 |
| solve 710/711 | Union merge set rule in stage 4 |
| solve 756 | Recognizes headed alternatives; does not remove or rebuild a member |
| solve 782 | Intersection merge set rule in stage 4 |
| display 711/712 | Norm union merge set rule in stage 5 |
| display 740/756 | Norm Top/subset rule in stage 5 |
| display 787/791/820 | Classifies headed and variable alternatives; retains full nodes |
| display 836/837/838 | Norm intersection merge set rule in stage 5 |
| known 27/29 | Set-sensitive members in stage 5 |
| known 42/48 | Selects the member for a head; returns the full member |
| known 66/69 | Set-sensitive disjointness in stage 5 |
| known 352/357 | Union known matching by value in stage 7 |
| coverage 231 | Set-sensitive difference in stage 5 |
| coverage 284/390/488 | Constructor domain/specialization in stages 5/7 |
| pattern 474 | Known difference in stage 6b |
| pattern 729 | Tests whether a member has a concrete head; sets do not affect validity |
| pattern 906 | Constructor head recorded for narrowing; literal candidates in stage 8 |
| infer 935 | Rejects variable members in a written difference; sets have no variables |
| infer 1134 | Expected list element shape; List always has All values |
| infer 1654 | Selects nominal constructor field forms; scalar heads have no fields |
| infer 1900/1959 | Service lookup; literal heads cannot be services |
| mod 288 | Lists nominal type identities; scalar literals introduce no nominal identity |

Rebuilders audited: solve map_children, display Coalescer::go/both/norm/
replace/tops/hide/not_general, known known_bound/ksubst_in, and infer
instantiation. The mergers retain the first set until their stage 4/5
algebra is installed. same_unmarked and shown tree are stage 5 work.
K1's invariant occurrence guard and A2's Namer are preserved.

Stage gate:

- Workspace `cargo check --workspace --tests --locked`: passed.
- Checker/CLI library, codegen, type and query tests: passed (7 new literal
  algebra/representation tests in the checker library).
- `cargo test -p ilang-cli --test spec --locked`: 139/139 cases at O0/O2.
- `WAT_DUMP=... cargo test -p ilang-cli --test wat_dump --locked`: 522
  outputs, all byte-equal to the provided `1df2fe0` directory; none added
  or removed. This is an implementation comparison, not evidence for new
  literal semantics, which remain disabled.
- No golden or derived file changed.

## Stage 4: solver rules

The solver now implements literal-types §5.1 rules 1–10. Same-head
constraints use set inclusion; route sends only the uncovered scalar
values to the eligible variable alternative; diff_left and diff_in
subtract values; unions batch scalar heads; intersections test for empty
sets before merging arguments. Lower bounds fuse by union and nonempty
upper bounds by intersection. Disjoint upper bounds remain separate
because reach ignores Never. Nonempty scalar creation uses Ty::lit;
with_values maps an empty operation result to Never before calling it.
This reconciles §4's nonempty ValueSet constructor with its sentence
“It returns Ty::Never for an empty set.”

Environment comparisons require equal sets. Bound propagation cannot skip
an environment comparison just because an ordinary subtype comparison
previously accepted the same stored scalar bound: the old propagation
may have used a different comparison mode. The regression checks this
directly. This follows §5.1 rule 1: “In env mode ... require S == T.”

Value-only failures remain Mismatch until stage 6b adds Values/sub_case.
Source inference still produces only All sets, as §12 requires before
stage 7. Display, known types and coverage have not yet acquired their
set-sensitive rules and must be completed before enabling literals.

Stage gate:

- 15 checker library tests passed, including 8 solver rule tests. These
  cover accepted/rejected subtyping, all three diff_left splits, raw
  same-head unions, ambiguity, delayed variable links, 1000-bound fusion,
  disjoint upper bounds, and literal service-argument equality.
- Existing codegen, type and query tests passed without regeneration.
- 139/139 spec cases passed at both O0 and O2.
- All 522 WAT outputs remained byte-equal to the provided `1df2fe0`
  export, with none added or removed.
- The first full dev-verify stopped at Clippy's items_after_test_module:
  the new solver tests preceded remaining helpers. Moving the test module
  to the end fixes the layout; the full gate must be rerun.
- old-syntax-check passed: 1012 snippets, 20 explicit future entries.

## Remaining stages

The display/known/coverage rules (stage 5), deferred Assert fix (6a),
pattern preparation (6b), literal inference (7), written types/narrowing
(8), joint fitted choices, performance scripts (9), and final soundness
review (10) remain unfinished after stage 4. Resume with stage 5; enable
the test-only set accessors when their production consumers are added.
Future entries must be removed when their syntax actually parses, never
in advance. Golden files must be generated in a final separate commit.

M4 handoff, outside this lane: literal-types §5.9 requires literal Syntax,
negative Int construction as a unary datum (including through groups),
TypeArg=Type, and Types.members expanding finite scalar sets into literal
items sorted by full shown text. Do not implement this by shortening or
reparsing hover text. The complete accepted rows remain in that section.

## Final stage-4 handoff

T1 is incomplete and stops at the committed stage-4 boundary. No source
literal inference, written literal parsing, new E0520 behavior, or literal
narrowing is enabled. The complete T1 corpus audit and performance budget
have not been run. This is not the finished T1 result for T2/T3 integration.

Required upstream merges were made without conflicts: C4 `f5cff7c`, B
`e1abcb6`, and path/sequence `12d4637`. Files outside T1's regions changed
only through these requested merges. The final checks below ran after
merging `12d4637`; redesign was checked again before handoff.

Final implementation checks:

- `ILANG_BROWSER_PORT=4181 ./scripts/dev-verify`: exit 0. This includes
  Rust fmt, workspace Clippy with -D warnings, workspace tests, library
  formatting, std/hello/bench at O0/O2, WASI smoke, JavaScript SDK tests,
  and 33 actual browser tests. Std passed 1399 tests at each level.
- The spec suite in that workspace run passed 159/159 cases at O0/O2.
  Earlier explicit `cargo test -p ilang-cli --test spec --locked` runs
  passed 139 cases before C4 and 140 after C4, both at O0/O2.
- `scripts/old-syntax-check`: exit 0; 1077 extracted snippets, exact
  expected diagnostics, 20 future-track entries. T1 entries intentionally
  remain until stage 8 implements their syntax.
- `WAT_DUMP=... cargo test -p ilang-cli --test wat_dump --locked`: 602
  outputs. Every byte equals the exact `12d4637` upstream baseline, with
  none added or removed. The archived baseline is under target/t1; all
  848 tracked blobs were verified against git. The four std additions
  were installed before its test started; its Rust sources are identical
  to e1abcb6. The Binaryen gitlink uses this lane's submodule.
- No type golden changed and no derived file remains changed.

The full verification log is `target/t1/dev-verify-12d.log`; old syntax is
`target/t1/old-syntax-12d.log`. WAT directories are `target/t1/wat-12d` and
`target/t1/wat-upstream-e1abcb6` (the latter contains the verified 12d4637
baseline, despite its original directory name). These are implementation
comparisons, not substitute performance measurements or proof of the
unfinished literal behavior. The interpretation checks and their source
sentences are recorded by stage above.

Failed checks and repairs:

- The first old-syntax check rejected the new host example before its
  required explicit T1 data entry was added. The final exact check passes.
- A mechanical field edit left trailing whitespace; cargo fmt fixed it.
- The first full gate failed Clippy's items_after_test_module. The solver
  tests were moved after the remaining helpers; final Clippy passes.
- The next full gate found a formatter skip-inventory difference for the
  unsupported host example. UPDATE=1 generated that line, without a hand
  edit. The following gate reached spec_blocks_parse and rejected that
  same not-yet-supported syntax. The example now has an explicit
  `i pending=T1` fence and a separate exact current-diagnostic test. The
  existing implemented-snippet parse/print assertion is unchanged, and
  old-syntax-check still parses the pending fence. Regenerating format
  tests returned all derived files to baseline. Stage 8 must remove the
  pending mark, its exact-diagnostic fixture, and its old-syntax entry.
- Two preliminary spec-edit scripts stopped with indexing errors before
  writing, and patch context mismatches made no changes. They were fixed;
  no partial repository edit was retained from those failures.
- No check was interrupted. No git configuration, lockfile, or dependency
  was changed. Environment overrides were command-local. Build artifacts
  and the archived baseline remain only in this lane's ignored target.

The accepted joint corrections control the older note's package-head
wording, Int|y example, conditional one-clause-choice wording, and T2-first
merge suggestion. Their corrected source sentences are listed in stage 1.
There is no unresolved source disagreement for the completed stages.

## Continuation through the remaining T1 stages

The user resumed this lane and requires completion of every done criterion.
Stage commits are checkpoints; the earlier stage-4 handoff is historical.
Each remaining stage runs the full gate before the next stage begins.
Generated files are kept for the final derived-files commit; their source
rules will be recorded separately, line by line.

### Stage 5: display, known facts, and coverage machinery

Norm joins/intersects/subtracts scalar sets, including the subset-based Top
rule. Exact ShownType trees contain canonical literal leaves; text() and
text_with() remain full, while diagnostic/hover text has the 12/10 limit.
Coverage keeps sets with heads and distinguishes unknown alternatives from
finite scalar information. The latent hidden-input/whole-literal walkers
are tested but remain test-only until stage 7, as §12 requires.

Interpretation sources: literal-types §5.7 says “Norm repeats the solver
rules” and specifies full versus limited text. §5.5 says “A side with other
... counts as all values for every head.” The coverage regression checks
both intersection orders with an explicit 0 beside unknown information
against the singleton 1. Decision 7 supplies the polarity walk and recursive
whole-literal rule; neither is enabled in source inference yet.

One new known-type unit test initially found a raw difference under an
intersection was skipped. Recursive member extraction fixed it. A syntax
error in the first mechanical edit was fixed before further checks.

Stage 5 gate: full dev-verify exited 0 (18 direct tests, unchanged type
goldens, 159 spec cases at O0/O2, 1399 std tests per level, 33 browser
tests). old-syntax-check passed 1078 snippets and 20 pending entries.
The initial partial unit run failed the raw-difference regression above;
the final full run passes. No golden was regenerated in this stage.

### Stage 6a: deferred fitted failures

Every non-catch-all local binding inside a function in a group gets its
own group-level effect variable in its current effect frame. After holder
linking and before generalization, coverage decides the saved row and
adds Assert only to that variable. Parameters and top-level patterns keep
their immediate decisions. Source: literal-types §5.3's “one fresh
variable e for that binding” and “after the holder link ... and before the
group is generalized”. Deferred rejection is E0501 at the pattern.

The note's immediate-call spelling `(() { ... })()` conflicts with §6.4's
“(x) is a tuple with one part, not a group” (also literal-types decision 5).
That expression first fails by calling a tuple. The regression instead
uses `{ () { ... } }()`, following §6.10's brace-group rule. This corrects
the example spelling and does not change tuple or binding semantics.

Gate: full dev-verify exited 0, 169 spec cases at O0/O2, 1399 std tests
per level and 33 browser tests. All 10 new may_fail_group cases passed;
old-syntax-check passed 1089 snippets. No existing golden changed.
The new may_fail_group.out is generated and reserved for the last commit.
Its first two effect lines are required by the holder and indirect-call
examples in §5.3; total_pair and f omit Assert by the total-pattern and
per-frame rules there. s/top retain whole String/Int until stage 7; written
keeps its annotation (§4.8 rule 4). Both diagnostic lines are exactly the
deferred E0501 required by §5.3, at the literal patterns, with Never from
the top-level and written function environment restrictions.

### Stage 6b: pattern preparation

Rule 11 now holds only Values failures until its own coverage check. Other
constraints still report E0501 immediately. Source: §5.1 rule 11 calls
Values a failure “only from value sets”, and §5.3 says “no failure is ever
lost”: check_case drops held failures only after reporting E0516.

Shared places are computed from written fields, expanding aliases and
counting occurrences across every constructor. Descendants of list items
and shared fields stay shared. The regression includes 80 nested tuples
around an aliased repeated parameter; only alias expansion consumes the
existing recursion budget. Source: §5.3 says “counting each occurrence,
also two in one field, and with aliases expanded”. Literal candidates
and their top-place-only fast path remain behind LITERAL_CANDIDATES=false.
The known filter subtracts actual types rather than dropping their heads.
No source inference creates restricted sets yet.

Gate: full dev-verify exited 0 (19 direct checker tests, 169 spec cases at
O0/O2, 1399 std tests per level, 33 browser tests). Existing type goldens
are unchanged. old-syntax-check passed 1091 snippets and 20 future entries.
The stage-6a generated output remains untracked for the last derived commit.

Failed checks: the first compile used owned alias arguments where subst
requires references; corrected. The first full gate found collapsible_if;
corrected. A superseded full run was interrupted after adding the deep
field regression, to prevent duplicate browser use of port 4181. The final
source ran the whole gate successfully. No persistent environment changed.

### Stage 7 source conflict

Literal inference was implemented experimentally with the required known
matching, hidden-input, whole-literal-bound, coverage witness and host
parameter rules. The required empty optional accumulator exposes a conflict
between §10's runnable example and decision 7 plus rule 13. The reduced
program, both texts, derivation, measured rejection, and successful annotated
substitute are recorded in `tests/t1-blockers/README.md`. The user requires
stopping when a controlling source must change. No amendment is assumed.

Stage-7 work is preserved under target/t1/continued/stage7-work and removed
from the branch's active sources. Stage 6b is the last verified feature
checkpoint. No stage 8 work began and no check was weakened. Generated
stage-7 output changes were regenerated back to the stage-6b state; the new
stage-6a output alone is retained for the final derived-files commit.

Failed experimental checks, all before restoring the source checkpoint:

- Initial coverage compile: Arc<str> items were converted with the wrong
  String iterator adapters; corrected before subsequent tests.
- Initial focused query tests: three failures from changed exact literal
  types, then a further drop/drop2 expectation; corrected in saved patch.
- Initial interpretation checks: two malformed fixture setups (missing
  header separator/std constructor import and a nonexistent export
  attribute). Corrected to a dependency manifest and wasm.exports. All
  eight interpretation checks subsequently passed in the workspace run.
- First spec run: 167/173 passed. Four existing runtime equality programs
  now hit mandatory E0520 and were routed behind parameters in the patch;
  two new files lacked std dependencies. The second run passed 172/173;
  its remaining fixture placed tests under src. Moving them under test
  exposed the actual E0506 source conflict in the workspace run.
- First full gate passed Clippy/library tests and stopped at a front-end
  assertion expecting Int instead of 1 | 2; corrected in the patch.
- Workspace no-fail-fast run: query and LSP expectations, one package
  interface expectation, and the E0506 acceptance case failed. Other
  targets passed. Text-only expectations were corrected in the patch;
  the acceptance case remains unresolved. No final stage-7 full gate was
  claimed. The stage-7 release build completed successfully.
- First reduced reproduction used one too many parent directories for
  std; corrected. Final reproduced E0506 is the only error. The amended
  reproduction ran at O0 and O2 with exit 0, explicitly a substitute.

Every operation and artifact stayed in this lane. There was no dependency,
lockfile, configuration or persistent environment change. The superseded
stage-6b gate is the only interrupted check. The initial interrupted worker
work was kept, completed and committed as cb83db1.

The only derived file committed on this continuation is the generated
`crates/ilang-check/tests/types/may_fail_group.out`. Its individual lines
are required by these sentences (not by regenerating an expectation):

| Output line | Requiring source sentence/rule |
|---|---|
| `pair : (String, () -> Int ! Assert)` | §5.3's self-holder example “gets Assert”; §12 says “before step 7 nothing makes a set other than ALL”, so its literals remain String/Int here. |
| `identity_pair : ((x: String) -> String, () -> Int ! Assert)` | §5.3: a value reaching the holder “only through the holder's upper bounds” must also be deferred; ordinary fitting constrains the identity's argument/result to String. |
| `total_pair : ((Int, Int), () -> Int)` | §5.3: “A binding that can not fail adds nothing to its e, which stays Never”; the total tuple example is named explicitly. |
| `f : (s: String) -> Int` | §5.3: “Every binding ... one fresh variable e for that binding”; the example with an inner Assert statement explicitly requires “no Assert in f's type”. Its written arrow fixes Int. |
| `s : String` | §12's before-step-7 ALL rule, applied to its literal expression. |
| `top : Int` | §12's same ALL rule, applied to the literal result of the immediate function call. |
| `written : () -> Int` | §4.8 rule 4: the body fits the written return/environment; the written arrow retains Int even when the binding is rejected. |
| E0501 at `28:3` | §5.3: “a failure of a deferred Assert <: e is one error E0501 at the binding's pattern”, with its specified exact text. Top-level environment is Never by §2.5. |
| E0501 at `34:2` | The same §5.3 sentence; the written function without Assert gives the Never upper bound by rule 4. |

No existing golden remains changed. The discarded experimental stage-7
outputs are retained only in the ignored audit patch and are not accepted
as expectations on this branch.

Final verification of the committed checkpoint after merging redesign
1ce1a7e (stream/URI) into lane/a:

- `ILANG_BROWSER_PORT=4181 ./scripts/dev-verify`: exit 0, including Rust
  fmt/Clippy/workspace tests, 169/169 spec cases at both O0/O2, 1422 std
  tests per level, hello/bench, WASI smoke, JavaScript SDK and 33 browser
  tests. Log: target/t1/continued/source-conflict-final-full.log.
- `scripts/old-syntax-check`: exit 0, 1096 snippets, exact diagnostics and
  20 future entries. Log: source-conflict-verified-old-syntax.log in the
  same directory. T1 future entries remain because its parser is not on.
- UPDATE=1 checker type generation: exit 0, five tests. No existing output
  remains changed; only the new deferred-Assert output is committed.
- The saved experimental binary checking the committed reduced program:
  exit 1, exactly E0506 at src/main.i:8:22. This confirms the source conflict;
  it is not a successful T1 stage gate. The annotated substitute ran at
  both optimization levels with exit 0, as recorded above.

Redesign was checked again at the end and remains 1ce1a7e. The source
conflict blocks stage 7 acceptance. Written types/narrowing, fitted choices,
full switched-on corpus audit, performance scripts/budget, final soundness
review and T1 future-entry removal are not done. No quiet-window performance
claim or finished-T1 claim is made. The coordinator must amend the §10
acceptance example or authorize different known-type rules before resumption.
