# M4 typed macros

State surface: none. `Types`, `Truth`, `TypeHandle` and `Node.TypeView` are the
requested compiler syntax/query API; they introduce no mutable i values or
bindings.

Implemented declaration/implementation pairs, ordinary signature checking
(including labels, pipes, expected types, error and service rows), the SCC/T2
barrier, outside-in expansion, checked input holes and disposable frozen-solver
validation. Only fresh output variables acquire bounds. Successful output and
lowering facts commit without re-inferring caller schemes. Runtime order is
recomputed after installing output; consumer-selected checks restore the
original calls and order first. Dropped arguments keep their first diagnostics
and signature effects. Dropped inner macros do not execute; retained and newly
introduced typed calls expand under the same restrictions.

Queries preserve scoped nominal/builtin identities and opaque frozen variable
identities. Simple products use surface record syntax; choices, suspended
record operations and other graphs that need handles use `TypeView`. Members
keep a record graph whole. The cache-key API writes the exact scheme graph
bundle with bounds, obligations, fixation and observation metadata, never a
printed type. No typed-result answer cache is enabled.

Typed results reject result-type changes, contextual result-only guesses,
delayed effectful inputs, item/type/value-position uses, new direct value-type
reads, unrelated runtime dependencies and non-ancestor value aliases. Generic
type observations remain opaque and return `NotKnown` when unproved. Query
proof exhaustion stays uncatchable E0546.

`i expand` renders and traces typed output. LSP argument hover and definition
use the signature and retained binding identities; macro hover also includes
its checked expansion. `examples/macro-tests/src/typed.is` implements ordinary
library `to_text` and `assert_eq`, used inside syntax-generated collected tests.
The normative text is spec §15.7; cases are under `tests/spec/15.7`.

## Sources and interpretations

- Followed Lean's saved elaboration state and original-syntax contexts
  (`Lean/Elab/Term/TermElabM.lean`, `Lean/Elab/Macro.lean`), Scala's typed call
  and generated-tree validation (`dotc/inlines/Inliner.scala`), and Racket's
  already-expanded input and phase-aware lookup
  (`expand/local-expand.rkt`, `expand/syntax-local.rkt`), all in `.rd/ref`.
- Meta §7.3 T2's “each argument type” includes the checked argument types as
  well as the instantiated parameter types. The same observation substitution
  is applied to both sides of the additional closed-component validation,
  after proving against the original signature. This preserves differential
  nested forwarding without adding bounds to frozen variables.
- Meta T3/T4 control the older wording about closed result variables: all
  snapshot variables are frozen. The procedure is deliberately conservative.
- The source extractor's `A{index}` literal is a Rust identifier-formatting
  template, not an i module. Its one `not_source` entry records that reason.

## Evidence

Rust integration tests include `typed_macro_identity_principal_graph_differential`,
`typed_macro_forwarding_principal_record_graph_differential`,
`typed_macro_forwarding_effect_rows_graph_differential`,
`typed_macro_nested_forwarding_runs_retained_inner_call`,
`typed_macro_result_can_introduce_another_typed_call`, and
`typed_macro_library_inside_syntax_generated_test_runs`.

Solver tests include `typed_macro_graph_bundle_gate_preserves_sharing_and_obligations`,
`typed_macro_query_cache_uses_graph_bundle_and_fixation_state`,
`typed_macro_proof_cache_replay_has_same_result`, frozen edge/bound, cycle and
difference proofs. Syntax tests reject construction and text round trips of
TypeView. `typed_macro_hover_and_definition_through_call` exercises the LSP.

Verification results will be recorded after the final acceptance run.
