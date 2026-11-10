# Design proposal: native packages and multicore workers

Status: proposal. It comes from a discussion with the user on 2026-11-10, and the user agreed to its direction. It is not reviewed. Nothing in it is implemented or measured. Design notes were read on `redesign` at `1e0d910` (`multicore.md`, `runtime.md`, `host-concurrency.md`, `lazy-reentry.md`), and code on `main` at `229e3a4`.

Revision 2 (2026-11-10) takes the findings of the independent review that the redesign coordinator ran: N4 is narrower, N9 has a representation that the host boundary allows and a clean-up rule, N12 is new, the sentence on moving continuations is corrected, and part 9 says what the review checked.

This note changes four things that the redesign plan lists as settled. Part 8 lists them. Each needs the user's decision in the plan's own process.

## 1. Goal

- `i build` makes **one executable file** by default: the program, compiled ahead of time, with the Wasmtime runtime.
- A program uses **all cores**, with multicore speed that is not bad.
- i stays small and easy to understand, in what the programmer sees and in how it is made.

## 2. What the programmer sees

Three rules. The language has no worker count.

1. `go` starts a task. The task may run on any core.
2. Values do not change, so a program cannot see if a value was copied or shared.
3. A task that holds a boundary, a continuation or a host reference stays on the core where it was made.

These are the rules of `multicore.md` (S7). This note keeps all of them: D1 (one semantic rule), D2 and D3 (portable values, checked on every worker count), D6 (stable ids), D11 (first stop wins), and `scope.share`.

## 3. Decisions

**N1. `native` is a new target, and the default of `i build`.** `--target ilang` and `--target wasi` stay. `i run` and `i test` do not change: they compile and run in the process. *Proposed default, which the user has not objected to:* only `i build` changes.

**N2. A native package is a runner with the program inside.** The runner is a program built ahead of time for each target triple. It has the Wasmtime runtime with no compiler (no Cranelift), the standard host services, and the reactor. `i build` compiles the module to a `.cwasm` with the Cranelift that `i` already has (`ilang-host` uses Wasmtime 50.0.0 with the `cranelift` feature), and puts it into a copy of the runner.

- A `.cwasm` belongs to one Wasmtime version, one target triple and one engine configuration. The runner and `i build` must use exactly the same three. A test must check this.
- Loading a `.cwasm` is unsafe in Wasmtime's API, because the bytes are trusted. The runner loads only the bytes inside its own file.
- To build for another target, `i` needs the runner of that target. The release archive has them, or `i` fetches one. This is open (part 9).

**N3. The native target uses the i binary interface (ABI v3), not WASI.** On the WASI target an async host function is a blocking call, so a program cannot wait for many requests.

**N4. On the native target, an import that the program needs and the runner cannot give is a compile error.** A native package has no embedder. The error names the import and `--target ilang`. A program that declares a host service and replaces it with ordinary i code (§9.7) needs no import, and builds. *Reason (review):* the first form of this rule refused every user host service, and so refused programs that run today.

**N5. Workers.** The runner starts N threads. Each thread has one Wasmtime `Store` with one instance of the module, and runs the same scheduler, which is ordinary i code (the M:1 scheduler of R2). N is the number of cores by default. The runner reads another number from its environment or from `i.toml` (open, part 9).

**N6. A started task does not move, and nothing is preempted.** A task that computes for a long time without a scheduler call delays the other started tasks of its worker. Other workers go on. The docs say this.

**N7. One shared queue of tasks, encoded at `go`.** This takes the place of cooperative stealing (`multicore.md` D8).

- At `go`, a portable task is encoded into bytes and put in a queue that the runner holds. The worker also keeps the task itself.
- A task that is not portable goes only to the worker's own queue (rule 3 of part 2).
- An idle worker claims an entry of the shared queue. One atomic step changes the entry from "ready" to "taken", so only one worker gets it.
- When the worker that made the task claims it, it runs the task it kept, and decodes nothing. Another worker decodes the bytes.
- The check for "portable" runs once, at `go`, so its answer does not depend on timing.

*Reason:* only the worker that owns a task can encode it, because no other worker can read its GC heap. Encoding later ("when stolen", or "when a worker is idle") needs the owner to reach a poll point, and an owner in a long computation does not. Encoding at `go` needs no protocol, no interrupt and no help from a busy worker. The cost is one encode for each `go`, also when the same worker runs the task.

**N8. Shared scheduler state is in the runner, behind one lock.** Channel state, wake-ups, the stop cell and the shared queue are in the runner. A message between workers is encoded bytes. This takes the place of the lock-free parallel CML protocol that `multicore.md` part 7 plans for the multicore wave. The scheduler stays i code: the runner gives a small set of general operations (a byte queue, a lock, a wait). Their exact form is open (part 9). *Reason:* one lock is the easiest to get right. It is replaced only when a measurement shows that it is the limit.

**N9. IO resources are integer handles, not host references.** A socket, a listener, a file and a child process are values of a private nominal type that holds one `Int`. The raw host service takes and gives that `Int`, and ordinary i code in the library module wraps and unwraps it. This changes `runtime.md` D9.

- *Reason for this form (review):* §10.3 lets only `Nil`, `Bool`, `Int`, `Float`, `String`, `BitArray` and custom types with no constructors go to the host. A custom type with an integer field gives E0320, and a type with no constructors is a host reference, which is not portable. A private type around an ABI `Int` needs no special case in the compiler.
- Such a handle is portable by the S7 rules, with no special case. So one accept loop with `go` for each connection uses all cores, as in Go.
- The number is never used again, so the table is a map from numbers to resources, and a use after close finds nothing. (The first form of this note had an index and a generation; one number that is not reused is simpler.)
- An `Int` can be made by any code, where a host reference cannot. So the raw services are not exported from the library modules; only the wrapped functions are. If this is enough is a question for the IO design (part 9).
- A handle has no GC lifetime. It is closed explicitly, as `resource.ensure` already does.
- **The table belongs to one run, and the end of the run closes everything left in it.** Final functions do not run at a stop (§9.5), so this rule, not a final function, frees host resources then. On native, a run is the process. Under `i run`, `i test` and an embedder, the table is dropped with the run, as dropping the instance frees host references today.
- Two workers that read one socket get a result that depends on the order. D1 of S7 allows this. The docs say it.
- A host request (`Request`, `Ticket`) stays not portable.

*Reason:* host references are not portable because of JS realms, and a browser has no listening socket.

**N10. With one worker, nothing of this runs.** The shared queue and the lock are skipped. The behavior and the cost are those of M:1 today.

**N11. One encoded format, and a transport for each host.** On native, the bytes are in the runner's memory, and the module needs no shared linear memory and no thread-safe `ilang_alloc`. In a browser, the same bytes go through a shared linear memory, as the plan says.

**N12. A stop ends every worker.** D11 of S7 says that a stop anywhere stops the whole run. A worker in a loop with no scheduler call never sees the stop cell. On native, the runner prints the message of the first stop and exits the process. In a host that lives on (an embedder with more than one worker), the host uses Wasmtime's epoch interruption so that the other stores trap. That is the plain use of epochs (a trap at the deadline, with no callback), and it is turned on only when there is more than one worker, because the checks cost time in all code. *Reason (review):* without this, a run with a stop may never return.

## 4. What is not done, and why

- **Cooperative stealing, or publishing only when a worker is idle.** Both need the owner to reach a poll point (N7).
- **An interrupt for encoding** (Wasmtime's epoch interruption). It closes that gap, but the task queue must then be safe against an interrupt at any point, and two facts about Wasmtime are not checked (part 9). N7 makes it unnecessary.
- **Moving a started task by copying its continuation.** After lazy re-entry a paused continuation is heap data (frame records with a function reference and saved values), so this is possible later. Its cost is the size of everything the task can reach. This note asks only that nothing blocks it. The stable ids of D6 are for the sameness of function values; they are not enough here. A frame record holds a reference to resume code, and moving it needs a stable number for each such code place, which is a different thing and is not designed.
- **A second back end on linear memory with Wasm threads.** It gives a real shared heap, but i must then have its own collector for many threads, and two back ends and two runtimes.
- **A fork of Wasmtime with shared-everything threads.** Rough estimate: 1 to 2 person-months for a trial with the null collector, and 1 to 2 person-years until it can be trusted, plus the cost of keeping a fork. Wasmtime 50 has the feature flag and the validation only.
- **The component model.** Components share nothing and copy only WIT types, so it does not help with cores. The runner may use Wasmtime's WASI implementation in Rust for its own IO; that is a choice of the IO stage.

## 5. Targets

The numbers are proposals. i has no HTTP measurements today, so the first run calibrates them.

| Group | Workload | Target |
|---|---|---|
| Server | TechEmpower Plaintext and JSON (the definitions; the project is reported as archived since March 2026) | Throughput on N cores at least 0.8 × N of one core for N ≤ 8, and 0.6 × N for 16. **Must hold.** |
| One worker | The programs in `bench/` | Within 2% of M:1 today. **Must hold.** |
| Parallel computing | Mandelbrot or spectral-norm in chunks, with the input in `scope.share` | Speedup at least 5.5 on 8 cores. **Must hold.** |
| Absolute | JSON on 8 cores | Not below Node.js `cluster`, and Gleam (mist) or Elixir (Bandit). Wanted. |
| Distance to Go | The same, against Go `net/http` | Recorded only. |
| Weak points | skynet (10^6 very small tasks); ping-pong between workers; p99 of requests while one long computation runs | Recorded only. They decide part 6. |

Run each with 1, 2, 4, 8 and 16 workers, and keep the curve. The server group needs `i/net` and the `http` module. Before that, measure the time to encode and decode the task of a usual request; this is the first number that tests N7.

## 6. Later steps

None of these changes a program, because the S7 rules hold.

1. The lock is the limit: use finer locks, or a lock-free protocol.
2. Encoding is the limit: build the Wasmtime trial with the null collector, and measure what sharing gives.
3. Wasmtime gets a shared GC heap: pass references, and remove the encoding.

## 7. Stages

1. **Native package with one worker.** A runner crate, the `.cwasm` inside it, the new default of `i build`. After R2 (`i/task` and the request table).
2. **Workers.** In the multicore wave (MC-D, gate G5), with N5 to N11 as the proposal.
3. **Measure** (part 5), and choose from part 6.

## 8. Changes to what the plan calls settled

1. **`multicore.md` D8:** cooperative stealing becomes the shared queue, encoded at `go` (N7).
2. **`runtime.md` D9:** IO resources become integer handles (N9). That note is in review now.
3. **`multicore.md` part 7, multicore wave:** the parallel CML protocol becomes one lock (N8).
4. **The default target of `i build`** becomes `native` (N1), and the plan gets the native package as new work.

## 9. Not checked, and open

Checked by the review, with probe programs on Wasmtime 50: a Wasmtime runtime with no compiler loads and runs a `.cwasm` that this compiler made; one `Store` for each worker works, and an idle `Store` can be called from another thread; a shared GC heap is not there. The review also found that `Program::from_bytes` in `ilang-host` calls the compiler, so the runner cannot use it as it is; it must be split first.

Still open:

- The cost of one encode and one decode. Nothing is measured.
- The size of a runner built with no Cranelift.
- If keeping the raw IO services private is enough against made-up handle numbers (N9), or the numbers must also be hard to guess.
- How the `.cwasm` is put into the runner (bytes after the end of the file, or a section), and what code signing on macOS and Windows needs.
- Where the runners of other targets come from.
- The name and the place of the worker count.
- The exact operations that the runner gives the i scheduler (N8), and that `choose` and `sync` across workers are right under one lock.
- Deadlock detection and the stop cell across workers (`multicore.md` parts 6 and D11) with the state in the runner.
- A handle after close while another worker waits on it (N9).
- For the interrupt of part 4 only: that an epoch callback may call into the same instance, and that it may allocate GC objects.
- When Wasmtime will have a shared GC heap. Its tracking issue is #9466. The last clear statement found (November 2024) says that nobody had started it for GC.
