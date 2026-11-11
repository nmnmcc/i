//! Interpretation checks for scope UNUSED rules 1–6 and spec §2.4.1.
//! Expected names below come from the root/reference rules, not goldens.
mod common;

use ilang_check::code;
use ilang_package::Diagnostic;

fn warnings(files: &[(&str, &str)]) -> Vec<(String, String)> {
    let (graph, _, diags) = common::check_files(files);
    assert!(
        !diags.iter().any(Diagnostic::is_error),
        "{:?}",
        graph.sources.render_all(&diags)
    );
    assert!(
        diags.iter().all(|d| d.code == code::UNUSED_BINDING),
        "{diags:?}"
    );
    diags
        .iter()
        .map(|d| {
            let source = graph.sources.get(d.source.unwrap());
            (
                source.text[d.span.start as usize..d.span.end as usize].to_string(),
                d.message.clone(),
            )
        })
        .collect()
}

fn names(files: &[(&str, &str)]) -> Vec<String> {
    warnings(files).into_iter().map(|(n, _)| n).collect()
}

#[test]
fn references_in_unused_bindings_count_as_uses() {
    // A reference counts even when its enclosing binding is unused.
    assert_eq!(
        names(&[(
            "src/main.i",
            r#"
(live)
leaf = 1
middle = () { leaf }
live = () { middle() }
dead_value = 2
dead_leaf = 3
dead_caller = () { dead_leaf }
even = (n: Int) { n == 0 || odd(n - 1) }
odd = (n: Int) { n != 0 && even(n - 1) }
"#
        )]),
        ["dead_value", "dead_caller"]
    );
}

#[test]
fn record_spreads_and_fields_count_references() {
    assert_eq!(
        names(&[(
            "src/main.i",
            "(use_record)\nbase = (x: 1)\nvalue = 2\ndead = 3\nuse_record = () { (..base, y: value) }\n"
        )]),
        ["dead"]
    );
}

#[test]
fn entries_tests_and_ordinary_exports_retain_their_helpers() {
    // Rules 1–2: no warning on public API, entries, tests, or their uses.
    assert!(
        names(&[
            ("src/main.i", " main = () { helper() }\nhelper = () { Nil }"),
            ("src/api.i", "(api)\napi = () { hidden }\nhidden = 1"),
            (
                "test/check.i",
                "(app/api)\nhelper = () { api.api() }\n@test check = () { helper()\nNil }"
            ),
        ])
        .is_empty()
    );
}

#[test]
fn test_exports_need_reachable_package_uses() {
    // Rule 4: importing an export and a reference from a dead function
    // can retain it. A test in another test module can retain it.
    assert_eq!(
        names(&[
            (
                "test/helpers.i",
                "(used, dead, imported_only)\nused = () { private() }\nprivate = () { Nil }\ndead = () { Nil }\nimported_only = 1"
            ),
            (
                "test/check.i",
                "((used, imported_only) = app/helpers)\n@test check = () { used() }\ndead_caller = () { helpers.dead() }"
            ),
        ]),
        ["dead_caller", "imported_only"]
    );
}

#[test]
fn underscore_suppression_still_counts_body_references() {
    // Rule 3 suppresses the name; its references still count.
    assert_eq!(
        names(&[(
            "src/main.i",
            "_kept = () { dead }\ndead = 1\n__discard = actual\nactual = 2"
        )]),
        Vec::<String>::new()
    );
}

#[test]
fn each_pattern_name_is_checked_without_retaining_its_siblings() {
    // §2.4.1: using one pattern name retains references, not sibling names.
    assert_eq!(
        names(&[(
            "src/main.i",
            "(public)\n(public, private, _quiet) = (dependency, 2, 3)\ndependency = 1"
        )]),
        ["private"]
    );
}

#[test]
fn hint_requires_test_module_no_value_parameters_and_exact_nil() {
    // Rule 5: only a function that could have @test gets the hint.
    let ws = warnings(&[
        ("src/main.i", "ordinary = () { Nil }"),
        (
            "test/check.i",
            r#"
forgotten = () { assert(True) }
explicit = () -> Nil { Nil }
alias = forgotten
has_argument = (n: Int) { assert(n > 0) }
returns_int = () { 1 }
never_returns = () { panic("stop") }
union_result = (b: Bool) { b |> { True -> Nil  False -> 1 } }
with_environment = (a ! Assert) { assert(True) }
"#,
        ),
    ]);
    assert_eq!(
        ws.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
        [
            "ordinary",
            "explicit",
            "alias",
            "has_argument",
            "returns_int",
            "never_returns",
            "union_result",
            "with_environment"
        ]
    );
    for (n, message) in ws {
        let hint = ["forgotten", "explicit", "alias", "with_environment"].contains(&n.as_str());
        assert_eq!(
            message,
            format!(
                "top-level binding `{n}` is not used{}",
                if hint {
                    "; did you forget `@test`?"
                } else {
                    ""
                }
            )
        );
    }
}

#[test]
fn a_type_spelled_nil_is_not_the_prelude_nil() {
    // Rule 5 says Nil, the nominal prelude type, rather than its spelling.
    assert_eq!(
        warnings(&[("test/check.i", "Nil { Local }\nf = () { Local }")]),
        [("f".into(), "top-level binding `f` is not used".into())]
    );
}

#[test]
fn types_services_constructors_and_external_declarations_are_exempt() {
    // Rule 6's conservative boundary includes unused type declarations.
    assert!(
        names(&[(
            "src/main.i",
            r#"
Private { Private(Int) }
Alias = String
Service { Service(run: () -> Nil) }
@external("env", "f") foreign: () -> Int
"#
        )])
        .is_empty()
    );
}

#[test]
fn value_type_reads_are_uses_even_without_runtime_calls() {
    // §2.4.1 counts references in types; exempt type definitions are roots.
    assert!(
        names(&[(
            "src/main.i",
            r#"
(api)
source = ("ok", 1)
Alias = source
annotation = 1
api: (annotation) -> annotation = (x) { x }
"#
        )])
        .is_empty()
    );
    assert_eq!(
        names(&[("src/main.i", "source = 1\ndead: source = 1")]),
        ["dead"]
    );
}

#[test]
fn pattern_lengths_and_nested_expression_references_count() {
    // Rule 2 includes pattern expressions, guards, nested bindings and lists.
    assert!(
        names(&[(
            "src/main.i",
            r#"
(read)
size = 8
guard = True
value = 1
read = (<<n:u(size)>>) {
  xs = [value]
  n |> { 0 ? guard -> xs  _ -> [] }
}
"#
        )])
        .is_empty()
    );
}

#[test]
fn broken_programs_do_not_gain_speculative_unused_warnings() {
    // Like Gleam's failed-scope handling, avoid incomplete reference data.
    let (_, _, diags) = common::check_files(&[("src/main.i", "dead = 1\nf = () { missing }")]);
    assert!(diags.iter().any(Diagnostic::is_error));
    assert!(!diags.iter().any(|d| d.code == code::UNUSED_BINDING));
}

#[test]
fn wasm_exports_and_editor_checks_have_the_same_roots() {
    // Rule 2 names [wasm].exports separately. The resolver currently
    // requires these names to be exported from source modules (§10.2).
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("i.toml"),
        [
            "name = \"app\"",
            "[wasm]",
            "exports = [\"app/helper.run\"]",
            "",
        ]
        .join("\n"),
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/helper.i"),
        "(run)\nrun = () { hidden }\nhidden = 1\ndead = 2",
    )
    .unwrap();
    let (graph, load) = ilang_package::load(dir.path());
    assert!(load.is_empty(), "{load:?}");
    let checked = ilang_check::check(&graph);
    let queried = ilang_check::query::check_for_queries(&graph);
    let a = graph.sources.render_all(&checked.diags);
    let b = graph.sources.render_all(&queried.diags);
    assert_eq!(a, b);
    assert_eq!(
        a,
        ["src/helper.i:4:1: warning[W0504]: top-level binding `dead` is not used"]
    );
    // Isolate the host-export root from the ordinary export root, using
    // the resolved identity rather than another spelling-based exemption.
    let mut p = checked.program.unwrap();
    let d = p.host_exports[0];
    p.modules[d.module.0 as usize].defs[d.index as usize].exported = false;
    let (_, diags) = ilang_check::types::check(&p, &graph.sources);
    assert_eq!(graph.sources.render_all(&diags), a);
}
