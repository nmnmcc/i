//! Row schemes preserve obligations, identity, and declaration independence.
mod common;

fn types(source: &str) -> std::collections::BTreeMap<String, String> {
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
    let checked = ilang_check::check(&graph);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    let p = checked.program.unwrap();
    checked
        .types
        .unwrap()
        .lines(&p, common::module(&p, "app/main"))
        .into_iter()
        .map(|s| {
            let (n, t) = s.split_once(" : ").unwrap();
            (n.to_string(), t.to_string())
        })
        .collect()
}

#[test]
fn mutual_service_rows_have_canonical_member_and_variable_order() {
    // §4.7 gives a group's bindings their principal schemes together;
    // §4.4 makes union commutative. Declaration order changes neither.
    let defs = [
        "S(N) { S(f: () -> Int ! N) }",
        "T(N) { T(f: () -> Int ! N) }",
        "us(N) = (s ! S(N)) { s.f() }",
        "ut(N) = (t ! T(N)) { t.f() }",
        "f = (n: Int) { n |> { 0 -> us() _ -> g(n - 1) } }",
        "g = (n: Int) { n |> { 0 -> ut() _ -> f(n - 1) } }",
    ];
    let expected = types(&defs.join("\n"));
    for order in [[1, 0, 3, 2, 5, 4], [0, 1, 3, 2, 4, 5], [1, 0, 2, 3, 5, 4]] {
        let source = order.map(|i| defs[i]).join("\n");
        assert_eq!(types(&source), expected);
    }
}

#[test]
fn a_hidden_handler_obligation_is_rejected_at_the_definition() {
    // F1 ruling: an annotation may omit a difference only when its
    // same-form obligation holds for every instance of the shown scheme.
    let source = "Svc(A) { Svc(A) }
wrap(N) = (body: () -> Int ! N, svc: Svc(String)) -> Int ! N {
 ! Svc(String) = svc
 body()
}";
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
    let checked = ilang_check::check(&graph);
    assert!(
        checked
            .diags
            .iter()
            .any(|d| d.code == "E0515" && d.message.contains("Svc")),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn a_constructor_with_one_form_has_no_hidden_form_restriction() {
    // §9.2: a constructor without parameters has only one service form.
    // Omitting its subtraction cannot hide an argument equality.
    types(
        "Only { Only }
wrap(N) = (body: () -> Nil ! N) -> Nil ! N { ! Only = Only body() }
example = () { wrap(() { Nil }) }",
    );
}

#[test]
fn closed_row_aliases_check_forms_before_merging_arguments() {
    // F2's rule applies before normalization can combine S's arguments.
    // Ordinary covariance must not turn these two service forms into one.
    let source = "Dep { Dep }
S(N) { S(() -> Nil ! N) }
T(N) { T(() -> Nil ! N) }
Bad = T(S(Never) | S(Dep))
use = (x: Bad) { x }";
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
    let checked = ilang_check::check(&graph);
    assert!(
        checked.diags.iter().any(|d| d.code == "E0515"),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    // Variance still applies to the row as a whole, as F2 explicitly says.
    types(
        "Other { Other }
S(A) { S(A) }
T(N) { T(() -> Nil ! N) }
widen = (x: T(S(Int))) -> T(S(Int) | Other) { x }",
    );
}
