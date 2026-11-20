//! A constructor is an ordinary function: a zero-payload type `True = ()`
//! gives its constructor the type `() -> True`, so a bare `True` is a
//! function value and `True()` is a `True`.

#[path = "common/test_project.rs"]
mod test_project;

use std::process::{Command, Output};

fn check(source: &str, types: bool) -> Output {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    test_project::depend_on_library(dir.path());
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.i"), source).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_i"));
    command.arg("check");
    if types {
        command.arg("--types");
    }
    command.arg("--project").arg(dir.path()).output().unwrap()
}

#[test]
fn bare_constructor_is_a_function() {
    let out = check(
        "Bool = True | False\n\
         apply = (f : () -> Bool, v : Bool) -> Bool { f() }\n\
         x = True\n\
         main = () { apply(True, x()) }\n",
        true,
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stdout: {stdout}\nstderr: {stderr}");
    assert!(stdout.contains("app/main.x : () -> True"), "{stdout}");
    assert!(stdout.contains("app/main.main : "), "{stdout}");
}

#[test]
fn bare_constructor_value_needs_a_call() {
    let out = check("x : Bool = True\nBool = True | False\n", false);
    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("error[E0501]"), "{err}");
    assert!(
        err.contains("is a constructor function; call it: `True()`"),
        "{err}"
    );
    assert!(err.contains("src/main.i:1:"), "{err}");
}

#[test]
fn constructor_call_passes() {
    let out = check("Bool = True | False\nx = True()\n", true);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stdout: {stdout}\nstderr: {stderr}");
    assert!(stdout.contains("app/main.x : True"), "{stdout}");
}
