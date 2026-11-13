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
        "S(N) = (f = () <- N -> Int)",
        "T(N) = (f = () <- N -> Int)",
        "us = <N>() <- (s: S(N)) { s.f() }",
        "ut = <N>() <- (t: T(N)) { t.f() }",
        "f = (n: Int) { n |> (0) { us() } | (_) { g(n - 1) } }",
        "g = (n: Int) { n |> (0) { ut() } | (_) { f(n - 1) } }",
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
    let source = "Svc(A) = (A)
wrap = <N>(body: () <- N -> Int, svc: Svc(String)) -> Int { body() <- (svc) }";
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
        "Only = ()
wrap = <N>(body: () <- N -> Nil) -> Nil { body() <- (Only) }
example = () { wrap(() { Nil }) }",
    );
}

#[test]
fn closed_row_aliases_check_forms_before_merging_arguments() {
    // F2's rule applies before normalization can combine S's arguments.
    // Ordinary covariance must not turn these two service forms into one.
    let source = "Dep = ()
S(N) = (() <- N -> Nil)
T(N) = (() <- N -> Nil)
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
        "Other = ()
S(A) = (A)
T(N) = (() <- N -> Nil)
widen = (x: T(S(Int))) -> T(S(Int) | Other) { x }",
    );
}

#[test]
fn equivalent_nested_row_orders_do_not_change_handler_acceptance() {
    // §4.4 defines unions as sets. F1's proof must use type equality,
    // including alias equality and union order, rather than source text.
    let base = "Dep = ()
Extra = ()
Svc(R) = (() <- R -> Nil)
wrap = <N>(
	body: () <- (Svc(Extra | Dep), ..N) -> Nil,
	svc: Svc(Dep | Extra)
) -> Nil { body() <- (svc) }";
    let before = types(base);
    let after = types(&base.replace("! Svc(Extra | Dep) = svc", "! Svc(Dep | Extra) = svc"));
    assert_eq!(before, after);
    assert!(before["app/main.wrap"].contains("Svc(Dep | Extra)"));
}

#[test]
fn row_kind_survives_an_alias_without_algebra() {
    // F2 requires N to remain a row even when a function alias is copied
    // without normalization. Its value positions must compare Svc's
    // arguments equally before generalization can combine the forms.
    for mode in ["!", "^"] {
        let source = "Svc(A) = (() -> A)
Row(N) = () <- N -> Nil
bad = <N>(body: Row(N), one: Svc(Int), two: Svc(Nil)) {
	first: N = one
	second: N = two
	second
}"
        .replace("! N", &format!("{mode} N"));
        let (graph, _, _) = common::check_files(&[("src/main.i", &source)]);
        let checked = ilang_check::check(&graph);
        assert!(
            checked.diags.iter().any(|d| d.code == "E0515"),
            "{:?}",
            graph.sources.render_all(&checked.diags)
        );
    }
}

#[test]
fn handler_witnesses_include_exact_bounds_of_annotation_holes() {
    // §4.5: L <: v <: U proves v = T when L = T = U. A hole in a
    // handler fixed to Int must prove the same witness as writing Int.
    let source = "Svc(A) = (f = () -> A)
wrap = <N>(body: () <- (Svc(Int), ..N) -> Int, svc: Svc(Int)) -> Int {
	body() <- (svc)
}";
    for inferred in [false, true] {
        let source = if inferred {
            source.replace(") -> Int ! N {", ") {")
        } else {
            source.to_string()
        };
        assert_eq!(
            types(&source),
            types(&source.replace("! Svc(Int) = svc", "! Svc(_) = svc"))
        );
    }
    // Never has an implicit lower bound, so v <: Never also fixes v.
    let never = "S(N) = (() <- N -> Nil)
use = () <- (s: S(Never)) {
	S(f) = s
	f()
}
wrap = <N>(body: () <- (S(Never), ..N) -> Nil, svc: S(Never)) -> Nil {
	body() <- (svc)
}";
    types(never);
}

#[test]
fn written_row_algebra_keeps_its_mode_before_normalization() {
    // F2 requires equal service arguments also before any variables or
    // instantiation exist. Ordinary value variance must not merge them.
    let head = "Dep = ()
S(N) = (() <- N -> Nil)
T(N) = (() <- N -> Nil)
Alias(N) = T(N)
";
    for op in ["|", "&", "-"] {
        let row = ["S(Never)", op, "S(Dep)"].join(" ");
        for written in [
            format!("T({row})"),
            format!("Alias({row})"),
            // The old `() -> Nil ! {row}` has no syntax-three form: a need
            // list holds named service types only (syntax-3 §3.1).
            format!("() <- (Fail({row})) -> Nil"),
        ] {
            let source = format!(
                "{head}use = (x: {written}) {{ x }}
"
            );
            let (graph, _, _) = common::check_files(&[("src/main.i", &source)]);
            let checked = ilang_check::check(&graph);
            assert!(
                checked.diags.iter().any(|d| d.code == "E0515"),
                "{source}: {:?}",
                graph.sources.render_all(&checked.diags)
            );
        }
    }
    let alias = "Dep = ()
S(N) = (() <- N -> Nil)
Bad = S(Never) | S(Dep)
use = (body: () <- (Bad) -> Nil) { body() }";
    let (graph, _, _) = common::check_files(&[("src/main.i", alias)]);
    let checked = ilang_check::check(&graph);
    assert!(
        checked.diags.iter().any(|d| d.code == "E0515"),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    // Value unions keep ordinary variance; distinct row constructors may
    // also coexist. Neither is a same-form collision under rule 4.
    types(&format!(
        "{head}value = (x: S(Never) | S(Dep)) {{ x }}
"
    ));
    types(&format!(
        "{head}value = (x: T(S(Never) | Dep)) {{ x }}
"
    ));
}

#[test]
fn instantiated_output_rows_are_checked_even_when_unused() {
    // §9.2 forbids two S forms in the returned callback's environment,
    // whether it is invoked or left unused. §4.8 unions the two rows.
    let source = "S(A) = (A)
T(N) = (f = () <- N -> Nil)
merge = <N, M>(a: T(N), b: T(M)) {
	T(
		f = () {
			a.f()
			b.f()
		},
	)
}
one = () <- (s: S(Int)) { Nil }
two = () <- (s: S(String)) { Nil }

main = () {
	unused = merge(T(f = one), T(f = two))
	Nil
}";
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
    let checked = ilang_check::check(&graph);
    assert!(
        checked.diags.iter().any(|d| d.code == "E0515"),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    let same = types(&source.replace("String", "Int"));
    assert_eq!(same["app/main.merge"], "(T(A), T(B)) -> T(A | B)");
}

#[test]
fn function_type_joins_preserve_scalar_error_inclusion() {
    // Rule 8 still governs scalar error value sets when function types
    // are joined; the row mode must reach merge_arg as well as calls.
    types(
        "(
	i/fail
)
one = () -> Nil { fail.raise(1) }
two = () -> Nil { fail.raise(2) }
pick = (b: Bool) { b |> (True) { one } | (False) { two } }",
    );
}
