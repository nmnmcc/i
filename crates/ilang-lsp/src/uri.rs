//! `file:` URIs and paths.

use lsp_types::Uri;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// The path of a `file:` URI, with its `%XX` escapes decoded.
pub fn to_path(uri: &Uri) -> Option<PathBuf> {
    let s = uri.as_str();
    let rest = s.strip_prefix("file://")?;
    // `file://host/path`: only an empty host or `localhost` is local.
    let path = match rest.find('/') {
        Some(0) => rest,
        Some(i) if &rest[..i] == "localhost" => &rest[i..],
        _ => return None,
    };
    let path = path.split(['?', '#']).next().unwrap_or(path);
    let bytes = decode(path)?;
    let text = String::from_utf8(bytes).ok()?;
    // `/C:/dir` on Windows.
    let b = text.as_bytes();
    if cfg!(windows) && b.len() >= 3 && b[0] == b'/' && b[1].is_ascii_alphabetic() && b[2] == b':' {
        return Some(PathBuf::from(&text[1..]));
    }
    Some(PathBuf::from(text))
}

fn decode(s: &str) -> Option<Vec<u8>> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    Some(out)
}

/// The `file:` URI of an absolute path.
pub fn from_path(path: &Path) -> Option<Uri> {
    let text = path.to_str()?.replace('\\', "/");
    let mut out = String::from("file://");
    if !text.starts_with('/') {
        out.push('/');
    }
    for b in text.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    Uri::from_str(&out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let p = Path::new("/tmp/a b/ü#x.i");
        let uri = from_path(p).unwrap();
        assert_eq!(uri.as_str(), "file:///tmp/a%20b/%C3%BC%23x.i");
        assert_eq!(to_path(&uri).unwrap(), p);
        let uri = Uri::from_str("file:///home/me/src/main.i").unwrap();
        assert_eq!(to_path(&uri).unwrap(), Path::new("/home/me/src/main.i"));
        let uri = Uri::from_str("untitled:Untitled-1").unwrap();
        assert_eq!(to_path(&uri), None);
    }
}
