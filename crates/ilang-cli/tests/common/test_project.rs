//! Ordinary manifests for packages that use the test macro library.
#![allow(dead_code)]
use std::path::Path;

/// The location of the standard library, which `i new` depends on.
pub const LIBRARY_PLACE: &str = "github.com/nmnmcc/i//stdlib";

/// The manifest that `i new` writes for `name` when the newest tag of the
/// standard library is `stdlib-v<version>`: the library `i` by location,
/// with no machine path and no copied package.
pub fn new_manifest(name: &str, version: &str) -> String {
    format!(
        "name = {name:?}\n[test]\ncollection = \"i/test.tests\"\n[dependencies]\ni = \"{LIBRARY_PLACE}@{version}\"\n"
    )
}

/// The standard library of this repository, `stdlib/`.
pub fn library_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../stdlib")
        .canonicalize()
        .unwrap()
}

/// The `[dependencies]` table that makes a literal manifest depend on the
/// standard library `i` of this repository by path, as every root package
/// must (E0641).
pub fn stdlib_dependency() -> String {
    format!(
        "[dependencies]\ni = {{ path = {:?} }}\n",
        library_dir().to_str().unwrap()
    )
}

/// A manifest for a package named `name` that depends on the standard
/// library of this repository by path, and collects tests with `i/test`.
pub fn manifest(name: &str) -> String {
    format!(
        "name = {name:?}\n[test]\ncollection = \"i/test.tests\"\n[dependencies]\ni = {{ path = {:?} }}\n",
        library_dir().to_str().unwrap()
    )
}

/// Makes the package in `project` collect tests with `i/test` and depend on
/// the standard library of this repository by path. Other keys are kept.
/// A package that will be got by location (as in the `i publish` tests)
/// should have no path dependency; use [`configure_tests_only`] for it.
pub fn configure(project: &Path) {
    configure_in(project, "dependencies");
}

/// As [`configure`]: the library is a normal dependency even when only the
/// tests import it, because a package's runtime modules are part of every
/// compiled program and a fetched package never loads dev-dependencies.
pub fn configure_tests_only(project: &Path) {
    configure_in(project, "dependencies");
}

/// Makes the package in `project` depend on the standard library of this
/// repository by path, as every root package must (E0641), without a test
/// collection. Other keys are kept.
pub fn depend_on_library(project: &Path) {
    let path = project.join("i.toml");
    let mut manifest: toml_edit::DocumentMut =
        std::fs::read_to_string(&path).unwrap().parse().unwrap();
    manifest["dependencies"]["i"]["path"] = toml_edit::value(library_dir().to_str().unwrap());
    std::fs::write(path, manifest.to_string()).unwrap();
}

fn configure_in(project: &Path, table: &str) {
    let path = project.join("i.toml");
    let mut manifest: toml_edit::DocumentMut =
        std::fs::read_to_string(&path).unwrap().parse().unwrap();
    manifest["test"]["collection"] = toml_edit::value("i/test.tests");
    manifest[table]["i"]["path"] = toml_edit::value(library_dir().to_str().unwrap());
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
            p.module(e.collection.module).path == "i/test" && p.def(e.collection).name == "tests"
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
