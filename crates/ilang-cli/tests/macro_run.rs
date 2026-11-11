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
