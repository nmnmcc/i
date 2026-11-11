//! Stop sites in expanded code point to the call and the template (§9).
use ilang_syntax::lisp::{
    Datum, DatumKind,
    scoped::ScopedSource,
    syntax::{Scope, Syntax},
};
use std::collections::BTreeSet;

fn shift(d: &mut Datum, n: u32) {
    d.span.start += n;
    d.span.end += n;
    match &mut d.kind {
        DatumKind::Round(xs)
        | DatumKind::Square(xs)
        | DatumKind::Curly(xs)
        | DatumKind::Bits(xs) => {
            for child in xs {
                shift(child, n);
            }
        }
        DatumKind::Quote(x) | DatumKind::Unquote(x) | DatumKind::Splice(x) => shift(x, n),
        _ => {}
    }
}

#[test]
fn template_stop_keeps_call_and_template_locations() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    let main = "\nmain = () { fake() }\n";
    let template = "// (assert False)\nhelper = 1\n";
    std::fs::write(dir.path().join("src/main.i"), main).unwrap();
    std::fs::write(dir.path().join("src/template.i"), template).unwrap();
    let (mut graph, load) = ilang_package::load(dir.path());
    assert!(load.is_empty(), "{load:?}");
    let (mut parsed, errors) = ilang_check::parse(&graph);
    assert!(errors.is_empty(), "{errors:?}");
    let ts = parsed
        .iter()
        .find(|p| p.path == "app/template")
        .unwrap()
        .source;
    let module = parsed.iter_mut().find(|p| p.path == "app/main").unwrap();
    let ms = module.source;
    let ilang_syntax::ast::ItemKind::Binding(binding) = &mut module.ast.items[0].kind else {
        panic!()
    };
    let ilang_syntax::ast::ExprKind::Fn(function) = &mut binding.value.kind else {
        panic!()
    };
    let Some(ilang_syntax::ast::Stmt::Expr(last)) = function.body.stmts.last_mut() else {
        panic!()
    };
    let call_span = last.span;
    let call = Syntax::from_datum(
        &Datum {
            kind: DatumKind::Name("fake".into()),
            span: call_span,
            docs: vec![],
        },
        &Scope {
            module: ms.0,
            marks: BTreeSet::new(),
        },
    );
    let mut datum = ilang_syntax::lisp::parse("(assert False)")
        .unwrap()
        .datums
        .remove(0);
    shift(&mut datum, 3);
    let generated = Syntax::from_datum(
        &datum,
        &Scope {
            module: ts.0,
            marks: [11].into_iter().collect(),
        },
    );
    let mut scoped = ScopedSource::new(main.into());
    let expanded = scoped.expression(&generated, &call).unwrap();
    scoped.inherit_call_chain(expanded.span, call_span, "fake");
    *last = expanded;
    graph.sources.set_text(ms, scoped.text().into());
    module.scoped = Some(scoped);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        checked.diags
    );
    let wasm = ilang_wasm::compile(
        &graph,
        checked.program.as_ref().unwrap(),
        checked.types.as_ref().unwrap(),
        Default::default(),
    )
    .unwrap();
    for level in [ilang_cli::Level::O0, ilang_cli::Level::O2] {
        let bytes = ilang_cli::optimize(&wasm, level).unwrap();
        let outcome = ilang_host::Program::from_bytes(&bytes)
            .unwrap()
            .run_entry(Default::default())
            .unwrap();
        let ilang_host::Outcome::Stop(stop) = outcome else {
            panic!("{outcome:?}")
        };
        let message = stop.message.unwrap();
        assert!(message.contains("app/main:2:"), "{message}");
        assert!(
            message.contains("expanded from app/template:1:"),
            "{message}"
        );
    }
}
