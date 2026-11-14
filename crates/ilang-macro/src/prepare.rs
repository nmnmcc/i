//! Prepares a macro module's parsed import closure for compiler execution.
use ilang_check::Parsed;
use ilang_package::{Diagnostic, PackageGraph};
use ilang_syntax::lisp::{expansion, quote_lower, syntax::Syntax};

/// One source macro and its written argument names.
#[derive(Clone, Debug)]
pub struct MacroBinding {
    pub module: String,
    pub name: String,
    /// Compiler-local shape-group binding; ordinary macros target their name.
    pub target: String,
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
        for definition in &definitions {
            if let Some(index) = definition.signature {
                module.ast.items[index].attributes.clear();
                // The runtime signature is not part of the compiler program.
                module.ast.items[index].kind =
                    ilang_syntax::ast::ItemKind::Binding(ilang_syntax::ast::Binding {
                        pattern: ilang_syntax::ast::Pattern {
                            kind: ilang_syntax::ast::PatternKind::Discard("_".into()),
                            span: module.ast.items[index].span,
                        },
                        value: ilang_syntax::ast::Expr {
                            kind: ilang_syntax::ast::ExprKind::Int(0u32.into()),
                            span: module.ast.items[index].span,
                        },
                        type_params: Vec::new(),
                        span: module.ast.items[index].span,
                    });
            }
        }
        for definition in definitions {
            let item = &mut module.ast.items[definition.item];
            item.attributes
                .retain(|a| !(a.module.is_none() && a.name.name == "macro"));
            if let ilang_syntax::ast::ItemKind::Binding(binding) = &mut item.kind
                && let ilang_syntax::ast::ExprKind::Fn(function) = &mut binding.value.kind
            {
                for param in &mut function.params {
                    if let ilang_syntax::ast::Param::Pattern(pattern) = param
                        && let ilang_syntax::ast::PatternKind::Named {
                            name,
                            pattern: inner,
                        } = &pattern.kind
                        && name.name.starts_with("%macro_label:")
                    {
                        *pattern = inner.as_ref().clone();
                    }
                }
            }

            let span = item.span;
            let variants = if definition.variants.is_empty() {
                vec![expansion::MacroVariant {
                    target: definition.name.clone(),
                    parameters: definition.parameters,
                    labels: definition.labels,
                }]
            } else {
                definition.variants
            };
            for variant in variants {
                // Private group names are exposed only in this compiler-owned
                // closure so the fixed-shape wrapper can invoke them normally.
                if variant.target != definition.name
                    && !module
                        .ast
                        .exports
                        .iter()
                        .any(|e| e.name.name == variant.target)
                {
                    module.ast.exports.push(ilang_syntax::ast::ExportItem {
                        name: ilang_syntax::ast::Ident {
                            name: variant.target.clone(),
                            span,
                        },
                        constructors: false,
                        span,
                    });
                }
                bindings.push(MacroBinding {
                    module: module.path.clone(),
                    name: definition.name.clone(),
                    target: variant.target,
                    parameters: variant.parameters,
                    typed: definition.signature.is_some(),
                });
            }
        }
        for item in &mut module.ast.items {
            item.attributes
                .retain(|a| a.module.is_some() || a.name.name != "%macro_group");
        }
        quote_lower::lower_expression_quotes(&mut module.ast, module.source.0, &mut templates);
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
                    first.span,
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
                    import.span,
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
                    import.span,
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
		Assert((failure : AssertionFailure) -> Never {{ panic(failure.message) }}),
	)
}}
"#,
        binding.module, binding.name
    );
    let (mut ast, diagnostics) = ilang_syntax::parse_module(&source);
    // Transition group names contain `%` and have no source spelling.
    struct Select<'a>(&'a str, &'a str);
    impl ilang_syntax::walk::Visitor for Select<'_> {
        fn expr(&mut self, e: &mut ilang_syntax::ast::Expr, enter: bool) {
            if enter
                && let ilang_syntax::ast::ExprKind::Field { target, name } = &mut e.kind
                && matches!(&target.kind, ilang_syntax::ast::ExprKind::Name { module: None, name } if name.name == "target")
                && name.name == self.0
            {
                name.name = self.1.into();
            }
        }
    }
    use ilang_syntax::walk::Walk;
    ast.walk(&mut Select(&binding.name, &binding.target));
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
        scoped: None,
        compiler_generated: true,
        typed_macros: None,
    });
    (path, "_entry".into())
}
