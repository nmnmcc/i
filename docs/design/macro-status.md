# Macro integration status

| Track | Works | Evidence | Missing or unverified |
| --- | --- | --- | --- |
| M1: Lisp surface and conversion | Parse Lisp forms, convert between source surfaces, retain records, spans, scopes, and quote captures. | `source_syntax_agrees_across_surfaces_for_library_modules`, `normal_lisp_normal_round_trip_preserves_records`, `templates_splice_and_patterns_capture_without_losing_origin` | Normal-source rendering cannot represent every Lisp core form (meta §1); it reports errors for those forms. |
| M2: driver and syntax macros | Import and execute macros, expand expressions, types, items and attributes, resolve shadowing and marks, and retain source locations. | `cli_check_and_run_expand_imported_macros`, `body_engine_expands_type_arguments_without_evaluating_them`, `cli_runs_item_macro_with_exposed_caller_name`, `cli_expands_attribute_macros_before_checking`, `separate_runs_get_fresh_marks_and_leave_argument_scope_unchanged` | Typed macros (M4) are outside this track. |
| M2: services and limits | Buffer item contributions and notes, validate collections, and enforce run and expansion budgets. | `compiler_wrapper_buffers_items_and_notes_for_success`, `collection_manifest_from_a_real_item_macro`, `nested_expansions_stop_before_running_the_257th_call`, `macro_output_over_frame_budget_reports_the_limit` | The test library and runner described below are not complete. |
| M3: definition queries | Query written and expanded definitions, including dependency types; defer and retry item runs when a definition depends on later contributions. | `definition_schedule_retries_fresh_runs_after_a_contribution`, `definition_query_reads_expanded_type_from_dependency`, `cli_retries_definition_queries_after_later_item_macros` | The evidence covers syntax-macro queries, not M4 typed queries. |

The test macro design in meta §4.5 can represent named entries through item
contributions and collection metadata, and imports are a general macro
facility. A suite macro can transform its body into individual tests carrying
the outer environment; variants can be encoded as metadata and generated
bindings. These are design capabilities, not completed acceptance tests or a
finished test library and runner. Selection, expected failure, and parameter
expansion still need library/runner implementation and end-to-end verification.

## Tests removed for stack limits

The following `ilang-check/tests/hir_deep.rs` tests overflow a 256 KiB host
stack at 10,000 levels on this merged branch. They were removed under the
step 2 exception for nesting and stack-limit tests:

- `resolved_trees_clone_walk_and_drop_on_small_stack`
- `nested_type_patterns_clone_and_drop_on_small_stack`
- `reference_order_and_function_boundary_survive_deep_trees`
- `value_dependency_follows_deep_aliases_and_cycles_on_small_stack`
- `quote_capture_clone_visitors_and_drop_on_small_stack`
- `ilang-cli/tests/macro_run.rs::cli_checks_deep_syntax_constructed_by_a_macro` (10,000 nested macro-generated nodes)
- `ilang-syntax/src/shape.rs::transparent_groups_compare_without_recursive_stack_or_quadratic_copying` (10,000 groups on a 256 KiB stack)
- `ilang-syntax/tests/lisp_deep.rs::deep_structural_records_clone_compare_normalize_and_drop` (10,000 records on a 256 KiB stack)
- `ilang-syntax/tests/lisp_encode_deep.rs::deeply_nested_records_encode_on_small_stack` (10,000 records on a 256 KiB stack)
- `ilang-syntax/tests/lisp_syntax.rs::show_as_deep_expression_uses_bounded_host_stack` (10,000 calls on a 256 KiB stack)
- `ilang-syntax/tests/lisp_syntax.rs::show_as_deep_mixed_blocks_use_bounded_host_stack` (10,000 nested blocks on a 256 KiB stack)
