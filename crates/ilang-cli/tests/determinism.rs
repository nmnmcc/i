//! Building the same project twice gives the same bytes (and `i expand` the same text). Caches
//! keyed by content are only sound if this holds. When it does not, the test names the first
//! section of the Wasm module that differs, so the cause can be found: this test does not hide
//! nondeterminism, and the compiler is not changed to make it pass.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

fn i(args: &[&str], project: &Path) -> Output {
    let out = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(args)
        .arg("--project")
        .arg(project)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "i {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

/// `(id, start, end)` of each section of a Wasm module (after the 8 byte header).
fn sections(bytes: &[u8]) -> Vec<(u8, usize, usize)> {
    fn leb(bytes: &[u8], at: &mut usize) -> usize {
        let (mut value, mut shift) = (0usize, 0);
        loop {
            let b = bytes[*at];
            *at += 1;
            value |= ((b & 0x7f) as usize) << shift;
            if b & 0x80 == 0 {
                return value;
            }
            shift += 7;
        }
    }
    let mut at = 8;
    let mut out = Vec::new();
    while at < bytes.len() {
        let id = bytes[at];
        at += 1;
        let size = leb(bytes, &mut at);
        out.push((id, at, at + size));
        at += size;
    }
    out
}

/// Where two modules differ: the first differing section, with the byte offset in it.
fn first_difference(a: &[u8], b: &[u8]) -> String {
    let (sa, sb) = (sections(a), sections(b));
    for (x, y) in sa.iter().zip(&sb) {
        let (ba, bb) = (&a[x.1..x.2], &b[y.1..y.2]);
        if x.0 != y.0 || ba != bb {
            let offset = ba.iter().zip(bb).take_while(|(p, q)| p == q).count();
            return format!(
                "section id {} (sizes {} and {}), first differing byte {offset} in it",
                x.0,
                ba.len(),
                bb.len()
            );
        }
    }
    format!("section counts {} and {}", sa.len(), sb.len())
}

fn check_project(name: &str, project: &Path) {
    let dir = tempfile::tempdir().unwrap();
    for opt in ["0", "2"] {
        let build = |tag: &str| {
            let out = dir.path().join(format!("{name}-{opt}-{tag}.wasm"));
            i(
                &[
                    "build",
                    "--target",
                    "host",
                    "--opt",
                    opt,
                    "--out",
                    out.to_str().unwrap(),
                ],
                project,
            );
            std::fs::read(out).unwrap()
        };
        let (first, second) = (build("a"), build("b"));
        assert!(first.starts_with(b"\0asm"), "{name}: not a Wasm module");
        assert!(
            first == second,
            "{name} --opt {opt}: two builds differ: {}",
            first_difference(&first, &second)
        );
    }
    let first = i(&["expand"], project).stdout;
    let second = i(&["expand"], project).stdout;
    assert!(!first.is_empty(), "{name}: empty expansion");
    assert!(first == second, "{name}: two `i expand` outputs differ");
}

#[test]
fn hello_builds_identically() {
    check_project("hello", &example("hello"));
}

#[test]
fn echo_builds_identically() {
    check_project("echo", &example("echo"));
}

#[test]
fn project_using_std_builds_identically() {
    // cli-log depends on std as a normal dependency.
    check_project("cli-log", &example("cli-log"));
}
