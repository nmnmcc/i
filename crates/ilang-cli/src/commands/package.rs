//! The commands for packages (§2.6–§2.8): `i new`, `i add`, `i remove`,
//! `i update`, `i publish`, and `i deps`.
//!
//! These are tools, not rules of the language (§0). `i.toml` is changed with
//! `toml_edit`, so its formatting and comments are kept, and then the
//! package is loaded, which brings `i.lock` up to date (§2.8). When loading
//! fails, `i.toml` and `i.lock` are both put back as they were.
//!
//! `i add` and `i update` print the host services and external functions
//! that the new versions can use and the old ones could not, as
//! `new permission: …`; `i deps` prints them all. A package can use what
//! its modules declare, and the host services of the library modules that
//! they import: a module that imports `i/io` can print (§9.7).

use anyhow::{Context, Result, anyhow, bail};
use clap::Subcommand;
use ilang_check::hir::{self, DefKind, TypeBody, TypeKind, TypeRef};
use ilang_check::identity::{PackageIdentity, type_identity};
use ilang_check::types::ShownType;
use ilang_package::git::Git;
use ilang_package::manifest::{self, DependencySource, Version, parse_location, parse_version};
use ilang_package::{Diagnostic, LoadOptions, ModuleKind, PackageGraph, PackageId, is_name};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use toml_edit::{DocumentMut, InlineTable, Item, TableLike, Value};

#[derive(Subcommand)]
pub enum PackageCommand {
    /// Make a new package, with a program and a test.
    New {
        /// The name of the package (§2.6).
        name: String,
        /// The directory to make; by default `NAME`.
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Add a dependency to `i.toml`, and bring `i.lock` up to date.
    Add {
        /// The dependency name (§2.6).
        name: String,
        /// `location` or `location@version`. With no version, the newest
        /// tag `vX.Y.Z` of the repository is used.
        #[arg(required_unless_present = "path", conflicts_with = "path")]
        location: Option<String>,
        /// A package in a directory on this computer, in place of a location.
        #[arg(long)]
        path: Option<PathBuf>,
        /// Add it to `[dev-dependencies]`: only the tests use it.
        #[arg(long)]
        dev: bool,
        /// The package directory, with its `i.toml`.
        #[arg(long, default_value = ".")]
        project: PathBuf,
    },
    /// Take a dependency out of `i.toml`, and bring `i.lock` up to date.
    Remove {
        /// The dependency name.
        name: String,
        #[arg(long, default_value = ".")]
        project: PathBuf,
    },
    /// Raise dependencies to the newest version with the same major version
    /// (§2.7), and bring `i.lock` up to date.
    Update {
        /// The dependencies to raise; by default all of them.
        names: Vec<String>,
        #[arg(long, default_value = ".")]
        project: PathBuf,
    },
    /// Check the package and tag a new version `vX.Y.Z` in git. Nothing is
    /// pushed.
    Publish {
        /// The version, as `1.4.0`.
        version: String,
        /// Do every check, but make no tag.
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = ".")]
        project: PathBuf,
    },
    /// Print the packages that the program uses, and the host services and
    /// external functions that each one can use.
    Deps {
        #[arg(long, default_value = ".")]
        project: PathBuf,
    },
}

/// Runs a package command.
pub fn run(command: PackageCommand) -> Result<ExitCode> {
    match command {
        PackageCommand::New { name, path } => new(&name, path),
        PackageCommand::Add {
            name,
            location,
            path,
            dev,
            project,
        } => add(&project, &name, location.as_deref(), path.as_deref(), dev),
        PackageCommand::Remove { name, project } => remove(&project, &name),
        PackageCommand::Update { names, project } => update(&project, &names),
        PackageCommand::Publish {
            version,
            dry_run,
            project,
        } => publish(&project, &version, dry_run),
        PackageCommand::Deps { project } => deps(&project),
    }
}

// ---- i new ----

fn new(name: &str, path: Option<PathBuf>) -> Result<ExitCode> {
    if !is_name(name) || name == "i" {
        bail!(
            "the package name `{name}` must be a name (§1.3): a lowercase letter, then lowercase letters, digits and `_`; and it may not be `i` or a keyword"
        );
    }
    let dir = path.unwrap_or_else(|| PathBuf::from(name));
    if dir.exists() && std::fs::read_dir(&dir)?.next().is_some() {
        bail!("`{}` is already there and is not empty", dir.display());
    }
    let files = [
        ("i.toml".to_string(), format!("name = \"{name}\"\n")),
        (".gitignore".to_string(), "/build/\n".to_string()),
        ("src/main.i".to_string(), formatted(main_module(name))),
        ("test/main_test.i".to_string(), formatted(test_module(name))),
    ];
    for (file, text) in &files {
        let p = dir.join(file);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&p, text)
            .with_context(|| format!("`{}` could not be written", p.display()))?;
    }
    println!("made the package `{name}` in `{}`", dir.display());
    println!("run it with `i run --project {}`", dir.display());
    Ok(ExitCode::SUCCESS)
}

/// The source as `i fmt` writes it, so that `i fmt --check` passes on a new
/// package.
fn formatted(source: String) -> String {
    ilang_syntax::format::format(&source).unwrap_or(source)
}

fn main_module(name: &str) -> String {
    format!(
        r#"//// The package `{name}`. It imports the standard service `Io` from `i/io`
//// (§9.7), and `i run` gives it, so the program can print.

((Io) = i/io)

(greeting)

/// The greeting for `who`.
greeting = (who: String) -> String {{ "Hello, " <> who <> "!" }}

@main
main = (io ! Io) {{
  io.print(greeting("{name}"))
}}
"#
    )
}

fn test_module(name: &str) -> String {
    format!(
        r#"({name}/main)

@test
greets = () {{
  main.greeting("i") |> {{
    "Hello, i!" -> Nil
    other -> panic(other)
  }}
}}
"#
    )
}

// ---- Loading ----

/// Loads the package in `project` with its tests, so that its
/// dev-dependencies are loaded too, with the options of the environment.
fn load(project: &Path, write_lock: bool) -> (PackageGraph, Vec<Diagnostic>) {
    ilang_package::load_with(
        project,
        &LoadOptions {
            tests: true,
            write_lock,
            ..LoadOptions::from_env()
        },
    )
}

/// The rendered problems, as an error.
fn problems(graph: &PackageGraph, diags: &[Diagnostic]) -> anyhow::Error {
    let errors = diags.iter().filter(|d| d.is_error()).count();
    let mut lines = graph.sources.render_all(diags);
    lines.push(format!(
        "{errors} error{}",
        if errors == 1 { "" } else { "s" }
    ));
    anyhow!(lines.join("\n"))
}

/// Git for the cache and the URLs of the environment.
fn git() -> Result<Git> {
    let options = LoadOptions::from_env();
    let cache = options.cache.ok_or_else(|| {
        anyhow!("there is no cache directory for packages got by location; set `ILANG_CACHE`")
    })?;
    Ok(Git::new(cache, options.offline, options.git_urls))
}

/// The versions of the tags `vX.Y.Z` of `repository`, newest last. The
/// repository is fetched first, unless offline.
fn fetched_tags(git: &mut Git, repository: &str) -> Result<Vec<Version>> {
    if !LoadOptions::from_env().offline {
        git.fetch(repository).map_err(|e| anyhow!(e.message))?;
    }
    git.tags(repository).map_err(|e| anyhow!(e.message))
}

// ---- i.toml ----

/// The `i.toml` of a package, to be changed.
struct PackageFile {
    path: PathBuf,
    /// The text as it was read, to put back when something fails.
    text: String,
    /// `i.lock` as it was before, or `None` when there was none, to put
    /// back when something fails: loading may write it before it finds a
    /// problem.
    lock: Option<Vec<u8>>,
    doc: DocumentMut,
    manifest: manifest::Manifest,
}

const TABLES: [&str; 2] = ["dependencies", "dev-dependencies"];

impl PackageFile {
    fn read(project: &Path) -> Result<PackageFile> {
        let path = project.join("i.toml");
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("there is no `i.toml` in `{}`", project.display()))?;
        let doc: DocumentMut = text
            .parse()
            .with_context(|| format!("`{}` is not correct TOML", path.display()))?;
        let (manifest, diags) = manifest::parse_manifest(&text);
        let Some(manifest) = manifest else {
            let why: Vec<String> = diags.iter().map(|d| d.message.clone()).collect();
            bail!("`{}` is not correct: {}", path.display(), why.join("; "));
        };
        let lock = std::fs::read(project.join("i.lock")).ok();
        Ok(PackageFile {
            path,
            text,
            lock,
            doc,
            manifest,
        })
    }

    /// The table `[dependencies]` or `[dev-dependencies]`, made when it is
    /// not there.
    fn table(&mut self, dev: bool) -> Result<&mut dyn TableLike> {
        let key = TABLES[dev as usize];
        self.doc
            .entry(key)
            .or_insert(toml_edit::table())
            .as_table_like_mut()
            .ok_or_else(|| anyhow!("`{key}` in `i.toml` must be a table"))
    }

    /// Which table has the dependency `name`: `Some(false)` for
    /// `[dependencies]`, `Some(true)` for `[dev-dependencies]`.
    fn find(&self, name: &str) -> Option<bool> {
        TABLES.iter().enumerate().find_map(|(i, key)| {
            self.doc
                .get(key)
                .and_then(Item::as_table_like)
                .filter(|t| t.contains_key(name))
                .map(|_| i == 1)
        })
    }

    /// Sets the dependency `name` in its table to `value`, keeping the
    /// comments around an entry that was there: the key, with the comments
    /// above it, stays, and the value keeps its trailing comment. An entry
    /// written as its own table is changed by [`set_table`].
    fn set(&mut self, dev: bool, name: &str, mut value: Value) -> Result<()> {
        if let Some(Item::Table(parent)) = self.doc.get_mut(TABLES[dev as usize])
            && matches!(parent.get(name), Some(Item::Table(_)))
        {
            set_table(parent, name, value);
            return Ok(());
        }
        let table = self.table(dev)?;
        match table.get_mut(name) {
            Some(old) => {
                if let Some(v) = old.as_value() {
                    *value.decor_mut() = v.decor().clone();
                }
                *old = Item::Value(value);
            }
            None => {
                table.insert(name, Item::Value(value));
            }
        }
        Ok(())
    }

    /// The changed text, with the line ends of the file as it was read.
    fn changed_text(&self) -> String {
        let text = self.doc.to_string();
        if self.text.contains("\r\n") {
            text.replace("\r\n", "\n").replace('\n', "\r\n")
        } else {
            text
        }
    }

    fn write(&self) -> Result<()> {
        std::fs::write(&self.path, self.changed_text())
            .with_context(|| format!("`{}` could not be written", self.path.display()))
    }

    /// Puts `i.toml` and `i.lock` back as they were read.
    fn restore(&self) {
        let _ = std::fs::write(&self.path, &self.text);
        let lock = self.path.with_file_name("i.lock");
        let _ = match &self.lock {
            Some(bytes) => std::fs::write(&lock, bytes),
            None => std::fs::remove_file(&lock),
        };
    }
}

/// Sets the dependency `name` of `parent`, written as its own table
/// `[dependencies.name]`, to `value`, keeping its comments. A path stays
/// that table, with its header and the comments of its entries. A location
/// is a string, so it becomes an entry `name = "…"`: when `parent` has no
/// header of its own, it takes the header's place and its comments;
/// otherwise the comments above the header go above the entry.
fn set_table(parent: &mut toml_edit::Table, name: &str, mut value: Value) {
    let Some(Item::Table(old)) = parent.get_mut(name) else {
        return;
    };
    if let Value::InlineTable(new) = value {
        let gone: Vec<String> = old
            .iter()
            .map(|(k, _)| k.to_string())
            .filter(|k| !new.contains_key(k))
            .collect();
        for k in gone {
            old.remove(&k);
        }
        for (k, mut v) in new {
            match old.get(&k).and_then(Item::as_value) {
                Some(o) => *v.decor_mut() = o.decor().clone(),
                None => v.decor_mut().clear(),
            }
            old.insert(&k, Item::Value(v));
        }
        return;
    }
    let decor = old.decor().clone();
    let position = old.position();
    // The key keeps its place in `parent`, but not the format it had in
    // the header.
    if parent.is_implicit() {
        parent.set_implicit(false);
        *parent.decor_mut() = decor;
        parent.set_position(position);
        parent.insert(name, Item::Value(value));
        if let Some(mut key) = parent.key_mut(name) {
            key.leaf_decor_mut().clear();
        }
        return;
    }
    let text = |raw: Option<&toml_edit::RawString>| {
        raw.and_then(|r| r.as_str()).unwrap_or_default().to_string()
    };
    let comments = text(decor.prefix());
    let comments = comments.trim_start_matches(['\n', '\r', ' ', '\t']);
    let after = text(decor.suffix());
    if after.trim_start().starts_with('#') {
        value.decor_mut().set_suffix(after);
    }
    parent.insert(name, Item::Value(value));
    if let Some(mut key) = parent.key_mut(name) {
        key.leaf_decor_mut().clear();
        if !comments.is_empty() {
            key.leaf_decor_mut().set_prefix(comments);
        }
    }
}

/// Writes the changed `i.toml`, loads the package, which brings `i.lock` up
/// to date, and prints what is new in the permissions compared to `before`.
/// When loading fails, `i.toml` is put back.
fn write_and_load(project: &Path, file: &PackageFile, before: &Report) -> Result<()> {
    file.write()?;
    let (graph, diags) = load(project, true);
    if diags.iter().any(Diagnostic::is_error) || graph.root.is_none() {
        file.restore();
        return Err(problems(&graph, &diags).context(
            "the package could not be loaded, so `i.toml` and `i.lock` were not changed",
        ));
    }
    match report(&graph) {
        Ok(after) => print_new_permissions(before, &after),
        Err(e) => eprintln!("warning: the permissions could not be listed: {e:#}"),
    }
    Ok(())
}

/// The report of the package as it is now, before a change. Empty when it
/// cannot be made, so that everything is shown as new after the change.
fn report_before(project: &Path) -> Report {
    let (graph, diags) = load(project, false);
    if diags.iter().any(Diagnostic::is_error) {
        return Report::default();
    }
    report(&graph).unwrap_or_default()
}

// ---- i add ----

fn add(
    project: &Path,
    name: &str,
    location: Option<&str>,
    path: Option<&Path>,
    dev: bool,
) -> Result<ExitCode> {
    let mut file = PackageFile::read(project)?;
    // §2.6: the dependency names.
    if !is_name(name) || name == "i" || name == file.manifest.name {
        bail!(
            "the dependency name `{name}` must be a name (§1.3), not `i`, and not the name of this package (§2.6)"
        );
    }
    if let Some(other) = file.find(name)
        && other != dev
    {
        bail!(
            "`{name}` is already in `[{}]`; a dependency name and a dev-dependency name may not be the same (§2.6)",
            TABLES[other as usize]
        );
    }
    let before = report_before(project);
    let (value, shown) = match (location, path) {
        (Some(location), _) => {
            let full = match location.rsplit_once('@') {
                Some(_) => location.to_string(),
                None => {
                    let place = parse_location(&format!("{location}@0.0.0"))
                        .map_err(|(_, m)| anyhow!(m))?;
                    let tags = fetched_tags(&mut git()?, &place.repository)?;
                    let newest = tags.last().ok_or_else(|| {
                        anyhow!(
                            "`{}` has no tag `vX.Y.Z`, so it has no version (§2.7)",
                            place.repository
                        )
                    })?;
                    format!("{location}@{newest}")
                }
            };
            parse_location(&full).map_err(|(_, m)| anyhow!(m))?;
            (Value::from(full.clone()), full)
        }
        (None, Some(path)) => {
            let target = path
                .canonicalize()
                .with_context(|| format!("there is no directory `{}`", path.display()))?;
            if !target.join("i.toml").is_file() {
                bail!("there is no `i.toml` in `{}`", path.display());
            }
            let base = project.canonicalize()?;
            let rel = relative(&base, &target);
            let mut t = InlineTable::new();
            t.insert("path", Value::from(rel.clone()));
            (Value::InlineTable(t), format!("{{ path = \"{rel}\" }}"))
        }
        (None, None) => bail!("give a location, or `--path`"),
    };
    file.set(dev, name, value)?;
    write_and_load(project, &file, &before)?;
    println!("added `{name} = {shown}` to [{}]", TABLES[dev as usize]);
    Ok(ExitCode::SUCCESS)
}

// ---- i remove ----

fn remove(project: &Path, name: &str) -> Result<ExitCode> {
    let mut file = PackageFile::read(project)?;
    let Some(dev) = file.find(name) else {
        bail!("`{name}` is not a dependency or a dev-dependency of this package");
    };
    file.table(dev)?.remove(name);
    file.write()?;
    let (graph, diags) = load(project, true);
    if diags.iter().any(Diagnostic::is_error) || graph.root.is_none() {
        file.restore();
        return Err(problems(&graph, &diags).context(
            "the package could not be loaded, so `i.toml` and `i.lock` were not changed",
        ));
    }
    println!("removed `{name}` from [{}]", TABLES[dev as usize]);
    Ok(ExitCode::SUCCESS)
}

// ---- i update ----

fn update(project: &Path, names: &[String]) -> Result<ExitCode> {
    let mut file = PackageFile::read(project)?;
    let all: Vec<(bool, manifest::Dependency)> = file
        .manifest
        .dependencies
        .iter()
        .map(|d| (false, d.clone()))
        .chain(
            file.manifest
                .dev_dependencies
                .iter()
                .map(|d| (true, d.clone())),
        )
        .collect();
    for n in names {
        if !all.iter().any(|(_, d)| &d.name == n) {
            bail!("`{n}` is not a dependency or a dev-dependency of this package");
        }
    }
    let before = report_before(project);
    let mut git = git()?;
    let mut fetched: HashSet<String> = HashSet::new();
    let mut changes = Vec::new();
    for (dev, d) in &all {
        if !names.is_empty() && !names.contains(&d.name) {
            continue;
        }
        let DependencySource::Location(loc) = &d.source else {
            if names.contains(&d.name) {
                println!("`{}` is a path dependency; it has no versions", d.name);
            }
            continue;
        };
        let tags = if fetched.insert(loc.repository.clone()) {
            fetched_tags(&mut git, &loc.repository)?
        } else {
            git.tags(&loc.repository).map_err(|e| anyhow!(e.message))?
        };
        // §2.7: a new major version is another package.
        let Some(&newest) = tags
            .iter()
            .filter(|v| v.major == loc.version.major && **v > loc.version)
            .max()
        else {
            continue;
        };
        let place = loc.place();
        file.set(*dev, &d.name, Value::from(format!("{place}@{newest}")))?;
        changes.push(format!("{}: {place} {} -> {newest}", d.name, loc.version));
    }
    if changes.is_empty() {
        println!("every dependency is at the newest version of its major version");
        return Ok(ExitCode::SUCCESS);
    }
    write_and_load(project, &file, &before)?;
    for c in changes {
        println!("updated {c}");
    }
    Ok(ExitCode::SUCCESS)
}

// ---- Permissions: i deps ----

/// What one package that the program uses can use.
#[derive(Clone, Debug, Default)]
struct PackageReport {
    /// How the package is shown: `location version`, `path dir`, or the
    /// compiled package.
    shown: String,
    /// Only the tests of the compiled package use it.
    dev: bool,
    /// Each permission, as `host service "ns"` or `external "m" "f"`, and
    /// where it comes from: the names that declare it, as
    /// `module/path.Name`, and the modules that import a library module
    /// that declares it, as `module/path imports i/io.Io`.
    permissions: BTreeMap<String, BTreeSet<String>>,
}

/// The packages of a program, by a key that is the same for every version
/// of one package (§2.7): `location@major`, `path <dir>`, or `` for the
/// compiled package.
#[derive(Clone, Debug, Default)]
struct Report {
    packages: BTreeMap<String, PackageReport>,
}

/// The host services (§9.7) and the external functions (§10.1) that each
/// package of the graph can use, from the resolved program: those that its
/// modules declare, and the host services of the library modules that they
/// import.
fn report(graph: &PackageGraph) -> Result<Report> {
    let (program, diags) = ilang_check::resolve_program(graph);
    let Some(program) = program else {
        return Err(problems(graph, &diags));
    };
    let root = graph
        .root
        .ok_or_else(|| anyhow!("the package could not be read"))?;
    let root_dir = graph.packages[root].dir.clone().unwrap_or_default();
    // The packages that the program uses without the tests.
    let mut used = HashSet::from([root]);
    let mut stack = vec![root];
    while let Some(p) = stack.pop() {
        for (_, d) in &graph.packages[p].dependencies {
            if let Some(d) = *d
                && used.insert(d)
            {
                stack.push(d);
            }
        }
    }
    let key = |id: PackageId| -> String {
        let p = &graph.packages[id];
        match (&p.location, &p.dir) {
            _ if id == root => String::new(),
            (Some(l), _) => format!("{}@{}", l.location, l.version.major),
            (None, Some(dir)) => format!("path {}", dir.display()),
            (None, None) => format!("package {}", p.name),
        }
    };
    let mut out = Report::default();
    for (id, p) in graph.packages.iter().enumerate() {
        if id == graph.library {
            continue;
        }
        let shown = match (&p.location, &p.dir) {
            _ if id == root => format!("{} (this package)", p.name),
            (Some(l), _) => format!("{} {}", l.location, l.version),
            (None, Some(dir)) => format!("{} (path {})", p.name, relative(&root_dir, dir)),
            (None, None) => p.name.clone(),
        };
        out.packages.insert(
            key(id),
            PackageReport {
                shown,
                dev: !used.contains(&id),
                permissions: BTreeMap::new(),
            },
        );
    }
    let mut add = |package: PackageId, what: String, at: String| {
        if let Some(r) = out.packages.get_mut(&key(package)) {
            r.permissions.entry(what).or_default().insert(at);
        }
    };
    let service = |host: &hir::Host| -> String {
        if host.is_async {
            format!("host service \"{}\" (async)", host.namespace)
        } else {
            format!("host service \"{}\"", host.namespace)
        }
    };
    for t in &program.types {
        if let Some(host) = &t.host {
            let m = program.module(t.module);
            add(m.package, service(host), format!("{}.{}", m.path, t.name));
        }
    }
    for m in &program.modules {
        if m.package == graph.library {
            continue;
        }
        for t in library_hosts(&program, graph.library, m) {
            let def = program.ty(t);
            let Some(host) = &def.host else { continue };
            let from = program.module(def.module);
            let at = format!("{} imports {}.{}", m.path, from.path, def.name);
            add(m.package, service(host), at);
        }
        for d in &m.defs {
            if let DefKind::External {
                module, function, ..
            } = &d.kind
            {
                let what = format!("external \"{module}\" \"{function}\"");
                add(m.package, what, format!("{}.{}", m.path, d.name));
            }
        }
    }
    Ok(out)
}

/// The host services (§9.7) of the library `i` that the module `m`
/// imports. The standard service is declared in `i/io`, so a package whose
/// module imports `i/io` can print as if it declared the service itself.
fn library_hosts<'a>(
    program: &'a hir::Program,
    library: PackageId,
    m: &'a hir::Module,
) -> impl Iterator<Item = hir::TypeId> + 'a {
    m.imports
        .iter()
        .map(|&i| program.module(i))
        .filter(move |i| i.package == library)
        .flat_map(|i| i.types.iter().copied())
        .filter(|&t| program.ty(t).host.is_some())
}

fn deps(project: &Path) -> Result<ExitCode> {
    let (graph, diags) = load(project, true);
    if diags.iter().any(Diagnostic::is_error) || graph.root.is_none() {
        return Err(problems(&graph, &diags));
    }
    let r = report(&graph)?;
    for p in r.packages.values() {
        println!("{}{}", p.shown, if p.dev { " (dev)" } else { "" });
        if p.permissions.is_empty() {
            println!("  no host services and no external functions");
        }
        for (what, at) in &p.permissions {
            let at: Vec<&str> = at.iter().map(String::as_str).collect();
            println!("  {what} ({})", at.join(", "));
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Prints each permission of `after` that `before` does not have, for the
/// same package (any version of one major version of a location).
fn print_new_permissions(before: &Report, after: &Report) {
    for (key, p) in &after.packages {
        let old = before.packages.get(key);
        for (what, at) in &p.permissions {
            if old.is_some_and(|o| o.permissions.contains_key(what)) {
                continue;
            }
            let at: Vec<&str> = at.iter().map(String::as_str).collect();
            println!(
                "new permission: {} can use {what} ({})",
                p.shown,
                at.join(", ")
            );
        }
    }
}

// ---- i publish ----

/// Runs git in `dir`; a failure is an error with what git printed.
fn git_in(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .context("`git` could not be run")?;
    if !out.status.success() {
        bail!(
            "`git {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

/// Fails unless the git working tree of `dir` is clean.
fn check_clean(dir: &Path) -> Result<()> {
    let status = git_in(dir, &["status", "--porcelain", "--untracked-files=all"])?;
    if !status.is_empty() {
        bail!(
            "the git working tree is not clean; commit these files first:\n{}",
            status
        );
    }
    Ok(())
}

fn publish(project: &Path, version: &str, dry_run: bool) -> Result<ExitCode> {
    let version = parse_version(version.strip_prefix('v').unwrap_or(version))
        .ok_or_else(|| anyhow!("`{version}` is not a version `major.minor.patch`"))?;
    let project = &project
        .canonicalize()
        .with_context(|| format!("there is no directory `{}`", project.display()))?;

    // 1. The working tree is clean.
    git_in(project, &["rev-parse", "--verify", "--quiet", "HEAD"])
        .context("the package must be in a git repository with a commit")?;
    check_clean(project)?;
    println!("ok: the git working tree is clean");

    // 2. `i check` and `i test` pass.
    check_and_test(project)?;
    // Checking may have written `i.lock`.
    check_clean(project).context("`i.lock` was changed by the check")?;

    // 3. The version is newer than every tag.
    let tags: Vec<Version> = git_in(project, &["tag", "--list", "v*"])?
        .lines()
        .filter_map(|t| t.strip_prefix('v').and_then(parse_version))
        .collect();
    let previous = tags.iter().copied().max();
    if let Some(p) = previous
        && version <= p
    {
        bail!("the version {version} must be newer than every tag; the newest is v{p}");
    }
    println!("ok: {version} is newer than every tag");

    // 4. The public interface, compared with the newest tag.
    let now = interface_of(project, false)?
        .map_err(|e| e.context("the interface of the package could not be worked out"))?;
    match previous {
        None => println!("ok: there is no earlier version to compare with"),
        Some(p) => compare_with(project, p, version, &now)?,
    }

    // 5. The tag.
    let tag = format!("v{version}");
    if dry_run {
        println!("dry run: every check passed; the tag {tag} was not made");
        return Ok(ExitCode::SUCCESS);
    }
    git_in(project, &["tag", &tag])?;
    let remotes = git_in(project, &["remote"])?;
    let remote = if remotes.lines().any(|r| r == "origin") || remotes.is_empty() {
        "origin"
    } else {
        remotes.lines().next().unwrap_or("origin")
    };
    println!("made the tag {tag}; nothing was pushed. To publish it:");
    println!("  git push {remote} {tag}");
    Ok(ExitCode::SUCCESS)
}

/// Builds the package with its tests and runs them, as `i test` does.
fn check_and_test(project: &Path) -> Result<()> {
    let built = crate_build(project)?;
    let program = ilang_host::Program::from_bytes(&built)?;
    let names: Vec<String> = program.abi().tests.iter().map(|t| t.name.clone()).collect();
    let mut failed = Vec::new();
    for name in &names {
        match ilang_cli::run_test(&program, name) {
            ilang_host::TestResult::Passed => {}
            ilang_host::TestResult::Stopped(stop) => failed.push(format!("FAIL {name}\n  {stop}")),
            ilang_host::TestResult::Failed(e) => failed.push(format!("FAIL {name}\n  {e:#}")),
        }
    }
    if !failed.is_empty() {
        bail!(
            "{}\n{} of {} tests failed",
            failed.join("\n"),
            failed.len(),
            names.len()
        );
    }
    println!(
        "ok: the package checks, and its {} test{} passed",
        names.len(),
        if names.len() == 1 { "" } else { "s" }
    );
    Ok(())
}

fn crate_build(project: &Path) -> Result<Vec<u8>> {
    let options = ilang_cli::BuildOptions {
        tests: true,
        opt: ilang_cli::Level::O0,
        offline: false,
        target: Default::default(),
    };
    match ilang_cli::build(project, options) {
        Ok(b) => Ok(b.wasm),
        Err(e) => Err(anyhow!("{e}").context("the package does not check")),
    }
}

/// Compares the interface `now` with the one of the tag `v<previous>`, and
/// fails when `version` is not a big enough step for the changes.
fn compare_with(
    project: &Path,
    previous: Version,
    version: Version,
    now: &Interface,
) -> Result<()> {
    let tag = format!("v{previous}");
    let earlier = earlier_interface(project, &tag)?;
    let step = if version.major > previous.major {
        Step::Major
    } else if version.minor > previous.minor {
        Step::Minor
    } else {
        Step::Patch
    };
    let (needed, why) = match &earlier {
        None => {
            println!("ok: {tag} has no package here, so there is nothing to compare with");
            return Ok(());
        }
        Some(Err(e)) => {
            println!("the interface of {tag} could not be worked out: {e:#}");
            (
                Step::Major,
                "the interface of the earlier version is not known",
            )
        }
        Some(Ok(old)) => {
            let diff = Diff::new(old, now);
            diff.print(&tag);
            if !diff.removed.is_empty() || !diff.changed.is_empty() {
                (Step::Major, "something was removed or changed")
            } else if !diff.added.is_empty() {
                (Step::Minor, "something was added")
            } else {
                (Step::Patch, "nothing changed")
            }
        }
    };
    if step < needed {
        let least = match needed {
            Step::Major => Version {
                major: previous.major + 1,
                minor: 0,
                patch: 0,
            },
            Step::Minor => Version {
                major: previous.major,
                minor: previous.minor + 1,
                patch: 0,
            },
            Step::Patch => previous,
        };
        bail!(
            "{why} since {tag}, so the version must be a new {} version, at least {least}; {version} is a {} version",
            needed.name(),
            step.name()
        );
    }
    println!(
        "ok: {why} since {tag}, and {version} is a new {} version",
        step.name()
    );
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Step {
    Patch,
    Minor,
    Major,
}

impl Step {
    fn name(self) -> &'static str {
        match self {
            Step::Patch => "patch",
            Step::Minor => "minor",
            Step::Major => "major",
        }
    }
}

/// The interface of the package at `tag`: its files are written from
/// `git archive` into a temporary directory and checked there. `None` when
/// the package was not in the repository at that tag.
fn earlier_interface(project: &Path, tag: &str) -> Result<Option<Result<Interface>>> {
    let top = PathBuf::from(git_in(project, &["rev-parse", "--show-toplevel"])?);
    let prefix = git_in(project, &["rev-parse", "--show-prefix"])?;
    let prefix = prefix.trim_end_matches('/');
    let toml = if prefix.is_empty() {
        format!("{tag}:i.toml")
    } else {
        format!("{tag}:{prefix}/i.toml")
    };
    if git_in(&top, &["cat-file", "-e", &toml]).is_err() {
        return Ok(None);
    }
    let tmp = tempfile::tempdir().context("a temporary directory could not be made")?;
    let archive = Command::new("git")
        .arg("-C")
        .arg(&top)
        .args(["archive", "--format=tar", tag])
        .stdin(Stdio::null())
        .output()
        .context("`git archive` could not be run")?;
    if !archive.status.success() {
        bail!(
            "`git archive {tag}` failed: {}",
            String::from_utf8_lossy(&archive.stderr).trim()
        );
    }
    let mut tar = Command::new("tar")
        .arg("-x")
        .arg("-C")
        .arg(tmp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("`tar` could not be run")?;
    {
        use std::io::Write as _;
        let mut stdin = tar.stdin.take().expect("piped");
        stdin.write_all(&archive.stdout)?;
    }
    let out = tar.wait_with_output()?;
    if !out.status.success() {
        bail!(
            "the files of {tag} could not be written: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let dir = tmp.path().join(prefix);
    Ok(Some(interface_of(&dir, false)?))
}

// ---- The public interface ----

/// One part of the public interface: how it is shown, and its canonical
/// form, which is what is compared. In the canonical form each custom type
/// is named by what it is (`ilang_check::identity`), whatever the module
/// that uses it calls it: its package (this package; the library `i`;
/// `location@major` for a package with a location, so that a new major
/// version of a dependency is another type (§2.7); `path dir` for a path
/// dependency), its module in that package, and its name. Type variables
/// are named by the order they first come in.
#[derive(Clone, Debug)]
struct Entry {
    shown: String,
    canonical: String,
}

/// The public interface of the `src/` modules of a package: for each
/// exported value its type, with the import of an external declaration
/// (§10.1); for each exported type its parameters and host namespace (or
/// its alias); and for each type exported with its constructors, and each
/// exported host service type (§9.7), the constructors with their fields.
/// The keys are `value m.x`, `type m.T`, and `constructors m.T`, where `m`
/// is the module path without the package name, which only the package
/// itself uses (§2.6).
///
/// What the host must give is part of it too, exported or not, since an
/// exported function may use it: each external function (§10.1), as
/// `external "module" "function"` with its type, and each host service
/// (§9.7) that it declares or imports from a library module, such as `Io`
/// of `i/io`, as `host "namespace"` with the fields of its constructor.
type Interface = BTreeMap<String, Entry>;

/// The interface of the package in `dir`, checked without its tests. The
/// outer error is a problem of the tool; the inner one says that the
/// package does not check.
fn interface_of(dir: &Path, write_lock: bool) -> Result<Result<Interface>> {
    let (graph, mut diags) = ilang_package::load_with(
        dir,
        &LoadOptions {
            tests: false,
            write_lock,
            ..LoadOptions::from_env()
        },
    );
    let Some(root) = graph.root else {
        return Ok(Err(problems(&graph, &diags)));
    };
    let checked = ilang_check::check(&graph);
    diags.extend(checked.diags);
    let (Some(program), Some(typed)) = (&checked.program, &checked.types) else {
        return Ok(Err(problems(&graph, &diags)));
    };
    if diags.iter().any(Diagnostic::is_error) {
        return Ok(Err(problems(&graph, &diags)));
    }
    let canonical_names = canonical_names(&graph, program);
    let mut out = Interface::new();
    // What the host must give, by its key: how each declaration of it is
    // shown, and its canonical forms.
    let mut host: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)> = BTreeMap::new();
    for (i, m) in program.modules.iter().enumerate() {
        if m.package != root || m.kind != ModuleKind::Src {
            continue;
        }
        let module = hir::ModuleId(i as u32);
        let shown_names = ilang_check::types::type_names(program, module);
        let path = inner_path(&m.path);
        for (index, d) in m.defs.iter().enumerate() {
            let id = hir::DefId {
                module,
                index: index as u32,
            };
            let ty = typed.schemes.get(&id).map_or("_", String::as_str);
            let canonical = typed
                .scheme_types
                .get(&id)
                .map_or_else(|| "_".to_string(), |t| canonical_type(t, &canonical_names));
            // The host gives an external function by its module and name.
            let import = match &d.kind {
                DefKind::External {
                    module, function, ..
                } => {
                    let (shown, canon) = host
                        .entry(format!("external \"{module}\" \"{function}\""))
                        .or_default();
                    shown.insert(ty.to_string());
                    canon.insert(canonical.clone());
                    format!("@external(\"{module}\", \"{function}\") ")
                }
                DefKind::Binding { .. } => String::new(),
            };
            if !d.exported {
                continue;
            }
            out.insert(
                format!("value {path}.{}", d.name),
                Entry {
                    shown: format!("{import}{ty}"),
                    canonical: format!("{import}{canonical}"),
                },
            );
        }
        // The host services of the library modules that this module
        // imports (§9.7) are part of what the host must give, as if the
        // module declared them.
        let types = m
            .types
            .iter()
            .copied()
            .chain(library_hosts(program, graph.library, m));
        for t in types {
            let def = program.ty(t);
            let shown = TypeShow::new(def, &shown_names, false);
            let canonical = TypeShow::new(def, &canonical_names, true);
            // The host gives the functions of a host service by their
            // labels (§9.7), so they are part of the interface even when
            // the type or its constructor is not exported.
            if let (Some(h), TypeBody::Custom(ctors)) = (&def.host, &def.body) {
                let fields = |s: &TypeShow| -> String {
                    let ctors: Vec<String> = ctors.iter().map(|c| s.fields(c)).collect();
                    let all = ctors.join(" | ");
                    if h.is_async {
                        format!("async {all}")
                    } else {
                        all
                    }
                };
                let (s, c) = host.entry(format!("host \"{}\"", h.namespace)).or_default();
                s.insert(fields(&shown));
                c.insert(fields(&canonical));
            }
            if !def.exported || def.module != module {
                continue;
            }
            out.insert(
                format!("type {path}.{}", def.name),
                Entry {
                    shown: shown.head(),
                    canonical: canonical.head(),
                },
            );
            if (def.constructors_exported || def.host.is_some())
                && let TypeBody::Custom(ctors) = &def.body
            {
                let all = |s: &TypeShow| -> String {
                    let ctors: Vec<String> = ctors.iter().map(|c| s.ctor(c)).collect();
                    ctors.join(" | ")
                };
                out.insert(
                    format!("constructors {path}.{}", def.name),
                    Entry {
                        shown: all(&shown),
                        canonical: all(&canonical),
                    },
                );
            }
        }
    }
    for (key, (shown, canonical)) in host {
        let join = |set: BTreeSet<String>| set.into_iter().collect::<Vec<_>>().join("; ");
        out.insert(
            key,
            Entry {
                shown: join(shown),
                canonical: join(canonical),
            },
        );
    }
    Ok(Ok(out))
}

/// A module path without the package name.
fn inner_path(path: &str) -> &str {
    path.split_once('/').map_or(path, |(_, p)| p)
}

/// The canonical name of each custom type of a program, by `TypeId` (see
/// [`Entry`]): `m.Name` for a type of this package, and else its package,
/// `:`, and `m.Name`.
fn canonical_names(graph: &PackageGraph, program: &hir::Program) -> Vec<String> {
    let root_dir = graph
        .root
        .and_then(|r| graph.packages[r].dir.clone())
        .unwrap_or_default();
    (0..program.types.len())
        .map(|i| {
            let t = type_identity(graph, program, hir::TypeId(i as u32));
            let inner = format!("{}.{}", t.module, t.name);
            match t.package {
                PackageIdentity::Root => inner,
                PackageIdentity::Library => format!("i:{inner}"),
                PackageIdentity::Location { location, major } => {
                    format!("{location}@{major}:{inner}")
                }
                PackageIdentity::Path(dir) => {
                    format!("path {}:{inner}", relative(&root_dir, &dir))
                }
                PackageIdentity::Named(name) => format!("package {name}:{inner}"),
            }
        })
        .collect()
}

/// The canonical form of a shown type (§4.7): each custom type is put as
/// its canonical name, and each type variable as `'0`, `'1`, … in the
/// order they first come in. Labels and the names of the built-in types
/// stay.
fn canonical_type(t: &ShownType, names: &[String]) -> String {
    let mut vars: Vec<String> = Vec::new();
    t.text_with(
        &mut |r| match r {
            TypeRef::Builtin(b) => b.name().to_string(),
            TypeRef::Custom(id) => names[id.0 as usize].clone(),
        },
        &mut |v| {
            let i = vars.iter().position(|x| x == v).unwrap_or_else(|| {
                vars.push(v.to_string());
                vars.len() - 1
            });
            format!("'{i}")
        },
    )
}

/// Shows the parts of a type definition. The parameters are named by their
/// place, `A`, `B`, …, so that renaming them is no change.
struct TypeShow<'a> {
    def: &'a hir::TypeDef,
    /// The name of each custom type, by `TypeId`.
    names: &'a [String],
    /// Other type variables are named by the order they first come in, as
    /// `'0`, `'1`, …, in place of their written names.
    canonical: bool,
    vars: RefCell<Vec<hir::TyVarId>>,
}

impl<'a> TypeShow<'a> {
    fn new(def: &'a hir::TypeDef, names: &'a [String], canonical: bool) -> Self {
        TypeShow {
            def,
            names,
            canonical,
            vars: RefCell::new(Vec::new()),
        }
    }

    fn param(&self, i: usize) -> String {
        if self.canonical {
            return format!("'p{i}");
        }
        let letters = b"ABCDFGHIJKLMOPQRSTUVWXYZ";
        (0..)
            .flat_map(|round| {
                letters.iter().map(move |c| {
                    if round == 0 {
                        (*c as char).to_string()
                    } else {
                        format!("{}{suffix}", *c as char, suffix = round + 1)
                    }
                })
            })
            .filter(|name| !self.names.contains(name))
            .nth(i)
            .unwrap()
    }

    /// The name of a type variable that is not a parameter.
    fn var(&self, v: hir::TyVarId) -> String {
        if self.canonical {
            let mut vars = self.vars.borrow_mut();
            let i = vars.iter().position(|x| *x == v).unwrap_or_else(|| {
                vars.push(v);
                vars.len() - 1
            });
            return format!("'{i}");
        }
        let mut vars = self.vars.borrow_mut();
        let i = vars.iter().position(|x| *x == v).unwrap_or_else(|| {
            vars.push(v);
            vars.len() - 1
        });
        self.param(self.def.params.len() + i)
    }

    /// `@host("ns") Name(A, B)`, or `Name(A) = Type` for an alias.
    fn head(&self) -> String {
        let mut s = String::new();
        if let Some(h) = &self.def.host {
            if h.is_async {
                s.push_str(&format!("@host(\"{}\", \"async\") ", h.namespace));
            } else {
                s.push_str(&format!("@host(\"{}\") ", h.namespace));
            }
        }
        s.push_str(&self.def.name);
        if !self.def.params.is_empty() {
            let ps: Vec<String> = (0..self.def.params.len()).map(|i| self.param(i)).collect();
            s.push_str(&format!("({})", ps.join(", ")));
        }
        if let TypeBody::Alias(t) = &self.def.body {
            s.push_str(&format!(" = {}", self.ty(t, 0)));
        }
        s
    }

    /// `C(label: Type, Type)`.
    fn ctor(&self, c: &hir::CtorDef) -> String {
        if c.fields.is_empty() {
            return c.name.clone();
        }
        format!("{}{}", c.name, self.fields(c))
    }

    /// `(label: Type, Type)`: the fields of a constructor.
    fn fields(&self, c: &hir::CtorDef) -> String {
        let fields: Vec<String> = c
            .fields
            .iter()
            .map(|f| match &f.label {
                Some(l) => format!("{}: {}", l.name, self.ty(&f.ty, 0)),
                None => self.ty(&f.ty, 0),
            })
            .collect();
        format!("({})", fields.join(", "))
    }

    /// A written type in the syntax of §3. `prec` is 1 inside a union, 2
    /// inside an intersection, and 3 inside a difference; a part that
    /// binds less is put in `{ }`.
    fn ty(&self, t: &hir::Type, prec: u8) -> String {
        let wrap = |s: String, own: u8| {
            if prec > own { format!("{{ {s} }}") } else { s }
        };
        let join = |ts: &[hir::Type], sep: &str, p: u8| {
            ts.iter()
                .map(|t| self.ty(t, p))
                .collect::<Vec<_>>()
                .join(sep)
        };
        match &t.kind {
            TypeKind::Named { ty, args } => {
                let name = match ty {
                    TypeRef::Builtin(b) => b.name().to_string(),
                    TypeRef::Custom(id) => self.names[id.0 as usize].clone(),
                };
                if args.is_empty() {
                    name
                } else {
                    format!("{name}({})", join(args, ", ", 0))
                }
            }
            TypeKind::Var(v) => match self.def.params.iter().position(|(_, p)| p == v) {
                Some(i) => self.param(i),
                None => self.var(*v),
            },
            TypeKind::Hole | TypeKind::Error => "_".to_string(),
            TypeKind::Tuple(ts) => format!("({})", join(ts, ", ", 0)),
            TypeKind::Fn {
                params,
                ret,
                err,
                env,
            } => {
                let ps: Vec<String> = params
                    .iter()
                    .map(|p| match &p.label {
                        Some(l) => format!("{}: {}", l.name, self.ty(&p.ty, 0)),
                        None => self.ty(&p.ty, 0),
                    })
                    .collect();
                let mut s = format!("({}) -> {}", ps.join(", "), self.ty(ret, 1));
                if let Some(e) = err {
                    s.push_str(&format!(" ? {}", self.ty(e, 1)));
                }
                if let Some(e) = env {
                    s.push_str(&format!(" ! {}", self.ty(e, 1)));
                }
                wrap(s, 0)
            }
            TypeKind::Union(ts) => wrap(join(ts, " | ", 1), 0),
            TypeKind::Inter(ts) => wrap(join(ts, " & ", 2), 1),
            TypeKind::Diff(ts) => wrap(join(ts, " - ", 3), 2),
        }
    }
}

/// The differences between two interfaces, by their canonical forms.
struct Diff {
    removed: Vec<(String, Entry)>,
    changed: Vec<(String, Entry, Entry)>,
    added: Vec<(String, Entry)>,
}

impl Diff {
    fn new(old: &Interface, new: &Interface) -> Diff {
        let mut d = Diff {
            removed: Vec::new(),
            changed: Vec::new(),
            added: Vec::new(),
        };
        for (k, v) in old {
            match new.get(k) {
                None => d.removed.push((k.clone(), v.clone())),
                Some(n) if n.canonical != v.canonical => {
                    d.changed.push((k.clone(), v.clone(), n.clone()))
                }
                Some(_) => {}
            }
        }
        for (k, v) in new {
            if !old.contains_key(k) {
                d.added.push((k.clone(), v.clone()));
            }
        }
        d
    }

    fn print(&self, tag: &str) {
        if self.removed.is_empty() && self.changed.is_empty() && self.added.is_empty() {
            println!("the public interface is the same as in {tag}");
            return;
        }
        println!("changes to the public interface since {tag}:");
        for (k, v) in &self.removed {
            println!("  removed {k}: {}", v.shown);
        }
        for (k, old, new) in &self.changed {
            // When the change does not show, the types are named by what
            // they are.
            let (old, new) = if old.shown == new.shown {
                (&old.canonical, &new.canonical)
            } else {
                (&old.shown, &new.shown)
            };
            println!("  changed {k}: {old}");
            println!("       to {k}: {new}");
        }
        for (k, v) in &self.added {
            println!("  added {k}: {}", v.shown);
        }
    }
}

// ---- Paths ----

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

#[cfg(test)]
mod syntax_templates {
    #[test]
    fn new_package_sources_parse() {
        for source in [super::main_module("example"), super::test_module("example")] {
            assert!(ilang_syntax::parse_module(&source).1.is_empty(), "{source}");
        }
    }
}
