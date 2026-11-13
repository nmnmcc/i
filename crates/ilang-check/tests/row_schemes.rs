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
    // §4.4: “When the parameter does not change, Ai and Bi must be equal”.
    // An invariant service still retains a hidden same-form obligation.
    let source = "Svc(A) { Svc(A, (A) -> Nil) }
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
fn closed_row_aliases_merge_by_the_defined_direction() {
    // §4.4: “when they have the same one, it is that type constructor
    // with each pair of arguments put together the other way round”.
    // A row parameter follows the same rule as any other parameter.
    let source = "Dep { Dep }
S(N) { S(() -> Nil ! N) }
T(N) { T(() -> Nil ! N) }
Bad = T(S(Never) | S(Dep))
use = (x: Bad) { x }";
    assert_eq!(
        types(source),
        types(&source.replace("S(Never) | S(Dep)", "S(Never)"))
    );
    // Variance still applies to the row as a whole, as F2 explicitly says.
    types(
        "Other { Other }
S(A) { S(A) }
T(N) { T(() -> Nil ! N) }
widen = (x: T(S(Int))) -> T(S(Int) | Other) { x }",
    );
}

#[test]
fn equivalent_nested_row_orders_do_not_change_handler_acceptance() {
    // §4.4 defines unions as sets. F1's proof must use type equality,
    // including alias equality and union order, rather than source text.
    let base = "Dep { Dep }
Extra { Extra }
Svc(R) { Svc(() -> Nil ! R) }
wrap(N) = (body: () -> Nil ! N | Svc(Extra | Dep), svc: Svc(Dep | Extra)) -> Nil ! N {
 ! Svc(Extra | Dep) = svc
 body()
}";
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
        let source = "Svc(A) { Svc(() -> A) }
Row(N) = () -> Nil ! N
bad(N) = (body: Row(N), one: Svc(Int), two: Svc(Nil)) {
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
    let source = "Svc(A) { Svc(f: () -> A) }
wrap(N) = (body: () -> Int ! N | Svc(Int), svc: Svc(Int)) -> Int ! N {
 ! Svc(Int) = svc
 body()
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
    let never = "S(N) { S(() -> Nil ! N) }
use = (s ! S(Never)) { S(f) = s f() }
wrap(N) = (body: () -> Nil ! N | S(Never), svc: S(Never)) -> Nil ! N {
 ! S(_) = svc
 body()
}";
    types(never);
}

#[test]
fn written_row_algebra_keeps_its_mode_before_normalization() {
    // §4.4: “When the parameter does not change, Ai and Bi must be equal”.
    // Keep the original algebra/alias coverage with an invariant row
    // parameter; directed parameters now follow ordinary intersection.
    let head = "Dep { Dep }
S(N) { S(() -> Nil ! N) }
T(N) { T(() -> Nil ! N) }
Alias(N) = T(N)
";
    let invariant_head = head.replace("S(() -> Nil ! N)", "S(() -> Nil ! N, (N) -> Nil)");
    for op in ["|", "&", "-"] {
        let row = ["S(Never)", op, "S(Dep)"].join(" ");
        for written in [
            format!("T({row})"),
            format!("Alias({row})"),
            format!("() -> Nil ! {row}"),
            format!("() -> Nil ^ {row}"),
        ] {
            let source = format!(
                "{invariant_head}use = (x: {written}) {{ x }}
"
            );
            let (graph, _, _) = common::check_files(&[("src/main.i", &source)]);
            let checked = ilang_check::check(&graph);
            let expected = if op == "-" || written.contains('^') {
                "E0515"
            } else {
                "E0503"
            };
            assert!(
                checked.diags.iter().any(|d| d.code == expected),
                "{source}: {:?}",
                graph.sources.render_all(&checked.diags)
            );
        }
    }
    let alias = "Dep { Dep }
S(N) { S(() -> Nil ! N) }
Bad = S(Never) | S(Dep)
use = (body: () -> Nil ! Bad) { body() }";
    assert_eq!(
        types(alias),
        types(&alias.replace("S(Never) | S(Dep)", "S(Never)"))
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
    let source = "S(A) { S(A) }
T(N) { T(f: () -> Nil ! N) }
merge(N, M) = (a: T(N), b: T(M)) { T(() { a.f() b.f() }) }
one = (s ! S(Int)) { Nil }
two = (s ! S(String)) { Nil }

main = () { unused = merge(T(one), T(two)) Nil }";
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
    let checked = ilang_check::check(&graph);
    assert!(
        checked.diags.iter().any(|d| d.code == "E0515"),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    let same = types(&source.replace("String", "Int"));
    assert_eq!(same["app/main.merge"], "(a: T(A), b: T(B)) -> T(A | B)");
}

#[test]
fn function_type_joins_preserve_scalar_error_inclusion() {
    // Rule 8 still governs scalar error value sets when function types
    // are joined; the row mode must reach merge_arg as well as calls.
    types(
        "(i/fail)
one = () -> Nil ^ 1 { fail.raise(1) }
two = () -> Nil ^ 2 { fail.raise(2) }
pick = (b: Bool) { b |> { True -> one False -> two } }",
    );
}

#[test]
fn a_directed_row_handler_keeps_the_callback_row_open() {
    // §4.4: the need is an intersection; §4.5 rule 6 also determines
    // the direction of N. Handling S(N) must not put S(N) inside N.
    let shown = types(
        "S(N) { S(f: () -> Int ! N) }
use(N) = (s ! S(N)) { S(f = f, ..) = s f() }
wrap(N) = (svc: S(N)) -> Int ! N { ! S(N) = svc use() }",
    );
    assert_eq!(shown["app/main.wrap"], "(svc: S(A)) -> Int ! A");
}

#[test]
fn a_function_union_shows_the_intersection_of_its_service_needs() {
    // §4.4 puts function environments together by the need merge of §9.2.
    let shown = types(
        "Yield(A) { Yield((A) -> Nil) }
one = (s ! Yield(Int)) { Nil }
two = (s ! Yield(String)) { Nil }
fns = [one, two]",
    );
    assert_eq!(
        shown["app/main.fns"],
        "List(() -> Nil ! Yield(Int | String))"
    );
}

#[test]
fn equal_invariant_arguments_do_not_block_other_argument_meets() {
    // §4.4: invariant arguments must be equal; the remaining arguments
    // still follow their defined directions in the same service entry.
    let shown = types(
        "Cell(I, A) { Cell(state: I, put: (I) -> Nil, value: A) }
one = (s ! Cell(Int, Int)) { Nil }
two = (s ! Cell(Int, 1)) { Nil }
both = () { one() two() }",
    );
    assert_eq!(shown["app/main.both"], "() -> Nil ! Cell(Int, 1)");
}
