//! Properties of the formatter over every `.i` file of the repository that
//! parses without errors, and every language-definition example:
//!
//! - the output parses without errors, to the same tree (spans aside);
//! - formatting the output again changes nothing;
//! - every comment is kept, in the same order, with the same text, but the
//!   spaces at its end, which are taken away when it is not documentation;
//! - indentation is tabs, and no line ends in a space or a tab, but in
//!   documentation.

#[path = "oracle.rs"]
mod migration_oracle;
#[path = "common/native_shape.rs"]
mod native_shape;

use ilang_syntax::format::format;
use ilang_syntax::lex;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn write_then_rename(temporary: &Path, path: &Path, content: &str) -> std::io::Result<()> {
    let mut file = fs::File::create(temporary)?;
    file.write_all(content.as_bytes())?;
    file.flush()?;
    file.sync_all()?;
    drop(file);
    fs::rename(temporary, path)
}

/// Replace `path` with `content` through a temporary file in the same
/// directory, then rename it over the golden. A crash or kill leaves either
/// the old bytes or all of the new ones, never a part of them.
fn write_atomically(path: &Path, content: &str) -> std::io::Result<()> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    let dir = path.parent().unwrap_or(Path::new("."));
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let serial = SERIAL.fetch_add(1, Ordering::Relaxed);
    let temporary = dir.join(format!("{name}.tmp-{}-{serial}", std::process::id()));
    let result = write_then_rename(&temporary, path, content);
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// Publish every staged golden after the run succeeded. When it did not,
/// leave each golden untouched and name it on standard error.
fn commit_goldens(updates: &[(PathBuf, String)], ok: bool) -> std::io::Result<()> {
    if !ok {
        for (path, _) in updates {
            eprintln!("not updated: {}", path.display());
        }
        return Ok(());
    }
    for (path, content) in updates {
        write_atomically(path, content)?;
    }
    Ok(())
}

/// Golden updates collected while the test runs. Dropping it without calling
/// [`Staged::commit`] (a panic, for example) leaves the files alone and names
/// each one on standard error.
#[derive(Default)]
struct Staged(Vec<(PathBuf, String)>);

impl Staged {
    fn push(&mut self, path: PathBuf, content: String) {
        self.0.push((path, content));
    }

    fn commit(mut self, ok: bool) -> std::io::Result<()> {
        let updates = std::mem::take(&mut self.0);
        commit_goldens(&updates, ok)
    }
}

impl Drop for Staged {
    fn drop(&mut self) {
        for (path, _) in &self.0 {
            eprintln!("not updated: {}", path.display());
        }
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn parse_module(source: &str) -> (ilang_syntax::ast::Module, Vec<ilang_syntax::Diagnostic>) {
    let (module, errors) = ilang_syntax::parse_module(source);
    migration_oracle::check_native_validation(source, &module);
    (module, errors)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy();
        if path.is_dir() {
            if !name.starts_with('.') && !matches!(&*name, "target" | "node_modules" | "binaryen") {
                walk(&path, out);
            }
        } else if name.ends_with(".i") {
            out.push(path);
        }
    }
}

fn spec_blocks() -> Vec<(String, String)> {
    let spec = fs::read_to_string(root().join("docs/spec.md")).unwrap();
    let mut out = Vec::new();
    let mut current: Option<(usize, String)> = None;
    for (n, line) in spec.lines().enumerate() {
        match &mut current {
            None if line.trim_end() == "```i" => current = Some((n + 1, String::new())),
            Some(_) if line.trim_end() == "```" => {
                let (line, text) = current.take().unwrap();
                out.push((format!("docs/spec.md:{line}"), text));
            }
            Some((_, text)) => {
                text.push_str(line);
                text.push('\n');
            }
            None => {}
        }
    }
    out
}

fn comments(src: &str) -> Vec<String> {
    lex(src).comments.into_iter().map(|c| c.text).collect()
}

/// The text of the comments that the formatter gives: without the spaces at
/// their ends, but in documentation.
fn kept_comments(src: &str) -> Vec<String> {
    let mut out = comments(src);
    for c in &mut out {
        if !c.starts_with("///") {
            c.truncate(c.trim_end().len());
        }
    }
    out
}

/// The lines (from 0) that end in documentation.
fn doc_lines(src: &str) -> Vec<usize> {
    let lines = ilang_syntax::span::LineIndex::new(src);
    let docs = lex(src).docs;
    docs.iter()
        .map(|d| lines.line_col(d.span.start).0 as usize - 1)
        .collect()
}

/// The lines (from 0) that go on from a string literal of several lines.
fn raw_string_lines(src: &str) -> Vec<usize> {
    let lexed = lex(src);
    let lines = ilang_syntax::span::LineIndex::new(src);
    let mut out = Vec::new();
    for t in &lexed.tokens {
        if matches!(
            t.kind,
            ilang_syntax::token::TokenKind::Str(_)
                | ilang_syntax::token::TokenKind::Interpolated(_)
        ) {
            let (a, _) = lines.line_col(t.span.start);
            let (b, _) = lines.line_col(t.span.end);
            out.extend(a as usize..b as usize);
        }
    }
    out
}

/// Checks the properties for one source that parses without errors.
fn check(src: &str) -> Result<(), String> {
    let (module, _) = parse_module(src);
    let out = format(src).map_err(|d| format!("not formatted: {d:?}"))?;
    let (again, diags) = parse_module(&out);
    if !diags.is_empty() {
        return Err(format!("the output has errors: {diags:?}\n{out}"));
    }
    if !native_shape::equivalent(&module, &again) {
        return Err(format!("the output has another tree:\n{out}"));
    }
    if kept_comments(src) != comments(&out) {
        return Err(format!(
            "the comments changed: {:?} -> {:?}",
            comments(src),
            comments(&out)
        ));
    }
    let twice = format(&out).map_err(|d| format!("output not formatted: {d:?}"))?;
    if twice != out {
        return Err(format!(
            "not idempotent:\n--- once ---\n{out}--- twice ---\n{twice}"
        ));
    }
    let raw = raw_string_lines(&out);
    let docs = doc_lines(&out);
    for (n, line) in out.lines().enumerate() {
        if raw.contains(&n) {
            continue;
        }
        let ends_badly = line.ends_with([' ', '\t']) && !docs.contains(&n);
        if ends_badly || line.trim_start_matches('\t').starts_with(' ') {
            return Err(format!("bad spaces in line {}: {line:?}\n{out}", n + 1));
        }
    }
    Ok(())
}

#[test]
fn format_properties() {
    let mut files = Vec::new();
    walk(&root(), &mut files);
    files.sort();
    let mut sources: Vec<(String, String)> = files
        .iter()
        .map(|p| {
            let name = p.strip_prefix(root()).unwrap_or(p).display().to_string();
            (name, fs::read_to_string(p).unwrap())
        })
        .collect();
    sources.extend(spec_blocks());
    let mut checked = 0;
    let mut failed = Vec::new();
    let mut skipped = Vec::new();
    for (name, src) in &sources {
        let (_, diags) = parse_module(src);
        if !diags.is_empty() {
            skipped.push(format!("parse {name}"));
            continue;
        }
        checked += 1;
        if let Err(msg) = check(src) {
            eprintln!("=== {name} ===\n{msg}");
            failed.push(name.clone());
        }
    }
    skipped.sort();
    let skipped = skipped.join("\n") + "\n";
    // review-rec/_loose/probes deliberately retains an illegal type spread
    // (a data type has no spread). Its E0107 is parsed
    // before resolution, so it belongs in the recorded invalid-source list.
    let skip_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/format_skip.txt");
    if std::env::var_os("UPDATE").is_some() {
        // A recorded failure must leave the previous list in place.
        if failed.is_empty() && checked > 250 {
            write_atomically(&skip_path, &skipped).unwrap();
        } else {
            eprintln!("not updated: {}", skip_path.display());
        }
    } else {
        assert_eq!(
            fs::read_to_string(skip_path).unwrap_or_default(),
            skipped,
            "skipped sources changed"
        );
    }
    eprintln!("checked {checked} of {} sources", sources.len());
    assert!(checked > 250, "only {checked} sources were checked");
    assert!(failed.is_empty(), "{} failed: {failed:?}", failed.len());
}

/// Golden tests of layout decisions: each `tests/format/*.i` is formatted and
/// compared with the `.out` file next to it. `UPDATE=1` writes the files.
#[test]
fn format_golden() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/format");
    let update = std::env::var_os("UPDATE").is_some();
    let mut paths: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "i"))
        .collect();
    paths.sort();
    assert!(paths.len() >= 10);
    let mut failed = Vec::new();
    let mut updates = Staged::default();
    for path in &paths {
        let src = fs::read_to_string(path).unwrap();
        assert!(parse_module(&src).1.is_empty(), "{}", path.display());
        if let Err(msg) = check(&src) {
            panic!("{}: {msg}", path.display());
        }
        let actual = format(&src).unwrap();
        let out = path.with_extension("out");
        if update {
            // Keep every new file in memory; a panic below publishes none.
            updates.push(out, actual);
            continue;
        }
        let expected = fs::read_to_string(&out).unwrap_or_default();
        if expected != actual {
            eprintln!("=== {} ===\n{actual}", path.display());
            failed.push(path.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    if update {
        updates.commit(failed.is_empty()).unwrap();
    }
    assert!(
        failed.is_empty(),
        "formatted files differ: {failed:?} (run with UPDATE=1 to bless)"
    );
}

/// The counterexample for the atomic write: with `UPDATE=1`, a run that
/// reports a failure leaves the golden byte for byte and says it did.
/// The child process runs this test alone, so `UPDATE` cannot reach another
/// test that writes goldens.
#[test]
fn a_failed_update_leaves_the_golden_unchanged() {
    const TEST: &str = "a_failed_update_leaves_the_golden_unchanged";
    if let Some(path) = std::env::var_os("ILANG_GOLDEN_CHILD") {
        // SAFETY: this test is the only one the child runs (`--exact`).
        unsafe { std::env::set_var("UPDATE", "1") };
        commit_goldens(&[(PathBuf::from(path), "new\n".to_string())], false).unwrap();
        return;
    }
    let dir = std::env::temp_dir().join(format!("ilang-format-golden-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("case.out");
    fs::write(&path, b"old\n").unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", TEST, "--nocapture"])
        .env("ILANG_GOLDEN_CHILD", &path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "counterexample child failed: {output:?}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("not updated: {}", path.display())),
        "stderr was {stderr:?}"
    );
    assert_eq!(
        fs::read(&path).unwrap(),
        b"old\n",
        "a failed run changed {path:?}"
    );
    commit_goldens(&[(path.clone(), "new\n".to_string())], true).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"new\n");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn header_group_spacing() {
    let import = "(\n\ti/core\n)";
    let export = "(\n\tvalue\n)";
    for gap in ["\n", "\n\n", "\n\n\n\n"] {
        for headers in [
            import.to_owned(),
            export.to_owned(),
            format!("{import}{gap}{export}"),
        ] {
            // Replace only the gap between complete groups, not their lines.
            let spaced = if headers.starts_with(import) && headers.ends_with(export) {
                format!("{import}\n\n{export}")
            } else {
                headers.clone()
            };
            for next in [
                "value = 1\n",
                "// next\nvalue = 1\n",
                "@inline\nvalue = 1\n",
                "/// value\nvalue = 1\n",
                "// end\n",
            ] {
                let source = format!("{headers}{gap}{next}");
                check(&source).unwrap();
                assert_eq!(format(&source).unwrap(), format!("{spaced}\n\n{next}"));
            }
            let source = format!("{headers}{gap}");
            check(&source).unwrap();
            assert_eq!(format(&source).unwrap(), format!("{spaced}\n"));
        }
    }
    for source in [
        "value = 1\n",
        "// next\nvalue = 1\n",
        "@inline\nvalue = 1\n",
    ] {
        check(source).unwrap();
        assert_eq!(format(source).unwrap(), source);
    }
}
