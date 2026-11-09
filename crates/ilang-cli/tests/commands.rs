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
        assert_eq!(String::from_utf8(out.stdout).unwrap(), "Hello from i!\n");
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
        let built = build(
            dir.path(),
            BuildOptions {
                tests: false,
                opt,
                offline: false,
            },
        )
        .unwrap();
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

#[test]
fn host_exports_give_errors() {
    let dir = package(&[
        (
            "i.toml",
            "name = \"app\"\n\n[wasm]\nexports = [\"app/math.divide\", \"app/math.check\", \"app/math.positive\", \"app/math.bits\"]\n",
        ),
        (
            "src/math.i",
            r#"import (i/fail)

export (divide, check, positive, bits)

// The errors are `String | Int`, given to the host in the canonical
// order: `Int` is member 1 and `String` member 2 (B.2).
divide = (a: Int, b: Int) {
  case b {
    0 -> fail.raise("division by zero")
    1 -> fail.raise(a)
    _ -> a / b
  }
}

// `Nil` and `Bool` errors have their own tags.
check = (n: Int) -> Float ? Bool | Nil {
  case n {
    0 -> fail.raise(Nil)
    1 -> fail.raise(True)
    _ -> 1.5
  }
}

// The `Assert` of a host export stops the program (§9.6).
positive = (n: Int) {
  assert(n > 0)
  n
}

// A `BitArray` error.
bits = (b: Bool) -> Nil ? BitArray {
  case b {
    True -> fail.raise(<<1, 2>>)
    False -> Nil
  }
}
"#,
        ),
    ]);
    for opt in [Level::O0, Level::O2] {
        let built = build(
            dir.path(),
            BuildOptions {
                tests: false,
                opt,
                offline: false,
            },
        )
        .unwrap();
        let program = Program::from_bytes(&built.wasm).unwrap();
        let divide = &program.abi().exports["app/math.divide"];
        assert_eq!(
            divide.errors,
            [ilang_host::AbiType::Int, ilang_host::AbiType::String]
        );
        let check = &program.abi().exports["app/math.check"];
        assert_eq!(
            check.errors,
            [ilang_host::AbiType::Nil, ilang_host::AbiType::Bool]
        );
        let mut instance = program.instantiate(HostOptions::default()).unwrap();
        let mut call =
            |name: &str, args: &[Value]| format!("{:?}", instance.call(name, args).unwrap());
        let int = |n: i64| Value::Int(n.into());
        assert_eq!(
            call("app/math.divide", &[int(7), int(2)]),
            format!("{:?}", Outcome::Ok(int(3)))
        );
        assert_eq!(
            call("app/math.divide", &[int(7), int(0)]),
            format!(
                "{:?}",
                Outcome::Err(ilang_host::GuestError(Value::String(
                    "division by zero".into()
                )))
            )
        );
        assert_eq!(
            call("app/math.divide", &[int(7), int(1)]),
            format!("{:?}", Outcome::Err(ilang_host::GuestError(int(7))))
        );
        assert_eq!(
            call("app/math.check", &[int(0)]),
            format!("{:?}", Outcome::Err(ilang_host::GuestError(Value::Nil)))
        );
        assert_eq!(
            call("app/math.check", &[int(1)]),
            format!(
                "{:?}",
                Outcome::Err(ilang_host::GuestError(Value::Bool(true)))
            )
        );
        assert_eq!(
            call("app/math.check", &[int(2)]),
            format!("{:?}", Outcome::Ok(Value::Float(1.5)))
        );
        assert_eq!(
            call("app/math.bits", &[Value::Bool(false)]),
            format!("{:?}", Outcome::Ok(Value::Nil))
        );
        assert_eq!(
            call("app/math.bits", &[Value::Bool(true)]),
            format!(
                "{:?}",
                Outcome::Err(ilang_host::GuestError(Value::BitArray(
                    BitArray::new(vec![1, 2], 16).unwrap()
                )))
            )
        );
        assert_eq!(
            call("app/math.positive", &[int(3)]),
            format!("{:?}", Outcome::Ok(int(3)))
        );
        match instance.call("app/math.positive", &[int(0)]).unwrap() {
            Outcome::Stop(stop) => assert_eq!(
                stop.message.as_deref(),
                Some("assertion failed: assert\napp/math:26:3")
            ),
            other => panic!("{other:?}"),
        }
    }
}

/// Host exports with host services: a sync service with errors, and an
/// async service, whose exports are async (§10.2, B.5) and wait for the
/// host through `ilang_resume:…`; their results come from `ilang_result:…`.
#[test]
fn host_exports_with_services_and_async() {
    let dir = package(&[
        (
            "i.toml",
            "name = \"app\"\n\n[wasm]\nexports = [\"app/api.lookup\", \"app/api.quadruple\", \"app/api.both\", \"app/api.pure\"]\n",
        ),
        (
            "src/api.i",
            r#"import (i/core)

export (Kv, Later, lookup, quadruple, both, pure)

@host("test:kv/v1")
Kv { Kv(get: (key: String) -> String ? String) }

@host("test:async/v1", "async")
Later { Later(double: (n: Int) -> Int ? String, echo: (text: String) -> String) }

lookup = (key: String, kv ! Kv) { kv.get(key) }

quadruple = (n: Int, later ! Later) { later.double(later.double(n)) }

both = (key: String, kv ! Kv, later ! Later) {
  later.echo(kv.get(key) <> "!")
}

pure = (n: Int) { n + 1 }
"#,
        ),
    ]);
    let int = |n: i64| Value::Int(n.into());
    for opt in [Level::O0, Level::O2] {
        let built = build(
            dir.path(),
            BuildOptions {
                tests: false,
                opt,
                offline: false,
            },
        )
        .unwrap();
        let program = Program::from_bytes(&built.wasm).unwrap();
        let abi = program.abi();
        assert!(!abi.exports["app/api.lookup"].is_async);
        assert!(abi.exports["app/api.quadruple"].is_async);
        assert!(abi.exports["app/api.both"].is_async);
        assert!(!abi.exports["app/api.pure"].is_async);
        assert_eq!(
            abi.services.keys().collect::<Vec<_>>(),
            ["test:async/v1", "test:kv/v1"]
        );
        assert!(abi.services["test:async/v1"]["double"].is_async);
        assert!(!abi.services["test:kv/v1"]["get"].is_async);
        assert!(abi.externals.is_empty());
        let log = ilang_host::services::Log::default();
        let mut instance = program
            .instantiate(HostOptions::new(ilang_host::services::testing(log.clone())))
            .unwrap();
        let show = |o: Outcome| format!("{o:?}");
        assert_eq!(
            show(
                instance
                    .call("app/api.lookup", &[Value::String("name".into())])
                    .unwrap()
            ),
            show(Outcome::Ok(Value::String("i".into())))
        );
        assert_eq!(
            show(
                instance
                    .call("app/api.lookup", &[Value::String("x".into())])
                    .unwrap()
            ),
            show(Outcome::Err(ilang_host::GuestError(Value::String(
                "no x".into()
            ))))
        );
        // An async export must be called with `call_async`.
        assert!(instance.call("app/api.quadruple", &[int(3)]).is_err());
        let r = ilang_host::block_on(instance.call_async("app/api.quadruple", &[int(3)]));
        assert_eq!(show(r.unwrap()), show(Outcome::Ok(int(12))));
        let r = ilang_host::block_on(instance.call_async("app/api.quadruple", &[int(-3)]));
        assert_eq!(
            show(r.unwrap()),
            show(Outcome::Err(ilang_host::GuestError(Value::String(
                "negative".into()
            ))))
        );
        let r = ilang_host::block_on(
            instance.call_async("app/api.both", &[Value::String("lang".into())]),
        );
        assert_eq!(
            show(r.unwrap()),
            show(Outcome::Ok(Value::String("i language!".into())))
        );
        // The error of the sync service comes before the async call.
        let r =
            ilang_host::block_on(instance.call_async("app/api.both", &[Value::String("y".into())]));
        assert_eq!(
            show(r.unwrap()),
            show(Outcome::Err(ilang_host::GuestError(Value::String(
                "no y".into()
            ))))
        );
        assert_eq!(
            show(instance.call("app/api.pure", &[int(1)]).unwrap()),
            show(Outcome::Ok(int(2)))
        );
        assert_eq!(
            *log.lock().unwrap(),
            ["double 3", "double 6", "double -3", "echo i language!"]
        );
        assert!(instance.unusable().is_none());
    }
}

/// An entry point is async when it may call a function of an async host
/// service (§10.2); a test that gives a fake in its place is not (§9.7). The
/// metadata lists only the host services that the entry points need (B.6).
#[test]
fn async_entry_points_and_needed_services() {
    let spec = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/spec/9.7/fake_service.i");
    let text = std::fs::read_to_string(spec).unwrap();
    let dir = package(&[
        ("i.toml", "name = \"spec\"\n"),
        ("test/fake_service.i", &text),
        (
            "src/main.i",
            "@host(\"test:unused/v1\")\nUnused { Unused(f: () -> Nil) }\n\n@main\nmain = () { Nil }\n",
        ),
    ]);
    let built = build(
        dir.path(),
        BuildOptions {
            tests: true,
            opt: Level::O0,
            offline: false,
        },
    )
    .unwrap();
    let program = Program::from_bytes(&built.wasm).unwrap();
    let abi = program.abi();
    assert!(!abi.entry.as_ref().unwrap().is_async);
    let tests: Vec<(&str, bool)> = abi
        .tests
        .iter()
        .map(|t| (t.name.as_str(), t.is_async))
        .collect();
    assert_eq!(
        tests,
        [
            ("spec/fake_service.with_fakes", false),
            ("spec/fake_service.with_the_host", true)
        ]
    );
    // `Kv` is given by a fake in both tests, and nothing needs `Unused`.
    assert_eq!(abi.services.keys().collect::<Vec<_>>(), ["test:async/v1"]);
}

/// `tests/browser/fixtures/compiled.wasm`, which the JavaScript and browser
/// tests run, has the metadata that the compiler gives its package now.
/// `bash scripts/build-abi-fixture.sh` builds it again.
#[test]
fn the_compiled_fixture_is_current() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/browser/fixtures");
    let built = build(
        &fixtures.join("compiled"),
        BuildOptions {
            tests: false,
            opt: Level::O2,
            offline: false,
        },
    )
    .unwrap();
    let fresh = ilang_host::Abi::from_wasm(&built.wasm).unwrap().unwrap();
    let saved = std::fs::read(fixtures.join("compiled.wasm")).unwrap();
    let saved = ilang_host::Abi::from_wasm(&saved).unwrap().unwrap();
    assert_eq!(
        fresh, saved,
        "run `bash scripts/build-abi-fixture.sh` to build the fixture again"
    );
    assert!(fresh.entry.unwrap().is_async);
}

/// `i run` gives the standard host services: `examples/hello` prints.
#[test]
fn run_gives_the_standard_services() {
    let dir = package(&[
        ("i.toml", "name = \"app\"\n"),
        (
            "src/main.i",
            "export (Io)\n\n@host(\"i:io/v1\")\nIo { Io(print: (text: String) -> Nil) }\n\n@main\nmain = (io ! Io) {\n  io.print(\"one\")\n  io.print(\"two\")\n}\n",
        ),
        (
            "test/main_test.i",
            "import (app/main)\n\n@test\nprints = (io ! main.Io) { io.print(\"in a test\") }\n",
        ),
    ]);
    let out = i(&["run"], dir.path());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "one\ntwo\n");
    let out = i(&["test"], dir.path());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "in a test\npass app/main_test.prints\n1 passed, 0 failed\n"
    );
}

#[test]
fn an_error_stops_a_test() {
    let dir = package(&[
        ("i.toml", "name = \"app\"\n"),
        (
            "test/app_test.i",
            "import (i/fail)\n\nf = (n) { fail.raise(n) }\n\n@test\nraises = () {\n  f(\"oops\")\n  Nil\n}\n",
        ),
    ]);
    for opt in [Level::O0, Level::O2] {
        let built = build(
            dir.path(),
            BuildOptions {
                tests: true,
                opt,
                offline: false,
            },
        )
        .unwrap();
        let program = Program::from_bytes(&built.wasm).unwrap();
        match ilang_cli::run_test(&program, "app/app_test.raises") {
            ilang_host::TestResult::Stopped(stop) => assert_eq!(
                stop.message.as_deref(),
                Some("error: \"oops\"\napp/app_test:6:1")
            ),
            other => panic!("{other:?}"),
        }
    }
}

/// Runs git with no user configuration and a fixed identity.
fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .args([
            "-c",
            "init.defaultBranch=main",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(["-c", "tag.gpgsign=false"])
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A package with a location, got from a local bare repository: its source
/// positions have the `location@major ` prefix (§11.7), and `--offline` uses
/// only the cache.
#[test]
fn located_packages() {
    let w = tempfile::tempdir().unwrap();
    let work = w.path().join("work/lib");
    let bare = w.path().join("remotes/lib");
    std::fs::create_dir_all(work.join("src")).unwrap();
    std::fs::create_dir_all(&bare).unwrap();
    git(&work, &["init", "-q"]);
    git(&bare, &["init", "-q", "--bare"]);
    std::fs::write(work.join("i.toml"), "name = \"lib\"\n").unwrap();
    std::fs::write(
        work.join("src/lib.i"),
        "export (boom)\n\nboom = (m) {\n  panic(m)\n}\n",
    )
    .unwrap();
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-q", "-m", "v1.2.0"]);
    git(&work, &["tag", "v1.2.0"]);
    let remote = bare.to_str().unwrap();
    git(&work, &["push", "-q", remote, "HEAD:refs/heads/main"]);
    git(&work, &["push", "-q", "--tags", remote]);
    let app = package(&[
        (
            "i.toml",
            "name = \"app\"\n[dependencies]\nlib = \"example.com/lib@1.2.0\"\n",
        ),
        (
            "src/main.i",
            "import (lib/lib)\n\n@main\nmain = () {\n  lib.boom(\"x\")\n  Nil\n}\n",
        ),
    ]);
    let run = |cache: &str, args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_i"))
            .args(args)
            .arg("--project")
            .arg(app.path())
            .env_remove("ILANG_OFFLINE")
            .env(
                "ILANG_GIT_URLS",
                format!(
                    "example.com/=file://{}/",
                    w.path().join("remotes").display()
                ),
            )
            .env("ILANG_CACHE", w.path().join(cache))
            .output()
            .unwrap()
    };
    for opt in ["0", "2"] {
        let out = run("cache", &["run", "--opt", opt]);
        assert_eq!(
            String::from_utf8(out.stderr).unwrap(),
            "panic: x\nexample.com/lib@1 lib/lib:4:3\napp/main:5:3\n"
        );
    }
    // Everything is in the cache now.
    let out = run("cache", &["run", "--offline"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .starts_with("panic: x\n")
    );
    // With an empty cache, `--offline` is an error.
    for command in ["check", "build", "run", "test"] {
        let out = run("empty", &[command, "--offline"]);
        assert_eq!(out.status.code(), Some(1), "{command}");
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(err.contains("E0634"), "{command}: {err}");
    }
}
