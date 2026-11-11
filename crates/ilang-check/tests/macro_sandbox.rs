//! Macro sandboxes account for transitive calls and program initialization.
mod common;
use ilang_check::hir::DefId;
use ilang_check::macro_sandbox::foreign_externals;
use ilang_package::Diagnostic;

fn check(files: &[(&str, &str)], entry: &str) -> Vec<String> {
    let (graph, program, diags) = common::check_files(files);
    assert!(
        !diags.iter().any(Diagnostic::is_error),
        "{:?}",
        graph.sources.render_all(&diags)
    );
    let program = program.unwrap();
    let main = common::module(&program, "app/main");
    let index = program
        .module(main)
        .defs
        .iter()
        .position(|d| d.name == entry)
        .unwrap();
    let modules = (0..program.modules.len())
        .map(|i| ilang_check::hir::ModuleId(i as u32))
        .collect::<Vec<_>>();
    foreign_externals(
        &program,
        graph.library,
        &modules,
        &[DefId {
            module: main,
            index: index as u32,
        }],
    )
    .iter()
    .map(|id| program.def(*id).name.clone())
    .collect()
}

#[test]
fn follows_cross_module_calls_but_not_unused_functions() {
    let modules = [
        (
            "src/main.i",
            "(helper = app/helper)\nentry = () { helper.a() }\n",
        ),
        (
            "src/helper.i",
            "(a)\n@external(\"test\", \"read\")\nread: () -> Int\na = () { b() }\nb = () { read() }\n",
        ),
    ];
    assert_eq!(check(&modules, "entry"), ["read"]);
    let unused = [(
        "src/main.i",
        "@external(\"test\", \"read\")\nread: () -> Int\nunused = () { read() }\nentry = () { 1 }\n",
    )];
    assert!(check(&unused, "entry").is_empty());
}

#[test]
fn follows_closures_in_executed_initializers() {
    let modules = [(
        "src/main.i",
        "@external(\"test\", \"read\")\nread: () -> Int\napply = (f) { f() }\ninitial = apply(() { read() })\nentry = () { 1 }\n",
    )];
    assert_eq!(check(&modules, "entry"), ["read"]);
}
