# Wleam JavaScript host

`wleam.mjs` is a self-contained ECMAScript module. It runs in modern browsers and
Node.js without npm runtime dependencies. The compiler can copy it beside its
Wasm output; `wleam.d.ts` describes the same API for TypeScript consumers.

```javascript
import { instantiate } from "./wleam.mjs";

const app = await instantiate(new URL("./app.wasm", import.meta.url), {
  stdout: (text) => document.body.append(text),
  stderr: (text) => console.error(text),
});
app.run();

// Explicit exported functions are described by the compiler's wleam.abi section.
const answer = app.call("app.calculate", [123456789012345678901234567890n]);
```

The module must support WasmGC, typed function references, and tail calls.
`supportsRequiredFeatures()` checks all three without a network request. Loading
compiles the module, validates its ABI metadata, registers host functions,
instantiates it, and invokes `__wleam_init` once. Calling `run()` invokes the ABI
entry, defaulting to `_start`.

The loader checks declared function signatures using Wasm's linker. Typed import
proxies and export checks reject scalar type mismatches before JavaScript can
silently coerce values, including when loading an already compiled
`WebAssembly.Module`. Signature checks and immutable embedded metadata are cached
per module when creating fresh test instances.

Custom functions are synchronous and take ordinary JavaScript values:

```javascript
const app = await instantiate(wasmBytes, {
  imports: {
    database: {
      lookup: (key) => records.get(key),
    },
  },
});
```

The compiler's `wleam.abi` custom section declares the parameter and result types.
`rawImports` is available for low-level Wasm imports that the application manages
itself. The `wleam:host/v1` namespace is supplied by this loader. Supplying `abi`
explicitly is supported for independently produced modules without an embedded
section; it cannot override compiler metadata.

| Gleam boundary type | JavaScript value | Core Wasm representation |
| --- | --- | --- |
| `Nil` | `undefined` or `null` | no values |
| `Bool` | `boolean` | `i32` 0 or 1 |
| `Float` | finite `number` | `f64` |
| `Int` | `bigint` | sign, pointer, unsigned 32-bit limb count |
| `String` | well-formed `string` | UTF-8 pointer and byte length |
| `BitArray` | `{ bytes: Uint8Array, bitLength: number }` | pointer and bit length |
| opaque host type | a JavaScript reference | `externref` |

Integer limbs use little-endian sign/magnitude encoding, with canonical zero
represented by sign zero and no limbs. Bit arrays have exactly the byte length
needed for their bits; unused low bits in the last byte are zero. Lists, records,
and callbacks require an explicit application encoding at the boundary.

Each call copies its arguments into guest allocations. Returned buffers transfer
ownership to the caller, which copies and frees them. All argument buffers are
released even if the call traps or a host function throws. Host callbacks receive
copies, so guest memory growth cannot invalidate their arguments. Guest code owns
the newly allocated buffers returned by a custom host callback. These ownership
rules also apply to non-null allocations of zero bytes.

When the guest exports `__wleam_abi_mark` and `__wleam_abi_reset`, calls establish a
nested ABI allocation frame before lowering arguments. They copy results and
release owned buffers, then restore the frame even after a trap. This also reclaims
guest-created temporary import buffers whose normal cleanup was interrupted by a
host exception. Cleanup errors do not replace the original exception.

The built-in host functions provide output, cryptographic randomness, panic
messages, and per-instance identities for opaque references. The output callbacks
receive the exact text written by the guest. Defaults use `console.log` and
`console.error`, whose presentation can add line formatting. A custom `random`
callback fills the supplied byte array synchronously. Language arithmetic and
Unicode processing execute in the Wasm guest.

`runTests(source, options)` creates a fresh instance for every exported test and
returns `{ name, passed, error? }` results. A trap or panic in one test does not
prevent later tests from running.

## Verification

From the Devenv shell:

```sh
npm ci
npm test
npm run test:browser:host
```

The browser suite uses the same checked-in ABI Wasm fixture in Chromium, Firefox,
and WebKit. Regenerate those fixtures with `bash scripts/build-abi-fixture.sh`.
They test the host contract, not the compiler's language implementation.

Native compiler conformance produces `target/conformance/browser.json`, an array
of `{ name, wasm, stdout, stderr?, tests? }` fixtures. Run `npm run test:browser:compiled` to execute
those exact binaries in all three engines. `npm run test:browser` runs both suites
and fails if the compiler manifest is missing or empty. Set
`WLEAM_BROWSER_MANIFEST` to use a different manifest.

The `host_abi` conformance library additionally exercises the compiler's real
foreign-function wrappers in all three browsers, including opaque equality,
nonzero bit offsets and recovery after host exceptions. After building that
project, `npm run test:compiler:abi` runs the same assertions in Node.js.
