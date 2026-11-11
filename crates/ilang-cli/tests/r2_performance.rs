//! Reproducible O2 primitive measurements; run under cache/quiet, never heavy
//! inside quiet. Instrumented modules are counted separately and never timed.
use ilang_cli::{BuildOptions, Level, build};
use ilang_host::{HostOptions, Outcome, Program, Value, block_on};
use std::{fmt::Write, path::Path, time::Instant};

#[test]
fn paired_inside_scope_measurements() {
    let Ok(directory) = std::env::var("R2_PERF_INTERVAL_DUMP") else {
        return;
    };
    let directory = Path::new(&directory);
    std::fs::create_dir_all(directory).unwrap();
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("src")).unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/r2-performance");
    let source = std::fs::read_to_string(fixture.join("src/kernels.i")).unwrap();
    let marker = "\tempty\n";
    assert_eq!(source.matches(marker).count(), 1);
    let source = source.replacen(
        marker,
        "\tempty\n\ttimed_empty\n\ttimed_spawn_join\n\ttimed_switch_pair\n\ttimed_channel_pair\n",
        1,
    ) + &std::fs::read_to_string(fixture.join("intervals.i")).unwrap();
    std::fs::write(project.path().join("src/kernels.i"), source).unwrap();
    let names = [
        "timed_empty",
        "timed_spawn_join",
        "timed_switch_pair",
        "timed_channel_pair",
    ];
    let exports: Vec<_> = names
        .iter()
        .map(|name| format!("r2perf/kernels.{name}"))
        .collect();
    std::fs::write(
        project.path().join("i.toml"),
        format!("name = \"r2perf\"\n[wasm]\nexports = {exports:?}\n"),
    )
    .unwrap();
    let built = build(
        project.path(),
        BuildOptions {
            opt: Level::O2,
            tests: false,
            offline: false,
            target: Default::default(),
        },
    )
    .unwrap();
    std::fs::write(directory.join("intervals.wasm"), &built.wasm).unwrap();
    let program = Program::from_bytes(&built.wasm).unwrap();
    let origin = Instant::now();
    let mut imports = ilang_host::HostImports::default();
    imports
        .external("r2:meter/v1", "now", move |_| {
            Ok(Value::Float(origin.elapsed().as_secs_f64() * 1e9))
        })
        .unwrap();
    block_on(async {
        let mut instance = program.instantiate(HostOptions::new(imports)).unwrap();
        let mut csv = String::from(
            "kernel,batch,sample,n,inside_ns_per_operation,outside_ns_per_operation\n",
        );
        for n in [32_i64, 64, 128] {
            for (kernel, loops, operations) in [
                ("spawn_join", 1, 1),
                ("switch_pair", 2, 2),
                ("channel_pair", 2, 1),
            ] {
                let name = format!("r2perf/kernels.timed_{kernel}");
                for _ in 0..200 {
                    instance
                        .call_async(&name, &[Value::Int(n.into())])
                        .await
                        .unwrap()
                        .into_value()
                        .unwrap();
                }
                for batch in 0..5 {
                    for sample in 0..21 {
                        let mut inside = [0.0; 4];
                        let mut outside = [0.0; 4];
                        let calls = [
                            (&name[..], n),
                            (&name[..], 0),
                            ("r2perf/kernels.timed_empty", n),
                            ("r2perf/kernels.timed_empty", 0),
                        ];
                        let order = if sample % 2 == 0 {
                            [0, 1, 2, 3]
                        } else {
                            [3, 2, 1, 0]
                        };
                        for index in order {
                            let (function, argument) = calls[index];
                            let start = Instant::now();
                            let value = instance
                                .call_async(function, &[Value::Int(argument.into())])
                                .await
                                .unwrap()
                                .into_value()
                                .unwrap();
                            outside[index] = start.elapsed().as_nanos() as f64;
                            let Value::Float(ns) = value else {
                                panic!("interval returns Float")
                            };
                            inside[index] = ns;
                        }
                        let net = |times: [f64; 4]| {
                            (times[0] - times[1] - loops as f64 * (times[2] - times[3]))
                                / (operations * n) as f64
                        };
                        writeln!(
                            csv,
                            "{kernel},{batch},{sample},{n},{},{}",
                            net(inside),
                            net(outside)
                        )
                        .unwrap();
                    }
                }
            }
        }
        std::fs::write(directory.join("timings.csv"), csv).unwrap();
    });
}

#[test]
fn paired_primitive_measurements() {
    let Ok(directory) = std::env::var("R2_PERF_DUMP") else {
        return;
    };
    let directory = Path::new(&directory);
    std::fs::create_dir_all(directory).unwrap();
    let built = build(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/r2-performance")
            .as_path(),
        BuildOptions {
            opt: Level::O2,
            tests: false,
            offline: false,
            target: Default::default(),
        },
    )
    .unwrap();
    std::fs::write(directory.join("kernels.wasm"), &built.wasm).unwrap();
    // Keep matching pre-optimization names for attribution. Counts still come
    // from the final O2 bytes above, after comparing all function bodies.
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/r2-performance");
    let (graph, _) = ilang_cli::load(&fixture, false, false);
    let checked = ilang_check::check(&graph);
    let p = checked.program.as_ref().unwrap();
    let t = checked.types.as_ref().unwrap();
    let options = ilang_wasm::Options::default();
    std::fs::write(
        directory.join("kernels-o0.wasm"),
        ilang_wasm::compile(&graph, p, t, options).unwrap(),
    )
    .unwrap();
    std::fs::write(
        directory.join("source.wat"),
        ilang_wasm::compile_wat(&graph, p, t, options).unwrap(),
    )
    .unwrap();
    let program = Program::from_bytes(&built.wasm).unwrap();
    block_on(async {
        let mut instance = program.instantiate(HostOptions::default()).unwrap();
        let mut csv = String::from(
            "kernel,batch,sample,n,target_ns,zero_ns,empty_ns,empty_zero_ns,net_ns_per_operation\n",
        );
        for n in [32_i64, 64, 128] {
            for (kernel, loops, operations) in [
                ("spawn_join", 1, 1),
                ("switch_pair", 2, 2),
                ("channel_pair", 2, 1),
            ] {
                let name = format!("r2perf/kernels.{kernel}");
                for _ in 0..200 {
                    let result = instance
                        .call_async(&name, &[Value::Int(n.into())])
                        .await
                        .unwrap();
                    assert!(
                        matches!(result, Outcome::Ok(Value::Int(ref value)) if *value == n.into())
                    );
                }
                for batch in 0..5 {
                    for sample in 0..21 {
                        let mut times = [0_i128; 4];
                        let calls = [
                            (&name[..], n),
                            (&name[..], 0),
                            ("r2perf/kernels.empty", n),
                            ("r2perf/kernels.empty", 0),
                        ];
                        let order = if sample % 2 == 0 {
                            [0, 1, 2, 3]
                        } else {
                            [3, 2, 1, 0]
                        };
                        for index in order {
                            let (function, argument) = calls[index];
                            let start = Instant::now();
                            let result = instance
                                .call_async(function, &[Value::Int(argument.into())])
                                .await
                                .unwrap();
                            times[index] = start.elapsed().as_nanos() as i128;
                            assert!(
                                matches!(result, Outcome::Ok(Value::Int(ref value)) if *value == argument.into())
                            );
                        }
                        let net = (times[0] - times[1] - loops * (times[2] - times[3])) as f64
                            / (operations * n) as f64;
                        writeln!(
                            csv,
                            "{kernel},{batch},{sample},{n},{},{},{},{},{net}",
                            times[0], times[1], times[2], times[3]
                        )
                        .unwrap();
                    }
                }
            }
        }
        std::fs::write(directory.join("timings.csv"), csv).unwrap();
    });
}

#[test]
fn count_instrumented_kernels() {
    use wasmtime::{Config, Engine, Linker, Module, Store, Val};
    let Ok(path) = std::env::var("R2_PROFILE_WASM") else {
        return;
    };
    let mut config = Config::new();
    config
        .wasm_gc(true)
        .wasm_function_references(true)
        .wasm_tail_call(true)
        .wasm_exceptions(true);
    config
        .collector(wasmtime::Collector::Copying)
        .gc_heap_initial_size(ilang_host::GC_HEAP);
    config.max_wasm_stack(16 << 20).async_stack_size(32 << 20);
    let engine = Engine::new(&config).unwrap();
    let module = Module::from_binary(&engine, &std::fs::read(&path).unwrap()).unwrap();
    let mut linker = Linker::<()>::new(&engine);
    linker
        .func_wrap(
            "ilang:host/v1",
            "panic",
            |_: i32, _: i32| -> wasmtime::Result<()> {
                Err(wasmtime::Error::msg("profile guest stopped"))
            },
        )
        .unwrap();
    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &module).unwrap();
    instance
        .get_typed_func::<(), ()>(&mut store, "ilang_start")
        .unwrap()
        .call(&mut store, ())
        .unwrap();
    let counters = std::fs::read_to_string(format!("{path}.tsv")).unwrap();
    let counters: Vec<_> = counters
        .lines()
        .map(|line| {
            let (name, rest) = line.split_once('\t').unwrap();
            (instance.get_global(&mut store, name).unwrap(), rest)
        })
        .collect();
    let alloc = instance
        .get_typed_func::<i32, i32>(&mut store, "ilang_alloc")
        .unwrap();
    let free = instance
        .get_typed_func::<(i32, i32), ()>(&mut store, "ilang_free")
        .unwrap();
    let memory = instance.get_memory(&mut store, "memory").unwrap();
    let mut output = String::from("kernel\tn\tfunction\tkind\tdetail\tcount\n");
    for kernel in ["empty", "spawn_join", "switch_pair", "channel_pair"] {
        let name = format!("r2perf/kernels.{kernel}");
        let function = instance
            .get_typed_func::<(i32, i32, i32), i32>(&mut store, &name)
            .unwrap();
        let result = instance
            .get_typed_func::<(), (i32, i32, i32)>(&mut store, &format!("ilang_result:{name}"))
            .unwrap();
        for n in [0_u32, 32, 128] {
            let ptr = alloc.call(&mut store, 4).unwrap();
            memory
                .write(&mut store, ptr as usize, &n.to_le_bytes())
                .unwrap();
            for (global, _) in &counters {
                global.set(&mut store, Val::I64(0)).unwrap();
            }
            let status = function
                .call(&mut store, (i32::from(n != 0), ptr, i32::from(n != 0)))
                .unwrap();
            assert_eq!(status, 0, "pure kernel must not return wait/poll status");
            let (sign, answer, limbs) = result.call(&mut store, ()).unwrap();
            assert_eq!(sign, i32::from(n != 0));
            assert_eq!(limbs, i32::from(n != 0));
            if n != 0 {
                let mut bytes = [0_u8; 4];
                memory.read(&store, answer as usize, &mut bytes).unwrap();
                assert_eq!(u32::from_le_bytes(bytes), n);
            }
            for (global, description) in &counters {
                let count = global.get(&mut store).unwrap_i64();
                if count != 0 {
                    writeln!(output, "{kernel}\t{n}\t{description}\t{count}").unwrap();
                }
            }
            free.call(&mut store, (ptr, 4)).unwrap();
            if limbs != 0 {
                free.call(&mut store, (answer, limbs * 4)).unwrap();
            }
        }
    }
    std::fs::write(format!("{path}.counts.tsv"), output).unwrap();
}
