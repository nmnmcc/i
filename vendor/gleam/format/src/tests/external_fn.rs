// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 The Gleam contributors

use crate::assert_format;

#[test]
fn import_without_body() {
    assert_format!(
        r#"@external(wasm, "one", "one")
fn one(x: Int) -> Int
"#
    );
}

#[test]
fn import_without_body_with_module_path() {
    assert_format!(
        r#"@external(wasm, "./host.wasm", "one")
fn one(x: Int) -> Int
"#
    );
}

#[test]
fn import_without_body_with_namespace() {
    assert_format!(
        r#"@external(wasm, "wleam:host/v1", "one")
fn one(x: Int) -> Int
"#
    );
}

#[test]
fn import_with_fallback_body() {
    assert_format!(
        r#"@external(wasm, "one", "one")
fn one(x: Int) -> Int {
  todo
}
"#
    );
}

#[test]
fn import_with_fallback_and_module_path() {
    assert_format!(
        r#"@external(wasm, "./host.wasm", "one")
fn one(x: Int) -> Int {
  todo
}
"#
    );
}

#[test]
fn import_with_fallback_and_namespace() {
    assert_format!(
        r#"@external(wasm, "wleam:host/v1", "one")
fn one(x: Int) -> Int {
  todo
}
"#
    );
}

// https://github.com/gleam-lang/gleam/issues/2259
#[test]
fn break_external_fn_arguments() {
    assert_format!(
        r#"@external(wasm, "ffi", "improper_list_append")
fn improper_list_append(
  a: item_a,
  b: item_b,
  c: improper_tail,
) -> List(anything)
"#
    );
}

// Bug found by Hayleigh
#[test]
fn long_long_external() {
    assert_format!(
        r#"@external(wasm, "./client-component.ffi.mjs", "register")
pub fn register(
  _app: App(WebComponent, Nil, model, msg),
  _name: String,
) -> Result(Nil, Error) {
  Error(NotABrowser)
}
"#
    );
}
