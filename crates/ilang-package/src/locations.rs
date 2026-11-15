//! Chooses and gets the packages with a location, and keeps `i.lock`.
//!
//! The packages got by path are read first, by the loader. Then their
//! dependencies by location are the start of minimal version selection,
//! which is done twice: without and with the tests of the compiled package.
//! `i.lock` pins every version reached by either selection, including
//! superseded manifests. The versions
//! of the one that is compiled are loaded.

use crate::git::{self, Git, GitError};
use crate::lock::{Lock, LockLine};
use crate::manifest::{self, DependencySource, Version};
use crate::mvs::{self, PackageKey, Requirement, Selection};
use crate::{Diagnostic, LoadOptions, Loader, Located, PackageLocation, SourceId, code};
use ilang_syntax::span::Span;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Where a requirement comes from.
#[derive(Clone, Debug)]
enum Origin {
    /// An entry of the `i.toml` of the package got by path in this
    /// canonical directory.
    Manifest(PathBuf, Span),
    /// The `i.toml` of this version (`location@version`) of a package with
    /// a location.
    Package(String),
}

/// A package got by path, as far as the selection needs it.
#[derive(Default)]
struct PathNode {
    deps: Vec<Requirement<Origin>>,
    dev: Vec<Requirement<Origin>>,
    paths: Vec<PathBuf>,
    dev_paths: Vec<PathBuf>,
}

/// The repository and the directory of a location.
fn split_place(place: &str) -> (&str, Option<&str>) {
    match place.split_once("//") {
        Some((r, d)) => (r, Some(d)),
        None => (place, None),
    }
}

/// Reads the `i.toml` of the compiled package and of the packages it gets
/// by path, with or without the tests. The flag is false when one of them
/// could not be read without errors.
fn walk(root: &Path) -> (HashMap<PathBuf, PathNode>, bool) {
    let mut nodes: HashMap<PathBuf, PathNode> = HashMap::new();
    let mut complete = true;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if nodes.contains_key(&dir) {
            continue;
        }
        let is_root = dir == root;
        let Ok(text) = std::fs::read_to_string(dir.join("i.toml")) else {
            complete = false;
            continue;
        };
        let (m, diags) = if is_root {
            manifest::parse_manifest(&text)
        } else {
            manifest::parse_dependency_manifest(&text)
        };
        complete &= !diags.iter().any(Diagnostic::is_error);
        let Some(m) = m else {
            complete = false;
            continue;
        };
        let mut node = PathNode::default();
        for (dev, list) in [(false, &m.dependencies), (true, &m.dev_dependencies)] {
            for d in list {
                match &d.source {
                    DependencySource::Location(loc) => {
                        let req = Requirement {
                            key: PackageKey::new(loc.place(), loc.version),
                            version: loc.version,
                            from: Origin::Manifest(dir.clone(), d.span),
                        };
                        if dev { &mut node.dev } else { &mut node.deps }.push(req);
                    }
                    DependencySource::Path(p) => match dir.join(p).canonicalize() {
                        Ok(p) => if dev {
                            &mut node.dev_paths
                        } else {
                            &mut node.paths
                        }
                        .push(p),
                        Err(_) => complete = false,
                    },
                }
            }
        }
        stack.extend(node.paths.iter().rev().cloned());
        if is_root {
            stack.extend(node.dev_paths.iter().rev().cloned());
        }
        nodes.insert(dir, node);
    }
    (nodes, complete)
}

/// The dependencies by location of the packages got by path from `root`:
/// the start of minimal version selection.
fn start(nodes: &HashMap<PathBuf, PathNode>, root: &Path, tests: bool) -> Vec<Requirement<Origin>> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if !seen.insert(dir.clone()) {
            continue;
        }
        let Some(node) = nodes.get(&dir) else {
            continue;
        };
        out.extend(node.deps.iter().cloned());
        let mut next: Vec<PathBuf> = node.paths.clone();
        if tests && dir == root {
            out.extend(node.dev.iter().cloned());
            next.extend(node.dev_paths.iter().cloned());
        }
        stack.extend(next.into_iter().rev());
    }
    out
}

/// What is known of a version that is reached: its commit and its
/// dependencies by location.
type Reached = Result<(String, Vec<Requirement<Origin>>), GitError>;

/// A version that could not be read, with the requirement that reached it.
type Failed = Box<(Requirement<Origin>, GitError)>;

struct Resolver<'a> {
    git: Git,
    lock: &'a Lock,
    reached: HashMap<(PackageKey, Version), Reached>,
}

impl Resolver<'_> {
    fn reach(&mut self, req: &Requirement<Origin>) -> Result<Vec<Requirement<Origin>>, Failed> {
        let k = (req.key.clone(), req.version);
        if !self.reached.contains_key(&k) {
            let r = self.read(&req.key, req.version);
            self.reached.insert(k.clone(), r);
        }
        match &self.reached[&k] {
            Ok((_, deps)) => Ok(deps.clone()),
            Err(e) => Err(Box::new((req.clone(), e.clone()))),
        }
    }

    /// The commit of a version, and the dependencies in its `i.toml`.
    /// Its path dependencies are not followed: a package got by location may
    /// not have path dependencies, so they are an error when the version is
    /// used.
    fn read(&mut self, key: &PackageKey, version: Version) -> Reached {
        let (repository, dir) = split_place(&key.location);
        let expected = self
            .lock
            .get(&key.location, version)
            .map(|l| l.commit.as_str());
        let commit = self.git.commit(repository, version, expected)?;
        let path = match dir {
            Some(d) => format!("{d}/i.toml"),
            None => "i.toml".to_string(),
        };
        let shown = format!("{}@{version}", key.location);
        let text = match self.git.read(repository, &commit, &path)? {
            Some(bytes) => String::from_utf8(bytes).map_err(|_| {
                GitError::new(
                    code::UNREADABLE,
                    format!("`{shown}/i.toml` is not UTF-8 text"),
                )
            })?,
            None => {
                let what = match dir {
                    Some(d) => format!("the directory `{d}` of the tag `v{version}`"),
                    None => format!("the tag `v{version}`"),
                };
                return Err(GitError::new(
                    code::NO_PACKAGE,
                    format!("there is no `i.toml` in {what} of `{repository}`"),
                ));
            }
        };
        // The problems of the `i.toml` of a version that is used are
        // reported when it is loaded.
        let (m, _) = manifest::parse_dependency_manifest(&text);
        let deps = m
            .map(|m| {
                m.dependencies
                    .iter()
                    .filter_map(|d| match &d.source {
                        DependencySource::Location(loc) => Some(Requirement {
                            key: PackageKey::new(loc.place(), loc.version),
                            version: loc.version,
                            from: Origin::Package(shown.clone()),
                        }),
                        DependencySource::Path(_) => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok((commit, deps))
    }
}

/// `i.lock`, added to the sources when a problem is in it.
struct LockFile {
    path: PathBuf,
    text: Option<String>,
    source: Option<SourceId>,
}

impl LockFile {
    fn source(&mut self, loader: &mut Loader) -> Option<SourceId> {
        let text = self.text.as_ref()?;
        if self.source.is_none() {
            let shown = loader.display(&self.path);
            self.source = Some(loader.graph.sources.add(shown, text.clone()));
        }
        self.source
    }

    /// The span of the line of a location and version.
    fn line_span(&self, location: &str, version: Version) -> Span {
        let Some(text) = &self.text else {
            return Span::default();
        };
        let head = format!("{location} {version} ");
        let mut offset = 0;
        for line in text.split_inclusive('\n') {
            if line.starts_with(&head) {
                let len = line.trim_end().len();
                return Span::new(offset as u32, (offset + len) as u32);
            }
            offset += line.len();
        }
        Span::default()
    }
}

fn report(loader: &mut Loader, origin: &Origin, what: &str, e: &GitError) {
    let (source, span, via) = match origin {
        Origin::Manifest(dir, span) => {
            let source = loader
                .by_dir
                .get(dir)
                .and_then(|&id| loader.graph.packages[id].manifest_source);
            let via = match source {
                Some(_) => String::new(),
                None => format!(" (in `{}`)", loader.display(&dir.join("i.toml"))),
            };
            (source, *span, via)
        }
        Origin::Package(p) => (None, Span::default(), format!(" (a dependency of `{p}`)")),
    };
    loader.diags.push(Diagnostic::error(
        e.code,
        source,
        span,
        format!("`{what}`: {}{via}", e.message),
    ));
}

/// Chooses the versions of the packages with a location, gets them, checks
/// them against `i.lock`, and brings `i.lock` up to date.
/// Gives the packages of the selection for [`LoadOptions::tests`], or none
/// when the files do not match `i.lock`.
pub(crate) fn resolve(
    loader: &mut Loader,
    root: &Path,
    options: &LoadOptions,
) -> HashMap<PackageKey, Located> {
    // `i.lock` is not written when a package file could not be read, as
    // the versions that are used may not be all known.
    let (nodes, complete) = walk(root);
    let mut errors = !complete;
    let without = start(&nodes, root, false);
    let with = start(&nodes, root, true);
    let lock_path = root.join("i.lock");
    let mut lock_file = LockFile {
        text: std::fs::read_to_string(&lock_path).ok(),
        path: lock_path,
        source: None,
    };
    if with.is_empty() && without.is_empty() && lock_file.text.is_none() {
        return HashMap::new();
    }
    let lock = match Lock::parse(lock_file.text.as_deref().unwrap_or("")) {
        Ok(l) => l,
        Err(e) => {
            let source = lock_file.source(loader);
            let span = Span::new(e.offset as u32, (e.offset + e.len) as u32);
            loader.diags.push(Diagnostic::error(
                code::LOCK_SYNTAX,
                source,
                span,
                format!("`i.lock` is not correct: {}", e.message),
            ));
            return HashMap::new();
        }
    };

    let mut lines: Vec<LockLine> = Vec::new();
    let mut got: HashMap<(PackageKey, Version), Located> = HashMap::new();
    let mut mismatch = false;
    let mut selected = None;
    if !with.is_empty() || !without.is_empty() {
        let cache = options
            .cache
            .clone()
            .or_else(|| git::cache_dir(|k| std::env::var_os(k)));
        let Some(cache) = cache else {
            loader.diags.push(Diagnostic::error(
                code::CACHE,
                None,
                Span::default(),
                "there is no cache directory for packages got by location; set `ILANG_CACHE`",
            ));
            return HashMap::new();
        };
        let mut resolver = Resolver {
            git: Git::new(cache, options.offline, options.git_urls.clone()),
            lock: &lock,
            reached: HashMap::new(),
        };
        let (sel_without, e1) = mvs::select(without, |r| resolver.reach(r));
        let (sel_with, e2) = mvs::select(with, |r| resolver.reach(r));
        let mut reported = HashSet::new();
        for (req, e) in e1.into_iter().chain(e2).map(|b| *b) {
            if reported.insert((req.key.clone(), req.version)) {
                let what = format!("{}@{}", req.key.location, req.version);
                report(loader, &req.from, &what, &e);
                errors = true;
            }
        }
        // Every consulted manifest is a resolution input, even if another
        // version supersedes it. Pin both configurations' complete closure.
        let reached: BTreeSet<(PackageKey, Version)> = resolver.reached.keys().cloned().collect();
        for (key, version) in reached {
            let Some(Ok((commit, _))) = resolver.reached.get(&(key.clone(), version)) else {
                continue;
            };
            let (repository, dir) = split_place(&key.location);
            let what = format!("{}@{version}", key.location);
            match resolver
                .git
                .package_snapshot(repository, dir, &key.location, commit)
            {
                Err(e) => {
                    mismatch |= matches!(e.code, code::LOCK_MISMATCH | code::PACKAGE_FILES);
                    let origin = origin(&sel_with, &sel_without, &key, version);
                    report(loader, &origin, &what, &e);
                    errors = true;
                }
                Ok((path, hash, files)) => {
                    if let Some(l) = lock.get(&key.location, version)
                        && (l.commit != *commit || l.hash != hash)
                    {
                        let source = lock_file.source(loader);
                        let span = lock_file.line_span(&key.location, version);
                        loader.diags.push(Diagnostic::error(
                            code::LOCK_MISMATCH,
                            source,
                            span,
                            format!(
                                "`{what}` was got with the commit `{commit}` and the hash `{hash}`, but `i.lock` has the commit `{}` and the hash `{}`; the tag may have been moved, or the files changed on the way",
                                l.commit, l.hash
                            ),
                        ));
                        mismatch = true;
                    }
                    lines.push(LockLine {
                        location: key.location.clone(),
                        version,
                        commit: commit.clone(),
                        hash: hash.clone(),
                    });
                    let location = PackageLocation {
                        location: key.location.clone(),
                        version,
                        commit: commit.clone(),
                        hash,
                    };
                    got.insert(
                        (key, version),
                        Located {
                            location,
                            dir: path,
                            files: std::sync::Arc::new(files),
                        },
                    );
                }
            }
        }
        selected = Some(if options.tests { sel_with } else { sel_without });
    }
    errors |= mismatch;

    if options.write_lock && !errors {
        let text = Lock { lines }.render();
        let changed = match &lock_file.text {
            Some(old) => *old != text,
            None => !text.is_empty(),
        };
        if changed {
            let tmp = lock_file.path.with_file_name(".i.lock.tmp");
            let written =
                std::fs::write(&tmp, &text).and_then(|()| std::fs::rename(&tmp, &lock_file.path));
            if let Err(e) = written {
                let _ = std::fs::remove_file(&tmp);
                loader.diags.push(Diagnostic::error(
                    code::LOCK_WRITE,
                    None,
                    Span::default(),
                    format!(
                        "`{}` could not be written: {e}",
                        loader.display(&lock_file.path)
                    ),
                ));
            }
        }
    }

    // When the files do not match `i.lock`, nothing is compiled.
    if mismatch {
        return HashMap::new();
    }
    let Some(selected) = selected else {
        return HashMap::new();
    };
    selected
        .versions
        .iter()
        .filter_map(|(k, v)| got.get(&(k.clone(), *v)).map(|l| (k.clone(), l.clone())))
        .collect()
}

fn origin(
    a: &Selection<Origin>,
    b: &Selection<Origin>,
    key: &PackageKey,
    version: Version,
) -> Origin {
    a.reached
        .iter()
        .chain(&b.reached)
        .find(|r| &r.key == key && r.version == version)
        .map_or(Origin::Package(key.location.clone()), |r| r.from.clone())
}
