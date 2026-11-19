# `i`, built from source. Binaryen (a C++ library) is built by the build
# script of `crates/ilang-binaryen` with CMake and linked in statically; the
# native runner embedded in `i` is built for `<arch>-unknown-linux-musl`
# (on Linux) so that `i build` copies out a statically linked executable.
{
  lib,
  stdenv,
  rustPlatform,
  rustToolchain,
  cmake,
  fetchzip,
  removeReferencesTo,
  pkgsCross,
  version,
  binaryenArchive ? {
    url = "https://github.com/WebAssembly/binaryen/archive/fba5005a132d85d578be5bd010e0487ed8e2ce3e.tar.gz";
    # Hash of the unpacked archive (nix-prefetch-url --unpack). The commit is
    # the one in the `crates/ilang-binaryen/binaryen` gitlink; the archive is
    # fetched rather than the submodule so that `nix build github:nmnmcc/i`
    # works without `submodules = true`, and so that the C++ source is pinned
    # in the flake instead of in a mutable submodule.
    hash = "sha256-ip10rXDnBfgSLrc4q8TXGckGnyq4SSoXQKoCU/KQIE8=";
  },
}:

let
  binaryenSrc = fetchzip {
    inherit (binaryenArchive) url;
    hash = binaryenArchive.hash;
  };

  # The musl compiler is used only for the nested `cargo build` of
  # `ilang-runner` (see `crates/ilang-cli/build.rs`). It is not added to
  # nativeBuildInputs: its setup hook would change how `i` itself and Binaryen
  # are built.
  muslCC = pkgsCross.musl64.stdenv.cc;
  muslPrefix = muslCC.targetPrefix;

  muslEnv = lib.optionalAttrs stdenv.hostPlatform.isLinux {
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER = "${muslCC}/bin/${muslPrefix}cc";
    CC_x86_64_unknown_linux_musl = "${muslCC}/bin/${muslPrefix}cc";
    CXX_x86_64_unknown_linux_musl = "${muslCC}/bin/${muslPrefix}c++";
    AR_x86_64_unknown_linux_musl = "${muslCC}/bin/${muslPrefix}ar";
    RANLIB_x86_64_unknown_linux_musl = "${muslCC}/bin/${muslPrefix}ranlib";
  };
in
assert lib.assertMsg (!stdenv.hostPlatform.isLinux || stdenv.hostPlatform.isx86_64)
  "nix/i.nix: the nested musl runner build is wired for x86_64 Linux; add the aarch64 musl cross compiler before enabling aarch64-linux";
rustPlatform.buildRustPackage {
  pname = "i";
  inherit version;

  # Only what the build reads: the workspace, the embedded `lib` and `std`
  # sources, and the spec that `i spec` embeds. A README or CI change does not
  # rebuild the compiler.
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../rust-toolchain.toml
      ../crates
      ../lib
      ../std
      ../docs/spec.md
    ];
  };

  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [ cmake removeReferencesTo ];
  # `cmake` is for the Binaryen build script, not for `configurePhase`.
  dontUseCmakeConfigure = true;

  # Binaryen is a submodule in a checkout; a flake source has only the
  # gitlink, so its pinned archive takes its place.
  postPatch = ''
    rm -rf crates/ilang-binaryen/binaryen
    cp -a ${binaryenSrc} crates/ilang-binaryen/binaryen
    chmod -R u+w crates/ilang-binaryen/binaryen
  '';

  cargoBuildFlags = [ "-p" "ilang-cli" "--bin" "i" ];

  # The workspace test suite is the CI workflow's job; this derivation only
  # has to produce the compiler.
  doCheck = false;

  # The nixpkgs Cargo hook passes `--target`, so the binary is under the
  # target's directory; the fallback keeps this working without it. The
  # release profile is the one in Cargo.toml (thin LTO, one codegen unit).
  installPhase = ''
    runHook preInstall
    binary="target/${stdenv.hostPlatform.rust.cargoShortTarget}/release/i"
    if [ ! -e "$binary" ]; then
      binary="target/release/i"
    fi
    install -Dm755 "$binary" "$out/bin/i"
    runHook postInstall
  '';

  # Rust's precompiled std embeds its source paths, which point into the
  # `rust-src` of the pinned toolchain. They are only panic messages, but Nix
  # counts them as references, so without this the i binary would pull the
  # whole 1.7 GB toolchain at install time.
  postFixup = ''
    remove-references-to -t ${rustToolchain} "$out/bin/i"
  '';

  # The nested build of the musl runner inherits this. nixpkgs sets the Cargo
  # strip setting to false for the outer build; this keeps the runner like a
  # release `cargo build` makes it.
  RUSTFLAGS = "-C strip=debuginfo";

  env = muslEnv;

  meta = {
    description = "The i programming language compiler";
    homepage = "https://i.nmnm.cc";
    license = lib.licenses.asl20;
    mainProgram = "i";
    platforms = [ "x86_64-linux" ];
  };
}
