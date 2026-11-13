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
wrap: <N>(() <- N -> Int, Svc(String)) <- N -> Int = (body, svc) {
 body() <- (svc)
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
wrap = <N>(
	body: () <- (Svc(Extra | Dep), ..N) -> Nil,
	svc: Svc(Dep | Extra)
) -> Nil { body() <- (svc) }";
    let before = types(base);
    let after = types(&base.replace("Svc(Extra | Dep)", "Svc(Dep | Extra)"));
    assert_eq!(before, after);
    assert!(before["app/main.wrap"].contains("Svc(Dep | Extra)"));
}

#[test]
fn row_kind_survives_an_alias_without_algebra() {
    // N stays a need row when a function alias is copied without
    // normalization, whether alone or beside a named service. Errors
    // use an ordinary Fail argument now (§9.3), not a second row kind.
    for need in ["N", "(Dep, ..N)"] {
        let source = "Dep = ()
Svc(A) = (() -> A)
Row(N) = () <- N -> Nil
bad = <N>(body: Row(N), one: Svc(Int), two: Svc(Nil)) {
	first: N = one
	second: N = two
	second
}"
        .replace("() <- N -> Nil", &format!("() <- {need} -> Nil"));
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
    // invariant handler fixed to Int must prove the same witness as writing Int.
    let source = "Svc(A) = (f = (A) -> A)
wrap = <N>(body: () <- (Svc(Int), ..N) -> Int, svc: Svc(Int)) -> Int {
	body() <- (svc)
}";
    for inferred in [false, true] {
        let source = if inferred {
            source.replace(") -> Int {", ") {")
        } else {
            source.to_string()
        };
        assert_eq!(
            types(&source),
            types(&source.replace("svc: Svc(Int)", "svc: Svc(_)"))
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
        for written in [
            format!("T({row})"),
            format!("Alias({row})"),
            // The old `() -> Nil ! {row}` has no syntax-three form: a need
            // list holds named service types only (syntax-3 §3.1).
            format!("() <- (Fail({row})) -> Nil"),
        ] {
            let source = format!(
                "{invariant_head}use = (x: {written}) {{ x }}
"
            );
            let (graph, _, _) = common::check_files(&[("src/main.i", &source)]);
            let checked = ilang_check::check(&graph);
            // Fail's argument is an ordinary value type after E3. Its
            // invariant union/intersection error is E0503, as elsewhere;
            // the former separate error-row diagnostic no longer applies.
            let expected = if op == "-" && written.contains("Fail(") {
                // A difference in an ordinary error value requires value
                // subtyping (§4.4); this is no longer an error-row check.
                "E0501"
            } else if op == "-" {
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
use = <N>() <- (s: S(N)) { s.f() }
wrap = <N>(svc: S(N)) -> Int { use() <- (svc) }",
    );
    // The retained key restriction is shown explicitly; A stays open to
    // every other service. The retained obligation uses the actual provider
    // form S(A), as required by the directed P <: S check (§9.2).
    assert_eq!(shown["app/main.wrap"], "(S(A)) <- (A - S(A)) -> Int");
    types(
        "Dep = (value = Int)\nS(N) = (f = () <- N -> Int)\nuse = <N>() <- (s: S(N)) { s.f() }\nwrap = <N>(svc: S(N)) -> Int { use() <- (svc) }\nneed = () <- (d: Dep) { d.value }\nexample = () { wrap(S(f = () { need() })) <- (Dep(value = 1)) }\n",
    );
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
both = () { one() two() }",
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
both = <A>(x: A) <- (r: Read(A)) { need_int() x }",
    );
    assert_eq!(shown["app/main.both"], "(A) <- (Read(Int & A)) -> A");
}

#[test]
fn closed_constructor_arguments_still_report_an_empty_need_meet() {
    let source = "S(A) = (value = A)\none = () <- (s: S(Int)) { s.value + 1 }\nrun = <N>(f: () <- N -> Int, x: N) { f() }\nbad = () { run(() { one() }, S(value = \"hi\")) }\n";
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
    let source = "S(A) = (value = A)\nread = () <- (s: S(String)) { s.value <> \"!\" }\nwith = <A>(x: A, f) { f() <- (S(value = x)) }\nmain = () {\n\twith(1, read)\n\tNil\n}\n";
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
    let checked = ilang_check::check(&graph);
    assert!(
        checked.diags.iter().any(|d| d.code == "E0515"
            && d.message.contains("S(")
            && !d.message.contains("hides")
            && d.source
                .is_some_and(|id| graph.sources.get(id).text == source)),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    types(&source.replace("with(1, read)", "with(\"ok\", read)"));
}

#[test]
fn higher_order_entry_keeps_unprovided_service() {
    let shown = types(
        "S(N) = (() <- N -> Nil)\nuse = <N>() <- (s: S(N)) {\n\tS(f) = s\n\tf()\n}\nrun = <A, N>(f: () <- N -> A) { f() }\nmain = <N>() <- (s: S(N)) { run(() { use() }) }\n",
    );
    assert!(shown["app/main.main"].contains("S("), "{shown:?}");
}

#[test]
fn nested_forwarding_keeps_the_provided_services_own_need() {
    // §9.2: providing S(Dep) handles S, not the Dep needed by its method.
    let shown = types(
        "Dep = (v = Int)\nget = () <- (d: Dep) { d.v }\nS(N) = (f = () <- N -> Int)\nuse = <N>() <- (s: S(N)) { s.f() }\nrun = <A, N>(f: () <- N -> A) { f() }\nfw = () { use() }\nfr = () { run(() { use() }) }\nmain = () {\n\t{\n\t\th = () { run(fr) }\n\t\tk = () { run(h) + fw() }\n\t\tk()\n\t\tNil\n\t} <- (S(f = () { get() }))\n}\n",
    );
    assert!(shown["app/main.main"].contains("Dep"), "{shown:?}");
}

#[test]
fn direct_forwarding_keeps_the_provided_services_own_need() {
    let shown = types(
        "Dep = (v = Int)\nget = () <- (d: Dep) { d.v }\nS(N) = (f = () <- N -> Int)\nuse = <N>() <- (s: S(N)) { s.f() }\nrun = <A, N>(f: () <- N -> A) { f() }\nfw = () { use() }\nfr = () { run(() { use() }) }\nmain = () {\n\t{\n\t\tfw() + fr()\n\t\tNil\n\t} <- (S(f = () { get() }))\n}\n",
    );
    assert!(shown["app/main.main"].contains("Dep"), "{shown:?}");
}

#[test]
fn annotated_mutual_forwarding_keeps_finite_service_rows() {
    types(
        "S(N) = (() <- N -> Int)\nuse = <N>() <- (s: S(N)) {\n\tS(f) = s\n\tf()\n}\nping = <N>(n: Int) <- (s: S(N)) -> Int {\n\tn |> (0) { use() } | (_) { pong(n - 1) }\n}\npong = (n: Int) { ping(n) }\n",
    );
}

#[test]
fn a_value_argument_cannot_erase_a_shared_row_constraint() {
    let source = "S(A) = (value = A)\none = () <- (s: S(Int)) { s.value + 1 }\nrun = <N>(f: () <- N -> Int, x: N) { f() }\nbad = () { run(() { one() }, S(value = \"hi\")) <- (S(value = 1)) }\n";
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
                "one = () <- (s: S(Int)) { s.value + 1 }\n",
                "one = () <- (s: S(Int | String)) { 1 }\n",
            )
            .replace("\"hi\"", "2"),
    );
}

#[test]
fn catch_generalizes_its_stated_fail_entry_and_residual_row() {
    // Rule B: the callback's (..N, Fail(E)) states the restriction on N.
    // The scheme retains it; it is not an unexpressed provider obligation.
    let shown = types(
        "(i/fail)\nforward = <A, E, N>(body: () <- (Fail(E), ..N) -> A) -> Result(A, E) { fail.catch(body) }\n",
    );
    assert!(shown["app/main.forward"].contains("Fail("), "{shown:?}");
    assert!(shown["app/main.forward"].contains("Result("), "{shown:?}");
}

#[test]
fn catch_cannot_split_a_fail_entry_into_the_residual_row() {
    // §2.10.4 Q1-B: choosing E = Int and N = Fail(String) would allow
    // an Error(String) to escape with the claimed Result(Nil, Int) type.
    let source = "(i/fail)\nsplit: (() <- (Fail(Int | String)) -> Nil) <- (Fail(String)) -> Result(Nil, Int) = (body) { fail.catch(body) }\n";
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
    let checked = ilang_check::check(&graph);
    assert!(
        checked.diags.iter().any(|d| d.is_error()
            && d.source
                .is_some_and(|id| graph.sources.get(id).text == source)),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    types(&source.replace("Result(Nil, Int)", "Result(Nil, Int | String)"));
}

#[test]
fn a_directed_handler_restriction_without_a_stated_entry_is_hidden() {
    let source = "Svc(A) = (value = A)\nhide: <N>(() <- N -> Nil) <- N -> Nil = (body) { body() <- (Svc(value = 1)) }\n";
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
    let checked = ilang_check::check(&graph);
    assert!(
        checked.diags.iter().any(|d| d.code == "E0515"
            && d.message.contains("hides")
            && d.message.contains("Svc")
            && d.source
                .is_some_and(|id| graph.sources.get(id).text == source)),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn a_closed_callback_can_instantiate_an_unused_fail_entry_as_never() {
    // Written callback types are closed. A fresh E may be Never, but an
    // actual raised value must not disappear when fitting the callback.
    types(
        "unused = <E>() <- (handler: Fail(E)) { Nil }\nrun = (f: () -> Nil) { f() }\nmain = () { run(() { unused() }) }\n",
    );
    let source =
        "(i/fail)\nrun = (f: () -> Nil) { f() }\nmain = () { run(() { fail.raise(1) }) }\n";
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
    let checked = ilang_check::check(&graph);
    assert!(
        checked.diags.iter().any(|d| d.is_error()
            && d.source
                .is_some_and(|id| graph.sources.get(id).text == source)),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn a_sink_with_a_stored_callback_row_has_a_finite_instantiation() {
    // E3 stores Fail in the callback row. The factory is pure and its
    // callback raises String; neither row contains the sink itself.
    let source = r#"(i/fail)
Entry = (message = String)
Sink(E, N) = ((Entry) <- (Fail(E), ..N) -> Nil, () <- N -> Nil)
make = <E, N>(send: (Entry) <- (Fail(E), ..N) -> Nil) -> Sink(E, N) {
    Sink(send, () { Nil })
}
write = <E, N>(entry: Entry) <- (sink: Sink(E, N)) {
    Sink(send, _) = sink
    send(entry)
}
example = () {
    fail.catch(() { write(Entry(message = "seen")) }) <- (
        make((entry) { fail.raise(entry.message) }),
    )
}
"#;
    types(source);
    let with_dependency = source
        .replace(
            "Entry =",
            "Dep = (read = () -> Nil)\nread = () <- (d: Dep) { d.read() }\nEntry =",
        )
        .replace(
            "fail.raise(entry.message)",
            "read()\n        fail.raise(entry.message)",
        );
    types(&with_dependency);
}

#[test]
fn a_stored_service_row_retains_the_callers_dependency() {
    let source = r#"Dep = (read = () -> String)
Other = (read = () -> String)
S(N) = (run = () <- N -> String)
need = () <- (s: S(Dep)) { Nil }
wrong = () <- (o: Other) { "value" }
main = () { need() <- (S(run = wrong)) }
"#;
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
    let checked = ilang_check::check(&graph);
    assert!(
        checked.diags.iter().any(|d| d.is_error()
            && d.source
                .is_some_and(|id| graph.sources.get(id).text == source)),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    types(&source.replace("(o: Other)", "(o: Dep)"));
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
        "(i/fail)\nmain = () { fail.catch(() { fail.raise(1) }) }\n",
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
        if module.path != "i/fail" {
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
fn a_generator_does_not_export_its_handled_yield_need() {
    let shown = types(
        r#"(i/control)
Yield(A) = (emit = (A) -> Nil)
collect: <A, N>(() <- (Yield(A), ..N) -> Nil) <- N -> List(A) = (body) {
    control.boundary((b) {
        {
            body()
            []
        } <- (Yield(emit = (x) {
            control.suspend(b, (k) { [x, ..control.resume(k, Nil)] })
        }))
    })
}
main = () { collect(() <- (y: Yield(Int)) { y.emit(1) }) }
"#,
    );
    assert!(!shown["app/main.main"].contains("Yield"), "{shown:?}");
    assert!(shown["app/main.main"].contains("List(Int)"), "{shown:?}");
}

#[test]
fn a_covariant_handler_hole_retains_its_upper_bound() {
    // P <: S gives A <: Int for a covariant provider, not A = Int.
    let shown = types(
        "Svc(A) = (f = () -> A)\nwrap = <N>(body: () <- (Svc(Int), ..N) -> Int, svc: Svc(_)) -> Int { body() <- (svc) }\n",
    );
    assert_eq!(
        shown["app/main.wrap"],
        "(() <- (Svc(Int), ..N) -> Int, Svc(A & Int)) <- (N - Svc(A & Int)) -> Int"
    );
}

#[test]
fn a_filtered_callback_vacates_only_an_unused_fail_need() {
    let source = "(i/fail)\nNoError(N) = N - Fail(Never)\nrun = <N>(f: () <- (NoError(N)) -> Nil) { f() }\npure = <E>() <- (unused: Fail(E)) { Nil }\nmain = () { run(pure) }\n";
    let shown = types(source);
    assert!(!shown["app/main.main"].contains("Fail"), "{shown:?}");
    let bad = source.replace("{ Nil }", "{ fail.raise(1) }");
    let (graph, _, _) = common::check_files(&[("src/main.i", &bad)]);
    let checked = ilang_check::check(&graph);
    assert!(
        checked
            .diags
            .iter()
            .any(|d| d.is_error() && d.source.is_some_and(|id| graph.sources.get(id).text == bad)),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
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
fn provider_conflicts_keep_call_and_binding_diagnostics() {
    // Spec §9.2: a provided P satisfies a need S only when P <: S.
    // Both the call obligation and the enclosing binding must retain the
    // Reader(String)/Reader(Int) conflict; keeping only the call loses reports.
    let source = include_str!("types/soundness.i");
    let (graph, _, diags) = common::check_files(&[("src/main.i", source)]);
    let mut sites: Vec<_> = diags
        .iter()
        .filter(|d| {
            d.code == "E0515"
                && d.message.contains("Reader(String)")
                && d.message.contains("Reader(Int)")
        })
        .map(|d| ilang_syntax::span::LineIndex::new(source).line_col(d.span.start))
        .collect();
    sites.sort();
    assert_eq!(
        sites,
        [(14, 5), (14, 19), (16, 5), (16, 29)],
        "{:?}",
        graph.sources.render_all(&diags)
    );
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

#[test]
fn named_function_provider_keeps_call_and_binding_diagnostics() {
    // §9.2: changing an inline provider function to a named helper must
    // retain the same call and binding obligations. The lowering boundary
    // fix sees inline function expressions; this case also needs cached-row
    // validation when the closed provider contains only a function name.
    let source = include_str!("types/soundness.i")
        .replace("Reader(get = () -> Int { 1 })", "Reader(get = get_i)")
        + "\nget_i = () -> Int { 1 }\n";
    assert!(source.contains("Reader(get = get_i)"));
    let source = source.as_str();
    let (graph, _, diags) = common::check_files(&[("src/main.i", source)]);
    let mut sites: Vec<_> = diags
        .iter()
        .filter(|d| {
            d.code == "E0515"
                && d.message.contains("Reader(String)")
                && d.message.contains("Reader(Int)")
        })
        .map(|d| ilang_syntax::span::LineIndex::new(source).line_col(d.span.start))
        .collect();
    sites.sort();
    assert_eq!(
        sites,
        [(14, 5), (14, 19), (16, 5), (16, 29)],
        "{:?}",
        graph.sources.render_all(&diags)
    );
}

#[test]
fn deferred_callback_need_meet_survives_generalization_and_transfer() {
    let helper = "Read(A) = (read = () -> A)
both = <A, B>(f: () <- (Read(A)) -> A, g: () <- (Read(B)) -> B) { f() g() }";
    // An open meet is admissible. Only the instance with disjoint closed
    // callback requirements is forbidden by §9.2's empty-meet paragraph.
    types(helper);
    types(&format!(
        "{helper}\nneed_int = () <- (r: Read(Int)) {{ r.read() }}\nuse = () {{ both(need_int, need_int) }}"
    ));
    for imported in [false, true] {
        let calls = "need_int = () <- (r: Read(Int)) { r.read() }
need_string = () <- (r: Read(String)) { r.read() }
use = () { both(need_int, need_string) }";
        let local = format!("{helper}\n{calls}");
        let library = format!("(Read(..), both)\n{helper}");
        let main = format!("(app/helper)\nRead(A) = helper.Read(A)\nboth = helper.both\n{calls}");
        let files = if imported {
            vec![
                ("src/helper.i", library.as_str()),
                ("src/main.i", main.as_str()),
            ]
        } else {
            vec![("src/main.i", local.as_str())]
        };
        let (graph, _, diags) = common::check_files(&files);
        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        assert_eq!(errors.len(), 1, "{:?}", graph.sources.render_all(&diags));
        assert_eq!(
            errors[0].code,
            "E0515",
            "{:?}",
            graph.sources.render_all(&diags)
        );
    }
}

#[test]
fn intersection_inference_never_selects_a_variable_by_operand_order() {
    for intersection in ["A & B", "B & A"] {
        for call in ["", "\nuse = () { choose(1, \"s\", 1) }"] {
            let source =
                format!("choose = <A, B>(x: {intersection}, a: A, b: B) {{ (x + 1, a, b) }}{call}");
            let (graph, _, diags) = common::check_files(&[("src/main.i", &source)]);
            let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
            assert_eq!(errors.len(), 1, "{:?}", graph.sources.render_all(&diags));
            assert_eq!(errors[0].code, "E0504");
            assert!(errors[0].message.contains("principal type"));
        }
        // A stated Int member proves the constraint without specializing
        // either generic component. §4.4 makes both spellings identical.
        types(&format!(
            "choose = <A, B>(x: {intersection} & Int, a: A, b: B) {{ (x + 1, a, b) }}\nuse = () {{ choose(1, \"s\", 1) }}"
        ));
    }
}
