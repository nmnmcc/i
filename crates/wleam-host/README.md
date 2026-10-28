# Wleam Rust host

`wleam-host` embeds Wasmtime 50 with the WasmGC, function-reference and tail-call
features used by Wleam. It is independent of the compiler and language server.

```rust,no_run
use wleam_host::{HostOptions, Program, Value};

fn main() -> anyhow::Result<()> {
    let wasm = std::fs::read("application.wasm")?;
    let program = Program::from_bytes(&wasm)?;
    let mut instance = program.instantiate(HostOptions::default())?;

    // An export declared by the module's wleam.abi custom section.
    let answer = instance.call("greet", &[Value::String("friend".into())])?;
    println!("{answer:?}");

    // Run an application's entry point instead of a typed library export.
    instance.call_raw_empty(program.abi().entry.as_deref().unwrap_or("_start"))?;
    print!("{}", instance.take_output().stdout);
    Ok(())
}
```

`HostOptions::default()` captures exact standard output and standard error.
`HostOptions::stdio()` streams to the process's standard streams without retaining
an unbounded copy. Custom output sinks can be combined with `capture_output`.
Instantiation calls `__wleam_init` once if it exists. Reuse a `Program` to create
independent instances; this avoids recompiling the module after a test traps.
Functions compile in parallel with Cranelift's `Speed` optimization level.
Build the embedding application in release mode for realistic loading times;
a debug build leaves the JIT compiler's own code unoptimized. This SDK does not
enable a persistent machine-code cache.

## Custom host functions

The guest describes imports in its `wleam.abi` custom section. Register matching
Rust implementations before instantiation:

```rust,no_run
use anyhow::{bail, Result};
use wleam_host::{HostImports, HostOptions, Program, Value};

fn instantiate(wasm: &[u8]) -> Result<wleam_host::Instance> {
    let mut imports = HostImports::default();
    imports.define("application", "greet", |arguments| {
        let [Value::String(name)] = arguments else {
            bail!("greet expects one String");
        };
        Ok(Value::String(format!("Hello, {name}!")))
    })?;

    Program::from_bytes(wasm)?.instantiate(HostOptions {
        imports,
        ..HostOptions::default()
    })
}
```

The SDK copies incoming strings, integers and bit arrays before invoking the
callback. Callbacks return owned Rust values; the SDK allocates and fills guest
buffers for their results. It does not execute Gleam arithmetic on behalf of the
guest. `Program::with_abi` supplies descriptors for an independent Wasm module
without an embedded descriptor; it cannot replace existing ABI metadata.

## ABI version 1

| Type | Flattened Wasm values | Rust value |
| --- | --- | --- |
| `nil` | no values | `Value::Nil` |
| `bool` | `i32` containing 0 or 1 | `Value::Bool` |
| `float` | finite `f64` | `Value::Float` |
| `int` | sign, pointer, limb count (`i32`) | `Value::Int(BigInt)` |
| `string` | pointer, UTF-8 byte length (`i32`) | `Value::String` |
| `bit_array` | pointer, bit length (`i32`) | `Value::BitArray` |
| `externref` | nullable `externref` | `Value::Externref` or `Value::NullExternref` |

Integer sign is -1, 0 or 1. Limbs are unsigned little-endian 32-bit words, least
significant first. Zero has sign 0 and no limbs; other values have no leading zero
limb. Bit arrays are packed most-significant-bit first and unused low bits in the
last byte are zero. Invalid UTF-8, nonfinite floats, malformed integers, invalid
padding and out-of-bounds memory are rejected.

The caller owns its argument buffers and frees them after the call, including
when the call traps. The callee returns freshly allocated result buffers, whose
ownership transfers to the caller. The receiving wrapper copies the results and
frees them. A returned buffer must not alias an argument allocation. Allocation
uses `__wleam_alloc(byte_length)` and release uses
`__wleam_free(pointer, byte_length)`; memory is exported as `memory`.

When the guest provides the optional pair `__wleam_abi_mark()` and
`__wleam_abi_reset(mark)`, the SDK opens an allocation frame before every typed or
raw zero-argument call. It copies results, releases known buffers and closes the
frame even on a trap. This reclaims bridge buffers allocated inside a guest that
calls a failing host function. Frame exports must appear as a pair; a cleanup
failure never replaces an earlier execution error.

`HostReference::new` stores an opaque `Send + Sync` Rust value in an `Arc`.
Cloning the reference preserves its identity even when separate Wasm `externref`
wrappers are created. References held by the guest remain rooted through
Wasmtime's GC. `reference_id` uses weak registry entries and never extends the
object's lifetime; null has identity 0. `collect_garbage` can request collection
when testing resource lifetimes.

The reserved host namespace `wleam:host/v1` provides exact UTF-8 writes (channel 1
for stdout, 2 for stderr), OS randomness, panic reporting and opaque reference
identity. User import descriptors and implementations cannot override it.

## Compiler integration verification

The executable example takes the actual library artifact built from
`tests/conformance/projects/host_abi` and fails if an ABI assertion fails:

```sh
cargo run --release --locked -p wleam-host --example verify_compiler_abi -- path/to/host_abi.wasm
```

It exercises compiler-generated import and export wrappers for every ABI type,
4096-bit integers, large UTF-8 strings, nonaligned bit slices, opaque identity,
host callback failures and guest panics. The instance must remain usable after
each failure. A successful run prints a JSON assertion summary.
