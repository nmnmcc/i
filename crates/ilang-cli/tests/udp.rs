//! Datagram behavior and cancellation use the same i cases on both providers.
use ilang_cli::{BuildOptions, Level, Target};
use ilang_host::Outcome;
use std::{fs, path::Path, process::Command};

#[test]
fn udp_datagrams_and_cancellation_on_rust_and_node() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = root.join("examples/io-udp");
    let output = tempfile::tempdir().unwrap();
    for opt in [Level::O0, Level::O2] {
        let (built, entries) = ilang_cli::build_tests(
            &project,
            BuildOptions {
                tests: true,
                opt,
                offline: true,
                target: Target::Native,
            },
            Some("std/test.tests"),
        )
        .unwrap();
        assert_eq!(entries.len(), 5);
        let program = built.host_program().unwrap();
        for entry in entries {
            let export = format!("i.tool.{}", entry.handle.rsplit(':').next().unwrap());
            let outcome = ilang_host::block_on(async {
                program
                    .instantiate(built.host_options())?
                    .run_async(&export)
                    .await
            })
            .unwrap();
            assert!(matches!(outcome, Outcome::Ok(_)), "{entry:?}: {outcome:?}");
        }
        let wasm = output.path().join("udp.wasm");
        fs::write(&wasm, &built.wasm).unwrap();
        let node = Command::new("node")
            .arg("--test")
            .arg(root.join("bindings/javascript/test/udp.test.mjs"))
            .env("I_UDP_TEST_WASM", &wasm)
            .output()
            .unwrap();
        assert!(
            node.status.success(),
            "Node UDP: {}\n{}",
            String::from_utf8_lossy(&node.stdout),
            String::from_utf8_lossy(&node.stderr)
        );
    }
}

#[test]
fn udp_is_unavailable_on_wasi_and_browser() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/io-udp");
    for target in [Target::Wasi, Target::Js] {
        let error = ilang_cli::build(
            &project,
            BuildOptions {
                tests: false,
                opt: Level::O0,
                offline: true,
                target,
            },
        )
        .err()
        .expect("UDP requires a provider")
        .to_string();
        assert!(error.contains("i/net.Net"), "{target:?}: {error}");
    }
}
