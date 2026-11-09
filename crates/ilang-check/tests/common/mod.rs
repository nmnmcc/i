//! Helpers shared by the tests.

#![allow(dead_code)]

use ilang_check::hir::{Def, ModuleId, Program};
use ilang_package::{Diagnostic, PackageGraph};
use std::fs;

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
    let (program, more) = ilang_check::check(&graph);
    diags.extend(more);
    (graph, program, diags)
}

pub fn module(program: &Program, path: &str) -> ModuleId {
    let i = program.modules.iter().position(|m| m.path == path).unwrap();
    ModuleId(i as u32)
}

pub fn def<'a>(program: &'a Program, path: &str, name: &str) -> &'a Def {
    let m = program.module(module(program, path));
    m.defs.iter().find(|d| d.name == name).unwrap()
}
