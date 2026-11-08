//! Build the vendored Binaryen (a git submodule at `binaryen/`) as a static
//! library, so the `i` binary needs nothing else installed.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=binaryen/CMakeLists.txt");
    println!("cargo:rerun-if-changed=binaryen/src/binaryen-c.h");

    if !std::path::Path::new("binaryen/CMakeLists.txt").exists() {
        panic!(
            "the Binaryen sources are missing; run `git submodule update --init crates/ilang-binaryen/binaryen`"
        );
    }

    // Binaryen is always built with optimizations: a debug build of the
    // optimizer is many times slower and would make every test slow.
    let out = cmake::Config::new("binaryen")
        .profile("Release")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("BUILD_TESTS", "OFF")
        .define("BUILD_TOOLS", "OFF")
        .define("BUILD_LLVM_DWARF", "ON")
        .define("BUILD_MIMALLOC", "OFF")
        .define("ENABLE_WERROR", "OFF")
        .define("BYN_ENABLE_LTO", "OFF")
        .build();

    println!("cargo:rustc-link-search=native={}/lib", out.display());
    println!("cargo:rustc-link-search=native={}/lib64", out.display());
    println!("cargo:rustc-link-lib=static=binaryen");

    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("apple") {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else if !target.contains("msvc") {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }
}
