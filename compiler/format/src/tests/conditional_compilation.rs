// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 The Gleam contributors

use crate::{assert_format, assert_format_rewrite};

#[test]
fn multiple() {
    assert_format!(
        "type X

@target(wasm)
type Y {
  Y
}

@target(wasm)
type Z {
  Z
}
"
    );
}

#[test]
fn formatter_preserves_wasm_target_spacing() {
    assert_format_rewrite!(
        "@target( wasm )\nfn wibble() {todo}",
        "@target(wasm)\nfn wibble() {\n  todo\n}\n",
    );
}

#[test]
fn formatter_reports_legacy_target_migration() {
    for target in ["erlang", "javascript", "erl", "js"] {
        let source = format!("@target({target})\nfn main() {{ Nil }}");
        let mut output = String::new();
        let error = crate::pretty(&mut output, &source.into(), "legacy.i".into())
            .expect_err("legacy platform declarations require a port");
        let rendered = error.pretty_string();
        assert!(rendered.contains("wasm"), "{rendered}");
    }
}
