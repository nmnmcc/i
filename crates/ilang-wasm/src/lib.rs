//! WasmGC code generation for the i compiler.
//!
//! - `runtime`: hand-written WAT that every program links; `runtime/mod.rs`
//!   documents its interface.
//! - `ir`: the intermediate form, in A-normal form, and the calling
//!   convention.
//! - `lower`: from the checked program to the intermediate form.
//! - `control`: finds the calls that may suspend and splits their functions
//!   (§9.4).
//! - `emit`: from the intermediate form to a Wasm module with the binary
//!   interface of Appendix B.
//!
//! The code is simple and not optimized: `ilang-binaryen` optimizes it.

pub mod budget;
pub mod control;
pub mod emit;
pub mod ir;
pub mod lower;
pub mod runtime;

use ilang_check::hir;
use ilang_check::types::Typed;
use ilang_package::{Diagnostic, PackageGraph};

pub use ir::Target;
pub use lower::{Options, code};

/// Why a program could not be compiled.
#[derive(Debug)]
pub enum Error {
    /// Problems in the program, such as features that are not supported yet.
    Diagnostics(Vec<Diagnostic>),
    /// A mistake of the compiler: the module it made is not valid.
    Internal(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Diagnostics(d) => write!(f, "{} problems", d.len()),
            Error::Internal(m) => write!(f, "internal compiler error: {m}"),
        }
    }
}

impl std::error::Error for Error {}

/// Compiles a checked program with no errors to a Wasm module (Appendix B).
/// The module is checked with `wasmparser`.
pub fn compile(
    graph: &PackageGraph,
    program: &hir::Program,
    typed: &Typed,
    options: Options,
) -> Result<Vec<u8>, Error> {
    compile_with_bindings(graph, program, typed, options).map(|(wasm, _)| wasm)
}

/// The artifact and its complete resolved binding declarations, separately from
/// ABI v3. The caller binds the plan to the final bytes after optimization.
pub fn compile_with_bindings(
    graph: &PackageGraph,
    program: &hir::Program,
    typed: &Typed,
    options: Options,
) -> Result<(Vec<u8>, Vec<serde_json::Value>), Error> {
    let (ir, bindings) =
        lower::lower_with_bindings(graph, program, typed, options).map_err(Error::Diagnostics)?;
    let text = emit::wat(&ir);
    let mut wasm = wat::parse_str(&text).map_err(|e| Error::Internal(e.to_string()))?;
    emit::add_abi_section(&mut wasm, &ir.abi_json);
    add_provider_section(&mut wasm, &bindings);
    validate(&wasm).map_err(Error::Internal)?;
    Ok((wasm, bindings))
}

fn add_provider_section(wasm: &mut Vec<u8>, bindings: &[serde_json::Value]) {
    let registrations: Vec<_> = bindings.iter().filter(|d| d["kind"] == "service").map(|d| {
        let declaration = d["declaration"].as_str().expect("service declaration");
        serde_json::json!({
            "service": format!("{}.{}", d["module"].as_str().expect("source module"), declaration.split('.').next().expect("service name")),
            "namespace": d["namespace"], "name": d["name"],
        })
    }).collect();
    emit::add_json_section(
        wasm,
        "ilang.providers",
        &serde_json::to_string(&registrations).expect("host registration metadata"),
    );
}

/// Compile one selected collection target without adding it to the ordinary
/// entry, test, or host-export tables. The export handle is owned by the tool.
pub fn compile_selected_with_bindings(
    graph: &PackageGraph,
    program: &hir::Program,
    typed: &Typed,
    options: Options,
    target: hir::DefId,
    export: &str,
) -> Result<(Vec<u8>, Vec<serde_json::Value>, bool), Error> {
    let (ir, bindings, is_async) =
        lower::lower_selected_with_bindings(graph, program, typed, options, target, export)
            .map_err(Error::Diagnostics)?;
    let text = emit::wat(&ir);
    let mut wasm = wat::parse_str(&text).map_err(|e| Error::Internal(e.to_string()))?;
    emit::add_abi_section(&mut wasm, &ir.abi_json);
    add_provider_section(&mut wasm, &bindings);
    validate(&wasm).map_err(Error::Internal)?;
    Ok((wasm, bindings, is_async))
}

/// The WAT text of a checked program, for reading.
pub fn compile_wat(
    graph: &PackageGraph,
    program: &hir::Program,
    typed: &Typed,
    options: Options,
) -> Result<String, Error> {
    let ir = lower::lower(graph, program, typed, options).map_err(Error::Diagnostics)?;
    Ok(emit::wat(&ir))
}

/// Compile a macro program with deterministic logical resource accounting.
/// Its trusted host must install the `ilang:macro/budget/v1` hooks.
pub fn compile_macro(
    graph: &PackageGraph,
    program: &hir::Program,
    typed: &Typed,
    mut options: Options,
) -> Result<(Vec<u8>, Vec<serde_json::Value>), Error> {
    // Macro evaluation is always on the compiler host, independently of the
    // target of the program being compiled (meta §8).
    options.target = Target::Host;
    let (ir, bindings) =
        lower::lower_with_bindings(graph, program, typed, options).map_err(Error::Diagnostics)?;
    let (text, cost) = emit::wat_metered(&ir);
    let mut wasm = wat::parse_str(&text).map_err(|e| Error::Internal(e.to_string()))?;
    emit::add_abi_section(&mut wasm, &ir.abi_json);
    add_provider_section(&mut wasm, &bindings);
    let mut static_cost = [0u8; 16];
    static_cost[..8].copy_from_slice(&cost.work.to_le_bytes());
    static_cost[8..].copy_from_slice(&cost.allocation.to_le_bytes());
    emit::add_custom_section(&mut wasm, "ilang.macro.budget", &static_cost);
    validate(&wasm).map_err(Error::Internal)?;
    Ok((wasm, bindings))
}

/// The Wasm features of Appendix B.1, and the small ones every engine with
/// them has.
pub fn features() -> wasmparser::WasmFeatures {
    use wasmparser::WasmFeatures as F;
    F::MUTABLE_GLOBAL
        | F::SATURATING_FLOAT_TO_INT
        | F::SIGN_EXTENSION
        | F::REFERENCE_TYPES
        | F::MULTI_VALUE
        | F::BULK_MEMORY
        | F::TAIL_CALL
        | F::FLOATS
        | F::EXCEPTIONS
        | F::FUNCTION_REFERENCES
        | F::GC
        | F::GC_TYPES
}

/// Checks that a module is valid with the features of [`features`].
pub fn validate(wasm: &[u8]) -> Result<(), String> {
    wasmparser::Validator::new_with_features(features())
        .validate_all(wasm)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// The module without its `name` custom section. Binaryen does not read
/// every name subsection that the text assembler writes.
pub fn strip_names(wasm: &[u8]) -> Vec<u8> {
    fn leb(bytes: &[u8], at: &mut usize) -> Option<usize> {
        let mut n = 0usize;
        let mut shift = 0;
        loop {
            let b = *bytes.get(*at)?;
            *at += 1;
            n |= ((b & 0x7f) as usize) << shift;
            if b & 0x80 == 0 {
                return Some(n);
            }
            shift += 7;
        }
    }
    let mut out = wasm[..8.min(wasm.len())].to_vec();
    let mut at = 8;
    while at < wasm.len() {
        let start = at;
        let id = wasm[at];
        at += 1;
        let Some(size) = leb(wasm, &mut at) else {
            return wasm.to_vec();
        };
        let end = at + size;
        if end > wasm.len() {
            return wasm.to_vec();
        }
        let mut name_at = at;
        let is_name = id == 0
            && leb(wasm, &mut name_at)
                .is_some_and(|n| wasm.get(name_at..name_at + n) == Some(b"name".as_slice()));
        if !is_name {
            out.extend_from_slice(&wasm[start..end]);
        }
        at = end;
    }
    out
}
