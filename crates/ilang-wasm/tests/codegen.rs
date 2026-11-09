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
                    @main\nmain = () { either(1) }\n",
    );
    let l = function(&text, "app/main.loop");
    assert!(l.contains("(return_call $f"), "{l}");
    let o = function(&text, "app/main.other");
    assert!(o.contains("(call $f") && !o.contains("return_call"), "{o}");
    // The right side of `||` is not a tail position (§11.6).
    let e = function(&text, "app/main.either");
    assert!(!e.contains("return_call"), "{e}");
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
        "Clock { Clock(now: () -> Int) }\nf = (c ! Clock) { c.now() }\n@main\nmain = () {\n  ! Clock = Clock(() { 1 })\n  f()\n}\n",
    );
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found.iter().all(|(c, _)| *c == "E0701"));
    // Functions that are not used are not compiled.
    let ok = wasm(
        "Clock { Clock(now: () -> Int) }\nf = (c ! Clock) { c.now() }\n@main\nmain = () { Nil }\n",
    );
    assert!(ok.is_ok());
}
