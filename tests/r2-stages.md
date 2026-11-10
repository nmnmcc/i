# R2 stages

The lane began clean at 5ec3d12 and fast-forwarded redesign to f8ba920.
There was no interrupted uncommitted work.

## Sources and interpretation

- runtime.md §11: "Commit R2 replacement spec text; merge S7 primitive text,
  preserving §9.8." Stage 1 adds the assigned spec sections before implementation.
  IO owns §9.7 and the opening/module-list migration; its existing text is retained.
- runtime.md §11: "Clock -> fs -> net -> process -> stdio" belongs to IO.
  R2 therefore leaves i/clock to lane c.
- runtime.md §6.3: "the first R2 implementation gate is to type-check this
  two-result-type/channel prototype before building the full scheduler."
  Stage 2 establishes keys, portability and retained environments, then that gate.
- runtime.md D5: "On the ilang target the entry is async exactly when N contains
  an async/request host service or Scheduler or Runner from i/task."
  Entries follow inferred needs; the unlanded provider-closure design is excluded.
- runtime.md §6.1: "A nested run takes an index from that same counter."
  Spawn and nested-run paths share the task counter.
- runtime.md §5.1: "A Ready cell has left its group and belongs to its handle."
  Group close preserves decoded answers; borrowed await does not cancel producers.

## Checkpoints

Each implementation stage requires the full dev-verify gate before its checkpoint
commit. The final audit also requires old-syntax-check. Generated expectations are checked separately
against their source rules and regenerated only in the final derived-files commit.

1. Spec migration: keys, retention, tasks, requests, entry classification and values.
2. Primitives and the heterogeneous monomorphic protocol prototype.
3. Request cells, rooted groups, observers, adapters and ABI v3 seam.
4. FIFO loop, entries, nested routing and retained task environments.
5. Structured lifetimes, cancellation, shielding, channels and shares.
6. Event registration, combinators and scripted tests.
7. Rust/Node/browser stress, public interface, WAT and benchmark audits.
8. Regenerate derived files.

## Stage 1 validation

`ILANG_BROWSER_PORT=4183 /home/agent/Programming/i/.rd/cache/heavy direnv exec
/home/agent/Programming/i ./scripts/dev-verify` completed successfully: Rust fmt,
clippy and workspace tests; 1,422 std, 2 hello and 12 bench tests at each opt
level; WASI smoke; 46 Node tests and 33 browser tests. Log:
`target/r2-stage1-verify.log` (local, not committed).

The first syntax-check invocation was interrupted with SIGTERM (143) while
waiting for a slot. Its outer heavy wrapper was redundant: the script already
wraps its cargo build. The corrected invocation is `direnv exec
/home/agent/Programming/i ./scripts/old-syntax-check`; it is pending a slot.
No source or environment setting changed during that correction.

Documentation describes the accepted contract; passing existing tests alone
does not establish the new runtime implementation.
