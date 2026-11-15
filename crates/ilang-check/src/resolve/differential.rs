//! Differential checks keep direct source resolution and the migration
//! adapter equal in HIR and diagnostics, including generated names and spans.

use crate as ilang_check;
use ilang_check::hir::{ModuleId, Program};
use ilang_check::{Parsed, resolve};
use ilang_package::{Diagnostic, LoadOptions, PackageGraph};
use migration_syntax::syn3;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Diagnostic display paths are relative to the loaded package, so many
/// unrelated fixtures are called `src/main.i`. Count the original files,
/// including a lone fixture copied into a temporary package, separately.
fn file_key(graph: &PackageGraph, module: &ilang_check::hir::Module, job: &str) -> String {
    if graph.root == Some(module.package) && job.ends_with(".i") {
        return job.to_string();
    }
    let package = &graph.packages[module.package];
    let Some(dir) = &package.dir else {
        return if module.kind == ilang_package::ModuleKind::Prelude {
            "lib/prelude.i".into()
        } else {
            format!("lib/{}.i", module.path)
        };
    };
    let relative = module
        .path
        .strip_prefix(&format!("{}/", package.name))
        .unwrap();
    let sub = if module.kind == ilang_package::ModuleKind::Test {
        "test"
    } else {
        "src"
    };
    let extension = if graph.sources.get(module.source).display.ends_with(".is") {
        "is"
    } else {
        "i"
    };
    let file = dir.join(sub).join(relative).with_extension(extension);
    file.strip_prefix(root())
        .unwrap_or(&file)
        .to_string_lossy()
        .into_owned()
}

/// The one recorded file where the direct resolver's repair hint has no
/// legacy counterpart, and the two diagnostics it may add it to: the source
/// span, the legacy message, and the complete message with the hint.
const UNION_ALIAS_FILE: &str = "tests/spec/4.3/syn3_one_constructor.i";
const UNION_ALIAS_ALLOWED: [(u32, u32, &str, &str); 2] = [
    (
        261,
        267,
        "the type `Circle` is not defined",
        "the type `Circle` is not defined. A union alias does not define its alternatives. \
         Define each alternative as its own type first: `Circle = (Float)`, then `Square = \
         (Float)`, then `Shape = Circle | Square`.",
    ),
    (
        277,
        283,
        "the type `Square` is not defined",
        "the type `Square` is not defined. A union alias does not define its alternatives. \
         Define each alternative as its own type first: `Circle = (Float)`, then `Square = \
         (Float)`, then `Shape = Circle | Square`.",
    ),
];

/// The legacy message, and only for that file, that code, that source span and
/// that complete message. Anything else, including a different span, a changed
/// word or an extra suffix, is left to the comparison unchanged.
fn without_union_alias_hint<'m>(
    key: &str,
    code: &str,
    span: (u32, u32),
    message: &'m str,
) -> Option<&'m str> {
    if key != UNION_ALIAS_FILE || code != ilang_check::code::UNKNOWN_TYPE {
        return None;
    }
    UNION_ALIAS_ALLOWED
        .iter()
        .find_map(|&(start, end, old, full)| {
            ((start, end) == span && full == message).then_some(old)
        })
}

fn comparable_message<'m>(key: &str, d: &'m Diagnostic) -> &'m str {
    without_union_alias_hint(key, d.code, (d.span.start, d.span.end), &d.message)
        .unwrap_or(&d.message)
}

/// The texts that must be equal, by file. The key `<program>` holds the
/// parts of the program that belong to no module.
fn texts(
    job: &str,
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
        let key = file_key(graph, module, job);
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
                    d.severity,
                    d.code,
                    d.span,
                    comparable_message(&key, d)
                ));
            }
        }
        assert!(out.insert(key, text).is_none());
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
        program.of_sites, program.alias_patterns, program.alias_dependencies, program.alias_sources
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
    extensions: BTreeSet<String>,
    reports: Vec<(String, String)>,
    /// What was compared: bindings, diagnostics of the resolver, errors of
    /// the lowering.
    bindings: usize,
    diagnostics: usize,
    lowering: usize,
}

fn compare(name: &str, dir: &Path, totals: &mut Totals) {
    compare_inner(name, dir, totals, false);
}

fn compare_inner(name: &str, dir: &Path, totals: &mut Totals, prepare_quotes: bool) {
    let options = LoadOptions {
        tests: true,
        ..LoadOptions::from_env()
    };
    let (graph, _) = ilang_package::load_with(dir, &options);
    let mut lowered = Vec::new();
    let mut lowered_modules = Vec::new();
    let mut direct = Vec::new();
    let mut modules = Vec::new();
    let mut expected = Vec::new();
    let mut old_templates = Vec::new();
    let mut new_templates = Vec::new();
    for (package, p) in graph.packages.iter().enumerate() {
        for m in &p.modules {
            let source = graph.sources.get(m.source);
            let lisp = source.display.ends_with(".is");
            let (mut module, _) = if lisp {
                syn3::lisp::from_lisp(&source.text)
            } else {
                syn3::parse_module(&source.text)
            };
            let (mut ast, errors) = if lisp {
                syn3::lower::lower_lisp(&module)
            } else {
                syn3::lower::lower(&module)
            };
            if prepare_quotes {
                assert!(
                    errors.is_empty(),
                    "quote fixture lowering errors: {errors:?}"
                );
                migration_syntax::lisp::quote_lower::lower_expression_quotes(
                    &mut ast,
                    m.source.0,
                    &mut old_templates,
                );
                let errors = ilang_syntax::lisp::quote_lower3::lower_expression_quotes(
                    &mut module,
                    m.source.0,
                    &mut new_templates,
                );
                assert!(errors.is_empty(), "native quote preparation: {errors:?}");
            }
            expected.extend(
                errors
                    .into_iter()
                    .map(|d| Diagnostic::error(d.code.as_str(), Some(m.source), d.span, d.message)),
            );
            let parsed = || Parsed {
                package,
                path: m.path.clone(),
                source: m.source,
                kind: m.kind,
                ast: Default::default(),
                lowering: Default::default(),
                scoped: None,
                compiler_generated: false,
                typed_macros: None,
            };
            lowered.push(parsed());
            lowered_modules.push(ast);
            direct.push(parsed());
            modules.push(module);
        }
    }
    if prepare_quotes {
        assert!(!old_templates.is_empty());
        assert_eq!(old_templates.len(), new_templates.len());
    }
    let (old_program, old_diags) =
        super::legacy::resolve_transition(&graph, &lowered, &lowered_modules);
    totals.lowering += expected.len();
    totals.diagnostics += old_diags.len();
    totals.bindings += old_program
        .modules
        .iter()
        .map(|m| m.bindings.len())
        .sum::<usize>();
    expected.extend(old_diags);
    let (new_program, new_diags) = resolve::resolve3(&graph, &direct, &modules);
    let old = texts(name, &graph, &old_program, &expected, false);
    let new = texts(name, &graph, &new_program, &new_diags, false);
    // These keyed-data fixtures intentionally go beyond the frozen migration
    // oracle. Keep checking its refusal and the native resolver's exact
    // errors; the spec runner checks inference and both runtime levels.
    if name.starts_with("tests/spec/7/keyed_data") {
        assert!(expected.iter().any(|d| {
            d.message
                .contains("a key in a data pattern is not supported here")
                || (name.ends_with("keyed_data_constructor_bad.i")
                    || name.ends_with("keyed_data_constructor_missing.i"))
                    && d.message
                        .contains("a pattern inside a labelled field is not supported here")
        }));
        let errors: Vec<_> = new_diags
            .iter()
            .filter(|d| d.is_error())
            .map(|d| d.code)
            .collect();
        let wanted: &[&str] = if name.ends_with("keyed_data_duplicate.i") {
            &["E0510"]
        } else if name.ends_with("keyed_data_duplicate_binder.i") {
            &["E0217"]
        } else if name.ends_with("keyed_data_label_is_not_binder.i")
            || name.ends_with("keyed_data_constructor_label.i")
        {
            &["E0201"]
        } else if name.contains("keyed_data_order") {
            &["E0101"]
        } else {
            &[]
        };
        assert_eq!(errors, wanted, "native resolution of {name}");
        for file in new.keys() {
            let key = if file == "<program>" {
                format!("<program of {name}>")
            } else {
                file.clone()
            };
            if old.get(file) != new.get(file) {
                totals.extensions.insert(key.clone());
            }
            totals.files.insert(key);
        }
        return;
    }
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
    let old = texts(name, &graph, &old_program, &expected, true);
    let new = texts(name, &graph, &new_program, &new_diags, true);
    for file in old.keys().chain(new.keys()).collect::<BTreeSet<_>>() {
        let key = if file == "<program>" {
            format!("<program of {name}>")
        } else {
            file.clone()
        };
        totals.files.insert(key.clone());
        let text = old.get(file).map_or("", |t| t.as_str());
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

fn normal_files(dir: &Path, files: &mut BTreeSet<String>) {
    for path in subdirs(dir) {
        if path.is_dir() {
            normal_files(&path, files);
        } else if path.extension().is_some_and(|e| e == "i") {
            files.insert(
                path.strip_prefix(root())
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
}

fn package_dirs(dir: &Path, packages: &mut Vec<PathBuf>) {
    if dir.join("i.toml").is_file() {
        packages.push(dir.to_path_buf());
    }
    for path in subdirs(dir) {
        if path.is_dir() {
            package_dirs(&path, packages);
        }
    }
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
    // A focused imported E1 counterexample. Compare resolver input/diagnostics
    // with the migration adapter; the independent inference audit keeps its own oracle.
    let provider_case = "tests/review/cases/review-k4d/proj/p1";
    if wanted(provider_case) {
        jobs.push((provider_case.into(), root.join(provider_case), None));
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
    // Packages can also be nested, such as std/bench/std4. Keep the
    // package context instead of comparing their files in isolation.
    let existing: BTreeSet<_> = jobs.iter().map(|(_, dir, _)| dir.clone()).collect();
    let mut packages = Vec::new();
    for sub in ["std", "examples", "tests/spec"] {
        package_dirs(&root.join(sub), &mut packages);
    }
    for dir in packages {
        let name = dir
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        if !existing.contains(&dir) && wanted(&name) {
            jobs.push((name, dir, None));
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
                eprintln!(
                    "extra/m{i}: {} syntax errors {:?}",
                    errors.len(),
                    errors.first()
                );
            }
            std::fs::write(dir.path().join(format!("src/m{i}.i")), text).unwrap();
        }
        for (i, text) in EXTRA_LISP.iter().enumerate() {
            let (_, errors) = syn3::lisp::from_lisp(text);
            assert!(errors.is_empty(), "extra/l{i}: {errors:?}");
            std::fs::write(dir.path().join(format!("src/l{i}.is")), text).unwrap();
        }
        jobs.push(("extra".into(), dir.path().to_path_buf(), Some(dir)));
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let all = std::sync::Mutex::new(Totals::default());
    // Parallel cases would multiply package graphs and exceed the test memory budget.
    let workers = std::thread::available_parallelism().map_or(2, |n| n.get().min(2));
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
                    all.extensions.extend(totals.extensions);
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
    let files = totals
        .files
        .iter()
        .filter(|f| !f.starts_with("<program of "))
        .count();
    let differ = totals
        .differ
        .iter()
        .filter(|f| !f.starts_with("<program of "))
        .count();
    let extension_files = totals
        .extensions
        .iter()
        .filter(|f| !f.starts_with("<program of "))
        .count();
    let equal = files - differ - extension_files;
    let programs = totals.files.len() - files;
    let programs_equal =
        programs - (totals.differ.len() - differ) - (totals.extensions.len() - extension_files);
    eprintln!(
        "resolve3 spec extensions: {extension_files} files checked against the new language rule"
    );
    eprintln!(
        "resolve3 differential: {equal} files equal / {} total ({} top-level bindings, {} resolver diagnostics, {} lowering errors over {} packages)",
        files,
        totals.bindings,
        totals.diagnostics,
        totals.lowering,
        jobs.len()
    );
    eprintln!("resolve3 programs: {programs_equal} equal / {programs} total");
    if only.is_none() {
        let mut expected = BTreeSet::new();
        for sub in ["lib", "std", "examples", "tests/spec"] {
            normal_files(&root.join(sub), &mut expected);
        }
        let missing: Vec<_> = expected.difference(&totals.files).collect();
        assert!(
            missing.is_empty(),
            "normal-surface files not compared: {missing:#?}"
        );
        eprintln!(
            "resolve3 coverage: all {} repository .i files compared or checked as specified extensions",
            expected.len()
        );
    }
    assert!(only.is_some() || totals.bindings > 1000 && totals.diagnostics > 0);
    assert!(
        totals.differ.is_empty(),
        "{} files or programs differ",
        totals.differ.len()
    );
}

/// One module each; a module with a syntax error is still compared, on the
/// part of the tree that was read.
const EXTRA: &[&str] = &[
    // Written branch result unions and ordinary Fail service patterns use
    // the single need, including several forms of the same Fail key.
    "f = (x: Int) -> Int { x } | (x: Float) -> Float { x }\ng = (x: Int) -> Int { x } | (x: Float) -> Float { x } | (x: String) { x }\nh = () <- (failure: Fail(Int)) { 1 }\nduplicate = () <- (a: Fail(Int), b: Fail(Float)) { 0 }\nk = (x: Int) <- (failure: Fail(Int)) -> Int { x } | (x: Float) <- (failure: Fail(Int)) -> Float { x }\n",
    // A finite callback need captured by a local annotation, then handled.
    "S(N) = (() <- N -> Nil)\nDependency = ()\nread = () <- (d: Dependency) { Nil }\nuse = <N>() <- (s: S(N)) { S(f) = s  f() }\napply = <N>() { local: () <- N -> Nil = () { use() }  local() }\nmain = () { apply() <- (S(() { read() })) }\ntyped = () { provided: S(Dependency) = S(() { read() })  apply() <- (provided) }\n",
    // Shape dispatch must agree for named and inline groups, aliases, key order,
    // recursion and missing/duplicate shape diagnostics.
    "(f, alias)\nf = (0) { 0 } | (n: Int) { f(n - 1) } | (x: Int, y: Int) { x + y }\nalias = f\na = alias(1, 2)\nb = 1 |> alias(2)\nc = f()\nlocal = () { g = (a = x: Int, z = y: Int) { x + y } | (x: Int) { x }  g(z = 2, a = 1) }\ninline = [(x: Int) { x } | (x: Int, y: Int) { x + y }]\n",
    // General parameter data, closed labelled parameter data, ordinary Fail
    // services and directed need rows use the same HIR as the transition.
    "(fail = i/fail)\nSvc(A) = (value = A)\nSynFail(E) = Fail(E)\nrun = <A, N>(f: () <- N -> A) { f() }\nlabelled: { (x = Int, y = Int) } -> Int = (x = a: Int, y = b: Int) { a + b }\nprovider = () <- (f: SynFail(Int)) { f }\nread = () <- (s: Svc(Int)) { s.value }\nprovided = () { read() <- (Svc(value = 1)) }\n",
    // Sums, data types, aliases, exports of members.
    "(Shape(..), Circle, Pair(..), area, swap)\n\nCircle = (radius = Float)\nSquare = (side = Float)\nShape = Circle | Square\nPair(A, B) = (first = A, second = B)\nToken = Opaque\nId(A) = A\nBoth = (Int, Int)\nRec = (a = Int, b = Int)\n\narea = (s: Shape) -> Float {\n	s |> (Circle(radius = r)) { r } | (Square(side = _)) { 0.0 }\n}\nswap = <A, B>(p: Pair(A, B)) -> Pair(B, A) { Pair(first = p.second, second = p.first) }\n",
    // Keys in constructor patterns.
    // old-syntax-check: E0101
    "P = (x = Int, y = Int)\nf = (P(x = x, y = other)) { x + other }\ng = (P(x = _, y = y: Int)) { y }\nh = (P(x = a = b, ..)) { a + b }\nk = (P(x = whole: Int, y = _)) { whole }\nm = (p: P) { P(..p, x = 1) }\nn = (P(x = 0, y = y)) { y }\n",
    // Chains: clauses, guards, several positions, alternatives.
    // old-syntax-check: E0110,E0101
    "f = (0) { 1 } | (n) { n }\ng = (x, y ? x > y) { x } | (x, y) { y }\nh = (1 | 2, a) { a } | (_, a) { a + 1 }\nk = (a = b, c) { (a, b, c) }\nm = (x) { x |> (0) { 1 } | (n ? n > 2) { 2 } | (_) { 3 } }\np = (x) { x |> (y) { y + 1 } }\nq = (x: Int) -> Int { x } | (x) { x }\nr = (x) { 1 } | (x, y) { 2 }\ns = (? True) { 1 }\nt = (x) { x |> (a, b) { a } | (c) { c } }\n",
    // Data, calls, strings, index.
    // old-syntax-check: E0110,E0101,E0101
    "f = (x) { \"a\\(x)b\\(x)\" }\ng = (x) { \"\\(x)\" }\nh = () { a = (1, 2)  a.0 }\nk = () { (a = 1, b = 2) }\nm = (r) { (..r, a = 1) }\nn = () { (a = 2, 1) }\np = (f) { f(1, ..f) }\nq = () { () }\nr = (x) { x |> Some }\ns = (x) { core.never(x) }\nt = core.never\n",
    // Services and provides; Captured-row forms parse, provided labels remain invalid.
    // old-syntax-check: E0101,E0101
    "A = (n = Int)\nB = (m = Int)\nuse = () <- (a: A, b: B) -> Int { a.n + b.m }\none = () { use() <- (A(n = 1), B(m = 2)) }\ntwo = (a: A, b: B) { use() <- (a, b) }\nthree = () { { x = 1  use() + x } <- (A(n = 1)) }\nfour = () { x = use() <- (a = A(n = 1))  x }\nfive = (e) { use() <- e }\nsix = (e) { use() <- (..e) }\nseven = () <- (..env: N) { 1 }\neight = () { { use() <- (A(n = 1)) } <- (B(m = 2), B(m = 3)) }\nnine = () { y = { use() } <- ()  y }\n",
    // Row variables and function types.
    // old-syntax-check: E0101,E0110,E0101,E0107,E0101
    "run = <A, N>(body: () <- N -> A) -> A { body() }\ncatch = <A, E, N>(body: () <- (Fail(E), ..N) -> A) -> A { body() }\nboth: <A, N>(() <- (..N) -> A) -> A = (body) { body() }\nlocal = () { inner = <N>(f: () <- N -> Int) { f() }  inner }\nbad = () { <A>(a: A) { a } }\nt: ((a = Int, Int)) -> Int = (x) { 1 }\nu: { Int | Float } -> Int = (x) { 1 }\nv: (x = Int, ..Int) -> Int = (x) { 1 }\nw: () <- (Fail(Int), Fail(Float), a = Int) -> Int = () { 1 }\n",
    // Patterns that the checker cannot type. The nested constructor-data probe
    // has checked type and shape failures in keyed_data_constructor_bad.i,
    // beside the positive keyed_data_constructor.i cases. The old keyed-data
    // probe is now tests/spec/7/keyed_data_order.i and keyed_data_order_lisp:
    // keys are accepted, but a position after a label is still rejected.
    // old-syntax-check: E0101,E0101,E0101
    "f = ((a, ..)) { a }\nP = (x = Int, y = Int)\nh = (P(..rest)) { rest }\nm = ((a, b) = (c, d)) { a }\nn = (key = x) { x }\np = ([a, ..rest] = whole) { whole }\nq = (\"ab\" <> rest) { rest }\n",
    // Attributes, declarations, heads.
    // old-syntax-check: E0101,E0101
    "@external(\"m\", \"f\")\nf: (Int) -> Int\n@external(\"m\")\ng: (Int) -> Int\n@deprecated(note = \"x\")\nh = 1\n@host(\"ns\")\nSvc = (run = (Int) -> Int)\n@host(\"ns2\")\nBad = (Int)\n@other\nk = 2\nBox(a) = (a)\nWrap(List(A)) = (A)\nAlias(List(A)) = A\nx = 1\nElem(List(A)) = x\nm: <A>(A) -> A\nimport()\n",
];

/// These rows reach lowering errors which the normal-surface parser
/// rejects first, and exercise quote captures outside skipped macro bodies.
const EXTRA_LISP: &[&str] = &[
    r#"
(= mixed (, (= a 1) 2))
(: row (-> (, (= a Int) Int) (<- Int (, (Fail Int) (Fail String) (= a Int)))))
(= row (-> (,) _ 1))
(= service (<- (row) (, (= a 1))))
(= keyed (-> (, (= a x) y) _ x))
"#,
    r#"
(= capture (-> (, `(f ~item)) _ item))
(= rest (-> (, `(f ~..[first ..rest])) _ first))
(= nested (-> (, `(f ~`(g ~item))) _ item))
(= typed (-> (, `(f ~(: item Int))) _ item))
(= both (-> (, `(f ~(= a b))) _ (, a b)))
(= tuple (-> (, `(f ~(, a b))) _ (, a b)))
(= bad_splice (-> (, `(f ~..rest x)) _ rest))
(= bad_nested (-> (, `(f ~`(g ~..rest x))) _ rest))
(= quoted `(f ~unknown ~..rest))
"#,
    r#"
(@ macro (: identity (-> [A] (, A) A)))
(@ macro (= identity (-> (, x) _ x)))
(@ macro (= guarded (| (-> (? (, x) True) _ x) (-> (, _) _ 0))))
(@ macro (= wrapped (| (-> (, x y) _ x) (-> (, x y) _ y))))
(@ macro (= generic (-> [A] (, x) _ x)))
(@ macro (= bad_pattern (-> (, (, a b)) _ a)))
"#,
];

/// Prepared quote templates and captures live on the native tree. Compare
/// with preparing the lowered tree, including nested and shared captures,
/// guards, and expression quotes with unquotes and splices.
#[test]
fn resolve3_prepared_quotes() {
    let source = r#"
(, (= syntax (/ i syntax)))
(= capture (-> (, `(f ~item)) _ item))
(= rest (-> (, `(f ~..[first ..rest])) _ first))
(= nested (-> (, `(f ~`(g ~item))) _ item))
(= typed (-> (, `(f ~(: item (. syntax Syntax)))) _ item))
(= both (-> (, `(f ~(= a b))) _ (, a b)))
(= tuple (-> (, `(f ~(, a b))) _ (, a b)))
(= choose (-> (, x) _
  (|> x (| (-> (? (, `(f ~item)) False) _ item)
             (-> (, (| `(g ~item) `(h ~item))) _ item)
             (-> (, _) _ x)))))
(= build (-> (, input rest) _ `(f ~input ~..rest)))
"#;
    let (_, errors) = syn3::lisp::from_lisp(source);
    assert!(errors.is_empty(), "{errors:?}");
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"quotes\"\n").unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.is"), source).unwrap();
    let mut totals = Totals::default();
    compare_inner("prepared quotes", dir.path(), &mut totals, true);
    assert!(totals.differ.is_empty(), "{:#?}", totals.reports);
}

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
            // Parsing is recursive too; keep this test focused on the resolver
            // entry point by preparing its input on a deep stack.
            let (module, errors) =
                syn3::deep::deep(|| syn3::parse_module(&graph.sources.get(m.source).text));
            assert!(errors.is_empty(), "{}: {errors:?}", m.path);
            let errors = resolve::lowering_errors(&module, false);
            assert!(errors.is_empty(), "{}: {errors:?}", m.path);
            parsed.push(Parsed {
                package,
                path: m.path.clone(),
                source: m.source,
                kind: m.kind,
                ast: Default::default(),
                lowering: Default::default(),
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

#[cfg(test)]
mod tests {
    use super::*;

    const CIRCLE: (u32, u32) = (261, 267);
    const CODE: &str = ilang_check::code::UNKNOWN_TYPE;
    const CIRCLE_OLD: &str = "the type `Circle` is not defined";

    fn circle_full() -> &'static str {
        UNION_ALIAS_ALLOWED[0].3
    }

    #[test]
    fn only_the_two_recorded_union_alias_diagnostics_are_ignored() {
        // The recorded pair: exact file, code, span and complete message.
        assert_eq!(
            without_union_alias_hint(UNION_ALIAS_FILE, CODE, CIRCLE, circle_full()),
            Some(CIRCLE_OLD)
        );
        // The legacy message alone is not rewritten.
        assert_eq!(
            without_union_alias_hint(UNION_ALIAS_FILE, CODE, CIRCLE, CIRCLE_OLD),
            None
        );
        // Another file keeps the hint.
        assert_eq!(
            without_union_alias_hint("tests/spec/4.3/other.i", CODE, CIRCLE, circle_full()),
            None
        );
        // Another code keeps the hint.
        assert_eq!(
            without_union_alias_hint(UNION_ALIAS_FILE, "E0203", CIRCLE, circle_full()),
            None
        );
        // Another span on the same file keeps the hint.
        assert_eq!(
            without_union_alias_hint(UNION_ALIAS_FILE, CODE, (262, 267), circle_full()),
            None
        );
        // A changed word in the hint keeps the hint.
        let changed = circle_full().replacen("Define each", "Define every", 1);
        assert_ne!(changed, *circle_full());
        assert_eq!(
            without_union_alias_hint(UNION_ALIAS_FILE, CODE, CIRCLE, &changed),
            None
        );
        // An extra suffix after the hint keeps the hint.
        let extra = format!("{} Extra.", circle_full());
        assert_eq!(
            without_union_alias_hint(UNION_ALIAS_FILE, CODE, CIRCLE, &extra),
            None
        );
        // A missing prefix (the hint alone) keeps the text.
        let sole_hint = &circle_full()[CIRCLE_OLD.len()..];
        assert_eq!(
            without_union_alias_hint(UNION_ALIAS_FILE, CODE, CIRCLE, sole_hint),
            None
        );
    }
}
