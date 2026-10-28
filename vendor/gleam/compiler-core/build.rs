// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Wleam contributors

use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn collect_sources(root: &Path, sources: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).expect("read compiler source directory") {
        let entry = entry.expect("read compiler source directory entry");
        let path = entry.path();
        let kind = entry.file_type().expect("read compiler source file type");
        if kind.is_dir() {
            if !matches!(
                entry.file_name().to_str(),
                Some("snapshots" | "target" | "generated" | ".git" | ".devenv" | ".wleam")
            ) {
                collect_sources(&path, sources);
            }
        } else if kind.is_file() && path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
}

fn hash_file(hash: &mut Sha256, label: &str, path: &Path) {
    hash.update((label.len() as u64).to_le_bytes());
    hash.update(label.as_bytes());
    match fs::read(path) {
        Ok(contents) => {
            hash.update([1]);
            hash.update((contents.len() as u64).to_le_bytes());
            hash.update(contents);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // Lockfiles may be absent in source archives. Their creation or
            // deletion still changes the revision and causes Cargo to rerun.
            hash.update([0]);
        }
        Err(error) => panic!("read compiler revision input {}: {error}", path.display()),
    }
}

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let upstream = manifest.parent().expect("vendored Gleam workspace");
    let mut sources = Vec::new();
    for package in ["compiler-core", "hexpm", "pretty-arena", "src-span"] {
        let directory = upstream.join(package);
        let source = directory.join("src");
        // Watching the directory also notices new and removed source files.
        println!("cargo:rerun-if-changed={}", source.display());
        collect_sources(&source, &mut sources);
        sources.push(directory.join("Cargo.toml"));
        let build_script = directory.join("build.rs");
        if build_script.is_file() {
            sources.push(build_script);
        }
    }
    sources.extend([upstream.join("Cargo.toml"), upstream.join("Cargo.lock")]);
    sources.sort();
    sources.dedup();

    let mut hash = Sha256::new();
    hash.update(b"wleam-core-source-revision-v1");
    for path in &sources {
        println!("cargo:rerun-if-changed={}", path.display());
        let label = path
            .strip_prefix(upstream)
            .expect("source is inside vendored workspace")
            .to_string_lossy()
            .replace('\\', "/");
        hash_file(&mut hash, &label, path);
    }

    // When built as a Wleam dependency, the outer lockfile selects dependency
    // versions. Include it as well as the nested workspace's own lockfile.
    if let Some(repository) = upstream.parent().and_then(Path::parent) {
        for name in ["Cargo.toml", "Cargo.lock"] {
            let path = repository.join(name);
            println!("cargo:rerun-if-changed={}", path.display());
            hash_file(&mut hash, &format!("wleam/{name}"), &path);
        }
    }
    println!("cargo:rustc-env=WLEAM_CORE_REVISION={:x}", hash.finalize());
}
