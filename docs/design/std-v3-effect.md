# Standard library v3: Effect 4 and i

Phase 1, part B. Read-only source audit. 2026-11-14.

## 1. Scope and evidence

This report compares capabilities. It does not propose an Effect interpreter for i.
The target is a complete standalone language with a useful standard library.
The binding rules are immutability, structured concurrency, ordinary libraries,
simple interfaces, and one definition for each concept.

The i checkout is `lane/stdB`, starting at
`68b720e9ebc0d643a3a7831a32a1b97c7ab3c915`. The Effect checkout is
`fdd319e4592a566a4ea9c9ad8cc60f2d62959feb`.
`effect/packages/effect/package.json` says `4.1.3`.
Here `effect/`, `gleam-stdlib/`, `gleam-http/`, `gleam-otp/`,
`gleam-erlang/`, and `go/` mean directories under
`/home/agent/Programming/i/.rd/ref/`. Other paths are relative to this i checkout.
Each reference claim below names its source file. API lists are selected entry
points, not complete export lists. Methods count as entry points too.

The language sources are `docs/spec.md`, especially §§4.6, 6.12, 7, 9,
11 and 14, and `docs/design/syntax-3.md`, especially §2.10 and §6.9.
The library inventory covers every module in `lib/i/` and `std/src/`.
Selected implementation bodies, `std/test/`, and `examples/` establish the
details below. Existing tests are source evidence, not proof of passing tests.
No build or test was run for this audit.

All proposed computation signatures use the coming E3 contract, `Fail(E)`.
There is no separate error channel. Existing library source is reported as found.
The later resource `Scope` contract and task `Group` rename are distinguished
from the exports currently in this branch.

Two version cautions matter:

- Platform interfaces have moved into the core package. Concrete backends remain
  in `packages/platform/*`. This is documented in `effect/MIGRATION.md` and
  visible in `effect/packages/effect/src/http/HttpClient.ts` and
  `effect/packages/platform/node/src/NodeHttpClient.ts`.
- A stable package release does not make every API stable. The HTTP, process,
  socket, workers and persistence modules inspected below say
  `@stability unstable`. So do `Arbitrary` and the Vitest integration.
  Core `Schema` says stable. Use their capabilities as evidence, not their
  stability labels as a promise for i. Sources: `effect/MIGRATION.md`,
  `effect/packages/effect/src/Schema.ts`,
  `effect/packages/effect/src/Arbitrary.ts`,
  `effect/packages/vitest/src/index.ts`, and the platform files in §16.

The main findings are:

1. i already has the computation and service model. The work is library design,
   not another effect system.
2. Resource scope is the first contract to reconcile. The spec and current
   `i/resource` exports differ. Streaming and service construction depend on it.
3. Tasks and events are substantial. Bounded traversal, reusable coordination,
   and a scheduler-aware test clock are the missing common tools.
4. Streams and schemas exist, but their representations limit the next stage.
   Streams do not own resources. Schemas hide structure in callbacks.
5. The platform is already useful. HTTP needs true native body streaming and
   common policies. Config, metrics, tracing and caching need coherent modules.

## 2. Translate the model once

These are translations, not new public wrapper types.

| Effect concept | i expression | Existing basis |
|---|---|---|
| `Effect<A, E, R>` | `() <- (..R, Fail(E)) -> A` | `docs/spec.md` §§4.6, 9.1–9.3 |
| Construct a lazy effect | A function value with parameters `()` | `docs/spec.md` §9.1 |
| Run, map, sequence a computation | Call it, bind its result, call another function | `docs/spec.md` §§6.6, 6.10, 9.1 |
| Service tag and service access | A labelled service type and a service pattern after `<-` | `docs/spec.md` §9.2 |
| Provide services | `e <- (s)`; provide a group or spread with explicit overrides | `docs/spec.md` §9.2; `docs/design/syntax-3.md` §2.10.5 |
| Layer construction and composition | Ordinary functions return ordinary groups of services | `docs/design/syntax-3.md` §2.10.10 |
| Layer sharing | Construct once, bind once, give that same value to all users | Same source; no automatic dependency ordering or sharing is specified |
| Error recovery | `fail.catch`, then ordinary matching and re-raising the remaining union | `docs/spec.md` §9.3 |
| Lifetime and cleanup | Resource scope, `ensure`, control unwinding, structured task groups | `docs/spec.md` §§9.4–9.5, 9.9 |
| Fiber | A task owned by a group; no detached public spawn | `lib/i/task.i`; `docs/spec.md` §9.9 |
| Select among concurrent operations | `Event`, `choose`, `sync`; mapping runs for the selected alternative | `lib/i/task.i`; `docs/spec.md` §9.9, events |
| Schema derivation | Ordinary schema descriptions and interpreters; optional ordinary macros | `docs/spec.md` §14; `lib/i/syntax.i`; proposed library work |

Use the normal need row for clocks, logging, configuration, caches and tracing.
Do not hide their installation in package imports. `docs/spec.md` §9.2 and
`docs/design/syntax-3.md` §2.10.11 require explicit service choice in program code.

Values remain immutable. `task.Ref(A)` is an immutable handle to scheduler-owned
state. Only `task.update` replaces that state, through a pure callback.
This existing distinction is explicit in `docs/spec.md` §9.9 and
`lib/i/task.i`. New queues and caches should use that mechanism or owner tasks,
not introduce mutable i objects or a second runtime state model.

## 3. Typed errors, Cause and Exit

### Effect evidence

| Source | Central types | Selected entry points | Problem solved |
|---|---|---|---|
| `effect/packages/effect/src/Effect.ts` | `Effect<A,E,R>` | `fail`, `catch`, `catchTag`, `catchIf`, `catchCause`, `catchDefect`, `mapError`, `result`, `exit`, `sandbox`, `tapError`, `orDie` | Expected errors, selective recovery, conversion to data, and observation without losing failure information |
| `effect/packages/effect/src/Cause.ts` | `Cause<E>`, `Reason<E>`, `Fail<E>`, `Die`, `Interrupt` | `fail`, `die`, `interrupt`, `fromReasons`, `combine`, `map`, `hasFails`, `hasDies`, `hasInterrupts`, `findError`, `findDefect`, `annotate`, `pretty` | Preserve typed failure, defect, interruption and annotations together |
| `effect/packages/effect/src/Exit.ts` | `Exit<A,E> = Success<A,E> \| Failure<A,E>` | `succeed`, `fail`, `failCause`, `die`, `interrupt`, `match`, `map`, `mapError`, `getSuccess`, `getCause` | Inspect a completed computation as ordinary data |

Effect 4's `Cause` contains a readonly array of reasons. It is not the old
sequential/parallel cause tree. A failed `Exit` contains a `Cause`.
Sources: `effect/packages/effect/src/Cause.ts`,
`effect/packages/effect/src/Exit.ts`.

### i today and the gap

`lib/i/fail.i` exports `raise`, `catch`, `from_result`, and `map_error`.
`std/src/result.i` adds `map`, `map_error`, `then`, `recover`, `partition`,
`all`, `catch`, and conversions to `Option`. `Result`, `Ok`, `Error` and
`Fail` are already described by `docs/spec.md` §§9.3 and 12.
`lib/i/failure.i` contains a separate transitional service-based implementation
using `Syn3ServiceFail`, `control.boundary`, and `control.escape`.
`lib/i/fail.i` still declares external `raise` and `catch` in this snapshot.
E3 should leave one ordinary `i/fail` implementation and one public error model.

Selective recovery needs no tag-dispatch framework. Catch all errors, match the
wanted nominal types, and raise the narrowed remainder. That is the example in
`docs/spec.md` §9.3. Catching a typed error does not catch assertion failure.
`lib/i/assertion.i` handles that separate service for testing.

There is no public general `Cause` or `Exit` library. The task implementation
has private completion and cancellation records. Its public join still returns
`A` or needs `Fail(E)`. Scope failure selects the least task path among actual
failures, after cleanup; it does not return every concurrent error.
Sources: `lib/i/task.i`; `docs/spec.md` §9.9, rules R2 and R4a.

The useful gap is structured diagnostics and explicit outcome data where an API
needs them. It is not permission to make `panic` recoverable. A stop runs no
i finalizers, and a cancelled join has specific rules, including a stop for a
shielded join of a cancelled child. Sources: `docs/spec.md` §§9.9, 11.7.
Any future outcome API must preserve those rules or request a separate language
decision. Ordinary `fail.catch` cannot observe all defects or cancellation.
For applicative validation, an ordinary `Result(A, List(E))` API can accumulate
independent errors without changing task failure semantics.

## 4. Services, Context and Layer

### Effect evidence

| Source | Central types | Selected entry points | Problem solved |
|---|---|---|---|
| `effect/packages/effect/src/Context.ts` | `Key<I,S>`, `Service<I,S>`, `Reference<S>`, `Context<Services>` | `Service`, `Reference`, `empty`, `make`, `add`, `get`, `getOption`, `merge`, `mergeAll`, `pick`, `omit` | Typed service identities, service collections, lookup and defaults |
| `effect/packages/effect/src/Layer.ts` | `Layer<ROut,E,RIn>`, `MemoMap` | `succeed`, `sync`, `effect`, `effectContext`, `merge`, `mergeAll`, `provide`, `provideMerge`, `build`, `buildWithScope`, `buildWithMemoMap`, `fresh`, `launch` | Construct a dependency graph with shared construction and resource lifetimes |

`Layer.effect` accepts an effect requiring resource scope and removes that scope
from the layer's public inputs. Building supplies scope and a memo map.
Memoization uses layer identity. Nested provides can reuse a parent's entries;
new child entries do not flow back into the parent. Sibling provides do not
automatically share newly built entries. Sources:
`effect/packages/effect/src/Layer.ts`,
`effect/migration/layer-memoization.md`.

### i today and the gap

Service values and provide are language features. Constructors give service
identity without `Context.Service` or string tags. A function returning
`(Config, Log, Db)` is sufficient to construct a service group. Dependency order
is written in normal code. Sources: `docs/spec.md` §9.2;
`docs/design/syntax-3.md` §§2.10.2, 2.10.10.

The current library already uses replaceable services: `clock.Clock`,
`process.Environment`, `http.Client`, and `log.Sink(E,N)`.
`std/test/clock_test.i` supplies a fake clock with `<-`.
`std/src/http.i` has native and fetch client factories. `examples/cli-log/src/main.i`
and `examples/http-server/src/main.i` are application entry examples.

The gap is a consistent construction guide and resource-safe service factories.
Construct a pool once in an enclosing resource scope and provide the same pool
to its users. Write independent construction explicitly when isolation is
wanted. This covers the important sharing use case without Layer objects,
graph resolution, or a hidden global memo table.

If a real use case needs lazy concurrent initialization, use a scoped cache or
one-shot coordination cell. Specify which scope owns it and whether failures
are cached. Do not turn optional memoization into a second service mechanism.
Also keep the E5 task `Scope(E)` to `Group(E)` rename separate from E3;
`lib/i/task.i` still exports `Scope` here, whereas
`docs/design/syntax-3.md` §2.10.9 names the future distinction.

## 5. Scope and resource safety

Effect offers `Scope` and `Closeable`, scope creation, finalizer registration,
forking and closing. The important entry points are `Scope.make`,
`Scope.addFinalizer`, `Scope.addFinalizerExit`, `Scope.fork`, `Scope.close`,
`Scope.use`, `Effect.scoped`, `Effect.acquireRelease`,
`Effect.acquireUseRelease`, `Effect.addFinalizer`, `Effect.ensuring`, and
`Effect.onExit`. Sources: `effect/packages/effect/src/Scope.ts`,
`effect/packages/effect/src/Effect.ts`.

These APIs make acquisition and cleanup one operation. Finalizers can see the
ending `Exit`. Scope supports sequential or parallel finalization.
`acquireRelease` and `addFinalizer` require releases with no typed error;
`acquireUseRelease` can include a typed release error in its result.
Sources: the signatures in those same two files.

The i foundation is strong: control unwinding runs final functions, including
on structured cancellation. Task scope exit waits for child cleanup.
Final functions may suspend but must not raise errors. A finalizer that never
finishes can prevent scope exit. Stops and host destruction do not promise i
cleanup. Sources: `docs/spec.md` §§9.4–9.5 and 9.9.

The immediate gap is a source/spec mismatch:

| Contract | Source read | Status in this checkout |
|---|---|---|
| `ensure(body, final)` | `lib/i/resource.i` | Implemented as an external unwinding primitive |
| `acquire(open, release, use)` | `lib/i/resource.i` | Implemented in i over `ensure` |
| `Scope`, `acquire(open, release)`, `scoped(body)` | `docs/spec.md` §9.5; `docs/design/syntax-3.md` §2.10.9 | Specified, absent from this module's exports |
| Cancel an outstanding host operation while releasing an unclaimed acquired handle | `lib/i/fs.i`, `lib/i/net.i`, `lib/i/tls.i`, `lib/i/fetch.i` | Explicit request-state and cleanup logic already exists |

Reconcile this before designing scoped streams, caches, pools or service
factories. Use one resource service. Keep the task group a different type.
Preserve reverse registration order as specified for i. Do not copy parallel
finalization merely because Effect offers it.

Audit the handoff from host acquisition to scope registration. Successful
acquisition must not be lost if cancellation arrives before registration.
The existing request wrappers address the unclaimed-result side; a new general
scope API must retain that protection. This is a design requirement, not a
claim that this audit found a failing execution.

Use ordinary handlers to turn fallible close operations into an explicit
non-failing cleanup policy. Do not silently add a release error channel or
make `ensure` catch stops. i does not currently offer Effect's whole `Exit`
as an argument to finalizers; add such observation only with an explicit
outcome contract.

## 6. Structured concurrency

### Effect evidence

| Source | Central types | Selected entry points | Problem solved |
|---|---|---|---|
| `effect/packages/effect/src/Fiber.ts`; `effect/packages/effect/src/Effect.ts` | `Fiber<A,E>`, `Effect<A,E,R>` | `Effect.forkChild`, `Effect.forkScoped`, `Fiber.await`, `Fiber.join`, `Fiber.interrupt`, `Fiber.interruptAll`, `Effect.race`, `Effect.raceFirst`, `Effect.timeout`, `Effect.forEach`, `Effect.onInterrupt`, `Effect.uninterruptibleMask` | Child work, cancellation, completion, time limits and bounded parallel traversal |
| `effect/packages/effect/src/FiberSet.ts` | `FiberSet<A,E>` | `make`, `add`, `run`, `clear`, `size`, `join`, `awaitEmpty` | Dynamically track fibers, remove completed fibers, interrupt remaining work when scope closes |
| `effect/packages/effect/src/FiberMap.ts` | `FiberMap<K,A,E>` | `make`, `set`, `get`, `has`, `remove`, `run`, `clear`, `size`, `join`, `awaitEmpty` | The same ownership, with keyed work and replacement |

`Effect.forEach` has a concurrency option. `Effect.race` waits for first
success despite early failures; `raceFirst` takes first success or failure.
The signatures and comments distinguish these. Effect also exports
`forkDetach`; it is outside i's allowed model. Source:
`effect/packages/effect/src/Effect.ts`.

### i today and the gap

`lib/i/task.i` already provides `run`, `scope`, `local_scope`, `go`, `join`,
`both`, `race`, `map_concurrently`, `yield`, channels, shared values and events.
It implements its scheduler over control, keys and requests. Task captures and
transported values obey portability rules. Group ownership and run identity
checks prevent detached use of handles. These are existing capabilities, not
missing Fiber machinery. Sources: `lib/i/task.i`; `docs/spec.md` §9.9.

Three gaps deserve early work:

- Bounded traversal. `map_concurrently` calls `spawn_values` for every input
  before joining. It has no public limit. Build a bounded worker group or
  bounded admission loop. A semaphore around an already spawned task per input
  limits active bodies but still allocates all task handles.
  Source: `lib/i/task.i`, `spawn_values`, `map_concurrently`.
- General timeout and retry helpers. HTTP has a timeout wrapper built from
  `task.race` and clock sleep. There is no common computation timeout module in
  the audited library inventory. Sources: `std/src/http.i`, `send_timeout`;
  `lib/i/task.i`; `lib/i/clock.i`.
- Dynamic scoped work management. There is no public FiberSet/FiberMap-style
  registry. Build it from groups, immutable dictionaries, refs and owner tasks.
  Key replacement needs a specified cancellation operation or cooperative
  cancellation protocol. The current task exports do not expose arbitrary
  task interruption. Source: `lib/i/task.i`.

i's `race` does not have Effect's first-success semantics. An observed failure
initiates ordinary scope failure; a later success cannot rescue it. Successful
completion cancels the loser and waits for finalizers. Sources:
`docs/spec.md` §9.9 R5; `lib/i/task.i`, `race` and scope completion logic.
Keep this difference visible. Add a separately named first-success helper only
if needed. Do not change race incidentally while implementing timeout.

Timeout means cancellation plus waiting for cleanup. It cannot guarantee a hard
wall-clock return if a finalizer never returns. Preserve the same rule for task
groups, request owners, stream consumers and server shutdown.

## 7. Schedule: retry, repeat and pacing

Effect has `Schedule<Output,Input,Error,Env>`, `InputMetadata`, and `Metadata`.
Selected functions are `fromStep`, `recurs`, `spaced`, `fixed`, `exponential`,
`fibonacci`, `jittered`, `max`, `min`, `concat`, `upTo`, `while`, `map`,
`tap`, and `cron`. Consumers include `Effect.retry` and `Effect.repeat`.
Sources: `effect/packages/effect/src/Schedule.ts`,
`effect/packages/effect/src/Effect.ts`.

A schedule separates policy from the operation. It can inspect input,
attempt count and elapsed time. It can stop, produce output, delay or fail.
`max` combines slower delays and stops when a constituent stops; `min` uses
faster delays and can continue while any constituent continues. The repository's
example combines exponential delay, a delay cap, jitter, an attempt bound and a
retryable-error predicate. Sources: `effect/packages/effect/src/Schedule.ts`,
`effect/ai-docs/src/06_schedule/10_schedules.ts`.

There is no schedule module or reusable retry/repeat API in `lib/i/` or
`std/src/`. `docs/spec.md` §9.1 has a simple recursive retry example, not a
shipped schedule library. Clocks, exact durations, pure generators and
structured tasks already exist in `lib/i/clock.i`, `std/src/time.i`,
`lib/i/random.i`, and `lib/i/task.i`.

Express a policy as immutable state plus a step function. Return a stop or a
next decision containing output, duration and next state. Any step needs are
ordinary needs; a step error is `Fail(E)`. Retry catches the body's errors and
feeds them to the policy. Repeat feeds successful results. Sleep belongs to
the runner, so policy calculations can be tested without waiting.

Start with count bounds, predicates, spacing, capped exponential backoff,
jitter and composition. Define attempt numbering, whether the initial call
counts, delay ordering and exhaustion errors once. Thread `random.Generator`
explicitly through jitter state, or use a small provided random service built
on the existing generator. Do not introduce ambient randomness.
Cron needs timezone/calendar policy and can follow the basic runner.

## 8. Stream, Sink and Channel

### Effect evidence

| Source | Central types | Selected entry points | Problem solved |
|---|---|---|---|
| `effect/packages/effect/src/Stream.ts` | `Stream<A,E,R>` | `fromIterable`, `fromQueue`, `fromPubSub`, `unfold`, `scoped`, `mapEffect`, `flatMap`, `merge`, `buffer`, `groupedWithin`, `debounce`, `retry`, `run`, `runCollect`, `runForEach` | Incremental, concurrent, timed, resource-safe data processing |
| `effect/packages/effect/src/Sink.ts` | `Sink<A,In,L,E,R>`, `End<A,L>` | `fold`, `foldUntil`, `collect`, `head`, `take`, `reduceEffect`, `forEach`, `map`, `mapInput`, `flatMap`, `unwrap`, `ensuring` | Reusable consumers with a final result and explicit leftovers |
| `effect/packages/effect/src/Channel.ts` | `Channel<OutElem,OutErr,OutDone,InElem,InErr,InDone,Env>` | `fromPull`, `fromTransform`, `acquireRelease`, `acquireUseRelease`, `map`, `flatMap`, `pipeTo`, `merge`, `buffer`, `scoped`, `toPullScoped`, `runCollect` | Implement streaming transforms with typed input, output, completion and resource lifetime |

Effect streams contain channels of non-empty arrays. Sinks consume pulls and
can return leftovers. `Pull<A,E,Done,R>` uses `Cause.Done<Done>` to distinguish
normal completion from failure. Sources:
`effect/packages/effect/src/Stream.ts`,
`effect/packages/effect/src/Sink.ts`,
`effect/packages/effect/src/Channel.ts`,
`effect/packages/effect/src/Pull.ts`.
This streaming Channel is not a task rendezvous channel.

### i today and the gap

`std/src/stream.i` already has `Stream(A,E,N)`, `Done`, `Next`, construction,
unfold, map/filter/flat-map, append, zip, take/drop, folds, `attempt` and
`map_environment`. Its pull is an ordinary computation needing
`Fail(E)` and `N`. Construction is pure. Pulling the same stream again repeats
the work; it does not advance or memoize a cursor.

The module explicitly owns no resource and does not close on early termination.
`std/test/stream_test.i` checks finite observations and call-time service use.
`std/src/csv.i` supplies stream consumers/producers for rows and records.
`std/src/http.i` and `lib/i/fs.i` have separate incremental I/O interfaces.

Missing capabilities are:

- One scoped acquisition/consumption protocol for files, sockets and HTTP bodies.
  Early take, decoder failure and cancellation must close producers.
- Bounded buffering, parallel mapping, merge, broadcast and timed operators.
  These depend on queues, coordination and the clock, not new language effects.
- Reusable sinks with leftovers. Current folds consume a stream but do not
  provide a common composable consumer description.
- Incremental byte and text codecs shared by HTTP, files and CSV. Chunked reads
  must preserve partial UTF-8 characters and parser state.

Keep pure replayable streams useful. Add scoped consumption explicitly, rather
than making an arbitrary dropped value responsible for cleanup. A common pull
protocol should represent end-of-input as data, separately from `Fail(E)`.
There is no need to copy Effect's done-as-error encoding.

Begin with ordinary producer and consumer functions, events and resource scope.
Prove that a public seven-parameter Channel type is needed before exposing it.
It may be an internal streaming implementation detail. Do not create a second
evaluator for a streaming instruction tree.

## 9. Coordination and state

### Effect evidence

| Source | Central types | Selected entry points | Problem solved |
|---|---|---|---|
| `effect/packages/effect/src/Queue.ts` | `Queue<A,E>`, `Enqueue<A,E>`, `Dequeue<A,E>` | `bounded`, `unbounded`, `sliding`, `dropping`, `offer`, `offerAll`, `take`, `takeAll`, `poll`, `end`, `fail`, `shutdown` | Buffered handoff, backpressure and terminal states |
| `effect/packages/effect/src/PubSub.ts` | `PubSub<A>`, `Subscription<A>` | `bounded`, `unbounded`, `sliding`, `dropping`, `publish`, `publishAll`, `subscribe`, `take`, `takeAll`, `end`, `shutdown` | Broadcast to active subscribers, with bounded and replay policies |
| `effect/packages/effect/src/Deferred.ts` | `Deferred<A,E>` | `make`, `await`, `succeed`, `fail`, `failCause`, `complete`, `done`, `poll`, `isDone` | Complete once; any number of waiters see the same outcome |
| `effect/packages/effect/src/Semaphore.ts` | `Semaphore` | `make`, `withPermit`, `withPermits`, `withPermitsIfAvailable`, `take`, `takeScoped`, `release`, `resize` | Bound simultaneous access and release permits on exit |
| `effect/packages/effect/src/Latch.ts` | `Latch` | `make`, `open`, `close`, `release`, `await`, `whenOpen`, `isOpen` | Reusable open/closed gate for waiting fibers |
| `effect/packages/effect/src/Ref.ts` | `Ref<A>` | `make`, `get`, `set`, `getAndSet`, `modify`, `update`, `updateAndGet` | Atomic pure state transitions |
| `effect/packages/effect/src/SynchronizedRef.ts` | `SynchronizedRef<A>` | `make`, `get`, `set`, `modifyEffect`, `updateEffect`, `getAndUpdateEffect`, `updateAndGetEffect` | Serialize transitions that themselves perform effects |

Transactional memory still exists, but there is no `STM.ts` in the inspected
source inventory. The current design is `Effect.Transaction`, `Effect.tx`,
`Effect.txRetry`, `TxRef`, `TxQueue`, and `TxHashMap`. Core entry points include
`TxRef.make/get/set/modify/update`, `TxQueue.bounded/offer/take`, and
`TxHashMap.empty/get/set`. The outermost `tx` commits a journal; conflicts and
explicit retry can rerun its body after accessed transactional values change.
Sources: `effect/packages/effect/src/Effect.ts`,
`effect/packages/effect/src/TxRef.ts`,
`effect/packages/effect/src/TxQueue.ts`,
`effect/packages/effect/src/TxHashMap.ts`.

### i today and the gap

`lib/i/task.i` has rendezvous channels, selectable send/receive/join events,
atomic `ref`/`update`, and immutable shared values. It has no public buffered
queue, broadcast hub, one-shot cell, semaphore, latch, synchronized ref or
transaction module. The complete public task export block is the basis for
this absence claim.

`Request(A)` is not Deferred. Requests have host ownership, tickets, cancellation
and consumption states. There is `request.ready`, but no public general pending
request constructor with a matching user completer. Sources:
`lib/i/request.i`; `docs/spec.md` §9.11.

Build a small common coordination family over scheduler state, owner tasks and
events. Values and handles remain immutable. Define capacity, FIFO order,
terminal state, waiting-task cancellation and lifetime once. An abandoned
selected alternative must not consume a value or a permit. Use events for
atomic choice, not “poll then receive.”

| Capability | Fit for i | Main missing contract |
|---|---|---|
| Queue | Immutable queue state in an owner task/ref; event-based waiting | Bounded admission, close/drain/error distinction, cancellation of waiters |
| PubSub | Scoped subscriber registrations and per-subscriber delivery state | Slow subscriber policy, replay bound, unsubscribe cleanup |
| Deferred | Stored `Result` plus waiters; one successful completion | Exactly-once completion, many readers, cancellation affects waiter versus producer |
| Semaphore | Permit accounting plus acquire/release bracket | No lost permit between acquisition and finalizer registration; fairness |
| Latch | Gate state and event waiters | Distinguish opening from releasing only current waiters |
| Ref | Existing `task.ref`/`task.update`; small read/set helpers if useful | Keep callback pure and preserve scope/run checks |
| SynchronizedRef | Serialized owner task accepting commands with effectful work | Cancellation, reentrancy and commit/rollback policy; never await inside `task.update` |
| Transactions | First use one ref holding aggregate immutable state | Multi-cell journals and retry subscriptions only if aggregate state proves insufficient |

Full transactions are a later, large feature. They need explicit retry and
side-effect rules. Re-executing arbitrary file writes or network calls is not
a safe implementation of transaction retry. No new computation wrapper or
compiler-recognized STM module is justified by the existing gap.

## 10. Schema

### Effect evidence

| Source | Central types | Selected entry points | Problem solved |
|---|---|---|---|
| `effect/packages/effect/src/Schema.ts` | `Schema<T>`, `Codec<T,Encoded,RD,RE>`, `Decoder`, `Encoder`, `SchemaError` | `Struct`, `Tuple`, `Array`, `Union`, `suspend`, `check`, `refine`, `decodeTo`, `decodeUnknownEffect`, `encodeEffect`, `annotate`, `annotateEncoded`, `toJsonSchemaDocument`, `toRepresentation`, `toEquivalence` | Validate, transform, encode and derive views of one description |
| `effect/packages/effect/src/SchemaAST.ts`; `effect/packages/effect/src/SchemaRepresentation.ts`; `effect/packages/effect/src/JsonSchema.ts` | `AST`, `Representation`, `Document`, `JsonSchema.Document` | `SchemaAST.struct`, `SchemaAST.union`, `SchemaAST.annotate`, `SchemaRepresentation.toRepresentation`, `SchemaRepresentation.toJsonSchemaDocument`, `SchemaRepresentation.fromJsonSchemaDocument`, `JsonSchema.toDocumentDraft07`, `JsonSchema.toMultiDocumentOpenApi3_1` | Inspect and transform schemas; generate or import external schema documents |
| `effect/packages/effect/src/Arbitrary.ts` | `Arbitrary<A>`, `CheckResult<A,E>`, `Replay` | `schema`, `Constant`, `array`, `map`, `filter`, `flatMap`, `all`, `sampleEffect`, `checkEffect`, `formatCheckFailure` | Generate data and check properties from schema descriptions |

Codec types distinguish decoding and encoding service needs. Schema supports
annotations, checks, defaults and transformations as part of its representation.
Sources: `effect/packages/effect/src/Schema.ts`,
`effect/packages/effect/src/SchemaAST.ts`.
`Arbitrary` is marked unstable, even though the core schema module is stable.
Source: `effect/packages/effect/src/Arbitrary.ts`.

### i today and the gap

`std/src/schema.i` is already a two-way JSON codec library. It has primitives,
lists, dictionaries, tuples, field builders, optional fields, unions, tagged
unions, lazy recursion, mapping and refinement. `SchemaError` has a path and
expected text. `std/test/schema_test.i` defines round-trip and path checks.
`std/src/http.i`, `std/src/csv.i` and `examples/http-server/src/main.i` already
consume this schema API.

The representation is just
`Schema(A,E) = (read = (json.Json) -> Result(A,E), write = (A) -> json.Json)`.
It cannot recover field metadata, structural checks or annotations from those
callbacks. `refine` checks decoding only. Encoding does not validate again.
`transform` is an alias of pure bidirectional `map`; it is not a fallible or
service-using transformation. Source: `std/src/schema.i`.

The missing foundation is schemas as inspectable immutable data. Add one
description that structural combinators build. Derive decoding, encoding,
validation, path-aware diagnostics, JSON Schema and `property.Gen` from it.
Keep custom mapping functions possible, but mark which custom rules have no
JSON Schema or generator interpretation. A predicate is not automatically an
exportable constraint. Recursive descriptions need explicit references and
bounded generation rules.

Generalize beyond JSON only where a real use case needs it. Pure `Result`
codecs remain convenient; effectful codecs use ordinary function needs and
`Fail(E)`. Define whether encode can fail, whether errors accumulate, and which
annotations describe encoded versus decoded values.

Macros can remove repetitive field getters and setters. They must use ordinary
`Definitions`, `Types`, syntax values and generated library calls. They cannot
depend on the removed record-graph machinery or bypass private constructors.
Sources: `docs/spec.md` §§14.7–14.8; `lib/i/syntax.i`;
`docs/design/syntax-3.md` §6.9. Runtime schema data must remain useful without
macros. Do not add compiler special cases for `std/schema`.

## 11. Config and ConfigProvider

Effect provides configuration descriptions `Config<T>`, structured `ConfigError`,
providers, provider paths and raw `Node` values. Central entry points are
`Config.schema`, `Config.String`, `Config.Int`, `Config.Boolean`, `Config.all`,
`Config.nested`, `Config.withDefault`, `Config.option`, `Config.orElse`,
`ConfigProvider.make`, `ConfigProvider.fromEnv`, `ConfigProvider.fromEnvRecord`,
`ConfigProvider.fromUnknown`, `ConfigProvider.fromDotEnvContents`, and
`ConfigProvider.orElse`. Sources: `effect/packages/effect/src/Config.ts`,
`effect/packages/effect/src/ConfigProvider.ts`.

This separates configuration structure and validation from the source.
The provider can distinguish absence from a source failure. The config
implementation separates recoverable absence from invalid values and source
errors, so fallback need not hide bad configuration. Sources: those same files.

i has `process.Environment`, `env`, arguments, a CLI parser, JSON, schemas and
filesystem access. It has no shared config/provider module. Sources:
`lib/i/process.i`, `std/src/cli.i`, `std/src/json.i`,
`std/src/schema.i`, `lib/i/fs.i`.

Make `ConfigProvider` an ordinary service whose reads return explicit missing,
present and source-error cases. Use schema data to decode config. Compose
environment, file, CLI and in-memory sources explicitly in the program entry.
Specify precedence, empty versus missing values, nested path syntax and
fallback policy. Do not default an invalid port as though it were absent.
Add safe rendering for secrets; string fields in `std/log.Entry` do not provide
redaction by themselves. A secret wrapper must not be advertised as erasing
the original string from memory.

## 12. Logger, Metric and Tracer

### Effect evidence

| Source | Central types | Selected entry points | Problem solved |
|---|---|---|---|
| `effect/packages/effect/src/Logger.ts`; `effect/packages/effect/src/Effect.ts` | `Logger<Message,Output>`, logger `Options` | `Logger.make`, `Logger.map`, `Logger.formatSimple`, `Logger.formatLogFmt`, `Logger.formatJson`, `Logger.batched`, `Logger.toFile`, `Logger.layer`, `Effect.log`, `Effect.annotateLogs`, `Effect.withLogSpan` | Structured events, filtering/formatting, sinks, batching and contextual fields |
| `effect/packages/effect/src/Metric.ts` | `Metric<Input,State>`, `Counter`, `Gauge`, `Frequency`, `Histogram`, `Summary`, `MetricRegistry` | `counter`, `gauge`, `frequency`, `histogram`, `summary`, `timer`, `update`, `modify`, `value`, `withAttributes`, `snapshot`, `dump` | Typed measurement aggregation and exportable snapshots |
| `effect/packages/effect/src/Tracer.ts`; `effect/packages/effect/src/Effect.ts` | `Tracer`, `Span`, `SpanStatus`, `SpanLink`, `ParentSpan` | `Tracer.make`, `Tracer.externalSpan`, `Effect.makeSpan`, `Effect.makeSpanScoped`, `Effect.withSpan`, `Effect.withParentSpan`, `Effect.currentSpan`, `Effect.annotateCurrentSpan`, `Effect.linkSpans` | Parent/child operation lifetimes, attributes, links and completion status |

### i today and the gap

`std/src/log.i` already has structured `Entry`, four levels, `Sink(E,N)`,
lazy messages, filtering, minimum level, deterministic formatting, stdout,
stderr, file output and explicit timestamps. Its errors and needs remain
visible. A sink is a provided service. `std/test/log_test.i` tests this model.
This should be extended, not replaced by Effect-shaped logging calls.

There is no metric registry or tracer/span module in the audited library
inventory. Common contextual fields, fan-out, JSON formatting and scoped
batching are also absent from `std/src/log.i` exports. File logging currently
opens, writes, flushes and closes for every accepted entry.

Use immutable log context and span context as services. Enter a span with a
resource bracket and provide its parent context to children. Record its ending
state explicitly; ordinary error catch alone cannot observe a stop. Metrics
can aggregate immutable state behind a scoped owner task/ref and export
snapshots. No process-global mutable registry is required.

Start with counters, gauges, histograms, durations and a test recorder.
Then add exporters and wire propagation through HTTP. Batching and exporters
must be scoped child tasks with bounded queues, defined overflow policy and
shutdown flushing. They must not detach work or hide logging failures.

## 13. Duration, DateTime, Clock, Random and test time

### Effect evidence

| Source | Central types | Selected entry points | Problem solved |
|---|---|---|---|
| `effect/packages/effect/src/Duration.ts` | `Duration`, `DurationValue`, `Input` | `nanos`, `millis`, `seconds`, `fromInput`, `fromIso`, `sum`, `subtract`, `times`, `divide`, `toNanos`, `format`, `formatIso` | Units, arithmetic, conversions and textual duration formats |
| `effect/packages/effect/src/DateTime.ts` | `Utc`, `Zoned`, `TimeZone`, `TimeZoneNamed`, `TimeZoneOffset`, `Disambiguation` | `make`, `makeZoned`, `makeZonedFromString`, `now`, `zoneMakeNamed`, `setZone`, `toUtc`, `toParts`, `add`, `startOf`, `endOf`, `formatIso` | Calendar and instant work, including named zones and local-time disambiguation |
| `effect/packages/effect/src/Clock.ts`; `effect/packages/effect/src/Effect.ts` | `Clock` | `Clock.clockWith`, `Clock.currentTimeMillis`, `Clock.currentTimeNanos`, `Clock.monotonicTimeNanos`, `Clock.sleep` service method, `Effect.sleep`, `Effect.timed` | Replaceable wall time, elapsed time and sleeping |
| `effect/packages/effect/src/Random.ts` | `Random` | `next`, `nextBoolean`, `nextInt`, `nextBetween`, `nextIntBetween`, `shuffle`, `choice`, `withSeed` | Replaceable random draws and repeatable seeded runs |
| `effect/packages/effect/src/testing/TestClock.ts` | `TestClock`, `TestClock.Options` | `make`, `layer`, `testClockWith`, `adjust`, `setTime`, `withLive` | Drive sleepers, timeouts and schedules without real waiting |

TestClock's layer also installs a tracking scheduler. Merely providing the
clock returned by `make` does not guarantee ordering of sleeps registered
after several scheduler hops. Source:
`effect/packages/effect/src/testing/TestClock.ts`.
The testing barrel exports TestClock, TestConsole and TestSchema, with no
separate TestRandom module. Seeded random control is `Random.withSeed`.
Sources: `effect/packages/effect/src/testing/index.ts`,
`effect/packages/effect/src/Random.ts`.

### i today and the gap

`std/src/time.i` has exact signed nanosecond `Duration` and Unix `Instant`,
arithmetic, comparison, sleep and monotonic measurement. `std/src/calendar.i`
has proleptic Gregorian dates, civil times, fixed offsets, weekday/day arithmetic,
and parsing/formatting. It explicitly has no timezone database or leap seconds.
`lib/i/clock.i` exposes replaceable wall/monotonic reads and sleep requests.

`lib/i/random.i` provides immutable SplitMix generators, bounded integer and
float draws, split and path-derived streams. `lib/i/task.i` has per-task
generator access. This is already stronger explicit control than replacing
every draw with ambient random state. Secure bytes are a separate capability:
`lib/i/process.i` exposes entropy; `std/src/crypto.i` provides `random_bytes`.

`std/test/clock_test.i` supplies a fixed fake whose sleeps return ready requests.
That tests clock substitution. It does not implement a virtual timer queue.
`task.test` supplies seeded scheduling and scripted task paths, but that alone
does not advance time. Sources: `std/test/clock_test.i`; `lib/i/task.i`.

The largest gap is reusable virtual time integrated with task waits. Define
sleep registration, advancing to deadlines, cancellation removal and quiescence
before advancing again. The public request API has no user-completable pending
request pair, so this may need a task-level clock abstraction or an ordinary
service factory with a controlled wait route. Do not implement a fake that
blocks the scheduler or just returns immediate sleeps.
Sources for the constraint: `lib/i/clock.i`, `lib/i/request.i`,
`lib/i/task.i`.

Smaller gaps are duration parsing/formatting and random collection helpers.
Named timezone support is a separate larger feature needing timezone data and
clear gap/fold policy. Keep exact i integers; do not import JavaScript safe
integer limits or Duration infinity by accident.

## 14. Request batching and Cache

Effect has logical `Request<A,E,R>` descriptions and
`RequestResolver<Request>`. Important entry points are `Request.of`,
`Request.tagged`, `Request.succeed`, `Request.fail`, `Effect.request`,
`RequestResolver.make`, `RequestResolver.makeGrouped`,
`RequestResolver.fromFunctionBatched`, `RequestResolver.fromEffectTagged`,
`RequestResolver.batchN`, `RequestResolver.setDelay`,
`RequestResolver.asCache`, and `RequestResolver.withCache`.
Sources: `effect/packages/effect/src/Request.ts`,
`effect/packages/effect/src/RequestResolver.ts`,
`effect/packages/effect/src/Effect.ts`.

`Cache<Key,A,E,R>` supports `make`, `makeWith`, `get`, `getOption`, `getSuccess`,
`set`, `has`, `refresh`, `invalidate`, `invalidateAll`, `size`, and `entries`.
It shares a lookup already in progress, bounds capacity, supports TTL and stores
successful or failed results. TTL can depend on the lookup's `Exit` and key.
Source: `effect/packages/effect/src/Cache.ts`.

These solve two distinct problems: combine many logical backend requests into
one batch, and avoid repeated or concurrent duplicate lookup work.

i's `Request(A)` is a host-operation handle, not this logical data-loading
description. It already has ownership, cancellation and replaceable waits.
Do not replace it or overload its name with a resolver abstraction.
Source: `lib/i/request.i`; `docs/spec.md` §9.11.
There is no public logical batching or cache module in `lib/i/` or `std/src/`.
HTTP's idle connection pool is specific transport reuse, not a general cache.
Source: `std/src/http.i`, `Pool`, `network_client`.

Add a scoped cache whose loader is a normal function with needs and `Fail(E)`.
Store immutable entry state behind an owner task/ref; coordinate in-progress
lookups with a one-shot cell. Specify eviction, error TTL, invalidation during
load, and whether cancelling one waiter leaves shared work running. Capture or
provide loader services deliberately. A value loaded with one service instance
must not be accidentally reused as though produced by another.

Add batching after coordination and cache contracts. Make maximum batch size,
flush delay, grouping keys and scope lifetime explicit. Missing per-request
responses must complete with a defined error, not leave waiters asleep.
Neither feature needs a library computation wrapper.

## 15. Match and value libraries

### Match

Effect's central types are `Matcher`, `TypeMatcher` and `ValueMatcher`.
Useful functions are `type`, `value`, `when`, `whenOr`, `whenAnd`, `tag`,
`discriminator`, `tagsExhaustive`, `orElse`, `result`, `option`, and
`exhaustive`. This provides composable narrowing and exhaustive dispatch in
TypeScript. Source: `effect/packages/effect/src/Match.ts`.

i already has first-match function branches, patterns, guards, unions, type
tests and exhaustiveness checks. Sources: `docs/spec.md` §§6.12 and 7;
`docs/design/syntax-3.md` §§2.5–2.6. There is no missing Match framework.
Small predicate combinators may be useful, but errors and domain data should
use normal constructors, not `_tag` strings or a second pattern evaluator.

### Data, equality, hashing and order

| Effect source and central types | Selected entry points | i today and remaining gap |
|---|---|---|
| `effect/packages/effect/src/Data.ts`: immutable data class and tagged enum helpers | `Class`, `TaggedClass`, `taggedEnum`, `Error`, `TaggedError` | Labelled data, constructors and unions are language features. No Data class hierarchy is needed. `docs/spec.md` §§4.3–4.4. |
| `effect/packages/effect/src/Equal.ts`: `Equal` protocol | `equals`, `asEquivalence`, `isEqual`, `makeCompareMap`, `makeCompareSet`, `byReference` | i has one `==` with specified structural and identity rules. Do not add overridable equality. `docs/spec.md` §11.4. |
| `effect/packages/effect/src/Hash.ts`: `Hash` protocol | `hash`, `combine`, `number`, `string`, `structure`, `array` | `core.hash` already agrees with i equality. No user hash protocol is required. `lib/i/core.i`; `docs/spec.md` §13. |
| `effect/packages/effect/src/Order.ts`: `Order<A>` | `make`, `flip`, `combine`, `combineAll`, `mapInput`, `Tuple`, `Struct`, `min`, `max`, `clamp` | `std/order` has order values and tie-breaking. Type modules have compare functions. Comparator projection and composition are useful ordinary functions. `std/src/order.i`, `std/src/int.i`, `std/src/string.i`. |

Effect 4's `Equal.equals` compares plain arrays and objects structurally by
default and supports custom equality. That is a change from v3, documented in
`effect/migration/equality.md` and implemented through
`effect/packages/effect/src/Equal.ts`. This does not justify a custom equality
hook in i.

Two existing collection contracts need attention before promising value
interchangeability. `std/src/dict.i` documents insertion-order differences
inside equal-hash collision buckets. `std/src/sequence.i` documents that `==`
and hash observe private tree shape; its `equal` compares elements instead.
`std/test/sequence_test.i` deliberately checks element observations. Decide
and document canonical representations or explicit conversion boundaries.
This audit recommends representation fixes where feasible, not a second `==`.

### Options, results and collections

| Effect source and central types | Selected entry points | i today and gap |
|---|---|---|
| `effect/packages/effect/src/Option.ts`: `Option<A>`, `Some`, `None` | `some`, `none`, `map`, `flatMap`, `match`, `all`, `filter`, `zipWith` | `std/src/option.i` already has `Some/None`, map/then, flatten, defaults, all/filter/map2 and Result conversion. Mostly naming and consistency work. |
| `effect/packages/effect/src/Result.ts`: `Result<A,E>`, `Success`, `Failure` | `succeed`, `fail`, `map`, `mapError`, `flatMap`, `match`, `all`, `orElse`, `fromOption` | This checkout has `Result.ts`, not `Either.ts`. i has prelude `Result/Ok/Error` and a rich `std/src/result.i`. Keep one result type; do not add Either. |
| `effect/packages/effect/src/Array.ts`: readonly and non-empty array types | `fromIterable`, `range`, `get`, `map`, `flatMap`, `filter`, `reduce`, `sortBy`, `groupBy`, `chunksOf`, `window`, `zip` | `std/src/list.i` already covers most transformations, grouping, sorting, windows, chunks, scans and combinations. `std/src/sequence.i` supplies persistent indexed operations. No need for a mutable Array clone. |
| `effect/packages/effect/src/Record.ts`: `ReadonlyRecord<K,V>` | `empty`, `fromEntries`, `get`, `set`, `remove`, `map`, `mapKeys`, `filter`, `union`, `intersection` | Dynamic string keys fit `std/dict.Dict(String,A)`. Closed record shapes remain language data, not runtime field maps. `std/src/dict.i`; `docs/spec.md` §6.4. |
| `effect/packages/effect/src/HashMap.ts`: `HashMap<K,V>` | `empty`, `fromIterable`, `get`, `set`, `remove`, `modifyAt`, `union`, `map`, `filter`, `reduce` | `std/src/dict.i` is an immutable hash trie with broad operations. No absent hash map foundation. Address collision equality and document traversal order. |
| `effect/packages/effect/src/HashSet.ts`: `HashSet<A>` | `empty`, `fromIterable`, `add`, `has`, `remove`, `union`, `intersection`, `difference`, `isSubset` | `std/src/set.i` is already built on Dict and has these capabilities plus symmetric difference. It inherits the representation concerns of Dict. |
| `effect/packages/effect/src/Chunk.ts`: `Chunk<A>`, `NonEmptyChunk<A>` | `fromIterable`, `append`, `appendAll`, `take`, `drop`, `get`, `map`, `flatMap`, `reduce`, `toArray` | `std/src/sequence.i` supplies a persistent measured finger tree with append, split and indexed access. It overlaps Chunk's role. Streaming batches can use List or Sequence; do not add another general sequence without a measured need. |

Effect's Array/Chunk/HashMap families also have different representation and
interop roles in TypeScript. Their existence is not evidence that i needs all
of those public type names. The capabilities above come from the named module
files; the recommendation is to keep one home for each operation in i.

### Text and numbers

| Effect source and central types | Selected entry points | i today and gap |
|---|---|---|
| `effect/packages/effect/src/String.ts`: string helpers | `length`, `slice`, `split`, `trim`, `replaceAll`, `normalize`, `codePointAt`, `linesIterator`, `snakeCase`, `camelCase` | `std/src/string.i` already has scalar indexing, slicing, searching, case conversion, line/word splitting and traversal. Missing normalization, grapheme operations and richer naming transforms need explicit Unicode policy. Preserve codepoint indexes. |
| `effect/packages/effect/src/Number.ts`: numeric helpers | `sum`, `multiply`, `divide`, `remainder`, `clamp`, `parse`, `round`, `sumAll` | `std/src/int.i` and `std/src/float.i` already cover parsing, arithmetic, aggregation, comparison, powers, rounding and trigonometry. i Float is finite; do not copy NaN/infinity or zero-divisor behavior. `docs/spec.md` §§11.2–11.3. |
| `effect/packages/effect/src/BigInt.ts`: arbitrary-size integer helpers | `sum`, `multiply`, `divide`, `remainder`, `gcd`, `lcm`, `sqrt`, `fromString`, `toNumber` | i Int is already arbitrary size. `lib/i/big.i` contains support arithmetic; `std/src/int.i` exposes the public numeric API. Add missing useful algorithms such as gcd/lcm there, not a second integer type. |
| `effect/packages/effect/src/BigDecimal.ts`: `BigDecimal`, `RoundingMode` | `make`, `normalize`, `fromString`, `format`, `sum`, `multiply`, `divide`, `scale`, `round`, `truncate`, `floor`, `ceil` | No decimal module appears in the audited i inventory. Exact decimal coefficient/scale, parsing and explicit rounding are a real gap for money and data exchange. Build on Int. |

Effect BigDecimal supplies an exact decimal representation and explicit rounding
facilities; the selected functions and model are in
`effect/packages/effect/src/BigDecimal.ts`. For i, specify division precision,
rounding, normalization and equality together. Do not represent money with
binary Float or copy an unsafe conversion as the only conversion API.

## 16. Platform

Platform capabilities are ordinary service values. Keep algorithms in i and
use host operations only where the host must act. Backend choice belongs at
the entry. The table separates interfaces from concrete adapters.

### HTTP client and server

| Effect source | Central types | Selected entry points |
|---|---|---|
| `effect/packages/effect/src/http/HttpClient.ts`; `effect/packages/effect/src/http/HttpClientRequest.ts`; `effect/packages/effect/src/http/HttpClientResponse.ts` | `HttpClient`, `HttpClientRequest`, `HttpClientResponse` | `HttpClient.make`, `execute`, `mapRequest`, `filterStatusOk`, `retryTransient`, `timeout`, `followRedirects`, `withCookiesRef`, `withRateLimiter`, `HttpClientRequest.bodyStream`, `HttpClientRequest.schemaBodyJson`, `HttpClientResponse.stream`, `HttpClientResponse.schemaJson` |
| `effect/packages/effect/src/http/HttpServer.ts`; `effect/packages/effect/src/http/HttpRouter.ts`; `effect/packages/effect/src/http/HttpServerRequest.ts`; `effect/packages/effect/src/http/HttpServerResponse.ts` | `HttpServer`, `HttpRouter`, `Route`, `HttpServerRequest`, `HttpServerResponse` | `HttpServer.serve`, `HttpServer.makeTestClient`, `HttpRouter.add`, `route`, `middleware`, `cors`, `schemaPathParams`, `schemaJson`, `HttpServerRequest.schemaBodyMultipart`, `HttpServerResponse.stream`, `HttpServerResponse.schemaJson`, `HttpServerResponse.file` |

The problem solved is a shared request/response model across backends, tests,
streaming bodies, routing, middleware and decoding. Concrete implementations
are in `effect/packages/platform/node/src/NodeHttpClient.ts`,
`effect/packages/platform/node/src/NodeHttpServer.ts`, and
`effect/packages/platform/browser/src/BrowserHttpClient.ts`.
The Node server accepts HTTP/1 and HTTP/2 server implementations and handles
upgrades; its comments delimit which features work with HTTP/2. Source:
`effect/packages/platform/node/src/NodeHttpServer.ts`.

i already has an incremental HTTP parser/serializer, headers, queries, routes,
client and transport services, native TCP/TLS and fetch clients, redirect
handling, idle connection reuse, body decoders, timeout and server loops.
Sources: `std/src/http.i`, `std/src/uri.i`, `lib/i/fetch.i`,
`lib/i/net.i`, `lib/i/tls.i`. `std/test/http_network_test.i` contains network,
redirect and timeout cases. `examples/http-get/src/main.i` and
`examples/http-server/src/main.i` show actual application use.

The most concrete gap is native response streaming. `native_client.stream`
calls the buffered `provider.send` first, then reads slices from a `task.ref`
holding `res.body`. It is a pull facade over a buffered body. `fetch_stream`
does incrementally call the host reader. Server `serve_stream` gives a live
request reader only during the handler, but returns the existing buffered
`Response` model. Source: `std/src/http.i`.

Make native and fetch lifetime/stream contracts consistent. Generalize timeout
and retry policies instead of keeping them HTTP-only. Add bounded server
admission; `accept_loop` currently spawns for each accepted connection without
a limit. Add body-size limits at consumption boundaries, response streaming,
middleware composition and test client support. Cookies, multipart and tracing
are later concrete capabilities. Sources for current behavior:
`std/src/http.i`, `serve_stream`, `accept_loop`, `ResponseStream`,
`native_client`, `fetch_stream`, `send_timeout`.

There is also a lifetime boundary to fix before supporting a live native body.
`send_stream` wraps `provider.stream(req)` in `resource.ensure` with
`provider.close`, so the provider is closed before the returned reader is
consumed. The current native body is buffered and the fetch provider's close
is a no-op; these do not establish safety for a transport whose reader needs
the provider to remain open. Make the consumer own that scope, or transfer
the provider's close into a scoped reader release. Source: `std/src/http.i`,
`send_stream`, `native_client`, `fetch_client`.

Connection reuse exists, but ordinary `http.send` creates and closes a client
per call. An explicitly retained `http.client()` can share its transport.
Document and scope that construction before claiming cross-call pooling as a
default. Source: `std/src/http.i`, `client`, `send`, `network_client`.

### FileSystem and Path

Effect's `FileSystem`, `File`, `WatchEvent` and `WatchBackend` define portable
filesystem access. Selected operations are `open`, `readFile`, `writeFile`,
`readDirectory`, `stat`, `makeDirectory`, `makeTempDirectoryScoped`,
`copy`, `rename`, `remove`, `stream`, `sink`, and `watch`.
File handles also offer `seek` and `sync`.
Source: `effect/packages/effect/src/FileSystem.ts`.
The Node adapter delegates to the shared Node backend. Source:
`effect/packages/platform/node/src/NodeFileSystem.ts`.

i already has file handles, modes, open/read/write/flush/close, remove, rename,
mkdir, listing, metadata, temp directory, whole-file helpers and line reading.
Source: `lib/i/fs.i`. `examples/io-fs/src/main.i` demonstrates filesystem use.
Gaps include seeking, richer metadata/permissions/links, scoped temporary
resources, copying, watching and shared stream/sink adapters. Some require
new host operations; the combinators and cleanup should remain ordinary i.

Effect `Path` supplies `join`, `normalize`, `basename`, `dirname`, `extname`,
`parse`, `format`, `relative`, `resolve`, `resolveWithin`, `fromFileUrl`, and
`toFileUrl`. Sources: `effect/packages/effect/src/Path.ts`,
`effect/packages/platform/node/src/NodePath.ts`. The latter has host-default,
POSIX and Windows providers.

`std/src/path.i` already offers lexical POSIX cleaning, joining, splitting,
base/dir/extension, absolute checks and relative paths. It deliberately does
not consult a current directory or treat backslashes as separators.
Keep this pure API. Add explicit native/Windows path support and file-URL
conversion if required. Do not silently turn lexical helpers into effects.
Do not equate lexical normalization with filesystem containment across links.

### Command and Terminal

Effect's OS command model is in `process/ChildProcess.ts`, not the CLI's
`Command.ts`. `Command`, `StandardCommand` and `PipedCommand` are descriptions;
`ChildProcessSpawner` produces `ChildProcessHandle`. Important entry points
include `ChildProcess.make`, `pipeTo`, `prefix`, `setCwd`, `setEnv`,
`ChildProcessSpawner.make`, and handle operations `stdin`, `stdout`, `stderr`,
`exitCode`, and `kill`. Sources:
`effect/packages/effect/src/process/ChildProcess.ts`,
`effect/packages/effect/src/process/ChildProcessSpawner.ts`.
The concrete Node entry re-exports its shared implementation. Source:
`effect/packages/platform/node/src/NodeChildProcessSpawner.ts`.

i has explicit process/argument/environment/cwd/entropy services, spawning,
piped stdio, environment and directory choices, wait, terminate and close.
Source: `lib/i/process.i`. `examples/io-process/src/main.i` and
`examples/io-process/test/process_test.i` show process use.
The gap is reusable immutable command descriptions and scoped pipeline
composition, output collection and streaming adapters. Drain stdout and stderr
concurrently. Specify child termination and waiting on parent cancellation.
No shell parser or detached child is necessary for the first API.

Effect `Terminal` has `Size`, `Key`, `UserInput`, `QuitError`, and operations
`columns`, `rows`, `size`, `readInput`, `readLine`, `display`, and `make`.
Sources: `effect/packages/effect/src/Terminal.ts`,
`effect/packages/platform/node/src/NodeTerminal.ts`.
These solve interactive sizing, key events and line input.

i has byte stdin/stdout/stderr with flush and text helpers, plus a CLI parser.
Sources: `lib/i/stdio.i`, `lib/i/io.i`, `std/src/cli.i`.
It lacks terminal size/events, raw input and interactive line editing.
Those need an optional terminal host service and scoped restoration of terminal
mode. Keep byte stdio available on targets with no terminal.

### KeyValueStore

Effect offers a `KeyValueStore` service, `SchemaStore` and storage errors.
Useful operations are `get`, `getUint8Array`, `set`, `remove`, `modify`,
`keys`, `size`, `prefix`, `toSchemaStore`, `layerMemory`, and `layerFileSystem`.
Sources: `effect/packages/effect/src/persistence/KeyValueStore.ts`,
`effect/packages/platform/browser/src/BrowserKeyValueStore.ts`.
The browser file supplies local-storage, session-storage and IndexedDB layers.

i has dictionaries, files and schemas but no common durable key/value service.
Sources: `std/src/dict.i`, `lib/i/fs.i`, `std/src/schema.i` and the audited
module inventory. Define an ordinary service with memory and file providers,
then typed schema views. Specify atomic update guarantees, missing keys,
serialization errors and persistence boundaries. A file backend's atomicity
must come from an explicit file protocol, not the dictionary API.

### Socket and Worker

Effect's `Socket`, `Reader`, `Writer`, `SocketError` and WebSocket interfaces
have `make`, `readerBytes`, `readerString`, `toChannel`, `toStream`,
`makeWebSocket`, `fromWebSocket`, `makeWebSocketChannel`, and
`fromTransformStream`. `SocketServer` exposes address and a `run` loop.
Sources: `effect/packages/effect/src/socket/Socket.ts`,
`effect/packages/effect/src/socket/SocketServer.ts`,
`effect/packages/platform/node/src/NodeSocket.ts`.
The socket reader is pull-based; the abstract interface does not guarantee
that every concrete WebSocket transport can pause incoming traffic.
The Node global-WebSocket adapter documents buffering limits and this concern.
Sources: those Socket and NodeSocket files.

i already has TCP, UDP, DNS and TLS, with shutdown and byte-stream adapters.
Sources: `lib/i/net.i`, `lib/i/tls.i`;
`examples/io-net/src/main.i`, `examples/io-udp/src/main.i`,
`examples/io-tls/src/main.i`.
Main gaps are common scoped stream adapters, WebSocket framing/upgrades and
explicit local-socket support. Keep transport-specific behavior visible.

Effect's worker interfaces are `Worker`, `WorkerPlatform`, `Spawner`,
`WorkerRunner` and `WorkerRunnerPlatform`. Core entry points are
`Worker.makeUnsafe`, `Worker.makePlatform`, `Worker.layerSpawner`,
worker `run`/`send`, and runner `run`/`send`. Concrete adapters offer
`NodeWorker.layerPlatform/layer` and `BrowserWorker.layerPlatform/layer`.
Sources: `effect/packages/effect/src/workers/Worker.ts`,
`effect/packages/effect/src/workers/WorkerRunner.ts`,
`effect/packages/platform/node/src/NodeWorker.ts`,
`effect/packages/platform/browser/src/BrowserWorker.ts`.
They bridge host worker/message protocols and bind cleanup to scope.

i already schedules portable tasks across host-selected workers. Worker count
is deliberately unobservable; no task API selects a worker. Sources:
`docs/spec.md` §§9.9, 11.9; `lib/i/task.i`.
Do not add Worker as a competing concurrency model. Host worker or process
interop, if needed, should be an explicit scoped transport service with schemas
and structured ownership. It is not required for ordinary parallel i code.

## 17. Testing support

Effect's selected testing surface is `it.effect`, `it.live`, `it.layer`,
`it.effect.each`, `it.effect.prop`, `beforeAll`, `beforeEach`,
`TestClock.adjust`, `TestClock.setTime`, `TestConsole.logLines`,
`TestConsole.errorLines`, `TestSchema.Asserts.decoding`,
`TestSchema.Asserts.encoding`, `Arbitrary.schema`, and
`Arbitrary.checkEffect`. Central models are scoped tests, shared test layers,
`TestClock`, `TestConsole`, schema assertions and generated property results.
Sources: `effect/packages/vitest/src/index.ts`,
`effect/ai-docs/src/09_testing/10_effect-tests.ts`,
`effect/packages/effect/src/testing/TestClock.ts`,
`effect/packages/effect/src/testing/TestConsole.ts`,
`effect/packages/effect/src/testing/TestSchema.ts`,
`effect/packages/effect/src/Arbitrary.ts`.

This solves deterministic time, replaceable outputs, fixture cleanup,
parameterized cases, property generation and codec laws. The testing integration,
TestConsole, TestSchema and Arbitrary inspected here are marked unstable in
their named files. Do not assume their whole surface is a settled standard.

i already has library macros `test`, `skip`, `only`, `fails`, `suite` and
`each` in `std/src/test.is`. They register collected items rather than giving
test syntax privileged compiler semantics. `lib/i/assertion.i` catches test
assertion failures. `std/src/property.i` has pure generators, lazy shrink
trees, deterministic seeds, budgets, reports and replay.
`std/test/property_test.i` exercises generator and shrinking behavior.
`lib/i/task.i` has seeded/scripted scheduling through `task.test`.

Extend these tools with shared scoped fixtures, output/metric/span recorders,
schema-derived generators and reusable round-trip assertions. The key missing
tool is the virtual timer scheduler described in §13. Preserve separate
controls for random seed, scheduler choices and virtual time; one seed alone
does not determine host I/O arrival order.

The checked-in `std/src/test.is` still recognizes an older suite environment
statement in its syntax matcher. Reconcile it with ordinary provide during
the syntax/effects migration. This is a source observation, not a claim about
a failing test run. Avoid redesigning the test runner solely to match Vitest.

## 18. Narrow checks against Gleam and Go

These checks constrain the translation. They are not a full audit of those
libraries, which is outside part B.

| Source read | Observation | Consequence for i |
|---|---|---|
| `gleam-stdlib/src/gleam/dict.gleam` | Public dictionaries are immutable, permit arbitrary key types and make no traversal-order guarantee. JavaScript transient construction is private. | Keep one immutable dictionary API and state order/equality laws. Do not expose mutation because a backend uses it internally. |
| `gleam-http/src/gleam/http/service.gleam` | A service and middleware are ordinary functions. This particular module is deprecated in favor of middleware packages. | Ordinary function composition fits i; copying this deprecated module's exact API would be a poor stability choice. |
| `gleam-otp/src/gleam/otp/actor.gleam` | Typed actors hold state and handle messages sequentially. | A scoped owner task is a plausible implementation for queues, synchronized updates and caches. Take the sequential ownership idea without importing the whole OTP lifecycle. |
| `gleam-erlang/src/gleam/erlang/process.gleam` | Typed Subjects and selectors coexist with linked and unlinked process spawning. | Typed communication is useful prior art. Unlinked spawn conflicts with i's structured-only rule. |
| `go/src/context/context.go` | Context carries cancellation, deadlines and request values. Cancellation does not itself state that all associated work has completed; callers must release cancel functions. | Put lifetime and completion in task groups and scopes. Do not add a catch-all Context parameter alongside typed needs. |
| `go/src/io/io.go` | Small Reader/Writer interfaces state partial-read, short-write and EOF behavior explicitly. | Use small composable byte protocols, translated to immutable buffers. Keep end-of-input and failure distinct, with precise progress laws. |
| `go/src/errors/join.go` | `Join` keeps several errors and exposes them for inspection. | Aggregated diagnostics are useful data. They do not require changing the language's one-error service or task-scope failure selection. |

## 19. Ranked gaps and dependencies

Importance measures a useful first release, not similarity to Effect.
P0 is a contract or correctness foundation. P1 is widely useful application
functionality. P2 is important but can follow the foundation. P3 is specialized.
Size estimates describe scope, not elapsed time: S is one focused module or
guide; M is a few modules; L crosses several families; XL is a major subsystem.
Ranks below refer back to the source evidence in §§3–17.

| Rank | Area and concrete deliverable | Importance | Size | i features needed | Dependencies |
|---|---|---|---|---|---|
| G1 | Reconcile resource `Scope`, scoped acquisition, cleanup and task `Group` naming | P0: reliable application lifetimes | M–L | Needs/provide, control unwinding, non-failing finals, requests, tasks | E3 `Fail(E)` baseline; E5 resource/name work must be accounted for separately |
| G2 | Publish one error API and explicit error/outcome/stop contracts | P0: stable composition and diagnostics | S–M | `Fail(E)`, unions, matching, Result, assertion service | E3; coordinate with G1, without changing stop/cancel rules |
| G3 | Bounded task traversal and general computation timeout | P1: safe fan-out from day one | M | Groups, tasks, events, `Fail(E)`, clock | G1; use G5 to verify time behavior |
| G4 | Queue, Deferred and Semaphore first; then PubSub and Latch | P1: shared foundation for pipelines and services | L | Immutable state, refs/owner tasks, events, scope | G1, G2; define cancellation and acquisition handoffs |
| G5 | Scheduler-aware virtual clock and test recorders | P1: fast deterministic lifetime and policy tests | M–L | Clock/Await services, requests, task scheduling, immutable timer state | G1; may use G4; settle how fake pending sleeps are completed |
| G6 | Schedule data/steps, retry/repeat, backoff, jitter and composition | P1: resilient network and polling programs | M | Ordinary computations, `Fail(E)`, Duration, Generator, clock | G2, G5; general timeout from G3 |
| G7 | Scoped stream consumption, byte/text adapters and reusable sinks | P1: process large data safely | L | Needs, resource scope, tasks/events, immutable parser state | G1, G4, G5; timed operators use G6 |
| G8 | Schema descriptions as data; shared validation, metadata, JSON Schema and generators | P1: safe input and useful integration from the start | L | Immutable sums/data, functions, `Fail(E)`, optional ordinary macros | G2; reuse property generators; no dependency on a new compiler schema facility |
| G9 | Typed ConfigProvider, precedence, missing/invalid distinction and safe secret rendering | P1: deployable applications | M | Ordinary services, schema data, Result/Fail, files/environment | G8 for full derivation; basic provider can land earlier |
| G10 | HTTP native body streaming, response streaming, bounded admission and unified policies | P1: usable services and clients | L | Existing HTTP/net/TLS, resource scope, tasks, streams, schemas | G1, G3, G6, G7; G8 for richer decode; tracing uses G11 |
| G11 | Metric and Tracer services; contextual logging; scoped exporters | P1: understandable running programs | L | Provided contexts, refs/owner tasks, clock, scope | G1, G4, G5; G7 for export streams; build small recorders first |
| G12 | Settle Dict collision and Sequence representation/equality contracts | P0 contract, localized work | M–L | Existing data equality/hash, canonical immutable representations | No new language feature; do before stable collection and cache promises |
| G13 | Scoped cache with shared in-progress loads, capacity, TTL and invalidation | P2: efficient service code | M–L | Dict, immutable entry state, needs, `Fail(E)`, scope | G1, G4, G5, G12 |
| G14 | Logical request batching with explicit flush policy | P2: avoid backend N+1 work | M–L | Owner task, events, ordinary request descriptions, schemas | G4, G5, G13; leave host `i/request` intact |
| G15 | Scoped immutable command pipelines and common file/socket stream protocols | P1: practical scripts and tools | M–L | Existing process/fs/net services, task groups, resource scope | G1, G3, G7; concurrent stdout/stderr draining |
| G16 | Filesystem depth: seek, permissions/links, temporary resources, watch | P2: complete host tools | L | New ordinary host service operations, request safety, streams | G1, G7; target providers must expose supported operations honestly |
| G17 | Shared scoped test fixtures and schema law helpers | P1: maintainable stable library | M | Existing item macros, provide, resource scope, property module | G1, G5, G8; fix suite provide matching during migration |
| G18 | Decimal arithmetic and explicit rounding; duration text; numeric/collection helpers | P2: exact business data and everyday completeness | M | Int, immutable data, Result, existing number/order modules | G12 for representation laws; no separate BigInt type |
| G19 | Dynamic keyed work registry and serialized effectful updates | P2: service supervision within one lifetime | M–L | Groups, tasks, events, Dict, owner-task services | G1, G4; define cancellation/replacement and reentrancy first |
| G20 | KeyValueStore with memory/file providers and schema views | P2: simple durable application state | M | Ordinary service, files, schema codecs, scoped state | G1, G8, G12; specify backend atomicity |
| G21 | Interactive Terminal, explicit native path variants, WebSocket/local socket support | P2: broader target usefulness | M–L | Optional host services, immutable events, scopes, protocol parsers | G1, G4, G7; keep pure POSIX path and byte stdio |
| G22 | Named timezone data, local-time disambiguation and cron | P2: correct civil scheduling | L | Calendar data, clock service, immutable database | G5, G6; data/version policy needed |
| G23 | Transaction journals and multi-cell retry subscriptions | P3: specialized atomic coordination | XL | Scheduler arbitration, refs, events, ordinary Transaction service | G4, G5, G12; first evaluate aggregate-state refs |
| G24 | External host worker protocol interop | P3: specific integrations | L | Schemas, scoped transport service, requests/tasks | G1, G7, G8; ordinary i parallelism already exists |

G12 ranks below new application foundations in delivery order but is P0 for
declaring the affected collection contracts stable. It can proceed independently.
The main dependency chain is G1 → G4 → G7 → G10/G15. G5 should accompany
G3/G4/G6. G8 feeds G9, G17 and G20. Start these foundations together rather
than expanding many unrelated module export lists.

## 20. Effect designs not to copy

| Effect design and source | Why i already covers it, or why it conflicts | What to keep instead |
|---|---|---|
| `Effect<A,E,R>`, `gen`, flat-map pipelines and run APIs — `effect/packages/effect/src/Effect.ts` | i functions already carry result, errors-as-needs and service needs. Calls and bindings already sequence computations. | Ordinary computations and higher-order helpers. No wrapper interpreter or `yield` DSL. |
| Context tags, string identities and service lookup — `effect/packages/effect/src/Context.ts` | i services are labelled values keyed by their type constructor; the type checker proves provision. | Service patterns and `<-`, distinct labelled types for distinct roles. |
| Layer graph objects and implicit memoization — `effect/packages/effect/src/Layer.ts` | Service constructors are ordinary functions. Construction order, sharing and provision can be explicit in one place. | Scoped factories, bind-once sharing, optional ordinary cache for demonstrated lazy construction. |
| A separate E channel and tag-specific error combinator family — `effect/packages/effect/src/Effect.ts` | `Fail(E)`, unions and normal matching express recovery. | One `i/fail` implementation and normal functions. |
| Effect's full defect/interruption capture via Cause/Exit — `effect/packages/effect/src/Cause.ts`; `effect/packages/effect/src/Exit.ts` | i stops and cancellation have narrower, binding semantics. Ordinary catch cannot observe all of them. | Useful diagnostic data without promising recoverable panic or silently changing cancelled join. |
| Detached fibers and interruption masks as a general user model — `effect/packages/effect/src/Effect.ts` | Structured-only lifetime is binding. i already specifies cancellation-safe unwinding and waiting for cleanup. | Group-owned tasks, scoped background loops and precise acquisition handoffs. |
| Mutable Ref objects, synchronized mutable wrappers and mutable HashMap builders — `effect/packages/effect/src/Ref.ts`; `effect/packages/effect/src/SynchronizedRef.ts`; `effect/packages/effect/src/HashMap.ts` | All i values are immutable. Scheduler state already has one authority. | Immutable handles, pure atomic updates, or serialized owner tasks. |
| `Match` builders — `effect/packages/effect/src/Match.ts` | i has branches, patterns, guards, narrowing and coverage checks. | Ordinary matching and small reusable predicates. |
| `Data.Class`, `TaggedClass`, tagged errors — `effect/packages/effect/src/Data.ts` | Labelled constructors, unions and immutable data are native. | Normal domain types and error values. |
| Custom Equal/Hash protocols — `effect/packages/effect/src/Equal.ts`; `effect/packages/effect/src/Hash.ts` | i defines one equality and a compatible core hash. | Fix representation laws; use explicit projection/comparison functions for domain ordering. |
| Separate Either and BigInt concepts — `effect/packages/effect/src/Result.ts`; `effect/packages/effect/src/BigInt.ts` | Effect 4 already uses Result here; i already has Result and arbitrary-size Int. | `std/result`, `std/int`, and a distinct decimal type only for decimal semantics. |
| The full Array/Chunk/Record naming split — `effect/packages/effect/src/Array.ts`; `effect/packages/effect/src/Chunk.ts`; `effect/packages/effect/src/Record.ts` | i has List, Sequence, Dict and closed language data. | Add missing operations to the correct existing module; prove a new representation's purpose first. |
| Worker as the way to request parallel execution — `effect/packages/effect/src/workers/Worker.ts` | i tasks already run across host-selected workers, with defined portability. | Keep worker placement hidden; use an explicit transport service only for external interop. |
| Schema classes and TypeScript type-level machinery — `effect/packages/effect/src/Schema.ts`; `effect/packages/effect/src/SchemaAST.ts` | i needs schema functionality, but ordinary immutable descriptions and macros can express it. No library may be compiler-special-cased. | One schema description interpreted by codecs, validators, exporters and generators. |
| A second streaming execution engine and done-as-error convention — `effect/packages/effect/src/Channel.ts`; `effect/packages/effect/src/Pull.ts` | i already has computations, continuation control, events and ordinary sum values. | Shared scoped pull/consumer functions with explicit end data; add internal machinery only where necessary. |

The authoritative reference is the set of problems solved by these source
files. The stable i design should use the language's own mechanisms once,
then build a rich library on them.
