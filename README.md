# The i Programming Language

**i** is a programming language made for WebAssembly. The name stands for *Internet*: i compiles straight to WebAssembly GC, so the same program runs in every browser and in Wasmtime. A single small letter also says what the language aims to be: small, simple and easy to understand. When a longer name is clearer, say **the i programming language**. Where a name must be longer than one letter, such as crate names, ABI symbols and file names, write `ilang`, all lowercase. The i is always lowercase.

i programs live in `.i` files, and each package is described by an `i.toml` file. The command-line tool is just `i`.

## Language definition

The language is defined in one document, [docs/spec.md](docs/spec.md). It is complete in itself: the syntax, the types and their inference, effects (errors, services in the environment, and one-shot delimited control), packages and versions, run-time behavior, and the binary interface with the host.

## Status

The compiler is being rebuilt to follow the spec. The first version of i grew out of the Gleam compiler; the new compiler is written from the spec and is much smaller:

- a new syntax crate and type checker (algebraic subtyping with unions, errors and environments in function types);
- a WasmGC code generator that keeps the existing runtime for big integers, bit arrays, strings and the text form of values;
- [Binaryen](https://github.com/WebAssembly/binaryen), built from source and linked into `i` as a library, for optimization, so the compiler itself only has to be simple and correct;
- a Wasmtime host and a JavaScript SDK for browsers.

The spec suite in `tests/spec` runs every case unoptimized and optimized, and requires the same result.

## Commands

| Command | What it does |
| --- | --- |
| `i new NAME` | Make a new package, with a program and a test |
| `i check` | Check a package for errors |
| `i build [--target ilang\|wasi]` | Build a package into a Wasm module (see [Run on a stock Wasmtime](#run-on-a-stock-wasmtime)) |
| `i run` | Build and run the program entry of a package |
| `i test [--time]` | Build and run the tests of a package |
| `i fmt [--check]` | Format the `.i` files of `src/` and `test/`, or the paths given |
| `i add NAME [LOCATION[@VERSION]]` | Add a dependency to `i.toml` and bring `i.lock` up to date |
| `i remove NAME` | Take a dependency out of `i.toml` and bring `i.lock` up to date |
| `i update [NAMES]` | Raise dependencies to the newest version with the same major version |
| `i deps` | Print the packages the program uses, and the host services and external functions each one declares |
| `i publish VERSION [--dry-run]` | Check the package and tag the new version `vX.Y.Z` in git; nothing is pushed |
| `i lsp` | Run the language server on standard input and output |

Packages are git repositories, found by their location as in Go, and versions are chosen by minimal version selection (§2 of the spec). Every command takes `--help`.

The VS Code extension, with syntax highlighting and the language server, is in [editors/vscode](editors/vscode).

## Host services of `i run` and `i test`

A program talks to its host through host services (§9.7 of the spec). `i run`
and `i test` give every program one service, which it declares like this:

```i
@host("i:io/v1")
Io { Io(print: (text: String) -> Nil) }

@main
main = (io ! Io) { io.print("Hello from i!") }
```

`print` writes the text and a line feed to the standard output; see
[examples/hello](examples/hello). A test may put a fake in the place of a host
service with an environment statement, `! Io = Io(print: (text) { Nil })`. An
embedder gives its own services with `ilang_host::HostImports` (Rust) or the
`imports` option of `bindings/javascript/ilang.mjs`. The spec suite gives the
services of `ilang_host::services::testing`, under `test:` namespaces.

## Run on a stock Wasmtime

`i build` makes a module for hosts of the i binary interface: `i run`, the
Rust host `ilang_host`, and the JavaScript SDK. To make a WASI preview 1
command instead, which any WASI host runs with no other step, build with
`--target wasi` (§B.7 of the spec):

```sh
i build --project examples/hello --target wasi
wasmtime run examples/hello/build/hello.wasm
```

Node runs the same file with `node:wasi`. On this target:

- `print` of the standard service `i:io/v1` writes to the standard output,
  and a stop writes its message to the standard error and exits with code 1.
- Every other host service and external function is still an import, encoded
  as in the spec's Appendix B. Give it with a module linked in, for example
  `wasmtime run --preload app:clock/v1=clock.wasm app.wasm`. Async host
  services are blocking calls.
- The program needs a `@main`. Tests run only with `i test`, because each test
  needs a new instance.
- The host chooses how deep the stack may be. Wasmtime allows less than
  `i run`, so deep recursion that is not in tail position stops earlier; give
  `wasmtime run -W max-wasm-stack=33554432` for the 32 MiB of `i run`. Running
  out of stack or memory has no stop message: Wasmtime prints its own error
  and exits with code 134.
- The engine needs WebAssembly GC, typed function references, tail calls, and
  exception handling. Wasmtime turns these on by default from version 47 on
  (tested with 50), with the Cranelift compiler, not Winch.

## Develop

Install Nix and Devenv 2.4.0, fetch the Binaryen sources once, then enter the checkout:

```sh
git submodule update --init crates/ilang-binaryen/binaryen
devenv shell
devenv tasks run dev:build
devenv tasks run dev:test
```

Run the complete verification pipeline with:

```sh
devenv test
```

See [the development guide](docs/development.md) for exact tool versions and verification commands, and [release packages](docs/release.md) for the archive layout.

i is distributed under the [Apache-2.0 license](LICENSE). The licenses of included third-party code are in [licenses](licenses).
