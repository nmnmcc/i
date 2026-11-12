//! Build the native runner (crates/ilang-runner) that `i build` copies into
//! every native package (docs/design/native.md, N2).
//!
//! The runner is built by a separate Cargo invocation of its own package,
//! into `target/runner`, so that Cargo's feature unification cannot give it
//! the compiler that `i` itself has: the runner's Wasmtime has no Cranelift.
//! The same `ilang-host` source, Cargo.lock and profile build both, so they
//! use one Wasmtime version, target triple and engine configuration.

use std::{env, path::PathBuf, process::Command};

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest_dir.join("../..");
    for path in [
        "crates/ilang-runner",
        "crates/ilang-host",
        "crates/ilang-syntax",
        "Cargo.lock",
        "Cargo.toml",
    ] {
        println!("cargo:rerun-if-changed={}", root.join(path).display());
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    // OUT_DIR is <target>/<profile>/build/ilang-cli-<hash>/out.
    let target_dir = out
        .ancestors()
        .nth(4)
        .expect("OUT_DIR is inside the target directory")
        .join("runner");
    let release = env::var("PROFILE").unwrap() == "release";
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command
        .current_dir(&root)
        .args([
            "build",
            "--locked",
            "-p",
            "ilang-runner",
            "--bin",
            "i-runner",
        ])
        .arg("--target-dir")
        .arg(&target_dir);
    if release {
        command.arg("--release");
    }
    // The variables that Cargo sets for this build script describe this
    // package, not the runner.
    for (key, _) in env::vars_os() {
        let key = key.to_string_lossy();
        if [
            "CARGO_FEATURE_",
            "CARGO_CFG_",
            "CARGO_PKG_",
            "CARGO_MANIFEST_",
        ]
        .iter()
        .any(|prefix| key.starts_with(prefix))
            || [
                "OUT_DIR",
                "TARGET",
                "HOST",
                "PROFILE",
                "OPT_LEVEL",
                "DEBUG",
                "NUM_JOBS",
                "RUSTC_LINKER",
                "CARGO_PRIMARY_PACKAGE",
                "CARGO_CRATE_NAME",
                "CARGO_TARGET_DIR",
            ]
            .contains(&key.as_ref())
        {
            command.env_remove(key.as_ref());
        }
    }
    let status = command.status().expect("running cargo to build the runner");
    assert!(status.success(), "building the native runner failed");
    let built = target_dir
        .join(if release { "release" } else { "debug" })
        .join(format!("i-runner{}", env::consts::EXE_SUFFIX));
    let copy = out.join("i-runner");
    std::fs::copy(&built, &copy).expect("copying the runner");
    println!("cargo:rustc-env=ILANG_RUNNER={}", copy.display());
}
