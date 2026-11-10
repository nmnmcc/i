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
    const PLAIN: &str = " (twice)\n\ntwice = (n: Int) { n * 2 }\n";
    const WITH_HOST: &str = r#" (twice, Net)

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
        &[("i.toml", APP_TOML), ("src/main.i", " (x)\n\nx = 1\n")],
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
    assert!(read(&dir.join("src/main.i")).contains("(Io) = i/io"));
    assert!(read(&dir.join("src/main.i")).contains("@main"));
    assert!(read(&dir.join("test/main_test.i")).contains("@test"));

    let project = dir.display().to_string();
    w.ok(&["check", "--project", &project]);
    let out = w.ok(&["run", "--project", &project]);
    assert_eq!(out, "Hello, demo!\n");
    let out = w.ok(&["test", "--project", &project]);
    assert!(out.contains("1 passed, 0 failed"), "{out}");
}

/// Two packages made by `i new` both print with `Io` of `i/io`, so one may
/// depend on the other: neither declares the standard service (§9.7). The
/// dependency is still shown as able to print.
#[test]
fn two_new_packages_work_together() {
    let w = World::new();
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
            " (\n\t(Io) = i/io\n\tgreet = greet/main\n)\n\n (greeting)\n\ngreeting = (who: String) { greet.greeting(who) }\n\n@main\nmain = (io ! Io) { io.print(greeting(\"i\")) }\n",
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
    for bad in ["i", "Big", "1x", "__bad", "a-b"] {
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
            ("src/math.i", " (twice)\n\ntwice = (n: Int) { n + n }\n"),
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
                " (pub/api)\n\n@test\nworks = () {\n   api.one |> {\n    1 -> Nil\n    other -> panic(\"no\")\n  }\n}\n",
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

const API_1: &str = " (one, add, Shape(..), Box)\n\none = 1\n\nadd = (a: Int, b: Int) { a + b }\n\nShape { Circle(r: Float) Square(side: Float) }\n\nBox(A) { Box(A) }\n";

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
        .replace("(one,", "(one, two,")
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
    assert!(err.contains("removed value api.two: 2"), "{err}");
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

/// The package `badmod`, whose module path is not correct (E0623): it is
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
    assert_eq!(read(&dir.join("i.toml")), APP_TOML);
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
    let toml = "name = \"app\"\n\n[dependencies]\n# why we use lib\nlib = \"example.com/lib@1.0.0\" # pinned\n# the other one\nother = \"example.com/lib@2.0.0\"\n";
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
    let toml =
        "# The app.\r\nname = \"app\"\r\n\r\n[dependencies]\r\nlib = \"example.com/lib@1.0.0\"\r\n";
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
            ("src/a.i", " (T(..))\n\nT { T(Int) }\n"),
            ("src/b.i", " (T(..))\n\nT { T(String) }\n"),
            ("src/api.i", api),
        ],
    );
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
    let dir = two_types(&w, "field", " (pub/a, pub/b)\n (W(..))\n\nW { W(a.T) }\n");
    let (ok, out) = try_publish(
        &w,
        &dir,
        &[("src/api.i", " (pub/a, pub/b)\n (W(..))\n\nW { W(b.T) }\n")],
        "1.0.1",
    );
    assert!(!ok, "{out}");
    assert!(out.contains("changed constructors api.W: W(a.T)"), "{out}");
    assert!(out.contains("to constructors api.W: W(b.T)"), "{out}");
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
        " (x = pub/a, y = pub/b)\n (f)\n\nf = (v: x.T) { v }\n",
    );
    let (ok, out) = try_publish(
        &w,
        &dir,
        &[(
            "src/api.i",
            " (x = pub/b, y = pub/a)\n (f)\n\nf = (v: x.T) { v }\n",
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
        " (pub/a)\n (id, f, g)\n\nid(A) = (x: A) -> A { x }\n\nf = (v: a.T) { v }\n\ng = (v: a.T) { v }\n",
    );
    let (ok, out) = try_publish(
        &w,
        &dir,
        &[(
            "src/api.i",
            " (aa = pub/a, (T) = pub/a)\n (id, f, g)\n\nid(B) = (x: B) -> B { x }\n\nf = (v: aa.T) { v }\n\ng = (v: T) { v }\n",
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
    let w = World::new();
    let v = |ty: &str| format!(" (V(..))\n\nV {{ V({ty}) }}\n");
    let (v1, v2) = (v("Int"), v("String"));
    w.repo(
        "dep",
        &[
            ("1.0.0", &[("i.toml", "name = \"dep\"\n"), ("src/v.i", &v1)]),
            ("2.0.0", &[("i.toml", "name = \"dep\"\n"), ("src/v.i", &v2)]),
        ],
    );
    let api = " (dep/v)\n (f, W(..))\n\nf = (x: v.V) { 1 }\n\nW { W(v.V) }\n";
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
    let project = dir.display().to_string();
    w.ok(&["check", "--project", &project]);
    git(&dir, &["init", "-q"]);
    commit(&dir, "first");
    w.ok(&["publish", "1.0.0", "--project", &project]);
    w.ok(&["add", "dep", "example.com/dep@2.0.0", "--project", &project]);
    let (ok, out) = try_publish(&w, &dir, &[], "1.0.1");
    assert!(!ok, "{out}");
    assert!(
        out.contains("changed value api.f: (x: example.com/dep@1:v.V) -> 1"),
        "{out}"
    );
    assert!(
        out.contains("changed constructors api.W: W(example.com/dep@1:v.V)"),
        "{out}"
    );
    assert!(out.contains("a new major version"), "{out}");
}

#[test]
fn publish_sees_what_the_host_must_give() {
    let w = World::new();
    let api = " (now, Io)\n\n@external(\"env\", \"now\")\nnow: () -> Int\n\n@host(\"pub:io/v1\")\nIo { Io(print: (text: String) -> Nil) }\n";
    let dir = w.project(
        "pub",
        &[
            ("i.toml", "name = \"pub\"\n"),
            (".gitignore", "/build/\n"),
            ("src/api.i", api),
        ],
    );
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
        &[("src/api.i", &api.replace("print:", "show:"))],
        "1.0.2",
    );
    assert!(!ok, "{out}");
    assert!(
        out.contains("changed constructors api.Io: Io(print: (text: String) -> Nil)"),
        "{out}"
    );
}

/// A package that starts to print imports `i/io`: the host must give the
/// standard service, as if the package declared it (§9.7).
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
    git(&dir, &["init", "-q"]);
    commit(&dir, "first");
    let project = dir.display().to_string();
    w.ok(&["publish", "1.0.0", "--project", &project]);

    let printing =
        " ((Io) = i/io)\n\n (f, say)\n\nf = () { 1 }\n\nsay = (io ! Io) { io.print(\"one\") }\n";
    let (ok, out) = try_publish(&w, &dir, &[("src/api.i", printing)], "1.0.1");
    assert!(!ok, "{out}");
    assert!(
        out.contains("added host \"i:io/v1\": (print: (text: String) -> Nil)"),
        "{out}"
    );
    assert!(out.contains("a new minor version"), "{out}");
}

#[test]
fn publish_tells_apart_types_that_are_shown_alike() {
    let w = World::new();
    // Two packages, both named `util`, each with a module `u` and a type
    // `T`; `pa` and `pb` each use one of them as `util`.
    let util = |ty: &str| format!(" (T(..))\n\nT {{ T({ty}) }}\n");
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
    let get = " (util/u)\n (get)\n\nget = (x: u.T) { x }\n";
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
        out.contains("changed value m.h: (x: example.com/u1@1:u.T) -> example.com/u1@1:u.T"),
        "{out}"
    );
    assert!(
        out.contains("to value m.h: (x: example.com/u2@1:u.T) -> example.com/u2@1:u.T"),
        "{out}"
    );
    assert!(out.contains("a new major version"), "{out}");
}

#[test]
fn publish_sees_what_the_host_must_give_though_it_is_not_exported() {
    let w = World::new();
    let api = " (f)\n\n@external(\"env\", \"now\")\nnow: () -> Int\n\n@host(\"pub:io/v1\")\nIo { Io(print: (text: String) -> Nil) }\n\nf = () { now() }\n";
    let dir = w.project(
        "pub",
        &[
            ("i.toml", "name = \"pub\"\n"),
            (".gitignore", "/build/\n"),
            ("src/api.i", api),
        ],
    );
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
            &api.replace("now: () -> Int", "now: (n: Int) -> Int")
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
        &[("src/api.i", &api.replace("print:", "show:"))],
        "1.0.2",
    );
    assert!(!ok, "{out}");
    assert!(
        out.contains("changed host \"pub:io/v1\": (print: (text: String) -> Nil)"),
        "{out}"
    );
    assert!(
        out.contains("to host \"pub:io/v1\": (show: (text: String) -> Nil)"),
        "{out}"
    );
}

#[test]
fn add_over_a_dependency_written_as_a_table_keeps_its_comments() {
    let w = World::new();
    lib_repo(&w);
    for name in ["util", "util2"] {
        w.project(
            name,
            &[("i.toml", "name = \"util\"\n"), ("src/u.i", "x = 1\n")],
        );
    }
    let toml = "name = \"app\"\n\n# the util\n[dependencies.lib]\npath = \"../util\" # p\n\n[wasm]\nexports = []\n";
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
        "name = \"app\"\n\n# the util\n[dependencies]\nlib = \"example.com/lib@1.2.0\"\n\n[wasm]\nexports = []\n"
    );

    // When `[dependencies]` has a header, the comments go above the entry.
    let toml = "name = \"app\"\n\n[dependencies]\nother = \"example.com/lib@2.0.0\"\n\n# the util\n[dependencies.lib] # mine\npath = \"../util\"\n";
    let dir = w.project("app2", &[("i.toml", toml), main]);
    let project = dir.display().to_string();
    w.ok(&["add", "lib", "example.com/lib@1.2.0", "--project", &project]);
    assert_eq!(
        read(&dir.join("i.toml")),
        "name = \"app\"\n\n[dependencies]\nother = \"example.com/lib@2.0.0\"\n# the util\nlib = \"example.com/lib@1.2.0\" # mine\n"
    );
}

#[test]
fn build_and_check_explain_an_old_syntax_dependency_tag() {
    let w = World::new();
    // old-syntax-check: intentional old tag; parser E0101 for the old match.
    let old = /* old-syntax-check: E0101 */ "import ((inspect) = i/core)\nexport (f)\nf = () { case 1 { 0 -> 0  _ -> 1 } }\n";
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
                "name = \"app\"\n[dependencies]\nlegacy = \"example.com/legacy@1.0.0\"\n",
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
