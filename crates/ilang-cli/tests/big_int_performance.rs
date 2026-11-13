//! Opt-in paired Linux measurements. The outer cargo command uses cache/heavy.
//! Process CPU time includes the guest worker thread, excluding descheduling.
#![cfg(all(target_os = "linux", target_pointer_width = "64"))]
use ilang_host::{HostImports, HostOptions, Outcome, Program, Value};
use num_bigint::BigInt;
use std::{path::Path, process::Command, time::Instant};

fn cpu_nanos() -> u64 {
    // Linux CLOCK_PROCESS_CPUTIME_ID includes all threads of this process.
    // Instance::call runs the guest on a worker, so a thread clock is wrong.
    #[repr(C)]
    struct Timespec {
        seconds: std::ffi::c_long,
        nanos: std::ffi::c_long,
    }
    unsafe extern "C" {
        fn clock_gettime(clock: i32, time: *mut Timespec) -> i32;
    }
    let mut time = Timespec {
        seconds: 0,
        nanos: 0,
    };
    // SAFETY: the pointer is writable and correctly laid out for 64-bit Linux;
    // clock_gettime writes this structure synchronously and retains no pointer.
    assert_eq!(unsafe { clock_gettime(2, &mut time) }, 0);
    time.seconds as u64 * 1_000_000_000 + time.nanos as u64
}

#[test]
#[ignore = "benchmark: set BIG_INT_BASE to the lane/jp release i binary"]
fn indexed_big_int_against_wat() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/performance/big-int");
    let temporary = tempfile::tempdir().unwrap();
    let baseline = temporary.path().join("baseline.wasm");
    let status = Command::new(std::env::var("BIG_INT_BASE").expect("BIG_INT_BASE"))
        .args(["build", "--project"])
        .arg(&fixture)
        .args(["--target", "host", "--opt", "2", "--out"])
        .arg(&baseline)
        .status()
        .unwrap();
    assert!(status.success());
    let current = ilang_cli::build(
        &fixture,
        ilang_cli::BuildOptions {
            opt: ilang_cli::Level::O2,
            tests: false,
            offline: true,
            target: ilang_wasm::Target::Native,
        },
    )
    .unwrap();
    if let Ok(directory) = std::env::var("BIG_INT_ARTIFACTS") {
        let directory = Path::new(&directory);
        std::fs::create_dir_all(directory).unwrap();
        std::fs::copy(&baseline, directory.join("wat.wasm")).unwrap();
        std::fs::write(directory.join("i.wasm"), &current.wasm).unwrap();
    }
    let programs = [
        Program::from_bytes(&std::fs::read(baseline).unwrap()).unwrap(),
        Program::from_bytes(&current.wasm).unwrap(),
    ];
    println!(
        "bits,operation,wat_iterations,i_iterations,wat_cpu_ns,i_cpu_ns,ratio,wat_wall_ns,i_wall_ns"
    );
    for bits in [100_usize, 200, 300, 8500] {
        // Near-equal values force compare to read all limbs. A half-width
        // divisor exercises a long quotient, rather than only one digit.
        let a = (BigInt::from(1) << bits) - 129_u16;
        let near = &a - 1_u8;
        let half = (BigInt::from(1) << (bits / 2)) + 123_u8;
        for operation in ["add", "compare", "multiply", "divide", "divide_half"] {
            let b = if operation == "divide_half" {
                &half
            } else {
                &near
            };
            let kernel = if operation == "divide_half" {
                "divide"
            } else {
                operation
            };
            let expected = match operation {
                "add" => Value::Int(&a + b),
                "compare" => Value::Bool(a < *b),
                "multiply" => Value::Int(&a * b),
                _ => Value::Int(&a / b),
            };
            let mut instances: Vec<_> = programs
                .iter()
                .map(|p| {
                    p.instantiate(HostOptions::new(HostImports::default()))
                        .unwrap()
                })
                .collect();
            let mut run = |which: usize, n: i64| {
                let args = [
                    Value::Int(n.into()),
                    Value::Int(a.clone()),
                    Value::Int(b.clone()),
                    if operation == "compare" {
                        Value::Bool(false)
                    } else {
                        Value::Int(0.into())
                    },
                ];
                let start = (Instant::now(), cpu_nanos());
                let got = instances[which]
                    .call(&format!("bigperf/main.{kernel}"), &args)
                    .unwrap();
                let elapsed = [
                    (cpu_nanos() - start.1) as f64 / 1e9,
                    start.0.elapsed().as_secs_f64(),
                ];
                let equal = match (&got, &expected) {
                    (Outcome::Ok(Value::Int(x)), Value::Int(y)) => x == y,
                    (Outcome::Ok(Value::Bool(x)), Value::Bool(y)) => x == y,
                    _ => false,
                };
                assert!(equal, "{bits} {operation}: {got:?}");
                elapsed
            };
            let mut iterations = [1, 1];
            for (which, n) in iterations.iter_mut().enumerate() {
                while run(which, *n)[0] < 0.05 && *n < 1_000_000 {
                    *n *= 2;
                }
            }
            let mut samples = [Vec::new(), Vec::new()];
            let mut walls = [Vec::new(), Vec::new()];
            for sample in 0..5 {
                for which in if sample % 2 == 0 { [0, 1] } else { [1, 0] } {
                    let [cpu, wall] = run(which, iterations[which]);
                    samples[which].push(cpu * 1e9 / iterations[which] as f64);
                    walls[which].push(wall * 1e9 / iterations[which] as f64);
                }
            }
            for sample in samples.iter_mut().chain(&mut walls) {
                sample.sort_by(f64::total_cmp);
            }
            let [wat, current] = samples.map(|s| s[2]);
            let [wat_wall, i_wall] = walls.map(|s| s[2]);
            println!(
                "{bits},{operation},{},{},{wat:.1},{current:.1},{:.2},{wat_wall:.1},{i_wall:.1}",
                iterations[0],
                iterations[1],
                current / wat
            );
        }
    }
}
