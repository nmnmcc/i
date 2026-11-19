//! Round-trip `tests/syn3/block_nesting` through the Lisp surface and the
//! source printer, then run both programs. Scope and order must agree.
use ilang_syntax::{parse_module, print, syn3};
use std::path::{Path, PathBuf};

/// The `[dependencies]` table with the standard library `i` of this
/// repository, which every root package needs (E0641).
fn library_dependencies() -> String {
    let library = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../stdlib")
        .canonicalize()
        .unwrap();
    format!(
        "[dependencies]\ni = {{ path = {:?} }}\n",
        library.to_str().unwrap()
    )
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/syn3/block_nesting")
}

fn run(dir: &Path) -> (bool, String, String) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["run", "--project"])
        .arg(dir)
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn a_round_tripped_program_keeps_scope_and_order() {
    let source = std::fs::read_to_string(fixture().join("src/main.i")).unwrap();
    let (module, errors) = parse_module(&source);
    assert!(errors.is_empty(), "{errors:?}");
    let lisp = syn3::lisp::to_lisp(&module);
    let (read, errors) = syn3::lisp::from_lisp(&lisp);
    assert!(errors.is_empty(), "{errors:?}");
    // One printing is a fixed point of the Lisp surface.
    assert_eq!(syn3::lisp::to_lisp(&read), lisp);
    let converted = print::print_module(&read);
    let (again, errors) = parse_module(&converted);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(again, read, "{converted}");

    // Keep the converted source for the record.
    let kept = std::env::temp_dir().join("g31b-csv-roundtrip");
    std::fs::create_dir_all(&kept).unwrap();
    std::fs::write(kept.join("block_nesting.i"), &converted).unwrap();

    let expected = std::fs::read_to_string(fixture().join("expected.txt")).unwrap();
    let (ok, out, err) = run(&fixture());
    assert!(ok, "the fixture failed:\n{err}");
    assert_eq!(out, expected);
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("i.toml"),
        format!(
            "name = \"block_nesting_roundtrip\"\n{}",
            library_dependencies()
        ),
    )
    .unwrap();
    std::fs::write(dir.path().join("src/main.i"), &converted).unwrap();
    let (ok, out, err) = run(dir.path());
    assert!(ok, "the round-tripped program failed:\n{err}\n{converted}");
    assert_eq!(out, expected, "{converted}");
}
