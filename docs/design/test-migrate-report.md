# Test macro migration

No new language/library state surface was added.

The USER ledger decision of 2026-11-11 removes `@test` after migration.
The implementation follows spec §10.2 and `testing-macro.md`: `i test` selects
only `[test] collection`, compiles its roots once, and runs fresh instances.
Unqualified `@test` has no built-in meaning and is rejected through ordinary
attribute/macro diagnostics. General attribute parsing, formatting and editor
highlighting remain available for ordinary user macros; no test-name exception
is needed in those components.

## Counts

| Command / suite | before | after |
| --- | ---: | ---: |
| `i test --project std --opt 0` | 1845 passed, 0 failed | 1845 passed, 0 failed (release O0) |
| `i test --project std --opt 2` | not separately rerun before migration | 1845 passed, 0 failed |
| `i test --project tests/perf/many-tests` | 2000 passed, 0 failed | 2000 passed, 0 failed |
| `i test --project examples/macro-tests` | 8 passed, 0 failed, 1 skipped | 8 passed, 0 failed, 1 skipped |
| `cargo test -p ilang-cli --test spec --locked -- --nocapture` | 417/417 cases, O0 and O2 | 417/417 cases, O0 and O2 |

The four requested before runs completed before repository source edits.
Logs are under `target/test-migrate/` (`before-*.log` and subsequent validation
logs). The existing release executable was used for the three CLI before runs;
the spec before run built the lane's original Rust sources.

## Mechanical migration and hand rewrites

`scripts/test-macro-migrate --embedded --standalone std lib examples tests
bindings bench docs crates editors` migrated 3,661 declarations in 36 packages.
Its extension decodes Rust/JavaScript literals, renders Rust format placeholders
for span inspection, reads Markdown fences (including indented fences), and
handles editor corpus source sections. Bodies/comments survive span edits;
referenced/exported functions remain named and get a registering wrapper.
Temporary-package manifests are explicitly configured by their Rust harnesses.
A repeated complete migration reports `0 tests migrated in 0 packages`.

Hand rewrites:

- `tests/spec/10.2/main_and_test.i` and
  `tests/spec/2.2/a1_builtin_attribute_call.i` now reject the removed attribute
  with E0301. `attr_entry` checker fixtures likewise retain deliberate negative
  uses. These five lines are the only active `.i` lines starting with `@test`.
- Checker-only fixtures `projects/entry_names`, `projects/ok_multi` and
  `types/entry_checks` use ordinary named functions. The type golden harness
  selects the latter through the general consumer-root API, preserving the
  Nil-result check without depending on macro expansion inside the checker.
  Unused-binding tests use ordinary underscore roots to retain helper references.
  `value_type_review/runtime` separates its ordinary source functions from the
  four macro test wrappers. The core review harness checks source without test
  expansion; the package still runs all four tests through the normal CLI.
- Generic attribute parser/printer fixtures (`golden/attributes` and
  `lisp_syntax`) use `@custom`; their subject is attribute syntax, not testing.
- `syntax_runtime`, `macro_budget`, `commands`, `compile_time_tests`, and
  package-command harnesses configure the collection and select its real roots.
  The Lisp quote fixture calls the imported `test` macro. Metered lowering takes
  explicit consumer-selected roots. `single_file` now writes ordinary source
  directly, removing the attribute-to-test-module rewrite.
- `i new` imports the ordinary test macro and writes a collection/dependency
  manifest. Its std dev-dependency path is the compiler source tree's standard-library package,
  matching the local path dependencies used by repository examples (spec §2.6).
  Test-only imports use dev-dependencies; source-module imports require regular
  dependencies. `i deps` and its permission reporting expand macros before
  checking the package.
- The deliberately invalid VS Code lexical sample uses a test call; its snapshot
  is regenerated. The general tree-sitter grammar is regenerated and tested.
- `std/test/property_failures.py`, the §9.10 vector generator/replay adapter,
  and the review-corpus startup probe emit/consume the macro forms. The active
  review copy of `std` includes the same ordinary `src/test.is` macro module; its
  corpus manifest includes that module when materializing the package.
- Spec §2.2, §2.4.1 and §10.2, the test-macro design, and runnable Markdown
  examples describe registration through the collection. The unused-function
  hint recommends `test` registration.

No tests or fixture files were deleted. Removed code consists of the legacy
attribute flag/validation, implicit test export discovery, CLI compatibility
loop, and the spec harness's legacy rewriting. Archived review evidence under
`tests/review/tools/` and original-payload inventory hashes remain historical
artifacts, as documented in `tests/review/README.md`; active review cases were
migrated. Old-syntax classifications were rekeyed for changed live snippets and
removed for deleted legacy assembly strings.

## Validation

Recorded checks:

- `cargo check -p ilang-cli -p ilang-check -p ilang-syntax -p ilang-macro -p ilang-wasm --all-targets`: passed; all Cargo commands used the shared heavy wrapper.
- `UPDATE=1 cargo test -p ilang-check --test projects --test types --locked`: passed (2 project checks and 5 type checks).
- Targeted CLI checks: check 10/10, commands 14/14, compile-time templates 3/3, macro budgets 9/9, many-tests 2/2, package commands 27/27, syntax runtime 10/10, unused warnings 1/1 passed. Intermediate failures below were corrected and the affected targets rerun.
- `scripts/old-syntax-check`: exact reviewed diagnostics match after rekeying changed snippets.
- Tree-sitter `npm ci`, `npm run generate`, `npm test`: passed, 10 corpus cases; generation made no parser-byte changes because attributes already use the general grammar.
- VS Code `npm ci`, `UPDATE=1 npm test`: passed, 8 tests; annotation expectations and the lexical snapshot now use the macro call.
- Embedded migration smoke check: 3 declarations migrated across raw/formatted Rust literals, ordinary Rust literals and an indented Markdown fence; repeat migrated zero.
- `python3 tests/spec/9.10/reference.py`: generated tokens match, including 510 rejection partitions.
- `python3 tests/spec/9.10/replay.py target/release/i`: passed 24 engine/case/optimization combinations. Replay extracts macro-call bodies by parser spans, including single-line calls and references to preserved functions.
- `I=$PWD/target/release/i scripts/build-abi-fixture.sh`: passed; derived browser artifacts remained byte-identical.

The final gate is the task's complete acceptance block: clean-tree check,
release build, removed-attribute rejection, the four CLI count checks,
`cargo test --workspace --locked`, `scripts/old-syntax-check`, and
`(cd bindings/javascript && direnv exec /home/agent/Programming/i npm test)`.
The invocation and its complete logs are in `target/test-migrate/acceptance.sh`
and `target/test-migrate/accept-*.log`.

Early failed commands and their corrected causes:

- A pre-existing release formatter printed `?` error types. Its formatting edits
  were discarded by reconstructing the parser-span migration, then formatting
  with the freshly built new-only executable.
- Core type-golden assertion failures: `app/main_test.passes has a tree`, then
  `app/main.main has no tree`. Reproduction: select an unexported ordinary
  function with `check_resolved_selected`; consumer roots have type trees while
  ordinary entries need not. The harness now permits selected roots' trees and
  retains the original public-binding requirements.
- CLI fixtures initially expected legacy export handles and stop positions;
  they now use collected handles and check macro expansion provenance.
- The first VS Code test run lacked `vscode-oniguruma`; `npm ci` installed the
  locked dependencies, and all eight editor tests passed.

The first after-spec run passed all 417 cases but exited nonzero because a
Python syntax check had left `tests/spec/9.10/__pycache__`, which the case
enumerator treated as a fixture (`the header has no spec: line`). The transient
bytecode directory was removed before final acceptance.

Other corrected intermediate failures were `error[E0308]: mismatched types`
when passing `ProjectChecked` to the provider planner (the command now supplies
its ordinary `Checked` view), stale manifest assertions after `i new` gained the
test collection, `templates must keep their source module: left 3, right 2`
(the assertion now inspects the two fixture quotes), and the old raw checker in
`i deps` reporting `error[E0234]: a call at the top level is kept for macros`
(the command and permission report now expand macros). The old-syntax check
initially reported obsolete hashes and the pre-existing formatter's `?` output;
all affected sources/classifications were corrected, with no broad exemptions.

No requested behavior remains deferred.
