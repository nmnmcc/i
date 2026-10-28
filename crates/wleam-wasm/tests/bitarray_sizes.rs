//! Exercise size handling through parsing, type checking, IR and Wasm execution.

use std::process::Command;

#[test]
fn dynamic_bit_sizes_distinguish_construction_from_matching() {
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
fn matches(size: Int) -> Bool {
  case <<>> {
    <<_:size(size)>> -> True
    _ -> False
  }
}

fn matches_unit(size: Int) -> Bool {
  case <<>> {
    <<_:size(size)-unit(8)>> -> True
    _ -> False
  }
}

fn construct(size: Int) -> BitArray {
  <<1:size(size)>>
}

pub fn sizes_test() -> Nil {
  assert matches(0)
  assert !matches(-1)
  assert !matches(-340282366920938463463374607431768211456)
  assert !matches(9223372036854775808)
  assert !matches(340282366920938463463374607431768211456)
  assert !matches_unit(2305843009213693952)
  assert construct(-1) == <<>>
  assert construct(-340282366920938463463374607431768211456) == <<>>
  assert <<256:size(-1)>> == <<>>
  let assert <<value:unit(2)-bytes-size(3)>> = <<1:6>>
  assert value == <<1:6>>
  let assert <<value:bytes-unit(2)-size(3)>> = <<1:6>>
  assert value == <<1:6>>
  let invalid = case <<1:6>> {
    <<_:bytes>> -> True
    _ -> False
  }
  assert !invalid
}

pub fn huge_construction_test() -> Nil {
  let _ = construct(340282366920938463463374607431768211456)
  Nil
}

pub fn overflowing_construction_test() -> Nil {
  let size = 2305843009213693952
  let _ = <<1:size(size)-unit(8)>>
  Nil
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
    .expect("type check bit-array size fixture");
    let tests = vec![
        "fixture.sizes_test".into(),
        "fixture.huge_construction_test".into(),
        "fixture.overflowing_construction_test".into(),
    ];
    let artifact = wleam_wasm::compile(
        &checked.modules,
        &wleam_wasm::Options {
            library: true,
            tests: tests.clone(),
            ..Default::default()
        },
    )
    .expect("compile bit-array size fixture");
    let module = directory.path().join("fixture.wasm");
    std::fs::write(&module, artifact.wasm).unwrap();

    // The compiler remains independent of the execution engine. Supplying a
    // tiny host module lets the pinned Wasmtime CLI execute the same artifact.
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
    for (index, test) in tests.into_iter().enumerate() {
        let output = Command::new(&executable)
            .arg("run")
            .arg("--preload")
            .arg(format!("wleam:host/v1={}", host.display()))
            .arg("--invoke")
            .arg(&test)
            .arg(&module)
            .output()
            .expect("Wasmtime is required; run inside Devenv or set WLEAM_WASMTIME");
        assert_eq!(
            output.status.success(),
            index == 0,
            "unexpected result for {test}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        if index != 0 {
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("wasm trap"),
                "expected a construction error, got {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}
