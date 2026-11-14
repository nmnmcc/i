# Native-tree integration with main9b806113

Branch: lane/fa. Merge parent: origin/main9b80611373944e7855ee77420b2e384bf523250b.
Previous green prefix:2d124a5c85bc4d90427c322ed94be43d672a205a.
Checkpoint19:10: workspace/Rust/reader/CLI pass; spec/snippets gate in progress.
Final phase results and the landable hash are recorded in shared status-fa.txt.

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
