use std::process::{Command, Output};

fn i(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_i"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn raw_and_piped_output_match_the_standard() {
    let expected = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/spec.md"));
    for args in [&["spec"][..], &["spec", "--raw"][..]] {
        let out = i(args);
        assert!(out.status.success());
        assert_eq!(out.stdout, expected);
    }
}

#[test]
fn section_includes_subsections_and_stops_at_the_next_peer() {
    let out = i(&["spec", "4"]);
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.starts_with("## 4."));
    assert!(text.contains("### 4.4 "));
    assert!(!text.contains("## 5."));
    let nested = String::from_utf8(i(&["spec", "2.4"]).stdout).unwrap();
    assert!(nested.contains("2.4.1 Unused top-level bindings"));
    assert_eq!(
        i(&["spec", "4.4"]).stdout,
        i(&["spec", "--section", "4.4"]).stdout
    );
}

#[test]
fn search_toc_version_and_help() {
    let search = i(&["spec", "--search", "source", "UTF-8"]);
    assert!(search.status.success());
    let text = String::from_utf8(search.stdout).unwrap();
    assert!(text.contains("1.1 Source files"));
    assert!(text.contains("UTF-8"));
    let toc = String::from_utf8(i(&["spec", "--toc"]).stdout).unwrap();
    assert!(toc.contains("4.4 "));
    assert!(toc.contains("Literal types"));
    assert!(toc.contains("B.7.1 "));
    let version = String::from_utf8(i(&["spec", "--version"]).stdout).unwrap();
    assert!(version.contains("Version: 1.0; Date: 2026-11-14"));
    assert!(
        String::from_utf8(i(&["--help"]).stdout)
            .unwrap()
            .contains("spec")
    );
}

#[test]
fn unknown_section_reports_nearest_headings() {
    let out = i(&["spec", "4.999"]);
    assert!(!out.status.success());
    let error = String::from_utf8(out.stderr).unwrap();
    assert!(error.contains("unknown section"));
    assert!(error.contains("nearest headings"));
    assert!(error.contains("4."));
}
