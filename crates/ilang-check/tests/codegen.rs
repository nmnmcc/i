//! The data that type checking gives to the code generator (`Typed`): field
//! accesses, the order of arguments, type tests, services, and which
//! top-level bindings are generalized.

use ilang_check::hir::{BuiltinType, ModuleId, TypeRef};
use ilang_check::types::{Access, Head, Typed};
use ilang_package::PackageGraph;
use ilang_syntax::span::Span;
use std::collections::HashMap;
use std::fs;

const SOURCE: &str = r#"
Cat { Cat(lives: Int, name: String) }
Dog { Dog(name: String) }
Clock { Clock(now: () -> Int) }
Watch = Clock

name = (p: Cat | Dog) { p.name }
second = (t: (Int, String)) { t.1 }
minus = (a: Int, b: Int) { a - b }
by_label = minus(b: 1, a: 2)
piped = 5 |> minus(b: 1)
tests = (x) {
  case x {
    _: Int | String -> 1
    _: { (Int, Int) } -> 2
    _ -> 3
  }
}
env = (c ! Watch) {
  ! Clock = c
  c.now()
}
mono = minus(1, 2)
"#;

/// The checked program, with the text of each span of the main module.
fn check() -> (PackageGraph, Typed, ModuleId) {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/main.i"), SOURCE).unwrap();
    let (graph, _) = ilang_package::load(dir.path());
    let checked = ilang_check::check(&graph);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        checked.diags
    );
    let program = checked.program.unwrap();
    let m = program
        .modules
        .iter()
        .position(|m| m.path == "app/main")
        .unwrap();
    (graph, checked.types.unwrap(), ModuleId(m as u32))
}

fn by_text<'a, T>(map: &'a HashMap<(ModuleId, Span), T>, m: ModuleId, text: &str) -> &'a T {
    map.iter()
        .find(|((mm, s), _)| *mm == m && &SOURCE[s.start as usize..s.end as usize] == text)
        .map(|(_, v)| v)
        .unwrap_or_else(|| panic!("nothing for `{text}`"))
}

#[test]
fn codegen_data() {
    let (_graph, typed, m) = check();
    // `p.name`: the field index for each constructor that may have made `p`.
    let Access::Field(fields) = by_text(&typed.access, m, "p.name") else {
        panic!("not a field");
    };
    let indexes: Vec<u32> = fields.iter().map(|(_, i)| *i).collect();
    assert_eq!(indexes, vec![1, 0]);
    assert_ne!(fields[0].0, fields[1].0);
    assert_eq!(fields[0].0.index, 0);
    assert_eq!(by_text(&typed.access, m, "t.1"), &Access::Part(1));
    // The argument that goes to each parameter.
    assert_eq!(by_text(&typed.args, m, "minus(b: 1, a: 2)"), &vec![1, 0]);
    assert_eq!(by_text(&typed.args, m, "5 |> minus(b: 1)"), &vec![0, 1]);
    assert_eq!(by_text(&typed.args, m, "minus(1, 2)"), &vec![0, 1]);
    // The heads of each type test.
    let int = Head::Named(TypeRef::Builtin(BuiltinType::Int));
    let string = Head::Named(TypeRef::Builtin(BuiltinType::String));
    assert_eq!(
        by_text(&typed.type_tests, m, "_: Int | String"),
        &vec![int, string]
    );
    assert_eq!(
        by_text(&typed.type_tests, m, "_: { (Int, Int) }"),
        &vec![Head::Tuple(2)]
    );
    // Services, through an alias.
    let clock = *by_text(&typed.services, m, "c");
    assert_eq!(by_text(&typed.services, m, "! Clock = c"), &clock);
    assert!(matches!(clock, TypeRef::Custom(_)));
    // Generalized top-level bindings.
    let generalized: Vec<bool> = {
        let mut v: Vec<_> = typed
            .generalized
            .iter()
            .filter(|(b, _)| b.module == m)
            .map(|(b, g)| (b.index, *g))
            .collect();
        v.sort();
        v.into_iter().map(|x| x.1).collect()
    };
    // name, second, minus, by_label, piped, tests, env, mono
    assert_eq!(
        generalized,
        vec![true, true, true, false, false, true, true, false]
    );
}
