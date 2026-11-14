//! Row schemes preserve obligations, identity, and declaration independence.
mod common;

#[test]
fn stage_three_provider_diagnostics_match_landed_conversion() {
    // A supplied value must satisfy the needed form by subtyping;
    // incompatible same-key requirements have an empty intersection.
    // Keep every concrete provider diagnostic, including its source binding.
    // A method returning Never inhabits the whole-method meet, so its
    // concrete provider failures must be checked independently.
    // Pin total errors and E0515 counts independently of the saved
    // messages. The full fourteen-project guard is in CLI providers tests:
    // two projects need their std dependency and test-macro expansion.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/review/cases");
    for (case, count, provider_count) in [
        ("review-k4/cases/provision_closure_False", 1, 1),
        ("review-k4d/_loose/p/x8", 2, 2),
        ("review-k4d/_loose/p/b1", 5, 4),
        // The method-only S meet is inhabited; retain the
        // independent provider mismatch at 10:48 in both corpus copies.
        ("review-k4d/_loose/p/m3", 1, 1),
        ("review-k4d/proj/m3", 1, 1),
    ] {
        let dir = root.join(case);
        let source = std::fs::read_to_string(dir.join("src/main.i")).unwrap();
        let observation: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("expected.json")).unwrap())
                .unwrap();
        let mut expected: Vec<_> = observation["outcome"]["check"]["stderr"]
            .as_str()
            .unwrap()
            .lines()
            .filter(|line| line.contains("error["))
            .collect();
        assert_eq!(expected.len(), count, "landed diagnostic count: {case}");
        assert_eq!(
            expected
                .iter()
                .filter(|line| line.contains("error[E0515]"))
                .count(),
            provider_count,
            "landed provider diagnostic count: {case}"
        );
        let (graph, _, _) = common::check_files(&[("src/main.i", &source)]);
        let checked = ilang_check::check(&graph);
        let mut actual: Vec<_> = checked
            .diags
            .iter()
            .filter(|d| d.is_error())
            .map(|d| graph.sources.render(d))
            .collect();
        expected.sort_unstable();
        actual.sort_unstable();
        assert_eq!(actual, expected, "{case}");
    }
}

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
    // A group's bindings get their principal schemes together, and union is
    // commutative. Declaration order changes neither.
    let defs = [
        "S(N) = (f = () <- N -> Int)",
        "T(N) = (f = () <- N -> Int)",
        "us = <N>() <- (s : S(N)) { s.f() }",
        "ut = <N>() <- (t : T(N)) { t.f() }",
        "f = (n : Int) { n |> (0) { us() } | (_) { g(n - 1) } }",
        "g = (n : Int) { n |> (0) { ut() } | (_) { f(n - 1) } }",
    ];
    let expected = types(&defs.join("\n"));
    for order in [[1, 0, 3, 2, 5, 4], [0, 1, 3, 2, 4, 5], [1, 0, 2, 3, 5, 4]] {
        let source = order.map(|i| defs[i]).join("\n");
        assert_eq!(types(&source), expected);
    }
}

#[test]
fn a_hidden_handler_obligation_is_rejected_at_the_definition() {
    // “When the parameter does not change, Ai and Bi must be equal”.
    // An invariant service still retains a hidden same-form obligation.
    let source = "Svc(A) = (A, (A) -> Nil)\nwrap : <N>(() <- (Fail(Never), ..N) -> Int, Svc(String)) <- (Fail(Never), ..N) -> Int = (body, svc) { body() <- (svc) }";
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
    // A constructor without parameters has only one service form.
    // Omitting its subtraction cannot hide an argument equality.
    types(
        "Only = ()\nwrap = <N>(body : () <- N -> Nil) -> Nil { body() <- (Only) }\nexample = () { wrap(() { Nil }) }",
    );
}

#[test]
fn closed_row_aliases_merge_by_the_defined_direction() {
    // “when they have the same one, it is that type constructor
    // with each pair of arguments put together the other way round”.
    // A row parameter follows the same rule as any other parameter.
    let source = "Dep = ()\nS(N) = (() <- N -> Nil)\nT(N) = (() <- N -> Nil)\nBad = T(S(Never) | S(Dep))\nuse = (x : Bad) { x }";
    assert_eq!(
        types(source),
        types(&source.replace("S(Never) | S(Dep)", "S(Never)"))
    );
    // Variance still applies to the row as a whole, for directed subtyping.
    types(
        "Other = ()\nS(A) = (A)\nT(N) = (() <- N -> Nil)\nwiden = (x : T(S(Int))) -> T(S(Int) | Other) { x }",
    );
}

#[test]
fn equivalent_nested_row_orders_do_not_change_handler_acceptance() {
    // Unions are sets. The proof must use type equality,
    // including alias equality and union order, rather than source text.
    let base = "Dep = ()\nExtra = ()\nSvc(R) = (() <- R -> Nil)\nwrap : <N>(\n\t() <- (Svc(Extra | Dep), ..N) -> Nil,\n\tSvc(Dep | Extra),\n) <- N -> Nil = (body, svc) {\n    provided : Svc(Extra | Dep) = svc\n    body() <- (provided)\n}";
    let before = types(base);
    let reordered = base.replace("provided : Svc(Extra | Dep)", "provided : Svc(Dep | Extra)");
    assert_ne!(base, reordered);
    let after = types(&reordered);
    assert_eq!(before, after);
    assert!(before["app/main.wrap"].contains("Svc(Dep | Extra)"));
}

#[test]
fn row_kind_survives_an_alias_without_algebra() {
    // N stays a need row when a function alias is copied without
    // normalization, whether alone or beside a named service. Errors
    // use an ordinary Fail argument now, not a second row kind.
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
    // L <: v <: U proves v = T when L = T = U. A hole in a
    // invariant handler fixed to Int must prove the same witness as writing Int.
    let source = "Svc(A) = (f = (A) -> A)
wrap = <N>(body: () <- (Svc(Int), ..N) -> Int, svc: Svc(Int)) -> Int {
	body() <- (svc)
}";
    let invariant_hole = source.replace("svc: Svc(Int)", "svc: Svc(_)");
    assert_ne!(source, invariant_hole);
    assert_eq!(types(source), types(&invariant_hole));
    for inferred in [false, true] {
        let head = if inferred {
            "wrap = <N>(body : () <- (Fail(Never), Svc(Int), ..N) -> Int, svc : Svc(Int))"
        } else {
            "wrap : <N>(() <- (Fail(Never), Svc(Int), ..N) -> Int, Svc(Int)) <- (Fail(Never), ..N) -> Int = (body, svc)"
        };
        let source = format!(
            "Svc(A) = (f = () -> A)\n{head} {{\n provided : Svc(Int) = svc\n body() <- (provided)\n}}\n"
        );
        let with_hole = source.replace("provided : Svc(Int)", "provided : Svc(_)");
        assert_ne!(source, with_hole);
        assert_eq!(types(&source), types(&with_hole), "inferred={inferred}");
    }
    // Never has an implicit lower bound, so v <: Never also fixes v.
    let never = "S(N) = (() <- N -> Nil)\nuse = () <- (s : S(Never)) {\n\tS(f) = s\n\tf()\n}\nwrap : <N>(() <- (S(Never), ..N) -> Nil, S(Never)) <- N -> Nil = (body, svc) {\n\tbody() <- (svc)\n}";
    types(never);
}

#[test]
fn a_handler_annotation_hole_cannot_hide_its_incompatible_value() {
    // The actual provider P must satisfy S by ordinary subtyping.
    // A covariant hole with incoming Int cannot provide Svc(String).
    for annotation in ["Int", "_"] {
        let source = format!(
            "Svc(A) = (f = () -> A)\nwrap = <N>(body: () <- (Svc(String), ..N) -> String, svc: Svc(Int)) -> String {{ provided: Svc({annotation}) = svc\n body() <- (provided) }}"
        );
        let (graph, _, _) = common::check_files(&[("src/main.i", &source)]);
        let checked = ilang_check::check(&graph);
        assert!(
            checked.diags.iter().any(|d| d.code == "E0515"),
            "{annotation}: {:?}",
            graph.sources.render_all(&checked.diags)
        );
    }
}

#[test]
fn written_row_algebra_keeps_its_mode_before_normalization() {
    // “When the parameter does not change, Ai and Bi must be equal”.
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
            // Need lists require named service types, so algebra over error
            // values belongs inside the ordinary Fail argument.
            format!("() <- (Fail({row})) -> Nil"),
        ] {
            let source =
                format!("{invariant_head}Written = {row}\nuse = (x : {written}) {{ x }}\n");
            let (graph, _, _) = common::check_files(&[("src/main.i", &source)]);
            let checked = ilang_check::check(&graph);
            // Fail's argument is an ordinary value type in the need row. Its
            // invariant union/intersection error is E0503, as elsewhere;
            // the former separate error-row diagnostic no longer applies.
            let expected = if op == "-" && written.contains("Fail(") {
                // A difference in an ordinary error value requires value
                // subtyping; this is no longer an error-row check.
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
    let alias = "Dep = ()\nS(N) = (() <- N -> Nil)\nBad = { S(Never) | S(Dep) }\nuse = (body : () <- (Bad) -> Nil) { body() }";
    assert_eq!(
        types(alias),
        types(&alias.replace("S(Never) | S(Dep)", "S(Never)"))
    );
    // Value unions keep ordinary variance; distinct row constructors may
    // also coexist. Neither is a collision between two forms of one service type.
    types(&format!(
        "{head}value = (x : {{ S(Never) | S(Dep) }}) {{ x }}
"
    ));
    types(&format!(
        "{head}value = (x : T(S(Never) | Dep)) {{ x }}
"
    ));
}

#[test]
fn instantiated_output_rows_are_checked_even_when_unused() {
    // Two S forms are forbidden in the returned callback's environment,
    // whether it is invoked or left unused. The two rows are unioned.
    let source = "S(A) = (A)\nT(N) = (f = () <- N -> Nil)\nmerge = <N, M>(a : T(N), b : T(M)) {\n\tT(\n\t\tf = () {\n\t\t\ta.f()\n\t\t\tb.f()\n\t\t},\n\t)\n}\none = () <- (s : S(Int)) { Nil }\ntwo = () <- (s : S(String)) { Nil }\n\nmain = () {\n\tunused = merge(T(f = one), T(f = two))\n\tNil\n}";
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
    // Subtype comparison governs scalar error value sets when function types
    // are joined; the row mode must reach merge_arg as well as calls.
    types(
        "(\n\ti/fail\n)\none = () -> Nil { fail.raise(1) }\ntwo = () -> Nil { fail.raise(2) }\npick = (b : Bool) { b |> (True) { one } | (False) { two } }",
    );
}

#[test]
fn a_directed_row_handler_keeps_the_callback_row_open() {
    // The need is an intersection; function subtyping also determines
    // the direction of N. Handling S(N) must not put S(N) inside N.
    let shown = types(
        "S(N) = (f = () <- N -> Int)
use = <N>() <- (s: S(N)) { s.f() }
wrap = <N>(svc: S(N)) -> Int { use() <- (svc) }",
    );
    // The retained key restriction is shown explicitly; A stays open to
    // every other service. The retained obligation uses the actual provider
    // form S(A), as required by the directed P <: S check.
    assert_eq!(shown["app/main.wrap"], "(S(A)) <- (A - S(A)) -> Int");
    types(
        "Dep = (value = Int)\nS(N) = (f = () <- N -> Int)\nuse = <N>() <- (s: S(N)) { s.f() }\nwrap = <N>(svc: S(N)) -> Int { use() <- (svc) }\nneed = () <- (d: Dep) { d.value }\nexample = () { wrap(S(f = () { need() })) <- (Dep(value = 1)) }\n",
    );
}

#[test]
fn a_function_union_shows_the_intersection_of_its_service_needs() {
    // Function environments combine by merging their service needs.
    let shown = types(
        "Yield(A) = ((A) -> Nil)\none = () <- (s : Yield(Int)) { Nil }\ntwo = () <- (s : Yield(String)) { Nil }\nfns = [one, two]",
    );
    assert_eq!(
        shown["app/main.fns"],
        "List(() <- (Yield(Int | String)) -> Nil)"
    );
}

#[test]
fn equal_invariant_arguments_do_not_block_other_argument_meets() {
    // Invariant arguments must be equal; the remaining arguments
    // still follow their defined directions in the same service entry.
    let shown = types(
        "Cell(I, A) = (state = I, put = (I) -> Nil, value = A)\none = () <- (s : Cell(Int, Int)) { Nil }\ntwo = () <- (s : Cell(Int, 1)) { Nil }\nboth = () {\n\tone()\n\ttwo()\n}",
    );
    assert_eq!(shown["app/main.both"], "() <- (Cell(Int, 1)) -> Nil");
}

#[test]
fn an_undetermined_need_argument_keeps_its_intersection() {
    // “a & T, with a type variable a, stays as it is.” The service
    // meet must not constrain the ordinary value parameter A to Int.
    let shown = types(
        "Read(A) = (value = A)\nneed_int = () <- (r : Read(Int)) { Nil }\nboth = <A>(x : A) <- (r : Read(A)) {\n\tneed_int()\n\tx\n}",
    );
    assert_eq!(shown["app/main.both"], "(A) <- (Read(Int & A)) -> A");
}

#[test]
fn closed_constructor_arguments_still_report_an_empty_need_meet() {
    let source = "S(A) = (value = A)\none = () <- (s : S(Int)) { s.value + 1 }\nrun = <N>(f : () <- N -> Int, x : N) { f() }\nbad = () { run(() { one() }, S(value = \"hi\")) }";
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
    // A provided value must be a subtype of the needed service.
    // Generalizing an inferred row must retain that obligation for callers.
    let source = "S(A) = (value = A)\nread = () <- (s : S(String)) { s.value <> \"!\" }\nwith = <A>(x : A, f) { provided : S(A) = S(value = x) f() <- (provided) }\nmain = () {\n\twith(1, read)\n\tNil\n}";
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
        "S(N) = (() <- N -> Nil)\nuse = <N>() <- (s : S(N)) {\n\tS(f) = s\n\tf()\n}\nrun = <A, N>(f : () <- N -> A) { f() }\nmain = <N>() <- (s : S(N)) { run(() { use() }) }",
    );
    assert!(shown["app/main.main"].contains("S("), "{shown:?}");
}

#[test]
fn nested_forwarding_keeps_the_provided_services_own_need() {
    // Providing S(Dep) handles S, not the Dep needed by its method.
    let shown = types(
        "Dep = (v = Int)\nget = () <- (d : Dep) { d.v }\nS(N) = (f = () <- N -> Int)\nuse = <N>() <- (s : S(N)) { s.f() }\nrun = <A, N>(f : () <- N -> A) { f() }\nfw = () { use() }\nfr = () { run(() { use() }) }\nmain = () {\n\t{\n\t\th = () { run(fr) }\n\t\tk = () { run(h) + fw() }\n\t\tk()\n\t\tNil\n\t} <- (S(f = () { get() }))\n}",
    );
    assert!(shown["app/main.main"].contains("Dep"), "{shown:?}");
}

#[test]
fn direct_forwarding_keeps_the_provided_services_own_need() {
    let shown = types(
        "Dep = (v = Int)\nget = () <- (d : Dep) { d.v }\nS(N) = (f = () <- N -> Int)\nuse = <N>() <- (s : S(N)) { s.f() }\nrun = <A, N>(f : () <- N -> A) { f() }\nfw = () { use() }\nfr = () { run(() { use() }) }\nmain = () {\n\t{\n\t\tfw() + fr()\n\t\tNil\n\t} <- (S(f = () { get() }))\n}",
    );
    assert!(shown["app/main.main"].contains("Dep"), "{shown:?}");
}

#[test]
fn annotated_mutual_forwarding_keeps_finite_service_rows() {
    types(
        "S(N) = (() <- N -> Int)\nuse = <N>() <- (s : S(N)) {\n\tS(f) = s\n\tf()\n}\nping = <N>(n : Int) <- (s : S(N)) -> Int {\n\tn |> (0) { use() } | (_) { pong(n - 1) }\n}\npong = (n : Int) { ping(n) }",
    );
}

#[test]
fn a_value_argument_cannot_erase_a_shared_row_constraint() {
    let source = "S(A) = (value = A)\none = () <- (s : S(Int)) { s.value + 1 }\nrun = <N>(f : () <- N -> Int, x : N) { f() }\nbad = () { run(() { one() }, S(value = \"hi\")) <- (S(value = 1)) }";
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
                "one = () <- (s : S(Int)) { s.value + 1 }",
                "one = () <- (s : S(Int | String)) { 1 }",
            )
            .replace("\"hi\"", "2"),
    );
}

#[test]
fn catch_generalizes_its_stated_fail_entry_and_residual_row() {
    // Handling: the callback's (..N, Fail(E)) states the restriction on N.
    // The scheme retains it; it is not an unexpressed provider obligation.
    let shown = types(
        "(i/fail)\nforward = <A, E, N>(body: () <- (Fail(E), ..N) -> A) -> Result(A, E) { fail.catch(body) }\n",
    );
    assert!(shown["app/main.forward"].contains("Fail("), "{shown:?}");
    assert!(shown["app/main.forward"].contains("Result("), "{shown:?}");
}

#[test]
fn catch_cannot_split_a_fail_entry_into_the_residual_row() {
    // Choosing E = Int and N = Fail(String) would allow
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
fn a_closed_callback_needs_a_provider_even_for_an_unused_fail_entry() {
    // Decision 2 (EF03/T4) removes universal-entry erasure. Preserve the
    // original unused-head source as a negative case beside a real raise;
    // a provider, not choosing E = Never, makes the pure callback valid.
    let unused = "unused = <E>() <- (handler: Fail(E)) { Nil }\nrun = (f: () -> Nil) { f() }\nmain = () { run(() { unused() }) }\n";
    let provided = unused.replace(
        "unused() })",
        "unused() <- (Fail(raise = (_: Never) { panic(\"unreachable\") })) })",
    );
    types(&provided);
    for source in [
        unused,
        "(i/fail)\nrun = (f: () -> Nil) { f() }\nmain = () { run(() { fail.raise(1) }) }\n",
    ] {
        let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
        let checked = ilang_check::check(&graph);
        assert!(
            checked.diags.iter().any(|d| d.is_error()
                && d.source
                    .is_some_and(|id| graph.sources.get(id).text == source)),
            "{source}: {:?}",
            graph.sources.render_all(&checked.diags)
        );
    }
}

#[test]
fn a_sink_with_a_stored_callback_row_has_a_finite_instantiation() {
    // The callback stores Fail in the callback row. The factory is pure and its
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
    let source = "Read(A) = (get = () -> A)\nwith_int = (body) { service : Read(Int) = Read(get = () -> Int { 1 }) body() <- (service) }\nmain = () { with_int(() <- (r : Read(Int)) { r.get() }) }\n";
    let shown = types(source);
    assert!(!shown["app/main.main"].contains("Read"), "{shown:?}");
    let bad = source.replace(
        "() <- (r : Read(Int)) { r.get() }",
        "() <- (r : Read(String)) { r.get() }",
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
fn a_filtered_callback_does_not_vacate_an_unused_fail_need() {
    // excluding Fail excludes its key even when E = Never.
    // Keep both old sources: an unused head and a real raise must reject.
    let source = "(i/fail)\nNoError(N) = N - Fail(Never)\nrun = <N>(f: () <- (NoError(N)) -> Nil) { f() }\npure = <E>() <- (unused: Fail(E)) { Nil }\nmain = () { run(pure) }\n";
    let plain = source.replace("pure = <E>() <- (unused: Fail(E))", "pure = ()");
    let shown = types(&plain);
    assert!(!shown["app/main.main"].contains("Fail"), "{shown:?}");
    for candidate in [
        source.to_owned(),
        source.replace("{ Nil }", "{ fail.raise(1) }"),
    ] {
        let (graph, _, _) = common::check_files(&[("src/main.i", &candidate)]);
        let checked = ilang_check::check(&graph);
        assert!(
            checked.diags.iter().any(|d| d.is_error()
                && d.source
                    .is_some_and(|id| graph.sources.get(id).text == candidate)),
            "{candidate}: {:?}",
            graph.sources.render_all(&checked.diags)
        );
    }
}

#[test]
fn syntax_three_written_provider_hole_keeps_its_obligation() {
    let base = "S(N) = (f = () <- N -> Int)\nuse = <N>() <- (s : S(N)) { s.f() }\n";
    let positive = [
        base,
        "wrap = <N>(body, svc : S(N)) { body() <- (svc) }\nmain = () { assert(wrap(() { use() }, S(f = () -> Int { 1 })) == 1) }\n",
    ].concat();
    types(&positive);
    let negative = [
        base,
        "wrap = (body) { provided : S(_) = S(f = () -> Int { 1 }) body() <- (provided) }\n",
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

#[test]
fn aliased_open_provider_contracts_are_not_generalized_away() {
    let base = "S(N) = (f = () <- N -> Int)\nuse = <N>() <- (s : S(N)) { s.f() }\n";
    for alias in [
        "Alias = S(_)\n",
        "Hidden = S(_)\nAlias = Hidden\n",
        "Hidden(A) = S(A | _)\nAlias = Hidden(Never)\n",
    ] {
        let source = [
            base,
            alias,
            "wrap = (body) { provided : Alias = S(f = () -> Int { 1 }) body() <- (provided) }",
        ]
        .concat();
        let (graph, _, diags) = common::check_files(&[("src/main.i", &source)]);
        assert_eq!(
            diags
                .iter()
                .filter(|d| d.is_error())
                .map(|d| d.code)
                .collect::<Vec<_>>(),
            ["E0515"],
            "{:?}",
            graph.sources.render_all(&diags)
        );
    }
    let (graph, _, diags) = common::check_files(&[
        (
            "src/svc.i",
            "(S(..), Alias)\nS(N) = (f = () <- N -> Int)\nAlias = S(_)\ntouch = <N>(s : S(N)) { s }",
        ),
        (
            "src/main.i",
            "(svc = app/svc)\nwrap = (body) { provided : svc.Alias = svc.S(f = () -> Int { 1 }) body() <- (provided) }",
        ),
    ]);
    assert_eq!(
        diags
            .iter()
            .filter(|d| d.is_error())
            .map(|d| d.code)
            .collect::<Vec<_>>(),
        ["E0515"],
        "{:?}",
        graph.sources.render_all(&diags)
    );
}

#[test]
fn a_closed_provider_alias_retains_all_reader_diagnostics() {
    let source = include_str!("types/soundness.i")
        .replacen("\nwith_int", "\nIntReader = Reader(Int)\nwith_int", 1)
        .replace("_syn3_0 : Reader(Int)", "_syn3_0 : IntReader");
    let (graph, _, diags) = common::check_files(&[("src/main.i", &source)]);
    let actual = diags
        .iter()
        .filter(|d| d.code == "E0515" && d.message.contains("`Reader(String)` and `Reader(Int)`"))
        .map(|d| ilang_syntax::span::LineIndex::new(&source).line_col(d.span.start))
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        [(15, 19), (15, 5), (17, 29), (17, 5)],
        "{:?}",
        graph.sources.render_all(&diags)
    );
}

#[test]
fn named_function_provider_keeps_call_and_binding_diagnostics() {
    // Changing an inline provider function to a named helper must
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
fn an_error_alias_uses_an_ordinary_value_parameter_after_e3() {
    // Fail(E) is ordinary data; E is not a need row. The
    // corpus negative retains its row-kind intent with Row(N) <- N.
    let row =
        include_str!("../../../tests/review/cases/review-k4-lane/alias_kind_error/src/main.i");
    let value = row.replace("Row(N) = () <- N -> Nil", "Row(N) = () <- (Fail(N)) -> Nil");
    let shown = types(&value);
    assert!(shown["app/main.bad"].contains("Fail("), "{shown:?}");
    assert!(
        shown["app/main.bad"].contains("Svc(Int | Nil)"),
        "{shown:?}"
    );
    let (graph, _, diags) = common::check_files(&[("src/main.i", row)]);
    assert!(
        diags.iter().any(|d| d.code == "E0515"),
        "{:?}",
        graph.sources.render_all(&diags)
    );
}

#[test]
fn unused_service_heads_keep_the_required_environment_key() {
    // Simplifying a service argument cannot supply a missing constructor key.
    // Keep both unused-head cases and the original used-head control.
    for source in [
        include_str!("../../../tests/review/cases/review-k4s/min_main_3_0/src/main.i"),
        include_str!("../../../tests/review/cases/review-k4s/min_main_5_0/src/main.i"),
    ] {
        let shown = types(source);
        assert!(shown["app/main.main"].contains("S("), "{shown:?}");
    }
    let used = types(include_str!(
        "../../../tests/review/cases/review-k4s/min_main_6_0/src/main.i"
    ));
    assert!(used["app/main.main"].contains("S("), "{used:?}");
}

#[test]
fn corpus_entry_errors_name_each_unprovided_service_once() {
    // Entry diagnostics identify the target's concrete missing keys. Repeated E0523 codes are distinct
    // errors for S and T, not duplicate reports of one service.
    for (source, expected) in [
        (
            include_str!(
                "../../../tests/review/cases/review-k4d/_loose/fuzz/e0505/21_160/src/main.i"
            ),
            vec!["S", "T"],
        ),
        (
            include_str!(
                "../../../tests/review/cases/review-k4d/_loose/fuzz/e0505/21_243/src/main.i"
            ),
            vec!["S", "T"],
        ),
        (
            include_str!(
                "../../../tests/review/cases/review-k4d/_loose/fuzz/e0505/21_135/src/main.i"
            ),
            vec!["Dep", "S", "T"],
        ),
    ] {
        let (graph, _, diags) = common::check_files(&[("src/main.i", source)]);
        assert!(
            !diags.iter().any(|d| d.is_error()),
            "{:?}",
            graph.sources.render_all(&diags)
        );
        let checked = ilang_check::check(&graph);
        let program = checked.program.unwrap();
        let typed = checked.types.unwrap();
        let module = common::module(&program, "app/main");
        let index = program
            .module(module)
            .defs
            .iter()
            .position(|d| d.name == "main")
            .unwrap();
        let entry = ilang_check::hir::DefId {
            module,
            index: index as u32,
        };
        let errors = ilang_check::platform::stub(
            &program,
            &typed,
            entry,
            ilang_check::platform::Target::Native,
        )
        .unwrap_err();
        assert_eq!(errors.len(), expected.len(), "{errors:?}");
        for (error, name) in errors.iter().zip(expected) {
            assert_eq!(error.code, "E0523");
            assert!(
                error.message.contains(&format!("`app/main.{name}`")),
                "{errors:?}"
            );
        }
    }
}

#[test]
fn a_vendored_sink_marker_keeps_its_row_invariant() {
    // The old (N) -> Nil marker used a row as an ordinary value. Moving N
    // into a callback need must retain the marker's opposite variance.
    let source = "Dep = ()\nOther = ()\nSink(N) = (() <- N -> Nil, (() <- N -> Nil) -> Nil)\nmake = <N>(send: () <- N -> Nil) -> Sink(N) { Sink(send, (_: () <- N -> Nil) { Nil }) }\n";
    types(source);
    let bad = format!("{source}widen = (sink: Sink(Dep)) -> Sink(Dep | Other) {{ sink }}\n");
    let (graph, _, diags) = common::check_files(&[("src/main.i", &bad)]);
    assert!(
        diags.iter().any(|d| d.is_error()),
        "{:?}",
        graph.sources.render_all(&diags)
    );
}

#[test]
fn a_bottom_returning_method_can_satisfy_two_covariant_needs() {
    // Constructor arguments intersect, while functions may stop or diverge.
    // A stopping method therefore inhabits Read(Never).
    let shown = types(
        "Read(A) = (read = () -> A)\nf = () <- (r : Read(Int)) { r.read() }\ng = () <- (r : Read(String)) { r.read() }\nboth = () { f() g() }\nmain = () { both() <- (Read(read = () { panic(\"bottom\") })) }",
    );
    assert!(shown["app/main.both"].contains("Read(Never)"), "{shown:?}");
    // Keep the negative value-field case: unlike a method returning Never,
    // a required field of type Never cannot be constructed.
    let source = "Box(A) = (value = A)\nf = () <- (b : Box(Int)) { b.value }\ng = () <- (b : Box(String)) { b.value }\nboth = () { f() g() }";
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
    let checked = ilang_check::check(&graph);
    assert!(
        checked.diags.iter().any(|d| d.code == "E0515"),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn a_two_parameter_method_meet_depends_on_required_data() {
    // A stopping function inhabits () -> Never.
    // This is the exact constructor shape of the b1 corpus summaries.
    let source = "S(A, N) = (() <- N -> A)\nleft = () <- (s: S(String, Never)) { Nil }\nright = () <- (s: S(Int, S(String, Never))) { Nil }\nboth = () { left() right() }\n";
    let shown = types(source);
    assert!(shown["app/main.both"].contains("S(Never,"), "{shown:?}");
    // being inhabited does not provide the constructor key.
    let provided = format!("{source}main = () {{ both() <- (S(() {{ panic(\"bottom\") }})) }}");
    types(&provided);
    let impossible = source.replace("S(A, N) = (() <- N -> A)", "S(A, N) = (A, () <- N -> A)");
    let (graph, _, _) = common::check_files(&[("src/main.i", &impossible)]);
    let checked = ilang_check::check(&graph);
    assert!(
        checked.diags.iter().any(|d| d.code == "E0515"),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn shipped_stored_callback_types_keep_their_need_invariant() {
    // Stored callback needs remain invariant. Read the shipped
    // declarations, rather than a second copy that could miss a port bug.
    let stream = include_str!("../../../std/src/stream.i");
    let log = include_str!("../../../std/src/log.i");
    let declarations = stream
        .lines()
        .filter(|line| {
            ["Stream(", "Done =", "Next(", "Step("]
                .iter()
                .any(|prefix| line.starts_with(prefix))
        })
        .chain(log.lines().filter(|line| line.starts_with("Sink(")))
        .collect::<Vec<_>>()
        .join("\n");
    let head = format!("Level = ()\nEntry = ()\nConsumer(A) = ((A) -> Nil)\n{declarations}\n");
    types(&format!(
        "{head}widen_error = (s: Stream(Int, Never, Never)) -> Stream(Int, String, Never) {{ s }}"
    ));
    for body in [
        "bad = (s: Stream(Int, String, Consumer(Int))) -> Stream(Int, String, Consumer(Never)) { s }",
        "bad = (s: Sink(String, Consumer(Int))) -> Sink(String, Consumer(Never)) { s }",
        "bad = (s: Stream(Int, String, Never)) -> Stream(Int, Never, Never) { s }",
    ] {
        let source = format!("{head}{body}");
        let (graph, _, _) = common::check_files(&[("src/main.i", &source)]);
        let checked = ilang_check::check(&graph);
        assert!(
            checked.diags.iter().any(|d| d.is_error()),
            "{body}: {:?}",
            graph.sources.render_all(&checked.diags)
        );
    }
}

#[test]
fn mixed_recursive_need_parameters_can_be_forwarded_as_values() {
    // the same variable may occur in a need and an
    // ordinary parameter. Keep every original accepted corpus variant.
    for source in [
        include_str!("../../../tests/review/cases/review-k4s/mixed_recursive_0_0/src/main.i"),
        include_str!("../../../tests/review/cases/review-k4s/mixed_recursive_0_17/src/main.i"),
        include_str!("../../../tests/review/cases/review-k4s/mixed_recursive_1_0/src/main.i"),
        include_str!("../../../tests/review/cases/review-k4s/mixed_recursive_1_17/src/main.i"),
    ] {
        let shown = types(source);
        assert!(shown["app/main.bad"].contains("S("), "{shown:?}");
    }
}

#[test]
fn provider_conflicts_keep_call_and_binding_diagnostics() {
    // A provided P satisfies a need S only when P <: S.
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
