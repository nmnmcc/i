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
    /// Compiler provenance outside ABI v3, bound to the optimized artifact.
    pub binding_plan: ilang_host::bindings::BindingPlan,
    /// Trusted library package content identity, supplied by resolution.
    pub trusted_package: String,
    /// The warnings, rendered as `path:line:col: warning[CODE]: …`.
    pub warnings: Vec<String>,
    /// Whether this artifact contains test entries.
    pub tests: bool,
    /// Source-scoped quote templates installed in each test instance.
    pub templates: Vec<ilang_syntax::lisp::syntax::Syntax>,
}

impl Built {
    /// Preserve the in-process compiler trust path; approve the loader's library
    /// content only. Custom imports remain explicit embedding authority.
    pub fn host_program(&self) -> anyhow::Result<ilang_host::Program> {
        ilang_host::Program::from_bytes(&self.wasm)?.with_binding_plan(self.binding_plan.clone())
    }
    pub fn host_options(&self) -> ilang_host::HostOptions {
        let mut options = if self.tests {
            test_options_with_templates(self.templates.clone().into())
        } else {
            standard_options()
        };
        options
            .imports
            .grant_package(&self.binding_plan, &self.trusted_package);
        options
    }
}

/// The native runner (crates/ilang-runner), built for this `i` by build.rs.
pub const RUNNER: &[u8] = include_bytes!(env!("ILANG_RUNNER"));

/// The target triple of [`RUNNER`]: statically linked (musl) on Linux.
pub const RUNNER_TARGET: &str = env!("ILANG_RUNNER_TARGET");

impl Built {
    /// A native package (docs/design/native.md, N2): a copy of [`RUNNER`]
    /// with this module compiled for the runner's target triple.
    pub fn native_package(&self, name: &str) -> anyhow::Result<Vec<u8>> {
        ilang_host::native::package(
            RUNNER,
            RUNNER_TARGET,
            name,
            &self.wasm,
            &self.binding_plan,
            &self.trusted_package,
        )
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
    /// The requested collection handle is stale or does not exist.
    Selection(String),
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
            BuildError::Selection(m) => write!(f, "{m}"),
            BuildError::Internal(m) => write!(f, "internal compiler error: {m}"),
        }
    }
}

impl std::error::Error for BuildError {}

/// Diagnostics, migration guidance for parse errors, and old-version notes.
pub fn diagnostic_lines(graph: &PackageGraph, diags: &[Diagnostic]) -> Vec<String> {
    let mut lines = graph.sources.render_all(diags);
    // The syntax-three reader reports the error at the token that cannot
    // follow. Keep the migration guidance on that error path, without
    // reserving ordinary names such as `case` and `if` in the grammar.
    for module in graph.packages.iter().flat_map(|package| &package.modules) {
        if diags
            .iter()
            .any(|d| d.is_error() && d.source == Some(module.source) && d.code.starts_with("E01"))
            && let Some(hint) =
                ilang_syntax::old_syntax_hint(&graph.sources.get(module.source).text)
        {
            lines.push(format!(
                "note: {}: {hint}",
                graph.sources.get(module.source).display
            ));
        }
    }
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
    graph.test_run = options.tests;
    graph.entry = entry.map(str::to_owned);
    let Some(root) = graph.root else {
        return Err(program_error(&graph, &diags));
    };
    if diags.iter().any(Diagnostic::is_error) {
        return Err(program_error(&graph, &diags));
    }
    let checked = ilang_macro::project::check(&mut graph);
    diags.extend(checked.diags);
    let (Some(program), Some(typed)) = (&checked.program, &checked.types) else {
        return Err(program_error(&graph, &diags));
    };
    if diags.iter().any(Diagnostic::is_error) {
        return Err(program_error(&graph, &diags));
    }
    let warnings: Vec<Diagnostic> = diags.iter().filter(|d| !d.is_error()).cloned().collect();
    let (wasm, declarations) = match ilang_wasm::compile_with_bindings_numeric(
        &graph,
        program,
        typed,
        ilang_wasm::Options {
            tests: options.tests,
            target: options.target,
        }
        .numeric(if options.opt == Level::O0 {
            ilang_wasm::Numeric::Reference
        } else {
            ilang_wasm::Numeric::Scalars
        }),
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
        templates: checked.templates,
    })
}

pub fn build_tests(
    project: &Path,
    options: BuildOptions,
    collection: Option<&str>,
) -> Result<(Built, Vec<CollectionEntry>), BuildError> {
    let stage = ilang_syntax::trace::stage("load");
    let (mut graph, mut diags) = load(project, options.tests, options.offline);
    drop(stage);
    graph.test_run = options.tests;
    let Some(root) = graph.root else {
        return Err(program_error(&graph, &diags));
    };
    let revision = snapshot(&graph);
    let checked = ilang_macro::project::check_tests(&mut graph, collection);
    diags.extend(checked.diags);
    let (Some(program), Some(typed)) = (&checked.program, &checked.types) else {
        return Err(program_error(&graph, &diags));
    };
    if diags.iter().any(Diagnostic::is_error) {
        return Err(program_error(&graph, &diags));
    }
    let manifest = make_manifest(&graph, program, revision, &checked.collections);
    let selected: Vec<_> = manifest
        .entries
        .iter()
        .zip(&checked.collections)
        .enumerate()
        .filter(|(_, (entry, _))| Some(entry.collection.as_str()) == collection)
        .map(|(index, (entry, item))| (entry.clone(), item.target, format!("i.tool.{index}")))
        .collect();
    let exports: Vec<_> = selected
        .iter()
        .map(|(_, target, name)| (*target, name.clone()))
        .collect();
    let warnings: Vec<Diagnostic> = diags.iter().filter(|d| !d.is_error()).cloned().collect();
    let (wasm, declarations) = match ilang_wasm::compile_tests_with_bindings_numeric(
        &graph,
        program,
        typed,
        ilang_wasm::Options {
            tests: options.tests,
            target: options.target,
        }
        .numeric(if options.opt == Level::O0 {
            ilang_wasm::Numeric::Reference
        } else {
            ilang_wasm::Numeric::Scalars
        }),
        &exports,
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
    Ok((
        Built {
            name: graph.packages[root].name.clone(),
            wasm,
            binding_plan,
            trusted_package,
            warnings: graph.sources.render_all(&warnings),
            tests: options.tests,
            templates: checked.templates,
        },
        selected.into_iter().map(|(entry, _, _)| entry).collect(),
    ))
}

/// Persistable description of a checked collection. Handles identify one
/// contribution in this exact source snapshot; they do not expose compiler IDs.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CollectionManifest {
    pub revision: String,
    pub entries: Vec<CollectionEntry>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CollectionEntry {
    pub module: String,
    pub handle: String,
    pub collection: String,
    pub signature: String,
    pub data: Vec<(String, String)>,
    pub target: SourcePosition,
    pub at: SourcePosition,
    pub calls: Vec<ExpansionCall>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ExpansionCall {
    pub macro_name: String,
    pub position: SourcePosition,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct SourcePosition {
    pub path: String,
    pub start: u32,
    pub end: u32,
    pub line: u32,
    pub column: u32,
}

fn position(
    graph: &PackageGraph,
    source: ilang_package::SourceId,
    span: ilang_syntax::span::Span,
) -> SourcePosition {
    let file = graph.sources.get(source);
    let (line, column) = ilang_syntax::span::LineIndex::new(&file.text).line_col(span.start);
    SourcePosition {
        path: file.display.clone(),
        start: span.start,
        end: span.end,
        line,
        column,
    }
}

fn snapshot(graph: &PackageGraph) -> String {
    use ilang_package::lock::{FileKind, PackageFile, hash};
    let mut files = Vec::new();
    for (index, package) in graph.packages.iter().enumerate() {
        if let Some(source) = package.manifest_source {
            files.push(PackageFile {
                path: format!("{index}/i.toml"),
                kind: FileKind::Regular,
                bytes: graph.sources.get(source).text.as_bytes().to_vec(),
            });
        }
        for module in &package.modules {
            files.push(PackageFile {
                path: format!("{index}/{}", module.path),
                kind: FileKind::Regular,
                bytes: graph.sources.get(module.source).text.as_bytes().to_vec(),
            });
        }
    }
    hash(&files)
}

fn make_manifest(
    graph: &PackageGraph,
    program: &ilang_check::hir::Program,
    revision: String,
    entries: &[ilang_macro::collections::Entry],
) -> CollectionManifest {
    let entries = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let module = program.module(entry.target.module);
            let target = program.origin(entry.target.module, entry.target_span);
            let (target_source, target_span) = target
                .as_ref()
                .map_or((module.source, entry.target_span), |o| {
                    o.template.unwrap_or((o.source, o.span))
                });
            let calls = target.as_ref().map_or_else(Vec::new, |o| {
                o.expansion
                    .call_chain
                    .iter()
                    .map(|call| ExpansionCall {
                        macro_name: call.macro_name.clone(),
                        position: position(
                            graph,
                            ilang_package::SourceId(call.syntax.scope().module),
                            call.syntax.span(),
                        ),
                    })
                    .collect()
            });
            let collection_module = program.module(entry.collection.module);
            CollectionEntry {
                module: module.path.clone(),
                handle: format!("{revision}:{index}"),
                collection: format!(
                    "{}.{}",
                    collection_module.path,
                    program.def(entry.collection).name
                ),
                signature: entry.signature.clone(),
                data: entry.data.clone(),
                target: position(graph, target_source, target_span),
                at: position(
                    graph,
                    ilang_package::SourceId(entry.at.scope().module),
                    entry.at.span(),
                ),
                calls,
            }
        })
        .collect();
    CollectionManifest { revision, entries }
}

/// Check and expand a project, exposing a serializable collection manifest.
pub fn collections(
    project: &Path,
    tests: bool,
    offline: bool,
) -> Result<CollectionManifest, BuildError> {
    let (mut graph, mut diags) = load(project, tests, offline);
    let revision = snapshot(&graph);
    let checked = ilang_macro::project::check_for_tools(&mut graph);
    diags.extend(checked.diags);
    let (Some(program), Some(_)) = (&checked.program, &checked.types) else {
        return Err(program_error(&graph, &diags));
    };
    if diags.iter().any(Diagnostic::is_error) {
        return Err(program_error(&graph, &diags));
    }
    Ok(make_manifest(
        &graph,
        program,
        revision,
        &checked.collections,
    ))
}

/// A tool decides whether it requires Nil or accepts any return value.
#[derive(Clone, Copy)]
pub enum CollectionResult<'a> {
    /// The consumer accepts any result.
    Any,
    /// The consumer requires Nil, checked by the type solver.
    Nil,
    /// A consumer-defined contract over the inferred complete type scheme.
    /// Named type identities are interpreted in the supplied checked program.
    Checked(
        &'a dyn Fn(
            &ilang_check::hir::Program,
            &ilang_check::types::ShownType,
        ) -> Result<(), String>,
    ),
}

/// Collection metadata bound to a compiled artifact. This is stored in its
/// `ilang.collections` custom section as well as returned to the consumer.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CollectionArtifact {
    pub manifest: CollectionManifest,
    pub selected: String,
    pub export: String,
    pub is_async: bool,
}

impl CollectionArtifact {
    /// Read the tool manifest carried by a compiled Wasm artifact.
    pub fn from_wasm(wasm: &[u8]) -> anyhow::Result<Option<Self>> {
        let mut found = None;
        for payload in wasmparser::Parser::new(0).parse_all(wasm) {
            if let wasmparser::Payload::CustomSection(section) = payload?
                && section.name() == "ilang.collections"
            {
                anyhow::ensure!(found.is_none(), "duplicate ilang.collections sections");
                found = Some(serde_json::from_slice(section.data())?);
            }
        }
        Ok(found)
    }
}

/// A selected collection target and the binary that executes only that root.
pub struct BuiltCollection {
    pub built: Built,
    pub manifest: CollectionManifest,
    pub selected: CollectionEntry,
    pub export: String,
    pub is_async: bool,
}

impl BuiltCollection {
    /// Run with one fresh instance and the selected entry's provider plan.
    pub fn run_with(
        &self,
        options: ilang_host::HostOptions,
    ) -> anyhow::Result<ilang_host::Outcome> {
        let program = self.built.host_program()?;
        ilang_host::block_on(async {
            let mut instance = program.instantiate(options)?;
            if self.is_async {
                instance.run_async(&self.export).await
            } else {
                instance.run(&self.export)
            }
        })
    }
}

/// Select by opaque handle from a checked manifest. The snapshot is checked
/// again, so a stale handle cannot silently select a different contribution.
pub fn build_collection(
    project: &Path,
    options: BuildOptions,
    handle: &str,
    result: CollectionResult<'_>,
) -> Result<BuiltCollection, BuildError> {
    let (mut graph, mut diags) = load(project, options.tests, options.offline);
    graph.test_run = options.tests;
    let revision = snapshot(&graph);
    let root = graph.root.ok_or_else(|| program_error(&graph, &diags))?;
    let mut checked = ilang_macro::project::check_for_tools(&mut graph);
    diags.extend(checked.diags);
    let (Some(program), Some(_)) = (&mut checked.program, &checked.types) else {
        return Err(program_error(&graph, &diags));
    };
    if diags.iter().any(Diagnostic::is_error) {
        return Err(program_error(&graph, &diags));
    }
    let manifest = make_manifest(&graph, program, revision, &checked.collections);
    let index = manifest
        .entries
        .iter()
        .position(|e| e.handle == handle)
        .ok_or_else(|| {
            BuildError::Selection(
                "collection handle does not belong to this checked source snapshot".into(),
            )
        })?;
    let selected = manifest.entries[index].clone();
    let target = checked.collections[index].target;
    let (typed, more) = ilang_check::check_selected(
        &graph,
        program,
        target,
        matches!(result, CollectionResult::Nil),
    );
    diags.extend(more);
    if diags.iter().any(Diagnostic::is_error) {
        return Err(program_error(&graph, &diags));
    }
    if let CollectionResult::Checked(contract) = result {
        let scheme = typed
            .scheme_types
            .get(&target)
            .expect("checked contribution has a type scheme");
        contract(program, scheme).map_err(|message| {
            BuildError::Selection(format!(
                "{}:{}:{}: {message}",
                selected.at.path, selected.at.line, selected.at.column
            ))
        })?;
    }
    let export = format!("i.tool.{index}");
    let (wasm, declarations, is_async) = match ilang_wasm::compile_selected_with_bindings_numeric(
        &graph,
        program,
        &typed,
        ilang_wasm::Options {
            tests: options.tests,
            target: options.target,
        }
        .numeric(if options.opt == Level::O0 {
            ilang_wasm::Numeric::Reference
        } else {
            ilang_wasm::Numeric::Scalars
        }),
        target,
        &export,
    ) {
        Ok(value) => value,
        Err(ilang_wasm::Error::Diagnostics(more)) => {
            diags.extend(more);
            return Err(program_error(&graph, &diags));
        }
        Err(ilang_wasm::Error::Internal(message)) => return Err(BuildError::Internal(message)),
    };
    let mut wasm = optimize(&wasm, options.opt)?;
    let artifact = CollectionArtifact {
        manifest: manifest.clone(),
        selected: handle.to_owned(),
        export: export.clone(),
        is_async,
    };
    let metadata =
        serde_json::to_vec(&artifact).map_err(|e| BuildError::Internal(e.to_string()))?;
    ilang_wasm::emit::add_custom_section(&mut wasm, "ilang.collections", &metadata);
    let declarations = declarations
        .into_iter()
        .map(|d| ilang_host::bindings::BindingDeclaration::from_json(&d.to_string()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| BuildError::Internal(e.to_string()))?;
    let binding_plan = ilang_host::bindings::BindingPlan::new(&wasm, declarations);
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
    let built = Built {
        name: graph.packages[root].name.clone(),
        wasm,
        binding_plan,
        trusted_package: hash(&files),
        warnings: graph.sources.render_all(
            &diags
                .iter()
                .filter(|d| !d.is_error())
                .cloned()
                .collect::<Vec<_>>(),
        ),
        tests: options.tests,
        templates: checked.templates,
    };
    Ok(BuiltCollection {
        built,
        manifest,
        selected,
        export,
        is_async,
    })
}

/// Optimizes a module with Binaryen, and checks it again.
pub fn optimize(wasm: &[u8], level: Level) -> Result<Vec<u8>, BuildError> {
    if level == Level::O0 {
        return Ok(wasm.to_vec());
    }
    let _stage = ilang_syntax::trace::stage("Binaryen");
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
    test_options_with_templates(std::sync::Arc::from([]))
}

/// The test host for an artifact with source-located quote templates.
pub fn test_options_with_templates(
    templates: std::sync::Arc<[ilang_syntax::lisp::syntax::Syntax]>,
) -> ilang_host::HostOptions {
    let mut options = standard_options();
    ilang_host::syntax::install_with_templates(&mut options.imports, templates)
        .expect("syntax service namespace is unique");
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
                target: Target::Native,
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
                target: Target::Native,
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
            target: Target::Native,
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
