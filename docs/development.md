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
cargo run --bin i -- test --project stdlib
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
  (`i fmt --check lib stdlib bench examples`), runs the tests of `stdlib`,
  `examples/hello`, and `bench` with `i test` at `--opt 0` and `--opt 2`,
  and runs `scripts/wasi-smoke` (WASI-target builds in Wasmtime and in Node's
  `node:wasi`), the review corpus, and the generated programs.
- `js`: the tests of the JavaScript SDK, and the browser tests in Chromium,
  Firefox, and WebKit. The browsers come from Nix; npm does not download them.
- `editors`: the VS Code extension tests, the tree-sitter grammar tests, and
  a build of the Zed extension.

`./scripts/checks rust` runs one part. CI (`.github/workflows/ci.yml`) runs
the parts as parallel jobs, each in the Devenv shell, with Devenv installed
from nixpkgs and the Cargo output kept between runs. Each job is named by what
it runs: `build (…)`, `lint (…)`, `test (…)`, `run (…)`, `corpus (…)`, and
`docs (…)`.

`scripts/checks` reads these variables, which CI sets for its shards:

- `CHECKS_TEST_GROUP` splits `rust-test`. With `targets`, it runs exactly
  the Cargo targets in `CHECKS_TEST_TARGETS`, a list of `<crate>:<target>`
  (crate `ilang-check` or `ilang-cli`; target `lib`, `bin`, `doc`, or the
  name of a file of the crate's `tests/`), one `cargo test --no-fail-fast`
  per crate in the order given; an unknown crate or an empty list stops it
  with exit status 2. The hand-written groups (`check-heavy`, `check`,
  `spec`, `spec-K/N`, `cli-heavy`, `cli`, `rest`) still work.
- `CHECKS_SHARD=K/N` makes the `i` part run shard K of N of the review
  corpus (`scripts/review-corpus run --shard K/N --jobs 4`) instead of the
  whole corpus; `CHECKS_SKIP_CORPUS=1` leaves the corpus out.
- `CHECKS_I_BINARY` gives the `i` part a built `i` instead of building one.
- `CHECKS_BUDGET` is the budget of one shard in seconds, for the shard
  tooling; it defaults to 270.

### CI on pull requests and on main

Every push to main runs the full gate. A pull request runs the same gate except for the jobs whose inputs did not change (Rust, docs, editors); the run summary names what ran and what was skipped, and `summary (required)` fails if a planned job did not report. The wall time of
a run is in its job summary. When a run fails, open the first red job in the
summary and start there.

### Cost table

`ci/costs.json` holds the measured seconds of every unit of every set. The
planner reads it; a unit with no cost takes the set median, and the plan
prints those keys. To refresh it, run `scripts/ci/refresh-costs` on the CI
logs (it accepts log files or directories). A set that is missing entirely
stops the plan; a stale table only makes the shards less even, never less
correct, so do not edit shard counts by hand when the table ages.

Per-case timings (spec, the value-types recovery loop and the review corpus)
are collected by setting `CHECKS_TIMINGS` to a file path; the runs then write
one `{"key": ..., "seconds": ...}` line per case, and CI uploads the file as
an artifact. The variable is unset in normal runs and changes neither the
order nor the concurrency of the cases.

The slow sets are split into shards by `i-shard` (`crates/ilang-shard`), so
no shard list is written by hand. The units of a set (test targets, spec
cases, corpus cases, projects) are its sorted keys, and their costs come from
`ci/costs.json`; a unit without a cost takes the median of its set. The shard
count is `ceil(total / budget)`, raised one at a time while a shard without
an over-budget unit would still go over the budget, and the units are packed
longest first onto the least loaded shard (LPT). The plan prints every unit
that is over the budget on its own and every unit that took the median. A
stale table changes only how evenly the shards are loaded, never which units
run. Refresh it from CI logs with `scripts/ci/refresh-costs --logs <dir>`, and
with `--sync-keys` after adding or removing units.

Release archives are built with `./scripts/dev-release`; see
[release packaging](release.md).

The release workflow's cold path is the **Nix store closure, not `i`**. A pin
change (`flake.lock`, or a package low in the closure such as `tzdata`) makes
the runner rebuild `rust-minimal`, `curl` and the Python packages from source,
which can take hours; the compiler itself is minutes. That cost is why a
pull-request run restores the archive built by the nightly run on `main` (same
Nix inputs, keyed per platform) and only smokes it in a container without
`/nix`; a tag still builds the tree it names and publishes it. Do not optimise
the compiler for the cold path.

The smoke is offline. Linux runs it in an empty network namespace and the
container with `--network none`; macOS has no `unshare -n` and relies on the
single `ILANG_GIT_URLS` source plus an assertion on the version in `i.toml`,
which is one step weaker. The library the smoke serves is tagged with
`<flake version>-dev.<commit>`, a version no published tag can have, so a
broken redirect asks for a tag that does not exist and fails loudly instead of
resolving the released, older library.

## Editor support

`i lsp` runs the language server (`crates/ilang-lsp`) on standard input and
output. Its answers come from `ilang_check::query`, and its formatting is
`ilang_syntax::format`, the formatter of `i fmt`. `editors/vscode` is the
VS Code extension: its README says how to build it and run its tests
(`npm test` checks the TextMate grammar against a snapshot of token scopes).
