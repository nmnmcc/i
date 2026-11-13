//! Tests parked until a later stage of syntax three (docs/design/syntax-3.md
//! §7.2): the names listed in `tests/syn3-pending.txt` return early.

/// Whether the test `file::test` is listed in `tests/syn3-pending.txt`; it
/// then returns early, and says so on stderr.
#[allow(dead_code)]
pub fn syn3_pending(name: &str) -> bool {
    let list = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/syn3-pending.txt"),
    )
    .unwrap();
    let listed = list
        .lines()
        .filter(|l| !l.starts_with('#'))
        .any(|l| l.split_whitespace().next() == Some(name));
    if listed {
        eprintln!("{name}: skipped, listed in tests/syn3-pending.txt");
    }
    listed
}
