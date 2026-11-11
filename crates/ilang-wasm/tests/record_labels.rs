//! records §16 step 2: runtime label ordering equals Rust's UTF-8 order.

use std::{cmp::Ordering, env, path::PathBuf, process::Command};

fn string(value: &str) -> String {
    let bytes = value
        .bytes()
        .map(|byte| format!(" (i32.const {byte})"))
        .collect::<String>();
    format!(
        "(call $string_from_bytes (array.new_fixed $bytes {}{bytes}))",
        value.len()
    )
}

#[test]
fn record_labels_have_the_same_order_in_rust_and_the_runtime() {
    // Fixed labels, quoting and Unicode follow records §4.1 and §9.3.
    let labels = [
        "",
        "\0",
        "_",
        "__",
        "a",
        "a\0",
        "a b",
        "a(",
        "a)",
        "q\"q",
        "z",
        "é",
        "中",
        "😀",
        "\u{10ffff}",
    ];
    let mut checks = String::new();
    for left in labels {
        for right in labels {
            let expected = match left.cmp(right) {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            };
            checks.push_str(&format!(
                "(call $assert (i32.eq (call $string_compare {} {}) (i32.const {expected})))\n",
                string(left),
                string(right)
            ));
        }
    }
    let module = format!(
        "(module\n{}\n\
         (func $host_panic (param i32 i32) unreachable)\n\
         (func $host_reference_id (param externref) (result i64) (i64.const 1))\n\
         {}\n\
         (func $assert (param i32) (if (i32.eqz (local.get 0)) (then unreachable)))\n\
         (func (export \"label_order\") (result i32) {checks} (i32.const 1)))",
        ilang_wasm::runtime::TYPES,
        ilang_wasm::runtime::wat(),
    );
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("labels.wat");
    let binary = temp.path().join("labels.wasm");
    std::fs::write(&source, module).unwrap();
    let wasm_tools = env::var_os("WASM_TOOLS").unwrap_or_else(|| "wasm-tools".into());
    let output = Command::new(&wasm_tools)
        .arg("parse")
        .arg(source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new(wasm_tools)
        .arg("validate")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let wasmtime = env::var_os("WASMTIME").unwrap_or_else(|| "wasmtime".into());
    let output = Command::new(PathBuf::from(wasmtime))
        .args(["run", "--invoke", "label_order"])
        .arg(binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1");
}
