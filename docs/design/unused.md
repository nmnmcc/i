# Unused top-level bindings

The accepted track UNUSED scope of 2026-11-11 supplies the requirements.
The normative rule is spec §2.4.1. This note records the prior art and the
choices that the scope leaves to the implementation.

## Prior art

All paths below are under `/home/agent/Programming/i/.rd/ref/`. These are
source readings, not runs of the reference compilers; no reference code was
copied or built.

- Rust: `rust/compiler/rustc_passes/src/dead.rs`,
  `create_and_seed_worklist`, `maybe_record_as_seed`, `check_def_id`, and
  `mark_live_symbols` seed entries, reachable exports, and explicit retention
  attributes, then walk references. `check_definition` includes functions,
  constants, types and aliases. `is_live_code` suppresses underscore names.
  Test harness references keep tests live. i takes the worklist method and
  underscore suppression, using its own entries, tests and host exports.
  It does not take Rust's trait, linker or field rules.
- Go: `go/src/cmd/compile/internal/types2/stmt.go`, `usage`, checks local
  variables, excluding parameters and results; `resolver.go`, `unusedImports`,
  rejects unused imports except `_`. `decl.go`, `declare`, makes `_` a blank
  rather than a binding. `typexpr.go`, `ident`, records uses even in unused
  functions. `go/src/cmd/vet/main.go`, `main`, delegates to the vet suite;
  it is not a package dead-code pass. The compiler does not reject unused
  package functions, types or constants: entries, tests and exports need no
  exemption there. `_name` is not the blank identifier. i needs a broader,
  transitive package warning to catch forgotten tests, rather than Go's
  compiler restriction. No Staticcheck source is present in this checkout;
  the compiler readings do not establish Staticcheck's behavior.
- Gleam: `gleam/compiler-core/src/reference.rs`, `register_value`,
  `register_constant`, `register_type`, `unused`, and `mark_entity_as_used`,
  use a reference graph rooted at public entities. Private recursive islands
  remain unused. `type_/environment.rs`, `handle_unused`, and `warning.rs`
  report private functions, constants, constructors, types and imports.
  `type_/pattern.rs`, `new_discarded_name`, tracks discarded variables
  separately. Public test functions and public entries are roots by visibility,
  rather than a special test rule in this pass. i takes the graph method,
  but has closed test-module exports and ordinary underscore bindings.
- Elixir: `elixir/lib/elixir/lib/module/types.ex`, `infer` and
  `warn_unused_def`, traverse public functions/macros and used private
  definitions before warning about unreachable private functions/macros.
  Generated definitions and super contexts are exempt. There is no underscore
  function-name exemption in `warn_unused_def`. Public tests and callbacks
  are kept by their visibility or generated references. i takes transitive
  reachability, without Elixir's macro or default-argument rules.
- TypeScript 5.9.3: `typescript-5.9.3/src/compiler/checker.ts`,
  `checkUnusedIdentifiers`, `checkUnusedLocalsAndParameters`, and
  `isValidUnusedLocalDeclaration`, check locals, imports, types, parameters
  and private members; referenced symbols and export symbols are exempt.
  There is no entry/test rule in this check. Underscore exemptions apply to
  parameters, imports and certain destructures, not every ordinary local.
  i takes resolved references and export roots, but uses a transitive graph
  and its own broader underscore convention (spec §1.3).

## Interpretations

- Scope rule 2 asks whether references from unused bindings count. Like Rust,
  Gleam and Elixir, i starts from roots and traces references transitively.
  Thus a recursive island has no use unless reached from a root.
- Scope rule 4 and spec §2.1 close test-module imports to the same package.
  An exported test helper is live only when reached from a used binding.
- Scope rule 6 says "when unsure, do not warn." Types, aliases, services,
  constructors and foreign declarations are exempt. Their written value-type
  references are roots, since these definitions are checked even when unused
  (spec §4.3). This avoids treating a type-only use as an unused value.
- Scope rules 3 and 2 distinguish suppression from use. `_name` suppresses
  only that name; its body does not retain other bindings. A discard-only
  binding has no name to use, so its references are roots.
- Spec §2.5 says a top-level value is worked out at start. The warning is
  about unused names and never removes initialization or changes Wasm.
- Scope rule 5's hint is confined to test modules: spec §2.2 permits `@test`
  only there. Environment parameters are allowed on test entries (§10.2).
  The result must be inferred as the prelude's nominal `Nil`, not a type
  merely spelled `Nil`, and must not be `Never` or a union containing `Nil`.
- Each pattern name is checked separately (scope rule 1, "a top-level
  binding that is not used"). A used name retains the binding's dependencies,
  but unused sibling names can still warn. This preserves the `_` convention.
