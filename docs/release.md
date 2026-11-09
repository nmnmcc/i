# Release packages

Run `devenv tasks run release` to build with Rust 1.99.1 and produce the archive
and its SHA-256 sidecar in `target/release`. The archive name is
`ilang-<version>-<Rust target triple>.tar.gz`.

To package artifacts that have already been built, use:

```sh
./scripts/dev-package
# Explicit cross-compiled artifact directory and target:
./scripts/dev-package --binaries target/cargo/aarch64-unknown-linux-gnu/release \
  --target aarch64-unknown-linux-gnu
# Include runtime libraries for transfer to another Nix host:
./scripts/dev-package --nix-runtime
```

The archive contains `bin/i`, which has the compiler, Binaryen, the library
modules, the Wasmtime host, the package commands, the formatter and the
language server built in, and the JavaScript SDK under
`share/ilang/javascript`. Nothing else is needed to build for either target:
`i build --target wasi` makes a module that a stock `wasmtime run` or Node's
`node:wasi` runs. The VS Code extension is not in the archive; it is built from
`editors/vscode` and runs `i lsp` with the `i` on `PATH`, or the one its
`ilang.path` setting names.

The package also contains documentation, the licenses of i and of included
third-party code (Gleam, Binaryen), dependency/toolchain lockfiles, and a
`share/ilang/build/release.json` manifest. The manifest records the source commit,
target, packaging toolchain, ELF interpreter paths where applicable, and SHA-256
hashes for every payload file.

## Installation

Verify the archive against its sidecar, then extract it into a versioned
directory and add its `bin` directory to `PATH`:

```sh
sha256sum --check ilang-0.1.0-x86_64-unknown-linux-gnu.tar.gz.sha256
tar -xzf ilang-0.1.0-x86_64-unknown-linux-gnu.tar.gz
export PATH="$PWD/ilang-0.1.0-x86_64-unknown-linux-gnu/bin:$PATH"
i --help
```

These are native artifacts for the selected target. Packaging preserves their
linkage. In particular, binaries built with the Nix development environment can
refer to dynamic loaders and libraries in `/nix/store`; extracting the archive
does not install that runtime on another machine. Use these artifacts on hosts
with the corresponding Nix runtime, or build and package on the intended
distribution/toolchain. The package does not claim a universal Linux ABI.

For Linux ELF artifacts, `--nix-runtime` adds the closure of store paths named in
the binaries' loader and runtime library search paths. It uses `nix-store` to
export those paths and their dependencies; it does not bundle the Rust compiler
merely because its path occurs in debug information. On the destination host,
install Nix, verify the archive, and run `./runtime/restore` from the extracted
package to import the closure before starting i. Importing requires the
destination Nix store's usual authorization. The package records every exported
store path in `runtime/paths.txt` and its build manifest. This mode does not
support Mach-O/Windows artifacts or replace a host's kernel and Nix installation.

## Reproducibility

`SOURCE_DATE_EPOCH` sets archive and gzip timestamps. When omitted, packaging
uses the checked-out commit's timestamp, or zero for a repository without a
commit. Paths are sorted; owner/group IDs are zero; ordinary files use mode 0644
and executable files use mode 0755. Cache/build directories are excluded.
Identical payloads, metadata, and epoch produce identical archives. This is an
archive reproducibility guarantee; it does not independently make native Rust
builds reproducible.

The packager fails if either binary or any required package input is missing.
It does not compile, run tests, or publish. Run the full verification workflow
before distributing a release.
