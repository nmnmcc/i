//! The package file `i.toml` (§2.6).

use crate::{Diagnostic, code, is_name};
use ilang_syntax::span::Span;
use std::collections::HashSet;
use toml::Spanned;
use toml::de::{DeTable, DeValue};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub provider_plan: crate::providers::ProviderPlan,
    pub wasm_synchronous: Vec<WasmExport>,
    pub name: String,
    pub name_span: Span,
    pub dependencies: Vec<Dependency>,
    pub dev_dependencies: Vec<Dependency>,
    pub wasm_exports: Vec<WasmExport>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dependency {
    /// The dependency name: the key (§2.6).
    pub name: String,
    pub source: DependencySource,
    /// The span of the key.
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DependencySource {
    Location(Location),
    /// `{ path = "…" }`, relative to the directory of the `i.toml`.
    Path(String),
}

/// `github.com/alice/json@1.4.0` or `codeberg.org/bob/tools//text@0.3.2`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub repository: String,
    pub dir: Option<String>,
    pub version: Version,
}

impl Location {
    /// The location without the version: the repository, then `//` and the
    /// directory when there is one. This is the location of `i.lock` (§2.8)
    /// and of source positions (§11.7).
    pub fn place(&self) -> String {
        match &self.dir {
            Some(d) => format!("{}//{d}", self.repository),
            None => self.repository.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

/// `major.minor.patch`.
impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// `"app/math.add"` in `[wasm] exports` (§2.6, §10.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WasmExport {
    pub module: String,
    pub function: String,
    pub span: Span,
}

fn span(r: std::ops::Range<usize>) -> Span {
    Span::new(r.start as u32, r.end as u32)
}

fn err(code: &'static str, span: Span, message: impl Into<String>) -> Diagnostic {
    Diagnostic::error(code, None, span, message)
}

/// Reads the `i.toml` of the package that is compiled. The manifest is
/// `None` only when there is no usable `name`. Other problems are reported,
/// and the bad entries are left out. Unknown keys are not used (§2.6).
pub fn parse_manifest(text: &str) -> (Option<Manifest>, Vec<Diagnostic>) {
    parse(text, true)
}

/// Reads the `i.toml` of a package that the compiled package uses. Its
/// `[dev-dependencies]` and `[wasm]` are not used (§2.6, §10.2), so they
/// are not read, and their shapes give no errors. They are empty in the
/// manifest.
pub fn parse_dependency_manifest(text: &str) -> (Option<Manifest>, Vec<Diagnostic>) {
    parse(text, false)
}

fn parse(text: &str, compiled: bool) -> (Option<Manifest>, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let doc = match DeTable::parse(text) {
        Ok(doc) => doc,
        Err(e) => {
            let at = e.span().map_or(Span::default(), span);
            let message = format!("`i.toml` is not correct TOML: {}", e.message().trim());
            diags.push(err(code::MANIFEST_SYNTAX, at, message));
            return (None, diags);
        }
    };
    let mut name = None;
    let mut deps = Vec::new();
    let mut dev_deps = Vec::new();
    let mut wasm_exports = Vec::new();
    let mut wasm_synchronous = Vec::new();
    let mut provider_plan = crate::providers::ProviderPlan::default();
    for (key, value) in doc.get_ref().iter() {
        match key.get_ref().as_ref() {
            "providers" => {
                provider_plan.span = Some(span(key.span()));
                crate::providers::rows(value, &mut provider_plan, &mut diags);
            }
            "provider_adapters" => {
                provider_plan.span = Some(span(key.span()));
                crate::providers::adapters(value, &mut provider_plan, &mut diags);
            }
            "name" => match value.get_ref() {
                DeValue::String(s) => name = Some((s.to_string(), span(value.span()))),
                _ => diags.push(err(
                    code::MANIFEST_NAME,
                    span(value.span()),
                    "`name` must be a string",
                )),
            },
            "dependencies" => deps = dependencies("dependencies", value, &mut diags),
            "dev-dependencies" if compiled => {
                dev_deps = dependencies("dev-dependencies", value, &mut diags)
            }
            "wasm" if compiled => {
                wasm_exports = wasm(value, "exports", &mut diags);
                if matches!(value.get_ref(), DeValue::Table(_)) { wasm_synchronous = wasm(value, "synchronous", &mut diags); }
            }
            _ => {}
        }
    }
    let Some((name, name_span)) = name else {
        if !diags.iter().any(|d| d.code == code::MANIFEST_NAME) {
            diags.push(err(
                code::MANIFEST_NAME,
                Span::default(),
                "`name` is required",
            ));
        }
        return (None, diags);
    };
    if !is_name(&name) || name == "i" {
        diags.push(err(
            code::MANIFEST_NAME,
            name_span,
            format!("the package name `{name}` must be a name (§1.3), and not `i`"),
        ));
    }
    // §2.6: the dependency names.
    let mut seen = HashSet::new();
    let mut keep = |list: Vec<Dependency>, diags: &mut Vec<Diagnostic>| {
        let mut out = Vec::new();
        for d in list {
            if !is_name(&d.name) || d.name == "i" || d.name == name {
                diags.push(err(
                    code::DEPENDENCY_NAME,
                    d.span,
                    format!(
                        "the dependency name `{}` must be a name, not `i`, and not the name of this package",
                        d.name
                    ),
                ));
            } else if !seen.insert(d.name.clone()) {
                diags.push(err(
                    code::DEPENDENCY_TWICE,
                    d.span,
                    format!("the dependency name `{}` is given more than once", d.name),
                ));
            } else {
                out.push(d);
            }
        }
        out
    };
    let dependencies = keep(deps, &mut diags);
    let dev_dependencies = keep(dev_deps, &mut diags);
    for e in &wasm_synchronous {
        if !wasm_exports
            .iter()
            .any(|x| x.module == e.module && x.function == e.function)
        {
            diags.push(err(
                code::WASM_EXPORT,
                e.span,
                "wasm.synchronous must be a subset of wasm.exports",
            ));
        }
    }
    let manifest = Manifest {
        provider_plan,
        wasm_synchronous,
        name,
        name_span,
        dependencies,
        dev_dependencies,
        wasm_exports,
    };
    (Some(manifest), diags)
}

impl Manifest {
    /// §2.6: only the compiled package, and the packages that it gets by
    /// path, may have path dependencies.
    pub fn check_path_dependencies(&self, by_location: bool) -> Vec<Diagnostic> {
        if !by_location {
            return Vec::new();
        }
        self.dependencies
            .iter()
            .chain(&self.dev_dependencies)
            .filter(|d| matches!(d.source, DependencySource::Path(_)))
            .map(|d| {
                err(
                    code::PATH_DEPENDENCY,
                    d.span,
                    format!(
                        "`{}` is a path dependency, but this package is got by location; only the compiled package and the packages it gets by path may have path dependencies",
                        d.name
                    ),
                )
            })
            .collect()
    }
}

fn dependencies(
    table_name: &str,
    value: &Spanned<DeValue>,
    diags: &mut Vec<Diagnostic>,
) -> Vec<Dependency> {
    let DeValue::Table(table) = value.get_ref() else {
        diags.push(err(
            code::MANIFEST_SHAPE,
            span(value.span()),
            format!("`{table_name}` must be a table"),
        ));
        return Vec::new();
    };
    let mut out = Vec::new();
    for (key, value) in table.iter() {
        let name = key.get_ref().to_string();
        let key_span = span(key.span());
        let source = match value.get_ref() {
            DeValue::String(s) => match parse_location(s) {
                Ok(loc) => DependencySource::Location(loc),
                Err((code, message)) => {
                    diags.push(err(code, span(value.span()), message));
                    continue;
                }
            },
            DeValue::Table(t) => match path_of(t) {
                Some(p) => DependencySource::Path(p),
                None => {
                    diags.push(err(
                        code::DEPENDENCY_VALUE,
                        span(value.span()),
                        "a path dependency is written `{ path = \"…\" }`",
                    ));
                    continue;
                }
            },
            _ => {
                diags.push(err(
                    code::DEPENDENCY_VALUE,
                    span(value.span()),
                    "a dependency is a location string, as in \"github.com/alice/json@1.4.0\", or `{ path = \"…\" }`",
                ));
                continue;
            }
        };
        out.push(Dependency {
            name,
            source,
            span: key_span,
        });
    }
    out
}

fn path_of(t: &DeTable) -> Option<String> {
    t.iter()
        .find_map(|(k, v)| match (k.get_ref().as_ref(), v.get_ref()) {
            ("path", DeValue::String(s)) if !s.is_empty() => Some(s.to_string()),
            _ => None,
        })
}

fn wasm(value: &Spanned<DeValue>, field: &str, diags: &mut Vec<Diagnostic>) -> Vec<WasmExport> {
    let DeValue::Table(table) = value.get_ref() else {
        diags.push(err(
            code::MANIFEST_SHAPE,
            span(value.span()),
            "`wasm` must be a table",
        ));
        return Vec::new();
    };
    let mut out = Vec::new();
    for (key, value) in table.iter() {
        if key.get_ref().as_ref() != field {
            continue;
        }
        let DeValue::Array(items) = value.get_ref() else {
            diags.push(err(
                code::WASM_EXPORT,
                span(value.span()),
                "`wasm.exports` must be a list of strings",
            ));
            continue;
        };
        for item in items.iter() {
            let at = span(item.span());
            let parsed = match item.get_ref() {
                DeValue::String(s) => parse_wasm_export(s),
                _ => None,
            };
            match parsed {
                Some((module, function)) => out.push(WasmExport {
                    module,
                    function,
                    span: at,
                }),
                None => diags.push(err(
                    code::WASM_EXPORT,
                    at,
                    "a host export is written \"module/path.function\", as in \"app/math.add\"",
                )),
            }
        }
    }
    out
}

/// `app/math.add` into `("app/math", "add")`.
fn parse_wasm_export(s: &str) -> Option<(String, String)> {
    let (module, function) = s.rsplit_once('.')?;
    let parts: Vec<&str> = module.split('/').collect();
    (parts.len() >= 2 && parts.iter().all(|p| is_name(p)) && is_name(function))
        .then(|| (module.to_string(), function.to_string()))
}

/// Parses a location with its version (§2.6): a git repository address
/// without `https://`, an optional `//dir`, then `@major.minor.patch`.
pub fn parse_location(s: &str) -> Result<Location, (&'static str, String)> {
    let bad = |why: &str| {
        Err((
            code::BAD_LOCATION,
            format!("`{s}` is not a location: {why}"),
        ))
    };
    let Some((place, version)) = s.rsplit_once('@') else {
        return bad("it needs `@` and the lowest version, as in \"github.com/alice/json@1.4.0\"");
    };
    let version = parse_version(version).ok_or((
        code::BAD_VERSION,
        format!("`{version}` is not a version `major.minor.patch` of three decimal numbers"),
    ))?;
    if place.contains("://") {
        return bad("it is written without `https://`");
    }
    let (repository, dir) = match place.split_once("//") {
        Some((r, d)) => (r, Some(d)),
        None => (place, None),
    };
    let good_part = |p: &str| {
        !p.is_empty()
            && p != "."
            && p != ".."
            && p.chars()
                .all(|c| !c.is_whitespace() && !c.is_control() && !"@\\:".contains(c))
    };
    let parts: Vec<&str> = repository.split('/').collect();
    if parts.len() < 2 || !parts.iter().all(|p| good_part(p)) {
        return bad("the repository is a host and a path, as in `github.com/alice/json`");
    }
    if let Some(d) = dir
        && !d.split('/').all(good_part)
    {
        return bad("the directory after `//` is a path, as in `tools//text`");
    }
    Ok(Location {
        repository: repository.to_string(),
        dir: dir.map(str::to_string),
        version,
    })
}

/// `major.minor.patch`: three decimal numbers, with no leading zeros, as in
/// Semantic Versioning.
pub fn parse_version(s: &str) -> Option<Version> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let mut n = [0u64; 3];
    for (i, p) in parts.iter().enumerate() {
        if p.is_empty()
            || !p.bytes().all(|b| b.is_ascii_digit())
            || (p.len() > 1 && p.starts_with('0'))
        {
            return None;
        }
        n[i] = p.parse().ok()?;
    }
    Some(Version {
        major: n[0],
        minor: n[1],
        patch: n[2],
    })
}
