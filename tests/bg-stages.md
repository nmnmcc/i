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
