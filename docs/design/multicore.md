# Design: multicore constraints

Status: draft for review (study S7 of the redesign plan). It is reviewed at gate G2. It fixes what the first `i/task` (track R) and the language must guarantee now, so that the multicore wave (MC-U, MC-D, G5, MC-I) can add more workers later without changing what any program does. It does not design the multicore runtime; it lists the invariants that runtime must keep. Code facts are from commit c69515c. Line numbers are at that commit.

The examples use the new syntax of `docs/design/syntax.md`. The spec text in part 13 uses the new syntax too, because it lands with track R, after A2 has moved `docs/spec.md` to the new syntax.

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
2. **Values that cannot move.** A boundary and a continuation are tied to one Wasm stack and to the function references of one instance. A host reference is rooted in one wasmtime `Store` or one JS realm. A host request (host-concurrency.md §6.4) has an id in the request table of one instance. None of them can be copied.
3. **No static rule.** A function type does not show its captures, width subtyping hides record fields, and a type variable hides everything. A `Send`-like constraint would be a qualified type and would lose principal inference.
4. **The environment.** §9.2 closures do not capture the environment, so a task started by `go` would get the services of the scheduler loop, not those of the place of `go`. And if the task did hold the scheduler's own boundary, no task could ever move.
5. **Order and timing.** Error choice, results, random numbers and handle ids must not depend on which worker ran first.
6. **Sharing and cycles.** WasmGC has no identity hash, and every `$obj` field is immutable (`runtime/mod.rs:234`). So a naive copy turns a DAG of depth d into a tree of 2^d nodes, and a naive walk never ends on a local recursive function, whose closure captures itself (part 3.6).
7. **Hosts.** wasmtime needs one `Store` per thread and a shared memory. Browsers need `SharedArrayBuffer`, which needs cross-origin isolation (COOP and COEP), and the main thread must never block.

## 2. The decisions

**D1. One semantic rule, tested from day one.** Tasks that run at the same time take their **steps** (sends and receives, host calls, failures and stops) in an order that the program does not fix. That order may differ from run to run and with the number of workers. Apart from it, a program gives the same values, the same errors and the same stops on one worker as on N. So these may change with the order, and only these: which of two tasks that stop stops first (and so the stop message, D11), which branch of a `race` or a `choose` commits, which children fail before a cancel reaches them, which children are blocked when a deadlock is found, and the order of host effects (output) of tasks that run at the same time. A program that leaves none of these to the order gives exactly one result. Running out of memory or stack is outside this rule, as it is today (§11.7, last row). *Reason:* this is the plan's rule, stated so that the design can keep it. It is only cheap if the one-worker scheduler already behaves as N workers will.

**D2. Portable values, checked at run time, on every worker count.** A value is **portable** when nothing it holds is a boundary, a continuation, a host reference or a host request (part 3.1). Every path on which a value leaves its task and may reach another worker checks this, also on one worker, and stops with `panic: task: value is not portable` when the check fails. *Reason:* a static rule cannot work (hard part 3). A check that only ran on N workers would make placement visible. A check that runs always gives the same stop everywhere.

**D3. `core.portable` is a public built-in, and captures are defined exactly.** `portable(value) -> Bool` is in `i/core`. The spec defines the values a function holds as the values of its **captures**: the local names that its body uses and that are bound outside it, after macro expansion (part 3.1). Closure conversion (`captures_of`, `lower.rs:389`) already computes exactly this set, and every optimization level must keep it. *Reason:* `i/task` is ordinary i code, so the check must be something any i code can call; there is no library-private built-in. It is general: besides tasks, it serves (a) persistence and serialization, which must refuse values that cannot be written out, before writing half of them; (b) caches and memo tables that outlive one request or one computation, which must not keep a continuation or a host reference alive past its owner; (c) sandbox and plugin boundaries, which must not let a continuation or boundary escape back into the caller. Defining captures by the source, and kept services by the run (D4), makes the answer independent of the optimizer.

**D4. `control.keep_env`: a general way to keep the services of a place.** A new function in `i/control`:

```
keep_env(S, A, E, N): (f: (x: S) -> A ? E ! N) -> ((x: S) -> A ? E) ! N
```

It gives a function that, when called anywhere, runs `f` with the services that are bound at the place of the `keep_env` call. It keeps **every** service bound there when the program runs, also those that `f` does not need. `go` uses it to give the child the parent's services. Before the call, `go` binds `Scheduler` to a portable stand-in, and the task gets its real scheduler as the parameter `x` (part 4). *Reason:* §9.2 closures get their services from the place of the call. A task is called from the scheduler loop, so without this it would lose the parent's environment statements (a fake `Clock` in a test, for example). The function is general: tasks; callbacks stored now and run later from a dispatch loop (an event emitter, a host event source); lazy values and stream stages kept in data and forced elsewhere. Its type has one environment variable, so inference stays principal. Keeping every bound service, not "exactly the services of `N`", is a choice: at c69515c the environment is one `$env` array indexed by service id, generic code is not specialized, and keeping exactly `N` would need the slot set of every environment variable passed at run time (rejected, part 4). The rule by the run is simple, exact, and the same at every optimization level.

**D5. The scheduler reaches a task only as an argument from the worker loop.** Each worker runs its own scheduler loop under its own root boundary. The loop gives each task the worker's `Scheduler` service when the task starts, as the argument of the kept function. No task closure, kept service, channel handle, scope handle or event ever holds a `Scheduler` that the task did not get this way. Every task runs directly from its worker's loop, never on another task's stack. *Reason:* if task closures held the scheduler's boundary, every task would be non-portable, nothing could be stolen, and every send of a closure would stop. Running every task from the loop makes "a boundary of another task is never active" true on any number of workers.

**D6. Stable ids for functions, in field 2 of `$clo{n}`.** In the multicore wave, every function value gets a 64-bit id in the existing `i64` field 2, and the arity moves to the `i32` field 1, next to the cycle flag that track R puts there (part 3.6). `==` on two functions that are not `ref.eq` compares ids. **`hash` stays tag-only** for functions, boundaries and continuations, as `values.rs:287` does today. Nothing gets an order on functions. *Reason:* copies must keep sameness (§11.4). Field 2 is a native `i64`, so there is no bit trick, and only one reader (`inspect.rs:262`) changes. Ids depend on the worker, so a hash or an order over them would make `Dict` order depend on placement.

**D7. Every worker works out the top-level values itself, and only portable ones are allowed on N workers.** `ilang_start` runs on every worker, with a fixed id stream, so every worker gets equal top-level values with equal ids. Top-level names are not captures, so top-level values never need to move. Two conditions keep this from being seen, and when either fails the run uses one worker (a load-time choice, D12): (a) after start-up on worker 0, every top-level value must be portable; (b) a **host external** (one that is not of the library package `i`; part 3.5) called during start-up must be declared pure by the host. *Reason:* a host reference made at start-up would be a different object on each worker, and a top-level function that uses it is a constant closure that a thief would run against its own object (part 3.5). Condition (a) catches every such value with the walk that `portable` already needs.

**D8. Steal is cooperative: the victim encodes.** A worker cannot read another worker's GC objects. So a thief only asks, through a request cell in shared memory, and the victim encodes its oldest unstarted task at its next poll point. If the encode meets a value that is not portable, that task stays home and the victim tries the next one or answers "none". Started tasks never move. *Reason:* this is the only way that works with per-instance GC heaps, and it costs one atomic load per poll point when nobody steals.

**D9. Copies keep sharing and cycles exactly.** The multicore wave's copy must write a node that was already written as a back-reference, so a DAG is never expanded and a cycle ends. Finding a node that was already written needs either a mark on the object or a table looked up by `ref.eq`. WasmGC has no identity hash and every `$obj` field is immutable, so a mark needs a layout change (a mutable `i32` field in `$obj` and `$cons`). MC chooses: with the mark, the copy costs O(distinct nodes); with a `ref.eq` table, O(n^2) in the distinct nodes. *Reason:* a memory stop that only happens on N workers would make placement visible in practice, even if the resource-stop exception allows it in theory.

**D10. Order comes from the task tree, never from time.** Each task has a **path**: its parent's path plus the index of its `go` in the parent. It is an ordinary `i/task` value (`task.path()`). Random splitting, handle ids, logs and traces use it. A scope's error is the failure with the **lowest go index** among the children that failed. `both` and `map_concurrently` give results in argument order. *Reason:* these make results depend only on which children failed, not on which failed first, so one worker and N workers agree whenever the same children fail.

**D11. A stop anywhere stops the whole run, and the first stop wins.** *Reason:* §9.5 already says no final functions run at a stop. A stop cell in shared memory, written once by compare-and-swap, keeps the message to one, as on one worker. When two tasks may stop, which one is first is part of the order of D1, so the message may differ; with at most one task that may stop, it is fixed.

**D12. The worker count is the host's choice and cannot be seen.** A browser without cross-origin isolation, a module that fails a condition of D7, and any host that just wants one worker, all run one worker. No `i/task` API shows worker indices, the worker count, or steals. *Reason:* D1.

**D13. A typed key for scheduler state.** The scheduler is ordinary i code, and one non-generic `Scheduler` service must hold the buffers of channels of many element types, the values of many `share` calls, and the waiting receivers of each. Rank-1 i has no existential, no top type and no type test on a type variable, so this cannot be typed without a new primitive. The note asks G2 for one general, safe primitive in `i/core`: `key.new(A): () -> Key(A)` gives a new key on each call; `key.wrap(A): (k: Key(A), value: A) -> Keyed` and `key.unwrap(A): (k: Key(A), x: Keyed) -> Found(A) | Missing`. `unwrap` gives the value only when `x` was made with the same key. It is safe because `key.new()` is not a direct value, so `k = key.new()` is not generalized (§4.7, the value restriction). A key is the same only as itself, like a function, and gets an id in the multicore wave (D6); a `Keyed` is portable when its value is. *Reason:* the alternative, handles that carry typed closures, makes handles non-portable as soon as those closures reach scheduler state. The primitive is general: per-task storage (open question 3), typed heterogeneous maps (plugin registries, typed property bags, request context in a web framework), and typed event buses, besides the scheduler. The exact names are track R's, and the primitive needs G2's approval (open question 7).

## 3. The rules

### 3.1 Definitions (spec §11.9 and §9.2)

- A **worker** runs tasks. A program run has one or more workers. In the multicore wave, each worker is one instance of the module, all sharing one linear memory.
- The **captures** of a function value made by a function expression are the local names that its body uses and that are bound outside the function expression, after macro expansion. A name counts when it is written in the body, also in a part that never runs. Top-level names are not captures. A constructor, a built-in, a library function and an `@external` have no captures. A function made by `control.keep_env` has the captures of its argument and the services it keeps.
- The values a value **holds**: the fields of a tuple, list, custom value or record (its parts, §11.4); the values of the captures of a function; the value in a `Keyed`. Ints, Floats, Strings, codepoints and bit arrays hold nothing. The word is new because §11.4 says a function has no parts that may be looked at, and that stays true: `==` never looks at captures.
- A value is **portable** when it is not a boundary, a continuation, a host reference or a host request, and every value it holds is portable. A value may hold itself (a local recursive function, part 3.6); the rule then asks only about the other values on the way, so such a function is portable when they are.

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
- **I1 (meaning).** `==`, `hash` and the text form of a value depend only on its content and on function and key ids. They never depend on addresses, instance globals, or which worker holds the value.
- **I2 (fast paths).** `ref.eq` is only a fast path (for `True`, `False`, `Nil`, `[]`, labels, constant functions, identity values). Every answer it gives, the slow path gives too.
- **I3 (sharing).** A copy keeps sharing and cycles exactly (D9).
- **I4 (owner writes).** Only the worker that owns a GC object reads it to encode it. The receiver decodes into its own heap. No worker touches another worker's GC objects.

### 3.3 Paths between tasks

A **task**, as the scheduler keeps it, is a record of three things: its kept start function (part 4), the handle of the scope it belongs to, and its path. Values pass between tasks only through these paths. There is no other shared state.

| Path | Checked with `portable` | Why |
|---|---|---|
| Captures and kept services of a `go` body | no | Checked at steal time instead (P1) |
| A send on a channel made by `channel.new`, or any event that sends on one | yes, at the send | The receiver may be on another worker |
| The result and the error a child gives to a scope made by `scope` | yes, when the child ends | The scope's task may be on another worker |
| The argument of `scope.share` | yes | It goes to shared memory |
| A send on a local channel; the result and the error a child gives to a local scope | no | Both ends are on the home worker (P4) |

**P1. Steal (multicore wave).** A thief may take only an unstarted task. The victim encodes the whole task record (D8): the kept start function with its captures and kept services, and the scope handle. If the encode meets tag 10, 11 or 12, or a host request, the encode stops, the task stays home, and nothing can be seen. So `go` itself does no check.

**P2. Started tasks never move.** This also pins a task that waits on an async host call: the call resumes on the worker that started it.

**P3. Portable paths check always.** The check is ordinary i code in `i/task` that calls `core.portable` and then `panic("task: value is not portable")`. It runs on one worker too. Its stop message is the same on any number of workers, and it is part of the spec text of `i/task` (track R).

**P4. Local channels and local scopes.** For values that must stay home, such as a host reference to a socket given to a handler task, `i/task` gives local channels and local scopes. A local handle holds the root boundary of its home worker's scheduler, which it gets from the `Scheduler` service when it is made. So the handle is itself not portable, by the general rule, with no flag and no special case. A boundary is a value like any other: it may be stored in data that outlives the body of `control.boundary` (§9.4 only limits `suspend` to active boundaries), and the root boundary of a worker loop stays alive until the worker stops. **A child of a local scope is never stolen**: its task record holds the local scope handle, so the encode of P1 fails. This holds whatever the child's body captures; `go(() { open_socket() })` in a local scope stays home and gives its host reference back on its own worker.

What a programmer sees: a closure that captures a local channel, given to `go`, simply stays on its worker, and so does every child of a local scope. Nothing else changes. Sending a local handle on a portable channel stops with `panic: task: value is not portable`, on one worker as on N.

**P5. Host requests stay home.** A host request value (host-concurrency.md §6.4, designed by R2) holds an id in the request table of its instance (H1 there). It is not portable (part 3.1). So only tasks on its home worker can wait on it, poll it or cancel it, and a send of it on a portable path stops. On one worker the same send stops in the same way, so a program cannot see that ids are per instance.

**Why nothing leaks (proof of H).** A non-portable value is made on some worker. It can reach a task in only two ways:

1. In the task's record at `go`: its closure, its kept services or its scope handle. The task is unstarted until it runs. If it is stolen, P1 walks all of it, so a task whose record reaches a non-portable value is never stolen.
2. By a receive, a join, or a `share.get` while the task runs. The task is then started, and by P2 it never moves. If it gives such a value to a new child with `go`, the child's record holds it, and case 1 applies to the child.

A value leaves a task in only three ways, and each keeps H:

- A send on a channel made by `channel.new`, a result or error given to a scope made by `scope`, or `share`: P3 stops it unless it is portable.
- A send on a local channel: the sending task holds the local handle. By cases 1 and 2, a task that holds a non-portable handle is on that handle's home worker, so the value stays there.
- A result or error given to a local scope: the child's record holds the local scope handle, so by P4 the child was never stolen and runs on the scope's home worker.

So a stolen task holds no local handle of its old worker and belongs to no local scope of it. Every local handle it gets later is made on, or received from, a local path of its new worker. H holds.

### 3.4 Function identity

- §11.4 is kept as it is. One sentence is added: a copy between workers is the same value.
- **Now:** nothing changes in `==`. One worker never copies, so `ref.eq` is exact.
- **Multicore wave** (the layout is fixed now and pinned by a test):
  - `$clo{n}` today is `(tag 7) (i32 0) (i64 arity) (f64 0) (bytes null) (captures) (limbs null) (call null) (funcref)` (`emit/mod.rs:228`). Track R sets bit 0 of field 1 for a closure whose captures are patched (part 3.6).
  - It becomes `(tag 7) (i32 flags | arity << 1) (i64 id) (f64 0) …`. `inspect.rs:262`, the tag table in `runtime/mod.rs:46`, `closure()` and its test change in the same commit. At c69515c no other code reads field 2 of a tag-7 value. The other field-2 readers (`values.rs:185`, `:197`, `:351`, `:361`, `bitarrays.rs:23`, `inspect.rs:151`, `emit/mod.rs:22`) are for tags 5 and 9 and sit behind a tag test or are called only for those tags; MC-I greps again.
  - Field 3 stays free. The readers of field 3 are `$float_get` (`values.rs:51-52`), which has no tag test of its own and is called from `core.rs`, `abi.rs`, `bitarrays.rs`, `bigint_conversions.rs`, `float_format.rs` and `emit/mod.rs:514` only on values the checker types as `Float`, and the readers `values.rs:178`, `:337` and `emit/mod.rs:34`, which sit behind a tag-2 (Float) test. A unit test pins this list, so a later use of field 3 by another tag cannot change `==` or `hash` without the test failing.
  - **Ids.** Constant function values (top-level functions `$c{f}`, constructors, built-ins, library functions, each `@external`) get static ids 1 to K from the compiler's numbering, the same in every instance. Start-up takes ids from K+1 upward, the same on every worker because start-up is the same (D7). At run time worker w takes `((w + 1) << 40) | n` from a plain per-worker counter: no atomics. If a counter passes 2^40, the program stops as when there is no more memory. Keys (D13) take ids from the same counter.
  - `$value_compare_i32` keeps `ref.eq` first. For two tag-7 values that are not `ref.eq`, it compares the ids for equality only. Its result for different identity values stays "not equal", with no order (§11.5 exposes none, and no `i/task` API may need one).
  - `hash` of tags 7, 11 and 12 stays the tag alone. A hash by id would make `Dict` iteration order depend on placement and timing.
  - Boundaries (tag 11) and continuations (tag 12) get no id. They never leave their worker (H), so `ref.eq` stays exact.
  - The copy writes a closure as (code index, id, flags, arity, captures). The code index is an index into one function table (an elem segment of every `$f{i}`); a funcref cannot be written to linear memory. A constant closure (static id) decodes to the receiver's own `$c{f}`, so the `ref.eq` fast path still hits. A closure in a cycle is made first with its captures array and the array is filled after, as the lowering does today.
- Trade-off recorded: putting the id in field 3 as `f64.reinterpret_i64` bits was rejected. It needs a bit trick, the bits can be NaN patterns, and it would tie field 3 to two meanings. Adding a field (a `$clo_base` with an extra `i64`) was rejected because it costs 8 bytes per closure.

### 3.5 Top-level values and start-up

- §2.5 says top-level values need no environment, so they never call host services. The only ways a top-level value can be non-portable are: an `@external` that gives a host reference (an opaque custom type, §10.3), and a boundary or continuation made by `control.boundary` during start-up.
- Such a value is a problem even though it never moves. Top-level names are not captures, so `counter = ext_counter_new()` and `bump = () { ext_counter_inc(counter) }` make `bump` a portable constant closure. Each worker's start-up makes its own counter (`reference_id` is per instance, `lib.rs:602`), and a stolen `bump` would increment the thief's counter: `scope { 100 times go(bump) }` would leave less than 100 in the counter that worker 0 reads. A top-level continuation is the same: each worker would have its own copy to resume.
- So every worker runs `ilang_start` (D7), and the run uses N workers only when both conditions hold:
  - (a) **Portable top level.** After start-up on worker 0, the runtime walks every top-level value with the `portable` walk (`ilang_toplevel_portable`, multicore wave). If one is not portable, the run uses one worker. Then `counter` above gives 100, as on one worker.
  - (b) **Pure start-up externals.** A **host external** is an `@external` that is not of the library package `i`. The `@external` declarations of `i/core` (`not`, `to_float`, `inspect`, `hash`, …; `@external("ilang:core", …)` and `ilang:prelude`'s `panic`) are operations of the runtime, which `lower.rs:1379` and the name tables near `lower.rs:120` map to ops; they do not count. The compiler knows statically whether the full needs of the top-level bindings (§2.5) reach a host external, and records it in the metadata (§B.6, multicore wave). Such a module runs N workers only when the host declares those externals pure: each call with equal arguments gives an equal result, and calling them again on each worker has no effect that the program or the user can see. Otherwise it runs one worker. This is the same line that `meta.md` §8 draws for macros.
- Both are choices made when the module is loaded or started, and they cannot be seen. `docs/performance.md` notes the lost parallelism.
- Per-instance runtime state is set up on every worker as today: label globals, `$constructors_install`, the shadow stack and the site table.

### 3.6 Cycles and the cost of the walk

- Data values are immutable and built from their parts, so they never form a cycle. The only cycles at c69515c come from direct value bindings: the closure is made first, and its captures are filled later with `array.set` on its captures array (`fx.patches` and `Stmt::SetCapture`, `lower.rs:1618-1655`; `emit/mod.rs:413`). So `loop = (n) { … loop(n - 1) … }` gives a closure that holds itself.
- Track R marks such closures: `closure()` sets bit 0 of field 1 (today always 0) when the lowering patches the closure's captures. `$core_portable` keeps a short list of the marked closures it has entered, compared with `ref.eq`, and does not enter one twice. Every cycle goes through a patched slot, so the walk always ends, and the list has one entry for each local recursive binding the walk reaches.
- A DAG is still walked once for each path to a node, as `==`, `hash` and the text form already do today. So `portable` of T15's depth-40 DAG takes 2^40 steps, on one worker as on N, like `x == y` does today for two such values that were built apart. This cost is the same on every worker count, so it is not a placement problem. If MC adds the mark of D9, `portable` uses it too and becomes O(distinct nodes).
- The copy (part 9) must handle the same cycles for correctness, not only to save work.

## 4. Services, the worker loop, and `go`

`keep_env` (D4) is lowered from the environment array that every call already passes (`lower.rs:17`, the `$env` array of `emit/mod.rs:806`). That array holds every service bound at the place, indexed by a global service id, because each environment statement copies the whole array (`$env_with`, `emit/control.rs:49`). The kept function holds that array as it is at the call, with nothing cleared. So it holds exactly the services that are bound at the place when the program runs, as the spec says, and the answer of `portable` does not depend on inlining or on how generic the code around the call is. The kept services are stored as ordinary values (for example a `$values` array of the kept slots and their service ids), so that the generic walk covers them.

The kept function is a real frame with its own re-entry record, `$keep_reenter`, like `$catch_reenter` and `$ensure_reenter`. `emit/control.rs:10-13` makes the environment locals again from the environment of the place of `resume`; only environment statements are kept. The record of `$keep_reenter` holds the kept array, so a task that suspends and is resumed by the loop still sees its parent's services (a fake `Clock`, for example), not the loop's.

Rejected: keeping exactly the services of `N`. Inside generic code, such as `go` itself or any wrapper of `keep_env`, `N` is a type variable, and there is no monomorphization in `lower.rs`. The lowering could clear only the named services that `f` does not need, and would keep every slot of the caller's array under a type variable. Then `portable` would give False inside a generic wrapper and True after the wrapper is inlined at O2, against D3. Doing it exactly would need the slot set of every environment variable passed at run time, as an extra argument of every generic function whose environment variable reaches `keep_env`.

`go` then looks like this (a sketch for track R, not its final API):

```i
go(A, E, N) = (body: () -> A ? E ! N | Scheduler, s ! Scheduler) -> Task(A, E) ! N | Scheduler {
  start = {
    ! Scheduler = scheduler.detached
    control.keep_env((own: Scheduler) {
      ! Scheduler = own
      body()
    })
  }
  s.spawn(start)
}
```

- `scheduler.detached` is a top-level record of constant functions that stop with `panic: task: no scheduler`. It is portable. Binding it before `keep_env` puts it, not the parent's scheduler, in the kept array, so the parent's scheduler boundary is never kept or captured. Inside, `! Scheduler = own` binds the real one, so `detached` is never called.
- `s.spawn(start)` makes the task record (part 3.3), with the handle of the scope that `s` belongs to.
- The worker loop calls `start(own)` with the `Scheduler` of the worker where the task starts. The environment statement is inside the task, so it stays with the task's paused part (§9.4 rule 3).
- Kept services may still be non-portable, for example a generator's `Yield` service, or the `Assert` of `assertion.catch`, which hold a boundary. Every service bound at the place is kept, also one that `body` does not need, so a task started under such a service is pinned by P1. That costs parallelism only, and it cannot be seen. When `body` uses that service, suspending to that boundary from the task stops with `control: boundary is not active` on any number of workers, because the task runs from the loop (D5).
- Host services given at an entry point are records of constant functions (static ids), so they are portable, and a stolen task uses the thief's own imports. §9.7 gets the rule that a host service behaves the same whichever worker calls it.

All shared mutable scheduler state (deques, channel state, wake-ups, steal requests, cancel requests, the stop cell) is behind the one `Scheduler` service. Values of many types are kept there as `Keyed` values (D13). In the M:1 version it is written in i on `i/control`. The multicore wave replaces its implementation without changing the `i/task` API.

## 5. Random, paths and handle ids

- `task.path()` gives the task's path: a list of `go` indices from the root task. The path is portable data. It does not depend on time, worker or steals.
- The Random state is a key plus a counter (SplitMix64 or a counter-based PRF). A task's stream is a function of the root seed, its path, and the number of draws in that task. So a child's numbers do not depend on when or where it runs, and a `go` does not change the parent's stream. `go` itself does not know about Random; `i/random` reads `task.path()`.
- Per-task state that `i/random` needs (the draw counter) is kept by a general per-task storage that `i/task` offers, built on keys (D13; open question 3).
- The root seed comes from a host entropy service at the entry. Tests can fix it.
- Channel, scope and task handles are made of a path-based id (the path of the task that made them and a counter in that task) and the keys that type their state (D13). So `==` on handles is "the same channel" and survives copies, and nothing in them depends on the worker except key ids, which only `==` sees.

## 6. Scopes, cancellation and errors

- A scope returns only after all its children have ended, including children that are being unwound.
- **Errors.** When a child fails, the scope cancels the other children. When all have ended, the scope gives the error of the failed child with the lowest `go` index. The scope's own body counts as index 0, before every child. Later errors are dropped. Error types stay unions; there is no aggregate error type, so inference does not change. Every error given to a scope made by `scope` is portable (P3). Which children fail before the cancel reaches them depends on the order of D1: for `go(() { yield()  fail(A) })` and `go(() { fail(B) })`, one FIFO worker gives `B`, and N workers may give `A`. The `i/task` docs say so.
- **Results.** `both` and `map_concurrently` give results in argument order. `race` gives the first result that commits; timing chooses, as with a CML `choose`. With one worker the FIFO scheduler makes it fixed, but that is not a promise for N workers.
- **Cancellation** is cooperative and goes to the task's home worker. An unstarted task is dropped; it has no final functions. A started task is cancelled at its next suspend point with `control.cancel` on its own worker, so its final functions run there with the environment of §9.4 rule 4.
- **Stops.** A stop on any worker stops the run (D11). The worker writes the stop cell once with compare-and-swap; the others stop at their next poll point or epoch check, and their messages are dropped. No final functions run (§9.5).
- **Deadlock.** When no task can run and no host request is outstanding on any worker, nothing can ever go on. The run then stops with `panic: task: deadlock`, on one worker as on N. On one worker the scheduler sees this directly: its queue is empty and its request table is empty. It must not return status 1 then, which host-concurrency.md H4 makes a fault of the module. On N workers it needs quiescence detection: a shared count of runnable and running tasks and of outstanding requests; the worker that sees it reach zero with every deque empty writes the stop cell. In a macro, which has no host services (`meta.md` §8), every wait with nothing to run is a deadlock.
- **Idle.** A worker with nothing to run, while others still work or requests are outstanding, waits. On one worker, and on worker 0 of a browser, it waits only by returning status 1 with an outstanding request (C-D's "wait for any"). On N workers an idle worker other than browser worker 0 parks with `memory.atomic.wait32`. Browser worker 0 has no request of its own when only other workers have work, so the multicore wave adds a host request that resolves when another worker wakes it (`Atomics.waitAsync` in JS). This is an import of `ilang:host/v1`, not a service, and a one-worker run never uses it.

## 7. Channels and CML events

- A channel is a handle as in part 5. `==` means "the same channel".
- Events are values. `choose` and `map` are pure. `sync` is the only commit point, and exactly one event of a `choose` commits. `map` functions run in the syncing task after the commit, never in the partner task. Nack and guard are not in v1; they are kept for later.
- **M:1 now.** Channel state lives in the scheduler state, as `Keyed` buffers and waiting continuations (D13). A commit is trivial on one thread.
- **Multicore wave.** Channel state, sync records and message buffers live in shared memory. A commit uses the parallel CML protocol of Reppy, Russo and Xiao (ICFP 2009): each sync record has a state word (WAITING, CLAIMED, SYNCED) set by compare-and-swap, and claims are taken in one global order to avoid deadlock. A message is encoded by the sender's worker (I4) and decoded by the receiver. A hand-off on the same worker passes the reference.

## 8. `scope.share`

- `scope.share(value) -> Shared(A)` belongs to the scope that makes it. `value` must be portable (P3).
- A `Shared(A)` handle is (scope id, index, key), so sending it costs O(1).
- `shared.get(s)` gives the value. On N workers, the value is encoded once into a region of the scope in shared memory, and each worker decodes it once and caches it for the life of the scope. Values from two workers are `==` (content plus ids, I1).
- A `get` after the scope has ended stops with `panic: task: shared value is gone`, in every build, also M:1, where the scheduler keeps the set of live shares. So the stop does not depend on placement. The region is freed when the scope ends.
- Considered and rejected: turning a `Shared` into a full copy when it leaves its scope. Finding handles inside arbitrary values at scope exit needs a walk, and width subtyping and type variables hide them from the types.

## 9. The copy format (multicore wave)

The format is MC's to design. It must:

- be a tag-directed walk with an explicit stack (no Wasm recursion, so lists of 10^6 elements are fine), covering every `$obj` tag, `$cons`, records (tag 14) and `Keyed`;
- keep sharing and cycles exactly (I3) with back-references, found by a mark or a `ref.eq` table (D9); a closure in a cycle is decoded with an empty captures array that is filled after;
- write Ints as i31 or limbs, Floats as bits, Strings and bit arrays as bytes, custom values with their constructor and type ids (fixed per module), closures as in part 3.4;
- write `True`, `False`, `Nil` and `[]` as markers that decode to the receiver's globals, and record labels as label indexes or bytes that the receiver interns again into its `$label_N`;
- stop the encode at tags 10, 11 and 12 and at a host request (P1). Because of P3, a portable path never gets that far.

## 10. Hosts

Facts at c69515c:

- `ilang-host` makes and joins a new 48 MiB thread for every call (`invoke_in`, `lib.rs:1162`).
- Both hosts insist on exactly one pending async import per yield (`lib.rs:1050`, `ilang.mjs:733`). The C/R request table lifts this per worker; that is needed for `i/task` to overlap I/O even with one worker.
- The module exports an unshared memory (`runtime/mod.rs:334`).
- wasmtime is built without the `threads` feature (`crates/ilang-host/Cargo.toml:15`), and the config does not turn on `wasm_threads` (`lib.rs:629`).
- Each `Store` reserves its own GC heap, 256 MiB by default (`lib.rs:1131`).
- `HostReference` is `Arc<dyn Any + Send + Sync>` (`lib.rs:349`); `reference_id` numbers are per instance (`State.next_reference`, `lib.rs:602`).
- The JS bridge copies with `.slice()` before decoding (`ilang.mjs:378`), which a `SharedArrayBuffer` view needs.
- The JS SDK takes host functions as JS functions on the calling thread, in `options.imports[module][name]` passed to `instantiate` (`ilang.mjs:624-703`).

**wasmtime (multicore wave).** Turn on the `threads` feature and `Config::wasm_threads(true)`. The module imports `(memory … shared)` instead of defining one. One Engine and Module serve all workers. Each worker is one long-lived OS thread that owns one `Store` and one instance; this replaces the per-call thread. The GC heap size is per worker. `ilang_alloc` and `ilang_free` take chunks from a shared bump word with an atomic add; a buffer freed by another worker goes on its owner's remote-free list. `$heap`, `$free_head`, `$shadow`, `$sp` and `$root_env` stay per-instance globals. The bridge uses `SharedMemory`. Idle workers park with `memory.atomic.wait32`. When the stop cell is set, the host makes the other Stores stop promptly (for example with epoch interruption, only in threaded runs). Rust host functions are `Send + Sync` and are linked into every Store.

**Browsers (multicore wave).** N workers only when `crossOriginIsolated` is true and `SharedArrayBuffer` exists; otherwise one worker, with the same meaning (D12). The `WebAssembly.Module` and the shared memory are posted to Web Workers. Worker 0 runs on the thread that called the entry, which in a page is the main thread.

- **Imports.** JS functions cannot be posted to a Web Worker, so the main-thread `imports` object of today cannot serve worker k > 0. The SDK gets a new option for multicore runs: an **imports module URL**, an ES module that each Web Worker imports to build its own imports object. A host function that needs the main thread (the DOM, for example) is written there as an async function that posts to the main thread and waits for the answer as a host request; it is never synchronous. A synchronous `@external` cannot be proxied: worker k would have to block with `Atomics.wait` until the main thread is free, while the main thread may itself be running worker 0's `ilang_start`. So a host that gives only main-thread functions, with no imports module URL, runs one worker. A host reference made on one thread stays there (H), so it never needs to cross as an `externref`.
- **Worker 0 never blocks.** It never uses `Atomics.wait`. Its scheduler idles only by returning status 1 with an outstanding request (part 6, Idle), so it returns to the event loop.

**Host rule (spec §9.7, §B).** A host service behaves the same whichever worker calls it. Host-side state that a service changes is shared by all workers (one Rust value behind an `Arc`, or state on the main thread). The number of workers is the host's choice.

## 11. Records and macros

- **Records** (`records.md`, tag 14). `ref.eq` on labels is only a fast path (I2). Width subtyping hides fields, which is one more reason the check is dynamic: `portable` and the copy look at tags, never at types. Records that hold functions (codecs) keep `==` across copies through ids. `i/task` needs no label-generic callbacks, and none of its functions has label parameters (records L5). Any later shape id or inline cache must stay out of the copy format, `==` and `hash`.
- **Macros** (`meta.md` §8). Macro programs run in the compiler, single-threaded, on exactly one worker with the FIFO scheduler and no clock unless a fake one is given. So `i/task` in a macro gives the same expansion on every machine. A deadlock in a macro stops with `panic: task: deadlock` (part 6).
- **Syntax values at run time.** `Syntax` values exist at run time in two kinds of programs: macro programs, and tests under `i test`, where a quote gives a value (`meta.md` §8). `Syntax` has no constructors, and its form is the compiler's (`meta.md` §8: "It is not seen in i"). This note fixes that form: in a program that runs, a `Syntax` value is a host reference given by the compiler, as §10.3 does for a custom type with no constructors. So it is not portable by the general rule, and its identity `==` (`meta.md` §3.1) is that of a host reference. A task may pass `Syntax` values on local channels and local scopes and keep them in its own data. Sending one on a portable path, or giving one as the result of a child of a portable scope, stops with `panic: task: value is not portable`, in a macro and in a test, on one worker as on N. Giving `Syntax` ids so that it can be copied is a later, compatible change (open question 8).

## 12. Future architecture and proposals to track

1. **Now (R):** M:1 scheduler in i, with every rule of this note. `core.portable`, `control.keep_env` and keys (D13) land.
2. **Wave 5 (MC):** N instances, one shared memory, private deques, cooperative steal of unstarted tasks with a sharing-keeping copy, `scope.share` regions, CAS-committed CML, ids in `$clo` field 2, quiescence detection, the imports module URL in JS.
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

No section is renumbered; new content is added as subsections or sentences. The text lands with track R, together with `core.portable`, `control.keep_env` and keys, after A2, so it is in the new syntax. The parts marked *(multicore wave)* land with MC-I.

**§2.5 Top-level bindings.** After "Its value is worked out once, when the program starts, before any entry point runs (§B.1).", add:

> When the program runs on more than one worker (§11.9), each worker works out the top-level values in the same way, and they are the same values on every worker. A program whose top-level values are not all portable (§11.9) runs on one worker.

**§9.2 Environment and services.** Add after the bullet "A service is a normal value. …":

> - `control.keep_env(f)` (§9.4) gives a function that, wherever it is called, calls `f` with the services that were bound at the place of the `keep_env` call when it ran. It keeps every service bound there, also those that `f` does not need. An environment statement inside `f`, as in `(x: S) { ! T = x  body() }`, gives `T` its own value there as usual.

**§9.4 Control.** Add a row to the table:

> | `control.keep_env` | `(f: (x: S) -> A ? E ! N) -> ((x: S) -> A ? E) ! N` |

Add rule 7:

> 7. A boundary and a continuation stay on the worker that made them (§11.9). They are not portable.

**§9.7 Host services.** Add a bullet:

> - A function of a host service does the same thing whichever worker calls it (§11.9). An async function goes on in the task that called it, on the same worker. A host request (when R2 adds it) stays on the worker that started it and is not portable.

**§10.1 External functions.** Add:

> An external function that is not of the library package `i` and is called while top-level values are worked out may be called once for each worker (§11.9). The program runs on more than one worker only when the host says that each such function gives equal results for equal arguments and that calling it again has no effect that can be seen.

**§11.4 Equality.** After the bullet on functions, boundaries, continuations and host references, add:

> - A value stays the same value when the program copies it from one worker to another (§11.9). So a function given to another task is the same as the function that was given.

**§11.7 Stops.** After the paragraph on the stop message, add:

> When the program runs on more than one worker and one of them stops, the whole program stops. The host gets one stop message, that of the first stop.

**§11.9 Workers (new).**

> A program may run on one or more **workers** at the same time. Which worker does which part of the work cannot be seen. Tasks that run at the same time take their steps (sends and receives, host calls, failures and stops) in an order that the program does not fix; this order may differ from run to run and with the number of workers. Apart from that order, a program gives the same values, the same errors and the same stops on one worker as on many. So the order may decide which of two tasks that stop stops first, which event of a `choose` commits, which tasks fail before a cancel reaches them, and the order of host effects of tasks that run at the same time. Running out of memory or stack (§11.7) is not covered by this rule. The host chooses the number of workers.
>
> The **captures** of a function made by a function expression are the local names that its body uses and that are bound outside it, after macro expansion; a name counts when it is written in the body, also in a part that never runs. Top-level names are not captures. A function made by `control.keep_env` (§9.4) has the captures of its argument and the services it keeps. Other functions have no captures.
>
> The values that a value **holds** are: the parts of a tuple, a list, a value of a custom type or a record (§11.4); the values of the captures of a function; and the value in a `Keyed` (§13). `==` does not look at what a function holds (§11.4). A value is **portable** when it is not a boundary, a continuation, a host reference or a host request, and every value it holds is portable. A value may hold itself, through a local function that calls itself; only the other values it holds then count.
>
> A value that is not portable never goes to another worker. Values may be copied between workers; a copy is the same value (§11.4).

**§13 The module `i/core`.** Add the rows:

> | `portable` | `(value: A) -> Bool` | `True` when the value is portable (§11.9) |
> | `key.new` | `() -> Key(A)` | A new key, the same only as itself |
> | `key.wrap` | `(k: Key(A), value: A) -> Keyed` | The value, marked with the key |
> | `key.unwrap` | `(k: Key(A), x: Keyed) -> Found(A) \| Missing` | The value, when `x` was made with `k` |

(The key rows wait for G2's answer to open question 7; the names are track R's.)

The `hash` row: "Which number is not fixed." becomes:

> Which number is not fixed, but in one run it is the same on every worker. A function, a boundary, a continuation and a key give a number from their kind alone.

**§B.1 The module.** Add *(now)*:

> This version runs one instance for each run of the program. Workers (§11.9) come with a later version of this interface.

*(multicore wave)* The module imports its memory as a shared memory and exports `ilang_worker(index: i32)` and `ilang_toplevel_portable`; `ilang_start` is called on every worker.

**§B.4 Imports.** Add to `reference_id`:

> A host reference belongs to the instance that got it. It never goes to another instance.

**§B.6 Metadata** *(multicore wave)*: whether start-up calls a host external (part 3.5).

**§9.x `i/task`** belongs to track R. It states the API rules of part 18 where programs can see them, including the stop messages `task: value is not portable`, `task: shared value is gone`, `task: deadlock` and `task: no scheduler`, and the order of D1.

## 14. Files to change

**Track R (now):**

- `docs/spec.md`: §2.5, §9.2, §9.4, §9.7, §10.1, §11.4, §11.7, new §11.9, §13, §B.1, §B.4 (part 13).
- `lib/i/core.i`: export `portable` and the key functions as `@external("ilang:core", …)` declarations, like `not` and `hash`.
- `lib/i/control.i`: `keep_env`.
- `crates/ilang-wasm/src/lower.rs`: add `portable` and the key functions to the `CORE` and `CORE_FUNCS` name tables near `:120` and `:130`; the `keep_env` op, which keeps the `$env` array as it is (part 4); bit 0 of field 1 for closures with patched captures (part 3.6). `captures_of` (`:389`) is the definition of captures; it does not change, and a comment points at §11.9.
- `crates/ilang-wasm/src/emit/mod.rs`: `closure()` writes the patch flag in field 1.
- `crates/ilang-wasm/src/emit/control.rs`: `$keep_reenter` and the frame record of a kept function (part 4).
- The checker's built-in types for `portable`, `keep_env` and keys (`crates/ilang-check`).
- `crates/ilang-wasm/src/runtime/values.rs`: `$core_portable`, a walk with an explicit stack like `$value_compare_i32`, with the list of marked closures of part 3.6. It gives False at tags 10, 11 and 12 and at a host request, and walks field 5 of `$obj` (tuples, custom values, closure captures, records), `$cons` heads and tails, `Keyed` values, and kept services. The key functions.
- `crates/ilang-wasm/src/runtime/mod.rs`: the tag table (`:46`) gains a "portable" note, the patch flag, the key tag, and the planned `$clo` layout of part 3.4.
- `crates/ilang-check/tests/library.rs`: the core and control export lists.
- `lib/i/task.i`, `lib/i/random.i`: following part 18.
- `docs/performance.md`: the cost of `portable` checks (also on DAGs, part 3.6), pinned tasks under boundary services (part 4), and the start-up conditions of part 3.5.

**Track B (now, small):** frame records carry the point index (part 12).

**Multicore wave (not now):**

- `crates/ilang-wasm/src/emit/mod.rs`: `closure()` with flags and arity in field 1 and id in field 2; static ids; the `$funcs` table; the id counter and worker globals.
- `crates/ilang-wasm/src/runtime/values/inspect.rs:262`: arity from field 1.
- `crates/ilang-wasm/src/runtime/values.rs`: id compare for tag 7 and keys.
- New runtime modules for the copy, request cells, the stop cell, quiescence, share regions and `ilang_toplevel_portable`; the shared-memory allocator in `runtime/mod.rs`; the walk mark of D9 if MC picks it.
- `crates/ilang-host`: the `threads` feature, `SharedMemory`, long-lived worker threads, per-worker GC heap, the stop path, the start-up conditions of part 3.5.
- `bindings/javascript/ilang.mjs`: the isolation check, Workers, the imports module URL, worker 0 rules, the wake request.
- `tests/browser`: COOP/COEP headers and a fallback test.

## 15. Tests

**Now (track R):**

- **T1** `tests/spec/13/portable.i`. True for: an Int, a big Int, a Float, a String, a bit array, `[]`, a tuple and a list of these, a top-level function, a constructor, a closure that captures only data, a closure that captures another such closure, a local recursive function (it holds itself), and two local functions that call each other. False for: a boundary `b` (inside `control.boundary`), a continuation (inside `suspend`), a closure that captures `b`, a local recursive function that also captures `b`, a list holding such a closure, a custom value with a boundary field. After records land: `{ k: b }` is False.
- **T2** `tests/spec/9.7`, with a host fixture as in `tests/spec/11/equality_externals.i`: `portable` of a host reference is False, and of a tuple that holds one too.
- **T3** `tests/spec/11/equality.i` stays unchanged; it guards that §11.4 did not change.
- **T4** Captures are exact: the example of part 3.1 is True; a closure whose only use of `b` is in a clause that never runs is False. Both run at O0 and O2 and give the same answers.
- **T5** `keep_env`: a kept function called under a different environment statement still sees the kept service, also after it suspends and is resumed under another environment (`$keep_reenter`). A kept function made where a non-portable service is bound is not portable, also when its argument does not need that service, and also when `keep_env` is called inside a generic wrapper; the answer is the same at O0 and O2. A kept function made after that service is bound again to a portable value is portable.
- **T6** A runtime unit test runs `$core_portable` on every tag, on a list of 10^6 elements (no Wasm stack overflow), and on a closure that holds itself (it ends).
- **T7** An emit test pins today's `$clo` layout (field 1 is the patch flag, field 2 is the arity, field 3 is 0) and names the planned layout in its message. A unit test pins the list of readers of `$obj` field 3 of part 3.4: `$float_get` and the readers behind a tag-2 test.
- **T8** Track R: a send of a host reference on `channel.new` stops with `panic: task: value is not portable` on one worker; the same send on a local channel works; a child giving back a host reference stops in a portable scope and works in a local scope; a `get` after the scope ended stops with `panic: task: shared value is gone`.
- **T9** Track R: with a fixed seed, per-task random streams are the same under two scheduling orders (a test scheduler service picks the order).
- **T10** Track R: a scope returns only after every child ended; final functions of cancelled children run once; with failures at go indices 3 and 1 that both happen before any suspend point, the error is that of index 1 under both scheduling orders; `both` and `map_concurrently` give argument order.
- **T11** Track R: a task started inside a generator body keeps the generator's `Yield` service; when it yields, it stops with `control: boundary is not active`.
- **T12** Track R: `i/task` in a macro program gives the same expansion twice. In a test under `i test`, `map_concurrently` over children that give `Syntax` values stops with `panic: task: value is not portable`, and the same work in a local scope works.
- **T20** Track R: `scope((s) { c = channel.new()  s.go(() { c.receive() }) })` stops with `panic: task: deadlock`, and the module never returns status 1 with no request outstanding. The same in a macro program.
- **T21** Track R: keys: `unwrap` with the key that made a `Keyed` gives `Found`, with another key `Missing`; `k = key.new()` is not generalized (using `k` at two types is a compile error); a `Keyed` is portable when its value is.

**Multicore wave:**

- **T13** Differential: every `tests/spec` case and every std test runs single-core, threaded with 1 worker, and threaded with 4 workers (the last 200 times). Results, stdout and stop messages must be the same, except in tests marked as order-dependent (D1). A marked test lists what may change: its stdout is compared as a multiset of lines, and its stop message must be one of a listed set.
- **T14** Copy round-trip for each kind: `==`, `hash` and text form equal; constant closures, `True`, `False`, `[]` and labels come back `ref.eq` to the receiver's globals.
- **T15** Sharing: `x0 = Leaf`, `x(i+1) = Node(x(i), x(i))` to depth 40, and the same with list cells: the copy has 41 nodes. A closure that holds itself copies to a closure that holds itself.
- **T16** Closure and key identity across a steal, also inside records and tuples. A child of a local scope, and a task whose record holds a host request, are never stolen.
- **T17** A stop on worker 3 while the others loop: exactly one message, all threads join, no final functions run.
- **T18** Cross-worker select under stress, 200 runs: no message lost or duplicated, one commit per sync.
- **T19** Browser: with COOP/COEP and an imports module URL, 4 Workers and a main thread that stays responsive; without isolation, or without the URL, one worker with the same output; strings through the shared memory decode right.
- **T22** Start-up: a module with a top-level host reference (`counter` of part 3.5) runs one worker and gives 100; a module whose start-up calls a host external that the host has not declared pure runs one worker; a module that only uses `i/core` externals at start-up runs 4.
- **T23** Deadlock and idle on 4 workers: T20 gives the same stop; a run where worker 0 has nothing to do while workers 1 to 3 work ends normally, and in a browser worker 0 waits only with the wake request outstanding.

## 16. Risks

- **`portable` makes captures visible.** An optimizer that drops or adds a capture changes `portable(f)`. The spec definition and T4 at O0 and O2 guard it; any later closure optimization must keep the capture set.
- **The check costs a walk.** Every portable send, result and error walks the value, also on one worker, and a DAG is walked once for each path (part 3.6). Large data should go through `scope.share` or local channels. A later mark (D9) or a "known portable" bit can make repeat checks O(1).
- **Stopping on a non-portable result may surprise.** `go(open_socket)` in a portable scope stops. The local scope API and a clear message are needed, or people will route around the rule.
- **`keep_env` keeps every bound service.** A task started under a boundary-holding service that it does not use (a generator's `Yield`, the `Assert` of `assertion.catch`) is pinned. So tests that run under `assertion.catch` get no parallelism. It cannot be seen, but `docs/performance.md` must say it. Changing `keep_env` later to keep fewer services would change `portable`'s answers.
- **`keep_env` and keys are new language surface.** G2 must accept them as general.
- **Order still chooses some things.** Which children fail before the cancel reaches them, which `race` branch wins, which stop is reported when two tasks stop, and the order of output from tasks that run at the same time, depend on the order on N workers (D1). Tests must not depend on them, or must be marked (T13); the M:1 order is fixed but is not a promise for N workers.
- **Start-up runs N times.** It costs N times the start-up work, in parallel. Modules that make host references at start-up, or call host externals the host has not declared pure, run one worker.
- **Host limits.** One pending async import per instance today blocks overlapping I/O; the request table of C/R must come first. N Stores each reserve a GC heap. In browsers, host functions must come from an imports module URL, and main-thread functions are async proxies with latency.
- **Field 1 and field 2 move.** If other work reads field 1 or 2 of closures before MC-I, the move breaks it. T7 and the tag-table note guard it.
- **Left to MC.** The CAS protocol, the allocator, the mark or table of D9, quiescence detection, poll-point placement, epoch stops and the memory model are not settled here. A long computation with no poll point delays steals and cancellation, as it blocks the worker in M:1 today.

## 17. Open questions

1. Is `core.portable` accepted as a public built-in (D3), or should G2 ask for a narrower form? This note argues for public, because `i/task` is ordinary i code.
2. `control.keep_env` takes one parameter. Is that enough, or should there be a form with no parameter as well (`() -> a` is `(x: Nil) -> a` today)?
3. Per-task storage for `i/random` (part 5): a general `task.local` facility in `i/task` built on keys, or Random as a value passed explicitly? Track R decides; the rule is only that streams depend on seed, path and draw count.
4. How does worker k > 0 enter `i/task`'s scheduler loop without a special case? One answer: the program's entry runs `task.run`, which, on N workers, is an ordinary host export or entry kind that every worker calls. MC-D decides.
5. How long may a victim wait before it answers a steal request? Poll points only, or a heartbeat in function prologues later?
6. Should a copy encode a message at offer time or only when a commit pairs it with another worker (part 7)? MC-D decides.
7. Does G2 accept keys (D13) as a public `i/core` primitive? Without them, or another safe way to keep values of many types in one service, the scheduler cannot be written in i, and rules 12 and 15 cannot hold.
8. Should `Syntax` later get ids and become portable, so that tests and tools can pass code between tasks of a portable scope (part 11)?

## 18. API rules for track R (`i/task`)

1. Values pass between tasks only through `go` captures and kept services, channel and event sends, task results and errors, and `scope.share`. There is no other shared state.
2. Every portable path (sends on `channel.new` channels, results and errors given to a scope made by `scope`, `share`) calls `core.portable` and stops with `panic("task: value is not portable")` when it is False. This happens always, also on one worker.
3. `go` never checks portability. Whether a task can move is decided only at steal time, from its whole record (kept start function and scope handle); a failed encode leaves the task home and cannot be seen.
4. `go` binds `Scheduler` to the portable `scheduler.detached` and then keeps the parent's services with `control.keep_env`, and takes the real `Scheduler` as the parameter of the kept function. So the parent's scheduler is never kept or captured.
5. Every task runs directly from its worker's scheduler loop, never on another task's stack. Nested scopes, `both`, `race` and `map_concurrently` never run a loop on the caller's stack.
6. Local channels and local scopes hold the root boundary of their home worker's scheduler, so they are not portable by the general rule. Every task record holds the handle of its scope, so a child of a local scope is never stolen. Local paths do not check. There is no flag and no special case.
7. A started task never moves. A task that waits on an async host call resumes on the worker that started the call. A host request value (R2) is not portable, so only tasks on its home worker wait on it, poll it or cancel it.
8. A scope returns only after all its children have ended. On a failure it cancels the rest and gives the error of the failed child with the lowest `go` index (its own body first). Error types stay unions.
9. `both` and `map_concurrently` give results in argument order. Only `race` and `choose` let timing pick, and the docs say so, together with the rest of the order of D1.
10. Cancellation is cooperative: an unstarted task is dropped; a started task gets `control.cancel` on its home worker at its next suspend point.
11. Every task has a path (parent path plus `go` index), given by `task.path()`. Random streams, handle ids and per-task names come from the path, never from time, a global counter, or the worker.
12. Task, scope, channel, event and `Shared` handles are made of path-based ids and keys (D13), and hold no boundary, closure over scheduler state, or other non-portable value (except local handles, rule 6). `==` on them means "the same handle". No API needs an order or a hash on functions, keys, boundaries or continuations; scheduler bookkeeping uses Int ids.
13. Events are values. `choose` and `map` are pure, `sync` commits exactly one event, and `map` functions run in the syncing task after the commit. Nack and guard are left for later.
14. `scope.share(value)` needs a portable value. A `Shared` belongs to its scope, and a `get` after the scope ends stops with `panic("task: shared value is gone")` in every build.
15. All shared mutable scheduler state is behind the one `Scheduler` service, kept as `Keyed` values where it has many types, so the multicore wave can replace its implementation without changing the API. No API needs an unsafe cast.
16. The scheduler has poll points (`go`, every `sync`, joins and scope ends, `yield`, async host calls, every switch of the loop) written as one call that does nothing on one worker.
17. On one worker the scheduler is FIFO and fixed, with no time unless a `Clock` service is given. It idles only by waiting for an outstanding host request (C-D's "wait for any"), never by a busy loop or a blocking import, and never returns status 1 with no request outstanding. When no task can run and no request is outstanding, it stops with `panic("task: deadlock")`.
18. Many host calls may wait at once, through the request table. No code relies on one pending import per instance.
19. A stop in any task stops the program. It is never caught or counted as a task error.
20. No API takes a label-generic callback (records L5), and no API needs a static `Send`-like type.
21. No API shows worker indices, the worker count, or steals.
22. Tests of `i/task` and of programs that use it must not depend on the order of D1 (which of two stops, which `race` branch, which children fail before a cancel, the order of output of concurrent tasks), or must be marked as order-dependent (T13).
