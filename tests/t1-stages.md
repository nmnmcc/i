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

## Remaining stages

The literal algebra, Con field, solver/display/coverage rules, deferred
Assert fix, literal inference, written types/narrowing, fitted choices,
performance scripts, and final soundness review are not enabled by stage 1.
Future entries must be removed when their syntax actually parses, never
in advance. Golden files must be generated in a final separate commit.
