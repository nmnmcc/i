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

fn path_fixture() -> (ilang_package::PackageGraph, ilang_check::hir::Program) {
    let (mut graph, program, _) = common::check_files(&[(
        "src/main.i",
        "(Public(..), public)\nPublic { Public }\nPrivate { Private }\npublic = () { Public }\nprivate = () { Private }\n",
    )]);
    let root = graph.root.unwrap();
    let library = graph.library;
    graph.packages[library]
        .dependencies
        .push(("dep".into(), Some(root)));
    (graph, program.unwrap())
}

#[test]
fn own_private_service_is_accessible() {
    let (g, p) = path_fixture();
    assert!(
        ilang_check::providers::resolve_service(&g, &p, g.root.unwrap(), "app/main.Private")
            .is_ok()
    );
}

#[test]
fn own_private_factory_is_accessible() {
    let (g, p) = path_fixture();
    assert!(
        ilang_check::providers::resolve_binding(&g, &p, g.root.unwrap(), "app/main.private")
            .is_ok()
    );
}

#[test]
fn dependency_public_service_is_accessible() {
    let (g, p) = path_fixture();
    assert!(ilang_check::providers::resolve_service(&g, &p, g.library, "dep/main.Public").is_ok());
}

#[test]
fn dependency_public_factory_is_accessible() {
    let (g, p) = path_fixture();
    assert!(ilang_check::providers::resolve_binding(&g, &p, g.library, "dep/main.public").is_ok());
}

#[test]
fn dependency_private_service_is_rejected() {
    let (g, p) = path_fixture();
    assert_eq!(
        ilang_check::providers::resolve_service(&g, &p, g.library, "dep/main.Private"),
        Err(ilang_check::providers::PathError::Private)
    );
}

#[test]
fn dependency_private_factory_is_rejected() {
    let (g, p) = path_fixture();
    assert_eq!(
        ilang_check::providers::resolve_binding(&g, &p, g.library, "dep/main.private"),
        Err(ilang_check::providers::PathError::Private)
    );
}

#[test]
fn unknown_package_is_rejected() {
    let (g, p) = path_fixture();
    assert_eq!(
        ilang_check::providers::resolve_binding(&g, &p, g.root.unwrap(), "other/main.make"),
        Err(ilang_check::providers::PathError::UnknownPackage)
    );
}

#[test]
fn unknown_module_is_rejected() {
    let (g, p) = path_fixture();
    assert_eq!(
        ilang_check::providers::resolve_binding(&g, &p, g.root.unwrap(), "app/missing.make"),
        Err(ilang_check::providers::PathError::UnknownModule)
    );
}

#[test]
fn unknown_binding_is_rejected() {
    let (g, p) = path_fixture();
    assert_eq!(
        ilang_check::providers::resolve_binding(&g, &p, g.root.unwrap(), "app/main.missing"),
        Err(ilang_check::providers::PathError::UnknownName)
    );
}

#[test]
fn unknown_service_is_rejected() {
    let (g, p) = path_fixture();
    assert_eq!(
        ilang_check::providers::resolve_service(&g, &p, g.root.unwrap(), "app/main.Missing"),
        Err(ilang_check::providers::PathError::UnknownName)
    );
}

#[test]
fn malformed_path_is_rejected() {
    let (g, p) = path_fixture();
    assert_eq!(
        ilang_check::providers::resolve_binding(&g, &p, g.root.unwrap(), "app/main"),
        Err(ilang_check::providers::PathError::InvalidPath)
    );
}
