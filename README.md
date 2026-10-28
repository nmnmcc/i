# Wleam

Wleam is a Rust implementation of Gleam targeting WebAssembly GC. It derives its frontend, formatter and language server from Gleam 1.19.2 and ports gleam_stdlib 1.0.5. The intended runtime contract is shared by browsers and Wasmtime: arbitrary-precision integers, UTF-8 strings, immutable GC-managed values, typed function references and tail calls.

The repository includes a Devenv development environment and a pinned Blaze development build. Standard Cargo with the project's fixed Rust version remains the verification and release toolchain.

## Develop

Install Nix and Devenv 2.4.0, then enter the checkout:

```sh
devenv shell
devenv tasks run dev:build
devenv tasks run dev:test
devenv tasks run dev:watch
```

Blaze uses its own pinned nightly and keeps project state under `.devenv`. The first environment realization and first workspace compilation have separate costs from subsequent edits. `dev:status` reports the project's daemon state and `dev:stop` releases it.

Run the complete verification pipeline with:

```sh
devenv test
```

See [the development guide](docs/development.md) for exact tool versions, toolchain integration and verification commands.

## Compile and run

```sh
wleam-dev --project examples/hello check
wleam-dev --project examples/hello build
devenv tasks run dev:runner
wleam-dev --project examples/hello run
```

The `build` command writes a `.wasm` module, ABI metadata, and a standalone JavaScript hosting SDK in the project's `build/wasm` directory. An explicit `--out` path must end in `.wasm`. The Wasmtime-based runner is a separate executable; it can be found beside `wleam`, on `PATH`, through `WLEAM_RUNNER`, or with `--runner`. The development launcher automatically selects the optimized runner built by `dev:runner`.

```sh
wleam --project my-project test
wleam --project my-project fmt --check
wleam lsp
```

Tests are public, zero-argument functions ending in `_test` and returning `Nil`, located in `test/`. Each runs in a fresh Wasm instance so a trap cannot stop the remaining tests.

Module type interfaces, typed syntax trees, IR, and validated output are cached.
Use `build --json` to inspect cache counters, or `--no-cache` to recompute them.
Changing a dependency's implementation preserves its callers' cached frontend
and IR results when the public interface stays the same.

## Host a library

Keep Gleam sources and `gleam.toml`, with `target = "wasm"` or no explicit target. Local dependencies use `{ path = "../package" }`. The bundled standard library is available without a registry download.

```toml
[wasm]
exports = ["calculator.add"]
```

```sh
wleam --project calculator build --lib
```

```js
import { instantiate } from "./calculator.mjs";

const program = await instantiate(await fetch("./calculator.wasm"));
const result = program.call("calculator.add", [2n, 3n]);
```

Foreign functions use `@external(wasm, "module", "function")`. The versioned ABI supports booleans, floats, arbitrary-precision integers, strings, bit arrays, `Nil` and opaque host references. Compound data and callbacks require explicit application-level encoding. Integer arithmetic executes inside Wasm; JavaScript `BigInt` is used for boundary conversion.

## Compatibility and verification

The target requires WasmGC, typed function references and tail calls. See [the compatibility inventory](docs/compatibility.md) and [performance methodology](docs/performance.md) for the executable coverage and measurement procedure. CI checks both Rust toolchains and runs the generated Wasm under Wasmtime and browser engines; performance numbers are recorded only after measurement.

This project preserves [upstream provenance](UPSTREAM.md) and is distributed under the [Apache-2.0 license](LICENSE).

Run `devenv tasks run release` to build and package the compiler, executor, SDK,
standard library and licenses. See [release packages](docs/release.md) for the
archive layout and native runtime requirements.
