# Tests that pin the Lisp rows from before syntax three

Integrator note (2026-11-13): this is the helper's conversion inventory,
including planned post-flip expectations. Before integration, pending edits
restored tests of the old encoder, scoped reader, built-in definitions and
`show_as` to the old rows while their production switches remain off. The
new-reader tests use `syn3::lisp` directly. Evening integration enables the
post-flip fixtures with `ROWS3`; the live gate results are recorded in
`syn3-final-ast.md` §6. The tables below retain the helper's historical
before/after measurements. Legacy encoder tests still exercise the old
encoder directly; production macro arguments use the new builder.

Status: inventory and conversion for step 2c (the rows flip) of
`docs/design/syn3-final-ast.md` (lane/faF, 2026-11-13). Controlling source:
`docs/design/syntax-3.md` §6.7 and §6.8.

About 150 tests held Lisp text in the rows from before syntax three. This
file lists them, says which failed on lane/fa (ca08e016) and why, and what
each became. A test is in one of three states after the conversion:

- **passes now**: its text is in the rows of §6.8 and it passes on lane/fa
  today, before the flip. Written `.is` code is read in the rows of §6.8
  since step 2a, so a test that only writes a `.is` file is in this state.
- **after the flip**: its text is in the rows of §6.8 and it can pass only
  when the macro engine's arguments, results and answers are in those rows.
  §3 lists these tests with the error each gives today. None has `#[ignore]`.
- **not changed**: it has no row that changes, or it tests a function that
  step 2c deletes and another part replaces (§4).

## 1. The rows that changed in the tests

| Before syntax three | §6.8 | Short name below |
|---|---|---|
| `(-> (x y) R e)`, `(-> () _ e)`, `(-> (T) R)` | `(-> (, x y) R e)`, `(-> (,) _ e)`, `(-> (, T) R)` | fn |
| `{ (-> p e) (-> (? p g) e) }`, `(-> (\| p q) e)` | `(\| (-> (, p) _ e) (-> (? (, p) g) _ e))`, `(-> (, (\| p q)) _ e)` | clauses |
| `(\| x { … })`, `(. x { … })`, `(\| x f)` | `(\|> x (\| F1 F2))`, `(\|> x f)` | pipe |
| `(-> (x (! it T)) _ e)` | `(-> (, x) (<- _ (, (: it T))) e)` | env |
| `(-> (x) (^ R E) e)` on a function literal | `(-> (, x) R e)`: the needs of a function literal are inferred | err |
| `()`, `((: x v))`, `(.. r (: x v))` | `(,)`, `(, (= x v))`, `(, .. r (= x v))` | record |
| `(T {C1 C2})`, `((Box A) {(Box A)})` | `(= T (\| C1 C2))` and one item per member, `(= (Box A) (, A))` | ctors |
| `(: (f A) (-> (A) A))` | `(: f (-> [A] (, A) A))` | tparams |
| a function body `{e}` | `e`: a body of one expression is that expression | |
| a name of a data type `(= (Box A A) (, A A))` | `(= (Box A A) { (, A A) })`: the `{ }` of the item form | |
| a label in a function type `(-> ((: left Int)) Int)` | `(-> (, Int) Int)`: a function type has no parameter names | |

How the text was converted: a string that holds a whole `.is` file went
through the migrator (`migrate-syntax3 convert`, which reads the old rows
with `lisp::parse_source_legacy` and writes the tree with
`syn3::lisp::to_lisp`); the two parts that `declared(signature, body)` of
`typed_macros.rs` joins went through it joined and were cut again; single
datums and the expected values were converted by hand with the table above.
A string that the migrator reads as syntax three already was left as it is,
so the conversion can run again.

## 2. The tests, file by file

Columns: "Old rows in its text" names the rows of §1 found in the test's text
at ca08e016 (and in the shared texts it uses); "On lane/fa before" is the
result of the test at ca08e016; "Converted text" is the result of the test as
it is now. The results before the conversion of `ilang-syntax`,
`ilang-check`, `ilang-macro`, `macro_run`, `typed_macros`, `lisp` and `fmt`
were measured with the test binaries of the `lane/fa` worktree (built there
from ca08e016 plus its uncommitted work of 07:39 to 08:10); "not run" means
no such binary existed. A test that is not listed embeds no old row, was not
changed, and passes before and after.

### `crates/ilang-cli/tests/macro_run.rs`

57 tests; 47 embed rows from before syntax three; on lane/fa before the conversion: 47 failed; 47 changed here; with the converted text today: 0 fail.

| Test | Old rows in its text | On lane/fa before | Changed | Converted text |
|---|---|---|---|---|
| `quote_pattern_selects_original_syntax_and_falls_through` | fn, clauses, pipe | fails: macro_run.rs:51:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `expression_quote_is_compiled_and_keeps_unquote_identity` | fn | fails: macro_run.rs:185:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `quote_splice_preserves_order_and_reuses_original_syntax` | fn | fails: macro_run.rs:220:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `prepares_and_executes_a_real_macro_binding` | fn | fails: macro_run.rs:254:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `compiler_program_selects_only_the_macro_import_closure` | fn | fails: macro_run.rs:292:5: assertion failed: diagnostics.is_empty() | yes | passes now |
| `compiler_wrapper_reports_a_macro_syntax_error_with_original_position` | fn, err | fails: macro_run.rs:364:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `compiler_wrapper_returns_a_successful_macro_value` | fn | fails: macro_run.rs:411:5: assertion failed: diagnostics.is_empty() | yes | passes now |
| `compiler_wrapper_rejects_a_macro_with_the_wrong_result_type` | fn | fails: macro_run.rs:486:5: assertion failed: diagnostics.is_empty() | yes | passes now |
| `compiler_wrapper_buffers_items_and_notes_for_success` | fn, env | fails: macro_run.rs:522:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `expression_macro_requiring_items_is_rejected_before_execution` | fn, env | fails: macro_run.rs:581:5: assertion failed: diagnostics.is_empty() | yes | passes now |
| `expansion_preserves_sharing_between_output_and_item_request` | fn, env | fails: macro_run.rs:618:5: assertion failed: diagnostics.is_empty() | yes | passes now |
| `separate_runs_get_fresh_marks_and_leave_argument_scope_unchanged` | fn | fails: macro_run.rs:701:5: assertion failed: diagnostics.is_empty() | yes | passes now |
| `macro_registry_respects_imports_exports_and_scoped_shadowing` | fn | fails: macro_run.rs:783:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `body_engine_expands_real_imported_macro_before_checking_arguments` | fn | fails: macro_run.rs:877:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `body_engine_expands_type_arguments_without_evaluating_them` | fn | fails: macro_run.rs:965:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `body_engine_expands_pipe_left_operand_as_first_argument` | fn | fails: macro_run.rs:1015:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `body_engine_discards_notes_if_macro_result_is_not_an_expression` | fn, env | fails: macro_run.rs:1075:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `body_engine_reports_invalid_macro_signature_at_its_definition` | fn | fails: macro_run.rs:1130:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `unused_macro_definitions_are_checked_for_signature_and_capabilities` | fn | fails: macro_run.rs:1184:9: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `item_phase_runs_imported_calls_and_preserves_written_types` | fn | fails: macro_run.rs:1269:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `item_phase_retries_definition_after_a_later_item` | fn | fails: macro_run.rs:1340:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `item_phase_reports_each_unresolved_waiter_and_restores_the_module` | fn | fails: macro_run.rs:1405:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `item_phase_rechecks_an_early_builtin_answer` | fn | fails: macro_run.rs:1476:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `item_phase_queries_the_expanded_generated_type` | fn | fails: macro_run.rs:1572:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `project_expands_imported_macro_and_runs_generated_program` | fn | fails: macro_run.rs:1626:61: called `Result::unwrap()` on an `Err` value: [Diagnostic { severity: Error, code: "E0101", message: "the parameters of a function are a data pattern `(, …)`", source: Some(SourceId(21)), span: @60..85, relat… | yes | passes now |
| `successful_macro_note_is_a_compiler_note_not_a_warning` | fn, env | fails: macro_run.rs:1685:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `cli_check_and_run_expand_imported_macros` | fn | fails: macro_run.rs:1716:9: check: src/macros.is:3:23: error[E0101]: the parameters of a function are a data pattern `(, …)` 1 error | yes | passes now |
| `cli_expands_imported_macro_with_standard_service` | fn | fails: macro_run.rs:1740:5: src/macros.is:3:23: error[E0101]: the parameters of a function are a data pattern `(, …)` 1 error | yes | passes now |
| `cli_expands_attribute_macros_before_checking` | fn | fails: macro_run.rs:1764:5: src/macros.is:3:22: error[E0101]: the parameters of a function are a data pattern `(, …)` 1 error | yes | passes now |
| `cli_runs_item_macro_with_exposed_caller_name` | fn, env | fails: macro_run.rs:1787:5: src/macros.is:3:22: error[E0101]: the parameters of a function are a data pattern `(, …)` 1 error | yes | passes now |
| `a_later_macro_failure_keeps_an_earlier_committed_note` | fn, env | fails: macro_run.rs:1813:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `cli_retries_definition_queries_after_later_item_macros` | fn | fails: macro_run.rs:1913:5: src/macros.is:3:21: error[E0101]: the parameters of a function are a data pattern `(, …)` src/macros.is:4:22: error[E0101]: the parameters of a function are a data pattern `(, …)` 2 errors | yes | passes now |
| `cli_macro_stop_reports_call_site_and_template` | fn | fails: macro_run.rs:1938:5: src/macros.is:3:22: error[E0101]: the parameters of a function are a data pattern `(, …)` 1 error | yes | passes now |
| `cli_nested_macro_stop_preserves_outer_call_and_templates` | fn | fails: macro_run.rs:1964:5: src/macros.is:3:23: error[E0101]: the parameters of a function are a data pattern `(, …)` src/macros.is:4:23: error[E0101]: the parameters of a function are a data pattern `(, …)` 2 errors | yes | passes now |
| `collection_manifest_from_a_real_item_macro` | fn, env | fails: macro_run.rs:1985:80: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `project_check_rejects_a_private_collection_binding` | fn, env | fails: macro_run.rs:2025:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `definition_query_reads_expanded_type_from_dependency` | fn | fails: macro_run.rs:2059:80: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `shared_macro_code_reads_each_callers_definition_context` | fn, clauses, pipe | fails: macro_run.rs:2126:5: src/macros.is:3:22: error[E0101]: the parameters of a function are a data pattern `(, …)` 1 error | yes | passes now |
| `cli_queries_a_dependency_type_after_its_type_macro_expands` | fn, clauses, pipe | fails: macro_run.rs:2154:5: src/macros.is:3:21: error[E0101]: the parameters of a function are a data pattern `(, …)` src/macros.is:4:22: error[E0101]: the parameters of a function are a data pattern `(, …)` src/macros.is:5:23: error[E0… | yes | passes now |
| `collection_error_from_template_points_at_call` | fn, env | fails: macro_run.rs:2170:80: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `item_collection_resolves_qualified_import` | fn, env | fails: macro_run.rs:2205:80: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `a_macro_cannot_be_used_in_its_defining_module` | fn | fails: macro_run.rs:2240:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `unused_compile_time_helper_does_not_break_a_normal_program` | fn | fails: macro_run.rs:2317:5: src/helpers.is:1:15: error[E0101]: the parameters of a function are a data pattern `(, …)` 1 error | yes | passes now |
| `cli_runs_task_scope_inside_expanded_macro_argument` | fn | fails: macro_run.rs:2336:5: src/macros.is:3:20: error[E0101]: the parameters of a function are a data pattern `(, …)` 1 error | yes | passes now |
| `body_engine_expands_real_macro_under_deep_mixed_body` | fn | fails: macro_run.rs:2365:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `real_macro_definition_query_stops_before_oversize_source_construction` | fn | fails: macro_run.rs:2519:5: E0101 the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `macro_output_over_frame_budget_reports_the_limit` | fn, clauses, pipe, err | fails: macro_run.rs:2671:5: src/macros.is:3:13: error[E0101]: the parameters of a function are a data pattern `(, …)` src/macros.is:5:22: error[E0101]: the parameters of a function are a data pattern `(, …)` 2 errors | yes | passes now |

### `crates/ilang-cli/tests/typed_macros.rs`

35 tests; 34 embed rows from before syntax three; on lane/fa before the conversion: 34 failed; 34 changed here; with the converted text today: 0 fail.

| Test | Old rows in its text | On lane/fa before | Changed | Converted text |
|---|---|---|---|---|
| `typed_macro_identity_differential` | fn, tparams | fails: typed_macros.rs:26:5: src/macros.is:3:13: error[E0101]: expected a name src/macros.is:4:26: error[E0101]: the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `typed_macro_signature_checks_dropped_arguments` | fn | fails: typed_macros.rs:64:5: expected "not a subtype": src/macros.is:3:19: error[E0101]: this is not a type src/macros.is:4:19: error[E0101]: the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `typed_macro_narrow_output_keeps_declared_type` | fn | fails: typed_macros.rs:82:6: called `Result::unwrap()` on an `Err` value: "src/macros.is:3:19: error[E0101]: this is not a type\nsrc/macros.is:4:19: error[E0101]: the parameters of a function are a data pattern `(, …)`" | yes | passes now |
| `typed_macro_output_cannot_change_result_type` | fn | fails: typed_macros.rs:64:5: expected "cannot depend on its expansion": src/macros.is:3:19: error[E0101]: this is not a type src/macros.is:4:19: error[E0101]: the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `typed_macro_result_only_variable_cannot_guess` | fn | fails: typed_macros.rs:64:5: expected "result-only signature variable has no lower evidence": src/macros.is:3:13: error[E0101]: expected a name src/macros.is:4:19: error[E0101]: the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `typed_macro_labels_and_pipe` | fn | fails: typed_macros.rs:127:6: called `Result::unwrap()` on an `Err` value: "src/macros.is:3:19: error[E0101]: this is not a type\nsrc/macros.is:4:19: error[E0101]: the parameters of a function are a data pattern `(, …)`" | yes | passes now |
| `typed_macro_type_of_observes_literal` | fn, clauses, pipe | fails: typed_macros.rs:135:73: called `Result::unwrap()` on an `Err` value: "src/macros.is:3:13: error[E0101]: expected a name\nsrc/macros.is:4:19: error[E0101]: the parameters of a function are a data pattern `(, …)`" | yes | passes now |
| `typed_macro_type_of_rejects_fabricated_syntax` | fn | fails: typed_macros.rs:64:5: expected "type_of needs an original": src/macros.is:3:19: error[E0101]: this is not a type src/macros.is:4:19: error[E0101]: the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `typed_macro_signature_query_is_reflexive` | fn, clauses, pipe | fails: typed_macros.rs:155:74: called `Result::unwrap()` on an `Err` value: "src/macros.is:3:13: error[E0101]: expected a name\nsrc/macros.is:4:19: error[E0101]: the parameters of a function are a data pattern `(, …)`" | yes | passes now |
| `typed_macro_members_preserve_finite_scalar_order` | fn, clauses, pipe | fails: typed_macros.rs:163:59: called `Result::unwrap()` on an `Err` value: "src/macros.is:3:19: error[E0101]: this is not a type\nsrc/macros.is:4:19: error[E0101]: the parameters of a function are a data pattern `(, …)`" | yes | passes now |
| `typed_macro_rejects_type_position` | fn, tparams | fails: typed_macros.rs:64:5: expected "cannot give a type": src/macros.is:3:13: error[E0101]: expected a name src/macros.is:4:26: error[E0101]: the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `typed_macro_rejects_item_position` | fn, tparams | fails: typed_macros.rs:64:5: expected "cannot make items": src/macros.is:3:13: error[E0101]: expected a name src/macros.is:4:26: error[E0101]: the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `typed_macro_rejects_value_position` | fn, tparams | fails: typed_macros.rs:64:5: expected "only as an expression call": src/macros.is:3:13: error[E0101]: expected a name src/macros.is:4:26: error[E0101]: the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `typed_macro_rejects_signature_holes` | fn | fails: typed_macros.rs:64:5: expected "fully written without _": src/macros.is:3:19: error[E0101]: this is not a type src/macros.is:4:19: error[E0101]: the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `typed_macro_rejects_implementation_arity` | fn | fails: typed_macros.rs:64:5: expected "one parameter for each": src/macros.is:3:19: error[E0101]: a function type whose parameter is not written as data needs shape groups (stage 9 of syntax three), which this compiler does not have yet… | yes | passes now |
| `typed_macro_forwarding_record_differential` | fn, tparams | fails: typed_macros.rs:209:156: called `Result::unwrap()` on an `Err` value: "src/macros.is:3:13: error[E0101]: expected a name\nsrc/macros.is:4:26: error[E0101]: the parameters of a function are a data pattern `(, …)`" | yes | passes now |
| `typed_macro_generic_binding_expands_once` | fn, tparams | fails: typed_macros.rs:214:135: called `Result::unwrap()` on an `Err` value: "src/macros.is:3:13: error[E0101]: expected a name\nsrc/macros.is:4:26: error[E0101]: the parameters of a function are a data pattern `(, …)`" | yes | passes now |
| `typed_macro_builds_and_runs_forwarded_argument` | fn, tparams | fails: typed_macros.rs:232:6: called `Result::unwrap()` on an `Err` value: Program { lines: ["src/macros.is:3:13: error[E0101]: expected a name", "src/macros.is:4:26: error[E0101]: the parameters of a function are a data pattern `(, …)`… | yes | passes now |
| `typed_macro_cannot_delay_effectful_argument` | fn | fails: typed_macros.rs:64:5: expected "cannot delay an argument": src/macros.is:3:13: error[E0101]: expected a name src/macros.is:4:19: error[E0101]: the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `typed_macro_can_delay_pure_argument` | fn | fails: typed_macros.rs:269:6: called `Result::unwrap()` on an `Err` value: "src/macros.is:3:13: error[E0101]: expected a name\nsrc/macros.is:4:19: error[E0101]: the parameters of a function are a data pattern `(, …)`" | yes | passes now |
| `typed_macro_identity_principal_graph_differential` | fn, tparams | fails: typed_macros.rs:319:6: called `Result::unwrap()` on an `Err` value: "src/macros.is:3:13: error[E0101]: expected a name\nsrc/macros.is:4:26: error[E0101]: the parameters of a function are a data pattern `(, …)`" | yes | passes now |
| `typed_macro_forwarding_principal_record_graph_differential` | fn, tparams | fails: typed_macros.rs:329:144: called `Result::unwrap()` on an `Err` value: "src/macros.is:3:13: error[E0101]: expected a name\nsrc/macros.is:4:26: error[E0101]: the parameters of a function are a data pattern `(, …)`" | yes | passes now |
| `typed_macro_nested_forwarding_runs_retained_inner_call` | fn, tparams | fails: typed_macros.rs:352:6: called `Result::unwrap()` on an `Err` value: Program { lines: ["src/macros.is:3:13: error[E0101]: expected a name", "src/macros.is:4:26: error[E0101]: the parameters of a function are a data pattern `(, …)`… | yes | passes now |
| `typed_macro_outer_drop_does_not_execute_inner_macro` | fn | fails: typed_macros.rs:376:6: called `Result::unwrap()` on an `Err` value: "src/macros.is:3:22: error[E0101]: this is not a type\nsrc/macros.is:4:22: error[E0101]: the parameters of a function are a data pattern `(, …)`\nsrc/macros.is:5… | yes | passes now |
| `typed_macro_result_can_introduce_another_typed_call` | fn | fails: typed_macros.rs:392:6: called `Result::unwrap()` on an `Err` value: "src/macros.is:3:23: error[E0101]: this is not a type\nsrc/macros.is:4:23: error[E0101]: the parameters of a function are a data pattern `(, …)`\nsrc/macros.is:5… | yes | passes now |
| `typed_macro_forwarding_effect_rows_graph_differential` | fn, tparams | fails: typed_macros.rs:425:6: called `Result::unwrap()` on an `Err` value: "src/macros.is:3:13: error[E0101]: expected a name\nsrc/macros.is:4:26: error[E0101]: the parameters of a function are a data pattern `(, …)`" | yes | passes now |
| `typed_macro_graph_members_keep_one_exact_record_graph` | fn, clauses, pipe | fails: typed_macros.rs:443:6: called `Result::unwrap()` on an `Err` value: "src/macros.is:3:13: error[E0101]: expected a name\nsrc/macros.is:4:19: error[E0101]: the parameters of a function are a data pattern `(, …)`" | yes | passes now |
| `typed_macro_frozen_variable_is_legal_in_output_annotation` | fn | fails: typed_macros.rs:455:6: called `Result::unwrap()` on an `Err` value: "src/macros.is:3:13: error[E0101]: expected a name\nsrc/macros.is:4:19: error[E0101]: the parameters of a function are a data pattern `(, …)`" | yes | passes now |
| `typed_macro_members_accept_symbolic_difference_without_new_bounds` | fn, clauses, pipe | fails: typed_macros.rs:468:6: called `Result::unwrap()` on an `Err` value: "src/macros.is:3:13: error[E0101]: expected a name\nsrc/macros.is:4:19: error[E0101]: the parameters of a function are a data pattern `(, …)`" | yes | passes now |
| `typed_macro_forwarding_keeps_original_value_type_reads` | fn, tparams | fails: typed_macros.rs:473:117: called `Result::unwrap()` on an `Err` value: "src/macros.is:3:13: error[E0101]: expected a name\nsrc/macros.is:4:26: error[E0101]: the parameters of a function are a data pattern `(, …)`" | yes | passes now |
| `typed_macro_argument_binder_cannot_escape` | fn, clauses, pipe | fails: typed_macros.rs:64:5: expected "cannot escape an argument binder": src/macros.is:3:19: error[E0101]: this is not a type src/macros.is:4:19: error[E0101]: the parameters of a function are a data pattern `(, …)` | yes | passes now |
| `typed_macro_partial_argument_rebuild_retains_original_binders` | fn, clauses, pipe | fails: typed_macros.rs:497:6: called `Result::unwrap()` on an `Err` value: "src/macros.is:3:40: error[E0101]: this is not a type\nsrc/macros.is:4:19: error[E0101]: the parameters of a function are a data pattern `(, …)`" | yes | passes now |
| `typed_macro_nested_queries_keep_parent_type_handles` | fn, clauses, pipe, tparams | fails: typed_macros.rs:523:6: called `Result::unwrap()` on an `Err` value: "src/macros.is:3:13: error[E0101]: expected a name\nsrc/macros.is:4:23: error[E0101]: the parameters of a function are a data pattern `(, …)`\nsrc/macros.is:8:13… | yes | passes now |
| `typed_macro_generic_choice_reports_restriction` | fn, clauses, pipe | fails: typed_macros.rs:64:5: expected "cannot choose by the type inside a binding with type variables": src/macros.is:3:13: error[E0101]: expected a name src/macros.is:4:19: error[E0101]: the parameters of a function are a data pattern … | yes | passes now |

### `crates/ilang-cli/tests/lisp.rs`

5 tests; 3 embed rows from before syntax three; on lane/fa before the conversion: 2 failed; 3 changed here; with the converted text today: 0 fail.

| Test | Old rows in its text | On lane/fa before | Changed | Converted text |
|---|---|---|---|---|
| `lisp_modules_are_checked_with_source_locations` | fn | fails: lisp.rs:23:5: src/math.is:2:14: error[E0101]: the parameters of a function are a data pattern `(, …)` 1 error | yes | passes now |
| `conversion_does_not_write_an_unrepresentable_module` | fn, clauses | passes | yes | passes now |
| `macro_quote_formats_in_lisp_and_cannot_convert_to_normal` | fn | fails: lisp.rs:151:5: macros.is:1:22: error[E0101]: the parameters of a function are a data pattern `(, …)` | yes | passes now |

### `crates/ilang-cli/tests/compile_time_tests.rs`

3 tests; 3 embed rows from before syntax three; on lane/fa before the conversion: not run; 3 changed here; with the converted text today: 1 fail.

| Test | Old rows in its text | On lane/fa before | Changed | Converted text |
|---|---|---|---|---|
| `ordinary_roots_and_initializers_cannot_reach_compile_time_bindings` | fn | not run | yes | passes now |
| `tests_use_each_builds_unmarked_quote_templates` | fn | not run | yes | after the flip; today: source_syntax.rs:260:51: parsed item remains valid: Diagnostic { code: Unexpected, message: "datum is not valid as a pattern", span: @104..105 } |
| `selected_test_collection_installs_its_quote_templates` | fn, env | not run | yes | passes now |

### `crates/ilang-cli/tests/collections.rs`

3 tests; 3 embed rows from before syntax three; on lane/fa before the conversion: not run; 3 changed here; with the converted text today: 1 fail.

| Test | Old rows in its text | On lane/fa before | Changed | Converted text |
|---|---|---|---|---|
| `generated_contributions_are_selected_independently` | fn, env | not run | yes | passes now |
| `selected_collection_uses_its_target_plan_and_transitive_requests` | fn, env | not run | yes | passes now |
| `expand_command_prints_both_surfaces_and_macro_trace` | fn, env | not run | yes | fails, not for the rows (§3): collections.rs:282:5: assertion failed: !output.status.success() |

### `crates/ilang-cli/tests/fmt.rs`

6 tests; 1 embed rows from before syntax three; on lane/fa before the conversion: not run; 1 changed here; with the converted text today: 0 fail.

| Test | Old rows in its text | On lane/fa before | Changed | Converted text |
|---|---|---|---|---|
| `lisp_files_are_formatted_in_default_roots_and_given_paths` | fn | not run | yes | passes now |

### `crates/ilang-cli/tests/syntax_runtime.rs`

10 tests; 1 embed rows from before syntax three; on lane/fa before the conversion: not run; 1 changed here; with the converted text today: 0 fail.

| Test | Old rows in its text | On lane/fa before | Changed | Converted text |
|---|---|---|---|---|
| `compiler_definition_service_returns_and_aborts_at_the_query` | ctors | not run | yes | passes now |

### `crates/ilang-lsp/tests/protocol.rs`

98 tests; 1 embed rows from before syntax three; on lane/fa before the conversion: not run; 1 changed here; with the converted text today: 5 fail.

| Test | Old rows in its text | On lane/fa before | Changed | Converted text |
|---|---|---|---|---|
| `lsp_rename_top_level_across_files` | — | not run | no | fails; not changed here, not the rows: protocol.rs:1349:5: ( Shape(..) surface origin Clock(..) Box(..) ) |
| `lsp_rename_field_keeps_pattern_names` | — | not run | no | fails; not changed here, not the rows: protocol.rs:1427:77: called `Result::unwrap()` on an `Err` value: "there is no name here to rename" |
| `lsp_rename_lisp_surface` | — | not run | no | fails; not changed here, not the rows: protocol.rs:1605:5: assertion failed: apply(SHAPES, &edit, &ws.shapes).contains("surface = (s = Shape)") |
| `lsp_completion_services_in_an_environment` | — | not run | no | fails; not changed here, not the rows: protocol.rs:1925:5: assertion failed: !items.iter().any(\|i\| i.label == "Point") |
| `lsp_hover_errors_and_environment` | — | not run | no | fails; not changed here, not the rows: protocol.rs:2105:5: ```i parse: (s = String) <- (Clock) -> Int ``` |
| `lsp_every_offset_of_broken_files` | fn | not run | yes | passes now |

### `crates/ilang-syntax/tests/lisp_syntax.rs`

46 tests; 8 embed rows from before syntax three; on lane/fa before the conversion: 0 failed; 19 changed here; with the converted text today: 13 fail.

| Test | Old rows in its text | On lane/fa before | Changed | Converted text |
|---|---|---|---|---|
| `show_as_checks_the_requested_syntactic_position` | — | passes | yes | after the flip; today: lisp_syntax.rs:57:52: called `Result::unwrap()` on an `Err` value: SyntaxError { message: "datum is not valid as a type", span: @0..3 } |
| `normal_source_encodes_to_syntax_with_original_leaf_spans` | fn | passes | yes | after the flip; today: lisp_syntax.rs:76:5: assertion `left == right` failed left: "(-> (x) _ {(+ x 2)})" right: "(-> (, x) _ (+ x 2))" |
| `expansion_cannot_create_or_duplicate_foreign_authority` | fn | passes | yes | passes now |
| `builtin_definitions_preserve_builtins_and_fresh_parameter_binders` | — | passes | yes | after the flip; today: lisp_syntax.rs:375:77: called `Option::unwrap()` on a `None` value |
| `written_definition_answers_give_parameters_their_own_binder` | — | passes | yes | after the flip; today: lisp_syntax.rs:425:5: assertion `left == right` failed left: "((Pair A B) {(Pair (: left A) (: right (List B)))})" right: "(= (Pair A B) (, (= left A) (= right (List B))))" |
| `copied_definition_freshens_its_binders_and_uses_together` | — | passes | yes | after the flip; today: lisp_syntax.rs:465:9: head |
| `definition_table_keeps_scopes_from_expanded_type_items` | — | passes | yes | after the flip; today: lisp_syntax.rs:769:9: head |
| `expanded_definition_identifies_only_parameters_with_the_same_scope` | — | passes | yes | passes now |
| `template_rebuild_preserves_origin_and_cannot_forge_foreign_attributes` | fn | passes | yes | passes now |
| `expanded_definition_preserves_attributes_and_parameter_identities` | — | passes | yes | passes now |
| `definition_answer_preserves_a_single_field_record_type` | record | passes | yes | after the flip; today: lisp_syntax.rs:916:88: called `Result::unwrap()` on an `Err` value: SyntaxError { message: "datum is not valid as a qualified name", span: @15..16 } |
| `scoped_conversion_reports_the_actual_scoped_bad_node` | — | passes | yes | after the flip; today: lisp_syntax.rs:980:48: called `Result::unwrap_err()` on an `Ok` value: Expr { kind: Call { callee: Expr { kind: Name { module: None, name: Ident { name: "f", span: @6..7 } }, span: @6..7 }, args: [Arg { label: None, value: Expr {… |
| `rewriting_a_definition_head_cannot_forge_parameter_binding_authority` | — | passes | yes | after the flip; today: lisp_syntax.rs:1086:9: head |
| `duplicate_definition_heads_are_freshened_with_all_uses` | — | passes | yes | after the flip; today: lisp_syntax.rs:1121:13: head |
| `records_parse_from_lisp_with_fields_spread_and_removal` | record | passes | yes | passes now |
| `show_as_distinguishes_postfix_and_standalone_clauses` | fn, clauses, pipe | passes | yes | passes now |
| `show_as_only_breaks_lines_at_ambiguous_statement_boundaries` | fn | passes | yes | after the flip; today: lisp_syntax.rs:1268:46: called `Result::unwrap()` on an `Err` value: SyntaxError { message: "datum is not valid as a pattern", span: @5..6 } |
| `show_as_attribute_item_uses_only_required_line_breaks` | fn | passes | yes | after the flip; today: lisp_syntax.rs:1287:38: called `Result::unwrap()` on an `Err` value: SyntaxError { message: "datum is not valid as a pattern", span: @20..21 } |
| `show_as_accepts_expressions_already_written_in_normal_surface` | — | passes | yes | after the flip; today: lisp_syntax.rs:1344:41: "value = x \|> (1) { 2 } \| (_) { 3 }\n": SyntaxError { message: "datum is not valid as a normal-surface expression", span: @8..34 } |

### `crates/ilang-syntax/tests/lisp_datum.rs`

26 tests; 6 embed rows from before syntax three; on lane/fa before the conversion: 0 failed; 12 changed here; with the converted text today: 2 fail.

| Test | Old rows in its text | On lane/fa before | Changed | Converted text |
|---|---|---|---|---|
| `datums_round_trip_and_keep_spans` | fn | passes | yes | passes now |
| `macro_quotes_are_core_nodes_and_can_be_formatted` | fn | passes | yes | passes now |
| `quote_splices_are_checked_in_their_written_position` | fn | passes | yes | after the flip; today: lisp_datum.rs:107:9: (= f (-> (,) _ `~..xs)) |
| `compiler_preparation_registers_nested_quote_pattern_templates` | fn | passes | yes | passes now |
| `invalid_datums_do_not_make_core_items` | — | passes | yes | passes now |
| `both_surfaces_preserve_attributes_and_grouped_result_holes` | — | passes | yes | passes now |
| `a_nonfinal_clause_choice_cannot_end_in_a_type_test` | clauses | passes | yes | fails, not for the rows (§3): lisp_datum.rs:262:5: assertion failed: !errors.is_empty() |
| `direct_decode_accepts_expression_function_bodies` | fn | passes | yes | passes now |
| `core_forms_convert_between_surfaces` | — | passes | yes | passes now |
| `format_corpus_has_a_lisp_round_trip` | — | passes | yes | passes now |
| `libraries_have_a_lisp_round_trip` | — | passes | yes | passes now |
| `spec_programs_have_a_lisp_round_trip` | — | passes | yes | passes now |

### `crates/ilang-syntax/tests/lisp_encode_deep.rs`

10 tests; 0 embed rows from before syntax three; on lane/fa before the conversion: 0 failed; 1 changed here; with the converted text today: 1 fail.

| Test | Old rows in its text | On lane/fa before | Changed | Converted text |
|---|---|---|---|---|
| `records_preserve_labels_spreads_and_streamed_cost` | — | passes | yes | after the flip; today: lisp_encode_deep.rs:664:13: assertion `left == right` failed left: "(= (f R K) (-> ((: r ((: k Int) (: full_name String)))) _ {(.. r (: full_name \"Ada\") (: x 1))}))" right: "(= empty (,))" |

### `crates/ilang-syntax/tests/lisp_record.rs`

6 tests; 2 embed rows from before syntax three; on lane/fa before the conversion: 1 failed; 6 changed here; with the converted text today: 1 fail.

| Test | Old rows in its text | On lane/fa before | Changed | Converted text |
|---|---|---|---|---|
| `record_values_types_and_source_locations_are_preserved` | record | passes | yes | passes now |
| `invalid_records_do_not_become_blocks_or_patterns` | — | passes | yes | passes now |
| `record_expression_and_type_handle_ten_thousand_nested_fields_and_spreads` | — | passes | yes | not run |
| `expanded_record_labels_keep_their_original_syntax_identity` | record | passes | yes | after the flip; today: lisp_record.rs:227:50: ScopedError: datum is not valid as an expression |
| `normal_lisp_normal_round_trip_preserves_records` | — | fails: lisp_record.rs:246:5: (= empty ()) (= single (, 1)) (= Single (, Int)) (= f (-> (x) (! (^ Int _) _) {x})) (= first ((: name "Ada") (: x_y 7))) (= second (.. first (: count 2))) (= (choose K R) (-> ((: key K) (: r ((: k Int)))) (!… | yes | passes now |
| `malformed_record_field_reports_the_original_label_span` | — | passes | yes | passes now |

### `crates/ilang-syntax/tests/string_forms.rs`

4 tests; 0 embed rows from before syntax three; on lane/fa before the conversion: 0 failed; 2 changed here; with the converted text today: 0 fail.

| Test | Old rows in its text | On lane/fa before | Changed | Converted text |
|---|---|---|---|---|
| `interpolation_is_concat_before_the_core_tree` | — | passes | yes | passes now |
| `formatter_preserves_interpolation_with_nested_expressions` | — | passes | yes | passes now |

### `crates/ilang-syntax/tests/lisp_deep.rs`

11 tests; 0 embed rows from before syntax three; on lane/fa before the conversion: 0 failed; 0 changed here; with the converted text today: 0 fail.

| Test | Old rows in its text | On lane/fa before | Changed | Converted text |
|---|---|---|---|---|
| `deep_datum_is_read_and_dropped_as_the_new_tree` | (new test) | — | new | passes now |

### `crates/ilang-syntax/tests/lisp_format.rs`

5 tests; 0 embed rows from before syntax three; on lane/fa before the conversion: 0 failed; 0 changed here; with the converted text today: 0 fail.

### `crates/ilang-check/tests/hygiene_resolve.rs`

20 tests; 3 embed rows from before syntax three; on lane/fa before the conversion: 0 failed; 12 changed here; with the converted text today: 3 fail.

| Test | Old rows in its text | On lane/fa before | Changed | Converted text |
|---|---|---|---|---|
| `template_binder_does_not_capture_caller_name` | — | passes | yes | passes now |
| `template_private_is_visible_but_made_name_must_be_exported` | — | passes | yes | passes now |
| `builtin_identity_bypasses_caller_type_shadow` | fn | passes | yes | after the flip; today: hygiene_resolve.rs:223:68: ScopedError: datum is not valid as a pattern |
| `an_exported_inferred_type_cannot_leak_a_foreign_private_type` | — | passes | yes | passes now |
| `unspellable_compiler_alias_can_call_private_member` | — | passes | yes | passes now |
| `same_text_in_one_tuple_pattern_binds_distinct_scopes` | fn | passes | yes | after the flip; today: hygiene_resolve.rs:58:53: ScopedError: datum is not valid as a pattern |
| `parameter_labels_still_compare_written_text` | fn | passes | yes | after the flip; today: hygiene_resolve.rs:58:53: ScopedError: datum is not valid as a pattern |
| `nested_expansions_keep_both_mark_sets_distinct` | — | passes | yes | passes now |
| `an_exported_written_type_cannot_leak_a_foreign_private_type` | — | passes | yes | passes now |
| `fixed_type_identity_never_falls_back_to_a_same_named_alias` | — | passes | yes | passes now |
| `builtin_constructor_identity_bypasses_caller_constructor_shadow` | — | passes | yes | passes now |
| `same_text_type_parameters_in_distinct_scopes_are_distinct` | — | passes | yes | passes now |

### `crates/ilang-check/tests/expansion_origins.rs`

4 tests; 0 embed rows from before syntax three; on lane/fa before the conversion: 0 failed; 4 changed here; with the converted text today: 0 fail.

| Test | Old rows in its text | On lane/fa before | Changed | Converted text |
|---|---|---|---|---|
| `template_error_uses_call_site_and_related_template_and_suppresses_warnings` | — | passes | yes | passes now |
| `argument_location_and_query_use_written_source` | — | passes | yes | passes now |
| `nested_template_error_uses_outer_call_and_both_template_locations` | — | passes | yes | passes now |
| `template_name_hover_and_definition_navigate_across_modules` | — | passes | yes | passes now |

### `crates/ilang-check/tests/quote_patterns.rs`

5 tests; 0 embed rows from before syntax three; on lane/fa before the conversion: 0 failed; 0 changed here; with the converted text today: 0 fail.

### `crates/ilang-check/tests/expanded_modules.rs`

1 tests; 0 embed rows from before syntax three; on lane/fa before the conversion: 0 failed; 1 changed here; with the converted text today: 0 fail.

| Test | Old rows in its text | On lane/fa before | Changed | Converted text |
|---|---|---|---|---|
| `checks_transformed_modules_without_reparsing_the_source` | — | passes | yes | passes now |

### `crates/ilang-check/tests/macro_names.rs`

5 tests; 0 embed rows from before syntax three; on lane/fa before the conversion: 0 failed; 0 changed here; with the converted text today: 0 fail.

### `crates/ilang-macro/tests/body_walk.rs`

9 tests; 0 embed rows from before syntax three; on lane/fa before the conversion: 0 failed; 0 changed here; with the converted text today: 0 fail.

Notes on single files:

- `lisp_syntax.rs`: the definition tests (`builtin_definition`,
  `source_definition`, `expanded_definition`, `freshen_parameters`,
  `freshen_duplicate_heads`) embed no row; they walk the answer by position,
  and the answer changes from `(Head {C…})` to `(= Head Right)` (question 1 of
  syn3-final-ast.md §5). Their new expectations agree with part B's
  `lisp_definition_rows.rs` (the `_rows` functions with `true`). The `show_as`
  expectations agree with part C's `lisp_syntax3.rs` (`show_as_rows3`): the
  printer of syntax three puts each statement of a block of several
  statements, and an attribute, on a line of its own.
  `show_as_distinguishes_postfix_and_standalone_clauses` keeps its text: a
  clause block is an error before the flip and is not a row after it.
- `lisp_datum.rs`, `lisp_record.rs`, `string_forms.rs`: the tests of the
  reader and writer before syntax three (`lisp::parse_module`,
  `lisp::parse_expression`, `lisp::print_module`, `lisp::normal_source`,
  `lisp::item_syntax`) now call what replaced them: `lisp::parse_source`
  (rows of §6.8, lowered), `syn3::lisp::{from_lisp, to_lisp, item_datum,
  expr_from_datum, type_from_datum, pattern_from_datum}` and
  `syn3::surface::{normal_to_lisp, lisp_to_normal}`. `lisp_record.rs` tests
  data where it tested records; a record type with a base or a removed key,
  a key parameter and a text label have no row, and the tests say so.
- `lisp_encode_deep.rs` tests `SourceEncoder` and `expression_syntax` of
  `lisp/encode.rs`, which step 2c deletes. Only the expected rows of
  `records_preserve_labels_spreads_and_streamed_cost` were converted; the
  calls must move to the encoder that replaces `encode.rs`
  (`lisp::source_tree::SourceTree` on lane/fa).
- `body_walk.rs` builds trees before syntax three by hand; it is part E's
  (`body_walk3.rs`).
- `quote_patterns.rs`, `macro_names.rs`, `lisp_format.rs`, `lisp_deep.rs`
  were in the rows of §6.8 already.
- `crates/ilang-check/tests/unused_audit.rs` reads the tree before syntax
  three (`ast::ItemKind::MacroItem`, `ast::Param::Env`); it has no rows and
  runs only with `UNUSED_AUDIT` set. It moves with step 2d.

## 3. Tests that pass only after the flip

The exact message of each today is in the tables of §2. By cause:

| Cause | Tests |
|---|---|
| A macro result is read in the old rows (`lisp/scoped.rs` → `to_ast.rs`): "datum is not valid as a pattern" at the `,` of `(-> (, x) …)`; "datum is not valid as an expression" at `(= name value)` | `hygiene_resolve`: `builtin_identity_bypasses_caller_type_shadow`, `same_text_in_one_tuple_pattern_binds_distinct_scopes`, `parameter_labels_still_compare_written_text`; `lisp_record`: `expanded_record_labels_keep_their_original_syntax_identity`; `lisp_syntax`: `scoped_conversion_reports_the_actual_scoped_bad_node` (today `(f ())` is read as a call with an empty record) |
| A written macro call of a `.is` file goes through the position table of the old rows (`lisp/source_syntax.rs`:260 panics: "parsed item remains valid: … datum is not valid as a pattern") | `compile_time_tests`: `tests_use_each_builds_unmarked_quote_templates` |
| A definition answer is `(Head {C…})` | `lisp_syntax`: `builtin_definitions_preserve_builtins_and_fresh_parameter_binders`, `written_definition_answers_give_parameters_their_own_binder`, `copied_definition_freshens_its_binders_and_uses_together`, `definition_table_keeps_scopes_from_expanded_type_items`, `rewriting_a_definition_head_cannot_forge_parameter_binding_authority`, `duplicate_definition_heads_are_freshened_with_all_uses`, `definition_answer_preserves_a_single_field_record_type` |
| `show_as` reads the old rows and writes the layout before syntax three | `lisp_syntax`: `show_as_checks_the_requested_syntactic_position`, `show_as_only_breaks_lines_at_ambiguous_statement_boundaries`, `show_as_attribute_item_uses_only_required_line_breaks`, `show_as_accepts_expressions_already_written_in_normal_surface` (its match leaf `x \|> (1) { 2 } \| (_) { 3 }` is encoded as `(. x {…})` and written as `x.{ … }`, which no parser reads) |
| A macro argument is encoded in the old rows (`lisp/encode.rs`) | `lisp_syntax`: `normal_source_encodes_to_syntax_with_original_leaf_spans`; `lisp_encode_deep`: `records_preserve_labels_spreads_and_streamed_cost` |
| A quote of a written `.is` file is not checked when the file is read (`lisp::parse_source` gives no error for a splice at the top of a template; `syn3::lisp::quote_captures` has the check and the lowering does not call it yet) | `lisp_datum`: `quote_splices_are_checked_in_their_written_position` |
| `std/src/test.is` and its copy `tests/review/cases/review-k4s/std/src/test.is`: the `suite` macro (§5) | `examples/macro-tests` ("outer / inner / …"), every project that uses `suite` |

These tests and the two `.is` files are in the second commit of lane/faF
("after the flip"); the first commit holds only what passes before the flip,
so the two can be merged at different times.

Two tests fail before and after for a reason that is not the rows; each is
a question for the integrator:

- `lisp_datum`: `a_nonfinal_clause_choice_cannot_end_in_a_type_test`. The
  reader before syntax three refused `{ (-> (| (= a (: b Int)) Unit) 1) }`,
  because the normal surface cannot write a choice that ends in a type test
  before another choice (`b: Int | Unit` is one typed pattern). The reader of
  syntax three (`syn3::lisp::from_lisp`) reads
  `(-> (, (| (= a (: b Int)) Unit)) _ 1)` with no error. Either the reader
  gets the check, or the rule becomes "`i fmt --to i` refuses it" and the
  test asserts that.
- `collections`: `expand_command_prints_both_surfaces_and_macro_trace`.
  `i expand src/macros.is --to i` of a module with a quote must fail and
  name `--to is`. On lane/fa it prints the quote inside normal text
  (`` `(= ~x (-> (,) _ {~body})) ``) and exits 0. Before the conversion the
  test failed earlier, on the parse error of the old rows.

## 4. Trees built by hand

`hygiene_resolve.rs` (8 places), `expansion_origins.rs` (4) and
`expanded_modules.rs` (1) changed `Parsed.ast` through the types of the tree
before syntax three. They now go through
`crates/ilang-check/tests/common/tree.rs`, which has the same functions for
both trees:

| Function | `tree::old` (`ilang_syntax::ast`) | `tree::new` (`ilang_syntax::syn3::ast`) |
|---|---|---|
| `statements(module, item)` | `ItemKind::Binding` → `ExprKind::Fn` → `body.stmts` | `ItemKind::Bind` → `ExprKind::Chain([f])` → `body.stmts` |
| `set_statement`, `expression`, `last_expression` | on those statements | the same |
| `value(module, item)` | `Binding.value` | the same |
| `written_type(module, item)` | `PatternKind::Typed { ty }` of the binding | the same |
| `set_import_alias(module, import, name)` | `ImportItem.alias` | `Import.alias` |
| `field_target_name(expr)` | `Field { target: Name }` | the same |
| `string(text)`, `span(expr)` | `ExprKind::String`, `expr.span` | `ExprKind::String`, `expr.span.0` |

Each test file has one line, `use common::tree::old as tree;`. When
`Parsed.ast` is a `syn3::ast::Module` (step 2d) that line becomes
`tree::new`. `crates/ilang-check/tests/tree_helpers.rs` (5 tests, pass)
proves that the two address the same place: a change made through `new` and
lowered with `syn3::lower` equals the change made through `old` on the
lowered module (`shape::equivalent`).

## 5. `.is` files

All 33 `.is` files of the repository are in the rows of §6.8 (the migrator
reads each as syntax three already). Only `std/src/test.is` (and its copy
under `tests/review/cases/review-k4s`) looks at nodes by hand; the others
build templates. Before the flip its quote patterns are lowered to the old
rows, where every function body is a block and a provide is a statement
`(! _ s)` before its target. After the flip a macro sees the tree as
written, so `suite` changes (second commit):

- A body of one expression is that expression, not a block:
  `statements_of` reads `` `(-> (,) _ {~..statements}) `` and then
  `` `(-> (,) _ ~statement) ``.
- `suite(…) <- (services)` is `` `(<- ~target ~services) ``: the services go
  on a list (innermost first) while the target's tests are collected.
- Each test body is wrapped by `provide`:
  `(<- (<- (body) inner) outer)`, where the old text copied `(! _ s)`
  statements before the call (§6.9 of syntax-3.md).

This text cannot run before the flip (its patterns have no old row). It was
formatted with `i fmt` and is read as syntax three; it was not executed.
`examples/macro-tests` passes today (9 passed, 1 skipped) and is the check
after the flip: "outer / inner / service", "outer / inner / cases / one" and
"… / two" must still pass.

One pattern in the Rust tests had the same cause and now passes in both
worlds: `typed_macros.rs` matched a function argument with
`` `(-> (~parameter) ~ret {~body}) ``; it is `` `(-> (, ~parameter) ~ret ~body) ``.

## 6. Decisions

- The needs of a function literal are inferred, so `(^ R E)` on a macro's
  function literal is dropped (migrator note "result-needs"), not turned
  into `(<- R (, (Fail E)))`.
- `hygiene_resolve`: the name `Box(A, A)` of the pair `(A, A)` is written
  `(= (Box A A) { (, A A) })`, which keeps its meaning (a name, not a
  labelled type). It passes before and after.
- `lisp.rs` `conversion_does_not_write_an_unrepresentable_module`: its module
  (a block with a function statement, as a call argument) can be written in
  syntax three, so the test uses a pipe whose right side is a pipe,
  `(|> a (|> b c))`, which `i fmt --to i` refuses.
- `lisp_datum` `invalid_datums_do_not_make_core_items`: `(1 .. 2)` is the
  call `1(..2)` in the rows of §6.8; the test uses `(1 ..)`.
- `syntax_runtime.rs`: the definition answer that the test's host gives is
  data of the test; it is `(= Box (, Int))` now.
