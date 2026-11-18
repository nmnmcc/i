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
rustPlatform.buildRustPackage {
  pname = "i";
  inherit version;

  src = ../.;

  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [ cmake ];

  # Binaryen is a submodule in a checkout; a flake source has only the
  # gitlink, so its pinned archive takes its place.
  postPatch = ''
    rm -rf crates/ilang-binaryen/binaryen
    cp -a ${binaryenSrc} crates/ilang-binaryen/binaryen
  '';

  cargoBuildFlags = [ "-p" "ilang-cli" "--bin" "i" ];

  # The workspace test suite is the CI workflow's job; this derivation only
  # has to produce the compiler.
  doCheck = false;

  installPhase = ''
    runHook preInstall
    install -Dm755 target/release/i "$out/bin/i"
    runHook postInstall
  '';

  env = muslEnv;

  meta = {
    description = "The i programming language compiler";
    homepage = "https://i.nmnm.cc";
    license = lib.licenses.asl20;
    mainProgram = "i";
    platforms = [ "x86_64-linux" "aarch64-darwin" ];
  };
}
