use std::{
    fs::{self, File, OpenOptions},
    io,
    time::SystemTime,
};

use camino::{Utf8Path, Utf8PathBuf};
use gleam_core::{
    Error, Result,
    build::Target,
    error::{FileIoAction, FileIoCause, FileKind},
    io::{DirEntry, FileSystemReader, FileSystemWriter, ReadDir, WrappedReader},
    manifest::Manifest,
    paths::ProjectPaths,
};
use gleam_language_server::{DownloadDependencies, LockGuard, Locker, MakeLocker};

#[derive(Clone, Copy, Debug)]
pub(crate) struct ProjectIo;

fn file_error(path: &Utf8Path, action: FileIoAction, error: io::Error) -> Error {
    Error::FileIo {
        action,
        kind: FileKind::File,
        path: path.to_owned(),
        cause: FileIoCause::std_io(error),
    }
}

fn directory_error(path: &Utf8Path, action: FileIoAction, error: io::Error) -> Error {
    Error::FileIo {
        action,
        kind: FileKind::Directory,
        path: path.to_owned(),
        cause: FileIoCause::std_io(error),
    }
}

impl FileSystemReader for ProjectIo {
    fn read_dir(&self, path: &Utf8Path) -> Result<ReadDir> {
        let entries =
            fs::read_dir(path).map_err(|error| directory_error(path, FileIoAction::Read, error))?;
        Ok(entries
            .map(|entry| {
                let entry = entry?;
                let path = Utf8PathBuf::from_path_buf(entry.path()).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "non-UTF-8 file name")
                })?;
                Ok(DirEntry::from_pathbuf(path))
            })
            .collect())
    }

    fn read(&self, path: &Utf8Path) -> Result<String> {
        fs::read_to_string(path).map_err(|error| file_error(path, FileIoAction::Read, error))
    }

    fn read_bytes(&self, path: &Utf8Path) -> Result<Vec<u8>> {
        fs::read(path).map_err(|error| file_error(path, FileIoAction::Read, error))
    }

    fn reader(&self, path: &Utf8Path) -> Result<WrappedReader> {
        let file = File::open(path).map_err(|error| file_error(path, FileIoAction::Open, error))?;
        Ok(WrappedReader::new(path, Box::new(file)))
    }

    fn is_file(&self, path: &Utf8Path) -> bool {
        path.is_file()
    }

    fn is_directory(&self, path: &Utf8Path) -> bool {
        path.is_dir()
    }

    fn is_same_file(&self, left: &Utf8Path, right: &Utf8Path) -> Result<bool> {
        Ok(self.canonicalise(left)? == self.canonicalise(right)?)
    }

    fn modification_time(&self, path: &Utf8Path) -> Result<SystemTime> {
        fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .map_err(|error| file_error(path, FileIoAction::ReadMetadata, error))
    }

    fn canonicalise(&self, path: &Utf8Path) -> Result<Utf8PathBuf> {
        let canonical = fs::canonicalize(path)
            .map_err(|error| file_error(path, FileIoAction::Canonicalise, error))?;
        Utf8PathBuf::from_path_buf(canonical).map_err(|_| {
            file_error(
                path,
                FileIoAction::Canonicalise,
                io::Error::new(io::ErrorKind::InvalidData, "non-UTF-8 file name"),
            )
        })
    }
}

impl FileSystemWriter for ProjectIo {
    fn mkdir(&self, path: &Utf8Path) -> Result<()> {
        fs::create_dir_all(path).map_err(|error| directory_error(path, FileIoAction::Create, error))
    }

    fn write(&self, path: &Utf8Path, content: &str) -> Result<()> {
        self.write_bytes(path, content.as_bytes())
    }

    fn write_bytes(&self, path: &Utf8Path, content: &[u8]) -> Result<()> {
        if let Some(parent) = path.parent() {
            self.mkdir(parent)?;
        }
        fs::write(path, content).map_err(|error| file_error(path, FileIoAction::WriteTo, error))
    }

    fn delete_directory(&self, path: &Utf8Path) -> Result<()> {
        match fs::remove_dir_all(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(directory_error(path, FileIoAction::Delete, error)),
        }
    }

    fn copy(&self, from: &Utf8Path, to: &Utf8Path) -> Result<()> {
        if let Some(parent) = to.parent() {
            self.mkdir(parent)?;
        }
        fs::copy(from, to)
            .map(|_| ())
            .map_err(|error| file_error(from, FileIoAction::Copy(Some(to.to_owned())), error))
    }

    fn copy_dir(&self, from: &Utf8Path, to: &Utf8Path) -> Result<()> {
        self.mkdir(to)?;
        for entry in self.read_dir(from)? {
            let entry = entry.map_err(|error| directory_error(from, FileIoAction::Read, error))?;
            let source = entry.into_path();
            let Some(name) = source.file_name() else {
                continue;
            };
            let destination = to.join(name);
            if source.is_dir() {
                self.copy_dir(&source, &destination)?;
            } else {
                self.copy(&source, &destination)?;
            }
        }
        Ok(())
    }

    fn hardlink(&self, from: &Utf8Path, to: &Utf8Path) -> Result<()> {
        if let Some(parent) = to.parent() {
            self.mkdir(parent)?;
        }
        fs::hard_link(from, to)
            .map_err(|error| file_error(from, FileIoAction::Link(to.to_owned()), error))
    }

    fn symlink_dir(&self, from: &Utf8Path, to: &Utf8Path) -> Result<()> {
        #[cfg(unix)]
        let result = std::os::unix::fs::symlink(from, to);
        #[cfg(windows)]
        let result = std::os::windows::fs::symlink_dir(from, to);
        #[cfg(not(any(unix, windows)))]
        let result: io::Result<()> = Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "directory symlinks",
        ));
        result.map_err(|error| directory_error(from, FileIoAction::Link(to.to_owned()), error))
    }

    fn delete_file(&self, path: &Utf8Path) -> Result<()> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(file_error(path, FileIoAction::Delete, error)),
        }
    }

    fn exists(&self, path: &Utf8Path) -> bool {
        path.exists()
    }
}

struct BuildLocker {
    path: Utf8PathBuf,
}

impl Locker for BuildLocker {
    fn lock_for_build(&self) -> Result<LockGuard> {
        if let Some(parent) = self.path.parent() {
            ProjectIo.mkdir(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|error| file_error(&self.path, FileIoAction::Open, error))?;
        file.lock()
            .map_err(|error| file_error(&self.path, FileIoAction::Lock, error))?;
        // The kernel releases the lock when the guard closes its file.
        Ok(LockGuard(Box::new(file)))
    }
}

impl MakeLocker for ProjectIo {
    fn make_locker(&self, paths: &ProjectPaths, _target: Target) -> Result<Box<dyn Locker>> {
        Ok(Box::new(BuildLocker {
            path: paths.root().join("build/.wleam-lsp.lock"),
        }))
    }
}

impl DownloadDependencies for ProjectIo {
    fn download_dependencies(&self, paths: &ProjectPaths) -> Result<Manifest> {
        wleam_frontend::dependency_manifest(paths.root()).map_err(|error| Error::FileIo {
            path: paths.root_config(),
            action: FileIoAction::Parse,
            kind: FileKind::File,
            cause: FileIoCause::Other(error.to_string()),
        })
    }
}
