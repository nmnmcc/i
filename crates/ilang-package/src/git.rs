//! Gets the versions of packages with a location from git.
//!
//! The `git` program is run as a subprocess; there is no git library.
//!
//! # The cache
//!
//! The cache directory is, in this order: `$ILANG_CACHE`,
//! `$XDG_CACHE_HOME/ilang`, `$HOME/.cache/ilang`, or `%LOCALAPPDATA%\ilang`.
//! In it:
//!
//! - `git/<escaped repository>/` is a bare repository with the tags
//!   `vX.Y.Z` of the repository. It is fetched with
//!   `+refs/tags/v*:refs/tags/v*`, so a tag that is moved or taken out there
//!   is moved or taken out here.
//! - `pkg/<escaped location>/<commit>/` is the files of the package at a
//!   commit: the tree under the `//dir` of the location, written from the git
//!   objects without a checkout, so their bytes are as git keeps them. The
//!   files are made read-only. Symbolic links are made as symbolic links on
//!   Unix, and as files with their target as bytes elsewhere. Submodules are
//!   left out. The directory is written next to it and then renamed, so it
//!   is either all there or not there.
//! - `pkg/<escaped location>/<commit>.hash` is the hash of those files
//!   (see [`crate::lock::hash`]), worked out from the git objects.
//!
//! An escaped name keeps the bytes `a-z`, `0-9`, `.`, `_` and `-`, and writes
//! every other byte as `%` and two lowercase hex digits, so
//! `github.com/alice/json` is `github.com%2falice%2fjson`.
//!
//! # Where a repository is got from
//!
//! A location has no scheme; its repository is got from `https://` and the
//! repository. [`Git::new`] takes a list of rewrites, from a prefix of
//! the repository to a prefix of a URL: the longest prefix that matches is
//! replaced. `ILANG_GIT_URLS` gives the list from the environment, as
//! entries `prefix=url` separated by white space, for example
//! `ILANG_GIT_URLS="example.com/=file:///srv/git/"` gets
//! `example.com/alice/json` from `file:///srv/git/alice/json`. Tests use it
//! with local bare repositories; it may also point at a mirror, because the
//! hash in `i.lock` makes certain the files are the same.
//!
//! # The network
//!
//! A repository is fetched at most once by one [`Git`], and only when a tag
//! that is needed is not in the cache, or when its commit there is not the
//! one in `i.lock`. When everything is in the cache, nothing is fetched.
//! When offline, nothing is ever fetched, and a tag that is not in the cache
//! is an error.

use crate::code;
use crate::lock::{FileKind, PackageFile};
use crate::manifest::{Version, parse_version};
use std::collections::HashMap;
use std::ffi::OsString;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// A problem while getting a package. `code` is one of [`crate::code`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitError {
    pub code: &'static str,
    pub message: String,
}

impl GitError {
    pub fn new(code: &'static str, message: impl Into<String>) -> GitError {
        GitError {
            code,
            message: message.into(),
        }
    }
}

type Result<T> = std::result::Result<T, GitError>;

/// The cache directory, from the environment variables that `get` reads.
pub fn cache_dir(get: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let set = |k: &str| get(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    if let Some(p) = set("ILANG_CACHE") {
        return Some(p);
    }
    if let Some(p) = set("XDG_CACHE_HOME").filter(|p| p.is_absolute()) {
        return Some(p.join("ilang"));
    }
    if let Some(p) = set("HOME") {
        return Some(p.join(".cache").join("ilang"));
    }
    set("LOCALAPPDATA").map(|p| p.join("ilang"))
}

/// Reads `ILANG_GIT_URLS`: entries `prefix=url` separated by white space.
/// Entries without `=` are left out.
pub fn parse_url_map(text: &str) -> Vec<(String, String)> {
    text.split_whitespace()
        .filter_map(|e| e.split_once('='))
        .filter(|(p, _)| !p.is_empty())
        .map(|(p, u)| (p.to_string(), u.to_string()))
        .collect()
}

/// A name for the cache: `a-z`, `0-9`, `.`, `_` and `-` are kept, and every
/// other byte is `%` and two lowercase hex digits.
pub fn escape(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'-') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02x}"));
        }
    }
    out
}

/// Gets packages from git into a cache.
#[derive(Debug)]
pub struct Git {
    cache: PathBuf,
    offline: bool,
    urls: Vec<(String, String)>,
    /// The repositories fetched by this value, and how it went.
    fetched: HashMap<String, Result<()>>,
}

impl Git {
    pub fn new(cache: PathBuf, offline: bool, urls: Vec<(String, String)>) -> Git {
        Git {
            cache,
            offline,
            urls,
            fetched: HashMap::new(),
        }
    }

    pub fn cache(&self) -> &Path {
        &self.cache
    }

    /// The URL that `repository` is got from.
    pub fn url(&self, repository: &str) -> String {
        let best = self
            .urls
            .iter()
            .filter(|(p, _)| repository.starts_with(p.as_str()))
            .max_by_key(|(p, _)| p.len());
        match best {
            Some((p, u)) => format!("{u}{}", &repository[p.len()..]),
            None => format!("https://{repository}"),
        }
    }

    fn repo_dir(&self, repository: &str) -> PathBuf {
        self.cache.join("git").join(escape(repository))
    }

    fn command(&self, repo: Option<&Path>) -> Command {
        let mut cmd = Command::new("git");
        if let Some(r) = repo {
            cmd.arg("--git-dir").arg(r);
        }
        cmd.env("GIT_TERMINAL_PROMPT", "0");
        for k in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_OBJECT_DIRECTORY",
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            "GIT_NAMESPACE",
            "GIT_COMMON_DIR",
        ] {
            cmd.env_remove(k);
        }
        cmd
    }

    /// Runs git, with `input` on its standard input.
    fn run(&self, mut cmd: Command, input: Option<Vec<u8>>) -> Result<std::process::Output> {
        cmd.stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
        let mut child = cmd.spawn().map_err(|e| {
            GitError::new(
                code::GIT,
                format!("`git` could not be run ({e}); it is needed to get packages by location"),
            )
        })?;
        let writer = match (input, child.stdin.take()) {
            (Some(bytes), Some(mut stdin)) => Some(std::thread::spawn(move || {
                let _ = stdin.write_all(&bytes);
            })),
            _ => None,
        };
        let out = child
            .wait_with_output()
            .map_err(|e| GitError::new(code::GIT, format!("`git` failed: {e}")))?;
        if let Some(w) = writer {
            let _ = w.join();
        }
        Ok(out)
    }

    /// Runs git in the bare repository of `repository`, and gives its
    /// output. A failure is an error.
    fn run_ok(&self, repository: &str, args: &[&str], input: Option<Vec<u8>>) -> Result<Vec<u8>> {
        let mut cmd = self.command(Some(&self.repo_dir(repository)));
        cmd.args(args);
        let out = self.run(cmd, input)?;
        if !out.status.success() {
            return Err(GitError::new(
                code::GIT,
                format!(
                    "`git {}` failed for `{repository}`: {}",
                    args.join(" "),
                    String::from_utf8_lossy(&out.stderr).trim()
                ),
            ));
        }
        Ok(out.stdout)
    }

    /// Fetches the tags `v*` of `repository` into the cache, at most once.
    pub fn fetch(&mut self, repository: &str) -> Result<()> {
        if let Some(r) = self.fetched.get(repository) {
            return r.clone();
        }
        let r = self.fetch_now(repository);
        self.fetched.insert(repository.to_string(), r.clone());
        r
    }

    fn fetch_now(&mut self, repository: &str) -> Result<()> {
        let url = self.url(repository);
        if self.offline {
            return Err(GitError::new(
                code::OFFLINE,
                format!(
                    "`{repository}` would have to be fetched from `{url}`, but the network may not be used (offline)"
                ),
            ));
        }
        let dir = self.repo_dir(repository);
        if !dir.join("HEAD").is_file() {
            self.init_bare(&dir)?;
        }
        let mut cmd = self.command(Some(&dir));
        cmd.args([
            "fetch",
            "--quiet",
            "--no-tags",
            "--prune",
            "--force",
            "--",
            &url,
            "+refs/tags/v*:refs/tags/v*",
        ]);
        let out = self.run(cmd, None)?;
        if !out.status.success() {
            return Err(GitError::new(
                code::GIT,
                format!(
                    "`{repository}` could not be fetched from `{url}`: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                ),
            ));
        }
        Ok(())
    }

    fn init_bare(&self, dir: &Path) -> Result<()> {
        let parent = dir.parent().unwrap_or(dir);
        let cache_err = |e: std::io::Error| {
            GitError::new(
                code::CACHE,
                format!("the cache `{}` could not be written: {e}", parent.display()),
            )
        };
        std::fs::create_dir_all(parent).map_err(cache_err)?;
        let tmp = temp_name(dir);
        let mut cmd = self.command(None);
        cmd.args(["init", "--bare", "--quiet"]).arg(&tmp);
        let out = self.run(cmd, None)?;
        if !out.status.success() {
            return Err(GitError::new(
                code::GIT,
                format!(
                    "`git init` failed in `{}`: {}",
                    tmp.display(),
                    String::from_utf8_lossy(&out.stderr).trim()
                ),
            ));
        }
        if std::fs::rename(&tmp, dir).is_err() {
            let _ = std::fs::remove_dir_all(&tmp);
            if !dir.join("HEAD").is_file() {
                return Err(GitError::new(
                    code::CACHE,
                    format!("the cache `{}` could not be written", dir.display()),
                ));
            }
        }
        Ok(())
    }

    /// The versions that the tags `vX.Y.Z` of `repository` give, from the
    /// cache, fetched first when the repository is not in the cache.
    pub fn tags(&mut self, repository: &str) -> Result<Vec<Version>> {
        if !self.repo_dir(repository).join("HEAD").is_file() {
            self.fetch(repository)?;
        }
        let out = self.run_ok(
            repository,
            &[
                "for-each-ref",
                "--format=%(refname:strip=2)",
                "refs/tags/v*",
            ],
            None,
        )?;
        let mut versions: Vec<Version> = String::from_utf8_lossy(&out)
            .lines()
            .filter_map(|l| l.strip_prefix('v').and_then(parse_version))
            .collect();
        versions.sort();
        Ok(versions)
    }

    /// The commit of the tag of `version` in the cache, if there is one.
    fn peel(&self, repository: &str, version: Version) -> Result<Option<String>> {
        let dir = self.repo_dir(repository);
        if !dir.join("HEAD").is_file() {
            return Ok(None);
        }
        let mut cmd = self.command(Some(&dir));
        cmd.args([
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/tags/v{version}^{{commit}}"),
        ]);
        let out = self.run(cmd, None)?;
        let commit = String::from_utf8_lossy(&out.stdout).trim().to_string();
        Ok((out.status.success() && !commit.is_empty()).then_some(commit))
    }

    /// Returns the commit of the tag `v<version>` of `repository`.
    ///
    /// The cache is used when it has the tag, and, when `expected` is given,
    /// when the tag is at that commit there. Otherwise the repository is
    /// fetched first. When offline, a tag in the cache at another commit is
    /// given as it is, and the caller finds that it is not `expected`.
    pub fn commit(
        &mut self,
        repository: &str,
        version: Version,
        expected: Option<&str>,
    ) -> Result<String> {
        let mut commit = self.peel(repository, version)?;
        let stale = match (&commit, expected) {
            (None, _) => true,
            (Some(c), Some(e)) => c != e,
            (Some(_), None) => false,
        };
        if stale {
            if self.offline && commit.is_some() {
                return Ok(commit.unwrap_or_default());
            }
            self.fetch(repository)?;
            commit = self.peel(repository, version)?;
        }
        match commit {
            Some(c) => Ok(c),
            None => {
                let there = self.tags(repository).unwrap_or_default();
                let list: Vec<String> = there.iter().map(|v| v.to_string()).collect();
                let there = if list.is_empty() {
                    "it has no versions".to_string()
                } else {
                    format!("its versions are {}", list.join(", "))
                };
                Err(GitError::new(
                    code::NO_VERSION,
                    format!("`{repository}` has no tag `v{version}` ({there})"),
                ))
            }
        }
    }

    /// Reads objects with `git cat-file --batch`. `None` for an object that
    /// is not there.
    fn objects(&self, repository: &str, names: &[String]) -> Result<Vec<Option<Vec<u8>>>> {
        if names.is_empty() {
            return Ok(Vec::new());
        }
        let mut input = Vec::new();
        for n in names {
            input.extend_from_slice(n.as_bytes());
            input.push(b'\n');
        }
        let out = self.run_ok(repository, &["cat-file", "--batch"], Some(input))?;
        let bad = || {
            GitError::new(
                code::GIT,
                format!(
                    "`git cat-file --batch` gave output that could not be read for `{repository}`"
                ),
            )
        };
        let mut rest: &[u8] = &out;
        let mut objects = Vec::new();
        for _ in names {
            let nl = rest.iter().position(|&b| b == b'\n').ok_or_else(bad)?;
            let header = std::str::from_utf8(&rest[..nl]).map_err(|_| bad())?;
            rest = &rest[nl + 1..];
            let parts: Vec<&str> = header.split(' ').collect();
            if parts.len() != 3 {
                // `<name> missing`, or `ambiguous`.
                objects.push(None);
                continue;
            }
            let size: usize = parts[2].parse().map_err(|_| bad())?;
            if rest.len() < size + 1 {
                return Err(bad());
            }
            objects.push((parts[1] == "blob").then(|| rest[..size].to_vec()));
            rest = &rest[size + 1..];
        }
        Ok(objects)
    }

    /// The bytes of the file `path` in the tree of `commit`, as git keeps
    /// them. `None` when there is no such file.
    pub fn read(&self, repository: &str, commit: &str, path: &str) -> Result<Option<Vec<u8>>> {
        let mut objects = self.objects(repository, &[format!("{commit}:{path}")])?;
        Ok(objects.pop().flatten())
    }

    /// The files of the package in `dir` of the tree of `commit`, or in the
    /// whole tree: read with `git ls-tree` and `git cat-file`, with
    /// their bytes as git keeps them, symbolic links with their target as
    /// bytes, and without submodules. In the order of their paths.
    pub fn files(
        &self,
        repository: &str,
        commit: &str,
        dir: Option<&str>,
    ) -> Result<Vec<PackageFile>> {
        let tree = format!("{commit}:{}", dir.unwrap_or(""));
        let out = self.run_ok(repository, &["ls-tree", "-r", "-z", &tree], None)?;
        let mut entries = Vec::new();
        for entry in out.split(|&b| b == 0).filter(|e| !e.is_empty()) {
            let tab = entry.iter().position(|&b| b == b'\t');
            let (meta, path) = match tab {
                Some(t) => (&entry[..t], &entry[t + 1..]),
                None => (entry, &b""[..]),
            };
            let meta = String::from_utf8_lossy(meta);
            let parts: Vec<&str> = meta.split(' ').collect();
            let [mode, kind, oid] = parts[..] else {
                return Err(GitError::new(
                    code::GIT,
                    format!("`git ls-tree` gave output that could not be read for `{repository}`"),
                ));
            };
            let Ok(path) = String::from_utf8(path.to_vec()) else {
                return Err(GitError::new(
                    code::PACKAGE_FILES,
                    format!(
                        "`{repository}` at `{commit}` has a path that is not UTF-8: `{}`",
                        String::from_utf8_lossy(path)
                    ),
                ));
            };
            let kind = match (mode, kind) {
                // A submodule is not part of the package.
                (_, "commit") => continue,
                ("120000", "blob") => FileKind::Symlink,
                ("100755", "blob") => FileKind::Executable,
                (_, "blob") => FileKind::Regular,
                _ => continue,
            };
            if !good_path(&path) {
                return Err(GitError::new(
                    code::PACKAGE_FILES,
                    format!(
                        "`{repository}` at `{commit}` has a path that cannot be a file: `{path}`"
                    ),
                ));
            }
            entries.push((path, kind, oid.to_string()));
        }
        let names: Vec<String> = entries.iter().map(|e| e.2.clone()).collect();
        let blobs = self.objects(repository, &names)?;
        let mut files = Vec::new();
        for ((path, kind, oid), bytes) in entries.into_iter().zip(blobs) {
            let Some(bytes) = bytes else {
                return Err(GitError::new(
                    code::GIT,
                    format!("`{repository}` does not have the object `{oid}` of `{path}`"),
                ));
            };
            files.push(PackageFile { path, kind, bytes });
        }
        files.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
        Ok(files)
    }

    /// The directory in the cache with the files of the package at `place`
    /// (the location with its `//dir`) at `commit`, and their hash.
    /// They are written when they are not in the cache yet.
    pub fn package(
        &self,
        repository: &str,
        dir: Option<&str>,
        place: &str,
        commit: &str,
    ) -> Result<(PathBuf, String)> {
        let base = self.cache.join("pkg").join(escape(place));
        let target = base.join(commit);
        let hash_file = base.join(format!("{commit}.hash"));
        if target.is_dir()
            && let Ok(h) = std::fs::read_to_string(&hash_file)
            && h.trim().len() == 71
            && h.starts_with("sha256-")
        {
            return Ok((target, h.trim().to_string()));
        }
        let files = self.files(repository, commit, dir)?;
        let hash = crate::lock::hash(&files);
        let cache_err = |e: std::io::Error| {
            GitError::new(
                code::CACHE,
                format!("the cache `{}` could not be written: {e}", base.display()),
            )
        };
        std::fs::create_dir_all(&base).map_err(cache_err)?;
        if !target.is_dir() {
            let tmp = temp_name(&target);
            let written = write_files(&tmp, &files);
            if let Err(e) = written {
                let _ = std::fs::remove_dir_all(&tmp);
                return Err(cache_err(e));
            }
            if std::fs::rename(&tmp, &target).is_err() {
                let _ = std::fs::remove_dir_all(&tmp);
                if !target.is_dir() {
                    return Err(GitError::new(
                        code::CACHE,
                        format!("the cache `{}` could not be written", target.display()),
                    ));
                }
            }
        }
        let tmp = temp_name(&hash_file);
        std::fs::write(&tmp, format!("{hash}\n")).map_err(cache_err)?;
        std::fs::rename(&tmp, &hash_file).map_err(cache_err)?;
        Ok((target, hash))
    }
}

/// A name next to `path` that no other process uses.
fn temp_name(path: &Path) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!(".tmp-{name}-{}-{nanos}", std::process::id()))
}

/// A path of a tree that can be written under a directory: parts that are
/// not empty, `.` or `..`, with no byte that a file name may not have.
fn good_path(path: &str) -> bool {
    path.split('/').all(|p| {
        !p.is_empty() && p != "." && p != ".." && !p.contains(['\0', '\\']) && {
            #[cfg(windows)]
            {
                !p.contains([':', '<', '>', '"', '|', '?', '*'])
            }
            #[cfg(not(windows))]
            {
                true
            }
        }
    })
}

/// Writes the files under `dir`, then makes them read-only. Symbolic links
/// are made last, so that no file is written through one.
fn write_files(dir: &Path, files: &[PackageFile]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let mut written = Vec::new();
    for f in files.iter().filter(|f| f.kind != FileKind::Symlink) {
        let path = dir.join(&f.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &f.bytes)?;
        written.push(path);
    }
    for f in files.iter().filter(|f| f.kind == FileKind::Symlink) {
        let path = dir.join(&f.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            let target = std::ffi::OsStr::from_bytes(&f.bytes);
            std::os::unix::fs::symlink(target, &path)?;
        }
        #[cfg(not(unix))]
        {
            std::fs::write(&path, &f.bytes)?;
            written.push(path);
        }
    }
    for path in written {
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&path, perms)?;
    }
    Ok(())
}
