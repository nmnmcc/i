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
    // §4.4: “When the parameter does not change, Ai and Bi must be equal”.
    // An invariant service still retains a hidden same-form obligation.
    let source = "Svc(A) = (A, (A) -> Nil)
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
fn closed_row_aliases_merge_by_the_defined_direction() {
    // §4.4: “when they have the same one, it is that type constructor
    // with each pair of arguments put together the other way round”.
    // A row parameter follows the same rule as any other parameter.
    let source = "Dep = ()
S(N) = (() <- N -> Nil)
T(N) = (() <- N -> Nil)
Bad = T(S(Never) | S(Dep))
use = (x: Bad) { x }";
    assert_eq!(
        types(source),
        types(&source.replace("S(Never) | S(Dep)", "S(Never)"))
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
wrap: <N>(
	() <- (Svc(Extra | Dep), ..N) -> Nil,
	Svc(Dep | Extra),
) <- N -> Nil = (body, svc) { body() <- (svc) }";
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
wrap: <N>(() <- (Svc(Int), ..N) -> Int, Svc(Int)) <- N -> Int = (body, svc) {
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
wrap: <N>(() <- (S(Never), ..N) -> Nil, S(Never)) <- N -> Nil = (body, svc) {
	body() <- (svc)
}";
    types(never);
}

#[test]
fn written_row_algebra_keeps_its_mode_before_normalization() {
    // §4.4: “When the parameter does not change, Ai and Bi must be equal”.
    // Keep the original algebra/alias coverage with an invariant row
    // parameter; directed parameters now follow ordinary intersection.
    let head = "Dep = ()
S(N) = (() <- N -> Nil)
T(N) = (() <- N -> Nil)
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
    let alias = "Dep = ()
S(N) = (() <- N -> Nil)
Bad = S(Never) | S(Dep)
use = (body: () <- (Bad) -> Nil) { body() }";
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
    assert_eq!(same["app/main.merge"], "(a: T(A), b: T(B)) -> T(A | B)");
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

#[test]
fn a_directed_row_handler_keeps_the_callback_row_open() {
    // §4.4: the need is an intersection; §4.5 rule 6 also determines
    // the direction of N. Handling S(N) must not put S(N) inside N.
    let shown = types(
        "S(N) = (f = () <- N -> Int)
use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}
wrap = <N>(svc: S(N)) -> Int { use() <- (svc) }",
    );
    assert_eq!(shown["app/main.wrap"], "(svc: S(A)) -> Int ! A");
}

#[test]
fn a_function_union_shows_the_intersection_of_its_service_needs() {
    // §4.4 puts function environments together by the need merge of §9.2.
    let shown = types(
        "Yield(A) = ((A) -> Nil)
one = () <- (s: Yield(Int)) { Nil }
two = () <- (s: Yield(String)) { Nil }
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
        "Cell(I, A) = (state = I, put = (I) -> Nil, value = A)
one = () <- (s: Cell(Int, Int)) { Nil }
two = () <- (s: Cell(Int, 1)) { Nil }
both = () {
	one()
	two()
}",
    );
    assert_eq!(shown["app/main.both"], "() -> Nil ! Cell(Int, 1)");
}

#[test]
fn an_undetermined_need_argument_keeps_its_intersection() {
    // §4.4: “a & T, with a type variable a, stays as it is.” The service
    // meet must not constrain the ordinary value parameter A to Int.
    let shown = types(
        "Read(A) = (value = A)
need_int = () <- (r: Read(Int)) { Nil }
both = <A>(x: A) <- (r: Read(A)) {
	need_int()
	x
}",
    );
    assert_eq!(shown["app/main.both"], "(x: A) -> A ! Read(Int & A)");
}

#[test]
fn closed_constructor_arguments_still_report_an_empty_need_meet() {
    let source = "S(A) = (value = A)
one = () <- (s: S(Int)) { s.value + 1 }
run = <N>(f: () <- N -> Int, x: N) { f() }
bad = () { run(() { one() }, S(value = \"hi\")) }";
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
    let checked = ilang_check::check(&graph);
    assert!(
        checked.diags.iter().any(|d| d.code == "E0515"
            && d.message.contains("S(Int)")
            && d.message.contains("S(\"hi\")")),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    types(&source.replace("\"hi\"", "1"));
}

#[test]
fn inferred_handlers_keep_future_provider_constraints() {
    // §9.2: a provided value must be a subtype of the needed service.
    // Generalizing an inferred row must retain that obligation for callers.
    let source = "S(A) = (value = A)
read = () <- (s: S(String)) { s.value <> \"!\" }
with = <A>(x: A, f) { f() <- (S(value = x)) }
main = () {
	with(1, read)
	Nil
}";
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
    let checked = ilang_check::check(&graph);
    assert!(
        checked.diags.iter().any(|d| d.code == "E0515"),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    types(&source.replace("with(1, read)", "with(\"ok\", read)"));
}

#[test]
fn higher_order_entry_keeps_unprovided_service() {
    let shown = types(
        "S(N) = (() <- N -> Nil)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
run = <A, N>(f: () <- N -> A) { f() }
main = <N>() <- (s: S(N)) { run(() { use() }) }",
    );
    assert!(shown["app/main.main"].contains("S("), "{shown:?}");
}

#[test]
fn nested_forwarding_keeps_the_provided_services_own_need() {
    // §9.2: providing S(Dep) handles S, not the Dep needed by its method.
    let shown = types(
        "Dep = (v = Int)
get = () <- (d: Dep) { d.v }
S(N) = (f = () <- N -> Int)
use = <N>() <- (s: S(N)) { s.f() }
run = <A, N>(f: () <- N -> A) { f() }
fw = () { use() }
fr = () { run(() { use() }) }
main = () {
	{
		h = () { run(fr) }
		k = () { run(h) + fw() }
		k()
		Nil
	} <- (S(f = () { get() }))
}",
    );
    assert!(shown["app/main.main"].contains("Dep"), "{shown:?}");
}

#[test]
fn direct_forwarding_keeps_the_provided_services_own_need() {
    let shown = types(
        "Dep = (v = Int)
get = () <- (d: Dep) { d.v }
S(N) = (f = () <- N -> Int)
use = <N>() <- (s: S(N)) { s.f() }
run = <A, N>(f: () <- N -> A) { f() }
fw = () { use() }
fr = () { run(() { use() }) }
main = () {
	{
		fw() + fr()
		Nil
	} <- (S(f = () { get() }))
}",
    );
    assert!(shown["app/main.main"].contains("Dep"), "{shown:?}");
}

#[test]
fn annotated_mutual_forwarding_keeps_finite_service_rows() {
    types(
        "S(N) = (() <- N -> Int)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
ping = <N>(n: Int) <- (s: S(N)) -> Int {
	n |> (0) { use() } | (_) { pong(n - 1) }
}
pong = (n: Int) { ping(n) }",
    );
}

#[test]
fn a_value_argument_cannot_erase_a_shared_row_constraint() {
    let source = "S(A) = (value = A)
one = () <- (s: S(Int)) { s.value + 1 }
run = <N>(f: () <- N -> Int, x: N) { f() }
bad = () { run(() { one() }, S(value = \"hi\")) <- (S(value = 1)) }";
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
    let checked = ilang_check::check(&graph);
    assert!(
        checked.diags.iter().any(|d| d.code == "E0515"
            && d.source
                .is_some_and(|id| graph.sources.get(id).text == source)),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    types(&source.replace("\"hi\"", "2"));
    types(
        &source
            .replace(
                "one = () <- (s: S(Int)) { s.value + 1 }",
                "one = () <- (s: S(Int | String)) { 1 }",
            )
            .replace("\"hi\"", "2"),
    );
}
