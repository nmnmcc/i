# Design proposal: root services for the `js` and `wasi` targets

Status: proposal. It comes from a discussion with the user on 2026-11-10. Part 2 lists what the user decided; everything else is proposed here and is not decided. It is not reviewed. Nothing in it is implemented or measured. The spec was read on `redesign` at `f38e100` (§9.2, §9.7, §10, Appendix B), with `docs/design/runtime.md` and `docs/design/meta.md`, and code on `main` at `229e3a4`. The examples use the syntax with no keywords.

Revision 2 (2026-11-10): the user asked for JavaScript and WASI to be one design. Part 3 is new, part 6 (WASI) is new, and the `js` target is now one case of part 3.

Revision 3 (2026-11-10): the user decided that a target also gives a default for each standard service (decision 8). T1 and T3 say this, T6 is new, and a program that prints today does not change.

Revision 4 (2026-11-10): the user decided that the target is a service too, so that a program can branch on it (decision 9). T7 is new. That each constructor of `Target` has the root service in it was proposed here, and the user agreed.

Revision 5 (2026-11-10): the user asked for the full list of targets. T8 is new, with the rule that says when two places are two targets. The list is proposed here and is not decided. The user chose two names in it: `NodeJS`, and `--target host` in place of `--target ilang`.

## 1. Goal

- A program calls JavaScript, and the JavaScript is written in i source files.
- `i build --target js` makes a Wasm module and one JavaScript file. Every line of JavaScript in that file comes from the i source or from a small fixed part of the compiler. The programmer writes no `.js` file and no imports object.
- The `js` target and the `wasi` target work in the same way, with no rule for one library.
- A call out of the program is an effect, and effects are services in the environment (§9.2).

## 2. Decided by the user

1. The output of the `js` target is a Wasm module and a JavaScript file that the compiler makes. i is not compiled to JavaScript.
2. JavaScript is written as text on a declaration. It is not written inside expressions.
3. A call of JavaScript is modelled with a service, `Js`, in the environment. The rules that change i values into JavaScript values are functions of this service, so a program can give its own: for example an `Int` as a `number` and not as a `bigint`.
4. The real `Js` and the loader are a fixed part of the compiler.
5. The `js` target gives the real `Js` to an entry point that needs it.
6. `Js` has two calls: one stops the program when the JavaScript function throws, and one gives what was thrown as an error.
7. JavaScript and WASI are one design.
8. Each of the two has a default for the standard services, written with its root service, and the target gives that default. For example, `Io` on WASI is the default that is written with `Wasi`.
9. A standard service that a target cannot do, such as files in a browser, has no default there, and an entry point that needs it is a compile error. The target itself is a service, `Target`, that every entry point can get. A program reads it and branches, and the branches of other targets are removed when the program is compiled.

## 3. One model for both targets

**T1. A target has one root service.** It is `Js` on the `js` target and `Wasi` on the `wasi` target. The root service is the only way out of the program on that target. The target gives it to an entry point that needs it.

**T2. A target has a kernel, and the kernel only moves values.** The kernel is the fixed part of the compiler that makes the real root service. It knows how a value crosses, and nothing about what the outside means. What `console.log` is, or how `fd_write` reads its buffers, is written in i.

**T3. Every other service is ordinary i on the root service.** `Io`, a clock, files: each is a function that takes the root service from the environment and gives the service. No service is made by the kernel or by the runtime. So the rule of spec §B.7 that the module itself does `print` of `i/io` on the WASI target is removed.

**T6. A target gives a default for each standard service.** The default is a function of T3 in the module of the root service: `i/js` has the `Io` that is written with `Js`, and `i/wasi` has the `Io` that is written with `Wasi`. When an entry point needs a standard service, the target calls that function with the real root service and gives the result. So `main = (io ! Io) { … }` builds for every target, as it does today.

- The program may give another one with an environment statement, which takes the place of the default (§9.2), as for any service.
- The default is made when the entry point starts, with the real root service. A root service that the program gives later, as in part 4, does not change a default that is made already.
- The standard services are the host services of the library modules. Today that is `Io` only. This is a rule of the target, of the same kind as the list of services that the native runner gives; it is not a rule of the language.
- A service of a program or of another package has no default. The program gives it, as `Console` in part 4.
- A standard service that the target cannot do has no default on that target: `i/js` has no files and no TCP. An entry point that needs such a service and does not get it from the program is a compile error (T5). The error names the service and the target.

**T7. The target is a service, and its value has the root service in it.** A new library module `i/target` has:

```i
Target {
	Js(js: js.Js)
	NodeJS(js: js.Js)
	Wasi(wasi: wasi.Wasi)
	Native(native: native.Native)
	Host
}
```

- The runtime gives `Target` to every entry point that needs it, on every target, as it gives `Assert` (§10.2). The value is the constructor of the target that the module was built for (T8), with the real root service in it.
- A program branches with an ordinary match. In the clause of a target it has the root service of that target as a value, so the clause needs nothing else from the environment. So the type of the entry point has `Target` only, and the same source is checked and built for every target. No part of the source is left out of type checking.
- *Reason for the root service in the constructor:* if `Target` only named the target, the clause for `wasi` would call code that needs `Wasi`, and `Wasi` would be in the environment of the entry point on the `js` target too, where nothing gives it (T5). Type checking comes before any code is removed, so removing the clause later does not help.
- A clause of another target never runs, because the runtime never makes that value. It has no imports: the imports of `i:wasi/v1` are made only where the real `Wasi` is made. So removing it is the work of the optimizer, and nothing is wrong if it stays. A test may give any value, for example `! target.Target = target.Wasi(fake)`.
- A match on `Target` with no `_` clause must be changed when a target is added.

**T8. The targets.** Two places where a program runs are two targets when the kernel is not the same, or when the standard services that can have a default are not the same. If neither differs, they are one target, and a program that must know more asks through the root service.

| Target | `--target` | Root service | Kernel | Standard services with a default | Stage |
|---|---|---|---|---|---|
| `Js` | `js` | `Js` | JavaScript in the file the compiler writes | What every JavaScript engine has: printing, clocks and timers, secure random numbers | First |
| `NodeJS` | `nodejs` | `Js` | The same | All of them, with the modules of Node.js | With the IO services |
| `Wasi` | `wasi` | `Wasi` | Wasm functions in the module | Printing, clocks, random numbers, files, arguments and variables, standard streams. Preview 1 cannot open or listen on a TCP socket, and cannot start a process | Printing first, the rest with the IO services |
| `Native` | `native` | `Native` | The runner of `native.md` | All of them | With `native.md` |
| `Host` | `host` | none | The embedder's own code | What the embedder gives | Today |

- `Js` is one target for a browser page, a worker, an edge runtime, and any other engine: the kernel is the same, and none of them has files, TCP, processes or standard streams. The DOM is not a standard service; a program uses it with its own `@js` values.
- `NodeJS` has the same kernel and the same root service as `Js`. It is a target of its own because more services have a default there. Deno and Bun run the modules of Node.js, so they may be this target too; this is not checked.
- `i run` and `i test` are not targets. They build for a target and run the module in the process. Today that is `Host`, with the Rust host; after `native.md` it is `Native`.
- The operating system and the processor of a native package are not targets. They are a parameter of the build, because the same services have a default on each of them. A program that must know asks through `Native`.
- A macro runs in the compiler, with no `Target`: it has no host services, and gives the same result on every machine (`meta.md` §8).
- WASI preview 2 is a target of its own when it is made, with its own root service and its own kernel: the kernel differs (the component model), and so do the services (it has TCP). It is not in this proposal.
- `Host` has no root service, because the compiler does not know the embedder. This is the only target with nothing in its constructor.

**T4. A root service is a host service of the kind the spec has now (§9.7).** `Js` has the namespace `i:js/v1` and `Wasi` has `i:wasi/v1`. So no rule of §9.7 or §10.2 changes: an entry point gets it because it is a host service, a test may give a fake, and i code may give another one with an environment statement.

**T5. On these two targets, an import that the program needs and the kernel does not give is a compile error.** The error names the service or the external function. This is the rule N4 of `docs/design/native.md`.

| | `js` | `wasi` |
|---|---|---|
| Root service | `Js` of `i/js` | `Wasi` of `i/wasi` |
| What the kernel is | JavaScript in the file the compiler writes | Wasm functions in the module |
| What the kernel moves | i values to and from JavaScript values | `Int` to and from `i32` and `i64`, bytes to and from the memory |
| Things of the outside, named in i | Values written with `@js("text")` | The functions of WASI preview 1, as fields of `Wasi` |
| Default `Io` | i code on `Js`, in `i/js` | i code on `Wasi`, in `i/wasi` |

This table has the two targets that this proposal designs. T8 has the full list.

## 4. What the programmer sees

A program that only prints does not change, and builds for every target (T6):

```i
@main
main = (io ! Io) { io.print("Hello from i!") }
```

A program that calls JavaScript of its own writes the value and a service for it:

```i
(
	js = i/js
)

@js("(text) => console.log(text)")
log: js.Value

Console { Console(print: (text: String) -> Nil) }

console = (j ! js.Js) -> Console {
	Console(print: (text) {
		! js.Js = j
		js.call(log, [js.string(text)])
		Nil
	})
}

@main
main = () {
	! Console = console()
	program()
}
```

- `log` is a **JavaScript value**. It is a reference that i code cannot look into. It does nothing until it is given to `Js`.
- `Js` is the service that acts. `js.call` needs it in the environment, so the type of a function says that it calls JavaScript.
- `console` takes `Js` when it is called and gives a `Console` that keeps it. The fields of `Console` have no `Js` in their types, so code that prints does not know that JavaScript is behind it, and a test gives a fake `Console`. This is a layer of Effect-TS.
- `main` has `Js` in its environment, and the target gives it.

The same on WASI, with no text of another language:

```i
(
	wasi = i/wasi
)

console = (w ! wasi.Wasi) -> Console {
	Console(print: (text) {
		! wasi.Wasi = w
		wasi.write(1, <<text:utf8, "\n":utf8>>)
		Nil
	})
}
```

`wasi.write` is ordinary i in `i/wasi`: it puts the bytes into the memory, makes the list of buffers that `fd_write` wants, and calls the field `fd_write` of `Wasi`.

A program that does one thing in two ways branches on the target (T7). Here `browser_storage` and `file_storage` are ordinary functions that take the root service as a parameter:

```i
(
	target = i/target
)

@main
main = (t ! target.Target) {
	t |> {
		target.Js(j) -> {
			! Storage = browser_storage(j)
			program()
		}
		target.Wasi(w) -> {
			! Storage = file_storage(w)
			program()
		}
		target.Host -> panic("this program needs the js or the wasi target")
	}
}
```

The type of `main` is `() -> Nil ! Target`, on every target.

A program that wants another rule gives its own root service:

```i
@main
main = (j ! js.Js) {
	! js.Js = js.with_int(j, (n) { j.float(to_float(n)) })
	program()
}
```

`with_int` is an ordinary function: it gives a `Js` with one field replaced.

## 5. The `js` target

**J1. `@js("text")` on a declaration makes a JavaScript value.** The item is `name: T`, where `T` is a custom type with no constructors (§4.3). The text is one JavaScript expression. The value is worked out once, before the program starts (§B.1), in the order of §2.5.

- The rule names no library: `js.Value` is one such type, and a program may define others, such as `Element {}`.
- The compiler does not read the text. Text that is not an expression makes the JavaScript file fail when it is loaded.
- Text that does work when it is evaluated, and not when it is called, breaks the rule that only `Js` acts. The compiler cannot check this. `i deps` prints every text of every package, so that it can be read.

**J2. The public functions of `i/js`.** Each needs `Js` in the environment. This list is the first stage.

| Function | Type | What it does |
|---|---|---|
| `call` | `(f: Value, args: List(Value)) -> Value` | Calls `f`. If `f` throws, the program stops. |
| `try_call` | `(f: Value, args: List(Value)) -> Value ? Thrown` | Calls `f`. `Thrown(value)` has what `f` threw. |
| `nil` | `() -> Value` | `undefined` |
| `bool`, `float`, `string`, `bits` | `(Bool) -> Value`, and so on | A `boolean`, a `number`, a `string`, a `Uint8Array` |
| `int` | `(Int) -> Value` | A `bigint` in the real `Js` |
| `to_bool`, `to_float`, `to_string`, `to_bits`, `to_int` | `(Value) -> Bool ? Mismatch`, and so on | The i value, or an error when the JavaScript value is of another kind |
| `with_int` and others like it | `(Js, (Int) -> Value) -> Js` | A `Js` with one rule replaced. They need no environment. |

There is no function that reads a property or makes an object. A JavaScript value written with `@js` does that: `@js("(o) => o.name") name_of: js.Value`. So one call does a whole piece of work, and the program does not cross to JavaScript for each property.

**J3. The files.** `i build --target js` writes three files into `build/`:

- `name.wasm`: the module. Its imports are `i:js/v1`, the JavaScript values of J1, and the runtime imports of §B.4.
- `name.mjs`: the kernel, then every text of J1, then the code that makes the instance, calls `ilang_start`, and runs the program entry. It also exports the host exports (§10.2) as functions.
- `name.d.ts`: the TypeScript types of the host exports, by the table that `bindings/javascript` has now.

`bindings/javascript` stays, for a host that gives its own imports to a module of `--target host`.

**J4. How it is made.**

- The fields of `Js` cross the boundary with the encodings of §B.2 that the compiler has now. So the code generator gets no new encoding, and the kernel is a part of what `bindings/javascript/ilang.mjs` does today. The first build may put that loader into `name.mjs` as it is, and a smaller kernel may take its place later.
- The list of arguments of `call` cannot cross the boundary as a `List`. So the fields of `Js` are lower than the functions of J2: they make a JavaScript array, add a value to it, and call with it. `call` and `try_call` are ordinary i on these fields. The exact fields are for the implementation.
- A JavaScript value of J1 is a new kind of import: a reference that the module gets when the instance is made. The metadata (§B.6) lists each one with its text.

## 6. The `wasi` target

**W1. `Wasi` has one field for each function of WASI preview 1, and four fields for the memory.**

- A field of a WASI function has the parameters of that function in the same order, each an `Int`, and gives the `Int` that the function gives (the error number). An example is `fd_write: (fd: Int, buffers: Int, count: Int, written: Int) -> Int`.
- The memory fields are `alloc: (size: Int) -> Int`, `free: (address: Int, size: Int) -> Nil`, `store: (address: Int, bytes: BitArray) -> Nil`, and `load: (address: Int, size: Int) -> BitArray`. The memory is the `memory` of §B.1.
- An `Int` that does not fit the `i32` or `i64` of its place, or an address that is not inside the memory, stops the program.

**W2. The kernel is made from a table.** For each field of a WASI function, the compiler writes a Wasm function that changes each `Int` to its `i32` or `i64`, calls the import of `wasi_snapshot_preview1`, and changes the result back. The table has the name and the Wasm type of each function. No function has code of its own. The module imports only the WASI functions whose fields the program can reach.

**W3. The library module `i/wasi` has the functions that programs call.** `write`, `read`, the clock, the arguments, and so on are ordinary i on the fields of W1. They know the layout of the buffers and the meaning of the error numbers. The first stage has `write` only, which is what the target can do today.

**W4. What stays in the runtime.** A stop (§11.7) still writes its message with `fd_write` and ends with `proc_exit`, because no i code can run then. `_start` and the rule that tests are not compiled stay as in §B.7. A program still may not import `fd_write` or `proc_exit` itself.

**W5. What is removed.** The special case for `print` of `i/io` in §B.7 and in the compiler (`ImportKind::WasiStdout` and `is_standard_service` in `crates/ilang-wasm/src/lower.rs`). On this target `Io` is the default of T6, which is i code, as on `js`.

## 7. Stages

1. The `js` target: J1 to J4 with the functions of J2. Tests run in Node.js and in the three browsers that `tests/browser` uses.
2. The `wasi` target: W1 to W5 with `write`. The programs that print on WASI today must give the same output.
3. **Promises.** `await: (promise: Value) -> Request(Result(Value, Thrown))`, with the `Request` of `runtime.md` D2. It needs the request form of host services (`runtime.md` §5.2), so it comes after R2.
4. **Callbacks.** `function: (f: (args: List(Value)) -> Value) -> Value` gives a JavaScript function that calls an i function. It needs a new export that calls a function value, and a rule for the environment of such a call. This is the largest piece and has no design yet.
5. **More of WASI in `i/wasi`,** and the IO services of `runtime.md` on `Wasi`. `runtime.md` §8.2 says that this target gives no adapters for them; with W1 they can be i code.
6. **Typed wrappers by macro.** The programmer writes a signature, and a macro writes the calls of `js.string`, `js.to_int` and so on. It needs typed macros (M4).

## 8. What this changes in the plan

- Three new library modules, `i/js`, `i/wasi` and `i/target`, and one new target, in the first two stages. `NodeJS` and `Native` (T8) each add a module and come later. Spec §10.2 says that an entry point may have `Target` in its environment, with host services and `Assert`. `README.md`, spec §0, §2.1 and §12 say how many library modules there are.
- **`--target ilang` gets the name `--target host`.** The option of `i build`, `Target::Ilang` in `crates/ilang-wasm/src/ir.rs` and `crates/ilang-cli/src/main.rs`, `README.md`, and the places in `native.md` (N1, N4) and `runtime.md` that name the target change. If the old name stays for some time as a second name is not decided.
- Spec: a new attribute in §2.2, a short new part of §10 for J1, a new part of Appendix B for the `js` target, and §B.7 with the `print` rule removed and W1 to W4 added. The sentence of §9.7 that says the WASI target gives the standard service itself is removed.
- A program that prints on WASI today does not change (T6). What changes is how the module does it: with i code of `i/wasi`, and not with a function of the runtime.
- `runtime.md` says that the JavaScript side of the IO services is a factory in the SDK, and that the WASI target gives no adapters. With this proposal, both can be ordinary i in the library. This proposal does not ask for that change. If it is wanted, it is a change to a settled decision and needs the user.
- Track C owns `emit/host.rs`, the ABI version and `bindings/javascript`. Stages 1 and 2 touch `emit/host.rs`, `lower.rs` and the metadata, so they must come after track C lands.

## 9. Not checked, and open

- **`Native` as a root service changes `native.md` and `runtime.md`.** There, the runner gives the IO services directly, each with its own namespace. In T8 the runner gives one root service, and the IO services are i code on it, as on every other target. The lower operations of `runtime.md` D9 (start a request with integers and bytes) are close to this already, but it is a change to two notes that the plan calls settled, and needs the user in that process. Until then the constructor is `Native` with nothing in it, and a clause for it that uses a service of the runner puts that service in the environment of the entry point on every target, so the `js` build fails (T5).
- **How fine a standard service is.** `runtime.md` has one service for processes, with arguments, variables, random numbers and child processes. On `Wasi` the first three can be done and the last cannot. With "a default or a compile error" for a whole service, such a service has no default there. `runtime.md` §8.2 gives every field and a typed error for what the target cannot do. The two rules do not agree; smaller services would make them agree.
- **Deno, Bun, and edge runtimes** are not checked: that their engines have what the module needs (WasmGC, tail calls, exception handling with `exnref`), and that Deno and Bun are `NodeJS` for every default.
- **How a `@js` text names a module of Node.js.** A text is an expression, and an `import` declaration is not one.
- **The `Host` constructor has nothing in it.** A clause for `Host` that uses a service of the embedder puts that service in the environment of the entry point on every target. This cannot be changed: the compiler does not know the embedder.
- **If the optimizer removes the clauses of other targets.** T7 does not need it, but the size of the module does. The runtime makes the value of `Target` in one place with a known constructor; that Binaryen then removes the other clauses is not checked.
- **A package with a service of its own** exports a function that takes `Target` and gives the service, and the program calls it. If such a service should also have a default, as the standard ones have, is not in this proposal.
- **Which services are standard on these targets** when the IO services of `runtime.md` land (clock, files, network, processes, standard streams). A browser has no files and no raw TCP.
- **The memory fields of `Wasi` are not safe.** `store` can write over a buffer that the runtime owns. A program that has `Wasi` can do this; a program that has only `Io` cannot. The other way is a field with i types for each WASI function, with the layout in the kernel: safe, but then the kernel has code for each function, which T2 says it has not.
- **Cost on `js`.** Each argument is one crossing to JavaScript, then the call is one more. The fixed interface of Appendix B is one crossing. Nothing is measured.
- **Cost on `wasi`.** `print` becomes i code that makes a bit array, copies it into the memory, and calls a field. Today it is one runtime function. Nothing is measured.
- **A `@js` value on other targets.** If code that names one is in the module, the module has an import that no host gives. Proposed: on other targets the value is the null reference, which i code cannot tell from another reference but with `==`. Not decided.
- **A fake `Js` cannot make a `Value`,** because the type has no constructors. So a test replaces the service above `Js`, not `Js`. A fake `Wasi` has no such limit.
- **Types of the host exports.** A program's own rule (decision 3) changes calls out of the program. A host export is called by the kernel, with the rules of the real `Js`. So `name.d.ts` is right for exports, and an export that takes a `js.Value` has the type `unknown`. Should an export be able to use the program's rule?
- **`i run` and `i test` for the `js` target.** They need Node.js or a browser. Not in stage 1.
- **The order in which the texts of J1 are evaluated** when two packages have them, and what a text may name (only globals, or also other texts).
- **A way to write the text.** A string literal needs `\"` and `\\`. A raw string literal is a change of syntax in the range of track A1.
- **WASI preview 2.** It is defined with the component model, not with functions on one memory. `Wasi` here is preview 1 only.

## 10. Wider questions from the same discussion

These came up, and none is part of this proposal. Each changes decisions of the redesign plan, so each needs its own note.

1. **A call out of the program is an effect.** Then `@external` for programs (§10.1) can go, and every function of the host is a field of a service. The built-in functions of `ilang:core` and `ilang:control` are part of the language and stay.
2. **The word "host" has three meanings:** who starts the program, who gives the services, and the other end of the encoding. With `@js`, the second is the package itself.
3. **A namespace such as `app:console/v1` is only a text** that the program and the host agree on. It is a second system of names next to packages and modules. The name of an import could come from the package, the module and the type, with a written name only for an interface that is fixed outside, such as WASI.
4. **The `"async"` form of `@host`.** A field that gives a `Request(T)` says the same thing in its type, and waiting is ordinary i (`runtime.md` D9 writes IO this way). The three forms of `runtime.md` §5.2 would be one.
5. **A call of JavaScript is a remote call:** the two sides share no memory, references are handles, and an async call is a request with an id. Seen this way, the types that cross (§10.3) are the types that can be encoded, which may be the "portable" values of `multicore.md`; and a host export is a service that the program gives, so the two directions could have one form.
6. **Other languages.** A module of another language that is compiled to Wasm can give a service. Values with buffers need code between two memories, which is what the Wasm component model defines. Not for now.
7. **TypeScript declarations into i.** A tool that reads `.d.ts` files and writes i wrappers. A macro cannot do it, because a macro reads no files.
8. **The native target in the same model.** Its kernel is the runner, which is large by design (the reactor, sockets, files). If its root service were one low service too, the IO services of `runtime.md` would be i code on every target. This is a change to `native.md`, and is not looked at here.
