//! Compile a checked project directly, useful when developing the backend.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let root = args.next().unwrap_or_else(|| ".".into());
    let output = args.next().unwrap_or_else(|| "app.wasm".into());
    let tests = args.any(|argument| argument == "--tests");
    let checked = wleam_frontend::check_project(
        root,
        &wleam_frontend::CheckOptions {
            include_tests: tests,
            ..Default::default()
        },
    )?;
    let mut options = wleam_wasm::Options {
        entry: format!("{}.main", checked.package_name),
        library: tests,
        ..Default::default()
    };
    if tests {
        for module in &checked.modules {
            for function in &module.definitions.functions {
                let Some((_, name)) = &function.name else {
                    continue;
                };
                if function.publicity.is_public()
                    && name.ends_with("_test")
                    && function.arguments.is_empty()
                    && function.return_type.is_nil()
                {
                    options.tests.push(format!("{}.{}", module.name, name));
                }
            }
        }
    }
    let mut roots = options.tests.clone();
    if !options.library {
        roots.push(options.entry.clone());
    }
    let program = wleam_wasm::lower::lower(&checked.modules, &roots)?;
    let artifact = wleam_wasm::binary::compile(&program, &options)?;
    std::fs::write(&output, &artifact.wasm)?;
    std::fs::write(
        format!("{output}.json"),
        serde_json::to_vec_pretty(&artifact.abi)?,
    )?;
    eprintln!(
        "Compiled {} modules, {} tests, {} bytes",
        checked.modules.len(),
        artifact.abi.tests.len(),
        artifact.wasm.len()
    );
    Ok(())
}
