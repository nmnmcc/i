//! The Wleam language server, using Gleam's semantic editor engine.
//!
//! Transport and file-system integration live here. The compiler and editor
//! operations share the same parser, inferred AST, and module interfaces.

mod io;

use anyhow::Context;
use gleam_language_server::LanguageServer;

/// Run the language server over the Language Server Protocol's stdio transport.
/// Standard output is reserved exclusively for protocol messages.
pub fn run() -> anyhow::Result<()> {
    let (connection, threads) = lsp_server::Connection::stdio();
    let result = run_connection(connection);
    threads
        .join()
        .context("joining language server transport threads")?;
    result
}

/// Run a server on an existing connection. Embedders and tests can supply an
/// in-memory transport without opening stdio or starting another process.
pub fn run_connection(connection: lsp_server::Connection) -> anyhow::Result<()> {
    let mut server = LanguageServer::new(&connection, io::ProjectIo)
        .map_err(|error| anyhow::anyhow!(error.pretty_string()))?;
    server
        .run()
        .map_err(|error| anyhow::anyhow!(error.pretty_string()))
}
