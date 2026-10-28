// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2024 The Gleam contributors

use crate::{assert_module_error, assert_module_infer};

#[test]
fn wasm_external_called_from_gleam() {
    assert_module_infer!(
        r#"@external(wasm, "one", "two")
fn foreign() -> Int

pub fn main() {
  foreign()
}
"#,
        vec![("main", "fn() -> Int")]
    );
}

#[test]
fn unused_private_wasm_external_is_accepted() {
    assert_module_infer!(
        r#"@external(wasm, "one", "two")
fn foreign() -> Int

pub fn main() { 10 }
"#,
        vec![("main", "fn() -> Int")]
    );
}

#[test]
fn public_wasm_external() {
    assert_module_infer!(
        r#"@external(wasm, "one", "two")
pub fn main() -> Int
"#,
        vec![("main", "fn() -> Int")]
    );
}

#[test]
fn external_with_gleam_body() {
    assert_module_infer!(
        r#"@external(wasm, "one", "two")
fn foreign() -> Int

@external(wasm, "one", "three")
pub fn main() -> Int { foreign() }
"#,
        vec![("main", "fn() -> Int")]
    );
}

#[test]
fn external_body_is_still_type_checked() {
    assert_module_error!(
        r#"@external(wasm, "one", "two")
pub fn main() -> Int { "wrong" }
"#
    );
}

#[test]
fn targeted_function_can_call_external() {
    assert_module_infer!(
        r#"@target(wasm)
pub fn main() -> Int { foreign() }

@external(wasm, "one", "two")
fn foreign() -> Int
"#,
        vec![("main", "fn() -> Int")]
    );
}

#[test]
fn imported_external() {
    assert_module_infer!(
        (
            "module",
            r#"@external(wasm, "one", "two")
pub fn foreign() -> Int"#
        ),
        "import module
pub fn main() { module.foreign() }",
        vec![("main", "fn() -> Int")]
    );
}

#[test]
fn external_function_constant_keeps_signature() {
    assert_module_infer!(
        (
            "module",
            r#"@external(wasm, "one", "two")
fn foreign() -> Int
const constant = foreign
pub const foreign_constant = constant
"#
        ),
        "import module
pub fn main() { module.foreign_constant() }",
        vec![("main", "fn() -> Int")]
    );
}

#[test]
fn unused_external_import_is_accepted() {
    assert_module_infer!(
        (
            "mod",
            r#"@external(wasm, "wibble", "wobble") pub fn wobble() -> Int"#
        ),
        "import mod.{wobble}",
        vec![]
    );
}

#[test]
fn imported_external_reexported_as_constant() {
    assert_module_infer!(
        (
            "mod",
            r#"@external(wasm, "wibble", "wobble") pub fn wobble() -> Int"#
        ),
        "import mod.{wobble}
pub const wobble = wobble",
        vec![("wobble", "fn() -> Int")]
    );
}

#[test]
fn wasm_import_names_are_arbitrary_utf8_strings() {
    assert_module_infer!(
        r#"@external(wasm, "宿主@package/namespace", "() => value with spaces")
pub fn main() -> Int
"#,
        vec![("main", "fn() -> Int")]
    );
}

#[test]
fn wasm_import_names_can_be_empty() {
    assert_module_infer!(
        r#"@external(wasm, "", "")
pub fn main() -> Int
"#,
        vec![("main", "fn() -> Int")]
    );
}

#[test]
fn external_requires_argument_annotation() {
    assert_module_error!(
        r#"@external(wasm, "host", "identity")
pub fn identity(value) -> Int
"#
    );
}

#[test]
fn external_requires_return_annotation() {
    assert_module_error!(
        r#"@external(wasm, "host", "identity")
pub fn identity(value: Int)
"#
    );
}

#[test]
fn opaque_host_reference_can_be_passed_between_externals() {
    assert_module_infer!(
        r#"pub type Handle

@external(wasm, "host", "create")
fn create() -> Handle

@external(wasm, "host", "size")
fn size(handle: Handle) -> Int

pub fn main() { size(create()) }
"#,
        vec![("main", "fn() -> Int")]
    );
}
