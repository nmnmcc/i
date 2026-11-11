# Macro integration status

This is an integration inventory, not a claim that all listed tests pass: the
merge must pass the commands below before the branch is usable.

| Track | Implemented paths and evidence in the tree | Missing or unverified |
| --- | --- | --- |
| M1 | Lisp parsing and normal/Lisp conversion; `lisp_syntax::records_parse_from_lisp_with_fields_spread_and_removal`, `source_syntax_agrees_across_surfaces_for_library_modules`. Syntax nodes, quoting and marks: `templates_splice_and_patterns_capture_without_losing_origin`, `expansion_runs_mark_arguments_and_freshen_templates`. | Full merged regression suite remains the gate. Conversion has deliberately unrepresentable normal-surface forms (meta §1). |
| M2 | Compiler wrapper and isolated services, imported expression and item expansion, hygiene and collection machinery have tests in `ilang-cli/tests/macro_run.rs`, including `cli_check_and_run_expand_imported_macros`, `cli_runs_item_macro_with_exposed_caller_name`, and `collection_manifest_from_a_real_item_macro`. Budget tests are in `macro_budget.rs`. | These tests must be run on the merged branch. Typed macros (M4) are not established by these syntax-macro tests. |
| M3 | Definition query and scheduling tests include `definition_schedule_retries_fresh_runs_after_a_contribution`, `definition_query_reads_expanded_type_from_dependency` and `cli_retries_definition_queries_after_later_item_macros`. | End-to-end behavior must be verified after integration. |

The test macro design in meta §4.5 can represent named entries through item
contributions and collection metadata, and imports are a general macro
facility. A suite macro can transform its body into individual tests carrying
the outer environment; variants can be encoded as metadata and generated
bindings. These are design capabilities, not completed acceptance tests or a
finished test library and runner. In particular, selection, expected failure,
and parameter expansion need library/runner implementation and end-to-end
verification. The user's four requirements are therefore possible in the design
but not demonstrated as a complete working feature here.

The two original heads contain parallel traversal and definition-encoding
implementations. Integration must retain one traversal and one definition
encoder with the driver's query interface and T3 record cases; competing versions
are not independent features. Any remaining dead or duplicated paths require
an audit after the merge compiles.

Required gates: `cargo build -p ilang-cli`, `cargo test -p ilang-macro -p
ilang-syntax -p ilang-check -p ilang-wasm`, and `cargo test -p ilang-cli --test
spec`, using the shared heavy wrapper.

## Merge blockers

The current branch does **not** build. Reproduce with `cargo build -p
ilang-cli`: the checker has lost the driver's `TyParts`, `order::references`,
selected checking, scoped HIR and quote-pattern interfaces while other merged
modules still call them; `hir_tree` also conflicts with derived HIR clones.
These need a semantic merge with the T3 record representations, not another
whole-file side selection. `cargo test -p ilang-macro -p ilang-syntax -p
ilang-check -p ilang-wasm` additionally fails to compile the syntax migration
example because `parse_module_legacy` is missing. Restore the legacy parser
behavior alongside the T3 parser before using that example.

All optional heads were trial-merged and aborted after conflicts: `macro-test`,
`macro-items`, `macro-entry`, `macro-walk`, `macro-ir`, `macro-ast`,
`macro-ty`, and `macro-infer`. The first five conflicted in one file each, the
last three in two each. They have not been validated against a passing base.
The current recursive AST walk is a single implementation, but it has not been
verified against the deep tests; no deep test was removed. The broader dead-code
and duplicate-code audit is unfinished.
