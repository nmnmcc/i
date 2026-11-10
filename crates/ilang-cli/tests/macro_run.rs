//! The compile-time runner uses the opaque Syntax ABI in a fresh sandbox.
use std::collections::BTreeSet;

use ilang_syntax::lisp::{
    parse,
    syntax::{Scope, Syntax},
};

fn build(source: &str) -> ilang_cli::Built {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("i.toml"),
        "name = \"app\"\n[wasm]\nexports = [\"app/main.identity\"]\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("src/main.i"), source).unwrap();
    ilang_cli::build(
        dir.path(),
        ilang_cli::BuildOptions {
            tests: false,
            opt: ilang_cli::Level::O0,
            offline: true,
            target: ilang_cli::Target::Ilang,
        },
    )
    .unwrap()
}

fn input() -> Syntax {
    Syntax::from_datum(
        &parse("(f x)").unwrap()[0],
        &Scope {
            module: 7,
            marks: BTreeSet::from([42]),
        },
    )
}

#[test]
fn syntax_crosses_the_compiler_export_without_losing_identity() {
    let built = build("(syntax = i/syntax)\n(identity)\nidentity = (x: syntax.Syntax) { x }\n");
    let original = input();
    for _ in 0..2 {
        let result =
            ilang_cli::macro_run::invoke(&built, "app/main.identity", &[original.clone()]).unwrap();
        assert!(result.identical(&original));
        assert_eq!(result.scope(), original.scope());
    }
}

#[test]
fn untrusted_external_cannot_be_supplied_to_a_macro_instance() {
    let built = build(
        "(syntax = i/syntax)\n(identity)\n@external(\"example:outside\", \"effect\")\neffect: () -> Nil\nidentity = (x: syntax.Syntax) { effect() x }\n",
    );
    let error = ilang_cli::macro_run::invoke(&built, "app/main.identity", &[input()]).unwrap_err();
    assert!(format!("{error:#}").contains("instantiate macro program"));
}
