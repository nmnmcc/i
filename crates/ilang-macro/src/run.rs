//! Execution of compiler-owned syntax exports in isolated Wasm instances.
//!
//! An invocation receives only the compiler syntax imports and grants to the
//! resolved library package. A fresh instance is created for each invocation,
//! so mutable guest state cannot leak from one expansion into another.

use anyhow::{Context, Result, bail};
use ilang_syntax::lisp::syntax::Syntax;

/// Compiled compiler program and the grants bound to its exact Wasm bytes.
pub struct MacroProgram {
    wasm: Vec<u8>,
    binding_plan: ilang_host::bindings::BindingPlan,
    trusted_package: String,
    metered: bool,
    program: std::sync::OnceLock<ilang_host::Program>,
}

impl MacroProgram {
    pub fn new(
        wasm: Vec<u8>,
        binding_plan: ilang_host::bindings::BindingPlan,
        trusted_package: String,
    ) -> Self {
        Self {
            wasm,
            binding_plan,
            trusted_package,
            metered: false,
            program: std::sync::OnceLock::new(),
        }
    }
    fn host_program(&self) -> Result<&ilang_host::Program> {
        if let Some(program) = self.program.get() {
            return Ok(program);
        }
        let program = ilang_host::Program::from_bytes(&self.wasm)?
            .with_binding_plan(self.binding_plan.clone())?;
        Ok(self.program.get_or_init(|| program))
    }
}

#[derive(Debug)]
pub enum CompileError {
    Program(Vec<ilang_package::Diagnostic>),
    Internal(String),
}
impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Program(diags) => write!(f, "{} compiler program diagnostics", diags.len()),
            Self::Internal(message) => write!(f, "{message}"),
        }
    }
}
impl std::error::Error for CompileError {}

/// Compile compiler-generated Syntax wrappers from an already expanded import
/// closure. This uses the ordinary checker and emitter, but checks the selected
/// wrappers under compiler capabilities rather than ordinary entry authority.
pub fn compile(
    graph: &ilang_package::PackageGraph,
    parsed: &[ilang_check::Parsed],
    entries: &[(&str, &str)],
) -> Result<MacroProgram, CompileError> {
    let mut graph = graph.clone();
    let first = entries
        .first()
        .expect("a compiler program must have an entry");
    let root = parsed
        .iter()
        .find(|m| m.path == first.0)
        .expect("compiler entry module must belong to the selected closure")
        .package;
    assert!(
        entries
            .iter()
            .all(|(path, _)| parsed.iter().any(|m| m.path == *path && m.package == root)),
        "compiler program entries must belong to one package"
    );
    graph.root = Some(root);
    // User manifest entries are not roots of the compiler program.
    graph.packages[root].wasm_exports.clear();
    graph.packages[root].entries.clear();
    graph.entry = None;
    // Compiler wrappers install the sandbox capabilities lexically. Runtime
    // target plans belong to the user program, not this separate compiler
    // program and its deliberately restricted import closure (§8).
    for package in &mut graph.packages {
        package.provider_plan = Default::default();
    }
    let checked = ilang_check::check_compiler_exports(&graph, parsed, entries);
    let mut diags = checked.diags;
    let (Some(program), Some(typed)) = (&checked.program, &checked.types) else {
        return Err(CompileError::Program(diags));
    };
    if diags.iter().any(ilang_package::Diagnostic::is_error) {
        return Err(CompileError::Program(diags));
    }
    let (wasm, declarations) = match ilang_wasm::compile_macro(
        &graph,
        program,
        typed,
        ilang_wasm::Options {
            tests: false,
            target: ilang_wasm::Target::Host,
        },
    ) {
        Ok(result) => result,
        Err(ilang_wasm::Error::Diagnostics(more)) => {
            diags.extend(more);
            return Err(CompileError::Program(diags));
        }
        Err(ilang_wasm::Error::Internal(message)) => {
            return Err(CompileError::Internal(message));
        }
    };
    let declarations = declarations
        .into_iter()
        .map(|d| ilang_host::bindings::BindingDeclaration::from_json(&d.to_string()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| CompileError::Internal(e.to_string()))?;
    let binding_plan = ilang_host::bindings::BindingPlan::new(&wasm, declarations);
    use ilang_package::lock::{FileKind, PackageFile, hash};
    let files = graph.packages[graph.library]
        .modules
        .iter()
        .map(|m| PackageFile {
            path: m.path.clone(),
            kind: FileKind::Regular,
            bytes: graph.sources.get(m.source).text.as_bytes().to_vec(),
        })
        .collect::<Vec<_>>();
    let mut result = MacroProgram::new(wasm, binding_plan, hash(&files));
    result.metered = true;
    Ok(result)
}

/// Invoke a compiler-generated export whose ABI takes and returns Syntax.
///
/// The compiler must check the export's signature and its reachable external
/// declarations before constructing `built`; the host also validates the ABI
/// and the opaque values on every call. The `MacroProgram` value binds the grant plan
/// to the exact compiled artifact.
pub fn invoke(built: &MacroProgram, export: &str, arguments: &[Syntax]) -> Result<Syntax> {
    invoke_with_templates(built, export, arguments, std::sync::Arc::from([]))
}

/// Invoke with this compiled program's source-located quote templates. The
/// table belongs to this fresh instance and is never shared with another run.
pub fn invoke_with_templates(
    built: &MacroProgram,
    export: &str,
    arguments: &[Syntax],
    templates: std::sync::Arc<[Syntax]>,
) -> Result<Syntax> {
    let program = built.host_program().context("load macro program")?;
    let mut imports = ilang_host::HostImports::default();
    if built.metered {
        ilang_host::syntax::install_with_budget(
            &mut imports,
            templates,
            None,
            None,
            Default::default(),
        )
    } else {
        ilang_host::syntax::install_with_templates(&mut imports, templates)
    }
    .context("install syntax service")?;
    imports.grant_package(&built.binding_plan, &built.trusted_package);
    let mut instance = program
        .instantiate(ilang_host::HostOptions::new(imports))
        .context("instantiate macro program")?;
    let arguments: Vec<_> = arguments
        .iter()
        .cloned()
        .map(ilang_host::syntax::encode)
        .collect();
    match instance
        .call(export, &arguments)
        .context("call macro export")?
    {
        ilang_host::Outcome::Ok(value) => ilang_host::syntax::decode(&value)
            .cloned()
            .context("macro export returned a value other than Syntax"),
        ilang_host::Outcome::Err(error) => bail!("macro export returned an error: {error:?}"),
        ilang_host::Outcome::Stop(stop) => Err(stop.into()),
    }
}

/// The observable completion of one fresh compiler instance. Definition
/// deferral, compiler failures, and a macro's SyntaxError retain their Syntax
/// positions and cannot be caught by the guest.
#[derive(Debug)]
pub enum RunOutcome {
    Returned(Syntax),
    Aborted(ilang_host::syntax::DefinitionAbort),
    Stopped(ilang_host::Stop),
}

/// Execute a compiler entry with per-run definition state and templates.
pub fn invoke_with_context(
    built: &MacroProgram,
    export: &str,
    arguments: &[Syntax],
    templates: std::sync::Arc<[Syntax]>,
    context: std::sync::Arc<ilang_host::syntax::DefinitionContext>,
) -> Result<RunOutcome> {
    invoke_with_items(built, export, arguments, templates, context, None)
}

/// Execute with an item journal for an item-position run. The driver commits
/// its requests and buffered notes only after the result has been validated.
pub fn invoke_with_items(
    built: &MacroProgram,
    export: &str,
    arguments: &[Syntax],
    templates: std::sync::Arc<[Syntax]>,
    context: std::sync::Arc<ilang_host::syntax::DefinitionContext>,
    items: Option<std::sync::Arc<ilang_host::syntax::ItemContext>>,
) -> Result<RunOutcome> {
    invoke_with_items_budget(
        built,
        export,
        arguments,
        templates,
        context,
        items,
        Default::default(),
    )
}

/// Execute with a budget shared with the compiler-side definition resolver.
pub fn invoke_with_items_budget(
    built: &MacroProgram,
    export: &str,
    arguments: &[Syntax],
    templates: std::sync::Arc<[Syntax]>,
    context: std::sync::Arc<ilang_host::syntax::DefinitionContext>,
    items: Option<std::sync::Arc<ilang_host::syntax::ItemContext>>,
    budget: ilang_host::macro_budget::MacroBudget,
) -> Result<RunOutcome> {
    let program = built.host_program().context("load macro program")?;
    let mut imports = ilang_host::HostImports::default();
    ilang_host::syntax::install_with_budget(
        &mut imports,
        templates,
        Some(context.clone()),
        items,
        budget.clone(),
    )
    .context("install compiler services")?;
    imports.grant_package(&built.binding_plan, &built.trusted_package);
    let result = (|| {
        let mut instance = program.instantiate(ilang_host::HostOptions::new(imports))?;
        let values = arguments
            .iter()
            .cloned()
            .map(ilang_host::syntax::encode)
            .collect::<Vec<_>>();
        instance.call(export, &values)
    })();
    if let Some(limit) = budget.exceeded() {
        return Err(ilang_host::macro_budget::Exceeded(limit).into());
    }
    if let Some(abort) = context.take_abort() {
        return Ok(RunOutcome::Aborted(abort));
    }
    match result.context("run compiler export")? {
        ilang_host::Outcome::Ok(value) => Ok(RunOutcome::Returned(
            ilang_host::syntax::decode(&value)
                .cloned()
                .context("compiler export returned a value other than Syntax")?,
        )),
        ilang_host::Outcome::Err(error) => bail!("compiler wrapper leaked an error: {error:?}"),
        ilang_host::Outcome::Stop(stop) => Ok(RunOutcome::Stopped(stop)),
    }
}
