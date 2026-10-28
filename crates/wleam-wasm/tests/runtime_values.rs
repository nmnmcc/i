//! Execute the production value and allocator runtime in the pinned Wasm engine.

use std::{env, path::Path, process::Command};

#[test]
fn deep_values_and_memory_boundaries_match_the_runtime_oracle() {
    let wasmtime = env::var_os("WLEAM_WASMTIME")
        .or_else(|| env::var_os("WASMTIME"))
        .unwrap_or_else(|| "wasmtime".into());
    let wasm_tools = env::var_os("WASM_TOOLS").unwrap_or_else(|| "wasm-tools".into());
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/runtime_values.py");
    let output = Command::new("python3")
        .arg(script)
        .arg("--wasmtime")
        .arg(wasmtime)
        .arg("--wasm-tools")
        .arg(wasm_tools)
        .output()
        .expect("Python 3 is required for the runtime oracle; run inside Devenv");

    assert!(
        output.status.success(),
        "runtime oracle failed ({}). Run inside Devenv, or set WLEAM_WASMTIME/WASMTIME and WASM_TOOLS.\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
