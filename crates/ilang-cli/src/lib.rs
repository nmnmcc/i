//! The pipeline behind the `i` command: load a package, check it, compile
//! it to Wasm, optimize it with Binaryen, and run it with the Wasmtime host.

use ilang_package::{Diagnostic, PackageGraph};
use std::path::Path;

pub use ilang_binaryen::Level;
pub use ilang_wasm::Target;

/// What to build.
#[derive(Clone, Copy, Debug)]
pub struct BuildOptions {
    /// Compile the tests of the package (§10.2).
    pub tests: bool,
    pub opt: Level,
    /// Do not use the network to get packages (§2.7); `ILANG_OFFLINE` in
    /// the environment also turns this on.
    pub offline: bool,
    /// The host to build for (B.7).
    pub target: ilang_wasm::Target,
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

/// Diagnostics followed by one old-version note per affected dependency.
pub fn diagnostic_lines(graph: &PackageGraph, diags: &[Diagnostic]) -> Vec<String> {
    let mut lines = graph.sources.render_all(diags);
    for (id, package) in graph.packages.iter().enumerate() {
        if Some(id) == graph.root || id == graph.library {
            continue;
        }
        if package.modules.iter().any(|m| {
            diags
                .iter()
                .any(|d| d.is_error() && d.source == Some(m.source))
                && ilang_syntax::old_syntax_marks(&graph.sources.get(m.source).text)
        }) {
            let tag = package
                .location
                .as_ref()
                .map(|l| format!("v{}", l.version))
                .unwrap_or_else(|| "path dependency".into());
            lines.push(format!("note: this version of `{}` (`{tag}`) is written in the syntax from before {} (A3); it cannot be used with this compiler; use a newer version of the package, or an older version of `i`", package.name, env!("CARGO_PKG_VERSION")));
        }
    }
    lines
}

fn program_error(graph: &PackageGraph, diags: &[Diagnostic]) -> BuildError {
    BuildError::Program {
        lines: diagnostic_lines(graph, diags),
        codes: diags
            .iter()
            .filter(|d| d.is_error())
            .map(|d| d.code)
            .collect(),
        errors: diags.iter().filter(|d| d.is_error()).count().max(1),
    }
}

/// Loads the package in `project` and the packages it uses, with its tests
/// only when `tests` is on, and offline when `offline` or the environment
/// says so.
pub fn load(project: &Path, tests: bool, offline: bool) -> (PackageGraph, Vec<Diagnostic>) {
    let env = ilang_package::LoadOptions::from_env();
    ilang_package::load_with(
        project,
        &ilang_package::LoadOptions {
            tests,
            offline: offline || env.offline,
            ..env
        },
    )
}

/// Loads, checks, and compiles the package in `project`.
pub fn build(project: &Path, options: BuildOptions) -> Result<Built, BuildError> {
    let (graph, mut diags) = load(project, options.tests, options.offline);
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
            target: options.target,
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

/// The host options of `i run` and `i test`: the standard host services
/// of `ilang_host::services::standard`, which print to the standard output.
pub fn standard_options() -> ilang_host::HostOptions {
    ilang_host::HostOptions::new(ilang_host::services::stdout())
}

/// Runs one test in a new instance (§10.2), with the standard host services.
pub fn run_test(program: &ilang_host::Program, name: &str) -> ilang_host::TestResult {
    run_test_with(program, name, standard_options())
}

/// Runs one test in a new instance (§10.2) with these host services, on one
/// `ilang` thread. An async test is waited for (B.5).
pub fn run_test_with(
    program: &ilang_host::Program,
    name: &str,
    options: ilang_host::HostOptions,
) -> ilang_host::TestResult {
    program.run_test_with(name, options)
}
