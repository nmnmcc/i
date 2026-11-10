//! Helpers shared by the tests.

#![allow(dead_code)]

use ilang_syntax::ast::Module;

/// The debug form of a module without spans, to compare trees.
pub fn shape(module: &Module) -> String {
    ilang_syntax::shape::shape(module)
}

/// The corpus of A1 design §6.1: tracked i files, Markdown i blocks, and
/// Rust literals (including the A2 list). Frozen old inputs remain here
/// after A2. Include new test inputs while they are still untracked.
pub fn corpus() -> Vec<(String, String)> {
    use std::{fs, path::Path, process::Command};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let files = Command::new("git")
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            "*.i",
            "*.md",
            "*.rs",
        ])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(files.status.success());
    let mut out = Vec::new();
    let mut paths: Vec<_> = String::from_utf8(files.stdout)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect();
    paths.sort();
    paths.dedup();
    for path in paths {
        let text = fs::read_to_string(root.join(&path)).unwrap();
        if path.ends_with(".i") {
            out.push((path, text));
        } else if path.ends_with(".md") {
            let mut block = None;
            for (n, line) in text.lines().enumerate() {
                let line = line.strip_prefix("> ").unwrap_or(line);
                match &mut block {
                    None if line == "```i" => block = Some((n + 1, String::new())),
                    Some(_) if line == "```" => {
                        let (n, src) = block.take().unwrap();
                        out.push((format!("{path}:{n}"), src));
                    }
                    Some((_, src)) => {
                        src.push_str(line);
                        src.push('\n');
                    }
                    None => {}
                }
            }
        } else if !path.starts_with("crates/ilang-syntax/") {
            for (n, src) in rust_literals(&text).into_iter().enumerate() {
                out.push((format!("{path}:literal{n}"), src));
            }
        }
    }
    out
}

/// Decode plain and raw Rust strings. Non-i literals are harmless: Old
/// mode rejects them, or they describe the same empty module in both modes.
fn rust_literals(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < bytes.len() {
        if bytes[i..].starts_with(b"//") {
            i += text[i..].find('\n').unwrap_or(bytes.len() - i);
        } else if bytes[i..].starts_with(b"/*") {
            i += 2;
            let mut depth = 1;
            while depth > 0 && i < bytes.len() {
                if bytes[i..].starts_with(b"/*") {
                    depth += 1;
                    i += 2;
                } else if bytes[i..].starts_with(b"*/") {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
        } else if bytes[i] == b'r' && matches!(bytes.get(i + 1), Some(b'#' | b'"')) {
            let start = i + 1;
            let mut open = start;
            while bytes.get(open) == Some(&b'#') {
                open += 1;
            }
            if bytes.get(open) != Some(&b'"') {
                i += 1;
                continue;
            }
            let close = format!("\"{}", "#".repeat(open - start));
            let end = text[open + 1..].find(&close).unwrap() + open + 1;
            out.push(text[open + 1..end].to_string());
            i = end + close.len();
        } else if bytes[i] == b'"' {
            i += 1;
            let mut value = String::new();
            while i < bytes.len() && bytes[i] != b'"' {
                let c = text[i..].chars().next().unwrap();
                i += c.len_utf8();
                if c != '\\' {
                    value.push(c);
                    continue;
                }
                let c = text[i..].chars().next().unwrap();
                i += c.len_utf8();
                match c {
                    'n' => value.push('\n'),
                    'r' => value.push('\r'),
                    't' => value.push('\t'),
                    '0' => value.push('\0'),
                    '\\' | '"' | '\'' => value.push(c),
                    '\n' | '\r' => {
                        while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
                            i += 1;
                        }
                    }
                    'x' => {
                        value.push(u8::from_str_radix(&text[i..i + 2], 16).unwrap() as char);
                        i += 2;
                    }
                    'u' => {
                        let end = text[i..].find('}').unwrap() + i;
                        let code = text[i + 1..end].replace('_', "");
                        value
                            .push(char::from_u32(u32::from_str_radix(&code, 16).unwrap()).unwrap());
                        i = end + 1;
                    }
                    _ => panic!("unknown Rust escape: {c}"),
                }
            }
            i += 1;
            out.push(value);
        } else if bytes[i] == b'\'' {
            // A character literal (a lifetime has no closing quote).
            let n = if bytes.get(i + 1) == Some(&b'\\') {
                4
            } else {
                2 + text[i + 1..].chars().next().map_or(0, char::len_utf8)
            };
            if bytes.get(i + n - 1) == Some(&b'\'') {
                i += n;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    out
}

/// The zero-exception claim of D4 is about the corpus at base. New error
/// goldens deliberately exercise E0113 and are still compared, separately.
pub fn baseline_paths() -> std::collections::HashSet<String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("git")
        .args(["ls-tree", "-r", "--name-only", "b28fca5"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}
