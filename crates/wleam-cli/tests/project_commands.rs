use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn cli(root: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wleam"))
        .arg("--project")
        .arg(root)
        .args(arguments)
        .output()
        .expect("start wleam")
}

fn project(source: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("src")).unwrap();
    fs::write(root.path().join("gleam.toml"), "name = \"sample\"\nversion = \"0.1.0\"\ntarget = \"wasm\"\n[wasm]\nexports = [\"sample.add\"]\n").unwrap();
    fs::write(root.path().join("src/sample.gleam"), source).unwrap();
    root
}

fn successful_json(output: Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn build_cache_reuses_valid_output_and_recovers_from_corruption_and_source_edits() {
    let root = project("pub fn add(a: Int, b: Int) -> Int { a + b }\n");
    let args = ["build", "--lib", "--json"];
    let first = successful_json(cli(root.path(), &args));
    assert_eq!(first["artifact_cache_hit"], false);
    assert!(first["lowering"]["lowered"].as_u64().unwrap() > 0);
    let output = root.path().join("build/wasm/sample.wasm");
    let original = fs::read(&output).unwrap();
    assert_eq!(&original[..4], b"\0asm");
    let second = successful_json(cli(root.path(), &args));
    assert_eq!(second["artifact_cache_hit"], true);
    assert_eq!(second["cache"]["checked"], 0);

    fs::write(&output, b"damaged artifact").unwrap();
    let repaired = successful_json(cli(root.path(), &args));
    assert_eq!(repaired["artifact_cache_hit"], false);
    assert_eq!(repaired["lowering"]["lowered"], 0);
    assert!(repaired["lowering"]["hits"].as_u64().unwrap() > 0);
    assert_eq!(fs::read(&output).unwrap(), original);

    fs::write(
        root.path().join("src/sample.gleam"),
        "pub fn add(a: Int, b: Int) -> Int { a - b }\n",
    )
    .unwrap();
    let edited = successful_json(cli(root.path(), &args));
    assert_eq!(edited["artifact_cache_hit"], false);
    assert_ne!(fs::read(&output).unwrap(), original);
    let uncached = successful_json(cli(
        root.path(),
        &["build", "--lib", "--json", "--no-cache"],
    ));
    assert_eq!(uncached["artifact_cache_hit"], false);
    assert!(uncached.get("lowering").is_none());
    assert!(uncached["cache"]["checked"].as_u64().unwrap() > 0);
}

#[test]
fn dependency_body_edit_reuses_the_callers_frontend_and_ir() {
    let root = project("import helper\npub fn add(a: Int, b: Int) -> Int { helper.add(a, b) }\n");
    let helper = root.path().join("src/helper.gleam");
    fs::write(&helper, "pub fn add(a: Int, b: Int) -> Int { a + b }\n").unwrap();
    let args = ["build", "--lib", "--json"];
    successful_json(cli(root.path(), &args));
    fs::write(&helper, "pub fn add(a: Int, b: Int) -> Int { a - b }\n").unwrap();
    let changed = successful_json(cli(root.path(), &args));
    assert_eq!(changed["cache"]["checked"], 1);
    assert!(changed["cache"]["disk_hits"].as_u64().unwrap() >= 1);
    assert!(changed["lowering"]["hits"].as_u64().unwrap() >= 1);
    assert_eq!(changed["lowering"]["lowered"], 1);
}

#[test]
fn invalid_source_fails_without_replacing_the_last_successful_artifact() {
    let root = project("pub fn add(a: Int, b: Int) -> Int { a + b }\n");
    successful_json(cli(root.path(), &["build", "--lib", "--json"]));
    let output = root.path().join("build/wasm/sample.wasm");
    let previous = fs::read(&output).unwrap();
    fs::write(
        root.path().join("src/sample.gleam"),
        "pub fn add(a: Int, b: Int) -> Int { a + \"wrong\" }\n",
    )
    .unwrap();
    let failed = cli(root.path(), &["build", "--lib"]);
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("sample.gleam"));
    assert_eq!(fs::read(&output).unwrap(), previous);
}

#[test]
fn formatter_parses_all_inputs_before_writing_anything() {
    let root = project("pub fn add(a:Int,b:Int)->Int{a+b}\n");
    let path = root.path().join("src/sample.gleam");
    let previous = fs::read(&path).unwrap();
    fs::write(root.path().join("src/invalid.gleam"), "pub fn broken( {").unwrap();
    assert!(!cli(root.path(), &["fmt"]).status.success());
    assert_eq!(fs::read(&path).unwrap(), previous);
    fs::remove_file(root.path().join("src/invalid.gleam")).unwrap();
    assert!(!cli(root.path(), &["fmt", "--check"]).status.success());
    assert!(cli(root.path(), &["fmt"]).status.success());
    let once = fs::read(&path).unwrap();
    assert!(cli(root.path(), &["fmt", "--check"]).status.success());
    assert!(cli(root.path(), &["fmt"]).status.success());
    assert_eq!(fs::read(&path).unwrap(), once);
}

#[test]
fn build_rejects_an_output_path_that_would_collide_with_a_companion_file() {
    let root = project("pub fn add(a: Int, b: Int) -> Int { a + b }\n");
    let output = root.path().join("library.mjs");
    fs::write(&output, "preserved").unwrap();
    let failed = cli(
        root.path(),
        &["build", "--lib", "--out", output.to_str().unwrap()],
    );
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("must end in .wasm"));
    assert_eq!(fs::read_to_string(&output).unwrap(), "preserved");
}

#[test]
fn default_application_entry_cannot_come_from_a_dependency() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("src")).unwrap();
    fs::create_dir_all(root.path().join("dependency/src")).unwrap();
    fs::write(root.path().join("gleam.toml"), "name = \"sample\"\nversion = \"0.1.0\"\n[dependencies]\ndependency = { path = \"dependency\" }\n").unwrap();
    fs::write(
        root.path().join("src/another.gleam"),
        "pub fn main() { Nil }\n",
    )
    .unwrap();
    fs::write(
        root.path().join("dependency/gleam.toml"),
        "name = \"dependency\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(
        root.path().join("dependency/src/sample.gleam"),
        "pub fn main() { Nil }\n",
    )
    .unwrap();
    let failed = cli(root.path(), &["build"]);
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("must define src/sample.gleam"));
    assert!(!root.path().join("build/wasm/sample.wasm").exists());
    successful_json(cli(
        root.path(),
        &["build", "--entry", "another.main", "--json"],
    ));
}
