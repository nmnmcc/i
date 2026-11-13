//! Expanded code points to written arguments or its call and template (§9).
mod common;

// The tree that `Parsed.ast` holds: `tree::new` once it is the tree of syntax
// three (docs/design/syn3-final-ast.md, step 2d).
use common::tree::old as tree;

use ilang_check::query::{Query, check_parsed_for_queries};
use ilang_syntax::lisp::{
    Datum, DatumKind,
    scoped::ScopedSource,
    syntax::{Scope, Syntax},
};
use ilang_syntax::span::Span;
use std::collections::BTreeSet;

fn syntax(name: &str, source: u32, span: Span, marks: &[u64]) -> Syntax {
    Syntax::from_datum(
        &Datum {
            kind: DatumKind::Name(name.into()),
            span,
            docs: vec![],
        },
        &Scope {
            module: source,
            marks: marks.iter().copied().collect::<BTreeSet<_>>(),
        },
    )
}

#[test]
fn template_error_uses_call_site_and_related_template_and_suppresses_warnings() {
    let main = "\nmain = () { fake(1) }\n";
    let template = "// missing\nhelper = 1\n";
    let (mut graph, _, _) =
        common::check_files(&[("src/main.i", main), ("src/template.i", template)]);
    let (mut parsed, errors) = ilang_check::parse(&graph);
    assert!(errors.is_empty(), "{errors:?}");
    let ts = parsed
        .iter()
        .find(|p| p.path == "app/template")
        .unwrap()
        .source;
    let module = parsed.iter_mut().find(|p| p.path == "app/main").unwrap();
    let ms = module.source;
    let last = tree::last_expression(&mut module.ast, 0);
    let call_span = tree::span(last);
    let call = syntax("fake", ms.0, call_span, &[]);
    let template_span = Span::new(3, 10);
    let generated = syntax("missing", ts.0, template_span, &[9]);
    let mut scoped = ScopedSource::new(main.into());
    let expanded = scoped.expression(&generated, &call).unwrap();
    let generated_span = tree::span(&expanded);
    scoped.inherit_call_chain(tree::span(&expanded), call_span, "fake");
    *last = expanded;
    graph.sources.set_text(ms, scoped.text().into());
    module.scoped = Some(scoped);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    let error = checked.diags.iter().find(|d| d.is_error()).unwrap();
    assert_eq!(error.source, Some(ms));
    assert_eq!(error.span, call_span);
    assert!(
        error.message.contains("in the expansion of `fake`"),
        "{error:?}"
    );
    assert_eq!(error.related.len(), 1);
    assert_eq!(
        (error.related[0].source, error.related[0].span),
        (ts, template_span)
    );
    assert!(
        graph
            .sources
            .render(error)
            .contains("expanded from this template")
    );
    let mut warnings = vec![ilang_package::Diagnostic::warning(
        "W0504",
        Some(ms),
        generated_span,
        "unused template binding",
    )];
    ilang_check::map_expansion_diagnostics(
        checked.program.as_ref().unwrap(),
        &graph.sources,
        &mut warnings,
    );
    assert!(warnings.is_empty());
}

#[test]
fn argument_location_and_query_use_written_source() {
    let main = "\nmain = () { x = 3\n fake(x) }\n";
    let template = "// template\nhelper = 1\n";
    let (mut graph, _, _) =
        common::check_files(&[("src/main.i", main), ("src/template.i", template)]);
    let (mut parsed, errors) = ilang_check::parse(&graph);
    assert!(errors.is_empty());
    let ts = parsed
        .iter()
        .find(|p| p.path == "app/template")
        .unwrap()
        .source;
    let module = parsed.iter_mut().find(|p| p.path == "app/main").unwrap();
    let ms = module.source;
    let last = tree::last_expression(&mut module.ast, 0);
    let call_span = tree::span(last);
    let x_offset = main.rfind('x').unwrap() as u32;
    let call = syntax("fake", ms.0, call_span, &[]);
    let argument = syntax("x", ms.0, Span::new(x_offset, x_offset + 1), &[]);
    let mut scoped = ScopedSource::new(main.into());
    let expanded = scoped.expression(&argument, &call).unwrap();
    let generated_span = tree::span(&expanded);
    scoped.inherit_call_chain(tree::span(&expanded), call_span, "fake");
    *last = expanded;
    graph.sources.set_text(ms, scoped.text().into());
    module.scoped = Some(scoped);
    let checked = check_parsed_for_queries(&graph, &parsed, vec![]);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        checked.diags
    );
    let program = checked.program.as_ref().unwrap();
    let mut errors = vec![ilang_package::Diagnostic::error(
        "E0501",
        Some(ms),
        generated_span,
        "argument error",
    )];
    ilang_check::map_expansion_diagnostics(program, &graph.sources, &mut errors);
    assert_eq!(
        (errors[0].source, errors[0].span),
        (Some(ms), Span::new(x_offset, x_offset + 1))
    );
    assert!(errors[0].related.is_empty());
    let module_id = common::module(program, "app/main");
    let query = Query::new(&graph, program, checked.types.as_ref());
    let hover = query.type_at(module_id, x_offset).unwrap();
    assert_eq!(hover.span, Span::new(x_offset, x_offset + 1));
    let definition = query.definition_at(module_id, x_offset).unwrap();
    let binder = main.find("x =").unwrap() as u32;
    assert_eq!(
        (definition.source, definition.span),
        (ms, Span::new(binder, binder + 1))
    );
    assert_ne!(definition.source, ts);
}

#[test]
fn nested_template_error_uses_outer_call_and_both_template_locations() {
    let main = "\nmain = () { first(1) }\n";
    let (mut graph, _, _) = common::check_files(&[
        ("src/main.i", main),
        ("src/first.i", "// second\nfirst_helper = 1\n"),
        ("src/second.i", "// missing\nsecond_helper = 1\n"),
    ]);
    let (mut parsed, errors) = ilang_check::parse(&graph);
    assert!(errors.is_empty());
    let first = parsed
        .iter()
        .find(|p| p.path == "app/first")
        .unwrap()
        .source;
    let second = parsed
        .iter()
        .find(|p| p.path == "app/second")
        .unwrap()
        .source;
    let module = parsed.iter_mut().find(|p| p.path == "app/main").unwrap();
    let ms = module.source;
    let last = tree::last_expression(&mut module.ast, 0);
    let call_span = tree::span(last);
    let call = syntax("first", ms.0, call_span, &[]);
    let mut scoped = ScopedSource::new(main.into());
    let inner = syntax("second", first.0, Span::new(3, 9), &[1]);
    let intermediate = scoped.expression(&inner, &call).unwrap();
    scoped.inherit_call_chain(tree::span(&intermediate), call_span, "first");
    let missing = syntax("missing", second.0, Span::new(3, 10), &[2]);
    let result = scoped.expression(&missing, &inner).unwrap();
    scoped.inherit_call_chain(tree::span(&result), tree::span(&intermediate), "second");
    *last = result;
    graph.sources.set_text(ms, scoped.text().into());
    module.scoped = Some(scoped);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    let error = checked.diags.iter().find(|d| d.is_error()).unwrap();
    assert_eq!((error.source, error.span), (Some(ms), call_span));
    assert!(error.message.contains("in the expansion of `second`"));
    assert!(
        error
            .related
            .iter()
            .any(|note| note.source == first && note.span == Span::new(3, 9))
    );
    assert!(
        error
            .related
            .iter()
            .any(|note| note.source == second && note.span == Span::new(3, 10))
    );
}

#[test]
fn template_name_hover_and_definition_navigate_across_modules() {
    let main = "\nmain = () { fake() }\n";
    let template = "// helper\nhelper = 7\n";
    let (mut graph, _, _) =
        common::check_files(&[("src/main.i", main), ("src/template.i", template)]);
    let (mut parsed, _) = ilang_check::parse(&graph);
    let ts = parsed
        .iter()
        .find(|p| p.path == "app/template")
        .unwrap()
        .source;
    let module = parsed.iter_mut().find(|p| p.path == "app/main").unwrap();
    let ms = module.source;
    let last = tree::last_expression(&mut module.ast, 0);
    let call_span = tree::span(last);
    let call = syntax("fake", ms.0, call_span, &[]);
    let reference = syntax("helper", ts.0, Span::new(3, 9), &[7]);
    let mut scoped = ScopedSource::new(main.into());
    let expanded = scoped.expression(&reference, &call).unwrap();
    scoped.inherit_call_chain(tree::span(&expanded), call_span, "fake");
    *last = expanded;
    graph.sources.set_text(ms, scoped.text().into());
    module.scoped = Some(scoped);
    let checked = check_parsed_for_queries(&graph, &parsed, vec![]);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        checked.diags
    );
    let program = checked.program.as_ref().unwrap();
    let template_module = common::module(program, "app/template");
    let query = Query::new(&graph, program, checked.types.as_ref());
    let hover = query.type_at(template_module, 4).unwrap();
    assert_eq!(hover.span, Span::new(3, 9));
    assert!(hover.code.contains("helper"), "{hover:?}");
    let target = query.definition_at(template_module, 4).unwrap();
    let binder = template.rfind("helper").unwrap() as u32;
    assert_eq!(
        (target.source, target.span),
        (ts, Span::new(binder, binder + 6))
    );
}
