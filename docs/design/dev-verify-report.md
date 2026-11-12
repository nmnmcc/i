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
- i review corpus: the two `10_macro_attr_tuple` observations still listed
  removed built-in `@main`/`@test` attributes in the E0301 message. Compared
  actual output with each saved observation: rejection, code and source
  position are unchanged. Regenerated only these two diagnostic observations
  against the current CLI; their independent requirements remain unchanged.
  Five further A1 observations were stale after syntax migration: both
  `09_attr_tuple_same_line` copies now contain only an ordinary tuple binding
  (the migration removed their attribute), both `26_guard_fn_ungrouped`
  copies use `?` as a guard after a function type (spec §§3, 7.1), and the
  historical `25_old_guard_fn/original` input still rejects with the current
  parser's E0101 rather than the old record-brace diagnostic. Inspected the
  sources and complete release-CLI observations, including both startup runs
  for the four now-valid inputs, before refreshing these five snapshots.
  The duplicate `runtime/25/original` input needs the same parser diagnostic.
  Reviewed later differences before copying their complete release results:
  K4 conflict/invariant observations change only rendered error slots from
  `? E` to `^ E`; JS coarse/raw missing-host diagnostics now identify their
  service, and startup-stop diagnostics explicitly name `ilang_start`.
  These preserve error codes, exits, source positions and runtime failures.
- i WASI smoke: its generated stopping program also used removed `@main`
  and failed with E0301 after the hello module built. Generate the ordinary
  main binding; retain both engines, both optimization levels, output/stop
  checks and expected exit codes.
- Corpus resource reproduction: two bounded runs of
  `scripts/review-corpus run --filter review-k4/perf/wide_generic_1024
  --binary target/release/i` reached test-phase timeouts with the saved
  10-second cap. The original source plus the ordinary injected
  `test("qa_startup", () { Nil })` reproduces it; an unbounded O0 execution
  completes successfully in about 10.1 seconds. The observation budget and
  required acceptance remain unchanged. Phase measurements isolated about
  6 seconds in type checking; a debugger sample identified `Norm::union_in`
  repeatedly comparing members with unrelated type heads. Index members by
  head while preserving first-occurrence order and all distinct same-head
  service arguments. The unchanged 10-second bounded reproduction now passes.
  Read Simple-sub's `TypeSimplifier.scala::CompactType`/`merge`, which groups
  components using sets and maps, before making this local normalization fix.
  Temporary profiling source was removed.
- K4c missing-service observations predated the source/target split. Under
  spec §2.6.2, untargeted checking may succeed; both test/run target builds
  must still reject before execution. Inspected representative sources,
  inferred rows and all four target-phase outcomes, then refreshed only
  observations whose previous errors were E0522 and whose four target phases
  now reject with the planner's E0523. The 80 explicit missing-service code
  requirements now require E0523 (`providers::PLAN_ERROR`) instead of the
  removed source-only E0522; each still requires rejection before execution,
  and incompatible-provision E0515 requirements remain unchanged. No extra
  diagnostic code or successful runtime outcome was allowed.
  Recursive K4c recovery observations also retained a previously printed
  `b : () -> Nil` line after E0505. Both the pre-normalization debug CLI and the current
  release CLI omit it; the same error, location and rejection remain. Updated
  only snapshots whose complete difference is that omitted recovery type
  (plus the separately reviewed error-slot spelling), consistent with
  `infer.rs` omitting recursive schemes after a diagnosed failure.
  K4d has the same source/target transition, including cases where E0501 or
  E0515 already rejects the source: removed only obsolete source E0522 lines
  and adjusted their diagnostic count, retaining the remaining errors exactly.
  For now-source-valid inputs, verified all observed O0/O2 target phases
  reject with E0523 and that inferred types agree apart from `^` spelling
  before updating their observations.
- The unchanged mutual100/150/200/300 fixtures also timed out (saved caps
  10/10/10/60 seconds). A second isolated mutual100 run reproduced it:
  `scripts/review-corpus run --filter review-k4d/perf/mutual100 --binary
  target/release/i`. A debugger sample found repeated structural equality in
  `Coalescer::go`: alias-class members shared identical stored bounds, which
  were recursively expanded before the resulting trees were deduplicated.
  Deduplicate those identical input bounds on the same active path before
  expansion. Alias-equivalent keys preserve row flags and graph constructor
  roles, so occurs checks see the same paths. Cycle detection and
  path-sensitive memoization remain intact; all checker type goldens pass.
- The preserved std fixture's source check exceeded its unchanged 10-second
  cap. `i expand` alone took 23 seconds; debugger samples found whole-module
  AST clones for every item macro. Share immutable item trees across frozen
  attempts and materialize the definition module on its first actual query.
  Snapshot ordering, retries, audits and rollback remain unchanged. Share
  compiled imported macro programs across the project's expansion session;
  each invocation retains its own fresh guest, budget and resolver. Followed
  rust-analyzer's `proc-macro-srv::ProcMacroSrv::expander` cache of shared
  compiled expanders. The existing macro and budget suites pass, including
  the new two-caller definition-context regression. Its first draft queried
  a value with the type-only definition API (E0234); corrected the test to
  query distinct caller type aliases, then reran the full suites successfully.
  Further samples found project-wide resolver copies and repeated lexical
  indexing of the same source for every argument. Freeze the existing import
  closure (which includes macro-template sources) and reuse the existing
  SourceEncoder once per engine. The preserved std source now checks within
  its original cap; the large literal key case also passes O0/O2 under ten
  seconds. Verified the std snapshot differs only by 1446 hygienic test names
  and six ordinary test helper bindings; its two existing censored runtime
  observations and budgets are unchanged.
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

- Later K4d/K4s/K4-lane and R2 observations had the same source/target
  split and error-slot changes. Refreshed 81 complete observations only
  after mechanically checking the full difference: 40 preserve target E0523
  rejection, nine remove obsolete E0522 alongside unchanged source errors,
  and 32 change only error-slot spelling or remove scope's accidental Root
  requirement. All saved budgets and semantic requirements stay intact.

- Reviewed the remaining historical observations: eight only rename
  macro-generated test bindings, seven add the service to missing host import
  errors, and 17 follow already-migrated test macros, source positions,
  declared entry names, scope schemes or parser diagnostics. Runtime test
  names, pass/fail counts, exit status and saved budgets are preserved.
  Three old `mixed_install` cases now reach E0515/E0501 instead of the removed
  built-in @test module restriction; they still reject. The cancelled-read
  source now checks because `i/net` exports Stream/read; its startup tests
  succeed at both levels. No semantic requirement was changed for these.

## Verification

Commands use `/home/agent/Programming/i/.rd/cache/heavy direnv exec
/home/agent/Programming/i` for cargo and development verification.

- `cargo check -p ilang-check --all-targets`: passed.
- `cargo check -p ilang-wasm --all-targets`: passed.
- `cargo check -p ilang-cli --all-targets`: passed.
- Final macro changes: `cargo check --release -p ilang-cli --all-targets`,
  `cargo build --release --locked -p ilang-cli`, and `cargo fmt --all --
  --check`: passed. `cargo test --release -p ilang-cli --test macro_run
  --test macro_budget --locked`: passed, 57 expansion and nine budget tests.
- Bounded reproductions: mutual50/100/150/200/300, nested50 and declaration
  order118 now match their successful snapshots. The large literal key case
  passes O0/O2. The std checker succeeds under ten seconds; refreshed only
  its previously described migrated type output, with existing runtime
  timeout observations retained.
- After the normalization fix, `cargo check -p ilang-check --all-targets`
  and `cargo test -p ilang-check --lib --test generic_services --test
  row_schemes --test types --locked`: passed, with type goldens unchanged.
- `cargo test -p ilang-syntax --test format --locked`: passed, both tests.
- `cargo test -p ilang-syntax --test format format_properties --locked` with
  the original task source copy: passed.
- `UPDATE=1 cargo test -p ilang-check --test types library --locked`: passed;
  inspected the snapshot changes (private scheduler constructor plus removal
  of the accidental Root requirements).
- Rebuilt the debug CLI; `i check --project bench --types` and
  `i test --project bench --opt 0`: passed, all 15 benchmark tests.
- `cargo test -p ilang-cli --test spec --locked -- 9.9`: passed, 78 cases at
  both optimization levels.
- `scripts/test-generation --mode short --binary target/debug/i`: passed,
  32 cases (20 accepted, 12 rejected by the model), 64 declaration
  permutations, 32 annotation removals, zero failures.
- `UPDATE=1 scripts/review-corpus run --filter 10_macro_attr_tuple --binary
  target/debug/i`: passed, both rejected inputs retain their rejection.
- `scripts/dev-verify js`: passed, 104 Node tests and 60 browser tests across
  Chromium, Firefox and WebKit.
- `scripts/dev-verify editors`: passed, including query compilation,
  repository parsing, generated parser equality and Zed cargo check.

The exact supplied clean-worktree acceptance script runs after this report
is committed. Its final combined output is captured in
`/tmp/js-acceptance-final-green.log`;
individual part outputs are `/tmp/dv-{rust,i,js,editors}.<shell-pid>`. It runs
each part separately through the heavy wrapper, requires each exit status to
be zero, compares the verification script against `lane/integ`, and checks
that this report exists. No check or existing test was removed or disabled.

An extra full-corpus diagnostic run with the debug CLI was stopped after
50 cases, then restarted with the release CLI used by the i gate. The
per-case runtime budgets are unchanged; final verification runs the complete
1973-case corpus with the release CLI.

One intermediate overlapping Cargo run failed at doctests with
`error[E0460]: found possibly newer version of crate ilang_check which
ilang_macro depends on`. Another Cargo build had replaced artifacts while
workspace tests were running. Final acceptance waits for that workspace
command to finish and runs without overlapping Cargo builds. No Cargo
command was interrupted. Earlier bounded std and literal failures were
`review-k4s/std: unclassified checker timeout` and
`review-t1/stress-keys-2000: full output changed`; the fixes above retain
their original limits and successful checker/runtime requirements.
