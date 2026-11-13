//! Row schemes preserve obligations, identity, and declaration independence.
mod common;

fn types(source: &str) -> std::collections::BTreeMap<String, String> {
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
    let checked = ilang_check::check(&graph);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{source}\n{:?}",
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
wrap: <N>(() <- (Fail(Never), ..N) -> Int, Svc(String)) <- (Fail(Never), ..N) -> Int = (body, svc) { body() <- (svc) }";
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
) <- N -> Nil = (body, svc) {
    provided: Svc(Extra | Dep) = svc
    body() <- (provided)
}";
    let before = types(base);
    let reordered = base.replace("provided: Svc(Extra | Dep)", "provided: Svc(Dep | Extra)");
    assert_ne!(base, reordered);
    let after = types(&reordered);
    assert_eq!(before, after);
    assert!(before["app/main.wrap"].contains("Svc(Dep | Extra)"));
}

#[test]
fn row_kind_survives_an_alias_without_algebra() {
    // F2 requires N to remain a row even when a function alias is copied
    // without normalization. Its value positions must compare Svc's
    // arguments equally before generalization can combine the forms.
    for need in ["(Fail(Never), ..N)", "(Fail(N))"] {
        let source = "Svc(A) = (() -> A)
Row(N) = () <- NEED -> Nil
bad = <N>(body: Row(N), one: Svc(Int), two: Svc(Nil)) {
	first: N = one
	second: N = two
	second
}"
        .replace("NEED", need);
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
    // §4.5: both bounds fix the handler hole to the same form.
    for inferred in [false, true] {
        let head = if inferred {
            "wrap = <N>(body: () <- (Fail(Never), Svc(Int), ..N) -> Int, svc: Svc(Int))"
        } else {
            "wrap: <N>(() <- (Fail(Never), Svc(Int), ..N) -> Int, Svc(Int)) <- (Fail(Never), ..N) -> Int = (body, svc)"
        };
        let source = format!(
            "Svc(A) = (f = () -> A)\n{head} {{\n provided: Svc(Int) = svc\n body() <- (provided)\n}}\n"
        );
        let with_hole = source.replace("provided: Svc(Int)", "provided: Svc(_)");
        assert_ne!(source, with_hole);
        assert_eq!(types(&source), types(&with_hole));
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
    let invariant_head = head.replace(
        "S(N) = (() <- N -> Nil)",
        "S(N) = (() <- N -> Nil, (N) -> Nil)",
    );
    for op in ["|", "&", "-"] {
        let row = ["S(Never)", op, "S(Dep)"].join(" ");
        for (written, error_row) in [
            (format!("T({row})"), false),
            (format!("Alias({row})"), false),
            ("() <- (Fail(Never), ..Written) -> Nil".to_string(), false),
            (format!("() <- (Fail({row})) -> Nil"), true),
        ] {
            let source = format!(
                "{invariant_head}Written = {row}\nuse = (x: {written}) {{ x }}
"
            );
            let (graph, _, _) = common::check_files(&[("src/main.i", &source)]);
            let checked = ilang_check::check(&graph);
            let expected = if op == "-" || error_row {
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
Bad = { S(Never) | S(Dep) }
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
wrap: <N>(S(N)) <- (Fail(Never), ..N) -> Int = (svc) { use() <- (svc) }",
    );
    assert_eq!(shown["app/main.wrap"], "(S(A)) <- A -> Int");
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
        "List(() <- (Yield(Int | String)) -> Nil)"
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
    assert_eq!(shown["app/main.both"], "() <- (Cell(Int, 1)) -> Nil");
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
    assert_eq!(shown["app/main.both"], "(A) <- (Read(Int & A)) -> A");
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
with = <A>(x: A, f) { provided: S(A) = S(value = x) f() <- (provided) }
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

#[test]
fn syntax_three_provision_retains_the_actual_provider_form() {
    let source = "Read(A) = (get = () -> A)\nwith_int = (body) { service: Read(Int) = Read(get = () -> Int { 1 }) body() <- (service) }\nmain = () { with_int(() <- (r: Read(Int)) { r.get() }) }\n";
    let shown = types(source);
    assert!(!shown["app/main.main"].contains("Read"), "{shown:?}");
    let bad = source.replace(
        "() <- (r: Read(Int)) { r.get() }",
        "() <- (r: Read(String)) { r.get() }",
    );
    let (graph, _, _) = common::check_files(&[("src/main.i", &bad)]);
    let checked = ilang_check::check(&graph);
    assert!(
        checked.diags.iter().any(|d| d.code == "E0515"),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    assert!(
        !checked.diags.iter().any(|d| d.code == "E0501"),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn converted_e1_provider_cases_keep_batch2_check_outcomes() {
    let cases: &[(&str, &[&str], &str)] = &[
        (
            "review-k4/cases/closure_inner_annotation",
            &["E0515"],
            include_str!(
                "../../../tests/review/cases/review-k4/cases/closure_inner_annotation/src/main.i"
            ),
        ),
        (
            "review-k4/cases/local_instances_annotated",
            &["E0515"],
            include_str!(
                "../../../tests/review/cases/review-k4/cases/local_instances_annotated/src/main.i"
            ),
        ),
        (
            "review-k4/cases/provision_closure_True",
            &[],
            include_str!(
                "../../../tests/review/cases/review-k4/cases/provision_closure_True/src/main.i"
            ),
        ),
        (
            "review-k4/cases/provision_direct_True",
            &[],
            include_str!(
                "../../../tests/review/cases/review-k4/cases/provision_direct_True/src/main.i"
            ),
        ),
        (
            "review-k4/cases/provision_helper_True",
            &[],
            include_str!(
                "../../../tests/review/cases/review-k4/cases/provision_helper_True/src/main.i"
            ),
        ),
        (
            "review-k4/cases/provision_inferred_True",
            &[],
            include_str!(
                "../../../tests/review/cases/review-k4/cases/provision_inferred_True/src/main.i"
            ),
        ),
        (
            "review-k4/cases/recursive_env_cov",
            &["E0505"],
            include_str!(
                "../../../tests/review/cases/review-k4/cases/recursive_env_cov/src/main.i"
            ),
        ),
        (
            "review-k4/cases/recursive_env_mutual",
            &["E0505"],
            include_str!(
                "../../../tests/review/cases/review-k4/cases/recursive_env_mutual/src/main.i"
            ),
        ),
    ];
    let mut failures = Vec::new();
    for (name, expected, source) in cases {
        let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
        let checked = ilang_check::check(&graph);
        let actual: std::collections::BTreeSet<_> = checked
            .diags
            .iter()
            .filter(|d| d.is_error())
            .map(|d| d.code)
            .collect();
        let expected: std::collections::BTreeSet<_> = expected.iter().copied().collect();
        if actual != expected {
            failures.push(format!(
                "{name}: expected {expected:?}, got {actual:?}: {:?}",
                graph.sources.render_all(&checked.diags)
            ));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn syntax_three_library_handler_has_no_unhandled_service() {
    let (graph, _, _) = common::check_files(&[(
        "src/main.i",
        "(i/failure)\nmain = () { failure.catch(() { failure.raise(1) }) }\n",
    )]);
    let checked = ilang_check::check(&graph);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    let p = checked.program.as_ref().unwrap();
    let mut found = false;
    let typed = checked.types.as_ref().unwrap();
    for (mi, module) in p.modules.iter().enumerate() {
        if module.path != "i/failure" {
            continue;
        }
        for (di, def) in module.defs.iter().enumerate() {
            if def.name != "catch" {
                continue;
            }
            let id = ilang_check::hir::DefId {
                module: ilang_check::hir::ModuleId(mi as u32),
                index: di as u32,
            };
            found = true;
            let shown = &typed.scheme_types[&id];
            let ilang_check::types::ShownType::Fn(function) = shown else {
                panic!("{shown:#?}");
            };
            assert!(function.err.is_none(), "{shown:#?}");
            assert!(
                matches!(
                    function.env.as_ref(),
                    Some(ilang_check::types::ShownType::Var(_))
                ),
                "{shown:#?}"
            );
        }
    }
    assert!(found, "the library handler was checked");
}

#[test]
fn syntax_three_written_provider_hole_keeps_its_obligation() {
    let base = "S(N) = (f = () <- N -> Int)\nuse = <N>() <- (s: S(N)) { s.f() }\n";
    let positive = [
        base,
        "wrap = <N>(body, svc: S(N)) { body() <- (svc) }\nmain = () { assert(wrap(() { use() }, S(f = () -> Int { 1 })) == 1) }\n",
    ].concat();
    types(&positive);
    let negative = [
        base,
        "wrap = (body) { provided: S(_) = S(f = () -> Int { 1 }) body() <- (provided) }\n",
    ]
    .concat();
    let (graph, _, _) = common::check_files(&[("src/main.i", &negative)]);
    let checked = ilang_check::check(&graph);
    let codes: std::collections::BTreeSet<_> = checked
        .diags
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.code)
        .collect();
    assert_eq!(
        codes,
        ["E0515"].into_iter().collect(),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn converted_p1_keeps_the_uniform_provider_error() {
    for source in [
        include_str!("../../../tests/review/cases/review-k4d/_loose/p/p1/src/main.i"),
        include_str!("../../../tests/review/cases/review-k4d/proj/p1/src/main.i"),
    ] {
        let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
        let checked = ilang_check::check(&graph);
        let errors: Vec<_> = checked.diags.iter().filter(|d| d.is_error()).collect();
        assert_eq!(
            errors.len(),
            1,
            "{:?}",
            graph.sources.render_all(&checked.diags)
        );
        assert_eq!(errors[0].code, "E0515");
        assert!(errors[0].message.contains("S("));
    }
}

#[test]
fn a_closed_provider_retains_callback_and_generalization_diagnostics() {
    // Keep every original Reader obligation, including the two checked
    // when generalizing g and k, without changing the soundness golden.
    let source = include_str!("types/soundness.i");
    let (graph, _, diags) = common::check_files(&[("src/main.i", source)]);
    let actual = diags
        .iter()
        .filter(|d| d.code == "E0515" && d.message.contains("`Reader(String)` and `Reader(Int)`"))
        .map(|d| ilang_syntax::span::LineIndex::new(source).line_col(d.span.start))
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        [(14, 19), (14, 5), (16, 29), (16, 5)],
        "{:?}",
        graph.sources.render_all(&diags)
    );
}
