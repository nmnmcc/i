{ config, pkgs, inputs, lib, ... }:
let
  system = pkgs.stdenv.hostPlatform.system;
  blazePackages = inputs.blaze.packages.${system};
  # Backport upstream 7145b282: std/Cargo static symbols must not depend on the
  # absolute source path of the sysroot that originally populated the cache.
  blazeDist = blazePackages.dist.overrideAttrs (old: {
    patches = (old.patches or []) ++ [ ./patches/ilang-blaze-upstream-static-path.patch ];
  });
  blazeBase = pkgs.replaceDependencies {
    drv = blazePackages.run;
    replacements = [ {
      oldDependency = blazePackages.dist;
      newDependency = blazeDist;
    } ];
  };
  blazePkgs = import inputs.blaze.inputs.nixpkgs {
    inherit system;
    overlays = [ inputs.blaze.inputs.rust-overlay.overlays.default ];
  };
  blazeRust = blazePkgs.rust-bin.nightly."2026-10-27".default.override {
    extensions = [ "rust-src" ];
    targets = [ "wasm32-wasip1" ];
  };
  blazeHost = {
    x86_64-linux = "x86_64-unknown-linux-gnu";
    aarch64-linux = "aarch64-unknown-linux-gnu";
    aarch64-darwin = "aarch64-apple-darwin";
  }.${system};
  blazeChannel = pkgs.fetchurl {
    url = "https://static.rust-lang.org/dist/2026-10-27/channel-rust-nightly.toml";
    hash = "sha256-NPPZvwRkCgguVhMQ8/JYhGOX9M9korz+vOqW6/GOJXI=";
  };
  # Blaze reads rustup's manifests to answer rustc queries itself. rust-overlay
  # omits these files, and upstream's flake also omits the proc-macro WASI target.
  # Keep the correction in a separate immutable sysroot, scoped to Blaze alone.
  blazeSysroot = pkgs.runCommand "ilang-blaze-sysroot-2026-10-27" {
    nativeBuildInputs = [ pkgs.makeWrapper ];
  } ''
    mkdir -p "$out/bin" "$out/lib/rustlib"
    for file in ${blazeRust}/*; do
      name=$(basename "$file")
      case "$name" in bin|lib) continue ;; esac
      ln -s "$file" "$out/$name"
    done
    for file in ${blazeRust}/bin/*; do
      name=$(basename "$file")
      case "$name" in rustc|rustdoc) continue ;; esac
      ln -s "$file" "$out/bin/$name"
    done
    for file in ${blazeRust}/lib/*; do
      name=$(basename "$file")
      if [ "$name" != rustlib ]; then ln -s "$file" "$out/lib/$name"; fi
    done
    for file in ${blazeRust}/lib/rustlib/*; do
      if [ "$(basename "$file")" = ${blazeHost} ]; then continue; fi
      ln -s "$file" "$out/lib/rustlib/$(basename "$file")"
    done
    cp -rsL ${blazeRust}/lib/rustlib/${blazeHost} "$out/lib/rustlib/${blazeHost}"
    ${lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
      # The pinned rust-overlay linker wrappers predate Nix's response-file
      # substitution. Complete those substitutions for current Devenv shells.
      chmod u+w "$out/lib/rustlib/${blazeHost}/bin/gcc-ld"
      for linker in "$out/lib/rustlib/${blazeHost}/bin/gcc-ld/"*; do
        if ! grep -Iq '@mktemp@' "$linker"; then continue; fi
        cp --remove-destination "$(readlink -f "$linker")" "$linker"
        chmod u+w "$linker"
        substituteInPlace "$linker" \
          --replace-warn '@mktemp@' '${pkgs.coreutils}/bin/mktemp' \
          --replace-warn '@rm@' '${pkgs.coreutils}/bin/rm'
      done
    ''}
    cp ${blazeChannel} "$out/lib/rustlib/multirust-channel-manifest.toml"
    touch "$out/lib/rustlib/manifest-rustc-${blazeHost}"
    makeWrapper ${blazeRust}/bin/rustc "$out/bin/rustc" --add-flags "--sysroot $out"
    makeWrapper ${blazeRust}/bin/rustdoc "$out/bin/rustdoc" --add-flags "--sysroot $out"
  '';
  blaze = pkgs.writeShellScriptBin "blaze" ''
    export BLAZE_HOME="''${BLAZE_HOME:-${root}/.devenv/state/blaze}"
    export RA_JIT_CACHE="$BLAZE_HOME/cache"
    export RUSTC=${blazeSysroot}/bin/rustc
    export RUSTDOC=${blazeSysroot}/bin/rustdoc
    export CARGO=${blazeSysroot}/bin/cargo
    exec ${blazeBase}/bin/blaze "$@"
  '';
  root = config.devenv.root;
  platform = {
    x86_64-linux = "x86_64-linux";
    aarch64-linux = "aarch64-linux";
    aarch64-darwin = "aarch64-macos";
  }.${pkgs.stdenv.hostPlatform.system};
  wasmTool = { name, version, archiveVersion, extension, hashes }:
    pkgs.stdenv.mkDerivation {
      pname = name;
      inherit version;
      src = pkgs.fetchurl {
        url = "https://github.com/bytecodealliance/${name}/releases/download/v${version}/${name}-${archiveVersion}-${platform}.${extension}";
        sha256 = hashes.${pkgs.stdenv.hostPlatform.system};
      };
      nativeBuildInputs = lib.optionals pkgs.stdenv.hostPlatform.isLinux [ pkgs.autoPatchelfHook ];
      buildInputs = lib.optionals pkgs.stdenv.hostPlatform.isLinux [ pkgs.stdenv.cc.cc.lib ];
      dontConfigure = true;
      dontBuild = true;
      installPhase = ''
        runHook preInstall
        install -Dm755 ${name} "$out/bin/${name}"
        runHook postInstall
      '';
      meta.mainProgram = name;
    };
  wasmtime = wasmTool {
    name = "wasmtime";
    version = "50.0.0";
    archiveVersion = "v50.0.0";
    extension = "tar.xz";
    hashes = {
      x86_64-linux = "77c19ee0437991f9ac7837b9c269d9c7076744f85dad263fea5214cc2723c7ff";
      aarch64-linux = "10f9ad1f8539d2e04e041cc622de6a270109ab1e43a6be1f69843c2082166832";
      aarch64-darwin = "323dc098dc0455d60f396c404627e967b7033ffcfa29abfffb243126cf82c9d6";
    };
  };
  wasm-tools = wasmTool {
    name = "wasm-tools";
    version = "1.262.0";
    archiveVersion = "1.262.0";
    extension = "tar.gz";
    hashes = {
      x86_64-linux = "655b8df67d54c3ef9b52fc237c4e94bbfbb5f5f43825245cb0940a7002a9ed65";
      aarch64-linux = "729dc47a114305ed60873ce6199d403eed1da89c3aa2367eef292da358b1e8d3";
      aarch64-darwin = "801b237643cc0f4ab27ebd1745e7863e23bf194a038bc6ca8ce5618212e43669";
    };
  };
in
{
  languages.rust = {
    enable = true;
    toolchainFile = ./rust-toolchain.toml;
  };

  packages = [
    blaze
    pkgs.git
    pkgs.stdenv.cc
    # Builds the vendored Binaryen in crates/ilang-binaryen.
    pkgs.cmake
    pkgs.pkg-config
    pkgs.watchexec
    wasmtime
    wasm-tools
    pkgs.nodejs_24
    pkgs.python3
    pkgs.hyperfine
    pkgs.time
    pkgs.playwright-driver.browsers
  ];

  env = {
    BLAZE_HOME = "${config.devenv.state}/blaze";
    ILANG_ROOT = root;
    ILANG_CARGO_TARGET_DIR = "${root}/target/cargo";
    ILANG_BLAZE = "${blaze}/bin/blaze";
    ILANG_BLAZE_BACKPORT = "7145b282f6c3adccee8f24b200cb0270eba477fc";
    ILANG_WASMTIME = "${wasmtime}/bin/wasmtime";
    ILANG_WASM_TOOLS = "${wasm-tools}/bin/wasm-tools";
    PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
    ILANG_PLAYWRIGHT_VERSION = pkgs.playwright-driver.version;
    PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD = "1";
  } // lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
    # WebKit's headless WPE backend still needs an EGL vendor on machines
    # without /run/opengl-driver. Mesa supplies software rendering here.
    __EGL_VENDOR_LIBRARY_FILENAMES = "${pkgs.mesa}/share/glvnd/egl_vendor.d/50_mesa.json";
  };

  scripts.i-dev = {
    description = "Run the development CLI with Blaze, forwarding all arguments";
    exec = ''
      exec "${root}/scripts/dev-run" "$@"
    '';
  };

  tasks = {
    "dev:build" = {
      description = "Build the compiler and language server with Blaze";
      exec = "${root}/scripts/dev-build";
    };
    "dev:test" = {
      description = "Run Rust development tests with Blaze";
      exec = "${root}/scripts/dev-test";
    };
    "dev:watch" = {
      description = "Run development tests serially after source changes";
      exec = "${root}/scripts/dev-watch";
    };
    "dev:status" = {
      description = "Inspect this project's Blaze daemon";
      exec = "blaze status";
    };
    "dev:stop" = {
      description = "Stop this project's Blaze daemon";
      exec = "blaze stop";
    };
    "verify:all" = {
      description = "Verify with the pinned Rust toolchain";
      exec = "${root}/scripts/dev-verify";
    };
    "release:all" = {
      description = "Build all release artifacts with the pinned Rust toolchain";
      exec = "${root}/scripts/dev-release";
    };
    "tools:versions" = {
      description = "Print the pinned developer tool versions";
      exec = "${root}/scripts/toolchain-versions";
    };
    "tools:blaze-smoke" = {
      description = "Check Blaze build scripts, proc macros, snapshots, and package selection";
      exec = "${root}/scripts/toolchain-blaze-smoke";
    };
  };

  enterTest = ''
    "${root}/scripts/dev-verify"
  '';
}
