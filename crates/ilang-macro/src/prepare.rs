//! Prepares a macro module's parsed import closure for compiler execution,
//! and the parts of `prepare` and `project` that read or write the tree.
use ilang_check::Parsed;
use ilang_package::{Diagnostic, PackageGraph};
use ilang_syntax::ast::{
    Attribute, Binding, Expr, ExprKind, Item, ItemKind, Module, Pattern, PatternKind,
};
use ilang_syntax::lisp::{
    expansion::{self, MacroDefinition},
    quote_lower,
    syntax::Syntax,
};

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
        errors.extend(
            compiler_module(
                &mut module.ast,
                module.source.0,
                &definitions,
                &mut templates,
            )
            .into_iter()
            .map(|d| Diagnostic::error(d.code.as_str(), Some(module.source), d.span, d.message)),
        );
        for definition in definitions {
            let span = module.ast.items[definition.item].span;
            let variants = if definition.variants.is_empty() {
                vec![ilang_syntax::lisp::expansion::MacroVariant {
                    target: definition.name.clone(),
                    parameters: definition.parameters,
                    labels: definition.labels,
                }]
            } else {
                definition.variants
            };
            for variant in variants {
                if variant.target != definition.name
                    && !module
                        .ast
                        .exports
                        .iter()
                        .any(|e| e.name.name == variant.target)
                {
                    module.ast.exports.push(ilang_syntax::ast::Export {
                        name: ilang_syntax::ast::Ident::new(&variant.target, span.0),
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
                    ilang_check::resolve::missing_module_message(
                        graph,
                        module.package,
                        module.kind,
                        &path,
                    ),
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
			& (Error(syntax.SyntaxError(message, at))) {{ syntax._report_error(message, at) }}
	}} <- (syntax._diagnostics()){items} <- (syntax._definitions()) <- (
		Assert((failure : AssertionFailure) -> Never {{ panic(failure.message) }}),
	)
}}
"#,
        binding.module, binding.name
    );
    let (mut ast, diagnostics) = ilang_syntax::parse::parse_module(&source);
    // Normalized group names contain `%` and have no source spelling.
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
    ilang_syntax::walk::module(&mut ast, &mut Select(&binding.name, &binding.target));
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

fn is_macro(attribute: &Attribute) -> bool {
    attribute.module.is_none() && attribute.name.name == "macro"
}

/// Whether a module defines a macro (an item with `@macro`).
pub fn defines_macro(module: &Module) -> bool {
    module
        .items
        .iter()
        .any(|item| item.attributes.iter().any(is_macro))
}

/// `_ = 0` in the place of an item, with its span.
pub fn placeholder(item: &Item) -> ItemKind {
    let span = item.span.0;
    ItemKind::Bind(Binding {
        pattern: Pattern::new(PatternKind::Discard("_".into()), span),
        value: Expr::new(ExprKind::Int(0u32.into()), span),
        span: span.into(),
    })
}

/// Makes a module ready for the compiler program: the runtime signature of
/// a typed macro is not part of it, the definitions lose their phase
/// marker, and the quotes are lowered. The quotes that do not read are
/// returned.
pub fn compiler_module(
    module: &mut Module,
    module_id: u32,
    definitions: &[MacroDefinition],
    templates: &mut Vec<Syntax>,
) -> Vec<ilang_syntax::Diagnostic> {
    for definition in definitions {
        if let Some(index) = definition.signature {
            module.items[index].attributes.clear();
            module.items[index].kind = placeholder(&module.items[index]);
        }
    }
    let mut groups = Vec::new();
    for definition in definitions {
        let item = &mut module.items[definition.item];
        item.attributes.retain(|a| !is_macro(a));
        let ItemKind::Bind(binding) = &mut item.kind else {
            continue;
        };
        let ExprKind::Chain(functions) = &mut binding.value.kind else {
            continue;
        };
        if definition.variants.is_empty() {
            for function in functions {
                canonical(function);
            }
        } else {
            let mut values = Vec::new();
            for variant in &definition.variants {
                let mut fs: Vec<_> = functions
                    .iter()
                    .filter(|f| ilang_syntax::lisp::expansion::function_shape(f) == variant.labels)
                    .cloned()
                    .collect();
                for f in &mut fs {
                    canonical(f);
                }
                let first = fs[0].span.0;
                let cover = first.to(fs.last().unwrap().span.0);
                let at = ilang_syntax::span::Span::new(first.start, first.start + 1);
                groups.push(Item {
                    doc: Vec::new(),
                    attributes: Vec::new(),
                    span: item.span,
                    kind: ItemKind::Bind(Binding {
                        pattern: Pattern::new(PatternKind::Var(variant.target.clone()), at),
                        value: Expr::new(ExprKind::Chain(fs), cover),
                        span: binding.span,
                    }),
                });
                let mut shape = variant
                    .labels
                    .iter()
                    .filter(|x| x.is_none())
                    .count()
                    .to_string();
                for label in variant.labels.iter().flatten() {
                    shape.push(',');
                    shape.push_str(label);
                }
                values.push(ilang_syntax::ast::DItem::Label(
                    ilang_syntax::ast::Ident::new(shape, at),
                    Expr::new(
                        ExprKind::Name {
                            module: None,
                            name: ilang_syntax::ast::Ident::new(&variant.target, at),
                        },
                        at,
                    ),
                ));
            }
            binding.value.kind = ExprKind::Call {
                callee: Box::new(Expr::new(
                    ExprKind::Name {
                        module: None,
                        name: ilang_syntax::ast::Ident::new("%multi", binding.value.span.0),
                    },
                    binding.value.span.0,
                )),
                args: values,
            };
        }
    }
    module.items.extend(groups);
    quote_lower::lower_expression_quotes(module, module_id, templates)
}

fn canonical(function: &mut ilang_syntax::ast::Function) {
    function.params.sort_by(|a, b| {
        a.key
            .as_ref()
            .map(|k| &k.name)
            .cmp(&b.key.as_ref().map(|k| &k.name))
    });
    for param in &mut function.params {
        param.key = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_compiler_module_has_no_marker_and_no_signature() {
        let source = "(@ macro (: m (-> (, Syntax) Syntax)))\n(@ macro (= m (-> (, a) _ a)))";
        let (mut module, errors) = ilang_syntax::rows::from_lisp(source);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(defines_macro(&module));
        let definitions = ilang_syntax::lisp::expansion::definitions(&module, true).unwrap();
        let mut templates = Vec::new();
        let errors = compiler_module(&mut module, 0, &definitions, &mut templates);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(!defines_macro(&module));
        assert!(matches!(&module.items[0].kind, ItemKind::Bind(b)
            if matches!(b.pattern.kind, PatternKind::Discard(_))));
        assert!(templates.is_empty());
    }
}
