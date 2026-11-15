# Native-tree integration with main9b806113

Branch: lane/fa. Merge parent: origin/main9b80611373944e7855ee77420b2e384bf523250b.
Previous green prefix:2d124a5c85bc4d90427c322ed94be43d672a205a.
**Targeted merge gate GREEN at19:18:02 HKT.** Main9b806113 is merged.
Compiler implementation is unchanged from74dc0b2f1815a5aa19d0b54102db95738af75238.
The final follow-up changes only this report, a test comment and its exact
negative snippet registry entry. Final hash is in shared status-fa.txt.

Main's MP01–MP16 and F1 behavior is authoritative. Production lower.rs and
old tree remain deleted. No incoming test or corpus fixture is weakened,
deleted or locally rerecorded.

## Ports and conflict decisions

- Main's F1 changes to known.rs, infer.rs, groups.rs, pattern.rs and typed
  macro checking merge into the shared HIR checker. No replacement behavior
  is introduced. Spec4.8 says: “Selection, rule-12 factual narrowing and
  W0503 use only factual known types.” The unchanged new F1 tests remain.
- MP06 package semantic identities are initialized in the native resolver
  and independent private reference. Main's package integrity, source
  snapshot, publication-contract and portable word-hash implementations
  merge intact (spec2.7,14.8,14.9).
- MP15 native entry validation retains definite literal errors and defers
  aliases/computed function values to inference, exactly as main. Main says:
  “The target is checked by its inferred function type; aliases and computed
  function values are allowed.” Shared entry runtime and collection changes
  merge intact. Main's alias acceptance assertion and invalid anchor stay.
- MP10/MP11 explicit labels and implementation shape groups are represented
  by native MacroDefinition metadata. Ordinary source grouping retains the
  written macro interface; preparation builds compiler-only native group
  bindings, canonicalizes syntax parameters, exposes selected targets only
  in that compiler closure, and rewrites wrapper fields directly in the
  native AST. No production lowering or old AST is restored.
- Native macro signature validation now uses the single native definition
  reader. It publishes typed_macro_shapes to HIR, validates every advertised
  shape including unused ones, and allows extra implementation groups.
  Spec14.8: “Correspondence is checked at the definition, including shapes
  that no caller uses.” The reference-only lowerer/reader receives main's
  metadata changes, retaining the previous native boundary adapters.
- Typed call labels come from the same native argument builder/token index
  as their syntax and spans; labels and arguments use the same query order.
  Body-engine compilation/cache selection includes the implementation group;
  validation compiles every group rather than selecting only the first.
- Pre-validation examines a grouped compiler view of each macro, retaining
  all other errors instead of issuing stale stage9 errors on supported
  labels/multiple shapes. The written interface remains unchanged.
- Reader fidelity includes generic shape branches and projection of invalid
  mixed/spread signature data as main does. Two extra cross-reader inputs
  pin this; main's new tests are unchanged. The existing reference adapter
  compares item/signature source spans instead of indices after insertion
  of compiler-only hidden items, preserving correct-item assertions.
- MP12/13/14/16 shared transaction, readiness, Diagnostics and lexical
  shadowing changes/tests remain intact. The snippet dictionary is merged
  with the supplied jsonmerge-snip.py after reading it, retaining both
  branches' additions and main's intended deletions.

## Verification record

All builds/tests use /home/agent/Programming/i/.rd/cache/heavy; one heavy
command at a time. Check-snippets is direct and owns its internal wrapper.
No std suite, corpus run, full CLI suite or unfiltered spec suite is run.

- Workspace check1 failed on a duplicate definitions re-export; removed.
- Check2 exposed old MacroDefinition test initializers. Restored Git's
  auto-merged nonconflicting test changes and retained both branches' full
  appended tests; no incoming assertion/body is discarded.
- Workspace check3 passed, then the first Rust run stopped at its first
  failure: three file probes emitted stale stage9 shape errors. All566
  complete programs already matched. Fixed native pre-validation; check4
  and the complete Rust retry pass, including the strict current-repository
  HIR/diagnostic differential.
- The Rust gate selects all library/bin/integration targets of ilang-syntax,
  ilang-macro, ilang-check and ilang-package except the requested value_types
  integration binary; the prior requested recovery skip is retained. Exact
  selector argv: target/fa-main/rust.command. Four harness threads, one
  binary at a time. The final reader fidelity corrections are additionally
  covered by a fresh workspace check and syntax library/lisp_syntax follow-up.
- Final workspace all-target check PASS; focused syntax follow-up PASS
  (29 library and48 lisp_syntax tests); all four crates' doctests PASS.
- Rust formatting of workspace and private migration crate PASS.
- Remaining commands/results are captured individually in
  target/fa-main/*.command, *.log and results.txt and will be completed below.

19:10:17 CLI gate PASS unchanged: compile_cache3, known_type_regressions3,
macro_run66, package_commands34. This includes all new main regressions,
portable hash vectors, explicit labels, deferred definitions, marked local
shadowing, cache byte/symlink checks and macro publication contracts.
Spec4.8 is active; spec9 and14 and direct snippets follow, stopping on any
failure. A19:10 status line records this exact remaining set. This commit
is a compiling/tested checkpoint; final green is reported only after the
remaining commands complete. All phase commands/results are in
`target/fa-main/results.txt` and matching `.command`/`.log` files.

## Final results

- Workspace `cargo check --workspace --all-targets --locked`:PASS.
- Requested Rust packages:725 passed,0 failed,4 existing ignored. The
  value_types integration binary is omitted as requested; the existing
  recovery-test skip is retained. Syntax follow-up:29 library+48 integration
  PASS; all four crates' doctests PASS.
- CLI compile_cache3, known_type_regressions3, macro_run66,
  package_commands34:all PASS unchanged.
- Spec filters4.8/,9/,14:31/141/14 cases PASS at both O0/O2. The14 substring
  covers14/ and all MP fixtures14.5–14.9, including labelled/multiple shape
  groups, typed diagnostics, absent-definition retries and portable hashing.
- Ordinary direct check-snippets built a fresh driver, then reported one
  new intentionally invalid mixed-signature probe with unregistered E0101.
  The probe tests main's definition metadata despite the separate unchanged
  stage6 mixed-data rejection; it was never a positive program fixture.
  Register that exact full diagnostic and an adjacent negative-case marker;
  direct `scripts/check-snippets --reuse-parser` PASS19:18:02:5129 blocks,
  48 Lisp files,101 macro fragments,5 unchanged future-track entries.
  No compiler producer, source literal, assertion or main fixture is changed
  by this registration. No expectation is blindly regenerated.
- Main's added macro/syntax test bodies remain byte-identical; all other
  incoming fixtures are imported unchanged. Production lower/old reader
  paths remain absent, and origin/main9b806113 is an ancestor.
- No std suite, corpus or full CLI suite was run. No required check remains
  pending. The19:10 pending status and19:11 merged checkpoint were posted;
  the final MERGED-MAIN green status follows this documentation commit.

## Exact phase commands

H is /home/agent/Programming/i/.rd/cache/heavy. All cargo commands below
use that wrapper. Source formatting is nonbuilding and snippets is direct.

`$H direnv exec /home/agent/Programming/i cargo test -p ilang-syntax -p ilang-macro -p ilang-check -p ilang-package --locked --lib --bins --test alias_unions --test body_walk --test body_walk3 --test coalescing --test codegen --test compiler_entries --test diagnostic_traps --test expanded_modules --test expansion_origins --test field_privacy --test format --test front_end --test generic_services --test git --test hir --test hir_deep --test hygiene --test hygiene_resolve --test known_facts --test lex --test library --test lisp_datum --test lisp_deep --test lisp_definition_rows --test lisp_encode_deep --test lisp_format --test lisp_record --test lisp_syntax --test lisp_syntax3 --test literal_types --test load --test lock --test macro_names --test macro_sandbox --test manifest --test mvs --test oracle --test parser_deep --test projects --test provider_graph --test providers --test query --test quote_patterns --test record_schemes --test row_schemes --test spec --test string_forms --test syn3 --test syn3_lower --test tree_helpers --test types --test types_joint --test unused --test unused_audit -- --skip spec_corpus_reverse_order_and_recovery --test-threads=4`

`$H direnv exec /home/agent/Programming/i cargo check --workspace --all-targets --locked`

`$H direnv exec /home/agent/Programming/i cargo test -p ilang-syntax --lib --test lisp_syntax --locked -- --test-threads=4`

`$H direnv exec /home/agent/Programming/i cargo test -p ilang-syntax -p ilang-macro -p ilang-check -p ilang-package --doc --locked`

`$H direnv exec /home/agent/Programming/i cargo test -p ilang-cli --locked --test macro_run --test known_type_regressions --test package_commands --test compile_cache -- --test-threads=4`

`$H direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test spec --locked -- 4.8/`

`$H direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test spec --locked -- 9/`

`$H direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test spec --locked -- 14`

`scripts/check-snippets`

Final snippet retry: `scripts/check-snippets --reuse-parser` (exit0).
Initial workspace check attempts and the first differential failure are
preserved in target/fa-main/check*.log and rust1.log; the complete successful
Rust retry is rust.log. Phase timings/exits are results.txt. No failure is
hidden by replacing an expectation or deleting a test.
