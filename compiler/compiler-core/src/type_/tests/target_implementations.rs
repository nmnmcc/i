// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2024 The Gleam contributors

use ecow::EcoString;
use itertools::Itertools;

use crate::{
    analyse::TargetSupport,
    build::Target,
    type_::{Error, error::MissingAnnotation, expression::Implementations},
};

use super::{DependencyModule, compile_module_with_opts};

const PURE: Implementations = Implementations {
    pure: true,
    can_run_on_wasm: true,
    uses_wasm_externals: false,
};

const FOREIGN: Implementations = Implementations {
    pure: false,
    can_run_on_wasm: true,
    uses_wasm_externals: true,
};

const FOREIGN_WITH_BODY: Implementations = Implementations {
    pure: true,
    can_run_on_wasm: true,
    uses_wasm_externals: true,
};

fn implementations_with_dependencies(
    src: &str,
    dependencies: Vec<DependencyModule<'_>>,
) -> Vec<(EcoString, Implementations)> {
    compile_module_with_opts(
        "test_module",
        src,
        None,
        dependencies,
        Target::Wasm,
        TargetSupport::Enforced,
        None,
    )
    .expect("Wasm implementations must pass target checking")
    .type_info
    .values
    .into_iter()
    .map(|(name, value)| (name, value.variant.implementations()))
    .sorted()
    .collect_vec()
}

fn assert_implementations(src: &str, expected: &[(&str, Implementations)]) {
    let expected = expected
        .iter()
        .map(|(name, implementations)| (EcoString::from(*name), *implementations))
        .collect_vec();
    assert_eq!(expected, implementations_with_dependencies(src, vec![]));
}

#[test]
fn pure_i_functions() {
    assert_implementations(
        "pub fn first() { 1 + 1 } pub fn second() { first() * 2 }",
        &[("first", PURE), ("second", PURE)],
    );
}

#[test]
fn foreign_function_and_transitive_callers() {
    assert_implementations(
        r#"
@external(wasm, "host", "value")
pub fn foreign() -> Int
pub fn middle() { foreign() * 2 }
pub fn outer() { middle() + 1 }
"#,
        &[
            ("foreign", FOREIGN),
            ("middle", FOREIGN),
            ("outer", FOREIGN),
        ],
    );
}

#[test]
fn foreign_function_with_i_body() {
    assert_implementations(
        r#"
@external(wasm, "host", "value")
pub fn foreign() -> Int { 1 + 1 }
pub fn caller() { foreign() * 2 }
"#,
        &[
            ("caller", FOREIGN_WITH_BODY),
            ("foreign", FOREIGN_WITH_BODY),
        ],
    );
}

#[test]
fn foreign_function_with_body_using_another_foreign_function() {
    assert_implementations(
        r#"
@external(wasm, "host", "one")
fn first() -> Int
@external(wasm, "host", "two")
pub fn second() -> Int { first() }
pub fn caller() { second() }
"#,
        &[("caller", FOREIGN), ("first", FOREIGN), ("second", FOREIGN)],
    );
}

#[test]
fn foreign_dependencies_are_order_independent() {
    for src in [
        r#"
pub fn caller() { foreign() }
@external(wasm, "host", "value")
fn foreign() -> Int
"#,
        r#"
@external(wasm, "host", "value")
fn foreign() -> Int
pub fn caller() { foreign() }
"#,
    ] {
        assert_implementations(src, &[("caller", FOREIGN), ("foreign", FOREIGN)]);
    }
}

#[test]
fn unused_foreign_function_does_not_taint_pure_function() {
    assert_implementations(
        r#"
@external(wasm, "host", "value")
fn foreign() -> Int
pub fn independent() { 10 }
"#,
        &[("foreign", FOREIGN), ("independent", PURE)],
    );
}

#[test]
fn foreign_function_reference_and_closure() {
    assert_implementations(
        r#"
@external(wasm, "host", "value")
fn foreign() -> Int
pub fn reference() { foreign }
pub fn closure() { fn() { foreign() } }
pub fn indirect() { let f = foreign f() }
"#,
        &[
            ("closure", FOREIGN),
            ("foreign", FOREIGN),
            ("indirect", FOREIGN),
            ("reference", FOREIGN),
        ],
    );
}

#[test]
fn foreign_function_passed_as_argument() {
    assert_implementations(
        r#"
@external(wasm, "host", "value")
fn foreign() -> Int
fn apply(f) { f() }
pub fn caller() { apply(foreign) }
"#,
        &[("apply", PURE), ("caller", FOREIGN), ("foreign", FOREIGN)],
    );
}

#[test]
fn constants_preserve_foreign_implementation_information() {
    assert_implementations(
        r#"
@external(wasm, "host", "value")
fn foreign() -> Int
const alias = foreign
pub const exported = alias
pub const number = 42
pub fn caller() { exported() }
"#,
        &[
            ("alias", FOREIGN),
            ("caller", FOREIGN),
            ("exported", FOREIGN),
            ("foreign", FOREIGN),
            ("number", PURE),
        ],
    );
}

#[test]
fn imported_foreign_function_and_constant() {
    let dependency = r#"
@external(wasm, "host", "value")
pub fn foreign() -> Int
pub const alias = foreign
"#;
    let values = implementations_with_dependencies(
        r#"
import dependency.{foreign}
pub fn qualified() { dependency.foreign() }
pub fn unqualified() { foreign() }
pub fn constant() { dependency.alias() }
"#,
        vec![("thepackage", "dependency", dependency)],
    );
    assert_eq!(
        values,
        vec![
            ("constant".into(), FOREIGN),
            ("qualified".into(), FOREIGN),
            ("unqualified".into(), FOREIGN),
        ],
    );
}

#[test]
fn foreign_declarations_require_parameter_and_return_annotations() {
    for (src, expected_kind) in [
        (
            r#"@external(wasm, "host", "value") pub fn foreign(x) -> Int"#,
            MissingAnnotation::Parameter,
        ),
        (
            r#"@external(wasm, "host", "value") pub fn foreign(x: Int)"#,
            MissingAnnotation::Return,
        ),
    ] {
        let errors = compile_module_with_opts(
            "test_module",
            src,
            None,
            vec![],
            Target::Wasm,
            TargetSupport::Enforced,
            None,
        )
        .into_result()
        .expect_err("an external cannot infer host parameter or return types");
        assert!(
            errors.iter().any(|error| matches!(
                error,
                Error::ExternalMissingAnnotation { kind, .. } if *kind == expected_kind
            )),
            "{errors:?}"
        );
    }
}

#[test]
fn function_requires_a_body_or_foreign_implementation() {
    let errors = compile_module_with_opts(
        "test_module",
        "pub fn missing() -> Int",
        None,
        vec![],
        Target::Wasm,
        TargetSupport::Enforced,
        None,
    )
    .into_result()
    .expect_err("an unavailable function must fail checking");
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, Error::NoImplementation { .. }))
    );
}
