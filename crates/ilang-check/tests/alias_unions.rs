//! Unions of one-constructor nominal types joined by an alias: checking
//! scales with the number of members, and a shown union that is exactly an
//! alias's expansion is shown by the alias's name.
mod common;

use std::time::{Duration, Instant};

/// The types shown for `app/main` by `--types`.
fn lines(source: &str) -> Vec<String> {
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
    let checked = ilang_check::query::check_for_queries(&graph);
    let errors: Vec<_> = checked.diags.iter().filter(|d| d.is_error()).collect();
    assert!(
        errors.is_empty(),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    let program = checked.program.unwrap();
    let types = checked.types.unwrap();
    types.lines(&program, common::module(&program, "app/main"))
}

/// `n` members: `C0`, and `Ci(A) = (A, E(A))` for each other `i`,
/// joined as `E(A)`, with an unannotated recursive match over all of them.
fn nominal_union(n: usize) -> String {
    let mut out = vec!["C0 = ()".to_string()];
    for i in 1..n {
        out.push(format!("C{i}(A) = (A, E(A))"));
    }
    let members: Vec<String> = std::iter::once("C0".to_string())
        .chain((1..n).map(|i| format!("C{i}(A)")))
        .collect();
    out.push(format!("E(A) = {}", members.join(" | ")));
    let clauses: Vec<String> = (1..n)
        .map(|i| format!("\t\t| (C{i}(a, r)) {{ a + f0(r) }}"))
        .collect();
    out.push(format!(
        "f0 = (e) {{\n\te |>\n\t\t(C0) {{ 0 }}\n{}\n}}",
        clauses.join("\n")
    ));
    out.push("main = () { f0(C1(1, C0)) }".to_string());
    out.join("\n") + "\n"
}

#[test]
fn nominal_union_recursive_match_scales() {
    // Checking was exponential in the number of members: each added member multiplied the time by
    // about ten, and 9 members took over a minute with optimizations. The work is now polynomial.
    // The bound is relative to the smallest case, so that a busy machine does not fail the test:
    // from 8 to 22 members a polynomial grows by far less than 2000 times, an exponential by more
    // than 10^13 times.
    let mut smallest = None;
    for n in [8, 12, 16, 22] {
        let source = nominal_union(n);
        let start = Instant::now();
        let lines = lines(&source);
        let took = start.elapsed();
        eprintln!("{n} members: {took:?}");
        let base = *smallest.get_or_insert(took.max(Duration::from_millis(50)));
        assert!(
            took < base * 2000,
            "{n} members took {took:?}, 8 members took {base:?}"
        );
        assert!(
            lines.contains(&"app/main.f0 : (E(Int)) -> Int".to_string()),
            "{n} members: {lines:?}"
        );
    }
}

#[test]
fn union_is_displayed_by_its_declared_name() {
    let source = "Some(A) = (A)
None = ()
Option(A) = Some(A) | None
first = (xs: List(Option(Int))) -> Int {
	xs |> ([Some(x), ..r]) { x } | (_) { 0 }
}
pick = (b: Bool) { b |> (True) { Some(1) } | (False) { None } }
main = () { first([pick(True)]) }
";
    let lines = lines(source);
    for expected in [
        "app/main.first : (List(Option(Int))) -> Int",
        "app/main.pick : (Bool) -> Option(1)",
    ] {
        assert!(lines.contains(&expected.to_string()), "{lines:?}");
    }
}
