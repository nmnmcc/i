//! Compiler-owned exports admit Syntax operations but not foreign capabilities.
mod common;

#[test]
fn compiler_export_can_use_syntax_without_granting_an_ordinary_entry() {
    let (graph, _, _) = common::check_files(&[(
        "src/main.i",
        "(syntax = i/syntax)\n(identity)\nidentity = (x: syntax.Syntax) { syntax.node(x) x }\n",
    )]);
    let (parsed, diags) = ilang_check::parse(&graph);
    assert!(diags.is_empty());
    let checked = ilang_check::check_compiler_exports(&graph, &parsed, &[("app/main", "identity")]);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    let program = checked.program.unwrap();
    let id = program.host_exports[0];
    assert!(checked.types.unwrap().host_exports.contains_key(&id));
}

#[test]
fn compiler_export_rejects_foreign_external_through_a_helper() {
    let (graph, _, _) = common::check_files(&[(
        "src/main.i",
        "(syntax = i/syntax)\n@external(\"outside\", \"effect\")\neffect: () -> Nil\nhelper = () { effect() }\nidentity = (x: syntax.Syntax) { helper() x }\n",
    )]);
    let (parsed, diags) = ilang_check::parse(&graph);
    assert!(diags.is_empty());
    let checked = ilang_check::check_compiler_exports(&graph, &parsed, &[("app/main", "identity")]);
    assert!(
        checked
            .diags
            .iter()
            .any(|d| d.code == ilang_check::code::COMPILER_CAPABILITY && d.is_error()),
        "{:?}",
        checked.diags
    );
}

#[test]
fn compiler_export_cannot_request_a_host_service() {
    let (graph, _, _) = common::check_files(&[(
        "src/main.i",
        "(syntax = i/syntax)\n@host(\"outside\")\nOutside { Outside(read: () -> Nil) }\nidentity = (x: syntax.Syntax, s ! Outside) { s.read() x }\n",
    )]);
    let (parsed, diags) = ilang_check::parse(&graph);
    assert!(diags.is_empty());
    let checked = ilang_check::check_compiler_exports(&graph, &parsed, &[("app/main", "identity")]);
    assert!(
        checked
            .diags
            .iter()
            .any(|d| d.code == ilang_check::code::COMPILER_CAPABILITY && d.is_error()),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}
