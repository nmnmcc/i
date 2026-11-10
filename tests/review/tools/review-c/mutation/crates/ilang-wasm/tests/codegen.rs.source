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
        Options::default(),
    )
    .unwrap()
}

fn wasm(source: &str) -> Result<Vec<u8>, Error> {
    let (graph, c, _dir) = checked(source);
    compile(
        &graph,
        c.program.as_ref().unwrap(),
        c.types.as_ref().unwrap(),
        Options::default(),
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
    // Every frame is left before a tail call, and the callee gets its
    // position as a tail site (§11.7).
    assert!(l.contains("(call $shadow_leave_tail (i32.const -"), "{l}");
    assert!(l.contains("))\n(return_call"), "{l}");
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
            "version": 3,
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

/// An external function is a Wasm import (§10.1, B.4), and the metadata
/// lists every external function, also one that is not used (B.6).
#[test]
fn externals_are_imports() {
    let external = "@external(\"app:math/v1\", \"sqrt\")\nsqrt: (x: Float) -> Float\n";
    for main in ["main = () { f()  Nil }", "main = () { Nil }"] {
        let bytes = wasm(&format!(
            "{external}f = () {{ sqrt(2.0) }}\n@main\n{main}\n"
        ))
        .unwrap();
        let mut imports = Vec::new();
        let mut abi = None;
        for payload in wasmparser::Parser::new(0).parse_all(&bytes) {
            match payload.unwrap() {
                wasmparser::Payload::ImportSection(s) => {
                    for i in s.into_imports() {
                        let i = i.unwrap();
                        imports.push(format!("{} {}", i.module, i.name));
                    }
                }
                wasmparser::Payload::CustomSection(s) if s.name() == "ilang.abi" => {
                    abi = Some(String::from_utf8(s.data().to_vec()).unwrap());
                }
                _ => {}
            }
        }
        assert!(
            imports.contains(&"app:math/v1 sqrt".to_string()),
            "{imports:?}"
        );
        let json: serde_json::Value = serde_json::from_str(&abi.unwrap()).unwrap();
        assert_eq!(
            json["externals"],
            serde_json::json!({ "app:math/v1": { "sqrt": { "params": ["float"], "result": "float" } } })
        );
    }
}

#[test]
fn calls_that_may_suspend_are_split() {
    let text = wat("import (i/control)\n\
         twice = (f) { f(1) + f(2) }\n\
         last = (f) { f(1) }\n\
         plain = (n) { n + 1 }\n\
         uses = (n) { plain(n) + twice((x) { x }) }\n\
         @main\nmain = () {\n  control.boundary((b) { uses(1) + last((x) { x }) })\n  Nil\n}\n");
    // Each call of a closure that is not a tail call is in a handler that
    // adds its frame record (§9.4), with a segment and a re-entry function.
    let twice = function(&text, "app/main.twice");
    assert_eq!(twice.matches("(catch_ref $capture").count(), 2, "{twice}");
    assert!(text.contains(";; app/main.twice after point"), "{text}");
    // A tail call leaves no frame, and a function that calls no closure and
    // nothing that may suspend is not split.
    let last = function(&text, "app/main.last");
    assert!(!last.contains("$capture"), "{last}");
    assert!(last.contains("return_call_ref"), "{last}");
    let plain = function(&text, "app/main.plain");
    assert!(!plain.contains("$capture"), "{plain}");
    // A known call of a function that may suspend may suspend, but not a
    // known call of one that does not.
    let uses = function(&text, "app/main.uses");
    assert_eq!(uses.matches("(catch_ref $capture").count(), 1, "{uses}");
    // `assertion.catch` is i code now.
    assert!(wasm("import (i/assertion)\n@main\nmain = () {\n  assertion.catch(() { assert(False) })\n  Nil\n}\n").is_ok());
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

/// A function with `n` calls that may suspend, each in a clause of its own
/// `case` (§9.4).
fn many_points(n: usize) -> String {
    let mut s = String::from(
        "import (i/control)\n\
         s = (b: Boundary(Int, Never, Never), v: Int) -> Int {\n  control.suspend(b, (k) { control.resume(k, v) })\n}\n\
         f = (b: Boundary(Int, Never, Never), x: Int) -> Int {\n  a0 = x\n",
    );
    for i in 1..=n {
        let p = i - 1;
        s.push_str(&format!(
            "  a{i} = case a{p} % 3 {{ 0 -> s(b, a{p} + 1)  _ -> a{p} + 2 }}\n"
        ));
    }
    s.push_str(&format!(
        "  a{n}\n}}\n@main\nmain = () {{ control.boundary((b) {{ f(b, 0) }})  Nil }}\n"
    ));
    s
}

#[test]
fn segments_grow_linearly() {
    // The code after each `case` with a point is one join that the
    // segments go on with, so it is not copied into every segment.
    let size = |n| {
        let text = wat(&many_points(n));
        assert_eq!(text.matches(";; app/main.f after point").count(), n);
        assert_eq!(text.contains(";; app/main.f join"), n > 0);
        text.len()
    };
    let (small, large) = (size(50), size(200));
    let base = size(0);
    let ratio = (large - base) as f64 / (small - base) as f64;
    assert!(ratio < 5.0, "{base} {small} {large}: {ratio}");
    let start = std::time::Instant::now();
    wasm(&many_points(200)).unwrap();
    assert!(start.elapsed().as_secs() < 30);
}
