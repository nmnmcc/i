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

/// The first 12 hexadecimal digits of the SHA-256 of `source`, the same
/// content key `scripts/check-doc-examples` uses. Written here so the skip
/// list names the bytes it was recorded for without another dependency.
fn content_hash(source: &str) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut message = source.as_bytes().to_vec();
    let bits = (message.len() as u64) * 8;
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bits.to_be_bytes());
    for chunk in message.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for (i, word) in chunk.as_chunks::<4>().0.iter().enumerate() {
            w[i] = u32::from_be_bytes(*word);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
    let mut hex = String::with_capacity(64);
    for word in state {
        hex.push_str(&format!("{word:08x}"));
    }
    hex.truncate(12);
    hex
}

/// Parse the recorded `parse <path> <hash>` lines. A malformed line fails the
/// test by itself: the list is written by this test, so it has one shape.
fn skip_entries(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut parts = line.splitn(3, ' ');
            let kind = parts.next().unwrap_or_default();
            let path = parts.next().unwrap_or_default();
            let hash = parts.next().unwrap_or_default();
            assert_eq!(kind, "parse", "bad skip line: {line:?}");
            assert!(
                !path.is_empty() && !hash.is_empty(),
                "bad skip line: {line:?}"
            );
            (path.to_string(), hash.to_string())
        })
        .collect()
}

/// The reasons the recorded skips no longer describe the current sources: an
/// entry whose file parses now or whose content changed since it was written,
/// and a source that fails to parse without an entry.
fn skip_problems(recorded: &str, failing: &[(String, String)]) -> Vec<String> {
    let recorded = skip_entries(recorded);
    let mut problems = Vec::new();
    for (path, hash) in &recorded {
        match failing.iter().find(|(name, _)| name == path) {
            None => problems.push(format!(
                "stale skip: {path} parses now or is gone; remove its `parse` line"
            )),
            Some((_, actual)) if actual != hash => problems.push(format!(
                "stale skip: {path} changed since it was recorded (recorded {hash}, now {actual}); \
                 remove or re-record its `parse` line"
            )),
            Some(_) => {}
        }
    }
    for (path, hash) in failing {
        if !recorded.iter().any(|(name, _)| name == path) {
            problems.push(format!(
                "missing skip: {path} does not parse (content {hash}) but has no `parse` line"
            ));
        }
    }
    problems
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
    let mut failing = Vec::new();
    for (name, src) in &sources {
        let (_, diags) = parse_module(src);
        if !diags.is_empty() {
            failing.push((name.clone(), content_hash(src)));
            continue;
        }
        checked += 1;
        if let Err(msg) = check(src) {
            eprintln!("=== {name} ===\n{msg}");
            failed.push(name.clone());
        }
    }
    let mut expected: Vec<String> = failing
        .iter()
        .map(|(name, hash)| format!("parse {name} {hash}"))
        .collect();
    expected.sort();
    let expected = expected.join("\n") + "\n";
    // review-rec/_loose/probes deliberately retains an illegal type spread
    // (a data type has no spread). Its E0107 is parsed
    // before resolution, so it belongs in the recorded invalid-source list.
    let skip_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/format_skip.txt");
    if std::env::var_os("UPDATE").is_some() {
        // A recorded failure must leave the previous list in place.
        if failed.is_empty() && checked > 250 {
            write_atomically(&skip_path, &expected).unwrap();
        } else {
            eprintln!("not updated: {}", skip_path.display());
        }
    } else {
        let recorded = fs::read_to_string(skip_path).unwrap_or_default();
        let problems = skip_problems(&recorded, &failing);
        assert!(
            problems.is_empty(),
            "format_skip.txt no longer matches the sources:\n{}",
            problems.join("\n")
        );
    }
    eprintln!("checked {checked} of {} sources", sources.len());
    assert!(checked > 250, "only {checked} sources were checked");
    assert!(failed.is_empty(), "{} failed: {failed:?}", failed.len());
}

#[test]
fn content_hash_is_sha256() {
    assert_eq!(content_hash(""), "e3b0c44298fc");
    assert_eq!(content_hash("abc"), "ba7816bf8f01");
    assert_eq!(
        content_hash("The quick brown fox jumps over the lazy dog"),
        "d7a8fbb307d7"
    );
}

/// The counterexample for content-keyed skips: a listed file that does not
/// parse stays skipped while its bytes are unchanged; adding a line changes
/// the hash, so the entry is stale and the test fails, naming the file. Temp
/// copies keep the repository's own list out of the test.
#[test]
fn a_changed_skip_goes_stale() {
    let dir = std::env::temp_dir().join(format!("ilang-format-skip-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let kept = dir.join("kept.i");
    let changed = dir.join("changed.i");
    let broken = "(";
    let broken_later = "// an added line\n(";
    assert!(!parse_module(broken).1.is_empty(), "{broken:?} parses");
    assert!(
        !parse_module(broken_later).1.is_empty(),
        "{broken_later:?} parses"
    );
    fs::write(&kept, broken).unwrap();
    fs::write(&changed, broken).unwrap();

    let recorded_path = dir.join("format_skip.txt");
    fs::write(
        &recorded_path,
        format!(
            "parse {} {}\nparse {} {}\n",
            kept.display(),
            content_hash(broken),
            changed.display(),
            content_hash(broken),
        ),
    )
    .unwrap();
    let recorded = fs::read_to_string(&recorded_path).unwrap();
    let failing = vec![
        (
            kept.display().to_string(),
            content_hash(&fs::read_to_string(&kept).unwrap()),
        ),
        (
            changed.display().to_string(),
            content_hash(&fs::read_to_string(&changed).unwrap()),
        ),
    ];
    let problems = skip_problems(&recorded, &failing);
    assert!(problems.is_empty(), "{problems:?}");

    // The same path with one more line: the entry is stale, and only that one.
    fs::write(&changed, broken_later).unwrap();
    let failing = vec![
        (
            kept.display().to_string(),
            content_hash(&fs::read_to_string(&kept).unwrap()),
        ),
        (
            changed.display().to_string(),
            content_hash(&fs::read_to_string(&changed).unwrap()),
        ),
    ];
    let problems = skip_problems(&recorded, &failing);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].contains(&changed.display().to_string()),
        "{problems:?}"
    );
    assert!(
        problems[0].contains("changed since it was recorded"),
        "{problems:?}"
    );
    assert!(
        !problems[0].contains(&kept.display().to_string()),
        "{problems:?}"
    );
    fs::remove_dir_all(&dir).unwrap();
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
