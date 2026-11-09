//! `i lsp`: the language server, on standard input and output (see the
//! crate `ilang-lsp`).

use std::process::ExitCode;

pub fn run() -> anyhow::Result<ExitCode> {
    ilang_lsp::run_stdio()?;
    Ok(ExitCode::SUCCESS)
}
