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
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (i, module) in program.modules.iter().enumerate() {
        let id = ModuleId(i as u32);
        let mut text = format!("{module:#?}\n");
        for t in &program.types {
            if t.module == id {
                text.push_str(&format!("{t:#?}\n"));
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
    reports: Vec<String>,
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
    expected.extend(old_diags);
    let (new_program, new_diags) = resolve::resolve3(&graph, &direct, &modules);
    let old = texts(&graph, &old_program, &expected);
    let new = texts(&graph, &new_program, &new_diags);
    for (file, text) in &old {
        let key = if file == "<program>" {
            format!("<program of {name}>")
        } else {
            file.clone()
        };
        totals.files.insert(key.clone());
        let other = new.get(file).map_or("", |t| t.as_str());
        if text != other && totals.differ.insert(key.clone()) {
            totals
                .reports
                .push(format!("{key} (in {name}): {}", first_difference(text, other)));
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
    let mut totals = Totals::default();
    if wanted("std") {
        compare("std", &root.join("std"), &mut totals);
    }
    for dir in subdirs(&root.join("examples")) {
        let name = format!("examples/{}", dir.file_name().unwrap().to_string_lossy());
        if dir.join("i.toml").exists() && wanted(&name) {
            compare(&name, &dir, &mut totals);
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
                    compare(&name, &case, &mut totals);
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
                // The file is named by its place in the suite.
                let before = totals.reports.len();
                compare(&name, dir.path(), &mut totals);
                for report in &mut totals.reports[before..] {
                    *report = report.replace(&dir.path().to_string_lossy().into_owned(), &name);
                }
            }
        }
    }
    for report in &totals.reports {
        eprintln!("DIFFERENT {report}\n");
    }
    let equal = totals.files.len() - totals.differ.len();
    eprintln!(
        "resolve3 differential: {equal} files equal / {} total",
        totals.files.len()
    );
    assert!(
        totals.differ.is_empty(),
        "{} files differ",
        totals.differ.len()
    );
}
