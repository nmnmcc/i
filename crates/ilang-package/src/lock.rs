//! The lock file `i.lock` (§2.8) and the hash of a package.

use crate::manifest::{Version, parse_version};
use sha2::{Digest, Sha256};

/// A file of a package, as git keeps it (§2.8).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageFile {
    /// The path from the package directory, with `/` between the parts.
    pub path: String,
    pub kind: FileKind,
    /// The bytes as git keeps them; for a symbolic link, its target.
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    Regular,
    Executable,
    Symlink,
}

/// The hash of a package (§2.8): `sha256-` and the lowercase hex SHA-256 of,
/// for each file in the order of their paths as UTF-8 bytes, the path, a zero
/// byte, the size as a decimal number, a zero byte, and the bytes.
pub fn hash(files: &[PackageFile]) -> String {
    let mut sorted: Vec<&PackageFile> = files.iter().collect();
    sorted.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
    let mut h = Sha256::new();
    for f in sorted {
        h.update(f.path.as_bytes());
        h.update([0]);
        h.update(f.bytes.len().to_string().as_bytes());
        h.update([0]);
        h.update(&f.bytes);
    }
    let digest = h.finalize();
    let mut out = String::with_capacity(7 + 64);
    out.push_str("sha256-");
    for b in digest {
        out.push(char::from_digit(u32::from(b >> 4), 16).unwrap());
        out.push(char::from_digit(u32::from(b & 15), 16).unwrap());
    }
    out
}

/// One line of `i.lock`: a version of a package with a location.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockLine {
    /// The location, with its `//dir`.
    pub location: String,
    pub version: Version,
    /// The git commit of the tag, as 40 (SHA-1) or 64 (SHA-256) lowercase
    /// hex digits.
    pub commit: String,
    /// `sha256-` and 64 lowercase hex digits.
    pub hash: String,
}

impl LockLine {
    /// The order of `i.lock`: locations, then versions, as UTF-8 bytes.
    fn sort_key(&self) -> (&[u8], String) {
        (self.location.as_bytes(), self.version.to_string())
    }
}

/// The lines of `i.lock`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lock {
    pub lines: Vec<LockLine>,
}

/// A line of `i.lock` that is not correct.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockError {
    /// From 1.
    pub line: usize,
    /// The byte offset of the line in the file.
    pub offset: usize,
    pub len: usize,
    pub message: String,
}

fn is_hex(s: &str) -> bool {
    s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

impl Lock {
    /// Reads `i.lock`. Empty lines are left out. A location and version
    /// that is given twice is an error.
    pub fn parse(text: &str) -> Result<Lock, LockError> {
        let mut lines: Vec<LockLine> = Vec::new();
        let mut offset = 0;
        for (i, raw) in text.split_inclusive('\n').enumerate() {
            let start = offset;
            offset += raw.len();
            let line = raw.strip_suffix('\n').unwrap_or(raw);
            let line = line.strip_suffix('\r').unwrap_or(line);
            if line.trim().is_empty() {
                continue;
            }
            let bad = |message: String| LockError {
                line: i + 1,
                offset: start,
                len: line.len(),
                message,
            };
            let parts: Vec<&str> = line.split(' ').collect();
            let [location, version, commit, hash] = parts[..] else {
                return Err(bad(
                    "a line of `i.lock` is a location, a version, a commit, and a hash, with one space between them".to_string(),
                ));
            };
            let place = crate::manifest::parse_location(&format!("{location}@{version}"));
            let Ok(place) = place else {
                return Err(bad(format!(
                    "`{location} {version}` is not a location and a version"
                )));
            };
            if place.place() != location || parse_version(version).is_none() {
                return Err(bad(format!("`{location}` is not a location")));
            }
            if !(commit.len() == 40 || commit.len() == 64) || !is_hex(commit) {
                return Err(bad(format!("`{commit}` is not a git commit")));
            }
            let digest = hash.strip_prefix("sha256-").unwrap_or("");
            if digest.len() != 64 || !is_hex(digest) {
                return Err(bad(format!(
                    "`{hash}` is not `sha256-` and 64 lowercase hex digits"
                )));
            }
            if lines
                .iter()
                .any(|l| l.location == location && l.version == place.version)
            {
                return Err(bad(format!(
                    "`{location} {version}` has more than one line"
                )));
            }
            lines.push(LockLine {
                location: location.to_string(),
                version: place.version,
                commit: commit.to_string(),
                hash: hash.to_string(),
            });
        }
        Ok(Lock { lines })
    }

    pub fn get(&self, location: &str, version: Version) -> Option<&LockLine> {
        self.lines
            .iter()
            .find(|l| l.location == location && l.version == version)
    }

    /// The text of `i.lock`: one line for each version, in the order of
    /// locations and then versions as UTF-8 bytes (§2.8).
    pub fn render(&self) -> String {
        let mut lines: Vec<&LockLine> = self.lines.iter().collect();
        lines.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
        let mut out = String::new();
        for l in lines {
            out.push_str(&format!(
                "{} {} {} {}\n",
                l.location, l.version, l.commit, l.hash
            ));
        }
        out
    }
}
