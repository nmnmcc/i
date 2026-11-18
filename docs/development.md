# Development environment

Use Nix and Devenv 2.4.0. Fetch the Binaryen sources once, then enter the
shell (or let direnv do it with the `.envrc`):

```sh
git submodule update --init crates/ilang-binaryen/binaryen
devenv shell
```

`devenv.yaml` and `devenv.lock` pin the Nix inputs, and `rust-toolchain.toml`
pins Rust 1.99.1 with rustfmt, Clippy, rust-src, and rust-analyzer, which
rust-overlay puts together from the archives of rust-lang.org. The shell also
has Node.js, Python 3, CMake, Wasmtime, wasm-tools, and the Playwright browsers
that match `package-lock.json`, from nixpkgs. These come prebuilt from the Nix
binary cache, so entering the shell compiles nothing. `devenv info` lists them
with their versions.

## Build and test

The workspace is built and tested with Cargo:

```sh
cargo build
cargo test -p ilang-wasm
cargo run --bin i -- test --project std
```

The workspace builds the `i` command, the code generator, the Wasmtime host,
and Binaryen. Binaryen is the git submodule `crates/ilang-binaryen/binaryen`,
built from source with CMake and linked into `i` as a library, so users need
nothing else installed. Its first build takes a few minutes; later builds
reuse it.

## Checks

`devenv test` runs `scripts/checks`, the local entry to what CI runs. It has
four independent parts:

- `rust`: the Rust formatting, Clippy, the snippet checks, and the workspace
  tests (`rust-lint` and `rust-test` run the two halves).
- `i`: builds the release `i`, checks the formatting of the i sources
  (`i fmt --check lib std bench examples`), runs the tests of `std`,
  `examples/hello`, and `bench` with `i test` at `--opt 0` and `--opt 2`,
  and runs `scripts/wasi-smoke` (WASI-target builds in Wasmtime and in Node's
  `node:wasi`), the review corpus, and the generated programs.
- `js`: the tests of the JavaScript SDK, and the browser tests in Chromium,
  Firefox, and WebKit. The browsers come from Nix; npm does not download them.
- `editors`: the VS Code extension tests, the tree-sitter grammar tests, and
  a build of the Zed extension.

`./scripts/checks rust` runs one part. CI (`.github/workflows/ci.yml`) runs
the parts as parallel jobs, each in the Devenv shell, with Devenv installed
from nixpkgs and the Cargo output kept between runs. The Rust tests are split
into groups (`CHECKS_TEST_GROUP`) and the review corpus into shards, so a run
has about 24 jobs, each named by what it runs: `build (…)`, `lint (…)`,
`test (…)`, `run (…)`, `corpus (…)`, and `docs (…)`.

Release archives are built with `./scripts/dev-release`; see
[release packaging](release.md).

## Editor support

`i lsp` runs the language server (`crates/ilang-lsp`) on standard input and
output. Its answers come from `ilang_check::query`, and its formatting is
`ilang_syntax::format`, the formatter of `i fmt`. `editors/vscode` is the
VS Code extension: its README says how to build it and run its tests
(`npm test` checks the TextMate grammar against a snapshot of token scopes).
