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
fn named_provider_diagnostics_match_all_fourteen_landed_projects() {
    // §9.2 checks every use of a supplied service, including cached handlers.
    // Use each real manifest: mixed_install_12 and mixed_install_6 import
    // std/test and must expand their macros before comparing diagnostics.
    // These counts come from landed tip cc26b8d9, not a stage-three recording.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/review/cases");
    for (case, count, provider_count) in [
        ("review-k4d/_loose/fuzz/e0505/21_4", 3, 2),
        ("review-k4d/_loose/p/b1", 5, 5),
        ("review-k4d/_loose/p/r6", 3, 3),
        ("review-k4d/fuzz/m0", 2, 2),
        ("review-k4d/fuzz/m21", 3, 2),
        ("review-k4d/fuzz/m32", 9, 8),
        ("review-k4d/fuzz/w2", 3, 2),
        ("review-k4d/fuzz/w3", 2, 2),
        ("review-k4d/fuzz/w4", 11, 4),
        ("review-k4d/proj/b1", 5, 5),
        ("review-k4d/proj/r6", 3, 3),
        ("review-k4s/hidden_conflict_install", 4, 4),
        ("review-k4s/mixed_install_12", 6, 5),
        ("review-k4s/mixed_install_6", 7, 6),
    ] {
        let dir = root.join(case);
        let observation: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("expected.json")).unwrap())
                .unwrap();
        let saved = &observation["outcome"]["check"];
        assert_eq!(saved["exit"].as_i64(), Some(1), "landed rejection: {case}");
        let mut expected: Vec<_> = saved["stderr"]
            .as_str()
            .unwrap()
            .lines()
            .filter(|line| line.contains("error["))
            .collect();
        assert_eq!(expected.len(), count, "landed diagnostic count: {case}");
        assert_eq!(
            expected
                .iter()
                .filter(|line| line.contains("error[E0515]"))
                .count(),
            provider_count,
            "landed provider diagnostic count: {case}"
        );
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_i"))
            .args(["check", "--types", "--offline", "--project"])
            .arg(&dir)
            .output()
            .unwrap();
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert_eq!(out.status.code(), Some(1), "{case}: {stderr}");
        let mut actual: Vec<_> = stderr
            .lines()
            .filter(|line| line.contains("error["))
            .collect();
        expected.sort_unstable();
        actual.sort_unstable();
        assert_eq!(actual, expected, "{case}");
    }
}

#[test]
fn private_user_factory_and_transitive_dependency_execute() {
    passes(
        "Config = (value = Int)\nService = (read = () -> Int)\nconfig = () { Config(value = 40) }\nmake = () <- (c: Config) { Service(read = () { c.value + 2 }) }\nmain = () <- (s: Service) { assert(s.read() == 42) }",
        &[row("Config", "config"), row("Service", "make")].concat(),
    );
}
#[test]
fn parameter_instances_have_separate_factory_environments() {
    passes(
        "(\n\ti/key\n)\nData(A) = (read = () -> A, key = Key(Nil))\nCell(A) = (read = () -> A, key = Key(Nil))\nLeft = (key = Key(Nil))\nRight = (key = Key(Nil))\ndata = <A>() -> Data(A) {\n\tData(read = () { panic(\"unused value\") }, key = key.new())\n}\ncell = <A>() <- (d: Data(A)) -> Cell(A) { Cell(read = d.read, key = d.key) }\nleft = () <- (c: Cell(Int)) { Left(key = c.key) }\nright = () <- (c: Cell(String)) { Right(key = c.key) }\nmain = () <- (l: Left, r: Right) { assert(l.key != r.key) }",
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
        "(\n\ti/core\n)\nS = (value = Int)\nretain = (x) { x }\nadapt = <A, N>(body: () <- N -> A) -> A {\n\tcore.portable(body) |>\n\t\t(True) { panic(\"entry body escaped portability\") }\n\t\t| (False) { Nil }\n\tretain(body()) <- (S(value = 40))\n}\nmake = (seeds: List(S)) -> S {\n\tseeds |> ([s]) { S(value = s.value + 2) } | (_) { panic(\"missing seed\") }\n}\nmain = () <- (s: S) { assert(s.value == 42) }",
        "[providers.\"app/main.S\"]\nrole = \"runner\"\nadapter = \"app/main.adapt\"\nnative = {factory=\"app/main.make\", arguments=[\"seed\"], seed_required=true}\n[provider_adapters.\"app/main.adapt\"]\nsupplies = [\"app/main.S\"]\nrequires = []\n",
    );
}
#[test]
fn adapter_cannot_call_body_twice() {
    let outcome = run(
        "S = ()\nadapt = <A, N>(body: () <- N -> A) -> A {\n\t{\n\t\tx = body()\n\t\tbody()\n\t} <- (S)\n}\nmake = () { S }\nmain = () <- (s: S) { Nil }",
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
        "(\n\tS\n\tprobe\n)\nS = (value = Int)\na = () { S(value = 1) }\nb = () { S(value = 2) }\nc = () { S(value = 3) }\nd = () { S(value = 4) }\ne = () { S(value = 5) }\nprobe = () <- (s: S) { s.value }\nmain = () { Nil }\n",
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
    for target in ["js", "nodejs", "native", "wasi", "host", "ilang"] {
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
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["run", "--opt", "0", "--target", "wasi", "--project"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
#[test]
fn missing_provider_is_a_build_error_but_source_check_passes() {
    let dir = project("Missing = ()\nmain = () <- (s: Missing) { Nil }", "");
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
        "@host(\"custom\")\nService = (read = () -> Int)\nmain = () <- (s: Service) { Nil }",
        "",
    );
    let b = build(dir.path(), options(Target::Native)).unwrap_or_else(|e| panic!("{e}"));
    let error = ilang_host::Program::from_bytes(&b.wasm)
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
        "(\n\tS\n\tprobe\n)\nS = ()\n@host(\"custom\", \"request\")\nWork = (start = () -> Request(Int))\nmake = () <- (w: Work) { S }\nprobe = () <- (s: S) { Nil }",
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
        "(\n\ti/key\n)\nD = (key = Key(Nil))\nB = (key = Key(Nil))\nC = (key = Key(Nil))\nd = () { D(key = key.new()) }\nb = () <- (d: D) { B(key = d.key) }\nc = () <- (d: D) { C(key = d.key) }\nmain = () <- (b: B, c: C) { assert(b.key == c.key) }",
        &[row("D", "d"), row("B", "b"), row("C", "c")].concat(),
    );
}
#[test]
fn generated_root_key_is_shared_without_a_request_service() {
    passes(
        "A = (key = Key(Nil))\nB = (key = Key(Nil))\na = (key: Key(Nil)) { A(key = key) }\nb = (key: Key(Nil)) { B(key = key) }\nmain = () <- (a: A, b: B) { assert(a.key == b.key) }",
        "[providers.\"app/main.A\"]\nnative = {factory=\"app/main.a\", arguments=[\"entry-root-key\"]}\n[providers.\"app/main.B\"]\nnative = {factory=\"app/main.b\", arguments=[\"entry-root-key\"]}\n",
    );
}

#[test]
fn nullary_override_replaces_the_adapters_service_value() {
    passes(
        "S = (value = Int)\nretain = (x) { x }\nadapt = <A, N>(body: () <- N -> A) -> A { retain(body()) <- (S(value = 40)) }\nmake = () { S(value = 42) }\nmain = () <- (s: S) { assert(s.value == 42) }",
        "[providers.\"app/main.S\"]\nrole = \"runner\"\nadapter = \"app/main.adapt\"\nnative = \"app/main.make\"\n[provider_adapters.\"app/main.adapt\"]\nsupplies = [\"app/main.S\"]\nrequires = []\n",
    );
}

#[test]
fn target_schema_is_checked_without_an_entry() {
    let dir = project("S = ()\nmake = () { 42 }", &row("S", "make"));
    let error = build(dir.path(), options(Target::Native))
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("must return its service"), "{error}");
}

#[test]
fn generated_discriminant_provisions_constructor_fields() {
    passes(
        "Part = (value = Int)\nOn = (Part)\nOff = ()\nBundleKind = On | Off\nBundle = (value = BundleKind)\npart = () { Part(value = 42) }\nmain = () <- (b: Bundle) {\n\tb.value |> (On(p)) { assert(p.value == 42) } | (Off) { panic(\"wrong target\") }\n}",
        &(row("Part", "part")
            + "[providers.\"app/main.Bundle\"]\ndiscriminant = {js=\"app/main.Off\", nodejs=\"app/main.On\", native=\"app/main.On\", wasi=\"app/main.Off\", host=\"app/main.Off\"}\n"),
    );
}

#[test]
fn factory_request_dependencies_classify_and_suspend_the_entry() {
    let dir = project(
        "(\n\ti/clock\n\ti/fail\n)\nS = (value = Int)\nmake = () {\n\tfail.catch(() { clock.sleep_ns(1000) }) |>\n\t\t(Ok(_)) { S(value = 42) }\n\t\t| (Error(_)) { panic(\"clock failed\") }\n}\nmain = () <- (s: S) { assert(s.value == 42) }",
        &row("S", "make"),
    );
    let b = build(dir.path(), options(Target::Native)).unwrap_or_else(|e| panic!("{e}"));
    let program = b.host_program().unwrap();
    assert!(program.abi().entry.as_ref().unwrap().is_async);
    assert!(matches!(
        program.run_entry(b.host_options()).unwrap(),
        Outcome::Ok(_)
    ));
}

#[test]
fn external_function_factory_uses_its_checked_calling_convention() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let dir = project(
        "Token = Opaque\n@external(\"custom\", \"make\")\nmake: () -> Token\nmain = () <- (token: Token) { Nil }",
        &row("Token", "make"),
    );
    let b = build(dir.path(), options(Target::Native)).unwrap_or_else(|e| panic!("{e}"));
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let mut host = b.host_options();
    host.imports
        .external("custom", "make", move |_| {
            count.fetch_add(1, Ordering::Relaxed);
            Ok(Value::NullExternref)
        })
        .unwrap();
    assert!(matches!(
        b.host_program().unwrap().run_entry(host).unwrap(),
        Outcome::Ok(_)
    ));
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}

#[test]
fn http_client_graph_environment_provisions_net_and_tls() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/http-get");
    let built = build(&path, options(Target::Native)).unwrap_or_else(|error| panic!("{error}"));
    let program = built.host_program().unwrap();
    assert!(program.abi().entry.as_ref().unwrap().is_async);
    for namespace in ["i:net/v1", "i:tls/v1"] {
        assert!(
            program.abi().services.contains_key(namespace),
            "missing {namespace}"
        );
    }
}

#[test]
fn named_entry_and_collected_tests_use_transitive_provider_plans() {
    let dir = project(
        r#"(
	i/clock
	i/fail
	test = std/test
)
S = (value = Int)
make = () {
	fail.catch(() { clock.sleep_ns(1000) }) |>
		(Ok(_)) { S(value = 42) }
		| (Error(_)) { panic("clock failed") }
}
main = () { panic("unselected main") }
worker = () <- (s: S) { assert(s.value == 42) }
test.test("configured", () { worker() })
"#,
        &(row("S", "make")
            + "[entries]\ntool = \"main.worker\"\n[test]\ncollection = \"std/test.tests\"\n"),
    );
    let std = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut manifest = std::fs::read_to_string(dir.path().join("i.toml")).unwrap();
    manifest.push_str(&format!("[dependencies]\nstd = {{path = {:?}}}\n", std));
    std::fs::write(dir.path().join("i.toml"), manifest).unwrap();
    for level in ["0", "2"] {
        let run = std::process::Command::new(env!("CARGO_BIN_EXE_i"))
            .args(["run", "tool", "--offline", "--opt", level, "--project"])
            .arg(dir.path())
            .output()
            .unwrap();
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let tests = std::process::Command::new(env!("CARGO_BIN_EXE_i"))
            .args(["test", "--offline", "--opt", level, "--project"])
            .arg(dir.path())
            .output()
            .unwrap();
        assert!(
            tests.status.success(),
            "{}{}",
            String::from_utf8_lossy(&tests.stdout),
            String::from_utf8_lossy(&tests.stderr)
        );
        assert!(String::from_utf8_lossy(&tests.stdout).contains("1 passed, 0 failed"));
    }
}

#[test]
fn dependency_defaults_own_services_and_root_override_keeps_authority() {
    let dir = project(
        "(\n\tdep = dep/main\n)\nmain = () <- (s: dep.Service) { assert(s.read() == 42) }",
        "[dependencies]\ndep = {path=\"dep\"}\n",
    );
    std::fs::create_dir_all(dir.path().join("dep/src")).unwrap();
    std::fs::write(
        dir.path().join("dep/i.toml"),
        "name=\"dep\"\n[providers.\"dep/main.Service\"]\nnative=\"dep/main.make\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("dep/src/main.i"),
        "(\n\tService(..)\n)\nService = (read = () -> Int)\nmake = () { Service(read = () { 42 }) }",
    )
    .unwrap();
    let b = build(dir.path(), options(Target::Native)).unwrap_or_else(|e| panic!("{e}"));
    assert!(matches!(
        b.host_program()
            .unwrap()
            .run_entry(b.host_options())
            .unwrap(),
        Outcome::Ok(_)
    ));
    std::fs::write(dir.path().join("src/main.i"), "(\n\tdep = dep/main\n)\nfake = () { dep.Service(read = () { 7 }) }\nmain = () <- (s: dep.Service) { assert(s.read() == 7) }").unwrap();
    std::fs::write(dir.path().join("i.toml"), "name=\"app\"\n[dependencies]\ndep={path=\"dep\"}\n[providers.\"dep/main.Service\"]\nnative=\"app/main.fake\"\n").unwrap();
    let b = build(dir.path(), options(Target::Native)).unwrap_or_else(|e| panic!("{e}"));
    assert!(matches!(
        b.host_program()
            .unwrap()
            .run_entry(b.host_options())
            .unwrap(),
        Outcome::Ok(_)
    ));
    std::fs::write(
        dir.path().join("dep/i.toml"),
        "name=\"dep\"\n[providers.\"i/clock.Clock\"]\nnative={host=true}\n",
    )
    .unwrap();
    let error = match build(dir.path(), options(Target::Native)) {
        Ok(_) => panic!("dependency replaced another package's capability"),
        Err(e) => e.to_string(),
    };
    assert!(
        error.contains("must belong to its declaring package"),
        "{error}"
    );
}
