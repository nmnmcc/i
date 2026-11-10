# Design: host concurrency and ABI v3

Status: proposal for review (stream C, step C-D). Base commit `dcb19df`. All file and line references are at that commit. Decision D4 departs from the approved plan and needs the user's explicit approval before C-I starts (see "Approval needed").

## Summary

- An async host import gets one more Wasm parameter, placed first: an `i32` **request id** that the module picks.
- The resume export becomes `ilang_resume:ns:label(id, results...) -> status`.
- Status 1 now means "waiting for one or more outstanding requests". A host may answer outstanding requests in any order, one at a time.
- Compiled modules still keep at most one request outstanding in this stream. Many outstanding requests from one module come with `i/task` in stream R2, and R2 needs no ABI change, because the hosts already accept many requests in any order.
- The ABI version becomes 3 everywhere. Hosts refuse every other version with a clear message.
- The Rust host stops starting one OS thread for every Wasm call. `i run` and `i test` run each program on one long-lived thread with a big stack, and that thread also runs a tokio `current_thread` runtime. This runtime is the reactor that `i/clock`, `i/net` and `i/fs` need later.
- Wasmtime fibers (its async support) are not used. Stack depth, trap texts and stop messages stay byte-identical.

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

A request ends in one of three ways: its resume is called; the async entry point ends (status 0) while it is still outstanding; or the instance traps or the host fails. In the last two cases the host drops the work and never answers it.

Reason: dropping at status 0 lets R2 end a task without a cancel import. A `cancel(id)` runtime import is a planned R2 addition, not part of v3: no module needs it before R2, and an import that no compiler emits would be untested.

### D4. No Wasmtime fibers (needs approval)

Reasons, from the code:

1. A wait does not keep a Wasm frame alive. A wait is `Op::Suspend` to `$host_root` (lower.rs `make_import_func`, ~:1140). The continuation is captured with exceptions and the export returns status 1. So no Wasm frame is live while the host waits. A fiber is only needed when a host function must suspend in the middle of a Wasm call, and no planned service needs that, because `i/net`, `i/fs` and `i/clock` are async imports.
2. Fibers break the synchronous re-entry into the module. In Wasmtime 50, `Store::validate_sync_call` (wasmtime `runtime/store.rs:2270`) fails with "cannot do blocking `call` from an async context" when it is called on a fiber. Once any `func_new_async` exists, it fails with "store configuration requires that `*_async` functions are used instead". `Bridge::write` (lib.rs:1499) and `Bridge::free_all` (lib.rs:1525) call `ilang_alloc` and `ilang_free` synchronously from inside host functions, for every sync service or external that returns a `String`, `Int` or `BitArray`. With fibers, every path would have to become async: `instantiate_async`, `ilang_start`, `call`, `run`, the tests' `raw()` helper, and every alloc inside a host function. That is a large change with no gain.
3. Stack and stop behaviour stay the same without fibers (D5).

Fibers can be looked at again if a later feature needs a host function to wait inside a Wasm call. That must start with a spike that measures nested `ilang_alloc` cost and the stack-overflow texts.

### D5. One "ilang" thread per run, with tokio `current_thread`

- A new `crates/ilang-host/src/run.rs` has `block_on`. It starts one scoped thread named `ilang` with a `THREAD_STACK` stack, marks the thread with a thread-local flag `ON_ILANG_THREAD`, builds a tokio `current_thread` runtime with time enabled, and runs the future on it. A panic is resumed on the caller, as `invoke_in` does today.
- `invoke_in` calls `function.call` directly when `ON_ILANG_THREAD` is set. Otherwise it keeps today's thread per call. This **fallback** is for embedders that await `call_async` on their own executor. On the fallback thread it enters `tokio::runtime::Handle::try_current()` when there is one, so a service that makes tokio timers still finds a reactor.
- `Program::run_entry`, `Program::run_test`, and a new public `Program::run_test_with` run their whole body (instantiate, `ilang_start`, then `run` or `run_async`) inside one `block_on`. So `i run` and `i test` use one thread per program or per test, and a sync entry point uses no extra thread.
- `block_on` starts a fresh thread with its own runtime, so it also works when it is called from inside a caller's tokio runtime (today's `block_on` parks forever there, and a plain tokio `block_on` would panic).

Reason: the cost today is one thread per Wasm call. One long-lived thread removes it, keeps `max_wasm_stack = WASM_STACK` and the native `THREAD_STACK` exactly as they are, and gives the program one reactor for later IO.

`block_on` gains the bounds `F: Future + Send, F::Output: Send`. This is a breaking change for an embedder that passes a future that is not `Send`. All in-repo callers pass `Instance` futures. `invoke_in` already takes `impl AsContextMut<Data = State> + Send` (lib.rs:1163), and `ServiceFuture` is `Send` (lib.rs:479), so these futures are `Send`. The tests check this at compile time (T15). Embedders with a future that is not `Send` keep using `call_async` on their own executor, which takes the fallback path. The public docs of `block_on` and `call_async` say both things in one sentence each.

### D6. The work of a request starts lazily: one rule for every path

The start closure of an async service runs during the import. The future it returns is first polled when the host next waits, after the Wasm call has returned. This is the same on the `ilang` thread and on the fallback path.

The rule for service authors, written in `services.rs` and allowed by spec §B.4: **fix anything that depends on time, such as a deadline, in the start closure, and copy the inputs there; do not do it inside the future.** For example `let deadline = Instant::now() + d; Box::pin(async move { sleep_until(deadline).await; ... })`. Services do not `tokio::spawn`; the host owns every future, so dropping a request drops its work with nothing left running.

Reason: an "eager" start that spawns the work on the `current_thread` runtime does not help. A task spawned on that runtime is not polled until the thread goes back to the runtime, which is after the Wasm call returns, so a timer made inside the task would still count from the wait. Polling once inside the import would need a waker and a runtime context inside a host function. Lazy start with the rule above is simpler and is the same on every path.

### D7. "No other export while waiting" is enforced by the hosts only

The module does not guard sync entry points, tests or host exports while an async entry point waits. `$async_begin` and `$async_result` keep their existing checks. The Rust host refuses through `usable()` and the `&mut Instance` borrow; the JS host through `bridge.waiting`.

Reason: a guard in every sync export would change the WAT of every module and edit `emit/mod.rs`, which is outside stream C. With host-side checks, a module with no async import changes only in its version string, so the WAT checks of the other lanes stay valid.

### D8. A one-entry request table in the module, with a fixed interface

`emit/host.rs` gets three small functions, `$req_start`, `$req_answer` and `$req_outstanding`, that are the whole request table of the module. In stream C the table has one entry. R2 replaces the bodies with a real table and keeps the callers. Answers still go through the per-import globals `$hr{i}_*`, which is correct only while one request is outstanding (section 6.4).

Reason: it keeps the change in C small, and it gives R2 one place to take over.

### D9. Version 3 on both targets, with clear messages

The version is 3 in the compiler, the Rust host, the JS SDK, the fixtures and the spec. WASI modules also say 3, because the version names the metadata format. A host refuses:

- a version below 3: "the module uses i ABI version {v}; build it again with this version of i";
- a version above 3: "the module uses i ABI version {v}, but this host reads version 3; update the host".

### D10. Hosts keep a request table keyed by id

- Rust: `requests.rs` keeps the request data in a map from id, and the work futures in a `FuturesUnordered` (from `futures-util`, already in `Cargo.lock`), so each wake polls only the futures that were woken.
- JS: a `Map` from id to entry, a FIFO of settled answers and one wake resolver. A settle is kept only while `requests.get(id) === entry`, so a late settle of a dropped request is ignored. Answers are given in the order they settle.

### D11. Tests are deterministic

The out-of-order tests use a "gate" service whose answer order comes from a seeded permutation, not from timers. The timer test uses tokio paused time. The JS tests resolve gates one at a time with a macrotask yield in between, so the order does not depend on how many promise steps the SDK puts around the work.

## 3. Exact rules (ABI v3)

- **R1. Request id.** An async import (target `ilang` only) has the Wasm parameters `(i32 id, B.2 parameters...)` and no results. The id is 0 or more, and differs from the id of every outstanding request of the instance. "Outstanding" means started and not yet answered or dropped. An id may be used again after its request has been answered or dropped. In stream C the compiler takes ids from a counter that starts at 0 and wraps at 2^31; with one outstanding request, a late or stale id is never outstanding.
- **R2. When a request may start.** An async import may only be called during a call of an async entry point or of `ilang_resume:…`. The host makes the import trap, and does not use the instance again, when it is called at another time (during `ilang_start`, a sync entry point, a test that is not async, a host export, `ilang_result:…`, or inside another import), or with an id that is below 0 or already outstanding. In the Rust host this is the `accepting` flag of the request table; in JS, `bridge.accepting`. Compiled modules cannot do this: `$host_root_boundary` already traps outside an async entry point.
- **R3. Copy at start.** The module frees the parameter buffers when the import returns (§B.3), so the host copies `Int`, `String` and `BitArray` parameters during the import. Both hosts already lift (copy) the parameters during the call.
- **R4. Status.** After an async entry point or a resume returns, 0 means ended and 1 means waiting with one or more outstanding requests. Another value is a fault of the module: the instance is not used again. Status 1 with no outstanding request is also a fault of the module, because the program can never go on: "the module waits, but no host function is running".
- **R5. Answer.** The host answers by calling `ilang_resume:ns:label(id, results...)` for an outstanding id that the import `ns.label` started. Only one resume runs at a time, and never inside another Wasm call of the instance. The host may answer any outstanding request whose work is done; ilang-host and the JS SDK answer in completion order. Result buffers are made with `ilang_alloc` just before the resume that carries them. A finished answer that has not been delivered stays a host value, so a later trap leaks nothing in linear memory.
- **R6. Wrong resume.** A resume with an id that is not outstanding, or that another import started, traps with a bare `unreachable` and no stop message: it is a host bug, not a program stop. Both hosts check first and fail with their own text: "the host answered request {id}, which is not outstanding".
- **R7. Leftovers at status 0.** When the status is 0 while requests are still outstanding, the host drops them and never answers them. Rust drops the future, which cancels it; JS forgets the entry and ignores a late settle. Compiled modules of stream C never leave requests open. The rule is there so that R2 can end a task without a cancel import.
- **R8. Trap.** When any call traps (a stop or another trap) while requests are outstanding, the host drops every outstanding request and every finished answer that was not delivered, before it reports the outcome. The instance is not used again, and a late answer never wakes it. The outcome shapes stay the same: `Outcome::Stop` for a stop, `Err(Stop)` for a stop during `ilang_start`, and a failure for a host failure.
- **R9. Host failure.** When a work future gives `Err(...)` (a failure, not an `? E` error, which is an answer), the host drops all other outstanding requests, does not use the instance again, and returns the error. This is v2's behaviour extended to many requests.
- **R10. One async entry point at a time.** While an async entry point waits, the host calls only `ilang_resume:…`, `ilang_alloc` and `ilang_free`. `$async_begin` and `$async_result` trap if a request is outstanding; other exports do not check (D7). Hosts refuse such calls before they reach Wasm.
- **R11. Dropping a waiting call (Rust).** If the future of `call_async` or `run_async` is dropped while it waits, the instance is not used again ("an async call was dropped while it waited"), and its requests are dropped with it. Implementation: `wait()` sets `unusable` to that text before its first await and clears it only when it returns normally.
- **R12. WASI.** `host_is_async` (lower.rs:937) is false on the WASI target, so there are no ids, no resume exports and no change to imports. WASI metadata still says version 3.

## 4. Spec text, section by section

All in `docs/spec.md`, Appendix B. No renumbering. Phase 1 of C-I commits this text alone.

### §B.3 Buffers

Add at the end of the last paragraph:

> An async host function (§B.4) is answered after its parameter buffers are freed, so the host copies its parameters during the import.

### §B.4 Imports

Replace the bullet "Async functions of host services" with:

> - **Async functions of host services**: the import has an `i32` **request id** first, then the parameters of §B.2, and no results. The host copies the parameters, keeps the request, and returns at once. The work may start at any time before the host next waits. The answer comes later through `ilang_resume` (§B.5). Until then the request is **outstanding**.
>   - The module chooses the id: a number of 0 or more that no other outstanding request of the instance has. After a request is answered or dropped (§B.5), its id may be used again.
>   - The module calls an async function only during a call of an async entry point or of `ilang_resume:…`. If it calls one at another time, or with an id that is below 0 or already outstanding, the import traps and the host does not use the instance again.

### §B.5 Exports

Replace the bullets from "Async entry points" to the end of the section with:

> - **Async entry points** (§10.2) have the parameters above and one result, an `i32` **status**. 0 means the entry point has ended. 1 means it is **waiting**: it has one or more outstanding requests (§B.4), and it goes on only when the host answers one. Another status is a fault of the module.
> - The host answers a request by calling the export `ilang_resume:namespace:label` of the function that started it. The first parameter is the request id, and the others are the results of the function as in §B.2. The result is a status again. The host may answer outstanding requests in any order, but one at a time. It makes the result buffers with `ilang_alloc` just before the call that gives them.
> - A call of `ilang_resume:…` with an id that is not outstanding, or that another function started, traps.
> - When the status is 1 and no request is outstanding, the program can never go on. This is a fault of the module, and the host does not use the instance again.
> - When the status is 0, the host calls the export `ilang_result:name` of the entry point, with no parameters, to get its results as in §B.2. Requests that are still outstanding are dropped: the host stops their work if it can and never answers them. This lets a program end work that it no longer needs; a module made by this compiler has no outstanding request at that point.
> - When the work of an async function fails in the host (not an error of its `? E`, which is an answer), the host drops every outstanding request and does not use the instance again.
> - While an async entry point is waiting, the host may only call `ilang_resume:…`, `ilang_alloc`, and `ilang_free`. So an instance runs one async entry point at a time. The module does not check every export for this; a host that calls another export breaks the instance.
> - When the program stops, the export traps after the `panic` import (§B.4). After a trap, the host may not use that instance of the module again. Every outstanding request is dropped, and an answer that comes later is ignored. The host makes a new instance and calls `ilang_start` on it.

### §B.6 Metadata

In the JSON example, `"version": 2` becomes `"version": 3`. Add this bullet first:

> - `version` is 3. A host refuses a module with another version. Version 3 differs from version 2 only in the request id of §B.4 and §B.5, so a module of an older version must be built again.

### §B.7 The WASI target

Change the async bullet to:

> - A function of an async host service is a blocking import, with no request id and the results of a function that is not async (§B.2). Nothing is async, and there are no `ilang_resume:…` and `ilang_result:…` exports.

### §10.2 and §9.7

No change. They do not say "exactly one"; only §B.5 did.

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
thread_local! { static ON_ILANG_THREAD: Cell<bool> = const { Cell::new(false) }; }

pub fn block_on<F>(future: F) -> F::Output
where F: Future + Send, F::Output: Send
{
    std::thread::scope(|scope| {
        let thread = std::thread::Builder::new()
            .name("ilang".into())
            .stack_size(THREAD_STACK)
            .spawn_scoped(scope, || {
                ON_ILANG_THREAD.set(true);
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_time().build().expect("a tokio runtime");
                runtime.block_on(future)
            })
            .expect("a thread for i");
        thread.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
}

pub(crate) fn on_ilang_thread() -> bool { ON_ILANG_THREAD.get() }
```

`lib.rs` re-exports `block_on` from `run.rs`, so `ilang_host::block_on` keeps its name. Its old body (lib.rs:797) goes away.

### 5.3 lib.rs

- Doc header (lib.rs:1): version 3; describe the `ilang` thread and the fallback.
- `ABI_VERSION = 3` (lib.rs:81). `Abi::validate` (lib.rs:259) gives the D9 messages.
- Comment at lib.rs:635: "`async_stack_size` is set only because `Config::validate` checks it against `max_wasm_stack`. The host does not use Wasmtime's async support."
- `State.pending: Vec<Pending>` (lib.rs:607) becomes `State.requests: Requests`.
- `invoke_in` (lib.rs:1162): if `run::on_ilang_thread()`, call `function.call(&mut *context, params, results)` directly. Otherwise use today's scoped thread and enter `Handle::try_current()` inside it. The error path calls `requests.drop_all()` instead of `pending.clear()` (lib.rs:1185).
- `define_import` (lib.rs:1324): for an async service, the `FuncType` is `[I32] ++ signature.param_types()` with no results. Read the id from `parameters[0]` and lift the B.2 parameters from offset 1. Call `requests.start(id, request)`. On `Err`, set `state.failure` with the text of the error and return the trap error, so the existing failure path marks the instance unusable.
- `call_async` (lib.rs:965) and `run_async` (lib.rs:1019) set `requests.accepting = true` only around the call of the async export. `call`, `run` and `ilang_start` leave it false (R2). At the start of `call_async`, an outstanding request cannot exist because `usable()` refuses; assert it as an internal error.
- `wait(name, status)` (lib.rs:1038):

```text
self.unusable = Some("an async call was dropped while it waited")   // R11
loop {
  match status { 0 => { requests.drop_all(); self.unusable = None; return Ok(None) }   // R7
                 1 => {}
                 other => fault("status {other} of {name}") }
  let Some((id, request, reply)) = requests.next().await
      else fault("the module waits, but no host function is running")                // R4
  match reply { Err(failure) => { requests.drop_all(); unusable = failure; return Err } // R9
                Ok(answer) => {} }
  let resume = export("ilang_resume:{ns}:{label}")
  let mut values = lower_reply(answer)       // allocates now (R5)
  values.insert(0, Val::I32(id))             // the id goes in at the call site
  requests.accepting = true; status = invoke_in(resume, values); requests.accepting = false
  settle(...)                                // a trap goes through invoke_in, which drops all (R8)
}
```

- `Program::run_entry` (lib.rs:742), `Program::run_test` (lib.rs:767) and the new public `Program::run_test_with(&self, name, options)` wrap their bodies in `run::block_on(async { ... })`. `run_test` calls `run_test_with`.
- Docs of `call_async` and `block_on`: "Called from a thread that `block_on` did not start, each Wasm call starts its own thread with a stack of `THREAD_STACK` bytes." and "`block_on` needs a `Send` future; with a future that is not `Send`, await `call_async` on your own executor."

### 5.4 requests.rs (new)

```rust
pub(crate) struct Request { namespace: String, label: String, signature: Signature }

pub(crate) struct Requests {
    pub accepting: bool,
    outstanding: HashMap<u32, Request>,
    work: FuturesUnordered<Pin<Box<dyn Future<Output = (u32, Result<Result<Value, Value>>)> + Send>>>,
}

impl Requests {
    /// R1, R2: refuses when not accepting, when id < 0, or when id is outstanding.
    pub fn start(&mut self, id: i32, request: Request, work: ServiceFuture) -> Result<()>;
    /// The next finished request, or None when nothing is outstanding.
    /// The id is removed from `outstanding` before it is returned.
    pub async fn next(&mut self) -> Option<(u32, Request, Result<Result<Value, Value>>)>;
    pub fn drop_all(&mut self);
}
```

The error texts of `start`: "the module started request {id} outside an async call", "the module started request {id}, which is below 0", "the module started request {id} twice". `next` returns `None` at once when `outstanding` is empty. R2 takes over this file when it adds `cancel(id)`.

### 5.5 services.rs

- Add the D6 rule to the docs of `AsyncServiceFunction` in one sentence: "The start closure runs during the import, and the future is first polled when the host waits; so work out deadlines and copy inputs in the start closure, and do not spawn tasks."
- `standard()` does not change in C. The IO stream adds `i/clock`, `i/net` and `i/fs` later.
- `testing()` does not change, so the output order of `spec.rs` holds. The test services of section 8 live in `tests.rs`.

### 5.6 Stack depth and stop behaviour

On the `ilang` thread, a few KB of `block_on`, tokio and `wait()` frames sit below the first Wasm frame. Wasmtime measures `max_wasm_stack` from the Wasm entry, so the Wasm limit (`WASM_STACK`, 32 MiB) is the same, and the native stack (`THREAD_STACK`, 48 MiB) keeps a margin of 16 MiB minus those frames. To prove that nothing changes, the executor change lands as its own first commit under ABI v2, gated by the existing deep-recursion and "call stack exhausted" tests plus T11, before the version bump.

## 6. Module side (crates/ilang-wasm)

### 6.1 emit/host.rs globals

`$host_pending` (host.rs:21) is replaced by:

```wat
(global $host_import (mut i32) (i32.const -1))  ;; import of the outstanding request; -1 = none
(global $host_request (mut i32) (i32.const -1)) ;; its id
(global $host_next (mut i32) (i32.const 0))     ;; the next id; wraps at 2^31
```

### 6.2 emit/host.rs request table functions

```wat
;; The request table (B.4, B.5). In stream C it has one entry. R2 replaces
;; these three functions with a table of requests (see the comment below).
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
- `$async_begin` (host.rs:56) and `$async_result` (host.rs:64): trap on `(call $req_outstanding)` instead of `$host_pending`.
- Header comment of `EFFECTS` (host.rs:10): describe the request id and the table.

### 6.4 The R2 hand-off (written as a comment in emit/host.rs)

> R2 takes over the request table. These parts are correct only while one request is outstanding, and R2 must replace them together:
> - `$req_start`, `$req_answer`, `$req_outstanding` become a table from id to { import, continuation, answer }, with a free list. The id may be the slot index.
> - The answer globals `$hr{i}_*`: each resume must decode its answer into the entry of its request before control leaves the export, or a second answer of the same import overwrites the first.
> - The one continuation `$host_k`: `$host_wait` stores `k` in the entry of the current request, and `$host_resume_k` takes `k` from the entry of the id.
> - The scheduler of `i/task` returns `$host_waiting` to the root only when no task can run.
> - `cancel(id)` in `ilang:host/v1`, if R2 needs to end one request without ending the entry point.
> None of this changes the ABI: hosts already accept many outstanding requests in any order.

### 6.5 Outside C's files (escalations)

These need the coordinator's approval:

1. `crates/ilang-wasm/src/emit/mod.rs` ~:141: for `ImportKind::AsyncService`, put `"i32"` first in `params`. One line. It changes only modules with async imports.
2. `crates/ilang-cli/src/lib.rs:162` `run_test_with`: call the new `ilang_host::Program::run_test_with`, so each test runs on one `ilang` thread and the two copies cannot drift.

`lower.rs` `make_import_func` needs no IR change (`HostStart`, `Suspend`, `HostReply` stay). `abi_json` (lower.rs:2680) writes `"version": 3`.

### 6.6 WAT expectations

- `(import "app:clock/v1" "later" (func $imp0 (param i32 f64)))`.
- Each `ilang_resume:…` has a leading `(param $id i32)`.
- Modules with async imports: the three new globals and `$req_*` functions replace `$host_pending`.
- WASI modules: no `$host_request` (WASI never has async imports), only the version string changes.
- Modules with no async import: only the version string in the custom section changes. `wat_expect.changed` lists the version change for every module and the full change only for modules with async imports.

## 7. JS bindings (bindings/javascript)

- `ilang.mjs`: `const version = 3` (ilang.mjs:15) and the header comment (ilang.mjs:2). `readAbi` (ilang.mjs:223) gives the D9 messages.
- The bridge: `pending = []` (ilang.mjs:359) becomes `requests = new Map()`, `settled = []`, `wake = null`, `accepting = false`.
- The async import (ilang.mjs:688): `guardedFunction` params are `[0x7f, ...paramTypes(signature)]`. The callback takes `(id, ...raw)`. If `!bridge.accepting`, `!Number.isInteger(id) || id < 0`, or `bridge.requests.has(id)`, it calls `breakIn` with a `GuestFault` with the R2 texts. Otherwise it lifts (copies) the arguments, creates the work as today, stores `entry = { namespace, label, signature }`, and adds:

```js
work.then(
  (outcome) => { if (bridge.requests.get(id) === entry) { bridge.settled.push({ id, entry, outcome }); bridge.wake?.(); } },
  (failure) => { if (bridge.requests.get(id) === entry) { bridge.settled.push({ id, entry, failure }); bridge.wake?.(); } });
```

- `wait(name, status)` (ilang.mjs:726): a loop.
  - Status 0: `requests.clear()`, `settled.length = 0`, return.
  - Status 1 with `requests.size === 0`: throw `trapped(new GuestFault("the module waits, but no host function is running"))`.
  - While `settled` is empty: `await new Promise((r) => (bridge.wake = r))`.
  - Shift the first entry and `requests.delete(id)`. A failure that is not a `GuestError` sets `unusable`, clears both, and rethrows.
  - Validate `ilang_resume:${ns}:${label}` with params `[0x7f, ...resultTypes(signature)]` and result `[0x7f]`.
  - `values = lowerReply(...)`, then `status = bridge.enter(() => { bridge.accepting = true; try { return exports[resume](id, ...values); } finally { bridge.accepting = false; } })`.
- `callAsync` and `runPoint` set `accepting` only around the async export call. `trapped()` (ilang.mjs:440) clears `requests` and `settled`.
- JS cannot stop a promise. The work of a dropped request keeps running, and its side effects happen; only its answer is ignored. The README says so.
- `ilang.d.ts:14`: `version: 3`. README: "version 3"; rewrite the waiting paragraph: the module picks a request id for each async host function it starts; the SDK answers them in the order they finish; while it waits, other calls are refused.

## 8. Fixtures and scripts

### 8.1 tests/browser/fixtures/host-abi.wat

Shared by the Rust and JS tests.

- Header says v3; metadata `"version":3` (host-abi.wat:247).
- `later` becomes `(param i32 f64)`. Existing exports pass an id from a counter global and keep their behaviour. `ilang_resume:app:clock/v1:later` takes `$id` first and checks it against the stored id.
- New hand-written exports (the compiler never keeps many requests outstanding in C):
  - `app/async.many(n: Float) -> Float`. It starts n `later` requests with sparse ids `k*37+5` and argument `x_k = k+1`. Linear memory keeps, for each id, a started flag and the expected value. Each resume checks that the id is started and not answered (else it panics with "fixture: bad id") and that the value is `2*x_k` (else "fixture: wrong answer"). It folds `h = (h*31 + k+1) mod (2^31-1)` in answer order. After each of the first n answers it starts one more request that **reuses the freed id** (with a new argument), while the others are still outstanding, until 2n requests have been answered. It returns status 1 until then. `ilang_result` gives `h`. The test computes the expected `h` from the order it chose.
  - `app/async.stuck`: returns status 1 with nothing started (R4).
  - `app/async.dup`: starts two requests with the same id (R2).
  - `app/async.negative`: starts a request with id -1 (R2).
  - `app/async.leftover`: starts 3 and returns 0 after the first answer (R7).
  - `app/async.stop_mid`: starts 3, then panics and traps on the first answer (R8).
  - `app/sync.starts`: a sync export that calls `later` (R2).
  - `app/deep(n)` and `app/async.deep(n)`: recursion that is not a tail call, n deep, in a sync call and after a resume.
- The metadata lists the new exports as host exports with `async` true or false.

### 8.2 Other fixtures

- `custom-imports.wat`: version 3 in its metadata.
- `fixture-abi.mjs:8`: `version: 3`, and the new exports.
- `fixtures/README.md`: version 3. Fix the sentence that says the script passes `--opt 2`; it does not.
- `compiled/src/app.i` does not change, so the stop position `app/app:35:50` holds. `compiled.wasm` is rebuilt.

### 8.3 scripts/build-abi-fixture.sh

No change to the steps. Add one check after the `wasm-tools` loop: read the version from `"version": N` in `crates/ilang-wasm/src/lower.rs` and from `{\"version\":N,` in `host-abi.wat`, and fail with "host-abi.wat says ABI version X, but the compiler writes Y" when they differ. Keep the format of the `featureProbe` sed line. Run the script after the emit change.

## 9. Files to change

| File | Region | Change |
|---|---|---|
| `docs/spec.md` | §B.3–B.7 | section 4 (phase 1, alone) |
| `crates/ilang-wasm/src/emit/host.rs` | :1–80, :99–130, :261–282 | section 6.1–6.4 |
| `crates/ilang-wasm/src/lower.rs` | ~:2680 | version 3 |
| `crates/ilang-wasm/src/emit/mod.rs` | ~:141 | escalation 1 |
| `crates/ilang-wasm` tests | WAT expectations | section 6.6 |
| `crates/ilang-host/Cargo.toml`, `Cargo.lock` | dependencies | section 5.1 |
| `crates/ilang-host/src/run.rs` | new | section 5.2 |
| `crates/ilang-host/src/requests.rs` | new | section 5.4 |
| `crates/ilang-host/src/lib.rs` | :1, :81, :259, :607, :635, :742–790, :797, :965–1100, :1162–1195, :1324–1370 | section 5.3 |
| `crates/ilang-host/src/services.rs` | docs of `AsyncServiceFunction` | section 5.5 |
| `crates/ilang-host/src/tests.rs` | version strings, `raw()` calls with an id, new tests | section 10 |
| `crates/ilang-cli/src/lib.rs` | :162 | escalation 2 |
| `bindings/javascript/ilang.mjs`, `ilang.d.ts`, `README.md` | | section 7 |
| `bindings/javascript/test/host.test.mjs`, `compiled.test.mjs` | | section 10 |
| `tests/browser/host.spec.mjs`, `fixture-abi.mjs`, `fixtures/*` | | sections 8, 10 |
| `scripts/build-abi-fixture.sh` | after the parse loop | section 8.3 |

Every version literal moves in one commit: `lower.rs`, `ABI_VERSION`, `ilang.mjs`, `ilang.d.ts`, both READMEs, `fixture-abi.mjs`, `host-abi.wat`, `custom-imports.wat`, `compiled.wasm`, the JSON in `tests.rs` and `host.test.mjs`. `tests/wasi.rs` needs no edit; it reads the version through `Abi::from_wasm`.

## 10. Tests

### 10.1 Test services (Rust, in tests.rs)

- **Gate.** Each start takes `rank = perm[k]` from a seeded permutation (xorshift). Its future is ready when the shared `next_rank == rank`; then it adds 1 to `next_rank` and wakes every stored waker. When it is not ready it stores its waker first, so it never hangs. Each future has a drop guard that counts drops. The answer order is exactly `perm`, with no timers.
- **Countdown.** Ready after a seeded number of polls; it calls `wake_by_ref` each time it returns `Pending`. The tests check only that the module's hash equals the hash of the order that the host recorded, because the order depends on how `FuturesUnordered` polls.
- **Timer.** `tokio::time::sleep_until(deadline)` with the deadline fixed in the start closure (D6), run with paused time.

### 10.2 Rust (crates/ilang-host/src/tests.rs)

- **T1 `many_requests_complete_out_of_order`**: for seed in 0..200, n = 1 + seed % 16, gate service. Call `app/async.many(n)` through `block_on`. Check `h` against the expected value, that every future was dropped once (2n drops), and that the instance stays usable (a second call reuses ids).
- **T2 `timers_answer_through_the_reactor`**: 200 repeats with paused time (`tokio::time::pause` in the runtime of the test), n = 8, deadlines `(n-k)` ms. Check the value and that the order is `n-1 … 0`. Paused time makes it fast and stable under parallel test threads.
- **T3 `a_stop_drops_outstanding_requests`** (`stop_mid`): `Outcome::Stop` with the message; 2 drops before the outcome is returned; the next call is refused.
- **T4 `a_host_failure_drops_the_others`**: one gate future gives `Err`; `Err` is returned, all others are dropped, the instance is unusable.
- **T5 `status_one_with_nothing_outstanding_is_a_fault`** (`stuck`).
- **T6 `duplicate_or_negative_id_traps`** (`dup`, `negative`).
- **T7 `a_sync_call_may_not_start_requests`** (`sync.starts`, and `ilang_start` of a small WAT that starts one).
- **T8 `leftovers_are_dropped_at_status_zero`** (`leftover`): `Ok`, 2 drops, usable afterwards.
- **T9 `a_wrong_resume_traps`**: raw resume calls with an unknown id, a stale id and the wrong label; each is a trap with no stop message.
- **T10 `dropping_a_waiting_call_makes_the_instance_unusable`**: poll `call_async` once with a closed gate, drop it, check that the next call is refused and the futures were dropped.
- **T11 `deep_recursion_after_a_resume_stops_like_a_sync_call`**: the trap texts of `app/deep(N)` and `app/async.deep(N)` are equal, and a moderate depth succeeds in both. Lands in the first (v2) commit with a v2 version of the fixture export.
- **T12 `block_on_works_inside_a_tokio_runtime`**: no park and no panic; and `call_async` awaited on a user's own runtime thread still works through the fallback.
- **T13 `old_and_new_versions_are_refused`**: versions 1 and 2 give the rebuild message; version 4 gives the update-the-host message.
- **T14 `run_entry_uses_one_thread`**: a sync service records `std::thread::current().id()` on every call during a multi-resume async entry; all ids are equal.
- **T15 `futures_are_send`**: compile-time checks, for example `fn send<T: Send>(_: &T) {}` on the futures of `Instance::run_async` and `call_async`, and the same check on `ServiceFuture`, `Outcome`, `TestResult` and `Value`. No new crate is needed.

### 10.3 JS (bindings/javascript/test, node --test)

- `host.test.mjs`: a gate made of stored resolvers. The test resolves them in `perm` order, **one at a time, with a macrotask yield (`setImmediate`) after each**, so every settle reaches the FIFO before the next resolve, whatever the depth of the promise chains in the SDK. The same 200-seed loop as T1; T3–T9; versions 1, 2 and 4 refused; the base metadata at version 3.
- `compiled.test.mjs`: resume calls get an explicit id: `(0, 0, 0, 0)` and `(0, 0, 1, 0, 0)` (the first request of a fresh instance has id 0), still expecting `RuntimeError` where they do today; a stale id must trap.

### 10.4 Browser (tests/browser/host.spec.mjs)

One test, "many requests complete out of order", runs the 200 seeds inside a single `page.evaluate`, in chromium, firefox and webkit. The macrotask yield is a `MessageChannel` message, not `setTimeout`, so there is no 4 ms clamping. The two existing async tests stay.

### 10.5 Unchanged tests that must still pass

`commands.rs` `the_compiled_fixture_is_current` (version 3 on both sides), `spec.rs` output order with `test:async/v1`, `wasi.rs` `async_services_are_blocking_calls` and `ilang_stop`, `tests/spec/9.7/async_services.i`.

### 10.6 Gate

`cargo test -p ilang-host -p ilang-cli -p ilang-wasm`; `npm test`; `npm run test:browser`; `dev-verify` rust and js; the WAT diff shows only the changes of section 6.6. C-RF runs T1 in a shell loop 200 times with `--test-threads=1`, as the plan's regression view asks.

## 11. Phases

1. Spec text of section 4, one commit.
2. Executor under ABI v2: `run.rs`, the fast path in `invoke_in`, `Program::run_*` through `block_on`, `Program::run_test_with` (and escalation 2), T11, T12, T14, T15. Gated by the existing tests alone, so a change in stack or stop behaviour shows up before the ABI changes.
3. ABI v3: module side, request table, Rust host, JS, fixtures, the remaining tests. These depend on each other through the version, so they land together; several commits, one gate at the end.

## 12. Risks

- **Fibers left out.** This departs from the plan and needs approval (D4). If the user still wants fibers, an all-async store spike must come first.
- **Stack below the first Wasm frame.** A few KB of tokio and wait frames now sit below Wasm on the `ilang` thread. The 16 MiB native margin covers them; T11 and the phase 2 gate confirm it.
- **`block_on` needs `Send`.** A breaking change for an embedder that passes a future that is not `Send`. T15 checks the in-repo futures; the docs say what to do instead.
- **The fallback path.** Embedders that await `call_async` on their own threads still pay one 48 MiB thread per Wasm call. Only `block_on` and `Program::run_*` get the fast path.
- **Lazy start.** A timer made inside the future counts from the first poll. The rule in `services.rs` and §B.4 says so, but nothing enforces it.
- **A long Wasm computation blocks the reactor.** Wasm runs on the reactor thread, so timers and later network IO wait while pure computation runs. This is inherent in M:1 and would be the same with one fiber.
- **One outstanding request in the module.** `$req_start` traps on a second request, and answers use the `$hr` globals of each import. Only the hand-written fixture keeps many requests outstanding. R2 must replace the table, the answer globals and `$host_k` together (section 6.4).
- **Host-only enforcement of R10.** A careless third-party host that calls a sync export while waiting can corrupt the module's state. The spec says this breaks the instance.
- **JS cannot stop dropped work.** Its side effects still happen; only the answer is ignored.
- **Version bump.** Every checked-in v2 artifact breaks at once. `the_compiled_fixture_is_current` compares only metadata, so a stale import signature would not be caught if the version did not also change. The new check in `build-abi-fixture.sh` catches part of this.
- **Escalations.** `emit/mod.rs` and `ilang-cli/src/lib.rs` are outside C's files. Without escalation 1 the ABI change cannot land; without escalation 2 `i test` keeps one thread per Wasm call.
- **Test time.** 200 seeds in three browsers. Each round uses `MessageChannel` yields and no timers, so a round takes well under a millisecond per request.

## 13. Open questions

1. Does the user approve leaving out Wasmtime fibers (D4), and may the plan line be changed as in "Approval needed"?
2. Does the coordinator approve the two escalations of section 6.5 (`emit/mod.rs` ~:141; `ilang-cli/src/lib.rs:162`)?
3. Should `block_on` keep a second form that runs on the current thread with no `Send` bound, for embedders? This design says no and points them to `call_async`.
4. Should v3 already reserve the name `cancel` in `ilang:host/v1`, so that R2 can add it without a version bump? This design does not reserve it; adding an optional runtime import that old modules do not use would not need a new version anyway.
