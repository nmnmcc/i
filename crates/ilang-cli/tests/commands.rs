//! `i build`, `i run`, and `i test`, and the host exports of B.2 and B.5.

use ilang_cli::{BuildOptions, Level, build};
use ilang_host::{BitArray, HostOptions, Outcome, Program, Value};
use std::path::Path;
use std::process::Command;

fn package(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (path, text) in files {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    dir
}

fn i(args: &[&str], project: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_i"))
        .args(args)
        .arg("--project")
        .arg(project)
        .output()
        .unwrap()
}

#[test]
fn build_writes_the_module() {
    let dir = package(&[
        ("i.toml", "name = \"app\"\n"),
        ("src/main.i", "@main\nmain = () { 1 + 2 }\n"),
    ]);
    let out = i(&["build"], dir.path());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wasm = std::fs::read(dir.path().join("build/app.wasm")).unwrap();
    let program = Program::from_bytes(&wasm).unwrap();
    assert_eq!(program.abi().entry.as_ref().unwrap().name, "app/main.main");
    assert!(program.abi().tests.is_empty());

    let out = i(&["build", "--opt", "0", "--out"], dir.path());
    assert!(!out.status.success(), "--out needs a file");
    let file = dir.path().join("other/x.wasm");
    let out = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["build", "--opt", "0", "--out"])
        .arg(&file)
        .arg("--project")
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(file.exists());
}

#[test]
fn build_reports_errors() {
    let dir = package(&[
        ("i.toml", "name = \"app\"\n"),
        ("src/main.i", "x: String = 1\n"),
    ]);
    let out = i(&["build"], dir.path());
    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("error[E0501]"), "{err}");
    assert!(!dir.path().join("build").exists());
}

#[test]
fn run_the_example() {
    let hello = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello");
    for opt in ["0", "2"] {
        let out = i(&["run", "--opt", opt], &hello);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let out = i(&["test"], &hello);
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("pass hello/main_test.repeats"), "{text}");
}

#[test]
fn run_prints_the_stop_message() {
    let dir = package(&[
        ("i.toml", "name = \"app\"\n"),
        (
            "src/main.i",
            "f = (x) {\n  panic(x)\n}\n\n@main\nmain = () {\n  f(\"boom\")\n  Nil\n}\n",
        ),
    ]);
    for opt in ["0", "2"] {
        let out = i(&["run", "--opt", opt], dir.path());
        assert_eq!(out.status.code(), Some(1));
        assert_eq!(
            String::from_utf8(out.stderr).unwrap(),
            "panic: boom\napp/main:2:3\napp/main:7:3\n"
        );
    }
}

#[test]
fn test_runs_each_test() {
    let dir = package(&[
        ("i.toml", "name = \"app\"\n"),
        (
            "src/lib.i",
            "export (twice)\n\ntwice = (n: Int) { n * 2 }\n",
        ),
        (
            "test/lib_test.i",
            "import (app/lib)\n\n@test\ngood = () { case lib.twice(2) { 4 -> Nil  _ -> panic(\"no\") } }\n\n@test\nbad = () { panic(\"bad\") }\n",
        ),
    ]);
    let out = i(&["test"], dir.path());
    assert_eq!(out.status.code(), Some(1));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("pass app/lib_test.good"), "{text}");
    assert!(text.contains("FAIL app/lib_test.bad"), "{text}");
    assert!(text.contains("  panic: bad"), "{text}");
    assert!(text.contains("1 passed, 1 failed"), "{text}");

    let out = i(&["test", "good"], dir.path());
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(!text.contains("bad"), "{text}");

    // Tests are not built by `i build`.
    let out = i(&["build"], dir.path());
    assert!(out.status.success());
    let wasm = std::fs::read(dir.path().join("build/app.wasm")).unwrap();
    assert!(Program::from_bytes(&wasm).unwrap().abi().tests.is_empty());
}

#[test]
fn host_exports() {
    let dir = package(&[
        (
            "i.toml",
            "name = \"app\"\n\n[wasm]\nexports = [\"app/math.add\", \"app/math.describe\", \"app/math.flip\", \"app/math.half\", \"app/math.bits\", \"app/math.unit\", \"app/math.alias\"]\n",
        ),
        (
            "src/math.i",
            r#"import (i/core)

export (add, describe, flip, half, bits, unit, alias)

add = (a: Int, b: Int) { a + b }
describe = (name: String, n: Int) { name <> "=" <> core.inspect(n) }
flip = (b: Bool) { core.not(b) }
half = (x: Float) { x /. 2.0 }
bits = (b: BitArray) -> BitArray { <<b:bits, 1:u1>> }
unit = (n: Nil) { n }
alias = add
"#,
        ),
    ]);
    for opt in [Level::O0, Level::O2] {
        let built = build(dir.path(), BuildOptions { tests: false, opt }).unwrap();
        let program = Program::from_bytes(&built.wasm).unwrap();
        assert!(program.abi().entry.is_none());
        let mut instance = program.instantiate(HostOptions::default()).unwrap();
        let call = |instance: &mut ilang_host::Instance, name: &str, args: &[Value]| match instance
            .call(name, args)
            .unwrap()
        {
            Outcome::Ok(v) => format!("{v:?}"),
            other => panic!("{name}: {other:?}"),
        };
        let big: num_bigint::BigInt = "100000000000000000000".parse().unwrap();
        assert_eq!(
            call(
                &mut instance,
                "app/math.add",
                &[Value::Int(big.clone()), Value::Int((-1).into())]
            ),
            format!("{:?}", Value::Int(big.clone() - 1))
        );
        assert_eq!(
            call(
                &mut instance,
                "app/math.alias",
                &[Value::Int(2.into()), Value::Int(3.into())]
            ),
            format!("{:?}", Value::Int(5.into()))
        );
        assert_eq!(
            call(
                &mut instance,
                "app/math.describe",
                &[Value::String("é".into()), Value::Int(7.into())]
            ),
            format!("{:?}", Value::String("é=7".into()))
        );
        assert_eq!(
            call(&mut instance, "app/math.flip", &[Value::Bool(true)]),
            format!("{:?}", Value::Bool(false))
        );
        assert_eq!(
            call(&mut instance, "app/math.half", &[Value::Float(3.0)]),
            format!("{:?}", Value::Float(1.5))
        );
        assert_eq!(
            call(
                &mut instance,
                "app/math.bits",
                &[Value::BitArray(BitArray::new(vec![0xff], 8).unwrap())]
            ),
            format!(
                "{:?}",
                Value::BitArray(BitArray::new(vec![0xff, 0x80], 9).unwrap())
            )
        );
        assert_eq!(
            call(&mut instance, "app/math.unit", &[Value::Nil]),
            format!("{:?}", Value::Nil)
        );
    }
}
