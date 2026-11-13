# Design: multicore constraints

Status: draft for review (study S7 of the redesign plan). It is reviewed at gate G2. It fixes what the first `i/task` (track R) and the language must guarantee now, so that the multicore wave (MC-U, MC-D, G5, MC-I) can add more workers later without changing what any program does. It does not design the multicore runtime; it lists the invariants that runtime must keep. Native adoption replaces D8, the MC channel commit backend and native transport
with native.md N7/N8/N11/N12. Runtime.md supplies the accepted request, join and
Runner refinements. Other rules remain. Code facts are from commit c69515c. Line numbers are at that commit.

The examples use the new syntax of `docs/design/syntax.md`. The spec text in part 13 uses the new syntax too, because it lands with track R, after A2 has moved `docs/spec.md` to the new syntax.

## 1. The problem

i compiles to WasmGC. Today a WasmGC object cannot be shared between threads: the shared-everything threads proposal is not done, and wasmtime 50 does not have shared structs. So the route to more than one core is this:

- One module, **N instances**. Each instance is a **worker**. Native transports bytes in runner memory; browsers use shared linear memory (native N11).
- At `go`, the maker silently classifies the complete task record. A portable record is encoded into one shared queue; other records stay local. A worker claims an unstarted record once. The maker runs its retained original; another worker decodes the bytes.
- `scope.share` stores immutable encoded data once in the host transport. It is freed when its scope ends.
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
7. **Hosts.** wasmtime needs one `Store` per executing worker; native transport needs no shared Wasm memory. Browsers need `SharedArrayBuffer`, which needs cross-origin isolation (COOP and COEP), and the main thread must never block.

## 2. The decisions

**D1. One semantic rule, tested from day one.** Tasks that run at the same time take their **steps** (sends and receives, joins, `shared.get`, host calls, failures and stops) one after another, in an order that the program does not fix. That order may differ from run to run and with the number of workers. The rule: **every run, on any number of workers, gives a result that some order of the steps gives, and the number of workers changes only which order happens.** Each step does what it would do if the tasks took their steps one at a time in that order. So anything that depends on the order of steps may change: which of several senders a plain receive pairs with, and so the value received; which branch of a `race` or a `choose` commits; which children fail before a cancel reaches them; whether a step comes before or after another task's step, such as a `shared.get` before or after its scope ends (and so whether a stop happens at all); a value a host call gives back when it depends on what other tasks did through the host; which of two stops is first (D11); which children are blocked when a deadlock is found; and the order of output. These are examples, not a closed list. A program whose results do not depend on the order of its steps gives exactly one result. Running out of memory or stack is outside this rule, as it is today (§11.7, last row). *Reason:* this is the plan's rule, stated so that the design can keep it and so that N workers can keep it: a list of the things that may change would always miss one. It is only cheap if the one-worker scheduler already behaves as one of the orders that N workers allow.

**D2. Portable values, checked at run time, on every worker count.** A value is **portable** when nothing it holds is a boundary, a continuation, a host reference, a host request or a `Syntax` value (part 3.1). Portable send/result/error/share paths check this, also on one worker, and stop with `panic: task: value is not portable` on failure. Go publication instead silently classifies the complete task record and pins on failure (D8/P1). *Reason:* a static rule cannot work (hard part 3). A check that only ran on N workers would make placement visible. A check that runs always gives the same stop everywhere.

**D3. `core.portable` is a public built-in, and captures are defined exactly.** `portable(value) -> Bool` is in `i/core`. The spec defines the values a function holds as the values of its **captures**: the local names that its body uses and that are bound outside it, after macro expansion (part 3.1). Closure conversion (`captures_of`, `lower.rs:389`) already computes exactly this set, and every optimization level must keep it. *Reason:* `i/task` is ordinary i code, so the check must be something any i code can call; there is no library-private built-in. It is general: besides tasks, it serves (a) persistence and serialization, which must refuse values that cannot be written out, before writing half of them; (b) caches and memo tables that outlive one request or one computation, which must not keep a continuation or a host reference alive past its owner; (c) sandbox and plugin boundaries, which must not let a continuation or boundary escape back into the caller. Defining captures by the source, and kept services by the run (D4), makes the answer independent of the optimizer.

**D4. `control.keep_env`: a general way to keep the services of a place.** A new function in `i/control`:

```
keep_env(S, A, E, N): (f: (x: S) -> A ^ E ! N) -> { (x: S) -> A ^ E } ! N
```

(In a type, `{ }` groups; `( … )` with no `->` after it is a tuple type, §3 and §6.4.) It gives a function that, when called anywhere, runs `f` with the services that are bound at the place of the `keep_env` call. It keeps **every** service bound there when the program runs, also those that `f` does not need. `go` uses it to give the child the parent's services. Before the call, `go` binds `Scheduler` to a portable stand-in, and the task gets its real scheduler as the parameter `x` (part 4).

Inside a task, the place of any other `keep_env` call has the task's real `Scheduler` bound, so the kept function keeps it, by the same rule: its scheduler calls (a send, a receive, a `go`, a join, a sleep) go to the boundary of the task that made it. Called from that task, it works. Called from another task, those calls stop with `control: boundary is not active`, on any number of workers (D5), and the kept function is not portable, because it holds that boundary. So a kept function made inside a task is **tied to that task** wherever it uses `i/task`. This is the general rule, not a special case, and it is the same on every worker count. For a callback that other tasks will call, `i/task` gives `task.keep(f)`, ordinary i code built like `go`: it binds `Scheduler` to `scheduler.detached` before it calls `keep_env`, and the function it gives needs `Scheduler` in its type and passes the `Scheduler` of the place of each call to the kept function, which binds it. *Reason:* §9.2 closures get their services from the place of the call. A task is called from the scheduler loop, so without this it would lose the parent's environment statements (a fake `Clock` in a test, for example). The function is general: tasks; callbacks stored now and run later from a dispatch loop (an event emitter, a host event source; inside a task, through `task.keep` when another task runs the loop); lazy values and stream stages kept in data and forced elsewhere. Its type has one environment variable, so inference stays principal. Keeping every bound service, not "exactly the services of `N`", is a choice: at c69515c the environment is one `$env` array indexed by service id, generic code is not specialized, and keeping exactly `N` would need the slot set of every environment variable passed at run time (rejected, part 4). The rule by the run is simple, exact, and the same at every optimization level. Each call of `keep_env` makes a new function value, the same only as itself (§11.4), as each run of a function expression does.

**D5. The scheduler reaches a task only as an argument from the worker loop.** Each worker runs its own scheduler loop. For each task start, the loop makes a **new boundary for that task**, outside the loop's own frames (`control.boundary((b) { start(scheduler_for(b)) })`), and gives the task a `Scheduler` service whose suspends go to `b`, as the argument of the kept function. When the task suspends, `$boundary_run` (`emit/control.rs:195-232`) runs `f(k)` in the boundary's place, so control comes back to the loop with the continuation `k`, and `b` is not active. `control.resume(k, v)`, called from the loop, makes `b` active again for the rest of the task. There is no long-lived boundary of the worker: a suspend to a boundary that encloses the loop would capture the loop itself into `k`. No task closure, kept service, channel handle, scope handle or event that `i/task` makes ever holds a `Scheduler` that the task did not get this way (a kept function that user code makes inside a task does hold its task's `Scheduler`, D4). Every task runs directly from its run's loop, never on another task's stack. The loop never calls `control.cancel` on a task's continuation: a cancel resumes the task, and the task unwinds itself under an inner boundary, so that `b` stays active while its final functions run (part 6). *Reason:* if task closures held the scheduler's boundary, every task would be non-portable, nothing could be remotely claimed, and every send of a closure would stop. Running every task from the loop under its own boundary makes "a boundary of another task is never active" true on any number of workers. A nested run (part 4) runs its loop inside the boundary body of its calling task; that is the one case where a task of one run is on the stack of a task of another run, and part 4 gives its rules.

**D6. Stable ids for functions, in field 2 of `$clo{n}`.** In the multicore wave, every function value gets a 64-bit id in the existing `i64` field 2, and the arity moves to the `i32` field 1, next to the cycle flag that track R puts there (part 3.6). `==` on two functions that are not `ref.eq` compares ids. **`hash` stays tag-only** for functions, boundaries and continuations, as `values.rs:287` does today. Nothing gets an order on functions. *Reason:* copies must keep sameness (§11.4). Field 2 is a native `i64`, so there is no bit trick, and only one reader (`inspect.rs:262`) changes. Ids depend on the worker, so a hash or an order over them would make `Dict` order depend on placement.

**D7. Every worker works out the top-level values itself, and only portable ones are allowed on N workers.** `ilang_start` runs on every worker, with a fixed id stream, so every worker gets equal top-level values with equal ids. Top-level names are not captures, so top-level values never need to move. Two conditions keep this from being seen, and when either fails the run uses one worker (a load-time choice, D12): (a) after start-up on worker 0, every top-level value must be portable; (b) a **host external** (one that is not of the library package `i`; part 3.5) called during start-up must be declared pure by the host. *Reason:* a host reference made at start-up would be a different object on each worker, and a top-level function that uses it is a constant closure that a remote claimant would run against its own object (part 3.5). Condition (a) catches every such value with the walk that `portable` already needs.

**D8. Publish at go into one shared queue.** On multiple workers the maker
silently classifies the complete immutable task record at `go`: kept start
function, captures, every kept service, scope handle, path and chain. If portable,
it encodes once and publishes bytes, retaining its original. Publication registers
the child in its scope and rechecks cancellation under the same lock; a cancel
during encoding drops the unstarted record and frees prepared bytes, with
runtime R4a applied to the caller. Otherwise a non-portable record queues
locally, with no rejection. A claim under N8's lock changes Ready to Taken(owner)
once. The maker uses its original; a remote owner decodes. Cancel/drop and claim
use the same transaction: cancel before claim drops the record; claim before
cancel routes cancellation to the claimant. Claimed-but-unstarted records can
still be dropped and wake joiners under runtime R4a; started tasks never move.
The maker drops its retained original after remote claim/drop acknowledgement;
queue entries are freed after claim/drop; a remote claimant owns payload bytes
until decoding finishes. Queue entries never retain cancelled
scopes indefinitely. One worker uses its FIFO originals and does no encode.
*Reason:* native N7 is adopted; only the maker can read its GC heap, and encoding
at go avoids waiting for a busy maker to poll. Failed portability classification
pins silently on every worker count; it never becomes P3's visible stop.

**D9. Copies keep sharing and cycles exactly.** The multicore wave's copy must write a node that was already written as a back-reference, so a DAG is never expanded and a cycle ends. Finding a node that was already written needs either a mark on the object or a table looked up by `ref.eq`. WasmGC has no identity hash and every `$obj` field is immutable, so a mark needs a layout change (a mutable `i32` field in `$obj` and `$cons`). MC chooses: with the mark, the copy costs O(distinct nodes); with a `ref.eq` table, O(n^2) in the distinct nodes. *Reason:* a memory stop that only happens on N workers would make placement visible in practice, even if the resource-stop exception allows it in theory.

**D10. Order comes from the task tree, never from time.** Each task has a **path**: the path of the task that called `go` plus the index of that `go` among that task’s `go` calls and nested runs counted together (1, 2, …), whatever scope each went to. It is an ordinary `i/task` value (`task.path()`). Random splitting, handle ids, logs and traces use it. Paths are compared in **path order**: element by element, and a path comes before every longer path that starts with it (`[1] < [1, 1] < [2]`). A scope's error is the failure of the task with the **lowest path** among the scope's body and the children of the scope that failed. The body's path is a prefix of the path of every child, so the body comes first. `s.go`, and a join of a child of `s`, may be called only by a task whose **chain** holds `s` (part 6); the chain of a task changes only by its own steps, so this is fixed by the program, not by time. `both` and `map_concurrently` give results in argument order. *Reason:* these make results depend only on which children failed, not on which failed first, so one worker and N workers agree whenever the same children fail. Paths are unique, so no two children compare equal, also when children of `s` call `s.go` themselves.

**D11. A stop anywhere stops the whole run, and the first stop wins.** Native N12 supplies forced termination of non-polling workers and host teardown; ordinary task scheduling/cancellation remain cooperative. *Reason:* §9.5 already says no final functions run at a stop. The shared stop cell, written once under N8's lock, keeps one message, as on one worker. When two tasks may stop, which one is first is part of the order of D1, so the message may differ; with at most one task that may stop, it is fixed.

**D12. The worker count is the host's choice and cannot be seen.** A browser without cross-origin isolation, a module that fails a condition of D7, and any host that just wants one worker, all run one worker. No `i/task` API shows worker indices, the worker count, or queue claims. *Reason:* D1.

**D13. A typed key for scheduler state.** The scheduler is ordinary i code, and one non-generic `Scheduler` service must hold the buffers of channels of many element types, the values of many `share` calls, and the waiting receivers of each. Rank-1 i has no existential, no top type and no type test on a type variable, so this cannot be typed without a new primitive. The note asks G2 for one general, safe primitive, in a new library module `i/key` of the package `i` (§1.3 names have no `.`, so `key.new` is the module name `key` and the export `new`, as `control.keep_env` is): `key.new(A): () -> Key(A)` gives a new key on each call; `key.wrap(A): (k: Key(A), value: A) -> Keyed` and `key.unwrap(A): (k: Key(A), x: Keyed) -> Lookup(A)`, where `Lookup(A) { Found(A) Missing }` is an ordinary custom type of `i/key`, so `Found` and `Missing` are not built-in names and a module gets them as it gets the constructors of any library type. `unwrap` gives the value only when `x` was made with the same key. `Key(A)` and `Keyed` are **built-in types** (spec §4.2, §11.1, §11.8 and §12 name them, and the new spec section for `i/key` defines them), not custom types of `i/core`: a custom type with no constructors would make its values host references (§10.3), so keys would not be portable, and the checker gives every parameter of such a type `Variance::Bi` (`infer.rs:2242`), whose constraints are dropped (`solve.rs:363`). Safety needs two things: **`Key(A)` is invariant in `A`** (§4.5 rule 7 gets the row, and `builtin_variance` in `solve.rs:1285-1292` gets `Key: [Inv]`; with no row, or with the same direction, `wrap` at `Int` and `unwrap` at `String` with one monomorphic key would type check and be an unchecked cast), and `key.new()` is not a direct value, so `k = key.new()` is not generalized (§4.7, the value restriction). Under §4.7, a key that is not generalized may still be given to `wrap` at two types (its parameter collects `Int | String`); then `unwrap` gives `Int | String`, and using it at a narrower type is a compile error. A key is the same only as itself, like a function, and gets an id in the multicore wave (D6); a `Keyed` is portable when its value is. *Reason:* the alternative, handles that carry typed closures, makes handles non-portable as soon as those closures reach scheduler state. The primitive is general: per-task storage (open question 3), typed heterogeneous maps (plugin registries, typed property bags, request context in a web framework), and typed event buses, besides the scheduler. The exact names are track R's, and the primitive needs G2's approval (open question 7).

## 3. The rules

### 3.1 Definitions (spec §11.9 and §9.2)

- A **worker** runs tasks. A program run has one or more workers. In the multicore wave, each worker is one instance; native uses runner byte storage and browsers shared linear memory (native N11).
- The **captures** of a function value made by a function expression are the local names that its body uses and that are bound outside the function expression, after macro expansion. A name counts when it is written in the body, also in a part that never runs. Top-level names are not captures. A constructor, a built-in, a library function and an `@external` have no captures. A function made by `control.keep_env` has the captures of its argument and the services it keeps.
- The values a value **holds**: the fields of a tuple, list, custom value or record (its parts, §11.4); the values of the captures of a function; the value in a `Keyed`. Ints, Floats, Strings, codepoints and bit arrays hold nothing. The word is new because §11.4 says a function has no parts that may be looked at, and that stays true: `==` never looks at captures.
- A value is **portable** when it is not a boundary, a continuation, a host reference, a host request or a `Syntax` value (part 11), and every value it holds is portable. A value may hold itself (a local recursive function, part 3.6); the rule then asks only about the other values on the way, so such a function is portable when they are.

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
- **I1 (meaning).** `==` depends only on the content of a value and on function and key ids. `hash` and the text form depend only on the content: **only `==` reads ids**. Ids carry the worker index and the state of that worker's counter (part 3.4), so a text form or a hash that showed one would show placement. The text form of a function stays `//fn(a, b) { ... }` (only the arity, as `$inspect_function`, `inspect.rs:262`, writes it today), that of a key is `//key`, and that of a `Keyed` is `//keyed`; none of them shows an id or what a key marks. None of the three depends on addresses, instance globals, or which worker holds the value.
- **I2 (fast paths).** `ref.eq` is only a fast path (for `True`, `False`, `Nil`, `[]`, labels, constant functions, identity values). Every answer it gives, the slow path gives too.
- **I3 (sharing).** A copy keeps sharing and cycles exactly (D9).
- **I4 (owner writes).** Only the worker that owns a GC object reads it to encode it. The receiver decodes into its own heap. No worker touches another worker's GC objects.

### 3.3 Paths between tasks

A **task**, as the scheduler keeps it, is a record of four things: its kept start function (part 4), the handle of the scope it was started in, its path, and its chain (part 6), which is data. Values pass between tasks only through these paths. There is no other shared state.

| Path | Checked with `portable` | Why |
|---|---|---|
| Complete task record of a `go` body | silent classification, no stop | At go on N workers; non-portable records stay local (P1) |
| A send on a channel made by `channel.new` | yes, at the call of `send` | The receiver may be on another worker |
| A send event on a channel made by `channel.new` | yes, when the event is made (`channel.send_event(c, v)`), not when `sync` starts or when it commits | The same; checking when the event is made makes the stop independent of which branch of a `choose` commits, and an event that holds a value that is not portable can never be built for such a channel |
| The result and the error a child gives to a scope made by `scope`, also inside `both`, `race` and `map_concurrently`, which are built on `scope` | yes, when the child ends | The scope's task may be on another worker |
| The argument of `scope.share` | yes | It goes to the shared host byte transport |
| A join of a Task handle (or a join event) of a child of a scope made by `scope` | no, the result was checked when the child ended | Any task whose chain holds that scope (part 6) may join; another task stops with `task: join outside its scope` |
| A send on a local channel; the result and the error a child gives to a local scope (also in the local forms `local_both`, `local_race` and `local_map_concurrently`, if R adds them); a join of the Task handle of a child of a local scope | no | Every end is on the home worker (P4) |

**P1. Publication (multicore wave).** At go the maker classifies the whole
record described in §3.3, never just the closure. A non-portable record stays
local without error (D8). A portable record is encoded once and put in the
shared queue, and any worker may claim it only while it is unstarted. The maker
uses its retained original if it claims; a remote claimant decodes. Every claim,
cancel and drop has the single owner/transition rule of D8. On one worker the
same pinning outcome follows from using only the original, with no encoding.

**P2. Started tasks never move.** This also pins a task that waits on an async host call: the call resumes on the worker that started it.

**P3. Portable paths check always.** The check is ordinary i code in `i/task` that calls `core.portable` and then `panic("task: value is not portable")`. It runs on one worker too. Its stop message is the same on any number of workers, and it is part of the spec text of `i/task` (track R).

**P4. Local channels and local scopes.** For values that must stay home, such as a user host reference given to a handler task (standard IO resources are Int wrappers, native N9), `i/task` gives local channels and local scopes. A local handle holds the boundary of the task that made it (D5), which it gets from that task's `Scheduler` service when it is made. The boundary is there **only to make the handle non-portable**; it is never used for a suspend (a suspend to it from another task would stop with `control: boundary is not active`, and from the making task would suspend that task, which the scheduler never wants). So the handle is itself not portable, by the general rule, with no flag and no special case. A boundary is a value like any other: it may be stored in data that outlives the body of `control.boundary` (§9.4 only limits `suspend` to active boundaries), also after its task has ended. **A child of a local scope is never remotely claimed**: its task record holds the local scope handle, so P1 classifies it as non-portable. **The Task handle of a child of a local scope, and every join event made from it, hold the local scope's handle too**, so they are not portable either: they can be sent only on local channels, and only a task on the home worker can join them. This holds whatever the child's body captures; `l.go(() { open_socket() })` in a local scope `l` stays home and gives its host reference back on its own worker, to tasks on that worker only.

What a programmer sees: a closure that captures a local channel, given to `go`, simply stays on its worker, and so does every child of a local scope. Nothing else changes. Sending a local handle, or the Task handle of a child of a local scope, on a portable channel stops with `panic: task: value is not portable`, on one worker as on N. So `local_scope((l) { t = l.go(() { open_socket() })  c.send(t) })` with `c = channel.new()` stops at the send, on any number of workers.

**P5. Host requests stay home.** A host request value (host-concurrency.md §6.4, designed by R2) holds an id in the request table of its instance (H1 there). It is not portable (part 3.1). Only tasks on its home worker can use that Request/Ticket to wait, poll or cancel; a send on a portable path stops. Portable Root/Owner operations instead act on group metadata, with host cancellation actions and home-cell reconciliation as runtime §5.1 specifies; they do not expose or move a Ticket. On one worker the same send stops in the same way, so a program cannot see that ids are per instance.

**Why nothing leaks (proof of H).** A non-portable value is made on some worker. It can reach a task in only two ways:

1. In the task's record at `go`: its closure, its kept services or its scope handle. The task is unstarted until it runs. Before publication, P1 walks all of it, so a task whose record reaches a non-portable value is never remotely claimed.
2. By a receive, a join, or a `shared.get` while the task runs. The task is then started, and by P2 it never moves. If it gives such a value to a new child with `go`, the child's record holds it, and case 1 applies to the child.

A value leaves a task in only three ways, and each keeps H:

- A send on a channel made by `channel.new`, a result or error given to a scope made by `scope`, or `share`: P3 stops it unless it is portable.
- A send on a local channel: the sending task holds the local handle. By cases 1 and 2, a task that holds a non-portable handle is on that handle's home worker, so the value stays there.
- A result or error given to a local scope: the child's record holds the local scope handle, so by P4 the child was never remotely claimed and runs on the scope's home worker. The result is read by the scope's own task, or by a join of the child's Task handle. That handle holds the local scope handle (P4), so by cases 1 and 2 every task that holds it is on the home worker.

A join of the Task handle of a child of a scope made by `scope` reads a result or error that P3 checked when the child ended, so it is portable and any task may read it.

So a remotely claimed task holds no local handle of its old worker and belongs to no local scope of it. Every local handle it gets later is made on, or received from, a local path of its new worker. H holds.

### 3.4 Function identity

- §11.4 is kept as it is. One sentence is added: a copy between workers is the same value.
- **Now:** nothing changes in `==`. One worker never copies, so `ref.eq` is exact.
- **Multicore wave** (the layout is fixed now and pinned by a test):
  - `$clo{n}` today is `(tag 7) (i32 0) (i64 arity) (f64 0) (bytes null) (captures) (limbs null) (call null) (funcref)` (`emit/mod.rs:228`). Track R sets bit 0 of field 1 for a closure whose captures are patched (part 3.6).
  - It becomes `(tag 7) (i32 flags | arity << 1) (i64 id) (f64 0) …`. `inspect.rs:262`, the tag table in `runtime/mod.rs:46`, `closure()` and its test change in the same commit. At c69515c no other code reads field 2 of a tag-7 value. The other field-2 readers (`values.rs:185`, `:197`, `:351`, `:361`, `bitarrays.rs:23`, `inspect.rs:151`, `emit/mod.rs:22`) are for tags 5 and 9 and sit behind a tag test or are called only for those tags; MC-I greps again.
  - Field 3 stays free. The readers of field 3 are `$float_get` (`values.rs:51-52`), which has no tag test of its own and is called from `core.rs`, `abi.rs`, `bitarrays.rs`, `bigint_conversions.rs`, `float_format.rs` and `emit/mod.rs:514` only on values the checker types as `Float`, and the readers `values.rs:178`, `:337` and `emit/mod.rs:34`, which sit behind a tag-2 (Float) test. A unit test pins this list, so a later use of field 3 by another tag cannot change `==` or `hash` without the test failing.
  - **Ids.** Constant function values (top-level functions `$c{f}`, constructors, built-ins, library functions, each `@external`) get static ids 1 to K from the compiler's numbering, the same in every instance. So do the closures that the runtime itself makes: at c69515c that is the `Assert` closure of the root environment, a fresh `$clo1` around `$entry_assert_fail` made in the init code (`emit/mod.rs:802-811`) and kept in slot `ASSERT_SERVICE` of `$root_env`. It is neither a `$c{f}` nor an `$f{i}`, and since `keep_env` keeps every bound service, almost every task record holds it. MC-I makes it a `$c` constant (a global made once) with a static id, puts `$entry_assert_fail` in the function table, and greps the runtime for every other `struct.new $clo` to do the same. Start-up takes ids from K+1 upward, the same on every worker because start-up is the same (D7). At run time worker w (at most 255 workers) takes `((w + 1) << 56) | n` from a plain per-worker counter: no atomics. The counter counts every closure and key made, not live ones, so it must not run out in a long run: 2^56 ids last more than 20 years at 10^8 per second on one worker, so no real run reaches the end. If MC adds the mutable field of D9 to closures, ids can instead be given only when a closure is first encoded, which also saves the counter work on every closure. Keys (D13) take ids from the same counter.
  - `$value_compare_i32` keeps `ref.eq` first. For two tag-7 values that are not `ref.eq`, it compares the ids for equality only. Its result for different identity values stays "not equal", with no order (§11.5 exposes none, and no `i/task` API may need one).
  - `hash` of tags 7, 11 and 12 stays the tag alone. A hash by id would make `Dict` iteration order depend on placement and timing.
  - Boundaries (tag 11) and continuations (tag 12) get no id. They never leave their worker (H), so `ref.eq` stays exact.
  - The copy writes a closure as (code index, id, flags, arity, captures). The code index is an index into one function table (an elem segment of every `$f{i}` and of every runtime function that a closure holds, such as `$entry_assert_fail`); a funcref cannot be written to linear memory. A constant closure (static id) decodes to the receiver's own `$c{f}`, so the `ref.eq` fast path still hits. A closure in a cycle is made first with its captures array and the array is filled after, as the lowering does today.
- Trade-off recorded: putting the id in field 3 as `f64.reinterpret_i64` bits was rejected. It needs a bit trick, the bits can be NaN patterns, and it would tie field 3 to two meanings. Adding a field (a `$clo_base` with an extra `i64`) was rejected because it costs 8 bytes per closure.

### 3.5 Top-level values and start-up

- §2.5 says top-level values need no environment, so they never call host services. The only ways a top-level value can be non-portable are: an `@external` that gives a host reference (an opaque custom type, §10.3), and a boundary or continuation made by `control.boundary` during start-up.
- Such a value is a problem even though it never moves. Top-level names are not captures, so `counter = ext_counter_new()` and `bump = () { ext_counter_inc(counter) }` make `bump` a portable constant closure. Each worker's start-up makes its own counter (`reference_id` is per instance, `lib.rs:602`), and a remotely claimed `bump` would increment the remote claimant's counter: `scope((s) { list.map(range(0, 100), (i) { s.go(bump) }) })` would leave less than 100 in the counter that worker 0 reads. A top-level continuation is the same: each worker would have its own copy to resume.
- So every worker runs `ilang_start` (D7), and the run uses N workers only when both conditions hold:
  - (a) **Portable top level.** After start-up on worker 0, the runtime walks every top-level value with the `portable` walk (`ilang_toplevel_portable`, multicore wave). If one is not portable, the run uses one worker. Then `counter` above gives 100, as on one worker.
  - (b) **Pure start-up externals.** A **host external** is an `@external` that is not of the library package `i`. The `@external` declarations of `i/core` (`not`, `to_float`, `inspect`, `hash`, …; `@external("ilang:core", …)` and `ilang:prelude`'s `panic`) are operations of the runtime, which `lower.rs:1379` and the name tables near `lower.rs:120` map to ops; they do not count. The compiler knows statically which host externals the full needs (§2.5) of the top-level bindings **that are not direct values** reach, and records them in the metadata (§B.6, multicore wave). These are the only bindings whose expressions start-up runs; the body of a top-level function such as `main` is not run at start-up, so an external that only function bodies use does not count. A module with such externals runs N workers only when the host declares those externals pure: each call with equal arguments gives an equal result, and calling them again on each worker has no effect that the program or the user can see. Otherwise it runs one worker. This is the same line that `meta.md` §8 draws for macros (the full needs of top-level values that are not a direct value).
- Both are choices made when the module is loaded or started, and they cannot be seen. `docs/performance.md` notes the lost parallelism.
- Per-instance runtime state is set up on every worker as today: label globals, `$constructors_install`, the shadow stack and the site table.

### 3.6 Cycles and the cost of the walk

- Data values are immutable and built from their parts, so they never form a cycle. The only cycles at c69515c come from direct value bindings: the closure is made first, and its captures are filled later with `array.set` on its captures array (`fx.patches` and `Stmt::SetCapture`, `lower.rs:1618-1655`; `emit/mod.rs:413`). So `loop = (n) { … loop(n - 1) … }` gives a closure that holds itself.
- Track R marks such closures: `closure()` sets bit 0 of field 1 (today always 0) when the lowering patches the closure's captures, and gives such a closure's captures array (field 5, a `$values`, which is `(array (mut eqref))`, `runtime/mod.rs:231`, and is already written with `array.set` by the patches) **one more last element, the walk stamp**, `null` when the closure is made. No type changes: `$clo{n}` stays `(sub final $obj …)` (`emit/mod.rs:102`), and code that calls a closure or reads a capture by index does not change, because the stamp comes after every capture. `$core_portable` is static code in `values.rs` and reaches the stamp through `struct.get $obj 5`, which works for every arity, so no per-arity dispatch is needed. Each walk makes one new GC object as its token (a `struct.new` of a small fixed type). When `$core_portable` enters a marked closure, it skips it if the stamp is `ref.eq` to the current token, and otherwise writes the token there and walks the other elements. Tokens are compared by `ref.eq` and a new one is made for each walk, so a stamp left by an old walk can never be taken for the current one, and nothing wraps. The stamp is not a capture: `portable`, the copy (part 9) and any later reader of captures skip the last element of a marked closure, and `==`, `hash` and the text form never look at captures. Only patched closures pay this one array element. Every cycle goes through a patched slot, so the walk always ends, and each marked closure costs O(1), so the walk is linear in what it walks. Only the worker that owns the closure walks it (I4), so the write has no race. A list of entered closures searched with `ref.eq` was rejected: each lookup scans it, so a value holding 10^5 local recursive closures costs about 5·10^9 compares, on every send, also on one worker.
- A DAG is still walked once for each path to a node, as `==`, `hash` and the text form already do today. So `portable` of T15's depth-40 DAG takes 2^40 steps, on one worker as on N, like `x == y` does today for two such values that were built apart. This cost is the same on every worker count, so it is not a placement problem. If MC adds the mark of D9, `portable` uses it too and becomes O(distinct nodes).
- The copy (part 9) must handle the same cycles for correctness, not only to save work.

## 4. Services, the worker loop, and `go`

`keep_env` (D4) is lowered from the environment array that every call already passes (`lower.rs:17`, the `$env` array of `emit/mod.rs:806`). That array holds every service bound at the place, indexed by a global service id, because each environment statement copies the whole array (`$env_with`, `emit/control.rs:49`). The kept function holds that array as it is at the call, with nothing cleared. So it holds exactly the services that are bound at the place when the program runs, as the spec says, and the answer of `portable` does not depend on inlining or on how generic the code around the call is. The kept services are stored as ordinary values (for example a `$values` array of the kept slots and their service ids), so that the generic walk covers them.

The kept function is a real frame with its own re-entry record, `$keep_reenter`, like `$catch_reenter` and `$ensure_reenter`. Under track B's lazy re-entry, which lands before R on lane b, it is a record whose base environment is the kept array (the way a delimiter's record gives its base to walk W), so the records inside it get the kept services and their own environment statements on top. `emit/control.rs:10-13` makes the environment locals again from the environment of the place of `resume`; only environment statements are kept. The record of `$keep_reenter` holds the kept array, so a task that suspends and is resumed by the loop still sees its parent's services (a fake `Clock`, for example), not the loop's. This changes what §9.4 rules 3 and 4 say today: a resumed paused part keeps only its environment statements and gets the rest from the place of `resume`, and final functions get the environment of the place of `cancel`. Part 13 amends both rules: the services kept by a kept function whose frame is in the paused part count like environment statements inside it. Without that, the code after a resume, and the final functions of a cancelled task, would get the loop's services, which may not bind a service the task needs (against §9.2, "getting it never fails at run time").

Rejected: keeping exactly the services of `N`. Inside generic code, such as `go` itself or any wrapper of `keep_env`, `N` is a type variable, and there is no monomorphization in `lower.rs`. The lowering could clear only the named services that `f` does not need, and would keep every slot of the caller's array under a type variable. Then `portable` would give False inside a generic wrapper and True after the wrapper is inlined at O2, against D3. Doing it exactly would need the slot set of every environment variable passed at run time, as an extra argument of every generic function whose environment variable reaches `keep_env`.

The following sketch shows only Scheduler routing. Runtime.md §6.3 supplies
its exact accepted extension: detach Scheduler, Runner and Await before keep_env,
rebind all three from the task's real Scheduler in a retained frame, and keep
Root/Owner unchanged. This is required at go and task.keep; keeping a real parent
Runner/Await would pin every child. The sketch is not the final API or a second
prescription for entry bootstrap:


```i
go(A, E, N) = (body: () -> A ^ E ! N | Scheduler, s ! Scheduler) -> Task(A, E) ! N | Scheduler {
  start = {
    ! Scheduler = scheduler.detached
    control.keep_env((own: Scheduler) {
      ! Scheduler = own
      r = task.unwind_inside(body)
      r
    })
  }
  s.spawn(start)
}
```

- **The call after `! Scheduler = own` is not a tail call, on purpose.** §11.6 says an environment statement does not change tail positions, and track B keeps a known gap (`lazy-reentry.md` D16, section 7a, open question 6): an environment statement in a frame that was left by a tail call is on no frame record, in eager and in lazy. If the kept function ended in `body()`, its frame would be gone; after the first suspend, the outermost record would be `$keep_reenter`, whose kept array has `Scheduler = scheduler.detached`, walk W would give that array to the records of `body`, and the second scheduler call (`s.go(() { task.yield()  task.yield() })`) would stop with `panic: task: no scheduler`. Binding the result and giving it back (`r = …  r`) keeps the frame, so its statement is on its record. Track R must check in the WAT, at O0 and O2, that this call is not turned into a tail call (if the optimizer does that, the sketch uses a form it keeps, such as the `keep(…)` call that the cases of `lazy-reentry.md` part 13 use). When the gap of section 7a is fixed, the plain form works too, but R does not wait for that fix. T24 checks a second and a third scheduler call after resumes.
- `task.unwind_inside(body)` (a private helper of `i/task`, ordinary i code) runs `body` inside an inner boundary of the task, so that a cancel can unwind the task while the task's own boundary stays active (part 6, Cancellation).

- `scheduler.detached` is a top-level record of constant functions that stop with `panic: task: no scheduler`. It is portable. Binding it before `keep_env` puts it, not the parent's scheduler, in the kept array, so the parent's scheduler boundary is never kept or captured. Inside, `! Scheduler = own` binds the real one, so `detached` is never called.
- `s.spawn(start)` makes the task record (part 3.3), with the handle of the scope `s`.
- The worker loop calls `start(own)` inside the task's new boundary (D5), with the `Scheduler` whose suspends go to that boundary. The environment statement is inside the task, in a frame that is kept, so it stays with the task's paused part (§9.4 rule 3).
- Kept services may still be non-portable, for example a generator's `Yield` service, or the `Assert` of `assertion.catch`, which hold a boundary. Every service bound at the place is kept, also one that `body` does not need, so a task started under such a service is pinned by P1. That costs parallelism only, and it cannot be seen. When `body` uses that service, suspending to that boundary from the task stops with `control: boundary is not active` on any number of workers, because the task runs from the loop (D5), and the loop does not run inside that boundary's body.
- **Runs and nested runs.** A **run** is one call of `task.run`: one loop, its tasks, and the handles they make. Nothing stops i code from calling `task.run` where a run is already going on, and the type of `task.run` cannot forbid it. So the rules are these, and they are the same on every host and every worker count:
  - **Which run uses N workers.** Only the run at the program's entry (open question 4) may use more than one worker. Every other run is a **nested run**: a one-worker FIFO loop on the stack of the code that called `task.run`. Its tasks are never remotely claimed. When it is called from inside a task, it runs inside that task's boundary body, so the whole nested run is part of the calling task, which is started and never moves (P2).
  - **The run service.** `task.run` needs one service of `i/task` (its name is R's; here `Runner`), and that is how it learns where it is, with no test for "is a service bound". Inside every task, the scheduler binds `Runner` to one that belongs to that task. The entry supplies the initial Runner from inferred needs (runtime D5); task.top(seed) uses that current Runner and replaces only its seed, retaining route and prefix. A run takes three things from `Runner`: its **path prefix**, its **root seed**, and its **wait**.
  - **Paths.** At the top the prefix is `[]`. In a task, it is the task's path plus the index of this `task.run` among the task's `go` calls and runs, counted together, so the paths of two runs never meet, and two sibling tasks that each start a nested run get different paths and different generators (part 5).
  - **Waits.** When a run has no task that can run but its owned-plus-borrowed program waitable set is nonempty, it waits through `Runner`. At the top, that is the one root wait of host-concurrency §6.4 (status 1). In a task, the nested run registers its owned group interests and explicitly borrowed local tickets with the outer run and suspends **only its calling task** to the outer `Scheduler`, so the whole nested run pauses as one task, and the outer run's other tasks go on, on one worker as on N; the outer run resumes the calling task when one of those requests ends. So a nested run that waits for a host flag that another outer task sets ends on one worker too. A nested run never returns status 1 from the caller's stack.
  - **Deadlock.** Use runtime §6.4's owned-plus-borrowed waitable set: outstanding work in every group of the run's stored program Root, union Waiting tickets explicitly awaited by its tasks, including borrowed outer-root requests on the same worker, deduplicated by identity; private group-observer tickets expand to their outstanding members (runtime §5.1). With no runnable task or pending program delivery and an empty set, stop with `panic: task: deadlock`; unrelated outer-root work does not prevent it. A borrowed outer request can wake a nested run. Observe owned groups by counts/revisions, not by returning foreign Tickets (runtime §5.1).
  - **Stateful handles belong to their run; events are descriptions.** Every channel, scope, Task and `Shared` handle holds its run key. Using those stateful handles in another run stops with `panic: task: handle of another run`, also when portable. An Event has a fresh identity key and need not have a creating run: always/never/map/choose need no Scheduler. At sync, validate the run of every leaf that refers to a channel, Task or scope, including losing alternatives; a pure leaf has no run restriction. Payloads and callback captures determine event portability, including local witnesses; there is no blanket portable-event promise (runtime §6.1/§6.4). So `task.run(() { scope((s) { c = channel.new()  s.go(() { c.send(1) })  task.run(() { c.receive() }) }) })` stops at `c.receive()` on any number of workers. Values that are not handles pass into and out of a nested run as captures and as its result, as with any function call.
  - **Boundaries.** A nested run inside a generator body at the top (`collect(() { task.run(() { scope((s) { s.go(() { yield(1) }) }) }) })`) runs inside that boundary body, so by §9.4 rule 1 a `yield` from its tasks succeeds and pauses the whole nested run. That is the same on every host, because that run is not the entry run and uses one worker. A kept function that an outer task made (D4), called inside its own nested run, suspends the outer task with the nested run inside it, which is also the same everywhere. Track R documents both.
- Host services given at an entry point are records of constant functions (static ids), so they are portable, and a remotely claimed task uses the remote claimant's own imports. The root environment's `Assert` closure must be a constant with a static id too (part 3.4); at c69515c it is not. §9.7 gets the rule that a host service behaves the same whichever worker calls it.

All shared mutable scheduler state (the published queue, channel state, wake-ups, cancel requests and stop cell) is behind the one `Scheduler` service. Worker-local values of many types are kept as Keyed (D13); shared transport carries encoded bytes/stable ids, never another worker's GC Keyed objects (N8). In the M:1 version it is written in i on `i/control`. The multicore wave replaces its implementation without changing the `i/task` API.

## 5. Random, paths and handle ids

- `task.path()` gives the task's path: a list of combined `go`/nested-run indices from the root task. The path is portable data. It does not depend on time, worker or queue claims.
- `task.path()` needs no suspend: the `Scheduler` that the loop gives a task holds the task's path as data, so reading it is a field read.
- i has no mutable values (§11.1), so a per-task draw counter would have to live in the scheduler loop, and every draw would be a suspend to the scheduler; `i/random` would need `! Scheduler`, and code with no scheduler, such as `main = () { … }` with no `task.run`, a top-level value or a macro, could not draw. So Random is a **value**, passed explicitly: a generator is immutable data (the pinned SplitMix state and tagged derivation history of runtime §4); `random.int(g, 10)` gives a number and the next generator, and `random.split(g)` gives two derived generators under that published algorithm. `i/random` does not need `i/task`.
- `task.random()` gives the generator of the current task, a pure function of the root seed and `task.path()`. So a child's numbers do not depend on when or where it runs, and a `go` does not change the parent's generator. Calling `task.random()` twice in one task gives the same generator; code that needs more splits it. `go` itself does not know about Random.
- **The root seed** is part of the run service (part 4). At the top, the program gives it: `task.top(seed)` takes a generator, made from a host entropy service when the program has one, or from a fixed seed (`random.from_seed(n)`), which needs no service. The contextual entry default is random.from_seed(0) (runtime D5), and task.top(seed) replaces only that seed while preserving the Runner route/prefix. There is no automatic entropy draw, so the type of `task.run` needs no host service, and a macro, which has no host services (`meta.md` §8), gives a fixed seed and gets the same expansion on every run. A nested run in a task takes the seed of its outer run, and its paths start with the calling task's path (part 4), so its generators differ from every other task's.
- **Split and paths never meet.** A generator is a key and a **stream**: a list of steps, where a step is either a path step `P(i)` or a split step `S(i)`, and a path step is never equal to a split step. `task.random()` at path `[i1, …, in]` gives the stream `[P(i1), …, P(in)]`; `random.split(g)` gives the streams of `g` followed by `S(1)` and by `S(2)`; numbers come from the pinned published SplitMix split tree in runtime.md §4, which replaces the earlier counter-based PRF sketch. So the first half of `random.split(task.random())` in the task at path `[]` is the stream `[S(1)]`, and the child at path `[1]` has `[P(1)]`: they differ. Different tagged derivation addresses remain distinct histories; no promise of disjoint outputs or perfect statistical independence is made. Runtime.md §4 fixes the exact algorithm and encoding.
- Channel, scope and task handles are made of a path-based id (the path of the task that made them and a counter in that task) and the keys that type their state (D13). So `==` on handles is "the same channel" and survives copies, and nothing in them depends on the worker except key ids, which only `==` sees.

## 6. Scopes, cancellation and errors

- A scope returns only after all its children have ended, including children that are being unwound.
- **Chains.** The **chain** of a scope `r` is `r` followed by the chain of the task that opened `r`, as it was when `r` was opened. The **chain** of a task, at a moment, is the scopes it has opened whose bodies it is still running (innermost first), followed by the chain of the scope it was started in (empty for a run's root task). A task's chain changes only when that task itself enters or leaves a scope body, so it is fixed by the task's own steps, not by time or placement. Every scope in a task's chain has not ended: the scopes it has opened are still running their bodies, the scope it was started in waits for it, and each scope in that scope's chain was in the opener's chain when it was opened, so by the same argument it cannot end before the opener leaves it. The scheduler keeps each task's chain as a list of scope ids, so the check is a list search.
- **Who may `go`.** `s.go` may be called only by a task whose chain holds `s`. Otherwise it stops with `panic: task: go outside its scope`, whether or not `s` has ended. So a `go` never meets an ended scope. Examples that stop: `h = scope((s) { s })  h.go(() { 1 })` (after the body returns, `s` is no longer in the root task's chain); `scope((u) { scope((s) { u.go(() { task.yield()  s.go(() { 1 }) }) }) })` (the `u` child was started in `u`, whose chain does not hold `s`); `scope((outer) { c = channel.new()  outer.go(() { h = c.receive()  h.go(() { 1 }) })  scope((s1) { c.send(s1) }) })` (the same). Each stops on any number of workers. A scope handle is portable and may be sent anywhere, but only tasks inside the scope may use it to start tasks. A `go` on a scope that is cancelling makes an unstarted task that is dropped at once; whether a `go` comes before or after the cancel is part of the order of D1.
- **Who may join.** A join of the Task handle of a child of `s` (and a `sync` of a join event made from it) may be done only by a task whose chain holds `s`. Otherwise it stops with `panic: task: join outside its scope`. A joiner still cancellable by `s` is unwound before observing a dropped/cancelled outcome; a shielded joiner, including protected final work, stops with `panic: task: join of a cancelled task` immediately when drop/cancellation is accepted, without waiting for target cleanup (runtime R4a). If R later adds a way to cancel one task while its scope goes on, a join of that task must complete with a value its type shows (for example a `Cancelled` case); no join may be left waiting.
- **Errors.** When a child fails, the scope cancels the other children. When all have ended, the scope gives the error of the failed task with the lowest path (D10) among the scope's body and its children; the body comes first. Later errors are dropped. Error types stay unions; there is no aggregate error type, so inference does not change. Every error given to a scope made by `scope` is portable (P3). For `scope((s) { s.go(() { s.go(() { fail.raise(A) })  fail.raise(C) }) })`, if both fail, the error is `C` (path `[1]` comes before `[1, 1]`), whichever failed first. Which children fail before the cancel reaches them depends on the order of D1: for `s.go(() { task.yield()  fail.raise(A) })` and `s.go(() { fail.raise(B) })`, one FIFO worker gives `B`, and N workers may give `A`. The `i/task` docs say so.
- **Results.** `both`, `race` and `map_concurrently` are ordinary i code built on `scope`, so they are portable paths: each result and error of their children is checked when the child ends (P3), and a result containing a user host reference stops with `panic: task: value is not portable`. Standard File/Listener/Stream/Child values are portable nominal Int wrappers (native N9); returning them through these combinators succeeds. R may add local forms built on `local_scope` (`local_both`, `local_race`, `local_map_concurrently`), which do not check and whose children stay home (P4); without them, the program writes the local scope itself. `both` and `map_concurrently` give results in argument order. `race` gives the first result that commits; timing chooses, as with a CML `choose`. With one worker the FIFO scheduler makes it fixed, but that is not a promise for N workers.
- **What a cancel of a scope reaches.** When a scope `s` is cancelled (a child failed, or a task whose chain holds `s` is itself cancelled), the cancel reaches every child of `s` **and the body of `s`**. The body is unwound back to the `scope` call: `scope` runs its body under its own boundary in the body's task, and the cancel suspends to that boundary and cancels the part up to it, so the body's final functions run and the body gives no value; `scope` then waits for its children and gives the error of D10. A cancelled task cancels every scope it is running a body of, so the cancel goes down the whole tree inside `s`. A cancelled body is not a failure; it does not take part in the choice of the error. So `scope((s) { s.go(() { fail.raise(A) })  t = s.go(() { 1 })  t.join() })` gives error `A` on the one-worker FIFO scheduler: child `[1]` fails, `t` is dropped, the body waiting in `t.join()` is unwound, and nothing waits forever.
- **Cancellation** is cooperative and goes to the task's home worker. An unstarted task is dropped; it has no final functions. A started task is cancelled when the cancel reaches it: when it is paused (waiting or runnable) at that time, the loop does not give it the answer it waits for and cancels it as the next point says; when it is running, at its next suspend point. When the cancel reaches a task is a step in the order of D1: on one FIFO worker it reaches every paused task at once, and on N workers it reaches a task on another worker at that worker's next poll point. A task that is being unwound is not cancelled again, and its final functions run to their end, with the scopes they open, which that cancel does not reach.
- **How a started task is cancelled.** The loop never calls `control.cancel(k)` on the task's continuation `k`: under §9.4 rule 4 the task's boundary `b` would stay not active while the final functions run (`$control_cancel` and `$cancel_run`, `emit/control.rs:270-300`, never set field 8 of the boundary), and a final function that uses `i/task` (a send, a receive, a join, a sleep, an async host call through the scheduler) would suspend to `b` and stop with `control: boundary is not active`. Instead the loop **resumes** `k` with a cancel answer. `b` is then active again. The `Scheduler` call inside the task that sees the answer suspends to the task's inner boundary (made by `task.unwind_inside`, part 4) or, for a scope body, to that scope's boundary, and its handler calls `control.cancel` on that inner part only. So the final functions run with `b` active, and they may suspend to `b`; `$cancel_run` already catches a capture through it and gives it the record `$cancel_reenter`, so the rest of the unwind resumes later. They get the environment of §9.4 rule 4 as part 13 amends it: the services kept by the task's kept function and the environment statements inside the task, not the loop's services. This is all ordinary i code on `i/control`.
- **Stops.** A stop on any worker closes the run's start/claim/delivery gate and records one message under the lock. Stopping invokes no i finals. Every user import checks the stopped-context gate, so spinning native workers cannot start new host effects while reaping delays process exit. Native's independent controller tears down host work/resources before process exit; it does not join a spinning Wasm worker. A continuing host uses stop-only epoch traps in that run's interruption domain, wakes idle reactors and joins workers, then returns the first stop (native N12). Epochs do not interrupt synchronous host code; multicore hosts must supply stop-aware synchronous imports or choose one worker/process isolation. Child cleanup follows runtime §8.1, including reaping before native exit.
- **Deadlock.** Use the program waitable set in part 4/runtime §6.4: all outstanding work owned by the run's program Root plus explicitly awaited borrowed Waiting tickets, deduplicated by identity, expanding group-observer tickets to their underlying members (runtime §5.1). Driver wake requests use a separate ordinary control Root/Owner; they are closed with the driver and never bound into user tasks or exposed as borrowed waits. General Root/Owner accounting distinguishes them by the roots selected by the driver, never by module/import name. With no runnable, claimed-but-unstarted or running task, no pending program completion/group-notification processing, and an empty program waitable set, record `panic: task: deadlock` under the shared lock. Start/claim/cancel/answer transitions and local work counts participate in that snapshot; an answer moves from the request count to pending delivery work atomically. Control requests and wake-only deliveries are excluded from that progress predicate. All actual outstanding requests still count for ABI status 1, so wake requests preserve H4 without masking deadlock. On one worker no wake is needed and the same predicate applies. In a macro without host services, nothing runnable and an empty waitable set is a deadlock.
- **Review wake trace.** Worker 0 is idle with one control wake request. The last runnable task on A executes sync(never()); program request count is zero. Once that registration finishes, runnable/running/claimed/pending-program-delivery counts are all zero, so the locked snapshot records deadlock even though the control count is one (or two if A also installed a wake). Stop closes the control roots and wakes the drivers; it does not wait for those wakes to justify progress. If an owned or explicitly borrowed program request is pending instead, its count prevents the stop until it settles.
- **Idle.** A worker with nothing to run, while others still work or requests are outstanding, waits. By host-concurrency H5, a host answers a request only through `ilang_resume:…` after the instance has returned status 1 or 2, never while it is inside Wasm. So a worker that has **outstanding requests of its own** (its pinned tasks wait on them, P2) must never park inside Wasm: it would never see its answers, and a task that waits on a 10 ms sleep would hang until something else woke the worker. The rule:
  - A worker with outstanding requests of its own idles only by returning status 1, on every host. Its host then waits for either an answer to one of its requests or a **wake** from another worker, whichever comes first. The wake is a request-mode ordinary host service added in the multicore wave (native N8): it resolves when another worker writes the worker's wake word (`Atomics.waitAsync` in JS; in wasmtime, a wait on that word in the host thread, raced with the reactor). The worker keeps one wake request outstanding while it idles, so status 1 is never given with no outstanding request (H4).
  - A native worker with no requests may park in the host queue wait; a browser worker may use memory.atomic.wait32, except browser worker 0, which never blocks: it idles like a worker with requests, with only the wake request outstanding.
  - A one-worker run never uses the wake request: on one worker, idle waits on the owned-plus-borrowed program set from part 4, and an empty set with no pending program delivery means deadlock.

## 7. Channels and CML events

- A channel is a handle as in part 5. `==` means "the same channel".
- **Lifetime.** A channel **belongs to a scope**: the first scope in the chain of the task that made it (part 6), or the run itself when that chain is empty. Its state (buffers, waiting senders and receivers, sync records) lives until that scope ends and is then freed, in M:1 from the scheduler's `Keyed` table and in MC from the scope's host-transport region, as for `scope.share`. A send, a receive or a `sync` on it after that scope has ended stops with `panic: task: channel is gone`, in every build. A task that is waiting on it when the scope ends gets the same stop. Whether a step comes before or after that end is part of the order of D1, as for `shared.get`. So `r = scope((s) { channel.new() })  r.send(1)` stops on any number of workers. *Reason:* i has no weak references and the shared memory has no GC, so "a channel lives while a handle to it is reachable" cannot be kept in M:1 or in MC; tying the state to a scope gives a bound that both can keep, and fixing it now means MC changes no program. The cost: a long-lived scope that makes a channel for each request keeps every one of them until it ends. The `i/task` docs say to make such channels inside a scope per request (`scope((r) { c = channel.new() … })`), which frees them when the request is done; `docs/performance.md` says so too. A local channel follows the same rule.
- Events are values. `choose` and `map` are pure. `sync` is the only commit point, and exactly one event of a `choose` commits. `map` functions run in the syncing task after the commit, never in the partner task. Nack and guard are not in v1; they are kept for later.
- **M:1 now.** Channel state lives in the scheduler state, as `Keyed` buffers and waiting continuations (D13). A commit is trivial on one thread.
- **Multicore wave: one lock (native N8).** Channel/scope state, registrations,
queue claims, cancellation, Root/Owner membership and cross-worker request
counts, and the stop cell live in the runner's transport
behind one lock. A sync transaction validates all leaves (run, scope, chain),
then registers one identity with source-ordered alternatives. Choose the first
ready alternative and oldest eligible partner, never two leaves of the same
registration. In the same critical section mark both registrations committed,
remove every losing leaf, transfer one payload and queue both wakeups. A cancel
before commit unregisters all leaves; after commit it cannot roll back a message.
No i callback, encoder/decoder, continuation, OS wait or Wasm call runs under
the lock. Prepare portable payload bytes on the sender before entering it; a
local payload remains in its owner's heap and has only a worker-local token in
the transaction. `map` callbacks run only after commit in their syncing tasks,
including the same cancellation-priority rules as runtime §6.4. Run/channel/scope
liveness must be rechecked after preparation and before commit. A winning local
handoff uses its retained reference; a remote receiver decodes the bytes.
- **Implementation gate.** Shared payloads/typed keys are encoded bytes and stable
ids; continuations and Keyed i heap objects stay in worker-local typed tables.
The i scheduler computes descriptions and applies returned decisions. The runner
supplies general locked byte-state transactions and async wake/wait services,
not a Rust task executor. A lock, queue and wait alone are not a complete typed
interface: MC-D must type-check the operation signatures and demonstrate two
worker rendezvous, cancellation and no callback under the lock before MC-I.
All request-mode fields use ordinary service imports; no new v3 runtime import,
status or host-called export is assumed. Changing that requires an ABI version
review under host-concurrency.md. One worker retains runtime §6.4's FIFO commit.

## 8. `scope.share`

- `scope.share(value) -> Shared(A)` belongs to the scope that makes it. `value` must be portable (P3).
- A `Shared(A)` handle is (scope id, index, key), so sending it costs O(1).
- `shared.get(s)` gives the value. On N workers, the value is encoded once into a scope-owned region of the host byte transport, and each worker decodes it once and caches it for the life of the scope. Values from two workers are `==` (content plus ids, I1).
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

**wasmtime (multicore wave).** One Store/instance per executing worker, with
private GC and linear memory. Native bytes, queues, shares, channels and wakeups
live in runner memory (native N11); no shared Wasm allocator or threads feature
is required for that transport. Each worker has its local typed continuation
and request tables. Host service state and the resource table are shared, with
short locks and no callback under them. Outstanding requests idle via status 1
and the general wake service, raced with reactor readiness. Continuing hosts use
an entry-context-specific epoch interruption domain, shared by its nested scheduler runs; one run's stop cannot trap another
run on a shared Engine. Native N12 fixes setup, re-entry gates and teardown.

**Browsers (multicore wave).** N workers require crossOriginIsolated,
SharedArrayBuffer, an imports module URL and a stop-safe dedicated-worker entry
route (native N12); otherwise one worker, with the same meaning (D12). Post the
module and shared memory to dedicated Web Workers for every executing instance,
including worker 0. The calling/main thread is the host controller, which can
terminate all workers without waiting for Wasm poll points and tear down shared
host resources. The earlier worker-0-on-main route cannot stop a pure infinite
loop there and must not enable multicore. Synchronous one-worker host exports
retain runtime D5's callback contract; no universal async entries are introduced.

- **Imports.** JS functions cannot be posted to a Web Worker, so the main-thread `imports` object of today cannot serve worker k > 0. The SDK gets a new option for multicore runs: an **imports module URL**, an ES module that each Web Worker imports to build its own imports object. A host function that needs the main thread (the DOM, for example) is written there as an async function that posts to the main thread and waits for the answer as a host request; it is never synchronous. A synchronous `@external` that requires main-thread execution is not proxied by this design; use an async service or one worker. All synchronous imports, host future polls and destructors must satisfy native N12's stop-aware host rule. So a host that gives only main-thread functions, with no imports module URL, runs one worker. A host reference made on one thread stays there (H), so it never needs to cross as an `externref`.
- **Worker 0 never blocks.** It never uses `Atomics.wait`. Its scheduler idles only by returning status 1 with an outstanding request (part 6, Idle), so it returns to the event loop.

**Host rule (spec §9.7, §B).** A host service behaves the same whichever worker calls it. Host-side state that a service changes is shared by all workers (one Rust value behind an `Arc`, or state on the main thread). The number of workers is the host's choice.

## 11. Records and macros

- **Records** (`records.md`, tag 14). `ref.eq` on labels is only a fast path (I2). Width subtyping hides fields, which is one more reason the check is dynamic: `portable` and the copy look at tags, never at types. Records that hold functions (codecs) keep `==` across copies through ids. `i/task` needs no label-generic callbacks, and none of its functions has label parameters (records L5). Any later shape id or inline cache must stay out of the copy format, `==` and `hash`.
- **Macros** (`meta.md` §8). Macro programs run in the compiler, single-threaded, on exactly one worker with the FIFO scheduler and no clock unless a fake one is given. So `i/task` in a macro gives the same expansion on every machine. A deadlock in a macro stops with `panic: task: deadlock` (part 6).
- **Syntax values at run time.** `Syntax` values exist at run time in two kinds of programs: macro programs, and tests under `i test`, where a quote gives a value (`meta.md` §8). `Syntax` has no constructors, and its form is the compiler's (`meta.md` §8: "It is not seen in i"). This note does not fix that form. Instead §11.9 says directly that **a `Syntax` value is not portable**, whatever its form, as it does for boundaries and continuations. Then `portable` shows nothing about the form, and its `==` stays as `meta.md` §3.1 says. `meta.md` §8 gets one sentence that points at this rule (part 14). A task may pass `Syntax` values on local channels and local scopes and keep them in its own data. Sending one on a portable path, or giving one as the result of a child of a portable scope, stops with `panic: task: value is not portable`, in a macro and in a test, on one worker as on N. Giving `Syntax` ids so that it can be copied is a later, compatible change (open question 8).

## 12. Future architecture and proposals to track

1. **Now (R):** M:1 scheduler in i, with every rule of this note. `core.portable`, `control.keep_env` and keys (D13) land.
2. **Wave 5 (MC):** N instances, native runner bytes or browser shared memory, one queue encoded at go, silent local pinning, sharing-keeping copies, scope.share regions, one-lock CML, ids in `$clo` field 2, quiescence detection, the imports module URL in JS.
3. **Later:** with shared-everything threads, immutable data gets shared types (`$obj`, `$values`, `$cons`), copies go away for it, ids allow direct sharing, and boundaries and continuations stay per thread.
4. **Later:** with Go-style per-frame resume, frame records that carry their point index could be encoded, so started tasks could move.
5. **IO values now; arbitrary host references later.** Standard IO resources
are portable private Int wrappers (native N9, runtime §7/§8). They need no host
reference transfer; the portable-scope echo server in runtime §9 can use more
cores. User-defined opaque host references still obey H and pin their holders.
Any future explicit transfer of those references must define its own equality
and ownership rules; it is not needed for standard sockets.

Proposals to track:

- shared-everything threads (shared structs and arrays, thread-local globals, `thread.spawn`); wasmtime support for shared structs;
- stack switching (WasmFX), for cheaper continuations;
- JSPI, for browser host calls;
- component-model async and WASI 0.3 (wasi-threads is deprecated, so the WASI target stays single-worker);
- `Atomics.waitAsync`, for nonblocking browser wake delivery;
- memory-control and custom page sizes, to give `scope.share` memory back;
- COOP/COEP `credentialless`, which makes isolation easier to deploy.

A request to **track B now**: frame records should carry the point index `p` next to `(ref.func $r{p})` (or the module should keep a `$points` table), so that moving a started task stays possible later without changing `emit/control.rs` again.

## 13. Replacement spec text, section by section

No section is renumbered; new content is added as subsections or sentences. The text lands with track R, together with `core.portable`, `control.keep_env` and keys, after A2, so it is in the new syntax. The parts marked *(multicore wave)* land with MC-I.

**§2.5 Top-level bindings.** After "Its value is worked out once, when the program starts, before any entry point runs (§B.1).", add:

> When the program runs on more than one worker (§11.9), each worker works out the top-level values in the same way, and they are the same values on every worker. A program whose top-level values are not all portable (§11.9) runs on one worker.

**§4.2 Built-in types.** After "The types `Boundary` and `Continuation` are in §9.4.", add:

> The types `Key` and `Keyed` are in §9.8.

**§4.5 Subtypes.** Add to rule 7:

> `Key(A)` (§9.8) does not change.

**§9.2 Environment and services.** Add after the bullet "A service is a normal value. …":

> - `control.keep_env(f)` (§9.4) gives a function that, wherever it is called, calls `f` with the services that were bound at the place of the `keep_env` call when it ran. It keeps every service bound there, also those that `f` does not need. An environment statement inside `f`, as in `(x: S) { ! T = x  body() }`, gives `T` its own value there as usual.

**§9.4 Control.** Add a row to the table:

> | `control.keep_env` | `(f: (x: S) -> A ^ E ! N) -> { (x: S) -> A ^ E } ! N` |

Add to rule 3:

> The services that a function made by `control.keep_env` keeps count, for the code inside that function, like environment statements inside the paused part: when a paused part that holds such a function's frame is resumed, that code still gets the kept services.

Add to rule 4:

> The same holds for final functions inside a function made by `control.keep_env`: they get the kept services, and the place of `cancel` gives only the services that are not kept.

Add rule 7:

> 7. A boundary and a continuation stay on the worker that made them (§11.9). They are not portable.

**§9.7 Host services.** Add a bullet:

> - A function of a host service does the same thing whichever worker calls it (§11.9). An async function goes on in the task that called it, on the same worker. A host request (when R2 adds it) stays on the worker that started it and is not portable.

**§9.8 Keys (new).** A new subsection for the module `i/key`, after §9.7:

> `Key(A)` and `Keyed` are built-in types. They are not custom types: their values are made only by `key.new` and `key.wrap`. A key is the same only as itself (§11.4). Two `Keyed` values are equal when their keys are the same and their values are equal. `Key(A)` does not change (§4.5 rule 7). A `Keyed` holds its value (§11.9).
>
> | Function | Type | What it does |
> |---|---|---|
> | `new` | `() -> Key(A)` | A new key, the same only as itself |
> | `wrap` | `(k: Key(A), value: A) -> Keyed` | The value, marked with the key |
> | `unwrap` | `(k: Key(A), x: Keyed) -> Lookup(A)` | `Found(v)` when `x` was made with `k` from `v`, and `Missing` when not |
>
> `Lookup(A) { Found(A) Missing }` is a custom type of `i/key`, not a built-in name.

(The key text waits for G2's answer to open question 7; the names are track R's.)

**§10.1 External functions.** Add:

> An external function that is not of the library package `i` and is called while top-level values that are not direct values are worked out may be called once for each worker (§11.9). The program runs on more than one worker only when the host says that each such function gives equal results for equal arguments and that calling it again has no effect that can be seen.

**§11.1 Values.** The list of values becomes:

> The values are integers, floats, strings, codepoints, bit arrays, tuples, lists, values of custom types, functions, boundaries, continuations, keys, keyed values, and host references.

**§11.4 Equality.** After the bullet on functions, boundaries, continuations and host references, add:

> - Each call of `control.keep_env` (§9.4) makes a new function, the same only as itself. A key (§9.8) is the same only as itself.
> - A value stays the same value when the program copies it from one worker to another (§11.9). So a function given to another task is the same as the function that was given.

**§11.7 Stops.** After the paragraph on the stop message, add:

> When the program runs on more than one worker and one of them stops, the whole program stops. The host gets one stop message, that of the first stop.

**§11.8 Text form of values.** Add a row after "Boundary, continuation":

> | Key, keyed value | `//key`, `//keyed` |

and add under the table:

> The text form of a function, a key or a keyed value never depends on which worker made it or holds it (§11.9).

**§11.9 Workers (new).**

> A program may run on one or more **workers** at the same time. Which worker does which part of the work cannot be seen. Tasks that run at the same time take their steps (sends and receives, joins, host calls, failures and stops) one after another, in an order that the program does not fix; this order may differ from run to run and with the number of workers. Every run gives a result that some order of the steps gives, and each step does what it would do if the steps were taken one at a time in that order. The number of workers only changes which order happens. So anything that depends on the order of steps may change, for example which sender a receive pairs with, which event of a `choose` commits, whether a step comes before or after the end of a scope, which tasks fail before a cancel reaches them, which of two stops is first, and the order of output. A program whose results do not depend on the order of its steps gives one result. Running out of memory or stack (§11.7) is not covered by this rule. The host chooses the number of workers.
>
> The **captures** of a function made by a function expression are the local names that its body uses and that are bound outside it, after macro expansion; a name counts when it is written in the body, also in a part that never runs. Top-level names are not captures. A function made by `control.keep_env` (§9.4) has the captures of its argument and the services it keeps. Other functions have no captures.
>
> The values that a value **holds** are: the parts of a tuple, a list, a value of a custom type or a record (§11.4); the values of the captures of a function; and the value in a `Keyed` (§9.8). `==` does not look at what a function holds (§11.4). A value is **portable** when it is not a boundary, a continuation, a host reference, a host request or a `Syntax` value (`meta.md` §8), and every value it holds is portable. A value may hold itself, through a local function that calls itself; only the other values it holds then count.
>
> A value that is not portable never goes to another worker. Values may be copied between workers; a copy is the same value (§11.4).

**§12 Built-in names.** Add a row after `Boundary` `Continuation`:

> | `Key` `Keyed` | Built-in types made by `i/key` | §9.8 |

and add `i/key` (§9.8) to the list of modules in the last sentence.

**§13 The module `i/core`.** Add the rows:

> | `portable` | `(value: A) -> Bool` | `True` when the value is portable (§11.9) |

The `hash` row: "Which number is not fixed." becomes:

> Which number is not fixed, but in one run it is the same on every worker. A function, a boundary, a continuation and a key give a number from their kind alone.

**§B.1 The module.** Add *(now)*:

> This version runs one instance for each run of the program. Workers (§11.9) come with a later version of this interface.

*(multicore wave)* Browser modules import shared memory; native keeps private linear memory. Worker entry and the top-level portability probe are MC-D interfaces; `ilang_start` still runs on every worker. The earlier `ilang_worker(index: i32)`/`ilang_toplevel_portable` export sketch is not an addition to ABI v3: new runtime exports require a reviewed version bump (native N3). Prefer ordinary configured host exports/services if they suffice; settle this before MC-I.

**§B.4 Imports.** Add to `reference_id`:

> A host reference belongs to the instance that got it. It never goes to another instance.

**§B.6 Metadata** *(multicore wave)*: the host externals that the top-level values that are not direct values may call at start-up (part 3.5).

**§9.9 `i/task` (new)** belongs to track R. It states the API rules of part 18 where programs can see them, including the stop messages `task: value is not portable`, `task: shared value is gone`, `task: channel is gone`, `task: go outside its scope`, `task: join outside its scope`, `task: handle of another run`, `task: deadlock` and `task: no scheduler`, the rule of D1, chains and cancels (part 6), and runs and nested runs (part 4).

## 14. Files to change

**Track R (now):**

- `docs/spec.md`: §2.5, §4.2, §4.5, §9.2, §9.4 (rules 3, 4 and 7), §9.7, new §9.8 (`i/key`, `Key`, `Keyed`), §10.1, §11.1, §11.4, §11.7, §11.8, new §11.9, §12, §13, §B.1, §B.4 (part 13).
- `docs/design/meta.md` §8: one sentence that a `Syntax` value is not portable (§11.9), whatever its form.
- `lib/i/core.i`: export `portable` as an `@external("ilang:core", …)` declaration, like `not` and `hash`.
- `lib/i/key.i` (new): `new`, `wrap` and `unwrap` as `@external("ilang:core", …)` declarations.
- `lib/i/control.i`: `keep_env`.
- `crates/ilang-wasm/src/lower.rs`: add `portable` and the key functions to the `CORE` and `CORE_FUNCS` name tables near `:120` and `:130`; the `keep_env` op, which keeps the `$env` array as it is (part 4); bit 0 of field 1 for closures with patched captures (part 3.6). `captures_of` (`:389`) is the definition of captures; it does not change, and a comment points at §11.9.
- `crates/ilang-wasm/src/emit/mod.rs`: `closure()` writes the patch flag in field 1 and gives a patched closure's captures array its last element, the walk stamp (part 3.6). The `$clo{n}` types do not change.
- `crates/ilang-wasm/src/emit/control.rs`: `$keep_reenter` and the frame record of a kept function (part 4).
- The checker's built-in types for `portable`, `keep_env` and keys (`crates/ilang-check`): `BuiltinType::Key` and `BuiltinType::Keyed`, and a `Key: [Inv]` row in `builtin_variance` (`crates/ilang-check/src/types/solve.rs:1285-1292`). Without that row the argument of `Key` is never compared, and `unwrap` is an unchecked cast.
- `crates/ilang-wasm/src/runtime/values.rs`: `$core_portable`, a walk with an explicit stack like `$value_compare_i32`, with the walk stamp of part 3.6 (one new token for each walk). It gives False at tags 10, 11 and 12 and at a host request, and walks field 5 of `$obj` (tuples, custom values, closure captures, records), `$cons` heads and tails, `Keyed` values, and kept services. The key functions.
- `crates/ilang-wasm/src/runtime/mod.rs`: the tag table (`:46`) gains a "portable" note, the patch flag, the key tag, and the planned `$clo` layout of part 3.4.
- `crates/ilang-check/tests/library.rs`: the core and control export lists.
- `lib/i/task.i`, `lib/i/random.i`: following part 18.
- `docs/performance.md`: channel state kept until the channel's scope ends (part 7), the cost of `portable` checks (also on DAGs, part 3.6), pinned tasks under boundary services (part 4), the start-up conditions of part 3.5, and the distinction between portable IO wrappers and pinned arbitrary host references (part 12).

**Track B (now, small):** frame records carry the point index (part 12).

**Multicore wave (not now):**

- `crates/ilang-wasm/src/emit/mod.rs`: `closure()` with flags and arity in field 1 and id in field 2; static ids, also for the root `Assert` closure, which becomes a `$c` constant; the `$funcs` table, with `$entry_assert_fail`; the id counter and worker globals.
- `crates/ilang-wasm/src/runtime/values/inspect.rs:262`: arity from field 1.
- `crates/ilang-wasm/src/runtime/values.rs`: id compare for tag 7 and keys.
- New runtime modules for copy/identity and the top-level portability probe; worker-local request/continuation tables; shared byte-state transactions, queue ownership, stop/quiescence and share storage. Only browser transport needs the shared-memory allocator in runtime/mod.rs; add the walk mark if MC selects it. Settle worker entry/ABI version at MC-D.
- `crates/ilang-host`: runner-owned byte transport and one-lock transactions, long-lived worker threads and private memories/heaps, resource contexts, wake/reactor integration, N12 stop domains/controller and the startup conditions of part 3.5. Native transport needs neither SharedMemory nor the Wasm threads feature.
- `bindings/javascript/ilang.mjs`: the isolation check, Workers, the imports module URL, worker 0 rules, the wake request.
- `tests/browser`: COOP/COEP headers and a fallback test.

## 15. Tests

**Now (track R):**

- **T1** `tests/spec/13/portable.i`. True for: an Int, a big Int, a Float, a String, a bit array, `[]`, a tuple and a list of these, a top-level function, a constructor, a closure that captures only data, a closure that captures another such closure, a local recursive function (it holds itself), and two local functions that call each other. False for: a boundary `b` (inside `control.boundary`), a continuation (inside `suspend`), a closure that captures `b`, a local recursive function that also captures `b`, a list holding such a closure, a custom value with a boundary field. After records land: `( k: b )` is False.
- **T2** `tests/spec/9.7`, with a host fixture as in `tests/spec/11/equality_externals.i`: `portable` of a host reference is False, and of a tuple that holds one too.
- **T3** `tests/spec/11/equality.i` stays unchanged; it guards that §11.4 did not change.
- **T4** Captures are exact: the example of part 3.1 is True; a closure whose only use of `b` is in a clause that never runs is False. Both run at O0 and O2 and give the same answers.
- **T5** `keep_env`: a kept function called under a different environment statement still sees the kept service, also after it suspends and is resumed under another environment (`$keep_reenter`). A kept function made where a non-portable service is bound is not portable, also when its argument does not need that service, and also when `keep_env` is called inside a generic wrapper; the answer is the same at O0 and O2. A kept function made after that service is bound again to a portable value is portable.
- **T6** A runtime unit test runs `$core_portable` on every tag, on a list of 10^6 elements (no Wasm stack overflow), on a closure that holds itself (it ends), on a list of 10^5 local recursive closures (time linear in the size; a quadratic walk fails the time limit), and on a marked closure walked twice (the second walk does not skip it because of the first walk's stamp: a closure that holds a boundary is False both times).
- **T7** An emit test pins today's `$clo` layout (field 1 is the patch flag, field 2 is the arity, field 3 is 0) and names the planned layout in its message. A unit test pins the list of readers of `$obj` field 3 of part 3.4: `$float_get` and the readers behind a tag-2 test.
- **T8** Track R: a send of a host reference on `channel.new` stops with `panic: task: value is not portable` on one worker; the same send on a local channel works; a child giving back a host reference stops in a portable scope and works in a local scope; a send of the Task handle of a child of a local scope on `channel.new` stops with `panic: task: value is not portable`, and the same handle sent on a local channel can be joined; `portable` of that Task handle, and of a join event made from it, is False; a `get` after the scope ended stops with `panic: task: shared value is gone`; `r = scope((s) { channel.new() })  r.send(1)` stops with `panic: task: channel is gone`; `channel.send_event(c, host_ref)` with `c = channel.new()` stops when the event is made, also when it would sit in a `choose` whose other branch commits; both returning user opaque host references stops, while both returning standard File values succeeds, and `map_concurrently` over children that give host references stops.
- **T9** Track R: with a fixed seed, `task.random()` gives the same generator in each task under two scheduling orders (a test scheduler service picks the order); `random.int` with `random.from_seed` works in a program with no `task.run`, in a top-level value and in a macro; a macro that runs `task.run` under `task.top(random.from_seed(0))` gives the same expansion twice; the first half of `random.split(task.random())` in the root task and the generator of the child at path `[1]` give different numbers; two sibling tasks that each start a nested run and call `task.random()` in its root task get different numbers.
- **T10** Track R: a scope returns only after every child ended; final functions of cancelled children run once; with children at go indices 1 and 3 that both fail before any other suspend point, and the test scheduler set to run 3 first and to deliver the scope's cancel only after both have failed (an order that N workers allow: the cancel reaches a task on another worker at its next poll point), the error is that of index 1; with the same order but the cancel delivered at once (as one FIFO worker does), index 1 is dropped unstarted and the error is that of index 3 (D10 fixes the error only when the same children fail); in `scope((s) { s.go(() { s.go(() { fail.raise(A) })  fail.raise(C) }) })`, with the test scheduler set so that both fail before the cancel reaches either, the error is `C` whichever fails first; `both` and `map_concurrently` give argument order.
- **T11** Track R: a task started inside a generator body, under the entry's scheduler loop, keeps the generator's `Yield` service; when it yields, it stops with `control: boundary is not active`. A nested run inside a generator body (`collect(() { task.run(…) })`, part 4) yields with the meaning of §9.4, the same on every host.
- **T25** Track R, chains (part 6): `h = scope((s) { s })  h.go(() { 1 })`, `scope((u) { scope((s) { u.go(() { task.yield()  s.go(() { 1 }) }) }) })` and `scope((outer) { c = channel.new()  outer.go(() { h = c.receive()  h.go(() { 1 }) })  scope((s1) { c.send(s1) }) })` each stop with `panic: task: go outside its scope`, under every order the test scheduler picks; a child of `s` that calls `s.go`, and a grandchild in a scope opened inside a child of `s` that calls `s.go`, work; a join of a child of `s` from a task started in an outer scope stops with `panic: task: join outside its scope`.
- **T26** Track R, cancels (part 6): `scope((s) { s.go(() { fail.raise(A) })  t = s.go(() { 1 })  t.join() })` gives error `A` on the FIFO scheduler and never `task: deadlock`, and the body's final functions run; `scope((s) { never = channel.new()  s.go(() { resource.ensure(() { never.receive() }, () { task.yield()  scope((f) { c = channel.new()  f.go(() { c.send(1) })  c.receive() }) }) })  s.go(() { fail.raise(E) }) })` gives error `E`, not `control: boundary is not active`: the final function of the cancelled child yields, starts a scope, sends, receives and waits for its child; the same with a fake `Clock` and a sleep in the final function; a second cancel of a task that is unwinding does nothing.
- **T27** Track R, nested runs (part 4): with a host fixture that has an async `wait_flag()` and a sync `set_flag()`, `task.run(() { scope((s) { s.go(() { task.yield()  host.set_flag() })  task.run(() { host.wait_flag() }) }) })` ends on one worker; `task.run(() { scope((s) { c = channel.new()  s.go(() { c.send(1) })  task.run(() { c.receive() }) }) })` stops with `panic: task: handle of another run`; a nested run with nothing to run and no request stops with `panic: task: deadlock`; the paths in a nested run start with the calling task's path.
- **T24** Track R: after a task suspends and is resumed by the loop, and in the final functions of a cancelled task, a service bound before `go` (a fake `Clock`) is still the kept one (§9.4 rules 3 and 4 as amended). `s.go(() { task.yield()  task.yield()  c.send(1) })` works: the second and third scheduler calls after a resume reach the task's own `Scheduler`, not `scheduler.detached` (part 4, the call that is not a tail call), at O0 and O2, and a WAT check at O2 finds no tail call after `! Scheduler = own`. A kept function made inside a task with `control.keep_env` that sends on a channel works when its task calls it and stops with `control: boundary is not active` when another task calls it; the same callback made with `task.keep` works from both.
- **T12** Track R: `i/task` in a macro program gives the same expansion twice. In a test under `i test`, `map_concurrently` over children that give `Syntax` values stops with `panic: task: value is not portable`, and the same work in a local scope works.
- **T20** Track R: `scope((s) { c = channel.new()  s.go(() { c.receive() }) })` stops with `panic: task: deadlock`, and the module never returns status 1 with no request outstanding. The same in a macro program.
- **T21** Track R: keys: `unwrap` with the key that made a `Keyed` gives `Found`, with another key `Missing`; `k = key.new()` is not generalized: after `a = key.wrap(k, 1)` and `b = key.wrap(k, "x")`, `key.unwrap(k, b) |> { Found(n) -> n + 1  Missing -> 0 }` is a compile error (the found value is `Int | String`), and a control line with a second key `k2 = key.new()` used only at `Int` (`key.unwrap(k2, key.wrap(k2, 1)) |> { Found(n) -> n + 1  Missing -> 0 }`) compiles and gives 2, so the error comes from the type check, not from the parser; `Key` is invariant: a `Key(Int)` given where a `Key(Int | String)` is wanted is a compile error, and a checker unit test reads the `Key: [Inv]` row; a `Keyed` is portable when its value is; `g = (f) { control.keep_env(f) }` gives `g(h) == g(h)` False.

**Multicore wave:**

- **T13** Differential: every `tests/spec` case and every std test runs single-core, threaded with 1 worker, and threaded with 4 workers (the last 200 times). Results, stdout and stop messages must be the same, except in tests marked as order-dependent (D1). A marked test lists every outcome it allows: a set of (stdout, stop message) pairs, or, when the set is large, a checker in i that is given the output and the stop and says whether some order of steps gives them.
- **T14** Copy round-trip for each kind: `==`, `hash` and text form equal; constant closures, `True`, `False`, `[]` and labels come back `ref.eq` to the receiver's globals.
- **T15** Sharing: `x0 = Leaf`, `x(i+1) = Node(x(i), x(i))` to depth 40, and the same with list cells: the copy has 41 nodes. A closure that holds itself copies to a closure that holds itself.
- **T16** Closure and key identity across a remote claim, also inside records and tuples; a task whose record holds the root `Assert` closure is remotely claimed, and its `Assert` is `ref.eq` to the remote claimant's. A child of a local scope, a task whose record holds the Task handle of such a child, and a task whose record holds a host request, are never remotely claimed.
- **T17** Stop on worker 3 while others spin without polling: one message and no i finals. The native subprocess tears down resources/children then exits without joining spinning workers. A continuing host interrupts/joins only that run; a concurrent and a subsequent run continue normally. Repeat with outstanding requests and a late provisional reply (native N12/§11).
- **T18** Cross-worker select under stress, 200 runs: no message lost or duplicated, one commit per sync.
- **T19** Browser: with COOP/COEP, an imports module URL and the dedicated-worker entry route, 4 Workers and a main thread that stays responsive; stop while worker 0 spins terminates all executing workers and tears down resources; without isolation, the URL or a stop-safe entry route, one worker with the same output; strings through the shared memory decode right.
- **T22** Start-up: a module with a top-level host reference (`counter` of part 3.5) runs one worker and gives 100; a module whose start-up calls a host external that the host has not declared pure runs one worker; a module that only uses `i/core` externals at start-up runs 4, and so does a module whose host externals are called only from the body of `main`.
- **T23** Deadlock and idle on 4 workers: T20 gives the same stop; a run where worker 0 has nothing to do while workers 1 to 3 work ends normally, and in a browser worker 0 waits only with the wake request outstanding; on wasmtime, a remotely claimed task on worker 1 that sleeps 10 ms while worker 0 runs a long pure loop wakes after about 10 ms (worker 1 returns status 1 and does not park in Wasm), and a cancel or a queue publication from worker 0 still reaches worker 1 while it waits (the wake request).

## 16. Risks

- **`portable` makes captures visible.** An optimizer that drops or adds a capture changes `portable(f)`. The spec definition and T4 at O0 and O2 guard it; any later closure optimization must keep the capture set.
- **The check costs a walk.** Every portable send, result and error walks the value, also on one worker, and a DAG is walked once for each path (part 3.6). Large data should go through `scope.share` or local channels. A later mark (D9) or a "known portable" bit can make repeat checks O(1).
- **Stopping on a non-portable result may surprise.** A child returning a user opaque host reference in a portable scope stops; standard IO wrappers do not. The local scope API and a clear message are needed, or people will route around the rule.
- **`keep_env` keeps every bound service.** A task started under a boundary-holding service that it does not use (a generator's `Yield`, the `Assert` of `assertion.catch`) is pinned. So tests that run under `assertion.catch` get no parallelism. It cannot be seen, but `docs/performance.md` must say it. Changing `keep_env` later to keep fewer services would change `portable`'s answers.
- **Kept functions made inside a task are tied to it.** A callback made with `control.keep_env` inside a task holds that task's `Scheduler` (D4); another task that calls it stops when it uses `i/task`. `task.keep` is the way out, and the docs must say so clearly.
- **Stricter scope rules than a naive scheduler.** `go` and joins check the chain, channels die with their scope, and handles check their run (part 6, part 7, part 4). Each stop is fixed by the program, the same on every worker count, but programs that pass scope handles or channels out of their scope will meet them.
- **`keep_env` and keys are new language surface.** G2 must accept them as general.
- **Order still chooses some things.** Anything that depends on the order of steps may change on N workers (D1): which sender a receive gets, which `race` branch wins, which children fail before the cancel, whether a `shared.get` comes before its scope ends, which stop is reported, and the order of output. Tests must not depend on the order, or must be marked with their allowed outcomes (T13); the M:1 order is fixed but is not a promise for N workers.
- **User host references still pin.** Standard sockets are portable Int wrappers; arbitrary opaque references still stay home by H (part 12, item 5). Capturing a Request/Ticket still pins a handler, even if its socket is portable.
- **Start-up runs N times.** It costs N times the start-up work, in parallel. Modules that make host references at start-up, or call host externals the host has not declared pure, run one worker.
- **Host limits.** One pending async import per instance today blocks overlapping I/O; the request table of C/R must come first. N Stores each reserve a GC heap. In browsers, host functions must come from an imports module URL, and main-thread functions are async proxies with latency.
- **Ids count every closure.** The 56-bit per-worker counter (part 3.4) does not run out in practice, but it counts closures made, not live ones; lazy ids (with the mutable field of D9) remove this cost.
- **Field 1 and field 2 move.** If other work reads field 1 or 2 of closures before MC-I, the move breaks it. T7 and the tag-table note guard it.
- **Left to MC.** The typed locked-state interface, browser allocator, mark or table of D9, quiescence proof and transport memory model remain MC-D work. A long computation delays cooperative task cancellation and local started tasks; it cannot delay remote queue claims, and run stop uses native N12.

## 17. Open questions

1. Is `core.portable` accepted as a public built-in (D3), or should G2 ask for a narrower form? This note argues for public, because `i/task` is ordinary i code.
2. `control.keep_env` takes a function of one parameter. Is that enough, or should there be a form for functions with no parameter as well? The number of parameters is part of a function type, and function types with different numbers of parameters cannot be put together (spec §4.4, §4.6), so `() -> A` is not `(x: Nil) -> A`, and a zero-parameter body such as the `go` body `() -> A ^ E ! N` cannot be passed to `keep_env` as it is. `go` wraps it in a one-parameter function (part 4), and so can any other caller; a second form would save that wrapper.
3. Per-task storage in general (a `task.local` facility built on keys) is not needed for Random any more (part 5): Random is a value. Should `i/task` still offer per-task storage for other uses? Any such storage lives in the scheduler, so reading it is a call to the `Scheduler` service.
4. How does worker k > 0 enter `i/task`'s scheduler loop without a special case? One answer: the program's entry runs `task.run`, which, on N workers, is an ordinary host export or entry kind that every worker calls. Only that entry run uses N workers; a `task.run` from inside other code is a nested run with one worker (part 4). MC-D decides.
5. Which general typed byte-state transaction interface implements part 7 without running i code under a lock? Recommended: ordinary request-mode services; require the MC-D prototype before MC-I.
6. Message bytes are prepared before a transaction under part 7. MC-D chooses caching/reclamation of those immutable bytes without changing offer-time portability checks.
7. Does G2 accept keys (D13) as a public primitive in a new library module `i/key`? Without them, or another safe way to keep values of many types in one service, the scheduler cannot be written in i, and rules 12 and 15 cannot hold.
8. Should `Syntax` later get ids and become portable, so that tests and tools can pass code between tasks of a portable scope (part 11)?

The N-R1–N-R6 fixtures in native §11.1 are required at 1/2/4 workers: remote
Root count without Ticket transfer; owned read cancellation; wake-only deadlock;
pure and captured events; borrowed nested requests; combined go/run paths.
MC-D must demonstrate atomic group subscription, cancellation actions before
group-cancel return, home-cell terminal reconciliation without foreign Tickets, and the
program/control quiescence snapshot through the one Scheduler boundary before
MC-I. These are fixed semantics, not choices left to that prototype.

## 18. API rules for track R (`i/task`)

1. Values pass between tasks only through `go` captures and kept services, channel and event sends, task results and errors (read by the scope or by a join of the Task handle), and `scope.share`. There is no other shared state.
2. Every portable path calls `core.portable` and stops with `panic("task: value is not portable")` when it is False: a `send` on a `channel.new` channel (checked at the call), a send event on such a channel (checked when the event is made, never at `sync` or at commit), results and errors given to a scope made by `scope`, also inside `both`, `race` and `map_concurrently`, which are built on `scope`, and `share`. This happens always, also on one worker. Local forms of the combinators, if R adds them, are built on `local_scope` and do not check.
3. `go` never rejects a task for non-portability. On N workers it silently classifies and encodes the complete record at go (D8/P1); failure pins locally. A shared entry has one claim/drop owner, coordinated with scope cancellation. One worker keeps its FIFO originals without encoding.
4. As runtime §6.3 requires, `go` also detaches/rebinds Runner and Await while preserving Root/Owner. `go` binds `Scheduler` to the portable `scheduler.detached` and then keeps the parent's services with `control.keep_env`, and takes the real `Scheduler` as the parameter of the kept function. So the parent's scheduler is never kept or captured. The kept function binds `! Scheduler = own` and then calls the body **not** in tail position (`r = …  r`), so that the statement stays on a frame record after a resume (`lazy-reentry.md` D16, section 7a); R checks at O2 that it stays a frame. `task.keep(f)` is built the same way for callbacks that other tasks call; a function that user code keeps with `control.keep_env` inside a task is tied to that task (D4), and the docs say so.
5. Every task runs directly from its run's loop, never on another task's stack, under a new boundary that the loop makes for it at its start (D5); `control.resume` makes it active again. The loop has no long-lived boundary of its own. Nested scopes, `both`, `race` and `map_concurrently` never run a loop on the caller's stack. Only the entry's `task.run` may use more than one worker; any other `task.run` is a nested run: one worker, FIFO, on the caller's stack, on every host. `task.run` needs the run service (part 4), which gives it its path prefix, root seed and wait. A nested run inside a task has paths that start with the calling task's path and its run index, waits by suspending only its calling task to the outer run (never by status 1 from the caller's stack), and uses the owned-plus-explicitly-borrowed waitable set of part 4/runtime §6.4; no runnable or pending program delivery and an empty set means `panic("task: deadlock")`. Channel/Task/scope/Shared handles check their run; Event descriptions have fresh keys without a required creating run, and sync checks every stateful leaf, including losing alternatives.
6. Local channels and local scopes hold the boundary of the task that made them, only to be non-portable; no suspend ever goes to it. The Task handle of a child of a local scope, and every join event made from it, hold the local scope handle, so they are not portable either. Every task record holds the handle of its scope, so a child of a local scope is never remotely claimed. Local paths do not check. There is no flag and no special case.
7. A started task never moves. A task that waits on an async host call resumes on the worker that started the call. Request/Ticket values (R2) are not portable: only their home worker operates on their cells. Portable Root/Owner operations observe or cancel whole groups through shared metadata and home-worker notifications; they never return foreign Tickets (runtime §5.1).
8. A scope returns only after all its children have ended. On a failure it cancels the rest **and its own body**, which is unwound back to the `scope` call, and then gives the error of the failed task with the lowest path in path order (D10; its own body first, `[1] < [1, 1] < [2]`); a cancelled body is not a failure. Every task has a chain of scopes (part 6), changed only by its own scope entries and exits. `s.go`, and a join of a child of `s`, may be done only by a task whose chain holds `s`; otherwise they stop with `panic("task: go outside its scope")` and `panic("task: join outside its scope")`, whether or not `s` has ended. A `go` on a cancelling scope makes an unstarted task dropped at once; a shielded caller receives its Task handle, while a cancellable caller unwinds before observing it (runtime R4a). At a valid join/selected join event, applicable cancellation first unwinds a cancellable joiner; a shielded joiner stops with `panic("task: join of a cancelled task")` as soon as target drop/cancellation is accepted, without waiting for cleanup (runtime R4a). Error types stay unions.
9. `both` and `map_concurrently` give results in argument order. The docs state the rule of D1: every run gives the result of some order of steps, so a plain receive with several senders, `race`, `choose`, and anything else that depends on the order may differ between runs and worker counts.
10. Cancellation is cooperative: an unstarted task is dropped; a started task is cancelled on its home worker when the cancel reaches it (at once when it is paused, at its next suspend point when it runs; when that is, is a step of D1). The loop never calls `control.cancel` on a task's continuation: it resumes the task with a cancel answer, and the task cancels its own inner part, so its boundary stays active and final functions may use `i/task`. A task being unwound is not cancelled again.
11. Every task has a path (the path of the task that called `go` plus the index of that `go` among its `go` calls and nested runs), given by `task.path()` with no suspend. Random is a value: `task.random()` is a pure function of the root seed and the path, and `i/random` does not need `i/task` or `Scheduler`. The root seed comes from the run service; the entry default is random.from_seed(0), and task.top(seed) changes only the current Runner's seed/route-preserving value, with no entropy draw (runtime D5). Path steps and split steps are different kinds of stream step, so `random.split` never gives the generator of a task. Task/scope/channel/share ids and per-task names come from the path, never from time, a global counter, or the worker. Event identity uses a fresh key independent of any task path or run; only equality observes it, and copies retain it (runtime §6.4).
12. Task, scope, channel and `Shared` handles use path-based ids and keys (D13), with local witnesses for local handles and children of local scopes (rule 6). Events instead contain a fresh identity key and ordinary descriptions, payloads and callbacks; their captures may contain boundaries or other non-portable values, which pin a capturing task under the normal complete-record test. Stateful handle implementations add no hidden closure over a live scheduler; user event callbacks may capture one and are then non-portable. `==` on these values means "the same handle", and event copies preserve their key. No API needs an order or a hash on functions, keys, boundaries or continuations; scheduler bookkeeping uses Int ids.
13. Events are values. `choose` and `map` are pure, `sync` commits exactly one event, and `map` functions run in the syncing task after the commit. Nack and guard are left for later.
14. `scope.share(value)` needs a portable value. A `Shared` belongs to its scope, and a `get` after the scope ends stops with `panic("task: shared value is gone")` in every build. A channel belongs to the first scope in its maker's chain (or to the run); its state is freed when that scope ends, and a later send, receive or `sync` on it, or a wait on it at that end, stops with `panic("task: channel is gone")` in every build.
15. All shared mutable scheduler state is behind the one `Scheduler` service, kept as `Keyed` values where it has many types, so the multicore wave can replace its implementation without changing the API. No API needs an unsafe cast.
16. The scheduler has poll points (`go`, every `sync`, joins and scope ends, `yield`, async host calls, every switch of the loop) written as one call that does nothing on one worker.
17. On one worker the scheduler is FIFO and fixed, with no time unless a `Clock` service is given. It idles only by waiting for an outstanding host request (C-D's "wait for any"), never by a busy loop or a blocking import, and never returns status 1 with no request outstanding. When no task or pending program delivery can run and the owned-plus-borrowed program waitable set is empty, it stops with `panic("task: deadlock")`; control-root wakes do not prevent this (part 6). On N workers (MC), a worker with outstanding requests of its own never parks inside Wasm; it returns status 1 with a wake request outstanding (part 6, Idle).
18. Many host calls may wait at once, through the request table. No code relies on one pending import per instance.
19. A stop in any task stops the program. It is never caught or counted as a task error.
20. No API takes a label-generic callback (records L5), and no API needs a static `Send`-like type.
21. No API shows worker indices, the worker count, or queue claims.
22. Tests of `i/task` and of programs that use it must not depend on the order of steps (D1: which sender a receive gets, which `race` branch, which children fail before a cancel, whether a step comes before a scope ends, which of two stops, the order of output), or must be marked with the outcomes they allow (T13).
23. `Key(A)` is invariant in `A`, and keys are only used through `i/key`; no API casts.
24. `s.go` and joins check the chain, channels and `Shared` values are freed with their scope, and handles check their run, all in M:1 already, so that the multicore wave changes no program.
25. Only `==` reads function and key ids. The text form of a function, a key, a `Keyed` value or any task/scope/channel/event/share handle never shows an id, a worker index or anything else that depends on placement, and no `i/task` API gives one out.

## 19. First native implementation (lane mc, 2026-11-13)

- **Workers.** One module, N instances: the entry's instance is worker 0; workers
  1..N-1 each have their own `Store`, host services and OS thread
  (`crates/ilang-host/src/workers.rs`), started as records are published. N is 1
  by default; `I_WORKERS=<n>` or `I_WORKERS=auto` (cores, at most one per GiB of
  available memory) or `Program::with_workers` opt in. Macro programs, WASI and
  JS run one worker. More than one worker turns on epoch instrumentation (N12).
- **Publication (D8/P1, N7).** At `go` in the entry run, `i/task` encodes the
  complete record (start function with kept services, path, run, root, runner)
  with the runtime codec (`crates/ilang-wasm/src/emit/workers.rs`, keeps sharing
  and cycles). A record holding values of worker 0 that cannot move stays local;
  one holding tokens of worker w is published for w only. Nested runs never
  publish (runtime §6.5).
- **Claim once, run where claimed.** The maker claims a record only when no
  worker is free for it (Go: work goes to idle Ps first); otherwise a worker
  that holds no task claims it. A claimed task runs on that worker to each next
  step; started tasks never move (N6).
- **One locked authority (N8).** All shared scheduler state stays in worker 0's
  i scheduler. A worker sends each step of its task as a message (the command,
  with the `finalizing` answers of the task's frames); worker 0 answers, and
  the answer goes back to the worker holding the stack. Values that cannot move
  travel as tokens of their worker. Shared values are read from the host
  transport directly while their scope lives.
- **IO on every worker.** A worker's requests are started in mirrors of the
  task's request groups, waited for there and settled through `ilang_settle:*`
  exports; worker 0 sees a cancellable wait that a `Woken` message ends.
- **Stops (N12).** A stop on a worker stops the run with its message.
- **Open: scheduling order with several workers.** With N > 1, outcomes that
  depend on the order of steps can differ from the one-worker FIFO schedule
  (e.g. spec 9.9/tasks.i's race winner, 9.9/final_cancel_phases.i's first child
  cancelled before it starts). Whether D1 permits these is an owner question;
  hence the default of one worker.
- **Not done.** Function ids for `==` across copies (a copied closure is not
  `==` its original); D7's top-level portability walk; worker-local Root/Owner
  counts are not shared with worker 0; worker token tables are never freed;
  browser workers (N11); the packaged runner runs one worker.

### Open: reclaiming worker-local token entries

The message encoder currently roots each nonportable value in the owning
worker's `$mc_locals` table and sends its worker and slot as a token. Forwarding
a foreign token copies that identity, so the end of the task that first sent it
does not prove the slot is dead. For example, a local-channel witness or a
value held in a local channel buffer can survive that task. A token may also
be held in scheduler state, a task record, an encoded message in transit, or a
value on another worker's heap, including inside a closure or a cycle.

Reclamation needs a protocol that accounts for every live copy and every
in-flight transfer, including cancellation, dropped records and messages, scope
teardown, and stop. It must coordinate transfers and releases with the owner so
an entry is never cleared while a token can still be decoded or forwarded;
local heap reachability and cycles need a solution as well as transport
ownership. Slot reuse must not let a delayed token refer to a different value
(for example, use generations or never reuse identities). Only after the last
possible reference is gone may the owner clear its GC root; teardown of the
whole run can release all remaining entries. The tracking mechanism and its
cost, including how it interacts with Wasm GC, remain to be designed and tested.
Do not free entries merely on task completion or scope exit.
