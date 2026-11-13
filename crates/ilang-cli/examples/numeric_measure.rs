//! cargo run -p ilang-cli --example numeric_measure -- <output-directory>
//! Alternate reference/N0/N1 at O2, outside compilation and instrumentation.
use ilang_wasm::{Numeric, Options};
use std::{fmt::Write, path::PathBuf, process::Command, time::Instant};
use wasmtime::{Config, Engine, Linker, Module, Store, Val};

fn median(xs: &mut [i128]) -> i128 {
    xs.sort_unstable();
    xs[xs.len() / 2]
}

fn main() -> anyhow::Result<()> {
    let output = PathBuf::from(std::env::args().nth(1).expect("output directory"));
    std::fs::create_dir_all(&output)?;
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/perf/numeric");
    let (graph, diags) = ilang_cli::load(&project, false, true);
    assert!(
        !diags.iter().any(ilang_package::Diagnostic::is_error),
        "{diags:?}"
    );
    let checked = ilang_check::check(&graph);
    assert!(
        !checked
            .diags
            .iter()
            .any(ilang_package::Diagnostic::is_error),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    let mut config = Config::new();
    config
        .wasm_gc(true)
        .wasm_function_references(true)
        .wasm_tail_call(true)
        .wasm_exceptions(true);
    config.cranelift_opt_level(wasmtime::OptLevel::Speed);
    config.collector(wasmtime::Collector::Copying);
    config.gc_heap_initial_size(256 << 20);
    let engine = Engine::new(&config)?;
    // The instrumented module also runs on a fuel-metering engine: Wasmtime
    // charges one unit per executed Wasm operator, a count that does not
    // depend on machine load (perf `instructions:u` is not available here).
    config.consume_fuel(true);
    let fuel_engine = Engine::new(&config)?;
    let only = std::env::var("NUMERIC_KERNELS").ok();
    let mut variants = Vec::new();
    for (name, numeric) in [
        ("baseline", Numeric::Reference),
        ("n0", Numeric::Visible),
        ("n1", Numeric::Scalars),
    ] {
        let (bytes, _) = ilang_wasm::compile_with_bindings_numeric(
            &graph,
            checked.program.as_ref().unwrap(),
            checked.types.as_ref().unwrap(),
            Options::default().numeric(numeric),
        )?;
        let bytes = if name == "baseline" {
            match std::env::var_os("NUMERIC_BASELINE") {
                Some(path) => std::fs::read(path)?,
                None => ilang_cli::optimize(&bytes, ilang_cli::Level::O2)?,
            }
        } else if let (true, Some(path)) = (name == "n1", std::env::var_os("NUMERIC_CANDIDATE")) {
            // Compare two saved modules, e.g. the Wasm of two earlier stages.
            std::fs::read(path)?
        } else {
            ilang_cli::optimize(&bytes, ilang_cli::Level::O2)?
        };
        let path = output.join(format!("{name}.wasm"));
        std::fs::write(&path, &bytes)?;
        let printed = Command::new("wasm-tools")
            .args(["print"])
            .arg(&path)
            .output()?;
        assert!(printed.status.success(), "wasm-tools print failed");
        let text = String::from_utf8(printed.stdout)?;
        std::fs::write(output.join(format!("{name}.wat")), &text)?;
        // Add instrumentation after optimization, with new functions/globals at
        // the end so all numeric indices in the original module stay valid.
        let mut instrumented = String::new();
        let mut allocation_sites = Vec::new();
        let mut function = String::new();
        for (line_number, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("(func ") {
                function = line.trim().to_owned();
            }
            if line.trim_start().starts_with("struct.new")
                || line.trim_start().starts_with("array.new")
            {
                instrumented.push_str("    (global.set $numeric_allocations (i64.add (global.get $numeric_allocations) (i64.const 1)))\n");
                let site = allocation_sites.len();
                writeln!(
                    instrumented,
                    "(global.set $numeric_site{site} (i64.add (global.get $numeric_site{site}) (i64.const 1)))"
                )?;
                allocation_sites.push((line_number + 1, function.clone(), line.trim().to_owned()));
            }
            writeln!(instrumented, "{line}")?;
        }
        let end = instrumented.rfind(')').unwrap();
        let mut counters = String::from(
            "(global $numeric_allocations (mut i64) (i64.const 0))\n(func (export \"numeric_count\") (result i64) (global.get $numeric_allocations))\n",
        );
        for site in 0..allocation_sites.len() {
            writeln!(
                counters,
                "(global $numeric_site{site} (mut i64) (i64.const 0))\n(func (export \"numeric_site{site}\") (result i64) (global.get $numeric_site{site}))"
            )?;
        }
        counters.push_str(
            "(func (export \"numeric_reset\") (global.set $numeric_allocations (i64.const 0))\n",
        );
        for site in 0..allocation_sites.len() {
            writeln!(counters, "(global.set $numeric_site{site} (i64.const 0))")?;
        }
        counters.push_str(")\n");
        instrumented.insert_str(end, &counters);
        let ipath = output.join(format!("{name}-count.wat"));
        std::fs::write(&ipath, instrumented)?;
        let counted = output.join(format!("{name}-count.wasm"));
        assert!(
            Command::new("wasm-tools")
                .arg("parse")
                .arg(&ipath)
                .arg("-o")
                .arg(&counted)
                .status()?
                .success()
        );
        let mut runs = Vec::new();
        for (counting, module) in [
            Module::new(&engine, &bytes)?,
            Module::from_file(&fuel_engine, &counted)?,
        ]
        .into_iter()
        .enumerate()
        {
            let mut linker = Linker::<()>::new(module.engine());
            for import in module.imports() {
                let ty = import.ty().func().expect("function import").clone();
                linker.func_new(import.module(), import.name(), ty, |_, _, _| {
                    wasmtime::bail!("unexpected benchmark host call")
                })?;
            }
            let mut store = Store::new(module.engine(), ());
            if counting == 1 {
                store.set_fuel(u64::MAX / 2)?;
            }
            let instance = linker.instantiate(&mut store, &module)?;
            instance
                .get_typed_func::<(), ()>(&mut store, "ilang_start")?
                .call(&mut store, ())?;
            runs.push((store, instance));
        }
        variants.push((name, bytes.len(), runs, allocation_sites));
    }
    let mut site_observations = Vec::new();
    let mut csv =
        String::from("variant,kernel,batch,sample,net_ns,allocations,result_bits,module_bytes\n");
    let mut summary =
        String::from("variant,kernel,batch,median_ns,allocations,fuel,module_bytes\n");
    for (kernel, expected) in [
        ("identity", 0.5_f64),
        ("fast", 0.479425538604203_f64),
        ("horner", 1.7509765625_f64),
        ("integer", 8256.0),
        ("dot", 13.25),
        ("tuples", 2.875),
        ("closures", 9.0),
    ] {
        if only
            .as_ref()
            .is_some_and(|only| !only.split(',').any(|k| k == kernel))
        {
            continue;
        }
        for batch in 0..5 {
            let order = if batch % 2 == 0 { [0, 1, 2] } else { [2, 1, 0] };
            for index in order {
                let (variant, size, runs, allocation_sites) = &mut variants[index];
                let name = format!("ilang:numeric/kernels.{kernel}");
                // Discover the actual export spelling through the ABI.
                let abi = ilang_host::Abi::from_wasm(&std::fs::read(
                    output.join(format!("{variant}.wasm")),
                )?)?
                .unwrap();
                let export = abi
                    .exports
                    .keys()
                    .find(|n| n.ends_with(&format!(".{kernel}")))
                    .expect(&name)
                    .clone();
                let identity = abi
                    .exports
                    .keys()
                    .find(|n| n.ends_with(".identity"))
                    .unwrap()
                    .clone();
                let (store, instance) = &mut runs[1];
                instance
                    .get_typed_func::<(), ()>(&mut *store, "numeric_reset")?
                    .call(&mut *store, ())?;
                let function = instance.get_func(&mut *store, &export).unwrap();
                let mut result = function
                    .ty(&*store)
                    .results()
                    .map(|ty| Val::default_for_ty(&ty).unwrap())
                    .collect::<Vec<_>>();
                let fuel_before = store.get_fuel()?;
                function.call(&mut *store, &[Val::F64(0.5_f64.to_bits())], &mut result)?;
                let fuel = fuel_before - store.get_fuel()?;
                assert!(
                    result
                        .iter()
                        .any(|v| matches!(v, Val::F64(bits) if *bits == expected.to_bits())),
                    "{variant}/{kernel}: {result:?}"
                );
                let count = instance
                    .get_typed_func::<(), i64>(&mut *store, "numeric_count")?
                    .call(&mut *store, ())?;
                if batch == 0 {
                    let mut sum = 0;
                    for (site, (line, function, instruction)) in allocation_sites.iter().enumerate()
                    {
                        let executed = instance
                            .get_typed_func::<(), i64>(&mut *store, &format!("numeric_site{site}"))?
                            .call(&mut *store, ())?;
                        sum += executed;
                        if executed != 0 {
                            site_observations.push(serde_json::json!({ "variant": variant, "kernel": kernel, "site": site, "line": line, "function": function, "instruction": instruction, "count": executed }));
                        }
                    }
                    assert_eq!(sum, count, "per-site allocation counts sum to the total");
                }
                let (store, instance) = &mut runs[0];
                let function = instance.get_func(&mut *store, &export).unwrap();
                let identity = instance.get_func(&mut *store, &identity).unwrap();
                for _ in 0..200 {
                    function.call(&mut *store, &[Val::F64(0.5_f64.to_bits())], &mut result)?;
                }
                let mut samples = Vec::new();
                for sample in 0..1001 {
                    let mut times = [0; 2];
                    let calls = if sample % 2 == 0 { [0, 1] } else { [1, 0] };
                    for call in calls {
                        let start = Instant::now();
                        [function, identity][call].call(
                            &mut *store,
                            &[Val::F64(0.5_f64.to_bits())],
                            &mut result,
                        )?;
                        times[call] = start.elapsed().as_nanos() as i128;
                    }
                    let net = times[0] - times[1];
                    samples.push(net);
                    writeln!(
                        csv,
                        "{variant},{kernel},{batch},{sample},{net},{count},{},{size}",
                        expected.to_bits()
                    )?;
                }
                let med = median(&mut samples);
                writeln!(
                    summary,
                    "{variant},{kernel},{batch},{med},{count},{fuel},{size}"
                )?;
                println!(
                    "{variant} {kernel} batch {batch}: {med} ns, {count} allocations, {fuel} fuel, {size} bytes"
                );
            }
        }
    }
    std::fs::write(
        output.join("allocation-sites.json"),
        serde_json::to_vec_pretty(&site_observations)?,
    )?;
    std::fs::write(output.join("samples.csv"), csv)?;
    std::fs::write(output.join("medians.csv"), summary)?;
    Ok(())
}
