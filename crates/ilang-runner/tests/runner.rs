use std::{io::Write, process::Command};

use serde_json::Value;
use tempfile::NamedTempFile;

fn fixture() -> NamedTempFile {
    let mut wasm = wat::parse_str(
        r#"(module
          (memory (export "memory") 1)
          (global $state (mut i32) (i32.const 0))
          (func (export "__ilang_init")
            i32.const 10 global.set $state)
          (func (export "_start"))
          (func (export "passes")
            global.get $state i32.const 10 i32.ne
            if unreachable end
            i32.const 99 global.set $state)
          (func (export "fails") unreachable)
          (func (export "passes_after_failure")
            global.get $state i32.const 10 i32.ne
            if unreachable end)
        )"#,
    )
    .expect("valid WAT");
    let abi = serde_json::json!({
        "version": 1,
        "entry": "_start",
        "tests": ["passes", "fails", "passes_after_failure"],
        "exports": {},
        "imports": {},
    });
    let name = b"ilang.abi";
    let mut custom = Vec::new();
    leb(name.len(), &mut custom);
    custom.extend_from_slice(name);
    custom.extend_from_slice(abi.to_string().as_bytes());
    wasm.push(0);
    leb(custom.len(), &mut wasm);
    wasm.extend_from_slice(&custom);
    let mut file = NamedTempFile::new().expect("temporary fixture");
    file.write_all(&wasm).expect("write fixture");
    file
}

fn leb(mut value: usize, output: &mut Vec<u8>) {
    loop {
        let byte = (value & 127) as u8;
        value >>= 7;
        output.push(byte | if value == 0 { 0 } else { 128 });
        if value == 0 {
            break;
        }
    }
}

fn run(file: &NamedTempFile, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ilang-runner"))
        .arg(file.path())
        .args(args)
        .output()
        .expect("execute runner")
}

#[test]
fn runs_default_entry() {
    let output = run(&fixture(), &["--json"]);
    assert!(output.status.success(), "{:?}", output);
    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON report");
    assert_eq!(report["passed"], 1);
    assert_eq!(report["results"][0]["name"], "_start");
}

#[test]
fn test_trap_does_not_skip_remaining_tests_or_reuse_state() {
    let output = run(&fixture(), &["--all-tests", "--json"]);
    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON report");
    assert_eq!(report["passed"], 2);
    assert_eq!(report["failed"], 1);
    assert_eq!(report["results"][2]["status"], "passed");
    assert!(report["results"][1]["error"].as_str().is_some());
}

#[test]
fn explicit_test_selection_is_deduplicated() {
    let output = run(
        &fixture(),
        &["--test", "passes", "--test", "passes", "--json"],
    );
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON report");
    assert_eq!(report["passed"], 1);
}

#[test]
fn missing_export_is_reported_as_failure() {
    let output = run(&fixture(), &["--invoke", "not_there", "--json"]);
    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON report");
    assert_eq!(report["failed"], 1);
    assert!(report["results"][0]["error"].as_str().is_some());
}

#[test]
fn lists_tests_without_execution() {
    let output = run(&fixture(), &["--list-tests", "--json"]);
    assert!(output.status.success());
    let tests: Vec<String> = serde_json::from_slice(&output.stdout).expect("test names");
    assert_eq!(tests, ["passes", "fails", "passes_after_failure"]);
}

fn shared_fixture() -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary fixture");
    file.write_all(include_bytes!(
        "../../../tests/browser/fixtures/host-abi.wasm"
    ))
    .expect("write shared fixture");
    file
}

#[test]
fn guest_stdout_is_captured_inside_the_json_report() {
    let output = run(&shared_fixture(), &["--invoke", "main", "--json"]);
    assert!(output.status.success(), "{:?}", output);
    let report: Value = serde_json::from_slice(&output.stdout).expect("one JSON report");
    assert_eq!(report["results"][0]["stdout"], "fixture main\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn host_panic_text_is_preserved_and_later_tests_run() {
    let output = run(
        &shared_fixture(),
        &["--test", "failing_test", "--test", "passing_test", "--json"],
    );
    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("one JSON report");
    assert_eq!(report["passed"], 1);
    assert_eq!(report["failed"], 1);
    assert!(
        report["results"][0]["error"]
            .as_str()
            .expect("panic message")
            .contains("fixture assertion failed")
    );
}
