//! Written keys and constraint notes retain usable source positions.
use std::path::Path;
use std::process::Command;

fn check(case: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["check", "--offline", "--project"])
        .arg(root.join("tests/spec").join(case))
        .output()
        .unwrap();
    assert!(!output.status.success());
    String::from_utf8(output.stderr).unwrap()
}

#[test]
fn macro_arguments_keep_numeric_and_named_key_positions() {
    let stderr = check("4.8/key_diagnostic_positions");
    let errors: Vec<_> = stderr.lines().filter(|l| l.contains("error[")).collect();
    let positions = ["4:21", "5:21", "6:27", "7:21", "8:22"];
    assert_eq!(errors.len(), positions.len(), "{stderr}");
    for (error, position) in errors.iter().zip(positions) {
        assert!(
            error.starts_with(&format!("src/main.i:{position}: error[E0506]:")),
            "{stderr}"
        );
    }
}

#[test]
fn lisp_keys_keep_their_own_tokens() {
    let stderr = check("4.8/key_diagnostic_lisp");
    let errors: Vec<_> = stderr.lines().filter(|l| l.contains("error[")).collect();
    let positions = ["1:27", "2:27", "3:29"];
    assert_eq!(errors.len(), positions.len(), "{stderr}");
    for (error, position) in errors.iter().zip(positions) {
        assert!(
            error.starts_with(&format!("src/main.is:{position}: error[E0506]:")),
            "{stderr}"
        );
    }
}

#[test]
fn macro_argument_type_notes_use_written_source() {
    let stderr = check("14.9/type_note_argument");
    assert_eq!(
        stderr.lines().filter(|l| l.contains("error[")).count(),
        2,
        "{stderr}"
    );
    assert!(stderr.contains("error[E0515]"), "{stderr}");
    assert!(
        stderr.contains("src/main.i:16:61: error[E0501]"),
        "{stderr}"
    );
    assert!(
        stderr.contains("(the type at src/main.i:16:55 needs it)"),
        "{stderr}"
    );
    assert!(!stderr.contains("18:88"), "{stderr}");
}

#[test]
fn template_type_notes_use_the_macro_call_site() {
    let stderr = check("14.9/type_note_template");
    assert_eq!(
        stderr.lines().filter(|l| l.contains("error[")).count(),
        1,
        "{stderr}"
    );
    assert!(stderr.contains("src/main.i:2:9: error[E0501]"), "{stderr}");
    assert!(
        stderr.contains("(the type at src/main.i:2:9 needs it)"),
        "{stderr}"
    );
    assert!(
        stderr.contains("src/macros.is:2:72: note: expanded from this template"),
        "{stderr}"
    );
}
