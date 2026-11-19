//! `i test` selects tests by `[test].collection`; a value that selects
//! nothing, or is not a string, is an error and never a green "0 passed".

#[path = "common/test_project.rs"]
mod test_project;
use std::path::Path;
use std::process::Command;

fn package(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (path, text) in files {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    if files.iter().any(|(_, source)| source.contains("std/test")) {
        test_project::configure(dir.path());
    }
    dir
}

fn i(args: &[&str], project: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_i"))
        .args(args)
        .arg("--project")
        .arg(project)
        .output()
        .unwrap()
}

/// A package with one passing test in `std/test.tests`.
fn tested() -> tempfile::TempDir {
    package(&[
        ("i.toml", "name = \"app\"\n"),
        ("src/lib.i", " (twice)\n\ntwice = (n : Int) { n * 2 }\n"),
        (
            "test/lib_test.i",
            "(\n\ttestlib = (test) = std/test\n\tapp/lib\n)\n\ntest(\"good\", () { lib.twice(2) |> (4) { () } & (_) { panic(\"no\") } })\n",
        ),
    ])
}

/// Sets `[test].collection` of the package to `value`.
fn set_collection(project: &Path, value: toml_edit::Item) {
    let path = project.join("i.toml");
    let mut manifest: toml_edit::DocumentMut =
        std::fs::read_to_string(&path).unwrap().parse().unwrap();
    manifest["test"]["collection"] = value;
    std::fs::write(path, manifest.to_string()).unwrap();
}

#[test]
fn a_missing_collection_is_an_error_not_zero_tests() {
    let dir = tested();
    set_collection(dir.path(), toml_edit::value("std/test.nothing"));
    let out = i(&["test"], dir.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stdout}{stderr}");
    assert!(
        stderr.contains("there is no test collection \"std/test.nothing\""),
        "{stderr}"
    );
    assert!(!stdout.contains("passed"), "{stdout}");
}

#[test]
fn a_non_string_collection_is_rejected() {
    let dir = tested();
    for (value, kind) in [
        (toml_edit::value(1), "integer"),
        (
            toml_edit::value(toml_edit::Array::from_iter(["std/test.tests"])),
            "array",
        ),
    ] {
        set_collection(dir.path(), value);
        let out = i(&["test"], dir.path());
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{stdout}{stderr}");
        assert!(
            stderr.contains(&format!(
                "error: `[test].collection` must be a string, not {kind}"
            )),
            "{stderr}"
        );
        assert!(!stdout.contains("passed"), "{stdout}");
    }
}

#[test]
fn the_configured_collection_still_runs_its_tests() {
    let dir = tested();
    let out = i(&["test"], dir.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("pass app/lib_test.good"), "{stdout}");
    assert!(stdout.contains("1 passed, 0 failed"), "{stdout}");
}

/// A package with five passing tests, `app/many_test.a` to `.e`, and a cost
/// table that makes two shards of them.
fn sharded() -> (tempfile::TempDir, std::path::PathBuf) {
    let mut test = String::from("(\n\ttestlib = (test) = std/test\n\tapp/lib\n)\n");
    for name in ["a", "b", "c", "d", "e"] {
        test.push_str(&format!(
            "\ntest(\"{name}\", () {{ lib.twice(2) |> (4) {{ () }} & (_) {{ panic(\"no\") }} }})\n"
        ));
    }
    let dir = package(&[
        ("i.toml", "name = \"app\"\n"),
        ("src/lib.i", " (twice)\n\ntwice = (n : Int) { n * 2 }\n"),
        ("test/many_test.i", &test),
    ]);
    let costs = dir.path().join("costs.json");
    std::fs::write(
        &costs,
        r#"{"version":1,"budget_seconds":10,"sets":{"ilang-cli-tests":{"keys":["app/many_test.a","app/many_test.b","app/many_test.c","app/many_test.d"],"costs":{"app/many_test.a":4,"app/many_test.b":4,"app/many_test.c":4,"app/many_test.d":4}}}}"#,
    )
    .unwrap();
    (dir, costs)
}

fn i_costs(args: &[&str], project: &Path, costs: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_i"))
        .args(args)
        .arg("--project")
        .arg(project)
        .env("CHECKS_COSTS", costs)
        .output()
        .unwrap()
}

/// The names of the tests that ran, from `pass NAME` and `FAIL NAME` lines.
fn ran(out: &std::process::Output) -> Vec<String> {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.strip_prefix("pass ").or(line.strip_prefix("FAIL ")))
        .map(str::to_string)
        .collect()
}

const ALL: [&str; 5] = [
    "app/many_test.a",
    "app/many_test.b",
    "app/many_test.c",
    "app/many_test.d",
    "app/many_test.e",
];

#[test]
fn without_shard_every_test_still_runs() {
    let (dir, costs) = sharded();
    let out = i_costs(&["test"], dir.path(), &costs);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(ran(&out), ALL, "{stdout}");
    assert!(stdout.contains("5 passed, 0 failed"), "{stdout}");
}

#[test]
fn shards_cover_every_test_once() {
    let (dir, costs) = sharded();
    let mut seen = Vec::new();
    for shard in ["0/2", "1/2"] {
        let out = i_costs(&["test", "--shard", shard], dir.path(), &costs);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "{shard}: {stdout}{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let names = ran(&out);
        assert!(!names.is_empty(), "{shard}: {stdout}");
        assert!(names.len() < ALL.len(), "{shard}: {stdout}");
        seen.extend(names);
    }
    let count = seen.len();
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), count, "a test ran in both shards: {seen:?}");
    assert_eq!(seen, ALL);
}

#[test]
fn the_same_shard_lists_the_same_tests() {
    let (dir, costs) = sharded();
    let first = i_costs(&["test", "--shard", "0/2"], dir.path(), &costs);
    let second = i_costs(&["test", "--shard", "0/2"], dir.path(), &costs);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&second.stdout)
    );
}

#[test]
fn the_name_filter_applies_within_the_shard() {
    let (dir, costs) = sharded();
    let shard = ran(&i_costs(&["test", "--shard", "0/2"], dir.path(), &costs));
    let other = ALL
        .iter()
        .find(|name| !shard.iter().any(|s| s == *name))
        .unwrap();
    let out = i_costs(&["test", "--shard", "0/2", other], dir.path(), &costs);
    assert!(
        ran(&out).is_empty(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let out = i_costs(&["test", "--shard", "0/2", &shard[0]], dir.path(), &costs);
    assert_eq!(ran(&out), [shard[0].clone()]);
}

#[test]
fn a_bad_shard_is_rejected() {
    let (dir, costs) = sharded();
    for (shard, says) in [
        ("2/2", "--shard 2/2: shard 2 is out of range"),
        ("0/0", "--shard 0/0: the number of shards must be positive"),
        ("x/2", "--shard x/2: expected K/N"),
        ("0/3", "--shard 0/3: the 5 tests make 2 shards, not 3"),
    ] {
        let out = i_costs(&["test", "--shard", shard], dir.path(), &costs);
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{shard}: {stdout}{stderr}");
        assert!(stderr.contains(says), "{shard}: {stderr}");
        assert!(ran(&out).is_empty(), "{shard}: {stdout}");
    }
}

#[test]
fn a_missing_cost_table_gives_every_test_the_same_cost() {
    let (dir, _) = sharded();
    let missing = dir.path().join("no-such-costs.json");
    let out = i_costs(&["test", "--shard", "0/1"], dir.path(), &missing);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert_eq!(ran(&out), ALL);
    let notes: Vec<&str> = stderr
        .lines()
        .filter(|l| l.contains("every test costs the same"))
        .collect();
    assert_eq!(notes.len(), 1, "{stderr}");
}
