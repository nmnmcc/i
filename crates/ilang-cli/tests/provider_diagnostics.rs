//! Removing automatic entry plans must preserve concrete diagnostics at
//! every use of an explicitly supplied service, including cached handlers.

#[test]
fn named_provider_diagnostics_match_all_fourteen_landed_projects() {
    // Every use of a supplied service must be checked, including cached handlers.
    // Use each real manifest: mixed_install_12 and mixed_install_6 import
    // std/test and must expand their macros before comparing diagnostics.
    // The b1 whole-method meet is inhabited; retain all four concrete
    // provider failures. m32 additionally rejects its duplicate key.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/review/cases");
    // Formatting shifts source coordinates. A separate coordinate map keeps
    // the diagnostic comparison complete, including every resulting position.
    let locations: serde_json::Value =
        serde_json::from_str(include_str!("providers_format_locations.json")).unwrap();
    for (case, count, provider_count) in [
        ("review-k4d/_loose/fuzz/e0505/21_4", 3, 2),
        ("review-k4d/_loose/p/b1", 5, 4),
        ("review-k4d/_loose/p/r6", 4, 3),
        ("review-k4d/fuzz/m0", 2, 2),
        ("review-k4d/fuzz/m21", 3, 2),
        ("review-k4d/fuzz/m32", 10, 8),
        ("review-k4d/fuzz/w2", 3, 2),
        ("review-k4d/fuzz/w3", 2, 2),
        ("review-k4d/fuzz/w4", 11, 4),
        ("review-k4d/proj/b1", 5, 4),
        ("review-k4d/proj/r6", 4, 3),
        ("review-k4s/hidden_conflict_install", 4, 4),
        ("review-k4s/mixed_install_12", 6, 5),
        ("review-k4s/mixed_install_6", 7, 6),
    ] {
        let dir = root.join(case);
        let observation: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("expected.json")).unwrap())
                .unwrap();
        let saved = &observation["outcome"]["check"];
        assert_eq!(saved["exit"].as_i64(), Some(1), "landed rejection: {case}");
        let mut expected: Vec<_> = saved["stderr"]
            .as_str()
            .unwrap()
            .lines()
            .filter(|line| line.contains("error["))
            .map(str::to_owned)
            .collect();
        assert_eq!(expected.len(), count, "landed diagnostic count: {case}");
        assert_eq!(
            expected
                .iter()
                .filter(|line| line.contains("error[E0515]"))
                .count(),
            provider_count,
            "landed provider diagnostic count: {case}"
        );
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_i"))
            .args(["check", "--types", "--offline", "--project"])
            .arg(&dir)
            .output()
            .unwrap();
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert_eq!(out.status.code(), Some(1), "{case}: {stderr}");
        let mut actual: Vec<_> = stderr
            .lines()
            .filter(|line| line.contains("error["))
            .map(str::to_owned)
            .collect();
        for correction in locations[case].as_array().unwrap() {
            let before = correction["before"].as_str().unwrap();
            let after = correction["after"].as_str().unwrap();
            if let Some(line) = expected.iter_mut().find(|line| line.as_str() == before) {
                *line = after.to_owned();
            } else {
                assert!(
                    expected.iter().any(|line| line == after),
                    "stale coordinate correction: {case}: {before}"
                );
            }
        }
        expected.sort_unstable();
        actual.sort_unstable();
        assert_eq!(actual, expected, "{case}");
    }
}
