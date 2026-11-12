# The i Programming Language

**i** is a programming language made for WebAssembly. The name stands for *Internet*: i compiles straight to WebAssembly GC, so the same program runs in every browser and in Wasmtime. A single small letter also says what the language aims to be: small, simple and easy to understand. When a longer name is clearer, say **the i programming language**. Where a name must be longer than one letter, such as crate names, ABI symbols and file names, write `ilang`, all lowercase. The i is always lowercase.

i programs live in `.i` files, and each package is described by an `i.toml` file. The command-line tool is just `i`.

## Language definition

The language is defined in one document, [docs/spec.md](docs/spec.md). It is complete in itself: the syntax, the types and their inference, effects (errors, services in the environment, and one-shot delimited control), packages and versions, run-time behavior, and the binary interface with the host.

## What is here

The compiler follows the spec and is kept small:

- `crates/ilang-syntax`: the lexer, parser and formatter;
- `crates/ilang-check`: name resolution and the type checker (algebraic subtyping with unions, errors and environments in function types);
- `crates/ilang-wasm`: the WasmGC code generator and its hand-written runtime;
- `crates/ilang-binaryen`: [Binaryen](https://github.com/WebAssembly/binaryen), built from source and linked into `i` as a library, for optimization, so the compiler itself only has to be simple and correct;
- `crates/ilang-package`: packages, versions and `i.lock`;
- `crates/ilang-host` and `bindings/javascript`: the Wasmtime host and the JavaScript SDK for browsers;
- `crates/ilang-lsp` and `editors/vscode`: the language server and the VS Code extension;
- `crates/ilang-cli`: the `i` command;
- `lib/i` and `std`: the library modules and the standard library. The `std` modules are `bit_array`, `bool`, `calendar`, `cli`, `dict`, `float`, `http`, `int`, `json`, `list`, `log`, `option`, `order`, `pair`, `path`, `property`, `result`, `schema`, `sequence`, `set`, `stream`, `string`, `time`, and `uri`.

The spec suite in `tests/spec` runs every case unoptimized and optimized, and requires the same result.

## Commands

| Command | What it does |
| --- | --- |
| `i new NAME` | Make a new package, with a program and a test |
| `i check` | Check a package for errors |
| `i build [NAME] [--target host\|ilang\|wasi]` | Build a package into one native executable, `build/<name>` (see [Native executables](#native-executables)); with `--target`, into a Wasm module (see [Run on a stock Wasmtime](#run-on-a-stock-wasmtime)) |
| `i run` | Build and run the program entry of a package |
| `i test [--time]` | Build and run the tests of a package |
| `i fmt [--check]` | Format the `.i` files of `src/` and `test/`, or the paths given |
| `i add NAME [LOCATION[@VERSION]]` | Add a dependency to `i.toml` and bring `i.lock` up to date |
| `i remove NAME` | Take a dependency out of `i.toml` and bring `i.lock` up to date |
| `i update [NAMES]` | Raise dependencies to the newest version with the same major version |
| `i deps` | Print the packages the program uses, and the host services and external functions each one can use |
| `i publish VERSION [--dry-run]` | Check the package and tag the new version `vX.Y.Z` in git; nothing is pushed |
| `i lsp` | Run the language server on standard input and output |

Packages are git repositories, found by their location as in Go, and versions are chosen by minimal version selection (§2 of the spec). Every command takes `--help`.

The VS Code extension, with syntax highlighting and the language server, is in [editors/vscode](editors/vscode).

## Host services of `i run` and `i test`

A program talks to its host through host services (§9.7 of the spec). `i run`
and `i test` give every program the standard service `Io` (`i:io/v1`), which
is declared in the library module `i/io`. A program that prints imports it:

```i
((Io) = i/io)


main = (io ! Io) { io.print("Hello from i!") }
```

`print` writes the text and a line feed to the standard output; see
[examples/hello](examples/hello). A test may put a fake in the place of a host
service with an environment statement, `! Io = Io(print: (text) { Nil })`. An
embedder gives `i:io/v1` and its own services with `ilang_host::HostImports`
(Rust; `ilang_host::services::standard` is the `Io` of `i run`) or the
`imports` option of `bindings/javascript/ilang.mjs`. The spec suite gives the
services of `ilang_host::services::testing`, under `test:` namespaces.

`i add`, `i update`, and `i deps` show what each package can use: the host
services and external functions that it declares, and `i:io/v1` when one of
its modules imports `i/io`.

## Native executables

`i build` makes one executable file, `build/<name>` (the package name, or
the entry `NAME` of `i build NAME`), that runs without `i` installed:

```sh
i build --project examples/hello
examples/hello/build/hello            # prints Hello from i!
```

It is a copy of the native runner, which has Wasmtime's runtime (without a
compiler) and the standard host services of `i run` (standard streams, files,
network, TLS, clocks, randomness, processes), followed by the program,
compiled ahead of time. The executable passes its arguments, environment and
standard streams to the program, and exits as `i run` does: 0 when the entry
returns, 1 with the message when it stops. It is built for the target triple
of the machine that builds it, for a baseline of CPU features (x86-64-v2 on
x86-64); building for other targets and signing are not supported yet. The
program needs a program entry, and may only import host services the runner
gives; a package that needs others is built with `--target host` and run by
an embedding host. See `docs/design/native.md` and
`docs/design/native-packaging-report.md`.

## Run on a stock Wasmtime

`i build --target host` (or `ilang`) makes a module for hosts of the i binary
interface: `i run`, the Rust host `ilang_host`, and the JavaScript SDK. To make a WASI preview 1
command instead, which any WASI host runs with no other step, build with
`--target wasi` (§B.7 of the spec):

```sh
i build --project examples/hello --target wasi
wasmtime run examples/hello/build/hello.wasm
```

Node runs the same file with `node:wasi`. On this target:

- `print` of the standard service `Io` of `i/io` writes to the standard
  output, and a stop writes its message to the standard error and exits with
  code 1.
- The runtime imports `fd_write` and `proc_exit` of `wasi_snapshot_preview1`
  itself, so a program may not import them; other WASI functions it may.
- Every other host service and external function is still an import, encoded
  as in the spec's Appendix B. Async host services are blocking calls.
- A module linked in can give an import whose parameters, value, and errors
  are `Nil`, `Bool`, or `Float`, for example
  `@host("app:math/v1") Math { Math(cbrt: (x: Float) -> Float) }` with
  `wasmtime run --preload app:math/v1=math.wasm app.wasm`, where `math.wasm`
  exports `cbrt` as `(f64) -> f64`. A preloaded module is made before the
  program, so it cannot reach the program's `memory` or `ilang_alloc`. An
  import with an `Int`, `String`, or `BitArray` needs a host that defines it
  with the exports of the instance: a Rust embedder with Wasmtime's `Linker`
  and `Caller::get_export`, a JavaScript host around `node:wasi`, or a
  module linked in statically. An import with a host reference needs such a
  host too: only the host makes host references, and the program then also
  imports `reference_id` of `ilang:host/v1`.
- The program needs `main` in `src/main.i` or a named entry in `i.toml`.
  Tests run only with `i test`, because each test
  needs a new instance.
- The host chooses how deep the stack may be. Wasmtime allows less than
  `i run`, so deep recursion that is not in tail position stops earlier; give
  `wasmtime run -W max-wasm-stack=33554432` for the 32 MiB of `i run`. Running
  out of stack or memory has no stop message: Wasmtime prints its own error
  and exits with code 134.
- The engine needs WebAssembly GC, typed function references, tail calls, and
  exception handling. Wasmtime turns these on by default from version 47 on
  (tested with 49 and 50), with the Cranelift compiler, not Winch.

## Develop

Install Nix and Devenv 2.4.0, fetch the Binaryen sources once, then work in the Devenv shell with Cargo:

```sh
git submodule update --init crates/ilang-binaryen/binaryen
devenv shell
cargo build
cargo test
```

Run the complete verification with `devenv test`, or one part of it with `./scripts/dev-verify rust`, `i`, or `js`.

See [the development guide](docs/development.md) for the tools of the shell and the verification commands, and [release packages](docs/release.md) for the archive layout.

i is distributed under the [Apache-2.0 license](LICENSE). The lexer is adapted from the [Gleam](https://github.com/gleam-lang/gleam) compiler, and the standard library in `std` is ported from [Gleam's standard library](https://github.com/gleam-lang/stdlib), both under Apache-2.0. The licenses of included third-party code are in [licenses](licenses).
