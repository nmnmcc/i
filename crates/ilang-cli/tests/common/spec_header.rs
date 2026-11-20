//! Header regeneration preserves source after the header byte for byte.

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
/// directory, then rename it over the file. A crash or kill leaves either the
/// old bytes or all of the new ones, never a part of them.
pub fn write_atomically(path: &Path, content: &str) -> std::io::Result<()> {
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

/// Publish the regenerated headers after the whole run succeeded. When it
/// did not, leave every header untouched and name it on standard error.
pub fn commit(updates: &[(PathBuf, String)], ok: bool) -> std::io::Result<()> {
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

/// The counterexample for [`commit`]: with `UPDATE=1`, a failed run leaves a
/// header byte for byte and reports it on standard error; a successful run
/// replaces it completely. `TEST` is the calling test's name: the child
/// process runs that test alone, so `UPDATE` cannot reach another test.
pub fn counterexample(test: &str) {
    if let Some(path) = std::env::var_os("ILANG_GOLDEN_CHILD") {
        // SAFETY: this test is the only one the child runs (`--exact`).
        unsafe { std::env::set_var("UPDATE", "1") };
        commit(
            &[(PathBuf::from(path), "// expect: new\n".to_string())],
            false,
        )
        .unwrap();
        return;
    }
    let dir =
        std::env::temp_dir().join(format!("ilang-spec-golden-{}-{}", std::process::id(), test));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("case.i");
    fs::write(&path, b"// expect: old\n").unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture"])
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
        b"// expect: old\n",
        "a failed run changed {path:?}"
    );
    commit(&[(path.clone(), "// expect: new\n".to_string())], true).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"// expect: new\n");
    fs::remove_dir_all(&dir).unwrap();
}

pub fn rewrite(source: &str, comment: &str, first: &str, at: &[String]) -> Result<String, String> {
    for value in std::iter::once(first).chain(at.iter().map(String::as_str)) {
        if value.trim() != value || value.contains(['\r', '\n']) {
            return Err(
                "a regenerated header value has leading/trailing whitespace or a line break".into(),
            );
        }
    }
    let mut out = String::new();
    let mut rest = source;
    let mut position = 0;
    let mut stop = false;
    while !rest.is_empty() {
        let end = rest.find('\n').map_or(rest.len(), |i| i + 1);
        let line = &rest[..end];
        let (text, ending) = if let Some(s) = line.strip_suffix("\r\n") {
            (s, "\r\n")
        } else if let Some(s) = line.strip_suffix('\n') {
            (s, "\n")
        } else {
            (line, "")
        };
        let header = match text.strip_prefix(comment) {
            Some(s) => s.trim(),
            None if text.trim().is_empty() => "",
            None => {
                out.push_str(rest);
                break;
            }
        };
        if header.starts_with("expect: stop ") {
            if stop {
                return Err("the header has more than one stop expectation".into());
            }
            stop = true;
            out.push_str(&format!("{comment} expect: stop \"{first}\"{ending}"));
        } else if header.starts_with("at:") {
            let value = at
                .get(position)
                .ok_or("the number of stop positions changed")?;
            out.push_str(&format!("{comment} at: {value}{ending}"));
            position += 1;
        } else {
            out.push_str(line);
        }
        rest = &rest[end..];
    }
    if !stop || position != at.len() {
        return Err("the stop header or its number of positions changed".into());
    }
    Ok(out)
}
