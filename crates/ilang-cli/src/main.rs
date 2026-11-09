//! The `i` command.
//!
//! The compiler is being rebuilt to follow `docs/spec.md`. Each command is
//! connected to the new pipeline as its milestone lands.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
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
    Build,
    /// Build and run the program entry of a package.
    Run,
    /// Build and run the tests of a package.
    Test,
}

fn main() -> anyhow::Result<ExitCode> {
    let cli = Cli::parse();
    let name = match cli.command {
        Command::Check { project, types } => return Ok(check(&project, types)),
        Command::Build => "build",
        Command::Run => "run",
        Command::Test => "test",
    };
    anyhow::bail!("`i {name}` is not available yet: the compiler is being rebuilt")
}

/// Parses, resolves, and type checks every module of the package, its
/// dependencies, and the library, and prints the problems as
/// `path:line:col: error[CODE]: …`. With `types`, prints the type of each
/// top-level name of the package's modules, in source order.
fn check(project: &std::path::Path, types: bool) -> ExitCode {
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
