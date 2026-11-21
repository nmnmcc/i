//! `i new`, `i add`, `i remove`, `i update`, `i publish`, and `i deps`,
//! with packages got from local bare repositories through `file://` URLs
//! (`ILANG_GIT_URLS`) and a cache in a temporary directory (`ILANG_CACHE`).
//! Nothing here uses the network.

#[path = "common/syn3_pending.rs"]
mod syn3_pending;
#[path = "common/test_project.rs"]
mod test_project;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use syn3_pending::syn3_pending;

/// Sets the environment of git: no user configuration, a fixed author and
/// a fixed date.
fn git_env(cmd: &mut Command) -> &mut Command {
    cmd.env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_AUTHOR_DATE", "2026-01-01T00:00:00Z")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_DATE", "2026-01-01T00:00:00Z")
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = git_env(&mut Command::new("git"))
        .current_dir(dir)
        .args([
            "-c",
            "init.defaultBranch=main",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "tag.gpgsign=false",
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

fn write(dir: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let p = dir.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
}

/// A temporary directory with bare repositories under `remotes/`, which the
/// locations `example.com/...` are got from, a cache, and projects.
struct World {
    tmp: tempfile::TempDir,
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

    /// `i` in the world's directory, with its cache and URLs: `example.com/`
    /// from `remotes/`, and the standard library's repository
    /// `github.com/nmnmcc/i` from `remotes/nmnmcc-i` (see [`World::library`]).
    fn command(&self) -> Command {
        let remotes = self.path().join("remotes");
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_i"));
        git_env(&mut cmd)
            .current_dir(self.path())
            .env("ILANG_CACHE", self.path().join("cache"))
            .env(
                "ILANG_GIT_URLS",
                format!(
                    "example.com/=file://{remotes}/ {LIBRARY_REPOSITORY}=file://{remotes}/{LIBRARY_REMOTE}",
                    remotes = remotes.display()
                ),
            )
            .env_remove("ILANG_OFFLINE");
        cmd
    }

    /// Runs `i` in the world's directory, with its cache and URLs.
    fn i(&self, args: &[&str]) -> Output {
        self.command().args(args).output().unwrap()
    }

    /// Runs `i` as [`World::i`] does, with `env` set as well.
    fn i_with(&self, args: &[&str], env: &[(&str, &std::ffi::OsStr)]) -> Output {
        self.command()
            .envs(env.iter().copied())
            .args(args)
            .output()
            .unwrap()
    }

    /// Runs `i` as [`World::i`], offline.
    fn i_offline(&self, args: &[&str]) -> Output {
        self.command()
            .env("ILANG_OFFLINE", "1")
            .args(args)
            .output()
            .unwrap()
    }

    /// The repository of the standard library, `github.com/nmnmcc/i`, got
    /// from `remotes/nmnmcc-i`: the package `//stdlib`, which is this
    /// repository's `stdlib/`, tagged `stdlib-v0.1.0` and then, later and
    /// at an older version, `stdlib-v0.0.9`; and a root tag `v9.9.9`, which
    /// is not a version of `//stdlib`. The newest version is 0.1.0.
    fn library(&self) {
        self.repo(LIBRARY_REMOTE, &[]);
        let files = library_files();
        let files: Vec<(&str, &str)> = files
            .iter()
            .map(|(path, text)| (path.as_str(), text.as_str()))
            .collect();
        self.tag_as(LIBRARY_REMOTE, "stdlib-v0.1.0", &files);
        self.tag_as(LIBRARY_REMOTE, "stdlib-v0.0.9", &files);
        self.tag_as(
            LIBRARY_REMOTE,
            "v9.9.9",
            &[
                ("i.toml", "name = \"root\"\n"),
                ("src/m.i", " (x)\n\nx = 1\n"),
            ],
        );
    }

    /// Runs `i` and checks that it succeeds; gives its standard output.
    fn ok(&self, args: &[&str]) -> String {
        let out = self.i(args);
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        assert!(
            out.status.success(),
            "i {args:?} failed:\n{stdout}\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        stdout
    }

    /// Runs `i` and checks that it fails; gives its standard output and
    /// standard error together.
    fn fails(&self, args: &[&str]) -> String {
        let out = self.i(args);
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!out.status.success(), "i {args:?} should fail:\n{text}");
        text
    }

    /// A repository for the location `example.com/<name>`, with each
    /// version's files committed and tagged `v<version>`.
    fn repo(&self, name: &str, versions: &[(&str, &[(&str, &str)])]) {
        let work = self.path().join("work").join(name);
        let bare = self.path().join("remotes").join(name);
        fs::create_dir_all(&work).unwrap();
        fs::create_dir_all(&bare).unwrap();
        git(&work, &["init", "-q"]);
        git(&bare, &["init", "-q", "--bare"]);
        for (version, files) in versions {
            self.tag(name, version, files);
        }
    }

    /// Adds a version to the repository of `example.com/<name>`.
    fn tag(&self, name: &str, version: &str, files: &[(&str, &str)]) {
        self.tag_as(name, &format!("v{version}"), files);
    }

    /// Commits exactly `files` in the repository of `example.com/<name>`,
    /// tags the commit `tag`, and pushes both.
    fn tag_as(&self, name: &str, tag: &str, files: &[(&str, &str)]) {
        let work = self.path().join("work").join(name);
        let bare = self.path().join("remotes").join(name);
        git(&work, &["rm", "-rq", "--ignore-unmatch", "."]);
        write(&work, files);
        git(&work, &["add", "-A"]);
        git(&work, &["commit", "-q", "--allow-empty", "-m", tag]);
        git(&work, &["tag", tag]);
        let bare = bare.display().to_string();
        git(&work, &["push", "-q", "-f", &bare, "HEAD:refs/heads/main"]);
        git(&work, &["push", "-q", "-f", &bare, "--tags"]);
    }

    /// Writes a project and gives its directory.
    fn project(&self, name: &str, files: &[(&str, &str)]) -> PathBuf {
        let dir = self.path().join(name);
        write(&dir, files);
        dir
    }
}

/// The repository of the standard library, and where [`World::library`]
/// puts it under `remotes/`.
const LIBRARY_REPOSITORY: &str = "github.com/nmnmcc/i";
const LIBRARY_REMOTE: &str = "nmnmcc-i";
/// The version of the standard library that `i new` resolves in a world
/// with [`World::library`].
const LIBRARY_NEWEST: &str = "0.1.0";

/// The `[dependencies]` line that makes this repository's `stdlib/` the
/// dependency `i` by path, as every compiled package needs (E0641). A
/// package got by location may not have it.
fn stdlib_dependency() -> String {
    format!(
        "i = {{ path = {:?} }}\n",
        test_project::library_dir().to_str().unwrap()
    )
}

/// Makes the package in `dir` collect tests with `i/test` and have the
/// library `i` as a normal dependency by location, from [`World::library`]:
/// for a package that is published and then got by location, which may
/// not have a path dependency.
fn configure_tests_by_location(dir: &Path) {
    let path = dir.join("i.toml");
    let mut manifest: toml_edit::DocumentMut = read(&path).parse().unwrap();
    manifest["test"]["collection"] = toml_edit::value("i/test.tests");
    manifest["dependencies"]["i"] =
        toml_edit::value(format!("{LIBRARY_REPOSITORY}//stdlib@{LIBRARY_NEWEST}"));
    fs::write(path, manifest.to_string()).unwrap();
}

/// This repository's `stdlib/`, as files under `stdlib/` of a repository:
/// its `i.toml`, licence and every file of `src/`, `test/` and `bench/`,
/// without build output.
fn library_files() -> Vec<(String, String)> {
    let source = test_project::library_dir();
    let mut files = Vec::new();
    for relative in ["i.toml", "UNICODE-LICENSE"] {
        files.push((format!("stdlib/{relative}"), read(&source.join(relative))));
    }
    for dir in ["src", "test", "bench"] {
        if !source.join(dir).is_dir() {
            continue;
        }
        let mut found = Vec::new();
        files_under(&source.join(dir), Path::new(dir), &mut found);
        for relative in found {
            if relative.components().any(|c| c.as_os_str() == "build") {
                continue;
            }
            let path = relative.to_str().unwrap().replace('\\', "/");
            files.push((format!("stdlib/{path}"), read(&source.join(&relative))));
        }
    }
    files
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

/// The package `lib`: v1.0.0 has a function, v1.1.0 adds a host service
/// and an external function, v1.2.0 is the same, and v2.0.0 is another major
/// version.
fn lib_repo(w: &World) {
    const PLAIN: &str = " (twice)\n\ntwice = (n : Int) { n * 2 }\n";
    const WITH_HOST: &str = r#"(
	twice
	Net
)

@host("lib:net/v1")
Net = (get = (String) -> String)

@external("env", "now")
now : () -> Int

twice = (n : Int) { n * 2 }
"#;
    w.repo(
        "lib",
        &[
            (
                "1.0.0",
                &[("i.toml", "name = \"lib\"\n"), ("src/math.i", PLAIN)],
            ),
            (
                "1.1.0",
                &[("i.toml", "name = \"lib\"\n"), ("src/math.i", WITH_HOST)],
            ),
            (
                "1.2.0",
                &[
                    ("i.toml", "name = \"lib\"\n# 1.2\n"),
                    ("src/math.i", WITH_HOST),
                ],
            ),
            (
                "2.0.0",
                &[("i.toml", "name = \"lib\"\n"), ("src/math.i", PLAIN)],
            ),
        ],
    );
}

/// The manifest of the package `app`, which depends on the library `i`.
fn app_toml() -> String {
    format!(
        "# The app.\nname = \"app\"\n\n[dependencies] # what it uses\n{}",
        stdlib_dependency()
    )
}

fn app(w: &World) -> PathBuf {
    w.project(
        "app",
        &[("i.toml", &app_toml()), ("src/main.i", " (x)\n\nx = 1\n")],
    )
}

// ---- i new ----

#[test]
fn new_makes_a_package_that_checks_runs_and_tests() {
    let w = World::new();
    w.library();
    let out = w.ok(&["new", "demo"]);
    assert!(out.contains("demo"), "{out}");
    let dir = w.path().join("demo");
    assert_eq!(
        read(&dir.join("i.toml")),
        test_project::new_manifest("demo", LIBRARY_NEWEST)
    );
    assert_eq!(read(&dir.join(".gitignore")), "/build/\n");
    assert!(read(&dir.join("src/main.i")).contains("(Io) = i/io"));
    assert!(read(&dir.join("src/main.i")).contains("main = () <- (io : Io)"));
    assert!(read(&dir.join("test/main_test.i")).contains("test(\"greets\""));

    let project = dir.display().to_string();
    w.ok(&["check", "--project", &project]);
    let out = w.ok(&["run", "--project", &project]);
    assert_eq!(out, "Hello, demo!\n");
    let out = w.ok(&["test", "--project", &project]);
    assert!(out.contains("1 passed, 0 failed"), "{out}");
}

/// Two packages made by `i new` both print with `Io` of `i/io`, so one may
/// depend on the other: neither declares the standard service. The
/// dependency is still shown as able to print.
#[test]
fn two_new_packages_work_together() {
    let w = World::new();
    w.library();
    w.ok(&["new", "greet"]);
    w.ok(&["new", "app"]);
    let app = w.path().join("app");
    let project = app.display().to_string();
    let greet = w.path().join("greet").display().to_string();
    let out = w.ok(&["add", "greet", "--path", &greet, "--project", &project]);
    assert!(
        out.contains(
            "new permission: greet (path ../greet) can use host service \"i:io/v1\" (greet/main imports i/io.Io)"
        ),
        "{out}"
    );
    w.ok(&["check", "--project", &project]);
    w.ok(&["run", "--project", &project]);

    // `app` uses `greet`.
    write(
        &app,
        &[(
            "src/main.i",
            "(\n\t(Io) = i/io\n\tgreet = greet/main\n)\n\n(\n\tgreeting\n)\n\ngreeting = (who : String) { greet.greeting(who) }\n\nmain = () <- (io : Io) { io.print(greeting(\"i\")) }\n",
        )],
    );
    let out = w.ok(&["check", "--project", &project]);
    assert_eq!(out, "", "no warnings");
    w.ok(&["build", "--project", &project]);
    let out = w.ok(&["run", "--project", &project]);
    assert_eq!(out, "Hello, i!\n");
    let out = w.ok(&["test", "--project", &project]);
    assert!(out.contains("1 passed, 0 failed"), "{out}");
    let out = w.ok(&["deps", "--project", &project]);
    assert!(
        out.contains(
            "greet (path ../greet)\n  host service \"i:io/v1\" (greet/main imports i/io.Io)\n"
        ),
        "{out}"
    );
}

/// The contents of the N-th fenced `text` block in a Markdown page.
fn text_block(markdown: &str, want: usize) -> &str {
    let opening = "```text\n";
    let mut rest = markdown;
    let mut seen = 0;
    loop {
        let start = rest
            .find(opening)
            .unwrap_or_else(|| panic!("README has no text block {want}"))
            + opening.len();
        let block = &rest[start..];
        let end = block
            .find("\n```")
            .unwrap_or_else(|| panic!("README text block {want} is not closed"));
        seen += 1;
        if seen == want {
            return &block[..end];
        }
        rest = &block[end + 1..];
    }
}

/// The files of a package made by `i new`, as paths relative to its
/// directory, sorted.
fn new_package_files(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    files_under(dir, Path::new(""), &mut files);
    let mut files: Vec<String> = files
        .iter()
        .map(|p| p.to_str().unwrap().replace('\\', "/"))
        .collect();
    files.sort();
    files
}

/// The dependency names of the table `table` of the manifest `text`.
fn dependency_names(text: &str, table: &str) -> Vec<String> {
    let manifest: toml_edit::DocumentMut = text.parse().unwrap();
    manifest
        .get(table)
        .and_then(|t| t.as_table_like())
        .map(|t| t.iter().map(|(k, _)| k.to_string()).collect())
        .unwrap_or_default()
}

#[test]
fn new_package_is_portable_and_runs_the_readme_program() {
    let w = World::new();
    w.library();
    w.ok(&["new", "demo"]);
    let dir = w.path().join("demo");
    let manifest = read(&dir.join("i.toml"));
    // No machine path and no copied library: the library `i` is an
    // ordinary dependency by location, at the newest `stdlib-v*` tag.
    assert_eq!(manifest, test_project::new_manifest("demo", LIBRARY_NEWEST));
    assert!(
        manifest.contains("\ni = \"github.com/nmnmcc/i//stdlib@0.1.0\"\n"),
        "{manifest}"
    );
    assert!(!manifest.contains(env!("CARGO_MANIFEST_DIR")), "{manifest}");
    assert!(!manifest.contains("path"), "{manifest}");
    assert_eq!(dependency_names(&manifest, "dependencies"), ["i"]);
    assert!(dependency_names(&manifest, "dev-dependencies").is_empty());
    assert!(!manifest.contains("std ="), "{manifest}");
    assert_eq!(
        new_package_files(&dir),
        [".gitignore", "i.toml", "src/main.i", "test/main_test.i"]
    );
    assert!(!dir.join("std").exists());
    assert!(!dir.join("stdlib").exists());
    assert!(!dir.join("i").exists());

    // The first README example imports `i/int` from a program, which
    // resolves through the dependency `i`.
    let readme = include_str!("../../../README.md");
    write(&dir, &[("src/main.i", text_block(readme, 1))]);
    let out = w.ok(&["run", "--project", &dir.display().to_string()]);
    assert_eq!(out, "42\n");
}

#[test]
fn new_package_tests_the_readme_service_example() {
    let w = World::new();
    w.library();
    w.ok(&["new", "svc"]);
    let dir = w.path().join("svc");
    let readme = include_str!("../../../README.md");
    write(&dir, &[("test/clock_test.i", text_block(readme, 2))]);
    let out = w.ok(&["test", "--project", &dir.display().to_string()]);
    assert!(
        out.contains("pass svc/clock_test.a test gives its own clock"),
        "{out}"
    );
    assert!(out.contains("0 failed"), "{out}");
}

/// `std` is an ordinary package name now: the package depends on the
/// library `i` like any other.
#[test]
fn new_package_named_std_is_an_ordinary_package() {
    let w = World::new();
    w.library();
    w.ok(&["new", "std"]);
    let dir = w.path().join("std");
    let manifest = read(&dir.join("i.toml"));
    assert_eq!(manifest, test_project::new_manifest("std", LIBRARY_NEWEST));
    assert_eq!(dependency_names(&manifest, "dependencies"), ["i"]);
    assert!(!manifest.contains("stdlib ="), "{manifest}");
    assert_eq!(
        new_package_files(&dir),
        [".gitignore", "i.toml", "src/main.i", "test/main_test.i"]
    );
    let test = read(&dir.join("test/main_test.i"));
    assert!(test.contains("i/test"), "{test}");
    assert!(test.contains("std/main"), "{test}");

    let project = dir.display().to_string();
    w.ok(&["check", "--project", &project]);
    let out = w.ok(&["run", "--project", &project]);
    assert_eq!(out, "Hello, std!\n");
    let out = w.ok(&["test", "--project", &project]);
    assert!(out.contains("1 passed, 0 failed"), "{out}");
}

/// The library is got by location and locked; the files that a package
/// made by `i new` uses are this repository's `stdlib/`, with its licence
/// and the notices in its sources, though nothing of it is copied.
#[test]
fn new_package_gets_the_library_by_location() {
    let w = World::new();
    w.library();
    w.ok(&["new", "demo"]);
    let dir = w.path().join("demo");
    w.ok(&["check", "--project", &dir.display().to_string()]);
    let lock = read(&dir.join("i.lock"));
    let commit = git(
        &w.path().join("work").join(LIBRARY_REMOTE),
        &["rev-parse", "stdlib-v0.1.0^{commit}"],
    );
    assert!(
        lock.starts_with(&format!(
            "github.com/nmnmcc/i//stdlib 0.1.0 {commit} sha256-"
        )),
        "{lock}"
    );
    assert_eq!(lock.lines().count(), 1, "{lock}");

    // The package in the cache: exactly the files of `stdlib/`, at the
    // root of the package.
    let place = w.path().join("cache").join("pkg");
    let places: Vec<_> = fs::read_dir(&place)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    assert_eq!(places.len(), 1, "{places:?}");
    let package = places[0].join(&commit);
    assert!(package.is_dir(), "{}", package.display());
    let source = test_project::library_dir();
    let mut expected: Vec<String> = library_files()
        .into_iter()
        .map(|(path, _)| path.strip_prefix("stdlib/").unwrap().to_string())
        .collect();
    expected.sort();
    assert_eq!(new_package_files(&package), expected);
    for relative in &expected {
        assert_eq!(
            read(&package.join(relative)),
            read(&source.join(relative)),
            "{relative}"
        );
    }
    // The licence and the notices inside the sources are there unchanged.
    assert_eq!(
        read(&package.join("UNICODE-LICENSE")),
        include_str!("../../../stdlib/UNICODE-LICENSE")
    );
    assert!(read(&package.join("src/float.i")).contains("BSD-style license"));
    assert!(read(&package.join("src/regex.i")).contains("Unicode license"));
}

/// `i new` resolves the newest `stdlib-vX.Y.Z` when it makes the package:
/// a newer tag is used by the next `i new`, and the packages already made
/// keep their version.
#[test]
fn new_resolves_the_newest_library_tag_each_time() {
    let w = World::new();
    w.library();
    w.ok(&["new", "first"]);
    let files = library_files();
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, text)| (path.as_str(), text.as_str()))
        .collect();
    w.tag_as(LIBRARY_REMOTE, "stdlib-v0.2.0", &files);
    w.ok(&["new", "second"]);
    assert_eq!(
        read(&w.path().join("first/i.toml")),
        test_project::new_manifest("first", "0.1.0")
    );
    assert_eq!(
        read(&w.path().join("second/i.toml")),
        test_project::new_manifest("second", "0.2.0")
    );
}

/// Offline, `i new` uses the tags in the cache; with none there, or when
/// the repository cannot be reached, it fails and makes nothing.
#[test]
fn new_needs_the_library_tags_and_otherwise_makes_nothing() {
    let w = World::new();
    // No repository at all.
    let err = w.fails(&["new", "demo"]);
    assert!(err.contains("standard library"), "{err}");
    assert!(err.contains("github.com/nmnmcc/i//stdlib"), "{err}");
    assert!(err.contains("no package was made"), "{err}");
    assert!(!w.path().join("demo").exists());

    // Offline with an empty cache.
    w.library();
    let out = w.i_offline(&["new", "demo"]);
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!out.status.success(), "{err}");
    assert!(err.contains("offline"), "{err}");
    assert!(err.contains("no package was made"), "{err}");
    assert!(!w.path().join("demo").exists());

    // Online once, then offline from the cache.
    w.ok(&["new", "demo"]);
    let out = w.i_offline(&["new", "again"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        read(&w.path().join("again/i.toml")),
        test_project::new_manifest("again", LIBRARY_NEWEST)
    );
}

/// Every regular file under `dir`, as paths relative to `dir`.
fn files_under(dir: &std::path::Path, prefix: &std::path::Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let relative = prefix.join(path.file_name().unwrap());
        if path.is_dir() {
            files_under(&path, &relative, out);
        } else {
            out.push(relative);
        }
    }
}

#[test]
fn new_with_a_path_and_bad_names() {
    let w = World::new();
    w.library();
    w.ok(&["new", "tool", "--path", "nested/place"]);
    assert_eq!(
        read(&w.path().join("nested/place/i.toml")),
        test_project::new_manifest("tool", LIBRARY_NEWEST)
    );
    // The directory is not empty now.
    let err = w.fails(&["new", "tool", "--path", "nested/place"]);
    assert!(err.contains("not empty"), "{err}");
    for bad in ["Big", "1x", "__bad", "a-b"] {
        let err = w.fails(&["new", bad]);
        assert!(err.contains("must be a name"), "{bad}: {err}");
        assert!(!w.path().join(bad).exists(), "{bad}");
    }
    // `i` is the standard library.
    let err = w.fails(&["new", "i"]);
    assert!(
        err.contains("the package name `i` is reserved for the standard library"),
        "{err}"
    );
    assert!(!w.path().join("i").exists());
}

// ---- i add ----

#[test]
fn add_takes_the_newest_tag_and_keeps_comments() {
    let w = World::new();
    lib_repo(&w);
    let dir = app(&w);
    let project = dir.display().to_string();
    let out = w.ok(&["add", "lib", "example.com/lib", "--project", &project]);
    assert!(out.contains("added `lib = example.com/lib@2.0.0`"), "{out}");
    assert_eq!(
        read(&dir.join("i.toml")),
        format!("{}lib = \"example.com/lib@2.0.0\"\n", app_toml())
    );
    let lock = read(&dir.join("i.lock"));
    assert!(lock.starts_with("example.com/lib 2.0.0 "), "{lock}");
    assert_eq!(lock.lines().count(), 1);

    // With a version, into `[dev-dependencies]`, and the permissions are new.
    let out = w.ok(&[
        "add",
        "old",
        "example.com/lib@1.1.0",
        "--dev",
        "--project",
        &project,
    ]);
    assert!(
        out.contains(
            "new permission: example.com/lib 1.1.0 can use host service \"lib:net/v1\" (lib/math.Net)"
        ),
        "{out}"
    );
    assert!(
        out.contains("new permission: example.com/lib 1.1.0 can use external \"env\" \"now\""),
        "{out}"
    );
    let toml = read(&dir.join("i.toml"));
    assert!(
        toml.ends_with("\n[dev-dependencies]\nold = \"example.com/lib@1.1.0\"\n"),
        "{toml}"
    );
    let lock = read(&dir.join("i.lock"));
    assert!(lock.contains("example.com/lib 1.1.0 "), "{lock}");
    assert!(lock.contains("example.com/lib 2.0.0 "), "{lock}");
}

#[test]
fn add_a_path_dependency() {
    let w = World::new();
    let dir = app(&w);
    w.project(
        "util",
        &[
            ("i.toml", "name = \"util\"\n"),
            ("src/text.i", " (hi)\n\nhi = \"hi\"\n"),
        ],
    );
    let project = dir.display().to_string();
    let util = w.path().join("util").display().to_string();
    let out = w.ok(&["add", "util", "--path", &util, "--project", &project]);
    assert!(
        out.contains("added `util = { path = \"../util\" }`"),
        "{out}"
    );
    let toml = read(&dir.join("i.toml"));
    assert!(toml.contains("util = { path = \"../util\" }\n"), "{toml}");
    assert!(!dir.join("i.lock").exists());
    // The package uses it.
    write(
        &dir,
        &[("src/main.i", " (util/text)\n (x)\n\nx = text.hi\n")],
    );
    w.ok(&["check", "--project", &project]);

    let err = w.fails(&["add", "nothing", "--path", "nowhere", "--project", &project]);
    assert!(err.contains("there is no directory"), "{err}");
}

#[test]
fn add_checks_the_names() {
    let w = World::new();
    lib_repo(&w);
    let dir = app(&w);
    let project = dir.display().to_string();
    for bad in ["i", "app", "Lib", "bad-name"] {
        let err = w.fails(&["add", bad, "example.com/lib@1.0.0", "--project", &project]);
        assert!(
            err.contains("not `i`, and not the name of this package"),
            "{bad}: {err}"
        );
    }
    w.ok(&["add", "lib", "example.com/lib@1.0.0", "--project", &project]);
    let err = w.fails(&[
        "add",
        "lib",
        "example.com/lib@1.0.0",
        "--dev",
        "--project",
        &project,
    ]);
    assert!(err.contains("already in `[dependencies]`"), "{err}");
    let err = w.fails(&[
        "add",
        "x",
        "https://example.com/lib@1.0.0",
        "--project",
        &project,
    ]);
    assert!(err.contains("without `https://`"), "{err}");
}

#[test]
fn add_puts_i_toml_back_when_loading_fails() {
    let w = World::new();
    lib_repo(&w);
    let dir = app(&w);
    let project = dir.display().to_string();
    let err = w.fails(&["add", "lib", "example.com/lib@1.9.0", "--project", &project]);
    assert!(err.contains("E0631"), "{err}");
    assert_eq!(read(&dir.join("i.toml")), app_toml());
    assert!(!dir.join("i.lock").exists());
    let err = w.fails(&["add", "none", "example.com/none", "--project", &project]);
    assert!(err.contains("could not be fetched"), "{err}");
    assert_eq!(read(&dir.join("i.toml")), app_toml());
}

// ---- i remove ----

#[test]
fn remove_takes_the_entry_and_the_lock_line_out() {
    let w = World::new();
    lib_repo(&w);
    let dir = app(&w);
    let project = dir.display().to_string();
    w.ok(&["add", "lib", "example.com/lib@1.0.0", "--project", &project]);
    w.ok(&[
        "add",
        "two",
        "example.com/lib@2.0.0",
        "--dev",
        "--project",
        &project,
    ]);
    assert_eq!(read(&dir.join("i.lock")).lines().count(), 2);

    let out = w.ok(&["remove", "two", "--project", &project]);
    assert!(
        out.contains("removed `two` from [dev-dependencies]"),
        "{out}"
    );
    let lock = read(&dir.join("i.lock"));
    assert!(lock.starts_with("example.com/lib 1.0.0 "), "{lock}");
    assert_eq!(lock.lines().count(), 1);

    w.ok(&["remove", "lib", "--project", &project]);
    assert_eq!(read(&dir.join("i.lock")), "");
    let toml = read(&dir.join("i.toml"));
    assert!(toml.starts_with(&app_toml()), "{toml}");
    // The path of the library `i` ends in `stdlib`.
    assert!(!toml.contains("lib ="), "{toml}");
    assert!(!toml.contains("example.com/lib"), "{toml}");

    let err = w.fails(&["remove", "lib", "--project", &project]);
    assert!(err.contains("is not a dependency"), "{err}");
}

// ---- i update ----

#[test]
fn update_raises_within_the_major_version_and_shows_new_permissions() {
    let w = World::new();
    lib_repo(&w);
    let dir = app(&w);
    let project = dir.display().to_string();
    w.ok(&["add", "lib", "example.com/lib@1.0.0", "--project", &project]);
    w.ok(&[
        "add",
        "two",
        "example.com/lib@2.0.0",
        "--dev",
        "--project",
        &project,
    ]);

    let out = w.ok(&["update", "--project", &project]);
    assert!(
        out.contains("updated lib: example.com/lib 1.0.0 -> 1.2.0"),
        "{out}"
    );
    assert!(!out.contains("two:"), "{out}");
    assert!(
        out.contains(
            "new permission: example.com/lib 1.2.0 can use host service \"lib:net/v1\" (lib/math.Net)"
        ),
        "{out}"
    );
    assert!(
        out.contains(
            "new permission: example.com/lib 1.2.0 can use external \"env\" \"now\" (lib/math.now)"
        ),
        "{out}"
    );
    let toml = read(&dir.join("i.toml"));
    assert!(toml.contains("lib = \"example.com/lib@1.2.0\"\n"), "{toml}");
    assert!(toml.contains("two = \"example.com/lib@2.0.0\"\n"), "{toml}");
    let lock = read(&dir.join("i.lock"));
    assert!(lock.contains("example.com/lib 1.2.0 "), "{lock}");
    assert!(!lock.contains("example.com/lib 1.0.0 "), "{lock}");

    // Nothing more; a newer tag is found by name.
    let out = w.ok(&["update", "--project", &project]);
    assert!(out.contains("newest version"), "{out}");
    assert!(!out.contains("new permission"), "{out}");
    w.tag(
        "lib",
        "1.3.0",
        &[
            ("i.toml", "name = \"lib\"\n"),
            ("src/math.i", " (twice)\n\ntwice = (n : Int) { n + n }\n"),
        ],
    );
    let out = w.ok(&["update", "lib", "--project", &project]);
    assert!(
        out.contains("updated lib: example.com/lib 1.2.0 -> 1.3.0"),
        "{out}"
    );
    assert!(!out.contains("new permission"), "{out}");

    let err = w.fails(&["update", "nope", "--project", &project]);
    assert!(err.contains("`nope` is not a dependency"), "{err}");
}

// ---- i deps ----

#[test]
fn deps_lists_the_packages_and_their_permissions() {
    let w = World::new();
    lib_repo(&w);
    let dir = app(&w);
    let project = dir.display().to_string();
    w.project(
        "util",
        &[
            ("i.toml", "name = \"util\"\n"),
            (
                "src/clock.i",
                "@host(\"util:clock/v1\", \"async\")\nClock = (now = () -> Int)\n",
            ),
        ],
    );
    let util = w.path().join("util").display().to_string();
    w.ok(&["add", "lib", "example.com/lib@1.1.0", "--project", &project]);
    w.ok(&[
        "add",
        "two",
        "example.com/lib@2.0.0",
        "--dev",
        "--project",
        &project,
    ]);
    w.ok(&["add", "util", "--path", &util, "--project", &project]);
    let out = w.ok(&["deps", "--project", &project]);
    assert_eq!(
        out,
        "app (this package)\n  no host services and no external functions\n\
         example.com/lib 1.1.0\n  external \"env\" \"now\" (lib/math.now)\n  host service \"lib:net/v1\" (lib/math.Net)\n\
         example.com/lib 2.0.0 (dev)\n  no host services and no external functions\n\
         util (path ../util)\n  host service \"util:clock/v1\" (async) (util/clock.Clock)\n"
    );
}

/// A package and its dependency both set `hostcall = true`: `i add` shows it
/// as new, `i deps` shows it under both, and `i build` names both once, in
/// the same order, before it writes the artifact.
#[test]
fn deps_and_build_show_the_hostcall_switch() {
    let w = World::new();
    let dir = w.project(
        "app",
        &[
            ("i.toml", &format!("hostcall = true\n{}", app_toml())),
            ("src/main.i", " (x)\n\nx = 1\n"),
        ],
    );
    let project = dir.display().to_string();
    w.project(
        "util",
        &[
            ("i.toml", "name = \"util\"\nhostcall = true\n"),
            ("src/math.i", " (one)\n\none = 1\n"),
        ],
    );
    let util = w.path().join("util").display().to_string();
    let out = w.ok(&["add", "util", "--path", &util, "--project", &project]);
    assert!(
        out.contains("new permission: util (path ../util) can use hostcall\n"),
        "{out}"
    );
    let out = w.ok(&["deps", "--project", &project]);
    assert_eq!(
        out,
        "app (this package)\n  hostcall\n  no host services and no external functions\n\
         util (path ../util)\n  hostcall\n  no host services and no external functions\n"
    );
    let out = w.ok(&["build", "--target", "host", "--project", &project]);
    assert_eq!(
        out.lines()
            .filter(|l| l.starts_with("hostcall: "))
            .collect::<Vec<_>>(),
        ["hostcall: <this package>", "hostcall: path ../util"],
        "{out}"
    );
    // `i check` prints nothing about it; with the switch off nothing is shown.
    assert_eq!(w.ok(&["check", "--project", &project]), "");
    write(
        &dir,
        &[(
            "i.toml",
            &read(&dir.join("i.toml")).replace("hostcall = true\n", ""),
        )],
    );
    write(&w.path().join("util"), &[("i.toml", "name = \"util\"\n")]);
    let out = w.ok(&["build", "--target", "host", "--project", &project]);
    assert!(!out.contains("hostcall"), "{out}");
    let out = w.ok(&["deps", "--project", &project]);
    assert!(!out.contains("hostcall"), "{out}");
}

// ---- i publish ----

/// A package in its own git repository, with one commit.
fn published(w: &World, src: &str) -> PathBuf {
    let dir = w.project(
        "pub",
        &[
            ("i.toml", "name = \"pub\"\n"),
            (".gitignore", "/build/\n"),
            ("src/api.i", src),
            (
                "test/api_test.i",
                "(\n\ttestlib = (test) = i/test\n\tpub/api\n)\n\ntest(\"works\", () { api.one |> (1) { () } & (other) { panic(\"no\") } })\n",
            ),
        ],
    );
    test_project::configure_tests_only(&dir);
    git(&dir, &["init", "-q"]);
    commit(&dir, "first");
    dir
}

fn commit(dir: &Path, message: &str) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", message]);
}

fn tags(dir: &Path) -> String {
    git(dir, &["tag", "--list"])
}

const API_1: &str = "(\n\tone\n\tadd\n\tShape(..)\n\tBox\n)\n\none = 1\n\nadd = (a : Int, b : Int) { a + b }\n\nCircle = (r = Float)\nSquare = (side = Float)\nShape = Circle | Square\n\nBox(A) = (A)\n";

#[test]
fn publish_tags_a_first_version() {
    let w = World::new();
    let dir = published(&w, API_1);
    let project = dir.display().to_string();
    let out = w.ok(&["publish", "1.0.0", "--dry-run", "--project", &project]);
    assert!(out.contains("no earlier version"), "{out}");
    assert!(out.contains("dry run"), "{out}");
    assert_eq!(tags(&dir), "");
    let out = w.ok(&["publish", "1.0.0", "--project", &project]);
    assert!(out.contains("1 test passed"), "{out}");
    assert!(out.contains("made the tag v1.0.0"), "{out}");
    assert!(out.contains("git push origin v1.0.0"), "{out}");
    assert_eq!(tags(&dir), "v1.0.0");
    assert_eq!(
        git(&dir, &["rev-parse", "v1.0.0^{commit}"]),
        git(&dir, &["rev-parse", "HEAD"])
    );
    // Not newer than every tag.
    let err = w.fails(&["publish", "1.0.0", "--project", &project]);
    assert!(err.contains("must be newer than every tag"), "{err}");
    let err = w.fails(&["publish", "0.9.0", "--project", &project]);
    assert!(err.contains("must be newer than every tag"), "{err}");
}

/// `i publish` shows a package that turns `hostcall` on since its newest tag
/// as a new permission; keeping it on shows nothing.
#[test]
fn publish_shows_hostcall_as_a_new_permission() {
    let w = World::new();
    let dir = published(&w, API_1);
    let project = dir.display().to_string();
    let out = w.ok(&["publish", "1.0.0", "--project", &project]);
    assert!(!out.contains("hostcall"), "{out}");
    let toml = read(&dir.join("i.toml"));
    write(&dir, &[("i.toml", &format!("hostcall = true\n{toml}"))]);
    commit(&dir, "hostcall");
    let out = w.ok(&["publish", "1.0.1", "--project", &project]);
    assert!(
        out.contains("new permission: pub can use hostcall\n"),
        "{out}"
    );
    let out = w.ok(&["publish", "1.0.2", "--dry-run", "--project", &project]);
    assert!(!out.contains("hostcall"), "{out}");
}

/// A `BrokenPipe` from writing to a child process is a failure, not a
/// closed standard output: when `tar` exits without reading the earlier
/// version, `i publish` fails and makes no tag.
#[test]
fn publish_fails_when_tar_does_not_read_the_earlier_version() {
    use std::os::unix::fs::PermissionsExt as _;
    let w = World::new();
    let dir = published(&w, API_1);
    let project = dir.display().to_string();
    // More than a pipe holds, so writing it to a `tar` that has gone gives
    // `BrokenPipe` rather than filling the pipe's buffer.
    write(&dir, &[("data.txt", &"x".repeat(1 << 20))]);
    commit(&dir, "data");
    w.ok(&["publish", "1.0.0", "--project", &project]);
    write(&dir, &[("src/api.i", &format!("{API_1}\nhidden = 3\n"))]);
    commit(&dir, "internal");

    let bin = w.path().join("fake-bin");
    fs::create_dir_all(&bin).unwrap();
    let tar = bin.join("tar");
    fs::write(&tar, "#!/bin/sh\nexit 2\n").unwrap();
    fs::set_permissions(&tar, fs::Permissions::from_mode(0o755)).unwrap();
    let path = std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let out = w.i_with(
        &["publish", "1.0.1", "--project", &project],
        &[("PATH", &path)],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "publish should fail:\n{}{stderr}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(stderr.contains("tar"), "{stderr}");
    assert_eq!(tags(&dir), "v1.0.0");
}

#[test]
fn publish_needs_a_clean_tree_and_passing_tests() {
    let w = World::new();
    let dir = published(&w, API_1);
    let project = dir.display().to_string();
    write(&dir, &[("notes.txt", "draft\n")]);
    let err = w.fails(&["publish", "1.0.0", "--project", &project]);
    assert!(err.contains("not clean"), "{err}");
    assert!(err.contains("notes.txt"), "{err}");
    fs::remove_file(dir.join("notes.txt")).unwrap();

    write(&dir, &[("src/api.i", &API_1.replace("one = 1", "one = 2"))]);
    commit(&dir, "wrong");
    let err = w.fails(&["publish", "1.0.0", "--project", &project]);
    assert!(err.contains("FAIL pub/api_test.works"), "{err}");

    write(
        &dir,
        &[("src/api.i", &API_1.replace("one = 1", "one = \"x\" + 1"))],
    );
    commit(&dir, "broken");
    let err = w.fails(&["publish", "1.0.0", "--project", &project]);
    assert!(err.contains("does not check"), "{err}");
    assert_eq!(tags(&dir), "");
}

#[test]
fn publish_compares_the_interface() {
    if syn3_pending("package_commands::publish_compares_the_interface") {
        return;
    }
    let w = World::new();
    let dir = published(&w, API_1);
    let project = dir.display().to_string();
    w.ok(&["publish", "1.0.0", "--project", &project]);

    // No change: a patch version is enough.
    write(&dir, &[("src/api.i", &format!("{API_1}\nhidden = 3\n"))]);
    commit(&dir, "internal");
    let out = w.ok(&["publish", "1.0.1", "--project", &project]);
    assert!(
        out.contains("the public interface is the same as in v1.0.0"),
        "{out}"
    );

    // An addition needs a minor version.
    let added = API_1
        .replace("\tone\n", "\tone\n\ttwo\n")
        .replace("one = 1\n", "one = 1\n\ntwo = 2\n");
    write(&dir, &[("src/api.i", &added)]);
    commit(&dir, "two");
    let err = w.fails(&["publish", "1.0.2", "--project", &project]);
    assert!(err.contains("added value api.two: 2"), "{err}");
    assert!(err.contains("a new minor version, at least 1.1.0"), "{err}");
    let out = w.ok(&["publish", "1.1.0", "--dry-run", "--project", &project]);
    assert!(out.contains("something was added since v1.0.1"), "{out}");
    assert_eq!(tags(&dir), "v1.0.0\nv1.0.1");
    w.ok(&["publish", "1.1.0", "--project", &project]);

    // Shape is a union alias; Dot adds its own type and constructor.
    // Changing the alias and the function requires a new major.
    let changed = added
        .replace(
            "add = (a : Int, b : Int) { a + b }",
            "add = (a : Int, b : Int, c : Int) { a + b + c }",
        )
        .replace(
            "Shape = Circle | Square\n",
            "Dot = ()\nShape = Circle | Square | Dot\n",
        );
    write(&dir, &[("src/api.i", &changed)]);
    commit(&dir, "change");
    let err = w.fails(&["publish", "1.2.0", "--project", &project]);
    let changes = err
        .split_once("changes to the public interface since v1.1.0:\n")
        .expect("interface comparison header")
        .1
        .lines()
        .take_while(|line| line.starts_with(' '))
        .collect::<Vec<_>>();
    assert_eq!(
        changes,
        [
            "  changed type api.Shape: Shape = Circle | Square",
            "       to type api.Shape: Shape = Circle | Square | Dot",
            "  changed value api.add: (Int, Int) -> Int",
            "       to value api.add: (Int, Int, Int) -> Int",
            "  added constructors api.Dot: Dot",
            "  added type api.Dot: Dot",
        ],
        "{err}"
    );
    assert!(err.contains("a new major version, at least 2.0.0"), "{err}");
    assert_eq!(tags(&dir), "v1.0.0\nv1.0.1\nv1.1.0");
    let out = w.ok(&["publish", "2.0.0", "--dry-run", "--project", &project]);
    assert!(out.contains("dry run"), "{out}");
    assert_eq!(tags(&dir), "v1.0.0\nv1.0.1\nv1.1.0");

    // Removing a name, or hiding constructors, is incompatible too.
    // The exported alias still needs its member types exported.
    let hidden = changed
        .replace("Shape(..)", "Shape, Circle, Square, Dot")
        .replace("\ttwo\n", "");
    write(&dir, &[("src/api.i", &hidden)]);
    commit(&dir, "hide");
    let err = w.fails(&["publish", "1.2.0", "--project", &project]);
    assert!(err.contains("removed value api.two: 2"), "{err}");
    // v1.1.0 exported Circle and Square through Shape(..); Dot was never
    // published, so only those two constructors can be removed here.
    assert_eq!(
        err.lines()
            .filter(|line| line.starts_with("  removed constructors "))
            .collect::<Vec<_>>(),
        [
            "  removed constructors api.Circle: Circle(r: Float)",
            "  removed constructors api.Square: Square(side: Float)",
        ],
        "{err}"
    );
    let out = w.ok(&["publish", "2.0.0", "--project", &project]);
    assert!(out.contains("git push origin v2.0.0"), "{out}");
    assert_eq!(tags(&dir), "v1.0.0\nv1.0.1\nv1.1.0\nv2.0.0");
}

// ---- Regression tests ----

#[test]
fn new_makes_files_that_are_formatted() {
    let w = World::new();
    w.library();
    w.ok(&["new", "demo"]);
    let project = w.path().join("demo").display().to_string();
    w.ok(&["fmt", "--check", "--project", &project]);
}

/// The package `badmod`, whose module path is not correct: it is
/// found only after its location is got.
fn bad_repo(w: &World) {
    w.repo(
        "badmod",
        &[(
            "1.0.0",
            &[
                ("i.toml", "name = \"badmod\"\n"),
                ("src/Bad-Name.i", "x = 1\n"),
            ],
        )],
    );
}

#[test]
fn a_failed_add_or_update_leaves_i_toml_and_i_lock_as_they_were() {
    let w = World::new();
    lib_repo(&w);
    bad_repo(&w);
    let dir = app(&w);
    let project = dir.display().to_string();

    // With no `i.lock` before, there is none after.
    let err = w.fails(&[
        "add",
        "bm",
        "example.com/badmod@1.0.0",
        "--project",
        &project,
    ]);
    assert!(err.contains("E0623"), "{err}");
    assert_eq!(read(&dir.join("i.toml")), app_toml());
    assert!(!dir.join("i.lock").exists());

    // With an `i.lock`, it stays as it was.
    w.ok(&["add", "lib", "example.com/lib@1.0.0", "--project", &project]);
    let toml = read(&dir.join("i.toml"));
    let lock = read(&dir.join("i.lock"));
    let err = w.fails(&[
        "add",
        "bm",
        "example.com/badmod@1.0.0",
        "--project",
        &project,
    ]);
    assert!(err.contains("E0623"), "{err}");
    assert_eq!(read(&dir.join("i.toml")), toml);
    assert_eq!(read(&dir.join("i.lock")), lock);

    // `i update` to a version that does not load.
    w.tag(
        "lib",
        "1.5.0",
        &[
            ("i.toml", "name = \"lib\"\n"),
            ("src/Bad-Name.i", "x = 1\n"),
        ],
    );
    let err = w.fails(&["update", "--project", &project]);
    assert!(err.contains("E0623"), "{err}");
    assert_eq!(read(&dir.join("i.toml")), toml);
    assert_eq!(read(&dir.join("i.lock")), lock);
}

#[test]
fn update_and_add_keep_the_comments_of_an_entry() {
    let w = World::new();
    lib_repo(&w);
    let toml = &format!(
        "name = \"app\"\n\n[dependencies]\n# why we use lib\nlib = \"example.com/lib@1.0.0\" # pinned\n# the other one\nother = \"example.com/lib@2.0.0\"\n{}",
        stdlib_dependency()
    );
    let dir = w.project(
        "app",
        &[("i.toml", toml), ("src/main.i", " (x)\n\nx = 1\n")],
    );
    let project = dir.display().to_string();
    w.ok(&["update", "lib", "--project", &project]);
    assert_eq!(
        read(&dir.join("i.toml")),
        toml.replace("lib@1.0.0", "lib@1.2.0")
    );
    w.ok(&["add", "lib", "example.com/lib@1.1.0", "--project", &project]);
    assert_eq!(
        read(&dir.join("i.toml")),
        toml.replace("lib@1.0.0", "lib@1.1.0")
    );
}

#[test]
fn add_and_update_keep_crlf_line_ends() {
    let w = World::new();
    lib_repo(&w);
    let toml = &format!(
        "# The app.\r\nname = \"app\"\r\n\r\n[dependencies]\r\nlib = \"example.com/lib@1.0.0\"\r\n{}",
        stdlib_dependency().replace('\n', "\r\n")
    );
    let dir = w.project(
        "app",
        &[("i.toml", toml), ("src/main.i", " (x)\n\nx = 1\n")],
    );
    let project = dir.display().to_string();
    w.ok(&["update", "--project", &project]);
    let toml = toml.replace("lib@1.0.0", "lib@1.2.0");
    assert_eq!(read(&dir.join("i.toml")), toml);
    w.ok(&[
        "add",
        "two",
        "example.com/lib@2.0.0",
        "--dev",
        "--project",
        &project,
    ]);
    assert_eq!(
        read(&dir.join("i.toml")),
        format!("{toml}\r\n[dev-dependencies]\r\ntwo = \"example.com/lib@2.0.0\"\r\n")
    );
}

/// Writes `files` in the package `dir`, commits them, and runs
/// `i publish <version> --dry-run`: whether it passes, and what it printed.
fn try_publish(w: &World, dir: &Path, files: &[(&str, &str)], version: &str) -> (bool, String) {
    write(dir, files);
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "--allow-empty", "-m", version]);
    let project = dir.display().to_string();
    let out = w.i(&["publish", version, "--dry-run", "--project", &project]);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

/// A package `pub` with the modules `a` and `b`, each with its own type
/// `T`, and `src/api.i` as given, published as v1.0.0.
fn two_types(w: &World, name: &str, api: &str) -> PathBuf {
    let dir = w.project(
        name,
        &[
            ("i.toml", "name = \"pub\"\n"),
            (".gitignore", "/build/\n"),
            ("src/a.i", "(\n\tT(..)\n)\n\nT = { (Int) }\n"),
            ("src/b.i", "(\n\tT(..)\n)\n\nT = { (String) }\n"),
            ("src/api.i", api),
        ],
    );
    test_project::configure_tests_only(&dir);
    git(&dir, &["init", "-q"]);
    commit(&dir, "first");
    let project = dir.display().to_string();
    w.ok(&["publish", "1.0.0", "--project", &project]);
    dir
}

#[test]
fn publish_compares_types_by_what_they_are_not_by_their_names() {
    let w = World::new();
    // A field whose type is another type of the same name.
    let dir = two_types(
        &w,
        "field",
        "(\n\tpub/a\n\tpub/b\n)\n\n(\n\tW(..)\n)\n\nW = { (a.T) }\n",
    );
    let (ok, out) = try_publish(
        &w,
        &dir,
        &[(
            "src/api.i",
            "(\n\tpub/a\n\tpub/b\n)\n\n(\n\tW(..)\n)\n\nW = { (b.T) }\n",
        )],
        "1.0.1",
    );
    assert!(!ok, "{out}");
    assert!(out.contains("changed type api.W: W = (a.T)"), "{out}");
    assert!(out.contains("to type api.W: W = (b.T)"), "{out}");
    assert!(out.contains("a new major version"), "{out}");

    // An alias of another type of the same name.
    let dir = two_types(&w, "alias", " (pub/a, pub/b)\n (U)\n\nU = a.T\n");
    let (ok, out) = try_publish(
        &w,
        &dir,
        &[("src/api.i", " (pub/a, pub/b)\n (U)\n\nU = b.T\n")],
        "1.0.1",
    );
    assert!(!ok, "{out}");
    assert!(out.contains("changed type api.U"), "{out}");

    // The same text, but the import names are swapped.
    let dir = two_types(
        &w,
        "swap",
        " (x = pub/a, y = pub/b)\n (f)\n\nf = (v : x.T) { v }\n",
    );
    let (ok, out) = try_publish(
        &w,
        &dir,
        &[(
            "src/api.i",
            " (x = pub/b, y = pub/a)\n (f)\n\nf = (v : x.T) { v }\n",
        )],
        "1.0.1",
    );
    assert!(!ok, "{out}");
    assert!(out.contains("changed value api.f"), "{out}");
    assert!(out.contains("a new major version"), "{out}");

    // Renamed type variables and other names for the same imports are no
    // change.
    let dir = two_types(
        &w,
        "same",
        "(\n\tpub/a\n)\n\n(\n\tid\n\tf\n\tg\n)\n\nid = <A>(x : A) -> A { x }\n\nf = (v : a.T) { v }\n\ng = (v : a.T) { v }\n",
    );
    let (ok, out) = try_publish(
        &w,
        &dir,
        &[(
            "src/api.i",
            "(\n\taa = pub/a\n\t(T) = pub/a\n)\n\n(\n\tid\n\tf\n\tg\n)\n\nid = <B>(x : B) -> B { x }\n\nf = (v : aa.T) { v }\n\ng = (v : T) { v }\n",
        )],
        "1.0.1",
    );
    assert!(ok, "{out}");
    assert!(
        out.contains("the public interface is the same as in v1.0.0"),
        "{out}"
    );
}

#[test]
fn publish_sees_a_new_major_version_of_a_dependency() {
    if syn3_pending("package_commands::publish_sees_a_new_major_version_of_a_dependency") {
        return;
    }
    let w = World::new();
    let v = |ty: &str| format!(" (V(..))\n\nV = ({ty})\n");
    let (v1, v2) = (v("Int"), v("String"));
    w.repo(
        "dep",
        &[
            ("1.0.0", &[("i.toml", "name = \"dep\"\n"), ("src/v.i", &v1)]),
            ("2.0.0", &[("i.toml", "name = \"dep\"\n"), ("src/v.i", &v2)]),
        ],
    );
    let api = "(\n\tdep/v\n)\n\n(\n\tf\n\tW(..)\n)\n\nf = (x : v.V) { 1 }\n\nW = { (v.V) }\n";
    let dir = w.project(
        "pub",
        &[
            (
                "i.toml",
                "name = \"pub\"\n\n[dependencies]\ndep = \"example.com/dep@1.0.0\"\n",
            ),
            (".gitignore", "/build/\n"),
            ("src/api.i", api),
        ],
    );
    test_project::configure_tests_only(&dir);
    let project = dir.display().to_string();
    w.ok(&["check", "--project", &project]);
    git(&dir, &["init", "-q"]);
    commit(&dir, "first");
    w.ok(&["publish", "1.0.0", "--project", &project]);
    w.ok(&["add", "dep", "example.com/dep@2.0.0", "--project", &project]);
    let (ok, out) = try_publish(&w, &dir, &[], "1.0.1");
    assert!(!ok, "{out}");
    assert!(
        out.contains("changed value api.f: (example.com/dep@1:v.V) -> 1"),
        "{out}"
    );
    assert!(
        out.contains("changed type api.W: W = (example.com/dep@1:v.V)"),
        "{out}"
    );
    assert!(out.contains("a new major version"), "{out}");
}

#[test]
fn publish_sees_what_the_host_must_give() {
    let w = World::new();
    let api = "(\n\tnow\n\tIo\n)\n\n@external(\"env\", \"now\")\nnow : () -> Int\n\n@host(\"pub:io/v1\")\nIo = (print = (String) -> ())\n";
    let dir = w.project(
        "pub",
        &[
            ("i.toml", "name = \"pub\"\n"),
            (".gitignore", "/build/\n"),
            ("src/api.i", api),
        ],
    );
    test_project::configure_tests_only(&dir);
    git(&dir, &["init", "-q"]);
    commit(&dir, "first");
    let project = dir.display().to_string();
    w.ok(&["publish", "1.0.0", "--project", &project]);

    // Another import of the external function.
    let (ok, out) = try_publish(
        &w,
        &dir,
        &[("src/api.i", &api.replace("\"now\")", "\"now2\")"))],
        "1.0.1",
    );
    assert!(!ok, "{out}");
    assert!(
        out.contains("changed value api.now: @external(\"env\", \"now\") () -> Int"),
        "{out}"
    );

    // Another label of a host service function, though its constructor is
    // not exported.
    let (ok, out) = try_publish(
        &w,
        &dir,
        &[("src/api.i", &api.replace("print =", "show ="))],
        "1.0.2",
    );
    assert!(!ok, "{out}");
    assert!(
        out.contains("changed constructors api.Io: Io(print: (String) -> ())"),
        "{out}"
    );
}

/// A package that starts to print imports `i/io`: the host must give the
/// standard service, as if the package declared it.
#[test]
fn publish_sees_the_standard_service_of_i_io() {
    let w = World::new();
    let api = " (f)\n\nf = () { 1 }\n";
    let dir = w.project(
        "pub",
        &[
            ("i.toml", "name = \"pub\"\n"),
            (".gitignore", "/build/\n"),
            ("src/api.i", api),
        ],
    );
    test_project::configure_tests_only(&dir);
    git(&dir, &["init", "-q"]);
    commit(&dir, "first");
    let project = dir.display().to_string();
    w.ok(&["publish", "1.0.0", "--project", &project]);

    let printing = "(\n\t(Io) = i/io\n)\n\n(\n\tf\n\tsay\n)\n\nf = () { 1 }\n\nsay = () <- (io : Io) { io.print(\"one\") }\n";
    let (ok, out) = try_publish(&w, &dir, &[("src/api.i", printing)], "1.0.1");
    assert!(!ok, "{out}");
    assert!(
        out.contains("added host \"i:io/v1\": (print: (String) -> ())"),
        "{out}"
    );
    assert!(out.contains("a new minor version"), "{out}");
}

#[test]
fn publish_tells_apart_types_that_are_shown_alike() {
    if syn3_pending("package_commands::publish_tells_apart_types_that_are_shown_alike") {
        return;
    }
    let w = World::new();
    // Two packages, both named `util`, each with a module `u` and a type
    // `T`; `pa` and `pb` each use one of them as `util`.
    let util = |ty: &str| format!(" (T(..))\n\nT = ({ty})\n");
    let (u1, u2) = (util("Int"), util("String"));
    w.repo(
        "u1",
        &[(
            "1.0.0",
            &[("i.toml", "name = \"util\"\n"), ("src/u.i", &u1)],
        )],
    );
    w.repo(
        "u2",
        &[(
            "1.0.0",
            &[("i.toml", "name = \"util\"\n"), ("src/u.i", &u2)],
        )],
    );
    let get = " (util/u)\n (get)\n\nget = (x : u.T) { x }\n";
    for (name, dep) in [("pa", "u1"), ("pb", "u2")] {
        let toml =
            format!("name = \"{name}\"\n\n[dependencies]\nutil = \"example.com/{dep}@1.0.0\"\n");
        w.repo(name, &[("1.0.0", &[("i.toml", &toml), ("src/m.i", get)])]);
    }
    let dir = w.project(
        "lib",
        &[
            (
                "i.toml",
                "name = \"lib\"\n\n[dependencies]\npa = \"example.com/pa@1.0.0\"\npb = \"example.com/pb@1.0.0\"\n",
            ),
            (".gitignore", "/build/\n"),
            ("src/m.i", " (pa/m)\n (h)\n\nh = m.get\n"),
        ],
    );
    test_project::configure_tests_only(&dir);
    let project = dir.display().to_string();
    w.ok(&["check", "--project", &project]);
    git(&dir, &["init", "-q"]);
    commit(&dir, "first");
    w.ok(&["publish", "1.0.0", "--project", &project]);

    // `h` is shown alike, but it is another type.
    let (ok, out) = try_publish(
        &w,
        &dir,
        &[("src/m.i", " (pb/m)\n (h)\n\nh = m.get\n")],
        "1.0.1",
    );
    assert!(!ok, "{out}");
    assert!(
        out.contains("changed value m.h: (example.com/u1@1:u.T) -> example.com/u1@1:u.T"),
        "{out}"
    );
    assert!(
        out.contains("to value m.h: (example.com/u2@1:u.T) -> example.com/u2@1:u.T"),
        "{out}"
    );
    assert!(out.contains("a new major version"), "{out}");
}

#[test]
fn publish_sees_what_the_host_must_give_though_it_is_not_exported() {
    let w = World::new();
    let api = "(\n\tf\n)\n\n@external(\"env\", \"now\")\nnow : () -> Int\n\n@host(\"pub:io/v1\")\nIo = (print = (String) -> ())\n\nf = () { now() }\n";
    let dir = w.project(
        "pub",
        &[
            ("i.toml", "name = \"pub\"\n"),
            (".gitignore", "/build/\n"),
            ("src/api.i", api),
        ],
    );
    test_project::configure_tests_only(&dir);
    git(&dir, &["init", "-q"]);
    commit(&dir, "first");
    let project = dir.display().to_string();
    w.ok(&["publish", "1.0.0", "--project", &project]);

    // Other names for them in the package are no change.
    let renamed = api
        .replace("now", "clock")
        .replace("\"clock\")", "\"now\")")
        .replace("Io", "Out");
    let (ok, out) = try_publish(&w, &dir, &[("src/api.i", &renamed)], "1.0.1");
    assert!(ok, "{out}");
    assert!(
        out.contains("the public interface is the same as in v1.0.0"),
        "{out}"
    );

    // Another import of the external function that `f` uses.
    let (ok, out) = try_publish(
        &w,
        &dir,
        &[("src/api.i", &api.replace("\"now\")", "\"now2\")"))],
        "1.0.2",
    );
    assert!(!ok, "{out}");
    assert!(
        out.contains("removed external \"env\" \"now\": () -> Int"),
        "{out}"
    );
    assert!(
        out.contains("added external \"env\" \"now2\": () -> Int"),
        "{out}"
    );
    assert!(out.contains("a new major version"), "{out}");

    // Another type of it.
    let (ok, out) = try_publish(
        &w,
        &dir,
        &[(
            "src/api.i",
            &api.replace("now : () -> Int", "now : (Int) -> Int")
                .replace("now()", "now(1)"),
        )],
        "1.0.2",
    );
    assert!(!ok, "{out}");
    assert!(
        out.contains("changed external \"env\" \"now\": () -> Int"),
        "{out}"
    );

    // Another label of a function of a host service.
    let (ok, out) = try_publish(
        &w,
        &dir,
        &[("src/api.i", &api.replace("print =", "show ="))],
        "1.0.2",
    );
    assert!(!ok, "{out}");
    assert!(
        out.contains("changed host \"pub:io/v1\": (print: (String) -> ())"),
        "{out}"
    );
    assert!(
        out.contains("to host \"pub:io/v1\": (show: (String) -> ())"),
        "{out}"
    );
}

#[test]
fn add_over_a_dependency_written_as_a_table_keeps_its_comments() {
    let w = World::new();
    lib_repo(&w);
    let library = stdlib_dependency();
    for name in ["util", "util2"] {
        w.project(
            name,
            &[("i.toml", "name = \"util\"\n"), ("src/u.i", "x = 1\n")],
        );
    }
    // The library `i` is a normal dependency, as every root package needs.
    let toml = &format!(
        "name = \"app\"\n\n# the util\n[dependencies]\n{}\n[dependencies.lib]\npath = \"../util\" # p\n\n[wasm]\nexports = []\n",
        stdlib_dependency()
    );
    let main = ("src/main.i", " (x)\n\nx = 1\n");

    // Another path: the table stays, with its comments.
    let dir = w.project("app", &[("i.toml", toml), main]);
    let project = dir.display().to_string();
    let util2 = w.path().join("util2").display().to_string();
    w.ok(&["add", "lib", "--path", &util2, "--project", &project]);
    assert_eq!(
        read(&dir.join("i.toml")),
        toml.replace("../util\"", "../util2\"")
    );

    // A location: an entry, under the comments of the table.
    w.ok(&["add", "lib", "example.com/lib@1.2.0", "--project", &project]);
    assert_eq!(
        read(&dir.join("i.toml")),
        format!(
            "name = \"app\"\n\n# the util\n[dependencies]\n{library}lib = \"example.com/lib@1.2.0\"\n\n[wasm]\nexports = []\n"
        )
    );

    // When `[dependencies]` has a header, the comments go above the entry.
    let toml = &format!(
        "name = \"app\"\n\n[dependencies]\n{library}other = \"example.com/lib@2.0.0\"\n\n# the util\n[dependencies.lib] # mine\npath = \"../util\"\n"
    );
    let dir = w.project("app2", &[("i.toml", toml), main]);
    let project = dir.display().to_string();
    w.ok(&["add", "lib", "example.com/lib@1.2.0", "--project", &project]);
    assert_eq!(
        read(&dir.join("i.toml")),
        format!(
            "name = \"app\"\n\n[dependencies]\n{library}other = \"example.com/lib@2.0.0\"\n# the util\nlib = \"example.com/lib@1.2.0\" # mine\n"
        )
    );
}

#[test]
fn build_and_check_explain_an_old_syntax_dependency_tag() {
    let w = World::new();
    // old-syntax-check: intentional old tag; parser E0101 for the old match.
    let old = /* old-syntax-check: E0101,E0101 */ "import ((inspect) = i/core)\nexport (f)\nf = () { case 1 { 0 -> 0  _ -> 1 } }\n";
    w.repo(
        "legacy",
        &[(
            "1.0.0",
            &[("i.toml", "name = \"legacy\"\n"), ("src/api.i", old)],
        )],
    );
    let dir = w.project(
        "app",
        &[
            (
                "i.toml",
                &format!(
                    "name = \"app\"\n[dependencies]\nlegacy = \"example.com/legacy@1.0.0\"\n{}",
                    stdlib_dependency()
                ),
            ),
            ("src/main.i", "(legacy/api)\nx = api.f()\n"),
        ],
    );
    for command in ["build", "check"] {
        let err = w.fails(&[command, "--project", dir.to_str().unwrap()]);
        assert!(err.contains("error[E0101]"), "{err}");
        assert_eq!(
            err.matches("this version of `legacy` (`v1.0.0`)").count(),
            1,
            "{err}"
        );
        assert!(
            err.contains("use a newer version of the package, or an older version of `i`"),
            "{err}"
        );
    }
}

#[test]
fn publish_skips_an_old_syntax_interface_and_allows_a_patch() {
    let w = World::new();
    // old-syntax-check: intentional old tag; export is a reserved MacroItem.
    let dir = w.project(
        "legacy",
        &[
            ("i.toml", "name = \"legacy\"\n"),
            ("src/api.i", "export (f)\nf = () { 1 }\n"),
        ],
    );
    test_project::depend_on_library(&dir);
    git(&dir, &["init", "-q"]);
    commit(&dir, "old syntax");
    git(&dir, &["tag", "v1.0.0"]);
    write(&dir, &[("src/api.i", "(f)\nf = () { 1 }\n")]);
    commit(&dir, "new syntax");
    let out = w.ok(&["publish", "1.0.1", "--project", dir.to_str().unwrap()]);
    assert!(
        out.contains("the interface of `v1.0.0` is in the syntax from before"),
        "{out}"
    );
    assert!(out.contains("check the interface by hand"), "{out}");
    assert_eq!(git(&dir, &["tag", "--list", "v1.0.1"]), "v1.0.1");
}

#[test]
fn publish_keeps_literal_alias_values_beyond_hover_limits() {
    // A published alias body is full
    // interface text; shortening the 14-member hover must not hide a change.
    let w = World::new();
    let literals = (0..14)
        .map(|n| format!("\"m{n:02}\""))
        .collect::<Vec<_>>()
        .join(" | ");
    let api = format!(" (Modes)\n\nModes = {literals}\n");
    let dir = w.project(
        "pub",
        &[
            ("i.toml", "name = \"pub\"\n"),
            (".gitignore", "/build/\n"),
            ("src/api.i", &api),
        ],
    );
    test_project::configure_tests_only(&dir);
    git(&dir, &["init", "-q"]);
    commit(&dir, "first");
    let project = dir.display().to_string();
    w.ok(&["publish", "1.0.0", "--project", &project]);
    let changed = api.replace("\"m13\"", "\"m99\"");
    let (ok, out) = try_publish(&w, &dir, &[("src/api.i", &changed)], "1.0.1");
    assert!(!ok, "{out}");
    assert!(out.contains("changed type api.Modes"), "{out}");
    assert!(out.contains("a new major version"), "{out}");
    assert!(!out.contains("…"), "{out}");
}

#[cfg(unix)]
#[test]
fn publish_rejects_committed_source_links_before_reading_their_targets() {
    let w = World::new();
    let outside = w.project(
        "outside",
        &[(
            "lib.i",
            include_str!("../../../tests/spec/2.8/mp01_external.i"),
        )],
    );
    let package = w.project(
        "pub",
        &[("i.toml", "name = \"pub\"\n"), (".gitignore", "/build/\n")],
    );
    test_project::depend_on_library(&package);
    fs::create_dir(package.join("src")).unwrap();
    std::os::unix::fs::symlink(outside.join("lib.i"), package.join("src/lib.i")).unwrap();
    git(&package, &["init", "-q"]);
    commit(&package, "source link");
    let err = w.fails(&[
        "publish",
        "1.0.0",
        "--dry-run",
        "--project",
        package.to_str().unwrap(),
    ]);
    assert!(err.contains("symbolic link"), "{err}");
    assert!(git(&package, &["tag", "--list"]).is_empty());
}

/// package commands reject the same source-link attack as compilation.
#[cfg(unix)]
#[test]
fn add_rejects_dependency_with_source_link() {
    let w = World::new();
    w.repo("dep", &[]);
    let outside = w.project(
        "outside",
        &[(
            "lib.i",
            include_str!("../../../tests/spec/2.8/mp01_external.i"),
        )],
    );
    let work = w.path().join("work/dep");
    write(&work, &[("i.toml", "name = \"dep\"\n")]);
    fs::create_dir_all(work.join("src")).unwrap();
    std::os::unix::fs::symlink(outside.join("lib.i"), work.join("src/lib.i")).unwrap();
    commit(&work, "source link");
    git(&work, &["tag", "v1.0.0"]);
    git(
        &work,
        &[
            "push",
            "-q",
            w.path().join("remotes/dep").to_str().unwrap(),
            "--tags",
        ],
    );
    let manifest = &format!("name = \"app\"\n[dependencies]\n{}", stdlib_dependency());
    let app = w.project(
        "app",
        &[("i.toml", manifest), ("src/main.i", "main = () { () }\n")],
    );
    let err = w.fails(&[
        "add",
        "dep",
        "example.com/dep@1.0.0",
        "--project",
        app.to_str().unwrap(),
    ]);
    assert!(err.contains("symbolic link"), "{err}");
    assert_eq!(read(&app.join("i.toml")), *manifest);
    assert!(!app.join("i.lock").exists());
}

#[test]
fn package_commands_reject_tampered_cache() {
    let w = World::new();
    w.repo(
        "dep",
        &[(
            "1.0.0",
            &[
                ("i.toml", "name = \"dep\"\n"),
                ("src/lib.i", "(value)\nvalue = 1\n"),
            ],
        )],
    );
    let app = w.project(
        "app",
        &[
            (
                "i.toml",
                &format!(
                    "name = \"app\"\n[dependencies]\ndep = \"example.com/dep@1.0.0\"\n{}",
                    stdlib_dependency()
                ),
            ),
            ("src/main.i", "main = () { () }\n"),
        ],
    );
    let project = app.to_str().unwrap();
    w.ok(&["check", "--project", project]);
    let locked = read(&app.join("i.lock"));
    let commit = locked.split_whitespace().nth(2).unwrap();
    let cache = w.path().join("cache/pkg/example.com%2fdep").join(commit);
    fs::remove_file(cache.join("src/lib.i")).unwrap();
    fs::write(
        cache.join("src/lib.i"),
        include_str!("../../../tests/spec/2.8/mp02_cached.i"),
    )
    .unwrap();
    for command in ["check", "deps", "update"] {
        let err = w.fails(&[command, "--project", project]);
        assert!(
            err.contains("cached files") && err.contains("do not match"),
            "{err}"
        );
        assert_eq!(read(&app.join("i.lock")), locked);
    }
}

#[test]
fn package_commands_reject_moved_superseded_resolution_input() {
    let w = World::new();
    w.repo(
        "c",
        &[
            ("1.0.0", &[("i.toml", "name = \"c\"\n")]),
            ("1.1.0", &[("i.toml", "name = \"c\"\n")]),
        ],
    );
    w.repo(
        "a",
        &[
            (
                "1.0.0",
                &[(
                    "i.toml",
                    "name = \"a\"\n[dependencies]\nc = \"example.com/c@1.0.0\"\n",
                )],
            ),
            ("1.1.0", &[("i.toml", "name = \"a\"\n")]),
        ],
    );
    w.repo(
        "b",
        &[(
            "1.0.0",
            &[(
                "i.toml",
                "name = \"b\"\n[dependencies]\na = \"example.com/a@1.1.0\"\n",
            )],
        )],
    );
    let app = w.project("app", &[
        ("i.toml", &format!("name = \"app\"\n[dependencies]\na = \"example.com/a@1.0.0\"\nb = \"example.com/b@1.0.0\"\n{}", stdlib_dependency())),
        ("src/main.i", include_str!("../../../tests/spec/2.8/mp03_main.i")),
    ]);
    let project = app.to_str().unwrap();
    w.ok(&["check", "--project", project]);
    let locked = read(&app.join("i.lock"));
    assert!(
        locked
            .lines()
            .any(|l| l.starts_with("example.com/a 1.0.0 "))
    );
    let work = w.path().join("work/a");
    git(&work, &["tag", "-d", "v1.0.0"]);
    w.tag(
        "a",
        "1.0.0",
        &[(
            "i.toml",
            "name = \"a\"\n[dependencies]\nc = \"example.com/c@1.1.0\"\n",
        )],
    );
    // Each command sees a fresh copy, as on a second computer.
    for command in ["check", "deps", "update"] {
        fs::remove_dir_all(w.path().join("cache")).unwrap();
        let err = w.fails(&[command, "--project", project]);
        assert!(
            err.contains("i.lock") && err.contains("example.com/a@1.0.0"),
            "{err}"
        );
        assert_eq!(read(&app.join("i.lock")), locked);
    }
}

#[test]
fn publish_pins_macro_kind_call_contract_and_compile_time_source() {
    let w = World::new();
    let macros = include_str!("../../../tests/spec/14.5/mp07_macro_contract/src/macros.is");
    let dir = w.project(
        "pub",
        &[
            ("i.toml", "name = \"pub\"\n"),
            (".gitignore", "/build/\n"),
            ("src/macros.is", macros),
        ],
    );
    test_project::depend_on_library(&dir);
    git(&dir, &["init", "-q"]);
    commit(&dir, "original macro");
    let project = dir.to_str().unwrap();
    w.ok(&["publish", "1.0.0", "--project", project]);
    // Identical source is a valid patch.
    let (ok, out) = try_publish(&w, &dir, &[], "1.0.1");
    assert!(ok, "{out}");
    // Renaming a parameter and changing the transformer both escape a plain
    // (Syntax) -> Syntax comparison, but change the conservative contract.
    for changed in [
        macros.replace("value", "renamed"),
        macros.replace("((. syntax make) ((. syntax node) value) value)", "`False"),
    ] {
        let (ok, out) = try_publish(&w, &dir, &[("src/macros.is", &changed)], "1.0.1");
        assert!(!ok, "{out}");
        assert!(
            out.contains("changed macro macros.id") && out.contains("a new major version"),
            "{out}"
        );
    }
}

#[test]
fn publish_macro_contract_includes_imported_compile_time_initializers() {
    let w = World::new();
    let dir = w.project("pub", &[
        ("i.toml", "name = \"pub\"\n"), (".gitignore", "/build/\n"),
        ("src/helper.i", "(answer)\nanswer = 42\n"),
        ("src/macros.is", "(, (= helper (/ pub helper)))\n(, m)\n(@ macro (= m (-> (, value) _ {(. helper answer) value})))\n"),
    ]);
    test_project::depend_on_library(&dir);
    git(&dir, &["init", "-q"]);
    commit(&dir, "original helper");
    w.ok(&["publish", "1.0.0", "--project", dir.to_str().unwrap()]);
    let (ok, out) = try_publish(
        &w,
        &dir,
        &[("src/helper.i", "(answer)\nanswer = 43\n")],
        "1.0.1",
    );
    assert!(!ok, "{out}");
    assert!(out.contains("changed macro macros.m"), "{out}");
}

#[test]
fn publish_compares_interfaces_after_macro_generated_exports() {
    let w = World::new();
    let dir = w.project("pub", &[
        ("i.toml", "name = \"pub\"\n"), (".gitignore", "/build/\n"),
        ("src/macros.is", "(, (= syntax (/ i syntax)))\n(, emit)\n(@ macro (= emit (-> (, name value) (<- _ (, (: items (. syntax Items)))) {((. items expose) name) `(= ~name ~value)})))\n"),
        ("src/api.i", "(m = pub/macros)\nm.emit(answer, 42)\n"),
    ]);
    test_project::depend_on_library(&dir);
    git(&dir, &["init", "-q"]);
    commit(&dir, "generated API");
    w.ok(&["publish", "1.0.0", "--project", dir.to_str().unwrap()]);
    let (ok, out) = try_publish(
        &w,
        &dir,
        &[(
            "src/api.i",
            "(m = pub/macros)\nm.emit(answer, \"changed\")\n",
        )],
        "1.0.1",
    );
    assert!(!ok, "{out}");
    assert!(out.contains("changed value api.answer"), "{out}");
}

/// The `i.toml` of `dir` with `[test].collection` set to `value`.
fn with_collection(dir: &Path, value: toml_edit::Item) -> String {
    let mut manifest: toml_edit::DocumentMut = read(&dir.join("i.toml")).parse().unwrap();
    manifest["test"]["collection"] = value;
    manifest.to_string()
}

#[test]
fn publish_a_missing_collection_is_an_error_not_zero_tests() {
    let w = World::new();
    let dir = published(&w, API_1);
    let manifest = with_collection(&dir, toml_edit::value("i/test.nothing"));
    let (ok, out) = try_publish(&w, &dir, &[("i.toml", &manifest)], "1.0.0");
    assert!(!ok, "{out}");
    assert!(
        out.contains("there is no test collection \"i/test.nothing\""),
        "{out}"
    );
    assert!(!out.contains("tests passed"), "{out}");
    assert_eq!(tags(&dir), "");
}

#[test]
fn publish_rejects_a_non_string_collection() {
    let w = World::new();
    let dir = published(&w, API_1);
    let manifest = with_collection(&dir, toml_edit::value(1));
    let (ok, out) = try_publish(&w, &dir, &[("i.toml", &manifest)], "1.0.0");
    assert!(!ok, "{out}");
    assert!(
        out.contains("`[test].collection` must be a string, not integer"),
        "{out}"
    );
    assert!(!out.contains("passed"), "{out}");
}

#[test]
fn publish_rejects_duplicate_test_names() {
    let w = World::new();
    let dir = published(&w, API_1);
    let (ok, out) = try_publish(
        &w,
        &dir,
        &[(
            "test/dup_test.i",
            "(\n\ttestlib = (test) = i/test\n)\n\ntest(\"same\", () { () })\n\ntest(\"same\", () { () })\n",
        )],
        "1.0.0",
    );
    assert!(!ok, "{out}");
    assert!(
        out.contains("duplicate test name \"pub/dup_test.same\": "),
        "{out}"
    );
    assert!(!out.contains("passed"), "{out}");
}

#[test]
fn publish_does_not_panic_on_an_unnamed_test() {
    let w = World::new();
    let dir = published(&w, API_1);
    // A collection of the package's own whose macro gives no "name".
    let manifest = with_collection(&dir, toml_edit::value("pub/unnamed.tests"));
    let (ok, out) = try_publish(
        &w,
        &dir,
        &[
            ("i.toml", &manifest),
            (
                "src/unnamed.is",
                "(, (= syntax (/ i syntax)))\n(, tests unnamed)\n(= tests (,))\n(@ macro\n  (= unnamed\n    (-> (, body) (<- _ (, (: items (. syntax Items))))\n      {(= target `generated_test)\n       ((. items add) `tests target [(, \"mode\" \"test\")] body)\n       `(= ~target (-> (,) _ {(~body)}))})))\n",
            ),
            (
                "test/unnamed_test.i",
                "(u = pub/unnamed)\nu.unnamed(() { () })\n",
            ),
        ],
        "1.0.0",
    );
    assert!(ok, "{out}");
    assert!(!out.contains("panicked"), "{out}");
    assert!(out.contains("its 1 test passed"), "{out}");
}

#[test]
fn two_processes_fetching_one_package_share_the_cache() {
    let w = World::new();
    lib_repo(&w);
    let dir = app(&w);
    let project = dir.display().to_string();
    w.ok(&["add", "lib", "example.com/lib@1.0.0", "--project", &project]);
    // Both processes start with no cache at all, so both fetch the package.
    fs::remove_dir_all(w.path().join("cache")).unwrap();
    let spawn = || {
        w.command()
            .args(["check", "--project", &project])
            .spawn()
            .unwrap()
    };
    let mut first = spawn();
    let mut second = spawn();
    let a = first.wait().unwrap();
    let b = second.wait().unwrap();
    assert!(a.success(), "first process failed");
    assert!(b.success(), "second process failed");
    // The cache is intact and usable after the race.
    w.ok(&["check", "--offline", "--project", &project]);
}

// ---- Packages in a directory of a repository ----

const SUB_SRC: &str = " (x)\n\nx = 1\n";

/// The repository `example.com/mono`: a root package with the tags `v0.1.5`
/// and `v0.9.0`, and the package `//stdlib` with the tags `stdlib-v0.1.0`,
/// `stdlib-v0.1.1` and `stdlib-v1.0.0`.
fn mono_repo(w: &World) {
    w.repo("mono", &[]);
    let root: &[(&str, &str)] = &[("i.toml", "name = \"mono\"\n"), ("src/m.i", SUB_SRC)];
    let stdlib: &[(&str, &str)] = &[
        ("stdlib/i.toml", "name = \"stdlib\"\n"),
        ("stdlib/src/text.i", SUB_SRC),
    ];
    w.tag_as("mono", "stdlib-v0.1.0", stdlib);
    w.tag_as("mono", "v0.1.5", root);
    w.tag_as("mono", "stdlib-v0.1.1", stdlib);
    w.tag_as("mono", "v0.9.0", root);
    w.tag_as("mono", "stdlib-v1.0.0", stdlib);
}

#[test]
fn add_and_update_use_the_tags_of_the_subpackage() {
    let w = World::new();
    mono_repo(&w);
    let dir = app(&w);
    let project = dir.display().to_string();
    // With no version: the newest `stdlib-v*` tag, not a root tag.
    let out = w.ok(&[
        "add",
        "stdlib",
        "example.com/mono//stdlib",
        "--project",
        &project,
    ]);
    assert!(
        out.contains("added `stdlib = example.com/mono//stdlib@1.0.0`"),
        "{out}"
    );
    // The root package keeps its own tags.
    let out = w.ok(&["add", "mono", "example.com/mono", "--project", &project]);
    assert!(
        out.contains("added `mono = example.com/mono@0.9.0`"),
        "{out}"
    );
    w.ok(&[
        "add",
        "old",
        "example.com/mono//stdlib@0.1.0",
        "--dev",
        "--project",
        &project,
    ]);
    let lock = read(&dir.join("i.lock"));
    assert!(lock.contains("example.com/mono//stdlib 0.1.0 "), "{lock}");
    assert!(lock.contains("example.com/mono//stdlib 1.0.0 "), "{lock}");
    assert!(lock.contains("example.com/mono 0.9.0 "), "{lock}");

    // `i update old`: the newest `stdlib-v0.*`, not the root's v0.9.0.
    let out = w.ok(&["update", "old", "--project", &project]);
    assert!(
        out.contains("updated old: example.com/mono//stdlib 0.1.0 -> 0.1.1"),
        "{out}"
    );
    let toml = read(&dir.join("i.toml"));
    assert!(
        toml.contains("old = \"example.com/mono//stdlib@0.1.1\"\n"),
        "{toml}"
    );
    let out = w.ok(&["update", "--project", &project]);
    assert!(out.contains("newest version"), "{out}");

    // A new root tag is not a version of `//stdlib`; a new stdlib tag is.
    w.tag_as(
        "mono",
        "v0.9.9",
        &[("i.toml", "name = \"mono\"\n"), ("src/m.i", SUB_SRC)],
    );
    w.tag_as(
        "mono",
        "stdlib-v0.1.2",
        &[
            ("stdlib/i.toml", "name = \"stdlib\"\n"),
            ("stdlib/src/text.i", SUB_SRC),
        ],
    );
    let out = w.ok(&["update", "--project", &project]);
    assert!(
        out.contains("updated old: example.com/mono//stdlib 0.1.1 -> 0.1.2"),
        "{out}"
    );
    assert!(
        out.contains("updated mono: example.com/mono 0.9.0 -> 0.9.9"),
        "{out}"
    );
    assert!(!out.contains("updated stdlib:"), "{out}");

    // A subpackage with no tags of its own has no version.
    let err = w.fails(&[
        "add",
        "none",
        "example.com/mono//none",
        "--project",
        &project,
    ]);
    assert!(err.contains("has no tag `none-vX.Y.Z`"), "{err}");
}

#[test]
fn publish_a_package_in_a_directory_of_the_repository() {
    let w = World::new();
    let top = w.path().join("mono");
    let dir = top.join("stdlib");
    write(
        &dir,
        &[
            ("i.toml", "name = \"pub\"\n"),
            (".gitignore", "/build/\n"),
            ("src/api.i", API_1),
            (
                "test/api_test.i",
                "(\n\ttestlib = (test) = i/test\n\tpub/api\n)\n\ntest(\"works\", () { api.one |> (1) { () } & (other) { panic(\"no\") } })\n",
            ),
        ],
    );
    write(&top, &[("README", "mono\n")]);
    // The rest of a big repository, which the package's tags must not reach.
    for n in 0..200 {
        write(
            &top,
            &[
                (&format!("crates/c{n}/src/lib.rs"), &format!("// {n}\n")),
                (&format!("docs/page{n}.md"), &format!("# {n}\n")),
            ],
        );
    }
    // The package is got by location below, so it may not have a path
    // dependency: its tests get the library by location, and `i test`
    // writes the `i.lock` that is committed with it.
    w.library();
    configure_tests_by_location(&dir);
    w.ok(&["test", "--project", &dir.display().to_string()]);
    git(&top, &["init", "-q"]);
    commit(&top, "first");
    // A root tag that is newer is not a version of this package.
    git(&top, &["tag", "v5.0.0"]);
    let project = dir.display().to_string();

    let out = w.ok(&["publish", "0.1.0", "--dry-run", "--project", &project]);
    assert!(out.contains("no earlier version"), "{out}");
    assert!(
        out.contains("dry run: every check passed; the tag stdlib-v0.1.0 was not made"),
        "{out}"
    );
    assert_eq!(tags(&top), "v5.0.0");

    let out = w.ok(&["publish", "0.1.0", "--project", &project]);
    assert!(out.contains("made the tag stdlib-v0.1.0"), "{out}");
    assert!(out.contains("git push origin stdlib-v0.1.0"), "{out}");
    assert_eq!(tags(&top), "stdlib-v0.1.0\nv5.0.0");
    // The tag is on a commit of the package's subtree only, with no parent.
    let tagged = git(&top, &["rev-parse", "stdlib-v0.1.0^{commit}"]);
    assert_ne!(tagged, git(&top, &["rev-parse", "HEAD"]));
    assert_eq!(
        git(&top, &["rev-list", "--parents", "-n1", &tagged]),
        tagged
    );
    assert_eq!(
        git(&top, &["rev-parse", "stdlib-v0.1.0^{tree}"]),
        git(&top, &["rev-parse", "HEAD:stdlib"])
    );
    assert_eq!(
        git(&top, &["log", "-1", "--format=%s", &tagged]),
        "stdlib 0.1.0"
    );
    let objects = git(&top, &["rev-list", "--objects", "stdlib-v0.1.0"]);
    let count = objects.lines().count();
    eprintln!("objects reachable from stdlib-v0.1.0: {count}");
    assert!(count < 300, "{count}");
    assert!(
        !objects
            .lines()
            .any(|o| o.contains(" crates/") || o.contains(" docs/") || o.contains(" stdlib/")),
        "{objects}"
    );
    assert!(
        objects.lines().any(|o| o.ends_with(" src/api.i")),
        "{objects}"
    );

    // Got by location, the package is the root of the tagged tree.
    let remote = w.path().join("remotes").join("mono");
    fs::create_dir_all(&remote).unwrap();
    git(&remote, &["init", "-q", "--bare"]);
    let push_tags = |top: &Path| {
        let remote = remote.display().to_string();
        git(top, &["push", "-q", "-f", &remote, "HEAD:refs/heads/main"]);
        git(top, &["push", "-q", "-f", &remote, "--tags"]);
    };
    push_tags(&top);
    let app = w.project(
        "app",
        &[
            (
                "i.toml",
                "name = \"app\"\n[dependencies]\npub = \"example.com/mono//stdlib@0.1.0\"\ni = \"github.com/nmnmcc/i//stdlib@0.1.0\"\n",
            ),
            ("src/main.i", " (pub/api)\n (x)\n\nx = api.one\n"),
        ],
    );
    let app_project = app.display().to_string();
    w.ok(&["check", "--project", &app_project]);
    let lock = read(&app.join("i.lock"));
    assert!(
        lock.starts_with(&format!("example.com/mono//stdlib 0.1.0 {tagged} sha256-")),
        "{lock}"
    );
    let cache_repo = w
        .path()
        .join("cache")
        .join("git")
        .join("example.com%2fmono");
    assert_eq!(
        git(&cache_repo, &["for-each-ref", "--format=%(refname)"]),
        "refs/tags/stdlib-v0.1.0"
    );
    let cached = git(&cache_repo, &["rev-list", "--objects", "--all"]);
    eprintln!(
        "objects in the cache after the fetch: {}",
        cached.lines().count()
    );
    assert!(cached.lines().count() < 300);
    assert!(!cached.contains(" crates/") && !cached.contains(" docs/"));

    // Not newer than the previous `stdlib-v*` tag.
    for old in ["0.1.0", "0.0.9"] {
        let err = w.fails(&["publish", old, "--project", &project]);
        assert!(err.contains("must be newer than every tag"), "{err}");
        assert!(err.contains("the newest is stdlib-v0.1.0"), "{err}");
    }
    assert_eq!(tags(&top), "stdlib-v0.1.0\nv5.0.0");

    // The interface is compared with the previous `stdlib-v*` tag.
    write(&dir, &[("src/api.i", &format!("{API_1}\nhidden = 3\n"))]);
    commit(&top, "internal");
    let out = w.ok(&["publish", "0.1.1", "--dry-run", "--project", &project]);
    assert!(
        out.contains("the public interface is the same as in stdlib-v0.1.0"),
        "{out}"
    );
    assert!(!out.contains("v5.0.0"), "{out}");
    assert!(out.contains("the tag stdlib-v0.1.1 was not made"), "{out}");
    assert_eq!(tags(&top), "stdlib-v0.1.0\nv5.0.0");
    // An addition since the subtree tag needs a minor version.
    let added = API_1
        .replace("\tone\n", "\tone\n\ttwo\n")
        .replace("one = 1\n", "one = 1\n\ntwo = 2\n");
    write(&dir, &[("src/api.i", &added)]);
    commit(&top, "two");
    let err = w.fails(&["publish", "0.1.1", "--project", &project]);
    assert!(
        err.contains("something was added since stdlib-v0.1.0"),
        "{err}"
    );
    let out = w.ok(&["publish", "0.2.0", "--project", &project]);
    assert!(out.contains("git push origin stdlib-v0.2.0"), "{out}");
    let tagged = git(&top, &["rev-parse", "stdlib-v0.2.0^{commit}"]);
    assert_eq!(
        git(&top, &["rev-list", "--parents", "-n1", &tagged]),
        tagged
    );
    assert_eq!(tags(&top), "stdlib-v0.1.0\nstdlib-v0.2.0\nv5.0.0");
}
