//! Packages (§2.1, §2.6): the package file, the modules of a package, the
//! library package `i`, and the graph of packages that a program uses.
//!
//! Packages with a location are got with git (§2.7, [`git`]), their versions
//! are chosen by minimal version selection ([`mvs`]), and they are checked
//! against the lock file `i.lock` ([`lock`]).
//!
//! The only authority is `docs/spec.md`; section numbers refer to it.

pub mod git;
pub mod library;
mod locations;
pub mod lock;
pub mod manifest;
pub mod modules;
pub mod mvs;

use ilang_syntax::span::{LineIndex, Span};
use manifest::{DependencySource, Manifest, Version, WasmExport};
use mvs::PackageKey;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Component, Path, PathBuf};

/// The stable codes of the problems found while loading packages.
pub mod code {
    pub const MANIFEST_SYNTAX: &str = "E0601";
    pub const MANIFEST_NAME: &str = "E0602";
    pub const MANIFEST_SHAPE: &str = "E0603";
    /// R2 rejects provider plans until the JP implementation lands.
    pub const PROVIDERS_PENDING: &str = "E0522";
    pub const DEPENDENCY_NAME: &str = "E0604";
    pub const DEPENDENCY_TWICE: &str = "E0605";
    pub const DEPENDENCY_VALUE: &str = "E0606";
    pub const BAD_LOCATION: &str = "E0607";
    pub const BAD_VERSION: &str = "E0608";
    pub const PATH_DEPENDENCY: &str = "E0609";
    pub const WASM_EXPORT: &str = "E0610";
    // "E0620" was "getting packages by location is not supported yet".
    pub const NO_PACKAGE: &str = "E0621";
    pub const PACKAGE_CIRCLE: &str = "E0622";
    pub const MODULE_PATH_NAME: &str = "E0623";
    pub const MODULE_TWICE: &str = "E0624";
    pub const UNREADABLE: &str = "E0625";
    /// `git` could not be run, or a repository could not be fetched.
    pub const GIT: &str = "E0630";
    /// A repository has no tag for a version that is reached (§2.7).
    pub const NO_VERSION: &str = "E0631";
    /// The commit or the hash of a version is not the one in `i.lock` (§2.8).
    pub const LOCK_MISMATCH: &str = "E0632";
    /// `i.lock` is not correct.
    pub const LOCK_SYNTAX: &str = "E0633";
    /// Offline, and something that is needed is not in the cache.
    pub const OFFLINE: &str = "E0634";
    /// The tree of a version has a path that cannot be a file.
    pub const PACKAGE_FILES: &str = "E0635";
    /// The cache could not be found or written.
    pub const CACHE: &str = "E0636";
    /// `i.lock` could not be written.
    pub const LOCK_WRITE: &str = "E0637";
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

/// A problem found by any part of the compiler after parsing, in a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    /// The file. `None` when the problem has no file.
    pub source: Option<SourceId>,
    pub span: Span,
    /// Additional source locations explaining the diagnostic.
    pub related: Vec<DiagnosticNote>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticNote {
    pub source: SourceId,
    pub span: Span,
    pub message: String,
}

impl Diagnostic {
    pub fn error(
        code: &'static str,
        source: Option<SourceId>,
        span: Span,
        message: impl Into<String>,
    ) -> Diagnostic {
        Diagnostic {
            severity: Severity::Error,
            code,
            message: message.into(),
            source,
            span,
            related: Vec::new(),
        }
    }

    pub fn warning(
        code: &'static str,
        source: Option<SourceId>,
        span: Span,
        message: impl Into<String>,
    ) -> Diagnostic {
        Diagnostic {
            severity: Severity::Warning,
            ..Diagnostic::error(code, source, span, message)
        }
    }

    pub fn note(source: Option<SourceId>, span: Span, message: impl Into<String>) -> Diagnostic {
        Diagnostic {
            severity: Severity::Note,
            ..Diagnostic::error("N0001", source, span, message)
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceId(pub u32);

#[derive(Clone, Debug)]
pub struct SourceFile {
    /// The path that is shown in diagnostics.
    pub display: String,
    pub text: String,
}

/// All the files that diagnostics may point into.
#[derive(Clone, Debug, Default)]
pub struct Sources {
    files: Vec<SourceFile>,
}

impl Sources {
    pub fn add(&mut self, display: String, text: String) -> SourceId {
        self.files.push(SourceFile { display, text });
        SourceId(self.files.len() as u32 - 1)
    }

    pub fn get(&self, id: SourceId) -> &SourceFile {
        &self.files[id.0 as usize]
    }

    /// Replaces the text of a file, as an editor does with a file that has
    /// changes that are not saved.
    pub fn set_text(&mut self, id: SourceId, text: String) {
        self.files[id.0 as usize].text = text;
    }

    /// `path:line:col: error[CODE]: message`.
    pub fn render(&self, d: &Diagnostic) -> String {
        let mut out = String::new();
        if let Some(id) = d.source {
            let file = self.get(id);
            let (line, col) = LineIndex::new(&file.text).line_col(d.span.start);
            let _ = write!(out, "{}:{line}:{col}: ", file.display);
        }
        let kind = match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
        };
        let _ = write!(out, "{kind}[{}]: {}", d.code, d.message);
        for note in &d.related {
            let file = self.get(note.source);
            let (line, col) = LineIndex::new(&file.text).line_col(note.span.start);
            let _ = write!(
                out,
                "\n  {}:{line}:{col}: note: {}",
                file.display, note.message
            );
        }
        out
    }

    /// Renders diagnostics in a stable order: by file, position, and code.
    pub fn render_all(&self, diags: &[Diagnostic]) -> Vec<String> {
        let mut keyed: Vec<_> = diags
            .iter()
            .map(|d| {
                let file = d.source.map(|s| self.get(s).display.clone());
                (file, d.span.start, d.code, self.render(d))
            })
            .collect();
        keyed.sort();
        keyed.dedup();
        keyed.into_iter().map(|k| k.3).collect()
    }
}

/// `[ "_" ] lower { lower | digit | "_" }` (§1.3, §1.4).
pub fn is_name(s: &str) -> bool {
    let rest = s.strip_prefix('_').unwrap_or(s);
    let mut chars = rest.chars();
    matches!(chars.next(), Some('a'..='z'))
        && chars.all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_'))
}

pub type PackageId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModuleKind {
    /// Under `src/`.
    Src,
    /// Under `test/` (§2.1).
    Test,
    /// The built-in names (§12), written in i. It is in the package `i`, but
    /// it may not be imported.
    Prelude,
}

#[derive(Clone, Debug)]
pub struct ModuleSource {
    /// The module path, as its own package names it (§2.1).
    pub path: String,
    pub source: SourceId,
    pub kind: ModuleKind,
}

#[derive(Clone, Debug)]
pub struct Package {
    pub name: String,
    /// The canonical directory. `None` for the library package `i`. For a
    /// package with a location, it is in the cache (see [`git`]).
    pub dir: Option<PathBuf>,
    pub manifest_source: Option<SourceId>,
    /// Dependency names and their packages. A package is `None` when it
    /// could not be loaded; the problem has been reported.
    pub dependencies: Vec<(String, Option<PackageId>)>,
    /// Only for the compiled package (§2.6). When its tests are not
    /// compiled ([`LoadOptions::tests`]), the names are here, but their
    /// packages are `None` and are not loaded.
    pub dev_dependencies: Vec<(String, Option<PackageId>)>,
    pub modules: Vec<ModuleSource>,
    pub wasm_exports: Vec<WasmExport>,
    pub entries: Vec<(String, WasmExport)>,
    /// For a package got by location: its location and version (§2.7).
    pub location: Option<PackageLocation>,
}

impl Package {
    /// What comes before the module path in a source position (§11.7):
    /// `location@major ` for a package with a location, and nothing
    /// otherwise.
    pub fn position_prefix(&self) -> String {
        self.location
            .as_ref()
            .map_or_else(String::new, PackageLocation::position_prefix)
    }
}

/// The version of a package with a location that a program uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageLocation {
    /// The location, with its `//dir` (§2.6).
    pub location: String,
    pub version: Version,
    /// The git commit of the tag (§2.8).
    pub commit: String,
    /// The hash of the files (§2.8).
    pub hash: String,
}

impl PackageLocation {
    /// The package (§2.7): the location and the major version.
    pub fn key(&self) -> PackageKey {
        PackageKey::new(self.location.clone(), self.version)
    }

    /// `location@major ` (§11.7), with the space.
    pub fn position_prefix(&self) -> String {
        format!("{}@{} ", self.location, self.version.major)
    }
}

/// The packages of a program: the library package `i`, the compiled package,
/// and its dependencies.
#[derive(Clone, Debug)]
pub struct PackageGraph {
    pub sources: Sources,
    pub packages: Vec<Package>,
    pub library: PackageId,
    /// `None` when the compiled package could not be read.
    pub root: Option<PackageId>,
    /// An explicit [entries] selection; None uses the source convention.
    pub entry: Option<String>,
}

/// How packages are loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadOptions {
    /// The tests of the compiled package are compiled: its test modules and
    /// its dev-dependencies are loaded, and its dev-dependencies take part
    /// in minimal version selection (§2.6, §2.7).
    pub tests: bool,
    /// The network may not be used. Something that is not in the cache is
    /// an error ([`code::OFFLINE`]).
    pub offline: bool,
    /// The cache directory (see [`git`]). When `None`, it is found from the
    /// environment when it is needed, with [`git::cache_dir`].
    pub cache: Option<PathBuf>,
    /// Rewrites from a prefix of a repository to a prefix of a URL (see
    /// [`git`]).
    pub git_urls: Vec<(String, String)>,
    /// `i.lock` is written when its lines change (§2.8).
    pub write_lock: bool,
}

impl Default for LoadOptions {
    /// With the tests, online, and writing `i.lock`.
    fn default() -> LoadOptions {
        LoadOptions {
            tests: true,
            offline: false,
            cache: None,
            git_urls: Vec::new(),
            write_lock: true,
        }
    }
}

impl LoadOptions {
    /// The defaults, with `ILANG_OFFLINE`, `ILANG_CACHE` (and the other
    /// variables of [`git::cache_dir`]) and `ILANG_GIT_URLS` from the
    /// environment.
    pub fn from_env() -> LoadOptions {
        LoadOptions::from_vars(|k| std::env::var_os(k))
    }

    /// As [`LoadOptions::from_env`], with the variables that `get` reads.
    /// `ILANG_OFFLINE` is on when it is set, and not empty or `0`.
    pub fn from_vars(get: impl Fn(&str) -> Option<std::ffi::OsString>) -> LoadOptions {
        let offline = get("ILANG_OFFLINE").is_some_and(|v| !v.is_empty() && v != "0");
        let urls = get("ILANG_GIT_URLS")
            .map(|v| git::parse_url_map(&v.to_string_lossy()))
            .unwrap_or_default();
        LoadOptions {
            offline,
            cache: git::cache_dir(&get),
            git_urls: urls,
            ..LoadOptions::default()
        }
    }
}

/// Loads the package in `root` and the packages it uses, with its tests,
/// and with the options from the environment ([`LoadOptions::from_env`]).
pub fn load(root: &Path) -> (PackageGraph, Vec<Diagnostic>) {
    load_with(root, &LoadOptions::from_env())
}

/// Loads the package in `root` and the packages it uses. Test modules and
/// dev-dependencies are loaded only for that package, and only with
/// [`LoadOptions::tests`] (§2.1, §2.6). Packages with a location are got
/// with git, chosen by minimal version selection, and checked against
/// `i.lock`, which is brought up to date (§2.7, §2.8).
pub fn load_with(root: &Path, options: &LoadOptions) -> (PackageGraph, Vec<Diagnostic>) {
    let mut loader = Loader {
        graph: PackageGraph {
            sources: Sources::default(),
            packages: Vec::new(),
            library: 0,
            root: None,
            entry: None,
        },
        diags: Vec::new(),
        by_dir: HashMap::new(),
        by_key: HashMap::new(),
        stack: Vec::new(),
        base: root.canonicalize().unwrap_or_else(|_| root.to_path_buf()),
        tests: options.tests,
        pending: Vec::new(),
        located: None,
    };
    library::add(&mut loader.graph);
    let root_id = loader.package(root, Kind::Root, None);
    loader.graph.root = root_id;
    if let Some(id) = root_id
        && let Some(dir) = loader.graph.packages[id].dir.clone()
    {
        let located = locations::resolve(&mut loader, &dir, options);
        loader.located = Some(located);
        for p in std::mem::take(&mut loader.pending) {
            let found = loader.located.as_ref().and_then(|l| l.get(&p.key)).cloned();
            let id = found
                .and_then(|l| loader.package(&l.dir, Kind::Location(l.location), Some(p.from)));
            let list = &mut loader.graph.packages[p.package];
            let list = if p.dev {
                &mut list.dev_dependencies
            } else {
                &mut list.dependencies
            };
            list[p.index].1 = id;
        }
    }
    (loader.graph, loader.diags)
}

/// A package with a location that a program uses, in the cache.
#[derive(Clone, Debug)]
pub(crate) struct Located {
    pub(crate) location: PackageLocation,
    pub(crate) dir: PathBuf,
}

/// A dependency by location of a package got by path, to be filled when
/// the versions are chosen.
struct Pending {
    package: PackageId,
    dev: bool,
    index: usize,
    key: PackageKey,
    from: (SourceId, Span),
}

#[derive(Clone, Debug)]
enum Kind {
    Root,
    Path,
    Location(PackageLocation),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Node {
    Dir(PathBuf),
    Key(PackageKey),
}

pub(crate) struct Loader {
    pub(crate) graph: PackageGraph,
    pub(crate) diags: Vec<Diagnostic>,
    pub(crate) by_dir: HashMap<PathBuf, PackageId>,
    by_key: HashMap<PackageKey, PackageId>,
    stack: Vec<Node>,
    /// Paths are shown relative to this directory.
    base: PathBuf,
    tests: bool,
    pending: Vec<Pending>,
    /// The packages with a location, once they are chosen.
    located: Option<HashMap<PackageKey, Located>>,
}

impl Loader {
    pub(crate) fn display(&self, path: &Path) -> String {
        relative(&self.base, path)
    }

    /// `from` is the dependency entry that leads here.
    fn package(
        &mut self,
        dir: &Path,
        kind: Kind,
        from: Option<(SourceId, Span)>,
    ) -> Option<PackageId> {
        let (from_source, from_span) =
            from.map_or((None, Span::default()), |(s, sp)| (Some(s), sp));
        if let Kind::Location(loc) = &kind {
            return self.location_package(dir, loc, from_source, from_span);
        }
        let is_root = matches!(kind, Kind::Root);
        let Ok(dir) = dir.canonicalize() else {
            let shown = self.display(dir);
            self.diags.push(Diagnostic::error(
                code::NO_PACKAGE,
                from_source,
                from_span,
                format!("there is no package directory `{shown}`"),
            ));
            return None;
        };
        if self.stack.contains(&Node::Dir(dir.clone())) {
            self.diags.push(Diagnostic::error(
                code::PACKAGE_CIRCLE,
                from_source,
                from_span,
                format!(
                    "this dependency leads back to the package in `{}`, so the packages are in a circle (§2.6)",
                    self.display(&dir)
                ),
            ));
            return None;
        }
        if let Some(&id) = self.by_dir.get(&dir) {
            return Some(id);
        }
        let toml_path = dir.join("i.toml");
        let Ok(text) = std::fs::read_to_string(&toml_path) else {
            self.diags.push(Diagnostic::error(
                code::NO_PACKAGE,
                from_source,
                from_span,
                format!("there is no `i.toml` in `{}`", self.display(&dir)),
            ));
            return None;
        };
        let display = self.display(&toml_path);
        let source = self.graph.sources.add(display, text);
        let text = &self.graph.sources.get(source).text;
        let (manifest, diags) = if is_root {
            manifest::parse_manifest(text)
        } else {
            manifest::parse_dependency_manifest(text)
        };
        self.diags.extend(diags.into_iter().map(|d| Diagnostic {
            source: Some(source),
            ..d
        }));
        let manifest = manifest?;
        let id = self.graph.packages.len();
        self.graph.packages.push(Package {
            name: manifest.name.clone(),
            dir: Some(dir.clone()),
            manifest_source: Some(source),
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
            modules: Vec::new(),
            wasm_exports: manifest.wasm_exports.clone(),
            entries: manifest.entries.clone(),
            location: None,
        });
        self.by_dir.insert(dir.clone(), id);
        self.stack.push(Node::Dir(dir.clone()));
        let deps = self.dependencies(id, &manifest, false, &dir, source);
        self.graph.packages[id].dependencies = deps;
        if is_root {
            let dev = if self.tests || !manifest.entries.is_empty() {
                self.dependencies(id, &manifest, true, &dir, source)
            } else {
                manifest
                    .dev_dependencies
                    .iter()
                    .map(|d| (d.name.clone(), None))
                    .collect()
            };
            self.graph.packages[id].dev_dependencies = dev;
        }
        self.stack.pop();
        let tests = is_root && (self.tests || !manifest.entries.is_empty());
        let (modules, diags) = modules::collect(self, &manifest.name, &dir, tests, None);
        self.graph.packages[id].modules = modules;
        self.diags.extend(diags);
        Some(id)
    }

    /// A package got by location, from its files in the cache.
    fn location_package(
        &mut self,
        dir: &Path,
        loc: &PackageLocation,
        from_source: Option<SourceId>,
        from_span: Span,
    ) -> Option<PackageId> {
        let key = loc.key();
        if self.stack.contains(&Node::Key(key.clone())) {
            self.diags.push(Diagnostic::error(
                code::PACKAGE_CIRCLE,
                from_source,
                from_span,
                format!(
                    "this dependency leads back to `{}@{}`, so the packages are in a circle (§2.6)",
                    loc.location, loc.version
                ),
            ));
            return None;
        }
        if let Some(&id) = self.by_key.get(&key) {
            return Some(id);
        }
        let shown = format!("{}@{}", loc.location, loc.version);
        let dir = &dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        let Ok(text) = std::fs::read_to_string(dir.join("i.toml")) else {
            self.diags.push(Diagnostic::error(
                code::NO_PACKAGE,
                from_source,
                from_span,
                format!("`{shown}` has no `i.toml`"),
            ));
            return None;
        };
        let source = self.graph.sources.add(format!("{shown}/i.toml"), text);
        let text = &self.graph.sources.get(source).text;
        let (manifest, diags) = manifest::parse_dependency_manifest(text);
        self.diags.extend(diags.into_iter().map(|d| Diagnostic {
            source: Some(source),
            ..d
        }));
        let manifest = manifest?;
        let id = self.graph.packages.len();
        self.graph.packages.push(Package {
            name: manifest.name.clone(),
            dir: Some(dir.to_path_buf()),
            manifest_source: Some(source),
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
            modules: Vec::new(),
            wasm_exports: Vec::new(),
            entries: Vec::new(),
            location: Some(loc.clone()),
        });
        self.by_key.insert(key.clone(), id);
        // §2.6: a package got by location may not have path dependencies.
        for d in manifest.check_path_dependencies(true) {
            self.diags.push(Diagnostic {
                source: Some(source),
                ..d
            });
        }
        self.stack.push(Node::Key(key));
        let deps = self.dependencies(id, &manifest, false, dir, source);
        self.graph.packages[id].dependencies = deps;
        self.stack.pop();
        let (modules, diags) = modules::collect(self, &manifest.name, dir, false, Some(&shown));
        self.graph.packages[id].modules = modules;
        self.diags.extend(diags);
        Some(id)
    }

    fn dependencies(
        &mut self,
        package: PackageId,
        manifest: &Manifest,
        dev: bool,
        dir: &Path,
        source: SourceId,
    ) -> Vec<(String, Option<PackageId>)> {
        let by_location = self.graph.packages[package].location.is_some();
        let list = if dev {
            &manifest.dev_dependencies
        } else {
            &manifest.dependencies
        };
        let mut out = Vec::new();
        for d in list {
            let id = match &d.source {
                DependencySource::Location(loc) => {
                    let key = PackageKey::new(loc.place(), loc.version);
                    match &self.located {
                        // The versions are chosen: a package got by location.
                        Some(located) => located.get(&key).cloned().and_then(|l| {
                            self.package(&l.dir, Kind::Location(l.location), Some((source, d.span)))
                        }),
                        // Filled when the versions are chosen.
                        None => {
                            self.pending.push(Pending {
                                package,
                                dev,
                                index: out.len(),
                                key,
                                from: (source, d.span),
                            });
                            None
                        }
                    }
                }
                // Reported by `check_path_dependencies`.
                DependencySource::Path(_) if by_location => None,
                DependencySource::Path(p) => {
                    self.package(&dir.join(p), Kind::Path, Some((source, d.span)))
                }
            };
            out.push((d.name.clone(), id));
        }
        out
    }
}

/// The path of `to` relative to the directory `from`, with `/`.
pub(crate) fn relative(from: &Path, to: &Path) -> String {
    let a: Vec<Component> = from.components().collect();
    let b: Vec<Component> = to.components().collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    if common == 0 {
        return to.display().to_string();
    }
    let mut parts: Vec<String> = vec!["..".to_string(); a.len() - common];
    parts.extend(
        b[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    if parts.is_empty() {
        return ".".to_string();
    }
    parts.join("/")
}
