//! End-to-end checking tests use real projects and the actual i analyser.
//! Cache assertions cover both reuse and invalidation of semantically relevant
//! changes; comparing counters alone would miss accidentally stale typed trees.

use std::{fs, path::Path, sync::Arc};

use ilang_frontend::{CheckOptions, Project, check_project, format_source};
use tempfile::TempDir;

fn write(root: &Path, relative: &str, source: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, source).unwrap();
}

fn project(files: &[(&str, &str)]) -> TempDir {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "i.toml",
        "name = \"app\"\nversion = \"1.0.0\"\ntarget = \"wasm\"\n",
    );
    for (path, source) in files {
        write(root.path(), path, source);
    }
    root
}

#[test]
fn checks_generic_custom_types_closures_patterns_and_preserves_them_on_disk() {
    let root = project(&[
        (
            "src/container.i",
            r#"
pub type Boxed(a) { Boxed(value: a) }
pub fn map(boxed: Boxed(a), mapper: fn(a) -> b) -> Boxed(b) {
  Boxed(mapper(boxed.value))
}
pub fn identity(value) { value }
"#,
        ),
        (
            "src/app.i",
            r#"
import container
pub fn main() {
  let increment = fn(value) { value + 1 }
  let boxed = container.map(container.Boxed(41), increment)
  let assert container.Boxed(answer) = boxed
  case container.identity(answer) {
    42 -> Nil
    _ -> panic as "unexpected answer"
  }
}
"#,
        ),
    ]);

    let first = check_project(root.path(), &CheckOptions::default()).unwrap();
    assert_eq!(first.stats.checked, 2);
    assert_eq!(first.stats.cache_writes, 2);
    assert_eq!(first.modules[0].name, "container");
    assert_eq!(first.modules[1].name, "app");

    let cached = check_project(root.path(), &CheckOptions::default()).unwrap();
    assert_eq!(cached.stats.disk_hits, 2);
    assert_eq!(cached.stats.parsed, 0);
    assert_eq!(cached.stats.checked, 0);
    assert_eq!(cached.modules[0].definitions.custom_types.len(), 1);
    assert_eq!(cached.modules[0].definitions.functions.len(), 2);
    assert_eq!(
        format!("{:?}", first.modules[1].definitions.functions),
        format!("{:?}", cached.modules[1].definitions.functions),
    );
}

#[test]
fn unchanged_session_reuses_the_actual_typed_tree() {
    let root = project(&[("src/app.i", "pub fn main() { Nil }")]);
    let mut session = Project::load(root.path()).unwrap();
    let first = session.check(&CheckOptions::default()).unwrap();
    let second = session.check(&CheckOptions::default()).unwrap();
    assert_eq!(second.stats.memory_hits, 1);
    assert_eq!(second.stats.disk_hits, 0);
    assert_eq!(second.stats.parsed, 0);
    assert_eq!(second.stats.checked, 0);
    assert!(Arc::ptr_eq(&first.modules[0], &second.modules[0]));
}

#[test]
fn disabling_cache_rechecks_and_writes_no_entries() {
    let root = project(&[("src/app.i", "pub fn main() { Nil }")]);
    let options = CheckOptions {
        use_cache: false,
        ..Default::default()
    };
    let first = check_project(root.path(), &options).unwrap();
    let second = check_project(root.path(), &options).unwrap();
    assert_eq!(first.stats.checked, 1);
    assert_eq!(second.stats.checked, 1);
    assert_eq!(second.stats.cache_writes, 0);
    assert_eq!(second.stats.disk_hits, 0);
    assert!(!root.path().join("build/.ilang/frontend-v1").exists());
}

#[test]
fn private_body_edit_keeps_dependent_typed_tree_cached() {
    let root = project(&[
        (
            "src/dep.i",
            "pub fn answer() -> Int { hidden() }\nfn hidden() -> Int { 41 }\n",
        ),
        ("src/app.i", "import dep\npub fn main() { dep.answer() }\n"),
    ]);
    check_project(root.path(), &CheckOptions::default()).unwrap();
    write(
        root.path(),
        "src/dep.i",
        "pub fn answer() -> Int { hidden() }\nfn hidden() -> Int { 42 }\n",
    );
    let second = check_project(root.path(), &CheckOptions::default()).unwrap();
    assert_eq!(second.stats.checked, 1);
    assert_eq!(second.stats.parsed, 1);
    assert_eq!(second.stats.disk_hits, 1);
    assert!(second.sources["dep"].code.contains("42"));
}

#[test]
fn public_return_type_change_rechecks_the_dependent_module() {
    let root = project(&[
        ("src/dep.i", "pub fn answer() -> Int { 42 }\n"),
        (
            "src/app.i",
            "import dep\npub fn main() { let _ = dep.answer() Nil }\n",
        ),
    ]);
    check_project(root.path(), &CheckOptions::default()).unwrap();
    write(
        root.path(),
        "src/dep.i",
        "pub fn answer() -> String { \"forty-two\" }\n",
    );
    let second = check_project(root.path(), &CheckOptions::default()).unwrap();
    assert_eq!(second.stats.checked, 2);
    assert_eq!(second.stats.disk_hits, 0);
}

#[test]
fn changed_exported_constant_invalidates_inlined_uses() {
    let root = project(&[
        ("src/dep.i", "pub const answer = 41\n"),
        ("src/app.i", "import dep\npub fn main() { dep.answer }\n"),
    ]);
    let first = check_project(root.path(), &CheckOptions::default()).unwrap();
    let old_body = format!("{:?}", first.module("app").unwrap().definitions.functions);
    write(root.path(), "src/dep.i", "pub const answer = 42\n");
    let second = check_project(root.path(), &CheckOptions::default()).unwrap();
    assert_eq!(second.stats.checked, 2);
    assert_eq!(second.stats.disk_hits, 0);
    assert_ne!(
        old_body,
        format!("{:?}", second.module("app").unwrap().definitions.functions)
    );
}

#[test]
fn cached_generic_modules_do_not_alias_new_inference_variables() {
    let root = project(&[
        ("src/generic.i", "pub fn identity(value) { value }\n"),
        (
            "src/app.i",
            "import generic\npub fn main() { #(generic.identity(1), generic.identity(\"one\")) }\n",
        ),
    ]);
    check_project(root.path(), &CheckOptions::default()).unwrap();
    for n in 0..4 {
        write(
            root.path(),
            "src/app.i",
            &format!(
                "import generic\npub fn main() {{ #(duplicate(generic.identity({n})), duplicate(generic.identity(\"one\"))) }}\nfn duplicate(value) {{ #(value, value) }}\n"
            ),
        );
        let checked = check_project(root.path(), &CheckOptions::default()).unwrap();
        assert_eq!(checked.stats.disk_hits, 1);
        assert_eq!(checked.stats.checked, 1);
    }
}

#[test]
fn local_dependency_interfaces_preserve_generic_types_and_package_origins() {
    let root = project(&[(
        "src/app.i",
        "import utility/box\npub fn main() { #(box.unwrap(box.Box(1)), box.unwrap(box.Box(\"hello\"))) }\n",
    )]);
    write(
        root.path(),
        "i.toml",
        "name = \"app\"\nversion = \"1.0.0\"\ntarget = \"wasm\"\n[dependencies]\nutility = { path = \"deps/utility\" }\n",
    );
    write(
        root.path(),
        "deps/utility/i.toml",
        "name = \"utility\"\nversion = \"1.0.0\"\ntarget = \"wasm\"\n",
    );
    write(
        root.path(),
        "deps/utility/src/utility/box.i",
        "pub type Box(a) { Box(value: a) }\npub fn unwrap(boxed: Box(a)) -> a { boxed.value }\n",
    );
    let first = check_project(root.path(), &CheckOptions::default()).unwrap();
    assert_eq!(first.stats.checked, 2);
    assert_eq!(first.modules[0].name, "utility/box");
    assert_eq!(first.sources["utility/box"].package_name, "utility");
    assert_eq!(first.sources["app"].package_name, "app");
    let cached = check_project(root.path(), &CheckOptions::default()).unwrap();
    assert_eq!(cached.stats.disk_hits, 2);
    assert_eq!(cached.stats.checked, 0);
}

#[test]
fn deleted_dependency_is_not_resurrected_from_cache() {
    let root = project(&[
        ("src/dep.i", "pub fn answer() { 42 }\n"),
        ("src/app.i", "import dep\npub fn main() { dep.answer() }\n"),
    ]);
    check_project(root.path(), &CheckOptions::default()).unwrap();
    fs::remove_file(root.path().join("src/dep.i")).unwrap();
    let error = check_project(root.path(), &CheckOptions::default())
        .unwrap_err()
        .to_string();
    assert!(error.contains("dep"), "{error}");
    assert!(error.contains("app.i"), "{error}");
}

#[test]
fn corrupted_disk_entry_is_rebuilt_from_source() {
    let root = project(&[("src/app.i", "pub fn main() { Nil }\n")]);
    check_project(root.path(), &CheckOptions::default()).unwrap();
    let cache = root.path().join("build/.ilang/frontend-v1");
    for entry in fs::read_dir(cache).unwrap() {
        fs::write(entry.unwrap().path(), b"interrupted cache write").unwrap();
    }
    let checked = check_project(root.path(), &CheckOptions::default()).unwrap();
    assert_eq!(checked.stats.disk_hits, 0);
    assert_eq!(checked.stats.checked, 1);
    assert_eq!(checked.stats.cache_writes, 1);
}

#[test]
fn editor_overrides_can_add_unsaved_modules_without_changing_disk() {
    let root = project(&[("src/app.i", "pub fn main() { Nil }\n")]);
    let mut options = CheckOptions::default();
    options.source_overrides.insert(
        "src/app.i".into(),
        "import extra\npub fn main() { extra.answer() }\n".into(),
    );
    options.source_overrides.insert(
        root.path().join("src/extra.i"),
        "pub fn answer() { 42 }\n".into(),
    );
    let checked = check_project(root.path(), &options).unwrap();
    assert_eq!(checked.modules.len(), 2);
    assert!(checked.sources.contains_key("extra"));
    assert!(!root.path().join("src/extra.i").exists());
    assert_eq!(
        fs::read_to_string(root.path().join("src/app.i")).unwrap(),
        "pub fn main() { Nil }\n"
    );
    let saved = check_project(root.path(), &CheckOptions::default()).unwrap();
    assert_eq!(saved.modules.len(), 1);
    assert!(!saved.sources.contains_key("extra"));
}

#[test]
fn test_sources_are_explicitly_opted_in_and_keep_their_origin() {
    let root = project(&[
        ("src/app.i", "pub fn main() { Nil }\n"),
        ("test/app_test.i", "pub fn smoke_test() { Nil }\n"),
    ]);
    let production = check_project(root.path(), &CheckOptions::default()).unwrap();
    assert_eq!(production.modules.len(), 1);
    let options = CheckOptions {
        include_tests: true,
        ..Default::default()
    };
    let tests = check_project(root.path(), &options).unwrap();
    assert_eq!(tests.modules.len(), 2);
    assert!(tests.sources["app_test"].is_test);
    assert_eq!(
        tests.sources["app_test"].origin,
        ilang_core::build::Origin::Test
    );
    assert!(!tests.sources["app"].is_test);
}

#[test]
fn type_error_retains_source_path_and_conflicting_types() {
    let root = project(&[("src/app.i", "pub fn main() { 1 + \"wrong\" }\n")]);
    let error = check_project(root.path(), &CheckOptions::default())
        .unwrap_err()
        .to_string();
    assert!(error.contains("app.i"), "{error}");
    assert!(error.contains("Int"), "{error}");
    assert!(error.contains("String"), "{error}");
}

#[test]
fn runtime_external_can_have_a_shared_generic_signature() {
    let root = project(&[(
        "src/app.i",
        "@external(wasm, \"i\", \"identity\")\npub fn identity(value: a) -> a\npub fn main() { #(identity(1), identity(\"hello\")) }\n",
    )]);
    let checked = check_project(root.path(), &CheckOptions::default()).unwrap();
    assert_eq!(checked.stats.checked, 1);
    assert_eq!(
        checked.module("app").unwrap().definitions.functions.len(),
        2
    );
}

#[test]
fn legacy_platform_attributes_require_wasm_migration() {
    for source in [
        "@external(erlang, \"host\", \"read\")\npub fn read() -> Int\n",
        "@target(javascript)\npub fn main() { Nil }\n",
    ] {
        let root = project(&[("src/app.i", source)]);
        let error = check_project(root.path(), &CheckOptions::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains("app.i"), "{error}");
        assert!(error.contains("wasm"), "{error}");
    }
}

#[test]
fn library_exports_are_available_to_the_backend_without_a_main_function() {
    let root = project(&[("src/api.i", "pub fn answer() { 42 }\n")]);
    write(
        root.path(),
        "i.toml",
        "name = \"app\"\nversion = \"1.0.0\"\n[wasm]\nexports = [\"api.answer\"]\n",
    );
    let checked = check_project(root.path(), &CheckOptions::default()).unwrap();
    assert_eq!(checked.exports, ["api.answer"]);
    assert!(checked.module("api").is_some());
}

#[test]
fn library_exports_require_existing_public_functions() {
    for source in [
        "pub fn another_function() { 42 }\n",
        "fn answer() { 42 }\n",
        "pub const answer = 42\n",
    ] {
        let root = project(&[("src/api.i", source)]);
        write(
            root.path(),
            "i.toml",
            "name = \"app\"\nversion = \"1.0.0\"\n[wasm]\nexports = [\"api.answer\"]\n",
        );
        let error = check_project(root.path(), &CheckOptions::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains("i.toml"), "{error}");
        assert!(error.contains("answer"), "{error}");
        assert!(error.to_lowercase().contains("export"), "{error}");
    }
}

#[test]
fn syntax_error_retains_its_source_path() {
    let root = project(&[("src/app.i", "pub fn main( { Nil }\n")]);
    let error = check_project(root.path(), &CheckOptions::default())
        .unwrap_err()
        .to_string();
    assert!(error.contains("app.i"), "{error}");
    assert!(error.contains("main"), "{error}");
}

#[test]
fn invalid_module_paths_are_reported_instead_of_silently_skipped() {
    for path in ["src/bad-name.i", "src/Upper/name.i"] {
        let root = project(&[(path, "pub fn answer() { 42 }\n")]);
        let error = check_project(root.path(), &CheckOptions::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains(path), "{error}");
        assert!(error.contains("Invalid module name"), "{error}");
    }
}

#[test]
fn import_cycles_are_reported_with_the_modules_involved() {
    let root = project(&[
        (
            "src/app.i",
            "import other\npub fn main() { other.value() }\n",
        ),
        ("src/other.i", "import app\npub fn value() { app.main() }\n"),
    ]);
    let error = check_project(root.path(), &CheckOptions::default())
        .unwrap_err()
        .to_string();
    assert!(error.to_lowercase().contains("cycle"), "{error}");
    assert!(error.contains("app") && error.contains("other"), "{error}");
}

#[test]
fn formatter_is_idempotent_and_retains_wasm_external_attributes() {
    let input = "@external(wasm,\"host\",\"read\")\npub fn read()->Int\npub fn main(){let x=1 case x{1->Nil _->Nil}}\n";
    let once = format_source(Path::new("app.i"), input).unwrap();
    let twice = format_source(Path::new("app.i"), &once).unwrap();
    assert_eq!(once, twice);
    assert!(once.contains("@external(wasm, \"host\", \"read\")"));
}
