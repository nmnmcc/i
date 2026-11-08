// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 The Gleam contributors

use super::*;

#[test]
fn module_name_validation() {
    assert!(validate_module_name(&"dream".into()).is_ok());

    assert!(validate_module_name(&"i".into()).is_err());

    assert!(validate_module_name(&"i/ok".into()).is_ok());

    assert!(validate_module_name(&"ok/gleam".into()).is_ok());

    assert!(validate_module_name(&"type".into()).is_err());

    assert!(validate_module_name(&"pub".into()).is_err());

    assert!(validate_module_name(&"ok/type".into()).is_err());

    assert!(validate_module_name(&"ok/pub".into()).is_err());
}

fn analyse_wasm(source: &str) -> Outcome<TypedModule, Vec1<Error>> {
    type_::tests::compile_module_with_opts(
        "wasm_externals",
        source,
        None,
        vec![],
        Target::Wasm,
        TargetSupport::Enforced,
        None,
    )
}

#[test]
fn wasm_import_names_are_arbitrary_strings() {
    let module = analyse_wasm(
        r#"
@external(wasm, "host:ログ/v1", "[read-value]")
pub fn read() -> Int
"#,
    )
    .expect("Wasm imports are not JavaScript identifiers");

    let function = module
        .definitions
        .functions
        .first()
        .expect("imported function");
    let (module_name, function_name, _) = function
        .external_wasm
        .as_ref()
        .expect("the import must remain in the typed AST");
    assert_eq!(module_name, "host:ログ/v1");
    assert_eq!(function_name, "[read-value]");
    assert!(function.implementations.can_run_on_wasm);
    assert!(function.implementations.uses_wasm_externals);

    let value = module
        .type_info
        .values
        .get("read")
        .expect("public function");
    match &value.variant {
        ValueConstructorVariant::ModuleFn { external_wasm, .. } => assert_eq!(
            external_wasm.as_ref(),
            Some(&("host:ログ/v1".into(), "[read-value]".into()))
        ),
        _ => panic!("expected a module function"),
    }
}

#[test]
fn wasm_imports_require_complete_type_annotations() {
    let result = analyse_wasm(
        r#"
@external(wasm, "host", "identity")
pub fn identity(value)
"#,
    );

    let errors = match result {
        Outcome::PartialFailure(_, errors) | Outcome::TotalFailure(errors) => errors,
        Outcome::Ok(_) => panic!("an untyped foreign signature must not be accepted"),
    };
    assert!(errors.iter().any(|error| matches!(
        error,
        Error::ExternalMissingAnnotation {
            kind: MissingAnnotation::Parameter,
            ..
        }
    )));
    assert!(errors.iter().any(|error| matches!(
        error,
        Error::ExternalMissingAnnotation {
            kind: MissingAnnotation::Return,
            ..
        }
    )));
}

#[test]
fn wasm_foreign_types_are_preserved() {
    let module = analyse_wasm(
        r#"
@external(wasm, "host", "Resource")
pub type Resource

@external(wasm, "host", "create-resource")
pub fn create() -> Resource
"#,
    )
    .expect("opaque host types must be usable in foreign signatures");

    let type_ = module
        .definitions
        .custom_types
        .first()
        .expect("foreign type");
    assert_eq!(
        type_
            .external_wasm
            .as_ref()
            .map(|(module, name, _)| (module.as_str(), name.as_str())),
        Some(("host", "Resource"))
    );
    assert!(type_.constructors.is_empty());
}

#[test]
fn wasm_foreign_types_cannot_define_i_constructors() {
    let result = analyse_wasm(
        r#"
@external(wasm, "host", "Resource")
pub type Resource { Resource }
"#,
    );
    let errors = match result {
        Outcome::PartialFailure(_, errors) | Outcome::TotalFailure(errors) => errors,
        Outcome::Ok(_) => panic!("foreign types cannot also define an i representation"),
    };
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, Error::ExternalTypeWithConstructors { .. }))
    );
}

#[test]
fn wasm_external_bodies_are_still_type_checked() {
    let result = analyse_wasm(
        r#"
@external(wasm, "host", "read")
pub fn read() -> Int { "not an integer" }
"#,
    );
    assert!(matches!(
        result,
        Outcome::PartialFailure(_, _) | Outcome::TotalFailure(_)
    ));
}

#[test]
fn wasm_has_arbitrary_width_integer_and_bit_array_types() {
    let module = analyse_wasm(
        "pub fn integer() { 340282366920938463463374607431768211456 }\n\
         pub fn bits() { <<1:size(3)>> }",
    )
    .expect("Wasm supports arbitrary integers and unaligned bit arrays");
    assert_eq!(module.definitions.functions.len(), 2);
}
