//! Execution of compiler-owned syntax exports in isolated Wasm instances.
//!
//! An invocation receives only the compiler syntax imports and grants to the
//! resolved library package. A fresh instance is created for each invocation,
//! so mutable guest state cannot leak from one expansion into another.

use anyhow::{Context, Result, bail};
use ilang_syntax::lisp::syntax::Syntax;

use crate::Built;

/// Invoke a compiler-generated export whose ABI takes and returns Syntax.
///
/// The compiler must check the export's signature and its reachable external
/// declarations before constructing `built`; the host also validates the ABI
/// and the opaque values on every call. The `Built` value binds the grant plan
/// to the exact compiled artifact.
pub fn invoke(built: &Built, export: &str, arguments: &[Syntax]) -> Result<Syntax> {
    let program = built.host_program().context("load macro program")?;
    let mut imports = ilang_host::HostImports::default();
    ilang_host::syntax::install(&mut imports).context("install syntax service")?;
    imports.grant_package(&built.binding_plan, &built.trusted_package);
    let mut instance = program
        .instantiate(ilang_host::HostOptions::new(imports))
        .context("instantiate macro program")?;
    let arguments: Vec<_> = arguments
        .iter()
        .cloned()
        .map(ilang_host::syntax::encode)
        .collect();
    match instance
        .call(export, &arguments)
        .context("call macro export")?
    {
        ilang_host::Outcome::Ok(value) => ilang_host::syntax::decode(&value)
            .cloned()
            .context("macro export returned a value other than Syntax"),
        ilang_host::Outcome::Err(error) => bail!("macro export returned an error: {error:?}"),
        ilang_host::Outcome::Stop(stop) => Err(stop.into()),
    }
}
