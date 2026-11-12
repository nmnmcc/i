//! Runs the program entry and the tests of a compiled module.
//!
//! cargo run --release -p ilang-host --example run -- MODULE.wasm
//!
//! The module may only need host services and externals that this example
//! gives: none. A real host registers them in `HostImports`.

use anyhow::{Context, Result, ensure};
use ilang_host::{HostOptions, Outcome, Program, TestResult};

fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let path = arguments.next().context("usage: run MODULE.wasm")?;
    ensure!(
        arguments.next().is_none(),
        "expected exactly one Wasm module path"
    );
    let bytes = std::fs::read(&path).context("reading the module")?;
    let program = Program::from_bytes(&bytes)?;
    let options = HostOptions::default();
    let mut failed = 0;
    if program.abi().entry.is_some() {
        match program.run_entry(options.clone())? {
            Outcome::Ok(_) => println!("entry: done"),
            Outcome::Err(error) => println!("entry: error {:?}", error.0),
            Outcome::Stop(stop) => {
                failed += 1;
                println!("entry: stopped\n{stop}");
            }
        }
    }
    for report in program.run_tests(&options) {
        match report.result {
            TestResult::Passed => println!("test {}: passed", report.name),
            TestResult::Stopped(stop) => {
                failed += 1;
                println!("test {}: failed\n{stop}", report.name);
            }
            TestResult::Failed(error) => {
                failed += 1;
                println!("test {}: could not run: {error:#}", report.name);
            }
        }
    }
    ensure!(failed == 0, "{failed} failed");
    Ok(())
}
