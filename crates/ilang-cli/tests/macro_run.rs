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
fn quote_pattern_selects_original_syntax_and_falls_through() {
    use ilang_syntax::lisp::syntax::Node;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.is"), "(, (= syntax (/ i syntax)))\n(@ macro (= choose (-> ((: x (. syntax Syntax))) (. syntax Syntax) (. x { (-> (? `(f ~item) False) item) (-> `(f ~`(g ~item)) item) (-> `(f ~item) item) (-> `(s ~..[first ..rest]) first) (-> (| `(h ~item) `(i ~item)) item) (-> _ x) }))))\n").unwrap();
    let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let source = parsed.iter().find(|m| m.path == "app/main").unwrap().source;
    let mut parsed = ilang_macro::prepare::import_closure(&graph, &parsed, source).unwrap();
    let templates = ilang_macro::prepare::prepare(&graph, &mut parsed)
        .unwrap()
        .templates;
    let built = ilang_macro::run::compile(&graph, &parsed, &[("app/main", "choose")]).unwrap();
    let original = input();
    let Node::Round(children) = original.node() else {
        panic!("expected round syntax");
    };
    let got = ilang_macro::run::invoke_with_templates(
        &built,
        "app/main.choose",
        &[original.clone()],
        templates.clone().into(),
    )
    .unwrap();
    assert!(got.identical(&children[1]));
    for code in ["(h a)", "(i a)"] {
        let choice = Syntax::from_datum(&parse(code).unwrap()[0], original.scope());
        let Node::Round(children) = choice.node() else {
            panic!("expected round syntax");
        };
        let got = ilang_macro::run::invoke_with_templates(
            &built,
            "app/main.choose",
            &[choice.clone()],
            templates.clone().into(),
        )
        .unwrap();
        assert!(got.identical(&children[1]));
    }
    let empty = Syntax::from_datum(&parse("(s)").unwrap()[0], original.scope());
    let got = ilang_macro::run::invoke_with_templates(
        &built,
        "app/main.choose",
        &[empty.clone()],
        templates.clone().into(),
    )
    .unwrap();
    assert!(got.identical(&empty));
    let nested = Syntax::from_datum(&parse("(f (g y))").unwrap()[0], original.scope());
    let Node::Round(outer) = nested.node() else {
        panic!("expected round syntax");
    };
    let Node::Round(inner) = outer[1].node() else {
        panic!("expected nested syntax");
    };
    let got = ilang_macro::run::invoke_with_templates(
        &built,
        "app/main.choose",
        &[nested.clone()],
        templates.clone().into(),
    )
    .unwrap();
    assert!(got.identical(&inner[1]));
    let spliced = Syntax::from_datum(&parse("(s z q)").unwrap()[0], original.scope());
    let Node::Round(children) = spliced.node() else {
        panic!("expected round syntax");
    };
    let got = ilang_macro::run::invoke_with_templates(
        &built,
        "app/main.choose",
        &[spliced.clone()],
        templates.clone().into(),
    )
    .unwrap();
    assert!(got.identical(&children[1]));
    let other = Syntax::from_datum(&parse("(g x)").unwrap()[0], original.scope());
    let got = ilang_macro::run::invoke_with_templates(
        &built,
        "app/main.choose",
        &[other.clone()],
        templates.into(),
    )
    .unwrap();
    assert!(got.identical(&other));
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

#[test]
fn compiler_run_preserves_uncatchable_macro_error_position() {
    use ilang_host::syntax::{DefinitionAbort, DefinitionContext, DefinitionReply};
    use ilang_macro::run::RunOutcome;
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.i"), "(syntax = i/syntax)\nreport = (x: syntax.Syntax) -> syntax.Syntax { syntax._report_error(\"bad macro\", x) }\n").unwrap();
    let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty());
    let (mut parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty());
    parsed
        .iter_mut()
        .find(|m| m.path == "app/main")
        .unwrap()
        .compiler_generated = true;
    let built = ilang_macro::run::compile(&graph, &parsed, &[("app/main", "report")]).unwrap();
    let original = input();
    let context = Arc::new(DefinitionContext::new(|at| DefinitionReply::CompileError {
        message: "unexpected definition".into(),
        at: at.clone(),
    }));
    let result = ilang_macro::run::invoke_with_context(
        &built,
        "app/main.report",
        &[original.clone()],
        Arc::from([]),
        context,
    )
    .unwrap();
    let RunOutcome::Aborted(DefinitionAbort::MacroError { message, at }) = result else {
        panic!("wrong completion: {result:?}")
    };
    assert_eq!(message, "bad macro");
    assert!(at.identical(&original));
}

#[test]
fn compiler_wrapper_reports_a_macro_syntax_error_with_original_position() {
    use ilang_host::syntax::{DefinitionAbort, DefinitionContext, DefinitionReply};
    use ilang_macro::run::RunOutcome;
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)) (= fail (/ i fail)))\n(, bad)\n(@ macro (= bad (-> ((: x (. syntax Syntax))) (^ (. syntax Syntax) (. syntax SyntaxError)) ((. fail raise) ((. syntax SyntaxError) \"bad\" x)))))\n").unwrap();
    let (mut graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty());
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let source = parsed
        .iter()
        .find(|m| m.path == "app/macros")
        .unwrap()
        .source;
    let mut parsed = ilang_macro::prepare::import_closure(&graph, &parsed, source).unwrap();
    let prepared = ilang_macro::prepare::prepare(&graph, &mut parsed).unwrap();
    let (path, name) = ilang_macro::prepare::add_wrapper(
        &mut graph,
        &mut parsed,
        &prepared.bindings[0],
        ilang_macro::prepare::MacroPosition::Item,
    );
    let built = ilang_macro::run::compile(&graph, &parsed, &[(&path, &name)]).unwrap();
    let input = input();
    let context = Arc::new(DefinitionContext::new(|at| DefinitionReply::CompileError {
        message: "unexpected query".into(),
        at: at.clone(),
    }));
    let result = ilang_macro::run::invoke_with_context(
        &built,
        &format!("{path}.{name}"),
        &[input.clone()],
        prepared.templates.into(),
        context,
    )
    .unwrap();
    let RunOutcome::Aborted(DefinitionAbort::MacroError { message, at }) = result else {
        panic!("wrong completion: {result:?}")
    };
    assert_eq!(message, "bad");
    assert!(at.identical(&input));
}

#[test]
fn compiler_wrapper_returns_a_successful_macro_value() {
    use ilang_host::syntax::{DefinitionContext, DefinitionReply};
    use ilang_macro::run::RunOutcome;
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, wrap)\n(@ macro (= wrap (-> ((: x (. syntax Syntax))) (. syntax Syntax) `(wrapped ~x))))\n").unwrap();
    let (mut graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty());
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty());
    let source = parsed
        .iter()
        .find(|m| m.path == "app/macros")
        .unwrap()
        .source;
    let mut parsed = ilang_macro::prepare::import_closure(&graph, &parsed, source).unwrap();
    let prepared = ilang_macro::prepare::prepare(&graph, &mut parsed).unwrap();
    let (path, name) = ilang_macro::prepare::add_wrapper(
        &mut graph,
        &mut parsed,
        &prepared.bindings[0],
        ilang_macro::prepare::MacroPosition::Item,
    );
    let built = ilang_macro::run::compile(&graph, &parsed, &[(&path, &name)]).unwrap();
    let context = Arc::new(DefinitionContext::new(|at| DefinitionReply::CompileError {
        message: "unexpected query".into(),
        at: at.clone(),
    }));
    let result = ilang_macro::run::invoke_with_context(
        &built,
        &format!("{path}.{name}"),
        &[input()],
        prepared.templates.clone().into(),
        context,
    )
    .unwrap();
    let RunOutcome::Returned(value) = result else {
        panic!("wrong completion: {result:?}")
    };
    assert_eq!(value.show(), "(wrapped (f x))");
    let budget = ilang_host::macro_budget::MacroBudget::new(ilang_host::macro_budget::Limits {
        work: 1,
        allocation: u64::MAX,
        frames: u64::MAX,
    });
    let context = Arc::new(DefinitionContext::new(|at| DefinitionReply::CompileError {
        message: "unexpected query".into(),
        at: at.clone(),
    }));
    let error = ilang_macro::run::invoke_with_items_budget(
        &built,
        &format!("{path}.{name}"),
        &[input()],
        prepared.templates.into(),
        context,
        None,
        budget.clone(),
    )
    .unwrap_err();
    assert_eq!(
        budget.exceeded(),
        Some(ilang_host::macro_budget::Limit::Work)
    );
    assert_eq!(
        error.downcast_ref::<ilang_host::macro_budget::Exceeded>(),
        Some(&ilang_host::macro_budget::Exceeded(
            ilang_host::macro_budget::Limit::Work
        ))
    );
}

#[test]
fn compiler_wrapper_rejects_a_macro_with_the_wrong_result_type() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/macros.is"),
        "(, wrong)\n(@ macro (= wrong (-> (x) _ 1)))\n",
    )
    .unwrap();
    let (mut graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty());
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty());
    let source = parsed
        .iter()
        .find(|m| m.path == "app/macros")
        .unwrap()
        .source;
    let mut parsed = ilang_macro::prepare::import_closure(&graph, &parsed, source).unwrap();
    let prepared = ilang_macro::prepare::prepare(&graph, &mut parsed).unwrap();
    let (path, name) = ilang_macro::prepare::add_wrapper(
        &mut graph,
        &mut parsed,
        &prepared.bindings[0],
        ilang_macro::prepare::MacroPosition::Item,
    );
    let error = match ilang_macro::run::compile(&graph, &parsed, &[(&path, &name)]) {
        Ok(_) => panic!("wrong macro result accepted"),
        Err(e) => e,
    };
    assert!(
        matches!(error, ilang_macro::run::CompileError::Program(ref diags) if diags.iter().any(|d| d.is_error())),
        "{error:?}"
    );
}

#[test]
fn compiler_wrapper_buffers_items_and_notes_for_success() {
    use ilang_host::syntax::{DefinitionContext, DefinitionReply, ItemContext, ItemRequest};
    use ilang_macro::run::RunOutcome;
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, keep)\n(@ macro (= keep (-> (x (! it (. syntax Items)) (! d (. syntax Diagnostics))) _ { ((. d note) \"note\") ((. it expose) x) x })))\n").unwrap();
    let (mut graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty());
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let source = parsed
        .iter()
        .find(|m| m.path == "app/macros")
        .unwrap()
        .source;
    let mut parsed = ilang_macro::prepare::import_closure(&graph, &parsed, source).unwrap();
    let prepared = ilang_macro::prepare::prepare(&graph, &mut parsed).unwrap();
    let (path, name) = ilang_macro::prepare::add_wrapper(
        &mut graph,
        &mut parsed,
        &prepared.bindings[0],
        ilang_macro::prepare::MacroPosition::Item,
    );
    let built = ilang_macro::run::compile(&graph, &parsed, &[(&path, &name)]).unwrap();
    let argument = Syntax::from_datum(
        &parse("public").unwrap()[0],
        &Scope {
            module: 7,
            marks: BTreeSet::new(),
        },
    );
    let context = Arc::new(DefinitionContext::new(|at| DefinitionReply::CompileError {
        message: "unexpected query".into(),
        at: at.clone(),
    }));
    let items = Arc::new(ItemContext::new(
        Arc::from([argument.clone()]),
        argument.clone(),
    ));
    let outcome = ilang_macro::run::invoke_with_items(
        &built,
        &format!("{path}.{name}"),
        &[argument.clone()],
        prepared.templates.into(),
        context.clone(),
        Some(items.clone()),
    )
    .unwrap();
    let RunOutcome::Returned(result) = outcome else {
        panic!("wrong completion: {outcome:?}")
    };
    assert!(result.identical(&argument));
    assert_eq!(context.take_notes(), ["note"]);
    let requests = items.take_requests();
    assert!(
        matches!(requests.as_slice(), [ItemRequest::Expose { name }] if name.identical(&argument))
    );
}

#[test]
fn expression_macro_requiring_items_is_rejected_before_execution() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, keep)\n(@ macro (= keep (-> (x (! it (. syntax Items))) _ { ((. it expose) x) x })))\n").unwrap();
    let (mut graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty());
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty());
    let source = parsed
        .iter()
        .find(|m| m.path == "app/macros")
        .unwrap()
        .source;
    let mut parsed = ilang_macro::prepare::import_closure(&graph, &parsed, source).unwrap();
    let prepared = ilang_macro::prepare::prepare(&graph, &mut parsed).unwrap();
    let (path, name) = ilang_macro::prepare::add_wrapper(
        &mut graph,
        &mut parsed,
        &prepared.bindings[0],
        ilang_macro::prepare::MacroPosition::Expression,
    );
    let error = match ilang_macro::run::compile(&graph, &parsed, &[(&path, &name)]) {
        Ok(_) => panic!("Items was allowed in an expression macro"),
        Err(e) => e,
    };
    assert!(
        matches!(error, ilang_macro::run::CompileError::Program(ref diags) if diags.iter().any(|d| d.is_error())),
        "{error:?}"
    );
}

#[test]
fn expansion_preserves_sharing_between_output_and_item_request() {
    use ilang_host::syntax::{DefinitionReply, ItemRequest};
    use ilang_macro::session::{Expanded, Session};
    use ilang_syntax::lisp::expansion::{MacroArgument, MacroDefinition};
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, keep)\n(@ macro (= keep (-> (x (! it (. syntax Items))) _ { ((. it expose) x) x })))\n").unwrap();
    let (mut graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty());
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty());
    let source = parsed
        .iter()
        .find(|m| m.path == "app/macros")
        .unwrap()
        .source;
    let mut parsed = ilang_macro::prepare::import_closure(&graph, &parsed, source).unwrap();
    let prepared = ilang_macro::prepare::prepare(&graph, &mut parsed).unwrap();
    let (path, name) = ilang_macro::prepare::add_wrapper(
        &mut graph,
        &mut parsed,
        &prepared.bindings[0],
        ilang_macro::prepare::MacroPosition::Item,
    );
    let built = ilang_macro::run::compile(&graph, &parsed, &[(&path, &name)]).unwrap();
    let argument = Syntax::from_datum(
        &parse("public").unwrap()[0],
        &Scope {
            module: 7,
            marks: BTreeSet::new(),
        },
    );
    let definition = MacroDefinition {
        item: 0,
        name: "keep".into(),
        parameters: vec!["x".into()],
    };
    let mut session = Session::default();
    let expanded = session
        .run(
            &definition,
            &[MacroArgument {
                label: None,
                syntax: argument.clone(),
            }],
            None,
            &argument,
            ilang_macro::prepare::MacroPosition::Item,
            &built,
            &format!("{path}.{name}"),
            prepared.templates.into(),
            Arc::new(|at, _budget| DefinitionReply::CompileError {
                message: "unexpected query".into(),
                at: at.clone(),
            }),
            |_, syntax, _budget| {
                Ok(Expanded {
                    syntax,
                    transactions: Vec::new(),
                })
            },
        )
        .unwrap();
    assert_eq!(expanded.syntax.scope(), argument.scope());
    let [transaction] = expanded.transactions.as_slice() else {
        panic!("missing transaction")
    };
    assert!(
        matches!(transaction.requests.as_slice(), [ItemRequest::Expose { name }] if name.identical(&expanded.syntax))
    );
}

#[test]
fn separate_runs_get_fresh_marks_and_leave_argument_scope_unchanged() {
    use ilang_host::syntax::DefinitionReply;
    use ilang_macro::session::{Expanded, Session};
    use ilang_syntax::lisp::{
        expansion::{MacroArgument, MacroDefinition},
        syntax::Node,
    };
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/macros.is"),
        "(, (= syntax (/ i syntax)))\n(, wrap)\n(@ macro (= wrap (-> (x) _ `(wrapped ~x))))\n",
    )
    .unwrap();
    let (mut graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty());
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty());
    let source = parsed
        .iter()
        .find(|m| m.path == "app/macros")
        .unwrap()
        .source;
    let mut parsed = ilang_macro::prepare::import_closure(&graph, &parsed, source).unwrap();
    let prepared = ilang_macro::prepare::prepare(&graph, &mut parsed).unwrap();
    let (path, name) = ilang_macro::prepare::add_wrapper(
        &mut graph,
        &mut parsed,
        &prepared.bindings[0],
        ilang_macro::prepare::MacroPosition::Expression,
    );
    let built = ilang_macro::run::compile(&graph, &parsed, &[(&path, &name)]).unwrap();
    let definition = MacroDefinition {
        item: 0,
        name: "wrap".into(),
        parameters: vec!["x".into()],
    };
    let argument = input();
    let templates: Arc<[Syntax]> = prepared.templates.into();
    let resolver: Arc<ilang_macro::session::DefinitionResolver> =
        Arc::new(|at, _budget| DefinitionReply::CompileError {
            message: "unexpected query".into(),
            at: at.clone(),
        });
    let mut session = Session::default();
    let mut run = || {
        session
            .run(
                &definition,
                &[MacroArgument {
                    label: None,
                    syntax: argument.clone(),
                }],
                None,
                &argument,
                ilang_macro::prepare::MacroPosition::Expression,
                &built,
                &format!("{path}.{name}"),
                templates.clone(),
                resolver.clone(),
                |_, syntax, _budget| {
                    Ok(Expanded {
                        syntax,
                        transactions: Vec::new(),
                    })
                },
            )
            .unwrap()
            .syntax
    };
    let a = run();
    let b = run();
    assert!(a.same_code(&b));
    assert_ne!(a.scope().marks, b.scope().marks);
    let (Node::Round(a), Node::Round(b)) = (a.node(), b.node()) else {
        panic!("macro must return a round datum")
    };
    assert_eq!(a[1].scope(), argument.scope());
    assert_eq!(b[1].scope(), argument.scope());
    assert_eq!(a[0].scope().marks.len(), 1);
    assert_eq!(b[0].scope().marks.len(), 1);
}

#[test]
fn macro_registry_respects_imports_exports_and_scoped_shadowing() {
    use ilang_check::hygiene::Binding;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, exported)\n(@ macro (= exported (-> ((: x (. syntax Syntax))) (. syntax Syntax) x)))\n(@ macro (= private (-> ((: x (. syntax Syntax))) (. syntax Syntax) x)))\n").unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros, (exported) = app/macros)\nmain = () { Nil }\n",
    )
    .unwrap();
    let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let caller = parsed.iter().find(|p| p.path == "app/main").unwrap().source;
    let macros = parsed
        .iter()
        .find(|p| p.path == "app/macros")
        .unwrap()
        .source;
    let registry = ilang_macro::registry::Registry::new(&graph, &parsed).unwrap();
    let scope = Scope {
        module: caller.0,
        marks: BTreeSet::new(),
    };
    let atom = |name: &str| Syntax::from_datum(&parse(name).unwrap()[0], &scope);
    let alias = atom("m");
    let exported = atom("exported");
    let private = atom("private");
    assert_eq!(
        registry
            .resolve(caller, Some(&alias), &exported, &[])
            .unwrap()
            .source,
        macros
    );
    assert_eq!(
        registry
            .resolve(caller, None, &exported, &[])
            .unwrap()
            .source,
        macros
    );
    assert!(
        registry
            .resolve(caller, Some(&alias), &private, &[])
            .is_none()
    );
    let locals = [Binding {
        text: "m".into(),
        scope: scope.clone(),
        identity: None,
        target: (),
    }];
    assert!(
        registry
            .resolve(caller, Some(&alias), &exported, &locals)
            .is_none()
    );
    let locals = [Binding {
        text: "exported".into(),
        scope: scope.clone(),
        identity: None,
        target: (),
    }];
    assert!(registry.resolve(caller, None, &exported, &locals).is_none());
    let mut expanded = parsed.clone();
    let module = expanded.iter_mut().find(|p| p.source == caller).unwrap();
    let mut scoped =
        ilang_syntax::lisp::scoped::ScopedSource::new(graph.sources.get(caller).text.clone());
    let generated = Syntax::from_datum(&parse("(= exported 1)").unwrap()[0], &scope).flip_mark(99);
    module
        .ast
        .items
        .extend(scoped.items(&generated, &exported).unwrap());
    module.scoped = Some(scoped);
    let registry = ilang_macro::registry::Registry::new(&graph, &expanded).unwrap();
    assert!(registry.resolve(caller, None, &exported, &[]).is_some());
    assert!(
        registry
            .resolve(caller, None, &exported.flip_mark(99), &[])
            .is_none()
    );
}

#[test]
fn body_engine_expands_real_imported_macro_before_checking_arguments() {
    use ilang_host::syntax::DefinitionReply;
    use ilang_macro::{body_engine::BodyEngine, body_walk, registry::Registry};
    use ilang_syntax::lisp::scoped::ScopedSource;
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("i.toml"),
        "name = \"app\"\n[wasm]\nexports = [\"app/main.calc\"]\n",
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, ignore outer)\n(@ macro (= ignore (-> ((: x (. syntax Syntax))) (. syntax Syntax) `123)))\n(@ macro (= outer (-> ((: x (. syntax Syntax))) (. syntax Syntax) x)))\n").unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\n(calc)\ncalc = () { m.outer(m.ignore(unknown(1))) }\n",
    )
    .unwrap();
    let (mut graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (mut parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let registry = Registry::new(&graph, &parsed).unwrap();
    let module = parsed.iter().find(|p| p.path == "app/main").unwrap();
    let mut ast = module.ast.clone();
    let source = module.source;
    let scoped = ScopedSource::new(graph.sources.get(source).text.clone());
    let resolver = Arc::new(|at: &Syntax, _: &ilang_host::macro_budget::MacroBudget| {
        DefinitionReply::CompileError {
            message: "unexpected definition".into(),
            at: at.clone(),
        }
    });
    let mut session = ilang_macro::session::Session::default();
    let mut engine = BodyEngine::new(
        &graph,
        &parsed,
        &registry,
        source,
        scoped,
        resolver,
        &mut session,
    );
    body_walk::expressions(&mut ast, &mut engine).unwrap();
    let ilang_syntax::ast::ItemKind::Binding(binding) = &ast.items[0].kind else {
        panic!("binding expected")
    };
    let ilang_syntax::ast::ExprKind::Fn(fun) = &binding.value.kind else {
        panic!("function expected")
    };
    let ilang_syntax::ast::Stmt::Expr(expr) = &fun.body.stmts[0] else {
        panic!("expression expected")
    };
    let ilang_syntax::ast::ExprKind::Int(value) = &expr.kind else {
        panic!("macro result was not an integer")
    };
    assert_eq!(value.to_string(), "123");
    assert!(engine.scoped.origin(expr.span).is_some());
    let scoped = engine.scoped.clone();
    drop(engine);
    graph.sources.set_text(source, scoped.text().to_owned());
    let module = parsed.iter_mut().find(|p| p.source == source).unwrap();
    module.ast = ast;
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
        ilang_wasm::Options {
            tests: false,
            target: ilang_wasm::Target::Ilang,
        },
    )
    .unwrap();
    let program = ilang_host::Program::from_bytes(&wasm).unwrap();
    let mut instance = program
        .instantiate(ilang_host::HostOptions::default())
        .unwrap();
    let result = instance.call("app/main.calc", &[]).unwrap();
    assert!(
        matches!(result, ilang_host::Outcome::Ok(ilang_host::Value::Int(ref value)) if value.to_string() == "123"),
        "{result:?}"
    );
}

#[test]
fn body_engine_expands_type_arguments_without_evaluating_them() {
    use ilang_host::syntax::DefinitionReply;
    use ilang_macro::{body_engine::BodyEngine, body_walk, registry::Registry};
    use ilang_syntax::lisp::scoped::ScopedSource;
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, use_int)\n(@ macro (= use_int (-> ((: x (. syntax Syntax))) (. syntax Syntax) `Int)))\n").unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\nx: m.use_int(Undefined)\nx = 1\n",
    )
    .unwrap();
    let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let registry = Registry::new(&graph, &parsed).unwrap();
    let module = parsed.iter().find(|p| p.path == "app/main").unwrap();
    let source = module.source;
    let mut ast = module.ast.clone();
    let scoped = ScopedSource::new(graph.sources.get(source).text.clone());
    let resolver = Arc::new(|at: &Syntax, _: &ilang_host::macro_budget::MacroBudget| {
        DefinitionReply::CompileError {
            message: "unexpected definition".into(),
            at: at.clone(),
        }
    });
    let mut session = ilang_macro::session::Session::default();
    let mut engine = BodyEngine::new(
        &graph,
        &parsed,
        &registry,
        source,
        scoped,
        resolver,
        &mut session,
    );
    body_walk::expressions(&mut ast, &mut engine).unwrap();
    let ilang_syntax::ast::ItemKind::Declaration { ty, .. } = &ast.items[0].kind else {
        panic!("declaration expected")
    };
    assert!(
        matches!(&ty.kind, ilang_syntax::ast::TypeKind::Named { name, .. } if name.name == "Int")
    );
    assert!(engine.scoped.origin(ty.span).is_some());
}

#[test]
fn body_engine_expands_pipe_left_operand_as_first_argument() {
    use ilang_host::syntax::DefinitionReply;
    use ilang_macro::{body_engine::BodyEngine, body_walk, registry::Registry};
    use ilang_syntax::lisp::scoped::ScopedSource;
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, first)\n(@ macro (= first (-> ((: x (. syntax Syntax)) (: ignored (. syntax Syntax))) (. syntax Syntax) x)))\n").unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\nmain = () { 42 | m.first(unknown) }\n",
    )
    .unwrap();
    let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let registry = Registry::new(&graph, &parsed).unwrap();
    let module = parsed.iter().find(|p| p.path == "app/main").unwrap();
    let source = module.source;
    let mut ast = module.ast.clone();
    let scoped = ScopedSource::new(graph.sources.get(source).text.clone());
    let resolver = Arc::new(|at: &Syntax, _: &ilang_host::macro_budget::MacroBudget| {
        DefinitionReply::CompileError {
            message: "unexpected definition".into(),
            at: at.clone(),
        }
    });
    let mut session = ilang_macro::session::Session::default();
    let mut engine = BodyEngine::new(
        &graph,
        &parsed,
        &registry,
        source,
        scoped,
        resolver,
        &mut session,
    );
    body_walk::expressions(&mut ast, &mut engine).unwrap();
    let ilang_syntax::ast::ItemKind::Binding(binding) = &ast.items[0].kind else {
        panic!("binding expected")
    };
    let ilang_syntax::ast::ExprKind::Fn(fun) = &binding.value.kind else {
        panic!("function expected")
    };
    let ilang_syntax::ast::Stmt::Expr(expr) = &fun.body.stmts[0] else {
        panic!("expression expected")
    };
    let ilang_syntax::ast::ExprKind::Int(value) = &expr.kind else {
        panic!("integer expected")
    };
    assert_eq!(value.to_string(), "42");
}

#[test]
fn body_engine_discards_notes_if_macro_result_is_not_an_expression() {
    use ilang_host::syntax::DefinitionReply;
    use ilang_macro::{
        body_engine::{BodyEngine, Error},
        body_walk,
        registry::Registry,
    };
    use ilang_syntax::lisp::scoped::ScopedSource;
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, bad)\n(@ macro (= bad (-> (x (! d (. syntax Diagnostics))) _ { ((. d note) \"discard me\") `(= x 1) })))\n").unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\nmain = () { m.bad(1) }\n",
    )
    .unwrap();
    let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty());
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let registry = Registry::new(&graph, &parsed).unwrap();
    let module = parsed.iter().find(|p| p.path == "app/main").unwrap();
    let source = module.source;
    let mut ast = module.ast.clone();
    let scoped = ScopedSource::new(graph.sources.get(source).text.clone());
    let resolver = Arc::new(|at: &Syntax, _: &ilang_host::macro_budget::MacroBudget| {
        DefinitionReply::CompileError {
            message: "unexpected definition".into(),
            at: at.clone(),
        }
    });
    let mut session = ilang_macro::session::Session::default();
    let mut engine = BodyEngine::new(
        &graph,
        &parsed,
        &registry,
        source,
        scoped,
        resolver,
        &mut session,
    );
    assert!(matches!(
        body_walk::expressions(&mut ast, &mut engine),
        Err(Error::Conversion(_))
    ));
    assert!(engine.transactions.is_empty());
}

#[test]
fn body_engine_reports_invalid_macro_signature_at_its_definition() {
    use ilang_host::syntax::DefinitionReply;
    use ilang_macro::{
        body_engine::{BodyEngine, Error},
        body_walk,
        registry::Registry,
    };
    use ilang_syntax::lisp::scoped::ScopedSource;
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/macros.is"),
        "(, bad)\n(@ macro (= bad (-> (x) _ 1)))\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\nmain = () { m.bad(1) }\n",
    )
    .unwrap();
    let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty());
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let registry = Registry::new(&graph, &parsed).unwrap();
    let module = parsed.iter().find(|p| p.path == "app/main").unwrap();
    let macro_source = parsed
        .iter()
        .find(|p| p.path == "app/macros")
        .unwrap()
        .source;
    let source = module.source;
    let mut ast = module.ast.clone();
    let scoped = ScopedSource::new(graph.sources.get(source).text.clone());
    let resolver = Arc::new(|at: &Syntax, _: &ilang_host::macro_budget::MacroBudget| {
        DefinitionReply::CompileError {
            message: "unexpected definition".into(),
            at: at.clone(),
        }
    });
    let mut session = ilang_macro::session::Session::default();
    let mut engine = BodyEngine::new(
        &graph,
        &parsed,
        &registry,
        source,
        scoped,
        resolver,
        &mut session,
    );
    let Err(Error::Compile(diagnostics)) = body_walk::expressions(&mut ast, &mut engine) else {
        panic!("bad macro must fail")
    };
    assert!(
        diagnostics
            .iter()
            .any(|d| d.source == Some(macro_source) && d.message.contains("macro `bad`")),
        "{diagnostics:?}"
    );
    for d in diagnostics {
        if let Some(id) = d.source {
            let _ = graph.sources.get(id);
        }
    }
}

#[test]
fn unused_macro_definitions_are_checked_for_signature_and_capabilities() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    let path = dir.path().join("src/macros.is");
    let check = |source: &str| {
        std::fs::write(&path, source).unwrap();
        let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let (parsed, diagnostics) = ilang_check::parse(&graph);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let source = parsed
            .iter()
            .find(|m| m.path == "app/macros")
            .unwrap()
            .source;
        ilang_macro::validate::module(&graph, &parsed, source)
    };
    assert!(check("(, (= syntax (/ i syntax)))\n(@ macro (= ok (-> ((: x (. syntax Syntax))) (. syntax Syntax) x)))\n").is_ok());
    let wrong = check("(@ macro (= wrong (-> (x) _ 1)))\n").unwrap_err();
    let (mut graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty());
    let checked = ilang_macro::project::check(&mut graph);
    assert!(
        checked
            .diags
            .iter()
            .any(|d| d.is_error() && d.message.contains("macro `wrong`")),
        "{:?}",
        checked.diags
    );
    assert!(
        wrong.iter().any(|d| d.message.contains("macro `wrong`")),
        "{wrong:?}"
    );
    let outside = check("(, (= syntax (/ i syntax)))\n(@ (external \"other\" \"effect\") (: effect (-> () Nil)))\n(@ macro (= unsafe (-> ((: x (. syntax Syntax))) (. syntax Syntax) { (effect) x })))\n").unwrap_err();
    assert!(outside.iter().any(|d| d.is_error()), "{outside:?}");
}

#[test]
fn macro_modules_expand_in_import_order_and_reject_cycles() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/a.i"), "a = 1\n").unwrap();
    std::fs::write(dir.path().join("src/b.i"), "(a = app/a)\nb = 2\n").unwrap();
    std::fs::write(dir.path().join("src/c.i"), "(b = app/b)\nc = 3\n").unwrap();
    let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty());
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let ordered = ilang_macro::module_order::order(&graph, &parsed).unwrap();
    let index = |path: &str| {
        let source = parsed.iter().find(|p| p.path == path).unwrap().source;
        ordered.iter().position(|id| *id == source).unwrap()
    };
    assert!(index("app/a") < index("app/b"));
    assert!(index("app/b") < index("app/c"));
    std::fs::write(dir.path().join("src/a.i"), "(c = app/c)\na = 1\n").unwrap();
    let (graph, _) = ilang_cli::load(dir.path(), false, true);
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty());
    assert!(ilang_macro::module_order::order(&graph, &parsed).is_err());
}

#[test]
fn item_phase_runs_imported_calls_and_preserves_written_types() {
    use ilang_host::syntax::DefinitionReply;
    use ilang_macro::{
        body_engine::BodyEngine, definition_world::DefinitionWorld, item_phase::expand_items,
        registry::Registry,
    };
    use ilang_syntax::lisp::scoped::ScopedSource;
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/macros.is"),
        r#"(, (= syntax (/ i syntax)))
(, emit keep empty)
(@ macro (= emit (-> ((: x (. syntax Syntax))) (. syntax Syntax) `(= generated ~x))))
(@ macro (= keep (-> ((: x (. syntax Syntax))) (. syntax Syntax) x)))
(@ macro (= empty (-> ((: x (. syntax Syntax))) (. syntax Syntax) `{})))
"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\nm.emit(42)\n@m.keep\nf = () { 1 }\n@m.empty\nT { T }\n",
    )
    .unwrap();
    let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let registry = Registry::new(&graph, &parsed).unwrap();
    let source = parsed.iter().find(|p| p.path == "app/main").unwrap().source;
    let mut ast = parsed
        .iter()
        .find(|p| p.source == source)
        .unwrap()
        .ast
        .clone();
    let scoped = ScopedSource::new(graph.sources.get(source).text.clone());
    let resolver = Arc::new(|at: &Syntax, _: &ilang_host::macro_budget::MacroBudget| {
        DefinitionReply::CompileError {
            message: "unexpected query".into(),
            at: at.clone(),
        }
    });
    let mut session = ilang_macro::session::Session::default();
    let mut engine = BodyEngine::new(
        &graph,
        &parsed,
        &registry,
        source,
        scoped,
        resolver,
        &mut session,
    );
    let world = Arc::new(DefinitionWorld::new(&graph, &parsed));
    let transactions = expand_items(&mut engine, &mut ast, world).unwrap();
    assert_eq!(transactions.len(), 3);
    assert_eq!(ast.items.len(), 3);
    assert!(
        matches!(&ast.items[0].kind, ilang_syntax::ast::ItemKind::Binding(b) if matches!(&b.pattern.kind, ilang_syntax::ast::PatternKind::Var(name) if name == "generated"))
    );
    assert!(
        matches!(&ast.items[1].kind, ilang_syntax::ast::ItemKind::Binding(b) if matches!(&b.pattern.kind, ilang_syntax::ast::PatternKind::Var(name) if name == "f"))
    );
    assert!(
        matches!(&ast.items[2].kind, ilang_syntax::ast::ItemKind::TypeDef(t) if t.name.name == "T")
    );
    assert!(ast.items[2].attributes.is_empty());
}

#[test]
fn item_phase_retries_definition_after_a_later_item() {
    use ilang_host::syntax::DefinitionReply;
    use ilang_macro::{
        body_engine::BodyEngine, definition_world::DefinitionWorld, item_phase::expand_items,
        registry::Registry,
    };
    use ilang_syntax::lisp::scoped::ScopedSource;
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/macros.is"),
        r#"(, (= syntax (/ i syntax)))
(, ask make)
(@ macro (= ask (-> (x) _ { (= d ((. syntax definition) x)) `{}})))
(@ macro (= make (-> (x) _ `(= ~x Int))))
"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\nm.ask(T)\nm.make(T)\n",
    )
    .unwrap();
    let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let registry = Registry::new(&graph, &parsed).unwrap();
    let source = parsed.iter().find(|p| p.path == "app/main").unwrap().source;
    let mut ast = parsed
        .iter()
        .find(|p| p.source == source)
        .unwrap()
        .ast
        .clone();
    let scoped = ScopedSource::new(graph.sources.get(source).text.clone());
    let resolver = Arc::new(|at: &Syntax, _: &ilang_host::macro_budget::MacroBudget| {
        DefinitionReply::CompileError {
            message: "unexpected query".into(),
            at: at.clone(),
        }
    });
    let mut session = ilang_macro::session::Session::default();
    let mut engine = BodyEngine::new(
        &graph,
        &parsed,
        &registry,
        source,
        scoped,
        resolver,
        &mut session,
    );
    let world = Arc::new(DefinitionWorld::new(&graph, &parsed));
    let transactions = expand_items(&mut engine, &mut ast, world).unwrap();
    assert_eq!(transactions.len(), 2);
    assert_eq!(ast.items.len(), 1);
    assert!(
        matches!(&ast.items[0].kind, ilang_syntax::ast::ItemKind::TypeDef(t) if t.name.name == "T")
    );
}

#[test]
fn item_phase_reports_each_unresolved_waiter_and_restores_the_module() {
    use ilang_host::syntax::DefinitionReply;
    use ilang_macro::{
        body_engine::BodyEngine,
        definition_world::DefinitionWorld,
        item_phase::{PhaseError, expand_items},
        registry::Registry,
    };
    use ilang_syntax::lisp::scoped::ScopedSource;
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/macros.is"),
        r#"(, (= syntax (/ i syntax)))
(, ask)
(@ macro (= ask (-> (x) _ { (= d ((. syntax definition) x)) `{}})))
"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\nm.ask(A)\nm.ask(B)\n",
    )
    .unwrap();
    let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let registry = Registry::new(&graph, &parsed).unwrap();
    let source = parsed.iter().find(|p| p.path == "app/main").unwrap().source;
    let mut ast = parsed
        .iter()
        .find(|p| p.source == source)
        .unwrap()
        .ast
        .clone();
    let original = ast.clone();
    let scoped = ScopedSource::new(graph.sources.get(source).text.clone());
    let resolver = Arc::new(|at: &Syntax, _: &ilang_host::macro_budget::MacroBudget| {
        DefinitionReply::CompileError {
            message: "unexpected query".into(),
            at: at.clone(),
        }
    });
    let mut session = ilang_macro::session::Session::default();
    let mut engine = BodyEngine::new(
        &graph,
        &parsed,
        &registry,
        source,
        scoped,
        resolver,
        &mut session,
    );
    let world = Arc::new(DefinitionWorld::new(&graph, &parsed));
    let error = expand_items(&mut engine, &mut ast, world)
        .err()
        .expect("both runs must wait");
    let PhaseError::Waiting(waiting) = error else {
        panic!("unexpected {error:?}")
    };
    assert_eq!(waiting.len(), 2);
    assert!(matches!(waiting[0].node(), ilang_syntax::lisp::syntax::Node::Up(name) if name == "A"));
    assert!(matches!(waiting[1].node(), ilang_syntax::lisp::syntax::Node::Up(name) if name == "B"));
    assert_eq!(ast, original);
    assert_eq!(engine.scoped.text(), graph.sources.get(source).text);
}

#[test]
fn item_phase_rechecks_an_early_builtin_answer() {
    use ilang_host::syntax::DefinitionReply;
    use ilang_macro::{
        body_engine::BodyEngine,
        definition_world::DefinitionWorld,
        item_phase::{PhaseError, expand_items},
        registry::Registry,
    };
    use ilang_syntax::lisp::scoped::ScopedSource;
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/macros.is"),
        r#"(, (= syntax (/ i syntax)))
(, replace)
(@ macro (= replace (-> () _ { (= d ((. syntax definition) `Result)) `(= Result Int)})))
"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\nm.replace()\n",
    )
    .unwrap();
    let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let registry = Registry::new(&graph, &parsed).unwrap();
    let source = parsed.iter().find(|p| p.path == "app/main").unwrap().source;
    let mut ast = parsed
        .iter()
        .find(|p| p.source == source)
        .unwrap()
        .ast
        .clone();
    let original = ast.clone();
    let scoped = ScopedSource::new(graph.sources.get(source).text.clone());
    let resolver = Arc::new(|at: &Syntax, _: &ilang_host::macro_budget::MacroBudget| {
        DefinitionReply::CompileError {
            message: "unexpected query".into(),
            at: at.clone(),
        }
    });
    let mut session = ilang_macro::session::Session::default();
    let mut engine = BodyEngine::new(
        &graph,
        &parsed,
        &registry,
        source,
        scoped,
        resolver,
        &mut session,
    );
    let world = Arc::new(DefinitionWorld::new(&graph, &parsed));
    assert!(matches!(
        expand_items(&mut engine, &mut ast, world),
        Err(PhaseError::Changed { .. })
    ));
    assert_eq!(ast, original);
}

#[test]
fn item_phase_queries_the_expanded_generated_type() {
    use ilang_host::{macro_budget::MacroBudget, syntax::DefinitionReply};
    use ilang_macro::{
        body_engine::BodyEngine,
        definition_world::DefinitionWorld,
        item_phase::{Definitions, Resolution, Snapshot, expand_items},
        registry::Registry,
    };
    use ilang_syntax::lisp::{definition_schedule::Mode, scoped::ScopedSource};
    use std::sync::{Arc, Mutex};
    struct Recording {
        world: DefinitionWorld,
        answers: Mutex<Vec<Syntax>>,
    }
    impl Definitions for Recording {
        fn fresh_answer(&self) -> u64 {
            self.world.fresh_answer()
        }
        fn prepare(&self, snapshot: &Snapshot) {
            self.world.prepare(snapshot);
        }
        fn resolve(
            &self,
            snapshot: &Snapshot,
            in_run: &Syntax,
            result: &Syntax,
            mode: Mode,
            mark: u64,
            budget: &MacroBudget,
        ) -> Resolution {
            let resolution = self
                .world
                .resolve(snapshot, in_run, result, mode, mark, budget);
            if let DefinitionReply::Answer(answer) = &resolution.reply {
                self.answers.lock().unwrap().push(answer.clone());
            }
            resolution
        }
    }
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/macros.is"),
        r#"(, (= syntax (/ i syntax)))
(, ask make)
(@ macro (= typ (-> () _ `Int)))
(@ macro (= ask (-> (x) _ { (= d ((. syntax definition) x)) `{}})))
(@ macro (= make (-> (x) _ `(= ~x (typ)))))
"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\nm.make(T)\nm.ask(T)\n",
    )
    .unwrap();
    let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let registry = Registry::new(&graph, &parsed).unwrap();
    let source = parsed.iter().find(|p| p.path == "app/main").unwrap().source;
    let mut ast = parsed
        .iter()
        .find(|p| p.source == source)
        .unwrap()
        .ast
        .clone();
    let scoped = ScopedSource::new(graph.sources.get(source).text.clone());
    let resolver = Arc::new(
        |at: &Syntax, _: &MacroBudget| DefinitionReply::CompileError {
            message: "unexpected query".into(),
            at: at.clone(),
        },
    );
    let mut session = ilang_macro::session::Session::default();
    let mut engine = BodyEngine::new(
        &graph,
        &parsed,
        &registry,
        source,
        scoped,
        resolver,
        &mut session,
    );
    let world = Arc::new(Recording {
        world: DefinitionWorld::new(&graph, &parsed),
        answers: Mutex::new(Vec::new()),
    });
    expand_items(&mut engine, &mut ast, world.clone()).unwrap();
    let answers = world.answers.lock().unwrap();
    assert_eq!(answers.len(), 1);
    let shown = ilang_syntax::lisp::show(&answers[0].to_datum());
    assert!(shown.contains("Int") && !shown.contains("typ"), "{shown}");
}

#[test]
fn project_expands_imported_macro_and_runs_generated_program() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("i.toml"),
        "name = \"app\"\n[wasm]\nexports = [\"app/main.value\"]\n",
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, twice)\n(@ macro (= twice (-> ((: x (. syntax Syntax))) (. syntax Syntax) `(+ ~x ~x))))\n").unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\n(value)\nvalue = () { m.twice(21) }\n",
    )
    .unwrap();
    let (mut graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let expanded = ilang_macro::project::expand(&mut graph).unwrap();
    let checked = ilang_check::check_parsed(&graph, &expanded.parsed);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        checked.diags
    );
    let wasm = ilang_wasm::compile(
        &graph,
        checked.program.as_ref().unwrap(),
        checked.types.as_ref().unwrap(),
        ilang_wasm::Options {
            tests: false,
            target: ilang_wasm::Target::Ilang,
        },
    )
    .unwrap();
    let program = ilang_host::Program::from_bytes(&wasm).unwrap();
    let mut instance = program
        .instantiate(ilang_host::HostOptions::default())
        .unwrap();
    let result = instance.call("app/main.value", &[]).unwrap();
    assert!(
        matches!(result, ilang_host::Outcome::Ok(ilang_host::Value::Int(ref value)) if value.to_string() == "42"),
        "{result:?}"
    );
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
    let program = ilang_host::Program::from_bytes(&built.wasm).unwrap();
    let mut instance = program.instantiate(built.host_options()).unwrap();
    let result = instance.call("app/main.value", &[]).unwrap();
    assert!(
        matches!(result, ilang_host::Outcome::Ok(ilang_host::Value::Int(ref value)) if value.to_string() == "42"),
        "{result:?}"
    );
}

#[test]
fn successful_macro_note_is_a_compiler_note_not_a_warning() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, mark)\n(@ macro (= mark (-> (x (! d (. syntax Diagnostics))) _ { ((. d note) \"hello\") x })))\n").unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\nmain = () { m.mark(1) }\n",
    )
    .unwrap();
    let (mut graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty());
    let checked = ilang_macro::project::check(&mut graph);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        checked.diags
    );
    let notes = checked
        .diags
        .iter()
        .filter(|d| d.severity == ilang_package::Severity::Note)
        .collect::<Vec<_>>();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].message, "hello");
}

#[test]
fn cli_check_and_run_expand_imported_macros() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, twice)\n(@ macro (= twice (-> ((: x (. syntax Syntax))) (. syntax Syntax) `(+ ~x ~x))))\n").unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\nmain = () { m.twice(21) }\n",
    )
    .unwrap();
    for command in ["check", "run"] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_i"))
            .args([command, "--offline", "--project"])
            .arg(dir.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{command}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn cli_expands_attribute_macros_before_checking() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, keep)\n(@ macro (= keep (-> ((: x (. syntax Syntax))) (. syntax Syntax) x)))\n").unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\n@m.keep\nvalue = () { 42 }\nmain = () { value() }\n",
    )
    .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["run", "--offline", "--project"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn cli_runs_item_macro_with_exposed_caller_name() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, emit)\n(@ macro (= emit (-> (x (! it (. syntax Items))) _ { ((. it expose) x) `(= ~x 42) })))\n").unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\nm.emit(value)\nmain = () { value }\n",
    )
    .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["run", "--offline", "--project"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn a_later_macro_failure_keeps_an_earlier_committed_note() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, mark bad)\n(@ macro (= mark (-> (x (! d (. syntax Diagnostics))) _ { ((. d note) \"earlier\") x })))\n(@ macro (= bad (-> (x) _ `(= x 1))))\n").unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\nmain = () { m.mark(1) m.bad(2) }\n",
    )
    .unwrap();
    let (mut graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty());
    let checked = ilang_macro::project::check(&mut graph);
    assert!(
        checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        checked.diags
    );
    assert!(
        checked
            .diags
            .iter()
            .any(|d| d.severity == ilang_package::Severity::Note && d.message == "earlier"),
        "{:?}",
        checked.diags
    );
}

#[test]
fn checked_collection_validates_targets_and_duplicates() {
    use ilang_host::syntax::ItemRequest;
    use ilang_macro::{
        collections,
        session::{ItemOrigin, Transaction},
    };
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(anchor)\nanchor = Nil\ngood = () { 42 }\nbad = (x: Int) { x }\nalias = good\n",
    )
    .unwrap();
    let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        checked.diags.iter().all(|d| !d.is_error()),
        "{:?}",
        checked.diags
    );
    let source = parsed.iter().find(|p| p.path == "app/main").unwrap().source;
    let atom = |name: &str| {
        Syntax::from_datum(
            &parse(name).unwrap()[0],
            &Scope {
                module: source.0,
                marks: BTreeSet::new(),
            },
        )
    };
    let request = |target: &str| ItemRequest::Add {
        collection: atom("anchor"),
        target: atom(target),
        data: vec![("name".into(), "example".into())],
        at: atom("good"),
    };
    let transaction = |requests| Transaction {
        notes: vec![],
        requests,
        item_origin: Some(ItemOrigin {
            source: source.0,
            call: ilang_syntax::span::Span::new(0, 1),
            path: vec![0],
        }),
        mark: 0,
        location: None,
    };
    let program = checked.program.as_ref().unwrap();
    let typed = checked.types.as_ref().unwrap();
    let good = collections::resolve(
        &parsed,
        program,
        typed,
        &[transaction(vec![request("good")])],
    )
    .unwrap();
    assert_eq!(good.len(), 1);
    assert_eq!(program.def(good[0].target).name, "good");
    assert_eq!(good[0].data[0].1, "example");
    for requests in [
        vec![request("bad")],
        vec![request("alias")],
        vec![request("good"), request("good")],
    ] {
        assert!(collections::resolve(&parsed, program, typed, &[transaction(requests)]).is_err());
    }
}

#[test]
fn cli_retries_definition_queries_after_later_item_macros() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, ask make)\n(@ macro (= ask (-> (x) _ { (= d ((. syntax definition) x)) `{}})))\n(@ macro (= make (-> (x) _ `(= ~x Int))))\n").unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\nm.ask(T)\nm.make(T)\nmain = () { 1 }\n",
    )
    .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["run", "--offline", "--project"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn cli_macro_stop_reports_call_site_and_template() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, bomb)\n(@ macro (= bomb (-> ((: x (. syntax Syntax))) (. syntax Syntax) `(panic \"boom\"))))\n").unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\nmain = () { m.bomb(1) }\n",
    )
    .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["run", "--offline", "--project"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("boom")
            && stderr.contains("app/main:2:13")
            && stderr.contains("expanded from app/macros:3:"),
        "{stderr}"
    );
}

#[test]
fn cli_nested_macro_stop_preserves_outer_call_and_templates() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, outer)\n(@ macro (= inner (-> (x) _ `(panic \"boom\"))))\n(@ macro (= outer (-> (x) _ `(inner ~x))))\n").unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\nmain = () { m.outer(1) }\n",
    )
    .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["run", "--offline", "--project"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("panic: boom")
            && stderr.contains("app/main:2:13")
            && stderr.contains("app/macros:3:"),
        "{stderr}"
    );
}

#[test]
fn collection_manifest_from_a_real_item_macro() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, emit)\n(@ macro (= emit (-> (c x (! it (. syntax Items))) _ { ((. it add) c x [] x) `(= ~x (-> () _ { 42 })) })))\n").unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\n(anchor)\nanchor = Nil\nm.emit(anchor, generated)\n",
    )
    .unwrap();
    let (mut graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let expanded = ilang_macro::project::expand(&mut graph).unwrap_or_else(|d| panic!("{d:?}"));
    let checked = ilang_check::check_parsed(&graph, &expanded.parsed);
    assert!(
        checked.diags.iter().all(|d| !d.is_error()),
        "{:?}",
        checked.diags
    );
    let entries = ilang_macro::collections::resolve(
        &expanded.parsed,
        checked.program.as_ref().unwrap(),
        checked.types.as_ref().unwrap(),
        &expanded.transactions,
    )
    .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        checked
            .program
            .as_ref()
            .unwrap()
            .def(entries[0].target)
            .name,
        "generated"
    );
}

#[test]
fn project_check_rejects_a_private_collection_binding() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, emit)\n(@ macro (= emit (-> (c x (! it (. syntax Items))) _ { ((. it add) c x [] x) `(= ~x (-> () _ { 42 })) })))\n").unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(m = app/macros)\nanchor = Nil\nm.emit(anchor, generated)\n",
    )
    .unwrap();
    let (mut graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty());
    let checked = ilang_macro::project::check(&mut graph);
    assert!(
        checked
            .diags
            .iter()
            .any(|d| d.is_error() && d.message.contains("collection must resolve")),
        "{:?}",
        checked.diags
    );
}

#[test]
fn definition_query_reads_expanded_type_from_dependency() {
    use ilang_host::{macro_budget::MacroBudget, syntax::DefinitionReply};
    use ilang_macro::{definition_world::DefinitionWorld, item_phase::{Definitions, Snapshot}};
    use ilang_syntax::lisp::definition_schedule::Mode;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, make)\n(@ macro (= typ (-> () _ `Int)))\n(@ macro (= make (-> (x) _ `(= ~x (typ)))))\n").unwrap();
    std::fs::write(dir.path().join("src/types.i"), "(m = app/macros)\n(T)\nm.make(T)\n").unwrap();
    std::fs::write(dir.path().join("src/main.i"), "((T) = app/types)\nvalue = 1\n").unwrap();
    let (mut graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let expanded = ilang_macro::project::expand(&mut graph).unwrap_or_else(|d| panic!("{d:?}"));
    let module = expanded.parsed.iter().find(|p| p.path == "app/main").unwrap();
    let snapshot = Snapshot { module: module.ast.clone(), ready: vec![true; module.ast.items.len()], scoped: module.scoped.clone().unwrap(), source: module.source.0, other_runs: false, active: None, epoch: 0, item_overrides: Default::default() };
    let world = DefinitionWorld::new(&graph, &expanded.parsed);
    world.prepare(&snapshot);
    let query = Syntax::from_datum(&parse("T").unwrap()[0], &Scope { module: module.source.0, marks: BTreeSet::new() });
    let answer = world.resolve(&snapshot, &query, &query, Mode::Normal, 999, &MacroBudget::default());
    let DefinitionReply::Answer(answer) = answer.reply else { panic!("expected a definition answer") };
    let shown = ilang_syntax::lisp::show(&answer.to_datum());
    assert!(shown.contains("Int") && !shown.contains("typ"), "{shown}");
}
