# i Rust host

`ilang-host` embeds Wasmtime 50 and runs i programs through the binary
interface version 3 of `docs/spec.md` (Appendix B). It enables the Wasm
features that i needs: garbage collection, typed function references, tail
calls, and exception handling with `exnref`.

```rust,no_run
use ilang_host::{HostImports, HostOptions, Outcome, Program, Value};

fn main() -> anyhow::Result<()> {
    let wasm = std::fs::read("application.wasm")?;
    let program = Program::from_bytes(&wasm)?;

    // Host services (spec §9.7) and external functions (§10.1).
    let mut imports = HostImports::default();
    imports.service("app:console/v1", "print", |arguments| {
        if let [Value::String(text)] = arguments {
            print!("{text}");
        }
        Ok(Ok(Value::Nil))
    })?;

    let mut instance = program.instantiate(HostOptions::new(imports))?;
    match instance.call("app/math.add", &[Value::Int(1.into()), Value::Int(2.into())])? {
        Outcome::Ok(value) => println!("{value:?}"),
        Outcome::Err(error) => println!("error {:?}", error.0),
        Outcome::Stop(stop) => println!("stopped:\n{stop}"),
    }
    Ok(())
}
```

## The module

`Program::from_bytes` compiles the module and reads its `ilang.abi` custom
section (B.6): `entry`, `tests` (with `async`), `exports` (with `errors` and
`async`), `services` and `externals`. Unknown fields, a version other than 3,
errors that are not in the canonical order, and names of the interface used for
exports are refused. `Program::with_abi` gives the metadata of a module without
the section; it cannot replace a section.

`Program::instantiate` links the three runtime imports of `ilang:host/v1`
(`panic`, `reference_id` and `cancel`, B.4) and every function in `services` and
`externals`. A missing host function fails instantiation. Then it calls
`ilang_start` once, before anything else (B.1). When the program stops in
`ilang_start`, the error is a `Stop`.

## Calls

`Instance::call(name, arguments)` calls a host export with the encoding of B.2
and gives an `Outcome`:

- `Outcome::Ok(value)`;
- `Outcome::Err(GuestError(value))` when the function gave an error of its
  `? E`; the tag selects the member of `E` in the canonical order Nil, Bool, Int,
  Float, String, BitArray;
- `Outcome::Stop(Stop { message, trap })` when the program stopped. `message` is
  the stop message given to `panic` (spec §11.7): the reason, then the source
  positions. It is `None` when the module trapped without one.

After a stop, or any other trap, the instance refuses every call (B.5); make a
new one. `Err(…)` of the outer `Result` is a mistake of the caller (an unknown
export, a wrong number of arguments), a failure of a host function, or a value
from the module that is not correct (B.2). The last two also make the instance
unusable.

`Program::run_entry` runs the program entry in a new instance, and
`Program::run_tests` runs every test of the metadata in a new instance (spec
§10.2). `Instance::run(name)` runs one of them by name.

| Type | Wasm values | Rust value |
| --- | --- | --- |
| `nil` | none | `Value::Nil` |
| `bool` | `i32` 0 or 1 | `Value::Bool` |
| `int` | sign, address, count (`i32`) | `Value::Int(BigInt)` |
| `float` | finite `f64` | `Value::Float` |
| `string` | address, UTF-8 length | `Value::String` |
| `bit_array` | address, number of bits | `Value::BitArray` |
| `externref` | `externref` | `Value::Externref` or `Value::NullExternref` |

Buffers follow B.3. For an export, the host makes the parameter buffers with
`ilang_alloc` and frees them with `ilang_free` after the call, and frees the
result buffers after copying them. For an import, the module owns the parameter
buffers and the host makes the result buffers with `ilang_alloc`. The host gives
empty buffers the address 0 and does not make or free them; the module frees
every result buffer whose address is not 0, also an empty one.

## Values that are not correct

Values from the host are checked as B.2 requires. An argument of an export that
is not a correct value of its type (a `Float` that is not finite, a `BitArray`
whose bits after the end are not 0, or a value of another type) stops the
program with `host: value is not correct`.

The answer of a host function that has the right type is given to the module as
it is, also when it is not correct: the module checks it and stops with the
positions of the call (B.2, §11.7). An answer of another type, or an error that
is not a member of `E`, cannot be given to the module, so the host stops the
program with `host: value is not correct` and no positions. With
`HostOptions::check_replies`, the host checks the answers of the right type too,
and stops the program for them in the same way.

A host function that returns `Err` traps the instance, with no handler of the
module run, and its error is given back by the call.

Values from the module are checked as well: a `Bool` that is not 0 or 1, a
`String` that is not UTF-8, an `Int` with a wrong sign or a last part of 0, or a
tag that is too big.

## Host functions

```rust,no_run
# use ilang_host::{HostImports, Value, BigInt};
# fn f(imports: &mut HostImports) -> anyhow::Result<()> {
// read(key: String) -> String ? String
imports.service("app:storage/v1", "read", |arguments| {
    Ok(match arguments {
        [Value::String(key)] if key == "name" => Ok(Value::String("i".into())),
        _ => Err(Value::String("missing".into())),
    })
})?;
// @external("app:math/v1", "sqrt")
imports.external("app:math/v1", "sqrt", |arguments| match arguments {
    [Value::Float(x)] => Ok(Value::Float(x.sqrt())),
    _ => anyhow::bail!("sqrt takes a Float"),
})?;
# Ok(()) }
```

## Async (B.4, B.5)

`HostImports::async_service` registers a function of an async host service. It
starts the work and gives a future. The module picks a request id for each
async host function that it starts, and may keep many requests outstanding.
`Instance::call_async` and `Instance::run_async` call an async export. While
the status is 1, they wait for the work of any outstanding request, and answer
the requests one at a time, in the order their work finishes, through
`ilang_resume:namespace:label` with the id first. At status 2 (the module can
go on without an answer) they let the runtime run once, give an answer that is
done, and otherwise call `ilang_continue`. At status 0 they read the results
from `ilang_result:name`. The module may drop a request with the runtime
import `cancel`. The requests that are left when the entry point ends, when it
stops, or when a host function fails are dropped, and so is their work.

The start closure of an async service runs during the import, and its future
is first polled when the host waits. So work out deadlines and copy inputs in
the start closure, and do not spawn tasks. Work handed to `spawn_blocking` is
detached when its request is dropped: it runs to its end and its result is
thrown away.

`ilang_host::block_on` runs such a call on one new thread named `ilang`, with a
stack of `THREAD_STACK` bytes and a tokio `current_thread` runtime with time
and IO enabled. IO is there whenever tokio is built with its IO driver, which a
service that uses tokio IO turns on through its own tokio features (for example
`net`). Every Wasm call of the future runs directly on that thread. It needs a
`Send` future; with a future that is not `Send`, await `call_async` on your own
executor. `Program::run_entry`, `Program::run_test_with` and
`Program::run_tests` run each program or test inside one `block_on`.

Called from a thread that `block_on` did not start, each Wasm call starts its
own thread with a stack of `THREAD_STACK` bytes, and the futures of the
services are polled where the call is awaited. Await it inside a tokio runtime
with time and IO enabled when a service uses tokio; otherwise such a service
panics with tokio's message. A Wasm call made from inside a host function, for
example into a second instance, also gets a thread of its own.

A sync service runs inside the tokio runtime of `block_on`. It must not start or
block on a runtime of its own (for example `Runtime::block_on` or
`reqwest::blocking`); write it as an async service, or do that work on a thread
of its own and join it.

## Host references

`HostReference::new` stores an opaque `Send + Sync` Rust value in an `Arc`.
`reference_id` gives the same number for clones of one reference, different
numbers for different references, and 0 for null, so `==` in i compares host
references by identity. References held by the module stay rooted through
Wasmtime's GC.

## Example

```sh
cargo run --release -p ilang-host --example run -- path/to/program.wasm
```

runs the program entry and the tests of a module that needs no host functions.

## Changes in version 3

- Async imports take a request id first, and the module may keep many
  requests outstanding; the host answers them in any order. The runtime import
  `cancel`, status 2 and the export `ilang_continue` are new. Modules of
  version 2 are refused; build them again.
- `block_on` runs the future on one `ilang` thread with a tokio
  `current_thread` runtime, and needs a `Send` future. With a future that is
  not `Send`, await `call_async` on your own executor.
- Sync services and externals run inside that tokio runtime. A sync service
  that starts or blocks on a runtime of its own (for example
  `Runtime::block_on` or `reqwest::blocking`) now panics with tokio's message;
  write it as an async service, or do that work on a thread of its own and
  join it.
