//! The lock file and the hash of a package (§2.8).

use ilang_package::lock::{FileKind, Lock, LockLine, PackageFile, hash};
use ilang_package::manifest::parse_version;

fn file(path: &str, kind: FileKind, bytes: &[u8]) -> PackageFile {
    PackageFile {
        path: path.to_string(),
        kind,
        bytes: bytes.to_vec(),
    }
}

/// The tree of the hash vector. The expected hash was worked out once, apart
/// from this crate, with Python:
///
/// ```text
/// h = hashlib.sha256()
/// for p in sorted(files):  # paths as UTF-8 bytes
///     h.update(p + b"\0" + str(len(files[p])).encode() + b"\0" + files[p])
/// ```
pub const VECTOR_HASH: &str =
    "sha256-c7432df6c2877e89b6414be5da72b517b09270e4e9be6210dc41f7ff4bb75348";

fn vector_files() -> Vec<PackageFile> {
    // Not in the order of their paths.
    vec![
        file("src/b.i", FileKind::Regular, b"z = 3\n"),
        file("i.toml", FileKind::Regular, b"name = \"vec\"\n"),
        file("src/a.i", FileKind::Regular, b"x = 1\r\ny = 2\r\n"),
        file("link", FileKind::Symlink, b"src/a.i"),
        file("src-x", FileKind::Regular, b""),
        file("run.sh", FileKind::Executable, b"#!/bin/sh\n"),
    ]
}

#[test]
fn hash_vector() {
    assert_eq!(hash(&vector_files()), VECTOR_HASH);
    // No files: the SHA-256 of nothing.
    assert_eq!(
        hash(&[]),
        "sha256-e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    // The bytes count, line ends too.
    let mut lf = vector_files();
    lf[2].bytes = b"x = 1\ny = 2\n".to_vec();
    assert_ne!(hash(&lf), VECTOR_HASH);
}

fn line(location: &str, version: &str, c: char) -> LockLine {
    LockLine {
        location: location.to_string(),
        version: parse_version(version).unwrap(),
        commit: c.to_string().repeat(40),
        hash: format!("sha256-{}", c.to_string().repeat(64)),
    }
}

#[test]
fn render_order_and_parse() {
    let lock = Lock {
        lines: vec![
            line("github.com/alice/json", "1.9.0", 'a'),
            line("github.com/alice/json", "1.10.0", 'b'),
            line("codeberg.org/bob/tools//text", "0.3.2", 'c'),
            line("github.com/alice/json", "2.0.1", 'd'),
            line("codeberg.org/bob/tools", "0.1.0", 'e'),
        ],
    };
    let text = lock.render();
    let firsts: Vec<String> = text
        .lines()
        .map(|l| l.split(' ').take(2).collect::<Vec<_>>().join(" "))
        .collect();
    // Locations, then versions, as UTF-8 bytes: "1.10.0" is before "1.9.0".
    assert_eq!(
        firsts,
        [
            "codeberg.org/bob/tools 0.1.0",
            "codeberg.org/bob/tools//text 0.3.2",
            "github.com/alice/json 1.10.0",
            "github.com/alice/json 1.9.0",
            "github.com/alice/json 2.0.1",
        ]
    );
    let back = Lock::parse(&text).unwrap();
    assert_eq!(back.render(), text);
    assert_eq!(
        back.get("github.com/alice/json", parse_version("2.0.1").unwrap())
            .unwrap()
            .commit,
        "d".repeat(40)
    );
    assert!(Lock::parse("").unwrap().lines.is_empty());
    assert!(Lock::parse("\n\n").unwrap().lines.is_empty());
}

#[test]
fn bad_lock_lines() {
    let good = format!(
        "github.com/alice/json 1.4.0 {} sha256-{}",
        "a".repeat(40),
        "b".repeat(64)
    );
    assert!(Lock::parse(&good).is_ok());
    let bad = [
        "github.com/alice/json 1.4.0".to_string(),
        good.replace("1.4.0", "1.4"),
        good.replace("github.com/alice/json", "json"),
        good.replace(&"a".repeat(40), &"A".repeat(40)),
        good.replace(&"a".repeat(40), "abc"),
        good.replace("sha256-", "sha512-"),
        good.replace(' ', "  "),
        format!("{good}\n{good}\n"),
    ];
    for text in bad {
        let e = Lock::parse(&format!("\n{text}")).unwrap_err();
        assert!(e.line >= 2, "{text}: {e:?}");
    }
}
