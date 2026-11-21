//! `i test --workers` chooses the worker count of the test instances:
//! `auto` is the product default (`i run`'s count), and the flag defaults
//! to one worker so a plain `i test` stays deterministic.
#[path = "common/test_project.rs"]
mod test_project;
use std::path::PathBuf;
use std::process::{Command, Output};

const SOURCE: &str = "\
(
\ttestlib = (test) = i/test
\ti/task
)

test(\"spawn\", () {
\ttask.scope((s) {
\t\tt = task.go(s, () { 1 })
\t\ttask.join(t) |> (1) { () } & (_) { panic(\"no\") }
\t})
})
";

/// A package whose one test starts a child task and joins it: a known-good
/// two-worker program, not one of the scheduler paths that still hang.
fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("test")).unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::write(dir.path().join("test/lib_test.i"), SOURCE).unwrap();
    test_project::configure(dir.path());
    dir
}

/// Runs `i test --opt 0` with `I_WORKERS` set and tracing on.
fn run(project: &PathBuf, args: &[&str], env_workers: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["test", "--opt", "0"])
        .args(args)
        .arg("--project")
        .arg(project)
        .env("I_WORKERS", env_workers)
        .env("I_WORKERS_TRACE", "1")
        .env("I_CACHE", "off")
        .output()
        .unwrap()
}

fn passed(out: &Output) {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success() && stdout.contains("1 passed, 0 failed"),
        "{stdout}{stderr}"
    );
}

#[test]
fn workers_two_runs_the_test_on_two_workers() {
    let dir = project();
    // The flag wins over the environment, so the count is the flag's.
    let out = run(&dir.path().to_path_buf(), &["--workers", "2"], "1");
    passed(&out);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("i: workers 2 "), "{stderr}");
}

#[test]
fn the_default_is_one_worker_whatever_the_environment_says() {
    let dir = project();
    let out = run(&dir.path().to_path_buf(), &[], "2");
    passed(&out);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("i: workers "), "{stderr}");
}

#[test]
fn workers_auto_is_the_product_default() {
    let dir = project();
    let out = run(&dir.path().to_path_buf(), &["--workers", "auto"], "2");
    passed(&out);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("i: workers 2 "), "{stderr}");
}

#[test]
fn a_worker_count_outside_one_to_255_is_a_usage_error() {
    let dir = project();
    for value in ["0", "256", "many"] {
        let out = run(&dir.path().to_path_buf(), &["--workers", value], "1");
        assert_eq!(out.status.code(), Some(2), "--workers {value}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("--workers expects `auto` or a count from 1 to 255"),
            "{stderr}"
        );
    }
}
