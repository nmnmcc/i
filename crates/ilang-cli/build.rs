//! Build the native runner (crates/ilang-runner) that `i build` copies into
//! every native package (docs/design/native.md, N2).
//!
//! The runner is built by a separate Cargo invocation of its own package,
//! into `target/runner`, so that Cargo's feature unification cannot give it
//! the compiler that `i` itself has: the runner's Wasmtime has no Cranelift.
//! The same `ilang-host` source, Cargo.lock and profile build both, so they
//! use one Wasmtime version and engine configuration.
//!
//! The runner is statically linked, so that a package depends on no library
//! of the building machine: on Linux it is built for the musl target of the
//! same architecture (`<arch>-unknown-linux-musl`, whose standard library
//! links musl statically, and whose `getaddrinfo` reads /etc/hosts and
//! /etc/resolv.conf itself, without glibc's NSS modules). Elsewhere (macOS)
//! it is built for the target of `i` and depends only on system libraries.

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
    let runner_target = if env::var("CARGO_CFG_TARGET_OS").unwrap() == "linux" {
        format!(
            "{}-unknown-linux-musl",
            env::var("CARGO_CFG_TARGET_ARCH").unwrap()
        )
    } else {
        env::var("TARGET").unwrap()
    };
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
        .arg("--target")
        .arg(&runner_target)
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
                // `cargo clippy` lints the runner's crates itself; the
                // nested build only compiles them.
                "RUSTC_WORKSPACE_WRAPPER",
                "CLIPPY_ARGS",
                "CLIPPY_CONF_DIR",
            ]
            .contains(&key.as_ref())
        {
            command.env_remove(key.as_ref());
        }
    }
    let status = command.status().expect("running cargo to build the runner");
    assert!(status.success(), "building the native runner failed");
    let built = target_dir
        .join(&runner_target)
        .join(if release { "release" } else { "debug" })
        .join(format!("i-runner{}", env::consts::EXE_SUFFIX));
    let copy = out.join("i-runner");
    std::fs::copy(&built, &copy).expect("copying the runner");
    println!("cargo:rustc-env=ILANG_RUNNER={}", copy.display());
    println!("cargo:rustc-env=ILANG_RUNNER_TARGET={runner_target}");
}
