//! Checks every written macro, including ones that are not called.
use crate::{
    prepare::{self, MacroPosition},
    run::{self, CompileError},
};
use ilang_check::Parsed;
use ilang_package::{Diagnostic, PackageGraph, SourceId};
use ilang_syntax::lisp::expansion3 as expansion;

/// Checks all macro signatures and capabilities of one module under the full
/// compiler environment available in item position. Dependencies must have
/// completed expansion before this is called.
pub fn module(
    graph: &PackageGraph,
    parsed: &[Parsed],
    source: SourceId,
) -> Result<(), Vec<Diagnostic>> {
    let root = parsed
        .iter()
        .find(|m| m.source == source)
        .expect("macro module exists");
    let lisp = graph.sources.get(source).display.ends_with(".is");
    let definitions = expansion::definitions(&root.ast, lisp).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| {
                Diagnostic::error(
                    ilang_check::code::ATTRIBUTE_PLACE,
                    Some(source),
                    error.span,
                    error.message,
                )
            })
            .collect::<Vec<_>>()
    })?;
    if definitions.is_empty() {
        return Ok(());
    }
    let mut graph = graph.clone();
    let mut selected = prepare::import_closure(&graph, parsed, source)?;
    let prepared = prepare::prepare(&graph, &mut selected)?;
    let mut entries = Vec::new();
    let mut wrappers = Vec::new();
    for definition in &definitions {
        for binding in prepared
            .bindings
            .iter()
            .filter(|b| b.module == root.path && b.name == definition.name)
        {
            let (path, name) =
                prepare::add_wrapper(&mut graph, &mut selected, binding, MacroPosition::Item);
            let wrapper_source = selected.last().expect("wrapper was appended").source;
            wrappers.push((
                wrapper_source,
                definition.name.clone(),
                root.ast.items[definition.item].span.0,
            ));
            entries.push((path, name));
        }
    }
    let refs = entries
        .iter()
        .map(|(path, name)| (path.as_str(), name.as_str()))
        .collect::<Vec<_>>();
    match run::compile_owned(graph, &selected, &refs) {
        Ok(_) => Ok(()),
        Err(CompileError::Internal(message)) => Err(vec![Diagnostic::error(
            ilang_check::code::MACRO_RESERVED,
            Some(source),
            root.ast.items[definitions[0].item].span.0,
            format!("compiler error while compiling macros: {message}"),
        )]),
        Err(CompileError::Program(mut diagnostics)) => {
            for diagnostic in &mut diagnostics {
                if let Some((_, name, span)) = wrappers
                    .iter()
                    .find(|(id, _, _)| diagnostic.source == Some(*id))
                {
                    diagnostic.source = Some(source);
                    diagnostic.span = *span;
                    diagnostic.message = format!("macro `{name}`: {}", diagnostic.message);
                }
            }
            Err(diagnostics)
        }
    }
}
