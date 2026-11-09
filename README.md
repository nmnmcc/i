# The i Programming Language

**i** is a programming language made for WebAssembly. The name stands for *Internet*: i compiles straight to WebAssembly GC, so the same program runs in every browser and in Wasmtime. A single small letter also says what the language aims to be: small, simple and easy to understand. When a longer name is clearer, call it **iLang**. The i is always lowercase.

i programs live in `.i` files, and each package is described by an `i.toml` file. The command-line tool is just `i`.

## Language definition

The language is defined in one document, [docs/spec.md](docs/spec.md). It is complete in itself: the syntax, the types and their inference, effects (errors, services in the environment, and one-shot delimited control), packages and versions, run-time behavior, and the binary interface with the host.

## Status

The compiler is being rebuilt to follow the spec. The first version of i grew out of the Gleam compiler; the new compiler is written from the spec and is much smaller:

- a new syntax crate and type checker (algebraic subtyping with unions, errors and environments in function types);
- a WasmGC code generator that keeps the existing runtime for big integers, bit arrays, strings and the text form of values;
- [Binaryen](https://github.com/WebAssembly/binaryen), built from source and linked into `i` as a library, for optimization, so the compiler itself only has to be simple and correct;
- a Wasmtime host and a JavaScript SDK for browsers.

`i check`, `i build`, `i run` and `i test` work on the whole language. The spec suite in `tests/spec` runs every case unoptimized and optimized, and requires the same result.

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
