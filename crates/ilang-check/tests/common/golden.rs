//! Atomic writes of golden files.
//!
//! A golden is replaced only after the run that made its content succeeded.
//! The new bytes go to a temporary file in the golden's own directory and are
//! renamed over it, so a crash or kill leaves either the old file or all of
//! the new one, never a part of it.

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
/// directory. The temporary name does not match a golden suffix, so test
/// discovery cannot pick up a file left behind by a crash.
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

/// Publish every staged golden after the run succeeded. When it did not,
/// leave each golden untouched and name it on standard error.
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

/// Golden updates collected while a test runs. Dropping it without calling
/// [`Staged::commit`] (a panic, for example) leaves the files alone and names
/// each one on standard error.
#[derive(Default)]
pub struct Staged(Vec<(PathBuf, String)>);

impl Staged {
    pub fn push(&mut self, path: PathBuf, content: String) {
        self.0.push((path, content));
    }

    pub fn commit(mut self, ok: bool) -> std::io::Result<()> {
        let updates = std::mem::take(&mut self.0);
        commit(&updates, ok)
    }
}

impl Drop for Staged {
    fn drop(&mut self) {
        for (path, _) in &self.0 {
            eprintln!("not updated: {}", path.display());
        }
    }
}

/// The counterexample for [`commit`]: with `UPDATE=1`, a failed run leaves a
/// golden byte for byte and reports it on standard error; a successful run
/// replaces it completely. `TEST` is the calling test's name: the child
/// process runs that test alone, so `UPDATE` cannot reach another test that
/// writes goldens.
pub fn counterexample(test: &str) {
    if let Some(path) = std::env::var_os("ILANG_GOLDEN_CHILD") {
        // SAFETY: this test is the only one the child runs (`--exact`).
        unsafe { std::env::set_var("UPDATE", "1") };
        commit(&[(PathBuf::from(path), "new\n".to_string())], false).unwrap();
        return;
    }
    let dir = std::env::temp_dir().join(format!(
        "ilang-check-golden-{}-{}",
        std::process::id(),
        test
    ));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("expected.out");
    fs::write(&path, b"old\n").unwrap();
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
        b"old\n",
        "a failed run changed {path:?}"
    );
    commit(&[(path.clone(), "new\n".to_string())], true).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"new\n");
    fs::remove_dir_all(&dir).unwrap();
}
