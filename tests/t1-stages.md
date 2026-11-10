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

### Resumption: coordinator ruling and stage 7

The coordinator (2026-11-10) keeps decision 7 and spec rule 13, and amends
only the §10 None-accumulator example to `Some(p: (Int, Int))`. That ruling
is quoted beside the edited acceptance row. The preserved stage-7 source
patch and seven new files were restored. The reduced unannotated source
stays unchanged; an interpretation test asserts its entire sole E0506
message. The tuple-shape-from-inference question remains open for a later
track, with no change to rule 13 in T1.

New production behavior follows decision 4 (no widening), decision 5
(group-transparent negative literals), decision 6 (whole scalar patterns
when fitted/shared), decision 7 (unknown matching, hidden inputs, whole
known bounds), §5.5 (listed-value/complement coverage and witnesses), and
§5.8 (inferred host inputs must be whole). Long private hovers also use
limited text; stored schemes and interfaces stay exact, as decision 11
requires. The new scheme_hovers map contains human text only.

K2 from report-std1b §5 was reproduced with literal patterns enabled: one
E0501, Int not a subtype of Never. Its cause was ours: an unguarded list
rest covered later item positions at runtime but did not mark those item
places as catch-alls in rule 11. Record the earliest rest index, propagate
to existing items and to later-created item places. Source: §5.3 says a
catch-all “at the place or above it” supplies rule-11 slack; a list rest
is a catch-all at each remaining item. The exact reported program and a
runtime test of all paths now pass. Guarded rests do not grant slack.

Stage-7 interpretation checks also cover negative groups in SCCs, fitted
scalar bounds, immediate versus delayed Values failures, exact E0520
operators, list/shared-field coverage, once-occurring Option parameters,
prefix witnesses, inferred export rejection and the exact E0506 ruling.
Runtime tests exercise scalar uses, scalar/tuple/Box/empty-list/annotated
None folds, grouped/chosen callbacks, continuation/setter inputs,
contravariant Sink, returned functions and own/callback service inputs.

The corpus script compares explicit before/after binaries across package
roots and standalone checker/spec fixtures. Stage 7: 272 cases, 265 kept,
6 new, 1 newly rejected: types/spec.i's `assert(1 == 2)`. It is required by
§8's named E0520 case and decision 8, not a lost correct program. All std,
hello, bench and browser packages keep checking. Std has 1989 type lines;
exactly 10 constants change, as §8 predicts. All other interface lines are
unchanged. Artifacts: target/t1/continued/stage7-corpus.{json,log} and
std-{before-stage7,stage7}.types. The baseline binary is the actual
pre-stage-7 checkpoint, copied before the release rebuild.

Existing runtime equality tests named by §8 keep their operands behind
parameters; stop traces still pass. The scalar missing-value diagnostics
and finite coverage changes were read individually. The generated audit
lists every changed golden line with a controlling rule; it is an audit,
not proof by regeneration, and remains for the final derived-files commit.

Failed checks on this resumption: K2 failed before its fix; an import in
one added Rust source fixture lacked the outer header group and was fixed.
The first old-syntax scan found two generated TOML strings and one rendered
diagnostic fragment; explicit not_source entries classify only those host
strings. The final scan passes 1122 snippets and 20 future entries. No i
source was exempted. No check was interrupted. The first full stage-7 gate
passed (174 spec cases); the expanded final gate includes the added tests.

Final stage-7 gate: full dev-verify exit 0, 175 spec cases at O0/O2,
1422 std tests per level and 33 browser tests. Clippy -D warnings, the K1
fixtures, 13 interpretation tests and all existing Rust targets pass.
Log: target/t1/continued/resumed-stage7-verified-full.log. Source stage 7
is committed with all generated outputs held for the final derived commit.

### Stage 8: written literals and narrowing

AST/HIR Lit use one shared LitValue (signed BigInt or decoded String).
TypeArg is TypeExpr, as types-joint §5 requires. The grammar accepts signed
integer primaries through brace groups and rejects Float with the existing
E0101 message “Float has no literal types”. Literal leaves introduce no
names, variables, variance occurrences or dependencies; the resolution,
substitution, place/variance and query visitors explicitly preserve that.
The written and inferred interface paths retain full values. Query-only
alias/value hovers have the 12/10 limit; the 14-value alias publication test
changes a value beyond that limit and requires a new major version.

Type tests now carry All/Only/Except sets through the existing TestMember
contract into the previously staged equality/tag-and-negation lowering.
Rule 12 is enabled only with these tests: actual top rows justify the fast
path; nested rows use usefulness across every column. Literal choices,
cofinite tests, guarded rows, tuple/list cross-columns and prefix-after-
Except all have accepted/rejected checks. Written finite list and repeated
Pair parameters run every alternative; wrong-tag and boxed Int tests run
at O0 and O2. Unicode/canonical integers, NUL witnesses, literal-negative
differences, noncovariant known bounds and cofinite prefix remainders are
also checked.

The host gate follows decision 9: “Where the host gives a value ... must
have all its values.” Alias/union flattening accepts the listed outgoing
scalar forms; cofinite outputs retain their scalar ABI. A same-head union
containing a whole head is whole and may be received. Literal returns of
externals/host services are rejected, with existing E0318/E0320; inferred
export inputs use E0521. Provider fitting remains ordinary subtyping,
while service use compares equal sets (E0515 for Counter(0)/Counter(Int)).

T1's three future entries are retired. The note's mixed statement example
needs a normal function-body prefix/suffix because it contains bare
expression statements; it remains parsed in full and has no future marker.
Mixed T1/T2/T3 entries now belong to T2/T3. Remaining reserved-record
recovery diagnostics reflect Literal Prim consuming scalar annotations
before expecting an arrow; the record rejection itself is unchanged.
The parser's former negative `x: 1 -> 2` test is now a positive clause
assertion, as replacement §6.1 requires. No i source is skipped.

Changed goldens: existing value/type lines retain stage-7 rules. Rule 12
now changes narrowing.literal's result to String | Int - 1. The syntax
err_record diagnostic now expects an arrow after consuming literal 1;
Literal Prim, not regeneration, requires that change. UPDATE=1 generated
all outputs; the audit inventory assigns each line its source rule.

Checks/failures: early compiles found missing Lit leaves in the place and
variance visitors, then passed after adding them. A prefix test incorrectly
forbade the warning on a redundant final wildcard; it now proves the second
clause useful by checking the exhaustive two-row form without that wildcard.
The first full gate passed 183/184 spec cases: an external fixture tried a
service-kind test provider. It now uses the supported external math provider;
the separate finite-string service test also runs. The next full gate passed
184 cases, then two complete runs added the remaining finite/shared and
written-bound cases (185 and 186). No check was interrupted.

Final full dev-verify: exit 0, 186 spec cases at both O0/O2, 1422 std tests
per level, 33 browser tests; Clippy -D warnings and all Rust targets pass.
Log: target/t1/continued/stage8-final-full.log. old-syntax-check: exit 0,
1161 snippets, exact diagnostics, 17 future entries, none owned by T1.
The corpus audit again finds only types/spec.i's required E0520 as a lost
accepted checker fixture. Four original runtime fixtures, before §8's
parameter rewrites, were separately checked with both binaries: all were
accepted before and rejected with E0520 after, exactly as §8 lists:
11/equality, 11/stop_tail_service, 6/blocks and 6/strings. Their current
runtime versions still pass. Artifact: original-runtime.json under the
continuation target directory.

### Mandatory fitted choices

Joint J5 says “A one-clause block with choices is fitted and can need
Assert; it does not become an exhaustive match.” Spec §4.8 fitting says
“choices with the same type constructor use the same new type variables”,
and §6.12 says their parameter has no label. The ordinary case visitor
therefore shares places and unions binder types, with a fitted flag:
scalar patterns keep their whole head; failures are ordinary constraints;
coverage and original-subject constructor flow do not apply. The union of
choice rows goes through the existing deferred binding decision in its
own effect frame. Lowering uses the same AssertionFailure path as a fitted
binding and the whole choices span, beginning at its first pattern.

The old E0235 path is removed. The required pending Result unwrapping
case is now a passing spec case. Independent checks require `(Int) -> 2
! Assert` for `{ 0 | 1 -> 2 }`, unioned binders, monomorphic fitted
binders, no constructor flow, and rejection of opaque Box(Int) tests on
Box(Int | String). Runtime cases cover both successes, deferred groups,
inner/outer handlers, written no-Assert rejection and the exact default
failure text and positions. A two-clause equivalent still needs coverage.

Development failures: the first interpretation/old-syntax checks found
malformed test choices with two type tests. Spec §7.1 says “A `|` after a
type is part of the type, so a type test is written after the other
choices”; the fixtures now put a literal before the test. Two focused
runtime runs expected a caller position for a tail call. §11.7 says “Calls
that were replaced by tail calls are gone”; the position fixture now makes
the call non-tail to verify both the pattern position and caller position.
These were expectation errors; no compiler check was weakened.

The old-syntax run also saw a concurrent dev-shell enterShell failure:
`.devenv/load-exports` disappeared during chmod in the shared environment.
The slot's subsequent dev-shell entry succeeded and the parser scan
returned 0 (1177 snippets, 17 other-track futures). No environment or
configuration was changed by this lane. Focused interpretation checks:
23/23 pass after the syntax correction.

Final fitted-choices full gate: exit 0, 198 spec cases at O0/O2,
1422 std tests at each level, 33 browser tests; fmt, Clippy -D warnings
and every workspace Rust target pass. Log: choices-full.log under the
continuation target directory. No golden changes beyond the held stage
7/8 outputs; no check was interrupted.

### Corpus and performance

The switch-on corpus audit after fitted choices has 295 cases: 265 kept,
29 new, one lost checker fixture (types/spec.i, `assert(1 == 2)`, required
E0520 by §8). No correct existing package is lost. The four original
runtime equality fixtures have the separately recorded required E0520
changes; their parameter-based replacements still run at both levels.

Performance sources: §9 says “k-way join”, “bound fusion”, “Cheap Eq and
Hash”, “Narrowing fast path”, then interning only if those are not enough.
The new scripts validate warm-ups/check exits, use one warm-up and 11
sequential samples per binary, retain hashes/load/affinity/all samples,
and enforce every numeric budget. The note's 1865366 revision has old
syntax: the supplied new-syntax 1df2fe0 binary is the explicitly labelled
substitute, checking exactly the same current sources. It accepts std.
The baseline generalized-calls problem has ticket T1-P1 in
`tests/t1/performance.md`; it is not conflated with the monomorphic gate.

The four ignored stresses pass at peak two bounds per head per side,
including all concrete heads, recorded at insertions and copied-bound
installation. The calls check proves the local binding is indirect and
all 5000 literals reach its one parameter. The first debug run measured
39/74 ms for 2500/5000 calls (a debug measurement, not the release gate).

Development checks: the first counter compile hit a Rust borrow error;
it now computes a local peak before updating the counter. A busy-machine
exploratory std run was +19.6%, not the quiet gate. Callgrind returned 0
and warned about its brk segment limit, then produced an instruction
profile (1.580 billion instructions); this is profiling evidence, not a
timing substitute. The read-only Norm occurrence walk was deep-cloning
its whole input at every recursive node. It now borrows that slice;
K1's invariant/environment traversal is unchanged, and all type goldens
and independent interpretation checks remain green.

The first scripted run passed std but failed keys' 2× ceiling (2.64×)
and pick's doubling limit (2.80×). No threshold was changed. Bulk finite
joins now build one tree, using a sorted singleton batch or a heap merge
of sorted streams; persistent incremental fusion remains in add_bound.
The independent infinite-domain model checks every triple combination.
Narrowing inspects the already-recorded earlier row and hashes taken
literal candidates. A top-level scalar-row prefix index avoids repeated
matrix scans for scalar literals and a closing catch-all. Other queries
retain the full matrix, and no solver constraint is added by the index.

The matrix oracle initially found a difference for a finite subject with
an earlier literal and a later prefix. That optimization was restricted:
prefixes, complements and structural tests retain the ordinary matrix's
conservative decision. A subsequent compile found a missed function
signature argument during that correction; it was fixed. The final
oracle agrees on usefulness and exact missing witnesses across finite,
cofinite, mixed, unknown and empty columns, choices and a closing wildcard.
Runtime/rejection tests separately require that guarded literals never
cover later rows. No golden was regenerated for these optimizations.

The first performance full checkpoint passed all gates (200 spec cases,
1422 std tests at each level, 33 browser tests). A final narrowing
optimization then skips a whole-head matrix query only when the actual
top rows are a finite scalar prefix: finitely many Int/String literals
cannot cover an infinite whole scalar head under Any columns. The matrix
oracle explicitly checks that fact. The final stage gate is repeated
after this addition; the earlier checkpoint is not claimed for it.

The timing harness now measures background CPU ticks minus benchmark
child CPU time. `--require-quiet` makes at most 5% background machine
capacity an explicit measurement gate, an operational interpretation of
§9's “quiet window”, separate from the unchanged timing ratios. A manual
failed-warm-up check found an argparse percent-sign help-string error;
the help now spells “percent”. A retry also found that `/bin/false` does
not exist in the Nix environment; the verified run discovers the actual
executable in the dev-shell PATH. It returns nonzero and records the
failed warm-up rather than timing it as a success. No system path or
configuration was changed.

Final performance implementation full gate: exit 0, 200 spec cases at
O0/O2, 1422 std tests per level, 33 browser tests; fmt, Clippy -D warnings
and every Rust target pass. Log: perf-final-full.log. old-syntax-check:
1183 snippets, 17 other-track futures, exit 0. The implementation is a
committed checkpoint, not a claim that the numeric gate has passed.
The order of the remaining work is soundness review/full gate, merge of
latest redesign, final full gate, quiet timing gate on that merged binary,
and final derived generation. Timing preparation is green under the full
gate before soundness work starts. The quiet timing criterion stays open;
the running sampler has seen 15–94% background CPU and has not timed an
invalid window. This schedules the environmental measurement last rather
than leaving the authorized soundness review idle while other lanes build.

### Soundness review and final syntax retirement audit

The rule-led review in tests/t1/soundness-review.md is independent of the
goldens. Ten new rejection/runtime/stop probes use strict helpers and
actual matching host providers, so false acceptance would be observable
as unreachable or a wrong handler. The focused spec command passes all
ten at O0/O2. Its first run failed three malformed fixtures (a tuple
clause after a numeric body on the same line was parsed as a call,
E0106); separating clauses fixes them. The other seven passed initially.
No new compiler hole was found and no implementation rule was changed.

The review preserves the coordinator's exact None-accumulator amendment
and E0506 rejection. K2 is fixed in the owned place preparation, with
both the exact source check and tests/spec/7/guarded_list_heads.i running.
The user's later-track question about rule 13 obtaining tuple shape from
inference remains open, with no implementation here. The accepted note's
constructor-family-changing callback limitation remains explicitly outside
the scalar widening claim. K1's invariant traversal is unchanged.

The explicit final JSON inventory audit finds no future/track entry
containing T1. The scan passes 1193 snippets, exact diagnostics and 17
other-track futures. The retirement was implemented with written literal
syntax in stage 8 and has remained green through the final review.

Soundness full gate: exit 0, 210 spec cases at both O0/O2, 1422 std
tests per level, 33 browser tests; all workspace Rust checks pass. Log:
rf-full.log. No implementation change was needed for the ten new probes.
The golden-rule generator now records its exact comparison revision; the
final audit will compare the post-merge source checkpoint so unrelated
upstream golden additions are not attributed to T1.

The quiet-window sampler was intentionally terminated before the final
upstream merge. It had no measurement child, no final timing reports, and
had only sampled busy windows (up to 98%). This interrupts a waiting
measurement gate, not an executing language/Rust test. It will be restarted
against the merged sources and final binary. No process outside this
lane's sampler was signalled; no environment setting was changed.

### Final upstream integration

Merged refs/heads/redesign through 50c44a9, including Std-2 and the JS/WASI
source amendments, at 4472a97. The sole conflict was the shared insertion
point in spec §10.3. Both T1's directional scalar paragraph/example and
the upstream request-declaration paragraph are retained, in that order.
This was a merge conflict, not a language contradiction. Git's automatic
conflict footer initially followed the co-author trailer; the merge
message was immediately amended to restore the required final trailer.

Before merging, 33 generated outputs were archived as an ignored patch
and restored to their tracked versions. UPDATE=1 then regenerated them
through the checker/syntax generation commands; no golden was hand-edited.
The final audit compares against source checkpoint 4472a97, and assigns a
rule to every one of 419 changed lines across 33 outputs. The only parser
recovery change is consuming literal 1 before expecting its arrow.

Merged full dev-verify: exit 0, 219 spec cases at O0/O2, 1446 std tests
per level, 36 browser tests; fmt, Clippy -D warnings and all Rust targets
pass. Logs: final-full.log, final-generated.log, final-old-syntax.log.
old-syntax-check parses 1298 snippets with exact diagnostics and 17
other-track futures, none T1. WAT export passes (832 files); the separate
wat-review.md gives exact byte counts, provenance and residual classes.
The optional slow Python std unified-diff analysis was terminated and
replaced with GNU diff; the complete byte inventory was already done,
and no actual test or WAT generation was interrupted.

Final corpus audit: 316 cases, 265 kept, 50 new, one lost pre-existing
checker fixture, types/spec.i's `assert(1 == 2)` (the required E0520).
The actual before binary is the archived pre-stage-7 checkpoint. The
four original runtime fixtures have their separately verified §8 E0520
changes; all parameter-based replacements run. The supplied 1df2fe0
binary also checks the expanded current std, so the final performance
comparison can keep the same source tree for both binaries.

A lower-priority source still has stale prose: syntax.md §6.1 says
`{ x: 1 -> 2 }` is a records error. Literal-types §6.1 expressly replaces
that sentence; spec §6.1 and the parser implement the replacement. The
accepted note wins under this task's source order. This design-note
cleanup is outside T1's assigned implementation/spec paths and is left
for its owner; it requires no compiler or normative-spec change.

Upstream moved during final verification. Binding grants (99a030e) were
merged cleanly at 5f53999; its full gate passed 219 spec cases, 1446 std
tests per level and 51 browser tests. Std/property and string additions
(94789e6) then merged cleanly at dae12ce. The final full gate again passes:
219 spec cases at O0/O2, 1465 std tests at each level, 51 browser tests,
fmt/Clippy/all Rust targets. Log: final-std3-full.log. All T1 code and
K1's invariant traversal are preserved across the automatic lower merge.
The upstream Cargo.lock change is retained as part of binding grants;
T1 did not update dependency versions or global configuration.

Final old-syntax scan: 1309 snippets, exact diagnostics, 17 other-track
futures, none T1. Final sanctioned regeneration after binding grants is
green, with the same 33 golden outputs; their audit now compares dae12ce.
The temporary relative WAT export directory under the CLI crate was moved
unchanged into the ignored root target directory and its empty parents
removed. The repeated absolute-path WAT export passes; both have 832
files. Final byte counts and all source-inventory differences are recorded
in wat-review.md. No untracked artifact directory remains in a crate.

The first reserved-window timing attempt met the numeric std gate (+6.8%)
but failed the independent quiet gate (17–19% background CPU). A second
attempt after binding grants likewise met the numeric std gate and failed
the quiet gate (~14%). Reports are retained; neither is final performance
evidence. The second retry wait was cancelled before the Std-3a merge,
with no perf.py/cargo child active. The next original-baseline warm-up
correctly failed: 1df2fe0 lacks the i/random Generator/split/at_path API
that the new property module imports. The harness recorded all 28 E0203
errors and returned nonzero; it did not time this failed check.

The compatible before binary is a copy of the other lane's pre-T1
94789e6 release build. All 910 tracked blobs of its source tree match
that revision; the binary is newer than those sources and its SHA-256
is preserved. Its control `x = 1` shows Int (T1 is off) and it checks the
same latest std. This borrowed build is an explicitly labelled substitute
for 1865366/1df2fe0, not a build performed by this lane. It includes none
of T1's latent representation changes, unlike the stage-6b binary.
Provenance and verification are saved in borrowed-baseline-provenance.json
and final-compatible-baseline.log under the continuation target directory.

The compatible-baseline corpus audit agrees: 316 cases, 265 kept, 50 new,
only the required types/spec.i E0520 lost. Std has 2197 type lines; exactly
ten constants change to their literal types and all 2187 other lines are
byte-identical. Source: no widening / literal expression rule (§4.2),
exactly the ten constants listed by §8. All four ignored stresses were
also rerun after integration and passed at peak two bounds/head/side.

### Final performance gate: met on the compatible pre-T1 baseline

The unchanged aggregate gate passed on attempt 18, using the verified
94789e6 before binary and the final T1 release binary, pinned equally to
CPU 6. Each binary has one warm-up and eleven samples per case. The
coordinator's quiet wrapper reserves all three heavy slots; the separate
background-CPU measurement also passed (std maximum 2.421%, stress maximum
3.407% of machine capacity). The extra 5% measurement threshold was never
relaxed. Original comparison to 1865366 is not claimed: 94789e6 is the
explicit compatible substitute, for the reasons/provenance above.

| Case | Before median ms | T1 median ms | Required ratio / result |
|---|---:|---:|---|
| std | 283.351 | 307.156 | 1.08401, +8.401%: within +10% and +18% ceiling |
| keys 250 | 6.310 | 7.262 | recorded sample size |
| keys 500 | 6.262 | 8.570 | recorded sample size |
| keys 1000 | 6.495 | 10.025 | 1.54357× before, doubling 1.16979× |
| pick 250 | 12.991 | 8.923 | recorded sample size |
| pick 500 | 31.867 | 11.511 | recorded sample size |
| pick 1000 | 106.633 | 20.714 | 0.194253× before, doubling 1.79951× |
| local calls 2500 | 12.720 | 20.523 | shared monotype verified by counter test |
| local calls 5000 | 19.060 | 36.883 | doubling 1.79713×, within 2.5× |

All four ignored tests then passed through the heavy wrapper, at peak
two bounds per head per side. Complete samples, binary hashes, background
measurements and test log are in
`target/t1/continued/final-verified-performance-18/{std.json,stress.json,ignored.log}`.
The script command is `tests/t1/perf-gate.sh BEFORE AFTER REPORT_DIR 6`
through the dev shell. It releases the quiet locks before the Rust tests.

Attempts 1–17 with this baseline were rejected measurement windows, with
all reports retained. Some numeric ratios also fluctuated in those busy
windows; none is substituted for the valid experiment. A separate early
stress run met every numeric limit but failed its quiet gate (8.195%).
The final pass meets both quality and numeric gates together. The earlier
async request for quiet-window availability is resolved; no user action
is needed. A read-only before/during/after diagnostic was queued while
investigating the background load; it changes no measurement condition.

Every implementation stage is now complete, with full functional gates
at its checkpoint, followed by latest-upstream verification. Final source
head before the report/derived commits is dae12ce, containing redesign
94789e6. All stages keep their independent interpretation tests. The final
coordinator follow-ups are the stale syntax.md example, the required later
rule-13 language question, existing callback-family question 8, baseline
call-scaling ticket T1-P1 and upstream K3 metadata cost. No action outside
T1's owned paths was taken for those follow-ups.

The read-only load diagnostic completed: its before/during/after CPU
fractions were about 8.1/7.2/7.0%, confirming that the surrounding workload
varies between windows. It did not change the estimator or threshold.
The accepted attempt 18 is a separate, actually quiet experiment. Final
verification and generation sessions all returned exit 0. The final
report-only old-syntax scan also checks the completed audit documents.
No global environment changes remain; build/audit artifacts stay in the
lane's ignored target directory, and upstream dependency changes stay as
intended merged changes. Only the documented waits/optional diff analysis
were interrupted; every required implementation and runtime check finished.

### Independent review: normalized host boundaries

Started from clean 7993590. The review's four failures were reproduced
with the previous release binary, then fixed using the existing scalar
set algebra. Exact sources, independent set identities, regression tests,
source quotes, the written-form audit inventory, and development failures
are recorded in [review-followup.md](t1/review-followup.md). The obsolete
syntax-design example is now corrected with the governing design quote.
Final upstream integration and gate results follow here after the run.
