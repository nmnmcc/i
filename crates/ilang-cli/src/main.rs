//! The `i` command.

mod commands;

use clap::{Parser, Subcommand};
use ilang_cli::{BuildOptions, build, level, run_test};
use ilang_host::{Outcome, Program, TestResult};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "i", version, about = "The i programming language")]
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
        /// Do not use the network to get packages; only the cache is used.
        #[arg(long)]
        offline: bool,
        /// Print the type of every top-level name of the package, as
        /// `module/path.name : type`.
        #[arg(long)]
        types: bool,
    },
    /// Build a package into a Wasm module.
    Build {
        #[arg(long, default_value = ".")]
        project: PathBuf,
        /// Do not use the network to get packages; only the cache is used.
        #[arg(long)]
        offline: bool,
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
        /// Do not use the network to get packages; only the cache is used.
        #[arg(long)]
        offline: bool,
        /// Optimization level: 0, 2, or 3.
        #[arg(long, default_value_t = 2)]
        opt: u8,
    },
    /// Build and run the tests of a package.
    Test {
        #[arg(long, default_value = ".")]
        project: PathBuf,
        /// Do not use the network to get packages; only the cache is used.
        #[arg(long)]
        offline: bool,
        /// Optimization level: 0, 2, or 3.
        #[arg(long, default_value_t = 0)]
        opt: u8,
        /// Only run the tests whose name has this text.
        filter: Option<String>,
        /// Print how long each test takes to run, without the build.
        #[arg(long)]
        time: bool,
    },
    #[command(flatten)]
    Package(commands::package::PackageCommand),
    /// Run the language server, on standard input and output.
    Lsp,
    /// Format source files; by default every `.i` file in `src/` and `test/`.
    Fmt {
        #[arg(long, default_value = ".")]
        project: PathBuf,
        /// Write nothing; list the files that would change, and fail if any.
        #[arg(long)]
        check: bool,
        /// Files or directories to format instead.
        paths: Vec<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Check {
            project,
            offline,
            types,
        } => return check(&project, offline, types),
        Command::Build {
            project,
            offline,
            opt,
            out,
            wat,
        } => cmd_build(&project, opt, offline, out, wat),
        Command::Run {
            project,
            offline,
            opt,
        } => cmd_run(&project, opt, offline),
        Command::Test {
            project,
            offline,
            opt,
            filter,
            time,
        } => cmd_test(&project, opt, offline, filter.as_deref(), time),
        Command::Package(c) => commands::package::run(c),
        Command::Lsp => commands::lsp::run(),
        Command::Fmt {
            project,
            check,
            paths,
        } => commands::fmt::run(&project, check, &paths),
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e:#}");
            ExitCode::FAILURE
        }
    }
}

fn options(opt: u8, tests: bool, offline: bool) -> anyhow::Result<BuildOptions> {
    let opt = level(opt).ok_or_else(|| anyhow::anyhow!("--opt must be 0, 2, or 3"))?;
    Ok(BuildOptions {
        tests,
        opt,
        offline,
    })
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

fn cmd_build(
    project: &Path,
    opt: u8,
    offline: bool,
    out: Option<PathBuf>,
    wat: bool,
) -> anyhow::Result<ExitCode> {
    let Some(b) = built(project, options(opt, false, offline)?)? else {
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
        let (graph, _) = ilang_cli::load(project, false, offline);
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

fn cmd_run(project: &Path, opt: u8, offline: bool) -> anyhow::Result<ExitCode> {
    let Some(b) = built(project, options(opt, false, offline)?)? else {
        return Ok(ExitCode::FAILURE);
    };
    let program = Program::from_bytes(&b.wasm)?;
    if program.abi().entry.is_none() {
        anyhow::bail!("the package has no `@main` (§10.2)");
    }
    match program.run_entry(ilang_cli::standard_options())? {
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

fn cmd_test(
    project: &Path,
    opt: u8,
    offline: bool,
    filter: Option<&str>,
    time: bool,
) -> anyhow::Result<ExitCode> {
    let Some(b) = built(project, options(opt, true, offline)?)? else {
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
        let start = (std::time::Instant::now(), cpu_seconds());
        let result = run_test(&program, name);
        let took = if time {
            let wall = start.0.elapsed().as_secs_f64();
            match (start.1, cpu_seconds()) {
                (Some(a), Some(b)) => format!(" ({wall:.3} s, cpu {:.2} s)", b - a),
                _ => format!(" ({wall:.3} s)"),
            }
        } else {
            String::new()
        };
        match result {
            TestResult::Passed => println!("pass {name}{took}"),
            TestResult::Stopped(stop) => {
                failed += 1;
                println!("FAIL {name}{took}");
                for line in stop.to_string().lines() {
                    println!("  {line}");
                }
            }
            TestResult::Failed(e) => {
                failed += 1;
                println!("FAIL {name}{took}");
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

/// The processor time of the process so far (user and system), where the
/// system tells it: on a busy machine it is steadier than the time taken.
fn cpu_seconds() -> Option<f64> {
    // Fields 14 and 15 of /proc/self/stat, after the command name in
    // parentheses, in clock ticks of 1/100 s.
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let rest = &stat[stat.rfind(')')? + 2..];
    let mut fields = rest.split(' ').skip(11);
    let user: f64 = fields.next()?.parse().ok()?;
    let system: f64 = fields.next()?.parse().ok()?;
    Some((user + system) / 100.0)
}

/// Parses, resolves, and type checks every module of the package, its
/// dependencies, and the library, and prints the problems as
/// `path:line:col: error[CODE]: …`. With `types`, prints the type of each
/// top-level name of the package's modules, in source order. The tests are
/// checked too.
fn check(project: &Path, offline: bool, types: bool) -> ExitCode {
    let (graph, mut diags) = ilang_cli::load(project, true, offline);
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
