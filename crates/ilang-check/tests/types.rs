//! Golden type tests. Each `tests/types/<case>.i` is the module `app/main`
//! of a package; each `tests/types/<case>/` with an `i.toml` is a package.
//! The shown type of every top-level name (§4.7), then the problems, are
//! compared with `<case>.out` or `<case>/expected.out`. `library.out` has
//! the types of the library modules. `UPDATE=1` writes the files.

use ilang_check::hir::{DefId, DefKind, ModuleId, Program, TypeRef};
use ilang_check::identity::{PackageIdentity, TypeIdentity, type_identity};
use ilang_check::types::{ShownType, Typed};
use ilang_package::{Diagnostic, PackageGraph};
use ilang_syntax::span::LineIndex;
use std::fs;
use std::path::Path;

fn output(graph: &PackageGraph, short: bool, library: bool) -> String {
    let checked = ilang_check::check(graph);
    let mut out = String::new();
    if let (Some(program), Some(types)) = (&checked.program, &checked.types) {
        same_as_text(program, types);
        for (i, m) in program.modules.iter().enumerate() {
            let wanted = if library {
                m.package == graph.library
            } else {
                Some(m.package) == graph.root
            };
            if !wanted {
                continue;
            }
            for line in types.lines(program, ilang_check::hir::ModuleId(i as u32)) {
                let line = if short {
                    line.strip_prefix("app/main.").unwrap_or(&line).to_string()
                } else {
                    line
                };
                out.push_str(&line);
                out.push('\n');
            }
        }
    }
    for d in &checked.diags {
        out.push_str(&render(graph, d, short));
        out.push('\n');
    }
    out
}

/// Each tree of `scheme_types` is written as the text of `schemes`, and
/// there is one for each exported name and each declaration that has a
/// shown type.
fn same_as_text(program: &Program, types: &Typed) {
    for (i, m) in program.modules.iter().enumerate() {
        let module = ModuleId(i as u32);
        let names = ilang_check::types::type_names(program, module);
        for (index, d) in m.defs.iter().enumerate() {
            let id = DefId {
                module,
                index: index as u32,
            };
            let wanted = d.exported || matches!(d.kind, DefKind::External { .. });
            match (types.schemes.get(&id), types.scheme_types.get(&id)) {
                (Some(text), Some(tree)) => {
                    assert!(wanted, "{}.{} has a tree", m.path, d.name);
                    assert_eq!(&tree.text(&names), text, "{}.{}", m.path, d.name);
                }
                (Some(_), None) => assert!(!wanted, "{}.{} has no tree", m.path, d.name),
                (None, Some(_)) => panic!("{}.{} has a tree but no text", m.path, d.name),
                (None, None) => {}
            }
        }
    }
}

fn render(graph: &PackageGraph, d: &Diagnostic, short: bool) -> String {
    if !short {
        return graph.sources.render(d);
    }
    let pos = d.source.map_or(String::new(), |s| {
        let (line, col) = LineIndex::new(&graph.sources.get(s).text).line_col(d.span.start);
        format!("{line}:{col}: ")
    });
    format!(
        "{pos}{}[{}]: {}",
        if d.is_error() { "error" } else { "warning" },
        d.code,
        d.message
    )
}

fn compare(path: &Path, actual: &str, failed: &mut Vec<String>) {
    if std::env::var_os("UPDATE").is_some() {
        fs::write(path, actual).unwrap();
        return;
    }
    let expected = fs::read_to_string(path).unwrap_or_default();
    if expected != actual {
        eprintln!("=== {} ===\n{actual}", path.display());
        failed.push(path.display().to_string());
    }
}

#[test]
fn types() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/types");
    let mut entries: Vec<_> = fs::read_dir(&base)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    let mut failed = Vec::new();
    let mut count = 0;
    for path in &entries {
        if path.extension().is_some_and(|e| e == "i") {
            let dir = tempfile::tempdir().unwrap();
            fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
            fs::create_dir_all(dir.path().join("src")).unwrap();
            fs::copy(path, dir.path().join("src/main.i")).unwrap();
            let (graph, diags) = ilang_package::load(dir.path());
            assert!(diags.is_empty());
            let actual = output(&graph, true, false);
            compare(&path.with_extension("out"), &actual, &mut failed);
            count += 1;
        } else if path.join("i.toml").exists() {
            let (graph, diags) = ilang_package::load(path);
            assert!(diags.is_empty());
            let actual = output(&graph, false, false);
            compare(&path.join("expected.out"), &actual, &mut failed);
            count += 1;
        }
    }
    assert!(count > 0);
    assert!(
        failed.is_empty(),
        "types differ: {failed:#?} (run with UPDATE=1 to bless)"
    );
}

/// The library modules type check with no problems, and their types are
/// those of the spec tables (§9, §12, §13).
#[test]
fn library() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    let (graph, _) = ilang_package::load(dir.path());
    let actual = output(&graph, false, true);
    let mut failed = Vec::new();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/types/library.out");
    compare(&path, &actual, &mut failed);
    assert!(
        failed.is_empty(),
        "library types differ (run with UPDATE=1 to bless)"
    );
}

/// A parameter used at many calls, each giving it an upper bound with one
/// type constructor (`many_uses.i`), checks in a time that grows slowly
/// with the number of calls: 200 calls took minutes when each two bounds
/// were made equal.
#[test]
fn many_uses_of_a_parameter() {
    let n = 200;
    let mut source = String::from(
        " (i/control)\ns = (b, v) { control.suspend(b, (k) { control.resume(k, v) }) }\nf = (b, x) {\n  a0 = x\n",
    );
    for i in 1..=n {
        let p = i - 1;
        source.push_str(&format!(
            "  a{i} = {{ a{p} % 3 }} |> {{ 0 -> s(b, a{p} + 1)  _ -> a{p} + 2 }}\n"
        ));
    }
    source.push_str(&format!("  a{n}\n}}\n"));
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/main.i"), source).unwrap();
    let (graph, diags) = ilang_package::load(dir.path());
    assert!(diags.is_empty());
    let start = std::time::Instant::now();
    let out = output(&graph, true, false);
    let elapsed = start.elapsed();
    assert!(
        out.contains("f : (b: Boundary(A, B, C), x: Int) -> Int\n") && !out.contains("error"),
        "{out}"
    );
    assert!(elapsed.as_secs() < 30, "{elapsed:?}");
}

#[test]
fn shown_names_skip_types_and_ignore_written_variable_names() {
    // A1 §8: first-use names skip unqualified types and never keep the
    // spelling of a declared variable. Error/environment roles keep E/N.
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    fs::write(
        dir.path().join("src/main.i"),
        "A { A }\nE { E }\nN { N }\nkeep(Z) = (v: Z) -> Z { v }\nagain(K) = (v: K) -> K { v }\neffect(X, Y, Z) = (f: () -> X ? Y ! Z) { f() }\n",
    )
    .unwrap();
    let (graph, diags) = ilang_package::load(dir.path());
    assert!(diags.is_empty());
    let out = output(&graph, true, false);
    assert!(out.contains("keep : (v: B) -> B\n"), "{out}");
    assert!(out.contains("again : (v: B) -> B\n"), "{out}");
    assert!(
        out.contains("effect : (f: () -> B ? E2 ! N2) -> B ? E2 ! N2\n"),
        "{out}"
    );
    assert!(!out.contains("error"), "{out}");
}

/// The tree of a type scheme names each custom type by its `TypeId`, whose
/// identity is its package, its module, and its name, though two modules
/// show it alike.
#[test]
fn scheme_types_know_each_type_by_what_it_is() {
    let tmp = tempfile::tempdir().unwrap();
    let write = |path: &str, text: &str| {
        let p = tmp.path().join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    };
    write("dep/i.toml", "name = \"dep\"\n");
    write("dep/src/u.i", " (T(..))\nT { T(String) }\n");
    write(
        "app/i.toml",
        "name = \"app\"\n[dependencies]\ndep = { path = \"../dep\" }\n",
    );
    write("app/src/geo/u.i", " (T(..))\nT { T(Int) }\n");
    write(
        "app/src/main.i",
        " (app/geo/u, d = dep/u)\n (f, g, ext)\n@external(\"env\", \"now\")\nnow: () -> Int\n@external(\"env\", \"ext\")\next: (n: Int) -> Int\nf(A) = (x: u.T, y: d.T, z: A) -> A { z }\ng = (o: Result(u.T, Int)) { o }\nhidden = (x: u.T) { x }\n",
    );
    let (graph, diags) = ilang_package::load(&tmp.path().join("app"));
    assert!(diags.is_empty(), "{diags:?}");
    let checked = ilang_check::check(&graph);
    assert!(checked.diags.is_empty(), "{:?}", checked.diags);
    let (program, types) = (checked.program.unwrap(), checked.types.unwrap());
    let main = program
        .modules
        .iter()
        .position(|m| m.path == "app/main")
        .unwrap();
    let def = |name: &str| {
        let m = program.module(ModuleId(main as u32));
        let index = m.defs.iter().position(|d| d.name == name).unwrap();
        DefId {
            module: ModuleId(main as u32),
            index: index as u32,
        }
    };
    let custom = |t: &ShownType| match t {
        ShownType::Named(TypeRef::Custom(id), _) => type_identity(&graph, &program, *id),
        other => panic!("not a custom type: {other:?}"),
    };
    let int = |t: &ShownType| matches!(t, ShownType::Named(TypeRef::Builtin(b), a) if b.name() == "Int" && a.is_empty());

    // `f`: both `T`s are named `T` in no module, but they are two types.
    let ShownType::Fn(f) = &types.scheme_types[&def("f")] else {
        panic!("f is not a function");
    };
    let labels: Vec<_> = f.params.iter().map(|p| p.0.as_deref()).collect();
    assert_eq!(labels, [Some("x"), Some("y"), Some("z")]);
    assert_eq!(
        custom(&f.params[0].1),
        TypeIdentity {
            package: PackageIdentity::Root,
            module: "geo/u".into(),
            name: "T".into(),
        }
    );
    let dep = graph.packages[graph.packages[graph.root.unwrap()].dependencies[0]
        .1
        .unwrap()]
    .dir
    .clone()
    .unwrap();
    assert_eq!(
        custom(&f.params[1].1),
        TypeIdentity {
            package: PackageIdentity::Path(dep),
            module: "u".into(),
            name: "T".into(),
        }
    );
    assert_eq!(f.params[2].1, ShownType::Var("A".into()));
    assert_eq!(f.ret, ShownType::Var("A".into()));
    assert_eq!((&f.err, &f.env), (&None, &None));

    // `g`: a type of the library.
    let ShownType::Fn(g) = &types.scheme_types[&def("g")] else {
        panic!("g is not a function");
    };
    let ShownType::Named(_, args) = &g.params[0].1 else {
        panic!("{:?}", g.params[0].1);
    };
    assert_eq!(custom(&g.params[0].1).package, PackageIdentity::Library);
    assert_eq!(custom(&g.params[0].1).name, "Result");
    assert_eq!(custom(&args[0]).module, "geo/u");

    // Declarations are there whether exported or not; other names that
    // are not exported are not.
    let ShownType::Fn(now) = &types.scheme_types[&def("now")] else {
        panic!("now is not a function");
    };
    assert!(now.params.is_empty() && int(&now.ret));
    let ShownType::Fn(e) = &types.scheme_types[&def("ext")] else {
        panic!("ext is not a function");
    };
    assert!(int(&e.params[0].1) && int(&e.ret));
    assert!(!types.scheme_types.contains_key(&def("hidden")));
    assert!(types.schemes.contains_key(&def("hidden")));
}
