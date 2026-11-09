# WasmGC code generation

This crate holds the runtime that every compiled i program links: hand-written
WAT for big integers, bit arrays, strings, equality and hashing, the text form
of values (§11.8 of the spec), the shadow stack and stop messages (§11.7), the
`i/core` operations (§13), and the conversions of the binary interface
(Appendix B). `src/runtime/mod.rs` documents the interface that the code
generator uses.

The code generator itself is being rebuilt to follow `docs/spec.md`. It emits
simple, unoptimized WasmGC, and `ilang-binaryen` optimizes the result.
