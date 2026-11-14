//! The comment policy is checked against actual comment tokens so diagnostic
//! strings and embedded programs remain outside the cleanup's scope.

use std::path::Path;
use std::process::Command;

#[test]
fn source_comments_keep_the_policy() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for arguments in [
        vec!["scripts/doc-comments", "check"],
        vec!["scripts/test-doc-comments.py"],
    ] {
        let output = Command::new("python3")
            .args(arguments)
            .current_dir(&root)
            .output()
            .expect("the comment policy check requires python3");
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
