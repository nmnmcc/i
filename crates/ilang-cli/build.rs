//! Build the native runner (crates/ilang-runner) that `i build` copies into
//! every native package, which is a prebuilt runner plus the compiled program.
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

    // Embed the standard library sources so that `i new` can copy the `std`
    // package into a new package. It stays an ordinary path dependency: the
    // compiler never resolves it specially.
    let std_dir = root.join("std");
    println!(
        "cargo:rerun-if-changed={}",
        std_dir.join("i.toml").display()
    );
    println!("cargo:rerun-if-changed={}", std_dir.join("src").display());
    let mut sources = vec![("i.toml".to_string(), std_dir.join("i.toml"))];
    fn collect(dir: &std::path::Path, prefix: &str, out: &mut Vec<(String, std::path::PathBuf)>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap_or_else(|error| panic!("reading {}: {error}", dir.display()))
            .map(|entry| entry.expect("reading a directory entry").path())
            .collect();
        entries.sort();
        for path in entries {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("std source names are UTF-8");
            let relative = format!("{prefix}/{name}");
            if path.is_dir() {
                collect(&path, &relative, out);
            } else {
                out.push((relative, path));
            }
        }
    }
    collect(&std_dir.join("src"), "src", &mut sources);
    let mut generated = String::from(
        "// Generated by build.rs from the `std` package; copied by `i new`.\n\
         pub const STD_FILES: &[(&str, &str)] = &[\n",
    );
    for (relative, path) in &sources {
        generated.push_str(&format!(
            "    ({relative:?}, include_str!({:?})),\n",
            path.display().to_string()
        ));
    }
    generated.push_str("];\n");
    std::fs::write(out.join("std_package.rs"), generated)
        .expect("writing the embedded std sources");
}
