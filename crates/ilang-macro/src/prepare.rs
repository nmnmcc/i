//! Prepares a macro module's parsed import closure for compiler execution.
use ilang_check::Parsed;
use ilang_package::{Diagnostic, PackageGraph};
use ilang_syntax::lisp::{expansion3 as expansion, syntax::Syntax};

/// One source macro and its written argument names.
#[derive(Clone, Debug)]
pub struct MacroBinding {
    pub module: String,
    pub name: String,
    pub parameters: Vec<String>,
    pub typed: bool,
}

/// Macro bindings and source-located template nodes for one compiler program.
pub struct Prepared {
    pub bindings: Vec<MacroBinding>,
    pub templates: Vec<Syntax>,
}

/// Validates macro declarations, removes their phase markers for the compiler
/// program, and lowers quotes. `parsed` is the exact import closure selected by
/// the caller; source identities remain stable when different closures are selected.
pub fn prepare(graph: &PackageGraph, parsed: &mut [Parsed]) -> Result<Prepared, Vec<Diagnostic>> {
    let mut found = Vec::new();
    let mut errors = Vec::new();
    // Validate the whole closure before mutating it, so an invalid declaration
    // never leaves a partially transformed module in the caller's state.
    for module in parsed.iter() {
        let lisp = graph.sources.get(module.source).display.ends_with(".is");
        match expansion::definitions(&module.ast, lisp) {
            Ok(bindings) => found.push(bindings),
            Err(problems) => {
                errors.extend(problems.into_iter().map(|e| {
                    Diagnostic::error(
                        ilang_check::code::ATTRIBUTE_PLACE,
                        Some(module.source),
                        e.span,
                        e.message,
                    )
                }));
                found.push(Vec::new());
            }
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut bindings = Vec::new();
    let mut templates = Vec::new();
    for (module, definitions) in parsed.iter_mut().zip(found) {
        errors.extend(
            crate::prepare3::compiler_module(
                &mut module.ast,
                module.source.0,
                &definitions,
                &mut templates,
            )
            .into_iter()
            .map(|d| Diagnostic::error(d.code.as_str(), Some(module.source), d.span, d.message)),
        );
        for definition in definitions {
            bindings.push(MacroBinding {
                module: module.path.clone(),
                name: definition.name,
                parameters: definition.parameters,
                typed: definition.signature.is_some(),
            });
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Prepared {
        bindings,
        templates,
    })
}

/// Selects exactly a macro module and the modules it imports, together with the
/// implicit prelude and compiler syntax module. Preserve the original order so
/// source and module positions are deterministic.
pub fn import_closure(
    graph: &PackageGraph,
    parsed: &[Parsed],
    source: ilang_package::SourceId,
) -> Result<Vec<Parsed>, Vec<Diagnostic>> {
    import_indices(graph, parsed, source).map(|indices| {
        indices
            .into_iter()
            .map(|index| parsed[index].clone())
            .collect()
    })
}

pub(crate) fn import_indices(
    graph: &PackageGraph,
    parsed: &[Parsed],
    source: ilang_package::SourceId,
) -> Result<Vec<usize>, Vec<Diagnostic>> {
    use std::collections::{BTreeSet, VecDeque};
    let start = parsed
        .iter()
        .position(|m| m.source == source)
        .expect("macro source belongs to parsed graph");
    let prelude = parsed
        .iter()
        .position(|m| m.kind == ilang_package::ModuleKind::Prelude)
        .expect("loaded graph includes the prelude");
    let syntax = parsed
        .iter()
        .position(|m| m.package == graph.library && m.path == "i/syntax")
        .expect("loaded graph includes compiler syntax module");
    let mut queue = VecDeque::from([start, prelude, syntax]);
    // The runtime calls these library modules, so every program has them.
    queue.extend(ilang_package::library::RUNTIME.iter().map(|&path| {
        parsed
            .iter()
            .position(|m| m.package == graph.library && m.path == path)
            .unwrap_or_else(|| panic!("loaded graph includes runtime module {path}"))
    }));
    let mut selected = BTreeSet::new();
    let mut errors = Vec::new();
    while let Some(index) = queue.pop_front() {
        if !selected.insert(index) {
            continue;
        }
        let module = &parsed[index];
        let package = &graph.packages[module.package];
        for import in &module.ast.imports {
            let first = import
                .path
                .first()
                .expect("parser guarantees a nonempty import path");
            let id = if first.name == "i" {
                Some(graph.library)
            } else if first.name == package.name {
                Some(module.package)
            } else {
                package
                    .dependencies
                    .iter()
                    .find(|(name, _)| *name == first.name)
                    .and_then(|(_, id)| *id)
                    .or_else(|| {
                        (module.kind == ilang_package::ModuleKind::Test)
                            .then(|| {
                                package
                                    .dev_dependencies
                                    .iter()
                                    .find(|(name, _)| *name == first.name)
                                    .and_then(|(_, id)| *id)
                            })
                            .flatten()
                    })
            };
            let Some(id) = id else {
                errors.push(Diagnostic::error(
                    ilang_check::code::UNKNOWN_PACKAGE,
                    Some(module.source),
                    first.span.0,
                    format!("unknown package `{}`", first.name),
                ));
                continue;
            };
            let tail = import
                .path
                .iter()
                .skip(1)
                .map(|part| part.name.as_str())
                .collect::<Vec<_>>()
                .join("/");
            let path = format!("{}/{tail}", graph.packages[id].name);
            let Some(target) = parsed.iter().position(|p| {
                p.package == id && p.path == path && p.kind != ilang_package::ModuleKind::Prelude
            }) else {
                errors.push(Diagnostic::error(
                    ilang_check::code::NO_MODULE,
                    Some(module.source),
                    import.span.0,
                    format!("there is no module `{path}`"),
                ));
                continue;
            };
            if parsed[target].kind == ilang_package::ModuleKind::Test
                && module.kind != ilang_package::ModuleKind::Test
            {
                errors.push(Diagnostic::error(
                    ilang_check::code::SRC_IMPORTS_TEST,
                    Some(module.source),
                    import.span.0,
                    "a source module cannot import a test module",
                ));
                continue;
            }
            queue.push_back(target);
        }
    }
    if errors.is_empty() {
        Ok(selected.into_iter().collect())
    } else {
        Err(errors)
    }
}

/// The position in which a macro is being expanded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MacroPosition {
    Item,
    Expression,
    Type,
}

/// Adds a compiler-owned, host-compatible entry for a macro. The wrapper
/// supplies compiler services, translates SyntaxError through the compiler
/// context, and keeps all user code in its original module.
pub fn add_wrapper(
    graph: &mut PackageGraph,
    parsed: &mut Vec<Parsed>,
    binding: &MacroBinding,
    position: MacroPosition,
) -> (String, String) {
    let definition = parsed
        .iter()
        .find(|module| module.path == binding.module)
        .expect("macro definition belongs to its import closure");
    let package = definition.package;
    let package_name = &graph.packages[package].name;
    let mut suffix = 0usize;
    let path = loop {
        let candidate = format!("{package_name}/_compiler_macro_{suffix}");
        if !parsed
            .iter()
            .any(|module| module.package == package && module.path == candidate)
        {
            break candidate;
        }
        suffix += 1;
    };
    let args = (0..binding.parameters.len())
        .map(|n| format!("arg{n}"))
        .collect::<Vec<_>>();
    let parameters = args
        .iter()
        .map(|name| format!("{name}: syntax.Syntax"))
        .collect::<Vec<_>>()
        .join(", ");
    let arguments = args.join(", ");
    let items = if binding.typed {
        " <- (syntax._types())"
    } else if position == MacroPosition::Item {
        " <- (syntax._items())"
    } else {
        ""
    };
    let source = format!(
        r#"(
	syntax = i/syntax
	fail = i/fail
	target = {}
)
(
	_entry
)
_entry = ({parameters}) -> syntax.Syntax {{
	{{
		body = () -> syntax.Syntax {{ target.{}({arguments}) }}
		fail.catch(body) |>
			(Ok(value)) {{ value }}
			| (Error(syntax.SyntaxError(message, at))) {{ syntax._report_error(message, at) }}
	}} <- (syntax._diagnostics()){items} <- (syntax._definitions()) <- (
		Assert((failure: AssertionFailure) -> Never {{ panic(failure.message) }}),
	)
}}
"#,
        binding.module, binding.name
    );
    let (ast, diagnostics) = ilang_syntax::syn3::parse_module(&source);
    assert!(
        diagnostics.is_empty(),
        "compiler macro wrapper must parse: {diagnostics:?}"
    );
    let source_id = graph.sources.add(format!("<compiler:{path}>"), source);
    graph.packages[package]
        .modules
        .push(ilang_package::ModuleSource {
            path: path.clone(),
            source: source_id,
            kind: ilang_package::ModuleKind::Src,
        });
    parsed.push(Parsed {
        package,
        path: path.clone(),
        source: source_id,
        kind: ilang_package::ModuleKind::Src,
        ast,
        lowering: Default::default(),
        scoped: None,
        compiler_generated: true,
        typed_macros: None,
    });
    (path, "_entry".into())
}
