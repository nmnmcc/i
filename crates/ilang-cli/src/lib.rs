//! The pipeline behind the `i` command: load a package, check it, compile
//! it to Wasm, optimize it with Binaryen, and run it with the Wasmtime host.

pub mod macro_prepare;
pub mod macro_run;

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
    /// Compiler provenance outside ABI v3, bound to the optimized artifact.
    pub binding_plan: ilang_host::bindings::BindingPlan,
    /// Trusted library package content identity, supplied by resolution.
    pub trusted_package: String,
    /// The warnings, rendered as `path:line:col: warning[CODE]: …`.
    pub warnings: Vec<String>,
    /// Whether this artifact contains test entries.
    pub tests: bool,
}

impl Built {
    /// Preserve the in-process compiler trust path; approve the loader's library
    /// content only. Custom imports remain explicit embedding authority.
    pub fn host_program(&self) -> anyhow::Result<ilang_host::Program> {
        ilang_host::Program::from_bytes(&self.wasm)?.with_binding_plan(self.binding_plan.clone())
    }
    pub fn host_options(&self) -> ilang_host::HostOptions {
        let mut options = if self.tests {
            test_options()
        } else {
            standard_options()
        };
        options
            .imports
            .grant_package(&self.binding_plan, &self.trusted_package);
        options
    }
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
    build_entry(project, options, None)
}

/// Build one named manifest entry, or the default source entry.
pub fn build_entry(
    project: &Path,
    options: BuildOptions,
    entry: Option<&str>,
) -> Result<Built, BuildError> {
    let (mut graph, mut diags) = load(project, options.tests, options.offline);
    graph.entry = entry.map(str::to_owned);
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
    let (wasm, declarations) = match ilang_wasm::compile_with_bindings(
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
    let declarations = declarations
        .into_iter()
        .map(|d| ilang_host::bindings::BindingDeclaration::from_json(&d.to_string()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| BuildError::Internal(error.to_string()))?;
    let binding_plan = ilang_host::bindings::BindingPlan::new(&wasm, declarations);
    // The library is selected by the package loader, never by a caller's name.
    use ilang_package::lock::{FileKind, PackageFile, hash};
    let files: Vec<_> = graph.packages[graph.library]
        .modules
        .iter()
        .map(|m| PackageFile {
            path: m.path.clone(),
            kind: FileKind::Regular,
            bytes: graph.sources.get(m.source).text.as_bytes().to_vec(),
        })
        .collect();
    let trusted_package = hash(&files);
    Ok(Built {
        name: graph.packages[root].name.clone(),
        wasm,
        binding_plan,
        trusted_package,
        warnings: graph.sources.render_all(&warnings),
        tests: options.tests,
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

/// The test host includes the compiler syntax service for ordinary tests of
/// macro helpers. Program entry hosts do not get this service.
pub fn test_options() -> ilang_host::HostOptions {
    let mut options = standard_options();
    ilang_host::syntax::install(&mut options.imports).expect("syntax service namespace is unique");
    options
}

/// Runs one test in a new instance (§10.2), with the standard host services.
pub fn run_test(program: &ilang_host::Program, name: &str) -> ilang_host::TestResult {
    run_test_with(program, name, test_options())
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

#[cfg(test)]
mod binding_tests {
    use super::*;

    // Native 3.1: two required declarations of one Wasm name must retain
    // separate authority. A public wrapper/re-export introduces no raw alias.
    #[test]
    fn compiler_plan_preserves_coalesced_external_declarations() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("src/main.i"),
            r#"
@external("plugin:raw/v1", "number")
first: (Float) -> Float
@external("plugin:raw/v1", "number")
second: (Float) -> Float
@main
main = () {
  first(1.0)
  second(2.0)
  Nil
}
"#,
        )
        .unwrap();
        let built = build(
            dir.path(),
            BuildOptions {
                tests: false,
                opt: Level::O0,
                offline: true,
                target: Target::Ilang,
            },
        )
        .unwrap();
        let plan = &built.binding_plan;
        assert_eq!(plan.declarations.len(), 2);
        assert_ne!(
            plan.declarations[0].declaration,
            plan.declarations[1].declaration
        );
        assert_eq!(
            plan.artifact_digest,
            ilang_host::bindings::artifact_digest(&built.wasm)
        );
        let program = built.host_program().unwrap();
        assert_eq!(program.abi().externals["plugin:raw/v1"].len(), 1);
        let mut options = built.host_options();
        options
            .imports
            .context_external("plugin:raw/v1", "number", |_, _| {
                Ok(ilang_host::Value::Float(0.0))
            })
            .unwrap();
        options.imports.grant(plan, &plan.declarations[0]);
        assert!(
            program
                .instantiate(options.clone())
                .err()
                .unwrap()
                .to_string()
                .contains("ungranted host import")
        );
        options.imports.grant(plan, &plan.declarations[1]);
        assert!(matches!(
            program.run_entry(options).unwrap(),
            ilang_host::Outcome::Ok(_)
        ));
        // Optimizing changes the final bytes, so it gets a fresh artifact digest.
        let optimized = build(
            dir.path(),
            BuildOptions {
                tests: false,
                opt: Level::O2,
                offline: true,
                target: Target::Ilang,
            },
        )
        .unwrap();
        assert_eq!(optimized.binding_plan.declarations, plan.declarations);
        assert_eq!(
            optimized.binding_plan.artifact_digest,
            ilang_host::bindings::artifact_digest(&optimized.wasm)
        );
    }
}

#[cfg(test)]
#[test]
fn cli_grants_the_resolved_library_content_to_restricted_default_bindings() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello");
    let built = build(
        &project,
        BuildOptions {
            tests: false,
            opt: Level::O0,
            offline: true,
            target: Target::Ilang,
        },
    )
    .unwrap();
    let declaration = built
        .binding_plan
        .declarations
        .iter()
        .find(|d| d.namespace == "i:io/v1" && d.name == "print")
        .unwrap();
    assert_eq!(declaration.package, built.trusted_package);
    let mut imports = ilang_host::HostImports::default();
    imports
        .context_service("i:io/v1", "print", |_, _| Ok(Ok(ilang_host::Value::Nil)))
        .unwrap();
    let program = built.host_program().unwrap();
    assert!(
        program
            .instantiate(ilang_host::HostOptions::new(imports.clone()))
            .err()
            .unwrap()
            .to_string()
            .starts_with("ungranted host import")
    );
    imports.grant_package(&built.binding_plan, &built.trusted_package);
    assert!(matches!(
        program
            .run_entry(ilang_host::HostOptions::new(imports))
            .unwrap(),
        ilang_host::Outcome::Ok(_)
    ));
}
