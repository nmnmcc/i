//! Expanded modules must enter the same resolver and checker as parsed source.
mod common;

#[test]
fn checks_transformed_modules_without_reparsing_the_source() {
    let (graph, _, _) = common::check_files(&[("src/main.i", "(value)\nvalue = 1\n")]);
    let (mut parsed, errors) = ilang_check::parse(&graph);
    assert!(errors.is_empty());
    let module = parsed.iter_mut().find(|m| m.path == "app/main").unwrap();
    let ilang_syntax::ast::ItemKind::Binding(binding) = &mut module.ast.items[0].kind else {
        panic!("fixture must contain a binding");
    };
    binding.value.kind = ilang_syntax::ast::ExprKind::String("changed".into());
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        checked.diags
    );
    let program = checked.program.unwrap();
    let types = checked.types.unwrap();
    let lines = types.lines(&program, common::module(&program, "app/main"));
    assert!(
        lines.iter().any(|line| line.contains("\"changed\"")),
        "{lines:?}"
    );
    let original = ilang_check::check(&graph);
    let original_lines = original.types.unwrap().lines(
        &original.program.unwrap(),
        common::module(&program, "app/main"),
    );
    assert!(
        !original_lines
            .iter()
            .any(|line| line.contains("\"changed\""))
    );
}
