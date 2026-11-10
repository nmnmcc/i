//! Review programs exercise the checker and the actual runtime boundary.
use std::process::Command;

fn project(source: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.i"), source).unwrap();
    dir
}

fn check_and_run(name: &str, source: &str, error: Option<&str>, stdout: &str) {
    let dir = project(source);
    for args in [
        vec!["check", "--types"],
        vec!["run", "--opt", "0"],
        vec!["run", "--opt", "2"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_i"))
            .args(&args)
            .arg("--project")
            .arg(dir.path())
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&out.stderr);
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            !err.contains("trap") && !err.contains("panicked"),
            "{name} {args:?}: {err}"
        );
        match error {
            Some(code) => {
                assert!(
                    !out.status.success() && err.contains(code),
                    "{name} {args:?}: {text}\n{err}"
                );
            }
            None => {
                assert!(out.status.success(), "{name} {args:?}: {text}\n{err}");
                if args[0] == "run" {
                    assert_eq!(text, stdout, "{name} {args:?}");
                }
            }
        }
    }
}

#[test]
fn same_service_obligations_survive_annotations() {
    // §4.8 rule 9 and §9.2: replacing S(Dep) cannot handle S(Never),
    // nor can replacing Svc(String) handle a body that requires Svc(Int).
    for name in ["x1", "x2", "x3", "x4", "x6", "x7", "x8", "z1", "z4"] {
        let source =
            std::fs::read_to_string(format!("{}/tests/k4/{name}.i", env!("CARGO_MANIFEST_DIR")))
                .unwrap();
        check_and_run(name, &source, Some("E0515"), "");
    }
}

#[test]
fn rows_inside_type_arguments_keep_service_arguments_equal() {
    // §4.5's row ruling applies inside T's argument. These callbacks need
    // a different S form or an unprovided Dep; neither may reach runtime.
    for name in [
        "row_alias",
        "direct_row",
        "effect_row_alias",
        "latent_row",
        "alias_kind_env",
        "alias_kind_error",
        "g_ann_SDep",
        "g_inf_SDep",
        "m3",
        "m4",
        "r6",
        "b4",
        "b7",
        "r2",
        "r3",
    ] {
        let source =
            std::fs::read_to_string(format!("{}/tests/k4/{name}.i", env!("CARGO_MANIFEST_DIR")))
                .unwrap();
        check_and_run(
            name,
            &source,
            Some(if ["b4", "b7", "r2", "r3"].contains(&name) {
                "E0522"
            } else {
                "E0515"
            }),
            "",
        );
    }
}

#[test]
fn field_access_has_the_same_finite_type_as_a_pattern() {
    // §4.8 rules 4, 5, 7, 13: reading the field retains N | S(N).
    // Install S(Never), whose callback returns 1, and invoke the helper.
    for n in 21..=40 {
        let path = format!("{}/tests/k4/q{n}.i", env!("CARGO_MANIFEST_DIR"));
        let mut source = std::fs::read_to_string(path).unwrap();
        source.push_str("\n@main\nmain = () { ! S(Never) = S(() { 1 }) assert(fw() == 1) }\n");
        check_and_run(&format!("q{n}"), &source, None, "");
    }
    let source = include_str!("k4/r4.i");
    check_and_run("r4", source, None, "four\nthree\n");
}

#[test]
fn generic_handlers_have_finite_types_with_or_without_annotations() {
    // §4.8 rule 9 removes S(N), retaining N and its same-form obligation.
    // The body and the supplied S(Never) both return 1.
    for n in 1..=16 {
        let mut source =
            std::fs::read_to_string(format!("{}/tests/k4/w{n}.i", env!("CARGO_MANIFEST_DIR")))
                .unwrap();
        let service = if n >= 9 {
            "provided = S(() { 1 }, (_: Never) { Nil })"
        } else {
            "provided = S(() { 1 })"
        };
        let service = service.split_once('=').unwrap().1.trim();
        let call = match n {
            1 | 2 | 9 | 10 => "wrap(() { use() })".to_string(),
            3 | 4 | 5 | 11 | 12 | 13 => "wrap(() { use() }, VALUE)".replace("VALUE", service),
            _ => "wrap(VALUE)".replace("VALUE", service),
        };
        let prefix = source.clone();
        source.push_str(&"\n@main\nmain = () { assert(CALL == 1) }\n".replace("CALL", &call));
        check_and_run(&format!("w{n}"), &source, None, "");
        let dependency = "Dep { Dep(value: Int) }\nget = (d ! Dep) { d.value }\n";
        let service = if n >= 9 {
            "provided = S(() { get() }, (_: Dep) { Nil })"
        } else {
            "provided = S(() { get() })"
        };
        let service = service.split_once('=').unwrap().1.trim();
        let call = match n {
            1 | 2 | 9 | 10 => "wrap(() { use() })".to_string(),
            3 | 4 | 5 | 11 | 12 | 13 => "wrap(() { use() }, VALUE)".replace("VALUE", service),
            _ => "wrap(VALUE)".replace("VALUE", service),
        };
        let expected = if [1, 2, 9, 10].contains(&n) { "1" } else { "7" };
        let main = "\n@main\nmain = () { ! Dep = Dep(7) assert(CALL == EXPECTED) }\n"
            .replace("CALL", &call)
            .replace("EXPECTED", expected);
        check_and_run(
            &format!("w{n}-dependency"),
            &format!("{dependency}{prefix}{main}"),
            None,
            "",
        );
    }
}

#[test]
fn previous_finite_schemes_reject_missing_providers_before_runtime() {
    // §4.8 preserves requirements across calls, and §10.2 rejects an entry
    // without providers. Exercise the original K4 shapes at the CLI boundary.
    let exact = "S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
@main
main(N) = () { local: () -> Nil ! N | S(N) = () { use() } local() }";
    for inferred in [false, true] {
        let source = if inferred {
            exact.replace("local: () -> Nil ! N | S(N)", "local")
        } else {
            exact.to_string()
        };
        check_and_run("original-entry", &source, Some("E0522"), "");
        // §4.8 rule 4: merely making an unused closure needs no service.
        check_and_run(
            "original-unused-closure",
            &source.replace("local() }", "Nil }"),
            None,
            "",
        );
    }
    let sources = [
        "S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) -> Nil ! N | S(N) { S(f) = s f() }
example = () { use() }",
        "S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(N) = (body: () -> Nil ! N) { body() }
example = () { run(() { use() }) }",
        "Inner(A, E, N) { Inner((A) -> A ? E ! N, (N) -> Nil) }
Outer(A, E, N) { Outer(Inner(A, E, N)) }
use(A, E, N) = (x: A, s ! Outer(A, E, N)) -> A ? E ! N | Outer(A, E, N) {
 Outer(Inner(f, _)) = s f(x)
}
run(A, E, N) = (body: () -> A ? E ! N) { body() }
example = () { run(() { use(1) }) }",
        "Dependency { Dependency(value: Int) }
get = (d ! Dependency) { d.value }
Inner(N) { Inner(() -> Int ! N, (N) -> Nil) }
read(N) = (s ! Inner(N)) { Inner(f, _) = s f() }
Outer(N) { Outer(() -> Int ! N, (N) -> Nil) }
use(N) = (s ! Outer(N)) -> Int ! N | Outer(N) { Outer(f, _) = s f() }
run(A, N) = (body: () -> A ! N) { body() }
example = () { run(() { use() }) }",
        "(i/fail)
Entry { Entry(message: String) }
Sink(E, N) { Sink((Entry) -> Nil ? E ! N, (N) -> Nil) }
write(E, N) = (entry: Entry, sink ! Sink(E, N)) -> Nil ? E ! N | Sink(E, N) {
 Sink(send, _) = sink send(entry)
}
example = () { fail.catch(() { write(Entry(\"seen\")) }) }",
        "S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
example(N) = () { local: () -> Nil ! N | S(N) = () { use() } local() }",
        "S(E, N) { S(() -> Nil ? E ! N) }
use(E, N) = (s ! S(E, N)) { S(f) = s f() }
run(E, N) = (body: () -> Nil ? E ! N) { body() }
example(E, N) = () { run(() -> Nil ? E ! N | S(E, N) { use() }) }",
    ];
    for (i, source) in sources.iter().enumerate() {
        for inferred in [false, true] {
            let mut source = source.to_string();
            if inferred {
                source = source
                    .replace("local: () -> Nil ! N | S(N)", "local")
                    .replace("-> Nil ! N | S(N)", "")
                    .replace("-> A ? E ! N | Outer(A, E, N)", "")
                    .replace("-> Int ! N | Outer(N)", "")
                    .replace("-> Nil ? E ! N | Sink(E, N)", "")
                    .replace("() -> Nil ? E ! N | S(E, N) {", "() {");
            }
            source.push_str("\n@main\nmain = () { example() }\n");
            check_and_run(
                &format!("original-{i}-{inferred}"),
                &source,
                Some("E0522"),
                "",
            );
        }
    }
    let mut chain = String::from(
        "S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(N) = (body: () -> Nil ! N) { body() }
forward_0 = () { run(() { use() }) }\n",
    );
    for n in 1..64 {
        chain.push_str(
            &"forward_next = () { run(() { forward_previous() }) }\n"
                .replace("forward_next", &format!("forward_{n}"))
                .replace("forward_previous", &format!("forward_{}", n - 1)),
        );
    }
    chain.push_str("\n@main\nmain = () { forward_63() }\n");
    check_and_run("original-helper-chain", &chain, Some("E0522"), "");
}

#[test]
fn concrete_handler_holes_have_the_same_runtime_type() {
    // §4.5's equality rule and §4.8 rule 9 give the same handler for
    // Svc(_) fixed to Int and Svc(Int). The installed callback returns 6.
    for inferred in [false, true] {
        for hole in [false, true] {
            let mut source = include_str!("k4/closed_handler_hole.i").to_string();
            if !hole {
                source = source.replace("! Svc(_) = svc", "! Svc(Int) = svc");
            }
            if inferred {
                source = source.replace(") -> Int ! N {", ") {");
            }
            check_and_run("closed-handler-hole", &source, None, "");
        }
    }
}

#[test]
fn fresh_row_unions_keep_equal_service_forms() {
    // §9.2 permits one S(Int) form to serve both callbacks. Freshening
    // must retain this principal T(A | B) result and reject S(String).
    check_and_run(
        "latent-row-same",
        include_str!("k4/latent_row_same.i"),
        None,
        "",
    );
}

#[test]
fn scalar_error_rows_keep_literal_set_inclusion() {
    // §4.5 rule 8: joining error rows 1 and 2 gives 1 | 2; §9.3
    // reifies the selected literal as Err. Service equality must not
    // turn those scalar value sets into a conflicting service form.
    check_and_run(
        "scalar-error-rows",
        include_str!("k4/error_row_values.i"),
        None,
        "",
    );
}
