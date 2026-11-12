//! The cross-process cache of compiled machine code (`ILANG_COMPILE_CACHE`):
//! a warm run gives the same output as a cold one and as a run without the
//! cache, and a changed source byte is never served from the cache.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            if entry.file_name() != "build" {
                copy_dir(&entry.path(), &target);
            }
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn project(root: &Path) -> PathBuf {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let project = root.join("hello");
    copy_dir(&examples.join("hello"), &project);
    let std_path = examples.join("../std").canonicalize().unwrap();
    let manifest = std::fs::read_to_string(project.join("i.toml"))
        .unwrap()
        .replace(
            "\"../../std\"",
            &format!("{:?}", std_path.display().to_string()),
        );
    std::fs::write(project.join("i.toml"), manifest).unwrap();
    project
}

fn i(project: &Path, cache: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_i"))
        .args(args)
        .args(["--offline", "--project"])
        .arg(project)
        .env("ILANG_COMPILE_CACHE", cache)
        .env("RAYON_NUM_THREADS", "1")
        .output()
        .unwrap()
}

/// The compiled modules in the cache (one file each, beside `functions/`).
fn modules(cache: &Path) -> usize {
    std::fs::read_dir(cache)
        .map(|entries| {
            entries
                .filter(|e| e.as_ref().unwrap().file_type().unwrap().is_file())
                .count()
        })
        .unwrap_or(0)
}

fn same(a: &Output, b: &Output) {
    assert_eq!(a.status.code(), b.status.code());
    assert_eq!(
        String::from_utf8_lossy(&a.stdout),
        String::from_utf8_lossy(&b.stdout)
    );
    assert_eq!(
        String::from_utf8_lossy(&a.stderr),
        String::from_utf8_lossy(&b.stderr)
    );
}

#[test]
fn warm_runs_match_cold_runs_and_changed_sources_miss() {
    let root = tempfile::tempdir().unwrap();
    let project = project(root.path());
    let cache = root.path().join("cache");
    let cache_text = cache.to_str().unwrap();
    for args in [
        &["run", "--opt", "0"][..],
        &["run", "--opt", "2"],
        &["test", "--opt", "0"],
        &["test", "--opt", "2"],
    ] {
        let uncached = i(&project, "0", args);
        assert!(
            uncached.status.success(),
            "{}",
            String::from_utf8_lossy(&uncached.stderr)
        );
        let before = modules(&cache);
        let cold = i(&project, cache_text, args);
        same(&uncached, &cold);
        let stored = modules(&cache);
        assert!(stored > before, "a cold run stores its module");
        let warm = i(&project, cache_text, args);
        same(&uncached, &warm);
        assert_eq!(
            modules(&cache),
            stored,
            "a warm run is served from the cache"
        );
    }
    assert!(cache.join("functions").read_dir().unwrap().next().is_some());

    // One changed byte in the source is a different module: never a hit.
    let main = project.join("src/main.i");
    let text = std::fs::read_to_string(&main).unwrap();
    std::fs::write(&main, text.replace("Hello from i!", "Hello from j!")).unwrap();
    let stored = modules(&cache);
    let changed = i(&project, cache_text, &["run", "--opt", "0"]);
    assert_eq!(String::from_utf8_lossy(&changed.stdout), "Hello from j!\n");
    assert_eq!(modules(&cache), stored + 1);
    same(&changed, &i(&project, "0", &["run", "--opt", "0"]));
}

#[test]
fn zero_turns_the_cache_off() {
    let root = tempfile::tempdir().unwrap();
    let project = project(root.path());
    let out = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["run", "--opt", "0", "--offline", "--project"])
        .arg(&project)
        .env("ILANG_COMPILE_CACHE", "0")
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), "Hello from i!\n");
    assert!(!root.path().join("0").exists());
}
