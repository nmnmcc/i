# i JavaScript host

`ilang.mjs` is a self-contained ECMAScript module that runs i programs through
the binary interface version 3 of `docs/spec.md` (Appendix B). It runs in modern
browsers and Node.js without npm runtime dependencies. `ilang.d.ts` describes the
same API for TypeScript.

```javascript
import { instantiate, GuestError, Stop } from "./ilang.mjs";

const app = await instantiate(new URL("./app.wasm", import.meta.url), {
  imports: {
    // Host services (spec §9.7): the import module is the namespace.
    "app:console/v1": { print: (text) => console.log(text) },
    "app:storage/v1": {
      read: (key) => {
        if (!records.has(key)) throw new GuestError(`no ${key}`); // an error of `^ String`
        return records.get(key);
      },
    },
    // External functions (spec §10.1).
    "app:math/v1": { hypot: (a, b) => Math.hypot(a, b) },
  },
});
app.run(); // the program entry

try {
  const sum = app.call("app/math.add", [1n, 2n]);
} catch (error) {
  if (error instanceof GuestError) console.log("error", error.value);
  else if (error instanceof Stop) console.log(error.stopMessage);
  else throw error;
}
```

The engine must support WasmGC, typed function references, tail calls and
exception handling with `exnref`. `supportsRequiredFeatures()` checks all four
with a tiny probe module, without a network request.

`instantiate` compiles the module, reads and checks its `ilang.abi` metadata
(B.6), links `ilang:host/v1` (`panic`, `reference_id` and `cancel`, B.4) and every
function in `services` and `externals`, and calls `ilang_start` once (B.1). The
loader checks every declared signature with Wasm's linker, also for an already
compiled `WebAssembly.Module`. `abi` gives the metadata of a module without the
section; it cannot replace a section. `rawImports` adds other Wasm imports.

## Calls and stops

`call(name, args)` calls a host export (B.2). It gives the value, throws
`GuestError` with `.value` when the function gives an error of its `^ E`, and
throws `Stop` when the program stops. `stopMessage` is the stop message of
spec §11.7 (the reason, then the source positions), or `null` when the module
trapped without one. After a stop, or any other trap, the instance refuses every
call (B.5); make a new one. `unusable` says why.

`run()` runs the program entry, and `runTests(source, options)` runs every test
of the metadata in a new instance (spec §10.2), giving `{ name, passed, error? }`.

| i type | JavaScript value | Wasm values |
| --- | --- | --- |
| `Nil` | `undefined` or `null` | none |
| `Bool` | `boolean` | `i32` 0 or 1 |
| `Int` | `bigint` | sign, address, count |
| `Float` | finite `number` | `f64` |
| `String` | well-formed `string` | address, UTF-8 length |
| `BitArray` | `{ bytes: Uint8Array, bitLength: number }` | address, number of bits |
| custom type with no constructors | any JavaScript value | `externref` |

When a function has errors, its results start with an `i32` tag: 0 for the
value, i for the i-th member of `E` in the canonical order Nil, Bool, Int, Float,
String, BitArray. A host service gives an error by throwing `GuestError(value)`;
the type of the value selects the member.

## Values that are not correct

An argument of `call` that is not a correct value of its type (a `Float` that
is not finite, a string with a lone surrogate, a bit array with bits after the
end, or a value of another type) stops the program with `host: value is not
correct` (B.2).

The answer of a host function that has the right type is given to the module
as it is, also when it is not correct: the module checks it, and stops with the
positions of the call (B.2, §11.7). An answer of another type, or an error that
is not a member of `E`, cannot be given to the module, so the loader stops the
program with `host: value is not correct` and no positions. With the option
`checkReplies: true`, the loader also checks the answers of the right type, and
stops the program for them in the same way.

A wrong number of arguments or an unknown export is an `AbiError` and
leaves the instance usable. A value from the module that is not correct is an
`AbiError` too, and the instance becomes unusable.

## Exceptions of host functions

Nothing is thrown into the module (B.1). Each import is a small Wasm function
that calls the host function, then traps when the host function threw or its
answer stops the program; a trap runs no handler of the module. The host
function gives results of the right types first, which the module never uses.
So when a host function throws, the call of the export throws that same
exception, and the instance becomes unusable.

## Buffers

Buffers follow B.3. For an export, the loader makes the parameter buffers with
`ilang_alloc` and frees them with `ilang_free` after the call, and frees the
result buffers after copying them. For an import, the module owns the parameter
buffers and the loader makes the result buffers. The loader gives empty
buffers the address 0 and does not make or free them; the module frees every
result buffer whose address is not 0, also an empty one.

## Async

A function of an async host service (`@host("…", "async")`) gives a Promise.
The module picks a request id for each async host function that it starts,
and may keep many requests outstanding (B.4). The loader calls the service
during the import, and answers the requests one at a time, in the order their
Promises settle, through `ilang_resume:namespace:label` with the id first.
`callAsync(name, args)` calls an async export (B.5). At status 2 (the module
can go on without an answer) it waits one macrotask, gives an answer that is
done, and otherwise calls `ilang_continue`. At status 0 it reads the results
from `ilang_result:name`. While it waits, and while Wasm runs, other calls are
refused; so a service may not call the instance. `run()` and `runTests` also
wait for async entry points.

The service is called during the import, so the part of an `async` function
before its first `await` runs then. Fix anything that depends on the time of
the call, such as a deadline, before the first `await`.

A request is dropped when the module cancels it with the runtime import
`cancel`, when the entry point ends while it is outstanding, when the program
stops, and when the work of another request fails. JavaScript cannot stop a
Promise, so the loader gives each request an `AbortSignal` and aborts it when
the request is dropped. Call `requestSignal()` before the first `await` of the
service to get it, and use it to clear a timer or abort a `fetch`:

```javascript
import { requestSignal } from "./ilang.mjs";

const sleep = (ms) => {
  const signal = requestSignal();
  return new Promise((resolve) => {
    const timer = setTimeout(resolve, ms);
    signal.addEventListener("abort", () => clearTimeout(timer));
  });
};
```

A service that ignores the signal keeps running, and its side effects still
happen; only its answer is ignored.

## Verification

From the Devenv shell:

```sh
npm ci
npm test
npm run test:browser:host
```

The browser suite uses the same checked-in Wasm fixtures in Chromium, Firefox and
WebKit. Regenerate them, and the feature probe in `ilang.mjs`, with
`bash scripts/build-abi-fixture.sh`. The hand-written ones test the host
contract; `compiled.wasm` is built by the compiler, and runs sync and async host
services, external functions, and host exports of a real program.

## Resource contexts and binding permissions

A `ResourceHost` is one live host identity domain. Reuse it in `resourceHost`
across instances and tests; the SDK otherwise uses a shared default host. Every
SDK entry/export invocation receives a fresh `ResourceContext`. Nested request
roots and workers share that context. Numbers are positive BigInts and never
reused in that host. Resources are looked up by context and kind; named leases
serialize competing operations and reject success after close. A close receipt
joins independent cleanup, including when the close waiter is abandoned.
`cleanupJob()` registers creation/supervision before OS work starts, so teardown
also joins jobs that have not produced a resource yet. `dispose()` closes the
invocation gate and aborts outstanding requests without running i functions.
Direct `instance.exports` access is the raw ABI; use SDK calls for invocation
ownership. Initialization has its own context, closed by the first SDK call or
disposal.

`contextImports` callbacks receive their immutable invocation context first.
They are restricted registrations: without approved declarations, loading fails
with `ungranted host import <namespace> <name>: a trusted binding plan or explicit
declaration grant is required`. Refusing a declaration gives
`ungranted host import <namespace> <name> declared by <module> <declaration>`.
Legacy `imports` registrations deliberately authorize all declarations of the
specified import name, preserving custom embeddings. A resource factory must
use `contextImports` and explicit declaration approval.

`i build` writes a separate `.bindings.json` compiler plan next to an ilang
artifact. It lists every required declaration before coalescing, its package
content identity, module digest, host type/field or external name, and wire
signature. Its artifact digest covers the final optimized Wasm bytes. This is
outside ABI v3. Authenticate the compiler plan and trusted package content
through your build/deployment path. A manifest's own names and digests do not
establish trust. Pass the artifact bytes and plan to `authorizeBindings` with
an explicit approval predicate; instantiate its returned `module` with that
opaque token in `bindings`. A token cannot authorize a separately compiled
module, even one supplied by an untrusted manifest.

The CLI grants its loader-selected library content identity. Its existing
printer is an ordinary public binding. Existing test/custom bindings retain
explicit embedding authority: console print, storage read, clock later, math
hypot; the spec host's test io print, math add/hypot, kv get/check, async
double/echo/nan and bad nan/bits. No IO resource service is implemented here.

Additional existing fixture grants are `custom.echo_string`, `custom.echo_int`,
`custom.echo_bits`, `custom.echo_ref`, `custom.echo_bool`, `custom.echo_float`,
`custom.echo_nil`, `custom:fallible/v1.fallible`, and the compiled fixture's
`app:clock/v1.tick`. They use the same explicit legacy registration authority.

Approval covers the complete authenticated binding plan: the artifact digest
and the entire ordered declaration list, including refused and duplicate entries.
JS authorization tokens retain an immutable snapshot, just as Rust grants do.
Replacing any part of it, even by deleting a refused alias while keeping identical
Wasm bytes, requires fresh explicit approval. Wire-import coverage alone cannot
prove the list complete. Legacy broad registration authority is unchanged.

Every SDK-managed entry into guest code checks the invocation gate, including
continuations, result exports, allocation and each free. Ending during a host
callback prevents return into guest code. Ending during request cancellation at
status zero prevents the later result export. An interrupted invocation leaves
the instance unusable. Synchronous opaque values are returned unchanged, without
reading `then` or `finally`; operation kind and ABI metadata determine whether
invocation handling is asynchronous. Direct raw Wasm export access remains the
embedding's ABI duty.

Teardown closes admission, removes every live entry and publishes close receipts
before running any callback. Reentrant teardown joins the same completion.
Callbacks cannot acquire resources, commit leases, deliver provisional values or
reenter an SDK invocation during this transition.

Cleanup failures are host diagnostics. Close and teardown still complete every
cancellation, lease wake and cleanup starter, and keep their error-free contract.
Thrown cancellation/cleanup errors, rejected cleanup Promises and exceptions in
listeners registered on the supplied AbortSignals (including `onabort`) are
isolated and collected. Drain records with `ResourceHost.takeCleanupErrors()` or
`guest.takeCleanupErrors()`. Each record retains `phase` (`cancel`, `wake` or
`cleanup`) and the original thrown `error`. The queue belongs to the host identity
domain and includes late cleanup; draining it does not enter Wasm or replace the
invocation's original result or failure. There are no implicit console reports
or unhandled rejections for these failures. Cancellation callbacks revoke work
synchronously; register independent asynchronous cleanup with `cleanupJob()`.
If a cancellation callback nevertheless returns a Promise, its rejection is
observed and recorded when it settles, without delaying synchronous revocation.

## Standard services on Node

Pass `standardServices: true` to `instantiate` to supply the standard clock,
stdio, filesystem, networking, process, TLS, arguments, environment, working
 directory and entropy services. Individual `imports` or `contextImports`
registrations override these defaults. Pass `{ args: [...] }` as the option to
set the arguments exposed to an i program. Browser clock and console output are
available; filesystem, networking, process and TLS report `unsupported`.

Run a compiled entry on Node with:

```sh
node bindings/javascript/run.mjs path/to/program.wasm -- arg1 arg2
```

## Fetch HTTP and WASI preview 1

Build `i build --project examples/http-get --target js` and instantiate with
`{standardServices: true}`. The ordinary `std/http.Client` provider uses host
fetch; `i deps --project examples/http-get --target js` shows Fetch and no
Net/Tls dependency. NodeJS keeps the native-style Net/Tls client by default.
Fetch supports request headers/body, response status/visible headers, buffered
bodies and `http.send_stream`. Browser CORS and forbidden-header restrictions
apply, trailers are empty, and redirects/decompression follow host fetch.
Timeout/task cancellation aborts pending fetch and body reads. Network errors
use Connect; available host DNS causes use Dns. Responses belong to the run
resource context and must be closed after streaming.

`i build --project examples/io-fs --target wasi` produces a preview-1 command.
Run with a preview-1 runner (for example `wasmtime run --dir /tmp app.wasm`),
or the included Node runner with explicit preopens:

```sh
node bindings/javascript/wasi-run.mjs examples/io-fs/build/io_fs.wasm '{"preopens":{"/tmp":"/tmp"}}'
```

The runner options also accept args and env. Defaults provide Io, Stdio,
Clock, Entropy, Arguments, Environment and Fs. Fs resolves paths against
preopened directories; temp_dir chooses /tmp or the first preopen. Net, Tls,
Process spawn and WorkingDirectory require explicit providers on WASI and
otherwise fail at compile time. Custom WASI imports keep the blocking ABI.
Checked adapters keep pointers/iovecs/dirents private and validate host extents.

Run `npm test` from this directory (the root package supplies the script).
Focused tests: `node --test bindings/javascript/test/{fetch,wasi}.test.mjs`.
Browser fixture: `npm run test:browser -- tests/browser/fetch.spec.mjs`.
