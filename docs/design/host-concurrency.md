# Design: host concurrency and ABI v3

Status: proposal for review (stream C, step C-D). Base commit `dcb19df`. All file and line references are at that commit. Decision D4 departs from the approved plan and needs the user's explicit approval before C-I starts (see "Approval needed").

## Summary

- An async host import gets one more Wasm parameter, placed first: an `i32` **request id** that the module picks.
- The resume export becomes `ilang_resume:ns:label(id, results...) -> status`.
- Status 1 now means "waiting for one or more outstanding requests". A host may answer outstanding requests in any order, one at a time.
- Status 2 is new: "I can go on without an answer". The host first gives the answers that are already done, and otherwise calls the new export `ilang_continue`. This lets the `i/task` scheduler of R2 pick up finished IO while some task can still run.
- A new runtime import `ilang:host/v1.cancel(id)` drops one outstanding request.
- Compiled modules still keep at most one request outstanding in this stream, never return status 2 and never import `cancel`. A hand-written fixture tests all three. Many outstanding requests from one module come with `i/task` in stream R2. R2 needs no other ABI change for the needs known today (section 6.4); a new runtime import, status or export later means version 4. On the language side R2 needs one general feature, host requests as values, which section 6.4 names and R2 designs; C does not build it.
- The ABI version becomes 3 everywhere. Hosts refuse every other version with a clear message.
- The Rust host stops starting one OS thread for every Wasm call. `i run` and `i test` run each program on one long-lived thread with a big stack, and that thread also runs a tokio `current_thread` runtime. This runtime is the reactor that `i/clock`, `i/net` and `i/fs` need later.
- Wasmtime fibers (its async support) are not used. Every Wasm call keeps the same stack limits as in v2: a top-level call runs at the base of the `ilang` thread, and a call made from inside a host function (for example into a second instance) still gets a thread of its own (D5). So stack depth, trap texts and stop messages stay the same.
- A finished program never waits for leftover blocking work: `block_on` shuts its runtime down in the background (section 5.2).

## Approval needed

The plan's C-D list (plan line 450) says "用 fiber 加 tokio `current_thread`" (use fibers plus tokio `current_thread`). This design keeps tokio `current_thread` but does **not** use fibers. The reasons are in D4. The coordinator should ask the user to approve this change and then update the plan line to "one big-stack thread per run, with tokio `current_thread`; no Wasmtime fibers". C-I does not start before that.

## 1. The problem

Today an async entry point can wait for exactly one host function:

- `crates/ilang-wasm/src/emit/host.rs` keeps one global, `$host_pending` (host.rs:21). `host_start` traps when it is already set (host.rs:103), and each resume export checks it (host.rs:275).
- `crates/ilang-host/src/lib.rs` `wait()` (lib.rs:1038) requires exactly one `Pending` (lib.rs:1050) and fails otherwise.
- `bindings/javascript/ilang.mjs` `wait()` (ilang.mjs:733) does the same.
- Spec §B.5 says "the program is waiting for exactly one host function that it has started".
- `invoke_in` (lib.rs:1162) starts a scoped OS thread with a 48 MiB stack (`THREAD_STACK`) for every Wasm call, also for every `ilang_resume:…`. A program that waits a thousand times starts a thousand threads.

i must be a standalone language: `i run` gives `i/clock`, `i/net`, `i/fs` and the scheduler of `i/task` from inside its runtime, and a host written by a user is secondary. So the ABI must allow many outstanding requests answered in any order, and `i run` needs one reactor that lives as long as the program.

## 2. Decisions

### D1. The module picks the request id

An async import has the Wasm parameters `(i32 id, B.2 parameters...)` and no results. The module picks the id.

Reason: R2 can then use the id as the index of its own request table, with no host round trip and no ABI change. The host never invents ids, so it needs no id space of its own.

### D2. Status 1 means "one or more"; answers come in any order, one at a time

Reason: this is the smallest change that lets R2 keep many requests outstanding. "One at a time" keeps the module single-threaded: no resume runs inside another Wasm call.

### D3. How a request ends

A request ends in one of four ways: its resume is called; the module calls the runtime import `cancel(id)` (D12); the async entry point ends (status 0) while it is still outstanding; or the instance traps or the host fails. In the last three cases the host drops the work and never answers it.

Reason: dropping at status 0 lets a program end without waiting for work it no longer needs.

### D12. v3 carries the ABI side of an M:1 scheduler: `cancel` and status 2

Spec §B.6 says the host must give every import in `ilang:host/v1`, or making the instance fails. So a runtime import added later breaks every older host, and that is exactly what the version number reports. The same holds for a new status value or a new export that the host must call. So v3 carries the two things an M:1 scheduler needs, and later additions of this kind bump the version:

- `cancel(id: i32)`, a runtime import: the module drops one outstanding request. The host drops its work and never answers it. A scheduler needs this to end one task (for example a timeout around a read) while the entry point goes on.
- Status 2, "runnable", with a new export `ilang_continue() -> status`. Without it, a host calls the module back only after status 1, and status 1 is right only when no task can run. A task that loops with `task.yield()` would then starve every task that waits for IO (a sleeping task never wakes; a server never sees the next `accept`). With status 2 the scheduler returns at its scheduling points; the host lets the reactor run once, gives every answer that is done, and calls `ilang_continue` when there is none. This is the role of Go's netpoller check in its scheduler. How often R2 returns status 2 is R2's choice.

Compiled modules of stream C use neither. The hand-written fixture tests both on both hosts (sections 8 and 10), so neither is untested code.

This is only the ABI side. How i code reaches ids, `cancel` and status 2 is a language question that C does not answer; section 6.4 says what it must look like (host requests as values, with one root continuation, the scheduler's), so that `i/task` stays ordinary i code and the compiler has no hook that only `i/task` uses.

### D4. No Wasmtime fibers (needs approval)

Reasons, from the code:

1. A wait does not keep a Wasm frame alive. A wait is `Op::Suspend` to `$host_root` (lower.rs `make_import_func`, ~:1140). The continuation is captured with exceptions and the export returns status 1. So no Wasm frame is live while the host waits. A fiber is only needed when a host function must suspend in the middle of a Wasm call, and no planned service needs that, because `i/net`, `i/fs` and `i/clock` are async imports.
2. Fibers break the synchronous re-entry into the module. In Wasmtime 50, `Store::validate_sync_call` (wasmtime `runtime/store.rs : 2270`) fails with "cannot do blocking `call` from an async context" when it is called on a fiber. Once any `func_new_async` exists, it fails with "store configuration requires that `*_async` functions are used instead". `Bridge::write` (lib.rs:1499) and `Bridge::free_all` (lib.rs:1525) call `ilang_alloc` and `ilang_free` synchronously from inside host functions, for every sync service or external that returns a `String`, `Int` or `BitArray`. With fibers, every path would have to become async: `instantiate_async`, `ilang_start`, `call`, `run`, the tests' `raw()` helper, and every alloc inside a host function. That is a large change with no gain.
3. Stack and stop behaviour stay the same without fibers (D5).

Fibers can be looked at again if a later feature needs a host function to wait inside a Wasm call. That must start with a spike that measures nested `ilang_alloc` cost and the stack-overflow texts.

### D5. One "ilang" thread per run, with tokio `current_thread`

- A new `crates/ilang-host/src/run.rs` has `block_on`. It starts one scoped thread named `ilang` with a `THREAD_STACK` stack, marks the thread with a thread-local flag `ON_ILANG_THREAD`, builds a tokio `current_thread` runtime with time enabled, and runs the future on it. A panic is resumed on the caller, as `invoke_in` does today.
- `invoke_in` calls `function.call` directly when `ON_ILANG_THREAD` is set **and no Wasm call is running on this thread** (a thread-local depth counter `WASM_DEPTH`, which the fast path raises around `function.call`). Otherwise it keeps today's thread per call. The depth check matters for a call made from inside a host function, for example an external that calls a second `Instance`: Wasmtime counts `max_wasm_stack` per store from that store's entry, so two nested stores on one 48 MiB native stack could use about 2 x 32 MiB and overflow the native stack, which aborts the process instead of giving "call stack exhausted". With the check, such a nested call starts a fresh `THREAD_STACK` thread as in v2 (T24). This **fallback** is for embedders that await `call_async` on their own executor. `invoke_in` takes `tokio::runtime::Handle::try_current()` on the caller's thread, before it starts the fallback thread, and enters that handle inside the thread when there is one. (Calling `try_current()` inside the new thread would always fail, because the tokio context belongs to a thread.) So a start closure that makes tokio timers still finds the caller's reactor.
- **The fallback contract.** On the fallback path, the work futures are polled in `wait()`, on the embedder's executor, not on the `ilang` thread. A service that uses tokio (every `i/clock`, `i/net` and `i/fs` service will) then needs a tokio runtime with time enabled (and IO enabled, once the IO stream adds it) around the `call_async` future. The docs of `call_async` say: "Await this inside a tokio runtime with time and IO enabled when a service uses tokio; otherwise such a service panics with tokio's message. `block_on` and `Program::run_*` give such a runtime." The host does not build a runtime for the fallback, because the caller owns its executor. `i run` and `i test` always use `block_on`, so they never meet this case. T12 checks the timer service on the fallback path.
- `Program::run_entry`, `Program::run_test`, and a new public `Program::run_test_with` run their whole body (instantiate, `ilang_start`, then `run` or `run_async`) inside one `block_on`. So `i run` and `i test` use one thread per program or per test, and a sync entry point uses no extra thread.
- `block_on` starts a fresh thread with its own runtime, so it also works when it is called from inside a caller's tokio runtime (today's `block_on` parks forever there, and a plain tokio `block_on` would panic).

Reason: the cost today is one thread per Wasm call. One long-lived thread removes it, keeps `max_wasm_stack = WASM_STACK` and the native `THREAD_STACK` exactly as they are, and gives the program one reactor for later IO.

`block_on` gains the bounds `F: Future + Send, F::Output: Send`. This is a breaking change for an embedder that passes a future that is not `Send`. All in-repo callers pass `Instance` futures. `invoke_in` already takes `impl AsContextMut<Data = State> + Send` (lib.rs:1163), and `ServiceFuture` is `Send` (lib.rs:479), so these futures are `Send`. The tests check this at compile time (T15). Embedders with a future that is not `Send` keep using `call_async` on their own executor, which takes the fallback path. The public docs of `block_on` and `call_async` say both things in one sentence each.

A second breaking change: sync services and externals now run inside a tokio runtime context. Under v2 each Wasm call ran on a fresh thread with no runtime, so a sync service could build its own runtime and `block_on` it, or use a blocking client such as `reqwest::blocking`. Now such a service panics with tokio's "Cannot start a runtime from within a runtime". Tokio has no public way to leave a runtime context, so the host cannot hide this. The docs of `ServiceFunction` and the "Changes in version 3" section of `crates/ilang-host/README.md` say: a service that needs its own runtime or a blocking client is written as an async service, or starts its own thread and joins it. T16 checks the second way.

### D6. The host calls the service during the import, and waits for it later: one rule for every host

Both hosts call the service during the import, and first wait for its work after the Wasm call has returned:

- Rust: the start closure of an async service runs during the import. The future it returns is first polled when the host next waits. This is the same on the `ilang` thread and on the fallback path.
- JS: the SDK calls the service function `fn(...args)` synchronously inside the import, not in a later microtask as today (ilang.mjs:683 uses `Promise.resolve().then(() => fn(...args))`). The part of an `async` function before its first `await` therefore runs during the import, so `const deadline = Date.now() + ms` written there is fixed at the time of the call. A synchronous throw from `fn` is turned into a rejected promise, so it gives the same answer or failure as today.

So spec §B.4 (section 4) states a rule about hosts that both hosts keep: the host calls the service during the import, and waits for the work only after the import has returned. The advice for service authors, written in `services.rs` and in the JS README: **fix anything that depends on time, such as a deadline, in the start closure (Rust) or before the first `await` (JS), and copy the inputs there.** For example `let deadline = Instant::now() + d; Box::pin(async move { sleep_until(deadline).await; ... })`. Services do not `tokio::spawn`; the host owns every future, so dropping a request drops its future at once. One kind of work outlives that drop: work that the future handed to tokio's blocking pool (`spawn_blocking`, and so `tokio::fs` and `tokio::io::stdin`). Tokio cannot stop it, so it is **detached, not cancelled**: it runs to its end and its result is thrown away. The rule for services (in `services.rs`): such work must not hold anything the program still needs, and it must be safe to leave running. `block_on` never waits for it (section 5.2), so a program that drops a blocking stdin read still ends at once (T23).

In JS the SDK gives each async request an `AbortSignal` and aborts it when the request is dropped (H7, H8, H9, H13, and on every other exit of `wait()` that leaves the instance unusable, section 7). JS cannot stop a promise, but a service can use the signal to clear a timer or abort a `fetch`, so that a dropped `sleep(1h)` does not keep Node alive for an hour. The signal is **not** an extra argument: an extra argument would fill a default or rest parameter of an existing service (`(x, scale = 2) => x * scale` would get the signal as `scale`). Instead the SDK exports `requestSignal()`. Called synchronously while a service starts (before its first `await`, which D6 makes the time of the import), it returns the signal of that request; called at any other time it throws "requestSignal() may only be called while an async service starts". The SDK sets the current signal just around `fn(...args)` and puts the old value back in a `finally`. So the calling convention `fn(...args)` stays as in v2, and services that do not use the signal do not change. A service reads it like this: `async (ms) => { const signal = requestSignal(); ... }`.

The JS host must also refuse a call into the instance from a service that runs during the import. Today `fn` runs in a later microtask, after `wait()` has set `bridge.waiting`; with D6 it runs inside the Wasm import, before the export has returned. So the SDK adds `bridge.depth`, a counter that `enter()` raises around every Wasm call it makes, and `usable()` refuses while it is above 0: "The instance is running Wasm; a host function may not call it (§B.3)". The public calls (`call`, `callAsync`, `run`, `runTest` and the rest) all check `usable()` first; the SDK's own `ilang_alloc` and `ilang_free` calls during an import do not go through `usable()`. A service that calls `guest.call(...)` or `guest.callAsync(...)` during the import then throws at once, which becomes a rejected promise and so a host failure (H9); no Wasm runs re-entrantly and no second request starts. The Rust host needs no such counter: the `&mut Instance` borrow already prevents the call.

Reason: an "eager" start that spawns the work on the `current_thread` runtime does not help. A task spawned on that runtime is not polled until the thread goes back to the runtime, which is after the Wasm call returns, so a timer made inside the task would still count from the wait. Polling once inside the import would need a waker and a runtime context inside a host function. Calling the service during the import and waiting later is simpler, and it is the same on every path and in both hosts.

### D7. "No other export while waiting" is enforced by the hosts only

The module does not guard sync entry points, tests or host exports while an async entry point waits. `$async_begin` and `$async_result` keep a check, but it now reads a new global `$async_live` (section 6.1), which `$async_begin` sets and `$async_run` clears when it gives status 0. The old check of `$host_pending` was enough only because, under v2, every unfinished state had a request outstanding; status 2 with nothing outstanding (H14) breaks that, so a check of `$req_outstanding` alone would let `ilang_result:…` or a second async entry point through while the first one is still runnable. The Rust host refuses other calls through `usable()` and the `&mut Instance` borrow; the JS host through `bridge.waiting` while it waits and `bridge.depth` while Wasm runs (D6).

Reason: a guard in every sync export would change the code that `emit/mod.rs` writes for each export, which is outside stream C.

What does change in every module: `emit/mod.rs` (mod.rs:181-182) adds `host::EFFECTS` to every module, WASI included, with no check of the target or the imports. `$host_pending`, `$async_begin`, `$async_result` and `$async_run` live in that block. So every module, sync and WASI ones too, gets the same new text there: the four globals of section 6.1 instead of `$host_pending`, the `$req_*` functions, and the new bodies of `$async_begin`, `$async_result` and `$async_run`. Writing this only when a module has async imports is not possible without editing `emit/mod.rs` and every caller, because `$async_begin` is always there and refers to it. The change is one fixed block of text, the same in every module, so the WAT audit of the other lanes treats it as one known diff (section 6.6).

### D8. A one-entry request table in the module, with a fixed interface

`emit/host.rs` gets four small functions, `$req_start`, `$req_answer`, `$req_outstanding` and `$req_clear`, that are the whole request table of the module. In stream C the table has one entry. R2 replaces the bodies with a real table and keeps the callers. Answers still go through the per-import globals `$hr{i}_*`, which is correct only while one request is outstanding (section 6.4).

Reason: it keeps the change in C small, and it gives R2 one place to take over.

### D9. Version 3 on both targets, with clear messages

The version is 3 in the compiler, the Rust host, the JS SDK, the fixtures and the spec. WASI modules also say 3, because the version names the metadata format. A host refuses:

- a version below 3: "the module uses i ABI version {v}; build it again with this version of i";
- a version above 3: "the module uses i ABI version {v}, but this host reads version 3; update the host".

### D10. Hosts keep a request table keyed by id

- Rust: `requests.rs` keeps the request data in a map from id, and polls the work through a `FuturesUnordered` (from `futures-util`, already in `Cargo.lock`), so each wake polls only the futures that were woken. Each work future lives in a shared **slot**, `Arc<Mutex<Slot>>` with `Slot { work : Option<ServiceFuture>, waker : Option<Waker> }`. The map entry holds one `Arc`, and `FuturesUnordered` holds a small wrapper future with the other. `cancel(id)` and `drop_all()` take the work out of the slot, so the work is dropped at once, inside the call, and then wake the wrapper. A wrapper that finds its slot empty ends as `Cancelled`, and `next` skips it, so it leaves `FuturesUnordered` soon and nothing builds up in a long program. `Abortable` is not used: an aborted future is dropped only when it is next polled, and that can come after another answer.
- JS: a `Map` from id to entry, a FIFO of settled answers and one wake resolver. A settle is kept only while `requests.get(id) === entry`, so a late settle of a dropped request is ignored. Answers are given in the order they settle.

### D11. Tests are deterministic

The out-of-order tests use a "gate" service: at each step a seeded random choice among the requests that are outstanding at that moment is answered next, not a timer. The timer test uses tokio paused time. The JS tests resolve gates one at a time with a macrotask yield in between, so the order does not depend on how many promise steps the SDK puts around the work. A burst switch releases two gates at once, so that tests reach a finished answer that has not been delivered (section 10.1).

## 3. Exact rules (ABI v3)

The rules are named H1 to H14 (H for host). "R2" in this note always means the stream that builds `i/task`, never a rule.

- **H1. Request id.** An async import (target `ilang` only) has the Wasm parameters `(i32 id, B.2 parameters...)` and no results. The id is 0 or more, and differs from the id of every outstanding request of the instance. "Outstanding" means started and not yet answered or dropped. An id may be used again after its request has been answered or dropped. In stream C the compiler takes ids from a counter that starts at 0 and wraps at 2^31; with one outstanding request, a late or stale id is never outstanding.
- **H2. When a request may start.** An async import, and `cancel`, may only be called during a call of an async entry point, of `ilang_resume:…` or of `ilang_continue`, and not inside another import. The host makes the import trap, and does not use the instance again, when it is called at another time (during `ilang_start`, a sync entry point, a test that is not async, a host export that is not async, `ilang_result:…`, or during an import, which includes a call such as `ilang_alloc` that the host makes into the module while an import runs), or with an id that is below 0 or already outstanding. Both hosts make this check **before** they call the service, so a refused request never runs its service and has no effect in the host (the Rust start closure is not called; the JS `fn` is not called; T6 and T7 count starts). In the Rust host this is the `accepting` flag of the request table; in JS, `bridge.accepting`. The async calls set it to true only around the Wasm call of the export. Every import wrapper (sync service, external, runtime import, async import) first saves the flag as `admitted`, then sets it to false while the host function runs, and puts the saved value back when it returns, also on the error path. The check that admits a request, in an async import and in `cancel`, reads `admitted`, the saved value, never the flag itself, which the wrapper has just cleared. The host function's own work (lifting, the service's start closure, lowering, the user's function) runs with the flag false. So `Bridge::write` (lib.rs:1499), which calls `ilang_alloc` while a sync import lowers its reply, runs with the flag false, and an import made from that `ilang_alloc` sees `admitted` false and traps. The error text is the first failure of the call: in the Rust host the `fail` closure of `define_import` (lib.rs:1343) keeps a failure that is already set (`failure.get_or_insert(error)`) instead of overwriting it, as JS already does (`failure ??=`, ilang.mjs:433). So in the nested case the outer sync import, which sees only a trap of `ilang_alloc`, does not replace the H2 text with "the module trapped in ilang_alloc", and both hosts report the H2 text (T7). The host check is the only guard of this rule. The module has none that helps: `make_import_func` (lower.rs ~:1160-1175) calls the import before it checks `$host_root_boundary`, and `$host_root` is set in `$host_boundary` (host.rs:43) and never cleared, so after the first async entry point that check never traps anyway.
- **H3. Copy at start.** The module frees the parameter buffers when the import returns (§B.3), so the host copies `Int`, `String` and `BitArray` parameters during the import. Both hosts already lift (copy) the parameters during the call.
- **H4. Status.** After an async entry point, a resume or `ilang_continue` returns, 0 means ended, 1 means waiting with one or more outstanding requests, and 2 means runnable (H14). Another value is a fault of the module: the instance is not used again. Status 1 with no outstanding request is also a fault of the module, because the program can never go on: "the module waits, but no host function is running".
- **H5. Answer.** The host answers by calling `ilang_resume:ns:label(id, results...)` for an outstanding id that the import `ns.label` started. Only one resume runs at a time, and never inside another Wasm call of the instance. The host may answer any outstanding request whose work is done; ilang-host and the JS SDK answer in completion order. Result buffers are made with `ilang_alloc` just before the resume that carries them. A finished answer that has not been delivered stays a host value, so a later trap leaks nothing in linear memory. If the module does not export `ilang_resume:ns:label` with the type `(i32, results...) -> i32` for an import it started, it is a fault of the module ("the module does not export ilang_resume:{ns}:{label}"): the host drops every request and does not use the instance again, as for every other fault (T22). It is not a plain error that leaves the instance usable, because the module is still inside its async entry point. The host checks the resume export during the import, before it returns to the module. The requirement applies even if the work is not ready, the entry point next returns status 0, or the module next cancels the request.
- **H6. Wrong resume.** A resume with an id that is not outstanding, or that another import started, traps with a bare `unreachable` and no stop message: it is a host bug, not a program stop. Both hosts check first and fail with their own text: "the host answered request {id}, which is not outstanding".
- **H7. Leftovers at status 0.** When the status is 0 while requests are still outstanding, the host drops them and never answers them. Rust drops the future, which cancels it; JS forgets the entry and ignores a late settle. The module clears its own table on the same path: `$async_run` calls `$req_clear` and clears `$async_live` before it gives status 0, so `$async_result` and the next `$async_begin` see an empty table and an ended entry point. Compiled modules of stream C never leave requests open, so `$req_clear` does nothing for them; it is there so that the rule holds unchanged when stream R2 replaces the table.
- **H8. Trap.** When any call traps (a stop or another trap) while requests are outstanding, the host drops every outstanding request and every finished answer that was not delivered, before it reports the outcome. The instance is not used again, and a late answer never wakes it. The outcome shapes stay the same: `Outcome::Stop` for a stop, `Err(Stop)` for a stop during `ilang_start`, and a failure for a host failure.
- **H9. Host failure.** When a work future gives `Err(...)` (a failure, not an `^ E` error, which is an answer), the host drops all other outstanding requests, does not use the instance again, and returns the error. This is v2's behaviour extended to many requests.
- **H10. One async entry point at a time.** While an async entry point waits or is runnable, the host calls only `ilang_resume:…`, `ilang_continue`, `ilang_alloc` and `ilang_free`. `$async_begin` and `$async_result` trap while the entry point has not ended (`$async_live`, set also at status 2 with nothing outstanding); other exports do not check (D7). Hosts refuse such calls before they reach Wasm: Rust through `usable()` and the `&mut Instance` borrow, JS through `bridge.waiting` between Wasm calls and `bridge.depth` during them (D6).
- **H11. Dropping a waiting call (Rust).** If the future of `call_async` or `run_async` is dropped while it waits, the instance is not used again ("an async call was dropped while it waited"), and its requests are dropped with it. The work futures live in `State.requests` inside the `Store`, not in the call future, so dropping the call future alone would keep them alive until the `Instance` is dropped. Implementation: `wait()` holds a drop guard that owns the `&mut Instance` borrow, and the loop reaches the instance through the guard. Unless the guard is disarmed, its `Drop` sets `unusable` to that text and calls `requests.drop_all()`. So the work is dropped at the moment the call future is dropped.
- **H11a. Every return from `wait()` drops the requests.** `wait()` returns through one exit, `WaitGuard::finish(self)`, on every path: status 0, a fault, a host failure (H9), a trap that `invoke_in` reports, a stop because of a host value that is not correct (`lower_reply` gives `Ok(false)`, and `wait` returns `host_value_stop_inner()`), and a trap inside `ilang_alloc` while `lower_reply` writes a result buffer (`Bridge::write` calls `alloc.call` directly, so this trap goes through `settle`, not `invoke_in`). `finish` calls `requests.drop_all()` and then disarms the guard. It does not touch `unusable`: the path that ends the call has already set its own text (the fault, the failure, the stop or the trap), or none at status 0. So the H11 text appears only when the call future is dropped, and no path leaves work futures alive in `State.requests` after `wait()` returns. A missing or wrongly typed `ilang_resume:…` or `ilang_continue` export is one of these faults: it sets `unusable` before it returns (H5, H14). T19 checks the host-value path, T22 the missing exports. The JS host has the same single exit (section 7).
- **H12. WASI.** `host_is_async` (lower.rs:937) is false on the WASI target, so there are no ids, no resume exports and no change to imports. WASI metadata still says version 3. The shared `host::EFFECTS` block changes in WASI modules as in every module (D7), but nothing in a WASI module calls it with a request.
- **H13. Cancel.** The runtime import `ilang:host/v1.cancel(id: i32)` (no results) drops the outstanding request `id`: the host drops its work, drops a finished answer that was not delivered, and never answers it. The id may be used again at once. It follows H2: called at another time, or with an id that is not outstanding, it traps and the host does not use the instance again ("the module cancelled request {id}, which is not outstanding"). A host gives `cancel` to every v3 module that imports it.
- **H14. Status 2, runnable.** Status 2 means the module can go on without an answer. The host then lets its IO run once without waiting (Rust: `tokio::task::yield_now().await`, so the `current_thread` runtime polls its timer and IO drivers; JS: one macrotask yield), and looks for a request whose work is done, without waiting for one. If there is one, it answers it as in H5, and the resume gives the next status. If there is none, it calls the export `ilang_continue` (no parameters, result `i32` status). Status 2 with no outstanding request is allowed. The host calls `ilang_continue` only in this case: when the last status was 2 and it gives no answer in its place. Calling it at any other time (after status 0 or 1, or with no async entry point running) is a host error that breaks the instance; a module may trap when that happens, and the hand-written fixture does (T22). Both in-repo hosts call it only from this branch of `wait()`. A module that returns status 2 must export `ilang_continue` with the type `() -> i32`; if it does not, it is a fault of the module ("the module is runnable, but has no ilang_continue"). The host checks this requirement as soon as it receives status 2, before yielding or choosing an answer, even if ready work would end the entry point without a call to `ilang_continue`. `ilang_continue` is a reserved export name, like `ilang_start` (§B.5): metadata may not list it as a host export or an entry point, and both hosts refuse a module whose metadata does (`is_reserved_export`, lib.rs:312; `reserved`, ilang.mjs:189). Compiled modules of stream C never return 2 and do not export `ilang_continue`.

## 4. Spec text, section by section

All in `docs/spec.md`, Appendix B, and only in §B.4–B.6, the sections the plan gives stream C (plan line 68). No renumbering. Phase 1 of C-I commits this text alone.

The other sections of Appendix B stay as they are, and the new text is written so that they stay correct:

- §B.1 lists the exports every module has. `ilang_continue` exists only in some modules, so §B.5 describes it, next to the entry points and `ilang_resume:…`, which §B.1 does not list either. The sentence "During `ilang_start`, imports may be called as at any other time" stays true: an async function may be called only during an async call, at any time, so not during `ilang_start` either; §B.4 says this in so many words.
- §B.3 needs no new sentence: §B.4 says that the host copies the parameters of an async function during the import.
- §B.7 (owned by the IO stream) already says that on WASI an async function is "a blocking import, with the results of a function that is not async", in a module that is "the module of §B.1 to §B.6 with these changes". §B.4 adds that this has no request id, so §B.7 needs no edit and the IO stream keeps it to itself.

### §B.4 Imports

Replace the bullet "Async functions of host services" with:

> - **Async functions of host services**: the import has an `i32` **request id** first, then the parameters of §B.2, and no results. The host copies the parameters, keeps the request, and returns at once. The answer comes later through `ilang_resume` (§B.5). Until then the request is **outstanding**.
>   - The host calls the service during the import, and waits for its work only after the import has returned. So a service can fix anything that depends on the time of the call, such as the deadline of a sleep, during the import. The parameters are copied then too.
>   - The module chooses the id: a number of 0 or more that no other outstanding request of the instance has. After a request is answered, cancelled or dropped (§B.5), its id may be used again.
>   - The module calls an async function only during a call of an async entry point, of `ilang_resume:…` or of `ilang_continue`, and not while an import runs (also not from a call such as `ilang_alloc` that the host makes during an import). So it does not call one during `ilang_start` (§B.1) either. If it calls one at another time, or with an id that is below 0 or already outstanding, the import traps and the host does not use the instance again.
>   - On the WASI target none of this holds: there, as §B.7 says, an async function is a blocking import, with no request id.

In the list of runtime imports, add:

> - `cancel(id: i32)` drops the outstanding request `id` (above): the host stops its work if it can and never answers it, and the id may be used again. It may be called at the same times as an async function. With an id that is not outstanding, it traps and the host does not use the instance again.

### §B.5 Exports

Replace the bullets from "Async entry points" to the end of the section with:

> - **Async entry points** (§10.2) have the parameters above and one result, an `i32` **status**. 0 means the entry point has ended. 1 means it is **waiting**: it has one or more outstanding requests (§B.4), and it goes on only when the host answers one. 2 means it is **runnable**: it can go on without an answer. Another status is a fault of the module.
> - When the status is 2, the host lets its own work run once without waiting, then answers a request whose work is done, if there is one. If there is none, it calls the export `ilang_continue() -> i32`, with no parameters, whose result is a status again. The host calls `ilang_continue` only then, right after status 2; a call at another time breaks the instance. A module that gives status 2 has this export; if it does not, it is a fault of the module. The names `ilang_continue`, `ilang_start`, `ilang_alloc`, `ilang_free`, `memory` and those that start with `ilang_resume:` or `ilang_result:` are reserved: the metadata may not use them for host exports or entry points.
> - The host answers a request by calling the export `ilang_resume:namespace:label` of the function that started it. The first parameter is the request id, and the others are the results of the function as in §B.2. The result is a status again. The host may answer outstanding requests in any order, but one at a time. It makes the result buffers with `ilang_alloc` just before the call that gives them. A module that starts a request of a function and has no such export for it, or one of the wrong type, has a fault, and the host does not use the instance again.
> - A call of `ilang_resume:…` with an id that is not outstanding, or that another function started, traps.
> - When the status is 1 and no request is outstanding, the program can never go on. This is a fault of the module, and the host does not use the instance again.
> - When the status is 0, the host calls the export `ilang_result:name` of the entry point, with no parameters, to get its results as in §B.2. Requests that are still outstanding are dropped: the host stops their work if it can and never answers them. This lets a program end work that it no longer needs; a module made by this compiler has no outstanding request at that point.
> - When the work of an async function fails in the host (not an error of its `^ E`, which is an answer), the host drops every outstanding request and does not use the instance again.
> - While an async entry point is waiting or runnable, the host may only call `ilang_resume:…`, `ilang_continue`, `ilang_alloc`, and `ilang_free`. So an instance runs one async entry point at a time. The module does not check every export for this; a host that calls another export breaks the instance.
> - When the program stops, the export traps after the `panic` import (§B.4). After a trap, the host may not use that instance of the module again. Every outstanding request is dropped, and an answer that comes later is ignored. The host makes a new instance and calls `ilang_start` on it.

### §B.6 Metadata

In the JSON example, `"version": 2` becomes `"version": 3`. Add this bullet first:

> - `version` is 3. A host refuses a module with another version, so a module of an older version must be built again. Version 3 differs from version 2 in these ways: an async function has a request id; status 1 means one or more outstanding requests, not exactly one, and the host may answer them in any order; status 2 and the export `ilang_continue` are new, and `ilang_continue` is a reserved export name; the runtime import `cancel` is new; outstanding requests are dropped when the entry point ends, on a trap, and when the work fails in the host; and an async function may only be called during an async entry point, a resume or `ilang_continue` (§B.4). A runtime import, status or export that a host must handle is added only with a new version.

### §B.1, §B.3, §B.7, §10.2 and §9.7

No change. None of them says "exactly one"; only §B.5 did. See the start of this section for why §B.1, §B.3 and §B.7 stay correct.

## 5. Host executor and request table (crates/ilang-host)

### 5.1 Cargo

`crates/ilang-host/Cargo.toml`:

```toml
[dependencies]
tokio = { version = "1.53.3", default-features = false, features = ["rt", "time"] }
futures-util = { version = "0.3.34", default-features = false, features = ["alloc"] }

[dev-dependencies]
tokio = { version = "1.53.3", default-features = false, features = ["rt", "time", "test-util"] }
```

Checked offline on a copy of `dcb19df` with `cargo metadata --offline`: the only new `Cargo.lock` package is `tokio 1.53.3` (its one dependency, `pin-project-lite 0.2.17`, is already locked). `futures-util 0.3.34` is already locked, so the lock only gains two lines in the `ilang-host` dependency list. `test-util` adds nothing to the lock. Both crates need Rust 1.71; the toolchain is 1.99.1. Update the lock with `cargo metadata --offline`, never `cargo update`. The wasmtime `async` feature stays off, so feature unification for `ilang-cli` and `ilang-binaryen` does not change.

### 5.2 run.rs (new)

```rust
thread_local! {
    static ON_ILANG_THREAD: Cell<bool> = const { Cell::new(false) };
    static WASM_DEPTH: Cell<u32> = const { Cell::new(0) };   // Wasm calls of the fast path that are running
}

/// Shuts the runtime down without waiting for its blocking pool, also when
/// the future panics. Dropping a `Runtime` normally waits, with no time
/// limit, for every `spawn_blocking` task.
struct Background(Option<tokio::runtime::Runtime>);
impl Drop for Background {
    fn drop(&mut self) { if let Some(runtime) = self.0.take() { runtime.shutdown_background() } }
}

pub fn block_on<F>(future: F) -> F::Output
where F: Future + Send, F::Output: Send
{
    std::thread::scope(|scope| {
        let thread = std::thread::Builder::new()
            .name("ilang".into())
            .stack_size(THREAD_STACK)
            .spawn_scoped(scope, || {
                ON_ILANG_THREAD.set(true);
                let runtime = Background(Some(tokio::runtime::Builder::new_current_thread()
                    .enable_time().build().expect("a tokio runtime")));
                runtime.0.as_ref().unwrap().block_on(future)
            })   // `runtime` drops here: shutdown_background
            .expect("a thread for i");
        thread.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
}

/// The fast path of `invoke_in`: on the `ilang` thread, and not inside
/// another Wasm call (so not from a host function, D5).
pub(crate) fn fast_path() -> bool { ON_ILANG_THREAD.get() && WASM_DEPTH.get() == 0 }

/// Runs `call` with WASM_DEPTH raised by one, also when it panics.
pub(crate) fn in_wasm<T>(call: impl FnOnce() -> T) -> T;
```

`lib.rs` re-exports `block_on` from `run.rs`, so `ilang_host::block_on` keeps its name. Its old body (lib.rs:797) goes away.

**How the runtime ends.** When the future of `block_on` is done (or panics), the runtime is shut down with `shutdown_background()`: the tasks that are left are dropped, and the threads of the blocking pool are not waited for. Futures of requests are already gone by then (H7, H8, H9, H11a drop them), so the only thing left is detached blocking work (D6), which runs to its end on its own thread and is then thrown away. So `i run` ends as soon as the program ends, also when the program has dropped a blocking read of stdin, and also after a stop. A blocking thread that is still running when the process exits is ended with the process. T23 checks this.

### 5.3 lib.rs

- Doc header (lib.rs:1): version 3; describe the `ilang` thread and the fallback.
- `ABI_VERSION = 3` (lib.rs:81). `Abi::validate` (lib.rs:259) gives the D9 messages.
- Comment at lib.rs:635: "`async_stack_size` is set only because `Config::validate` checks it against `max_wasm_stack`. The host does not use Wasmtime's async support."
- `State.pending : Vec<Pending>` (lib.rs:607) becomes `State.requests : Requests`.
- `invoke_in` (lib.rs:1162): if `run::fast_path()`, call `run::in_wasm(|| function.call(&mut *context, params, results))` directly. Otherwise (another thread, or a call from inside a host function on the `ilang` thread, D5) call `Handle::try_current().ok()` on the caller's thread, move the result into today's scoped thread, and enter it there (`let _entered = handle.as_ref().map(|h| h.enter());`) before the Wasm call. The error path calls `requests.drop_all()` instead of `pending.clear()` (lib.rs:1185).
- Every import wrapper made by `define_import` and the runtime imports starts with `let admitted = requests.take_accepting();`, which returns the flag and sets it to false, and puts `admitted` back on return, also on the error path (H2).
- `define_import` (lib.rs:1324): for an async service, the `FuncType` is `[I32] ++ signature.param_types()` with no results. Read the id from `parameters[0]`. **First** call `requests.admit(admitted, id)` (H1, H2). Only when it gives `Ok`, lift the B.2 parameters from offset 1, call the start closure, and pass its future to `requests.insert(id, request, work)`. Before returning from the import, check `ilang_resume : {ns} : {label}` with params `[I32] ++ signature.result_types()` and result `[I32]`. If it is missing or wrongly typed, set `state.failure` to the H5 fault and trap; the normal failure path drops every request and makes the instance unusable. Checking here prevents cancellation or status 0 from hiding the fault. So the start closure of a refused request never runs. On `Err` from `admit`, set `state.failure` with the text of the error and return the trap error, so the existing failure path marks the instance unusable. The `fail` closure (lib.rs:1343) keeps a failure that is already set (`get_or_insert`), so an outer import that only sees an inner trap does not hide the inner text (H2).
- A new runtime import `ilang:host/v1.cancel(i32)` calls `requests.cancel(admitted, id)`; on `Err` it fails like an async import with a bad id. Its wrapper saves and clears the flag like every other import wrapper.
- `is_reserved_export` (lib.rs:312) also reserves `ilang_continue` (H14).
- `call_async` (lib.rs:965) and `run_async` (lib.rs:1019) set `requests.accepting = true` only around the Wasm call of the async export, after the parameters are lowered. `call`, `run` and `ilang_start` leave it false (H2). At the start of `call_async`, an outstanding request cannot exist because `usable()` refuses; assert it as an internal error.
- `wait(name, status)` (lib.rs:1038). The body is an inner function that returns the outcome; the outer `wait` calls it and then `this.finish()` on every result (H11a):

```text
let mut this = WaitGuard::new(self)       // H11: Drop while armed sets unusable and calls requests.drop_all()
let outcome = this.wait_inner(name, status).await   // every `return` below leaves through here
this.finish()                             // H11a: requests.drop_all(), then disarm; unusable is not touched
outcome

wait_inner:
loop {
  let (id, request, reply) = match status {
    0 => return Ok(None)                                                            // H7
    1 => match requests.next().await {
           Some(done) => done,
           None => return fault("the module waits, but no host function is running") } // H4
    2 => { let continue = export("ilang_continue")   // check before yielding or choosing ready work
               .or_fault("the module is runnable, but has no ilang_continue")?  // () -> i32; H14
           tokio::task::yield_now().await                                           // H14
           match requests.next().now_or_never().flatten() {
             Some(done) => done,
             None => { status = call_with_accepting(continue, [])  // ilang_continue may start and cancel requests
                       settle(...)?; continue } } }
    other => return fault("status {other} of {name}") }                             // H4
  let answer = match reply { Err(failure) => { unusable = failure; return Err }     // H9
                             Ok(answer) => answer }
  let resume = export("ilang_resume:{ns}:{label}")   // already checked during the import (H5)
      .or_fault("the module does not export ilang_resume:{ns}:{label}")?   // sets unusable, not a plain Err
  let mut values = match lower_reply(answer) {   // allocates now, with accepting false (H5)
      Ok(Some(values)) => values,
      Ok(None) => return Ok(Some(host_value_stop_inner())),   // sets unusable as today
      Err(trap) => return settle(Err(trap)) }    // a trap in ilang_alloc: settle sets unusable
  values.insert(0, Val::I32(id))             // the id goes in at the call site
  status = call_with_accepting(resume, values)   // accepting = true only around invoke_in
  settle(...)?                               // a trap or stop sets unusable and returns
}
```

A fault sets `unusable` to its own text. `or_fault` is the one helper for this; `Instance::export`, which returns a plain `Err` (lib.rs:842), is not used on its own inside `wait()`. Because `finish` runs on every return, the guard's H11 text is used only when the call future is dropped, and no return leaves work futures behind.

- `Program::run_entry` (lib.rs:742), `Program::run_test` (lib.rs:767) and the new public `Program::run_test_with(&self, name, options)` wrap their bodies in `run::block_on(async { ... })`. `run_test` calls `run_test_with`.
- Docs of `call_async` and `block_on`: "Called from a thread that `block_on` did not start, each Wasm call starts its own thread with a stack of `THREAD_STACK` bytes.", the fallback contract of D5, and "`block_on` needs a `Send` future; with a future that is not `Send`, await `call_async` on your own executor."

### 5.4 requests.rs (new)

```rust
pub(crate) struct Request { namespace: String, label: String, signature: Signature }

struct Slot { work: Option<ServiceFuture>, waker: Option<Waker> }

pub(crate) struct Requests {
    accepting: bool,
    outstanding: HashMap<u32, (Request, u64, Arc<Mutex<Slot>>)>,   // u64: sequence number
    work: FuturesUnordered<Polling>,   // Polling: { id, sequence, slot } -> (u32, u64, Option<Result<Result<Value, Value>>>)
}

impl Requests {
    /// Returns the flag and sets it to false (H2). The wrapper puts it back.
    pub fn take_accepting(&mut self) -> bool;
    pub fn set_accepting(&mut self, value: bool);
    /// H1, H2: refuses when `admitted` is false, when id < 0, or when id is outstanding.
    /// Called before the service's start closure, so a refused request never runs it.
    pub fn admit(&self, admitted: bool, id: i32) -> Result<u32>;
    /// Adds a request that `admit` accepted, with the future of its start closure.
    pub fn insert(&mut self, id: u32, request: Request, work: ServiceFuture);
    /// The next finished request, or None when nothing is outstanding.
    /// The id is removed from `outstanding` before it is returned.
    pub async fn next(&mut self) -> Option<(u32, Request, Result<Result<Value, Value>>)>;
    /// H2, H13: refuses when `admitted` is false or `id` is not outstanding.
    /// Takes the work out of its slot and drops it now, wakes the wrapper, forgets `id`.
    pub fn cancel(&mut self, admitted: bool, id: i32) -> Result<()>;
    /// Drops the work of every slot now, then clears `outstanding` and `work`.
    pub fn drop_all(&mut self);
}
```

`Polling::poll` locks its slot. If the work is gone, it gives `None` (cancelled). Otherwise it stores the waker and polls the work; when the work is done it takes it out of the slot (so it is dropped then) and gives `Some(reply)`. The lock is never held while the host calls back into Wasm, so it cannot deadlock; it is only there because `ServiceFuture` must be `Send`.

The error texts of `admit`: "the module started request {id} outside an async call", "the module started request {id}, which is below 0", "the module started request {id} twice". `cancel` uses the H13 text for an id that is not outstanding, and "the module cancelled request {id} outside an async call" when `admitted` is false. `next` returns `None` at once when `outstanding` is empty. It skips a wrapper that gives `None`, and one whose id is no longer in `outstanding` or now belongs to a newer request with the same id (each entry keeps a sequence number, and the wrapper carries it). Because `cancel` drops the work inside the call, a cancelled future is always dropped before the resume that cancelled it returns, so before the next answer. A finished answer that was not delivered is a wrapper that is ready but not yet polled; `cancel` empties its slot, so its answer is never given, also when the id is used again at once. Stream R2 needs no change to this file: it keeps many requests outstanding, which the file already handles.

### 5.5 services.rs

- Add the D6 rule to the docs of `AsyncServiceFunction`: "The start closure runs during the import, and the future is first polled when the host waits; so work out deadlines and copy inputs in the start closure, and do not spawn tasks. When a request is dropped, its future is dropped at once; work handed to `spawn_blocking` (also inside `tokio::fs` or `tokio::io::stdin`) is detached, not cancelled: it runs to its end and its result is thrown away, so it must not hold anything the program still needs."
- Add to the docs of `ServiceFunction`: "A sync service runs inside the tokio runtime of `block_on`. It must not start or block on a runtime of its own (for example `Runtime::block_on` or `reqwest::blocking`); write it as an async service, or do that work on a thread of its own and join it." (D5)
- `standard()` does not change in C. The IO stream adds `i/clock`, `i/net` and `i/fs` later.
- `testing()` does not change, so the output order of `spec.rs` holds. The test services of section 8 live in `tests.rs`.

### 5.6 Stack depth and stop behaviour

On the `ilang` thread, a few KB of `block_on`, tokio and `wait()` frames sit below the first Wasm frame. Wasmtime measures `max_wasm_stack` from the Wasm entry, so the Wasm limit (`WASM_STACK`, 32 MiB) is the same, and the native stack (`THREAD_STACK`, 48 MiB) keeps a margin of 16 MiB minus those frames. This holds for one store. Wasmtime sets the limit per store, counted from that store's entry, so a second store entered from inside a host function would get its own 32 MiB on top of the first one's and could overflow the 48 MiB native stack, which aborts the process. `WASM_DEPTH` prevents this: such a call takes the thread-per-call path and gets a fresh 48 MiB thread, as every call did in v2 (T24). Calls into the same store from a host function (`ilang_alloc` and `ilang_free` in `Bridge`) do not go through `invoke_in`, run on the same stack under the same store limit, and are the same as in v2. To prove that nothing changes, the executor change lands as its own first commit under ABI v2, gated by the existing deep-recursion and "call stack exhausted" tests plus T11, before the version bump.

## 6. Module side (crates/ilang-wasm)

### 6.1 emit/host.rs globals

`$host_pending` (host.rs:21) is replaced by:

```wat
(global $host_import (mut i32) (i32.const -1))  ;; import of the outstanding request; -1 = none
(global $host_request (mut i32) (i32.const -1)) ;; its id
(global $host_next (mut i32) (i32.const 0))     ;; the next id; wraps at 2^31
(global $async_live (mut i32) (i32.const 0))    ;; 1 from $async_begin until status 0
```

`$async_live` is not part of the request table: it says that an async entry point has started and not ended, whether it waits (status 1) or is runnable (status 2, possibly with nothing outstanding).

### 6.2 emit/host.rs request table functions

```wat
;; The request table (B.4, B.5). In stream C it has one entry. R2 replaces
;; these four functions with a table of requests (see the comment below).
(func $req_start (param $import i32) (result i32)
  (local $id i32)
  (if (i32.ne (global.get $host_import) (i32.const -1)) (then unreachable))
  (local.set $id (global.get $host_next))
  (global.set $host_next (i32.and (i32.add (local.get $id) (i32.const 1)) (i32.const 0x7fffffff)))
  (global.set $host_import (local.get $import))
  (global.set $host_request (local.get $id))
  (local.get $id))

(func $req_answer (param $import i32) (param $id i32)
  (if (i32.or (i32.ne (global.get $host_import) (local.get $import))
              (i32.ne (global.get $host_request) (local.get $id)))
    (then unreachable))
  (global.set $host_import (i32.const -1))
  (global.set $host_request (i32.const -1)))

(func $req_outstanding (result i32)
  (i32.ne (global.get $host_import) (i32.const -1)))

;; The entry point has ended: the host drops what is left (B.5), so forget it.
(func $req_clear
  (global.set $host_import (i32.const -1))
  (global.set $host_request (i32.const -1)))
```

The traps are bare `unreachable`, with no new panic text, so stop messages stay byte-identical.

### 6.3 emit/host.rs code changes

- `host_start` (host.rs:99): `(call $imp{i} (call $req_start (i32.const {i})) <args>)`, then free the parameter buffers as today. The old check and the set of `$host_pending` go away. `host_call` (host.rs:111) gets an optional leading id expression, used only by `host_start`.
- `resume_exports` (host.rs:261):

```wat
(func (export "ilang_resume:ns:label") (param $id i32) <B.2 results> (result i32)
  (call $req_answer (i32.const {i}) (local.get $id))
  <global.set $hr{i}_j (local.get j+1)>
  (call $async_run (ref.null eq)))
```

- `$host_resume_k` (host.rs:47): delete the line that sets `$host_pending`; `$req_answer` clears the entry now. Its signature and its `return_call $control_resume(k, null, $root_env)` stay exactly the same. `$host_boundary` is not touched. So the seam with `control.rs` (only `$host_boundary` and `$host_resume_k`, host.rs:4) holds, and B can land before or after C.
- `$async_begin` (host.rs:56): trap on `(global.get $async_live)` instead of `$host_pending`, then set `$async_live` to 1. `$async_result` (host.rs:64): trap on `(global.get $async_live)`. Not `$req_outstanding`: with status 2 an entry point may be unfinished with nothing outstanding (D7).
- `$async_run` (host.rs:70; status 0 at :81 and :90): `(call $req_clear)` and `(global.set $async_live (i32.const 0))` on each path that gives status 0 (the value path and the error path), so the module's table matches the host's after H7. For a compiled module of stream C, `$req_clear` does nothing. After a trap `$async_live` stays 1, which is fine because the instance is not used again.
- Header comment of `EFFECTS` (host.rs:10): describe the request id and the table.

### 6.4 The R2 hand-off

This section has two parts: what stream R2's language design must give (C does not design it, but C's ABI must fit it), and the comment that C writes in `emit/host.rs` for the module side.

**One root continuation, not one per request.** Under §9.7 and the code at `dcb19df`, an async call is `HostStart` plus a suspend to `$host_root`, and `$host_boundary` wraps the whole entry point (host.rs:11-16, 39-45). So the `k` of an async call holds every frame up to the root. If `i/task` were built on that, the first `clock.sleep` inside one task would capture the scheduler and its run queue too, the module would return status 1, and a second task could never start. Keeping one root `k` per request and answering them in any order does not help either: each `k` holds its own snapshot of the scheduler's state (values never change, §11.1), so resuming B's `k` after A's brings back a scheduler that has not seen A's progress, and tasks run twice or are lost. So under R2 there is at most **one** root continuation at a time: the place where the scheduler waits for any answer. A task's own continuation is an ordinary i value that the scheduler holds, made with `control.boundary` and `suspend` (§9.4), as schedulers are meant to be.

**The general feature R2 needs: host requests as values.** For i code to start a request without waiting, wait for any of several, poll, and cancel one, the request must be a value. A sketch, for R2 to design and get approved (nothing here is decided): starting an async host function without waiting gives a value of type `Request(T)`, the set of outstanding requests whose answer has type `T`. Its operations are: wait for one of a list of requests (the module returns status 1 and resumes the one root `k` with which request ended and its answer), poll (status 2 and `ilang_continue`), and cancel (the runtime import `cancel`). The id stays inside the value and is never seen by i code; the compiler lowers the operations to the `$req_*` functions, `cancel` and the statuses. A plain call `f(x)` stays "start, then wait for this one request", so "the program waits at the call" (§9.7) stays true for code that does not use requests. `Request(T)` is an ordinary type constructor, so principal types are kept. It is useful in at least three unrelated places: an M:1 scheduler (`i/task`), a timeout or race around one call (start two, wait for the first, cancel the other), and starting several independent reads at once and joining them, with no tasks at all. Nothing in it names `i/task`, so the compiler gets no hook that only `i/task` uses. Whether a task's async call reaches the scheduler implicitly (async calls suspend to the nearest handler instead of `$host_root`, which changes §9.7) or the library wraps it in an explicit request is R2's question; either way the §9.7 change belongs to R2, not C.

**Ending a task runs its final functions.** Because a task's continuation is an i value held by the scheduler, ending a task (a timeout that cancels its read) or a task that is left over when the entry point is about to end is cancelled by the scheduler in i code, through the ordinary cancel path of `control` (§9.4 rule 6; track B covers cancellation through frames that have not resumed). So its final functions run. The host-side `cancel(id)` (H13) and the drop at status 0 (H7) only drop **host work**; they never hold a continuation. R2's scheduler must cancel every leftover task before the entry point returns. So the seam with `control.rs` stays `$host_boundary` and `$host_resume_k`, with no `$host_cancel_k`.

The comment that C writes in `emit/host.rs`:

> Stream R2 takes over the request table. These parts are correct only while one request is outstanding, and R2 must replace them together:
> - `$req_start`, `$req_answer`, `$req_outstanding`, `$req_clear` become a table from id to { import, answer }, with a free list. The id may be the slot index. `$req_clear` frees every entry; `$async_run` already calls it before status 0 (H7). `$async_result` and `$async_begin` check `$async_live`, not `$req_outstanding`, because an entry point that returns status 2 may have nothing outstanding and still not have ended; keep that check, and keep clearing `$async_live` only at status 0.
> - The answer globals `$hr{i}_*`: each resume must decode its answer into the entry of its request before control leaves the export, or a second answer of the same import overwrites the first.
> - `$host_k` stays **one** continuation: the root continuation of whatever waits (today the one async call; under R2 the place where the scheduler waits for any answer). `$host_resume_k` keeps resuming that one `k`, and passes it which request ended. Do not store a root continuation per request: each would hold its own copy of the scheduler's state.
> - The scheduler returns status 1 to the root only when no task can run. At its scheduling points, while some task can still run, it returns status 2 so that the host can give finished answers, and goes on in the export `ilang_continue`, which R2 adds to the module (H14). That export must trap when the last status the module gave was not 2.
> - Cancelling a request value lowers to the import `cancel(id)` from `ilang:host/v1` and frees the entry (H13). It drops host work only; the task's continuation is an i value, and the scheduler cancels it in i code so that its final functions run.
> None of this changes the ABI: v3 hosts already accept many outstanding requests in any order, status 2 and `cancel`. A need that v3 does not cover (a new runtime import, status or export) means ABI version 4.

### 6.5 Outside C's files (escalations)

These need the coordinator's approval:

1. `crates/ilang-wasm/src/emit/mod.rs` ~:141: for `ImportKind::AsyncService`, put `"i32"` first in `params`. One line. It changes only modules with async imports.
2. `crates/ilang-cli/src/lib.rs : 162` `run_test_with`: call the new `ilang_host::Program::run_test_with`, so each test runs on one `ilang` thread and the two copies cannot drift.
3. `crates/ilang-wasm/tests/codegen.rs : 103`: `"version": 2` becomes `"version": 3` in the expected metadata JSON. One line, forced by the version bump that the plan gives C; it belongs to no other stream's work.
4. `tests/spec/9.7/async_many_waits.i` (new file, phase 4): an i-side test of many waits in a row. `tests/spec/9.7` sits next to the spec section of the IO stream, so the file is listed here; it adds a file and edits none.

The spec edits stay inside §B.4–B.6, the sections the plan gives C (section 4), so they need no escalation. In particular §B.7 belongs to the last IO phase and is not touched. `docs/spec.md` Appendix B merges in the plan's order C → R2 → IO → M2.

`lower.rs` `make_import_func` needs no IR change (`HostStart`, `Suspend`, `HostReply` stay). `abi_json` (lower.rs:2680) writes `"version": 3`.

### 6.6 WAT expectations

- `(import "app:clock/v1" "later" (func $imp0 (param i32 f64)))`.
- Each `ilang_resume:…` has a leading `(param $id i32)`.
- Every module, sync, async and WASI alike (D7): in the `host::EFFECTS` block, the four new globals of section 6.1 and the four `$req_*` functions replace `$host_pending`, and `$async_begin`, `$async_result`, `$async_run` and `$host_resume_k` get their new bodies. This text is the same in every module. The version string in the custom section changes too.
- Modules with async imports, in addition: the import signatures and the `ilang_resume:…` exports above, and the calls of `$req_start` in `host_start`.
- `wat_expect.changed` lists the shared `EFFECTS` diff and the version change once, as they apply to every module, and lists separately the modules with async imports and their extra changes. The plan's WAT audit of the other lanes first takes out this one known `EFFECTS` diff (applied as a patch to their baseline) and then expects no other change.

## 7. JS bindings (bindings/javascript)

- `ilang.mjs`: `const version = 3` (ilang.mjs:15) and the header comment (ilang.mjs:2). `readAbi` (ilang.mjs:223) gives the D9 messages.
- The bridge: `pending = []` (ilang.mjs:359) becomes `requests = new Map()`, `settled = []`, `wake = null`, `accepting = false`, `depth = 0`.
- `enter(operation)` (ilang.mjs:418) raises `depth` before the operation and lowers it in a `finally`. `usable()` (ilang.mjs:406) also refuses while `depth > 0`, with "The instance is running Wasm; a host function may not call it (§B.3)" (D6). Every public call checks `usable()` before it reaches Wasm; the SDK's own `ilang_alloc` and `ilang_free` calls during an import do not.
- `guardedFunction` (ilang.mjs:106), which wraps every import, saves `const admitted = bridge.accepting`, sets `bridge.accepting = false`, calls the callback as `fn(admitted, ...raw)`, and restores `bridge.accepting = admitted` in a `finally` (H2). Every import callback gets `admitted` first; only the async import and `cancel` use it. So a call of `ilang_alloc` that the SDK makes during an import cannot start a request, while the async import itself still sees the value from before its own wrapper cleared it.
- The async import (ilang.mjs:688): `guardedFunction` params are `[0x7f, ...paramTypes(signature)]`. The callback takes `(admitted, id, ...raw)`. If `!admitted`, `!Number.isInteger(id) || id < 0`, or `bridge.requests.has(id)`, it calls `breakIn` with a `GuestFault` with the H2 texts, before `fn` is called (as the Rust host does, H2). Otherwise it lifts (copies) the arguments, makes `const abort = new AbortController()`, and calls the service **synchronously** (D6), with the request's signal as the current one for `requestSignal()`: `const saved = currentSignal; currentSignal = abort.signal; let work; try { work = Promise.resolve(fn(...args)); } catch (error) { work = Promise.reject(error); } finally { currentSignal = saved; }`, then maps it to `{ value }` or `{ error }` as today. The arguments are exactly the declared ones, as in v2, so default and rest parameters keep their meaning (D6). It stores `entry = { namespace, label, signature, abort }` and checks the required resume export before returning from the import (H5): params `[0x7f, ...resultTypes(signature)]`, result `[0x7f]`. Missing or wrongly typed means `breakIn` with the H5 module fault, so the import traps, every request is dropped and the instance becomes unusable. It attaches the settlement handlers even on this fault path, so a later rejected promise is handled. The handlers are:

```js
work.then(
  (outcome) => { if (bridge.requests.get(id) === entry) { bridge.settled.push({ id, entry, outcome }); bridge.wake?.(); } },
  (failure) => { if (bridge.requests.get(id) === entry) { bridge.settled.push({ id, entry, failure }); bridge.wake?.(); } });
```

- **Dropping requests.** One helper, `bridge.dropRequests()`, calls `entry.abort.abort()` for every entry left in `requests`, then clears `requests` and `settled`. It is called in a `finally` around the whole body of `wait()`, so **every** exit of `wait()` drops the requests, as `WaitGuard::finish` does in Rust (H11a): status 0 (H7), a trap through `trapped()` (H8), a host failure (H9), the host-value stop (`lowerReply` gives `null`, then `throw bridge.hostValueStop()`), a fault, and any other throw. `trapped()` also calls it, for traps outside `wait()`. `cancel` aborts the one entry it removes (H13). An entry is removed from `requests` before its answer is given, so an answered request is never aborted.
- `wait(name, status)` (ilang.mjs:726): a loop.
  - Status 0: `dropRequests()`, return.
  - Status 1 with `requests.size === 0`: throw `trapped(new GuestFault("the module waits, but no host function is running"))`.
  - While `settled` is empty: `await new Promise((r) => (bridge.wake = r))`.
  - Shift the first entry and `requests.delete(id)`. A failure that is not a `GuestError` sets `unusable`, calls `dropRequests()`, and rethrows.
  - The import has already validated `ilang_resume:${ns}:${label}`; the delivery path may check it again with params `[0x7f, ...resultTypes(signature)]` and result `[0x7f]`. If it is missing or has another type, throw `trapped(new GuestFault("the module does not export ilang_resume:…"))`, so the instance becomes unusable (H5); a bare throw from `validateFunction` would leave it usable while the module is still inside its entry point.
  - `values = lowerReply(...)`, then `status = bridge.enter(() => { bridge.accepting = true; try { return exports[resume](id, ...values); } finally { bridge.accepting = false; } })`.
  - Status 2 (H14): first validate `ilang_continue` (no params, result `[0x7f]`; missing or of another type is `trapped(new GuestFault("the module is runnable, but has no ilang_continue"))`, which makes the instance unusable). Then one macrotask yield (`setImmediate` in Node, a `MessageChannel` message in browsers, through one helper, `macrotask()`, whose browser branch the browser tests run, section 10.4), then take the first settled entry if there is one; otherwise call the validated `ilang_continue` the same way as a resume. This is the only place the SDK calls `ilang_continue` (H14).
- A runtime import `cancel` next to `reference_id` (ilang.mjs:643), with the callback `(admitted, id)`: it checks `admitted` (else `breakIn` with "the module cancelled request {id} outside an async call") and that `requests.has(id)` (else `breakIn` with the H13 text); then it deletes the entry, removes its settled answer from the FIFO, if any, and aborts its signal. A late settle is then ignored by the `requests.get(id) === entry` check, also when the id has been used again.
- `reserved` (ilang.mjs:189) also reserves `ilang_continue` (H14).
- `callAsync` and `runPoint` set `accepting` only around the async export call. `trapped()` (ilang.mjs:440) calls `dropRequests()`.
- JS cannot stop a promise. A dropped request's signal is aborted; a service that listens to it can stop its timer, `fetch` or socket. A service that ignores it keeps running and its side effects happen; only its answer is ignored. The README says so.
- `ilang.d.ts : 14`: `version : 3`; a new export `requestSignal(): AbortSignal`; the type of a service function does not change. README: "version 3"; rewrite the waiting paragraph: the module picks a request id for each async host function it starts; the SDK calls the service during the import and answers the requests in the order they finish; while it waits, and while Wasm runs, other calls are refused. Add the D6 advice: fix deadlines before the first `await`; call `requestSignal()` before the first `await` to get an `AbortSignal` that stops work that is dropped; do not call the instance from a service.

## 8. Fixtures and scripts

### 8.1 tests/browser/fixtures/host-abi.wat

Shared by the Rust and JS tests.

- Header says v3; metadata `"version":3` (host-abi.wat:247).
- `later` becomes `(param i32 f64)`. Existing exports pass an id from a counter global and keep their behaviour. `ilang_resume:app:clock/v1:later` takes `$id` first and checks it against the stored id.
- New hand-written exports (the compiler never keeps many requests outstanding in C):
  - `app/async.many(n : Float) -> Float`. It starts n `later` requests with sparse ids `k*37+5` and argument `x_k = k+1`. Linear memory keeps, for each id, a started flag and the expected value. Each resume checks that the id is started and not answered (else it panics with "fixture: bad id") and that the value is `2*x_k` (else "fixture: wrong answer"). It folds `h = (h*31 + k+1) mod (2^31-1)` in answer order. After each of the first n answers it starts one more request that **reuses the freed id** (with a new argument), while the others are still outstanding, until 2n requests have been answered. It returns status 1 until then. `ilang_result` gives `h`. The test computes the expected `h` from the order it chose.
  - `app/async.fan(n : Float) -> Float`: starts n `later` requests with ids `k*37+5` and `x_k = k+1`, with no further requests; checks and folds `h` as `async.many` does, and ends after n answers. Used by the timer test, where each answer must not start a new request.
  - `app/async.cancel(n : Float) -> Float`: starts n requests (n >= 2). On each answer it calls `cancel` for the outstanding request with the lowest id, if one is left, and folds `h` over the answered ones only. It panics with "fixture: cancelled request answered" if a resume arrives for a cancelled id. Ends when nothing is outstanding.
  - `app/async.yielding(m : Float) -> Float`: starts one request, then returns status 2. `ilang_continue` counts and returns status 2 until the answer has come, or status 0 with the count when it has. After m calls of `ilang_continue` with no answer it panics with "fixture: starved". It also exports `ilang_continue`.
  - `app/async.stuck`: returns status 1 with nothing started (H4).
  - `app/async.dup`: starts two requests with the same id (H2).
  - `app/async.negative`: starts a request with id -1 (H2).
  - `app/async.leftover`: starts 3 and returns 0 after the first answer (H7).
  - `app/async.stop_mid`: starts 3, then panics and traps on the first answer (H8).
  - `app/sync.starts`: a sync export that calls `later` (H2).
  - `app/async.nested`: sets a flag that makes the fixture's `ilang_alloc` call `later`, then calls a sync service that returns a `String`, so the host calls `ilang_alloc` during the import (H2).
  - `app/async.cancel_bad`: calls `cancel` with an id that is not outstanding (H13).
  - `app/sync.cancels`: a sync export that calls `cancel(0)` (H2).
  - `app/async.nested_cancel`: like `async.nested`, but the flag makes the fixture's `ilang_alloc` call `cancel` for the request that the export started first (H2).
  - `app/async.result_starts` and `app/async.result_cancels`: async exports that start one request, end with status 0 after its answer, and whose `ilang_result:…` export calls `later` or `cancel(0)` (H2: not during `ilang_result:…`).
  - `app/async.cancel_reuse(n : Float) -> Float`: starts n requests (n >= 2) with ids `k*37+5`. On each answer it cancels the outstanding request with the lowest id, if one is left, and **at once starts a new request with that same id** and a new argument (the expected value in memory changes with it), until 2n requests have started. It panics with "fixture: wrong answer" when a value does not match the current request of its id, which is what a stale answer of the cancelled work would give. Used with the gate's burst switch (section 10.1), so the cancelled request is often one whose answer is already finished but not delivered.
  - `app/async.continue_io(n : Float) -> Float`: returns status 2 with **no request outstanding**. Each call of `ilang_continue` then starts one `later` request, until n have started; the second call also cancels the request that the first call started. The first request has the argument 0, which the test gate services **hold** (section 10.1): they never release it, so it is still outstanding when the second call cancels it, whatever order the host uses. (Without the hold, a host that follows H14 may answer the first request before it calls `ilang_continue` again, and the cancel would then trap with the H13 text.) The other requests have arguments `k+1`. Each call returns 2 until n have started, then 1. Ends when every request that was not cancelled is answered. So requests are started and cancelled from inside `ilang_continue`.
  - `app/async.bad_status`: returns status 7 (H4).
  - The one `ilang_continue` export of the fixture dispatches on a mode global that each of `async.yielding` and `async.continue_io` sets. A second global keeps the last status that any export of the fixture gave; `ilang_continue` traps (bare `unreachable`) when it is not 2, so a host that calls `ilang_continue` at another time is caught (H14).
  - `app/deep(n)` and `app/async.deep(n)`: recursion that is not a tail call, n deep, in a sync call and after a resume.
- The metadata lists the new exports as host exports with `async` true or false. The entry point stays the sync `app/main.main`, and the list of tests does not change, so `entry_and_tests_run_by_name_each_test_in_a_new_instance` (tests.rs:349, "fixture main") holds.
- Two more small WATs, each with its own metadata. `build-abi-fixture.sh` adds both to its `for fixture in ...` loop, so a checked-in `.wasm` sits next to the other fixtures. The Rust tests read the `.wasm` with `include_bytes!`, as they read `host-abi.wasm` (tests.rs:4); the JS and browser tests load the `.wasm` as they load `host-abi.wasm`.
  - `tests/browser/fixtures/no-continue.wat`: one async host export, `app/async.runnable`, that returns status 2, and no `ilang_continue` export (H14). A second async host export, `app/async.no_resume`, starts one request of the async import `app:clock/v1.later`, for which the file has no `ilang_resume:app:clock/v1:later` export, and returns status 1 (H5). A sync host export `app/sync.nil` checks that a later call is refused. Phase 3, v3 metadata.
  - `tests/browser/fixtures/one-thread.wat`: for T14. Its entry point `app/main.main` is **async**, and it also lists one async test, `app/test.waits`. Both call the sync import `app:console/v1.print`, then wait for `app:clock/v1.later` three times in a row, calling `print` after each answer. It lands in phase 2 with v2 metadata (no id), and phase 3 moves it to v3 with the other fixtures.

### 8.2 Other fixtures

- `custom-imports.wat` has no `ilang.abi` metadata (its header says the tests give it), so the file itself does not change. Its version lives in `customAbi` in `tests/browser/fixture-abi.mjs : 8`, and the async import signatures in the WAT do not change because the module has no async imports.
- `tests/browser/fixture-abi.mjs`: `customAbi` gets `version : 3` (line 8). The file does not repeat the metadata of `host-abi.wat`, which lives in the WAT's own `ilang.abi` section, so nothing else in it changes. Its `hostImports` keep their shape; the tests give the gate service through `overrides.later`.
- `fixtures/README.md`: it is the one document of the fixture contract that both hosts share, so it is brought up to date in the same commit as the fixtures:
  - version 3, and fix the sentence that says the script passes `--opt 2`; it does not;
  - `later` takes the request id first, and `ilang_resume:app:clock/v1:later` takes it first and checks it;
  - one line for each new export of `host-abi.wat` in section 8.1: what it starts, what it checks, how it ends and which rule it tests;
  - the conventions that the tests depend on: ids `k*37+5`, arguments `k+1`, the argument 0 that test gates hold, the panic texts ("fixture: bad id", "fixture: wrong answer", "fixture: cancelled request answered", "fixture: starved"), the `ilang_alloc` flag behind `async.nested` and `async.nested_cancel`, and the `ilang_continue` mode and last-status globals;
  - the files table gains `one-thread.wat` and `no-continue.wat`, with their exports and what `build-abi-fixture.sh` builds from them.
- `compiled/src/app.i` does not change, so the stop position `app/app : 35 : 50` holds. `compiled.wasm` is rebuilt.

### 8.3 scripts/build-abi-fixture.sh

Add `one-thread` (phase 2) and `no-continue` (phase 3) to the `for fixture in ...` loop. Add one check after that loop: read the version from `"version": N` in `crates/ilang-wasm/src/lower.rs`, and compare it with `ABI_VERSION` in `crates/ilang-host/src/lib.rs`, `{\"version\":N,` in `host-abi.wat`, `one-thread.wat` and `no-continue.wat`, `version : N` in `tests/browser/fixture-abi.mjs` (this covers `customAbi`, the version of `custom-imports.wat`), `const version = N` in `bindings/javascript/ilang.mjs` and `version : N` in `bindings/javascript/ilang.d.ts`. Fail with "{file} says ABI version X, but the compiler writes Y" for each one that differs. Keep the format of the `featureProbe` sed line. Run the script after the emit change.

## 9. Files to change

| File | Region | Change |
|---|---|---|
| `docs/spec.md` | §B.4–B.6 only | section 4 (phase 1, alone) |
| `crates/ilang-wasm/src/emit/host.rs` | :1–90, :99–130, :261–282 | section 6.1–6.4 |
| `crates/ilang-wasm/src/lower.rs` | ~:2680 | version 3 |
| `crates/ilang-wasm/src/emit/mod.rs` | ~:141 | escalation 1 |
| `crates/ilang-wasm/tests/codegen.rs` | :103 | escalation 3: `"version": 3` in the expected metadata JSON |
| `tests/spec/9.7/async_many_waits.i` | new (phase 4) | escalation 4 |
| `crates/ilang-host/Cargo.toml`, `Cargo.lock` | dependencies | section 5.1 |
| `crates/ilang-host/src/run.rs` | new | section 5.2 |
| `crates/ilang-host/src/requests.rs` | new | section 5.4 |
| `crates/ilang-host/src/lib.rs` | :1, :81, :259, :607, :635, :742–790, :797, :965–1100, :1162–1195, :1324–1370 (with the `fail` closure, :1343) | section 5.3 |
| `crates/ilang-host/src/services.rs` | docs of `AsyncServiceFunction` and `ServiceFunction` | section 5.5 |
| `crates/ilang-host/src/tests.rs` | version strings, `raw()` calls with an id, new tests | section 10 |
| `crates/ilang-cli/src/lib.rs` | :162 | escalation 2 |
| `crates/ilang-host/README.md` | :4, :38, :133–139, new section at the end | version 3; the waiting paragraph says the host keeps many requests and answers them in the order they finish; the `block_on` paragraph says it runs the call on one `ilang` thread with a tokio runtime and needs a `Send` future; the fallback contract of D5; a new section "Changes in version 3" with the two breaking changes of D5 (`block_on` needs `Send`; a sync service may not start or block on its own runtime) |
| `bindings/javascript/ilang.mjs`, `ilang.d.ts`, `README.md` | | section 7 |
| `bindings/javascript/test/host.test.mjs`, `compiled.test.mjs` | | section 10 |
| `tests/browser/host.spec.mjs`, `tests/browser/fixture-abi.mjs`, `fixtures/*` (with the new `one-thread.wat` and `no-continue.wat`) | | sections 8, 10 |
| `scripts/build-abi-fixture.sh` | after the parse loop | section 8.3 |

Every version literal moves in one commit: `lower.rs`, `ABI_VERSION`, `ilang.mjs`, `ilang.d.ts`, `bindings/javascript/README.md`, `crates/ilang-host/README.md` (line 4 "interface version 2" and line 38 "a version other than 2"), `tests/browser/fixture-abi.mjs` (`customAbi`), `host-abi.wat`, `one-thread.wat`, `compiled.wasm`, the metadata JSON in `crates/ilang-wasm/tests/codegen.rs : 103`, the JSON in `tests.rs` and `host.test.mjs`. The tests that check that old and new versions are refused (T13 and the JS test "versions 1, 2 and 4 refused") do not write the numbers 2 and 4: they build them from the current version, as `ABI_VERSION - 1` and `ABI_VERSION + 1` in Rust and `base.version - 1` and `base.version + 1` in JS, and they write version 1 as `1`. So they need no v2 literal and do not have to change at the next bump. Before the commit, `grep -rnE '"version": *2|\\"version\\": *2|version: *2|version = 2|version other than 2|interface version 2' crates bindings tests scripts --exclude-dir=ilang-binaryen` must print nothing. The second pattern matches the escaped form `\"version\":2` inside the metadata strings of the WAT fixtures. The check in `build-abi-fixture.sh` (section 8.3) compares the same files with the compiler's version, so the two checks cover each other. `crates/ilang-binaryen/src/tests.rs : 38` and `:110` are left out on purpose: they use `{"version":2}` only as opaque bytes of a custom section, to check that the optimizer keeps `ilang.*` sections unchanged, and no host reads them. They are outside C's files and do not change. `tests/wasi.rs` needs no edit; it reads the version through `Abi::from_wasm`.

The repository has no changelog file. The breaking changes of D5 go in the "Changes in version 3" section of `crates/ilang-host/README.md` and in the message of the ABI v3 commit.

## 10. Tests

### 10.1 Test services (Rust, in tests.rs)

- **Gate.** The order is chosen at answer time, among the requests that are outstanding then, never from a fixed permutation of all starts. A fixed permutation can hang: `async.many` keeps only n requests outstanding, so a rank can belong to a request that has not started yet (seed 0 gives n = 1; with the permutation [1, 0] the first request waits for a rank that only the second, unstarted request has). The shared controller keeps a list of live gates. The start closure adds the gate to this list (D6: it runs during the import, so the list is complete before the host waits, whatever order the futures are polled in). When a gate is polled, no gate is released, and `live` is not empty, the controller releases `live[rng() % live.len()]` with a seeded xorshift, records it in the release log, and wakes it if it has a stored waker. A gate that is not released stores its waker and returns `Pending`. The released gate, when polled, removes itself from `live`, clears the release and wakes every stored waker, so the next poll releases the next gate. **A gate's `Drop` does the same cleanup**: it removes the gate from `live`, and if the gate was released and had not completed, it clears the release and wakes every stored waker. So a gate dropped by `cancel` (which drops the work at once, section 5.4) or by `drop_all` (status 0, as in T8) is never chosen later, and a release never points at a gate that is gone. When `live` is empty (only held gates are outstanding), the polled gate just stores its waker and returns `Pending`; the controller never computes `% 0`. Since only one gate is released at a time, the answer order is exactly the release log, with no timers, and a choice always exists while a gate that is not held is outstanding, so it cannot hang.
- **Held gates.** A gate whose argument is 0 is held: it is never put in `live`, so it is never released, and it ends only when it is dropped. Only `async.continue_io` uses the argument 0 (its first request, which it cancels); every other fixture export uses arguments of 1 or more. A held gate never blocks the choice, because it is not in `live`, and polling it when `live` is empty releases nothing (T21 reaches this: the first `ilang_continue` starts only the held request, and H14's `now_or_never` polls it). The JS driver does the same: a request with the argument 0 gets a resolver that is never put in `live`.
- **Burst switch.** A test switch makes the controller release two gates at once (or all, when fewer than two are live) and choose again only when both have completed or been dropped. The second released gate is then a finished answer that has not been delivered while the first one's resume runs. The answer order is the order in which the host takes them, which the gates record when they complete. The controller counts gates that were dropped while released and not completed; tests that use the burst switch check that this count is above 0 over the 200 seeds, so the state "cancelled while the answer is finished" is reached. In the JS driver the same switch resolves two resolvers and then yields one macrotask, so both settles are in the FIFO before the first resume runs.
- **Drop guard.** Each test future has a guard that counts drops and records whether the future had completed. Tests check two numbers: all drops, and drops of futures that had not completed. `FuturesUnordered` drops a future when it completes, so all drops = starts once everything is dropped.
- **Countdown.** Ready after a seeded number of polls; it calls `wake_by_ref` each time it returns `Pending`. The tests check only that the module's hash equals the hash of the order that the host recorded, because the order depends on how `FuturesUnordered` polls.
- **Timer.** `tokio::time::sleep_until(deadline)` with the deadline fixed in the start closure (D6), run with paused time. The delay comes from the argument: `(n + 1 - x)` ms for `x = k+1`, so it is 1 to n ms and never 0 or negative; it is computed in `f64` and checked to be at least 1 before it becomes a `Duration`.
- **Ready.** A future that is ready on its first poll. Used with `async.yielding`.
- **Start counter.** Every test service counts calls of its start closure (and the JS services count calls of `fn`), so the tests can check that a refused request never ran its service (H2).

### 10.2 Rust (crates/ilang-host/src/tests.rs)

- **T1 `many_requests_complete_out_of_order`**: for seed in 0..200, n = 1 + seed % 16, gate service. Call `app/async.many(n)` through `block_on`. Check `h` against the value computed from the release log, that 2n futures were dropped and none before it completed, and that the instance stays usable (a second call reuses ids).
- **T2 `timers_answer_through_the_reactor`**: 200 repeats with paused time (`tokio::time::pause` in the runtime of the test), n = 8, timer service, `app/async.fan(n)` (which starts no more requests). Check the value and that the recorded order is `n-1 … 0`. Paused time makes it fast and stable under parallel test threads.
- **T3 `a_stop_drops_outstanding_requests`** (`stop_mid`): `Outcome::Stop` with the message; before the outcome is returned, 3 drops in all, 2 of them not completed; the next call is refused.
- **T4 `a_host_failure_drops_the_others`**: one gate future gives `Err`; `Err` is returned, all others are dropped, the instance is unusable.
- **T5 `status_one_with_nothing_outstanding_is_a_fault`** (`stuck`).
- **T6 `duplicate_or_negative_id_traps`** (`dup`, `negative`): the start counter shows 1 start for `dup` (the first request only) and 0 for `negative`.
- **T7 `a_sync_call_may_not_start_requests`** (`sync.starts`; `ilang_start` of a small WAT that starts one; and `async.nested`, where the request comes from `ilang_alloc` during a sync import of an async call). Each traps and makes the instance unusable, and the reported error is the H2 text ("the module started request {id} outside an async call"), also in the nested case, where the outer sync import must not replace it with "the module trapped in ilang_alloc". The start counter of `later` is 0 for each: the check comes before the service runs. The same test covers `cancel` and `ilang_result:…`: `sync.cancels`, `async.nested_cancel` and `async.result_cancels` give "the module cancelled request {id} outside an async call", and `async.result_starts` gives the start text. So a host that checks `admitted` in the async import but forgets it in `cancel` fails here.
- **T8 `leftovers_are_dropped_at_status_zero`** (`leftover`): `Ok`, 3 drops in all, 2 not completed, usable afterwards, and the next `app/async.many` call works (the module table was cleared).
- **T9 `a_wrong_resume_traps`**: raw resume calls with an unknown id, a stale id and the wrong label; each is a trap with no stop message.
- **T10 `dropping_a_waiting_call_makes_the_instance_unusable`**: poll `call_async("app/async.many", 3)` once with a gate controller that releases nothing (a test switch), so 3 requests are outstanding. Drop the call future while the `Instance` is still alive; check that 3 futures were dropped, none completed, right after the drop, and that the next call is refused with the H11 text.
- **T11 `deep_recursion_after_a_resume_stops_like_a_sync_call`**: the trap texts of `app/deep(N)` and `app/async.deep(N)` are equal, and a moderate depth succeeds in both. Lands in the first (v2) commit with a v2 version of the fixture export.
- **T12 `block_on_works_inside_a_tokio_runtime`**: no park and no panic; and `call_async` awaited on a user's own runtime thread still works through the fallback. A second part runs the timer service through `call_async` on a user's own `current_thread` runtime with time enabled (the fallback path, D5). In phase 2, under ABI v2, it uses `app/async.later`, the one async export the v2 fixture has, and checks the value. Phase 3 changes it to `app/async.fan(4)`, which needs the v3 fixture, and checks the value and the order. This covers the handle that `invoke_in` takes on the caller's thread and the fallback contract.
- **T13 `old_and_new_versions_are_refused`**: versions 1 and 2 give the rebuild message; version 4 gives the update-the-host message.
- **T14 `run_entry_uses_one_thread`**: on `one-thread.wasm` (section 8.1), the sync service `print` and the start closure of `later` record `std::thread::current().id()` on every call. Run the async entry point with `Program::run_entry` and the async test with `Program::run_test_with`. In each run, all recorded ids are equal (4 prints and 3 starts per run), and the run gives `Ok`.
- **T15 `futures_are_send`**: compile-time checks, for example `fn send<T: Send>(_: &T) {}` on the futures of `Instance::run_async` and `call_async`, and the same check on `ServiceFuture`, `Outcome`, `TestResult` and `Value`. No new crate is needed.
- **T16 `a_sync_service_may_block_on_its_own_thread`**: a sync service that builds its own tokio runtime on a thread it starts and joins works under `Program::run_entry`. The same service without the thread panics with tokio's message, which the test expects, so the breaking change of D5 stays visible.
- **T17 `cancel_drops_one_request`** (`async.cancel`, gate service, 200 seeds): `h` matches the release log of the answered requests; drops: n in all, the cancelled ones not completed, each dropped inside its `cancel` call (the gate records the number of answers delivered when it is dropped, and it equals the number at the time of the `cancel`); no cancelled id is answered. Also `async.cancel_bad` traps with the H13 text.
- **T18 `status_two_gives_ready_answers`** (`async.yielding`, ready service): ends with status 0 and a count below m. With a timer service in real time (1 ms) and m = 10^7, it also ends well before m, because the yield lets the timer driver run. Paused time is not used here: it only moves forward when the runtime is idle, and a module that keeps returning status 2 never lets it be idle.
- **T19 `every_return_from_wait_drops_the_requests`**: `app/async.fan(3)` with `check_replies` off and a gate service whose first answer is a `String` where `later` gives a `Float`. The outcome is the host-value stop, with its own text; before the outcome is returned, 3 futures were dropped and 2 of them had not completed; `unusable()` gives the host-value text, not the H11 text.
- **T20 `cancel_and_reuse_with_a_finished_answer`** (`async.cancel_reuse`, gate service with the burst switch, 200 seeds): no "fixture: wrong answer" panic; the module's `h` equals the hash of the order the host delivered answers; every work future is dropped; the count of gates dropped while released and not completed is above 0 over the 200 seeds.
- **T21 `requests_from_ilang_continue`** (`async.continue_io`, gate service with held gates, 200 seeds, n = 2 + seed % 8): status 2 with nothing outstanding goes on to `ilang_continue`; requests started from `ilang_continue` are answered; the first one, held, is cancelled by the second `ilang_continue` call, dropped inside that call without completing, and never answered; `h` matches. A host that calls `ilang_continue` without setting `accepting` fails here with "outside an async call".
- **T23 `a_finished_program_does_not_wait_for_blocking_work`**: the blocking work must really be running before the program ends, or the test cannot fail. `FuturesUnordered` polls in push order and `next()` returns at the first ready future, so blocking work that is only spawned inside the future of a later request would never be spawned. So the `later` service calls `tokio::task::spawn_blocking` **in its start closure** (which runs on the `ilang` thread inside the runtime, D6) for every request after the first; the blocking closure raises a shared counter and then sleeps 30 s, and the future awaits the `JoinHandle`. The future of the first request waits, with `yield_now` between polls, until the counter is 2, and only then answers. `app/async.leftover` then ends with status 0 while two blocking closures are sleeping; `block_on` around the call must return in under 5 s, and the test checks that the counter is 2. A second part does the same with `app/async.stop_mid`, which stops on the first answer. The test fails if `Background` is replaced by a plain drop of the `Runtime`, which waits for the blocking pool; C-I checks this once by hand. This checks `shutdown_background` (section 5.2).
- **T24 `a_nested_store_gets_its_own_thread`**: inside `block_on`, an outer instance of the fixture calls `read_through`, and its `read` service holds a second instance and calls `app/deep(N)` on it, with N deep enough to pass `WASM_STACK`. The inner call must end with the clean trap text of a stack overflow, the same as `app/deep(N)` called at the top level. Before that, the `read` service calls `app/main.main` on the inner instance, whose `print` service records its thread id; that id must differ from the `ilang` thread's. (The fixture cannot also put the outer call deep in its own stack first; the thread id is what shows that the inner store no longer shares the outer stack.) Lands in phase 2.
- **T22 `bad_statuses_are_faults`**: `async.bad_status` (status 7) and `no-continue.wat` (status 2 with no `ilang_continue`) are faults with the H4 and H14 texts, and the instance is unusable afterwards. A module whose metadata lists `ilang_continue` as a host export is refused at load time. `no-continue.wasm`'s `app/async.no_resume` (a request of an import with no `ilang_resume:…` export) is a fault with the H5 text, and afterwards `instance.call("app/sync.nil")` is refused because the instance is unusable, so no Wasm runs inside the unfinished entry point (H10). A raw call of the fixture's `ilang_continue` after `app/async.later` returned status 1 traps (H14). Also test status 2 with ready work whose valid resume would return 0, and a started request followed immediately by status 0 or cancellation, with missing and wrongly typed required exports. Each is a fault before delivery or result extraction, drops requests, and refuses a later sync call (§B.5, H5, H14). Each raw trap assertion uses a separate instance, with no later answer or successful call on it (§B.5).

### 10.3 JS (bindings/javascript/test, node --test)

- `host.test.mjs`: a gate made of stored resolvers; the start of each request adds its resolver to a `live` list. The test starts the call, yields one macrotask, and then, while `live` is not empty, resolves `live[rng() % live.length]` (removing it), **one at a time, with a macrotask yield (`setImmediate`) after each**. So every settle reaches the FIFO and its resume runs (and may start a new request) before the next choice, whatever the depth of the promise chains in the SDK. The release log gives the expected `h`. A choice is made only among requests that exist, so the test cannot hang. The same 200-seed loop as T1; T3–T10 where they apply, T17 to T22 (T19 with an answer of the wrong type, which also checks that the signals of the two other requests are aborted before `callAsync` settles and that their late settles are not pushed into `settled`; T20 with the burst switch; T21 with a held resolver for the argument 0; T22 with `no-continue.wasm`, both missing exports); T6 and T7 check the H2 texts and that `fn` was never called, also for `sync.cancels`, `async.nested_cancel`, `async.result_starts` and `async.result_cancels`; versions 1, 2 and 4 refused; the base metadata at version 3. More tests:
  - D6: a service records a mark when it is called, and the mark is set before the export call of the async entry point returns.
  - No re-entry during an import (D6): a `later` service that calls `guest.call("nil_roundtrip")` during the import, and one that calls `guest.callAsync("app/async.later", [2])`. In both, `callAsync("app/async.later", [1])` rejects with the `bridge.depth` text, no second request is started, and the instance is unusable afterwards.
  - The calling convention is unchanged: a `later` service with a default parameter, `(x, scale = 2) => x * scale`, gives `2x` (no signal fills `scale`). `requestSignal()` called after the first `await`, or outside a service, throws its text.
  - Dropped work gets an aborted signal: a `later` service that records `requestSignal()` for each request. After `async.cancel`, every cancelled request's signal is aborted, by the time the `cancel` import returns, and no answered request's signal is. After `async.leftover` (status 0) and `async.stop_mid` (a stop), the signals of the requests left over are aborted before `callAsync` settles.
  - The status-2 tests (T18, T21) also pass under Node with the helper's `setImmediate` branch; the `MessageChannel` branch runs in the browser tests.
- `compiled.test.mjs`: resume calls get an explicit id: `(0, 0, 0, 0)` and `(0, 0, 1, 0, 0)` (the first request of a fresh instance has id 0), still expecting `RuntimeError` where they do today; a stale id must trap.

### 10.4 Browser (tests/browser/host.spec.mjs)

In chromium, firefox and webkit, each test inside a single `page.evaluate`, with the same `live` list driver as `host.test.mjs`. The driver's macrotask yield is a `MessageChannel` message, not `setTimeout`, so there is no 4 ms clamping.

- "many requests complete out of order": `async.many`, 200 seeds.
- "status 2 and cancel": `async.yielding` (a ready service), `async.continue_io` (held resolver, 50 seeds) and `async.cancel_reuse` (burst switch, 50 seeds). These return status 2 or cancel, so they run the SDK's own `macrotask()` helper on its browser branch (a `MessageChannel` message), which no Node test reaches. A broken browser branch hangs or fails here; each test has a time limit.

The two existing async tests stay.

### 10.5 Unchanged tests that must still pass

`commands.rs` `the_compiled_fixture_is_current` (version 3 on both sides), `spec.rs` output order with `test:async/v1`, `wasi.rs` `async_services_are_blocking_calls` and `ilang_stop`, `tests/spec/9.7/async_services.i`.

### 10.6 Gate

`cargo test -p ilang-host -p ilang-cli -p ilang-wasm`; `npm test`; `npm run test:browser`; `dev-verify` rust and js; the WAT diff shows only the changes of section 6.6. C-RF runs T1 in a shell loop 200 times with `--test-threads=1`, as the plan's regression view asks.

## 11. Phases

1. Spec text of section 4, one commit.
2. Executor under ABI v2: `run.rs` (with `WASM_DEPTH` and `shutdown_background`), the fast path in `invoke_in`, `Program::run_*` through `block_on`, `Program::run_test_with` (and escalation 2), `one-thread.wat` at v2, T11, T12 (its fallback part on `app/async.later`, section 10.2), T14, T15, T24. T23 needs the v3 fixture exports and lands in phase 3. Gated by the existing tests alone, so a change in stack or stop behaviour shows up before the ABI changes.
3. ABI v3: module side, request table, Rust host, JS, fixtures, the remaining tests. These depend on each other through the version, so they land together; several commits, one gate at the end.
4. After A1 and A2 land (plan, C-I): the rebase agent rebases the branch onto the new `redesign`. A2 owns every existing line of i code, so A2 rewrites `tests/browser/fixtures/compiled/src/app.i` and `tests/spec/9.7/async_services.i` in the new syntax. After the rebase C rebuilds `compiled.wasm` with `build-abi-fixture.sh`, and updates the stop position `app/app : 35 : 50` in the host tests (section 8.2) if A2 moved it. C then adds its own i-side tests in the new syntax: `tests/spec/9.7/async_many_waits.i`, an async entry point that waits for `test:async/v1` 1000 times in a row and checks the sum, so the id counter moves on and one thread serves many waits. In the last milestone C splits `crates/ilang-host/src/services.rs` into `services/mod.rs`, with one empty file for each of the five IO phases of the plan: `clock.rs`, `fs.rs`, `net.rs`, `process.rs` and `stdio.rs`, each already declared in `services/mod.rs`. (`i/random` has no IO phase and so no file; if IO needs one, it adds it with its module line.) Then C hands these lines of its own files to the IO stream, which may change them without asking:
   - the runtime builder line in `run.rs` (`new_current_thread().enable_time()`), which becomes `enable_all()` when net or process lands; `tokio::process` on Unix needs the IO and signal drivers, or child reaping panics;
   - the `tokio` features list in `[dependencies]` of `crates/ilang-host/Cargo.toml` (`net`, `io-util`, `process`, `io-std`, `fs`, `signal` as needed), with the lock updated offline as in section 5.1;
   - the module list in `services/mod.rs` and the body of `standard()`.
   The plan's IO file list (plan line 73) says only "new files under `services/`"; the coordinator adds these three to it when C's last milestone lands.

## 12. Risks

- **Fibers left out.** This departs from the plan and needs approval (D4). If the user still wants fibers, an all-async store spike must come first.
- **Stack below the first Wasm frame.** A few KB of tokio and wait frames now sit below Wasm on the `ilang` thread. The 16 MiB native margin covers them; T11 and the phase 2 gate confirm it.
- **`block_on` needs `Send`.** A breaking change for an embedder that passes a future that is not `Send`. T15 checks the in-repo futures; the docs say what to do instead.
- **Sync services run inside a tokio runtime.** A breaking change for an embedder whose sync service starts its own runtime or uses a blocking client: it now panics (D5). The docs of `ServiceFunction` and the "Changes in version 3" section of `crates/ilang-host/README.md` say what to do; T16 keeps it visible.
- **Status 2 and `cancel` are tested only by the fixture.** No compiled module uses them before R2. The fixture tests of both run on both hosts, and in three browsers (section 10.4, which also runs the browser branch of the SDK's macrotask helper), so the host side is covered; the module side is R2's work.
- **Nested stores.** A Wasm call made from inside a host function takes the thread-per-call path (D5), so it costs one thread as in v2. This is rare (an external that runs another instance) and keeps its stack safe; T24 covers it.
- **Detached blocking work.** Work a service hands to `spawn_blocking` keeps running after its request is dropped, until it ends on its own (D6). `block_on` does not wait for it (T23), but it still uses a thread of the blocking pool until then. The IO stream's services must follow the rule in `services.rs`.
- **The fallback path.** Embedders that await `call_async` on their own threads still pay one 48 MiB thread per Wasm call. Only `block_on` and `Program::run_*` get the fast path.
- **Work runs when the host waits.** Both hosts call the service during the import (D6), but a Rust timer made inside the future, or a JS timer made after the first `await`, counts from the first poll. The advice in `services.rs` and the JS README says so, but nothing enforces it.
- **The fallback needs the caller's tokio runtime.** On the fallback path, a service that uses tokio panics unless `call_async` is awaited inside a tokio runtime with time (and later IO) enabled. The docs say so; T12 covers the case that works.
- **A long Wasm computation blocks the reactor.** Wasm runs on the reactor thread, so timers and later network IO wait while pure computation runs. This is inherent in M:1 and would be the same with one fiber.
- **One outstanding request in the module.** `$req_start` traps on a second request, and answers use the `$hr` globals of each import. Only the hand-written fixture keeps many requests outstanding. R2 must replace the table and the answer globals together, keep `$host_k` as one root continuation, and design host requests as values (section 6.4).
- **Host-only enforcement of H10.** A careless third-party host that calls a sync export while waiting can corrupt the module's state. The spec says this breaks the instance. Both in-repo hosts refuse: Rust by its borrow, JS by `bridge.waiting` and `bridge.depth`. The module's own `$async_live` check catches a second async entry point and an early `ilang_result:…`.
- **JS cannot stop dropped work.** The SDK aborts the request's `AbortSignal`; a service that ignores the signal keeps running, and its side effects still happen. Only the answer is ignored.
- **Version bump.** Every checked-in v2 artifact breaks at once. `the_compiled_fixture_is_current` compares only metadata, so a stale import signature would not be caught if the version did not also change. The new check in `build-abi-fixture.sh` catches part of this.
- **Escalations.** `emit/mod.rs`, `ilang-cli/src/lib.rs`, `crates/ilang-wasm/tests/codegen.rs` and the new `tests/spec/9.7/async_many_waits.i` are outside C's files (section 6.5). Without escalation 1 the ABI change cannot land; without escalation 2 `i test` keeps one thread per Wasm call; without escalation 3 `cargo test -p ilang-wasm` fails after the version bump. The spec edits stay in §B.4–B.6.
- **Test time.** 200 seeds in three browsers. Each round uses `MessageChannel` yields and no timers, so a round takes well under a millisecond per request.

## 13. Open questions

1. Does the user approve leaving out Wasmtime fibers (D4), and may the plan line be changed as in "Approval needed"?
2. Does the coordinator approve the four escalations of section 6.5 (`emit/mod.rs` ~:141; `ilang-cli/src/lib.rs : 162`; `crates/ilang-wasm/tests/codegen.rs : 103`; the new `tests/spec/9.7/async_many_waits.i`)? The spec text stays inside §B.4–B.6 and needs none.
3. Should `block_on` keep a second form that runs on the current thread with no `Send` bound, for embedders? This design says no and points them to `call_async`.
4. Does the user agree to put `cancel` and status 2 with `ilang_continue` into v3 now (D12), so that R2 needs no version 4? The other choice is to leave them out and bump to version 4 in R2; then a v3 host fails on an R2 module with the D9 "update the host" message, which is correct but means two bumps in a row.
5. Does the user agree with the R2 direction of section 6.4: one root continuation (the scheduler's wait point), task continuations as i values, and host requests as values (`Request(T)`) as the general feature that R2 designs? This changes §9.7 in R2, not in C.
