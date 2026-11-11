mod common;

fn entry_needs(source: &str) -> (ilang_check::types::ShownType, ilang_check::hir::Program) {
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
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
    let id = ilang_check::hir::DefId {
        module,
        index: index as u32,
    };
    (typed.entry_provider_needs[&id].clone(), program)
}

#[test]
fn entry_without_services_has_empty_provider_environment() {
    let (needs, _) = entry_needs("@main\nmain = () { Nil }\n");
    assert_eq!(needs, ilang_check::types::ShownType::Never);
}

#[test]
fn entry_retains_ordinary_service_for_provider_planning() {
    let (needs, program) =
        entry_needs("Service { Service }\n@main\nmain = (s ! Service) { Nil }\n");
    let service = program
        .types
        .iter()
        .position(|t| t.name == "Service" && program.module(t.module).path == "app/main")
        .unwrap();
    assert_eq!(
        needs,
        ilang_check::types::ShownType::Named(
            ilang_check::hir::TypeRef::Custom(ilang_check::hir::TypeId(service as u32)),
            vec![]
        )
    );
}

#[test]
fn entry_retains_generic_service_arguments() {
    let (needs, program) =
        entry_needs("Service(A) { Service(value: A) }\n@main\nmain = (s ! Service(Int)) { Nil }\n");
    let service = program
        .types
        .iter()
        .position(|t| t.name == "Service" && program.module(t.module).path == "app/main")
        .unwrap();
    assert_eq!(
        needs,
        ilang_check::types::ShownType::Named(
            ilang_check::hir::TypeRef::Custom(ilang_check::hir::TypeId(service as u32)),
            vec![ilang_check::types::ShownType::Named(
                ilang_check::hir::TypeRef::Builtin(ilang_check::hir::BuiltinType::Int),
                vec![]
            )]
        )
    );
}

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

#[test]
fn empty_binding_name_is_invalid() {
    let (g, p) = path_fixture();
    assert_eq!(
        ilang_check::providers::resolve_binding(&g, &p, g.root.unwrap(), "app/main."),
        Err(ilang_check::providers::PathError::InvalidPath)
    );
}

#[test]
fn empty_module_path_is_invalid() {
    let (g, p) = path_fixture();
    assert_eq!(
        ilang_check::providers::resolve_service(&g, &p, g.root.unwrap(), "app/.Public"),
        Err(ilang_check::providers::PathError::InvalidPath)
    );
}

#[test]
fn missing_module_separator_is_invalid() {
    let (g, p) = path_fixture();
    assert_eq!(
        ilang_check::providers::resolve_service(&g, &p, g.root.unwrap(), "app.Public"),
        Err(ilang_check::providers::PathError::InvalidPath)
    );
}

#[test]
fn service_path_does_not_resolve_factory() {
    let (g, p) = path_fixture();
    assert_eq!(
        ilang_check::providers::resolve_service(&g, &p, g.root.unwrap(), "app/main.public"),
        Err(ilang_check::providers::PathError::UnknownName)
    );
}

#[test]
fn factory_path_does_not_resolve_service() {
    let (g, p) = path_fixture();
    assert_eq!(
        ilang_check::providers::resolve_binding(&g, &p, g.root.unwrap(), "app/main.Public"),
        Err(ilang_check::providers::PathError::UnknownName)
    );
}

#[test]
fn dependency_alias_selects_nominal_service() {
    let (g, p) = path_fixture();
    let own = ilang_check::providers::resolve_service(&g, &p, g.root.unwrap(), "app/main.Public")
        .unwrap();
    let dependency =
        ilang_check::providers::resolve_service(&g, &p, g.library, "dep/main.Public").unwrap();
    assert_eq!(own, dependency);
}

#[test]
fn dependency_alias_selects_nominal_factory() {
    let (g, p) = path_fixture();
    let own = ilang_check::providers::resolve_binding(&g, &p, g.root.unwrap(), "app/main.public")
        .unwrap();
    let dependency =
        ilang_check::providers::resolve_binding(&g, &p, g.library, "dep/main.public").unwrap();
    assert_eq!(own, dependency);
}

#[test]
fn dependency_alias_does_not_grant_private_service_access() {
    let (g, p) = path_fixture();
    assert!(
        ilang_check::providers::resolve_service(&g, &p, g.root.unwrap(), "app/main.Private")
            .is_ok()
    );
    assert_eq!(
        ilang_check::providers::resolve_service(&g, &p, g.library, "dep/main.Private"),
        Err(ilang_check::providers::PathError::Private)
    );
}

#[test]
fn dependency_alias_does_not_grant_private_factory_access() {
    let (g, p) = path_fixture();
    assert!(
        ilang_check::providers::resolve_binding(&g, &p, g.root.unwrap(), "app/main.private")
            .is_ok()
    );
    assert_eq!(
        ilang_check::providers::resolve_binding(&g, &p, g.library, "dep/main.private"),
        Err(ilang_check::providers::PathError::Private)
    );
}

#[test]
fn missing_dependency_module_is_rejected() {
    let (g, p) = path_fixture();
    assert_eq!(
        ilang_check::providers::resolve_service(&g, &p, g.library, "dep/missing.Public"),
        Err(ilang_check::providers::PathError::UnknownModule)
    );
}

#[test]
fn missing_dependency_service_is_rejected() {
    let (g, p) = path_fixture();
    assert_eq!(
        ilang_check::providers::resolve_service(&g, &p, g.library, "dep/main.Missing"),
        Err(ilang_check::providers::PathError::UnknownName)
    );
}

#[test]
fn missing_dependency_binding_is_rejected() {
    let (g, p) = path_fixture();
    assert_eq!(
        ilang_check::providers::resolve_binding(&g, &p, g.library, "dep/main.missing"),
        Err(ilang_check::providers::PathError::UnknownName)
    );
}

#[test]
fn service_name_is_case_sensitive() {
    let (g, p) = path_fixture();
    assert_eq!(
        ilang_check::providers::resolve_service(&g, &p, g.root.unwrap(), "app/main.PUBLIC"),
        Err(ilang_check::providers::PathError::UnknownName)
    );
}

#[test]
fn binding_name_is_case_sensitive() {
    let (g, p) = path_fixture();
    assert_eq!(
        ilang_check::providers::resolve_binding(&g, &p, g.root.unwrap(), "app/main.PUBLIC"),
        Err(ilang_check::providers::PathError::UnknownName)
    );
}

#[test]
fn pending_fixture_retains_host_target_factory() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/spec/9.11/providers_pending");
    let (graph, diagnostics) = ilang_package::load(&root);
    assert!(!diagnostics.iter().any(|d| d.is_error()), "{diagnostics:?}");
    let package = &graph.packages[graph.root.unwrap()];
    let row = package
        .provider_plan
        .rows
        .iter()
        .find(|row| row.service == "app/main.Bundle")
        .unwrap();
    assert!(
        matches!(row.cells.get(&ilang_package::providers::Target::Host), Some(ilang_package::providers::Cell::Factory { factory, .. }) if factory == "app/main.make")
    );
}
