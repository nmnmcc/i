//! Reproduce the documentation-only trial with its original, unedited programs.
//! Archive: /home/agent/Programming/i/.rd/cache/ai-eval/round2/runs/subjects/gpt-6-luna/condition-2/
//! Each fixture's exact archive path and SHA-256 are in fixtures/diagnostic_repairs/README.md.

use std::path::Path;
use std::process::Command;

fn check(files: &[(&str, &str)]) -> String {
    check_status(files, 1)
}

fn check_status(files: &[(&str, &str)], status: i32) -> String {
    let dir = tempfile::tempdir().unwrap();
    let library = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../stdlib")
        .canonicalize()
        .unwrap();
    std::fs::write(
        dir.path().join("i.toml"),
        format!(
            "name = \"app\"\n[dependencies]\ni = {{ path = {:?} }}\n",
            library.to_str().unwrap()
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
    assert_eq!(output.status.code(), Some(status));
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
    // Every input here is the model's project/src/main.i, byte for byte but
    // for value-level chain separators rewritten from `|` to `&` (see README).
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
        let diagnostics = check(&[("src/main.i", source)]);
        assert_eq!(diagnostics, expected, "{name}");
        assert!(!diagnostics.contains('§'), "{name}: {diagnostics}");
    }
}

#[test]
fn import_candidates_are_sorted_limited_and_respect_existing_imports() {
    let diagnostics = check(&[
        ("src/main.i", "(app/z)\nmain = () { missing() }\n"),
        ("src/z.i", "(missing)\nmissing = () { () }\n"),
        ("src/c.i", "(missing)\nmissing = () { () }\n"),
        ("src/b.i", "(missing)\nmissing = () { () }\n"),
        ("src/a.i", "(missing)\nmissing = () { () }\n"),
    ]);
    assert!(diagnostics.contains("`missing` is not defined. Import `(missing) = app/a` to use `missing`. Import `(missing) = app/b` to use `missing`. Import `(missing) = app/c` to use `missing`.\n"), "{diagnostics}");
    assert!(
        !diagnostics.contains("existing import of `app/z`"),
        "{diagnostics}"
    );
    let diagnostics = check(&[("src/main.i", "(i/option)\nmain = () { Some(1) }\n")]);
    assert!(
        diagnostics.contains(
            "Add `Some` to the names in your existing import of `i/option`, or use `option.Some`."
        ),
        "{diagnostics}"
    );
}

#[test]
fn same_named_service_advice_works_for_user_modules() {
    let diagnostics = check(&[
        (
            "src/main.i",
            "((Channel) = app/settings\napp/service)\nmain = () <- (stream : Channel) { () }\n",
        ),
        (
            "src/settings.i",
            "(Channel)\nLeft = ()\nRight = ()\nChannel = Left | Right\n",
        ),
        (
            "src/service.i",
            "(Channel(..))\nChannel = (write = (String) -> ())\n",
        ),
    ]);
    assert!(diagnostics.contains("`Channel` here comes from `app/settings`. `app/service` also exports a type named `Channel`. Use `service.Channel` if that is the type you intended."), "{diagnostics}");
}

#[test]
fn wrong_standard_namespace_is_suggested_in_both_directions() {
    let diagnostics = check(&[("src/main.i", "(std/process)\nmain = () { () }\n")]);
    assert!(
        diagnostics.contains(
            "`std` is not the name of this package or a dependency name; the standard library is `i`, so write `i/process`"
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

#[test]
fn a_literal_outside_all_branches_gets_one_argument_error() {
    // Supervisor's original app.word(2) example. No matching archive program
    // exists; unlike the archived fixtures, this is a new regression input.
    let diagnostics = check(&[
        (
            "src/main.i",
            "(app = app/words)\nbad = app.word(2)\n_ = bad\n",
        ),
        (
            "src/words.i",
            "(word)\nword = (0) { \"zero\" } & (1) { \"one\" }\n",
        ),
    ]);
    assert_eq!(
        diagnostics,
        "src/main.i:2:16: error[E0501]: `2` is not one of the values `app.word` accepts for argument 1: `0 | 1`\n1 error\n"
    );
    // Strings and a later argument use the same rule, including piped calls.
    for source in [
        "word = (\"zero\") { 0 } & (\"one\") { 1 }\nbad = word(\"two\")\n_ = bad\n",
        "word = (x : Int, 0) { x } & (x : Int, 1) { x }\nbad = word(3, 2)\n_ = bad\n",
        "word = (0) { \"zero\" } & (1) { \"one\" }\nbad = 2 |> word()\n_ = bad\n",
    ] {
        let diagnostics = check(&[("src/main.i", source)]);
        assert_eq!(diagnostics.matches("error[").count(), 1, "{diagnostics}");
        assert!(diagnostics.contains("error[E0501]"), "{diagnostics}");
        assert!(
            diagnostics.contains("accepts for argument"),
            "{diagnostics}"
        );
    }
    let diagnostics = check_status(
        &[(
            "src/main.i",
            "word = (0) { \"zero\" } & (1) { \"one\" }\ngood = word(0)\n_ = good\n",
        )],
        0,
    );
    assert!(diagnostics.is_empty(), "{diagnostics}");
}

#[test]
fn an_unmatched_combination_still_reports_branch_selection() {
    let diagnostics = check(&[(
        "src/main.i",
        "word = (0, 0) { \"zero\" } & (1, 1) { \"one\" }\nbad = word(0, 1)\n_ = bad\n",
    )]);
    // The definition independently fails coverage: keep that E0516 too.
    assert_eq!(
        diagnostics.matches("error[E0552]").count(),
        1,
        "{diagnostics}"
    );
    assert_eq!(
        diagnostics.matches("error[E0516]").count(),
        1,
        "{diagnostics}"
    );
    assert!(!diagnostics.contains("error[E0501]"), "{diagnostics}");
    assert!(
        diagnostics.contains("known types are `(0, 1)`"),
        "{diagnostics}"
    );
    assert!(!diagnostics.contains('§'), "{diagnostics}");
}

#[test]
fn unmatched_types_keep_grouping_advice_and_independent_argument_errors() {
    let diagnostics = check(&[(
        "src/main.i",
        "word = (0) { \"zero\" } & (1) { \"one\" }\nbad = word(True)\n_ = bad\n",
    )]);
    assert_eq!(
        diagnostics,
        "src/main.i:2:12: error[E0501]: `word` does not accept argument 1 with type `True`; this argument must be `0 | 1`\n1 error\n"
    );
    let diagnostics = check(&[(
        "src/main.i",
        "word = (0) { \"zero\" } & (1) { \"one\" }\nbad = word((2))\n_ = bad\n",
    )]);
    assert_eq!(
        diagnostics,
        "src/main.i:2:12: error[E0501]: `word` does not accept argument 1 with type `(2)`; this argument must be `0 | 1`. Parentheses construct one-item data. Use braces `{ }` to group an expression: `{ 2 }`.\n1 error\n"
    );
    let diagnostics = check(&[(
        "src/main.i",
        "word = (0) { \"zero\" } & (1) { \"one\" }\nbad = word({ value : String = 2\n _ = value\n \"two\" })\n_ = bad\n",
    )]);
    assert_eq!(
        diagnostics.matches("error[E0501]").count(),
        2,
        "{diagnostics}"
    );
    assert!(!diagnostics.contains("error[E0552]"), "{diagnostics}");
    assert!(
        diagnostics.contains("`2` is not a subtype of `String`"),
        "{diagnostics}"
    );
    assert!(
        diagnostics
            .contains("`\"two\"` is not one of the values `word` accepts for argument 1: `0 | 1`"),
        "{diagnostics}"
    );
    let diagnostics = check(&[(
        "src/main.i",
        "word = (0, 0 | 1) { \"zero\" } & (1, 0 | 1) { \"one\" }\nbad = word(2, 3)\n_ = bad\n",
    )]);
    assert_eq!(
        diagnostics.matches("error[E0501]").count(),
        2,
        "{diagnostics}"
    );
    assert!(!diagnostics.contains("error[E0552]"), "{diagnostics}");
    assert!(diagnostics.contains("argument 1: `0 | 1`"), "{diagnostics}");
    assert!(diagnostics.contains("argument 2: `0 | 1`"), "{diagnostics}");
}
