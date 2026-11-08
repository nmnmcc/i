// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 The Gleam contributors

#[cfg(test)]
mod tests;

use std::{collections::HashSet, time::SystemTime};

use camino::{Utf8Path, Utf8PathBuf};

use ecow::EcoString;

use super::{
    Mode, Origin, SourceFingerprint, Target,
    package_compiler::{CacheMetadata, CachedModule, Input, UncompiledModule},
    package_loader::IFile,
};
use crate::{
    Error, Result,
    error::{FileIoAction, FileIoCause, FileKind},
    io::{FileSystemReader, FileSystemWriter},
    warning::WarningEmitter,
};

#[derive(Debug)]
pub(crate) struct ModuleLoader<'a, IO> {
    pub io: IO,
    pub warnings: &'a WarningEmitter,
    pub mode: Mode,
    pub target: Target,
    pub package_name: &'a EcoString,
    pub artefact_directory: &'a Utf8Path,
    pub origin: Origin,
    /// The set of modules that have had partial compilation done since the last
    /// successful compilation.
    pub incomplete_modules: &'a HashSet<EcoString>,
}

impl<'a, IO> ModuleLoader<'a, IO>
where
    IO: FileSystemReader + FileSystemWriter + Clone,
{
    /// Load a module from the given path.
    ///
    /// If the module has been compiled before and the source file has not been
    /// changed since then, load the precompiled data instead.
    ///
    /// Source content determines freshness, rather than its modification time.
    /// Editors, restored files, and coarse filesystem clocks can otherwise hide
    /// a changed module behind an unchanged or older timestamp.
    pub fn load(&self, file: IFile) -> Result<Input> {
        let name = file.module_name.clone();
        let source_mtime = self.io.modification_time(&file.path)?;

        let read_source = |name| self.read_source(file.path.clone(), name, source_mtime);

        let meta = match self.read_cache_metadata(&file)? {
            Some(meta) => meta,
            None => {
                return read_source(name).map(|module| Input::New {
                    module: Box::new(module),
                });
            }
        };

        // Hash the bytes without reparsing unchanged modules. A partial LSP
        // analysis must also be retried even when the source has been restored.
        let code = self.io.read(&file.path)?;
        if meta.fingerprint != SourceFingerprint::new(&code)
            || (self.mode == Mode::Lsp && self.incomplete_modules.contains(&name))
        {
            tracing::debug!(?name, "cache_stale");
            return read_source(name).map(|module| Input::New {
                module: Box::new(module),
            });
        }

        Ok(Input::Cached {
            module: self.cached(file, meta),
        })
    }

    /// Read the cache metadata file from the artefact directory for the given
    /// source file. If the file does not exist, return `None`.
    fn read_cache_metadata(&self, source_file: &IFile) -> Result<Option<CacheMetadata>> {
        let meta_path = source_file.cache_files(self.artefact_directory).meta_path;

        if !self.io.is_file(&meta_path) {
            return Ok(None);
        }

        let binary = self.io.read_bytes(&meta_path)?;
        let cache_metadata = CacheMetadata::from_binary(&binary).map_err(|()| Error::FileIo {
            action: FileIoAction::Parse,
            kind: FileKind::File,
            path: meta_path,
            cause: FileIoCause::CacheMetadataFormatIncorrect,
        })?;
        Ok(Some(cache_metadata))
    }

    fn read_source(
        &self,
        path: Utf8PathBuf,
        name: EcoString,
        mtime: SystemTime,
    ) -> Result<UncompiledModule, Error> {
        read_source(
            self.io.clone(),
            self.target,
            self.origin,
            path,
            name,
            self.package_name.clone(),
            mtime,
            self.warnings.clone(),
        )
    }

    fn cached(&self, file: IFile, meta: CacheMetadata) -> CachedModule {
        CachedModule {
            dependencies: meta.dependencies,
            source_path: file.path,
            origin: self.origin,
            name: file.module_name,
        }
    }
}

#[expect(clippy::too_many_arguments)]
pub(crate) fn read_source<IO>(
    io: IO,
    target: Target,
    origin: Origin,
    path: Utf8PathBuf,
    name: EcoString,
    package_name: EcoString,
    mtime: SystemTime,
    emitter: WarningEmitter,
) -> Result<UncompiledModule>
where
    IO: FileSystemReader + FileSystemWriter + Clone,
{
    let code: EcoString = io.read(&path)?.into();

    let parsed = crate::parse::parse_module(path.clone(), &code, &emitter).map_err(|error| {
        Error::Parse {
            path: path.clone(),
            src: code.clone(),
            error: Box::new(error),
        }
    })?;
    let mut ast = parsed.module;
    let extra = parsed.extra;
    let dependencies = ast.dependencies(target);

    ast.name = name.clone();
    let module = UncompiledModule {
        package: package_name,
        dependencies,
        origin,
        extra,
        mtime,
        path,
        name,
        code,
        ast,
    };
    Ok(module)
}
