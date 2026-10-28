//! WasmGC code generation for the Wleam compiler.
//!
//! The frontend and execution engine are deliberately separate dependencies:
//! building this crate never builds or links a WebAssembly execution engine.

pub mod abi;
pub mod binary;
pub mod ir;
pub mod lower;
mod runtime;

use gleam_core::ast::TypedModule;
use std::sync::Arc;

pub use abi::Abi;

#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Qualified Gleam function name, for example `app.main`.
    pub entry: String,
    pub exports: Vec<String>,
    pub library: bool,
    pub tests: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Artifact {
    pub wasm: Vec<u8>,
    pub abi: Abi,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot lower Gleam program: {0}")]
    Lowering(String),
    #[error("cannot generate Wasm: {0}")]
    Codegen(String),
    #[error("invalid host interface: {0}")]
    Abi(String),
    #[error("cannot encode Wasm: {0}")]
    Encoding(String),
    #[error("generated Wasm failed validation: {0}")]
    Validation(String),
}

pub fn compile(modules: &[Arc<TypedModule>], options: &Options) -> Result<Artifact, Error> {
    let mut roots = options.exports.clone();
    roots.extend(options.tests.iter().cloned());
    if !options.library {
        roots.push(options.entry.clone());
    }
    roots.sort();
    roots.dedup();
    let program = lower::lower(modules, &roots)?;
    binary::compile(&program, options)
}

/// Compile while reusing serialized module IR from an explicitly versioned cache.
pub fn compile_cached(
    modules: &[Arc<TypedModule>],
    options: &Options,
    cache_dir: &std::path::Path,
    compiler_fingerprint: &str,
) -> Result<(Artifact, lower::CacheStats), Error> {
    let mut roots = options.exports.clone();
    roots.extend(options.tests.iter().cloned());
    if !options.library {
        roots.push(options.entry.clone());
    }
    roots.sort();
    roots.dedup();
    let (program, stats) = lower::lower_cached(modules, &roots, cache_dir, compiler_fingerprint)?;
    Ok((binary::compile(&program, options)?, stats))
}
