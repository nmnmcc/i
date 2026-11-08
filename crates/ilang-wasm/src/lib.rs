//! WasmGC code generation for the i compiler.
//!
//! The code generator is being rebuilt to follow `docs/spec.md`. For now this
//! crate only holds the runtime: hand-written WAT for big integers, bit arrays,
//! strings, the text form of values and float printing, which the new code
//! generator links into every program.

pub mod runtime;
