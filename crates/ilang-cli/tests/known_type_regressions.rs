//! The unchanged red-team probes must fail checking before runtime.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture(chapter: &str, name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/spec")
        .join(chapter)
        .join(name)
}

fn invoke(project: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_i"))
        .args(args)
        .arg("--project")
        .arg(project)
        .output()
        .unwrap()
}

#[test]
fn hidden_inputs_cannot_skip_the_running_branch() {
    for (chapter, name, locations) in [
        ("4.8", "f1_probe_hidden_svc_sel", vec!["7:3"]),
        ("4.8", "f1_probe_hidden_cb_param", vec!["7:5"]),
        ("4.8", "f1_probe_ref_sel", vec!["8:3"]),
    ] {
        let out = invoke(&fixture(chapter, name), &["check"]);
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(!out.status.success(), "{name} was accepted");
        assert!(err.contains("error[E0501]"), "{name}: {err}");
        for location in locations {
            assert!(err.contains(location), "{name}: missing {location}: {err}");
        }
        assert!(
            err.contains("`\"wrong\"` is not a subtype of `Int`"),
            "{err}"
        );
    }
}

#[test]
fn fold_accumulator_keeps_its_inferred_record_bound() {
    let out = invoke(&fixture("4.8", "f1_accept_acc1"), &["check", "--types"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let types = String::from_utf8(out.stdout).unwrap();
    assert!(
        types.contains("probe/main.total : (List(Int)) -> (count = Int, sum = Int)"),
        "{types}"
    );
}

#[test]
fn service_parameter_can_return_the_provided_integer() {
    let project = fixture("4.8", "f1_accept_acc2");
    let out = invoke(&project, &["check"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    for opt in ["0", "2"] {
        let out = invoke(&project, &["run", "--opt", opt]);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(String::from_utf8(out.stdout).unwrap().trim(), "6");
    }
}
