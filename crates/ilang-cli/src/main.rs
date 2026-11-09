//! The `i` command.

use clap::{Parser, Subcommand};
use ilang_cli::{BuildOptions, build, level, run_test};
use ilang_host::{HostOptions, Outcome, Program, TestResult};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "i", version, about = "The i language")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check a package for errors.
    Check {
        /// The package directory, with its `i.toml`.
        #[arg(long, default_value = ".")]
        project: PathBuf,
        /// Print the type of every top-level name of the package, as
        /// `module/path.name : type`.
        #[arg(long)]
        types: bool,
    },
    /// Build a package into a Wasm module.
    Build {
        #[arg(long, default_value = ".")]
        project: PathBuf,
        /// Optimization level: 0, 2, or 3.
        #[arg(long, default_value_t = 2)]
        opt: u8,
        /// The output file; by default `build/<name>.wasm` in the package.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Also print the unoptimized WAT text next to the output.
        #[arg(long, hide = true)]
        wat: bool,
    },
    /// Build and run the program entry of a package.
    Run {
        #[arg(long, default_value = ".")]
        project: PathBuf,
        /// Optimization level: 0, 2, or 3.
        #[arg(long, default_value_t = 2)]
        opt: u8,
    },
    /// Build and run the tests of a package.
    Test {
        #[arg(long, default_value = ".")]
        project: PathBuf,
        /// Optimization level: 0, 2, or 3.
        #[arg(long, default_value_t = 0)]
        opt: u8,
        /// Only run the tests whose name has this text.
        filter: Option<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Check { project, types } => return check(&project, types),
        Command::Build {
            project,
            opt,
            out,
            wat,
        } => cmd_build(&project, opt, out, wat),
        Command::Run { project, opt } => cmd_run(&project, opt),
        Command::Test {
            project,
            opt,
            filter,
        } => cmd_test(&project, opt, filter.as_deref()),
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e:#}");
            ExitCode::FAILURE
        }
    }
}

fn options(opt: u8, tests: bool) -> anyhow::Result<BuildOptions> {
    let opt = level(opt).ok_or_else(|| anyhow::anyhow!("--opt must be 0, 2, or 3"))?;
    Ok(BuildOptions { tests, opt })
}

fn built(project: &Path, options: BuildOptions) -> anyhow::Result<Option<ilang_cli::Built>> {
    match build(project, options) {
        Ok(b) => {
            for w in &b.warnings {
                eprintln!("{w}");
            }
            Ok(Some(b))
        }
        Err(e @ ilang_cli::BuildError::Program { .. }) => {
            eprintln!("{e}");
            Ok(None)
        }
        Err(e) => Err(e.into()),
    }
}

fn cmd_build(project: &Path, opt: u8, out: Option<PathBuf>, wat: bool) -> anyhow::Result<ExitCode> {
    let Some(b) = built(project, options(opt, false)?)? else {
        return Ok(ExitCode::FAILURE);
    };
    let out = out.unwrap_or_else(|| project.join("build").join(format!("{}.wasm", b.name)));
    if let Some(dir) = out.parent()
        && !dir.as_os_str().is_empty()
    {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&out, &b.wasm)?;
    if wat {
        let (graph, _) = ilang_package::load(project);
        let checked = ilang_check::check(&graph);
        if let (Some(p), Some(t)) = (&checked.program, &checked.types)
            && let Ok(text) =
                ilang_wasm::compile_wat(&graph, p, t, ilang_wasm::Options { tests: false })
        {
            std::fs::write(out.with_extension("wat"), text)?;
        }
    }
    eprintln!("built {}", out.display());
    Ok(ExitCode::SUCCESS)
}

fn cmd_run(project: &Path, opt: u8) -> anyhow::Result<ExitCode> {
    let Some(b) = built(project, options(opt, false)?)? else {
        return Ok(ExitCode::FAILURE);
    };
    let program = Program::from_bytes(&b.wasm)?;
    if program.abi().entry.is_none() {
        anyhow::bail!("the package has no `@main` (§10.2)");
    }
    match program.run_entry(HostOptions::default())? {
        Outcome::Ok(_) => Ok(ExitCode::SUCCESS),
        Outcome::Stop(stop) => {
            eprintln!("{stop}");
            Ok(ExitCode::FAILURE)
        }
        Outcome::Err(error) => {
            eprintln!("error: {:?}", error.0);
            Ok(ExitCode::FAILURE)
        }
    }
}

fn cmd_test(project: &Path, opt: u8, filter: Option<&str>) -> anyhow::Result<ExitCode> {
    let Some(b) = built(project, options(opt, true)?)? else {
        return Ok(ExitCode::FAILURE);
    };
    let program = Program::from_bytes(&b.wasm)?;
    let tests: Vec<String> = program
        .abi()
        .tests
        .iter()
        .map(|t| t.name.clone())
        .filter(|n| filter.is_none_or(|f| n.contains(f)))
        .collect();
    let mut failed = 0;
    for name in &tests {
        let result = run_test(&program, name);
        match result {
            TestResult::Passed => println!("pass {name}"),
            TestResult::Stopped(stop) => {
                failed += 1;
                println!("FAIL {name}");
                for line in stop.to_string().lines() {
                    println!("  {line}");
                }
            }
            TestResult::Failed(e) => {
                failed += 1;
                println!("FAIL {name}");
                println!("  {e:#}");
            }
        }
    }
    println!("{} passed, {failed} failed", tests.len() - failed);
    Ok(if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Parses, resolves, and type checks every module of the package, its
/// dependencies, and the library, and prints the problems as
/// `path:line:col: error[CODE]: …`. With `types`, prints the type of each
/// top-level name of the package's modules, in source order.
fn check(project: &Path, types: bool) -> ExitCode {
    let (graph, mut diags) = ilang_package::load(project);
    if graph.root.is_some() {
        let checked = ilang_check::check(&graph);
        diags.extend(checked.diags);
        if types && let (Some(program), Some(typed)) = (&checked.program, &checked.types) {
            for (i, m) in program.modules.iter().enumerate() {
                if Some(m.package) == graph.root {
                    for line in typed.lines(program, ilang_check::hir::ModuleId(i as u32)) {
                        println!("{line}");
                    }
                }
            }
        }
    }
    for line in graph.sources.render_all(&diags) {
        eprintln!("{line}");
    }
    let errors = diags.iter().filter(|d| d.is_error()).count();
    if errors > 0 || graph.root.is_none() {
        eprintln!("{errors} error{}", if errors == 1 { "" } else { "s" });
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
