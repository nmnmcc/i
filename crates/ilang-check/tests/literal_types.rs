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
    let (graph, problems) = ilang_package::load(dir.path());
    assert!(problems.is_empty(), "{problems:?}");
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
        assert_eq!(ds[0].0, "E0520");
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
        assert_eq!(ds[0].0, "E0520");
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
