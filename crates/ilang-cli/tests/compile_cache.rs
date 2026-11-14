//! The cross-process cache of compiled machine code, kept in the global cache
//! store (`I_CACHE_DIR`): a warm run gives the same output as a cold one and as
//! a run without the cache, and a changed source byte is never served from the
//! cache. `I_CACHE=off` and the older `ILANG_COMPILE_CACHE=0` turn it off.

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
        .env_remove("ILANG_COMPILE_CACHE")
        .env(
            if cache == "0" {
                "I_CACHE"
            } else {
                "I_CACHE_DIR"
            },
            if cache == "0" { "off" } else { cache },
        )
        .env("RAYON_NUM_THREADS", "1")
        .output()
        .unwrap()
}

/// The entries of one kind in the store at `cache`.
fn entries(cache: &Path, kind: &str) -> usize {
    let Ok(shards) = std::fs::read_dir(cache.join("v1").join(kind)) else {
        return 0;
    };
    shards
        .map(|shard| {
            std::fs::read_dir(shard.unwrap().path())
                .unwrap()
                .filter(|e| {
                    let e = e.as_ref().unwrap();
                    e.file_type().unwrap().is_file()
                        && !e.file_name().to_string_lossy().starts_with('.')
                })
                .count()
        })
        .sum()
}

/// The compiled modules in the cache.
fn modules(cache: &Path) -> usize {
    entries(cache, "wasmtime-module")
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
    assert!(entries(&cache, "cranelift-function") > 0);
    assert!(cache.join("CACHEDIR.TAG").is_file());

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
fn cache_command_reports_collects_and_cleans() {
    let root = tempfile::tempdir().unwrap();
    let project = project(root.path());
    let cache = root.path().join("cache");
    let cache_text = cache.to_str().unwrap();
    assert!(
        i(&project, cache_text, &["run", "--opt", "0"])
            .status
            .success()
    );
    assert!(modules(&cache) > 0);
    let cache_cmd = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_i"))
            .arg("cache")
            .args(args)
            .env_remove("ILANG_COMPILE_CACHE")
            .env("I_CACHE_DIR", &cache)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    let status = cache_cmd(&["status"]);
    assert!(status.contains("wasmtime-module: "), "{status}");
    assert!(status.contains(cache_text), "{status}");
    // Every entry was used within the hour: a collection keeps them all.
    let gc = cache_cmd(&["gc", "--max-size", "1"]);
    assert!(gc.starts_with("removed 0 entries"), "{gc}");
    assert!(cache.join("gc.stamp").is_file());
    cache_cmd(&["clean", "--tests"]);
    assert_eq!(modules(&cache), 0);
    assert_eq!(entries(&cache, "cranelift-function"), 0);
    let status = cache_cmd(&["status"]);
    assert!(status.contains("total : 0 entries"), "{status}");
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
    let cache = root.path().join("cache");
    let out = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["run", "--opt", "0", "--offline", "--project"])
        .arg(&project)
        .env_remove("ILANG_COMPILE_CACHE")
        .env("I_CACHE", "off")
        .env("I_CACHE_DIR", &cache)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(!cache.exists(), "I_CACHE=off writes nothing");
}
