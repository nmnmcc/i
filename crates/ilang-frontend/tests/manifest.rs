use std::{
    fs,
    path::{Path, PathBuf},
};

use camino::Utf8Path;
use ilang_core::manifest::ManifestPackageSource;
use ilang_frontend::{dependency_manifest, load_packages};
use tempfile::TempDir;

fn package(base: &Path, directory: &str, name: &str, extra: &str) -> PathBuf {
    let root = base.join(directory);
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("i.toml"),
        format!("name = {name:?}\nversion = \"1.0.0\"\n{extra}\n"),
    )
    .unwrap();
    root
}

#[test]
fn resolves_shared_local_dependencies_once_and_injects_stdlib() {
    let temp = TempDir::new().unwrap();
    let common = package(temp.path(), "common", "common", "");
    package(
        temp.path(),
        "left",
        "left",
        "[dependencies]\ncommon = { path = \"../common\" }",
    );
    package(
        temp.path(),
        "right",
        "right",
        "[dependencies]\ncommon = { path = \"../common\" }",
    );
    let root = package(
        temp.path(),
        "app",
        "app",
        "[dependencies]\nleft = { path = \"../left\" }\nright = { path = \"../right\" }",
    );
    let packages = load_packages(&root, false).unwrap();
    let names = packages
        .iter()
        .map(|package| package.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names.iter().filter(|&&name| name == "common").count(), 1);
    assert_eq!(
        names.iter().filter(|&&name| name == "i_stdlib").count(),
        1
    );
    assert_eq!(names.last(), Some(&"app"));
    assert_eq!(
        packages
            .iter()
            .find(|package| package.name == "common")
            .unwrap()
            .root,
        common.canonicalize().unwrap()
    );
    assert!(
        packages
            .last()
            .unwrap()
            .dependencies
            .contains("i_stdlib")
    );
    assert!(packages.last().unwrap().is_root);
}

#[test]
fn dependency_manifest_contains_canonical_paths_and_implicit_stdlib() {
    let temp = TempDir::new().unwrap();
    let dep = package(temp.path(), "dep", "dep", "");
    let root = package(
        temp.path(),
        "app",
        "app",
        "[dependencies]\ndep = { path = \"../dep\" }",
    );
    let manifest = dependency_manifest(Utf8Path::from_path(&root).unwrap()).unwrap();
    assert!(manifest.requirements.contains_key("i_stdlib"));
    let dependency = manifest
        .packages
        .iter()
        .find(|package| package.name == "dep")
        .unwrap();
    assert!(
        matches!(&dependency.source, ManifestPackageSource::Local { path } if path.as_std_path() == dep.canonicalize().unwrap())
    );
    assert!(
        dependency
            .requirements
            .iter()
            .any(|name| name == "i_stdlib")
    );
    assert!(
        !manifest
            .packages
            .iter()
            .any(|package| package.name == "app")
    );
}

#[test]
fn implicit_stdlib_dependencies_follow_the_roots_explicit_local_library() {
    let temp = TempDir::new().unwrap();
    let stdlib = package(temp.path(), "stdlib", "i_stdlib", "");
    package(temp.path(), "dependency", "dependency", "");
    let root = package(
        temp.path(),
        "app",
        "app",
        "[dependencies]\ndependency = { path = \"../dependency\" }\ni_stdlib = { path = \"../stdlib\" }",
    );
    let packages = load_packages(&root, false).unwrap();
    let libraries = packages
        .iter()
        .filter(|package| package.is_stdlib)
        .collect::<Vec<_>>();
    assert_eq!(libraries.len(), 1);
    assert_eq!(libraries[0].root, stdlib.canonicalize().unwrap());
    assert!(
        packages
            .iter()
            .find(|package| package.name == "dependency")
            .unwrap()
            .dependencies
            .contains("i_stdlib")
    );
}

#[test]
fn reports_dependency_cycles_with_names() {
    let temp = TempDir::new().unwrap();
    let root = package(
        temp.path(),
        "alpha",
        "alpha",
        "[dependencies]\nbeta = { path = \"../beta\" }",
    );
    package(
        temp.path(),
        "beta",
        "beta",
        "[dependencies]\nalpha = { path = \"../alpha\" }",
    );
    let error = load_packages(&root, false).unwrap_err().to_string();
    assert!(error.contains("alpha -> beta -> alpha"), "{error}");
}

#[test]
fn rejects_package_aliases() {
    let temp = TempDir::new().unwrap();
    package(temp.path(), "other", "actual_name", "");
    let root = package(
        temp.path(),
        "app",
        "app",
        "[dependencies]\nalias = { path = \"../other\" }",
    );
    let error = load_packages(&root, false).unwrap_err().to_string();
    assert!(error.contains("dependency names must match"), "{error}");
}

#[test]
fn rejects_same_name_at_distinct_paths() {
    let temp = TempDir::new().unwrap();
    package(temp.path(), "common_a", "common", "");
    package(temp.path(), "common_b", "common", "");
    package(
        temp.path(),
        "left",
        "left",
        "[dependencies]\ncommon = { path = \"../common_a\" }",
    );
    package(
        temp.path(),
        "right",
        "right",
        "[dependencies]\ncommon = { path = \"../common_b\" }",
    );
    let root = package(
        temp.path(),
        "app",
        "app",
        "[dependencies]\nleft = { path = \"../left\" }\nright = { path = \"../right\" }",
    );
    let error = load_packages(&root, false).unwrap_err().to_string();
    assert!(error.contains("resolves to two directories"), "{error}");
}

#[test]
fn rejects_registry_and_git_dependencies_with_migration_advice() {
    for (requirement, expected) in [
        ("\">= 1.0.0\"", "local path dependencies"),
        (
            "{ git = \"https://example.test/library.git\", ref = \"main\" }",
            "Check it out locally",
        ),
    ] {
        let temp = TempDir::new().unwrap();
        let root = package(
            temp.path(),
            "app",
            "app",
            &format!("[dependencies]\nlibrary = {requirement}"),
        );
        let error = load_packages(&root, false).unwrap_err().to_string();
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn only_root_dev_dependencies_are_resolved_for_test_builds() {
    let temp = TempDir::new().unwrap();
    package(
        temp.path(),
        "normal",
        "normal",
        "[dev-dependencies]\nmissing = { path = \"../absent\" }",
    );
    package(temp.path(), "tests", "tests", "");
    let root = package(
        temp.path(),
        "app",
        "app",
        "[dependencies]\nnormal = { path = \"../normal\" }\n[dev-dependencies]\ntests = { path = \"../tests\" }",
    );
    let normal = load_packages(&root, false).unwrap();
    assert!(!normal.iter().any(|package| package.name == "tests"));
    let testing = load_packages(&root, true).unwrap();
    assert!(testing.iter().any(|package| package.name == "tests"));
}

#[test]
fn rejects_legacy_targets_and_platform_tables() {
    for extra in [
        "target = \"erlang\"",
        "target = \"javascript\"",
        "[erlang]",
        "[javascript.deno]\nallow_read = [\"./\"]",
    ] {
        let temp = TempDir::new().unwrap();
        let root = package(temp.path(), "app", "app", extra);
        let error = load_packages(&root, false).unwrap_err().to_string();
        assert!(
            error.contains("iLang") || error.contains("unsupported backend"),
            "{error}"
        );
    }
}

#[test]
fn rejects_invalid_versions_and_duplicate_dependency_sections() {
    let temp = TempDir::new().unwrap();
    let root = package(temp.path(), "app", "app", "");
    fs::write(
        root.join("i.toml"),
        "name = \"app\"\nversion = \"nonsense\"\n",
    )
    .unwrap();
    let error = load_packages(&root, false).unwrap_err().to_string();
    assert!(error.contains("Invalid package configuration"), "{error}");

    package(temp.path(), "dep", "dep", "");
    fs::write(root.join("i.toml"), "name = \"app\"\n[dependencies]\ndep = { path = \"../dep\" }\n[dev-dependencies]\ndep = { path = \"../dep\" }\n").unwrap();
    let error = load_packages(&root, true).unwrap_err().to_string();
    assert!(
        error.contains("both dependencies and dev-dependencies"),
        "{error}"
    );
}
