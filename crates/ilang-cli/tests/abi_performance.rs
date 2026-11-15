//! Opt-in boundary and internal small-Int measurements; run through cache/heavy.
use ilang_host::{BitArray, HostImports, Outcome, Value};
use num_bigint::BigInt;
use std::{path::Path, time::Instant};

fn program(opt: ilang_cli::Level) -> ilang_cli::Built {
    let built = ilang_cli::build(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/performance/abi"),
        ilang_cli::BuildOptions {
            opt,
            tests: false,
            offline: true,
            target: ilang_wasm::Target::Native,
        },
    )
    .unwrap();
    if let Ok(path) = std::env::var("ABI_PERF_ARTIFACT") {
        std::fs::write(path, &built.wasm).unwrap();
    }
    let artifacts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/abi-performance");
    std::fs::create_dir_all(&artifacts).unwrap();
    let version = ilang_host::Abi::from_wasm(&built.wasm)
        .unwrap()
        .unwrap()
        .version;
    std::fs::write(
        artifacts.join(format!("v{version}-{opt:?}.wasm")),
        &built.wasm,
    )
    .unwrap();
    std::fs::write(
        artifacts.join(format!("v{version}-{opt:?}.bindings.json")),
        serde_json::to_vec(&built.binding_plan).unwrap(),
    )
    .unwrap();
    built
}

fn imports(increment: bool) -> HostImports {
    let mut imports = HostImports::default();
    imports
        .external("abi:bench", "tick", move |args| {
            let [Value::Int(n)] = args else {
                panic!("Int expected")
            };
            Ok(Value::Int(if increment { n + 1 } else { n.clone() }))
        })
        .unwrap();
    // A deterministic fs.read-shaped call: length in, fixed 8-byte payload out.
    // Excludes filesystem latency to isolate the boundary cost of the length.
    imports
        .external("abi:bench", "read", |args| {
            assert!(matches!(args, [Value::Int(n)] if *n == BigInt::from(8)));
            Ok(Value::BitArray(BitArray::new(vec![0; 8], 64).unwrap()))
        })
        .unwrap();
    imports
}

#[test]
fn integer_boundary_values_cross_compiled_imports_and_exports() {
    for opt in [ilang_cli::Level::O0, ilang_cli::Level::O2] {
        let built = program(opt);
        let mut options = built.host_options();
        options.imports.merge(imports(false)).unwrap();
        let mut instance = built.host_program().unwrap().instantiate(options).unwrap();
        let big: BigInt = BigInt::from(1) << 4096;
        for n in [
            0.into(),
            1.into(),
            (-1).into(),
            i32::MIN.into(),
            i32::MAX.into(),
            (i64::from(i32::MIN) - 1).into(),
            (i64::from(i32::MAX) + 1).into(),
            big.clone(),
            -big,
        ] {
            let result = instance
                .call("abiperf/main.echo", &[Value::Int(n.clone())])
                .unwrap();
            assert!(matches!(result, Outcome::Ok(Value::Int(actual)) if actual == n));
        }
    }
}

#[test]
#[ignore = "benchmark: run with --release --ignored --nocapture through cache/heavy"]
fn small_int_calls_and_internal_arithmetic() {
    if ilang_host::ABI_VERSION == 4 {
        let error = ilang_host::Abi::from_json(
            br#"{"version":5,"entry":null,"tests":[],"exports":{},"services":{},"externals":{}}"#,
        )
        .unwrap_err();
        assert_eq!(
            format!("{error:#}"),
            "the module uses i ABI version 5, but this host reads version 4; update the host"
        );
        println!("ABI_PERF old host rejection: {error:#}");
    }
    let built = program(ilang_cli::Level::O2);
    let p = built.host_program().unwrap();
    let mut options = built.host_options();
    options.imports.merge(imports(true)).unwrap();
    let mut instance = p.instantiate(options).unwrap();
    let cases = [
        (
            "counter",
            vec![Value::Int(1_000_000.into()), Value::Int(0.into())],
        ),
        (
            "fs_reads",
            vec![Value::Int(10_000.into()), Value::String("/dev/zero".into())],
        ),
        (
            "read_lengths",
            vec![Value::Int(1_000_000.into()), Value::Int(8.into())],
        ),
        (
            "add",
            vec![
                Value::Int(1_000_000.into()),
                Value::Int(123.into()),
                Value::Int(456.into()),
                Value::Int(0.into()),
            ],
        ),
        (
            "compare",
            vec![
                Value::Int(1_000_000.into()),
                Value::Int(123.into()),
                Value::Int(456.into()),
                Value::Bool(false),
            ],
        ),
        (
            "empty",
            vec![
                Value::Int(1_000_000.into()),
                Value::Int(123.into()),
                Value::Int(456.into()),
                Value::Int(0.into()),
            ],
        ),
    ];
    for (name, args) in cases {
        let mut samples = Vec::new();
        for sample in 0..6 {
            let start = Instant::now();
            let result = if name == "fs_reads" {
                ilang_host::block_on(instance.call_async(&format!("abiperf/main.{name}"), &args))
                    .unwrap()
            } else {
                instance
                    .call(&format!("abiperf/main.{name}"), &args)
                    .unwrap()
            };
            let elapsed = start.elapsed().as_secs_f64();
            match name {
                "counter" => assert!(
                    matches!(result, Outcome::Ok(Value::Int(n)) if n == BigInt::from(1_000_000))
                ),
                "fs_reads" => {
                    assert!(matches!(result, Outcome::Ok(Value::Int(n)) if n == BigInt::from(0)))
                }
                "read_lengths" => {
                    assert!(matches!(result, Outcome::Ok(Value::Int(n)) if n == BigInt::from(8)))
                }
                "add" => assert!(
                    matches!(result, Outcome::Ok(Value::Int(n)) if n == BigInt::from(456_000_000))
                ),
                "empty" => {
                    assert!(matches!(result, Outcome::Ok(Value::Int(n)) if n == BigInt::from(123)))
                }
                _ => assert!(matches!(result, Outcome::Ok(Value::Bool(true)))),
            }
            if sample != 0 {
                samples.push(elapsed);
            }
        }
        samples.sort_by(f64::total_cmp);
        let iterations = if name == "fs_reads" {
            10_000
        } else {
            1_000_000
        };
        println!(
            "ABI_PERF version={} {name}: median {:.6} s / {iterations} iterations; samples {samples:?}",
            p.abi().version,
            samples[2]
        );
    }
}
