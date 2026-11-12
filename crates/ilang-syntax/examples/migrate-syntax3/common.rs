//! What every subcommand of `migrate-syntax3` shares: finding the i sources of
//! the repository (whole `.i` files, `i` fences in Markdown, i source in Rust
//! string literals), writing a migrated source back in its host's encoding,
//! and keeping `scripts/old-syntax-check.json` keyed by the new text.
//!
//! A subcommand is a function from one source to an [`Outcome`]; it never
//! touches files itself.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A source marked with this text is never changed or counted: it keeps an
/// old form on purpose (a fixture that expects the old form to be rejected).
pub const KEEP: &str = "migrate-syntax3: keep";

pub enum Kind {
    /// A whole `.i` file.
    File,
    /// The body of a ```` ```i ```` fence in Markdown.
    Fence,
    /// A Rust string literal; `raw` when written `r"…"` or `r#"…"#`.
    Embedded { raw: bool },
}

pub struct Source {
    pub path: String,
    /// 1-based line of the source in its host file.
    pub line: usize,
    pub kind: Kind,
    /// The i text.
    pub text: String,
    /// Byte range in the host file that `text` was read from (for a Rust
    /// literal, the whole literal with its quotes).
    pub start: usize,
    pub end: usize,
}

/// What a subcommand made of one source.
#[derive(Default)]
pub struct Outcome {
    /// The migrated text, when it differs.
    pub text: Option<String>,
    /// Forms of the subcommand in the source before migration.
    pub found: usize,
    /// Forms of the subcommand still left in the source after migration.
    pub remaining: usize,
    /// Forms the migrator does not rewrite, with the suggested text.
    pub manual: Vec<String>,
}

pub type Migrate = fn(&Source) -> Outcome;

pub fn root() -> PathBuf {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .expect("git runs");
    PathBuf::from(String::from_utf8(out.stdout).unwrap().trim())
}

fn tracked(root: &Path, patterns: &[&str]) -> Vec<String> {
    let mut args = vec![
        "ls-files",
        "--cached",
        "--others",
        "--exclude-standard",
        "--",
    ];
    args.extend_from_slice(patterns);
    let out = Command::new("git")
        .args(&args)
        .current_dir(root)
        .output()
        .expect("git runs");
    let mut files: Vec<String> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(str::to_string)
        .filter(|p| root.join(p).is_file())
        .collect();
    files.sort();
    files.dedup();
    files
}

/// Every i source the old-syntax check reads, grouped by host file. `only`
/// limits the files to those under one of its paths (all when empty).
pub fn sources(root: &Path, only: &[String]) -> BTreeMap<String, (String, Vec<Source>)> {
    let mut out = BTreeMap::new();
    for path in tracked(root, &["*.i", "*.md", "*.rs"]) {
        if !only.is_empty()
            && !only
                .iter()
                .any(|o| path.starts_with(o.trim_end_matches('/')))
        {
            continue;
        }
        // The migrator's own tests hold old forms on purpose.
        if path.starts_with("crates/ilang-syntax/examples/migrate-syntax3") {
            continue;
        }
        let host = std::fs::read_to_string(root.join(&path)).unwrap();
        let mut list = Vec::new();
        if path.ends_with(".i") {
            list.push(Source {
                path: path.clone(),
                line: 1,
                kind: Kind::File,
                text: host.clone(),
                start: 0,
                end: host.len(),
            });
        } else if path.ends_with(".md") {
            list.extend(fences(&path, &host));
        } else {
            list.extend(rust_strings(&path, &host));
        }
        if !list.is_empty() {
            out.insert(path, (host, list));
        }
    }
    out
}

fn line_of(host: &str, at: usize) -> usize {
    host[..at].matches('\n').count() + 1
}

/// ```` ```i ```` fences (with or without words after `i`), as the old-syntax
/// check reads them.
fn fences(path: &str, host: &str) -> Vec<Source> {
    let mut out = Vec::new();
    let mut at = 0;
    let lines: Vec<(usize, &str)> = host
        .split_inclusive('\n')
        .map(|l| {
            let s = at;
            at += l.len();
            (s, l)
        })
        .collect();
    let mut i = 0;
    while i < lines.len() {
        let (_, l) = lines[i];
        let t = l.trim_end_matches(['\n', '\r']);
        let opens = t
            .strip_prefix("```i")
            .is_some_and(|rest| rest.is_empty() || rest.starts_with([' ', '\t']));
        if !opens {
            i += 1;
            continue;
        }
        let body_start = lines.get(i + 1).map_or(host.len(), |x| x.0);
        let mut j = i + 1;
        while j < lines.len() && !lines[j].1.trim_end().starts_with("```") {
            j += 1;
        }
        if j < lines.len() && lines[j].1.trim_end() == "```" {
            let body_end = lines[j].0;
            out.push(Source {
                path: path.to_string(),
                line: i + 1,
                kind: Kind::Fence,
                text: host[body_start..body_end].to_string(),
                start: body_start,
                end: body_end,
            });
        }
        i = j + 1;
    }
    out
}

/// Rust string literals, skipping comments and character literals (the
/// old-syntax check's lexer). Only literals with an `UpName {` or `UpName(…) {`
/// are returned: the others cannot hold a type with constructors.
fn rust_strings(path: &str, text: &str) -> Vec<Source> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if text[i..].starts_with("//") {
            i = text[i..].find('\n').map_or(b.len(), |e| i + e);
            continue;
        }
        if text[i..].starts_with("/*") {
            let mut depth = 1;
            i += 2;
            while depth > 0 && i < b.len() {
                if text[i..].starts_with("/*") {
                    depth += 1;
                    i += 2;
                } else if text[i..].starts_with("*/") {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            continue;
        }
        let ident_before = i > 0 && (b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_');
        if b[i] == b'r' && !ident_before {
            let mut j = i + 1;
            while j < b.len() && b[j] == b'#' {
                j += 1;
            }
            if j < b.len() && b[j] == b'"' {
                let hashes = &text[i + 1..j];
                let close = format!("\"{hashes}");
                let body = j + 1;
                let end = body + text[body..].find(&close).expect("raw string ends");
                push_string(
                    &mut out,
                    path,
                    text,
                    i,
                    end + close.len(),
                    text[body..end].to_string(),
                    true,
                );
                i = end + close.len();
                continue;
            }
        }
        if b[i] == b'"' {
            let start = i;
            i += 1;
            let mut value = String::new();
            while b[i] != b'"' {
                let c = text[i..].chars().next().unwrap();
                i += c.len_utf8();
                if c != '\\' {
                    value.push(c);
                    continue;
                }
                let c = text[i..].chars().next().unwrap();
                i += c.len_utf8();
                match c {
                    '\n' | '\r' => {
                        while i < b.len() && (b[i] as char).is_ascii_whitespace() {
                            i += 1;
                        }
                    }
                    'n' => value.push('\n'),
                    'r' => value.push('\r'),
                    't' => value.push('\t'),
                    '0' => value.push('\0'),
                    '\\' | '"' | '\'' => value.push(c),
                    'x' => {
                        value.push(u8::from_str_radix(&text[i..i + 2], 16).unwrap() as char);
                        i += 2;
                    }
                    'u' => {
                        let e = i + text[i..].find('}').unwrap();
                        let n = u32::from_str_radix(&text[i + 1..e].replace('_', ""), 16).unwrap();
                        value.push(char::from_u32(n).unwrap());
                        i = e + 1;
                    }
                    _ => panic!("{path}: unknown escape \\{c}"),
                }
            }
            i += 1;
            push_string(&mut out, path, text, start, i, value, false);
            continue;
        }
        if b[i] == b'\'' {
            // A character literal, or a lifetime.
            let rest = &text[i + 1..];
            // The length of the character between the quotes.
            let len = match rest.chars().next() {
                Some('\\') => rest[1..].find('\'').map(|e| e + 1),
                Some(c) => rest[c.len_utf8()..]
                    .starts_with('\'')
                    .then_some(c.len_utf8()),
                None => None,
            };
            i += len.map_or(1, |l| l + 2);
            continue;
        }
        i += text[i..].chars().next().unwrap().len_utf8();
    }
    out
}

fn push_string(
    out: &mut Vec<Source>,
    path: &str,
    host: &str,
    start: usize,
    end: usize,
    value: String,
    raw: bool,
) {
    if !has_type_body(&value) {
        return;
    }
    out.push(Source {
        path: path.to_string(),
        line: line_of(host, start),
        kind: Kind::Embedded { raw },
        text: value,
        start,
        end,
    });
}

/// An `UpName`, maybe with a group, then `{`: the start of a type with constructors.
fn has_type_body(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_uppercase()
            && (i == 0
                || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_' || b[i - 1] == b'.'))
        {
            let mut j = i;
            while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
                j += 1;
            }
            if j < b.len() && b[j] == b'(' {
                let mut depth = 0;
                while j < b.len() {
                    match b[j] {
                        b'(' => depth += 1,
                        b')' => {
                            depth -= 1;
                            if depth == 0 {
                                j += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
            }
            while j < b.len() && b[j] == b' ' {
                j += 1;
            }
            if j < b.len() && b[j] == b'{' {
                return true;
            }
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
    false
}

/// The new host file text after replacing each migrated source.
pub fn rewrite_host(host: &str, changes: &[(&Source, String)]) -> String {
    let mut out = host.to_string();
    let mut sorted: Vec<_> = changes.iter().collect();
    sorted.sort_by_key(|(s, _)| s.start);
    for (s, text) in sorted.into_iter().rev() {
        let replacement = match s.kind {
            Kind::File | Kind::Fence => text.clone(),
            Kind::Embedded { raw: true } => {
                let old = &host[s.start..s.end];
                let open = old.find('"').unwrap() + 1;
                let close = old.len() - (open - 1);
                format!("{}{}{}", &old[..open], text, &old[close..])
            }
            Kind::Embedded { raw: false } => {
                let old = &host[s.start..s.end];
                let real_newlines = old.contains('\n');
                let mut lit = String::from("\"");
                for c in text.chars() {
                    match c {
                        '\\' => lit.push_str("\\\\"),
                        '"' => lit.push_str("\\\""),
                        '\n' if !real_newlines => lit.push_str("\\n"),
                        '\t' if old.contains("\\t") => lit.push_str("\\t"),
                        c => lit.push(c),
                    }
                }
                lit.push('"');
                lit
            }
        };
        out.replace_range(s.start..s.end, &replacement);
    }
    out
}

/// The old-syntax check's identity of a source: the path and the first 16
/// hex digits of the SHA-256 of the text, with the metadata comments of an
/// `.i` file removed.
pub fn identity(path: &str, kind: &Kind, text: &str) -> String {
    let text = match kind {
        Kind::File => text
            .split_inclusive('\n')
            .filter(|l| !l.starts_with("// old-syntax-check:"))
            .collect::<String>(),
        _ => text.to_string(),
    };
    let digest = sha256(text.as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("{path}:{}", &hex[..16])
}

/// Moves each entry of `scripts/old-syntax-check.json` whose source changed
/// to the identity of the new text. The file keeps its order and layout.
pub fn rekey(root: &Path, moves: &[(String, String)]) {
    if moves.is_empty() {
        return;
    }
    let path = root.join("scripts/old-syntax-check.json");
    let mut data = std::fs::read_to_string(&path).unwrap();
    for (old, new) in moves {
        let quoted = format!("\"{old}\":");
        if data.contains(&quoted) {
            data = data.replacen(&quoted, &format!("\"{new}\":"), 1);
        }
    }
    std::fs::write(path, data).unwrap();
}

/// SHA-256 (FIPS 180-4), for the identities of the old-syntax check.
fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for block in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *x = x.wrapping_add(y);
        }
    }
    let mut out = [0u8; 32];
    for (i, x) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&x.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn sha256_known_value() {
        let hex: String = super::sha256(b"abc")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(
            hex,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
