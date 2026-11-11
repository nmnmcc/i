mod common;

#[test]
fn private_factory_scheme_is_available_to_provider_checking() {
    let (graph, _, _) =
        common::check_files(&[("src/main.i", "Service { Service }\nmake = () { Service }\n")]);
    let checked = ilang_check::check(&graph);
    let program = checked.program.unwrap();
    let typed = checked.types.unwrap();
    let module = common::module(&program, "app/main");
    let index = program
        .module(module)
        .defs
        .iter()
        .position(|d| d.name == "make")
        .unwrap();
    let id = ilang_check::hir::DefId {
        module,
        index: index as u32,
    };
    assert!(typed.provider_scheme_types.contains_key(&id));
    assert!(!typed.scheme_types.contains_key(&id));
}
