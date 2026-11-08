use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn files_under(root: &Path, files: &mut Vec<PathBuf>) {
    if !root.exists() {
        return;
    }
    if root.is_file() {
        files.push(root.to_owned());
        return;
    }
    let mut entries = fs::read_dir(root)
        .expect("read bundled source directory")
        .map(|entry| entry.expect("read source directory entry").path())
        .collect::<Vec<_>>();
    entries.sort();
    for entry in entries {
        files_under(&entry, files);
    }
}

fn fingerprint(root: &Path, files: &[PathBuf], marker: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(marker);
    for path in files {
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let bytes = fs::read(path).expect("read source for fingerprint");
        hash.update((relative.len() as u64).to_le_bytes());
        hash.update(relative.as_bytes());
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    format!("{:x}", hash.finalize())
}

fn main() {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let repository = manifest_dir
        .join("../..")
        .canonicalize()
        .expect("repository root");
    let stdlib = repository.join("stdlib");
    // Generated package caches live next to these inputs, under stdlib/build.
    // Watching the package directory would make checking the standard library
    // rebuild the Rust compiler even when none of its bundled sources changed.
    for path in [stdlib.join("i.toml"), stdlib.join("src")] {
        if path.exists() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
    let mut stdlib_files = vec![stdlib.join("i.toml")];
    files_under(&stdlib.join("src"), &mut stdlib_files);
    stdlib_files.retain(|path| {
        path.file_name().is_some_and(|name| name == "i.toml")
            || path.extension().is_some_and(|ext| ext == "i")
    });
    for name in [
        "LICENSE",
        "NOTICE",
        "LICENCE",
        "LICENSE-APACHE",
        "LICENSE-MIT",
    ] {
        let path = stdlib.join(name);
        if path.is_file() {
            println!("cargo:rerun-if-changed={}", path.display());
            stdlib_files.push(path);
        }
    }
    stdlib_files.sort();
    let stdlib_id = fingerprint(&stdlib, &stdlib_files, b"ilang-stdlib-v1");
    let mut generated = format!(
        "pub const STDLIB_ID: &str = {stdlib_id:?};\npub const DEVELOPMENT_STDLIB: &str = {:?};\npub const STDLIB_FILES: &[(&str, &str)] = &[\n",
        stdlib.to_string_lossy(),
    );
    for path in &stdlib_files {
        let relative = path
            .strip_prefix(&stdlib)
            .expect("stdlib-relative path")
            .to_string_lossy()
            .replace('\\', "/");
        generated.push_str(&format!(
            "    ({relative:?}, include_str!({:?})),\n",
            path.to_string_lossy()
        ));
    }
    generated.push_str("];");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    fs::write(out.join("bundled_stdlib.rs"), generated).expect("write embedded standard library");

    let mut roots = vec![
        repository.join("compiler/compiler-core/src"),
        repository.join("compiler/format/src"),
        repository.join("compiler/language-server/src"),
        repository.join("compiler/Cargo.toml"),
        repository.join("compiler/Cargo.lock"),
        repository.join("Cargo.toml"),
        repository.join("Cargo.lock"),
        manifest_dir.join("build.rs"),
    ];
    let crates_directory = repository.join("crates");
    for entry in fs::read_dir(&crates_directory).expect("read workspace crates") {
        let directory = entry.expect("read workspace crate").path();
        let manifest = directory.join("Cargo.toml");
        if !manifest.is_file() {
            continue;
        }
        roots.push(manifest);
        let source = directory.join("src");
        if source.is_dir() {
            roots.push(source);
        }
    }
    let upstream_directory = repository.join("compiler");
    for entry in fs::read_dir(&upstream_directory).expect("read upstream crates") {
        let directory = entry.expect("read upstream crate").path();
        let manifest = directory.join("Cargo.toml");
        if manifest.is_file() {
            roots.push(manifest);
        }
    }
    let mut frontend_files = Vec::new();
    for path in roots {
        // Missing rerun-if-changed paths are always dirty in Cargo. New crates
        // are discovered when their workspace manifest or lockfile changes.
        if !path.exists() {
            continue;
        }
        println!("cargo:rerun-if-changed={}", path.display());
        files_under(&path, &mut frontend_files);
    }
    frontend_files.sort();
    let revision = fingerprint(&repository, &frontend_files, b"ilang-frontend-cache-abi-v1");
    println!("cargo:rustc-env=ILANG_FRONTEND_REVISION={revision}");
}
