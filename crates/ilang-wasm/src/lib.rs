//! WasmGC code generation for the i compiler.
//!
//! For now this crate holds the runtime: hand-written WAT that every program
//! links. `runtime/mod.rs` documents the interface for the code generator.

pub mod runtime;
