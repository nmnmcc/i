// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 The Gleam contributors

use ilang_core::{
    Error, Result,
    build::SourceFingerprint,
    error::{FileIoAction, FileIoCause, FileKind},
    io::{FileSystemReader, FileSystemWriter},
    paths::ProjectPaths,
};
use std::collections::{HashMap, hash_map::Entry};

use camino::{Utf8Path, Utf8PathBuf};

use super::{
    DownloadDependencies, MakeLocker, engine::LanguageServerEngine, feedback::FeedbackBookKeeper,
    files::FileSystemProxy, progress::ProgressReporter,
};

/// The language server instance serves a language client, typically a text
/// editor. The editor could have multiple i projects open at once, so run
/// an instance of the language server engine for each project.
///
/// This router is responsible for finding or creating an engine for a given
/// file using the nearest parent `i.toml` file.
///
#[derive(Debug)]
pub(crate) struct Router<IO, Reporter> {
    io: FileSystemProxy<IO>,
    engines: HashMap<Utf8PathBuf, Project<IO, Reporter>>,
    progress_reporter: Reporter,
}

impl<IO, Reporter> Router<IO, Reporter>
where
    // IO to be supplied from outside of ilang-core
    IO: FileSystemReader + FileSystemWriter + DownloadDependencies + MakeLocker + Clone,
    // IO to be supplied from inside of ilang-core
    Reporter: ProgressReporter + Clone,
{
    pub fn new(progress_reporter: Reporter, io: FileSystemProxy<IO>) -> Self {
        Self {
            io,
            engines: HashMap::new(),
            progress_reporter,
        }
    }

    pub fn project_path(&self, path: &Utf8Path) -> Option<Utf8PathBuf> {
        find_i_project_parent(&self.io, path)
    }

    pub fn project_for_path(
        &mut self,
        path: Utf8PathBuf,
    ) -> Result<Option<&mut Project<IO, Reporter>>> {
        // If the path is the root of a known project then return it. Otherwise
        // find the nearest parent project.
        let path = if self.engines.contains_key(&path) {
            path
        } else {
            let Some(path) = find_i_project_parent(&self.io, &path) else {
                return Ok(None);
            };
            path
        };

        // Recreate the compiler when configuration or its cache changes, but
        // retain diagnostic bookkeeping so old errors can be cleared and open
        // documents remain tracked after reloading.
        if let Some(project) = self.engines.get(&path) {
            let paths = ProjectPaths::new(path.clone());
            if project.invalidated
                || !self.io.exists(&paths.build_directory())
                || Self::i_toml_changed(&paths, project, &self.io)?
            {
                let mut replacement = Self::new_project(
                    path.clone(),
                    self.io.clone(),
                    self.progress_reporter.clone(),
                )?;
                if let Some(previous) = self.engines.get_mut(&path) {
                    replacement.feedback = std::mem::take(&mut previous.feedback);
                }
                let _ = self.engines.insert(path.clone(), replacement);
            }
        }

        // Look up the project, creating a new one if it does not exist.
        Ok(Some(match self.engines.entry(path.clone()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let project =
                    Self::new_project(path, self.io.clone(), self.progress_reporter.clone())?;
                entry.insert(project)
            }
        }))
    }

    /// Has i.toml changed since the last time we saw this project?
    fn i_toml_changed(
        paths: &ProjectPaths,
        project: &Project<IO, Reporter>,
        io: &FileSystemProxy<IO>,
    ) -> Result<bool, Error> {
        // Get the location of i.toml for this project
        let config_path = paths.root_config();

        // Editors and file synchronisation tools can preserve modification
        // times. Configuration determines the entire dependency graph, so use
        // its contents as the cache identity on every project lookup.
        let toml = io.read(&config_path)?;
        let i_toml_changed = project.i_toml_fingerprint != SourceFingerprint::new(&toml);
        Ok(i_toml_changed)
    }

    pub fn delete_engine_for_path(&mut self, path: &Utf8Path) {
        if let Some(path) = find_i_project_parent(&self.io, path) {
            if let Some(project) = self.engines.get_mut(&path) {
                project.invalidated = true;
            }
        }
    }

    fn new_project(
        path: Utf8PathBuf,
        io: FileSystemProxy<IO>,
        progress_reporter: Reporter,
    ) -> Result<Project<IO, Reporter>, Error> {
        tracing::info!(?path, "creating_new_language_server_engine");
        let paths = ProjectPaths::new(path);
        let config_path = paths.root_config();
        let toml = io.read(&config_path)?;
        let config = toml::from_str(&toml).map_err(|error| Error::FileIo {
            action: FileIoAction::Parse,
            kind: FileKind::File,
            path: config_path,
            cause: FileIoCause::Other(error.to_string()),
        })?;
        let engine = LanguageServerEngine::new(config, progress_reporter, io, paths)?;
        let project = Project {
            engine,
            feedback: FeedbackBookKeeper::default(),
            needs_compilation: true,
            invalidated: false,
            i_toml_fingerprint: SourceFingerprint::new(&toml),
        };
        Ok(project)
    }
}

/// Given a given path, find the nearest parent directory containing a
/// `i.toml` file.
///
/// The file must be in either the `src`, `test` or `dev` directory if it is not a
/// `.i` file.
fn find_i_project_parent<IO>(io: &IO, path: &Utf8Path) -> Option<Utf8PathBuf>
where
    IO: FileSystemReader,
{
    let is_module = path.extension().map(|x| x == "i").unwrap_or(false);
    let mut directory = path.to_path_buf();

    // If we are finding the gleam project of a directory then we want to check the directory itself
    let is_directory = path.extension().is_none();
    if is_directory {
        directory.push("src");
    }

    while let Some(root) = directory.parent() {
        // If there's no i.toml in the root then we continue to the next parent.
        if !io.is_file(&root.join("i.toml")) {
            _ = directory.pop();
            continue;
        }

        // If it is an i module then it must reside in the src, test dev directory.
        if is_module
            && !(directory.ends_with("test")
                || directory.ends_with("src")
                || directory.ends_with("dev"))
        {
            _ = directory.pop();
            continue;
        }

        return Some(root.to_path_buf());
    }
    None
}

#[derive(Debug)]
pub(crate) struct Project<A, B> {
    pub engine: LanguageServerEngine<A, B>,
    pub feedback: FeedbackBookKeeper,
    pub needs_compilation: bool,
    invalidated: bool,
    pub i_toml_fingerprint: SourceFingerprint,
}

#[cfg(test)]
mod find_i_project_parent_tests {
    use super::*;
    use ilang_core::io::{FileSystemWriter, memory::InMemoryFileSystem};

    #[test]
    fn root() {
        let io = InMemoryFileSystem::new();
        assert_eq!(find_i_project_parent(&io, Utf8Path::new("/")), None);
    }

    #[test]
    fn outside_a_project() {
        let io = InMemoryFileSystem::new();
        assert_eq!(
            find_i_project_parent(&io, Utf8Path::new("/app/src/one.i")),
            None
        );
    }

    #[test]
    fn i_toml_itself() {
        let io = InMemoryFileSystem::new();
        io.write(Utf8Path::new("/app/i.toml"), "").unwrap();
        assert_eq!(
            find_i_project_parent(&io, Utf8Path::new("/app/i.toml")),
            Some(Utf8PathBuf::from("/app"))
        );
    }

    #[test]
    fn directory_with_i_toml() {
        let io = InMemoryFileSystem::new();
        io.write(Utf8Path::new("/app/i.toml"), "").unwrap();
        assert_eq!(
            find_i_project_parent(&io, Utf8Path::new("/app")),
            Some(Utf8PathBuf::from("/app"))
        );
    }

    #[test]
    fn test_module() {
        let io = InMemoryFileSystem::new();
        io.write(Utf8Path::new("/app/i.toml"), "").unwrap();
        assert_eq!(
            find_i_project_parent(&io, Utf8Path::new("/app/test/one/two/three.i")),
            Some(Utf8PathBuf::from("/app"))
        );
    }

    #[test]
    fn src_module() {
        let io = InMemoryFileSystem::new();
        io.write(Utf8Path::new("/app/i.toml"), "").unwrap();
        assert_eq!(
            find_i_project_parent(&io, Utf8Path::new("/app/src/one/two/three.i")),
            Some(Utf8PathBuf::from("/app"))
        );
    }

    #[test]
    fn dev_module() {
        let io = InMemoryFileSystem::new();
        io.write(Utf8Path::new("/app/i.toml"), "").unwrap();
        assert_eq!(
            find_i_project_parent(&io, Utf8Path::new("/app/dev/one/two/three.i")),
            Some(Utf8PathBuf::from("/app"))
        );
    }

    // https://github.com/gleam-lang/gleam/issues/2121
    #[test]
    fn module_in_project_but_not_src_or_test_or_dev() {
        let io = InMemoryFileSystem::new();
        io.write(Utf8Path::new("/app/i.toml"), "").unwrap();
        assert_eq!(
            find_i_project_parent(&io, Utf8Path::new("/app/other/one/two/three.i")),
            None,
        );
    }

    #[test]
    fn nested_projects() {
        let io = InMemoryFileSystem::new();
        io.write(Utf8Path::new("/app/i.toml"), "").unwrap();
        io.write(Utf8Path::new("/app/examples/wibble/i.toml"), "")
            .unwrap();
        assert_eq!(
            find_i_project_parent(&io, Utf8Path::new("/app/src/one.i")),
            Some(Utf8PathBuf::from("/app"))
        );
        assert_eq!(
            find_i_project_parent(&io, Utf8Path::new("/app/examples/wibble/src/one.i")),
            Some(Utf8PathBuf::from("/app/examples/wibble"))
        );
    }
}
