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
  assert(source() == value)
  ! syntax.Definitions = syntax.Definitions((of) { of })
  assert(syntax.definition(value) == value)
  assert(syntax.show(value) == "(f x)")
  syntax.node(value).{
    syntax.Round([first, second]) -> {
      syntax.node(value).{
        syntax.Round([again, _]) -> assert(first == again)
        _ -> panic("unexpected syntax node")
      }
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
    std::fs::write(
        &main,
        r#"
(syntax = i/syntax)
@external("test:syntax", "source")
source: () -> syntax.Syntax
main = () -> Nil ^ syntax.SyntaxError {
  ! syntax.Definitions = syntax.Definitions((s) { s })
  syntax.definition(source())
  Nil
}
"#,
    )
    .unwrap();
    let error = match ilang_cli::build(dir.path(), options) {
        Ok(_) => panic!("ordinary entry called a compiler-only helper"),
        Err(e) => e,
    };
    assert!(
        matches!(&error, ilang_cli::BuildError::Program { codes, .. } if codes.contains(&"E0322")),
        "{error}"
    );
    std::fs::write(
        &main,
        "@external(\"i:compiler/syntax/v1\", \"show\")\nforged: () -> String\nmain = () { Nil }\n",
    )
    .unwrap();
    let error = match ilang_cli::build(dir.path(), options) {
        Ok(_) => panic!("program declared a compiler-owned external"),
        Err(e) => e,
    };
    assert!(
        matches!(&error, ilang_cli::BuildError::Program { codes, .. } if codes.contains(&"E0306")),
        "{error}"
    );
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

#[test]
fn compiler_templates_rebuild_through_wasm() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("test")).unwrap();
    std::fs::write(
        dir.path().join("test/main.i"),
        r#"
(syntax = i/syntax)
make_many = (value: syntax.Syntax, n: Int, acc: List(syntax.Syntax)) -> List(syntax.Syntax) {
  n.{ 0 -> acc _ -> make_many(value, n - 1, [value, ..acc]) }
}
count = (items: List(syntax.Syntax), n: Int) -> Int {
  items.{ [] -> n [_, ..rest] -> count(rest, n + 1) }
}
@test
exercise = () {
  template = syntax._template(0)
  first = syntax._template(1)
  replacement = syntax._template(2)
  result = syntax._rebuild(template, [first, replacement])
  combined = syntax._append([first, replacement], [first])
  assert(syntax.show(syntax._rebuild(template, combined)) == "(f y f)")
  assert(count(syntax._append(make_many(first, 3000, []), [replacement]), 0) == 3001)
  assert(syntax.show(result) == "(f y)")
  syntax.node(result).{
    syntax.Round([_, child]) -> assert(child == replacement)
    _ -> panic("unexpected template")
  }
  Nil
}
"#,
    )
    .unwrap();
    let parsed = parse("(f x) y").unwrap();
    let scope = Scope {
        module: 8,
        marks: BTreeSet::new(),
    };
    let template = Syntax::from_datum(&parsed[0], &scope);
    let ilang_syntax::lisp::syntax::Node::Round(children) = template.node() else {
        panic!("template")
    };
    let templates: std::sync::Arc<[Syntax]> = vec![
        template.clone(),
        children[0].clone(),
        Syntax::from_datum(&parsed[1], &scope),
    ]
    .into();
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
        let mut options = ilang_cli::standard_options();
        ilang_host::syntax::install_with_templates(&mut options.imports, templates.clone())
            .unwrap();
        options
            .imports
            .grant_package(&built.binding_plan, &built.trusted_package);
        let result = built
            .host_program()
            .unwrap()
            .run_test_with("app/main.exercise", options);
        assert!(
            matches!(result, ilang_host::TestResult::Passed),
            "{result:?}"
        );
    }
}

#[test]
fn compiler_quote_patterns_capture_nested_nodes_and_splices() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("test")).unwrap();
    std::fs::write(
        dir.path().join("test/main.i"),
        r#"
(syntax = i/syntax)
@test
exercise = () {
  pattern = syntax._template(0)
  input = syntax._template(1)
  matched = syntax._match(pattern, input)
  assert(syntax._match_ok(matched))
  assert(syntax.show(syntax._match_one(matched, 0)) == "x")
  syntax._match_many(matched, 1).{
    [a, b] -> {
      assert(syntax.show(a) == "y")
      assert(syntax.show(b) == "z")
    }
    _ -> panic("wrong splice captures")
  }
  failed = syntax._match(pattern, syntax._template(2))
  assert(syntax._match_ok(failed) == False)
  Nil
}
"#,
    )
    .unwrap();
    let parsed = parse("`(f (g ~a) ~..rest) (f (g x) y z) (h x)").unwrap();
    let scope = Scope {
        module: 9,
        marks: BTreeSet::new(),
    };
    let templates: std::sync::Arc<[Syntax]> = parsed
        .iter()
        .map(|d| Syntax::from_datum(d, &scope))
        .collect::<Vec<_>>()
        .into();
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
        let mut options = ilang_cli::standard_options();
        ilang_host::syntax::install_with_templates(&mut options.imports, templates.clone())
            .unwrap();
        options
            .imports
            .grant_package(&built.binding_plan, &built.trusted_package);
        let result = built
            .host_program()
            .unwrap()
            .run_test_with("app/main.exercise", options);
        assert!(
            matches!(result, ilang_host::TestResult::Passed),
            "{result:?}"
        );
    }
}
