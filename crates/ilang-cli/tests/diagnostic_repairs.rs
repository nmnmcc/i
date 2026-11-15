//! Reproduce the documentation-only trial with its original, unedited programs.
//! Archive: /home/agent/Programming/i/.rd/cache/ai-eval/round2/runs/subjects/gpt-6-luna/condition-2/
//! Each fixture's exact archive path and SHA-256 are in fixtures/diagnostic_repairs/README.md.

use std::path::Path;
use std::process::Command;

fn check(files: &[(&str, &str)]) -> String {
    let dir = tempfile::tempdir().unwrap();
    let std = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../std")
        .canonicalize()
        .unwrap();
    std::fs::write(
        dir.path().join("i.toml"),
        format!(
            "name = \"app\"\n[dependencies]\nstd = {{ path = {:?} }}\n",
            std.to_str().unwrap()
        ),
    )
    .unwrap();
    for (name, source) in files {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source).unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["check", "--offline", "--project"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    String::from_utf8(output.stderr).unwrap()
}

#[test]
fn original_model_programs_get_actionable_repairs() {
    macro_rules! archived {
        ($name:literal) => {
            (
                $name,
                include_str!(concat!("fixtures/diagnostic_repairs/", $name, ".txt")),
                include_str!(concat!("fixtures/diagnostic_repairs/", $name, ".stderr")),
            )
        };
    }
    // Every input here is byte-for-byte the model's project/src/main.i.
    // Exact stderr pins all original error codes, positions and counts. E4's
    // Fail-based bit_array.to_string also warns about the archive's unchanged
    // Ok pattern; its two additional warnings are retained in t5-a1-r3.stderr.
    for (name, source, expected) in [
        // T1/i/attempt-1/execution-00/round-3/project/src/main.i
        archived!("t1-a1-r3"),
        // T1/i/attempt-2/execution-00/round-3/project/src/main.i
        archived!("t1-a2-r3"),
        // T2/i/attempt-1/execution-00/round-3/project/src/main.i
        archived!("t2-a1-r3"),
        // T2/i/attempt-2/execution-00/round-3/project/src/main.i
        archived!("t2-a2-r3"),
        // T5/i/attempt-1/execution-00/round-3/project/src/main.i
        archived!("t5-a1-r3"),
        // T5/i/attempt-2/execution-00/round-3/project/src/main.i
        archived!("t5-a2-r3"),
        // T5/i/attempt-1/execution-00/round-1/project/src/main.i
        archived!("t5-a1-r1"),
        // T6/i/attempt-2/execution-00/round-3/project/src/main.i
        archived!("t6-a2-r3"),
    ] {
        assert_eq!(check(&[("src/main.i", source)]), expected, "{name}");
    }
}

#[test]
fn import_candidates_are_sorted_limited_and_respect_existing_imports() {
    let diagnostics = check(&[
        ("src/main.i", "(app/z)\nmain = () { missing() }\n"),
        ("src/z.i", "(missing)\nmissing = () { Nil }\n"),
        ("src/c.i", "(missing)\nmissing = () { Nil }\n"),
        ("src/b.i", "(missing)\nmissing = () { Nil }\n"),
        ("src/a.i", "(missing)\nmissing = () { Nil }\n"),
    ]);
    assert!(diagnostics.contains("`missing` is not defined. Import `(missing) = app/a` to use `missing`. Import `(missing) = app/b` to use `missing`. Import `(missing) = app/c` to use `missing`.\n"), "{diagnostics}");
    assert!(
        !diagnostics.contains("existing import of `app/z`"),
        "{diagnostics}"
    );
    let diagnostics = check(&[("src/main.i", "(std/option)\nmain = () { Some(1) }\n")]);
    assert!(
        diagnostics.contains(
            "Add `Some` to the names in your existing import of `std/option`, or use `option.Some`."
        ),
        "{diagnostics}"
    );
}

#[test]
fn same_named_service_advice_works_for_user_modules() {
    let diagnostics = check(&[
        (
            "src/main.i",
            "((Channel) = app/settings\napp/service)\nmain = () <- (stream : Channel) { Nil }\n",
        ),
        (
            "src/settings.i",
            "(Channel)\nLeft = ()\nRight = ()\nChannel = Left | Right\n",
        ),
        (
            "src/service.i",
            "(Channel(..))\nChannel = (write = (String) -> Nil)\n",
        ),
    ]);
    assert!(diagnostics.contains("`Channel` here comes from `app/settings`. `app/service` also exports a type named `Channel`. Use `service.Channel` if that is the type you intended."), "{diagnostics}");
}

#[test]
fn wrong_standard_namespace_is_suggested_in_both_directions() {
    let diagnostics = check(&[("src/main.i", "(std/process)\nmain = () { Nil }\n")]);
    assert!(
        diagnostics.contains(
            "there is no module `std/process`. Import `i/process` instead; that module exists."
        ),
        "{diagnostics}"
    );
}

#[test]
fn one_item_data_repair_also_explains_numeric_scalars() {
    let diagnostics = check(&[("src/main.i", "main = () { (1 + 2) + 3 }\n")]);
    assert!(diagnostics.contains("Parentheses construct one-item data. Use braces `{ }` to group an expression: `{ 1 + 2 }`."), "{diagnostics}");
    let diagnostics = check(&[("src/main.i", "main = () { (label = 1) + 3 }\n")]);
    assert!(
        !diagnostics.contains("Parentheses construct"),
        "{diagnostics}"
    );
}
