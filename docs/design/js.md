# Design: JS bindings, target providers, and checked WASI adapters

Status: revision 6, after independent review, 2026-11-10. This note applies
rulings J1–J8. It is a design contract conditional on the explicit spec
amendments in §9, not a claim that these features exist. Only this note is
changed here. In particular the current WASI printer remains until its spec
amendment lands. The wider questions formerly in §10 (removing externals,
merging host forms, portable arbitrary values, other foreign languages) are
out of scope.

Sources, in order: the user's current rulings; `docs/spec.md`; accepted
`syntax.md`, `runtime.md`, `meta.md`; native N9 and its reconciled runtime on
`lane/note-native` at `60f37f2260bf604e426ecb1869291ad9db2af96c`;
the review `.rd/cache/gpt/report-review-js.md`; the previous note. The plan's
“已定的设计”, IO and native stages and the coordinator ledger were read.
Code citations below refer to this lane's base
`48ad582f5f00fcf4d98a9ee14e8afd84a7a225ce`, not to the newer implementation
branches mentioned in the ledger. All i examples use accepted syntax.

## 1. Problem and criteria

i runs on its own under `i run`. A browser or Wasm embedder is one place to
run a program, not the definition of the language. A JS build should supply
its loader and bindings. A WASI build should retain its command behavior and
custom imports. Application and standard-library code should use ordinary
values, functions and environments on both.

The old note left six contracts inconsistent: whole-service requirements
versus partial platform support; JS objects versus integer IO resources;
other-target foreign bindings; request result/error encodings; memory
ownership; and macro execution. It also claimed a kernel only moved values,
although invocation, validation, identity, cancellation and cleanup need
fixed implementations.

Choose by sound sets of values, principal inference, ordinary library code,
standalone behavior, compatibility with Appendix B, complete lifetimes,
structured concurrency and a small implementation that can be tested before
IO integration. Considered designs:

| Design | Benefit | Reason not to choose it unchanged |
|---|---|---|
| Large host services with unsupported stubs and opaque IO objects | Closest to the older runtime table and SDK | Violates J1 and J2; requesting entropy also requests child processes |
| One raw root per target, integer pointers, arbitrary promise results | Small-looking surface, libraries write all protocols | Double free and stale addresses are unsound; constructed errors cannot cross §10.3; native has no accepted universal root |
| Infer only the fields used, erase unavailable values, optimize away other targets | Fewer apparent requirements | Changes whole-service inference and reference identity; optimization cannot justify an observable null |
| Capability-sized providers, checked foreign kernels and ordinary wrappers | Keeps inference and ABI value sets; makes missing capabilities explicit | Needs a specified link pass and checked adapters; chosen |

Keep the first design's explicit SDK factories for resource operations, the
second's ordinary library composition, and the third's removal of *proven*
unreachable code. Do not infer a different type on each target.

The new mechanisms are general. Foreign values serve DOM, cryptography and
numerical packages. Raw outcomes serve rejected promises, synchronous foreign
exceptions and third-party job APIs. Checked buffers serve file output,
packet codecs and compression. Entry providers serve clocks, entropy and
application logging. None requires the checker to recognize a library name.
A nominal type is still a set of constructor values or opaque host values;
all functions keep ordinary value/error/environment types. There is no new
subtyping, field-sensitive service inference, keyword or unstructured task.

## 2. Target provisioning and capability split

**T1 — Entry providers are ordinary values. [J1]** A target build has an
explicit provider plan mapping resolved service type identities to ordinary
i factory functions or existing host registrations. This is build/entry
provisioning, not name resolution or a special lowering for `i/io`. Apply the
same mechanism to a user-supplied provider plan. The shipped plan supplies
standard defaults; it does not discover arbitrary package services by name.
The compiler checks every factory's ordinary type and its dependencies;
provider dependency cycles are build errors. No provider is a new language
attribute. The implementation's internal plan format need not be public in
stage 1.

Infer source types first. An environment statement removes a requirement in
the usual way (spec §9.2). For remaining entry requirements, instantiate a
whole service, never selected fields. Construct its default once per entry
invocation, after top-level initialization, and capture the root/host values
it uses. Replacing `Js` later does not change an `Io` already constructed.
A function that reads `Js` at call time still gets the caller's replacement.
Provider construction cannot introduce async work outside the R2 entry
context. Factories that need requests run inside that context.

A required standard capability with no default and no explicit provider is
a compile error naming the target, service and requiring entry. The user
accepted this cost verbatim: **“接受这个代价” (J1)**. For example:

```i
(i/fs, i/fail)

@main
main = () {
  fail.catch(() { fs.read_file("settings.txt") })
  Nil
}
```

On `js`, the missing `Fs` default is a compile error even though this code
catches every `FsError`, including `unsupported`. Catching an error never
removes a service requirement. Choose an implementation through `Target`,
or give an explicit service before it reaches the entry. Smaller services
make entropy usable; they do **not** restore this fallback pattern.

**T2 — Split by capability, not module. [J1]** Use the following exact split
of runtime §7.4; keep the functions in `i/process` and their existing result,
error, argument-codec and cancellation rules. A wrapper requires only the
service in its row, plus R2 `Await | Owner` when it waits.

| Service / namespace | Raw fields, with runtime §7.4 signatures except J2's Int resources |
|---|---|
| `Entropy` / `i:entropy/v1` | `entropy` |
| `Arguments` / `i:arguments/v1` | `args` |
| `Environment` / `i:environment/v1` | `env` |
| `WorkingDirectory` / `i:working-directory/v1` | `cwd` |
| `Process` / `i:process/v1` | `spawn`, `wait`, `terminate`, `close` |

Keep `Clock`, `Fs`, `Net`, `Stdio` and legacy `Io` separate. `Arguments` and
`Environment` do not drag in `cwd`: preview 1 has no general cwd query.
Entropy, Arguments, Environment and WorkingDirectory have public service
constructors for ordinary fakes. Process follows N9's private raw fields.
There is no aggregate Process requirement on `process.entropy` or
`process.args`. Keep `ProcessError` for these wrappers; changing error names
is unnecessary. This decomposition supersedes runtime §7.4's single service.

**T3 — Defaults describe shipped implementations, not theoretical platform
support. [J1, J4]** The initial synchronous milestone supplies only `Io`,
`Target` and the relevant root. Add the following planned defaults only when
their implementations and tests ship:

| Target / CLI | Root available through Target | Defaults after IO integration |
|---|---|---|
| `Js` / `js` | `Js` | Io, Clock, Entropy |
| `NodeJS` / `nodejs` | `Js` | Io, Clock, Entropy, Arguments, Environment, WorkingDirectory, Fs, Net, Process, Stdio |
| `Wasi` / `wasi` | `Wasi` | Io initially; Clock, Entropy, Arguments, Environment, Fs, Stdio require separately approved preview-1 adapters |
| `Native` / `native` | None in this revision | Accepted native runner's ordinary standard registrations, split as T2 |
| `Host` / `host` | None | Explicit embedder/provider plan; no claim of arbitrary defaults |

A supported JS environment must supply the declared runtime feature profile
(WasmGC, tail calls, exception handling, console, clocks/timers and, at the IO
milestone, secure entropy). This is not a claim about every JS engine or every
browser context. The generated loader checks its required profile before
starting. No insecure random fallback. Deno/Bun compatibility is unverified;
Node compatibility is not inferred from their names. Preview 2 is outside
this note. `--target ilang` remains an alias of `--target host` with no removal
scheduled; emitted legacy host artifacts keep the current Appendix B format.
This preserves native N1's explicit `ilang` usage while adopting the user's
preferred spelling. `i run` and `i test` retain their in-process behavior.

**T4 — Target is an ordinary nominal service value.** Proposed `i/target`:

```i
(js = i/js, wasi = i/wasi)

(Target(..))

Target {
  Js(js: js.Js)
  NodeJS(js: js.Js)
  Wasi(wasi: wasi.Wasi)
  Native
  Host
}
```

Its configured entry provider constructs the actual target. Native is
nullary here: an invented `native.Native` root would contradict accepted
native N3/N4 and runtime §8.1. A future provider bundle needs its own design;
packaging does not depend on it. Host is nullary because an arbitrary
embedder's services cannot be enumerated by the compiler.

This complete example covers all five constructors and needs only Target:

```i
(target = i/target)

@main
main = (t ! target.Target) {
  t |> {
    target.Js(_) -> Nil
    target.NodeJS(_) -> Nil
    target.Wasi(_) -> Nil
    target.Native -> Nil
    target.Host -> Nil
  }
}
```

In a JS clause, bind `! js.Js = j` from its payload before calling JS code;
in a WASI clause, bind `! wasi.Wasi = w`. These branches can discharge their
root requirements. Native and Host clauses cannot discharge arbitrary needs
from their tags. Any such unsupplied requirement remains in the principal
entry type on **all** targets. A test may supply any constructed Target;
linking must follow that actual value, not assume every Target is the default.

The printing example has its required import:

```i
((Io) = i/io)

@main
main = (io ! Io) { io.print("Hello from i!") }
```

It works where Io is provided, including the standalone runner and WASI.
An arbitrary Host embedder still has to provide it (spec §9.7).

**T5 — Custom imports are not missing standard defaults. [J4]** Preserve
spec B.7: required custom WASI services and externals stay imports, with their
requirements reported at build time. A suitable embedder may supply them;
plain preload is sufficient only for the scalar types listed in B.7.
Request-mode custom imports remain blocking and return Ready requests under
R2, with no async hand-off. An explicit host provider plan can also satisfy a
standard service; absent that plan, J1 applies. Do not extend native N4's
closed executable rule to WASI. JS generated artifacts, in contrast, must
supply all their required imports or fail the build; arbitrary embedding uses
Host. None of these policies changes legal service substitution in i.

## 3. Foreign declarations, linking, and loader lifecycle

**F1 — A declaration holds an opaque JS value. [J3]** The new form is
`@js("expression") name: T`, where T expands to one closed nominal type with
no constructors. No holes, type variables, unions, functions or constructed
resource types. The annotation is required. It creates no i constructor or
cast between nominal types. `i/js.Value {}` is one ordinary use; packages can
define `Element {}` or other opaque types. JS is specific to this attribute;
it is not a target-neutral foreign language feature.

Each retained declaration becomes a zero-argument value import returning
`externref`, with a cache in the loader, rather than an imported mutable
global. Its first and only initialization call is emitted in `ilang_start`,
in §2.5 dependency/module/source order alongside ordinary initializers.
The host parses the text as one parenthesized expression in strict ESM scope.
It may name globals but not generated loader locals, other declarations or i
bindings. The compiler does not type-check JS. Syntax failure is a loader
error before startup; an initializer exception stops startup and no entry
runs. Initializers may have host side effects: the old assertion that *only*
Js acts was false. Use function expressions to defer work to an explicit Js
call. The source listing exposes these initializers as foreign code to audit.
No initializer or synchronous call may re-enter i; callbacks are a later gate.

**F2 — Binding retention is specified before optimization. [J3]** All source
is type-checked with the same principal types on every target. Then a required
link pass specializes the known provider values and computes required code
and foreign declarations. Roots include every ordinary top-level initializer
under §2.5, every emitted entry/export/test and its callable dependencies.
Foreign declarations themselves are link declarations, not unconditional
ordinary initializer roots: retain one when a residual read can execute.
This distinction must be added explicitly to §2.5; it does not authorize
removing a side-effecting ordinary top-level binding.

Use conservative constructor propagation: a known constructor selects its
match clause; a set keeps all possible clauses; unknown keeps all clauses.
Propagate through ordinary bindings, actual parameters and captured values;
recursive summaries reach a finite union of constructor alternatives, widening
unknown payloads to unknown. Do not evaluate user computations to prove
termination. Unknown calls retain all possible callees; a closure escaping
analysis retains its body. Specialize a body per known constructor input or
merge its uses conservatively. This analysis does not subtract an inferred
service requirement; a branch must supply its needed service in source.

A retained `@js` on wasi/native/host is a **compile error**, at the declaration
and requiring use. Never replace it by null or a late trap. On js/nodejs the
loader supplies it. Reachability is about the actual provider, including
explicit Target substitutions and exported code, not the name of a pattern.
If analysis cannot prove a binding unused, reject conservatively. Results
must agree at O0/O2/O3: Binaryen is not the link-safety mechanism.

Thus a helper's `@js` used only in a proven Js/NodeJS branch is absent on WASI.
But `x = foreign` as an ordinary top-level initializer requires that binding
even if main later ignores x. Two retained `@js("({})")` declarations on JS
produce distinct objects; repeated reads of one preserve its identity.
Removing an unretained foreign initializer also removes its host effects:
that is the defined foreign-declaration link semantics, and a migration cost
from the old proposal's unconditional initialization promise.

**F3 — Artifacts and versioning.** `name.wasm`, `name.mjs` and `name.d.ts` are
the generated JS artifact. The mjs contains the fixed bridge, retained source
expressions and loader; each instance has its own declaration caches and
entry resource contexts. It starts the module once, then calls main or the
requested host export. JS source may use `import("node:...")` inside a
function and await its opaque promise through §5. There are no static import
declarations inside an expression, implicit Node module variables, or
compiler-generated user callbacks. Standard Node factories may use their
own ESM imports. Their source is fixed support code, not user handwritten JS.

Keep the v3 wire for ordinary service/request calls. Put foreign declaration
text, stable package/module/name identity, generated import name, nominal type
identity, source span and availability in a separate versioned target section
`ilang.js` (initial version 1). A matching generated loader requires that
section and `target: "js"` or `"nodejs"`; old Appendix B hosts reject the
non-host target instead of accidentally loading it. The new mandatory target
metadata is an explicit B.6 extension, not an undocumented v3 runtime export.
The value getters are target imports listed there, not additions to v3's
`ilang:host/v1`. A digest binds the JS manifest to its Wasm artifact; it is
not self-authenticating permission for raw IO (N9 §3.1 still applies).

`bindings/javascript` remains the Host SDK. Generated-target and explicit SDK
usage share encoding and request lifecycle code. Host exports always use the
real SDK conversions and d.ts representations, never a dynamically replaced
Js service; a custom outgoing converter cannot change an export's ABI.

## 4. Synchronous bridge and conversion contract

**F4 — Lower operations use admissible boundary types. [J7]** Public calls
need Js. At the request milestone it is an ordinary bundle:

```i
Js { Js(sync: JsSync, requests: JsRequests) }
```

JsSync is the synchronous raw host service (`i:js/v1`); JsRequests is the
request-mode raw service (`i:js-requests/v1`). The target provider builds the
bundle from their ordinary values. Before the request milestone only the sync
part exists. There is no mixed-mode host declaration. Merely constructing
request function values starts no work; only calling a starter needs Owner.
Public wrappers read the bundle; callers do not separately require its raw
services. Thus the Target payload can discharge both sync and async JS use.
The pure `with_int` helper copies this bundle and its sync service with the
outgoing converter replaced. A fake can reuse opaque seed values or replace
an application service above Js; no constructor for Value is added.

`call(f, args)` gives Value or stops for a thrown exception;
`try_call(f, args)` gives Value with error `Thrown { Thrown(Value) }`.
The fixed bridge returns opaque `Outcome {}` for invocation. Its checked
`is_error: (Outcome) -> Bool` and `value: (Outcome) -> Value` accessors let
ordinary i construct/raise Thrown *after* crossing. Thrown never crosses
§10.3 as an ABI error. Non-callable f produces a caught JS TypeError by the
same rule. `call` uses `panic("js: call threw")` with source positions;
`try_call` preserves the exact thrown value, including null and undefined.

A private argument builder uses `Args {}` with `new_args: () -> Args`,
`push_arg: (Args, Value) -> Nil`, `invoke: (Value, Args) -> Outcome`.
Build a fresh array per call, copy the completed argument sequence at invoke,
and never expose the mutable builder through public i/js functions. List
traversal, result decoding and error construction are ordinary i. An Args
value used by a low-level client remains opaque; its identity, not its
contents, is the i value. Host state can change without mutating an i list.
No List or arbitrary constructed error is added to the wire. Forged opaque
outcomes/builders from a foreign host fail brand validation and stop as a
host fault. This is a dynamic host-protocol check, not a new nominal cast.

A synchronous result declared externref is opaque data, even if its JS value
is a Promise or has a `then` property. Do not inspect or await that property.
The SDK currently rejects thenable synchronous results without this type
qualification (`bindings/javascript/ilang.mjs:836–839`); adjust that general
boundary check for **every** synchronous externref result, not just i/js.
Otherwise `value(outcome)` could not return a Promise for A1 to observe, and
a user object's `then` getter could run outside the explicit call. Ordinary
async service starters still adopt their returned promise as work. The raw
Outcome itself is a private branded object without a then property.

Public `nil`, `bool`, `int`, `float`, `string`, `bits` convert into Value.
`to_nil`, `to_bool`, `to_int`, `to_float`, `to_string`, `to_bits` give the
corresponding i type with error `Mismatch { Mismatch }`. Implement each raw
extractor with a primitive error, e.g. `(Value) -> Float ? Nil`; validate in
the JS bridge *before* returning the Float. The ordinary wrapper maps that
Nil error to Mismatch. This is implementable by §10.3 and avoids using the
SDK's permissive `ofType` predicate as a conversion test.

Exactly one outcome applies to each case below. “Mismatch” means the public
conversion error; “stop” means §11.7, not a catchable mismatch. These are the
real Js defaults; `with_int` and analogous ordinary service-copy functions
may replace outgoing conversions for application calls.

| Case | Outcome | Representation or reason |
|---|---|---|
| i Nil to JS | convert | undefined |
| JS undefined or null through to_nil | convert | Nil |
| i Bool / JS boolean in the matching conversion | convert | boolean / Bool |
| i Int / JS bigint in the matching conversion | convert | Exact arbitrary-precision bigint / Int; never Number |
| i Float / finite JS number in the matching conversion | convert | binary64; negative zero follows existing SDK/spec equality |
| i String / well-formed JS string in the matching conversion | convert | Unicode scalar sequence, UTF-8 at the wire |
| i BitArray to JS, including non-byte lengths | convert | Fresh `{ bytes: Uint8Array, bitLength: number }`, zero padding |
| Correct SDK bit-array object through to_bits | convert | Snapshot copy of exactly ceil(bitLength/8) bytes and exact bit length |
| Wrong JS primitive kind, including Number through to_int | Mismatch | No coercion |
| NaN or either infinity through to_float | Mismatch | Not in the set Float |
| Lone surrogate through to_string | Mismatch | Not in the set String |
| Bare Uint8Array through to_bits | Mismatch | Missing bit length |
| Bad bit length, wrong byte count, nonzero padding or detached bytes | Mismatch | Not a canonical SDK BitArray |
| Getter/proxy throws while inspecting a conversion input | Mismatch | No valid snapshot was obtained |
| Shared-memory-backed byte view through to_bits | Mismatch | No stable snapshot contract; caller must make a private copy |
| Arbitrary JS result or thrown value entering Value | convert | Opaque reference; inspect only with explicit converters |
| i lists, records, functions or File as a declared ABI value | reject at compile time | §10.3; use ordinary codecs or wrappers |
| Constructed Thrown/Mismatch as an ABI error | reject at compile time | §10.3 error list |
| Invalid value from a typed ABI import/export path | stop | Existing `host: value is not correct`; converters do not weaken B.2 |
| Allocation/stack exhaustion while copying or encoding | stop | §11.7 |
| JS call throws and the public operation is call | stop | F4's panic; try_call instead decodes Thrown |

For bit conversion read properties once, validate the private snapshot, then
encode that snapshot; do not validate one getter result and encode another.
Mutation of JS bytes afterwards cannot mutate an i BitArray. Outgoing bytes
are also copies; no view aliases guest linear memory. No operation silently
pads a bit array, rounds an Int, repairs Unicode or maps a nonfinite number.
A plain `@js` value may be a promise or object; it is not thereby an i Float,
String or IO resource. JS object identity uses the existing reference identity
bridge. Distinct nominal types remain distinct under spec §4.1/§11.4.

## 5. Requests, cancellation and IO ownership

**A1 — Await an opaque outcome, then decode in i. [J5]** The raw request
service has `observe: (Value) -> Request(Outcome)` and
`start: (Value, Args) -> Request(Outcome)`, with Owner added by runtime §5.2.
There is no `Request(Result(Value, Thrown))` import and no general request map.
`observe` adopts an already-started promise/thenable; fulfilled and rejected
values both settle as an Outcome success at the ABI level. Accessor decoding
from F4 runs after `request.await`. Public `await(promise)` therefore has
value Value, error Thrown, and environment `Js | Await | Owner`. It obtains
JsRequests from the ordinary Js bundle (F4), not from another entry requirement.
Use distinct raw JsSync and request-mode JsRequests service types;
do not put mixed host modes in one declaration. The public starter similarly
builds Args in i and invokes the request service. Arbitrary promises may keep
working after cancellation; observe only withdraws delivery.

**A2 — Standard IO starts with cancellation already installed. [J5]** Use
runtime §8.2's standard request factories. The request slot/id and generation
exist before the host starter runs. During that import, the starter reads
`requestSignal()` synchronously, records the immutable entry ResourceContext,
registers abort cleanup and copies input buffers before returning. It then
starts the OS/JS operation. The general start operation calls its JS function
with `(signal, ...args)` under the same protocol, capturing synchronous throws
as rejected outcomes. A function may ignore the signal, but is then only an
uncancellable foreign operation, not a standard IO implementation.

Abort removes the outstanding registration before invoking listeners. Late
completion is ignored using request identity/generation, even after id reuse;
resource-producing late results must also be closed. Completion delivery is
serialized through R2's one root continuation. Status 2 yields a macrotask;
no microtask-only spin. Owner finalization cancels Waiting and unclaimed Ready
requests under runtime §5.4. A raw outcome has no implicit IO-resource cleanup:
standard resource factories use the explicit guards and table below. No
promise is a detached i task, and callbacks/re-entry are not authorized here.

**A3 — All IO targets use native N9. [J2]** File, Listener, Stream and Child
have private constructors with one Int field, e.g. inside i/fs:

```i
(File)

File { File(Int) }
```

Raw resource services pass Int. No JS object or fd itself is an i IO resource.
Public wrappers wrap validated returned numbers and unwrap arguments in
ordinary i. Export the service type names needed in public environments,
but hide raw constructors, fields and live field functions. No public helper
may wrap caller numbers or accept a caller-created raw resource service;
otherwise a fake can launder a guessed live number. General nominal field
privacy and N9 §3.1's trusted declaration grants are prerequisites, including
external aliases and import coalescing. A namespace spelling is not a grant.
Use application-level service fakes or explicitly granted host test fixtures.

One ResourceContext per entry invocation owns the resources across its workers,
Roots and nested task runs. A host-wide positive arbitrary Int allocator
never reissues numbers, including failed/provisional numbers; JS uses BigInt.
Fresh tests/export invocations get fresh contexts. Check context, kind and
liveness on every operation; unknown/closed/foreign/wrong-kind gives `closed`,
close gives Nil. OS descriptors may be reused privately, language numbers may
not. Allocation failure stops before exposure and disposes of the OS result.

Lookup, operation leases, completion and close have one commit order. Same
direction competitors give `busy`; completion before close retains its answer;
close before completion wakes it with `closed`. File cursor operations share
one lease; Child has one wait lease. Idempotent close joins the cleanup receipt;
child close waits for reaping. Cancelling close cannot undo cleanup. Keep
runtime's blocking-file cancellation, stdin broker and partial-write rules.

The context owns provisional resources before delivery. A host drop guard
closes an undelivered result; after Ready publication an ordinary i guard
handles abandonment before Take. Take-to-wrapper conversion has no scheduler
step. Forgotten Ready or Taken resources remain owned until close/context end.
Register pending spawn jobs before OS creation so teardown catches late
children. Entry end, stop, trap, host failure, abandonment and disposal close
the start/delivery gate and run host cleanup independently of i finals or GC.
These are N9.1–N9.5, not new JS-specific resource semantics.

**A4 — Keep resource factories, share their implementation. [J2, J5]**
Generated Node builds package the same granted raw factories as the SDK;
ordinary i wrappers still define public service composition, errors and
protocol codecs. A generic Js root cannot safely manufacture Fs from an
arbitrary user-created Js and expose its private raw numbers. Withdraw the
old “every service is built only on Js” promise for resource services. Likewise
native retains its accepted raw namespaces; no Native root redesign is a
prerequisite. Host SDK embedders may explicitly choose unsupported stubs;
a generated target's automatic plan must omit absent capabilities under J1.
This reconciles the factory mechanism with the new default policy without
pretending that OS cleanup can be ordinary i code after a stop.

## 6. Checked WASI memory and the first adapter

**W1 — Buffers are checked tokens, never addresses. [J6]** The public root's
memory fields below are synchronous. Token is an Int internally naming a live
allocation in this instance's dedicated checked-buffer table. The table never
contains ABI argument/result buffers, runtime scratch, GC data or allocator
metadata. Export no operation giving a token's address. Allocation produces
zeroed bytes and a fresh positive token, never reusing one in that instance.
End of entry/startup failure/stop frees remaining checked buffers. A token
from another instance is not a portable memory reference; no library IO value
wraps it. Root APIs expose operations, not the table.

| Field | Type |
|---|---|
| alloc | `(size: Int) -> Int ? String` |
| free | `(token: Int) -> Nil ? String` |
| store | `(token: Int, offset: Int, bytes: BitArray) -> Nil ? String` |
| load | `(token: Int, offset: Int, size: Int) -> BitArray ? String` |
| fd_write | `(fd: Int, token: Int, offset: Int, size: Int) -> Int ? String` |
| proc_exit | `(code: Int) -> Nil ? String` |

This deliberately replaces the old four raw-address fields and the promise
of an unchecked field for every preview-1 function. `Wasi` is an ordinary
service type with this checked signature. Its compiler-provided implementation
is a new adapter, not the existing generic Int ABI lowering. Library helpers
may wrap buffer tokens privately and add structured cleanup, as ordinary code.
Possession of the root grants use of its buffer table; tokens are not secrets.

Validation order is fixed: validate scalar ranges/whole-byte inputs, then
look up a live token, then check its extent. Negative sizes/offsets, an fd or
exit code outside unsigned 32 bits, and non-whole-byte store input give
`invalid`. Unknown/nonpositive/freed tokens (including double free) give
`closed`. A valid token with offset or size outside its allocation gives
`bounds`. Use mathematical checked addition, never wrapping pointer arithmetic.
For empty loads/stores require `0 <= offset <= allocation size`; they do not
bypass liveness. An allocation size above memory32's addressable range gives
`invalid`; exhaustion within that range stops for memory exhaustion.
Successful free removes exactly one table entry before freeing its allocation;
there is no size argument or interior-pointer free. Load returns a copy.

Tokens are identifiers, so `token + 1` is **not** a pointer into token. It may
name another allocation only if that token is live in this authorized table;
there is no promise of unforgeability. To access a subrange pass token and
checked offset. Stale tokens cannot become valid after allocator address
reuse. A call pins its allocation until it returns. These synchronous fields
do not yield; no i callback can free a pinned buffer inside a host call.
For future async adapters, retain a lease or copy before returning; they
cannot inherit raw addresses from this synchronous contract.

**W2 — The kernel understands checked layouts. [J6]** For fd_write, validate
W1 inputs; allocate private scratch for one iovec and a u32 count; translate
the checked slice to the real pointer; call the runtime-owned preview-1 import;
then free scratch on every returning path. On errno != 0, raise String
`errno:<unsigned decimal>`. On success return the number of bytes written,
0..size; a larger count is an invalid host value and stops. No user-controlled
pointer-bearing bytes are ever passed as an iovec. Future adapters require
reviewed pointer/length/output layouts; a scalar-width table alone is not
sufficient. Use a table for the raw i32/i64 signatures, checked signedness and
result shape, with checked layout adapters where necessary. No promise that
every function needs identical code remains.

proc_exit validates its unsigned code, runs context teardown, then calls the
existing raw `(i32) -> ()` import. Its i signature is Nil because §10.3 has no
Never result; successful execution never returns an i Nil. If the host returns
unexpectedly, trap and retire the instance. Invalid code raises `invalid`
before teardown or exit. There is no fabricated errno/result from a void
Wasm call. Teardown does not run i finals; explicit process exit is termination,
not a way to detach work. WASI resource cleanup must finish before this call.

**W3 — Ordinary printing, with existing behavior preserved.** The proposed
Io factory captures Wasi and makes a closure that UTF-8 encodes text and LF,
allocates/stores a buffer, and repeats fd_write on its remaining slice.
Positive short writes advance the offset; zero progress or a reported write
error gives up; release the buffer on every returning path. Give Nil, never
an IO error, as legacy Io requires. Stdio's error-reporting API stays distinct.
This moves encoding, looping and output policy into i, while W2 keeps unsafe
layout construction inside the kernel. The review's raw-iovec prototype is
evidence for the loop, not verification of this checked adapter.

**W4 — Runtime stops and custom imports stay. [J4, J6]** Keep the independent
runtime fd_write/proc_exit stop path, including startup stops, source positions,
partial writes and exit 1. It cannot call a replaceable i Io. Retain `_start`,
no compiled tests, and blocking custom imports. User declarations of the
runtime-reserved fd_write/proc_exit imports remain errors under B.7; W2 reuses
those internal imports rather than declaring duplicates. Do not remove the
shared runtime write loop when retiring the special Io lowering.

## 7. Macro boundary

**M1 — Extend the existing rule, not macro authority. [J8]** In meta §4.3,
add `js` to built-in attribute names and `@js` to the caller-origin restriction
beside external/host/main/test. A dependency macro cannot invent it; a caller
may hand its own unmarked attribute to a macro that preserves it. JS text as
quoted syntax data is not an executable JS binding.

In meta §8, a compiler-run macro program holds no executable `@js` binding,
value getter, Js/JsRequests provider or JS initializer. Compute executable
needs through macro entries and all ordinary top-level initializers; if they
need one, reject before instantiation, even when no macro entry needs Target.
Unused runtime functions containing such declarations can be excluded from
the macro program, but cannot leave an initializer behind. Macro quotation of
a declaration is data and does not execute it. Caller-origin checks still
apply when that data becomes an expansion. No file-reading/d.ts import tool,
callback bridge or JS evaluator is part of the compiler's macro sandbox.

## 8. Evidence from code and implementation regions

These are **read-code facts**, not observations of the proposed implementation.
Every implementation statement in this note refers to the following anchors.
The review's probes are reported evidence only; they were not rerun here.

| Fact checked in this lane | File and lines |
|---|---|
| Host provisioning constructs all fields; current WASI Io selection is special | `crates/ilang-wasm/src/lower.rs:969–1005`, `:2600–2605` |
| Ordinary top-level initializers run in program order | `crates/ilang-wasm/src/lower.rs:776–807` |
| Metadata is v3, lists services/externals, omits WasiStdout and marks WASI | `crates/ilang-wasm/src/lower.rs:2649–2690` |
| Current IR targets are Ilang/Wasi; special print import kind exists | `crates/ilang-wasm/src/ir.rs:642–665` |
| Checker recognizes external/host attributes, not js | `crates/ilang-check/src/resolve.rs:596–647` |
| ABI checker admits only built-ins and constructorless types | `crates/ilang-check/src/resolve.rs:1650–1677`; `crates/ilang-check/src/types/infer.rs:528–537` |
| Nominal field selection enumerates constructors without a visibility check in that enumeration | `crates/ilang-check/src/types/infer.rs:1538–1614` |
| SDK bits retain length/padding; correct validates finite/scalar values; ofType is weaker | `bindings/javascript/ilang.mjs:324–378` |
| SDK wire lowering/lifting copies buffers and retains bit length | `bindings/javascript/ilang.mjs:560–608` |
| SDK currently rejects thenable synchronous results before lowering | `bindings/javascript/ilang.mjs:836–839` |
| SDK checks reference identity separately from value encoding | `bindings/javascript/ilang.mjs:757–770` |
| Signal is available during start; registration captures request identity; cancel deletes before abort | `bindings/javascript/ilang.mjs:26–36`, `:794–825`, `:744–755` |
| Late settlement checks entry identity; dropping clears before abort; status 2 yields macrotask | `bindings/javascript/ilang.mjs:494–507`, `:884–925` |
| Allocator free links the given address with no ownership validation | `crates/ilang-wasm/src/runtime/mod.rs:369–374` |
| WASI fd_write returns i32, proc_exit has no result; stops use a private short-write loop | `crates/ilang-wasm/src/runtime/mod.rs:255–256`, `:273–304` |
| Custom WASI services, blocking async and host references have regression tests | `crates/ilang-cli/tests/wasi.rs:313–355` |
| CLI checks before compiling/optimizing; current target option defaults to Ilang | `crates/ilang-cli/src/lib.rs:89–119`; `crates/ilang-cli/src/main.rs:44–51` |
| Legacy Io writes a line and ignores output errors in the Rust host | `crates/ilang-host/src/services.rs:44–69`; declaration `lib/i/io.i:11–14` |

Implementation handoff, with changed regions (new files are explicitly new):

| Stage | Files/regions and required change |
|---|---|
| Front end | `crates/ilang-syntax/src/ast.rs:49–63`, `crates/ilang-check/src/resolve.rs:596–647`: represent/validate foreign declarations; preserve spans and resolved nominal identity; accepted-syntax parser/formatter work must use their landed owners' current regions |
| Provider/link pass | New target-provider and foreign-reachability modules beside `crates/ilang-wasm/src/lower.rs`; integrate before `:776–807` and `:969–1005`; do not modify principal inference |
| Target metadata | `crates/ilang-wasm/src/ir.rs:642–675`, `lower.rs:2649–2690`; new JS artifact emitter and versioned target manifest |
| JS bridge | `bindings/javascript/ilang.mjs:324–378`, `:560–608`, `:784–849`: share validators, add snapshot converters, target getters/outcomes/Args and loader; treat synchronous externref results as opaque, including promises; new i/js and i/target modules |
| WASI prototype | `crates/ilang-wasm/src/runtime/mod.rs:255–304`, `:369–374`: new checked table and adapter beside allocator, keep runtime stops; new i/wasi; retire `WasiStdout` in `lower.rs:989–990` and `ir.rs:642–644` only after §9 amendments and parity tests |
| CLI | `crates/ilang-cli/src/main.rs:44–51`, `src/lib.rs:89–123`: target names/alias, provider diagnostics and three-file output |
| R2 then IO | Request fields and lifecycle per runtime §§5/8; shared JS start/cancel regions above; split process services before IO uses them; new standard factories obey native N9; general field privacy in `infer.rs:1538–1614` is a prerequisite |
| Macro integration | M2/M4 owners add M1 to the attribute origin checks and executable-needs validator described in meta §§4.3/8; no existing macro implementation is claimed here |
| Regression tests | Extend `crates/ilang-cli/tests/wasi.rs:313–355`; new JS-target/compiler diagnostics and generated-artifact tests; SDK and browser fixtures in their existing suites |

Order: land the source reconciliations first. On landed C and accepted syntax,
prototype synchronous JS/linking and the checked WASI printer. Integrate
promises only after R2's typed Request/Owner layer; integrate standard IO only
after T2/A3 and binding grants. Preview-1 adapters beyond the printer are a
separate target stage, not implicit IO scope. Native packaging remains after
R2 as accepted and needs no invented Native root. Typed wrappers wait for M4
and M1. Callbacks require a separate re-entry/environment/lifetime design and
ABI review; they do not block ordinary WASI IO.

## 9. Replacement source text and conflicts

This section is the precise amendment handoff, not an edit to another lane.
**Until the coordinator lands it, preserve the current spec on every conflict.**
The review and old proposal cannot override the spec. No ruling requires
widening the §10.3 ABI value/error lists.

| Source section | Replacement/addition |
|---|---|
| Spec §2.2/§2.5; syntax §2.5 | Add js to built-in attributes. Replace the external-only declaration restriction with: “A declaration requires external or js, or expansion by a macro. A js declaration has the closed opaque type and foreign link semantics of §10.1. It is initialized once during start only if retained by the target link pass; ordinary bindings retain this section's initialization rules.” Add F1's ordering and F2's roots/conservative analysis verbatim as the foreign-declaration subsection. |
| Spec §4.3/§4.7 | Keep sets, nominal identity and principal inference. Add js to the mandatory-full-annotation exceptions. Explicitly say target linking does not change inferred types. No general cast or request-map rule. |
| Spec §9.2 | No semantic change. Add the captured-provider versus call-site replacement example described in T1. |
| Spec §9.7/§10.2 | Keep general host imports and substitution. Replace entry's host/Assert-only restriction with: “An entry may also require a service supplied by its explicit target provider plan. The plan constructs ordinary service values; missing required standard capabilities are build errors, including when all operation errors are caught. Providers do not alter inferred source types.” Use T1–T5 for Target, capability tables and explicit host plans. Retain legacy Io's line/no-error behavior. |
| Spec §10.1 | Retain externals. Add F1–F3 as “JavaScript value declarations”: full nominal annotation, supported targets, source scope/order, retained getter imports, compile-time unavailable errors, loader failures and no re-entry. |
| Spec §10.3 | Keep the lists unchanged. Cross-reference R2's existing outer Request/Result declaration forms. “IO wrappers over Int and constructed Thrown/Mismatch are not boundary types; their libraries pass Int or opaque Outcome and decode in i.” |
| Spec §11.7 | Keep host-value, memory and stack stops. A bridge call that does not expose exceptions uses F4's panic; malformed raw outcome/host reply uses the existing host-value stop. Do not convert these into Mismatch. |
| Spec B.1/B.3 | Keep ABI allocation ownership. “Checked Wasi buffers are separate table-owned allocations and are not addresses supplied to ilang_free by i code.” Use W1/W2 as the checked root contract; ABI exports remain host machinery. |
| Spec B.4–B.6 | Keep v3 ids, cancel, statuses and import-time re-entry restrictions. Add F3's JS target marker/ilang.js v1 metadata and retained getter imports, without changing the host artifact contract. Mandatory changes to runtime imports/statuses/host-called exports still require a version bump. |
| Spec B.7 | Replace only the special Io implementation bullet after adoption: “The target's Io provider writes UTF-8 text plus LF through the checked Wasi adapter; positive short writes are retried, zero progress/errors give up, and print gives Nil.” Keep it absent from external required-import metadata because internally provided. Keep custom imports, runtime-reserved import names, stops, `_start`, blocking calls and no tests. Add the no-result proc_exit adapter rule W2. |
| Runtime §§7/7.4 | Use native branch's reconciled N9 resource text, not this lane's older opaque types. Split process fields by T2, preserving other signatures/errors. |
| Runtime §8.2 | Generated js/nodejs defaults follow T1–T3, with no automatic unsupported stubs. Explicit Host factories remain configurable. Generated builds share A4's factories. Keep WASI custom imports; list printer as this target project's separate adapter stage, and do not promise later adapters before they land. |
| Meta §4.3 | Add js to the built-in names and @js to the caller-origin list, exactly as M1. |
| Meta §8 | Add: “A macro program contains no executable JS binding or initializer. Reject required js declarations before instantiation; quoted syntax remains data.” Apply M1's full-needs rule. |
| Native N1/N3/N4/N9 | Keep runner ABI, registrations, packaging order and N9. Document host as the preferred ilang alias, and the nullary Native tag; do not add a Native root. |

Concrete conflicts recorded: current spec §2.5 forbids js declarations;
§10.2 does not admit ordinary Target/provider services; §9.7/B.7 mandate the
special WASI Io implementation; B.6 lacks JS target imports. These block
claiming the proposal is already spec-conforming, not completing this note.
Runtime §8.2's automatic unsupported browser defaults conflict with J1;
its no-preview-adapter scope must explicitly coexist with this separate
printer project. This lane's opaque resources are superseded by the supplied
native branch under J2. No conflict remains with native N9 or custom-import
preservation under J4. Replacing resource factories or adding a Native root
would create new conflicts, so neither is part of this revision.

## 10. Required tests and adversarial revision

Expected results below are tied to rules. These are implementation fixtures,
not claims that future APIs compile in the baseline. Use gates and counters
for cancellation/cleanup, not sleep timing. Run availability tests at O0/O2/O3.

| Test | Expected result and rule |
|---|---|
| process.entropy(32) on supported JS profile | 32 bytes; only Entropy plus request services required, no Process import (T2) |
| process.args on WASI without/with explicit adapter plan | Missing Arguments diagnostic / decoded arguments; no spawn/cwd requirement (T2/T3) |
| fs.read_file inside fail.catch on js | Compile error naming Fs; no executable unsupported handler (T1/J1) |
| Same nominal service used through one field | Whole service type; no target-dependent principal type (T1) |
| Capture real Js in Io, then substitute Js | Old Io keeps captured implementation; a later direct JS call uses substitute (T1) |
| Five-constructor Target match | Exhaustive; entry needs Target only (T4) |
| Unavailable @js in known-other-target clause | No declaration getter/import on WASI (F2) |
| Unavailable @js in unknown call, ordinary initializer or explicit fake Target branch | Compile error, no null, at all optimization levels (F2) |
| Two @js object expressions and repeated reads | Distinct objects compare False, same declaration True (F1/F2) |
| Custom WASI Float service and existing greet fixture | Import retained; supplied host result preserved (T5, wasi.rs:313) |
| One-bit round trip; mutate JS copy afterwards | `<<1:u1>>`; original i value remains that one bit (F4) |
| Nonfinite number, lone surrogate, malformed bits | Mismatch at explicit converters; typed invalid ABI reply still stops (F4) |
| Conversion getter returns good value once, bad later | One snapshot used; no unchecked second read (F4) |
| Sync call returns a Promise or an object with a throwing then getter | Return that opaque Value without touching then; only explicit observe adopts it (F4/A1) |
| Promise rejects with null or arbitrary object | await raises Thrown of that same raw value after wait (A1) |
| Started IO cancelled, id reused, old completion arrives | Old response cannot resume new request; provisional resource closed once (A2/A3) |
| Stop with waiting IO and late spawn | No i finals, no new delivery, registered job cleans/reaps late child (A3/N9) |
| Close races a gated read | Close first gives closed; completion first preserves bytes; competing read gives busy (A3) |
| Attempt File(1), raw field projection, or raw alias without grant | Privacy error / permission denial; no live table access (A3) |
| Resource number above 2^53, stale number from previous entry | Exact BigInt round trip / closed; never new resource (A3) |
| alloc/free/free, then two allocations | Second free Error("closed"); new tokens differ; live allocations never overlap (W1) |
| Freed token reused after physical address reuse | closed; no access to new buffer (W1) |
| Negative offset, oversized slice, empty slice at end, one-bit store | invalid, bounds, empty BitArray, invalid respectively (W1) |
| Attempt to store at an ABI/runtime address interpreted as a token | closed unless it independently names a checked allocation; never arbitrary memory (W1) |
| Printer with two-byte host writes | Exact UTF-8 text plus LF, zero remaining allocations (W2/W3) |
| fd_write reports count above size | host-value stop; no underflow/repeat loop (W2) |
| proc_exit valid/invalid, malicious host returns | Raw import has no result / invalid error without exit / trap (W2) |
| Startup panic with no Io and three-byte writes | Full reason/source positions on stderr, LF, exit 1 (W4) |
| Dependency macro invents @js; compiler-run initializer needs it | Expansion error / macro-program compile error (M1) |
| Caller supplies @js to wrapper macro; quote stores its text | Origin rule permits preservation / syntax data does not execute (M1) |

### Attack that changed the draft

These are concrete source/sequence attacks; declarations of future services
are fixtures. They exposed issues beyond merely copying the review fixes.

**Soundness: address reuse and malicious conversion.**

```i
(i/fail, wasi = i/wasi)

probe = (w: wasi.Wasi) {
  p = w.alloc(8)
  w.free(p)
  again = fail.catch(() { w.free(p) })
  q = w.alloc(8)
  stale = fail.catch(() { w.load(p, 0, 1) })
  (again, stale, p == q)
}
```

Checking only physical bounds still accepts stale access if q reuses p's
address. W1 now separates never-reused tokens from physical allocations:
(Error("closed"), Error("closed"), False). A JS bit object whose `bytes` getter
changes between validation and encoding similarly defeats a shallow check;
F4 now snapshots once and rejects shared/detached storage.

**Cannot be built: constructed errors and mixed request modes.**

```i
(i/request)

Value {}
Thrown { Thrown(Value) }

@host("probe:js/v1", "request")
Bad { Bad(wait: (Value) -> request.Request(Result(Value, Thrown))) }
```

§10.3 and runtime §5.2 reject this declaration. A1 now returns only opaque
Outcome, with separate synchronous accessors and i decoding. JsRequests is
separate from synchronous JsSync, with both inside the ordinary Js bundle.
Otherwise a mixed raw host declaration would impose request semantics on
converters, or a second public environment requirement would escape a guarded
Target branch. Constructing the bundle performs no request. W2 likewise uses a Nil
i signature for the raw void proc_exit instead of an inadmissible Never ABI
result. Raw ABI fields returning File(Int) also remain rejected under A3.

A further concrete buildability probe is:

```i
(js = i/js)

@js("() => Promise.resolve(7n)")
seven: js.Value

@main
main = () { js.to_int(js.await(js.call(seven, []))) }
```

The current SDK's unconditional synchronous thenable rejection would reject
the promise when the Outcome accessor returns it as Value. F4 now requires a
general fix: synchronous externref results are data; only request starters
adopt promises. The analogous `@js("() => ({ get then() { throw 7; } })")`
result must remain opaque until an explicit operation inspects it. This is
an implementation finding from the cited SDK source, not a run of new APIs.

**Source conflict and reachability: shadowed Target and startup use.**

```i
(js = i/js, target = i/target)

@js("({})")
object: js.Value

uses = (t: target.Target) {
  t |> {
    target.Js(_) -> { object == object }
    target.NodeJS(_) -> { object == object }
    target.Wasi(_) -> True
    target.Native -> True
    target.Host -> True
  }
}

@main
main = (t ! target.Target) { uses(t) }
```

On WASI with its default Target, no binding is required. Add an ordinary
top-level `forced = object`, or call uses with a locally constructed Js value:
the binding is required and the build fails. Treating all Target values as
the selected target, or deleting ordinary initializers by unused-name
analysis, was wrong. F2 now roots all ordinary initialization, follows actual
values, and rejects unknown uses. Its foreign-only retention exception is an
explicit spec amendment. The factory attack `make_fs(fake_js)` returning a
live guessed Int also defeats N9; A4 removes public root-to-resource factories
and preserves granted raw factories instead.

### Two passes after the fixes

Pass 1 revisited the three programs above with zero-size buffers, stale tokens,
interior offsets, sequential entry contexts, getters throwing during copies,
resource fakes, raw aliases and pending close. It also traced synchronous
try_call and promise rejection through the exact §10.3 raw signatures, and
checked ordinary initializer order against F1/F2. It repeated the promise-as-
opaque-data attack after the generic synchronous externref fix, and checked
that Target's Js bundle discharges both raw service dependencies. No further
serious defect was found by this manual analysis.

Pass 2 changed the roots: exported helper versus main, a captured or replaced
Target, an unknown higher-order caller, a macro program's non-direct initializer,
and a startup stop without Io. It checked id reuse versus late completion,
Ready abandonment, custom WASI request imports, proc_exit's absent result,
all five Target branches, and native's no-root provider contract. No further
serious defect was found. Conservative rejection is intentional when F2
cannot prove a foreign binding absent. These passes are not a compiler,
scheduler or OS proof.

Checks run for this revision: all 10 i code blocks parsed with the cached
accepted-syntax compiler `i-1df2fe0` using `fmt --check` on in-memory files
(formatting differences allowed). This checks grammar, not future names or
types. A bounded Python model of W1 visited 90 states and 165 transitions,
including physical-address reuse, with no token reuse or overlapping live
regions. A separate three-order model checked cancel/id-reuse/old completion
under A2; no old result reached the new request. These are abstract-model
checks, not implementations. The text audit checked balanced fences, table
widths, ruling/review coverage and cited code paths; `git diff --check` passed.
No Cargo build or runtime integration suite was run for this note-only change.
The review's cached-compiler probes were read, not rerun.

## 11. Risks and coordinator questions

- **Land the spec/provider amendments before implementing them?** Recommended:
  yes, as one reviewed source update with the corresponding runtime/meta text
  in §9. Preserve current spec behavior until then. This lane owns only js.md.
- **Keep Native nullary and keep raw resource factories?** Recommended: yes;
  it follows accepted N9 and avoids an unapproved universal Native root or a
  public raw-number injection path. A future explicit capability bundle can
  be designed independently of packaging.
- **Approve the smaller checked WASI adapter surface?** Recommended: W1/W2,
  one-buffer fd_write first. Pointer-layout-aware adapters are necessary for
  safety; do not restore raw addresses or promise all preview-1 functions
  from scalar signatures alone.
- **Adopt the foreign-only retention/initialization rule?** Recommended: F1/F2,
  with conservative errors and identical O0/O2 behavior. Its effect removal
  is explicit; ordinary top-level code keeps §2.5. Global-only ESM scope and
  dynamic import inside functions settle stage-1 module loading.
- **Which team owns the provider plan and binding-grant trust plumbing?**
  Recommended: target integration owns general providers/linking; IO owns
  native N9's general privacy/grants and shared context-aware factories;
  neither treats a self-signed manifest as authority.

Unverified: inference/link-pass implementation, all new API fixtures, generated
JS artifacts and browser support profiles, checked adapter code, resource
factories, performance/size costs, and future callbacks. Each JS argument may
still need a bridge crossing; checked WASI printing allocates/copies. There
are no measurements here. Host unavailability during a run remains an ordinary
operation error where a supported provider exists; it is different from a
missing default at build time.

## 12. Review disposition

| Review finding | Changed rule and location |
|---|---|
| 1 blocker: service granularity and caught unsupported | T1/T2/T3 (§2), exact split and accepted cost; §9 runtime handoff |
| 2 blocker: resource representation and cleanup | A3/A4 (§5), native N9 on every target; W1/W2 (§6) never expose OS fds as File |
| 3 major: other-target foreign imports/null | F1/F2/F3 (§3), required target linking and compile errors |
| 4 major: removal of custom WASI imports | T5 (§2), W4 (§6), B.7 handoff (§9) |
| 5 major: inadmissible promise errors and late cancellation | F4 (§4), A1/A2 (§5), opaque outcomes and start-time signal |
| 6 major: memory lifetime and proc_exit | W1/W2 (§6), checked token table and void-result case |
| 7 major: incomplete conversions | F4 (§4), exhaustive outcome table and copy semantics |
| 8 major: macro escape | M1 (§7), exact meta §§4.3/8 additions in §9 |
| 9 minor: target coverage and missing Io import | T4 (§2), both corrected complete examples |
| Kernel/provisioning claims | §1, T1, F4, A4, W2: actual fixed duties and ordinary provider composition |
| Factory/WASI/native scope, ABI and callbacks | A4, T3/T5, F3, implementation order (§8), conflicts (§9) |
| CLI aliases, exports, initializer scope/order, Node loading | T3, F1–F3 (§§2–3) |

Decisions beyond rulings, with reasons/sources: T1's general provider plan
implements the user's defaults without library-specific lowering (spec §9.2);
T2's five service names separate preview-1 cwd and child capabilities (review
1/runtime §7.4); T3/T4 keep ilang as an alias and Native nullary (native N1–N4,
review reconciliation); F1/F2 choose getter initialization and conservative
foreign-only retention (spec §2.5/§4.7, review 3); F3 versions target metadata
outside v3 and uses global ESM scope/dynamic imports (B.6, review minors);
F4 uses synchronous opaque outcomes and primitive conversion errors (spec
§10.3, review 5/7), a Js bundle to discharge both raw service requirements
(spec §9.2), snapshot validation to preserve immutable values (§11.1), and
opaque synchronous externref results despite thenable payloads (§10.3/B.2;
SDK `ilang.mjs:836–839`); A4 shares accepted factories (N9/runtime §8.2); W1/W2 reduce the first
adapter to a checked slice (J6, allocator evidence); §8 keeps callbacks behind
a separate gate (B.3–B.5/runtime R2). These are design choices, not additional
claims of user rulings or implemented features.
