//! `i fmt --to`: it refuses a whole run before writing anything when a
//! conversion would drop text or a target exists, keeps the source intact when
//! a write fails, and converts an ordinary module in either direction.
//!
//! The checks in this file pin the observable behaviour: the exit code, the
//! exact diagnostic lines (file, line, column and the text that would be lost),
//! and the exact bytes and existence of the source and target files. They are
//! written against the contract in `docs/spec.md`: conversion checks every file
//! first and writes nothing; a conversion that would drop a comment or other
//! content, or whose target exists, refuses the whole run; a write failure
//! keeps the source and lists the files already converted; a new file is fully
//! written before its original is removed.
//!
//! A successful conversion changes layout, token spelling and source spans by
//! definition, so the success tests compare the parsed module instead of the
//! bytes. `ilang_syntax::ast::Sp` compares equal regardless of the offsets it
//! holds, so `Module` equality ignores positions and keeps every other datum,
//! including string payloads and module documentation.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

fn i(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_i"))
        .current_dir(dir)
        .arg("fmt")
        .args(args)
        .output()
        .unwrap()
}

fn bytes(dir: &Path, path: &str) -> Vec<u8> {
    std::fs::read(dir.join(path)).unwrap()
}

fn text(dir: &Path, path: &str) -> String {
    String::from_utf8(bytes(dir, path)).unwrap()
}

fn stderr_of(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).unwrap()
}

fn assert_missing(dir: &Path, path: &str) {
    assert!(
        !dir.join(path).exists(),
        "{path} should not exist, but it does"
    );
}

fn write_file(dir: &Path, path: &str, contents: &str) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

/// Parse a `.i` module, insisting that it parses.
fn module_of_i(source: &str) -> ilang_syntax::ast::Module {
    let (module, errors) = ilang_syntax::parse_module(source);
    assert!(errors.is_empty(), "unexpected .i parse errors: {errors:?}");
    module
}

/// Parse a `.is` module, insisting that it parses.
fn module_of_lisp(source: &str) -> ilang_syntax::ast::Module {
    let (module, errors) = ilang_syntax::lisp::parse_source(source);
    assert!(errors.is_empty(), "unexpected .is parse errors: {errors:?}");
    module
}

fn assert_parsed_equal(left: &ilang_syntax::ast::Module, right: &ilang_syntax::ast::Module) {
    assert_eq!(left, right, "the modules are not structurally equivalent");
}

fn package() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

// ---- a) comments that would be lost, in both directions -------------------

#[test]
fn trailing_comment_is_refused_i_to_is() {
    let dir = package();
    let source = "f = () { 1 } // trailing\n";
    write_file(dir.path(), "a.i", source);

    let out = i(dir.path(), &["--to", "is", "a.i"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr_of(&out));
    assert_eq!(out.stdout, b"");
    assert_eq!(
        stderr_of(&out),
        "a.i:1:14: error: converting to `is` would lose this text: \"// trailing\"\n\
         a.i: the source is unchanged; nothing in this run was converted\n\
         no files were converted: the check phase refused the batch and nothing was written\n\
         files not converted (all left as they were):\n\
         \x20 a.i\n"
    );
    assert_eq!(bytes(dir.path(), "a.i"), source.as_bytes());
    assert_missing(dir.path(), "a.is");
}

#[test]
fn nested_trailing_comment_is_refused_i_to_is() {
    let dir = package();
    let source = "f = () {\n  g = () {\n    1 // nested trailing\n  }\n  g()\n}\n";
    write_file(dir.path(), "a.i", source);

    let out = i(dir.path(), &["--to", "is", "a.i"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr_of(&out));
    assert_eq!(out.stdout, b"");
    assert_eq!(
        stderr_of(&out),
        "a.i:3:7: error: converting to `is` would lose this text: \"// nested trailing\"\n\
         a.i: the source is unchanged; nothing in this run was converted\n\
         no files were converted: the check phase refused the batch and nothing was written\n\
         files not converted (all left as they were):\n\
         \x20 a.i\n"
    );
    assert_eq!(bytes(dir.path(), "a.i"), source.as_bytes());
    assert_missing(dir.path(), "a.is");
}

#[test]
fn eof_comment_without_newline_is_refused_i_to_is() {
    let dir = package();
    // No final newline: the last comment has no line break to sit on.
    let source = "f = () { 1 }\n// last note";
    write_file(dir.path(), "a.i", source);

    let out = i(dir.path(), &["--to", "is", "a.i"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr_of(&out));
    assert_eq!(out.stdout, b"");
    assert_eq!(
        stderr_of(&out),
        "a.i:2:1: error: converting to `is` would lose this text: \"// last note\"\n\
         a.i: the source is unchanged; nothing in this run was converted\n\
         no files were converted: the check phase refused the batch and nothing was written\n\
         files not converted (all left as they were):\n\
         \x20 a.i\n"
    );
    assert_eq!(bytes(dir.path(), "a.i"), source.as_bytes());
    assert_missing(dir.path(), "a.is");
}

#[test]
fn eof_comment_with_newline_is_refused_i_to_is() {
    let dir = package();
    let source = "f = () { 1 }\n// last note\n";
    write_file(dir.path(), "a.i", source);

    let out = i(dir.path(), &["--to", "is", "a.i"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr_of(&out));
    assert_eq!(
        stderr_of(&out),
        "a.i:2:1: error: converting to `is` would lose this text: \"// last note\"\n\
         a.i: the source is unchanged; nothing in this run was converted\n\
         no files were converted: the check phase refused the batch and nothing was written\n\
         files not converted (all left as they were):\n\
         \x20 a.i\n"
    );
    assert_eq!(bytes(dir.path(), "a.i"), source.as_bytes());
    assert_missing(dir.path(), "a.is");
}

#[test]
fn trailing_and_eof_comments_are_refused_is_to_i() {
    let dir = package();
    let source =
        "// kept note\n(= f (-> (,) _ 1))  // trailing\n(= g (-> (,) _ 2))\n// last note\n";
    write_file(dir.path(), "a.is", source);

    let out = i(dir.path(), &["--to", "i", "a.is"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr_of(&out));
    assert_eq!(out.stdout, b"");
    assert_eq!(
        stderr_of(&out),
        "a.is:2:21: error: converting to `i` would lose this text: \"// trailing\"\n\
         a.is:4:1: error: converting to `i` would lose this text: \"// last note\"\n\
         a.is: the source is unchanged; nothing in this run was converted\n\
         no files were converted: the check phase refused the batch and nothing was written\n\
         files not converted (all left as they were):\n\
         \x20 a.is\n"
    );
    assert_eq!(bytes(dir.path(), "a.is"), source.as_bytes());
    assert_missing(dir.path(), "a.i");
}

/// A `//` inside a string literal is not a comment. Comments that do have a
/// place in the other surface (leading, nested, documentation) are carried, so
/// this conversion must succeed in both directions.
#[test]
fn comments_and_comment_like_strings_convert_i_to_is() {
    let dir = package();
    let source = "// leading note\n\
                  /// docs for f\n\
                  //// module docs\n\
                  f = () {\n\
                  \t// inner note\n\
                  \t\"text with // and /* and */ inside\"\n\
                  }\n\
                  \n\
                  g = () { \"quote \\\" and backslash \\\\ and //\" }\n";
    write_file(dir.path(), "a.i", source);

    let out = i(dir.path(), &["--to", "is", "a.i"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
    assert_missing(dir.path(), "a.i");
    assert!(dir.path().join("a.is").exists());

    let converted = text(dir.path(), "a.is");
    for kept in [
        "// leading note",
        "/// docs for f",
        "//// module docs",
        "// inner note",
        "text with // and /* and */ inside",
        "and //",
    ] {
        assert!(
            converted.contains(kept),
            "missing {kept:?} in:\n{converted}"
        );
    }
    assert_parsed_equal(&module_of_i(source), &module_of_lisp(&converted));

    // And back: the original program is recovered.
    let out = i(dir.path(), &["--to", "i", "a.is"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
    assert_missing(dir.path(), "a.is");
    assert_parsed_equal(&module_of_i(source), &module_of_i(&text(dir.path(), "a.i")));
}

#[test]
fn comments_and_comment_like_strings_convert_is_to_i() {
    let dir = package();
    let source = "// leading note\n\
                  /// docs for f\n\
                  (= f\n\
                  \x20 (-> (,) _\n\
                  \x20   {\n\
                  \x20     // inner note\n\
                  \x20     \"text with // and /* and */ inside\"\n\
                  \x20   }))\n";
    write_file(dir.path(), "a.is", source);

    let out = i(dir.path(), &["--to", "i", "a.is"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
    assert_missing(dir.path(), "a.is");
    assert!(dir.path().join("a.i").exists());

    let converted = text(dir.path(), "a.i");
    for kept in [
        "// leading note",
        "/// docs for f",
        "// inner note",
        "text with // and /* and */ inside",
    ] {
        assert!(
            converted.contains(kept),
            "missing {kept:?} in:\n{converted}"
        );
    }
    assert_parsed_equal(&module_of_lisp(source), &module_of_i(&converted));

    let out = i(dir.path(), &["--to", "is", "a.i"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
    assert_parsed_equal(
        &module_of_lisp(source),
        &module_of_lisp(&text(dir.path(), "a.is")),
    );
}

#[test]
fn doc_comments_survive_both_directions() {
    let dir = package();
    let source = "//// module documentation\n\
                  /// documentation of f\n\
                  f = () { 1 }\n\
                  \n\
                  /// documentation of g\n\
                  g = () { 2 }\n";
    write_file(dir.path(), "a.i", source);

    let out = i(dir.path(), &["--to", "is", "a.i"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
    let converted = text(dir.path(), "a.is");
    assert_parsed_equal(&module_of_i(source), &module_of_lisp(&converted));
    assert!(converted.contains("/// documentation of f\n"));
    assert!(converted.contains("/// documentation of g\n"));

    let out = i(dir.path(), &["--to", "i", "a.is"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
    let back = text(dir.path(), "a.i");
    assert_parsed_equal(&module_of_i(source), &module_of_i(&back));
    assert!(back.contains("//// module documentation"));
    assert!(back.contains("/// documentation of f"));
    assert!(back.contains("/// documentation of g"));
}

// ---- lossless beyond comments ---------------------------------------------

/// Literal spelling may be canonicalised between surfaces (spec §1: a
/// conversion may change literal spelling but not the parsed program). So a
/// base prefix, digit separators and a trailing dot are allowed to change; the
/// parsed values are compared instead. Blank lines are layout and may change
/// too. Attributes and macro calls keep their meaning, which `Module` equality
/// covers.
#[test]
fn literals_attributes_macros_and_blank_lines_convert_losslessly() {
    let dir = package();
    let source = "@external(\"env\", \"puts\")\n\
                  puts : (String) -> Int\n\
                  \n\
                  @js(\"console.log\")\n\
                  log : (String) -> ()\n\
                  \n\
                  @m(1, \"two\")\n\
                  item = () { 3 }\n\
                  \n\
                  numbers = (1_000_000, 0xFF_FF, 0o7_7, 0b1010_0101, 1_0.5e-1_0, 3.)\n\
                  \n\
                  text = \"quote \\\" and backslash \\\\ and // not a comment\"\n";
    write_file(dir.path(), "a.i", source);

    let out = i(dir.path(), &["--to", "is", "a.i"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
    assert_missing(dir.path(), "a.i");
    let converted = text(dir.path(), "a.is");
    assert_parsed_equal(&module_of_i(source), &module_of_lisp(&converted));

    let out = i(dir.path(), &["--to", "i", "a.is"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
    let back = text(dir.path(), "a.i");
    assert_parsed_equal(&module_of_i(source), &module_of_i(&back));
    // The values survive even though the spelling was canonicalised.
    assert!(back.contains("@external(\"env\", \"puts\")"));
    assert!(back.contains("@js(\"console.log\")"));
    assert!(back.contains("@m(1, \"two\")"));
    assert!(back.contains("quote \\\" and backslash \\\\ and // not a comment"));
}

/// A Lisp-only quotation has no normal-surface spelling. This is not a lost
/// comment but a module the target surface cannot represent, and the refusal
/// must still happen before any file in the batch is written.
#[test]
fn unrepresentable_lisp_module_is_refused_before_any_write() {
    let dir = package();
    write_file(dir.path(), "good.is", "(= f (-> (,) _ 1))\n");
    write_file(dir.path(), "quote.is", "(= q `(f 1))\n");

    let out = i(dir.path(), &["--to", "i", "good.is", "quote.is"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr_of(&out));
    assert_eq!(out.stdout, b"");
    assert_eq!(
        stderr_of(&out),
        "quote.is:1:1: error[E0101]: the module cannot be represented in the other surface\n\
         no files were converted: the check phase refused the batch and nothing was written\n\
         files not converted (all left as they were):\n\
         \x20 good.is\n\
         \x20 quote.is\n"
    );
    assert_eq!(bytes(dir.path(), "good.is"), b"(= f (-> (,) _ 1))\n");
    assert_eq!(bytes(dir.path(), "quote.is"), b"(= q `(f 1))\n");
    assert_missing(dir.path(), "good.i");
    assert_missing(dir.path(), "quote.i");
}

// ---- b) an existing target is never overwritten ---------------------------

#[test]
fn existing_destination_is_not_overwritten_is_to_i() {
    let dir = package();
    write_file(dir.path(), "a.is", "(= f (-> (,) _ 1))\n");
    let existing = "f = () { 42 }\n";
    write_file(dir.path(), "a.i", existing);

    let out = i(dir.path(), &["--to", "i", "a.is"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr_of(&out));
    assert_eq!(out.stdout, b"");
    assert_eq!(
        stderr_of(&out),
        "a.is: error: a.i already exists; it will not be overwritten\n\
         no files were converted: the check phase refused the batch and nothing was written\n\
         files not converted (all left as they were):\n\
         \x20 a.is\n"
    );
    assert_eq!(bytes(dir.path(), "a.is"), b"(= f (-> (,) _ 1))\n");
    assert_eq!(bytes(dir.path(), "a.i"), existing.as_bytes());
}

#[test]
fn existing_untracked_destination_is_not_overwritten() {
    let dir = package();
    write_file(dir.path(), "a.i", "greeting = () { \"hi\" }\n");
    let existing = "(= placeholder (-> (,) _ 42))\n";
    write_file(dir.path(), "a.is", existing);

    let out = i(dir.path(), &["--to", "is", "a.i"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr_of(&out));
    assert_eq!(out.stdout, b"");
    assert_eq!(
        stderr_of(&out),
        "a.i: error: a.is already exists; it will not be overwritten\n\
         no files were converted: the check phase refused the batch and nothing was written\n\
         files not converted (all left as they were):\n\
         \x20 a.i\n"
    );
    assert_eq!(bytes(dir.path(), "a.i"), b"greeting = () { \"hi\" }\n");
    assert_eq!(bytes(dir.path(), "a.is"), existing.as_bytes());
}

#[test]
fn existing_tracked_destination_is_not_overwritten() {
    let dir = package();
    write_file(dir.path(), "a.i", "greeting = () { \"hi\" }\n");
    let existing = "(= placeholder (-> (,) _ 42))\n";
    write_file(dir.path(), "a.is", existing);
    for command in [
        vec!["init", "-q", "-b", "main"],
        vec!["config", "user.email", "t@example.com"],
        vec!["config", "user.name", "t"],
        vec!["add", "a.i", "a.is"],
        vec!["commit", "-q", "-m", "before"],
    ] {
        let status = Command::new("git")
            .current_dir(dir.path())
            .args(&command)
            .status()
            .unwrap();
        assert!(status.success(), "git {command:?} failed");
    }

    let out = i(dir.path(), &["--to", "is", "a.i"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr_of(&out));
    assert_eq!(
        stderr_of(&out),
        "a.i: error: a.is already exists; it will not be overwritten\n\
         no files were converted: the check phase refused the batch and nothing was written\n\
         files not converted (all left as they were):\n\
         \x20 a.i\n"
    );
    assert_eq!(bytes(dir.path(), "a.i"), b"greeting = () { \"hi\" }\n");
    assert_eq!(bytes(dir.path(), "a.is"), existing.as_bytes());
    // A tracked target: the refusal leaves the worktree clean.
    let status = Command::new("git")
        .current_dir(dir.path())
        .args(["status", "--porcelain"])
        .output()
        .unwrap();
    assert_eq!(status.stdout, b"", "the worktree should be clean");
}

// ---- c) a mixed batch is refused as a whole -------------------------------

#[test]
fn mixed_batch_with_one_lossy_file_is_refused_entirely() {
    let dir = package();
    write_file(dir.path(), "a.i", "a = () { 1 }\n");
    write_file(dir.path(), "b.i", "b = () { 1 } // note\n");
    write_file(dir.path(), "c.i", "c = () { 2 }\n");

    let out = i(dir.path(), &["--to", "is", "a.i", "b.i", "c.i"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr_of(&out));
    assert_eq!(out.stdout, b"");
    // The diagnostic names only the file that cannot be converted.
    assert_eq!(
        stderr_of(&out),
        "b.i:1:14: error: converting to `is` would lose this text: \"// note\"\n\
         b.i: the source is unchanged; nothing in this run was converted\n\
         no files were converted: the check phase refused the batch and nothing was written\n\
         files not converted (all left as they were):\n\
         \x20 a.i\n\
         \x20 b.i\n\
         \x20 c.i\n"
    );
    for name in ["a.i", "b.i", "c.i"] {
        assert!(dir.path().join(name).exists(), "{name} must stay");
    }
    assert_eq!(bytes(dir.path(), "a.i"), b"a = () { 1 }\n");
    assert_eq!(bytes(dir.path(), "b.i"), b"b = () { 1 } // note\n");
    assert_eq!(bytes(dir.path(), "c.i"), b"c = () { 2 }\n");
    for name in ["a.is", "b.is", "c.is"] {
        assert_missing(dir.path(), name);
    }
}

/// When the check phase refuses one file, the whole batch is refused. The
/// report must say that nothing was converted and name every file of the batch
/// that was left unconverted.
#[test]
fn preflight_refusal_reports_all_unconverted_i_to_is() {
    let dir = package();
    write_file(dir.path(), "a.i", "a = () { 1 }\n");
    write_file(dir.path(), "b.i", "b = () { 1 } // note\n");
    write_file(dir.path(), "c.i", "c = () { 2 }\n");

    let out = i(dir.path(), &["--to", "is", "a.i", "b.i", "c.i"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr_of(&out));
    assert_eq!(out.stdout, b"");
    // The report says that nothing was converted and lists every file of the
    // batch, not only the one that caused the refusal.
    assert_eq!(
        stderr_of(&out),
        "b.i:1:14: error: converting to `is` would lose this text: \"// note\"\n\
         b.i: the source is unchanged; nothing in this run was converted\n\
         no files were converted: the check phase refused the batch and nothing was written\n\
         files not converted (all left as they were):\n\
         \x20 a.i\n\
         \x20 b.i\n\
         \x20 c.i\n"
    );
    for name in ["a.i", "b.i", "c.i"] {
        assert_eq!(
            bytes(dir.path(), name),
            match name {
                "a.i" => b"a = () { 1 }\n".as_slice(),
                "b.i" => b"b = () { 1 } // note\n".as_slice(),
                _ => b"c = () { 2 }\n".as_slice(),
            }
        );
    }
    for name in ["a.is", "b.is", "c.is"] {
        assert_missing(dir.path(), name);
    }
}

/// The same preflight reporting from Lisp to normal, so both directions are
/// covered.
#[test]
fn preflight_refusal_reports_all_unconverted_is_to_i() {
    let dir = package();
    write_file(dir.path(), "a.is", "(= a (-> (,) _ 1))\n");
    write_file(dir.path(), "b.is", "(= b (-> (,) _ 1)) // note\n");
    write_file(dir.path(), "c.is", "(= c (-> (,) _ 2))\n");

    let out = i(dir.path(), &["--to", "i", "a.is", "b.is", "c.is"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr_of(&out));
    assert_eq!(
        stderr_of(&out),
        "b.is:1:20: error: converting to `i` would lose this text: \"// note\"\n\
         b.is: the source is unchanged; nothing in this run was converted\n\
         no files were converted: the check phase refused the batch and nothing was written\n\
         files not converted (all left as they were):\n\
         \x20 a.is\n\
         \x20 b.is\n\
         \x20 c.is\n"
    );
    assert_eq!(bytes(dir.path(), "a.is"), b"(= a (-> (,) _ 1))\n");
    assert_eq!(bytes(dir.path(), "b.is"), b"(= b (-> (,) _ 1)) // note\n");
    assert_eq!(bytes(dir.path(), "c.is"), b"(= c (-> (,) _ 2))\n");
    for name in ["a.i", "b.i", "c.i"] {
        assert_missing(dir.path(), name);
    }
}

// ---- d) a real write failure ---------------------------------------------

#[cfg(unix)]
fn running_as_root() -> bool {
    let out = Command::new("id").arg("-u").output().unwrap();
    String::from_utf8_lossy(&out.stdout).trim() == "0"
}

#[cfg(unix)]
struct RestorePermissions(PathBuf);

#[cfg(unix)]
impl Drop for RestorePermissions {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
    }
}

/// The first file converts; the second cannot be created in a read-only
/// directory. The converted file stays converted and is listed, and the second
/// source is untouched.
///
/// Running as root makes mode 0555 irrelevant, so the test reports why it is
/// skipped instead of passing silently.
#[cfg(unix)]
#[test]
fn read_only_destination_directory_write_failure_keeps_source_and_lists_converted() {
    use std::os::unix::fs::PermissionsExt;

    if running_as_root() {
        eprintln!(
            "SKIPPED read_only_destination_directory_write_failure_keeps_source_and_lists_converted: \
             running as root (uid 0), where a 0555 directory is still writable, so a write \
             failure cannot be induced this way"
        );
        return;
    }

    let dir = package();
    write_file(dir.path(), "aa/a.i", "a = () { 1 }\n");
    write_file(dir.path(), "ro/b.i", "b = () { 2 }\n");
    // A writable file after the failing one: it must be reported as not
    // converted and must keep its bytes.
    write_file(dir.path(), "zz/c.i", "c = () { 3 }\n");
    set_mode(&dir.path().join("ro/b.i"), 0o600);
    let ro = dir.path().join("ro");
    std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o555)).unwrap();
    let _restore = RestorePermissions(ro);

    let out = i(dir.path(), &["--to", "is", "aa/a.i", "ro/b.i", "zz/c.i"]);
    assert_ne!(out.status.code(), Some(0), "{}", stderr_of(&out));
    assert_eq!(out.stdout, b"");
    // The real I/O error, then the three groups: converted, failed, and not
    // processed. Every file of the batch appears exactly where its state says.
    assert_eq!(
        stderr_of(&out),
        "converted aa/a.i -> aa/a.is\n\
         ro/b.i: error: cannot create a temporary file in ro: Permission denied (os error 13)\n\
         already converted in this run (destination written, source removed):\n\
         \x20 aa/a.i -> aa/a.is\n\
         failed:\n\
         \x20 ro/b.i -> ro/b.is: source is unchanged, destination was not written\n\
         not processed (left as they were):\n\
         \x20 zz/c.i\n\
         recover each converted file by converting it back (`i fmt --to i DESTINATION`) or by restoring the source from version control\n\
         ro/b.i: conversion stopped; the files above were already converted\n"
    );

    // The first file was converted and its source removed.
    assert_missing(dir.path(), "aa/a.i");
    let converted = text(dir.path(), "aa/a.is");
    assert_parsed_equal(&module_of_i("a = () { 1 }\n"), &module_of_lisp(&converted));
    // The failing source and the unprocessed one are untouched, and neither
    // has a partial target.
    assert_eq!(bytes(dir.path(), "ro/b.i"), b"b = () { 2 }\n");
    assert_eq!(
        mode_of(&dir.path().join("ro/b.i")),
        0o600,
        "a write failure must not change the source mode"
    );
    assert_missing(dir.path(), "ro/b.is");
    assert_eq!(bytes(dir.path(), "zz/c.i"), b"c = () { 3 }\n");
    assert_missing(dir.path(), "zz/c.is");
    // Only temporary files this command created are cleaned up; none are left.
    for sub in ["aa", "ro", "zz"] {
        for entry in std::fs::read_dir(dir.path().join(sub)).unwrap() {
            let name = entry.unwrap().file_name();
            let name = name.to_string_lossy();
            assert!(
                !name.contains(".tmp-"),
                "a temporary file was left behind: {sub}/{name}"
            );
        }
    }
}

/// A write failure on the very first file converts nothing and formats nothing,
/// so the last line must not claim that files were converted.
#[cfg(unix)]
#[test]
fn first_file_write_failure_reports_no_file_converted() {
    use std::os::unix::fs::PermissionsExt;

    if running_as_root() {
        eprintln!(
            "SKIPPED first_file_write_failure_reports_no_file_converted: running as root \
             (uid 0), where a 0555 directory is still writable, so the write failure cannot \
             be induced this way"
        );
        return;
    }

    let dir = package();
    write_file(dir.path(), "ro/a.i", "a = () { 1 }\n");
    set_mode(&dir.path().join("ro/a.i"), 0o600);
    let ro = dir.path().join("ro");
    std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o555)).unwrap();
    let _restore = RestorePermissions(ro);

    let out = i(dir.path(), &["--to", "is", "ro/a.i"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr_of(&out));
    assert_eq!(
        stderr_of(&out),
        "ro/a.i: error: cannot create a temporary file in ro: Permission denied (os error 13)\n\
         already written in this run: none\n\
         failed:\n\
         \x20 ro/a.i -> ro/a.is: source is unchanged, destination was not written\n\
         not processed (left as they were):\n\
         \x20 none\n\
         ro/a.i: conversion stopped; no file was converted\n"
    );
    assert_eq!(bytes(dir.path(), "ro/a.i"), b"a = () { 1 }\n");
    assert_eq!(mode_of(&dir.path().join("ro/a.i")), 0o600);
    assert_missing(dir.path(), "ro/a.is");
}

/// A batch may contain a file that is already the requested surface; it is
/// formatted in place. A later write failure must leave that file complete and
/// not truncated, and must not disturb the files after it.
#[cfg(unix)]
#[test]
fn in_place_target_survives_a_later_write_failure() {
    use std::os::unix::fs::PermissionsExt;

    if running_as_root() {
        eprintln!(
            "SKIPPED in_place_target_survives_a_later_write_failure: running as root \
             (uid 0), where a 0555 directory is still writable, so the later write \
             failure cannot be induced this way"
        );
        return;
    }

    let dir = package();
    // Ugly Lisp in the target surface: this file is rewritten in place.
    write_file(dir.path(), "aa/b.is", "(= f\n(-> (,) _ 1))\n");
    write_file(dir.path(), "ro/c.i", "c = () { 2 }\n");
    let ro = dir.path().join("ro");
    std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o555)).unwrap();
    let _restore = RestorePermissions(ro);

    let out = i(dir.path(), &["--to", "is", "aa/b.is", "ro/c.i"]);
    assert_ne!(out.status.code(), Some(0), "{}", stderr_of(&out));
    assert_eq!(
        stderr_of(&out),
        "ro/c.i: error: cannot create a temporary file in ro: Permission denied (os error 13)\n\
         already converted in this run (destination written, source removed):\n\
         \x20 none\n\
         already formatted in place:\n\
         \x20 aa/b.is\n\
         failed:\n\
         \x20 ro/c.i -> ro/c.is: source is unchanged, destination was not written\n\
         not processed (left as they were):\n\
         \x20 none\n\
         ro/c.i: conversion stopped; the files above were already formatted in place\n"
    );
    // The in-place file holds exactly the formatted bytes, and its content is
    // still a valid module.
    let formatted = text(dir.path(), "aa/b.is");
    assert_eq!(formatted, "(= f (-> (,) _ 1))\n");
    assert_parsed_equal(
        &module_of_lisp("(= f\n(-> (,) _ 1))\n"),
        &module_of_lisp(&formatted),
    );
    // The later file is untouched.
    assert_eq!(bytes(dir.path(), "ro/c.i"), b"c = () { 2 }\n");
    assert_missing(dir.path(), "ro/c.is");
    // No temporary files anywhere.
    for sub in ["aa", "ro"] {
        for entry in std::fs::read_dir(dir.path().join(sub)).unwrap() {
            let name = entry.unwrap().file_name();
            let name = name.to_string_lossy();
            assert!(
                !name.contains(".tmp-"),
                "a temporary file was left behind: {sub}/{name}"
            );
        }
    }
}

// ---- a race: the target appears after the check, before the write --------

/// A real race, not a hook: a batch is converted one file at a time after the
/// check phase, so the test watches for the first target and creates the last
/// target while the run is between the two files. The run must fail without
/// overwriting the created target or removing its source, and it must list the
/// files it had already converted.
///
/// Record-only after the scope was narrowed: a race is not a blocking
/// requirement. Run it with `--ignored` to observe it.
#[ignore = "record-only race coverage; not a blocking requirement"]
#[test]
fn destination_created_during_conversion_is_not_overwritten() {
    const COUNT: usize = 2000;
    let dir = package();
    let body = "f = () { 1 }\n";
    let names: Vec<String> = (0..COUNT).map(|n| format!("src/{n:04}.i")).collect();
    for name in &names {
        write_file(dir.path(), name, body);
    }
    let last_source = format!("src/{:04}.i", COUNT - 1);
    let last_target = format!("src/{:04}.is", COUNT - 1);
    let sentinel = b"// created by the test during the run\n";

    let mut args: Vec<String> = vec!["--to".into(), "is".into()];
    args.extend(names.iter().cloned());
    let mut child = Command::new(env!("CARGO_BIN_EXE_i"))
        .current_dir(dir.path())
        .arg("fmt")
        .args(&args)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let first_target = dir.path().join("src/0000.is");
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        if first_target.exists() {
            break;
        }
        if let Some(status) = child.try_wait().unwrap() {
            panic!(
                "the race could not be induced: the run finished ({status}) before the first \
                 target appeared; this test is invalid and must not be counted as a pass"
            );
        }
        assert!(
            Instant::now() < deadline,
            "the race could not be induced within 120 s"
        );
        std::thread::sleep(Duration::from_micros(200));
    }
    let created = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.path().join(&last_target));
    let mut created = match created {
        Ok(file) => file,
        Err(e) => panic!("the race was lost: cannot create {last_target}: {e}"),
    };
    created.write_all(sentinel).unwrap();
    drop(created);

    let out = child.wait_with_output().unwrap();
    assert_ne!(out.status.code(), Some(0), "{}", stderr_of(&out));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains(&format!(
            "{last_target}: error: {last_target} already exists; it will not be overwritten"
        )) || err.contains(&format!(
            "{last_source}: error: {last_target} already exists; it will not be overwritten"
        )),
        "{err}"
    );
    assert!(err.contains("already converted in this run:"), "{err}");
    assert!(err.contains("0000.i -> 0000.is"), "{err}");

    // The target the test created is byte for byte what the test wrote.
    assert_eq!(bytes(dir.path(), &last_target), &sentinel[..]);
    // Its source was kept and is intact.
    assert_eq!(bytes(dir.path(), &last_source), body.as_bytes());
    // The first file was converted.
    assert_missing(dir.path(), "src/0000.i");
    assert!(dir.path().join("src/0000.is").exists());
    // No file disappeared without a replacement in either surface.
    for name in &names {
        let stem = name.trim_end_matches(".i");
        assert!(
            dir.path().join(name).exists() || dir.path().join(format!("{stem}.is")).exists(),
            "{name} was lost without producing its target"
        );
    }
    // Only temporary files this command created are removed; the file another
    // process placed at a target path is not touched.
    for entry in std::fs::read_dir(dir.path().join("src")).unwrap() {
        let name = entry.unwrap().file_name();
        let name = name.to_string_lossy();
        assert!(
            !name.contains(".tmp-"),
            "a temporary file was left behind: src/{name}"
        );
    }
    assert_eq!(bytes(dir.path(), &last_target), &sentinel[..]);
}

// ---- e) the successful path, both directions -----------------------------

#[test]
fn no_comments_round_trips_i_to_is_and_back() {
    let dir = package();
    let source = "gcd = (a : Int, b : Int) -> Int {\n\
                  \ta\n\
                  }\n\
                  \n\
                  greeting = () { \"hello\" }\n";
    write_file(dir.path(), "a.i", source);

    let out = i(dir.path(), &["--to", "is", "a.i"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
    assert_eq!(out.stdout, b"");
    assert_missing(dir.path(), "a.i");
    let converted = text(dir.path(), "a.is");
    assert_parsed_equal(&module_of_i(source), &module_of_lisp(&converted));

    let out = i(dir.path(), &["--to", "i", "a.is"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
    assert_missing(dir.path(), "a.is");
    assert_parsed_equal(&module_of_i(source), &module_of_i(&text(dir.path(), "a.i")));
}

#[test]
fn no_comments_round_trips_is_to_i_and_back() {
    let dir = package();
    let source = "(= gcd\n  (-> (, (= a Int) (= b Int)) Int\n    { a }))\n\n(= greeting (-> (,) _ \"hello\"))\n";
    write_file(dir.path(), "a.is", source);

    let out = i(dir.path(), &["--to", "i", "a.is"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
    assert_missing(dir.path(), "a.is");
    let converted = text(dir.path(), "a.i");
    assert_parsed_equal(&module_of_lisp(source), &module_of_i(&converted));

    let out = i(dir.path(), &["--to", "is", "a.i"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
    assert_missing(dir.path(), "a.i");
    assert_parsed_equal(
        &module_of_lisp(source),
        &module_of_lisp(&text(dir.path(), "a.is")),
    );
}

// ---- permissions: a conversion keeps the source file's mode --------------

/// Run the binary under `umask 022` through a shell, so the mode of a newly
/// created file is predictable.
#[cfg(unix)]
fn i_with_umask(dir: &Path, args: &[&str]) -> Output {
    let binary = env!("CARGO_BIN_EXE_i");
    Command::new("sh")
        .current_dir(dir)
        .arg("-c")
        .arg("umask 022; exec \"$0\" \"$@\"")
        .arg(binary)
        .args(args)
        .output()
        .unwrap()
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

#[cfg(unix)]
fn mode_of(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[cfg(unix)]
fn skip_permissions_as_root(test: &str) -> bool {
    if running_as_root() {
        eprintln!(
            "SKIPPED {test}: running as root (uid 0); the permission tests are meant to run \
             unprivileged, as the orchestrator requires"
        );
        return true;
    }
    false
}

#[cfg(unix)]
#[test]
fn permissions_kept_i_to_is() {
    if skip_permissions_as_root("permissions_kept_i_to_is") {
        return;
    }
    let dir = package();
    write_file(dir.path(), "a.i", "f = () { 1 }\n");
    set_mode(&dir.path().join("a.i"), 0o600);

    let out = i_with_umask(dir.path(), &["fmt", "--to", "is", "a.i"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
    assert_missing(dir.path(), "a.i");
    assert_eq!(
        mode_of(&dir.path().join("a.is")),
        0o600,
        "the target must keep the source mode"
    );
}

#[cfg(unix)]
#[test]
fn permissions_kept_is_to_i() {
    if skip_permissions_as_root("permissions_kept_is_to_i") {
        return;
    }
    let dir = package();
    write_file(dir.path(), "a.is", "(= f (-> (,) _ 1))\n");
    set_mode(&dir.path().join("a.is"), 0o600);

    let out = i_with_umask(dir.path(), &["fmt", "--to", "i", "a.is"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
    assert_missing(dir.path(), "a.is");
    assert_eq!(
        mode_of(&dir.path().join("a.i")),
        0o600,
        "the target must keep the source mode"
    );
}

/// A file that is already the target surface is rewritten in place; its mode
/// must not change either.
#[cfg(unix)]
#[test]
fn permissions_kept_when_rewriting_in_place() {
    if skip_permissions_as_root("permissions_kept_when_rewriting_in_place") {
        return;
    }
    let dir = package();
    write_file(dir.path(), "a.is", "(= f\n(-> (,) _ 1))\n");
    set_mode(&dir.path().join("a.is"), 0o600);

    let out = i_with_umask(dir.path(), &["fmt", "--to", "is", "a.is"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
    assert_eq!(
        mode_of(&dir.path().join("a.is")),
        0o600,
        "an in-place rewrite must keep the mode"
    );
}

/// A refusal in the check phase changes neither bytes nor mode.
#[cfg(unix)]
#[test]
fn permissions_kept_on_preflight_refusal() {
    if skip_permissions_as_root("permissions_kept_on_preflight_refusal") {
        return;
    }
    let dir = package();
    write_file(dir.path(), "a.i", "a = () { 1 }\n");
    write_file(dir.path(), "b.i", "b = () { 1 } // note\n");
    set_mode(&dir.path().join("a.i"), 0o600);
    set_mode(&dir.path().join("b.i"), 0o600);

    let out = i_with_umask(dir.path(), &["fmt", "--to", "is", "a.i", "b.i"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr_of(&out));
    assert_eq!(mode_of(&dir.path().join("a.i")), 0o600);
    assert_eq!(mode_of(&dir.path().join("b.i")), 0o600);
    assert_eq!(bytes(dir.path(), "a.i"), b"a = () { 1 }\n");
    assert_eq!(bytes(dir.path(), "b.i"), b"b = () { 1 } // note\n");
}

/// The implementation has no hook to observe its temporary file. This test
/// watches the directory while a large conversion runs and, if it catches the
/// temporary file, requires it to be no wider than the source. It is
/// record-only: if the temporary file is never observed, the coverage is a
/// gap and the test says so instead of pretending to pass.
#[cfg(unix)]
#[ignore = "record-only: observes the temporary file by polling; no hook exists"]
#[test]
fn temporary_file_is_not_wider_than_source() {
    if skip_permissions_as_root("temporary_file_is_not_wider_than_source") {
        return;
    }
    let dir = package();
    // A large module keeps the temporary file on disk long enough to observe.
    let mut source = String::new();
    for n in 0..4000 {
        source.push_str(&format!("f{n} = () {{ {n} }}\n"));
    }
    write_file(dir.path(), "a.i", &source);
    set_mode(&dir.path().join("a.i"), 0o600);

    let mut child = Command::new(env!("CARGO_BIN_EXE_i"))
        .current_dir(dir.path())
        .args(["fmt", "--to", "is", "a.i"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let mut observed: Option<u32> = None;
    let deadline = Instant::now() + Duration::from_secs(120);
    while Instant::now() < deadline {
        if let Ok(entries) = std::fs::read_dir(dir.path()) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if name.to_string_lossy().contains(".tmp-")
                    && let Ok(meta) = entry.metadata()
                {
                    use std::os::unix::fs::PermissionsExt;
                    observed = Some(meta.permissions().mode() & 0o777);
                }
            }
        }
        if observed.is_some() {
            break;
        }
        if let Some(_status) = child.try_wait().unwrap() {
            break;
        }
        std::thread::sleep(Duration::from_micros(200));
    }
    let _ = child.wait();

    match observed {
        Some(mode) => assert_eq!(
            mode & !0o600,
            0,
            "the temporary file mode {mode:o} is wider than the 0600 source"
        ),
        None => eprintln!(
            "GAP temporary_file_is_not_wider_than_source: the temporary file was not observed \
             during the run, so the creation mode could not be checked"
        ),
    }
}

// ---- f) ordinary `i fmt` and `fmt --check` are unchanged -----------------

#[test]
fn plain_fmt_and_check_keep_their_output_bytes() {
    const UGLY_I: &str = "main = () {\n  x = 1\n  x\n}\n";
    const PRETTY_I: &str = "main = () {\n\tx = 1\n\tx\n}\n";
    const UGLY_LISP: &str = "(= f\n(-> (,) _ 1))\n";
    const PRETTY_LISP: &str = "(= f (-> (,) _ 1))\n";

    let dir = package();
    write_file(dir.path(), "src/a.i", UGLY_I);
    write_file(dir.path(), "src/b.is", UGLY_LISP);
    let project = dir.path().to_str().unwrap();

    let out = i(dir.path(), &["--check", "--project", project]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr_of(&out));
    assert_eq!(out.stderr, b"");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("{project}/src/a.i\n{project}/src/b.is\n")
    );
    assert_eq!(bytes(dir.path(), "src/a.i"), UGLY_I.as_bytes());
    assert_eq!(bytes(dir.path(), "src/b.is"), UGLY_LISP.as_bytes());

    let out = i(dir.path(), &["--project", project]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
    assert_eq!(out.stdout, b"");
    assert_eq!(out.stderr, b"");
    assert_eq!(bytes(dir.path(), "src/a.i"), PRETTY_I.as_bytes());
    assert_eq!(bytes(dir.path(), "src/b.is"), PRETTY_LISP.as_bytes());

    let out = i(dir.path(), &["--check", "--project", project]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
    assert_eq!(out.stdout, b"");
    assert_eq!(out.stderr, b"");
}
