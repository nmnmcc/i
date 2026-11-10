//! Run i/syntax through ordinary i calls and the opaque-reference ABI.
use ilang_syntax::lisp::{
    parse,
    syntax::{Scope, Syntax},
};
use std::collections::BTreeSet;

#[test]
fn syntax_library_round_trips_through_the_host() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("test")).unwrap();
    std::fs::write(
        dir.path().join("test/main.i"),
        r#"
(i/fail, syntax = i/syntax)
@external("test:syntax", "source")
source: () -> syntax.Syntax
@test
exercise = () -> Nil ^ syntax.SyntaxError ! Assert {
  value = source()
  assert(syntax.show(value) == "(f x)")
  syntax.node(value).{
    syntax.Round([first, second]) -> {
      assert(syntax.show(first) == "f")
      assert(syntax.show(second) == "x")
    }
    _ -> panic("unexpected syntax node")
  }
  made = syntax.make(syntax.Name("made"), value)
  assert(syntax.show(made) == "made")
  assert(syntax.same_code(made, made))
  assert(syntax.show_as(made, syntax.Expression) == "made")
  pair = syntax.make(syntax.Round([made, value]), value)
  assert(syntax.show(pair) == "(made (f x))")
  fail.catch(() { syntax.make(syntax.Name("Bad Name"), value) }).{
    Error(syntax.SyntaxError(message, at)) -> {
      assert(message != "")
      assert(syntax.same_code(at, value))
    }
    Ok(_) -> panic("invalid syntax atom was accepted")
  }
  comma = syntax.make(syntax.Symbol(","), value)
  bad = syntax.make(syntax.Round([comma]), value)
  fail.catch(() { syntax.show_as(bad, syntax.Expression) }).{
    Error(syntax.SyntaxError(message, at)) -> {
      assert(message != "")
      assert(syntax.same_code(at, bad))
    }
    Ok(_) -> panic("invalid expression was accepted")
  }
  Nil
}
"#,
    )
    .unwrap();
    let source = Syntax::from_datum(
        &parse("(f x)").unwrap()[0],
        &Scope {
            module: 3,
            marks: BTreeSet::new(),
        },
    );
    for opt in [ilang_cli::Level::O0, ilang_cli::Level::O2] {
        let built = ilang_cli::build(
            dir.path(),
            ilang_cli::BuildOptions {
                tests: true,
                opt,
                offline: true,
                target: ilang_cli::Target::Ilang,
            },
        )
        .unwrap();
        let mut options = built.host_options();
        let source = source.clone();
        options
            .imports
            .external("test:syntax", "source", move |_| {
                Ok(ilang_host::syntax::encode(source.clone()))
            })
            .unwrap();
        let program = built.host_program().unwrap();
        let result = program.run_test_with("app/main.exercise", options);
        assert!(
            matches!(result, ilang_host::TestResult::Passed),
            "{result:?}"
        );
    }
}

#[test]
fn compiler_operations_are_limited_to_compiler_instances() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/helper.i"),
        r#"
(syntax = i/syntax)
(show)
@external("test:syntax", "source")
source: () -> syntax.Syntax
show = () { syntax.show(source()) }
"#,
    )
    .unwrap();
    let main = dir.path().join("src/main.i");
    let options = ilang_cli::BuildOptions {
        tests: false,
        opt: ilang_cli::Level::O0,
        offline: true,
        target: ilang_cli::Target::Ilang,
    };
    std::fs::write(
        &main,
        "(helper = app/helper)\nmain = () { helper.show()  Nil }\n",
    )
    .unwrap();
    let error = match ilang_cli::build(dir.path(), options) {
        Ok(_) => panic!("ordinary entry acquired a compiler capability"),
        Err(e) => e,
    };
    assert!(
        matches!(&error, ilang_cli::BuildError::Program { codes, .. } if codes.contains(&"E0322")),
        "{error}"
    );
    std::fs::write(
        &main,
        "(helper = app/helper)\nsnapshot = helper.show()\nmain = () { Nil }\n",
    )
    .unwrap();
    let error = match ilang_cli::build(dir.path(), options) {
        Ok(_) => panic!("ordinary initialization acquired a compiler capability"),
        Err(e) => e,
    };
    assert!(
        matches!(&error, ilang_cli::BuildError::Program { codes, .. } if codes.contains(&"E0322")),
        "{error}"
    );
    std::fs::write(&main, "(helper = app/helper)\napply = (f) { f() }\nsnapshot = apply(() { helper.show() })\nmain = () { Nil }\n").unwrap();
    let error = match ilang_cli::build(dir.path(), options) {
        Ok(_) => panic!("ordinary initialization called a compiler capability through a closure"),
        Err(e) => e,
    };
    assert!(
        matches!(&error, ilang_cli::BuildError::Program { codes, .. } if codes.contains(&"E0322")),
        "{error}"
    );
    std::fs::write(
        &main,
        "(helper = app/helper)\nunused = () { helper.show() }\nmain = () { Nil }\n",
    )
    .unwrap();
    ilang_cli::build(dir.path(), options).unwrap();
}

#[test]
fn syntax_values_cross_export_boundary() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("i.toml"),
        "name = \"app\"\n[wasm]\nexports = [\"app/main.identity\"]\n",
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(syntax = i/syntax)\n(identity)\nidentity = (x: syntax.Syntax) { x }\n",
    )
    .unwrap();
    let built = ilang_cli::build(
        dir.path(),
        ilang_cli::BuildOptions {
            tests: false,
            opt: ilang_cli::Level::O0,
            offline: true,
            target: ilang_cli::Target::Ilang,
        },
    )
    .unwrap();
    let mut instance = built
        .host_program()
        .unwrap()
        .instantiate(built.host_options())
        .unwrap();
    let input = Syntax::from_datum(
        &parse("(a b)").unwrap()[0],
        &Scope {
            module: 8,
            marks: BTreeSet::new(),
        },
    );
    let result = instance
        .call(
            "app/main.identity",
            &[ilang_host::syntax::encode(input.clone())],
        )
        .unwrap();
    let ilang_host::Outcome::Ok(value) = result else {
        panic!("export stopped: {result:?}")
    };
    assert!(
        ilang_host::syntax::decode(&value)
            .unwrap()
            .same_code(&input)
    );
}
