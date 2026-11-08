# Development environment

Use Devenv 2.4.0 and Nix to enter the reproducible development environment:

```sh
nix run github:cachix/devenv/v2.4.0 -- shell
devenv tasks run tools:versions
```

The first command selects the exact Devenv CLI version. If Devenv 2.4.0 is
already installed, `devenv shell` is equivalent.

`devenv.yaml` and `devenv.lock` pin all Nix inputs. `rust-toolchain.toml` pins
Rust 1.99.1, rustfmt, Clippy, rust-src, and rust-analyzer. Wasmtime 50.0.0 and
wasm-tools 1.262.0 use checksum-pinned upstream release archives. Node.js and the
Playwright browser bundle are pinned by the Nix input; the JavaScript lockfile
pins the corresponding Playwright client. The optional `.envrc` supports direnv.
The same locked input provides Hyperfine 2.0.0 and GNU Time 1.10 for timing and
per-command peak-memory measurements.

## Rust edit loop

Blaze v0.20261027.0 is pinned to commit
`5d4f78f7577cf7170993b74f735099dd03f5c6ca`. The environment takes the `run`
package from that revision's flake: its `default` package is a graphical demo.
The selected package provides the `blaze` command and its own
`nightly-2026-10-27` toolchain. The first shell may take time while Nix builds
Blaze's PGO distribution. This cost is separate from project compilation and
incremental edit timings.

The pinned upstream Nix package needs packaging corrections: Blaze's own
rustc-query engine expects rustup channel/host manifests that rust-overlay does
not install, and its sandboxed proc macros require the `wasm32-wasip1` standard
library. Devenv supplies both in an immutable compatibility sysroot, using the
same pinned nightly and a checksum-verified channel manifest. Only the Blaze
launcher selects this sysroot; the project's Cargo and rust-analyzer keep using
Rust 1.99.1. On Linux, it also completes missing `mktemp`/`rm` substitutions in
the pinned nightly's linker wrappers so Devenv's response-file mode works.

The pinned base also carries the small upstream fix
[`7145b282`](https://github.com/indexable-inc/blaze/commit/7145b282f6c3adccee8f24b200cb0270eba477fc),
recorded in `patches/ilang-blaze-upstream-static-path.patch`. It canonicalizes
standard-library/Cargo source paths in static symbol identities and increments
the std cache format. Without it, a std archive made under another sysroot can
make even a minimal Rust test fail to find `__RUST_STD_INTERNAL_VAL`. The
backport keeps the selected release and nightly, including the upstream
regression test; only the pre-rename engine path/cache prefix were adapted.
The upstream MIT license for this patch is retained in
`patches/LICENSE-blaze-MIT`.

```sh
devenv tasks run dev:build
devenv tasks run dev:test
i-dev --help
devenv tasks run dev:watch
```

The default Cargo workspace members select the compiler and language server.
The Wasmtime executor is built independently as part of full verification or a
release. Pass package/test selectors directly when needed:

```sh
./scripts/dev-test -p ilang-wasm
./scripts/dev-run build
devenv tasks run tools:blaze-smoke
```

For `i-dev run` and `i-dev test`, build the independent executor once
with `devenv tasks run dev:runner`. The development launcher selects the
release executor when available: Wasmtime's JIT is much slower when its Rust
implementation is compiled in the debug profile.

The toolchain smoke check uses a temporary workspace to compare Blaze and
Rust 1.99.1 Cargo across build scripts, derive macros, snapshots, package
selection, function/type/manifest changes, and daemon restart. Failures retain
the fixture and full command output under `.devenv/state/blaze-smoke-*`.
Sources live outside `target`: Blaze deliberately treats files below a
directory named `target` as immutable generated output.

The watcher serializes test runs after Rust, i, manifest, lockfile, or snapshot
changes. Tests start in fresh processes. Build output, Nix state, Git metadata,
and node_modules are excluded. It does not apply live patches to a running
compiler or language server.

Blaze uses its default `target` directory. Its daemon state and toolchain cache
are isolated under this project's `.devenv/state/blaze` via `BLAZE_HOME`:

```sh
devenv tasks run dev:status
devenv tasks run dev:stop
```

The launcher also sets `RA_JIT_CACHE` to `$BLAZE_HOME/cache`, isolating Blaze's
artifact store as well as its daemon. On Linux, Devenv selects the pinned Mesa
EGL vendor for WebKit's headless backend, including hosts without a system GPU
driver. Browser testing requires no global graphics or sandbox configuration.

The project never enables a global Cargo wrapper. Blaze errors remain failures;
use the explicit Cargo verification path to diagnose unsupported Blaze behavior.
Dependency changes and lockfile updates use the pinned Cargo executable.

## Verification and release

```sh
devenv tasks run verify
devenv test
devenv tasks run release
```

Verification uses Rust 1.99.1 and `target/cargo`, independently of Blaze output.
It checks formatting and Clippy, runs the root and `compiler` workspace
tests, builds the compiler and executor, runs the conformance fixtures, and
executes the Node SDK and Chromium/Firefox/WebKit browser tests. The browser
binaries come from Nix; npm does not download a second browser bundle. Release artifacts are built
with Cargo in `target/cargo/release`, then packaged with checksums under
`target/release`. See [release packaging](release.md) for archive contents and
native/Nix runtime requirements.

Blaze uses a different Rust toolchain from the release compiler. A Blaze build is
useful development feedback; fixed-toolchain verification determines whether a
change is ready. Differential checks compare the compiler's diagnostics, Wasm,
and runtime results across both builds. Benchmark reports distinguish Blaze's
Nix bootstrap and daemon cold start from repeated edits, and distinguish Rust
developer iteration from compilation of i programs.

## Initial environment validation

On 2026-10-29, the pinned x86_64 Linux environment completed its tool version
check, the full `tools:blaze-smoke` task, the iLang `dev:build` task, and
`i-dev --help`. Package-selected Blaze tests also passed all 122 Rust unit
and integration tests across the four default workspace members, including
cross-module LSP protocol tests. These are development smoke observations from
a changing checkout with concurrent work, not the controlled performance results in
[the benchmark guide](performance.md).

| Stage | Observed duration | Measurement scope |
| --- | --- | --- |
| First Nix build of the pinned Blaze distribution | Not measured | No reliable elapsed-time record was retained. |
| Nix build with the upstream cache-path backport | Not measured | Built successfully; its elapsed time was not independently recorded. |
| Generated two-package Cargo/Blaze smoke task | 105 s | Latest Devenv task duration, including manifest-only feature invalidation, both toolchains, and edit/restart checks. |
| First successful iLang Blaze build | 54.85 s | Blaze-reported build duration after earlier failed attempts had populated caches. |
| Subsequent `i-dev --help` | 17.03 s | Blaze-reported build portion while source changes were still occurring. |
| Frozen-source iLang build after daemon restart | 23.90 s | Full Devenv command; Blaze reported 23.67 s. Nix was already realized and the disk cache was populated. |

These observations do not establish empty-cache or repeated unchanged-build
performance. Measure those separately against a frozen source snapshot, with
the Nix bootstrap outside the timed project build. The frozen-source build's
command, scope, artifact hash, and elapsed time were saved for this run in
`target/verification/environment-final-blaze.json`.
