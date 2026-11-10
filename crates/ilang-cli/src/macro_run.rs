//! Execution of compiler-owned syntax exports in isolated Wasm instances.
//!
//! An invocation receives only the compiler syntax imports and grants to the
//! resolved library package. A fresh instance is created for each invocation,
//! so mutable guest state cannot leak from one expansion into another.

use anyhow::{Context, Result, bail};
use ilang_syntax::lisp::syntax::Syntax;

use crate::Built;

/// Compile compiler-generated Syntax wrappers from an already expanded import
/// closure. This uses the ordinary checker and emitter, but checks the selected
/// wrappers under compiler capabilities rather than ordinary entry authority.
pub fn compile(
    graph: &ilang_package::PackageGraph,
    parsed: &[ilang_check::Parsed],
    entries: &[(&str, &str)],
) -> Result<Built, crate::BuildError> {
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
    let checked = ilang_check::check_compiler_exports(&graph, parsed, entries);
    let mut diags = checked.diags;
    let (Some(program), Some(typed)) = (&checked.program, &checked.types) else {
        return Err(crate::program_error(&graph, &diags));
    };
    if diags.iter().any(ilang_package::Diagnostic::is_error) {
        return Err(crate::program_error(&graph, &diags));
    }
    let (wasm, declarations) = match ilang_wasm::compile_with_bindings(
        &graph,
        program,
        typed,
        ilang_wasm::Options {
            tests: false,
            target: ilang_wasm::Target::Ilang,
        },
    ) {
        Ok(result) => result,
        Err(ilang_wasm::Error::Diagnostics(more)) => {
            diags.extend(more);
            return Err(crate::program_error(&graph, &diags));
        }
        Err(ilang_wasm::Error::Internal(message)) => {
            return Err(crate::BuildError::Internal(message));
        }
    };
    let declarations = declarations
        .into_iter()
        .map(|d| ilang_host::bindings::BindingDeclaration::from_json(&d.to_string()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| crate::BuildError::Internal(e.to_string()))?;
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
    Ok(Built {
        name: graph.packages[root].name.clone(),
        wasm,
        binding_plan,
        trusted_package: hash(&files),
        warnings: graph.sources.render_all(&diags),
        tests: false,
    })
}

/// Invoke a compiler-generated export whose ABI takes and returns Syntax.
///
/// The compiler must check the export's signature and its reachable external
/// declarations before constructing `built`; the host also validates the ABI
/// and the opaque values on every call. The `Built` value binds the grant plan
/// to the exact compiled artifact.
pub fn invoke(built: &Built, export: &str, arguments: &[Syntax]) -> Result<Syntax> {
    invoke_with_templates(built, export, arguments, std::sync::Arc::from([]))
}

/// Invoke with this compiled program's source-located quote templates. The
/// table belongs to this fresh instance and is never shared with another run.
pub fn invoke_with_templates(
    built: &Built,
    export: &str,
    arguments: &[Syntax],
    templates: std::sync::Arc<[Syntax]>,
) -> Result<Syntax> {
    let program = built.host_program().context("load macro program")?;
    let mut imports = ilang_host::HostImports::default();
    ilang_host::syntax::install_with_templates(&mut imports, templates)
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
