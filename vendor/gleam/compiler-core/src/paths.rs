// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2021 The Gleam contributors

use crate::build::{Mode, Target};

use camino::{Utf8Path, Utf8PathBuf};

pub const ARTEFACT_DIRECTORY_NAME: &str = "_gleam_artefacts";

#[derive(Debug, Clone)]
pub struct ProjectPaths {
    root: Utf8PathBuf,
}

impl ProjectPaths {
    pub fn new(root: Utf8PathBuf) -> Self {
        Self { root }
    }

    pub fn at_filesystem_root() -> Self {
        let path = if cfg!(target_family = "windows") {
            r"C:\"
        } else {
            "/"
        };

        Self::new(Utf8PathBuf::from(path))
    }

    pub fn root(&self) -> &Utf8Path {
        &self.root
    }

    pub fn root_config(&self) -> Utf8PathBuf {
        self.root.join("gleam.toml")
    }

    pub fn readme(&self) -> Utf8PathBuf {
        self.root.join("README.md")
    }

    pub fn manifest(&self) -> Utf8PathBuf {
        self.root.join("manifest.toml")
    }

    pub fn src_directory(&self) -> Utf8PathBuf {
        self.root.join("src")
    }

    pub fn test_directory(&self) -> Utf8PathBuf {
        self.root.join("test")
    }

    pub fn dev_directory(&self) -> Utf8PathBuf {
        self.root.join("dev")
    }

    pub fn build_directory(&self) -> Utf8PathBuf {
        self.root.join("build")
    }

    pub fn build_packages_directory(&self) -> Utf8PathBuf {
        self.build_directory().join("packages")
    }

    pub fn build_packages_toml(&self) -> Utf8PathBuf {
        self.build_packages_directory().join("packages.toml")
    }

    pub fn build_packages_package(&self, package_name: &str) -> Utf8PathBuf {
        self.build_packages_directory().join(package_name)
    }

    // build_deps_package_config
    pub fn build_packages_package_config(&self, package_name: &str) -> Utf8PathBuf {
        self.build_packages_package(package_name).join("gleam.toml")
    }

    pub fn build_directory_for_mode(&self, mode: Mode) -> Utf8PathBuf {
        self.build_directory().join(mode.to_string())
    }

    pub fn build_directory_for_target(&self, mode: Mode, target: Target) -> Utf8PathBuf {
        let target = match target {
            Target::Wasm => "wasm",
        };
        self.build_directory_for_mode(mode).join(target)
    }

    /// Compiled artefacts are isolated by mode, target, and package name.
    pub fn build_directory_for_package(
        &self,
        mode: Mode,
        target: Target,
        package_name: &str,
    ) -> Utf8PathBuf {
        self.build_directory_for_target(mode, target)
            .join(package_name)
    }

    /// A path to a special file that contains the version of gleam
    /// that last built the artefacts along with build-impacting
    /// configuration. If
    /// this file does not match the current version of gleam we
    /// will rebuild from scratch
    pub fn build_configuration_fingerprint(&self, mode: Mode, target: Target) -> Utf8PathBuf {
        self.build_directory_for_target(mode, target)
            .join("fingerprint")
    }

    /// The path to the source gleam.toml file for a path dependency.
    pub fn path_dependency_gleam_toml_path(&self, dependency_path: &Utf8Path) -> Utf8PathBuf {
        self.root().join(dependency_path).join("gleam.toml")
    }

    pub fn dependency_gleam_toml_fingerprint_path(&self, dependency_name: &str) -> Utf8PathBuf {
        self.build_packages_directory()
            .join(format!("{}.config_fingerprint", dependency_name))
    }
}

pub fn unnest(within: &Utf8Path) -> Utf8PathBuf {
    let mut path = Utf8PathBuf::new();
    for _ in within {
        path = path.join("..");
    }
    path
}

#[test]
fn wasm_package_paths_are_isolated() {
    let paths = ProjectPaths::new("/workspace".into());
    assert_eq!(
        paths.build_directory_for_package(Mode::Dev, Target::Wasm, "sample"),
        Utf8Path::new("/workspace/build/dev/wasm/sample")
    );
    assert_eq!(
        paths.build_directory_for_package(Mode::Prod, Target::Wasm, "sample"),
        Utf8Path::new("/workspace/build/prod/wasm/sample")
    );
}
