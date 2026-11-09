//! The code generator on small programs: the shape of the code it makes,
//! the binary interface metadata, and the features that are not supported
//! yet. Running programs is tested by the spec suite in `tests/spec/`.

use ilang_wasm::{Error, Options, compile, compile_wat};

fn checked(
    source: &str,
) -> (
    ilang_package::PackageGraph,
    ilang_check::Checked,
    tempfile::TempDir,
) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.i"), source).unwrap();
    let (graph, diags) = ilang_package::load(dir.path());
    assert!(diags.is_empty(), "{diags:?}");
    let checked = ilang_check::check(&graph);
    let errors: Vec<_> = checked.diags.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "{errors:?}");
    (graph, checked, dir)
}

fn wat(source: &str) -> String {
    let (graph, c, _dir) = checked(source);
    compile_wat(
        &graph,
        c.program.as_ref().unwrap(),
        c.types.as_ref().unwrap(),
        Options { tests: false },
    )
    .unwrap()
}

fn wasm(source: &str) -> Result<Vec<u8>, Error> {
    let (graph, c, _dir) = checked(source);
    compile(
        &graph,
        c.program.as_ref().unwrap(),
        c.types.as_ref().unwrap(),
        Options { tests: false },
    )
}

/// The text of the function with this comment line.
fn function<'a>(wat: &'a str, name: &str) -> &'a str {
    let start = wat.find(&format!(";; {name}\n")).expect("the function");
    let rest = &wat[start + 3..];
    &rest[..rest.find("\n;; ").unwrap_or(rest.len())]
}

#[test]
fn tail_calls_are_return_calls() {
    let text = wat(
        "loop = (n) {\n  case n {\n    0 -> 0\n    _ -> loop(n - 1)\n  }\n}\n\
                    other = (n) { loop(n) + 1 }\n\
                    either = (n) { n == 0 || other(n) == 1 }\n\
                    right = (n) { n == 0 || right(n - 1) }\n\
                    @main\nmain = () { (either(1), right(1)) }\n",
    );
    let l = function(&text, "app/main.loop");
    assert!(l.contains("(return_call $f"), "{l}");
    let o = function(&text, "app/main.other");
    assert!(o.contains("(call $f") && !o.contains("return_call"), "{o}");
    // The right side of `||` in tail position is a tail position (§11.6),
    // but a call inside it is not.
    let e = function(&text, "app/main.either");
    assert!(!e.contains("return_call"), "{e}");
    let r = function(&text, "app/main.right");
    assert!(r.contains("(return_call $f"), "{r}");
    // Every frame is left before a tail call (§11.7).
    assert!(l.contains("(call $shadow_leave)\n(return_call"), "{l}");
}

#[test]
fn closures_capture_what_they_use() {
    let text = wat("make = (a, b) { (x) { x + a } }\n@main\nmain = () { make(1, 2)(3) }\n");
    let m = function(&text, "app/main.make");
    assert!(m.contains("(array.new_fixed $values 1"), "{m}");
    let main = function(&text, "app/main.main");
    assert!(main.contains("call_ref $fn1"), "{main}");
}

#[test]
fn metadata() {
    let bytes = wasm("@main\nmain = () { Nil }\n").unwrap();
    let mut found = None;
    for payload in wasmparser::Parser::new(0).parse_all(&bytes) {
        if let wasmparser::Payload::CustomSection(s) = payload.unwrap()
            && s.name() == "ilang.abi"
        {
            found = Some(String::from_utf8(s.data().to_vec()).unwrap());
        }
    }
    let json: serde_json::Value = serde_json::from_str(&found.unwrap()).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "version": 2,
            "entry": { "name": "app/main.main", "async": false },
            "tests": [],
            "exports": {},
            "services": {},
            "externals": {},
        })
    );
    // The exports of B.1 and the program entry of B.5.
    let mut exports = Vec::new();
    for payload in wasmparser::Parser::new(0).parse_all(&bytes) {
        if let wasmparser::Payload::ExportSection(s) = payload.unwrap() {
            for e in s {
                exports.push(e.unwrap().name.to_string());
            }
        }
    }
    for name in [
        "memory",
        "ilang_alloc",
        "ilang_free",
        "ilang_start",
        "app/main.main",
    ] {
        assert!(exports.iter().any(|e| e == name), "{name}: {exports:?}");
    }
}

#[test]
fn not_supported_yet() {
    let codes = |source: &str| match wasm(source) {
        Err(Error::Diagnostics(d)) => d.iter().map(|d| (d.code, d.message.clone())).collect(),
        other => panic!("{:?}", other.map(|_| ())),
    };
    let found: Vec<(&str, String)> = codes(
        "import (i/control)\nf = () { control.boundary((b) { 1 }) }\n@main\nmain = () { f() }\n",
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found.iter().all(|(c, _)| *c == "E0701"));
    // Functions that are not used are not compiled.
    let ok = wasm(
        "import (i/control)\nf = () { control.boundary((b) { 1 }) }\n@main\nmain = () { Nil }\n",
    );
    assert!(ok.is_ok());
}

#[test]
fn environments() {
    let text = wat("Clock { Clock(now: () -> Int) }\n\
         f = (c ! Clock) { c.now() }\n\
         g = () {\n  ! Clock = Clock(() { 1 })\n  f()\n}\n\
         @main\nmain = () { g() }\n");
    // An environment parameter reads its slot (§9.2); `Assert` is slot 0.
    let f = function(&text, "app/main.f");
    assert!(
        f.contains("(array.get $env (local.get $env) (i32.const 1))"),
        "{f}"
    );
    // An environment statement makes a new environment for the rest of
    // the block, and the tail call passes it.
    let g = function(&text, "app/main.g");
    assert!(
        g.contains("(call $env_with (local.get $env) (i32.const 1)"),
        "{g}"
    );
    assert!(g.contains("(return_call $f"), "{g}");
    // The entry point starts with the root environment, and stops for an
    // error that comes to it.
    assert!(text.contains("(global.get $root_env)"), "{text}");
    assert!(text.contains("(call $stop_error"), "{text}");
}

#[test]
fn bindings_that_may_fail_report_to_assert() {
    let text = wat("f = (r) {\n  Ok(v) = r\n  v\n}\n\
         g = ((a, b)) { a + b }\n\
         @main\nmain = () { (f(Ok(1)), g((1, 2))) }\n");
    let f = function(&text, "app/main.f");
    assert!(
        f.contains("(array.get $env (local.get $env) (i32.const 0))"),
        "{f}"
    );
    assert!(f.contains("$core_inspect"), "{f}");
    // A pattern that can not fail has no test.
    let g = function(&text, "app/main.g");
    assert!(!g.contains("array.get $env"), "{g}");
}
