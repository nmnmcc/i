//! Interpretation checks: expectations come from literal-types, not snapshots.
use ilang_check::types::Typed;

fn check(source: &str) -> (Vec<(String, String)>, Vec<String>) {
    check_export(source, false)
}

fn check_export(source: &str, export: bool) -> (Vec<(String, String)>, Vec<String>) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("i.toml"),
        format!(
            "name = \"app\"\n[dependencies]\nstd = {{ path = {:?} }}\n",
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std")
        ),
    )
    .unwrap();
    if export {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(dir.path().join("i.toml"))
            .unwrap();
        writeln!(f, "[wasm]\nexports = [\"app/main.f\"]").unwrap();
    }
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.i"), source).unwrap();
    let (mut graph, problems) = ilang_package::load(dir.path());
    assert!(problems.is_empty(), "{problems:?}");
    // These fixtures inspect every top-level value's type. Make that use
    // explicit without exporting private nominal types or changing any
    // constraints on the values. Unused binding behavior has its own suite.
    let (program, _) = ilang_check::resolve_program(&graph);
    if let Some(p) = program {
        let m = p
            .modules
            .iter()
            .find(|m| Some(m.package) == graph.root)
            .unwrap();
        let names = m
            .defs
            .iter()
            .filter_map(|d| {
                matches!(d.kind, ilang_check::hir::DefKind::Binding { .. })
                    .then_some(d.name.as_str())
            })
            .collect::<Vec<_>>();
        let reads = names
            .iter()
            .map(|name| format!("\n_ = {name}\n"))
            .collect::<String>();
        graph.sources.set_text(m.source, format!("{source}{reads}"));
    }
    let checked = ilang_check::check(&graph);
    let lines = match (&checked.program, &checked.types) {
        (Some(p), Some(t)) => p
            .modules
            .iter()
            .enumerate()
            .filter(|(_, m)| Some(m.package) == graph.root)
            .flat_map(|(i, _)| Typed::lines(t, p, ilang_check::hir::ModuleId(i as u32)))
            .collect(),
        _ => Vec::new(),
    };
    (
        checked
            .diags
            .iter()
            .map(|d| (d.code.to_string(), d.message.clone()))
            .collect(),
        lines,
    )
}

#[test]
fn expressions_do_not_widen_or_compute_arithmetic() {
    // Decision 4/5: values are never widened; operators give whole types;
    // unary minus sees only an unsigned literal through brace groups.
    let (ds, lines) =
        check("a = 0x10\nb = -{{1}}\nc = -{-1}\nd = 1 + 1\ne = [1, 2]\nf = [1, \"a\"]\n");
    assert!(ds.is_empty(), "{ds:?}");
    for text in [
        "a : 16",
        "b : -1",
        "c : Int",
        "d : Int",
        "e : List(1 | 2)",
        "f : List(1 | \"a\")",
    ] {
        assert!(lines.iter().any(|l| l.ends_with(text)), "{text}: {lines:?}");
    }
}

#[test]
fn missing_values_report_coverage_and_later_constraints_are_kept() {
    // §5.3: immediate Values is replaced by E0516; a later binding adds E0501.
    let (ds, _) = check("f = (n: Int) { n |> { 0 -> \"zero\"  1 -> \"one\" } }\n");
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(ds[0].0, "E0516");
    assert!(ds[0].1.ends_with("matches `2`"), "{ds:?}");
    let (ds, _) = check("f: (Int) -> String = { 0 -> \"zero\"  1 -> \"one\" }\n");
    assert_eq!(ds.len(), 2, "{ds:?}");
    assert!(ds.iter().any(|d| d.0 == "E0501"), "{ds:?}");
    assert!(ds.iter().any(|d| d.0 == "E0516"), "{ds:?}");
}

#[test]
fn scalar_matches_infer_literals_but_fitting_keeps_whole_types() {
    // Decision 6: a literal pattern is exact only in a real match.
    let (ds, lines) = check("kind = { \"a\" -> 1  \"b\" -> 2 }\nfit = (\"a\") { 1 }\n");
    assert!(ds.is_empty(), "{ds:?}");
    assert!(
        lines
            .iter()
            .any(|l| l.ends_with("kind : (\"a\" | \"b\") -> 1 | 2")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.ends_with("fit : (String) -> 1 ! Assert")),
        "{lines:?}"
    );
}

#[test]
fn equality_diagnostics_distinguish_the_operator() {
    // §5.4: disjoint constants give False for == and True for !=.
    for (op, result) in [("==", "False"), ("!=", "True")] {
        let (ds, _) = check(&format!("x = 1 {op} 2\n"));
        assert_eq!(ds.len(), 1, "{ds:?}");
        assert_eq!(ds[0].0, "W0503");
        assert!(
            ds[0].1.contains(&format!("`{op}` is always `{result}`")),
            "{ds:?}"
        );
    }
}

#[test]
fn shared_fields_do_not_infer_an_empty_parameter() {
    // §5.3: repeated written parameters keep String and require coverage.
    let (ds, lines) = check(
        "Pair(A) { Pair(A, A) }\nk = (p) { p |> { Pair(\"a\", \"x\") -> 1  Pair(\"b\", \"y\") -> 2 } }\nx = () { k(Pair(\"a\", \"x\")) }\n",
    );
    assert_eq!(ds.iter().filter(|d| d.0 == "E0516").count(), 1, "{ds:?}");
    assert!(!ds.iter().any(|d| d.0 == "E0501"), "{ds:?}");
    assert!(
        lines.iter().any(|l| l.contains("(p: Pair(String))")),
        "{lines:?}"
    );
}

#[test]
fn unknown_parts_and_all_hidden_input_directions_are_conservative() {
    // Decision 7: unknown parts contribute Top, including generalized variables;
    // callback results, returned functions, environments and setters are hidden.
    let (ds, _) = check(
        r#"
(list = std/list, result = std/result)
State(S) { State(get: () -> S) }
cur(S) = (seed: S, st ! State(S)) { st.get() }
make(S) = (init: S) { (f: (S) -> S) { f(init) } }
f = () {
    ! State(Int) = State(get: () { 5 })
    assert(cur(0) == 5)
    assert(make(0)((n) { n + 1 }) == 1)
    assert(result.unwrap(Ok(1), 50) == 1)
    assert(list.fold([1,2,3,4], 0, { (n, _) { { n == 3 } |> { True -> n  False -> n + 1 } } }) == 3)
}
"#,
    );
    assert!(ds.is_empty(), "{ds:?}");
}

#[test]
fn prefix_witness_does_not_match_a_prefix_row() {
    // §5.5 concrete witnesses: after empty string and "a" prefix, choose "b".
    let (ds, _) = check("f = (s: String) { s |> { \"\" -> 0  \"a\" <> _ -> 1 } }\n");
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(ds[0].0, "E0516");
    assert!(ds[0].1.ends_with("matches `\"b\"`"), "{ds:?}");
}

#[test]
fn inferred_host_export_cannot_receive_a_restricted_scalar() {
    // Decision 9/§5.8: the host validates form only; inferred input sets must be All.
    let (ds, _) = check_export("(f)\nf = { \"a\" -> 1  \"b\" -> 2 }\n", true);
    assert!(ds.iter().any(|d| d.0 == "E0521"), "{ds:?}");
}

#[test]
fn empty_optional_accumulator_requires_a_written_tuple_shape() {
    // Coordinator ruling: keep rule 13 and decision 7; the unannotated
    // reduced example is a rejection, and the acceptance example is annotated.
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/t1-blockers/none-accumulator/src/main.i"),
    )
    .unwrap();
    let (ds, _) = check(&source);
    assert_eq!(ds,vec![("E0506".into(),"`.k` needs a known type for its value (§4.8 rule 13); write its type, such as `x: (A, B)`".into())]);
    let (ds, _) = check(&source.replace("Some(p) ->", "Some(p: (Int, Int)) ->"));
    assert!(ds.is_empty(), "{ds:?}");
}

#[test]
fn guarded_list_heads_leave_tail_items_unrestricted() {
    // Std-1b report §5: rule 11 must pass catch-all slack to every
    // homogeneous list item, including items reached through the rest.
    let (ds, _) = check(
        r#"
probe = (xs: List(Int)) -> Nil {
    xs |> {
        [] -> Nil
        [37, a, b, ..tail] ? a >= 0 && b >= 0 -> Nil
        [37, .._] -> Nil
        [x, ..tail] -> Nil
    }
}
"#,
    );
    assert!(ds.is_empty(), "{ds:?}");
}

#[test]
fn grouped_negatives_supply_the_same_written_group_facts() {
    // Decision 5 and §6.10: groups do not change directness or known literal facts.
    for value in ["1", "-1", "-{1}", "-{{1}}"] {
        let (ds, _) = check(&format!("pair = ({value}, f)\nf = () {{ pair.0 == 2 }}\n"));
        assert_eq!(ds.len(), 1, "{value}: {ds:?}");
        assert_eq!(ds[0].0, "W0503");
        let (ds, _) = check(&format!("pair = ({value}, f)\nf = () {{ pair.0 }}\n"));
        assert!(ds.is_empty(), "{value}: {ds:?}");
    }
}

#[test]
fn shared_list_items_need_coverage_but_an_option_parameter_keeps_literals() {
    // §5.3: shared places get whole scalars; a once-occurring Some(A) keeps literals.
    let (ds, lines) =
        check("((Option, Some, None) = std/option)\nk = { Some(\"a\") -> 1  None -> 2 }\n");
    assert!(ds.is_empty(), "{ds:?}");
    assert!(
        lines.iter().any(|l| l.contains("Option(\"a\")")),
        "{lines:?}"
    );
    let (ds, _) = check("k = (xs) { xs |> { [] -> 0  [\"a\", ..] -> 1  [\"b\", ..] -> 2 } }\n");
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(ds[0].0, "E0516");
    assert!(
        ds[0]
            .1
            .contains("the items of a list get the whole type `String`"),
        "{ds:?}"
    );
}

#[test]
fn nested_missing_values_are_one_coverage_error() {
    // §5.3 explicitly requires one E0516 rather than E0501 for immediate Values.
    for (source, witness) in [
        (
            "f = (r: Result(Int, Nil)) { r |> { Ok(0) -> 1  Error(_) -> 2 } }",
            "Ok(1)",
        ),
        ("f = (n: Int | Nil) { n |> { 0 -> 1  Nil -> 2 } }", "1"),
    ] {
        let (ds, _) = check(source);
        assert_eq!(ds.len(), 1, "{ds:?}");
        assert_eq!(ds[0].0, "E0516");
        assert!(ds[0].1.ends_with(&format!("matches `{witness}`")), "{ds:?}");
    }
}

#[test]
fn written_literals_and_negative_differences_constrain_calls() {
    // §3 and §10: at a primary, minus builds a literal; after a type it subtracts.
    for (annotation, accepted, rejected) in [
        ("Int - -1", "1", "-1"),
        ("Int - 1", "-1", "1"),
        ("-1 | 1", "-1", "0"),
        ("-{1}", "-1", "1"),
    ] {
        let source = format!("f = (n: {annotation}) {{ n }}\nv = f({accepted})\n");
        let (ds, _) = check(&source);
        assert!(ds.is_empty(), "{source}: {ds:?}");
        let (ds, _) = check(&source.replace(&format!("f({accepted})"), &format!("f({rejected})")));
        assert!(ds.iter().any(|d| d.0 == "E0501"), "{annotation}: {ds:?}");
    }
}

#[test]
fn typed_complements_keep_the_head_and_cover_cross_columns() {
    // §5.5: each excluded scalar value has its own specialization.
    let cases = [
        (
            "f = (s: String) { s |> { \"a\" -> 0  x: String - \"a\" -> 1 } }",
            None,
        ),
        (
            "f = (s: String) { s |> { \"a\" -> 0  x: String - \"a\" - \"b\" -> 1 } }",
            Some("\"b\""),
        ),
        (
            "f = (s: String) { s |> { \"a\" -> 0  \"\" <> r -> 1 } }",
            None,
        ),
        (
            "f = (p: (String, Bool)) { p |> {\n (x: String - \"a\", _) -> 1\n (\"a\", True) -> 2\n} }",
            Some("(\"a\", False)"),
        ),
        (
            "f = (m: \"a\" | \"b\" | \"c\") { m |> { \"a\" -> 0  x: String - \"a\" -> 1 } }",
            None,
        ),
    ];
    for (source, witness) in cases {
        let (ds, _) = check(source);
        if let Some(w) = witness {
            assert_eq!(ds.len(), 1, "{source}: {ds:?}");
            assert_eq!(ds[0].0, "E0516");
            assert!(ds[0].1.contains(&format!("matches `{w}`")), "{ds:?}");
        } else {
            assert!(ds.is_empty(), "{source}: {ds:?}");
        }
    }
}

#[test]
fn narrowing_checks_the_entire_row_and_ignores_guards() {
    // §5.3: only a fully covered candidate row may be removed. The fast
    // path is reserved for a literal alone at the top place of the real row.
    for (pattern, catch, annotation, accept) in [
        ("\"a\"", "t", "String", true),
        ("\"a\" ? c", "t", "String", false),
        ("\"a\" | \"b\"", "t", "String", true),
        ("Ok(\"a\")", "Ok(t)", "Result(String, Nil)", true),
        ("(\"a\", True)", "(t, _)", "(String, Bool)", false),
        ("(\"a\", _)", "(t, _)", "(String, Bool)", true),
        ("(\"a\", x: Int)", "(t, _)", "(String, Int | String)", false),
        ("[\"a\"]", "[t, ..]", "List(String)", false),
        ("[\"a\", ..]", "[t, ..]", "List(String)", true),
        ("x: String - \"a\"", "t", "String", true),
    ] {
        let need = if pattern.starts_with("x: String -") {
            "\"a\""
        } else {
            "String - \"a\""
        };
        let source = format!(
            "g = (s: {need}) {{ 1 }}\nf = (v: {annotation}, c: Bool) {{ v |> {{\n {pattern} -> 0\n {catch} -> g(t)\n _ -> 2\n}} }}\n"
        );
        let (ds, _) = check(&source);
        let errors: Vec<_> = ds.iter().filter(|d| d.0.starts_with('E')).collect();
        if accept {
            assert!(errors.is_empty(), "{source}: {ds:?}");
        } else {
            assert!(errors.iter().any(|d| d.0 == "E0501"), "{source}: {ds:?}");
        }
    }
}

#[test]
fn prefix_after_complement_does_not_erase_other_columns() {
    // §5.5: an excluded string starting with a prefix prevents treating
    // that Except row as covering the whole prefix.
    let source = "h = (n: Int) { n }\nf = (p: (String, Int)) { p |> {\n (x: String - \"ab\", _) -> 0\n (\"a\" <> r, t) -> h(t)\n _ -> 2\n} }\n";
    let (ds, _) = check(source);
    assert!(!ds.iter().any(|d| d.0 == "E0501"), "{ds:?}");
    // The last wildcard is redundant: the two preceding rows cover String.
    // Without it, neither remaining clause may be called never used.
    let (covered, _) = check(&source.replace(" _ -> 2\n", ""));
    assert!(covered.is_empty(), "{covered:?}");
    let (ds, _) = check(&source.replace("(n: Int)", "(n: String)"));
    assert!(ds.iter().any(|d| d.0 == "E0501"), "{ds:?}");
}

#[test]
fn host_directions_expand_aliases_and_same_scalar_unions() {
    // Decision 9/§5.8: i may send literal sets; host values must be whole.
    let (ds, _) = check(
        "Level = \"info\" | \"warn\"\n@external(\"test\", \"log\")\nlog: (Level | \"error\") -> Nil\n",
    );
    assert!(ds.is_empty(), "{ds:?}");
    for ty in [
        "\"a\"",
        "\"a\" | \"b\"",
        "String - \"a\"",
        "1 | 2",
        "Int - 0",
    ] {
        let (ds, _) = check(&format!(
            "Level = {ty}\n@external(\"test\", \"get\")\nget: () -> Level\n"
        ));
        assert!(ds.iter().any(|d| d.0 == "E0318"), "{ty}: {ds:?}");
        let (ds, _) = check(&format!(
            "@host(\"test:state\")\nState {{ State(get: () -> {ty}) }}\n"
        ));
        assert!(ds.iter().any(|d| d.0 == "E0320"), "{ty}: {ds:?}");
    }
}

#[test]
fn host_boundaries_depend_on_scalar_sets_not_written_forms() {
    // Spec §10.3: outgoing scalar members allow any value set; incoming
    // members must have all values. §4.2/§4.4 define set operations.
    // These four programs are the independent review's reproductions.
    for source in [
        "One = {1 | 2} - 2\n@external(\"app\", \"write\")\nwrite: (One) -> Nil\n",
        "One = Int & 1\n@external(\"app\", \"write\")\nwrite: (One) -> Nil\n",
        "Whole = {Int - 0} | 0\n@external(\"app\", \"read\")\nread: () -> Whole\n",
        "@host(\"app\")\nSvc { Svc(get: () -> String ^ String | \"error\") }\n",
    ] {
        let (ds, _) = check(source);
        assert!(ds.is_empty(), "{source}: {ds:?}");
    }
    // Include nested aliases, cross-head subtraction, and members that
    // disappear before the ABI check. No special written form is privileged.
    for ty in [
        "{1 | 2} - 2",
        "Int & 1",
        "Int - {Int - 1}",
        "{Int | String} & 1",
        "{1 | String} - String",
        "{\"a\" | \"b\"} - \"b\"",
        "String & \"a\"",
    ] {
        let (ds, _) = check(&"Part = BoundaryType\nAlias = Part\n@external(\"app\", \"write\")\nwrite: (Alias) -> Nil\n".replace("BoundaryType", ty));
        assert!(ds.is_empty(), "{ty}: {ds:?}");
        let (ds, _) = check(
            &"Part = BoundaryType\n@external(\"app\", \"read\")\nread: () -> Part\n"
                .replace("BoundaryType", ty),
        );
        assert_eq!(
            ds.iter().map(|d| d.0.as_str()).collect::<Vec<_>>(),
            ["E0318"],
            "{ty}: {ds:?}"
        );
        let (ds, _) = check(
            &"Part = BoundaryType\n@host(\"app\")\nSvc { Svc(get: () -> Part) }\n"
                .replace("BoundaryType", ty),
        );
        assert_eq!(
            ds.iter().map(|d| d.0.as_str()).collect::<Vec<_>>(),
            ["E0320"],
            "{ty}: {ds:?}"
        );
    }
    for ty in [
        "Int & {Int | List(Int)}",
        "Int & {Int | (Int, String)}",
        "Int & {Int | {() -> Nil}}",
        "{Int - 0} | 0",
        "Int - {1 - 1}",
        "{String - \"a\"} | \"a\"",
        "String | \"error\"",
    ] {
        let (ds, _) = check(&"Whole = BoundaryType\n@external(\"app\", \"read\")\nread: () -> Whole\n@host(\"app\")\nSvc { Svc(get: () -> Whole ^ Whole | Nil) }\n".replace("BoundaryType", ty));
        assert!(ds.is_empty(), "{ty}: {ds:?}");
    }
    for env in ["Never", "Int - Int", "String - String"] {
        let (ds, _) = check(
            &"@host(\"app\")\nSvc { Svc(get: () -> String ! BoundaryType) }\n"
                .replace("BoundaryType", env),
        );
        assert!(ds.is_empty(), "{env}: {ds:?}");
    }
    for empty in ["1 - 1", "Int & String"] {
        let (ds, _) = check(
            &"Empty = BoundaryType\n@external(\"app\", \"write\")\nwrite: (Empty) -> Nil\n"
                .replace("BoundaryType", empty),
        );
        assert_eq!(
            ds.iter().map(|d| d.0.as_str()).collect::<Vec<_>>(),
            ["E0318"],
            "{empty}: {ds:?}"
        );
    }
    // Each scalar head in an incoming error row is checked after merging.
    for err in [
        "String | \"error\" | Nil",
        "{{Int - 0} | 0} | String | \"error\"",
        "Int - Int",
    ] {
        let (ds, _) = check(
            &"Errors = BoundaryType\n@host(\"app\")\nSvc { Svc(get: () -> String ^ Errors) }\n"
                .replace("BoundaryType", err),
        );
        assert!(ds.is_empty(), "{err}: {ds:?}");
    }
    for err in [
        "{Int - 0} | 1",
        "String | 1",
        "{String - \"error\"} | \"other\"",
    ] {
        let (ds, _) = check(
            &"@host(\"app\")\nSvc { Svc(get: () -> String ^ BoundaryType) }\n"
                .replace("BoundaryType", err),
        );
        assert_eq!(
            ds.iter().map(|d| d.0.as_str()).collect::<Vec<_>>(),
            ["E0320"],
            "{err}: {ds:?}"
        );
    }
}

#[test]
fn literal_values_use_numeric_and_decoded_scalar_identity() {
    // Decision 12 and §11.4: canonical numbers, decoded strings, no NFC normalization.
    for (ty, accepted, rejected) in [
        ("0x10", "16", "17"),
        ("-0", "0", "1"),
        ("\"\\u{e9}\"", "\"é\"", "\"e\\u{301}\""),
    ] {
        let source = format!("f = (x: {ty}) {{ x }}\nv = f({accepted})\n");
        let (ds, _) = check(&source);
        assert!(ds.is_empty(), "{ds:?}");
        let (ds, _) = check(&source.replace(&format!("f({accepted})"), &format!("f({rejected})")));
        assert!(ds.iter().any(|d| d.0 == "E0501"), "{ds:?}");
    }
    let (ds, _) =
        check("f = (m: \"\\u{0}\" | \"a\" | \"b\") { m |> { \"a\" -> 1  \"b\" -> 2 } }\n");
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert!(ds[0].1.contains("matches `\"\\u{0000}\"`"), "{ds:?}");
}

#[test]
fn shared_parameters_require_all_combinations_of_a_written_finite_set() {
    // §5.3: inference keeps a shared head whole; written finite sets constrain coverage.
    let (ds, _) = check(
        "Pair(A) { Pair(A, A) }\nf = (p: Pair(\"a\" | \"b\")) { p |> { Pair(\"a\", \"a\") -> 0  Pair(\"b\", \"b\") -> 1 } }\n",
    );
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(ds[0].0, "E0516");
    let (ds, _) = check(
        "Pair(A) { Pair(A, A) }\nf = (p: Pair(\"a\" | \"b\")) { p |> { Pair(\"a\", \"a\") -> 0  Pair(\"b\", \"b\") -> 1  Pair(\"a\", \"b\") -> 2  Pair(\"b\", \"a\") -> 3 } }\n",
    );
    assert!(ds.is_empty(), "{ds:?}");
}

#[test]
fn whole_known_bounds_keep_non_covariant_literal_parts_as_holes() {
    // Decision 7/rule 13: copying a known function, invariant cell or
    // contravariant sink must not impose whole String in the wrong direction.
    let source = r#"
Sink(A) { Sink(put: (A) -> Nil) }
Cell(A) { Cell(get: () -> A, set: (A) -> Nil) }
State(A) { State(get: () -> A) }
g = (s: String - "a") { 1 }
p = (g, 0).1
read = (c: Cell("a")) { c.get() }
s = Sink(put: (m: "GET" | "POST") { Nil })
use = () { s.put("GET") }
stored = (st ! State("a")) { st.get() }
e = (stored, 0).1
apply(A, B) = (x: A, f: (A) -> B) { (f, f(x)) }
q = apply("a", { "a" -> 1  "b" -> 2 }).1
"#;
    let (ds, _) = check(source);
    assert!(ds.is_empty(), "{ds:?}");
    let (ds, _) = check(&source.replace("s.put(\"GET\")", "s.put(\"x\")"));
    assert!(ds.iter().any(|d| d.0 == "E0501"), "{ds:?}");
}

#[test]
fn one_clause_choices_are_fitted_and_never_constructor_flow() {
    // Spec §4.8 fitting / §6.12: literals stay whole; choices have no label.
    let (ds, lines) = check("f = { 0 | 1 -> 2 }\n");
    assert!(ds.is_empty(), "{ds:?}");
    assert!(
        lines.iter().any(|l| l.ends_with("f : (Int) -> 2 ! Assert")),
        "{lines:?}"
    );
    // §7.5: fitting does not narrow the original subject to a constructor.
    let (ds, _) = check(
        "T = Yes | No\nYes { Yes(value: Int) }\nNo { No }\nf = (r: T) { r |> { Yes(_) | Yes(_) -> r.value } }\n",
    );
    assert!(ds.iter().any(|d| d.0 == "E0512"), "{ds:?}");
    // §4.8 / §6.12: opaque tests still constrain all values of that head.
    let (ds, _) = check(
        "Box(A) { Box(A) }\nf = (r: Box(Int | String) | Int) { r |> { 0 | _: Box(Int) -> Nil } }\n",
    );
    assert!(ds.iter().any(|d| d.0 == "E0501"), "{ds:?}");
    // §6.12: fitting binds a monotype, without the binding's generalization.
    let (ds, _) = check(
        "id(A) = (x: A) { x }\nf = () { id |> { g | g -> (g(1) + 1, g(\"a\") <> \"b\") } }\n",
    );
    assert!(ds.iter().any(|d| d.0 == "E0501"), "{ds:?}");
}

#[test]
fn fitted_choices_keep_the_deferred_binding_error() {
    // Joint J4/J5: deferred Assert failure is E0501 at this binding, not E0516.
    let (ds, _) = check("s = \"b\"\nf = () -> Int { s |> { \"a\" | \"c\" -> 1 } }\n");
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(ds[0].0, "E0501");
}

#[test]
fn everyday_disjoint_equalities_warn_without_widening() {
    // W520/§6.9: comparisons keep Bool and exact operand types, with a warning.
    // Explicit broad interfaces still avoid the warning.
    for source in [
        "x = 1\nf = () { x == 2 }\n",
        "status = () { \"ok\" }\nf = () { status() == \"error\" }\n",
        "(list = std/list)\nf = () { list.map([\"a\"], (x) { x == \"b\" }) }\n",
    ] {
        let (ds, lines) = check(source);
        assert_eq!(
            ds.iter().map(|d| d.0.as_str()).collect::<Vec<_>>(),
            ["W0503"],
            "{source}: {ds:?}"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.ends_with("-> Bool") || l.ends_with("-> List(Bool)")),
            "{lines:?}"
        );
        if source.starts_with("x ") {
            assert!(lines.iter().any(|l| l.ends_with("x : 1")), "{lines:?}");
        }
        if source.starts_with("status ") {
            assert!(
                lines.iter().any(|l| l.ends_with("status : () -> \"ok\"")),
                "{lines:?}"
            );
        }
    }
    let (ds, _) = check(
        "(list = std/list)\nx = 1\nequal = (a, b) { a == b }\nstatus = () -> String { \"ok\" }\nf = () { (equal(x, 2), status() == \"error\", list.map([\"a\"], (x: String) { x == \"b\" })) }\n",
    );
    assert!(ds.is_empty(), "{ds:?}");
}

#[test]
fn disjoint_guards_do_not_narrow_or_cover() {
    // W520/§6.9 and §7.4: even an always-true guard does not cover a value.
    for source in [
        "f = (x: 1) { x |> { _ ? x == 2 -> 0 } }\n",
        "f = (x: 1) { x |> { _ ? x != 2 -> 0 } }\n",
    ] {
        let (ds, _) = check(source);
        assert_eq!(ds.iter().filter(|d| d.0 == "W0503").count(), 1, "{ds:?}");
        assert_eq!(ds.iter().filter(|d| d.0 == "E0516").count(), 1, "{ds:?}");
    }
    let (ds, lines) = check("f = (x: 1) { x |> { _ ? x == 2 -> x  _ -> 0 } }\n");
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(ds[0].0, "W0503");
    assert!(
        lines.iter().any(|l| l.ends_with("f : (x: 1) -> 0 | 1")),
        "{lines:?}"
    );
    // The impossible guard does not let an invalid body escape checking.
    let (ds, _) = check("f = (x: 1) { x |> { _ ? x == 2 -> { y: 2 = x  y }  _ -> 0 } }\n");
    assert!(ds.iter().any(|d| d.0 == "E0501"), "{ds:?}");
}
