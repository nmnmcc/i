# Compiler-produced host ABI fixture

Build this library using the real iLang frontend and backend:

```sh
target/cargo/debug/i --project tests/conformance/projects/host_abi --no-cache build --lib
```

The result is `build/wasm/host_abi.wasm` inside this project, accompanied by its
`.abi.json` descriptor. The native and browser harnesses load these exact bytes,
require ABI version 1 and all declared exports and imports, and reject missing
callbacks. This fixture has no application entry point or discoverable tests:
its success is established by host assertions, never by an empty `i test`.

Every `echo_*` export calls the corresponding import in the `custom` namespace.
The harness counts callback invocations and checks their arguments and returned
values. It covers positive and negative 4097-bit integers, finite floats including
negative zero, booleans, Nil, Unicode and NUL strings, memory growth, nine-bit
arrays, and nullable opaque references. Repeated calls must preserve reference
identity and must recover after a callback throws.

`reference_same` compares two independently lifted opaque references in Gleam.
`unaligned_bits` returns a nine-bit slice starting at a three-bit internal offset;
the host must observe bytes `[255, 128]`, bit length `9`, and zero padding.
`panic_now` deliberately raises `host_abi intentional guest panic`; the harness
requires a failure and then verifies that a subsequent valid call succeeds.

The expected descriptor in `expected-abi.json` is the contract shared by the
compiler conformance harness and both host SDK suites. Values that cannot cross
the synchronous ABI, such as records, lists, and functions, are deliberately not
part of this public interface.
