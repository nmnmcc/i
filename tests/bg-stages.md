# BG stages

BG implements host prerequisites only. No IO service or i library changes belong
in this track. ABI v3 and generated Wasm stay unchanged.

## Source and interpretation checks

- Native N9.1: "One host ResourceContext per live entry invocation."
  Each entry/export call gets a new context; workers share its clone.
- Native N9.2: "Numbers are never reissued within one live host."
  One host allocator issues positive arbitrary-precision numbers, including failed
  and provisional creations. A test limit stops allocation rather than recycling.
- Native N9.3: "Lookup, leases and close have one order."
  Short table transactions decide acquisition, completion and close. Cleanup runs
  outside that lock; close receipts survive removal and disappear on completion.
- Native N9.5: "General context-aware host registrations receive the invocation
  context at start; legacy callbacks may ignore it."
  Callbacks capture their immutable context. Entry exit, failure, abandonment and
  disposal close admission and initiate cleanup independently of language finals.
- Native 3.1: "Check every required declaration before instantiation/import
  coalescing; an unauthorized alias must not inherit a grant from an authorized
  declaration of the same Wasm name."
  The compiler's separate plan retains declarations and binds them to artifact
  bytes. A caller's supplied plan is not evidence of trust. Explicit approval or
  declaration grants are embedding authority; no library spelling confers it.
- Native 10: "Keep ABI v3 wire and H1–H14."
  The plan is an out-of-band loader contract, never new mandatory ABI metadata.

## Checkpoints

1. Resource primitives on Rust and JS: allocator, typed/kind lookup, leases,
   completion ordering, cleanup receipts, provisional ownership and teardown.
2. Invocation ownership and context-aware registrations on both hosts, with
   lifecycle and isolation regression tests.
3. Declaration binding plans and grants on both hosts, CLI defaults, documentation
   and authorization tests (including coalesced external aliases).
4. Merge current redesign, run the complete gates and compare WAT with that merge
   baseline. Record all failures and substitutes here.

A checkpoint is not completion. Continue through all four stages.

## Initial state

The lane was clean at b9173ce. There was no interrupted uncommitted work.

## Stage 1 results

- Rust resource tests: all seven passed in the first host test run; that run
  also found two integration failures outside these primitives (missing-import
  diagnostic priority and a test rebuilding private registration state). Both
  are being corrected in stage 2; the complete host suite will run again.
- `node --test bindings/javascript/test/resources.test.mjs`: 5 passed.
- The expectations above come from N9.1–N9.5, not generated expected outputs.
- `cargo metadata --no-deps --format-version 1`: workspace root is this lane.

## Stage 2 results

- `cargo test -p ilang-host --locked`: 66 passed, no ignored tests.
- `cargo clippy -p ilang-host -p ilang-cli --all-targets --locked -- -D warnings`:
  passed. The first Clippy run found one test initializer style error, corrected
  without changing any assertion.
- `npm test`: 58 passed. `npm run test:browser -- tests/browser/bg.spec.mjs`:
  6 passed across Chromium, Firefox and WebKit on port 4184.
- The first JS lifecycle run failed two compatibility tests: ending the startup
  context too early, and throwing synchronously from an async wrapper. Corrected;
  the existing assertions pass. The initial private HostImports layout update
  also caused an E0609 test build error, corrected in that owned test file.
- The first complete Rust host run found missing-import diagnostic precedence
  and a test rebuilding registrations without preserving their authority.
  Corrected; missing registrations still report the original load error.
- An initial JS concurrent-run test hung because the existing fixture starts
  two requests per call. Terminated that test process, corrected its deterministic
  gate, and reran successfully. This was a test-harness correction, not a runtime
  semantic change.
- Request futures now have controller-owned cells: teardown drops pending work
  and wakes idle waiters without a Store borrow or another worker poll. Active
  finite polls finish outside every host lock and cannot publish after teardown.
- New regression tests cover close during an outstanding request, late success,
  reentrant teardown, creation jobs before a resource exists, and both sequential
  and concurrent invocations. Expectations are N9.1–N9.5 and native 3.1.
- Test logs were initially written to temporary files outside the lane; they
  have been moved into this lane's ignored target/bg directory. No temporary
  logs remain at their original paths. No other checkout was edited.

## Stage 3 results

- `cargo test -p ilang-cli --lib --locked`: compiler provenance regression passed
  at O0 and O2. Two used external declarations produce one wire import and two
  independently authorized plan entries (native 3.1's alias sentence).
- Binding plans cover the final optimized artifact digest, package content,
  module/source digest, declared host type/field or external name, kind and wire
  signature. They never enter ilang.abi. `i run`/`i test` retain the compiler plan
  in process and approve only the loader-selected library content identity.
  `i build` exports a separate .bindings.json for explicit embedding approval.
- JS authorization compiles exact bytes after checking SHA-256. Its opaque token
  can authorize only that module object. JSON assertions alone are not authority.
- Interpretation: native 3.1 says "arbitrary custom imports remain the embedder's
  explicit authority." Existing legacy registration APIs deliberately grant all
  declarations of their named custom import. New context registrations are
  restricted by default. Standard resource factories must use the latter.
  Both host READMEs list every existing built-in/spec/custom fixture binding.
- Shared compiler/CLI files: lower.rs captures declarations before coalescing;
  ilang-wasm/src/lib.rs exposes the separate plan; ilang-cli/src/lib.rs and
  main.rs preserve the compiler trust path and emit the sidecar. These implement
  native 10's explicit "IO trust plumbing" handoff. No checker, i library,
  request-table implementation, opaque lowering or equality changes were made.
- The stage-2 SHA-256 dependency updated Cargo.lock through `cargo check`, without
  cargo update. This generated lock change is held for the required final derived
  commit; it remains in the working tree during verification.

## Stage 4 results

- Merged redesign 7b3cea3ed2602744bcbb8c747ebffe177864f1a2, including P1 and
  the accepted JS design. The only conflict was appended host tests; preserved
  both complete sets and the incoming opaque-import setup. No opaque lowering
  or equality implementation was edited by BG.
- Merge incident: a guarded helper stopped, but the shell continued and committed
  unresolved markers. The first full verification stopped at formatting/parsing.
  Corrected the file, verified both test sets, and amended that merge to 68a4040.
  No conflict markers remain in the branch.
- Heavy-wrapped `ILANG_BROWSER_PORT=4184 ./scripts/dev-verify`: exit 0 (rust, i,
  js); 67 Rust host tests, 60 Node tests, 42 browser tests. The spec suite reported
  168/168 cases passing at both --opt 0 and --opt 2. No tests were ignored or
  assertions weakened. The dev shell's pre-existing Playwright host-requirement
  validation setting was inherited unchanged; all three engines ran their tests.
- Additional final `./scripts/dev-verify rust`: exit 0 after adding the CLI
  library-content grant regression; both CLI provenance tests passed. Full
  formatting, Clippy -D warnings and workspace tests passed again.
- `scripts/old-syntax-check`: exit 0; 1163 extracted literals/blocks, exact
  expected diagnostics, 19 unchanged future-track entries. BG adds no syntax.
- Actual WAT dumps: `WAT_DUMP=... cargo test -p ilang-cli --test wat_dump --locked`
  on a source archive of redesign 7b3cea3 and on the merged lane. Compared every
  filename and byte: 636/636 equal, including 410 WAT outputs and 226 identical
  ERROR outputs; no missing/new/different files. These are real compiler runs,
  not a substitute measurement. The archive and outputs are inside target/bg.
- Build-cache incident: compiling the archived baseline in the lane's Cargo
  target directory left its older ilang-wasm artifact visible to a later CLI
  build (E0425 and cascading E0277). An mtime-only retry did not fix it. Ran
  `cargo clean -p ilang-wasm` through the heavy wrapper, rebuilt the lane's real
  crate, then reran both CLI tests, the lane WAT dump and the complete Rust gate
  successfully. No baseline build ran afterward. The cache is restored to lane
  sources; no persistent environment variable was changed.
- `I=.../target/release/i ./scripts/build-abi-fixture.sh` generated the compiler
  binding-plan sidecar. Existing Wasm fixtures and the feature probe are
  byte-unchanged. The JS integration test loads that real compiler plan through
  restricted context callbacks and verifies fresh invocation ownership.
- Initialization interpretation: Appendix B.1 permits synchronous imports during
  ilang_start; native N9.1 requires fresh entry/export invocations. Initialization
  therefore has a separate context, torn down before the first SDK invocation
  or disposal. Direct raw Wasm export access remains the embedder's ABI duty.
- No semantic conflict between controlling sources was found. There are no
  outside-lane implementation escalations. IO services, R2's request rows and
  multicore worker transport remain assigned to their later tracks.
