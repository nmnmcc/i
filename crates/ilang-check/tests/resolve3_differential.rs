//! The resolver of the syntax-three tree (`resolve3`) against the resolver
//! of the lowered tree (`resolve` after `syn3::lower`), for every package
//! under `std/`, `examples/` and `tests/spec` (with the library `lib/`): the
//! same HIR and the same diagnostics (docs/design/syn3-final-ast.md, step 3).
//!
//! The comparison is by file: the module, its types and its diagnostics.
//! The first difference of each file is printed, and the count of equal
//! files.

use ilang_check::hir::{ModuleId, Program};
use ilang_check::{Parsed, resolve};
use ilang_package::{Diagnostic, LoadOptions, PackageGraph};
use ilang_syntax::syn3;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The texts that must be equal, by file. The key `<program>` holds the
/// parts of the program that belong to no module.
fn texts(
    graph: &PackageGraph,
    program: &Program,
    diags: &[Diagnostic],
    pretty: bool,
) -> BTreeMap<String, String> {
    let show = |d: &dyn std::fmt::Debug| {
        if pretty {
            format!("{d:#?}\n")
        } else {
            format!("{d:?}\n")
        }
    };
    let mut out = BTreeMap::new();
    for (i, module) in program.modules.iter().enumerate() {
        let id = ModuleId(i as u32);
        let mut text = show(module);
        for t in &program.types {
            if t.module == id {
                text.push_str(&show(t));
            }
        }
        for d in diags {
            if d.source == Some(module.source) {
                text.push_str(&format!(
                    "{:?} {} {:?} {}\n",
                    d.severity, d.code, d.span, d.message
                ));
            }
        }
        out.insert(graph.sources.get(module.source).display.clone(), text);
    }
    let sources: BTreeSet<_> = program.modules.iter().map(|m| m.source).collect();
    let mut text = String::new();
    for d in diags {
        if !d.source.is_some_and(|s| sources.contains(&s)) {
            text.push_str(&format!(
                "{:?} {} {:?} {:?} {}\n",
                d.severity, d.code, d.source, d.span, d.message
            ));
        }
    }
    let sorted = |mut lines: Vec<String>| {
        lines.sort();
        lines.join("\n")
    };
    text.push_str(&format!(
        "of_sites {:#?}\nalias_patterns {:#?}\nalias_dependencies {:?}\nalias_sources {:?}\n",
        program.of_sites,
        program.alias_patterns,
        program.alias_dependencies,
        program.alias_sources
    ));
    text.push_str(&format!(
        "prelude {:?} root {:?} host_exports {:?} declared_entries {:?} vars {} ty_vars {} {:?}\n",
        program.prelude,
        program.root,
        program.host_exports,
        program.declared_entries,
        program.var_count,
        program.ty_var_count,
        program.ty_var_names
    ));
    text.push_str(&format!(
        "typed_macros {}\nfailed_reads {}\nhost_fns {}\nexternal_fns {}\n",
        sorted(
            program
                .typed_macros
                .iter()
                .map(|d| format!("{d:?}"))
                .collect()
        ),
        sorted(
            program
                .failed_reads
                .iter()
                .map(|d| format!("{d:?}"))
                .collect()
        ),
        sorted(program.host_fns.iter().map(|d| format!("{d:?}")).collect()),
        sorted(
            program
                .external_fns
                .iter()
                .map(|d| format!("{d:?}"))
                .collect()
        ),
    ));
    out.insert("<program>".into(), text);
    out
}

fn first_difference(a: &str, b: &str) -> String {
    let a: Vec<&str> = a.lines().collect();
    let b: Vec<&str> = b.lines().collect();
    let at = (0..a.len().max(b.len()))
        .find(|i| a.get(*i) != b.get(*i))
        .unwrap_or(0);
    let show = |lines: &[&str]| {
        let from = at.saturating_sub(4);
        let to = (at + 4).min(lines.len());
        lines[from.min(to)..to].join("\n      ")
    };
    format!(
        "line {at}\n    lowered:\n      {}\n    direct:\n      {}",
        show(&a),
        show(&b)
    )
}

#[derive(Default)]
struct Totals {
    files: BTreeSet<String>,
    differ: BTreeSet<String>,
    reports: Vec<(String, String)>,
    /// What was compared: bindings, diagnostics of the resolver, errors of
    /// the lowering.
    bindings: usize,
    diagnostics: usize,
    lowering: usize,
}

fn compare(name: &str, dir: &Path, totals: &mut Totals) {
    let options = LoadOptions {
        tests: true,
        ..LoadOptions::from_env()
    };
    let (graph, _) = ilang_package::load_with(dir, &options);
    let mut lowered = Vec::new();
    let mut direct = Vec::new();
    let mut modules = Vec::new();
    let mut expected = Vec::new();
    for (package, p) in graph.packages.iter().enumerate() {
        for m in &p.modules {
            let source = graph.sources.get(m.source);
            let lisp = source.display.ends_with(".is");
            let (module, _) = if lisp {
                syn3::lisp::from_lisp(&source.text)
            } else {
                syn3::parse_module(&source.text)
            };
            let (ast, errors) = if lisp {
                syn3::lower::lower_lisp(&module)
            } else {
                syn3::lower::lower(&module)
            };
            expected.extend(errors.into_iter().map(|d| {
                Diagnostic::error(d.code.as_str(), Some(m.source), d.span, d.message)
            }));
            let parsed = |ast| Parsed {
                package,
                path: m.path.clone(),
                source: m.source,
                kind: m.kind,
                ast,
                scoped: None,
                compiler_generated: false,
                typed_macros: None,
            };
            lowered.push(parsed(ast));
            direct.push(parsed(Default::default()));
            modules.push(module);
        }
    }
    let (old_program, old_diags) = resolve::resolve(&graph, &lowered);
    totals.lowering += expected.len();
    totals.diagnostics += old_diags.len();
    totals.bindings += old_program
        .modules
        .iter()
        .map(|m| m.bindings.len())
        .sum::<usize>();
    expected.extend(old_diags);
    let (new_program, new_diags) = resolve::resolve3(&graph, &direct, &modules);
    let old = texts(&graph, &old_program, &expected, false);
    let new = texts(&graph, &new_program, &new_diags, false);
    if old == new {
        totals.files.extend(old.into_keys().map(|file| {
            if file == "<program>" {
                format!("<program of {name}>")
            } else {
                file
            }
        }));
        return;
    }
    let old = texts(&graph, &old_program, &expected, true);
    let new = texts(&graph, &new_program, &new_diags, true);
    for (file, text) in &old {
        let key = if file == "<program>" {
            format!("<program of {name}>")
        } else {
            file.clone()
        };
        totals.files.insert(key.clone());
        let other = new.get(file).map_or("", |t| t.as_str());
        if text != other && totals.differ.insert(key.clone()) {
            let report = format!("{key} (in {name}): {}", first_difference(text, other));
            totals.reports.push((key, report));
        }
    }
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .collect();
    out.sort();
    out
}

#[test]
fn resolve3_differential() {
    let root = root();
    let only = std::env::var("RESOLVE3_ONLY").ok();
    let wanted = |name: &str| only.as_ref().is_none_or(|o| name.contains(o.as_str()));
    // The packages: a name, a directory, and the directory to keep for a
    // lone file.
    let mut jobs: Vec<(String, PathBuf, Option<tempfile::TempDir>)> = Vec::new();
    if wanted("std") {
        jobs.push(("std".into(), root.join("std"), None));
    }
    for dir in subdirs(&root.join("examples")) {
        let name = format!("examples/{}", dir.file_name().unwrap().to_string_lossy());
        if dir.join("i.toml").exists() && wanted(&name) {
            jobs.push((name, dir, None));
        }
    }
    for section in subdirs(&root.join("tests/spec")) {
        if !section.is_dir() {
            continue;
        }
        for case in subdirs(&section) {
            let name = case
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            if !wanted(&name) {
                continue;
            }
            if case.is_dir() {
                if case.join("i.toml").exists() {
                    jobs.push((name, case, None));
                }
            } else if case.extension().is_some_and(|e| e == "i") {
                // A lone file is `src/main.i` of a package, as in the spec
                // suite.
                let text = std::fs::read_to_string(&case).unwrap();
                let dir = tempfile::tempdir().unwrap();
                let manifest = if text.contains("std/test") {
                    format!(
                        "name = \"spec\"\n[test]\ncollection = \"std/test.tests\"\n[dependencies]\nstd = {{ path = {:?} }}\n",
                        root.join("std").to_str().unwrap()
                    )
                } else {
                    "name = \"spec\"\n".to_string()
                };
                std::fs::write(dir.path().join("i.toml"), manifest).unwrap();
                std::fs::create_dir_all(dir.path().join("src")).unwrap();
                std::fs::write(dir.path().join("src/main.i"), &text).unwrap();
                jobs.push((name, dir.path().to_path_buf(), Some(dir)));
            }
        }
    }
    // Forms that the packages above write seldom or never: every error of
    // the lowering, sums, keys, provides, row variables.
    if wanted("extra") {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("i.toml"), "name = \"extra\"\n").unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        for (i, text) in EXTRA.iter().enumerate() {
            let (_, errors) = syn3::parse_module(text);
            if std::env::var("RESOLVE3_SHOW").is_ok() {
                eprintln!("extra/m{i}: {} syntax errors {:?}", errors.len(), errors.first());
            }
            std::fs::write(dir.path().join(format!("src/m{i}.i")), text).unwrap();
        }
        jobs.push(("extra".into(), dir.path().to_path_buf(), Some(dir)));
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let all = std::sync::Mutex::new(Totals::default());
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let Some((name, dir, _)) = jobs.get(i) else {
                        break;
                    };
                    let mut totals = Totals::default();
                    compare(name, dir, &mut totals);
                    let mut all = all.lock().unwrap();
                    all.files.extend(totals.files);
                    all.bindings += totals.bindings;
                    all.diagnostics += totals.diagnostics;
                    all.lowering += totals.lowering;
                    for (file, report) in totals.reports {
                        if all.differ.insert(file.clone()) {
                            all.reports.push((file, report));
                        }
                    }
                }
            });
        }
    });
    let mut totals = all.into_inner().unwrap();
    totals.reports.sort();
    for (_, report) in &totals.reports {
        eprintln!("DIFFERENT {report}\n");
    }
    let equal = totals.files.len() - totals.differ.len();
    eprintln!(
        "resolve3 differential: {equal} files equal / {} total ({} top-level bindings, {} resolver diagnostics, {} lowering errors over {} packages)",
        totals.files.len(),
        totals.bindings,
        totals.diagnostics,
        totals.lowering,
        jobs.len()
    );
    assert!(only.is_some() || totals.bindings > 1000 && totals.diagnostics > 0);
    assert!(
        totals.differ.is_empty(),
        "{} files differ",
        totals.differ.len()
    );
}

/// One module each; a module with a syntax error is still compared, on the
/// part of the tree that was read.
const EXTRA: &[&str] = &[
    // Sums, data types, aliases, exports of members.
    "(Shape(..), Circle, Pair(..), area, swap)\n\nCircle = (radius = Float)\nSquare = (side = Float)\nShape = Circle | Square\nPair(A, B) = (first = A, second = B)\nToken = Opaque\nId(A) = A\nBoth = (Int, Int)\nRec = (a = Int, b = Int)\n\narea = (s: Shape) -> Float {\n	s |> (Circle(radius = r)) { r } | (Square(side = _)) { 0.0 }\n}\nswap = <A, B>(p: Pair(A, B)) -> Pair(B, A) { Pair(first = p.second, second = p.first) }\n",
    // Keys in constructor patterns.
    "P = (x = Int, y = Int)\nf = (P(x = x, y = other)) { x + other }\ng = (P(x = _, y = y: Int)) { y }\nh = (P(x = a = b, ..)) { a + b }\nk = (P(x = whole: Int, y = _)) { whole }\nm = (p: P) { P(..p, x = 1) }\nn = (P(x = 0, y = y)) { y }\n",
    // Chains: clauses, guards, several positions, alternatives.
    "f = (0) { 1 } | (n) { n }\ng = (x, y ? x > y) { x } | (x, y) { y }\nh = (1 | 2, a) { a } | (_, a) { a + 1 }\nk = (a = b, c) { (a, b, c) }\nm = (x) { x |> (0) { 1 } | (n ? n > 2) { 2 } | (_) { 3 } }\np = (x) { x |> (y) { y + 1 } }\nq = (x: Int) -> Int { x } | (x) { x }\nr = (x) { 1 } | (x, y) { 2 }\ns = (? True) { 1 }\nt = (x) { x |> (a, b) { a } | (c) { c } }\n",
    // Data, calls, strings, index.
    "f = (x) { \"a\\(x)b\\(x)\" }\ng = (x) { \"\\(x)\" }\nh = () { a = (1, 2)  a.0 }\nk = () { (a = 1, b = 2) }\nm = (r) { (..r, a = 1) }\nn = () { (a = 2, 1) }\np = (f) { f(1, ..f) }\nq = () { () }\nr = (x) { x |> Some }\ns = (x) { core.never(x) }\nt = core.never\n",
    // Services and provides.
    "A = (n = Int)\nB = (m = Int)\nuse = () <- (a: A, b: B) -> Int { a.n + b.m }\none = () { use() <- (A(n = 1), B(m = 2)) }\ntwo = (a: A, b: B) { use() <- (a, b) }\nthree = () { { x = 1  use() + x } <- (A(n = 1)) }\nfour = () { x = use() <- (a = A(n = 1))  x }\nfive = (e) { use() <- e }\nsix = (e) { use() <- (..e) }\nseven = () <- (..env: N) { 1 }\neight = () { { use() <- (A(n = 1)) } <- (B(m = 2), B(m = 3)) }\nnine = () { y = { use() } <- ()  y }\n",
    // Row variables and function types.
    "run = <A, N>(body: () <- N -> A) -> A { body() }\ncatch = <A, E, N>(body: () <- (Fail(E), ..N) -> A) -> A { body() }\nboth: <A, N>(() <- (..N) -> A) -> A = (body) { body() }\nlocal = () { inner = <N>(f: () <- N -> Int) { f() }  inner }\nbad = () { <A>(a: A) { a } }\nt: ((a = Int, Int)) -> Int = (x) { 1 }\nu: { Int | Float } -> Int = (x) { 1 }\nv: (x = Int, ..Int) -> Int = (x) { 1 }\nw: () <- (Fail(Int), Fail(Float), a = Int) -> Int = () { 1 }\n",
    // Patterns that this stage cannot type.
    "f = ((a, ..)) { a }\ng = ((a = b, c)) { c }\nP = (x = Int, y = Int)\nh = (P(..rest)) { rest }\nk = (P(x = (a, b))) { a }\nm = ((a, b) = (c, d)) { a }\nn = (key = x) { x }\np = ([a, ..rest] = whole) { whole }\nq = (\"ab\" <> rest) { rest }\n",
    // Attributes, declarations, heads.
    "@external(\"m\", \"f\")\nf: (Int) -> Int\n@external(\"m\")\ng: (Int) -> Int\n@deprecated(note = \"x\")\nh = 1\n@host(\"ns\")\nSvc = (run = (Int) -> Int)\n@host(\"ns2\")\nBad = (Int)\n@other\nk = 2\nBox(a) = (a)\nWrap(List(A)) = (A)\nAlias(List(A)) = A\nx = 1\nElem(List(A)) = x\nm: <A>(A) -> A\nimport()\n",
];

/// A tree deeper than a normal stack is resolved (the resolver runs in
/// `syn3::deep::deep`).
#[test]
fn resolve3_deep() {
    let depth = 20_000;
    let text = format!("x = {}1{}\n", "[".repeat(depth), "]".repeat(depth));
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"deep\"\n").unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.i"), &text).unwrap();
    let (graph, _) = ilang_package::load(dir.path());
    let mut parsed = Vec::new();
    let mut modules = Vec::new();
    for (package, p) in graph.packages.iter().enumerate() {
        for m in &p.modules {
            let (module, errors) = syn3::parse_module(&graph.sources.get(m.source).text);
            assert!(errors.is_empty(), "{}: {errors:?}", m.path);
            parsed.push(Parsed {
                package,
                path: m.path.clone(),
                source: m.source,
                kind: m.kind,
                ast: Default::default(),
                scoped: None,
                compiler_generated: false,
                typed_macros: None,
            });
            modules.push(module);
        }
    }
    let (program, diags) = resolve::resolve3(&graph, &parsed, &modules);
    let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "{errors:?}");
    // The HIR is as deep as the tree: it is dropped on a deep stack too.
    syn3::deep::deep(move || drop(program));
}
