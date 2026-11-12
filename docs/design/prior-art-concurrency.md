# Concurrency prior art: tasks, scheduling, channels, cancellation, and IO

Study, 2026-11-11. Proposals for the coordinator; no language or runtime rule is changed by this note.

The best first target is the amount of bookkeeping per scheduling point. The reference implementations use local queues, explicit waiter registrations, and direct ready paths. i already has lazy continuation capture. Its experimental scheduler still repeatedly traverses persistent lists and its task construction retains a complete service environment. Fixing those costs is more promising than replacing the continuation mechanism without measurements. This is a source-based conclusion, not a measured attribution of the reported timings.

## Evidence and boundaries

All reference paths below are relative to `/home/agent/Programming/i/.rd/ref/`. A citation names the file and the function or definition actually read. i citations give file and line; **B** means the pinned `lane/b` snapshot, not the base checkout. Short i paths `task.i`, `emit/control.rs`, `control.rs`, `ir.rs`, and `runtime/values.rs` mean `lib/i/task.i`, `crates/ilang-wasm/src/emit/control.rs`, `crates/ilang-wasm/src/control.rs`, `crates/ilang-wasm/src/ir.rs`, and `crates/ilang-wasm/src/runtime/values.rs` respectively. Short design-note paths are under `docs/design/`. Lines refer to these snapshots:

| Tree | Commit read |
| --- | --- |
| i base, also redesign when this study began | `c2f41b734f2af530a82e3510eb778f8e1402075b` |
| i B, scheduler implementation | `3b9393a88bc50a4e083a0174b716c661513d0510` |
| Go | `cd98b3b9e2f0a79dc207049202f09748e8afe580` |
| OTP | `045d30badd4c426bc51b386ecf0c51116d2baa9b` |
| Elixir | `dedb0f2aaf8085cbb5b3f90200ac3af5d392af70` |
| OCaml | `75c151eb70092b6dac7aed6cc01a1a21870a0e70` |
| Eio | `d10d717a2874d297e697f6c9eb4a3f2685a54a45` |
| GHC | `19ff36f2aede0ca2e554016ff22c71e73aa9803b` |
| Tokio | `d6102c7e0a97b939c23d056122490faf777d2875` |
| Effect | `fdd319e4592a566a4ea9c9ad8cc60f2d62959feb` |
| Koka | `7e9c1e593a46b704c915832c2da702b7ec41f45f` |

Eio was missing and was shallow-cloned from `https://github.com/ocaml-multicore/eio.git` for this study. No reference was built or edited, and no source was copied. The requested Effect `internal/fiberRuntime.ts` is absent in this revision; its implementation is in `packages/effect/src/internal/effect.ts`. The requested Koka mechanisms were read in `koka/lib/std/core/hnd.kk` and `koka/src/Type/Infer.hs`, rather than a presumed compiler code-generation file. No claim below relies on an unlabelled recollection.

The controlling sources are `docs/spec.md` §§2.6.2, 9.2–9.5; accepted `syntax.md`; `runtime.md`, `multicore.md`, `native.md`, `lazy-reentry.md`, and `host-concurrency.md`; the settled design and B/C/R2 tracks in `/home/agent/.claude/plans/twinkling-swinging-perlis.md`; and `/home/agent/Programming/i/.rd/cache/ledger.md`. In particular:

- Effects remain functions, errors, services, and general control. A service is not necessarily a captured continuation (`spec.md:1123`, `spec.md:1145`). Types remain sets of values. No new type feature or inference rule is proposed.
- A suspended handler runs at the boundary environment; resumption combines retained inner bindings with the resumer environment. Continuations have one use; abandonment alone does not run finals (`spec.md:1217`). Normal/error/cancel unwinding runs finals; program stop does not (`spec.md:1251`).
- Task creation keeps all services, including unused ones, apart from the named routing detach/rebind protocol. State is threaded through one loop outside task continuations (`runtime.md:800`). One host root continuation prevents independently resumed copies of an immutable scheduler snapshot (`host-concurrency.md:429`).
- FIFO turns, parent continuation after `go`, scheduling even on ready `sync`/`join`, cancellation priority, prevalidation of every event leaf, ordered alternatives, and absence of implicit preemption are accepted rules (`runtime.md:867`). They are constraints here, not bugs to erase for a benchmark.
- Native workers have separate Stores. Started tasks stay on their worker; only complete, portable, unstarted records travel. Shared operations use the accepted transaction lock, with no user callback or codec under it (`native.md:88`). `portable` is observable even with one worker (`multicore.md:25`).

The ledger reports R2 as a default-plan implementation; it does not establish that every accepted provider combination is implemented. This study compares actual B code where possible and names accepted rules separately.

## What the timings do and do not say

The supplied initial busy-machine results are 543 microseconds for a spawn/scope/join cycle, 16 microseconds for a switch, and 57 microseconds for a channel transfer. A later report in the pinned B `tests/r2-stages.md:618` gives five-run medians of 602, 15, and 52 microseconds respectively, also on a busy machine. These are two reports, not a regression measurement.

B `bench/test/r2_task_test.i:10` performs 1,000 whole scope/spawn/join cycles and expects 1,000. Its switch case at line 20 yields 1,000 times. The channel case at line 30 transfers 1,000 integers and expects 500,500; it includes scheduling and scope work. They do not isolate allocation of one dormant task, register-only switching, or a channel payload copy. B `tests/r2-bench.sh` runs the timed tests five times. I read these sources and reports; I did not run them.

There is no new timing comparison in this study, and no verified claim that all reference task operations cost less than one microsecond. Their code explains how small common-case costs are possible, not a portable latency bound. Reported native Go or Tokio numbers would not alone identify the cost of WasmGC, compiler continuations, portable capture checking, or i scope cleanup.

Proposed measurement protocol: first run the existing baseline through `/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i bash tests/r2-bench.sh` in its owning lane. Add separate kernels for dormant creation; ready enqueue/dequeue at queue sizes 1, 32, 1,024; yield with varying live locals and boundary depth; ready/blocked rendezvous; settled/pending request waits; scope exit with finished children; and captures with varying service count and shared graph size. Count allocations/bytes, saved locals, copied environment slots, portability visits, list/table visits, and host polls. Counters must be opt-in compiler/runtime instrumentation, not a privileged task API. Record compiler/engine versions, optimization flags, warmup, iteration count, GC, host load, median and tail, and whether startup is excluded.

For a future reference comparison, write equivalent kernels first, then record the exact commands: Go native spawn-and-join and channel ping-pong; the same Go source under `GOOS=js GOARCH=wasm` and its matching runner; Eio fiber fork/join; Tokio on a current-thread executor; and GHC fork/join with an MVar. Match what is included in each operation, especially cleanup. A missing benchmark source cannot be replaced with a plausible command or a result from memory. None of these comparisons was run here.

## Three directions considered

| Direction | Benefit | Cost or conflict | Choice |
| --- | --- | --- | --- |
| Move scheduling into a Go/Tokio-style privileged mutable runtime | Direct queues and compact task headers | Would special-case the task library, bypass the ordinary-i requirement, and duplicate accepted scheduling semantics in the host | Reject as the architecture; take the queue and wakeup algorithms |
| Keep the existing general control and ordinary-i scheduler; index state and reduce repeated work | Targets visible scans and allocation without changing user types or ordering | Persistent structures have larger constants than intrusive mutable queues; careful stale-entry handling required | First choice |
| Apply compiler specialization, direct tail-resumptive paths, shared environments, and representation changes | Can remove captures, copies, boxing, and repeated graph traversals across many libraries | Requires proofs about environments, control escape, cleanup, and source-observable portability; performance benefit unmeasured | Second stage, guided by counters |

Selection criteria, in order: preserve the normative and accepted behavior; remain an ordinary library; retain principal inference; address a demonstrated source cost; offer a useful change across unrelated programs; and minimize implementation/proof effort. Queue/index improvements are data structures, not a language feature. Proposed compiler optimizations must also help generators, service adapters, and parsers or interpreters; environment sharing also helps callbacks and request wrappers; portability traversal improvements also help export/transport and persistent data inspection. None introduces a special task type or keyword.

## 1. Go: small starts, local queues, explicit blocking, and Wasm reentry

### Creation and scheduling

`go/src/runtime/proc.go:newproc`, `newproc1`, `malg`, `gfget`, and `gfput` start a goroutine from a function value, initial stack pointer/program counter, and a reused or newly allocated G/stack. They do not copy the caller stack. `go/src/runtime/stack.go:stackMin` specifies a 2,048-byte minimum stack; that is not the complete cost of a goroutine. The free-G paths batch transfers between per-P and global caches. These are concrete mechanisms to avoid repeated expensive allocation, not a measured spawn latency.

The G/M/P division is stated at `go/src/runtime/proc.go` in the scheduler introduction and represented by `go/src/runtime/runtime2.go:g`, `m`, and `p`: a goroutine, a worker thread, and the resource needed to execute Go work. `p.runq` is a bounded ring with a separate `runnext`. `proc.go:runqput`, `runqget`, `runqgrab`, and `runqsteal` provide local enqueue/dequeue, overflow handling, and roughly half-queue stealing. `stealWork` probes other Ps in randomized order. `findRunnable` also checks global work, timers, and network readiness; its periodic global-queue check prevents local work from permanently hiding global work. Take locality, bounded queue operations, and explicit fairness checks; do not copy the topology into a one-worker scheduler.

`proc.go:gopark`/`park_m` publish a waiting G and release the relevant lock through a commit callback; a failed park can immediately resume it. `goready` makes a waiter runnable. The useful split is registration/commit/wakeup versus execution: making a task ready need not run user code within the wakeup operation. i needs the same split even without native races, because event choice and reentrant callbacks still require a commit boundary.

### Channels and selection

`go/src/runtime/chan.go:hchan`, `chansend`, and `send` check a waiting receiver or buffer capacity before allocating a blocked waiter. `waitq.enqueue`/`dequeue` maintain per-channel waiter queues; dequeue skips a select waiter whose other arm already won. `send` transfers the value before readying the peer. The useful baseline is a channel-local queue and a single committed transfer, with suspension only when execution must stop.

`go/src/runtime/select.go:selectgo` randomizes poll order, sorts channel locks, probes ready cases, registers blocked cases, parks, and then removes losing registrations. `selparkcommit` releases locks only after registration. Take the registration/commit/loser-removal structure. Leave randomized alternative priority and multi-lock acquisition: i chooses alternatives in source order and has the accepted native shared transaction lock (`runtime.md:867`; `native.md:117`). An i ready `sync` still has to schedule; the Go direct return is not permission to remove that point.

### Network readiness, timers, and the one-thread Wasm port

`go/src/runtime/netpoll.go:netpollblock` and `netpollunblock` distinguish already-ready, preparing-to-wait, and blocked states, closing the readiness-before-park race. `go/src/runtime/netpoll_epoll.go:netpoll` gathers ready descriptors into a runnable list and checks descriptor generations before accepting events. `go/src/runtime/time.go:timers`, `wakeTime`, and `check` maintain a timer heap and an earliest wake time; they need not scan every timer on every switch. The reusable idea is readiness-driven work with generation checks and an indexed next deadline.

Go Wasm is especially relevant. `go/src/runtime/os_wasm.go:getCPUCount` returns one, `newosproc` rejects thread creation, and `preemptMSupported` is false. `go/src/runtime/lock_js.go:beforeIdle` arranges the next host timeout, and `handleEvent` runs the event work and eventually returns control to JavaScript. Its file-level contract excludes native thread preemption.

The port does not rely on native stack switching. The ABI explanation in `go/src/cmd/compile/internal/wasm/ssa.go` describes a Go stack in linear memory, an unwind flag returned through Wasm calls, and a program-counter block argument for resumption. `go/src/runtime/asm_wasm.s:gogo`, `mcall`, `wasm_pc_f_loop`, and `pause` save/restore the Go SP and PC, dispatch the resumed function, and unwind to the host. `wasm_pc_f_loop` begins with the innermost resumable frame; callers reenter as execution returns. Take compiler-managed state and lazy caller reentry. Do not transplant linear-memory Go stacks into WasmGC merely to resemble Go.

### Comparison and recommendations, in gain/effort order

**Already alike:** i captures live locals lazily only when `$capture` crosses a frame, and tail calls need no frame (`crates/ilang-wasm/src/control.rs:1`; `emit/control.rs:619`, under the same crate). Its one-shot consumption is checked by `$continuation_take` (`emit/control.rs:527`). The accepted lazy-reentry bound is resume O(1 + D + E), with delimiters D and environment records E; it is not O(1) for arbitrary nesting (`lazy-reentry.md:493`).

**Different:** B uses lists for ready work and task lookup (`lib/i/task.i:297`, `452`, `476`, `869`). This is an implementation choice, not required by immutability. Go-style `runnext` is also a policy difference: i explicitly keeps the parent in its turn after spawn and queues the child (`runtime.md` §6.4; B `task.i:1516`). Copying Go or Tokio execution preference would change traces.

1. Replace `append`/`contains` ready insertion with a two-list FIFO and indexed queued state; replace linear task/scope lookup with a private persistent integer/radix index (B `task.i:452–514`, `869–889`, `1881–1961`). Preserve scripted scheduling as well as FIFO. A stale queue entry must carry a generation or an identity that is never reused. This is the simplest direct transfer of the queue lesson.
2. Index registrations by channel and direction, retaining FIFO sequence and per-choice membership (B `task.i:863`, `1076–1127`, `1200–1238`). Keep source-order probing and remove all losing arms before user callbacks. A doubly linked mutable Go waiter is not directly expressible as ordinary immutable i data; use indexed persistent records, not a new mutation privilege.
3. Count environment and saved-frame allocation before changing reentry (`emit/control.rs:692`; B `emit/control.rs:586`). The compiler already performs liveness analysis (`control.rs:495`); “add liveness” is not a new proposal.
4. Keep the one-root IO handoff (`host-concurrency.md:429`). The host already has `FuturesUnordered` and per-request wakers (`crates/ilang-host/src/requests.rs:28–149`); first optimize B `task.i:1829` interest discovery rather than add a second network poller.

## 2. BEAM and Elixir: isolated starts, explicit fairness, and failure relationships

`otp/erts/emulator/beam/erl_process.c:erl_create_process` allocates a process and heap, sizes/copies its arguments, initializes execution registers and a termination continuation, and sets up links, monitors, and message queues. Its cost depends on the argument graph and heap requirements; “cheap process” does not mean zero copying or constant time. There is no caller-stack copy in this creation path.

`erl_process.c:erts_schedule` accounts for consumed reductions and selects the next execution budget. The interpreter dispatch macros in `otp/erts/emulator/beam/emu/macros.tab:DISPATCH`, `DISPATCH_EXPORT`, `DISPATCH_FUN`, and `DISPATCH_RETURN` decrement `FCALLS` and transfer to `context_switch` when the budget is exhausted. This is compiler/runtime cooperation for preemption at dispatch points. It is not proof that an arbitrary long native function is preemptible; JIT and dirty-NIF scheduling were not audited here.

`otp/erts/emulator/beam/erl_message.c:erts_send_message` sizes and copies terms to receiver-owned storage or message fragments, with a self-send shortcut under the appropriate heap mode; `erts_queue_message` publishes a message. This is asynchronous mailbox delivery, not an unbuffered rendezvous. An i channel can borrow explicit wait queues but should not acquire an unbounded mailbox merely because processes are small.

`erl_process.c:erts_proc_exit_handle_monitor` sends monitor-down information from a dying target; `erts_proc_exit_handle_link` sends link-exit signals. `otp/lib/stdlib/src/supervisor.erl:do_restart` distinguishes permanent, transient, and temporary children; `restart` enforces intensity and implements one-for-one, rest-for-one, and one-for-all strategies. Take explicit relationship state and distinct outcome signals. A monitor, a link, a restart policy, and a lexical lifetime are different relationships.

`elixir/lib/elixir/lib/task.ex:async` starts a linked task and establishes reply identity; `await` checks ownership and receives either its reply or monitor-down, with a timeout path; `shutdown` has additional termination/monitor logic. `elixir/lib/elixir/lib/task/supervisor.ex:async` starts a temporary child, optionally links it, establishes an alias, and sends the work. A Task.Supervisor task is therefore not automatically a permanently restarted child. Neither API by itself is the rule “all children are joined before this lexical scope returns.”

There is a second useful BEAM lesson for i capture costs. `otp/erts/emulator/beam/copy.c:copy_shared_calculate` explicitly detects revisited objects, records shared objects, and avoids recounting them; its comments and code show temporary tagging and a sharing table. This study does not establish which OTP build flags enable that path for every message/spawn. Take graph-aware traversal, not its raw-header mutation technique: WasmGC representation and GC ownership differ.

### Comparison and recommendations, in gain/effort order

**Already alike:** B stores a dormant start, not a copied parent continuation, for a new task (`task.i:199`, `1516–1564`). Cancellation has an explicit cause and task state (`task.i:210`, `980–1010`). i goes further in requiring structured cleanup (`runtime.md:699`).

**Different:** i does not promise BEAM reductions or process heap isolation. Cooperative scheduling is an accepted policy; it is not a consequence of types being sets. B classifies unstarted records even on one worker; a nonportable record stays local without rejecting `go`. Portable sends/results/shares still enforce their checks on every worker count, and live continuations cannot migrate (`multicore.md:25`; `native.md:88`). Links and supervisors cannot replace scope joins.

1. Add portability graph-visit counters and a shared-DAG regression case to the task baseline. B `$core_portable` uses a work array and visitation token, but stamps only recursive closure capture arrays, not ordinary shared constructor/tuple nodes (`crates/ilang-wasm/src/runtime/values.rs:409–467`). B spawn calls it on `(start, info, scope.chain)` (`task.i:1530`). Repeated traversal of a shared constructor DAG can therefore be exponential in its depth; this is a code-derived inference, not a run result. This is also an explicit accepted cost in `multicore.md` §3.6, not a newly discovered semantic bug. See attack A below.
2. Bring forward the sharing-aware traversal anticipated by `multicore.md` D9/§3.6, or cache a valid summary for immutable objects. Change `runtime/values.rs:409`, with representation support chosen by that track. Do not assume WasmGC exposes a pointer hash, do not cache partially constructed recursive closures, and do not change `portable` answers or omit unused services. The BEAM method establishes the problem and goal, not a drop-in implementation.
3. Replace whole-task cancellation/cleanup scans with affected-scope worklists and child counters (B `task.i:980–1010`, `1295–1398`). Keep cause identity, deterministic error selection, and the distinction between outcome known and cleanup complete (`runtime.md:699`).
4. Keep preemption as a separate coordinator decision, with a starvation test and explicit latency budget. Do not silently add reductions in a performance patch. A general compiler safe-point feature would need separate rules for tight loops, cancellation, and host fairness; this study recommends no such feature now.

## 3. OCaml 5 and Eio: one-shot fibers and structured lifetime

`ocaml/runtime/fiber.c:alloc_size_class_stack_noexc`, `caml_alloc_stack`, and `caml_free_stack` allocate fiber stacks with per-domain size-class reuse. `caml_continuation_use_noexc` removes the stack pointer from a continuation, using a direct store in the single-domain case and a CAS where required; `caml_continuation_use` rejects a second use. `ocaml/stdlib/effect.ml:Deep.continue`, `Deep.discontinue`, and `Deep.match_with` connect this representation to continuation resume, exceptional resume, and handler execution. Take one-shot ownership transfer and the separation of storage reuse from language semantics. OCaml stack chunks are not the same representation as i heap frames.

`ocaml/runtime/domain.c:caml_domain_spawn` creates a platform thread and performs a startup handshake. A domain is not a cheap fiber. This distinction matters when comparing i worker creation with task creation: the former can be expensive without explaining every task switch.

Eio was read rather than described from memory. `eio/lib_eio/core/suspend.ml:enter` checks cancellation before performing suspension. `eio/lib_eio/core/fiber.ml:fork` creates a fiber context with inherited variables and uses switch operation accounting; `both` and `all` use a switch. `eio/lib_eio/core/switch.ml:with_op`, `await_idle`, and `run_internal` track active work, wait for children, run release callbacks under protection, and repeat if release creates more work. The switch exit waits on both successful and exceptional paths. This is a close model for “outcome is not cleanup completion.”

`eio/lib_eio/core/cancel.ml:with_cc`, `protect`, `cancel_internal`, and `cancel` maintain cancellation contexts. The traversal marks affected contexts before invoking cancellation callbacks, respects protected children, and clears each callback before invoking it. `eio/lib_eio_posix/sched.ml:run` handles context lookup by immediate resumption, suspension by registering the continuation with its context, and fork by enqueueing the parent and running the child. Take direct nonblocking context lookup, explicit waiter ownership, and cancellation publication before callbacks. Leave child-first execution: it differs from i policy.

### Comparison and recommendations, in gain/effort order

**Already alike:** one-shot continuation use is checked in i (`emit/control.rs:527`). The accepted scope rules wait for cleanup and shield final work (`runtime.md:699`). B already distinguishes `outcome` from `clean` (`task.i:210–231`).

**Different:** Eio has mutable fiber/switch context records; i B threads immutable scheduler state outside task boundaries (`task.i:297`, `1904`; `runtime.md:800`). Copying an Eio callback that mutates the switch into i would not implement the same operation. i also has explicit service retention and portable capture semantics beyond the particular Eio operations read.

1. Use active-child/cleanup counters and dirty scopes to avoid repeated `refresh_clean`, `scopes_of`, and retirement scans (B `task.i:1295–1398`). Ensure a finalizer-created cleanup scope increments accounting before the parent can become clean. Start with a trace-equivalent index; measure before making more complex incremental propagation.
2. Separate a published cancellation state from callbacks/wakeups, following Eio and channel commit practice. Audit B `propagate`, `recheck_events`, and retirement together (`task.i:980`, `1200`, `1318`). No callback may observe half-removed choice registrations.
3. Consider shared immutable environment snapshots, not service pruning (B `emit/control.rs:586–641`). `keep_env` copies slots to a values array and `keep_call` copies them back to a new environment. Audit all environment writes and construction first; this is a compiler/runtime representation proposal, not an assertion that replacing those copies with one pointer is already sound.

## 4. GHC: cancellation is a protocol, and blocking owns its registration

`ghc/rts/Threads.c:createThread` allocates a TSO and stack separately, initializes blocking/exception state, and attaches the thread to runtime bookkeeping. `ghc/rts/Schedule.c:schedule` dispatches runnable TSOs through the STG runtime and handles their return reasons. `appendToRunQueue` and `popRunQueue` update queue links rather than scan the set of threads. Take distinct task state, execution state, and explicit ready links; the STG stack format is not portable to i.

`ghc/rts/PrimOps.cmm:stg_takeMVarzh` and `stg_putMVarzh` test the ready state first, and allocate/link a blocked waiter when needed. An MVar is a one-place synchronization cell, not i unbuffered channel semantics. The transferable method is the fast ready path plus a waiter record that owns its queue membership.

`ghc/rts/RaiseAsync.c:throwTo`, `throwToMsg`, and the surrounding protocol distinguish cross-capability delivery, masked targets, and a sender waiting for delivery. In the MVar-blocked case, the code locks the object, rechecks the target state, and removes or retries its wait registration before raising the exception. `maybePerformBlockedException` permits delivery when unmasked, or at an interruptible blocked operation under the corresponding mask. `raiseAsync` handles catch frames and restores the mask state through stack frames. Take atomic removal from a wait structure, restoration of prior shielding, and a separate delivery/cleanup protocol. Do not equate an async exception request with completed cleanup.

`ghc/rts/STM.c:stmCommitTransaction` validates and establishes ownership before publishing updates and waking watchers. `stmWait` validates the read set, installs watchers, and establishes the waiting state while the required locks are held. Take the order “validate, publish registration/commit, then wake.” This does not propose STM for i, nor claim that arbitrary event callbacks are transactions.

### Comparison and recommendations, in gain/effort order

**Already alike:** i distinguishes cancel, errors, and terminal stop, and requires structured finalization (`spec.md:1217`, `1251`; `runtime.md:699`). Its native design puts shared matching and cancellation under one transaction lock, with callbacks outside (`native.md:117`).

**Different:** i cancellation uses explicit task/scope causes and cooperative observation, rather than arbitrary asynchronous exceptions delivered by an RTS. The difference follows accepted cancellation and scheduling rules; it is not necessary to weaken those rules for speed. `throwTo` blocking is not automatically the right public `cancel` contract.

1. Make waiter ownership explicit in any channel/index optimization. Generation-tag registrations and retain one commit authority; cancelling a waiter removes every choice arm before it can be run (B `task.i:863–889`, `1110–1127`, `1200–1238`). Test completion racing cancellation in the native protocol; test both serial orders in M:1.
2. Preserve the independent outcome and cleanup states while replacing scans with counters (B `task.i:210`, `1299`; `runtime.md:699`). A successful wait on cleanup is not inferred from an already-present result.
3. Keep shield restoration lexical and nested; do not replace cancellation with dropping a continuation. The spec explicitly says abandonment does not run finals (`spec.md:1237`). Retain tests for finalizers that suspend and for cancelled joins inside shielding (`runtime.md:699–798`).

## 5. Tokio and Effect: deduplicated readiness, cold fields, and scopes

### Tokio

`tokio/tokio/src/runtime/task/core.rs:Cell`, `Header`, `Core`, and `Trailer` separate task scheduling state and links, the future/result, and colder completion/ownership information. `tokio/tokio/src/runtime/task/state.rs:transition_to_notified_by_ref` coalesces repeated notifications and handles running versus sleeping tasks. `transition_to_notified_and_cancel` combines cancellation with the wake protocol. The lesson is not “use atomics on one worker”; it is “one runnable entry per task, with explicit state transitions.”

`tokio/tokio/src/runtime/scheduler/current_thread/mod.rs:next_task` selects between a local FIFO and remote work; its scheduler loop periodically services IO (`CoreGuard::block_on` in this file). `tokio/tokio/src/runtime/scheduler/multi_thread/worker.rs:run_task` limits repeated LIFO-slot use, and `steal_work` probes other workers. Take local readiness and a fairness budget. Do not copy either LIFO preference or arbitrary live-future stealing into i: i has FIFO rules and Store-local continuations (`runtime.md` §6.4; `native.md:88`).

`tokio/tokio/src/runtime/task/harness.rs:remote_abort`, `poll_inner`, and `cancel_task` arrange notification, observe cancellation, and drop the future while producing a cancellation result. `tokio/tokio/src/task/join_set.rs:shutdown` aborts and then waits for joins; its `Drop` aborts without the same asynchronous wait loop. Dropping a future is not an implementation of arbitrary suspending i finalizers. Take coalesced notification and explicit shutdown-and-wait; leave drop-as-cleanup semantics.

### Effect

`effect/packages/effect/src/internal/effect.ts:FiberImpl` stores context, interpreter stack, and lazily present children/observers. `forkUnsafe` constructs the child with the parent context and, unless using its daemon path, records the child relation. This is evidence that a task can share context and avoid eagerly allocating every optional collection; it is not a claim that Effect forbids detached fibers.

`FiberImpl.runLoop` counts interpreter operations and consults the scheduler; `yieldNow` schedules a guarded resumption. `effect/packages/effect/src/Scheduler.ts:MixedScheduler.shouldYield` checks the operation count, and `Dispatcher` batches callbacks. This cooperates between interpreted effects; it cannot preempt arbitrary synchronous JavaScript inside one operation.

`FiberImpl.interruptUnsafe` records interruption and defers action if running; `evaluate` processes child interruption before publishing its final exit. `forkIn` attaches an interrupting finalizer to a scope; `forkScoped` uses that route. `scopeClose` marks the scope closed before invoking finalizers and has empty/single-finalizer cases; `scopeCloseFinalizers` handles reverse-order sequential cleanup or a parallel fork-and-wait path. `asyncFinalizer` and `uninterruptibleMask` preserve the intended interruption state during cleanup. Take explicit closure state, cheap empty scopes, context sharing, and completion after cleanup. Leave its alternate concurrency policies unless i adopts them deliberately.

### Comparison and recommendations, in gain/effort order

**Already alike:** B has dormant/runnable/waiting/running/ended states and distinct completion information (`task.i:199–231`). The host uses a request map, a `FuturesUnordered`, and epoch-aware wakers; stale sequence numbers are rejected (`crates/ilang-host/src/requests.rs:28–149`). Its per-run OS thread builds a current-thread Tokio runtime, not one OS thread per i task (`crates/ilang-host/src/run.rs:41–68`).

**Different:** the i scheduler is ordinary persistent code, not a Rust future executor or an effect bytecode interpreter. B `maintain` runs cancellation, message, ticket, event, and cleanup passes, and `loop` invokes maintenance repeatedly (`task.i:1393`, `1904–1932`). i polling is tied to accepted interests/turn rules, not a tunable Tokio event interval (`runtime.md` §6.4).

1. Add a queued-state index and generation check, then make repeated wakeups idempotent without `contains` over the ready list (B `task.i:869`). Do this together with recommendation 1 under Go.
2. Maintain explicit pending interests and dirty registration sets, rather than scanning all tasks and all registrations per turn (B `task.i:1200–1263`, `1393`, `1829`). Keep the required cancellation-before-ordinary-work and poll cadence. Owned root producers, borrowed waits, and control-only wakes retain their different liveness roles (`runtime.md` §6.4; `host-concurrency.md:429`).
3. Measure the hot/cold split of `TaskState` and scope creation (B `task.i:210–231`, `297–319`). Move rarely used foreign/driver/cleanup metadata into optional records only if total allocations/field accesses improve. Effect offers a model for lazy collections, but WasmGC layouts and immutable record reconstruction must be measured.
4. Keep a separate “abort requested” and “cleanup joined” path. Use the existing spec finalization rules, not Tokio future-drop behavior, and retain the single host continuation. Dropping host work remains the host request cancellation mechanism (`crates/ilang-host/src/requests.rs:160–183`); it does not prove that an i task has finished its finals.

## 6. Koka: avoid capturing a resumption when a direct call suffices

`koka/lib/std/core/hnd.kk:Ev` carries a handler, marker, and handler evidence vector. `@perform1` selects the clause through the supplied evidence. `@evv-index` documents static indexing when the effect type permits it. `clause-control1` yields to the marker and protects the resumption; `clause-tail1` calls `under1` directly; `clause-tail-noop1` calls the operation directly without changing evidence. `under1` swaps to the handler evidence, restores it after the call, and extends a continuation only if the operation itself yields. The known solution is a direct operation call when resumption is exactly the tail return, while preserving the handler execution context.

This is not just an optimizer guessing from a function body. `koka/src/Type/Infer.hs:inferHandler` constructs tail clauses for `OpVal`/`OpFun` and control clauses for `OpControl`; it explicitly avoids an apparently tempting conversion from a nonresuming control clause to an exception clause because finalizer order differs. Take context-aware specialization and respect for finalizer order. Do not add Koka keywords, effect-row semantics, or multi-use resumption to i.

### Comparison and recommendations, in gain/effort order

**Already alike:** ordinary i service operations are functions (`spec.md:1123–1185`). They do not inherently require a scheduler continuation. The compiler propagates `may_suspend` through known callees and conservatively treats unknown value calls as possibly suspending (`crates/ilang-wasm/src/control.rs:245–285`). Captured frames contain saved live values, with environment handling kept separate (`emit/control.rs:692–735`; `ir.rs:565–570`).

**Different:** B `make_scheduler.route` calls `control.suspend` for every routed command (`task.i:668–745`), including commands whose reply keeps the parent in the same turn (`task.i:876–889`, `1516–1564`). This is not merely a missed Koka-style tail call: the updated immutable `State` lives in the scheduler loop outside the task boundary. A direct service closure cannot mutate that state in ordinary i. Further, `suspend` changes the handler environment (`spec.md:1229`). The source architecture, not the surface effect label, explains why a capture currently occurs.

1. Improve general indirect-call specialization/devirtualization, then remeasure frame captures at service call sites (`control.rs:245–285`, `emit/control.rs:619`). Prove the callee, not “no error/service annotation implies no suspend.” A function can close over a boundary and suspend with no service requirement.
2. Classify actual command cost: query only, state transition without a different turn, blocking transition, and host handoff. Use this for counters and optimization proofs, not as a new public effect taxonomy. B already returns some local information without `route` (`task.i:589–601`); preserve that useful distinction. Keep suspension for a blocked task and the single root host handoff.
3. Study a general state-threading/CPS specialization capable of keeping ordinary state in the caller path without capturing to the scheduler. No implementation is claimed here. It must preserve scheduler ownership, poll/cancel points, dynamic environments, one-use checks, and finalizer order. Do not special-case `task.go`, or merely replace `suspend(b, (k) { resume(k, v) })` with `v`.
4. Share immutable service environments where representation permits (B `emit/control.rs:586–641`); retain every source-required capture. This may reduce creation cost before more difficult control specialization. Any unused-service pruning would contradict `runtime.md:800` and `multicore.md:25` even if it made the microbenchmark faster.

## Concrete change map and proposed documentation text

These are coordinator proposals, ordered approximately by gain per effort. The ordering reflects visible work avoided, not measured speedup. All entries leave public type inference and ordinary-library status intact.

| Priority | Files/regions to change | Exact constraint and evidence to add |
| --- | --- | --- |
| 1 | B `bench/test/r2_task_test.i:10–63`, `tests/r2-bench.sh`, `tests/r2-stages.md:618` | Separate full cycle from primitive costs; retain expected sums; add queue-size/capture-size slopes and allocation/visit counters |
| 2 | B `lib/i/task.i:452–514`, `869–889`, `1881–1961` | Two-list FIFO plus indexed task/queued state; one runnable entry; preserve scripted picks, FIFO, and parent-after-spawn behavior |
| 3 | B `lib/i/task.i:863`, `1076–1263` | Channel-local wait queues and choice index; validate all leaves first; commit once; remove losers before callbacks; no self-rendezvous |
| 4 | B `lib/i/task.i:980–1010`, `1295–1398`, `1829–1849` | Dirty scopes/tasks/interests and explicit child cleanup counts; preserve cancellation priority and host polling semantics |
| 5 | B `crates/ilang-wasm/src/runtime/values.rs:409–467` and object representation | Diagnose shared-DAG traversal, then add safe memoization/identity support; same `portable` result and same captured graph |
| 6 | B `crates/ilang-wasm/src/emit/control.rs:586–641` | Audit environment writes, then share immutable snapshots where valid; retain environment overlays and routing detach/rebind |
| 7 | `crates/ilang-wasm/src/control.rs:245–285`, `emit/control.rs:619–735` | General callee specialization and proven direct paths; no suspension annotation inferred solely from effect rows |
| 8 | B `lib/i/task.i:210–231`, `297–319`, `545–587` | Profile cold metadata and empty scopes; do not remove portable-record/cancellation fields just because a local benchmark does not use them |

Keep semantic spawn paths for `task.path`, error ordering, and scripted scheduling; any compact internal integer slot is additional bookkeeping, with a stable path-to-entry mapping and a generation if slots are reused. A two-list queue is amortized O(1) only along the single threaded sequence of queue versions; do not promise that bound for arbitrary reuse of old snapshots or scripted arbitrary removal. A private radix index can stay in `lib/i/task.i` or an ordinary `lib/i` support module. The existing standard dictionary illustrates path-copying (`std/src/dict.i:55–59`, `89–119`, `145–187`), including a collision-list case; it is not an unconditional logarithmic bound. Embedded core modules are enumerated in `crates/ilang-package/src/library.rs:7–26`; this study does not assume `lib/i/task.i` can simply import an arbitrary standard package. No dependency or support module is changed in this lane.

Replacement-spec disposition, section by section:

| Section | Proposed disposition |
| --- | --- |
| `spec.md` §2.6.2 async classification | No replacement. Provider classification is not proof that all internal functions avoid `suspend`. |
| §9.2 services | No replacement. Keep caller environments and explicit captured services. |
| §9.4 control | No replacement. Keep active-boundary, boundary-environment, overlay, one-use, and abandonment rules. |
| §9.5 unwinding | No replacement. Optimizations preserve finalizer execution and stop behavior. |
| `runtime.md` R1–R6 and event/scheduler rules | No semantic replacement. Require trace equivalence for indexed state and ready paths. |
| `multicore.md` capture/portable rules; `native.md` N5–N8 | No replacement. Complete unstarted records may travel; live continuations stay local; shared commit stays locked. |
| `host-concurrency.md` root handoff | No replacement. Keep one root snapshot and the owned/borrowed distinction. |
| `lazy-reentry.md` cost discussion | Retain O(1 + D + E); add the proposed paragraph below after coordinator review. |

Suggested text for a performance note, not normative language text:

> A task cycle includes library scheduling, scope accounting, service retention, portability checking, and cleanup as well as compiler continuation operations. Report these costs separately. Resume cost depends on delimiter and environment records, and capture cost depends on saved frames and values. A scheduler optimization must preserve observable turn order, cancellation points, environment behavior, and event validation. A measured native reference latency is not a bound on i or on WasmGC.

No new general language feature is needed for priorities 1–4. Priorities 5–7 concern representation and optimization, not new user-visible capabilities. If they require a new exposed primitive, that is a separate design: it must pass the three-unrelated-uses, sets-of-values, principal-inference, and ordinary-library tests before acceptance.

## Adversarial checks and concrete test proposals

These are source arguments and proposed tests, **not executed tests**. Fragments use accepted new syntax, with module imports omitted. Test fixtures must arrange the usual task entry and services. Expected results come from the named rules, not from what an untested optimizer happens to do.

### A. Shared graph: a cost hidden in “small capture”

```i
D = End | Fork
End { End }
Fork { Fork(D, D) }

shared = (n: Int) -> D {
  n |> {
    0 -> End
    _ -> {
      x = shared(n - 1)
      Fork(x, x)
    }
  }
}
```

`core.portable(shared(16))` must be `True`: all leaves and constructors are portable. There are only linearly many distinct constructed nodes, but B `$core_portable` pushes both children on every visit without marking these nodes (`runtime/values.rs:409–467`). Inference: its visits obey a doubling recurrence. Count visits on small inputs before testing large depths. After a sharing-aware change, require visits proportional to distinct reachable nodes/edges; the public Boolean remains unchanged. Also capture this graph in a task and verify the same outcome. Do not “fix” the cost by dropping unused captured services or rejecting the graph.

First attack found a real omission in an initial optimization direction: reducing continuation frames alone leaves this possible capture-time blowup. The recommendation now profiles and fixes graph traversal separately. It does not copy BEAM header tagging or pretend an identity hash is already available.

### B. A tail-resume rewrite can run under the wrong service

```i
Read { Read(get: () -> Int) }

read = (r ! Read) { r.get() }

probe = () {
  ! Read = Read(() { 1 })
  control.boundary((b) {
    ! Read = Read(() { 2 })
    x = control.suspend(b, (k) { control.resume(k, read()) })
    x
  })
}
```

Expected result: `1`, because the suspend handler runs at the boundary environment (`spec.md:1229`); the resumed inner binding remains `2` if the resumed body reads it. A naive direct call to `read()` in the inner body gives `2`. The non-tail `x` binding also keeps the inner frame explicit in this probe; implementation fixtures should retain the accepted D16 environment tests (`runtime.md:800`). Koka `under1` is evidence that context restoration is part of the fast path, not optional overhead.

First attack rejected blanket tail-resume elimination. Only a proven transformation preserving handler and resumer environments, active-boundary checks, and one-use behavior is recommended.

### C. An apparently pure closure can still suspend

```i
call = (f: () -> Int) { f() }

probe = () {
  control.boundary((b) {
    call(() { control.suspend(b, (k) { control.resume(k, 7) }) })
  })
}
```

Expected result: `7`, with a real control transfer under §9.4. The function type need not expose an error or service row for `suspend`; its boundary is captured. Therefore an effect-row-only no-suspend inference is unsound. The compiler conservative unknown-callee rule is justified (`control.rs:245–285`). A direct-callee proof can improve it; deleting that conservatism globally cannot.

A related negative probe resumes an aliased continuation a second time. Expected: program stop, from §9.4 rule 5, even if the first resume is optimized. A ready-path optimization must not make the second use succeed.

### D. A direct scheduler command has nowhere to put its new state

```i
probe = () {
  task.scope((s) {
    t = task.go(s, () { 1 })
    task.join(t)
  })
}
```

Expected result: `1`, and the child enters the ready queue while the parent retains its turn until the join rule schedules (`runtime.md` §6.4). In B, `route` suspends to return a `Switched` command to the outside loop (`task.i:677–679`, `1904`). Replacing it with a direct reply creates no new scheduler State containing the child. This is a concrete “cannot be built by that local edit” counterexample. The proposal now explicitly defers capture-free state transitions to a general state-threading/control specialization, rather than claiming a one-line Koka optimization.

### E. Choice and cancellation must own every registration

Build a choice with a send and receive on the same otherwise-unused unbuffered channel. Expected: no self-transfer; absent another producer or waiter, the task deadlocks. Add an always-ready alternative before an invalid channel leaf. Expected: validation failure before choosing the ready arm. Add two waiting peer tasks, then cancel the oldest before transfer. Expected: the next eligible FIFO peer receives exactly once. After any choice wins, invoke the losing channel from another task. Expected: no stale arm consumes its value. All expectations follow `runtime.md:867–945`.

For a send/receive event mapper that raises after commitment, the transfer remains committed and the mapper error is propagated; it cannot roll back a value another task already received. Check exactly one receive, not merely that the sender failed. These scenarios attack registration indexing, callback order, and cancellation removal together. A queue tombstone is safe only if generation/commit checks make it ineligible and cleanup bounds retained garbage.

### F. A result is not a clean task

Run a child whose finalizer yields once and starts then joins a fresh cleanup child. Cancel the parent scope while the child waits. Expected: finalizers finish and cleanup children join before scope exit; cancellation must not keep restarting the same finalizer. Separately, join a cancelled handle from a shielded region. Expected: the accepted cancelled-join stop behavior, not a fabricated successful result (`runtime.md:699–798`). Repeat cancellation with the same cause: no duplicated cleanup. Reuse a task-table slot only with a new identity/generation: an old ready entry must not execute the replacement task.

These tests rule out replacing `outcome`/`clean` with one bit, replacing structured cancellation with continuation abandonment, or making counters reach zero before finalizer-created work is registered. They preserve the cause-sensitive behavior already present in B `task.i:980–1010` and `cleanup_chain`/`cause_allowed` at `2461` onward.

### G. An already-ready operation can still be a scheduling point

Cancel a task just before it awaits a settled request, performs an immediately ready channel choice, or joins a completed handle. Expected: accepted cancellation priority is observed; ready operations do not bypass required scheduling. B `lib/i/request.i:152–159` deliberately invokes `Await` even for a settled request. Conversely, a pure task program must not acquire host polling/handoffs merely because a global host runtime exists (`runtime.md` §6.4). A control wake without an owned root producer or borrowed waiter must not conceal deadlock.

For multi-worker transport, capture a nonportable service which is unused by a child returning `1`. Expected: `go` succeeds and the complete record is silently pinned to its home worker; joining gives `1` on every worker count (`multicore.md` D2/D4/P1; `native.md` N7). Separately send a nonportable value over a portable channel: expected stop on every worker count, under D2/P3. `core.portable` of a kept function retaining the nonportable service remains `False`. A worker may steal an unstarted complete record; it must never move a live one-shot continuation (`native.md:88`). These cases reject capture pruning and unrestricted work stealing.

### Review passes

The initial adversarial passes exposed the known shared-DAG cost as a missing optimization target, the missing state destination in a proposed direct spawn, and the dynamic-environment counterexample above. Source review also corrected a draft test that wrongly expected nonportable capture to reject `go`: it pins; portable transfer paths reject. The note was corrected and narrowed: index ordinary scheduler data first; treat graph traversal as a separate issue; require context-aware compiler proofs; preserve capture and cancellation semantics.

A subsequent code/feasibility pass checked the proposals against B queue operations, scope cleanup, request interests, portability traversal, and compiler lazy capture. It found no further serious conflict after adding generation checks, collision-bound caveats, private-index dependency limits, and the requirement to audit environment writes before sharing. It did not establish a measured gain or a complete implementation proof.

A final source-conflict pass checked §9.4–9.5, task R1–R6, ordered choice, one root handoff, one-worker portability, and native worker ownership against tests A–G. It found no further serious conflict in the bounded proposals. This is two clean review passes after corrections, not two successful executable test runs. General CPS specialization and representation changes remain explicitly unproved work.

## Risks, coordinator questions, and limits

- **Which work should go first?** Recommend counters and queue/task indexing, then registration and dirty-scope indexing. The DAG probe is urgent and cheap to add; the representation fix can proceed separately if counters confirm its relevance. Do not promise a speedup ratio from reading code.
- **Should i add preemption or adopt child-first/LIFO execution?** Recommend no change in this track. These alter accepted traces and fairness; they need a separate explicit policy decision and workload evidence.
- **May unused services be removed from task capture?** Recommend no. The current capture/portable contract observes them. Share immutable representation while preserving the graph instead.
- **Should capture-free scheduler commands be the immediate goal?** Recommend no as the first implementation step. The mutable reference algorithms cannot be pasted into the current immutable loop. Approve a general compiler investigation only after counters distinguish capture from bookkeeping.
- **How should portability memoization be represented?** Recommend that the representation track advance the existing D9/§3.6 work and choose between safe object summaries and identity visitation after auditing recursive closure initialization and GC. Do not add pointer identity to ordinary i just for the scheduler.
- **What performance target is justified?** Recommend reporting operation-specific curves and allocations first, with native and Wasm reference comparisons on the same machine. Sub-microsecond creation is a target to test, not an established requirement or a source-code fact.

Not verified: no i executable tests, benchmark, compiler dump, reference build, or comparative measurement was run. No engine allocation profiler or GC trace was collected. No full audit of all environment mutation sites or proof of general CPS specialization was completed. The OTP JIT/dirty-NIF paths, all platform pollers, GHC assembly context-switch paths, and Koka generated native code were not exhaustively read. The requested old Effect file does not exist in this pin; the current implementation was read instead. Later changes to `lane/b` are outside this pinned study. No result is claimed for those gaps, and no mechanism here depends on a claim from memory.
