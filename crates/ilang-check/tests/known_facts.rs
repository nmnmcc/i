//! F1: hidden-input hints cannot remove possible runtime behavior.
mod common;

use ilang_package::Diagnostic;

fn check(source: &str) -> Vec<Diagnostic> {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("i.toml"),
        format!(
            "name = \"app\"\n[dependencies]\nstd = {{ path = {:?} }}\n",
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std")
        ),
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.i"), source).unwrap();
    let (graph, ds) = ilang_package::load(dir.path());
    assert!(ds.is_empty(), "{ds:?}");
    ilang_check::check(&graph).diags
}

#[test]
fn hidden_inputs_cannot_skip_the_branch_that_runs() {
    for (name, source) in [
        (
            "service",
            include_str!("../../../tests/spec/4.8/f1_hidden_svc_sel.i"),
        ),
        (
            "callback parameter",
            include_str!("../../../tests/spec/4.8/f1_hidden_cb_param.i"),
        ),
        ("cell", include_str!("../../../tests/spec/4.8/f1_ref_sel.i")),
        (
            "later caller",
            include_str!("../../../tests/spec/4.8/f1_hidden_ret_sel.i"),
        ),
        (
            "later caller alias",
            include_str!("../../../tests/spec/4.8/f1_hidden_ret_sel2.i"),
        ),
        (
            "callback result",
            include_str!("../../../tests/spec/4.8/f1_hidden_cb_sel.i"),
        ),
    ] {
        let ds = check(source);
        assert!(ds.iter().any(|d| d.code == "E0501"), "{name}: {ds:?}");
    }
}

#[test]
fn hidden_inputs_do_not_justify_disjoint_equality_warnings() {
    let ds = check(
        r#"
repeat = <S>(start: S, step: (S) -> S) -> S { step(step(start)) }
f = () {
    result = repeat(0, (acc) {
        alias = acc
        _ = alias == "s"
        "s"
    })
    result == "s"
}
_ = f
"#,
    );
    assert!(
        ds.iter().all(|d| !d.is_error() && d.code != "W0503"),
        "{ds:?}"
    );
}

#[test]
fn access_hints_preserve_accumulator_field_inference() {
    let ds = check(
        r#"
(list = std/list)
Box(A) = (value = A)
f = () { list.fold([1, 2], (n = 0), (acc, x) { (n = acc.n + x) }).n }
g = () { list.fold([1, 2], (0, 0), (acc, x) { (acc.0 + x, acc.1) }).0 }
h = () { list.fold([1, 2], Box(value = 0), (acc, x) { Box(value = acc.value + x) }).value }
_ = (f, g, h)
"#,
    );
    assert!(ds.iter().all(|d| !d.is_error()), "{ds:?}");
}

#[test]
fn an_imported_weak_value_does_not_take_facts_from_an_earlier_caller() {
    let (_, _, ds) = common::check_files(&[
        ("i.toml", "name = \"probe\"\n"),
        (
            "src/lib.i",
            include_str!("../../../tests/spec/4.8/f1_imported_weak/src/lib.i"),
        ),
        (
            "src/main.i",
            include_str!("../../../tests/spec/4.8/f1_imported_weak/src/main.i"),
        ),
    ]);
    assert!(ds.iter().any(|d| d.code == "E0501"), "{ds:?}");
}
