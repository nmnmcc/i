//! The i language frontend: project discovery, semantic analysis and module caches.
//!
//! Both command-line tools and editor clients share the same source loading and
//! type checking. Typed trees are immutable and can be shared with a code generator.

mod cache;
mod check;
mod manifest;
mod semantic;

pub use check::{CacheStats, CheckOptions, CheckedProject, Project, SourceFile, check_project};
pub use manifest::{
    Package, bundled_stdlib_path, dependency_manifest, load_packages, prepare_package_config,
};

use std::path::{Path, PathBuf};

/// Errors retain the upstream compiler's source diagnostics instead of reducing
/// parse and type errors to unlocated strings.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{path}: {message}")]
    Project { path: PathBuf, message: String },
    #[error("{}", .0.pretty_string())]
    Core(#[from] ilang_core::Error),
}

impl Error {
    pub fn project(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::Project {
            path: path.into(),
            message: message.into(),
        }
    }

    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Content fingerprint shared by the frontend, IR and final-artifact caches.
pub fn compiler_fingerprint() -> &'static str {
    static FINGERPRINT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    FINGERPRINT.get_or_init(|| {
        cache::digest(
            format!(
                "{}:{}",
                env!("ILANG_FRONTEND_REVISION"),
                ilang_core::version::COMPILER_REVISION
            )
            .as_bytes(),
        )
    })
}

pub fn format_source(path: &Path, source: &str) -> Result<String> {
    let path = camino::Utf8Path::from_path(path)
        .ok_or_else(|| Error::project(path, "source paths must be valid UTF-8"))?;
    let mut formatted = String::new();
    ilang_format::pretty(&mut formatted, &source.into(), path)?;
    Ok(formatted)
}
