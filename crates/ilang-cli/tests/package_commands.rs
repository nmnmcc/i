//! `i new`, `i add`, `i remove`, `i update`, `i publish`, and `i deps`,
//! with packages got from local bare repositories through `file://` URLs
//! (`ILANG_GIT_URLS`) and a cache in a temporary directory (`ILANG_CACHE`).
//! Nothing here uses the network.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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

    /// Runs `i` in the world's directory, with its cache and URLs.
    fn i(&self, args: &[&str]) -> Output {
        let remotes = self.path().join("remotes");
        git_env(&mut Command::new(env!("CARGO_BIN_EXE_i")))
            .current_dir(self.path())
            .env("ILANG_CACHE", self.path().join("cache"))
            .env(
                "ILANG_GIT_URLS",
                format!("example.com/=file://{}/", remotes.display()),
            )
            .env_remove("ILANG_OFFLINE")
            .args(args)
            .output()
            .unwrap()
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
        let work = self.path().join("work").join(name);
        let bare = self.path().join("remotes").join(name);
        git(&work, &["rm", "-rq", "--ignore-unmatch", "."]);
        write(&work, files);
        git(&work, &["add", "-A"]);
        git(&work, &["commit", "-q", "--allow-empty", "-m", version]);
        git(&work, &["tag", &format!("v{version}")]);
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

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

/// The package `lib`: v1.0.0 has a function, v1.1.0 adds a host service
/// and an external function, v1.2.0 is the same, and v2.0.0 is another major
/// version.
fn lib_repo(w: &World) {
    const PLAIN: &str = "export (twice)\n\ntwice = (n: Int) { n * 2 }\n";
    const WITH_HOST: &str = r#"export (twice, Net)

@host("lib:net/v1")
Net { Net(get: (url: String) -> String) }

@external("env", "now")
now: () -> Int

twice = (n: Int) { n * 2 }
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

const APP_TOML: &str = "# The app.\nname = \"app\"\n\n[dependencies] # what it uses\n";

fn app(w: &World) -> PathBuf {
    w.project(
        "app",
        &[
            ("i.toml", APP_TOML),
            ("src/main.i", "export (x)\n\nx = 1\n"),
        ],
    )
}

// ---- i new ----

#[test]
fn new_makes_a_package_that_checks_runs_and_tests() {
    let w = World::new();
    let out = w.ok(&["new", "demo"]);
    assert!(out.contains("demo"), "{out}");
    let dir = w.path().join("demo");
    assert_eq!(read(&dir.join("i.toml")), "name = \"demo\"\n");
    assert_eq!(read(&dir.join(".gitignore")), "/build/\n");
    assert!(read(&dir.join("src/main.i")).contains("@host(\"i:io/v1\")"));
    assert!(read(&dir.join("src/main.i")).contains("@main"));
    assert!(read(&dir.join("test/main_test.i")).contains("@test"));

    let project = dir.display().to_string();
    w.ok(&["check", "--project", &project]);
    let out = w.ok(&["run", "--project", &project]);
    assert_eq!(out, "Hello, demo!\n");
    let out = w.ok(&["test", "--project", &project]);
    assert!(out.contains("1 passed, 0 failed"), "{out}");
}

#[test]
fn new_with_a_path_and_bad_names() {
    let w = World::new();
    w.ok(&["new", "tool", "--path", "nested/place"]);
    assert_eq!(
        read(&w.path().join("nested/place/i.toml")),
        "name = \"tool\"\n"
    );
    // The directory is not empty now.
    let err = w.fails(&["new", "tool", "--path", "nested/place"]);
    assert!(err.contains("not empty"), "{err}");
    for bad in ["i", "Big", "1x", "import", "a-b"] {
        let err = w.fails(&["new", bad]);
        assert!(err.contains("must be a name"), "{bad}: {err}");
    }
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
        format!("{APP_TOML}lib = \"example.com/lib@2.0.0\"\n")
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
            "new permission: example.com/lib 1.1.0 declares host service \"lib:net/v1\" (lib/math.Net)"
        ),
        "{out}"
    );
    assert!(
        out.contains("new permission: example.com/lib 1.1.0 declares external \"env\" \"now\""),
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
            ("src/text.i", "export (hi)\n\nhi = \"hi\"\n"),
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
        &[(
            "src/main.i",
            "import (util/text)\nexport (x)\n\nx = text.hi\n",
        )],
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
    for bad in ["i", "app", "Lib", "case"] {
        let err = w.fails(&["add", bad, "example.com/lib@1.0.0", "--project", &project]);
        assert!(err.contains("§2.6"), "{bad}: {err}");
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
    assert_eq!(read(&dir.join("i.toml")), APP_TOML);
    assert!(!dir.join("i.lock").exists());
    let err = w.fails(&["add", "none", "example.com/none", "--project", &project]);
    assert!(err.contains("could not be fetched"), "{err}");
    assert_eq!(read(&dir.join("i.toml")), APP_TOML);
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
    assert!(toml.starts_with(APP_TOML), "{toml}");
    assert!(!toml.contains("lib"), "{toml}");

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
            "new permission: example.com/lib 1.2.0 declares host service \"lib:net/v1\" (lib/math.Net)"
        ),
        "{out}"
    );
    assert!(
        out.contains(
            "new permission: example.com/lib 1.2.0 declares external \"env\" \"now\" (lib/math.now)"
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
            (
                "src/math.i",
                "export (twice)\n\ntwice = (n: Int) { n + n }\n",
            ),
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
                "@host(\"util:clock/v1\", \"async\")\nClock { Clock(now: () -> Int) }\n",
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
                "import (pub/api)\n\n@test\nworks = () {\n  case api.one {\n    1 -> Nil\n    other -> panic(\"no\")\n  }\n}\n",
            ),
        ],
    );
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

const API_1: &str = "export (one, add, Shape(..), Box)\n\none = 1\n\nadd = (a: Int, b: Int) { a + b }\n\nShape { Circle(r: Float) Square(side: Float) }\n\nBox(a) { Box(a) }\n";

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
        .replace("export (one,", "export (one, two,")
        .replace("one = 1\n", "one = 1\n\ntwo = 2\n");
    write(&dir, &[("src/api.i", &added)]);
    commit(&dir, "two");
    let err = w.fails(&["publish", "1.0.2", "--project", &project]);
    assert!(err.contains("added value api.two: Int"), "{err}");
    assert!(err.contains("a new minor version, at least 1.1.0"), "{err}");
    let out = w.ok(&["publish", "1.1.0", "--dry-run", "--project", &project]);
    assert!(out.contains("something was added since v1.0.1"), "{out}");
    assert_eq!(tags(&dir), "v1.0.0\nv1.0.1");
    w.ok(&["publish", "1.1.0", "--project", &project]);

    // A changed type and a new constructor are incompatible.
    let changed = added
        .replace(
            "add = (a: Int, b: Int) { a + b }",
            "add = (a: Int, b: Int, c: Int) { a + b + c }",
        )
        .replace("Square(side: Float) }", "Square(side: Float) Dot }");
    write(&dir, &[("src/api.i", &changed)]);
    commit(&dir, "change");
    let err = w.fails(&["publish", "1.2.0", "--project", &project]);
    assert!(
        err.contains("changed value api.add: (a: Int, b: Int) -> Int"),
        "{err}"
    );
    assert!(
        err.contains("to value api.add: (a: Int, b: Int, c: Int) -> Int"),
        "{err}"
    );
    assert!(err.contains("changed constructors api.Shape"), "{err}");
    assert!(err.contains("a new major version, at least 2.0.0"), "{err}");
    assert_eq!(tags(&dir), "v1.0.0\nv1.0.1\nv1.1.0");
    let out = w.ok(&["publish", "2.0.0", "--dry-run", "--project", &project]);
    assert!(out.contains("dry run"), "{out}");
    assert_eq!(tags(&dir), "v1.0.0\nv1.0.1\nv1.1.0");

    // Removing a name, or hiding constructors, is incompatible too.
    let hidden = changed
        .replace("Shape(..)", "Shape")
        .replace("one, two,", "one,");
    write(&dir, &[("src/api.i", &hidden)]);
    commit(&dir, "hide");
    let err = w.fails(&["publish", "1.2.0", "--project", &project]);
    assert!(err.contains("removed value api.two: Int"), "{err}");
    assert!(
        err.contains("removed constructors api.Shape: Circle(r: Float) | Square(side: Float)"),
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
    w.ok(&["new", "demo"]);
    let project = w.path().join("demo").display().to_string();
    w.ok(&["fmt", "--check", "--project", &project]);
}
