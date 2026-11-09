# WasmGC code generation

This crate turns a checked i program into one WasmGC module with the binary
interface of Appendix B of the spec.

- `src/runtime/` is the runtime that every program links: hand-written WAT for
  big integers, bit arrays, strings, equality and hashing, the text form of
  values (§11.8), the shadow stack and stop messages (§11.7), the `i/core`
  operations (§13), and the conversions of the binary interface.
  `src/runtime/mod.rs` documents the interface that the code generator uses.
- `src/ir.rs` is a small intermediate form in A-normal form, and documents the
  calling convention.
- `src/lower.rs` lowers the resolved program and its types to it.
- `src/control.rs` splits the functions that may suspend (§9.4).
- `src/emit.rs` writes WAT text with the runtime, which is assembled, and adds
  the `ilang.abi` custom section.

The code is simple and not optimized: `ilang-binaryen` optimizes it. The
environment (§9.2), errors (§9.3), control (§9.4), final functions (§9.5), and
`Assert` (§9.6) are lowered as `src/ir.rs` documents; host services and
external functions become imports (Appendix B).
