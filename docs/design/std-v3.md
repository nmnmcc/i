# Standard library v3: decisions and delivery plan

14 November 2026. Phase 1 synthesis. Branch `lane/stdv3`.

Authority, in order: [specification](../spec.md),
[syntax three §2.10](syntax-3.md#210-services-errors-and-the-environment),
then this design. Evidence: the complete [inventory](std-v3-inventory.md),
[Effect capability audit](std-v3-effect.md), and
[style and stability audit](std-v3-style.md). Their source baseline is
`68b720e9`; this synthesis starts at `aa03523e6`. The audits describe source,
not passing tests. This document changes no implementation.

Effect 4.1.3 is the capability reference. Its package version does not make
its platform APIs stable. Reference paths below name the supplied checkouts
under `.rd/ref/`, as recorded by the audits. They are source references,
not a requirement to download a moving upstream version.

## 1. Principles

The standard library makes i a useful standalone language. A program should
read files, validate input, call and serve HTTP, manage concurrent work, and
explain its failures using the shipped distribution. Gleam supplies the model
for small, readable modules. Go supplies precise, composable IO contracts.
Effect supplies the breadth and the lifetime discipline. i supplies the means.

A computation is an ordinary function `(P) <- (S) -> R`. Errors are the need
`Fail(E)`. A service is an ordinary labelled value. Service construction,
sharing and provision are ordinary calls, bindings and `<-`. All values stay
immutable. Scheduler-owned state is accessed through immutable handles and
pure `task.update` transitions. Every child belongs to a task group. Every
resource has an enclosing lifetime. Cleanup may wait; a timeout waits for
cleanup too. A stop is not a recoverable error and runs no i finalizers.

The library is the language's best textbook. Write the most natural code
available in the language now. Every public function has a doc comment with
its purpose, needs, errors and a runnable example. Every module starts with
how to use it and why its design has this shape. Tests are executable examples
of observable behavior. State units, callback order, costs, laziness and
ownership where they matter. Prefer one function fewer to an inelegant one.
Do not preserve an alias merely to keep an export count high. Preserve useful
algorithms and their regression examples. Implement each policy in one place.

The following decisions cover every exclusion in the Effect audit, §20.

| Do not copy | Reason and i replacement |
|---|---|
| Effect wrapper, `gen`, effect-map/run DSL | Function calls and native needs already sequence computations. |
| Context tags, string service keys, Layer graph or automatic memoization | Nominal service types, explicit construction order, bind-once sharing, `<-`. Scoped factories must still supply Layer's useful lifetime guarantees. |
| Separate error channel or tag-specific recovery family | `Fail(E)`, union narrowing, ordinary matching, one `i/fail`. |
| Full Cause/Exit capture of defects and interruption | Preserve stops, cancelled-join rules and least-task-path failure selection. Diagnostics are data; catch cannot observe every outcome. |
| Detached fibers, general interruption masks, worker placement API | Structured groups, cancellation-safe handoffs, host-selected workers. |
| Mutable Ref/collections, synchronized mutation protocol | Existing scheduler state and pure atomic replacement; owner tasks for effectful protocols. |
| Match builder, Data classes, tagged-error hierarchy | Branches, guards, labelled constructors and unions are language features. |
| Custom equality/hash protocol | One language equality and its compatible hash. Domain comparisons are explicit functions. |
| Either, second BigInt, Array/Chunk/Record replicas | Existing Result, arbitrary Int, List, Sequence, Dict and closed data. Decimal has different semantics and earns a type. |
| Schema classes or compiler-known derivation | Immutable schema descriptions, ordinary interpreters and optional ordinary macros. |
| Second streaming evaluator, seven-parameter public Channel, EOF encoded as failure | Ordinary pull functions, explicit end data, scoped consumption and small sinks. Task channels retain their separate rendezvous meaning. |

No standard module gains compiler privilege. Existing `big`, `bits` and
`float_text` runtime hooks need an explicit primitive/linking boundary review;
renaming the files does not settle it. This plan does not expand those hooks.

## 2. Conventions: settled rules

### 2.1 Names and calls

Use snake_case for modules, functions, bindings and fields. Use UpperCamelCase
for types and constructors. Use nouns for observations (`size`, `keys`) and
verbs for work (`read`, `decode`). Predicates use `is_`, `has_`, or `contains`:
`is_empty`, `has_scheme`, `contains(collection, item)`. Use `new` for empty
collections and `singleton` for one element. Keep `then` for Option/Result
data; use `flat_map` for sequences. Do not introduce `try_` or `map_effect`.

Put the subject first, configuration next, callback last. Fold callbacks take
the accumulator first. A builder is the subject while being built. Input data
is the subject while being decoded. Thus `schema.decode(value, codec)` and
`regex.find(text, pattern)`. Cryptographic input bytes precede key/salt/options.
Use `from_` and `to_` for representation boundaries. Name index units in docs;
raw clock names retain `_ns`, while policy APIs take `Duration`.

Use distinct arities for default/configured shapes of one operation. Use a
label to distinguish eager and lazy alternatives of equal arity: `default =`
versus `make =`. They are shapes, not optional parameters or type overloads.
All shapes must have one coherent result and need contract. Do not merge a
pure function with an IO function under one group.

This current-library example demonstrates naming, the pipe's position-zero
rule, and callback-last order. It does not depend on any new API:

```i
(std/list)

doubled = (values: List(Int)) -> List(Int) {
  values |> list.map((n) { n * 2 })
}
example = () { assert(doubled([2, 3]) == [4, 6]) }
```

### 2.2 Errors and data

Use `Fail(ModuleError)` whenever an operation cannot deliver its requested
answer, including pure parsing and validation. Use Option for expected
absence. Use Result for caught outcomes that are stored, inspected or
collected. `fail.catch` handles typed failure, not Assert or a stop. Catch at
recovery/reporting/storage boundaries. Do not catch each internal call.
Protocol progress and validation reports remain explicit domain data.

Each module owns one public error family, named `ModuleError`; its cases have
recovery meaning and useful fields. Existing `FsError`, `NetError`, etc. keep
those qualified names. Supporting reason/path enums are not extra error
channels. Consolidate today's CLI definition errors and URI codec errors into
their module family. Unrelated module errors compose as `Fail(A | B)`.
Wrap a lower error only to add domain context; retain it in a typed field.
Never turn an upstream error into an English string for recovery. Adding a
case to a stable closed error family is breaking.

This complete example already uses syntax three and today's failure API:

```i
(i/fail, std/list, std/option)

BadPort = (text = String)
PortError = BadPort
parse_port = (text: String) -> Int {
  text |> ("80") { 80 } | ("443") { 443 }
    | (_) { fail.raise(BadPort(text = text)) }
}
ports = () { list.map(["80", "443"], parse_port) }
example = () {
  assert(fail.catch(ports) == Ok([80, 443]))
  assert(fail.catch(() { parse_port("bad") }) == Error(BadPort(text = "bad")))
  assert(list.first([]) == option.None)
}
```

To compose errors, call both operations normally. To recover selectively,
match `Error(e: WantedError)` and re-raise the narrowed remainder. No generic
`Error` hierarchy, universal Cause service or error-code string parser is added.

### 2.3 Services, defaults and tests

A service is a nominal labelled type with the smallest useful function fields.
Name its responsibility: `Clock`, `Client`, `ConfigProvider`, `Tracer`.
Two independently replaceable roles have two constructors. Provider functions
name their choice: `native_client`, `fetch_client`, `stderr`, `memory`, `fixed`.
Return an ordinary service or positional group of services. Bind once to share;
call twice to isolate. State captured services versus call-time needs explicitly.
Never capture a live Fail handler in a long-lived provider.

Choose defaults explicitly in program entry code. Platform services are only
those of spec §9.7. Imports and package files install no other services.
This follows syntax-three §2.10.11 over the style audit's §4.5 provider-plan
suggestion: that suggestion describes removed machinery. Test replacement is
ordinary provision. Resource-backed factories require Scope around all uses.

```i
Settings = (port = () -> Int)
port = () <- (settings: Settings) { settings.port() }
local = () -> Settings { Settings(port = () { 8080 }) }
example = () {
  live = local()
  assert((port() <- (live)) == 8080)
  assert((port() <- (Settings(port = () { 9000 }))) == 9000)
}
```

### 2.4 Module boundary

`i/…` holds specified language/runtime support, host contracts and minimal
owned-request adapters. `std/…` holds data structures, protocols, codecs,
policies and reusable algorithms, including those with host needs. `i/` never
imports `std/`. Shared small traversals may live in an ordinary internal
foundation where an import cycle forbids reuse of `std/list`; that does not
create a second public API. Keep specified `i/random` in place. Do not move
specified primitives or break host namespaces as a cosmetic cleanup.

Generic inspection and hashing have one public home, `i/core.inspect/hash`.
Type-specific number formatting remains in int/float because it is a
representation operation. Generic string/byte inspection aliases are deleted.
`i/io.Io` remains the specified minimal best-effort print capability;
`i/stdio` remains fallible byte IO. Their different error promises are explicit.

```i
((Io) = i/io)
main = () <- (io: Io) { io.print("hello") }
```

### 2.5 Doc and test form

Use `////` for the module introduction. It gives the normal composition first,
then the design reason, units, lifetime and stability. Use `///` on every
exported value, type and visible constructor. Avoid a handwritten duplicate
signature catalogue in source comments. The checker owns signatures.

```i
//// Integer list transformations. Ordinary callbacks keep their native needs.
//// Stability: experimental. Since: next. Tracking: docs/design/std-v3.md.
(std/list)

/// Doubles each integer, preserving order.
/// Needs: none. Errors: none.
/// Example: doubled([2, 3]) == [4, 6].
doubled = (values: List(Int)) -> List(Int) {
  values |> list.map((n) { n * 2 })
}
```

Each `Example:` expression is checked in an ordinary test body with the
module's declared example imports. Multi-statement examples use fenced i
blocks, an explicit `example = () { … }`, and assertions. The docs runner must
execute them, not merely parse the comments. Deferred APIs state when callbacks
run. Resources state who closes them and what happens on early exit. Examples
must work on main with the documented package dependency; proposed APIs in
§3 are contracts for implementation, not claimed current exports.

Tests use the existing macros and Assert. Name the observable behavior.
Table-driven examples cover related operations. Add laws where useful, not a
new test per trivial wrapper. Need-bearing callbacks, failures, empty inputs,
limits, early exit and cancellation need visible examples. Assert failures
stay separate from Fail. No test keyword or special standard-library runner.

```i
(testlib = (test) = std/test, std/list)

test("mapping preserves order", () {
  assert(list.map([2, 3], (n) { n * 2 }) == [4, 6])
  Nil
})
```

### 2.6 Decisions where audits differ

| Question | Decision and reason |
|---|---|
| Keep Result traversals from Gleam? | Delete list.try_map/try_fold/try_each. `fail.catch(() { list.map(xs, f) })` supplies control; `result.all` handles stored data. Style's optional retention adds a synonym. |
| Keep provider plans? | No. Syntax-three §2.10.11 and spec §9.7 require explicit program choices. |
| Expose Effect Cause/Exit? | Diagnostic Cause data only, no general exit observer. Spec cancellation and stop semantics take precedence. |
| Canonicalize Dict and Sequence to make `==` extensional? | No rushed representation rewrite. Document structural equality, add explicit content comparisons, and keep these containers experimental until reviewed. There is still one language equality. Arbitrary collision keys have no universal order for canonicalization. |
| Replace JSON scalar constructors by built-ins now? | Keep existing JSON tags. This preserves explicit wire-number intent and limits migration work; Node's numeric syntax migration does not mandate JSON's representation. |
| Move every pure `i/` algorithm? | Keep specified runtime support in place; review hooks separately. No name-based compiler privilege is legitimized. |
| Stream CSV errors as Result elements? | Pull raises CsvError. `stream.attempt` is the explicit capture boundary. End remains data. |
| Copy all Go packages / all Effect exports? | No. Finish the paths in §5. Specialized wire formats, named zones, transactions and host-worker interop are named extensions, with no fictitious implementation in today's estimate. |

## 3. Finished module map

The complete typed catalogue is added in the second synthesis commit. The
outline commit fixes conventions, ownership and implementation interfaces first.

## 4. Stability and the machine check

All changed/new items start experimental. Existing published-major obligations
still apply. Stability is documentation metadata enforced around the existing
semantic interface comparator in `crates/ilang-cli/src/commands/package.rs`.
The completed policy and exact gates accompany the catalogue commit.

## 5. Work breakdown and acceptance

### 5.1 Eight-hour execution window

Use ten implementation workers, one per lane A–J. Worker eleven does examples
K for hours 0–4 and stability CI L for hours 4–7. Hour 7–8 is integration and
regression reserve for everyone. Thus twelve file-disjoint lanes use eleven
workers. Estimates below include docs and tests; they are focused worker-hours,
not a claim that every future reference capability fits in one day.

Freeze the contracts below at hour 0. Each lane publishes its exported heads
and test doubles by hour 1, then implements bodies. Consumers may write against
those exact heads before the provider merges. Merge usable foundations by hour
2, policy/codec slices by hour 4, application paths by hour 6. Test after each
merge. No lane waits until hour 7 to reveal an interface mismatch.

The hard prerequisite is the syntax-three E3/E5 language baseline: ordinary
Fail, row-only control types, Group, explicit entries and Scope provisioning.
The audits and this worktree still show migration residue. The standard-library
budget does not hide a compiler migration. A owns library-side reconciliation;
the existing syntax migration owner must deliver compiler support in parallel.
If that baseline is unavailable by hour 2, report the deadline risk immediately.
Do not restore provider plans or a second error mechanism to make a build pass.

File sets below are exclusive. A lane owns its current and destination files,
its matching `std/test/<name>_test.i`, and a matching new
`std/test/examples/<name>_examples.i`. Existing multi-module test files belong
to the lane explicitly listed. Only that owner edits them. Consumers report
needed changes to the owner. No lane edits shared spec, manifests, workflow or
compiler files unless explicitly assigned. K edits application callers; L alone
edits package manifests, release lists and CI. Existing unrelated examples are
migrated by K, never by module lanes.

### 5.2 Lanes

| Lane / budget | Exclusive source files and modules | Order and assumed cross-lane interfaces | Required tests and examples |
|---|---|---|---|
| A. Native effects and lifetime; 6 h | `lib/i/{fail,failure,assertion,control,resource,key,request,syntax,core,big,bits,float_text}.i`; `tests/spec/9/{keep_env,control_lazy_env,control_lazy_env_unwind}.i`; new `std/test/{fail,resource,request,syntax,key}_test.i` | Consolidate fail → reconcile row-only control → resource Scope/scoped/acquire → request handoff → syntax docs/hooks review. Exact primitive contracts are spec §§9.3–9.5, 9.8, 9.11 and syntax-three §2.10.7–9. No dependency on std. Coordinate E3/E5 compiler work with its existing owner. | Catch separation, narrowed rethrow, outlived handler rejection, reverse cleanup, nested/empty scope, cancel between acquisition and registration, unclaimed Ready release, no finalizer Fail. Preserve existing primitive regression suites. |
| B. Task coordination; 7 h | `lib/i/task.i`; new `std/src/{deferred,semaphore,queue,pubsub,latch,fiber_set}.i`; matching new tests | Group rename/ref helpers and bounded traversal → Deferred → Semaphore → bounded Queue → PubSub/Latch → scoped FiberSet. Assume A's Scope and ordinary Fail. Use task events and owner loops; no public scheduler protocol extension. Publish capacity/termination heads first. | FIFO/backpressure, losing select consumes nothing, cancelled waiter removal, exactly-once completion, permit handoff, scope closure, O(limit) live traversal children, no leaked handles. Existing task Rust/spec fixtures run unchanged apart from Group naming owned through integration. |
| C. Time and policy; 6 h | `lib/i/{clock,random}.i`; `std/src/{time,calendar,schedule,concurrent,test_clock,test_random}.i`; `std/test/{clock,time,calendar}_test.i` and new matching tests | Keep exact Duration/Instant → task-level Clock service and virtual sleepers → timeout → schedule/retry/repeat → random test helpers. Assume B's Ref/channel/scopes and A's resource lifetime. Existing i/clock remains raw adapter. | Quiescent virtual time, sleepers registered after yields, cancellation removes sleeps, wall jumps do not affect duration, timeout waits for cleanup, exact attempt/delay vectors, bounded seeded jitter, seed/path replay. No real sleeping in policy unit tests. |
| D. Collections and data; 6 h | `std/src/{option,result,list,dict,set,sequence,order,pair,bool}.i`; their current tests | Delete duplicate bridges → normalize names/shapes → preserve need propagation → content comparison laws. Assume core equality/hash from A, unchanged. Preserve collection algorithms; no canonical-tree project today. | Existing collection regression/property examples; different insertion/tree histories, hash collisions, stable sorting, empty lookup, effectful callbacks and first failure. Show explicit conversion to list for value keys. |
| E. Numbers, text and binary data; 7 h | `std/src/{int,float,string,bit_array,bytes,unicode,decimal,crypto}.i`; matching tests plus `string_additions_test.i`; Unicode tables currently included by regex move only through F's coordinated deletion | Typed numeric/byte errors → subject-first codecs → shared Unicode data → incremental UTF-8 → byte assembly/search → small normalized Decimal. Assume A's core/entropy boundary; D's Order/List. Keep i/float_text algorithm. | Existing vectors and float-text regressions; invalid radix/domain/UTF-8 offsets; every UTF-8 split; whole-byte enforcement; byte search; Decimal normalization/rounding; no new timing-security claim. |
| F. Formats, schema and configuration; 7 h | `std/src/{json,schema,csv,regex,uri,cli,config}.i`; matching current/new tests | Fail migration and argument order → retain parser algorithms → schema description + decode/encode/validate → config provider precedence. Assume E's UTF-8 and numeric errors, D's collections, H's environment/file adapters; describe schema metadata before consumers use it. | Round trips, duplicate JSON fields, unknown/missing/null distinctions, encode validation, unsupported custom derivation, CSV chunk boundaries and Fail, regex zero-width demand, URI byte offsets, CLI fake args, config missing/invalid/source-error precedence. |
| G. Stream, sink and common IO; 7 h | `std/src/{stream,sink,io}.i`; their tests | Freeze Reader/Writer → scoped run → incremental adapters → Sink leftovers → bounded map/merge/buffer and time operators. Assume A Scope, B queues/events, C clock and E UTF-8. No std import in lib/i adapters: high-level adapters live here. | Pure replay/demand; scoped single-consumption IO; early take closes; decoder Fail/cancel closes; bounded memory and producers; leftovers are not read twice; every chunk boundary; short/zero-progress writes and exact-read EOF. |
| H. Host IO and tools; 7 h | `lib/i/{fs,process,net,tls,stdio,io,fetch}.i`; `std/src/{fs,path,process,terminal}.i`; matching tests including `stdio_test.i`; existing IO example tests stay K-owned | Preserve raw ABI and creation guards → scoped high-level handles/adapters → file streams/temp/copy/walk → immutable commands with concurrent output drains → cooked terminal. Assume A Scope, G Reader/Writer/source contract, C timeout, F typed env/config errors. No new host imports in this day. | File cleanup at every handoff; injected read/write/flush errors; TLS option behavior; UDP empty packets; resolver/handshake cancel; child termination+wait and both pipe drains; temp cleanup; native/node/wasi capability differences. |
| I. HTTP; 7 h | `std/src/{http,http_client,http_server}.i`; `std/test/{http,http_network}_test.i`; new client/server tests | Pure messages/framing → scoped Client factory → true native/fetch response pulls → streaming server response + bounded admission → generic timeout/retry adapters. Assume A Scope, B admission, C policies, F codecs, G IO, H net/TLS/fetch. | A response chunk arrives before body completion; early-close at every stage; pool survives body use; size limits; redirects never replay unsafe bodies; cross-origin sensitive headers; framing, request/response streams, cancellation waits for handlers. |
| J. Application services and testing; 7 h | `std/src/{log,metric,trace,cache,batch,kv,property,test_services}.i`, `std/src/test.is`; matching tests, `property_failures.py` | Test macros/properties and recorders → contextual log/metric/trace → scoped single-flight cache → batch dispatcher → memory/file KV. Assume A Scope, B Deferred/Queue, C monotonic clock, D Dict, F schema/config, H files. Start recorders/cache state machines before dependencies merge. | Lazy filtered logs, output failure policy, deterministic metric/span records, early/cancelled spans, cache capacity/TTL/invalidation/load races, cancellation of one shared waiter, partial/missing batch answers, KV round trips, property seed/shrink replay and suite `<-` handling. |
| K. Programs and caller migration; 4 h, worker 11 first | All `examples/**` files that import affected modules; four new packages `examples/std-v3-{http,files,crawl,config}/**`; acceptance guide `docs/examples/std-v3.md` | Start with fakes against frozen heads. HTTP retry+timeout → scoped streaming file CSV processing → crawl with input-order bounded results → schema-validated config. Assume contracts A–J; module owners never edit these callers. | Each package runs and has a fake-service test. HTTP: transient failure then success and deadline cleanup. Files: early stop, malformed UTF-8/CSV, no leak. Crawl: measured max in-flight ≤ limit, fail/cancel cleanup. Config: env overrides file only when present, invalid override fails, secret rendered redacted. |
| L. Stability list and CI; 3 h, worker 11 second | `std/i.toml`; `docs/design/std-v3-api.json`; `docs/design/std-v3-behavior.md`; `scripts/std-api-check`; `std/test/std3_interfaces.py`; CI workflow files under `.github/workflows/`; `crates/ilang-cli/src/commands/package.rs` and its `package_commands.rs` tests only for read-only comparator exposure | Extract final heads → metadata/dependency closure → read-only existing semantic comparator → tag and compiled-client gate. Assume all lanes' item docs and example links; coordinate compiler owner before touching package.rs. No handwritten second type-diff engine. | Deliberate removal/need/error/constructor/shape/host-namespace changes fail; independent export requires minor; dropped stable mark fails; experimental reachability fails; pre-A3 baseline skip is a hard review gate; examples compile. |

B and J are the largest new-code lanes. Deliver their minimal complete
contracts first: bounded FIFO queue, one-shot Deferred, fixed Semaphore;
then PubSub/Latch and cache/batch. Exclude resizing semaphores, replaying hubs,
key replacement, transactions and exporter ecosystems from today's work.
Do not substitute a semaphore around an unbounded spawn-per-input traversal.

### 5.3 Interfaces frozen at hour zero

| Producer → consumers | Contract consumers may assume |
|---|---|
| A → all | `fail.raise/catch/from_result/map_error` exactly spec §9.3; `resource.ensure/acquire/scoped` exactly §9.5. Resource Scope is different from task Group. Release callbacks cannot raise. |
| B → C/G/I/J | Existing task event and Ref contracts; bounded queue with FIFO, explicit end after draining, failure terminal; Deferred many-read exactly-once completion; Semaphore bracket releases once on all structured exits. No arbitrary per-task interrupt. |
| C → G/I/J/K | Signed exact Duration; distinct Unix Instant; monotonic deadline arithmetic; provided task-level Clock with `now`, `monotonic`, `sleep`; deterministic `test_clock.advance`; timeout returns after loser cleanup; retries count retries after initial attempt. |
| E → F/G/H | BitArray stays the language type. `bytes` requires alignment. UTF-8 decoder keeps partial codepoints between chunks and reports byte offsets. No Unicode index reinterpretation. |
| F → H/I/J/K | JSON retains tagged scalars and ordered object pairs. Schema decode/encode both raise SchemaError; validate returns accumulated issues. ConfigProvider read returns Option(Json), source failure raises ConfigError. |
| G → H/I/K | Reader.read positive bound → Option(BitArray), None = EOF; Some is nonempty and within bound. Writer.write accepts immutable bytes and completes all or raises, with partial external effects allowed. Scope encloses the entire use. Sink returns result and remaining Stream. |
| H → I/J/K | Existing raw ABI and request ownership remain. High-level file/socket/child acquisition registers release in Scope before exposing handle. No fabricated support on targets without the host service. |
| I → K | Explicit scoped Client; buffered send is built on streaming send plus bounded collection; native streaming reads live connection, not a buffered whole response. |
| J → all/K/L | Test recorder values are ordinary provided services. Cache captures loader non-Fail needs deliberately, owns work in Scope, and shares in-flight outcomes. `std/test` remains ordinary item macros. |

### 5.4 Integration and size control

At 07:00–08:00 Hong Kong time, freeze heads, allocate the files, and publish
fakes. At 09:00 merge A and each lane's smallest usable slice. At 11:00 run the
four programs with fakes. At 13:00 require true HTTP streaming and native IO
cleanup tests. At 14:00 freeze implementations; finish CI, examples and target
checks. Merge to main by 15:00 only after the acceptance gates pass. If work
starts at another time, preserve the same eight-hour offsets and the final
hour of reserve. Do not treat a failing slice as complete to hit the clock.

The ten estimates total 67 worker-hours; K/L add 7. The nominal capacity is
88 worker-hours. The difference is integration and uneven-lane reserve, not
permission to add host subsystems. New capability families are experimental;
complete behavior and executable examples are required even for experimental
items. Existing 23,979 source lines and 2,170 named std tests are migration
assets, not work to rewrite from scratch or proof that new behavior works.

No new native ABI operations, timezone database, XML/ASN.1/gob, interactive
raw terminal, filesystem watcher, WebSocket/HTTP2 stack, transaction journal,
or external worker transport is included in the eight-hour estimate. They
are explicit extension boundaries in §3. A complete first release supplies
all requested everyday paths; reference breadth is not a promise to reproduce
every specialized upstream export today.

### 5.5 Owner decisions only

| Decision | Default proposed here; decision needed before |
|---|---|
| Which commit supplies the E3/E5 compiler baseline? | Name the syntax migration integration commit and its owner at hour 0. Without it the eight-hour std integration promise is conditional. |
| Which package major is the migration release? | Use a new major if any existing published interface breaks. Experimental does not waive spec §2.7. Decide before L creates the baseline. |
| Which targets must pass today? | Native full path; nodejs and wasi their available IO paths; js pure data and fetch only when explicitly supplied; host contract fakes. Approve the matrix at hour 0. |

API naming, error style, resource lifetime, default provision and lane ownership
are decided here. They need no further owner polling. A failed critical gate
is reported with its exact test and impact. Scope cuts require an explicit
release decision; do not silently replace resource safety with documentation.
