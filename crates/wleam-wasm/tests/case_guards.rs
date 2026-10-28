//! Regressions for alternatives with guards and expression-valued bindings.

use std::process::Command;

#[test]
fn alternatives_retry_the_guard_with_each_patterns_bindings() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join("src")).unwrap();
    std::fs::write(
        directory.path().join("gleam.toml"),
        "name = \"fixture\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    std::fs::write(
        directory.path().join("src/fixture.gleam"),
        r#"
fn choose(first: Int, second: Int) -> Int {
  case first, second {
    value, _ | _, value if value == 2 -> value
    _, _ -> 0
  }
}

fn choose_record(value: Result(Int, Int)) -> Int {
  case value {
    Ok(value) | Error(value) if value > 1 -> value
    _ -> 0
  }
}

fn final_let() -> Int {
  let value = 42
}

fn final_destructure() -> #(Int, Int) {
  let #(first, second) = #(12, 34)
}

fn final_assert() -> Result(Int, String) {
  let assert Ok(value) = Ok(42)
}

fn valid_utf8(value: BitArray) -> Bool {
  case value {
    <<_:utf8_codepoint>> -> True
    _ -> False
  }
}

pub fn control_flow_test() -> Nil {
  // Both alternatives match structurally. The first guard fails, and the
  // second must be evaluated with value rebound from the other subject.
  assert choose(1, 2) == 2
  assert choose(2, 1) == 2
  assert choose(1, 3) == 0
  assert choose_record(Ok(1)) == 0
  assert choose_record(Error(2)) == 2
  assert final_let() == 42
  assert final_destructure() == #(12, 34)
  assert final_assert() == Ok(42)
  assert valid_utf8(<<0xf0, 0x9f, 0xa6, 0x80>>)
  assert !valid_utf8(<<0xff>>)
  assert !valid_utf8(<<0xe2, 0x82>>)
}
"#,
    )
    .unwrap();
    let checked = wleam_frontend::check_project(
        directory.path(),
        &wleam_frontend::CheckOptions {
            use_cache: false,
            ..Default::default()
        },
    )
    .expect("type check control-flow fixture");
    let artifact = wleam_wasm::compile(
        &checked.modules,
        &wleam_wasm::Options {
            library: true,
            tests: vec!["fixture.control_flow_test".into()],
            ..Default::default()
        },
    )
    .expect("compile control-flow fixture");
    let module = directory.path().join("fixture.wasm");
    std::fs::write(&module, artifact.wasm).unwrap();
    let host = directory.path().join("host.wasm");
    std::fs::write(
        &host,
        wat::parse_str(
            r#"(module
              (func (export "write") (param i32 i32 i32))
              (func (export "random") (param i32 i32) (result i32) (i32.const 1))
              (func (export "panic") (param i32 i32) unreachable)
              (func (export "reference_id") (param externref) (result i64) (i64.const 0)))"#,
        )
        .unwrap(),
    )
    .unwrap();
    let executable = std::env::var_os("WLEAM_WASMTIME")
        .or_else(|| std::env::var_os("WASMTIME"))
        .unwrap_or_else(|| "wasmtime".into());
    let output = Command::new(executable)
        .arg("run")
        .arg("--preload")
        .arg(format!("wleam:host/v1={}", host.display()))
        .arg("--invoke")
        .arg("fixture.control_flow_test")
        .arg(&module)
        .output()
        .expect("Wasmtime is required; run inside Devenv or set WLEAM_WASMTIME");
    assert!(
        output.status.success(),
        "control-flow regression: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
