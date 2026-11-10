# Design: multicore constraints

Status: draft for review (study S7 of the redesign plan). It is reviewed at gate G2. It fixes what the first `i/task` (track R) and the language must guarantee now, so that the multicore wave (MC-U, MC-D, G5, MC-I) can add more workers later without changing what any program does. It does not design the multicore runtime; it lists the invariants that runtime must keep. Code facts are from commit c69515c. Line numbers are at that commit.

The examples use the new syntax of `docs/design/syntax.md`. The spec text in part 14 uses the old syntax, like the rest of `docs/spec.md` until A2.

## 1. The problem

i compiles to WasmGC. Today a WasmGC object cannot be shared between threads: the shared-everything threads proposal is not done, and wasmtime 50 does not have shared structs. So the route to more than one core is this:

- One module, **N instances**. Each instance is a **worker**. All workers import one shared linear memory.
- Each worker has its own task deque. Only **unstarted** tasks are stolen, and a stolen task is **copied**: encoded into the shared memory by its home worker and decoded by the thief.
- `scope.share` puts large immutable data in the shared memory once. It is freed when its scope ends.
- Later, a shared GC heap takes the place of most copies.

The plan sets one rule for all of this: **copy versus share cannot be seen.** Which worker runs a task, and whether a value was copied or shared, must never change what a program computes.

The first `i/task` is an M:1 scheduler written in i on `i/control`, with one worker. It comes long before the multicore wave. If it allowed something that N workers cannot do, programs written against it would change meaning when workers arrive. So the one-worker scheduler must already follow every rule that N workers need. That is the job of this note: to say which rules those are, at the least cost now.

The hard parts:

1. **Function identity.** §11.4 says a function is the same only as itself. After a copy, the copy is a different GC object, and `ref.eq` says "different".
2. **Values that cannot move.** A boundary and a continuation are tied to one Wasm stack and to the function references of one instance. A host reference is rooted in one wasmtime `Store` or one JS realm. None of them can be copied.
3. **No static rule.** A function type does not show its captures, width subtyping hides record fields, and a type variable hides everything. A `Send`-like constraint would be a qualified type and would lose principal inference.
4. **The environment.** §9.2 closures do not capture the environment, so a task started by `go` would get the services of the scheduler loop, not those of the place of `go`. And if the task did hold the scheduler's own boundary, no task could ever move.
5. **Order and timing.** Error choice, results, random numbers and handle ids must not depend on which worker ran first.
6. **Sharing in DAGs.** WasmGC has no identity hash, so a naive copy turns a DAG of depth d into a tree of 2^d nodes. One worker would never hit that.
7. **Hosts.** wasmtime needs one `Store` per thread and a shared memory. Browsers need `SharedArrayBuffer`, which needs cross-origin isolation (COOP and COEP), and the main thread must never block.

## 2. The decisions

**D1. One semantic rule, tested from day one.** A program gives the same values, the same errors and the same stop messages on one worker as on N. Only time, and the order of things the program does not order, may change. Running out of memory or stack is outside this rule, as it is today (§11.7, last row). *Reason:* this is the plan's rule, and it is only cheap if the one-worker scheduler already behaves as N workers will.

**D2. Portable values, checked at run time, on every worker count.** A value is **portable** when no part of it is a boundary, a continuation, or a host reference (part 3.1). Every path on which a value leaves its task and may reach another worker checks this, also on one worker, and stops with `panic: task: value is not portable` when the check fails. *Reason:* a static rule cannot work (hard part 3). A check that only ran on N workers would make placement visible. A check that runs always gives the same stop everywhere.

**D3. `core.portable` is a public built-in, and captures are defined exactly.** `portable(value) -> Bool` is in `i/core`. The spec defines the parts of a function as the values of its **captures**: the local names that its body uses and that are bound outside it, after macro expansion (part 3.1). Closure conversion (`captures_of`, `lower.rs:389`) already computes exactly this set, and every optimization level must keep it. *Reason:* `i/task` is ordinary i code, so the check must be something any i code can call; there is no library-private built-in. It is general: besides tasks, it serves (a) persistence and serialization, which must refuse values that cannot be written out, before writing half of them; (b) caches and memo tables that outlive one request or one computation, which must not keep a continuation or a host reference alive past its owner; (c) sandbox and plugin boundaries, which must not let a continuation or boundary escape back into the caller. Defining captures by the source makes the answer independent of the optimizer.

**D4. `control.keep_env`: a general way to keep the services of a place.** A new function in `i/control`:

```
control.keep_env: (f: (x: S) -> A ? E ! N) -> ((x: S) -> A ? E) ! N
```

It gives a function that, when called anywhere, runs `f` with the services of `N` taken from the place of the `keep_env` call. `go` uses it to give the child the parent's services, and leaves the scheduler out by taking it as the parameter `x` (part 4). *Reason:* §9.2 closures get their services from the place of the call. A task is called from the scheduler loop, so without this it would lose the parent's environment statements (a fake `Clock` in a test, for example). The function is general: tasks; callbacks stored now and run later from a dispatch loop (an event emitter, a host event source); lazy values and stream stages kept in data and forced elsewhere. Its type has one environment variable, so inference stays principal.

**D5. The scheduler reaches a task only as an argument from the worker loop.** Each worker runs its own scheduler loop under its own root boundary. The loop gives each task the worker's `Scheduler` service when the task starts, as the argument of the kept function. No task closure, channel handle, scope handle or event ever holds a `Scheduler` that the task did not get this way. Every task runs directly from its worker's loop, never on another task's stack. *Reason:* if task closures held the scheduler's boundary, every task would be non-portable, nothing could be stolen, and every send of a closure would stop. Running every task from the loop makes "a boundary of another task is never active" true on any number of workers.

**D6. Stable ids for functions, in field 2 of `$clo{n}`.** In the multicore wave, every function value gets a 64-bit id in the existing `i64` field 2, and the arity moves to the unused `i32` field 1. `==` on two functions that are not `ref.eq` compares ids. **`hash` stays tag-only** for functions, boundaries and continuations, as `values.rs:287` does today. Nothing gets an order on functions. Nothing changes in code now; the plan is pinned by a test. *Reason:* copies must keep sameness (§11.4). Field 2 is a native `i64`, so there is no bit trick, and only one reader (`inspect.rs:262`) changes. Ids depend on the worker, so a hash or an order over them would make `Dict` order depend on placement.

**D7. Every worker works out the top-level values itself.** `ilang_start` runs on every worker. Start-up is the same code on every worker and uses a fixed id stream, so every worker gets equal top-level values with equal ids. Top-level names are not captures, so top-level values never need to move. *Reason:* this needs no copying at start and no fallback for non-portable top-level values. The one condition is that `@external` functions called during start-up give the same results on every worker (part 7).

**D8. Steal is cooperative: the victim encodes.** A worker cannot read another worker's GC objects. So a thief only asks, through a request cell in shared memory, and the victim encodes its oldest unstarted task at its next poll point. If the encode meets a value that is not portable, that task stays home and the victim tries the next one or answers "none". Started tasks never move. *Reason:* this is the only way that works with per-instance GC heaps, and it costs one atomic load per poll point when nobody steals.

**D9. Copies keep sharing exactly.** The multicore wave's copy must write a node that was already written as a back-reference (with marks or a memo; MC chooses). A DAG is never expanded. Cost: O(distinct nodes) on each side, O(1) for a same-worker hand-off. *Reason:* a memory stop that only happens on N workers would make placement visible in practice, even if the resource-stop exception allows it in theory.

**D10. Order comes from the task tree, never from time.** Each task has a **path**: its parent's path plus the index of its `go` in the parent. It is an ordinary `i/task` value (`task.path()`). Random splitting, handle ids, logs and traces use it. A scope's error is the failure with the **lowest go index** among the children that failed. `both` and `map_concurrently` give results in argument order. *Reason:* these make results depend only on which children failed, not on which failed first, so one worker and N workers agree whenever the same children fail.

**D11. A stop anywhere stops the whole run, and the first stop message wins.** *Reason:* §9.5 already says no final functions run at a stop. A stop cell in shared memory, written once by compare-and-swap, keeps the message to one, as on one worker.

**D12. The worker count is the host's choice and cannot be seen.** A browser without cross-origin isolation, a host that cannot promise equal start-up externals, and any host that just wants one worker, all run one worker. No `i/task` API shows worker indices, the worker count, or steals. *Reason:* D1.

## 3. The rules

### 3.1 Definitions (spec §11.9 and §9.2)

- A **worker** runs tasks. A program run has one or more workers. In the multicore wave, each worker is one instance of the module, all sharing one linear memory.
- The **captures** of a function value made by a function expression are the local names that its body uses and that are bound outside the function expression, after macro expansion. A name counts when it is written in the body, also in a part that never runs. Top-level names are not captures. A constructor, a built-in, a library function and an `@external` have no captures. A function made by `control.keep_env` has the captures of its argument and the services it keeps.
- The **parts** of a value: the fields of a tuple, list, custom value or record; the values of the captures of a function. Ints, Floats, Strings, codepoints and bit arrays have no parts.
- A value is **portable** when it is not a boundary, a continuation or a host reference, and all its parts are portable.

Test of exactness: in

```i
f = (b) {
  unused = b
  () { 1 }
}
```

the inner function does not use `unused` or `b`, so it has no captures, and `portable(f(b))` is `True` even when `b` is a boundary. `captures_of` gives the same answer at every optimization level (part 15, T4).

### 3.2 Invariants handed to the multicore wave

- **H (home).** A value that is not portable is only ever reachable from the worker that made it.
- **I1 (meaning).** `==`, `hash` and the text form of a value depend only on its content and on function ids. They never depend on addresses, instance globals, or which worker holds the value.
- **I2 (fast paths).** `ref.eq` is only a fast path (for `True`, `False`, `Nil`, `[]`, labels, constant functions, identity values). Every answer it gives, the slow path gives too.
- **I3 (sharing).** A copy keeps sharing exactly (D9).
- **I4 (owner writes).** Only the worker that owns a GC object reads it to encode it. The receiver decodes into its own heap. No worker touches another worker's GC objects.

### 3.3 Paths between tasks

Values pass between tasks only through these paths. There is no other shared state.

| Path | Checked with `portable` | Why |
|---|---|---|
| Captures and kept services of a `go` body | no | Checked at steal time instead (P1) |
| A send on a channel made by `channel.new`, or any event that sends on one | yes, at the send | The receiver may be on another worker |
| The result and the error a child gives to its scope | yes, when the child ends | The scope's task may be on another worker |
| The argument of `scope.share` | yes | It goes to shared memory |
| A send on a local channel, a child of a local scope | no | They never leave the home worker (P4) |

**P1. Steal (multicore wave).** A thief may take only an unstarted task. The victim encodes it (D8). The encode walks the whole task: its closure, its captures and its kept services. If it meets tag 10, 11 or 12, the encode stops, the task stays home, and nothing can be seen. So `go` itself does no check.

**P2. Started tasks never move.** This also pins a task that waits on an async host call: the call resumes on the worker that started it.

**P3. Portable paths check always.** The check is ordinary i code in `i/task` that calls `core.portable` and then `panic("task: value is not portable")`. It runs on one worker too. Its stop message is the same on any number of workers, and it is part of the spec text of `i/task` (track R).

**P4. Local channels and local scopes.** For values that must stay home, such as a host reference to a socket given to a handler task, `i/task` gives local channels and local scopes. A local handle holds the root boundary of its home worker's scheduler, which it gets from the `Scheduler` service when it is made. So the handle is itself not portable, by the general rule, with no flag and no special case. A boundary is a value like any other: it may be stored in data that outlives the body of `control.boundary` (§9.4 only limits `suspend` to active boundaries), and the root boundary of a worker loop stays alive until the worker stops. Their paths need no check.

What a programmer sees: a closure that captures a local channel, given to `go`, simply stays on its worker. Nothing else changes. Sending a local handle on a portable channel stops with `panic: task: value is not portable`, on one worker as on N.

**Why nothing leaks (proof of H).** A value can reach a task in only two ways:

1. In the task's closure or kept services at `go`. The task is unstarted until it runs. If it is stolen, P1 walks all of it, so a task that reaches a non-portable value is never stolen.
2. By a receive, a join, or a `share.get` while the task runs. The task is then started, and by P2 it never moves. If it gives such a value to a new child with `go`, the child's closure holds it, and case 1 applies to the child.

A non-portable value can leave a task only by a portable path (stopped by P3) or a local path (which stays on the worker). So a stolen task holds no local handle of its old worker. Every local handle it gets later is made on, or received from, a local path of its new worker, and its later local sends stay there. Its result goes to its scope by a portable path, which P3 checks. H holds.

### 3.4 Function identity

- §11.4 is kept as it is. One sentence is added: a copy between workers is the same value.
- **Now:** nothing changes in the code. One worker never copies, so `ref.eq` is exact.
- **Multicore wave** (the layout is fixed now and pinned by a test):
  - `$clo{n}` today is `(tag 7) (i32 0) (i64 arity) (f64 0) (bytes null) (captures) (limbs null) (call null) (funcref)` (`emit/mod.rs:228`).
  - It becomes `(tag 7) (i32 arity) (i64 id) (f64 0) …`. `inspect.rs:262`, the tag table in `runtime/mod.rs:46`, `closure()` and its test change in the same commit. At c69515c no other code reads field 2 of a tag-7 value. The other field-2 readers (`values.rs:185`, `:197`, `:351`, `:361`, `bitarrays.rs:23`, `emit/mod.rs:22`) are for tags 5 and 9 and sit behind a tag test; MC-I greps again.
  - Field 3 stays free. The readers of field 3 (`values.rs:52`, `:178`, `:337`, `emit/mod.rs:34`) all sit behind a tag-2 (Float) test today. A unit test pins that, so a later use of field 3 by another tag cannot change `==` or `hash`.
  - **Ids.** Constant function values (top-level functions `$c{f}`, constructors, built-ins, library functions, each `@external`) get static ids 1 to K from the compiler's numbering, the same in every instance. Start-up takes ids from K+1 upward, the same on every worker because start-up is the same (D7). At run time worker w takes `((w + 1) << 40) | n` from a plain per-worker counter: no atomics. If a counter passes 2^40, the program stops as when there is no more memory.
  - `$value_compare_i32` keeps `ref.eq` first. For two tag-7 values that are not `ref.eq`, it compares the ids for equality only. Its result for different identity values stays "not equal", with no order (§11.5 exposes none, and no `i/task` API may need one).
  - `hash` of tags 7, 11 and 12 stays the tag alone. A hash by id would make `Dict` iteration order depend on placement and timing.
  - Boundaries (tag 11) and continuations (tag 12) get no id. They never leave their worker (H), so `ref.eq` stays exact.
  - The copy writes a closure as (code index, id, arity, captures). The code index is an index into one function table (an elem segment of every `$f{i}`); a funcref cannot be written to linear memory. A constant closure (static id) decodes to the receiver's own `$c{f}`, so the `ref.eq` fast path still hits.
- Trade-off recorded: putting the id in field 3 as `f64.reinterpret_i64` bits was rejected. It needs a bit trick, the bits can be NaN patterns, and it would tie field 3 to two meanings. Adding a field (a `$clo_base` with an extra `i64`) was rejected because it costs 8 bytes per closure.

### 3.5 Top-level values and start-up

- §2.5 says top-level values need no environment, so they never call host services. The only ways a top-level value can be non-portable are: an `@external` that gives a host reference (an opaque custom type, §10.3), and a boundary or continuation made by `control.boundary` during start-up.
- Every worker runs `ilang_start` (D7). Non-portable top-level values are then simply per worker. They never need to move, because top-level names are not captures. If code sends one on a portable path, P3 stops it, on one worker as on N.
- The one condition: every `@external` called while top-level values are worked out must give the same result on every worker. The compiler knows statically whether the full needs of the top-level bindings (§2.5) reach any `@external`, and records it in the metadata (§B.6, multicore wave). A host that cannot promise equal results for such a module runs one worker. This is a choice made when the module is loaded, and it cannot be seen. `docs/performance.md` notes the lost parallelism.
- Per-instance runtime state is set up on every worker as today: label globals, `$constructors_install`, the shadow stack and the site table.

## 4. Services, the worker loop, and `go`

`keep_env` (D4) is lowered from the environment array that every call already passes (`lower.rs:17`, the `$env` array of `emit/mod.rs:806`). At the call, the lowering knows the named services of the place and of `f`. It makes a closure that holds a copy of the array in which the slots of the place that `f` does not need are cleared. If `f`'s environment has a type variable, that variable is also in the place's environment, and its slots are kept; only the named services that `f` does not need are cleared. So a kept function holds exactly the services of `N`, as the spec says, and the portable walk and the copy see exactly those. (The kept services are stored as ordinary values, for example a `$values` array of the kept slots and their service ids, so that the generic walk covers them.)

`go` then looks like this (a sketch for track R, not its final API):

```i
go(A, E, N) = (body: () -> A ? E ! N | Scheduler, s ! Scheduler) -> Task(A, E) ! N | Scheduler {
  start = control.keep_env((own: Scheduler) {
    ! Scheduler = own
    body()
  })
  s.spawn(start)
}
```

- The environment of the inner function is `N | Scheduler` minus `Scheduler`, which its environment statement gives. So `keep_env` keeps `N` and clears the parent's `Scheduler`. The parent's scheduler boundary is never captured.
- The worker loop calls `start(own)` with the `Scheduler` of the worker where the task starts. The environment statement is inside the task, so it stays with the task's paused part (§9.4 rule 3).
- Kept services may still be non-portable, for example a generator's `Yield` service that holds a boundary. Then the task is pinned by P1, which is right: suspending to that boundary from the task stops with `control: boundary is not active` on any number of workers, because the task runs from the loop (D5).
- Host services given at an entry point are records of constant functions (static ids), so they are portable, and a stolen task uses the thief's own imports. §9.7 gets the rule that a host service behaves the same whichever worker calls it.

All shared mutable scheduler state (deques, channel state, wake-ups, steal requests, cancel requests, the stop cell) is behind the one `Scheduler` service. In the M:1 version it is written in i on `i/control`. The multicore wave replaces its implementation without changing the `i/task` API.

## 5. Random, paths and handle ids

- `task.path()` gives the task's path: a list of `go` indices from the root task. The path is portable data. It does not depend on time, worker or steals.
- The Random state is a key plus a counter (SplitMix64 or a counter-based PRF). A task's stream is a function of the root seed, its path, and the number of draws in that task. So a child's numbers do not depend on when or where it runs, and a `go` does not change the parent's stream. `go` itself does not know about Random; `i/random` reads `task.path()`.
- Per-task state that `i/random` needs (the draw counter) is kept by a general per-task storage that `i/task` offers (open question 3).
- The root seed comes from a host entropy service at the entry. Tests can fix it.
- Channel, scope and task handles are plain data with an id made from the path of the task that made them and a counter in that task. So `==` on handles is structural, survives copies, and does not depend on the worker.

## 6. Scopes, cancellation and errors

- A scope returns only after all its children have ended, including children that are being unwound.
- **Errors.** When a child fails, the scope cancels the other children. When all have ended, the scope gives the error of the failed child with the lowest `go` index. The scope's own body counts as index 0, before every child. Later errors are dropped. Error types stay unions; there is no aggregate error type, so inference does not change. Every error is portable (P3). Which children fail before the cancel reaches them may depend on timing, as in any concurrent program, and the `i/task` docs say so.
- **Results.** `both` and `map_concurrently` give results in argument order. `race` gives the first result that commits; that is the one place where timing chooses, as with a CML `choose`. With one worker the FIFO scheduler makes it fixed.
- **Cancellation** is cooperative and goes to the task's home worker. An unstarted task is dropped; it has no final functions. A started task is cancelled at its next suspend point with `control.cancel` on its own worker, so its final functions run there with the environment of §9.4 rule 4.
- **Stops.** A stop on any worker stops the run (D11). The worker writes the stop cell once with compare-and-swap; the others stop at their next poll point or epoch check, and their messages are dropped. No final functions run (§9.5).

## 7. Channels and CML events

- A channel is a plain-data handle (part 5). `==` means "the same channel".
- Events are values. `choose` and `map` are pure. `sync` is the only commit point, and exactly one event of a `choose` commits. `map` functions run in the syncing task after the commit, never in the partner task. Nack and guard are not in v1; they are kept for later.
- **M:1 now.** Channel state lives in the scheduler state. A commit is trivial on one thread.
- **Multicore wave.** Channel state, sync records and message buffers live in shared memory. A commit uses the parallel CML protocol of Reppy, Russo and Xiao (ICFP 2009): each sync record has a state word (WAITING, CLAIMED, SYNCED) set by compare-and-swap, and claims are taken in one global order to avoid deadlock. A message is encoded by the sender's worker (I4) and decoded by the receiver. A hand-off on the same worker passes the reference.

## 8. `scope.share`

- `scope.share(value) -> Shared(A)` belongs to the scope that makes it. `value` must be portable (P3).
- A `Shared(A)` handle is plain data (scope id, index), so sending it costs O(1).
- `shared.get(s)` gives the value. On N workers, the value is encoded once into a region of the scope in shared memory, and each worker decodes it once and caches it for the life of the scope. Values from two workers are `==` (content plus ids, I1).
- A `get` after the scope has ended stops with `panic: task: shared value is gone`, in every build, also M:1, where the scheduler keeps the set of live shares. So the stop does not depend on placement. The region is freed when the scope ends.
- Considered and rejected: turning a `Shared` into a full copy when it leaves its scope. Finding handles inside arbitrary values at scope exit needs a walk, and width subtyping and type variables hide them from the types.

## 9. The copy format (multicore wave)

The format is MC's to design. It must:

- be a tag-directed walk with an explicit stack (no Wasm recursion, so lists of 10^6 elements are fine), covering every `$obj` tag, `$cons`, and records (tag 14);
- keep sharing exactly (I3) with back-references;
- write Ints as i31 or limbs, Floats as bits, Strings and bit arrays as bytes, custom values with their constructor and type ids (fixed per module), closures as in part 3.4;
- write `True`, `False`, `Nil` and `[]` as markers that decode to the receiver's globals, and record labels as label indexes or bytes that the receiver interns again into its `$label_N`;
- stop the encode at tags 10, 11 and 12 (P1). Because of P3, a portable path never gets that far.

## 10. Hosts

Facts at c69515c:

- `ilang-host` makes and joins a new 48 MiB thread for every call (`invoke_in`, `lib.rs:1162`).
- Both hosts insist on exactly one pending async import per yield (`lib.rs:1050`, `ilang.mjs:733`). The C/R request table lifts this per worker; that is needed for `i/task` to overlap I/O even with one worker.
- The module exports an unshared memory (`runtime/mod.rs:334`).
- wasmtime is built without the `threads` feature (`crates/ilang-host/Cargo.toml:15`), and the config does not turn on `wasm_threads` (`lib.rs:629`).
- Each `Store` reserves its own GC heap, 256 MiB by default (`lib.rs:1131`).
- `HostReference` is `Arc<dyn Any + Send + Sync>` (`lib.rs:349`); `reference_id` numbers are per instance (`State.next_reference`, `lib.rs:602`).
- The JS bridge copies with `.slice()` before decoding (`ilang.mjs:378`), which a `SharedArrayBuffer` view needs.

**wasmtime (multicore wave).** Turn on the `threads` feature and `Config::wasm_threads(true)`. The module imports `(memory … shared)` instead of defining one. One Engine and Module serve all workers. Each worker is one long-lived OS thread that owns one `Store` and one instance; this replaces the per-call thread. The GC heap size is per worker. `ilang_alloc` and `ilang_free` take chunks from a shared bump word with an atomic add; a buffer freed by another worker goes on its owner's remote-free list. `$heap`, `$free_head`, `$shadow`, `$sp` and `$root_env` stay per-instance globals. The bridge uses `SharedMemory`. Idle workers park with `memory.atomic.wait32`. When the stop cell is set, the host makes the other Stores stop promptly (for example with epoch interruption, only in threaded runs).

**Browsers (multicore wave).** N workers only when `crossOriginIsolated` is true and `SharedArrayBuffer` exists; otherwise one worker, with the same meaning (D12). The `WebAssembly.Module` and the shared memory are posted to Web Workers; each Worker has its own imports object. Worker 0 runs on the thread that called the entry, which in a page is the main thread. **Worker 0 never blocks:** it never uses `Atomics.wait`, and its scheduler idles only through an async host function, so it returns to the event loop (status 1). A host service whose state lives on the main thread (the DOM, for example) is proxied to worker 0, or the run uses one worker. Host references never cross (H), so no proxy is needed for them.

**Host rule (spec §9.7, §B).** A host service behaves the same whichever worker calls it. The number of workers is the host's choice.

## 11. Records and macros

- **Records** (`records.md`, tag 14). `ref.eq` on labels is only a fast path (I2). Width subtyping hides fields, which is one more reason the check is dynamic: `portable` and the copy look at tags, never at types. Records that hold functions (codecs) keep `==` across copies through ids. `i/task` needs no label-generic callbacks, and none of its functions has label parameters (records L5). Any later shape id or inline cache must stay out of the copy format, `==` and `hash`.
- **Macros** (`meta.md` §8). Macro programs run in the compiler, single-threaded, on exactly one worker with the FIFO scheduler and no clock unless a fake one is given. So `i/task` in a macro gives the same expansion on every machine. Syntax values (identity `==`) exist only in the compiler and never reach run time, so they never cross.

## 12. Future architecture and proposals to track

1. **Now (R):** M:1 scheduler in i, with every rule of this note. `core.portable` and `control.keep_env` land.
2. **Wave 5 (MC):** N instances, one shared memory, private deques, cooperative steal of unstarted tasks with a sharing-keeping copy, `scope.share` regions, CAS-committed CML, ids in `$clo` field 2.
3. **Later:** with shared-everything threads, immutable data gets shared types (`$obj`, `$values`, `$cons`), copies go away for it, ids allow direct sharing, and boundaries and continuations stay per thread.
4. **Later:** with Go-style per-frame resume, frame records that carry their point index could be encoded, so started tasks could move.

Proposals to track:

- shared-everything threads (shared structs and arrays, thread-local globals, `thread.spawn`); wasmtime support for shared structs;
- stack switching (WasmFX), for cheaper continuations;
- JSPI, for browser host calls;
- component-model async and WASI 0.3 (wasi-threads is deprecated, so the WASI target stays single-worker);
- `Atomics.waitAsync`, for worker 0 on the main thread;
- memory-control and custom page sizes, to give `scope.share` memory back;
- COOP/COEP `credentialless`, which makes isolation easier to deploy.

A request to **track B now**: frame records should carry the point index `p` next to `(ref.func $r{p})` (or the module should keep a `$points` table), so that moving a started task stays possible later without changing `emit/control.rs` again.

## 13. Replacement spec text, section by section

No section is renumbered; new content is added as subsections or sentences. The text lands with track R, together with `core.portable` and `control.keep_env`. The parts marked *(multicore wave)* land with MC-I.

**§2.5 Top-level bindings.** After "Its value is worked out once, when the program starts, before any entry point runs (§B.1).", add:

> When the program runs on more than one worker (§11.9), each worker works out the top-level values in the same way, and they are the same values on every worker.

**§9.2 Environment and services.** Add after the bullet "A service is a normal value. …":

> - `control.keep_env(f)` (§9.4) gives a function that, wherever it is called, calls `f` with the services of `f`'s environment taken from the place of the `keep_env` call. It keeps exactly those services and no others: in `(x: s) { ! T = x  body() }`, the service `T` is given inside, so it is not kept.

**§9.4 Control.** Add a row to the table:

> | `control.keep_env` | `(f: (x: s) -> a ? e ! n) -> ((x: s) -> a ? e) ! n` |

Add rule 7:

> 7. A boundary and a continuation stay on the worker that made them (§11.9). They are not portable.

**§9.7 Host services.** Add a bullet:

> - A function of a host service does the same thing whichever worker calls it (§11.9). An async function goes on in the task that called it, on the same worker.

**§10.1 External functions.** Add:

> While top-level values are worked out, an external function may be called once for each worker (§11.9), and it must give the same result each time.

**§11.4 Equality.** After the bullet on functions, boundaries, continuations and host references, add:

> - A value stays the same value when the program copies it from one worker to another (§11.9). So a function given to another task is the same as the function that was given.

**§11.7 Stops.** After the paragraph on the stop message, add:

> When the program runs on more than one worker and one of them stops, the whole program stops. The host gets one stop message, that of the first stop.

**§11.9 Workers (new).**

> A program may run on one or more **workers** at the same time. Which worker does which part of the work cannot be seen: a program gives the same values, the same errors and the same stops on one worker as on many. Only time, and the order of things that the program does not order, may change. Running out of memory or stack (§11.7) is not covered by this rule. The host chooses the number of workers.
>
> The **captures** of a function made by a function expression are the local names that its body uses and that are bound outside it, after macro expansion; a name counts when it is written in the body, also in a part that never runs. Top-level names are not captures. A function made by `control.keep_env` (§9.4) has the captures of its argument and the services it keeps. Other functions have no captures.
>
> The **parts** of a tuple, a list, a value of a custom type or a record are its fields. The parts of a function are the values of its captures. A value is **portable** when it is not a boundary, a continuation or a host reference, and all its parts are portable.
>
> A value that is not portable never goes to another worker. Values may be copied between workers; a copy is the same value (§11.4).

**§13 The module `i/core`.** Add the row:

> | `portable` | `(a) -> Bool` | `True` when the value is portable (§11.9) |

The `hash` row: "Which number is not fixed." becomes:

> Which number is not fixed, but in one run it is the same on every worker. A function, a boundary and a continuation give a number from their kind alone.

**§B.1 The module.** Add *(now)*:

> This version runs one instance for each run of the program. Workers (§11.9) come with a later version of this interface.

*(multicore wave)* The module imports its memory as a shared memory and exports `ilang_worker(index: i32)`; `ilang_start` is called on every worker.

**§B.4 Imports.** Add to `reference_id`:

> A host reference belongs to the instance that got it. It never goes to another instance.

**§B.6 Metadata** *(multicore wave)*: whether start-up calls an `@external` (part 3.5).

**§9.x `i/task`** belongs to track R. It states the API rules of part 17 where programs can see them, including the stop messages `task: value is not portable` and `task: shared value is gone`.

## 14. Files to change

**Track R (now):**

- `docs/spec.md`: §2.5, §9.2, §9.4, §9.7, §10.1, §11.4, §11.7, new §11.9, §13, §B.1, §B.4 (part 13).
- `lib/i/core.i`: export `portable` as an `@external` of `ilang:prelude`.
- `lib/i/control.i`: `keep_env`.
- `crates/ilang-wasm/src/lower.rs`: the built-in name tables near `:120` and `:130`; `portable` and `keep_env` ops; the slot clearing of part 4. `captures_of` (`:389`) is the definition of captures; it does not change, and a comment points at §11.9.
- The checker's built-in types for `portable` and `keep_env` (`crates/ilang-check`).
- `crates/ilang-wasm/src/runtime/values.rs`: `$core_portable`, a walk with an explicit stack like `$value_compare_i32`. It gives False at tags 10, 11 and 12 and walks field 5 of `$obj` (tuples, custom values, closure captures, records), `$cons` heads and tails, and kept services.
- `crates/ilang-wasm/src/runtime/mod.rs`: the tag table (`:46`) gains a "portable" note and the planned `$clo` layout of part 3.4.
- `crates/ilang-check/tests/library.rs`: the core and control export lists.
- `lib/i/task.i`, `lib/i/random.i`: following part 17.
- `docs/performance.md`: the cost of `portable` checks, and the start-up condition of part 3.5.

**Track B (now, small):** frame records carry the point index (part 12).

**Multicore wave (not now):**

- `crates/ilang-wasm/src/emit/mod.rs`: `closure()` with arity in field 1 and id in field 2; static ids; the `$funcs` table; the id counter and worker globals.
- `crates/ilang-wasm/src/runtime/values/inspect.rs:262`: arity from field 1.
- `crates/ilang-wasm/src/runtime/values.rs`: id compare for tag 7.
- New runtime modules for the copy, request cells, the stop cell and share regions; the shared-memory allocator in `runtime/mod.rs`.
- `crates/ilang-host`: the `threads` feature, `SharedMemory`, long-lived worker threads, per-worker GC heap, the stop path.
- `bindings/javascript/ilang.mjs`: the isolation check, Workers, worker 0 rules, proxies.
- `tests/browser`: COOP/COEP headers and a fallback test.

## 15. Tests

**Now (track R):**

- **T1** `tests/spec/13/portable.i`. True for: an Int, a big Int, a Float, a String, a bit array, `[]`, a tuple and a list of these, a top-level function, a constructor, a closure that captures only data, a closure that captures another such closure. False for: a boundary `b` (inside `control.boundary`), a continuation (inside `suspend`), a closure that captures `b`, a list holding such a closure, a custom value with a boundary field. After records land: `{ k: b }` is False.
- **T2** `tests/spec/9.7`, with a host fixture as in `tests/spec/11/equality_externals.i`: `portable` of a host reference is False, and of a tuple that holds one too.
- **T3** `tests/spec/11/equality.i` stays unchanged; it guards that §11.4 did not change.
- **T4** Captures are exact: the example of part 3.1 is True; a closure whose only use of `b` is in a clause that never runs is False. Both run at O0 and O2 and give the same answers.
- **T5** `keep_env`: a kept function called under a different environment statement still sees the kept service; a service given inside its argument is not kept (`portable` of a kept function made under a non-portable service that the argument gives itself is True).
- **T6** A runtime unit test runs `$core_portable` on every tag, and on a list of 10^6 elements (no Wasm stack overflow).
- **T7** An emit test pins today's `$clo` layout (field 1 is 0, field 2 is the arity, field 3 is 0) and names the planned layout in its message. A unit test pins that every reader of `$obj` field 3 is behind a tag-2 test.
- **T8** Track R: a send of a host reference on `channel.new` stops with `panic: task: value is not portable` on one worker; the same send on a local channel works; a child giving back a host reference stops in a portable scope and works in a local scope; a `get` after the scope ended stops with `panic: task: shared value is gone`.
- **T9** Track R: with a fixed seed, per-task random streams are the same under two scheduling orders (a test scheduler service picks the order).
- **T10** Track R: a scope returns only after every child ended; final functions of cancelled children run once; with failures at go indices 3 and 1, the error is that of index 1 under both scheduling orders; `both` and `map_concurrently` give argument order.
- **T11** Track R: a task started inside a generator body keeps the generator's `Yield` service; when it yields, it stops with `control: boundary is not active`, and it is never stolen.
- **T12** Track R: `i/task` in a macro program gives the same expansion twice.

**Multicore wave:**

- **T13** Differential: every `tests/spec` case and every std test runs single-core, threaded with 1 worker, and threaded with 4 workers (the last 200 times). stdout, results and stop messages must be the same.
- **T14** Copy round-trip for each kind: `==`, `hash` and text form equal; constant closures, `True`, `False`, `[]` and labels come back `ref.eq` to the receiver's globals.
- **T15** Sharing: `x0 = Leaf`, `x(i+1) = Node(x(i), x(i))` to depth 40, and the same with list cells: the copy has 41 nodes.
- **T16** Closure identity across a steal, also inside records and tuples.
- **T17** A stop on worker 3 while the others loop: exactly one message, all threads join, no final functions run.
- **T18** Cross-worker select under stress, 200 runs: no message lost or duplicated, one commit per sync.
- **T19** Browser: with COOP/COEP, 4 Workers and a main thread that stays responsive; without isolation, one worker with the same output; strings through the shared memory decode right.

## 16. Risks

- **`portable` makes captures visible.** An optimizer that drops or adds a capture changes `portable(f)`. The spec definition and T4 at O0 and O2 guard it; any later closure optimization must keep the capture set.
- **The check costs a walk.** Every portable send, result and error walks the value, also on one worker. Large data should go through `scope.share` or local channels. A later "known portable" bit (for example in `$clo` field 3, or in a per-object flag) can make repeat checks O(1).
- **Stopping on a non-portable result may surprise.** `go(open_socket)` in a portable scope stops. The local scope API and a clear message are needed, or people will route around the rule.
- **`keep_env` is new language surface.** G2 must accept it as general. Its slot clearing must be exact, or tasks become non-portable for no visible reason.
- **Timing still chooses some things.** Which children fail before the cancel reaches them, and which `race` branch wins, depend on timing on N workers. Tests must not depend on them; the M:1 order is fixed but is not a promise for N workers.
- **Start-up runs N times.** It costs N times the start-up work, in parallel, and depends on the contract that start-up externals give equal results. A host whose externals are impure would make workers disagree; it must run one worker.
- **Host limits.** One pending async import per instance today blocks overlapping I/O; the request table of C/R must come first. N Stores each reserve a GC heap. The browser main-thread proxy adds latency.
- **Field 2 move.** If other work reads field 2 of closures before MC-I, the move breaks it. T7 and the tag-table note guard it.
- **Left to MC.** The CAS protocol, the allocator, poll-point placement, epoch stops and the memory model are not settled here. A long computation with no poll point delays steals and cancellation, as it blocks the worker in M:1 today.

## 17. Open questions

1. Is `core.portable` accepted as a public built-in (D3), or should G2 ask for a narrower form? This note argues for public, because `i/task` is ordinary i code.
2. `control.keep_env` takes one parameter. Is that enough, or should there be a form with no parameter as well (`() -> a` is `(x: Nil) -> a` today)?
3. Per-task storage for `i/random` (part 5): a general `task.local` facility in `i/task`, or Random as a value passed explicitly? Track R decides; the rule is only that streams depend on seed, path and draw count.
4. How does worker k > 0 enter `i/task`'s scheduler loop without a special case? One answer: the program's entry runs `task.run`, which, on N workers, is an ordinary host export or entry kind that every worker calls. MC-D decides.
5. How long may a victim wait before it answers a steal request? Poll points only, or a heartbeat in function prologues later?
6. Should a copy encode a message at offer time or only when a commit pairs it with another worker (part 7)? MC-D decides.

## 18. API rules for track R (`i/task`)

1. Values pass between tasks only through `go` captures and kept services, channel and event sends, task results and errors, and `scope.share`. There is no other shared state.
2. Every portable path (sends on `channel.new` channels, results, errors, `share`) calls `core.portable` and stops with `panic("task: value is not portable")` when it is False. This happens always, also on one worker.
3. `go` never checks portability. Whether a task can move is decided only at steal time; a failed encode leaves the task home and cannot be seen.
4. `go` keeps the parent's services with `control.keep_env`, and takes the `Scheduler` as the parameter of the kept function, so the parent's scheduler is never kept or captured.
5. Every task runs directly from its worker's scheduler loop, never on another task's stack. Nested scopes, `both`, `race` and `map_concurrently` never run a loop on the caller's stack.
6. Local channels and local scopes hold the root boundary of their home worker's scheduler, so they are not portable by the general rule. Their paths do not check. There is no flag and no special case.
7. A started task never moves. A task that waits on an async host call resumes on the worker that started the call.
8. A scope returns only after all its children have ended. On a failure it cancels the rest and gives the error of the failed child with the lowest `go` index (its own body first). Error types stay unions.
9. `both` and `map_concurrently` give results in argument order. Only `race` and `choose` let timing pick, and the docs say so.
10. Cancellation is cooperative: an unstarted task is dropped; a started task gets `control.cancel` on its home worker at its next suspend point.
11. Every task has a path (parent path plus `go` index), given by `task.path()`. Random streams, handle ids and per-task names come from the path, never from time, a global counter, or the worker.
12. Task, scope, channel, event and `Shared` handles are plain data. `==` on them is structural. No API needs an order or a hash on functions, boundaries or continuations; scheduler bookkeeping uses Int ids.
13. Events are values. `choose` and `map` are pure, `sync` commits exactly one event, and `map` functions run in the syncing task after the commit. Nack and guard are left for later.
14. `scope.share(value)` needs a portable value. A `Shared` belongs to its scope, and a `get` after the scope ends stops with `panic("task: shared value is gone")` in every build.
15. All shared mutable scheduler state is behind the one `Scheduler` service, so the multicore wave can replace its implementation without changing the API.
16. The scheduler has poll points (`go`, every `sync`, joins and scope ends, `yield`, async host calls, every switch of the loop) written as one call that does nothing on one worker.
17. On one worker the scheduler is FIFO and fixed, with no time unless a `Clock` service is given. It idles only through an async host function, never a busy loop or a blocking import.
18. Many host calls may wait at once, through the request table. No code relies on one pending import per instance.
19. A stop in any task stops the program. It is never caught or counted as a task error.
20. No API takes a label-generic callback (records L5), and no API needs a static `Send`-like type.
21. No API shows worker indices, the worker count, or steals.
