# `i`, built from source. Binaryen (a C++ library) is built by the build
# script of `crates/ilang-binaryen` with CMake and linked in statically; the
# native runner embedded in `i` is built for `<arch>-unknown-linux-musl`
# (on Linux) so that `i build` copies out a statically linked executable.
#
# On Linux the compiler itself is built by the same rule, with the same
# musl cross compiler: it is a static `<arch>-unknown-linux-musl` binary. A
# glibc `i` names a `/nix/store/.../ld-linux` as its ELF interpreter and
# cannot start on a machine without the same store, so an archive of it was
# not a portable binary; the musl build has no interpreter, no RPATH and no
# shared library.
{
  lib,
  pkgs,
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

  # The musl compiler builds everything that ends up in the binary: `i`
  # itself, Binaryen and the nested `cargo build` of `ilang-runner` (see
  # `crates/ilang-cli/build.rs`). It is passed through the target variables
  # below rather than added to nativeBuildInputs: its setup hook would change
  # how the host side is built. The cross package set is one data entry per
  # architecture, so an aarch64 host cross-compiles to musl too.
  arch = stdenv.hostPlatform.parsed.cpu.name;
  muslCross = {
    x86_64 = pkgsCross.musl64;
    aarch64 = pkgsCross.aarch64-multiplatform-musl;
  }.${arch} or (throw "nix/i.nix: no musl cross compiler configured for ${arch}-linux");
  muslCC = muslCross.stdenv.cc;
  muslTarget = "${arch}-unknown-linux-musl";
  muslTargetUpper = lib.toUpper (lib.replaceStrings [ "-" ] [ "_" ] muslTarget);

  muslCCBin = "${muslCC}/bin/${muslCC.targetPrefix}";
  muslCCLib = "${muslCC.cc}/${muslTarget}/lib";

  # The `cmake` crate passes `CMAKE_TOOLCHAIN_FILE` to CMake, which is where
  # the compiler has to come from: the stdenv hook owns `CC` and `PATH`, and
  # CMake reads neither `CMAKE_C_COMPILER` from the environment nor the
  # `CC_<target>` names. A toolchain file also tells CMake it is cross
  # compiling, so its compiler probes are not run.
  muslToolchain = pkgs.writeText "musl-toolchain.cmake" ''
    set(CMAKE_SYSTEM_NAME Linux)
    set(CMAKE_C_COMPILER "${muslCCBin}cc")
    set(CMAKE_CXX_COMPILER "${muslCCBin}c++")
    set(CMAKE_AR "${muslCCBin}ar")
    set(CMAKE_RANLIB "${muslCCBin}ranlib")
  '';
  # The linker is the musl driver with `-static` first: rustc appends user
  # link arguments after its own `-lc`, so `-static` there would come too late
  # and the link would pick `libc.so`. `--no-dynamic-linker` keeps `PT_INTERP`
  # out of the header. Build scripts and proc macros are host code and do not
  # go through this.
  muslStaticCC = pkgs.writeShellScript "musl-static-cc" ''
    exec ${muslCCBin}cc -static -Wl,--no-dynamic-linker -L ${muslCCLib} "$@"
  '';

  muslEnv = lib.optionalAttrs stdenv.hostPlatform.isLinux {
    "CARGO_TARGET_${muslTargetUpper}_LINKER" = muslStaticCC;
    # Crates that compile C through the `cc` crate look for `CC`/`CXX` suffixed
    # with the upper-cased target before the binaries on `PATH`, and the stdenv
    # hook overwrites the bare names; the suffixed ones are what selects the
    # musl compiler.
    "CC_${muslTargetUpper}" = "${muslCCBin}cc";
    "CXX_${muslTargetUpper}" = "${muslCCBin}c++";
    "AR_${muslTargetUpper}" = "${muslCCBin}ar";
    "RANLIB_${muslTargetUpper}" = "${muslCCBin}ranlib";
    CC = "${muslCCBin}cc";
    CXX = "${muslCCBin}c++";
    AR = "${muslCCBin}ar";
    RANLIB = "${muslCCBin}ranlib";
    CMAKE_TOOLCHAIN_FILE = muslToolchain;
  };
in
rustPlatform.buildRustPackage {
  pname = "i";
  inherit version;

  # Only what the build reads: the workspace, the built-in `lib` and the `stdlib` package
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
      ../stdlib
      ../docs/spec.md
    ];
  };

  cargoLock.lockFile = ../Cargo.lock;

  # `buildRustPackage` defaults this to true, and cargo-auditable is then built
  # with nixpkgs' own rustc, which builds LLVM from source: a cold release
  # build took hours for a `.dep-v0` section that `cargo audit` can read from
  # the lockfile anyway. The lockfile audit runs in CI at release time. See
  # docs/development.md before turning this back on, and keep the tag and PR
  # build paths identical if you do.
  auditable = false;

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

  # The compiler is built for the musl target on Linux; the runner it
  # embeds was built for the same one by the build script.
  cargoBuildFlags = [ "-p" "ilang-cli" "--bin" "i" ]
    ++ lib.optionals stdenv.hostPlatform.isLinux [ "--target" muslTarget ];

  # The workspace test suite is the CI workflow's job; this derivation only
  # has to produce the compiler.
  doCheck = false;

  # The binary is under the target directory asked for above. The release
  # profile is the one in Cargo.toml (thin LTO, one codegen unit).
  installPhase = ''
    runHook preInstall
    binary="target/${muslTarget}/release/i"
    if [ ! -e "$binary" ]; then
      binary="target/release/i"
    fi
    install -Dm755 "$binary" "$out/bin/i"
    # The library package travels with the distribution, so that `i new`
    # can build a local repository for offline use.
    mkdir -p "$out/share/i"
    cp -a stdlib "$out/share/i/stdlib"
    runHook postInstall
  '';

  # Rust's precompiled std embeds its source paths, which point into the
  # `rust-src` of the pinned toolchain. They are only panic messages, but Nix
  # counts them as references, so without this the i binary would pull the
  # whole 1.7 GB toolchain at install time.
  postFixup = ''
    remove-references-to -t ${rustToolchain} "$out/bin/i"
  '' + lib.optionalString stdenv.hostPlatform.isDarwin ''
    # nixpkgs' stdenv links libiconv from the store on Darwin. macOS ships
    # the same ABI as a system library, and a single /nix/store dependency
    # would make the release archive unusable on a machine that does not have
    # that path. Rewrite it, then sign the binary again (arm64 needs this).
    for dep in $(otool -L "$out/bin/i" | awk '/\/nix\/store\/.*\/libiconv[^ ]*\.dylib/ {print $1}'); do
      install_name_tool -change "$dep" /usr/lib/libiconv.2.dylib "$out/bin/i"
    done
    if otool -L "$out/bin/i" | tail -n +2 | grep -q '/nix/store'; then
      echo "i still links a Nix store library:" >&2
      otool -L "$out/bin/i" >&2
      exit 1
    fi
    /usr/bin/codesign --force --sign - "$out/bin/i"
  '';

  # The nested build of the runner inherits this. nixpkgs sets the Cargo strip
  # setting to false for the outer build; this keeps the runner like a release
  # `cargo build` makes it.
  #
  # Build scripts and proc macros are host binaries and must keep the host
  # (glibc) linkage, so the static settings are scoped to the musl target.
  # `+crt-static` is that target's default and is spelled out so a changed
  # default cannot bring the dynamic loader back; the C++ runtime is linked
  # from its `.a` by `-static` in `muslStaticCC` above.
  RUSTFLAGS = "-C strip=debuginfo"
    + lib.optionalString stdenv.hostPlatform.isLinux " -L native=${muslCCLib}";
  env = muslEnv // lib.optionalAttrs stdenv.hostPlatform.isLinux {
    "CARGO_TARGET_${muslTargetUpper}_RUSTFLAGS" =
      "-C strip=debuginfo -C target-feature=+crt-static -C link-arg=-Wl,--no-dynamic-linker";
  };

  meta = {
    description = "The i programming language compiler";
    homepage = "https://i.nmnm.cc";
    license = lib.licenses.asl20;
    mainProgram = "i";
    platforms = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
  };
}
