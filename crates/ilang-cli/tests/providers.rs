//! Execution of the checked provider graph (§2.6.1), through real Wasm entries.
use ilang_cli::{BuildOptions, Level, Target, build};
use ilang_host::{Outcome, Value};

fn project(source: &str, plan: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.i"), source).unwrap();
    std::fs::write(dir.path().join("i.toml"), format!("name = \"app\"\n{plan}")).unwrap();
    dir
}
fn options(target: Target) -> BuildOptions {
    BuildOptions {
        tests: true,
        opt: Level::O0,
        offline: true,
        target,
    }
}
fn run(source: &str, plan: &str) -> Outcome {
    let dir = project(source, plan);
    let b = build(dir.path(), options(Target::Native)).unwrap_or_else(|e| panic!("{e}"));
    b.host_program()
        .unwrap()
        .run_entry(b.host_options())
        .unwrap()
}
fn passes(source: &str, plan: &str) {
    assert!(matches!(run(source, plan), Outcome::Ok(_)));
}
fn row(service: &str, factory: &str) -> String {
    format!("[providers.\"app/main.{service}\"]\nnative = \"app/main.{factory}\"\n")
}
#[test]
fn private_user_factory_and_transitive_dependency_execute() {
    passes(
        "Config { Config(value: Int) }\nService { Service(read: () -> Int) }\nconfig = () { Config(40) }\nmake = (c ! Config) { Service(() { c.value + 2 }) }\nmain = (s ! Service) { assert(s.read() == 42) }",
        &[row("Config", "config"), row("Service", "make")].concat(),
    );
}
#[test]
fn parameter_instances_have_separate_factory_environments() {
    passes(
        "(i/key)\nData(A) { Data(read: () -> A, key: Key(Nil)) }\nCell(A) { Cell(read: () -> A, key: Key(Nil)) }\nLeft { Left(key: Key(Nil)) }\nRight { Right(key: Key(Nil)) }\ndata(A) = () -> Data(A) { Data(() { panic(\"unused value\") }, key.new()) }\ncell(A) = (d ! Data(A)) -> Cell(A) ! Data(A) { Cell(d.read, d.key) }\nleft = (c ! Cell(Int)) { Left(c.key) }\nright = (c ! Cell(String)) { Right(c.key) }\nmain = (l ! Left, r ! Right) { assert(l.key != r.key) }",
        &[
            row("Data", "data"),
            row("Cell", "cell"),
            row("Left", "left"),
            row("Right", "right"),
        ]
        .concat(),
    );
}
#[test]
fn seed_aware_factory_runs_inside_user_adapter() {
    passes(
        "S { S(value: Int) }\nretain = (x) { x }\nadapt(A,E,N) = (body: () -> A ^ E ! N) -> A ^ E ! N - S { ! S = S(40) retain(body()) }\nmake = (seeds: List(S)) -> S { seeds |> { [s] -> S(s.value + 2) _ -> panic(\"missing seed\") } }\nmain = (s ! S) { assert(s.value == 42) }",
        "[providers.\"app/main.S\"]\nrole = \"runner\"\nadapter = \"app/main.adapt\"\nnative = {factory=\"app/main.make\", arguments=[\"seed\"], seed_required=true}\n[provider_adapters.\"app/main.adapt\"]\nsupplies = [\"app/main.S\"]\nrequires = []\n",
    );
}
#[test]
fn adapter_cannot_call_body_twice() {
    let outcome = run(
        "S { S }\nadapt(A,E,N) = (body: () -> A ^ E ! N) -> A ^ E ! N - S { ! S = S x = body() body() }\nmake = () { S }\nmain = (s ! S) { Nil }",
        "[providers.\"app/main.S\"]\nrole = \"scheduler\"\nadapter = \"app/main.adapt\"\nnative = \"app/main.make\"\n[provider_adapters.\"app/main.adapt\"]\nsupplies = [\"app/main.S\"]\nrequires = []\n",
    );
    assert!(
        matches!(&outcome, Outcome::Stop(s) if s.message.as_deref().is_some_and(|s| s.starts_with("panic: provider: body already called"))),
        "{outcome:?}"
    );
}
#[test]
fn columns_and_cli_ilang_alias_select_their_factories() {
    let dir = project(
        "(S, probe)\nS { S(value: Int) }\na = () { S(1) }\nb = () { S(2) }\nc = () { S(3) }\nd = () { S(4) }\ne = () { S(5) }\nprobe = (s ! S) { s.value }\n",
        "[wasm]\nexports = [\"app/main.probe\"]\n[providers.\"app/main.S\"]\njs = \"app/main.a\"\nnodejs = \"app/main.b\"\nnative = \"app/main.c\"\nwasi = \"app/main.d\"\nhost = \"app/main.e\"\n",
    );
    for (target, expected) in [
        (Target::Js, 1),
        (Target::Nodejs, 2),
        (Target::Native, 3),
        (Target::Host, 5),
    ] {
        let b = build(dir.path(), options(target)).unwrap_or_else(|e| panic!("{e}"));
        let mut instance = b
            .host_program()
            .unwrap()
            .instantiate(b.host_options())
            .unwrap();
        assert!(
            matches!(instance.call("app/main.probe", &[]).unwrap(), Outcome::Ok(Value::Int(n)) if n == expected.into())
        );
    }
    for target in ["js", "nodejs", "native", "host", "ilang"] {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_i"))
            .args(["build", "--opt", "0", "--target", target, "--project"])
            .arg(dir.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
#[test]
fn missing_provider_is_a_build_error_but_source_check_passes() {
    let dir = project("Missing { Missing }\nmain = (s ! Missing) { Nil }", "");
    let error = build(dir.path(), options(Target::Native))
        .err()
        .unwrap()
        .to_string();
    for text in ["native", "app/main.main", "app/main.Missing"] {
        assert!(error.contains(text), "{error}");
    }
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["check", "--project"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["check", "--target", "native", "--project"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
}
#[test]
fn missing_host_registration_names_the_service_before_execution() {
    let dir = project(
        "@host(\"custom\")\nService { Service(read: () -> Int) }\nmain = (s ! Service) { Nil }",
        "",
    );
    let b = build(dir.path(), options(Target::Native)).unwrap_or_else(|e| panic!("{e}"));
    let error = b
        .host_program()
        .unwrap()
        .instantiate(b.host_options())
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("app/main.Service"), "{error}");
}
#[test]
fn synchronous_export_rejects_transitive_request_provider() {
    let dir = project(
        "(S, probe)\nS { S }\n@host(\"custom\", \"request\")\nWork { Work(start: () -> Request(Int)) }\nmake = (w ! Work) { S }\nprobe = (s ! S) { Nil }",
        "[wasm]\nexports = [\"app/main.probe\"]\nsynchronous = [\"app/main.probe\"]\n[providers.\"app/main.S\"]\nnative = \"app/main.make\"\n",
    );
    let error = build(dir.path(), options(Target::Native))
        .err()
        .unwrap()
        .to_string();
    for text in ["synchronous", "app/main.Work", "app/main.S"] {
        assert!(error.contains(text), "{error}");
    }
}

#[test]
fn diamond_shares_a_factory_value_once_per_invocation() {
    passes(
        "(i/key)\nD { D(key: Key(Nil)) }\nB { B(key: Key(Nil)) }\nC { C(key: Key(Nil)) }\nd = () { D(key.new()) }\nb = (d ! D) { B(d.key) }\nc = (d ! D) { C(d.key) }\nmain = (b ! B, c ! C) { assert(b.key == c.key) }",
        &[row("D", "d"), row("B", "b"), row("C", "c")].concat(),
    );
}
#[test]
fn generated_root_key_is_shared_without_a_request_service() {
    passes(
        "A { A(key: Key(Nil)) }\nB { B(key: Key(Nil)) }\na = (key: Key(Nil)) { A(key) }\nb = (key: Key(Nil)) { B(key) }\nmain = (a ! A, b ! B) { assert(a.key == b.key) }",
        "[providers.\"app/main.A\"]\nnative = {factory=\"app/main.a\", arguments=[\"entry-root-key\"]}\n[providers.\"app/main.B\"]\nnative = {factory=\"app/main.b\", arguments=[\"entry-root-key\"]}\n",
    );
}

#[test]
fn nullary_override_replaces_the_adapters_service_value() {
    passes(
        "S { S(value: Int) }\nretain = (x) { x }\nadapt(A,E,N) = (body: () -> A ^ E ! N) -> A ^ E ! N - S { ! S = S(40) retain(body()) }\nmake = () { S(42) }\nmain = (s ! S) { assert(s.value == 42) }",
        "[providers.\"app/main.S\"]\nrole = \"runner\"\nadapter = \"app/main.adapt\"\nnative = \"app/main.make\"\n[provider_adapters.\"app/main.adapt\"]\nsupplies = [\"app/main.S\"]\nrequires = []\n",
    );
}

#[test]
fn target_schema_is_checked_without_an_entry() {
    let dir = project("S { S }\nmake = () { 42 }", &row("S", "make"));
    let error = build(dir.path(), options(Target::Native))
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("must return its service"), "{error}");
}
