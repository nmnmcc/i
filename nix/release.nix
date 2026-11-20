# Prebuilt archives installed by `packages.<system>.i-bin`.
#
# Every entry is filled in by hand after the `release` workflow has uploaded
# the archive for that platform: copy the `url` and the SRI `hash` from the
# run's summary (or its `i-bin-fragment.nix` artifact) here. `version` must be
# the version in Cargo.toml; `nix flake check` runs both binaries and fails if
# the archive is stale.
{
  artifacts = {
    x86_64-linux = {
      version = "0.1.0";
      url = "https://github.com/nmnmcc/i/releases/download/v0.1.0/i-0.1.0-x86_64-linux.tar.gz";
      hash = "sha256-1UDl/CXvG+xti/sS5ganQm3bXC6kg3mZhmveBxT/oHs=";
    };
    aarch64-linux = {
      version = "0.1.0";
      url = "https://github.com/nmnmcc/i/releases/download/v0.1.0/i-0.1.0-aarch64-linux.tar.gz";
      hash = "sha256-INX7nPgZ/WvCAW4+5zVoKLv3O1D8s6Y/VIIdwLsVmY8=";
    };
    aarch64-darwin = {
      version = "0.1.0";
      url = "https://github.com/nmnmcc/i/releases/download/v0.1.0/i-0.1.0-aarch64-darwin.tar.gz";
      hash = "sha256-z5HTuOTosNE7uZCxS2No6KzfR5JtT4F49Ven8sIOyUc=";
    };
  };
}
