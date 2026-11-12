mod common;

fn errors(files: &[(&str, &str)]) -> Vec<String> {
    let (graph, _, diagnostics) = common::check_files(files);
    let errors = diagnostics
        .iter()
        .filter(|d| d.is_error())
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        eprintln!("{:?}", graph.sources.render_all(&diagnostics));
    }
    errors.into_iter().map(|d| d.message.clone()).collect()
}

const MACRO: &str = "(, expand)\n(@ macro (= expand (-> (, x) _ `(f ~x))))\n";

#[test]
fn source_macros_are_exportable_and_importable_without_checking_their_bodies() {
    let diags = errors(&[
        ("src/macros.is", MACRO),
        ("src/main.i", "((expand) = app/macros)\n"),
    ]);
    assert!(diags.is_empty(), "{diags:?}");
}

#[test]
fn a_macro_is_not_a_runtime_value_even_when_imported() {
    for caller in [
        "((expand) = app/macros)\n(value)\nvalue = expand\n",
        "(m = app/macros)\n(value)\nvalue = m.expand\n",
    ] {
        let diags = errors(&[("src/macros.is", MACRO), ("src/main.i", caller)]);
        assert!(
            diags
                .iter()
                .any(|e| e.contains("macro `expand` can only be used as a macro call")),
            "{diags:?}"
        );
    }
}

#[test]
fn local_macro_is_not_a_runtime_value() {
    let diags = errors(&[(
        "src/main.is",
        "(, value)\n(@ macro (= expand (-> (, x) _ `(f ~x))))\n(= value expand)\n",
    )]);
    assert!(
        diags
            .iter()
            .any(|e| e.contains("macro `expand` can only be used as a macro call")),
        "{diags:?}"
    );
}

#[test]
fn private_macros_and_collisions_obey_ordinary_value_rules() {
    let diags = errors(&[
        ("src/macros.is", "(@ macro (= expand (-> (, x) _ x)))\n"),
        ("src/main.i", "((expand) = app/macros)\n"),
    ]);
    assert!(
        diags.iter().any(|e| e.contains("not exported")),
        "{diags:?}"
    );
    let diags = errors(&[(
        "src/main.is",
        "(@ macro (= expand (-> (, x) _ x)))\n(= expand 1)\n",
    )]);
    assert!(
        diags.iter().any(|e| e.contains("defined more than once")),
        "{diags:?}"
    );
}

#[test]
fn invalid_macro_definitions_are_reported() {
    for source in ["@macro expand = (x) { x }", "@macro expand = 1"] {
        assert!(!errors(&[("src/main.i", source)]).is_empty());
    }
    assert!(!errors(&[("src/main.is", "(@ macro (= expand 1))")]).is_empty());
    let diags = errors(&[("src/main.is", "(@ main (@ macro (= expand (-> (,) _ `x))))")]);
    assert!(
        diags.iter().any(|e| e.contains("cannot be put on a macro")),
        "{diags:?}"
    );
    let diags = errors(&[(
        "src/main.is",
        "(@ bogus (@ macro (= expand (-> (,) _ `x))))",
    )]);
    assert!(diags.iter().any(|e| e.contains("@bogus")), "{diags:?}");
}
