# i

i is a small, statically typed functional language that compiles to WebAssembly GC and runs natively, in JavaScript hosts and on WASI.

```text
(
	i/stdio
	i/int
)

Num = (Int)
Add = (Expr, Expr)
Mul = (Expr, Expr)
Expr = Num | Add | Mul

// Inferred: (Expr) -> Int
eval =
	(Num(n)) { n }
	& (Add(a, b)) { eval(a) + eval(b) }
	& (Mul(a, b)) { eval(a) * eval(b) }

main = () {
	sum = Add(Num(1), Num(5))
	expr = Mul(sum, Num(7))
	expr |> eval |> int.to_string |> stdio.print
}
```

`i new demo` makes a package with a copy of the standard library in `demo/std`; `demo/i.toml` names that copy with `std = { path = "std" }`:

```sh
i new demo
rm demo/test/main_test.i
```

Write the program above to `demo/src/main.i`; the generated test is removed because it tests the program it replaces. `i run --project demo` prints `42`. There are no keywords: a type is a name for data, a sum is a union of types, a function is `(parameters) { body }`, and branches joined by `&` are one function that matches its argument. No type is written, and every type is checked.

The `stdlib/` copy is an ordinary path dependency that belongs to the package: 30 files, about 465 KiB, readable and editable. `.gitignore` lists only `/build/`, so the copy is committed with the package and shipped by `i publish`. It includes `stdlib/UNICODE-LICENSE` and the copyright notices in its sources; keep them when you redistribute it. Nothing refreshes the copy: `i update` handles versioned dependencies, not this directory. To move to a newer standard library, make a temporary package with the newer compiler and replace `stdlib/` with that package's `stdlib/`. A package named `std` imports the copy as `stdlib/…`.

A function type has three parts, `(P) <- (S) -> R`: the parameters, the services the function needs, and the result. An error is a service too, `Fail(E)`. All of it is inferred. A service is an ordinary value: a function takes it with `<-` in its head, and the caller gives it with `<-` after an expression, so a test gives its own:

```text
(
	testlib = (test) = i/test
	i/fail
)

Clock = (now = () -> Int)

Late = (by = Int)

// Inferred: (Int) <- (Clock, Fail(Late)) -> Int
remaining = (deadline) <- (clock: Clock) {
	deadline - clock.now() |>
		(left ? left < 0) { fail.raise(Late(by = 0 - left)) }
		& (left) { left }
}

test("a test gives its own clock", () {
	{
		assert(remaining(42) == 2)
		late = fail.catch(() { remaining(30) })
		expected = Error(Late(by = 10))
		assert(late == expected)
	} <- (Clock(now = () { 40 }))
})
```

This is `test/clock_test.i` in [examples/tour](examples/tour), where `i test` prints `pass tour/clock_test.a test gives its own clock`; in your own package the line has your package's name. `test` is not part of the language either. It is a macro of the standard library, and a macro is an i function from code to code. This one is written in the Lisp surface of the same language (`.is`), where code can be quoted. i has no `if`; here is one, and it is the whole of `src/sugar.is` in the example package [examples/tour](examples/tour):

```text
(, if)
(= _branches (-> (, then else) _ `(& (-> (, True) _ ~then) (-> (, False) _ ~else))))
(@ macro (= if (-> (, test then else) _ `(|> ~test ~(_branches then else)))))
```

A module's path is its package name, then its path under `src/` without the extension, so `tour/sugar` is `src/sugar.is` of the package `tour`. To use the macro in your own package, save the block above as `src/sugar.is` and import it with your package name, as in `sugar = (if) = demo/sugar` for a package named `demo`. Make a package and remove the test that `i new` writes for the program it generates:

```sh
i new demo
rm demo/test/main_test.i
```

Then write `demo/src/main.i`:

```text
(
	i/stdio
	sugar = (if) = demo/sugar
)

sign = (n) { if(n < 0, "negative", "not negative") }

main = () { stdio.print(sign(-1)) }
```

`i run --project demo` prints `negative`, and `i expand --project demo` prints the code after expansion, with `sign` as a branch chain:

```text
(= sign (-> (, n) _ (|> (< n 0) (& (-> (, True) _ "negative") (-> (, False) _ "not negative")))))
```

Concurrency is structured: a task belongs to a scope, and a scope ends only when all its tasks have ended. This is a TCP echo server that serves every connection in its own task and closes each one however it ends:

```text
(
	i/net
	i/task
	i/stdio
	i/core
	i/resource
)

echo = (stream) {
	bytes = net.read(stream, 65536)
	core.bit_size(bytes) |>
		(0) { Nil }
		& (_) {
			net.write(stream, bytes)
			echo(stream)
		}
}

handle = (stream) {
	resource.ensure(() { echo(stream) }, () { net.close_stream(stream) })
}

serve = (listener, scope) {
	stream = net.accept(listener)
	task.go(scope, () { handle(stream) })
	serve(listener, scope)
}

// Program entry.
main = () {
	listener = net.listen("127.0.0.1", 0)
	port = net.local_port(listener)
	stdio.print("listening " <> core.inspect(port))
	server = () { task.scope((scope) { serve(listener, scope) }) }
	task.run(server)
}
```

These programs are [examples/tour](examples/tour) and [examples/echo](examples/echo).

## What keeps it small

- No keywords. Every word is a name; imports, exports, matching and guards are brackets and symbols.
- No mutation. Every value and binding is immutable; state that cannot be avoided is an explicit `task.ref`.
- Types are sets of values, with unions, intersections, differences and literal types, and the compiler infers the principal type of every binding.
- One function type carries the parameters, the needed services and the result; an error is a service, so errors and dependencies need no other mechanism.
- Structured concurrency only: tasks, scopes, channels and events, and no task outlives its scope.
- No library is special. Services, schemas, HTTP, logging and tests are i code over the general features.
- Two surfaces, `.i` and `.is`, for one language; `i fmt` converts between them, and both use the same macros. It checks every file first and, if any conversion would drop a comment or other content or a target already exists, refuses the whole conversion and changes no file.
- One compiler output, WasmGC, for every host; `i build` makes one static native executable.

## Install

i is built from source, with [Nix](https://nixos.org) and [devenv](https://devenv.sh) 2.4.0:

```sh
git submodule update --init crates/ilang-binaryen/binaryen
devenv shell
cargo build --release
```

The compiler, the optimizer, the library, the formatter and the language server are the one file `target/release/i`. [docs/development.md](docs/development.md) has the verification commands, and [docs/release.md](docs/release.md) the release archive.

## Commands

| Command | What it does |
| --- | --- |
| `i new NAME` | Make a package: `i.toml`, a program and a test |
| `i check` | Check a package for errors |
| `i run [NAME]` | Build and run `main`, or the entry `NAME` of `i.toml` |
| `i test [FILTER]` | Build and run the tests |
| `i build [NAME]` | Build one static native executable; `--target js`, `nodejs`, `wasi` or `host` builds a Wasm module |
| `i spec` | Read the language standard |
| `i fmt` | Format `.i` and `.is` files; `--to` converts a file to the other surface, checking every file first and refusing the whole conversion if any would drop content or a target exists |
| `i expand` | Print a module after macro expansion |
| `i add`, `remove`, `update`, `deps`, `publish` | Manage dependencies: packages are git repositories, with minimal version selection |
| `i lsp` | Run the language server |

Every command takes `--help`. Editor support is in [editors](editors): a VS Code extension, a Zed extension and a tree-sitter grammar.

## Documentation and status

The language is defined in one document, [docs/spec.md](docs/spec.md). The library modules `i/…` are in [lib/i](lib/i) and the standard library `stdlib/…` is in [i/src](i/src); each module is documented in its source. The reasons for the design are in [docs/design](docs/design).

The examples above and the files under `examples` join branch chains with `&`; a value-level `|` separator is a compile error that asks for `&`. i is in development and has no release: the spec comes first, the compiler does not yet do everything in it, and the language still changes.

## License

i is distributed under the [Apache-2.0 license](LICENSE). The lexer is adapted from the [Gleam](https://github.com/gleam-lang/gleam) compiler, and the standard library is ported from [Gleam's standard library](https://github.com/gleam-lang/stdlib), both under Apache-2.0. The licenses of included third-party code are in [licenses](licenses).

## Acknowledgements

- [Gleam](https://gleam.run) — i began as a fork of Gleam's compiler: the lexer is adapted from it, the standard library is ported from [its standard library](https://github.com/gleam-lang/stdlib), and the language server follows its language server.
- [Wasmtime](https://wasmtime.dev) — runs every i program outside a browser: it is the host of `i run` and `i test` and the runtime inside the executables of `i build`.
- [Binaryen](https://github.com/WebAssembly/binaryen) — the optimizer, built from source and linked into `i`, so the compiler itself only has to be simple and correct.
- [Effect](https://effect.website) — the three parts of a function type are `Effect<A, E, R>`; a computation as a value, services given by the caller, scopes, and the shape of schemas and nested test suites come from it.
- [MLsub](https://github.com/stedolan/mlsub), [Simple-sub](https://github.com/LPTK/simple-sub) and [MLstruct](https://github.com/hkust-taco/mlstruct) — algebraic subtyping: principal type inference with unions and intersections, the extrusion and simplification of the inference engine, and records.
- [Go](https://go.dev) — packages found by their location, minimal version selection, statically linked executables, and the scheduler, channels and escape analysis studied for the task library and the numeric lowering.
- [OCaml 5](https://ocaml.org) and [Eio](https://github.com/ocaml-multicore/eio) — one-shot continuations, a scope that waits for its children, and cancellation that is checked before a task suspends.
- [Concurrent ML](https://github.com/smlnj/smlnj) — events as values: `choose`, `map`, `sync`, and a selection that commits exactly one alternative.
- [Koka](https://github.com/koka-lang/koka) — the lesson that a handler which resumes in tail position is a direct call and captures no continuation.
- [Erlang/OTP](https://www.erlang.org) and [Elixir](https://elixir-lang.org) — tasks that share nothing mutable and a copy that keeps sharing; ExUnit's split of a test into a function and its registered metadata.
- [Racket](https://racket-lang.org) — items that a macro lifts to the end of a module, and the hang-or-flow choice of its pretty printer.
- [Lean 4](https://lean-lang.org) — deterministic heartbeats, kept apart from recursion depth, as the budget of a macro run.
- [cljfmt](https://github.com/weavejester/cljfmt) and [zprint](https://github.com/kkinnear/zprint) — the layout of `.is` files.
- [CORE-MATH](https://core-math.gitlabpages.inria.fr) — the argument reduction and the certification of rounding studied for `i/float`.
- [GHC](https://www.haskell.org/ghc/) — worker and wrapper splitting for unboxed numbers, cancellation as a protocol in which a blocked task owns its registration, and the [SplitMix](https://github.com/haskellari/splitmix) generator behind `i/random`.
- [TypeScript](https://www.typescriptlang.org) — literal types, and how large unions are reduced.
- [Wasm_of_ocaml](https://github.com/ocsigen/js_of_ocaml) — how a functional language is laid out in WasmGC, and what Binaryen can then recover.
- [Swift](https://www.swift.org) and [Rust](https://www.rust-lang.org) — multi-line and raw string literals; [rust-analyzer](https://rust-analyzer.github.io) for the structure of the language server.
- [Scala 3](https://www.scala-lang.org), [Dart](https://dart.dev) and [Zig](https://ziglang.org) — parsing methods: matching as a suffix, the lookahead after a closing bracket, and a separate slot for errors in a type.
- [Deno](https://deno.com) and [Bun](https://bun.sh) — one executable as a prebuilt runner with the program appended.
- [Tokio](https://tokio.rs) and [rustls](https://github.com/rustls/rustls) — the reactor and the TLS of the native host.
- [wasm-tools](https://github.com/bytecodealliance/wasm-tools) — `wasmparser` and `wat` read and validate modules in the compiler and its tests.
- [tree-sitter](https://tree-sitter.github.io/tree-sitter/) — the grammar behind highlighting in editors.
- [Hypothesis](https://hypothesis.readthedocs.io), [Hedgehog](https://github.com/hedgehogqa/haskell-hedgehog) and [QuickCheck](https://hackage.haskell.org/package/QuickCheck) — generators with attached shrinking, sizes and replay in `i/property`.
- [A prettier printer](https://homepages.inf.ed.ac.uk/wadler/papers/prettier/prettier.pdf) by Philip Wadler — the document algebra of `i fmt`.
- [Unicode](https://www.unicode.org) and [Rust regex](https://github.com/rust-lang/regex) — the character data in the standard library.
- [Nix](https://nixos.org), [devenv](https://devenv.sh) and [Playwright](https://playwright.dev) — the reproducible development shell and the browser tests.
