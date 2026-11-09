//! The pipeline behind the `i` command: load a package, check it, compile
//! it to Wasm, optimize it with Binaryen, and run it with the Wasmtime host.

use ilang_package::{Diagnostic, PackageGraph};
use std::path::Path;

pub use ilang_binaryen::Level;

/// What to build.
#[derive(Clone, Copy, Debug)]
pub struct BuildOptions {
    /// Compile the tests of the package (§10.2).
    pub tests: bool,
    pub opt: Level,
}

/// A module that was built.
pub struct Built {
    /// The `name` of the package (§2.6).
    pub name: String,
    pub wasm: Vec<u8>,
    /// The warnings, rendered as `path:line:col: warning[CODE]: …`.
    pub warnings: Vec<String>,
}

/// Why a build failed.
#[derive(Debug)]
pub enum BuildError {
    /// Problems in the program, rendered; the errors are also counted.
    Program {
        lines: Vec<String>,
        codes: Vec<&'static str>,
        errors: usize,
    },
    /// A mistake of the compiler or of Binaryen.
    Internal(String),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::Program { lines, errors, .. } => {
                for l in lines {
                    writeln!(f, "{l}")?;
                }
                write!(f, "{errors} error{}", if *errors == 1 { "" } else { "s" })
            }
            BuildError::Internal(m) => write!(f, "internal compiler error: {m}"),
        }
    }
}

impl std::error::Error for BuildError {}

fn program_error(graph: &PackageGraph, diags: &[Diagnostic]) -> BuildError {
    BuildError::Program {
        lines: graph.sources.render_all(diags),
        codes: diags
            .iter()
            .filter(|d| d.is_error())
            .map(|d| d.code)
            .collect(),
        errors: diags.iter().filter(|d| d.is_error()).count().max(1),
    }
}

/// Loads, checks, and compiles the package in `project`.
pub fn build(project: &Path, options: BuildOptions) -> Result<Built, BuildError> {
    let (graph, mut diags) = ilang_package::load(project);
    let Some(root) = graph.root else {
        return Err(program_error(&graph, &diags));
    };
    let checked = ilang_check::check(&graph);
    diags.extend(checked.diags);
    let (Some(program), Some(typed)) = (&checked.program, &checked.types) else {
        return Err(program_error(&graph, &diags));
    };
    if diags.iter().any(Diagnostic::is_error) {
        return Err(program_error(&graph, &diags));
    }
    let warnings: Vec<Diagnostic> = diags.iter().filter(|d| !d.is_error()).cloned().collect();
    let wasm = match ilang_wasm::compile(
        &graph,
        program,
        typed,
        ilang_wasm::Options {
            tests: options.tests,
        },
    ) {
        Ok(w) => w,
        Err(ilang_wasm::Error::Diagnostics(more)) => {
            diags.extend(more);
            return Err(program_error(&graph, &diags));
        }
        Err(ilang_wasm::Error::Internal(m)) => return Err(BuildError::Internal(m)),
    };
    let wasm = optimize(&wasm, options.opt)?;
    Ok(Built {
        name: graph.packages[root].name.clone(),
        wasm,
        warnings: graph.sources.render_all(&warnings),
    })
}

/// Optimizes a module with Binaryen, and checks it again.
pub fn optimize(wasm: &[u8], level: Level) -> Result<Vec<u8>, BuildError> {
    if level == Level::O0 {
        return Ok(wasm.to_vec());
    }
    let out = ilang_binaryen::optimize(&ilang_wasm::strip_names(wasm), level)
        .map_err(|e| BuildError::Internal(format!("Binaryen: {e}")))?;
    ilang_wasm::validate(&out)
        .map_err(|e| BuildError::Internal(format!("the optimized module is not valid: {e}")))?;
    Ok(out)
}

/// The level of `--opt`.
pub fn level(n: u8) -> Option<Level> {
    match n {
        0 => Some(Level::O0),
        2 => Some(Level::O2),
        3 => Some(Level::O3),
        _ => None,
    }
}

/// Runs one test in a new instance (§10.2).
pub fn run_test(program: &ilang_host::Program, name: &str) -> ilang_host::TestResult {
    let mut instance = match program.instantiate(ilang_host::HostOptions::default()) {
        Ok(i) => i,
        Err(e) => {
            return match e.downcast::<ilang_host::Stop>() {
                Ok(stop) => ilang_host::TestResult::Stopped(stop),
                Err(e) => ilang_host::TestResult::Failed(e),
            };
        }
    };
    match instance.run(name) {
        Ok(ilang_host::Outcome::Ok(_)) => ilang_host::TestResult::Passed,
        Ok(ilang_host::Outcome::Stop(stop)) => ilang_host::TestResult::Stopped(stop),
        Ok(ilang_host::Outcome::Err(e)) => {
            ilang_host::TestResult::Failed(anyhow::anyhow!("the test gave the error {:?}", e.0))
        }
        Err(e) => ilang_host::TestResult::Failed(e),
    }
}
