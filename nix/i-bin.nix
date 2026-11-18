# `i-bin`, installed from a prebuilt release archive. The archive is
# `i-<version>-<system>.tar.gz` and contains `bin/i`; see `nix/package.py`.
#
# `artifact` is `null` until the first archive for a platform is published
# (and recorded in `nix/release.nix`). CI passes an archive from the current
# run through `.override { artifact = ...; }`, so this path is tested before
# any release exists.
{ pkgs, version, artifact }:

let
  inherit (pkgs) lib;
in
if artifact == null then
  pkgs.runCommand "i-bin-unavailable-${version}" {
    meta = {
      description = "i-bin, unavailable until a release archive is pinned";
    };
  } ''
    echo "i-bin: no prebuilt archive is pinned for ${pkgs.stdenv.hostPlatform.system} at version ${version}." >&2
    echo "i-bin: add its url and SRI hash to nix/release.nix, or build .#i from source." >&2
    exit 1
  ''
else
  pkgs.stdenv.mkDerivation {
    pname = "i-bin";
    inherit version;

    src =
      if artifact ? path then
        artifact.path
      else
        pkgs.fetchurl {
          inherit (artifact) url hash;
        };

    # A release archive is built by Nix on the machine of the release
    # workflow, so its ELF interpreter and RPATH name that machine's store.
    # Rewriting them against this machine's libraries is what makes the same
    # archive installable from any store.
    nativeBuildInputs =
      lib.optionals pkgs.stdenv.isLinux [ pkgs.autoPatchelfHook ]
      ++ lib.optionals pkgs.stdenv.isDarwin [ pkgs.fixDarwinDylibNames ];

    buildInputs =
      lib.optionals pkgs.stdenv.isLinux [
        pkgs.glibc
        pkgs.zlib
        pkgs.stdenv.cc.cc.lib
      ]
      ++ lib.optionals pkgs.stdenv.isDarwin [
        pkgs.libcxx
        pkgs.libcxxabi
      ];

    unpackPhase = ''
      mkdir source
      tar -xzf "$src" -C source
      cd source
    '';

    installPhase = ''
      runHook preInstall
      # The archive is named after a version. A fragment copied from a
      # different release must not install here as if it were this version.
      test "${artifact.version or version}" = "${version}" || {
        echo "i-bin: archive is for ${artifact.version or version}, flake is ${version}" >&2
        exit 1
      }
      install -Dm755 bin/i "$out/bin/i"
      runHook postInstall
    '';

    meta = {
      description = "The i programming language compiler, prebuilt";
      homepage = "https://i.nmnm.cc";
      license = lib.licenses.asl20;
      mainProgram = "i";
      platforms = [ "x86_64-linux" "aarch64-darwin" ];
    };
  }
