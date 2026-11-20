//! A macro whose quote holds an `&` chain and a `|>` match must produce the
//! same user-visible program as the equivalent writing without a macro.
//!
//! Resolution recomputes source facts from the parsed tree. For an expanded
//! module that tree is no longer the written one, so the boundary facts that
//! decide chain grouping could silently change. This pins the result users
//! see: `check`, `run`, the expanded `main`, and the Wasm module's output.

#[path = "common/test_project.rs"]
mod test_project;

use std::path::Path;
use std::process::{Command, Output};

fn package(root: &Path, name: &str, files: &[(&str, &str)]) {
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("i.toml"),
        format!("name = \"{name}\"\n{}", test_project::stdlib_dependency()),
    )
    .unwrap();
    for (path, text) in files {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

fn i(args: &[&str], project: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_i"))
        .args(args)
        .arg("--project")
        .arg(project)
        .output()
        .unwrap()
}

fn ok(out: &Output, what: &str) {
    assert!(
        out.status.success(),
        "{what} failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The `(= name …)` item of the `// app/main` block of `i expand --to is`.
fn expanded_item(project: &Path, name: &str) -> String {
    let out = i(&["expand", "--to", "is"], project);
    ok(&out, "expand");
    let text = String::from_utf8(out.stdout).unwrap();
    let head = format!("(= {name} ");
    let mut in_main = false;
    for line in text.lines() {
        if line.starts_with("// ") {
            in_main = line == "// app/main";
            continue;
        }
        if in_main && line.starts_with(&head) {
            return line.to_string();
        }
    }
    panic!("no `{head}…)` in the expansion of {}", project.display());
}

fn expanded_main(project: &Path) -> String {
    expanded_item(project, "main")
}

fn run_wasm(wasm: &Path) -> String {
    let out = Command::new("wasmtime")
        .arg("run")
        .arg(wasm)
        .output()
        .expect("wasmtime runs the built module");
    assert!(
        out.status.success(),
        "wasmtime failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn macro_facts_boundary_keeps_chain_and_match_semantics() {
    let dir = tempfile::tempdir().unwrap();
    let macro_project = dir.path().join("macro");
    let literal_project = dir.path().join("literal");

    // `pick` is a macro: its body is a quote whose holes contain the `&`
    // chain and the `|>` match. The expansion is inlined at the call site.
    package(
        &macro_project,
        "app",
        &[
            (
                "src/picks.is",
                "(, pick)\n\
                 (@ macro (= pick (-> (, a b) _ `(, ((& (-> (, (True)) _ ~a) (-> (, (False)) _ ~b)) (True)) (|> (True) (& (-> (, (True)) _ ~b) (-> (, (False)) _ ~a)))))))\n",
            ),
            (
                "src/main.is",
                "(, (= (, Io) (/ i io)) (= (, pick) (/ app picks)))\n\
                 (= main (-> (,) (<- _ (, (: io Io))) {(= (, x y) (pick 1 2)) ((. io print) (|> x (& (-> (, 1) _ \"x=1\") (-> (, _) _ \"x=other\")))) ((. io print) (|> y (& (-> (, 1) _ \"y=1\") (-> (, _) _ \"y=other\"))))}))\n",
            ),
        ],
    );
    // The same program with the tuple written out instead of expanded.
    package(
        &literal_project,
        "app",
        &[(
            "src/main.is",
            "(, (= (, Io) (/ i io)))\n\
                 (= main (-> (,) (<- _ (, (: io Io))) {(= (, x y) (, ((& (-> (, (True)) _ 1) (-> (, (False)) _ 2)) (True)) (|> (True) (& (-> (, (True)) _ 2) (-> (, (False)) _ 1))))) ((. io print) (|> x (& (-> (, 1) _ \"x=1\") (-> (, _) _ \"x=other\")))) ((. io print) (|> y (& (-> (, 1) _ \"y=1\") (-> (, _) _ \"y=other\"))))}))\n",
        )],
    );

    let expected = "x=1\ny=other\n";
    for project in [&macro_project, &literal_project] {
        ok(&i(&["check"], project), "check");
        let run = i(&["run"], project);
        ok(&run, "run");
        assert_eq!(expected, String::from_utf8(run.stdout).unwrap());
    }

    // The expanded main is the literal writing of the same program.
    assert_eq!(
        expanded_main(&macro_project),
        expanded_main(&literal_project),
    );

    // The Wasm built from the expanded module runs the same program.
    ok(
        &i(&["build", "--target", "wasi"], &macro_project),
        "build macro",
    );
    ok(
        &i(&["build", "--target", "wasi"], &literal_project),
        "build literal",
    );
    assert_eq!(
        expected,
        run_wasm(&macro_project.join("build/app.wasm")),
        "the macro-expanded Wasm runs the literal program",
    );
    assert_eq!(
        expected,
        run_wasm(&literal_project.join("build/app.wasm")),
        "the literal Wasm runs the literal program",
    );
}

#[test]
fn quote_hole_marks_keep_a_repeated_chain_and_match_argument() {
    let dir = tempfile::tempdir().unwrap();
    let macro_project = dir.path().join("macro");
    let literal_project = dir.path().join("literal");

    // The same hole appears twice, and its syntax is a `|>` match over a
    // chain. Losing the hole's expression mark would regroup or renumber it.
    package(
        &macro_project,
        "app",
        &[
            (
                "src/repeat.is",
                "(, twice)\n(@ macro (= twice (-> (, x) _ `(, ~x ~x))))\n",
            ),
            (
                "src/main.i",
                "(\n\t(Io) = i/io\n\t(twice) = app/repeat\n)\n\n\
                 main = () <- (io : Io) {\n\tn = 7\n\t\
                 (p, q) = twice(n |> (7) { \"seven\" } & (_) { \"other\" })\n\t\
                 io.print(p)\n\tio.print(q)\n}\n",
            ),
        ],
    );
    package(
        &literal_project,
        "app",
        &[(
            "src/main.i",
            "(\n\t(Io) = i/io\n)\n\n\
             main = () <- (io : Io) {\n\tn = 7\n\t\
             (p, q) = (n |> (7) { \"seven\" } & (_) { \"other\" }, n |> (7) { \"seven\" } & (_) { \"other\" })\n\t\
             io.print(p)\n\tio.print(q)\n}\n",
        )],
    );

    let expected = "seven\nseven\n";
    for project in [&macro_project, &literal_project] {
        ok(&i(&["check"], project), "check");
        let run = i(&["run"], project);
        ok(&run, "run");
        assert_eq!(expected, String::from_utf8(run.stdout).unwrap());
    }
    assert_eq!(
        expanded_main(&macro_project),
        expanded_main(&literal_project),
    );
}

#[test]
fn a_generated_binding_without_parameters_resolves_like_a_written_one() {
    let dir = tempfile::tempdir().unwrap();
    let macro_project = dir.path().join("macro");
    let literal_project = dir.path().join("literal");

    // The item macro returns `(= value "ok")`, a binding with no parameter
    // list of its own; resolution used to get an empty formal list for it.
    package(
        &macro_project,
        "app",
        &[
            (
                "src/macros.is",
                "(, (= syntax (/ i syntax)))\n(, make)\n\
                 (@ macro (= make (-> (, x) (<- _ (, (: it (. syntax Items)))) {((. it expose) x) `(= ~x \"ok\")})))\n",
            ),
            (
                "src/main.i",
                "(\n\t(Io) = i/io, m = app/macros\n)\nm.make(value)\n\
                 main = () <- (io : Io) { io.print(value) }\n",
            ),
        ],
    );
    package(
        &literal_project,
        "app",
        &[(
            "src/main.i",
            "(\n\t(Io) = i/io\n)\nvalue = \"ok\"\n\
             main = () <- (io : Io) { io.print(value) }\n",
        )],
    );

    let expected = "ok\n";
    for project in [&macro_project, &literal_project] {
        ok(&i(&["check"], project), "check");
        let run = i(&["run"], project);
        ok(&run, "run");
        assert_eq!(expected, String::from_utf8(run.stdout).unwrap());
    }
    assert_eq!(
        expanded_item(&macro_project, "value"),
        expanded_item(&literal_project, "value"),
    );
}
