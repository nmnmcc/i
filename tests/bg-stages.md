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

## Independent BG review repairs

The lane was clean at eb05ac3. Read the independent review and both sets of
reproductions. Ported them into repository tests; none uses /tmp or an external
probe. Both hosts were inspected for each class before choosing the repair.

| Finding | Cause and general repair | Regression names | Other host |
|---|---|---|---|
| 1 | Rust grants kept only a digest and individual declaration. Retain the entire ordered authenticated plan snapshot; any replacement needs fresh approval. | Rust bg_review_complete_plan_cannot_drop_a_denied_coalesced_declaration; JS "BG review 1: approval retains the complete plan despite source mutation" | JS already retained the snapshot; control passes. Both READMEs state the same rule. |
| 2 | Continuation and status-zero/result paths bypassed admission checks. Gate every managed guest call, including worker handoff, allocation and each free, and reject interrupted-instance reuse. | Rust bg_review_ended_context_blocks_runnable_continuations, bg_review_no_request_runnable_and_result_paths_do_not_enter_ended_wasm, bg_review_drop_at_status_zero_blocks_later_result_export, bg_review_gate_covers_sync_callbacks_allocations_and_each_free; JS review 2 tests and "BG review audit: sync callbacks, allocators and each free are gated" | Both affected and fixed; memory counters prove no guest call occurred. |
| 3 | JS inferred async handling from opaque result properties. Use operation/ABI kind; skip Promise-shape checks for synchronous externref replies too. | Rust bg_review_opaque_shape_is_not_an_async_protocol_on_rust; JS "BG review 3: synchronous opaque objects are never inspected for then or finally" | Rust values are statically tagged and were unaffected; its identity control passes. |
| 4 | JS left entries accessible while cancellation callbacks ran. Remove all entries and publish receipts before any callback; enforce liveness on acquire, commit and delivery. | Rust bg_review_reentrant_cancellation_cannot_publish_after_teardown; JS "BG review 4: cancellation cannot acquire, commit or deliver after teardown" and reentrant teardown test | Rust already removed entries first; strengthened explicit liveness checks and retained a reentry control. |
| 5 | JS exceptions, and Rust cleanup/destructor/wake panics, could interrupt later cleanup. Isolate each step; collect host diagnostics without changing close or the original invocation result. | Rust bg_review_cleanup_panics_do_not_skip_cancellation_wakes_or_cleanup, bg_review_cleanup_failure_is_reported_without_replacing_a_success; JS review 5 tests and request-abort diagnostic test | Rust had the same class through unwinding panics; both hosts fixed. |

Source checks are separate from execution checks:

- Finding 1 implements native 3.1's "Check every required declaration before
  instantiation/import coalescing" and the user's explicit complete-plan rule.
  The same bytes plus a shorter plan do not authenticate the omitted obligations.
- Finding 2 implements N9.5's "close the start/delivery gate" and no worker/i
  callback requirement. Appendix B.5 requires the result export after status 0;
  it does not permit entering it after the host has ended the context.
- Finding 3 preserves Appendix B.2's opaque host-reference value domain. A
  synchronous externref can be a thenable, Promise or proxy; its properties are
  not the SDK's scheduling protocol.
- Finding 4 follows N9.3's ordered lookup/lease/close transaction. Counter fixtures
  and closed expectations come from those source rules, not generated goldens.
- Finding 5 keeps N9.3's "Close is idempotent/error-free" and N9.5's scheduling
  of every cleanup. Errors are diagnostics in a host-wide drainable queue:
  take_cleanup_errors (Rust) / takeCleanupErrors (JS), also forwarded by the
  instance/guest. Records have cancel/wake/cleanup phases; Rust retains panic
  messages and its normal panic hook, JS retains original thrown values and
  observes rejected cleanup Promises. No console/panic hook or ABI is installed.

The further audit found and covered argument getters ending a JS invocation
before its actual export call, raw allocator/free paths on both hosts, later
result exports after request-drop callbacks, reuse after a failed gate, and
fresh-invocation reentry during synchronous cleanup. Supplied JS AbortSignals
keep native brands/listener removal while isolating user listeners, including
onabort. Rejected cancellation callback Promises are observed as late diagnostics;
registered cleanup jobs remain the way to join independent asynchronous cleanup.

Checkpoint checks: cargo test -p ilang-host --locked: 76 passed; targeted Clippy
-D warnings passed; npm test: 75 passed; browser review suite: 9 passed across
all three engines. The initial JS reproduction run failed four cases with its
plan-token control passing, then passed after repair. The first Rust reproduction
run waited for a heavy slot and compiled after repairs were present, so it is
not a pre-fix measurement. One additional Rust regression build failed because
its boxed test future omitted Send; corrected the test type without weakening an
assertion. Existing tests and all new paired controls pass. Full post-merge
verification follows this checkpoint.

The final cancellation audit also found that JS Set membership used the callback
itself as its key, so two registrations of the same callback could collapse or
one detach could remove both. Registrations now have independent tokens; "BG
review audit: repeated cancellation callbacks remain independent registrations"
checks two remaining registrations both execute. Rust work cells already have
independent identities (the paired two-work cancellation test checks both).
The 16 JS review tests pass after this change.

Rust panic payloads can themselves have panicking destructors. The error boundary
now isolates finite payload-destructor panic chains outside the diagnostic lock;
bg_review_a_panicking_panic_payload_cannot_skip_later_cleanup passes, as does
Clippy -D warnings. JS thrown values have no synchronous destructor and remain
stored unchanged. One edit of this audit helper initially omitted its loop-state
assignment, causing E0382 and an unused-mut warning in the targeted build; fixed
both without changing the regression assertion. The full post-Std-2 gate passed
(76 Rust host tests, 76 Node tests, 51 browser tests); the next full gate also
covers this final Rust payload test after merging the newer documentation.
