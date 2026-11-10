//! The structured i scheduler on the real Rust host. Completion gates and
//! invocation teardown are tested separately from spec output expectations.

use ilang_cli::{BuildOptions, Level, build};
use ilang_host::{HostOptions, Outcome, Program, services};

fn program(source: &str, opt: Level) -> Program {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(directory.path().join("src")).unwrap();
    std::fs::write(directory.path().join("src/main.i"), source).unwrap();
    let built = build(
        directory.path(),
        BuildOptions {
            opt,
            tests: false,
            offline: false,
            target: Default::default(),
        },
    )
    .unwrap();
    Program::from_bytes(&built.wasm).unwrap()
}

#[test]
fn both_async_children_complete_on_the_rust_host() {
    let source = include_str!("../../../tests/spec/9.9/async_tasks.i");
    for opt in [Level::O0, Level::O2] {
        let program = program(source, opt);
        let log = services::Log::default();
        let outcome = program
            .run_entry(HostOptions::new(services::testing(log.clone())))
            .unwrap();
        assert!(matches!(outcome, Outcome::Ok(_)), "{outcome:?}");
        assert_eq!(*log.lock().unwrap(), ["double 21", "echo answer"]);
    }
}
