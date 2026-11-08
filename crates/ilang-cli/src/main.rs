//! The `i` command.
//!
//! The compiler is being rebuilt to follow `docs/spec.md`. Each command is
//! connected to the new pipeline as its milestone lands.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "i", version, about = "The i language")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check a package for errors.
    Check,
    /// Build a package into a Wasm module.
    Build,
    /// Build and run the program entry of a package.
    Run,
    /// Build and run the tests of a package.
    Test,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let name = match cli.command {
        Command::Check => "check",
        Command::Build => "build",
        Command::Run => "run",
        Command::Test => "test",
    };
    anyhow::bail!("`i {name}` is not available yet: the compiler is being rebuilt")
}
