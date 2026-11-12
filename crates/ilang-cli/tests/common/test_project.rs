//! Ordinary manifests for packages that use the test macro library.
#![allow(dead_code)]
use std::path::Path;

pub fn manifest(name: &str) -> String {
    let std = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../std")
        .canonicalize()
        .unwrap();
    format!(
        "name = {name:?}\n[test]\ncollection = \"std/test.tests\"\n[dependencies]\nstd = {{ path = {:?} }}\n",
        std.to_str().unwrap()
    )
}

pub fn configure(project: &Path) {
    let path = project.join("i.toml");
    let mut manifest: toml_edit::DocumentMut =
        std::fs::read_to_string(&path).unwrap().parse().unwrap();
    manifest["test"]["collection"] = toml_edit::value("std/test.tests");
    manifest["dependencies"]["std"]["path"] = toml_edit::value(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../std")
            .to_str()
            .unwrap(),
    );
    std::fs::write(path, manifest.to_string()).unwrap();
}

pub struct SelectedTests {
    pub graph: ilang_package::PackageGraph,
    pub checked: ilang_check::Checked,
    pub exports: Vec<(ilang_check::hir::DefId, String)>,
    pub templates: Vec<ilang_syntax::lisp::syntax::Syntax>,
}

/// Low-level host tests keep readable export handles while selecting the same
/// collection roots as the CLI. Internal syntax tests mark their own modules.
pub fn selected(project: &Path, generated: bool) -> SelectedTests {
    let (mut graph, diags) = ilang_cli::load(project, true, true);
    assert!(
        !diags.iter().any(ilang_package::Diagnostic::is_error),
        "{:?}",
        graph.sources.render_all(&diags)
    );
    graph.test_run = true;
    let root = graph.root.unwrap();
    let mut expanded = ilang_macro::project::expand(&mut graph)
        .unwrap_or_else(|ds| panic!("{:?}", graph.sources.render_all(&ds)));
    expanded.lower_quotes();
    if generated {
        for module in &mut expanded.parsed {
            if module.package == root {
                module.compiler_generated = true;
            }
        }
    }
    let (program, mut diags) = ilang_check::resolve_parsed(&graph, &expanded.parsed);
    diags.extend(expanded.diagnostics);
    assert!(
        !diags.iter().any(ilang_package::Diagnostic::is_error),
        "{:?}",
        graph.sources.render_all(&diags)
    );
    let p = program.as_ref().unwrap();
    let entries = ilang_macro::collections::resolve_with_types(
        &expanded.parsed,
        p,
        None,
        &expanded.transactions,
    )
    .unwrap_or_else(|ds| panic!("{:?}", graph.sources.render_all(&ds)));
    let exports = entries
        .iter()
        .filter(|e| {
            p.module(e.collection.module).path == "std/test" && p.def(e.collection).name == "tests"
        })
        .map(|e| {
            let name = &e.data.iter().find(|(key, _)| key == "name").unwrap().1;
            (
                e.target,
                format!("{}.{name}", p.module(e.target.module).path),
            )
        })
        .collect::<Vec<_>>();
    assert!(!exports.is_empty(), "host fixture has no collected tests");
    let roots = exports
        .iter()
        .map(|(id, _)| (*id, true))
        .collect::<Vec<_>>();
    let checked = ilang_check::check_resolved_selected(&graph, program, diags, &roots);
    assert!(
        !checked
            .diags
            .iter()
            .any(ilang_package::Diagnostic::is_error),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    SelectedTests {
        graph,
        checked,
        exports,
        templates: expanded.templates,
    }
}
