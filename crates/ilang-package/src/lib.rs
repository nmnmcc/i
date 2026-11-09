//! Packages (§2.1, §2.6): the package file, the modules of a package, the
//! library package `i`, and the graph of packages that a program uses.
//!
//! The only authority is `docs/spec.md`; section numbers refer to it.

pub mod library;
pub mod manifest;
pub mod modules;

use ilang_syntax::span::{LineIndex, Span};
use manifest::{DependencySource, Manifest, WasmExport};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Component, Path, PathBuf};

/// The stable codes of the problems found while loading packages.
pub mod code {
    pub const MANIFEST_SYNTAX: &str = "E0601";
    pub const MANIFEST_NAME: &str = "E0602";
    pub const MANIFEST_SHAPE: &str = "E0603";
    pub const DEPENDENCY_NAME: &str = "E0604";
    pub const DEPENDENCY_TWICE: &str = "E0605";
    pub const DEPENDENCY_VALUE: &str = "E0606";
    pub const BAD_LOCATION: &str = "E0607";
    pub const BAD_VERSION: &str = "E0608";
    pub const PATH_DEPENDENCY: &str = "E0609";
    pub const WASM_EXPORT: &str = "E0610";
    pub const LOCATION_NOT_SUPPORTED: &str = "E0620";
    pub const NO_PACKAGE: &str = "E0621";
    pub const PACKAGE_CIRCLE: &str = "E0622";
    pub const MODULE_PATH_NAME: &str = "E0623";
    pub const MODULE_TWICE: &str = "E0624";
    pub const UNREADABLE: &str = "E0625";
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Error,
    Warning,
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
        };
        let _ = write!(out, "{kind}[{}]: {}", d.code, d.message);
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

/// `[ "_" ] lower { lower | digit | "_" }`, and not a keyword (§1.3, §1.4).
pub fn is_name(s: &str) -> bool {
    let rest = s.strip_prefix('_').unwrap_or(s);
    let mut chars = rest.chars();
    matches!(chars.next(), Some('a'..='z'))
        && chars.all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_'))
        && !matches!(s, "import" | "export" | "case" | "if")
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
    /// The canonical directory. `None` for the library package `i`.
    pub dir: Option<PathBuf>,
    pub manifest_source: Option<SourceId>,
    /// Dependency names and their packages. A package is `None` when it
    /// could not be loaded; the problem has been reported.
    pub dependencies: Vec<(String, Option<PackageId>)>,
    /// Only for the compiled package (§2.6).
    pub dev_dependencies: Vec<(String, Option<PackageId>)>,
    pub modules: Vec<ModuleSource>,
    pub wasm_exports: Vec<WasmExport>,
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
}

/// Loads the package in `root` and the packages it uses. Test modules and
/// dev-dependencies are loaded only for that package (§2.1, §2.6).
pub fn load(root: &Path) -> (PackageGraph, Vec<Diagnostic>) {
    let mut loader = Loader {
        graph: PackageGraph {
            sources: Sources::default(),
            packages: Vec::new(),
            library: 0,
            root: None,
        },
        diags: Vec::new(),
        by_dir: HashMap::new(),
        stack: Vec::new(),
        base: root.canonicalize().unwrap_or_else(|_| root.to_path_buf()),
    };
    library::add(&mut loader.graph);
    let root = loader.package(root, true, None);
    loader.graph.root = root;
    (loader.graph, loader.diags)
}

struct Loader {
    graph: PackageGraph,
    diags: Vec<Diagnostic>,
    by_dir: HashMap<PathBuf, PackageId>,
    stack: Vec<PathBuf>,
    /// Paths are shown relative to this directory.
    base: PathBuf,
}

impl Loader {
    fn display(&self, path: &Path) -> String {
        relative(&self.base, path)
    }

    /// `from` is the dependency entry that leads here.
    fn package(
        &mut self,
        dir: &Path,
        is_root: bool,
        from: Option<(SourceId, Span)>,
    ) -> Option<PackageId> {
        let (from_source, from_span) =
            from.map_or((None, Span::default()), |(s, sp)| (Some(s), sp));
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
        if self.stack.contains(&dir) {
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
        });
        self.by_dir.insert(dir.clone(), id);
        self.stack.push(dir.clone());
        // Packages got by location are not loaded yet, so every package
        // here is the compiled package or got by path.
        for d in manifest.check_path_dependencies(false) {
            self.diags.push(Diagnostic {
                source: Some(source),
                ..d
            });
        }
        let deps = self.dependencies(&manifest, false, &dir, source);
        self.graph.packages[id].dependencies = deps;
        if is_root {
            let dev = self.dependencies(&manifest, true, &dir, source);
            self.graph.packages[id].dev_dependencies = dev;
        }
        self.stack.pop();
        let (modules, diags) = modules::collect(self, &manifest.name, &dir, is_root);
        self.graph.packages[id].modules = modules;
        self.diags.extend(diags);
        Some(id)
    }

    fn dependencies(
        &mut self,
        manifest: &Manifest,
        dev: bool,
        dir: &Path,
        source: SourceId,
    ) -> Vec<(String, Option<PackageId>)> {
        let list = if dev {
            &manifest.dev_dependencies
        } else {
            &manifest.dependencies
        };
        let mut out = Vec::new();
        for d in list {
            let id = match &d.source {
                DependencySource::Location(_) => {
                    self.diags.push(Diagnostic::error(
                        code::LOCATION_NOT_SUPPORTED,
                        Some(source),
                        d.span,
                        format!(
                            "`{}`: getting packages by location is not supported yet; use `{{ path = \"…\" }}`",
                            d.name
                        ),
                    ));
                    None
                }
                DependencySource::Path(p) => {
                    self.package(&dir.join(p), false, Some((source, d.span)))
                }
            };
            out.push((d.name.clone(), id));
        }
        out
    }
}

/// The path of `to` relative to the directory `from`, with `/`.
fn relative(from: &Path, to: &Path) -> String {
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
