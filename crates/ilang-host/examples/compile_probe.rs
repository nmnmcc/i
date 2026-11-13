//! Measurement only: compiles one Wasm module the way `Program::from_bytes`
//! does and prints CPU time and memory at each step. Each toggle is an
//! environment variable, so that one process measures one configuration:
//!
//! - `PROBE_PARALLEL=0` turns parallel compilation off (default on);
//! - `PROBE_OPT=none|speed|speed_and_size` (default `speed`);
//! - `PROBE_ADDRESS_MAP=0`, `PROBE_UNWIND=0`, `PROBE_REGALLOC=single_pass`;
//! - `PROBE_TRIM=1` calls `malloc_trim(0)` after the compile;
//! - `PROBE_SERIALIZE=1` serializes the module and prints its size;
//! - `PROBE_RELOAD=1` serializes, drops the module, and deserializes it.
//!
//! `cargo run --release -p ilang-host --example compile_probe -- <module.wasm>`

use std::time::Instant;
use wasmtime::{Config, Engine, Module, OptLevel};

unsafe extern "C" {
    fn malloc_trim(pad: usize) -> i32;
}

fn status(stage: &str, start: Instant) {
    let text = std::fs::read_to_string("/proc/self/status").unwrap();
    let field = |k: &str| {
        text.lines()
            .find(|l| l.starts_with(k))
            .map(|l| l[k.len()..].trim().to_string())
            .unwrap()
    };
    let stat = std::fs::read_to_string("/proc/self/stat").unwrap();
    let f: Vec<&str> = stat
        .rsplit(')')
        .next()
        .unwrap()
        .split_whitespace()
        .collect();
    let ticks = |i: usize| f[i].parse::<f64>().unwrap() / 100.0;
    println!(
        "{stage:>12}: wall {:7.2}s cpu {:7.2}s rss {:>12} peak {:>12} threads {}",
        start.elapsed().as_secs_f64(),
        ticks(11) + ticks(12),
        field("VmRSS:"),
        field("VmHWM:"),
        field("Threads:"),
    );
}

fn on(name: &str, default: bool) -> bool {
    match std::env::var(name).as_deref() {
        Ok("0") => false,
        Ok("1") => true,
        _ => default,
    }
}

fn main() -> wasmtime::Result<()> {
    let path = std::env::args().nth(1).expect("a module path");
    let bytes = std::fs::read(path)?;
    let start = Instant::now();
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_function_references(true);
    config.wasm_tail_call(true);
    config.wasm_exceptions(true);
    config.max_wasm_stack(ilang_host::WASM_STACK);
    config.async_stack_size(ilang_host::THREAD_STACK);
    config.collector(wasmtime::Collector::Copying);
    config.gc_heap_initial_size(ilang_host::GC_HEAP);
    config.parallel_compilation(on("PROBE_PARALLEL", true));
    config.cranelift_opt_level(match std::env::var("PROBE_OPT").as_deref() {
        Ok("none") => OptLevel::None,
        Ok("speed_and_size") => OptLevel::SpeedAndSize,
        _ => OptLevel::Speed,
    });
    config.generate_address_map(on("PROBE_ADDRESS_MAP", true));
    config.native_unwind_info(on("PROBE_UNWIND", true));
    if std::env::var("PROBE_REGALLOC").as_deref() == Ok("single_pass") {
        config.cranelift_regalloc_algorithm(wasmtime::RegallocAlgorithm::SinglePass);
    }
    let engine = Engine::new(&config)?;
    status("engine", start);
    let mut module = Module::new(&engine, &bytes)?;
    status("compiled", start);
    if on("PROBE_TRIM", false) {
        unsafe { malloc_trim(0) };
        status("trimmed", start);
    }
    if on("PROBE_SERIALIZE", false) || on("PROBE_RELOAD", false) {
        let image = module.serialize()?;
        println!("serialized: {} bytes", image.len());
        status("serialized", start);
        if on("PROBE_RELOAD", false) {
            drop(module);
            unsafe { malloc_trim(0) };
            status("dropped", start);
            module = unsafe { Module::deserialize(&engine, &image)? };
            drop(image);
            unsafe { malloc_trim(0) };
            status("reloaded", start);
        }
    }
    println!("functions: {}", module.exports().count());
    status("end", start);
    Ok(())
}
