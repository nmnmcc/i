{
  description = "The i programming language";

  # `i` builds the compiler from the source in this flake. `i-bin` installs a
  # prebuilt archive published for the same version, so it needs no C++
  # toolchain, no CMake and no Binaryen submodule.
  #
  # Adding a platform is a change of data, not of the build logic:
  #   1. add the system to `systems` below;
  #   2. add its runner to the release matrix (the NIX_RELEASE_MATRIX
  #      repository variable) and to the CI job for that runner;
  #   3. after its first archive is published, add its url and hash to
  #      `nix/release.nix`.
  inputs = {
    # nixpkgs is the package set; rust-overlay supplies exactly the toolchain
    # that rust-toolchain.toml pins (Nixpkgs' Rust is usually older).
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      inherit (nixpkgs) lib;

      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];

      forAllSystems = f: lib.genAttrs systems (system: f system);

      pkgsFor = system: import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };

      # The version in Cargo.toml is the version the compiler reports and the
      # version every release archive is named after.
      version = (lib.importTOML ./Cargo.toml).workspace.package.version;

      release = import ./nix/release.nix;

      # The toolchain is the one rust-toolchain.toml pins, plus the musl
      # target the nested runner build needs on the host architecture: the
      # file can only name one architecture's target. Everything else
      # (channel, profile, components) comes from the file.
      rustToolchainFor = pkgs:
        let
          pinned = (lib.importTOML ./rust-toolchain.toml).toolchain;
          muslTarget =
            "${pkgs.stdenv.hostPlatform.parsed.cpu.name}-unknown-linux-musl";
        in
        pkgs.rust-bin.fromRustupToolchain {
          inherit (pinned) channel profile components;
          targets = lib.unique (
            (pinned.targets or [ ])
            ++ lib.optional pkgs.stdenv.hostPlatform.isLinux muslTarget
          );
        };

      mkI = system:
        let
          pkgs = pkgsFor system;
          rustToolchain = rustToolchainFor pkgs;
        in pkgs.callPackage ./nix/i.nix {
          inherit version rustToolchain;
          rustPlatform = pkgs.makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };
        };

      # `artifact` is what CI overrides to test an archive from the current
      # run: `(packages.x86_64-linux.i-bin).override { artifact = { ... }; }`.
      mkIBin = system: let pkgs = pkgsFor system;
      in pkgs.callPackage ./nix/i-bin.nix {
        inherit version;
        artifact = release.artifacts.${system} or null;
      };
    in {
      packages = forAllSystems (system:
        let i = mkI system;
        in {
          i = i;
          i-bin = mkIBin system;
          default = i;
        });

      apps = forAllSystems (system: {
        i = {
          type = "app";
          program = "${self.packages.${system}.i}/bin/i";
          meta.description = "The i compiler, built from source";
        };
        i-bin = {
          type = "app";
          program = "${self.packages.${system}.i-bin}/bin/i";
          meta.description = "The i compiler, from a prebuilt release archive";
        };
        default = self.apps.${system}.i;
      });

      checks = forAllSystems (system:
        let
          pkgs = pkgsFor system;
          i = self.packages.${system}.i;
          iBin = self.packages.${system}.i-bin;
          pinned = lib.hasAttr system release.artifacts;
        in {
          i-runs = pkgs.runCommand "i-runs-${version}" {
            meta.description = "i reports its version and runs a new package";
          } ''
            set -eu
            export HOME="$TMPDIR/home"
            mkdir -p "$HOME"
            cd "$TMPDIR"

            test "$(${i}/bin/i --version)" = "i ${version}"

            ${i}/bin/i new demo
            test "$(${i}/bin/i run --project demo)" = "Hello, demo!"
            touch "$out"
          '';
        } // lib.optionalAttrs pinned {
          i-bin-runs = pkgs.runCommand "i-bin-runs-${version}" {
            meta.description = "the prebuilt i reports the same version";
          } ''
            set -eu
            test "$(${iBin}/bin/i --version)" = "i ${version}"
            touch "$out"
          '';

          # The archive is named after a version; without running both, a
          # stale archive would silently install an older compiler.
          versions-match = pkgs.runCommand "i-versions-match-${version}" {
            meta.description = "i and i-bin report the same version";
          } ''
            set -eu
            from_source="$(${i}/bin/i --version | awk '{print $2}')"
            from_archive="$(${iBin}/bin/i --version | awk '{print $2}')"
            if test "$from_source" != "$from_archive"; then
              echo "i is $from_source but i-bin is $from_archive" >&2
              exit 1
            fi
            touch "$out"
          '';
        });
    };
}
