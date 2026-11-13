//! Packages with a location, got from local bare repositories
//! through `file://` URLs. Nothing here uses the network.

use ilang_package::git::{Git, cache_dir, escape, parse_url_map};
use ilang_package::lock::{FileKind, Lock, PackageFile, hash};
use ilang_package::manifest::parse_version;
use ilang_package::{Diagnostic, LoadOptions, ModuleKind, PackageGraph, code, load_with};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The same hash as in `tests/lock.rs`, worked out apart from this crate.
const VECTOR_HASH: &str = "sha256-c7432df6c2877e89b6414be5da72b517b09270e4e9be6210dc41f7ff4bb75348";

/// Runs git with no user configuration, a fixed author and a fixed date.
fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_AUTHOR_DATE", "2026-01-01T00:00:00Z")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_DATE", "2026-01-01T00:00:00Z")
        .args([
            "-c",
            "init.defaultBranch=main",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "tag.gpgsign=false",
            "-c",
            "core.autocrlf=false",
            "-c",
            "core.symlinks=true",
        ])
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// A temporary directory with bare repositories under `remotes/`, which the
/// locations `example.com/...` are got from, a cache, and projects.
struct World {
    tmp: tempfile::TempDir,
}

/// A bare repository and a work tree that pushes to it.
struct Repo {
    work: PathBuf,
    bare: PathBuf,
}

impl World {
    fn new() -> World {
        World {
            tmp: tempfile::tempdir().unwrap(),
        }
    }

    fn path(&self) -> &Path {
        self.tmp.path()
    }

    fn cache(&self) -> PathBuf {
        self.path().join("cache")
    }

    fn urls(&self) -> Vec<(String, String)> {
        let remotes = self.path().join("remotes");
        vec![(
            "example.com/".to_string(),
            format!("file://{}/", remotes.display()),
        )]
    }

    fn options(&self, tests: bool) -> LoadOptions {
        LoadOptions {
            tests,
            offline: false,
            cache: Some(self.cache()),
            git_urls: self.urls(),
            write_lock: true,
        }
    }

    /// The repository of the location `example.com/<name>`.
    fn repo(&self, name: &str) -> Repo {
        let work = self.path().join("work").join(name);
        let bare = self.path().join("remotes").join(name);
        fs::create_dir_all(&work).unwrap();
        fs::create_dir_all(&bare).unwrap();
        git(&work, &["init", "-q"]);
        git(&bare, &["init", "-q", "--bare"]);
        Repo { work, bare }
    }

    /// Writes a project and gives its directory.
    fn project(&self, name: &str, files: &[(&str, &str)]) -> PathBuf {
        let dir = self.path().join(name);
        write(&dir, files);
        dir
    }

    fn load(&self, project: &Path, tests: bool) -> (PackageGraph, Vec<Diagnostic>) {
        load_with(project, &self.options(tests))
    }
}

fn write(dir: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let p = dir.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
}

impl Repo {
    /// Makes the tree exactly `files`, commits it, tags it `tag`, and pushes
    /// both. Gives the commit.
    fn commit(&self, files: &[(&str, &str)], tag: &str) -> String {
        git(&self.work, &["rm", "-rq", "--ignore-unmatch", "."]);
        write(&self.work, files);
        self.commit_as_is(tag)
    }

    fn commit_as_is(&self, tag: &str) -> String {
        git(&self.work, &["add", "-A"]);
        git(&self.work, &["commit", "-q", "--allow-empty", "-m", tag]);
        git(&self.work, &["tag", "-f", tag]);
        self.push();
        git(&self.work, &["rev-parse", "HEAD"])
    }

    fn push(&self) {
        let bare = self.bare.to_str().unwrap();
        git(
            &self.work,
            &["push", "-q", "--force", bare, "HEAD:refs/heads/main"],
        );
        git(&self.work, &["push", "-q", "--force", "--tags", bare]);
    }

    /// A package `name` with these dependencies (`i.toml` lines), at
    /// version `version`.
    fn package(&self, name: &str, deps: &[&str], version: &str) -> String {
        let toml = package_toml(name, deps);
        let module = format!("src/{name}.i");
        self.commit(
            &[("i.toml", &toml), (&module, "x = 1\n")],
            &format!("v{version}"),
        )
    }
}

fn package_toml(name: &str, deps: &[&str]) -> String {
    let mut toml = format!("name = \"{name}\"\n[dependencies]\n");
    for d in deps {
        toml.push_str(d);
        toml.push('\n');
    }
    toml
}

fn errors(diags: &[Diagnostic]) -> Vec<&'static str> {
    diags
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.code)
        .collect()
}

fn no_errors(graph: &PackageGraph, diags: &[Diagnostic]) {
    let rendered = graph.sources.render_all(diags);
    assert!(errors(diags).is_empty(), "{rendered:#?}");
}

/// The packages with a location in the graph: `location@version`.
fn located(graph: &PackageGraph) -> Vec<String> {
    let mut out: Vec<String> = graph
        .packages
        .iter()
        .filter_map(|p| p.location.as_ref())
        .map(|l| format!("{}@{}", l.location, l.version))
        .collect();
    out.sort();
    out
}

/// The location and version of each line of `i.lock`.
fn lock_lines(project: &Path) -> Vec<String> {
    let text = fs::read_to_string(project.join("i.lock")).unwrap_or_default();
    text.lines()
        .map(|l| l.split(' ').take(2).collect::<Vec<_>>().join(" "))
        .collect()
}

/// The package that the dependency name `dep` of package `of` is.
fn dep<'g>(graph: &'g PackageGraph, of: usize, dep: &str) -> &'g ilang_package::Package {
    let id = graph.packages[of]
        .dependencies
        .iter()
        .chain(&graph.packages[of].dev_dependencies)
        .find(|(n, _)| n == dep)
        .unwrap_or_else(|| panic!("no dependency `{dep}`"))
        .1
        .unwrap_or_else(|| panic!("`{dep}` is not loaded"));
    &graph.packages[id]
}

#[test]
fn tags_and_commits() {
    let w = World::new();
    let r = w.repo("alice/json");
    let c1 = r.package("json", &[], "1.0.0");
    let c2 = r.package("json", &[], "1.2.0");
    git(&r.work, &["tag", "release"]);
    git(&r.work, &["tag", "v1.3"]);
    git(&r.work, &["tag", "v01.0.0"]);
    // An annotated tag is taken to its commit.
    let c3 = r.package("json", &[], "2.0.0");
    git(&r.work, &["tag", "-d", "v2.0.0"]);
    git(&r.work, &["tag", "-a", "-m", "two", "v2.0.0"]);
    r.push();

    let mut g = Git::new(w.cache(), false, w.urls());
    let tags: Vec<String> = g
        .tags("example.com/alice/json")
        .unwrap()
        .iter()
        .map(|v| v.to_string())
        .collect();
    assert_eq!(tags, ["1.0.0", "1.2.0", "2.0.0"]);
    let v = |s| parse_version(s).unwrap();
    assert_eq!(
        g.commit("example.com/alice/json", v("1.0.0"), None)
            .unwrap(),
        c1
    );
    assert_eq!(
        g.commit("example.com/alice/json", v("1.2.0"), None)
            .unwrap(),
        c2
    );
    assert_eq!(
        g.commit("example.com/alice/json", v("2.0.0"), None)
            .unwrap(),
        c3
    );
    let e = g
        .commit("example.com/alice/json", v("1.1.0"), None)
        .unwrap_err();
    assert_eq!(e.code, code::NO_VERSION);
    assert!(e.message.contains("1.0.0, 1.2.0, 2.0.0"), "{}", e.message);
    // The cache layout.
    assert!(
        w.cache()
            .join("git")
            .join("example.com%2falice%2fjson")
            .join("HEAD")
            .is_file()
    );
    let text = g
        .read("example.com/alice/json", &c2, "i.toml")
        .unwrap()
        .unwrap();
    assert_eq!(text, b"name = \"json\"\n[dependencies]\n");
    assert_eq!(
        g.read("example.com/alice/json", &c2, "nothing").unwrap(),
        None
    );
}

#[test]
fn urls_and_cache_names() {
    let g = Git::new(
        PathBuf::from("/c"),
        false,
        parse_url_map("example.com/=file:///a/ example.com/alice/=file:///b/ bad"),
    );
    assert_eq!(g.url("example.com/bob/x"), "file:///a/bob/x");
    assert_eq!(g.url("example.com/alice/json"), "file:///b/json");
    assert_eq!(
        g.url("github.com/alice/json"),
        "https://github.com/alice/json"
    );
    assert_eq!(
        escape("codeberg.org/Bob/tools//text"),
        "codeberg.org%2f%42ob%2ftools%2f%2ftext"
    );
    let vars = |pairs: &'static [(&'static str, &'static str)]| {
        move |k: &str| {
            pairs
                .iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| OsString::from(v))
        }
    };
    assert_eq!(
        cache_dir(vars(&[("ILANG_CACHE", "/x"), ("HOME", "/h")])),
        Some(PathBuf::from("/x"))
    );
    assert_eq!(
        cache_dir(vars(&[("XDG_CACHE_HOME", "/xdg"), ("HOME", "/h")])),
        Some(PathBuf::from("/xdg/ilang"))
    );
    assert_eq!(
        cache_dir(vars(&[("HOME", "/h")])),
        Some(PathBuf::from("/h/.cache/ilang"))
    );
    let o = LoadOptions::from_vars(vars(&[
        ("ILANG_OFFLINE", "1"),
        ("ILANG_CACHE", "/x"),
        ("ILANG_GIT_URLS", "example.com/=file:///r/"),
    ]));
    assert!(o.offline && o.tests && o.write_lock);
    assert_eq!(o.cache, Some(PathBuf::from("/x")));
    assert_eq!(
        o.git_urls,
        [("example.com/".to_string(), "file:///r/".to_string())]
    );
    assert!(!LoadOptions::from_vars(vars(&[("ILANG_OFFLINE", "0")])).offline);
    assert!(!LoadOptions::from_vars(vars(&[])).offline);
}

/// The tree of the hash vector, with a submodule, which is left out.
#[cfg(unix)]
fn vector_repo(w: &World) -> (Repo, String) {
    use std::os::unix::fs::PermissionsExt;
    let r = w.repo("vec/tree");
    write(
        &r.work,
        &[
            ("i.toml", "name = \"vec\"\n"),
            ("src/a.i", "x = 1\r\ny = 2\r\n"),
            ("src/b.i", "z = 3\n"),
            ("src-x", ""),
            ("run.sh", "#!/bin/sh\n"),
        ],
    );
    let run = r.work.join("run.sh");
    fs::set_permissions(&run, fs::Permissions::from_mode(0o755)).unwrap();
    std::os::unix::fs::symlink("src/a.i", r.work.join("link")).unwrap();
    git(&r.work, &["add", "-A"]);
    git(
        &r.work,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            "160000,0123456789012345678901234567890123456789,sub",
        ],
    );
    let c = r.commit_as_is("v1.0.0");
    (r, c)
}

#[cfg(unix)]
#[test]
fn hash_vector_from_git() {
    let w = World::new();
    let (_r, c) = vector_repo(&w);
    let mut g = Git::new(w.cache(), false, w.urls());
    let commit = g
        .commit(
            "example.com/vec/tree",
            parse_version("1.0.0").unwrap(),
            None,
        )
        .unwrap();
    assert_eq!(commit, c);
    let files = g.files("example.com/vec/tree", &c, None).unwrap();
    let paths: Vec<(&str, FileKind)> = files.iter().map(|f| (f.path.as_str(), f.kind)).collect();
    assert_eq!(
        paths,
        [
            ("i.toml", FileKind::Regular),
            ("link", FileKind::Symlink),
            ("run.sh", FileKind::Executable),
            ("src-x", FileKind::Regular),
            ("src/a.i", FileKind::Regular),
            ("src/b.i", FileKind::Regular),
        ]
    );
    assert_eq!(hash(&files), VECTOR_HASH);

    // The files in the cache: as git keeps them, read-only, with the link.
    let (dir, h) = g
        .package("example.com/vec/tree", None, "example.com/vec/tree", &c)
        .unwrap();
    assert_eq!(h, VECTOR_HASH);
    assert_eq!(
        dir,
        w.cache()
            .join("pkg")
            .join("example.com%2fvec%2ftree")
            .join(&c)
    );
    assert_eq!(
        fs::read(dir.join("src/a.i")).unwrap(),
        b"x = 1\r\ny = 2\r\n"
    );
    assert!(
        fs::metadata(dir.join("src/a.i"))
            .unwrap()
            .permissions()
            .readonly()
    );
    let link = dir.join("link");
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read_link(&link).unwrap(), PathBuf::from("src/a.i"));
    assert!(!dir.join("sub").exists());
    // The hash is kept next to the files.
    let kept = fs::read_to_string(dir.with_extension("hash")).unwrap();
    assert_eq!(kept.trim(), VECTOR_HASH);
}

#[test]
fn crlf_is_hashed_as_git_keeps_it() {
    let w = World::new();
    let r = w.repo("alice/crlf");
    // A checkout would turn the line ends of `src/lf.i` into CRLF; git keeps
    // them as LF. `src/crlf.i` is kept with CRLF.
    let c = r.commit(
        &[
            (".gitattributes", "*.i text eol=crlf\n"),
            ("i.toml", "name = \"crlf\"\n"),
            ("src/lf.i", "a = 1\nb = 2\n"),
        ],
        "v1.0.0",
    );
    let _ = c;
    // Stored with CRLF: `-text` keeps git from changing it on the way in.
    write(
        &r.work,
        &[
            (
                ".gitattributes",
                "src/crlf.i -text\nsrc/lf.i text eol=crlf\n",
            ),
            ("src/crlf.i", "a = 1\r\n"),
        ],
    );
    let c = r.commit_as_is("v1.1.0");
    let g = {
        let mut g = Git::new(w.cache(), false, w.urls());
        g.commit(
            "example.com/alice/crlf",
            parse_version("1.1.0").unwrap(),
            None,
        )
        .unwrap();
        g
    };
    let files = g.files("example.com/alice/crlf", &c, None).unwrap();
    let get = |p: &str| files.iter().find(|f| f.path == p).unwrap().bytes.clone();
    assert_eq!(get("src/lf.i"), b"a = 1\nb = 2\n");
    assert_eq!(get("src/crlf.i"), b"a = 1\r\n");
    let expected = hash(&[
        PackageFile {
            path: ".gitattributes".into(),
            kind: FileKind::Regular,
            bytes: b"src/crlf.i -text\nsrc/lf.i text eol=crlf\n".to_vec(),
        },
        PackageFile {
            path: "i.toml".into(),
            kind: FileKind::Regular,
            bytes: b"name = \"crlf\"\n".to_vec(),
        },
        PackageFile {
            path: "src/crlf.i".into(),
            kind: FileKind::Regular,
            bytes: b"a = 1\r\n".to_vec(),
        },
        PackageFile {
            path: "src/lf.i".into(),
            kind: FileKind::Regular,
            bytes: b"a = 1\nb = 2\n".to_vec(),
        },
    ]);
    assert_eq!(hash(&files), expected);
    let (dir, _) = g
        .package("example.com/alice/crlf", None, "example.com/alice/crlf", &c)
        .unwrap();
    assert_eq!(fs::read(dir.join("src/lf.i")).unwrap(), b"a = 1\nb = 2\n");
}

#[test]
fn lock_is_written() {
    let w = World::new();
    let json = w.repo("alice/json");
    let c = json.package("json", &[], "1.4.0");
    let app = w.project(
        "app",
        &[
            (
                "i.toml",
                "name = \"app\"\n[dependencies]\njson = \"example.com/alice/json@1.4.0\"\n",
            ),
            ("src/main.i", "x = 1\n"),
        ],
    );
    let (graph, diags) = w.load(&app, true);
    no_errors(&graph, &diags);
    let root = graph.root.unwrap();
    let pkg = dep(&graph, root, "json");
    assert_eq!(pkg.name, "json");
    let loc = pkg.location.as_ref().unwrap();
    assert_eq!(loc.commit, c);
    assert_eq!(pkg.position_prefix(), "example.com/alice/json@1 ");
    assert_eq!(graph.packages[root].position_prefix(), "");
    assert_eq!(pkg.modules.len(), 1);
    assert_eq!(pkg.modules[0].path, "json/json");
    assert_eq!(pkg.modules[0].kind, ModuleKind::Src);
    assert_eq!(
        graph.sources.get(pkg.modules[0].source).display,
        "example.com/alice/json@1.4.0/src/json.i"
    );
    let expected_hash = hash(&[
        PackageFile {
            path: "i.toml".into(),
            kind: FileKind::Regular,
            bytes: package_toml("json", &[]).into_bytes(),
        },
        PackageFile {
            path: "src/json.i".into(),
            kind: FileKind::Regular,
            bytes: b"x = 1\n".to_vec(),
        },
    ]);
    assert_eq!(loc.hash, expected_hash);
    let text = fs::read_to_string(app.join("i.lock")).unwrap();
    assert_eq!(
        text,
        format!("example.com/alice/json 1.4.0 {c} {expected_hash}\n")
    );
    // Loading again changes nothing.
    let (graph, diags) = w.load(&app, true);
    no_errors(&graph, &diags);
    assert_eq!(fs::read_to_string(app.join("i.lock")).unwrap(), text);
    // A program with no packages with a location has no `i.lock`.
    let plain = w.project("plain", &[("i.toml", "name = \"plain\"\n")]);
    let (graph, diags) = w.load(&plain, true);
    no_errors(&graph, &diags);
    assert!(!plain.join("i.lock").exists());
}

#[test]
fn dir_packages() {
    let w = World::new();
    let tools = w.repo("bob/tools");
    let c = tools.commit(
        &[
            ("README", "tools\n"),
            ("text/i.toml", "name = \"text\"\n"),
            ("text/src/words.i", "x = 1\n"),
            ("num/i.toml", "name = \"num\"\n"),
            ("num/src/n.i", "x = 1\n"),
        ],
        "v0.3.2",
    );
    let app = w.project(
        "app",
        &[(
            "i.toml",
            "name = \"app\"\n[dependencies]\ntext = \"example.com/bob/tools//text@0.3.2\"\nnum = \"example.com/bob/tools//num@0.3.0\"\n",
        )],
    );
    // There is no tag v0.3.0.
    let (_, diags) = w.load(&app, true);
    assert_eq!(errors(&diags), [code::NO_VERSION]);
    assert!(!app.join("i.lock").exists());

    write(
        &app,
        &[(
            "i.toml",
            "name = \"app\"\n[dependencies]\ntext = \"example.com/bob/tools//text@0.3.2\"\nnum = \"example.com/bob/tools//num@0.3.2\"\n",
        )],
    );
    let (graph, diags) = w.load(&app, true);
    no_errors(&graph, &diags);
    let root = graph.root.unwrap();
    let text = dep(&graph, root, "text");
    assert_eq!(text.position_prefix(), "example.com/bob/tools//text@0 ");
    assert_eq!(text.modules[0].path, "text/words");
    assert_eq!(
        graph.sources.get(text.modules[0].source).display,
        "example.com/bob/tools//text@0.3.2/src/words.i"
    );
    // Only the files of the directory are the package.
    assert!(text.dir.as_ref().unwrap().join("i.toml").is_file());
    assert!(!text.dir.as_ref().unwrap().join("README").exists());
    let h = hash(&[
        PackageFile {
            path: "i.toml".into(),
            kind: FileKind::Regular,
            bytes: b"name = \"text\"\n".to_vec(),
        },
        PackageFile {
            path: "src/words.i".into(),
            kind: FileKind::Regular,
            bytes: b"x = 1\n".to_vec(),
        },
    ]);
    assert_eq!(text.location.as_ref().unwrap().hash, h);
    let lock = Lock::parse(&fs::read_to_string(app.join("i.lock")).unwrap()).unwrap();
    let names: Vec<&str> = lock.lines.iter().map(|l| l.location.as_str()).collect();
    assert_eq!(
        names,
        ["example.com/bob/tools//num", "example.com/bob/tools//text"]
    );
    assert!(lock.lines.iter().all(|l| l.commit == c));

    // A directory that is not in the repository.
    write(
        &app,
        &[(
            "i.toml",
            "name = \"app\"\n[dependencies]\nx = \"example.com/bob/tools//nothing@0.3.2\"\n",
        )],
    );
    let (graph, diags) = w.load(&app, true);
    assert_eq!(errors(&diags), [code::NO_PACKAGE]);
    let line = graph.sources.render(&diags[0]);
    assert!(line.starts_with("i.toml:3:1: error[E0621]: "), "{line}");
}

#[test]
fn diamond_and_highest_reached_version() {
    let w = World::new();
    let a = w.repo("x/a");
    let b = w.repo("x/b");
    let c = w.repo("x/c");
    let d = w.repo("x/d");
    let e = w.repo("x/e");
    c.package("c", &[], "1.1.0");
    c.package("c", &[], "1.2.0");
    // Never asked for, so never used.
    c.package("c", &[], "1.3.0");
    d.package("d", &[], "1.2.0");
    d.package("d", &[], "1.5.0");
    e.package("e", &[], "0.1.0");
    a.package(
        "a",
        &[
            "c = \"example.com/x/c@1.1.0\"",
            "d = \"example.com/x/d@1.5.0\"",
            "e = \"example.com/x/e@0.1.0\"",
        ],
        "1.0.0",
    );
    a.package("a", &["d = \"example.com/x/d@1.2.0\""], "1.1.0");
    b.package(
        "b",
        &[
            "c = \"example.com/x/c@1.2.0\"",
            "a = \"example.com/x/a@1.1.0\"",
        ],
        "1.0.0",
    );
    let app = w.project(
        "app",
        &[(
            "i.toml",
            "name = \"app\"\n[dependencies]\na = \"example.com/x/a@1.0.0\"\nb = \"example.com/x/b@1.0.0\"\n",
        )],
    );
    let (graph, diags) = w.load(&app, true);
    no_errors(&graph, &diags);
    // `a@1.0.0` is not used, but its dependencies are reached: `d` is
    // 1.5.0, from its `i.toml`, and not 1.2.0.
    assert_eq!(
        located(&graph),
        [
            "example.com/x/a@1.1.0",
            "example.com/x/b@1.0.0",
            "example.com/x/c@1.2.0",
            "example.com/x/d@1.5.0",
        ]
    );
    let root = graph.root.unwrap();
    let a_id = graph.packages[root]
        .dependencies
        .iter()
        .find(|(n, _)| n == "a")
        .unwrap()
        .1
        .unwrap();
    let b_pkg = dep(&graph, root, "b");
    // One package for `a`, whoever asks for it.
    let b_a = b_pkg.dependencies.iter().find(|(n, _)| n == "a").unwrap();
    assert_eq!(b_a.1, Some(a_id));
    // `e` is reached only through `a@1.0.0`: it has a version, and a line.
    assert_eq!(
        lock_lines(&app),
        [
            "example.com/x/a 1.1.0",
            "example.com/x/b 1.0.0",
            "example.com/x/c 1.2.0",
            "example.com/x/d 1.5.0",
            "example.com/x/e 0.1.0",
        ]
    );
}

#[test]
fn two_majors_side_by_side() {
    let w = World::new();
    let json = w.repo("alice/json");
    json.package("json", &[], "1.4.0");
    json.package("json", &[], "1.6.0");
    json.package("json", &[], "2.0.1");
    let app = w.project(
        "app",
        &[(
            "i.toml",
            "name = \"app\"\n[dependencies]\nold = \"example.com/alice/json@1.4.0\"\nnew = \"example.com/alice/json@2.0.1\"\n",
        )],
    );
    let (graph, diags) = w.load(&app, true);
    no_errors(&graph, &diags);
    let root = graph.root.unwrap();
    let old = dep(&graph, root, "old");
    let new = dep(&graph, root, "new");
    assert_eq!(old.position_prefix(), "example.com/alice/json@1 ");
    assert_eq!(new.position_prefix(), "example.com/alice/json@2 ");
    assert_ne!(
        graph.packages[root].dependencies[0].1,
        graph.packages[root].dependencies[1].1
    );
    assert_eq!(old.name, new.name);
    assert_eq!(
        lock_lines(&app),
        [
            "example.com/alice/json 1.4.0",
            "example.com/alice/json 2.0.1"
        ]
    );
}

#[test]
fn moved_tag_is_an_error() {
    let w = World::new();
    let json = w.repo("alice/json");
    json.package("json", &[], "1.4.0");
    let app = w.project(
        "app",
        &[(
            "i.toml",
            "name = \"app\"\n[dependencies]\njson = \"example.com/alice/json@1.4.0\"\n",
        )],
    );
    let (graph, diags) = w.load(&app, true);
    no_errors(&graph, &diags);
    let locked = fs::read_to_string(app.join("i.lock")).unwrap();

    // The tag is moved to other files.
    json.commit(
        &[("i.toml", "name = \"json\"\n"), ("src/evil.i", "x = 2\n")],
        "v1.4.0",
    );
    // With a new cache, as on another computer.
    let mut options = w.options(true);
    options.cache = Some(w.path().join("cache2"));
    let (graph, diags) = load_with(&app, &options);
    assert_eq!(errors(&diags), [code::LOCK_MISMATCH]);
    let line = graph.sources.render(&diags[0]);
    assert!(line.starts_with("i.lock:1:1: error[E0632]: "), "{line}");
    // Nothing is compiled, and `i.lock` is not changed.
    let root = graph.root.unwrap();
    assert_eq!(graph.packages[root].dependencies[0].1, None);
    assert!(located(&graph).is_empty());
    assert_eq!(fs::read_to_string(app.join("i.lock")).unwrap(), locked);

    // The old cache still has the old tag, so it still loads, offline too.
    let mut options = w.options(true);
    options.offline = true;
    let (graph, diags) = load_with(&app, &options);
    no_errors(&graph, &diags);
    // Not offline, the old cache is used as it matches `i.lock`.
    let (graph, diags) = w.load(&app, true);
    no_errors(&graph, &diags);

    // A hash in `i.lock` that is not that of the files.
    let at = locked.find("sha256-").unwrap() + "sha256-".len();
    let bad = format!("{}{}\n", &locked[..at], "0".repeat(64));
    fs::write(app.join("i.lock"), &bad).unwrap();
    let (_, diags) = w.load(&app, true);
    assert_eq!(errors(&diags), [code::LOCK_MISMATCH]);
    assert_eq!(fs::read_to_string(app.join("i.lock")).unwrap(), bad);
}

#[test]
fn lines_are_added_and_taken_out() {
    let w = World::new();
    let json = w.repo("alice/json");
    json.package("json", &[], "1.4.0");
    json.package("json", &[], "1.5.0");
    let util = w.repo("alice/util");
    util.package("util", &[], "0.1.0");
    let app = w.project(
        "app",
        &[(
            "i.toml",
            "name = \"app\"\n[dependencies]\njson = \"example.com/alice/json@1.4.0\"\nutil = \"example.com/alice/util@0.1.0\"\n",
        )],
    );
    let (graph, diags) = w.load(&app, true);
    no_errors(&graph, &diags);
    assert_eq!(
        lock_lines(&app),
        [
            "example.com/alice/json 1.4.0",
            "example.com/alice/util 0.1.0"
        ]
    );
    // A line for a version that is not used, by hand.
    let mut text = fs::read_to_string(app.join("i.lock")).unwrap();
    text.push_str(&format!(
        "example.com/zed/gone 9.9.9 {} sha256-{}\n",
        "a".repeat(40),
        "b".repeat(64)
    ));
    fs::write(app.join("i.lock"), text).unwrap();
    // The version is raised, and `util` is no longer used.
    write(
        &app,
        &[(
            "i.toml",
            "name = \"app\"\n[dependencies]\njson = \"example.com/alice/json@1.5.0\"\n",
        )],
    );
    let (graph, diags) = w.load(&app, true);
    no_errors(&graph, &diags);
    assert_eq!(lock_lines(&app), ["example.com/alice/json 1.5.0"]);
    // No dependencies by location: every line is taken out.
    write(&app, &[("i.toml", "name = \"app\"\n")]);
    let (graph, diags) = w.load(&app, true);
    no_errors(&graph, &diags);
    assert_eq!(fs::read_to_string(app.join("i.lock")).unwrap(), "");
    // With `write_lock` off, nothing is written.
    write(
        &app,
        &[(
            "i.toml",
            "name = \"app\"\n[dependencies]\njson = \"example.com/alice/json@1.5.0\"\n",
        )],
    );
    let mut options = w.options(true);
    options.write_lock = false;
    let (graph, diags) = load_with(&app, &options);
    no_errors(&graph, &diags);
    assert_eq!(fs::read_to_string(app.join("i.lock")).unwrap(), "");
}

#[test]
fn bad_lock_file() {
    let w = World::new();
    let json = w.repo("alice/json");
    json.package("json", &[], "1.4.0");
    let app = w.project(
        "app",
        &[
            (
                "i.toml",
                "name = \"app\"\n[dependencies]\njson = \"example.com/alice/json@1.4.0\"\n",
            ),
            ("i.lock", "\nexample.com/alice/json 1.4.0 abc\n"),
        ],
    );
    let (graph, diags) = w.load(&app, true);
    assert_eq!(errors(&diags), [code::LOCK_SYNTAX]);
    let line = graph.sources.render(&diags[0]);
    assert!(line.starts_with("i.lock:2:1: error[E0633]: "), "{line}");
    assert!(located(&graph).is_empty());
}

#[test]
fn dev_dependencies_only_with_tests() {
    let w = World::new();
    let json = w.repo("alice/json");
    // The dev-dependencies of a package that is not compiled are not used:
    // there is no such repository.
    let toml = "name = \"json\"\n[dev-dependencies]\nx = \"example.com/nobody/nothing@1.0.0\"\n";
    json.commit(&[("i.toml", toml), ("src/json.i", "x = 1\n")], "v1.4.0");
    json.commit(&[("i.toml", toml), ("src/json.i", "x = 1\n")], "v1.5.0");
    let check = w.repo("carol/check");
    check.package(
        "check",
        &["json = \"example.com/alice/json@1.5.0\""],
        "1.0.0",
    );
    let app = w.project(
        "app",
        &[
            (
                "i.toml",
                "name = \"app\"\n[dependencies]\njson = \"example.com/alice/json@1.4.0\"\n[dev-dependencies]\ncheck = \"example.com/carol/check@1.0.0\"\n",
            ),
            ("src/main.i", "x = 1\n"),
            ("test/main_test.i", "x = 1\n"),
        ],
    );
    let (graph, diags) = w.load(&app, false);
    no_errors(&graph, &diags);
    assert_eq!(located(&graph), ["example.com/alice/json@1.4.0"]);
    let root = &graph.packages[graph.root.unwrap()];
    assert_eq!(root.dev_dependencies, [("check".to_string(), None)]);
    assert!(root.modules.iter().all(|m| m.kind == ModuleKind::Src));
    // The lines are for the program with and without its tests.
    let all = [
        "example.com/alice/json 1.4.0",
        "example.com/alice/json 1.5.0",
        "example.com/carol/check 1.0.0",
    ];
    assert_eq!(lock_lines(&app), all);

    let (graph, diags) = w.load(&app, true);
    no_errors(&graph, &diags);
    assert_eq!(
        located(&graph),
        [
            "example.com/alice/json@1.5.0",
            "example.com/carol/check@1.0.0"
        ]
    );
    let root = graph.root.unwrap();
    assert!(
        graph.packages[root]
            .modules
            .iter()
            .any(|m| m.kind == ModuleKind::Test)
    );
    assert_eq!(dep(&graph, root, "check").name, "check");
    assert_eq!(lock_lines(&app), all);
}

#[test]
fn dev_dependencies_through_path_packages() {
    let w = World::new();
    let json = w.repo("alice/json");
    json.package("json", &[], "1.4.0");
    json.package("json", &[], "1.7.0");
    let app = w.project(
        "app",
        &[(
            "i.toml",
            "name = \"app\"\n[dependencies]\njson = \"example.com/alice/json@1.4.0\"\n[dev-dependencies]\nhelp = { path = \"../help\" }\n",
        )],
    );
    w.project(
        "help",
        &[(
            "i.toml",
            "name = \"help\"\n[dependencies]\njson = \"example.com/alice/json@1.7.0\"\n[dev-dependencies]\nno = \"example.com/nobody/nothing@1.0.0\"\n",
        )],
    );
    let (graph, diags) = w.load(&app, false);
    no_errors(&graph, &diags);
    assert_eq!(located(&graph), ["example.com/alice/json@1.4.0"]);
    assert_eq!(
        lock_lines(&app),
        [
            "example.com/alice/json 1.4.0",
            "example.com/alice/json 1.7.0"
        ]
    );
    let (graph, diags) = w.load(&app, true);
    no_errors(&graph, &diags);
    assert_eq!(located(&graph), ["example.com/alice/json@1.7.0"]);
}

#[test]
fn package_circle() {
    let w = World::new();
    let a = w.repo("x/a");
    let b = w.repo("x/b");
    a.package("a", &["b = \"example.com/x/b@1.0.0\""], "1.0.0");
    b.package("b", &["a = \"example.com/x/a@1.0.0\""], "1.0.0");
    let app = w.project(
        "app",
        &[(
            "i.toml",
            "name = \"app\"\n[dependencies]\na = \"example.com/x/a@1.0.0\"\n",
        )],
    );
    let (graph, diags) = w.load(&app, true);
    assert_eq!(errors(&diags), [code::PACKAGE_CIRCLE]);
    let line = graph.sources.render(&diags[0]);
    assert!(
        line.starts_with("example.com/x/b@1.0.0/i.toml:3:1: error[E0622]: "),
        "{line}"
    );
}

#[test]
fn path_dependency_in_a_location_package() {
    let w = World::new();
    let a = w.repo("x/a");
    a.package("a", &["p = { path = \"../p\" }"], "1.0.0");
    let app = w.project(
        "app",
        &[(
            "i.toml",
            "name = \"app\"\n[dependencies]\na = \"example.com/x/a@1.0.0\"\n",
        )],
    );
    w.project("p", &[("i.toml", "name = \"p\"\n")]);
    let (graph, diags) = w.load(&app, true);
    assert_eq!(errors(&diags), [code::PATH_DEPENDENCY]);
    let line = graph.sources.render(&diags[0]);
    assert!(
        line.starts_with("example.com/x/a@1.0.0/i.toml:3:1: error[E0609]: "),
        "{line}"
    );
    let a = dep(&graph, graph.root.unwrap(), "a");
    assert_eq!(a.dependencies, [("p".to_string(), None)]);
}

#[test]
fn offline() {
    let w = World::new();
    let json = w.repo("alice/json");
    json.package("json", &[], "1.4.0");
    json.package("json", &[], "1.5.0");
    let app = w.project(
        "app",
        &[(
            "i.toml",
            "name = \"app\"\n[dependencies]\njson = \"example.com/alice/json@1.4.0\"\n",
        )],
    );
    let mut offline = w.options(true);
    offline.offline = true;
    // Nothing is in the cache.
    let (graph, diags) = load_with(&app, &offline);
    assert_eq!(errors(&diags), [code::OFFLINE]);
    let line = graph.sources.render(&diags[0]);
    assert!(line.starts_with("i.toml:3:1: error[E0634]: "), "{line}");
    assert!(!app.join("i.lock").exists());

    let (graph, diags) = w.load(&app, true);
    no_errors(&graph, &diags);
    // The repository is gone, so the network cannot be used: everything
    // is in the cache and matches `i.lock`.
    fs::remove_dir_all(&json.bare).unwrap();
    let (graph, diags) = w.load(&app, true);
    no_errors(&graph, &diags);
    let (graph, diags) = load_with(&app, &offline);
    no_errors(&graph, &diags);
    assert_eq!(located(&graph), ["example.com/alice/json@1.4.0"]);

    // A version that the cache has, as the tags were fetched.
    write(
        &app,
        &[(
            "i.toml",
            "name = \"app\"\n[dependencies]\njson = \"example.com/alice/json@1.5.0\"\n",
        )],
    );
    let (graph, diags) = load_with(&app, &offline);
    no_errors(&graph, &diags);
    assert_eq!(lock_lines(&app), ["example.com/alice/json 1.5.0"]);
    // A version that it does not have.
    write(
        &app,
        &[(
            "i.toml",
            "name = \"app\"\n[dependencies]\njson = \"example.com/alice/json@1.6.0\"\n",
        )],
    );
    let (_, diags) = load_with(&app, &offline);
    assert_eq!(errors(&diags), [code::OFFLINE]);
    // Online, the repository is gone.
    let (_, diags) = w.load(&app, true);
    assert_eq!(errors(&diags), [code::GIT]);
    assert_eq!(lock_lines(&app), ["example.com/alice/json 1.5.0"]);
}

#[test]
fn transitive_problems_say_where_they_come_from() {
    let w = World::new();
    let a = w.repo("x/a");
    a.package("a", &["b = \"example.com/x/b@1.0.0\""], "1.0.0");
    let b = w.repo("x/b");
    b.package("b", &[], "0.9.0");
    let app = w.project(
        "app",
        &[(
            "i.toml",
            "name = \"app\"\n[dependencies]\na = \"example.com/x/a@1.0.0\"\n",
        )],
    );
    let (graph, diags) = w.load(&app, true);
    assert_eq!(errors(&diags), [code::NO_VERSION]);
    let line = graph.sources.render(&diags[0]);
    assert!(
        line.contains("`example.com/x/b@1.0.0`")
            && line.contains("a dependency of `example.com/x/a@1.0.0`"),
        "{line}"
    );
    assert!(!app.join("i.lock").exists());
}

/// MP01: a committed link must not let an unchanged lock read outside bytes.
#[cfg(unix)]
#[test]
fn located_source_links_cannot_escape_the_hashed_tree() {
    use std::os::unix::fs::symlink;
    for link in ["src/lib.i", "src", "test", "i.toml"] {
        let w = World::new();
        let r = w.repo("dep");
        write(&r.work, &[("i.toml", "name = \"dep\"\n")]);
        let outside = w.project(
            "outside",
            &[(
                "lib.i",
                include_str!("../../../tests/spec/2.8/mp01_external.i"),
            )],
        );
        let target = if link.ends_with(".i") {
            outside.join("lib.i")
        } else if link == "i.toml" {
            write(&outside, &[("i.toml", "name = \"dep\"\n")]);
            fs::remove_file(r.work.join("i.toml")).unwrap();
            outside.join("i.toml")
        } else {
            outside.clone()
        };
        fs::create_dir_all(r.work.join(link).parent().unwrap()).unwrap();
        symlink(target, r.work.join(link)).unwrap();
        r.commit_as_is("v1.0.0");
        let app = w.project(
            "app",
            &[
                (
                    "i.toml",
                    "name = \"app\"\n[dependencies]\ndep = \"example.com/dep@1.0.0\"\n",
                ),
                ("src/main.i", "main = () { Nil }\n"),
            ],
        );
        let (g, ds) = w.load(&app, false);
        assert!(!errors(&ds).is_empty(), "link {link} was accepted");
        assert!(located(&g).is_empty());
        assert!(!app.join("i.lock").exists());
    }
}
