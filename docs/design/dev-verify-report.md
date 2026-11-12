# Full development verification

No language or library surface was added, including state mechanisms.

## Failures and fixes

- Rust: `providers.rs` nested an adapter comparison inside `if let`
  (`clippy::collapsible_if`). Combined the conditions in a let chain.
- Rust: the provider seed set triggered `clippy::mutable_key_type` through
  `ShownType`'s graph pool. The pool only appends immutable interned nodes;
  existing graph shapes and hashes do not change. Kept the set and added a
  local allow with that reason, consistent with the existing graph-key uses
  in `types/records/shown.rs`.
- Rust: `Lowerer::provider_steps` had eight arguments
  (`clippy::too_many_arguments`). Grouped the captured root key and adapter
  seeds into `ProviderInputs`, the runtime inputs for one invocation.
- Rust: CLI fixtures borrowed owned manifest strings unnecessarily
  (`clippy::needless_borrows_for_generic_args`): two in `macro_budget.rs`,
  eight in `syntax_runtime.rs`, one in `check.rs`. Passed the strings directly.
  Replaced `assert_eq!(status.success(), true, ...)` in `check.rs` with
  `assert!(status.success(), ...)` (`clippy::bool_assert_comparison`).
- i: `lib/i/task.i` contained one over-width `_provider_runner` signature.
  Formatted the signature with the existing formatter. A second format is
  byte-stable. Ran the existing `format_properties` test with an additional
  temporary copy of the original source: parsing before and after formatting
  produces the same canonical tree; comments and idempotence also pass.
  Removed the temporary copy after the check. The formatter is correct.
- i benchmark: `i test --project bench --opt 0` failed with
  `error[E0501]: i/request.Root is not a subtype of task.Scheduler` in the
  explicitly Scheduler-only `spawns` helper. `scope_call` used
  `request.within`, leaking its internal Root requirement through scope,
  local_scope, both, race and map_concurrently. Keep the run Root in the
  private immutable Scheduler value and use it only to allocate the scope's
  Owner. Retain the caller's services around the body, bind that Owner, and
  close it through `resource.ensure` on all returning/unwinding paths. The
  benchmark source and its requirements stay unchanged. Added Scheduler-only
  scope/local_scope helpers to the existing §9.9 task case and updated the
  library type snapshot to the specified schemes.
- i generation: `scripts/qa/generate.py` still emitted `@main`, producing
  `error[E0301]: the attribute @main is kept for macros` instead of exercising
  accepted models. Generate ordinary `main` bindings in `src/main.i`, and
  update the clause minimizer to replace that binding by name. All model,
  runtime, permutation and annotation-removal checks remain enabled.
- JS: the tests execute `target/debug/i`, but the JS verification part did
  not build it. Added Binaryen initialization and a locked debug CLI build
  before npm installation and tests, so this part runs independently.
- Editors: Zed's bracket query referred to the removed anonymous quote token
  and failed with `Query error at 5:3. Invalid node type "\""`. Pair named
  `string_start`/`string_end` nodes and `interpolation_start`/
  `interpolation_end` nodes within their parent nodes. The existing highlight
  queries already capture string pieces and escapes without capturing an
  entire interpolated string; both copies remain byte-equal. Manual queries
  on the string-form corpus confirm raw and multiline delimiters, escapes,
  nested interpolation, and function/variable captures inside interpolation.

## Sources and interpretations

- The ledger's USER decisions removing built-in `@main` and keeping named
  entries, and spec §10.2, control generated entry syntax.
- Spec §1.6 and Appendix C control raw, multiline and interpolated strings.
  Read the reference `tree-sitter-gleam/grammar.js` string rule and
  `queries/highlights.scm`: queries follow actual grammar nodes and capture
  escapes separately. i additionally captures string pieces so expressions
  inside interpolation retain ordinary highlighting.
- Read reference `gleam/format/src/lib.rs` (`pretty`) and its formatter
  tests, plus the formatter route in `.rd/ref/INDEX.md` §25. Used i's existing
  parse/format/parse and idempotence property instead of changing layout rules.
- Verified the lint exception against `records/graph.rs::Arena::intern`,
  `records/bridge.rs::Pool`, and `records/shown.rs` equality/hash: allocation
  uses interior mutability, while stored nodes and graph keys stay immutable.
- Spec §9.9's scope schemes control over the old library snapshot's extra
  Root requirement. Scope resource ownership belongs to its scheduler run;
  caller service bindings remain available to the body. Read reference
  `effect/packages/effect/src/internal/effect.ts` (`scoped`, `scopedWith`,
  `scopeUse`): allocate the internal scope, provide it around the body, and
  always close it on exit. Used i's existing Owner and resource.ensure for
  the same lifetime pattern, with no new state API.

## Verification

Commands use `/home/agent/Programming/i/.rd/cache/heavy direnv exec
/home/agent/Programming/i` for cargo and development verification.

- `cargo check -p ilang-check --all-targets`: passed.
- `cargo check -p ilang-wasm --all-targets`: passed.
- `cargo check -p ilang-cli --all-targets`: passed.
- `cargo test -p ilang-syntax --test format --locked`: passed, both tests.
- `cargo test -p ilang-syntax --test format format_properties --locked` with
  the original task source copy: passed.
- `scripts/dev-verify js`: passed, 104 Node tests and 60 browser tests across
  Chromium, Firefox and WebKit.
- `scripts/dev-verify editors`: passed, including query compilation,
  repository parsing, generated parser equality and Zed cargo check.
- Full Rust/i verification and the clean-worktree acceptance run are recorded
  below after completion.
