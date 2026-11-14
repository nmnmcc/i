mod common;

#[test]
fn quote_pattern_captures_typecheck_and_a_fallback_is_reachable() {
    let (graph, _, _) = common::check_files(&[(
        "src/main.is",
        "(, (= syntax (/ i syntax)))\n(= choose (-> (, (: x (. syntax Syntax))) (. syntax Syntax) (|> x (| (-> (, `(f ~item)) _ item) (-> (, _) _ x)))))\n",
    )]);
    let (mut parsed, diagnostics) = ilang_check::parse(&graph);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let mut templates = Vec::new();
    for module in &mut parsed {
        ilang_syntax::lisp::quote_lower3::lower_expression_quotes(
            &mut module.ast,
            module.source.0,
            &mut templates,
        );
    }
    let checked = ilang_check::check_compiler_exports(&graph, &parsed, &[("app/main", "choose")]);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    assert_eq!(templates.len(), 1);
}

#[test]
fn nested_captures_choices_and_guards_typecheck() {
    for body in [
        "(|> x (| (-> (? (, `(f ~item)) False) _ item) (-> (, `(g ~item)) _ item) (-> (, _) _ x)))",
        "(|> x (| (-> (, (| `(f ~item) `(g ~item))) _ item) (-> (, _) _ x)))",
        "(|> x (| (-> (, `(f ~`(g ~item))) _ item) (-> (, _) _ x)))",
        "(|> x (| (-> (, `(f ~..[first ..rest])) _ first) (-> (, _) _ x)))",
    ] {
        let source = format!(
            "(, (= syntax (/ i syntax)))\n(= choose (-> (, (: x (. syntax Syntax))) (. syntax Syntax) {body}))\n"
        );
        let (graph, _, _) = common::check_files(&[("src/main.is", &source)]);
        let (mut parsed, diagnostics) = ilang_check::parse(&graph);
        assert!(diagnostics.is_empty(), "{body}: {diagnostics:?}");
        let mut templates = Vec::new();
        for module in &mut parsed {
            ilang_syntax::lisp::quote_lower3::lower_expression_quotes(
                &mut module.ast,
                module.source.0,
                &mut templates,
            );
        }
        let checked =
            ilang_check::check_compiler_exports(&graph, &parsed, &[("app/main", "choose")]);
        assert!(
            !checked.diags.iter().any(|d| d.is_error()),
            "{body}: {:?}",
            graph.sources.render_all(&checked.diags)
        );
    }
}

#[test]
fn quote_choice_requires_the_same_capture_bindings() {
    let source = "(, (= syntax (/ i syntax)))\n(= choose (-> (, (: x (. syntax Syntax))) (. syntax Syntax) (|> x (| (-> (, (| `(f ~a) `(g ~b))) _ a) (-> (, _) _ x)))))\n";
    let (graph, _, _) = common::check_files(&[("src/main.is", source)]);
    let (mut parsed, _) = ilang_check::parse(&graph);
    let mut templates = Vec::new();
    for module in &mut parsed {
        ilang_syntax::lisp::quote_lower3::lower_expression_quotes(
            &mut module.ast,
            module.source.0,
            &mut templates,
        );
    }
    let checked = ilang_check::check_compiler_exports(&graph, &parsed, &[("app/main", "choose")]);
    assert!(
        checked
            .diags
            .iter()
            .any(|d| d.code == ilang_check::code::CHOICES_DIFFER),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn quote_capture_must_be_a_pattern_for_syntax() {
    let source = "(, (= syntax (/ i syntax)))\n(= choose (-> (, (: x (. syntax Syntax))) (. syntax Syntax) (|> x (| (-> (, `(f ~12)) _ x) (-> (, _) _ x)))))\n";
    let (graph, _, _) = common::check_files(&[("src/main.is", source)]);
    let (mut parsed, _) = ilang_check::parse(&graph);
    let mut templates = Vec::new();
    for module in &mut parsed {
        ilang_syntax::lisp::quote_lower3::lower_expression_quotes(
            &mut module.ast,
            module.source.0,
            &mut templates,
        );
    }
    let checked = ilang_check::check_compiler_exports(&graph, &parsed, &[("app/main", "choose")]);
    assert!(
        checked
            .diags
            .iter()
            .any(|d| d.code == ilang_check::code::TYPE_MISMATCH),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn fitted_quote_patterns_are_refutable() {
    let source =
        "(, (= syntax (/ i syntax)))\n(= choose (-> (, `(f ~item)) (. syntax Syntax) item))\n";
    let (graph, _, _) = common::check_files(&[("src/main.is", source)]);
    let (mut parsed, _) = ilang_check::parse(&graph);
    let mut templates = Vec::new();
    for module in &mut parsed {
        ilang_syntax::lisp::quote_lower3::lower_expression_quotes(
            &mut module.ast,
            module.source.0,
            &mut templates,
        );
    }
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    assert!(checked.types.unwrap().may_fail.values().any(|v| *v));
}
