# i

i is a small, statically typed functional language that compiles to WebAssembly GC and runs natively, in JavaScript hosts and on WASI.

```i
(
	i/stdio
	std/int
)

Expr {
	Num(Int)
	Add(Expr, Expr)
	Mul(Expr, Expr)
}

// Inferred: (e: Expr) -> Int
eval = (e) {
	e |> {
		Num(n)    -> n
		Add(a, b) -> eval(a) + eval(b)
		Mul(a, b) -> eval(a) * eval(b)
	}
}

main = () { stdio.print(int.to_string(eval(Mul(Add(Num(1), Num(5)), Num(7))))) }
```

`i run` prints `42`. There are no keywords: a type is a name with its constructors in braces, a function is `(parameters) { body }`, and matching gives a value to a block of clauses with `|>`. No type is written, and every type is checked.

A function type has three parts, `(T) -> A ^ E ! N`: the value, the errors, and the services the function needs. All three are inferred. A service is an ordinary value, and the caller gives it with `!`, so a test gives its own:

```i
(
	testlib = (test) = std/test
	i/fail
)

Clock {
	Clock(now: () -> Int)
}

Late {
	Late(by: Int)
}

// Inferred: (deadline: Int) -> Int ^ Late ! Clock
remaining = (deadline, clock ! Clock) {
	deadline - clock.now() |> {
		left ? left < 0 -> fail.raise(Late(by: 0 - left))
		left            -> left
	}
}

test("a test gives its own clock", () {
	! Clock = Clock(now: () { 40 })
	assert(remaining(42) == 2)
	assert(fail.catch(() { remaining(30) }) == Error(Late(by: 10)))
})
```

`i test` prints `pass tour/clock_test.a test gives its own clock`. `test` is not part of the language either. It is a macro of the standard library, and a macro is an i function from code to code. This one is written in the Lisp surface of the same language (`.is`), where code can be quoted. i has no `if`; here is one:

```is
(, if)
(@ macro
  (= if (-> (test then else) _ `(| ~test {(-> True ~then) (-> False ~else)}))))
```

A module that imports it with `sugar = (if) = tour/sugar` writes `sign = (n) { if(n < 0, "negative", "not negative") }`, and `i expand` prints the code after expansion.

Concurrency is structured: a task belongs to a scope, and a scope ends only when all its tasks have ended. This is a TCP echo server that serves every connection in its own task and closes each one however it ends:

```i
(
	i/net
	i/task
	i/stdio
	i/core
	i/resource
)

echo = (stream) {
	bytes = net.read(stream, 65536)
	core.bit_size(bytes) |> {
		0 -> Nil
		_ -> {
			net.write(stream, bytes)
			echo(stream)
		}
	}
}

serve = (listener, scope) {
	stream = net.accept(listener)
	task.go(scope, () {
		resource.ensure(() { echo(stream) }, () { net.close_stream(stream) })
	})
	serve(listener, scope)
}

// Program entry.
main = () {
	listener = net.listen("127.0.0.1", 0)
	stdio.print("listening " <> core.inspect(net.local_port(listener)))
	task.run(() { task.scope((scope) { serve(listener, scope) }) })
}
```

These files are in [examples/tour](examples/tour) and [examples/echo](examples/echo).

## What keeps it small

- No keywords. Every word is a name; imports, exports, matching and guards are brackets and symbols.
- No mutation. Every value and binding is immutable; state that cannot be avoided is an explicit `task.ref`.
- Types are sets of values, with unions, intersections, differences and literal types, and the compiler infers the principal type of every binding.
- One function type carries the value, the errors and the environment, so errors and dependencies need no other mechanism.
- Structured concurrency only: tasks, scopes, channels and events, and no task outlives its scope.
- No library is special. Services, schemas, HTTP, logging and tests are i code over the general features.
- Two surfaces, `.i` and `.is`, for one language; `i fmt` converts between them, and both use the same macros.
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
| `i fmt` | Format `.i` and `.is` files; `--to` converts a file to the other surface |
| `i expand` | Print a module after macro expansion |
| `i add`, `remove`, `update`, `deps`, `publish` | Manage dependencies: packages are git repositories, with minimal version selection |
| `i lsp` | Run the language server |

Every command takes `--help`. Editor support is in [editors](editors): a VS Code extension, a Zed extension and a tree-sitter grammar.

## Documentation and status

The language is defined in one document, [docs/spec.md](docs/spec.md). The library modules `i/…` are in [lib/i](lib/i) and the standard library `std/…` is in [std/src](std/src); each module is documented in its source. The reasons for the design are in [docs/design](docs/design).

i is in development and has no release: the spec comes first, the compiler does not yet do everything in it, and the language still changes.

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
- [CORE-MATH](https://core-math.gitlabpages.inria.fr) — the argument reduction and the certification of rounding studied for `std/float`.
- [GHC](https://www.haskell.org/ghc/) — worker and wrapper splitting for unboxed numbers, cancellation as a protocol in which a blocked task owns its registration, and the [SplitMix](https://github.com/haskellari/splitmix) generator behind `i/random`.
- [TypeScript](https://www.typescriptlang.org) — literal types, and how large unions are reduced.
- [Wasm_of_ocaml](https://github.com/ocsigen/js_of_ocaml) — how a functional language is laid out in WasmGC, and what Binaryen can then recover.
- [Swift](https://www.swift.org) and [Rust](https://www.rust-lang.org) — multi-line and raw string literals; [rust-analyzer](https://rust-analyzer.github.io) for the structure of the language server.
- [Scala 3](https://www.scala-lang.org), [Dart](https://dart.dev) and [Zig](https://ziglang.org) — parsing methods: matching as a suffix, the lookahead after a closing bracket, and a separate slot for errors in a type.
- [Deno](https://deno.com) and [Bun](https://bun.sh) — one executable as a prebuilt runner with the program appended.
- [Tokio](https://tokio.rs) and [rustls](https://github.com/rustls/rustls) — the reactor and the TLS of the native host.
- [wasm-tools](https://github.com/bytecodealliance/wasm-tools) — `wasmparser` and `wat` read and validate modules in the compiler and its tests.
- [tree-sitter](https://tree-sitter.github.io/tree-sitter/) — the grammar behind highlighting in editors.
- [Hypothesis](https://hypothesis.readthedocs.io), [Hedgehog](https://github.com/hedgehogqa/haskell-hedgehog) and [QuickCheck](https://hackage.haskell.org/package/QuickCheck) — generators with attached shrinking, sizes and replay in `std/property`.
- [A prettier printer](https://homepages.inf.ed.ac.uk/wadler/papers/prettier/prettier.pdf) by Philip Wadler — the document algebra of `i fmt`.
- [Unicode](https://www.unicode.org) and [Rust regex](https://github.com/rust-lang/regex) — the character data in the standard library.
- [Nix](https://nixos.org), [devenv](https://devenv.sh) and [Playwright](https://playwright.dev) — the reproducible development shell and the browser tests.
