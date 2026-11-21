# `i-bin`, installed from a prebuilt release archive. The archive is
# `i-<version>-<system>.tar.gz` and contains `bin/i`; see `nix/package.py`.
#
# `artifact` is `null` until the first archive for a platform is published
# (and recorded in `nix/release.nix`). CI overrides it with the archive of the
# current run (`...i-bin.override { artifact = ...; }`), so this path is tested
# before any release exists.
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
  assert lib.assertMsg (artifact ? version)
    "i-bin: the nix/release.nix entry for ${pkgs.stdenv.hostPlatform.system} has no version";
  pkgs.stdenv.mkDerivation {
    pname = "i-bin";
    inherit version;

    src = pkgs.fetchurl {
      inherit (artifact) url hash;
    };

    # Archives from the next release on are static on Linux: no interpreter,
    # no RPATH, nothing to rewrite. The `v0.1.0` archives that `nix/release.nix`
    # still pins are glibc binaries whose ELF interpreter and RPATH name the
    # release machine's store, and rewriting them against this machine's
    # libraries (or the Mach-O install names) is what makes those installable
    # from another store.
    nativeBuildInputs =
      lib.optionals pkgs.stdenv.hostPlatform.isLinux [ pkgs.autoPatchelfHook ]
      ++ lib.optionals pkgs.stdenv.hostPlatform.isDarwin [ pkgs.fixDarwinDylibNames ];

    buildInputs =
      lib.optionals pkgs.stdenv.hostPlatform.isLinux [
        pkgs.glibc
        pkgs.zlib
        pkgs.stdenv.cc.cc.lib
      ]
      ++ lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
        pkgs.libcxx
      ];

    unpackPhase = ''
      mkdir source
      tar -xzf "$src" -C source
      cd source
    '';

    installPhase = ''
      runHook preInstall
      install -Dm755 bin/i "$out/bin/i"
      runHook postInstall
    '';

    # Runs after autoPatchelf/fixDarwinDylibNames. The archive must be the
    # version the flake says it is, and it must run on this machine; this is
    # what a stale release entry would break.
    postFixup = ''
      test "$("$out/bin/i" --version)" = "i ${version}" || {
        echo "i-bin: the archive from ${artifact.url} does not report i ${version}" >&2
        exit 1
      }
    '';

    meta = {
      description = "The i programming language compiler, prebuilt";
      homepage = "https://i.nmnm.cc";
      license = lib.licenses.asl20;
      mainProgram = "i";
      platforms = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
    };
  }
