//! The compile-time runner uses the opaque Syntax ABI in a fresh sandbox.
use std::collections::BTreeSet;

use ilang_syntax::lisp::{
    parse,
    syntax::{Scope, Syntax},
};

fn build(source: &str) -> ilang_macro::run::MacroProgram {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("i.toml"),
        "name = \"app\"\n[wasm]\nexports = [\"app/main.identity\"]\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("src/main.i"), source).unwrap();
    let built = ilang_cli::build(
        dir.path(),
        ilang_cli::BuildOptions {
            tests: false,
            opt: ilang_cli::Level::O0,
            offline: true,
            target: ilang_cli::Target::Ilang,
        },
    )
    .unwrap();
    ilang_macro::run::MacroProgram::new(built.wasm, built.binding_plan, built.trusted_package)
}

fn input() -> Syntax {
    Syntax::from_datum(
        &parse("(f x)").unwrap()[0],
        &Scope {
            module: 7,
            marks: BTreeSet::from([42]),
        },
    )
}

#[test]
fn syntax_crosses_the_compiler_export_without_losing_identity() {
    let built = build("(syntax = i/syntax)\n(identity)\nidentity = (x: syntax.Syntax) { x }\n");
    let original = input();
    for _ in 0..2 {
        let result =
            ilang_macro::run::invoke(&built, "app/main.identity", &[original.clone()]).unwrap();
        assert!(result.identical(&original));
        assert_eq!(result.scope(), original.scope());
    }
}

#[test]
fn untrusted_external_cannot_be_supplied_to_a_macro_instance() {
    let built = build(
        "(syntax = i/syntax)\n(identity)\n@external(\"example:outside\", \"effect\")\neffect: () -> Nil\nidentity = (x: syntax.Syntax) { effect() x }\n",
    );
    let error = ilang_macro::run::invoke(&built, "app/main.identity", &[input()]).unwrap_err();
    assert!(format!("{error:#}").contains("instantiate macro program"));
}

#[test]
fn compiler_export_is_checked_and_executed_with_syntax_services() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(syntax = i/syntax)\nidentity = (x: syntax.Syntax) { syntax.node(x) x }\n",
    )
    .unwrap();
    let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty());
    let built = ilang_macro::run::compile(&graph, &parsed, &[("app/main", "identity")]).unwrap();
    let value = input();
    let result = ilang_macro::run::invoke(&built, "app/main.identity", &[value.clone()]).unwrap();
    assert!(result.identical(&value));
}

#[test]
fn expression_quote_is_compiled_and_keeps_unquote_identity() {
    use ilang_syntax::lisp::syntax::Node;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.is"), "(, (= syntax (/ i syntax)))\n(= quoted (-> ((: x (. syntax Syntax))) (. syntax Syntax) `(f ~x)))\n").unwrap();
    let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (mut parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let mut templates = Vec::new();
    for (id, module) in parsed.iter_mut().enumerate() {
        ilang_syntax::lisp::quote_lower::lower_expression_quotes(
            &mut module.ast,
            id as u32,
            &mut templates,
        );
    }
    let built = ilang_macro::run::compile(&graph, &parsed, &[("app/main", "quoted")]).unwrap();
    let original = input();
    let result = ilang_macro::run::invoke_with_templates(
        &built,
        "app/main.quoted",
        &[original.clone()],
        templates.into(),
    )
    .unwrap();
    let Node::Round(items) = result.node() else {
        panic!("quote did not return a round datum")
    };
    assert_eq!(result.show(), "(f (f x))");
    assert!(items[1].identical(&original));
}

#[test]
fn quote_splice_preserves_order_and_reuses_original_syntax() {
    use ilang_syntax::lisp::syntax::Node;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.is"), "(, (= syntax (/ i syntax)))\n(= quoted (-> ((: x (. syntax Syntax))) (. syntax Syntax) `(f ~x ~..[x x] z)))\n").unwrap();
    let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (mut parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let mut templates = Vec::new();
    for (id, module) in parsed.iter_mut().enumerate() {
        ilang_syntax::lisp::quote_lower::lower_expression_quotes(
            &mut module.ast,
            id as u32,
            &mut templates,
        );
    }
    let built = ilang_macro::run::compile(&graph, &parsed, &[("app/main", "quoted")]).unwrap();
    let original = input();
    let result = ilang_macro::run::invoke_with_templates(
        &built,
        "app/main.quoted",
        &[original.clone()],
        templates.into(),
    )
    .unwrap();
    let Node::Round(items) = result.node() else {
        panic!("quote did not return a round datum")
    };
    assert_eq!(result.show(), "(f (f x) (f x) (f x) z)");
    assert!(items[1..4].iter().all(|item| item.identical(&original)));
}

#[test]
fn prepares_and_executes_a_real_macro_binding() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, wrap)\n(@ macro (= wrap (-> ((: x (. syntax Syntax))) (. syntax Syntax) `(wrapped ~x))))\n").unwrap();
    let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let macro_source = parsed
        .iter()
        .find(|module| module.path == "app/macros")
        .unwrap()
        .source;
    let mut parsed = ilang_macro::prepare::import_closure(&graph, &parsed, macro_source).unwrap();
    let prepared = ilang_macro::prepare::prepare(&graph, &mut parsed).unwrap();
    assert_eq!(prepared.bindings.len(), 1);
    assert_eq!(prepared.bindings[0].name, "wrap");
    assert_eq!(prepared.bindings[0].parameters, ["x"]);
    let built = ilang_macro::run::compile(&graph, &parsed, &[("app/macros", "wrap")]).unwrap();
    let original = input();
    let result = ilang_macro::run::invoke_with_templates(
        &built,
        "app/macros.wrap",
        &[original],
        prepared.templates.into(),
    )
    .unwrap();
    assert_eq!(result.show(), "(wrapped (f x))");
    assert_eq!(result.scope().module, macro_source.0);
}

#[test]
fn compiler_program_selects_only_the_macro_import_closure() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, id)\n(@ macro (= id (-> ((: x (. syntax Syntax))) (. syntax Syntax) x)))\n").unwrap();
    std::fs::write(
        dir.path().join("src/unrelated.i"),
        "@external(\"outside\", \"effect\")\neffect: () -> Int\ninitial = effect()\n",
    )
    .unwrap();
    let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty());
    let source = parsed
        .iter()
        .find(|m| m.path == "app/macros")
        .unwrap()
        .source;
    let mut selected = ilang_macro::prepare::import_closure(&graph, &parsed, source).unwrap();
    assert!(!selected.iter().any(|m| m.path == "app/unrelated"));
    let prepared = ilang_macro::prepare::prepare(&graph, &mut selected).unwrap();
    let built = ilang_macro::run::compile(&graph, &selected, &[("app/macros", "id")]).unwrap();
    let original = input();
    let result = ilang_macro::run::invoke_with_templates(
        &built,
        "app/macros.id",
        &[original.clone()],
        prepared.templates.into(),
    )
    .unwrap();
    assert!(result.identical(&original));
}
