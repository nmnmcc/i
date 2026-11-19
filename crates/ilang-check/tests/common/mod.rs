//! Helpers shared by the tests.

#![allow(dead_code)]

pub mod tree;

use ilang_check::hir::{Def, ModuleId, Program};
use ilang_package::{Diagnostic, PackageGraph};
use std::fs;

/// Exact warnings for fixtures with no entry or export roots.
pub fn expect_unused(diags: &[Diagnostic], names: &[&str]) {
    let actual = diags
        .iter()
        .map(|d| (d.code, d.message.clone()))
        .collect::<Vec<_>>();
    let expected = names
        .iter()
        .map(|name| {
            (
                ilang_check::code::UNUSED_BINDING,
                format!("top-level binding `{name}` is not used"),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

/// Writes a package named `app` with the given files into a new directory,
/// then loads and checks it.
pub fn check_files(files: &[(&str, &str)]) -> (PackageGraph, Option<Program>, Vec<Diagnostic>) {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    for (path, text) in files {
        let p = dir.path().join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
    let (graph, mut diags) = ilang_package::load(dir.path());
    let checked = ilang_check::check(&graph);
    diags.extend(checked.diags);
    (graph, checked.program, diags)
}

/// Like [`check_files`], but checks every module of the graph, also the
/// library and dependency modules that no import reaches (which
/// [`ilang_check::parse`] leaves out). For tests about the library itself.
pub fn check_all_modules(
    files: &[(&str, &str)],
) -> (PackageGraph, Option<Program>, Vec<Diagnostic>) {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    for (path, text) in files {
        let p = dir.path().join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
    let (graph, mut diags) = ilang_package::load(dir.path());
    let checked = check_all_parsed(&graph);
    diags.extend(checked.diags);
    (graph, checked.program, diags)
}

/// [`ilang_check::check`] over every module of the graph.
pub fn check_all_parsed(graph: &PackageGraph) -> ilang_check::Checked {
    let (parsed, diags) = ilang_check::parse_all(graph);
    assert!(diags.is_empty(), "{diags:?}");
    ilang_check::check_parsed(graph, &parsed)
}

pub fn module(program: &Program, path: &str) -> ModuleId {
    let i = program.modules.iter().position(|m| m.path == path).unwrap();
    ModuleId(i as u32)
}

pub fn def<'a>(program: &'a Program, path: &str, name: &str) -> &'a Def {
    let m = program.module(module(program, path));
    m.defs.iter().find(|d| d.name == name).unwrap()
}

/// Listed cases need unsupported forms. Skips are printed so those gaps stay
/// visible while the remaining cases continue to run.
#[allow(dead_code)]
pub fn syn3_pending(name: &str) -> bool {
    let list = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/syn3-pending.txt"),
    )
    .unwrap();
    let listed = list
        .lines()
        .filter(|l| !l.starts_with('#'))
        .any(|l| l.split_whitespace().next() == Some(name));
    if listed {
        eprintln!("{name}: skipped, listed in tests/syn3-pending.txt");
    }
    listed
}
