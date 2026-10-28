//! The runtime lives in this process so the compiler and language server do not
//! load or link a WebAssembly JIT.

use std::{
    collections::HashSet,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
    time::Instant,
};

use anyhow::{Context, Result, bail};
use clap::Parser;
use serde::Serialize;
use wleam_host::{Abi, HostOptions, Program};

#[derive(Debug, Parser)]
#[command(version, about = "Run a Wleam Wasm module using Wasmtime")]
struct Arguments {
    /// Compiled WebAssembly module.
    module: PathBuf,

    /// Invoke this zero-argument export instead of the module's entry point.
    #[arg(long, conflicts_with_all = ["test", "all_tests", "list_tests"])]
    invoke: Option<String>,

    /// Execute this zero-argument test export in a fresh instance. Repeatable.
    #[arg(long, conflicts_with = "list_tests")]
    test: Vec<String>,

    /// Run every test listed in the module's wleam.abi metadata.
    #[arg(long, conflicts_with = "list_tests")]
    all_tests: bool,

    /// Print test names without instantiating the module.
    #[arg(long)]
    list_tests: bool,

    /// Emit a single machine-readable JSON report. Guest output is captured.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Passed,
    Failed,
}

#[derive(Debug, Serialize)]
struct Execution {
    name: String,
    status: Status,
    duration_ms: f64,
    stdout: String,
    stderr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct Report {
    module: String,
    mode: &'static str,
    passed: usize,
    failed: usize,
    results: Vec<Execution>,
}

fn main() -> ExitCode {
    let arguments = Arguments::parse();
    match execute(&arguments) {
        Ok(success) => {
            if success {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            if arguments.json {
                let report = serde_json::json!({
                    "module": arguments.module.to_string_lossy(),
                    "mode": "error",
                    "passed": 0,
                    "failed": 1,
                    "results": [],
                    "error": format!("{error:#}"),
                });
                // A broken stdout should fail the command without causing a
                // second panic in the error-reporting path.
                let _ = writeln!(io::stdout().lock(), "{report}");
            } else {
                let _ = writeln!(io::stderr().lock(), "error: {error:#}");
            }
            ExitCode::FAILURE
        }
    }
}

fn execute(arguments: &Arguments) -> Result<bool> {
    let bytes = std::fs::read(&arguments.module)
        .with_context(|| format!("could not read {}", arguments.module.display()))?;
    if arguments.list_tests {
        // Discovery only needs the custom section. In particular, do not JIT
        // compile a large test module just to ask which tests it contains.
        let abi = Abi::from_wasm(&bytes)
            .with_context(|| format!("could not read ABI from {}", arguments.module.display()))?;
        let mut output = io::stdout().lock();
        if arguments.json {
            serde_json::to_writer(&mut output, &abi.tests)?;
            writeln!(output)?;
        } else {
            for test in &abi.tests {
                writeln!(output, "{test}")?;
            }
        }
        return Ok(true);
    }

    let program = Program::from_bytes(&bytes)
        .with_context(|| format!("could not load {}", arguments.module.display()))?;

    let test_mode = arguments.all_tests || !arguments.test.is_empty();
    let exports = if test_mode {
        let mut tests = arguments.test.clone();
        if arguments.all_tests {
            tests.extend(program.abi().tests.iter().cloned());
        }
        // Explicit selections and --all-tests can overlap. Execute each export
        // once while retaining deterministic discovery order.
        let mut seen = HashSet::new();
        tests.retain(|test| seen.insert(test.clone()));
        tests
    } else {
        vec![
            arguments
                .invoke
                .clone()
                .or_else(|| program.abi().entry.clone())
                .unwrap_or_else(|| "_start".into()),
        ]
    };

    let mut report = Report {
        module: arguments.module.to_string_lossy().into_owned(),
        mode: if test_mode { "test" } else { "run" },
        passed: 0,
        failed: 0,
        results: Vec::with_capacity(exports.len()),
    };

    for export in exports {
        if export.is_empty() {
            bail!("an entry or test export name cannot be empty");
        }
        let started = Instant::now();
        let options = if arguments.json || test_mode {
            HostOptions::default()
        } else {
            HostOptions::stdio()
        };

        // Instantiation per case isolates mutable guest state and allows the
        // runner to continue after a panic/trap without reusing a broken guest.
        let (result, stdout, stderr) = match program.instantiate(options) {
            Ok(mut instance) => {
                let result = instance.call_raw_empty(&export);
                let output = instance.take_output();
                (result, output.stdout, output.stderr)
            }
            Err(error) => (Err(error), String::new(), String::new()),
        };
        let error = result.err().map(|error| format!("{error:#}"));
        let status = if error.is_some() {
            report.failed += 1;
            Status::Failed
        } else {
            report.passed += 1;
            Status::Passed
        };

        let execution = Execution {
            name: export,
            status,
            duration_ms: started.elapsed().as_secs_f64() * 1000.0,
            stdout,
            stderr,
            error,
        };

        if !arguments.json {
            if test_mode {
                print_test_result(&execution)?;
            } else if let Some(error) = &execution.error {
                writeln!(io::stderr().lock(), "error: {error}")?;
            }
        }
        report.results.push(execution);
    }

    if arguments.json {
        let mut output = io::stdout().lock();
        serde_json::to_writer(&mut output, &report)?;
        writeln!(output)?;
    } else if test_mode {
        writeln!(
            io::stdout().lock(),
            "{} passed; {} failed; {} total",
            report.passed,
            report.failed,
            report.results.len()
        )?;
    }
    Ok(report.failed == 0)
}

fn print_test_result(execution: &Execution) -> Result<()> {
    let mut output = io::stdout().lock();
    if !execution.stdout.is_empty() {
        write!(output, "{}", execution.stdout)?;
    }
    if !execution.stderr.is_empty() {
        write!(io::stderr().lock(), "{}", execution.stderr)?;
    }
    let status = match execution.status {
        Status::Passed => "ok",
        Status::Failed => "FAILED",
    };
    writeln!(output, "test {} ... {status}", execution.name)?;
    if let Some(error) = &execution.error {
        writeln!(output, "  {error}")?;
    }
    Ok(())
}
