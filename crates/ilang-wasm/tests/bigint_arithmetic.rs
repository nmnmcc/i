//! Differential execution tests for the emitted integer runtime.
//! NODE can select the Node.js executable supplied by Devenv.

#[path = "../src/runtime/bigint.rs"]
mod bigint;

fn module() -> Vec<u8> {
    let harness = include_str!("fixtures/bigint_harness.wat");
    let source = format!(
        "{}\n{}\n)",
        harness
            .trim_end()
            .strip_suffix(')')
            .expect("module closing paren"),
        bigint::wat()
    );
    let bytes = wat::parse_str(source).expect("bigint runtime must be valid WAT");
    wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::all())
        .validate_all(&bytes)
        .expect("bigint runtime must be valid WasmGC");
    bytes
}

#[test]
fn integer_runtime_validates() {
    assert!(!module().is_empty());
}

#[test]
fn arithmetic_bits_and_conversions_match_javascript_bigint() {
    let directory = tempfile::tempdir().expect("temporary bigint test directory");
    let binary = directory.path().join("bigint.wasm");
    std::fs::write(&binary, module()).expect("write runtime test module");
    let node = std::env::var_os("NODE").unwrap_or_else(|| "node".into());
    let script =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bigint_property.mjs");
    let output = std::process::Command::new(node)
        .arg(script)
        .arg(binary)
        .output()
        .expect("Node.js is required; run tests inside devenv shell or set NODE");
    assert!(
        output.status.success(),
        "BigInt differential tests failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
