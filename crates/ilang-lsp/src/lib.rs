//! The language server of i (`i lsp`): diagnostics, hover, go to
//! definition, document symbols, completion, and formatting, over the
//! Language Server Protocol on standard input and output.
//!
//! The answers come from `ilang_check::query`. Positions are converted
//! between the UTF-16 positions of LSP and the byte offsets of the compiler
//! in [`position`].

mod analysis;
mod change;
pub mod format;
pub mod position;
mod server;
pub mod uri;

pub use server::{Options, capabilities, run};

/// Runs the server on standard input and output until the client exits.
pub fn run_stdio() -> anyhow::Result<()> {
    let (conn, io) = lsp_server::Connection::stdio();
    run(&conn, Options::default())?;
    drop(conn);
    io.join()?;
    Ok(())
}
