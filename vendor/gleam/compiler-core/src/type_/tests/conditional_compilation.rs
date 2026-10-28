// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2021 The Gleam contributors

use crate::{
    assert_module_infer, parse::error::ParseErrorType, type_::Error, warning::WarningEmitter,
};

#[test]
fn targeted_definitions_are_type_checked() {
    let errors = super::compile_module(
        "test_module",
        "@target(wasm) pub type X = Unknown",
        None,
        vec![],
    )
    .into_result()
    .expect_err("Wasm declarations must not be silently excluded");
    assert!(errors.iter().any(|error| matches!(
        error, Error::UnknownType { name, .. } if name == "Unknown"
    )));
}

#[test]
fn alias() {
    assert_module_infer!(
        "@target(wasm)
pub type X = Int

pub const x: X = 1
",
        vec![("x", "Int")],
    );
}

#[test]
fn alias_and_constant() {
    assert_module_infer!(
        "@target(wasm)
pub type X = Int

@target(wasm)
pub const x: X = 1
",
        vec![("x", "Int")],
    );
}

#[test]
fn generalising() {
    assert_module_infer!(
        "
@target(wasm)
pub fn id(x) { x }

@target(wasm)
pub fn x() { id(1) }
",
        vec![("id", "fn(a) -> a"), ("x", "fn() -> Int")],
    );
}

#[test]
fn targeted_and_untargeted_definitions_share_inference() {
    assert_module_infer!(
        "
@target(wasm)
pub fn id(x) { x }

pub fn number() { id(1) }

@target(wasm)
pub fn text() { id(\"hello\") }
",
        vec![
            ("id", "fn(a) -> a"),
            ("number", "fn() -> Int"),
            ("text", "fn() -> String"),
        ],
    );
}

#[test]
fn included_const_ref_earlier() {
    assert_module_infer!(
        "
@target(wasm)
const x = 1

pub fn main() { x }
",
        vec![("main", "fn() -> Int")],
    );
}

#[test]
fn included_const_ref_later() {
    assert_module_infer!(
        "pub fn main() { x }

@target(wasm)
const x = 1
",
        vec![("main", "fn() -> Int")],
    );
}

#[test]
fn target_does_not_need_to_be_the_first_attribute() {
    assert_module_infer!(
        r#"
@external(wasm, "host", "value")
@target(wasm)
pub fn main() -> Int
"#,
        vec![("main", "fn() -> Int")],
    );
}

#[test]
fn target_can_precede_external_attribute() {
    assert_module_infer!(
        r#"
@target(wasm)
@external(wasm, "host", "value")
pub fn main() -> Int
"#,
        vec![("main", "fn() -> Int")],
    );
}

#[test]
fn legacy_targets_are_rejected_instead_of_excluding_definitions() {
    for target in ["erlang", "javascript", "erl", "js"] {
        for src in [
            format!("@target({target}) pub fn main() {{ 1 }}"),
            format!(r#"@external({target}, "host", "value") pub fn main() -> Int"#),
        ] {
            let error = crate::parse::parse_module(
                "test_module.gleam".into(),
                &src,
                &WarningEmitter::null(),
            )
            .expect_err("legacy platform attributes require migration to Wasm");
            assert_eq!(error.error, ParseErrorType::UnknownTarget, "{src}");
            assert_eq!(
                &src[error.location.start as usize..error.location.end as usize],
                target,
            );
        }
    }
}
