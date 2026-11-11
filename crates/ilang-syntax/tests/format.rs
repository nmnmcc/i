//! Properties of the formatter over every `.i` file of the repository that
//! parses without errors, and every ```i block of `docs/spec.md`:
//!
//! - the output parses without errors, to the same tree (spans aside);
//! - formatting the output again changes nothing;
//! - every comment is kept, in the same order, with the same text, but the
//!   spaces at its end, which are taken away when it is not documentation;
//! - indentation is tabs, and no line ends in a space or a tab, but in
//!   documentation.

mod common;

use ilang_syntax::format::format;
use ilang_syntax::{lex, parse_module};
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
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
        if let ilang_syntax::token::TokenKind::Str(_) = t.kind {
            let (a, _) = lines.line_col(t.span.start);
            let (b, _) = lines.line_col(t.span.end);
            out.extend(a as usize..b as usize);
        }
    }
    out
}

/// Checks the properties for one source that parses without errors.
fn check(src: &str) -> Result<(), String> {
    let (mut module, _) = parse_module(src);
    for item in &mut module.items {
        item.attributes
            .retain(|a| a.module.is_some() || a.name.name != "main");
    }
    let out = format(src).map_err(|d| format!("not formatted: {d:?}"))?;
    let (again, diags) = parse_module(&out);
    if !diags.is_empty() {
        return Err(format!("the output has errors: {diags:?}\n{out}"));
    }
    if ilang_syntax::shape::canon(&module) != ilang_syntax::shape::canon(&again) {
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
    let skip_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/format_skip.txt");
    if std::env::var_os("UPDATE").is_some() {
        fs::write(&skip_path, &skipped).unwrap();
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
    for path in &paths {
        let src = fs::read_to_string(path).unwrap();
        assert!(parse_module(&src).1.is_empty(), "{}", path.display());
        if let Err(msg) = check(&src) {
            panic!("{}: {msg}", path.display());
        }
        let actual = format(&src).unwrap();
        assert_ne!(src, actual, "{} must exercise formatting", path.display());
        let out = path.with_extension("out");
        if update {
            fs::write(&out, &actual).unwrap();
            continue;
        }
        let expected = fs::read_to_string(&out).unwrap_or_default();
        if expected != actual {
            eprintln!("=== {} ===\n{actual}", path.display());
            failed.push(path.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    assert!(
        failed.is_empty(),
        "formatted files differ: {failed:?} (run with UPDATE=1 to bless)"
    );
}

#[test]
fn removes_legacy_main_attribute_and_keeps_comments() {
    let source = "// before\n@main // entry\nmain = () { Nil }\n";
    let output = format(source).unwrap();
    assert!(!output.contains("@main"), "{output}");
    assert_eq!(kept_comments(source), comments(&output));
    assert_eq!(format(&output).unwrap(), output);
}
