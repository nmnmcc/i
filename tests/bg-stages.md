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
