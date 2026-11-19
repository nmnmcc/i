# Prebuilt archives installed by `packages.<system>.i-bin`.
#
# Every entry is filled in once, by hand, after the `release` workflow has
# uploaded the archive for that platform: copy the `url` and the SRI `hash`
# from the run's summary (or its `i-bin-fragment.nix` artifact) here. Until
# then `i-bin` is a broken package that says which line is missing, and
# `nix build .#i` (from source) still works.
#
# `version` must be the version in Cargo.toml; `nix flake check` runs both
# binaries and fails if the archive is stale.
{
  artifacts = {
    # One entry per platform; the key is the system in `flake.nix`.
    # x86_64-linux = {
    #   version = "0.1.0";
    #   url = "https://github.com/nmnmcc/i/releases/download/v0.1.0/i-0.1.0-x86_64-linux.tar.gz";
    #   hash = "sha256-...=";
    # };
    # aarch64-linux = {
    #   version = "0.1.0";
    #   url = "https://github.com/nmnmcc/i/releases/download/v0.1.0/i-0.1.0-aarch64-linux.tar.gz";
    #   hash = "sha256-...=";
    # };
    # aarch64-darwin = {
    #   version = "0.1.0";
    #   url = "https://github.com/nmnmcc/i/releases/download/v0.1.0/i-0.1.0-aarch64-darwin.tar.gz";
    #   hash = "sha256-...=";
    # };
  };
}
