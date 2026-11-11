# Design: native packages and multicore workers

Status: adopted, revision 4 (2026-11-10). The ledger's “Native proposal ADOPTED” records
the user's adoption of revision 2, including the four replacements in §8. This revision
reconciles runtime.md and multicore.md and closes the lifetime, stop and forged-number
gaps. Revision 4 applies coordinator rulings N-R1–N-R6 after the independent
review of 60f37f2; §11.1 records the repairs and regression cases. It changes no implementation. Performance targets are acceptance goals, not
measurements.

Controlling sources, in order: spec.md (§4.1/§4.7, §9.5/§9.7, §10.3, §11.2/§11.7,
Appendix B); accepted syntax.md; runtime.md, multicore.md, host-concurrency.md and
lazy-reentry.md; the plan's “已定的设计” and R/IO/MC stages; the ledger's subsequent adoption
decision. All examples use the accepted syntax. Runtime's accepted contextual entry,
Root/Owner, shielded-final and join corrections stay.

## 1. Problem and goal

i is a standalone language: i run supplies its host; i build should make one executable
containing the program and Wasmtime. One portable accept loop should be able to use all
cores. No keyword, compiler rule for an IO library, static Send constraint or change to
principal inference is needed.

Separate three lifetimes. A task/scope belongs to an i scheduler run. A Request belongs
to a worker and its Root/Owner group. An OS resource belongs to the host context of the
enclosing entry invocation, across its workers and nested scheduler runs. Closing a
request group does not close resources already given to callers. Ending the entry
context does. Confusing these lifetimes leaks provisional children or lets the next test
reuse a stale number.

## 2. Alternatives, criteria and choice

Choose by the controlling spec, set-of-values typing, principal inference, ordinary
libraries, precise ownership, isolated sequential/concurrent runs, finite host critical
sections, and the amount R1/R2/IO would have to redo. These alternatives were considered
before selecting the rules below:

1. **Keep opaque host references and cooperative stealing.** References cannot
   be fabricated as Ints and have existing host ownership. But they pin socket
   handlers and cooperative stealing waits for a busy maker. Keep their locality
   for user opaque references, and keep the existing request cancellation seam.
2. **Sequential numbers, private raw bindings, immediate native exit.** This
   is small and makes ordinary resource values portable. Privacy alone does not
   authorize a host import; immediate exit kills an in-process child supervisor.
   Keep nominal Int wrappers, checked non-reuse and process exit as the final
   native action, after host teardown.
3. **Random Int tokens or a new unforgeable portable capability type.** Random
   tokens reduce guessing but cannot make equal Ints different authorities.
   A new primitive adds typing/ABI/copy work; opaque references would reintroduce
   locality. Reject both. Keep kind checks and explicit host permissions, which
   are needed even with secret tokens.
4. **Run-owned Int table, authenticated binding grants and one locked queue.**
   Choose this. It needs no new value type: File is the set of values made by
   its private constructor over Int; the compiler sees ordinary nominal data.
   Host permissions are a general binding policy, useful for IO, plugin stores
   and application credential services. They add no typing constraint or
   attribute. Keep choice 1's local Request/Ticket and choice 2's simple counter.

## 3. Decisions and exact rules

**N1. Native is the default target of i build.** Explicit --target ilang and --target
wasi remain. i run/i test continue to compile and execute in a host process. Native
packaging requires the program entry; a package with only host exports uses --target
ilang, with a clear missing-entry diagnostic. Only build's default changes. Source:
adopted revision 2/ledger.

**N2. A native package is a prebuilt runner plus AOT program bytes.** Build precompiles
with its compiler-enabled Wasmtime; the runner has the runtime, standard services and
reactor, without a compiler backend. The artifact/runner contract pins Wasmtime version,
target triple, CPU feature baseline, collector, engine tunables and
epoch-instrumentation mode. Do not infer build-machine CPU features for a distributed
baseline. Check a manifest before unsafe deserialize; load only the trusted payload
packaged for that runner, never an arbitrary user .cwasm. Program::from_bytes must be
factored, not reused unchanged (§9/§10). Reason: the review's runtime-only probe and
current constructor, cited in §9.

**N3. Native uses the i ABI v3 wire, not WASI.** WASI async imports are blocking under
§B.7 and cannot provide overlapping accept/read. Ordinary new host services can use v3.
New runtime imports/statuses/host-called exports need an ABI review and version bump
under host-concurrency.md; N8 does not silently add them.

**N4. Reject only required imports the runner cannot supply.** Report the
namespace/label and suggest --target ilang for embedding. Apply this after inferred
needs and ordinary i service substitution; a declared service replaced with a local fake
needs no import. Permission withheld under §3.1 means the runner cannot supply that
binding. Merely declaring a host type is never a native packaging error. This is the
user's adopted N4, not a namespace ban.

**N5. One Store and instance per executing worker.** Each runs the ordinary i M:1
scheduler, with all mutable scheduler state behind Scheduler. Only the entry run may use
multiple workers; nested runs stay one-worker FIFO as runtime §6.5 requires. Top-level
initialization, static ids and one-worker fallback remain multicore D7. Starting N
copies of main is not the worker entry protocol. N defaults to cores; where its override
lives remains open (§12).

**N6. Started tasks never move; task scheduling is cooperative.** A long pure
computation delays other started tasks on its worker. Other workers can claim
already-published tasks. Whole-run stop is an explicit exception (N12), not task
preemption or migrating a continuation. Keep complete task records, silent pinning, kept
environments, identity and local witnesses (S7 D2–D7/P1–P5).

**N7. One shared queue, encoded at go.** On N workers the maker classifies the complete
immutable record: kept start function, captures, every kept service, scope handle, path
and chain. If portable it encodes once, publishes bytes and retains its original;
otherwise it queues locally, without a stop. Publication registers the child with its
scope and rechecks cancellation under the same lock: a cancel during encoding drops the
unstarted record, frees the prepared bytes and applies R4a to its caller. One locked
transition claims a Ready entry as Taken(owner). The maker uses its original; a remote
owner decodes. Claim/cancel/drop serialize with scope cancellation: cancel first drops
it; claim first routes cancellation to that owner. A claimed but unstarted task may
still be dropped. A started task stays there. Reclaim the queue entry after claim/drop;
a remote claimant owns the payload bytes until decoding ends. Reclaim the maker's
retained original after acknowledgement. Keep outcome versus cleanup completion and
runtime R4a's join rules. Reason: only the maker reads its GC heap; eager encoding needs
no cooperation from it later. Cost: one encode per portable spawn even if it eventually
runs locally.

**N8. Shared scheduler state uses one lock.** Queue ownership, scope/channel state, sync
records, wakeups and the stop cell live in the runner transport. Worker continuations,
Request/Ticket and GC Keyed objects stay in local typed tables; shared state contains
bytes, stable ids and owner tokens. The i scheduler computes commands and applies
decisions. No i code, callback, codec, Wasm call or OS wait executes under the host
lock. CML's exact single transaction is in multicore §7, retaining runtime §6.4's
validation, source/FIFO order, no self-pair, one commit, losing-leaf removal and
post-commit mapper placement. General request-mode byte-state/wake services keep ABI v3;
their exact typed interface and progress proof are an MC-D implementation gate, not a
claim that a mutex alone implements the scheduler. Replace the lock only after
measurements.

N-R1/N-R3 refine that boundary: Root/Owner are portable group capabilities,
with shared count/readiness/cancel-all/wait-none semantics and no Ticket
enumeration (runtime §5.1). Request/Ticket cells and observer cells stay local;
metadata notifications route changes to their home workers. Group waits use
Await and count only their underlying producers, never their own observation
cells. The driver owns separate ordinary program and control Roots, selected
by its accounting policy, not by any module/import rule. Only the program Root
and explicit borrowed producer interests enter quiescence; wake requests in the
control Root still count for ABI status 1. Snapshot runnable/running/claimed work,
pending program deliveries and that program waitable set under the same lock
(runtime §6.4; multicore §6). Atomic delivery publication prevents a false zero.

**N9. IO values are private nominal wrappers over Int.** File, Listener, Stream and
Child each have one private constructor containing an Int, defined in their ordinary
modules. Raw request services take/return Int; wrappers unwrap/wrap and translate errors
explicitly (runtime §7). §10.3 remains unchanged: a wrapper itself is not an ABI value.
A user opaque host reference stays non-portable. The wrappers are portable by ordinary
S7 rules, while Request/Ticket stay local. Export raw service type names alone when
public function environments use them (spec §2.4); hide their constructors, field access
and live field values. The existing general field-privacy rule must be enforced before
Fs (§9/§10). Public exports must not construct wrappers from caller numbers or accept
caller-made raw services. Otherwise a fake raw producer could return a guessed number,
have the module wrap it and pass the forged resource to live wrappers. Resource fixtures
override host bindings explicitly; applications can inject ordinary public wrapper
functions at their call sites. Accessible user services and Clock/Stdio still support
ordinary i constructor fakes. The resource values use ordinary custom
equality/hash/text. An IO number can therefore appear in text; it is not a secret or a
scheduler path id.

**N9.1. One host ResourceContext per live entry invocation.** It is shared by all
workers of that invocation. Nested task.run, Root and Owner keep that same resource
context. A subsequent export call/test/instance gets a fresh one. A host-wide allocator
is shared across those contexts. Each map entry records its context, kind, OS resource
and operation leases; a number is never an OS fd, pointer or request-table slot.
Unknown, nonpositive, closed, wrong-kind or foreign-context numbers give code closed;
close on them gives Nil. Checking arguments never downcasts a resource merely because
its number exists.

**N9.2. Numbers are never reissued within one live host.** Allocate positive
arbitrary-precision Ints monotonically across all contexts and workers, including
provisional replies and failed creations. JS uses BigInt. Free entries, not numbers; no
tombstone for every old number is needed. Never wrap, truncate, reset between tests, or
fall back to a reused fd. Allocation/bookkeeping failure is a memory-exhaustion stop
before exposure; dispose of any already-created OS resource. Tests may inject a small
counter limit: the next allocation stops, it never recycles. A host restart begins a new
identity domain. Raw numbers retained outside it carry no promise; public wrappers
cannot cross §10.3 anyway. Reason: spec §11.2's Int does not overflow; a bounded host
counter cannot silently change that contract.

**N9.3. Lookup, leases and close have one order.** Under a short host lock, test
context/kind/liveness and acquire the operation's lease. Socket directions each have one
lease; read+write may overlap, two reads/writes/accepts give busy. Files have one
read/write/flush cursor lease, children one wait lease. Removal on close revokes leases
and wakes pending operations with closed. A completion committed first retains its
answer; close committed first prevents success publication. In-flight code holds a safe
reference/private OS duplicate, never a pointer to a removed table entry. Release
happens once when references are safe. Close is idempotent/error-free; cancelling its
request cannot undo host cleanup. Child close awaits reaping. A context/kind-checked
cleanup receipt survives removal; concurrent closes join that receipt, including
reaping, and teardown includes already-closing jobs. Free receipts on completion without
permanent tombstones. Preserve the blocking-file cancellation, stdin broker,
partial-write and supervisor rules of runtime §8.1. Non-creating raw operations
use the ordinary request.await_owned helper with an ensure/cancel final
(runtime §5.4); a borrowed request.await does not cancel its producer. Creators
retain the separate await_resource guard.

**N9.4. Provisional resources have an owner before a reply exists.** The context owns
them as soon as created. The host reply's drop guard removes and closes an undelivered
resource. Successful validation and Ready publication disarm that guard, but the table
retains teardown ownership. The ordinary i Ready/Taken guard closes an unclaimed
resource if its awaiting task unwinds. No scheduler step occurs between taking the
number and wrapping it. A forgotten decoded Ready reply or a discarded Taken wrapper
stays in the table until explicit close/context end. Register a host-owned spawn
supervisor/creation job before starting OS spawn. Teardown reaches that job even before
a child exists. Any late-created child is terminated/reaped; native exit waits for every
such job's completion, not only children already entered in the map. Request
cancellation cannot recognize arbitrary Int resources, so it adds no hidden
resource-specific compiler hook or v3 acknowledgement.

**N9.5. Teardown is independent of i finals, GC and worker polling.** Normal entry end,
stop, trap, host failure, abandoned async invocation and dispose all run the same
idempotent host controller: close the start/delivery gate; cancel and detach request
work; remove entries and revoke leases; schedule each OS cleanup once; mark ended. No
Store borrow or i code is required. A creator racing teardown either commits before
ending or closes its late result; it cannot insert into an ending table. Idle reactor
waits are woken. Finite critical sections never wait for a worker executing Wasm.
Resources outlive request Roots when appropriate, but cannot escape entry teardown
through an independent Root/Owner. Deferred cleanup holds host data only, never a
Store/context. General context-aware host registrations receive the invocation context
at start; legacy callbacks may ignore it. Each future captures that immutable context,
never a mutable “current run” pointer that a later invocation can retarget. Registration
factories/options may be reused across tests; resource maps may not.

**N10. One worker keeps the FIFO fast path.** No shared task queue, encode or scheduler
lock. Portability stops at sends/results/shares still run; go never rejects for
non-portability. N9 table ownership/binding rules apply on every worker count.
Epoch-instrumented artifacts still contain checks on fallback to one worker; budget
those checks separately rather than promise zero cost.

**N11. One encoded format, host-specific byte transport.** Native keeps bytes in runner
memory with private Wasm linear memories; it needs no shared Wasm allocator or threads
feature for this. Browser workers use shared linear memory and the S7
isolation/import-module gates. Same-worker paths retain references; remote decoding
preserves identity, sharing and cycles (S7 D6/D9/I1–I4).

**N12. A stop closes the whole run, including non-polling workers.** The first stop is
recorded once with its original message. Close the start/claim/delivery gate in that
transaction; later traps/stop attempts cannot replace it. Stopping invokes no i finals and resumes no pending i
callbacks. An independent host controller performs N9.5, so a worker spinning with
pending requests cannot defer host cleanup.

Every user import, including synchronous services and externals, checks the entry
context's stop gate before starting its host action. An import admitted before
stop may finish its already-started OS effect; no later user action is admitted.
Controller cleanup bypasses user admission and never calls i. This matters on
native while child reaping delays final process exit: remaining Wasm may compute,
but cannot print, start IO or deliver results after the stop. Such code is
abandoned, not unwound through i final functions.

- **Native executable:** report the first stop once, perform host teardown,
  finish every registered spawn/reap job, then exit the process with failure. Do not
  join spinning Wasm workers before exit. Their remaining private heaps and
  process-local OS references die with the process. Immediate process exit
  before teardown would skip Rust destructors and kill the child supervisor;
  it is not the implementation. Detached file/stdin work is not joined (C D6).
  Child cleanup may await natural exit if termination fails, as runtime §7.4
  already permits; no bounded OS-cleanup duration is promised.
- **A continuing Wasmtime host:** use epoch-instrumented code for multicore,
  with a separate Engine interruption domain per simultaneously live entry
  context, including its nested scheduler runs (an Engine clone is not a new domain). Before any Wasm, set trap-on-deadline
  and deadline current+1, with no periodic ticks. Deadline setup and start
  admission serialize with stop under the run gate, releasing it before
  Wasm. The first stop advances only
  that domain's epoch and wakes all reactors. A stopped-context gate forbids
  fresh instantiation, deadline reset, resume/continue or any later Wasm call.
  Thus a worker racing startup cannot arm itself after the stop. Join all
  stopped workers and dispose their Stores before returning the run result.
  A later run gets a fresh domain. Top-level initialization is under the same
  gate and deadline; a startup stop tears down the partially made worker set.
  Host-only child supervisors may outlive that result; they terminate/reap without the old run reactor or Store.
- **Host calls:** epochs interrupt Wasm, not synchronous host code. A multicore
  continuing host must give nonblocking or stop-aware synchronous imports,
  host future polls and destructors;
  otherwise choose one worker or a process boundary for that invocation.
  Request-mode IO starters are short and their waits are cancellable host work.
  Finite lock holds may delay stop; arbitrary user blocking imports cannot be
  advertised as promptly interruptible. Native's controller can exit without
  joining such a worker once resource teardown is safe.
- **Browser:** do not claim Wasmtime epochs work in JS. Multicore requires
  terminable dedicated workers for every executing store, with a host controller
  and resource cleanup outside them. If worker 0 is the calling/main thread as
  in the initial S7 route, a pure infinite loop cannot be terminated there;
  that route must stay one-worker until the dedicated-worker entry route is
  implemented. No callback/epoch re-entry or allocation from an interrupt is
  needed for any of these rules.

Reason: S7 D11/T17, spec §9.5/§11.7, and Wasmtime 50 epoch limitations (§9). Only
scheduling and structured cancellation remain cooperative. Logical end of an entry and
physical OS cleanup are separate; explicit Child.close waits, while a live host may
retain a host-only reaper after a stopped entry returns.

### 3.1 General host-binding permissions (the forged-Int answer)

Private raw constructors/fields are necessary API privacy, but insufficient authority.
§9.7 permits another declaration of a namespace when it has no duplicate field in the
compiled program. A module need not import i/net at all to ask for `i:net/v1.read`. An
external alias is another potential path. Equal Ints are equal values: a kind check
cannot distinguish a forged live Stream number from the original. Random numbers cannot
make a mathematical guarantee here.

Use the smallest exact closure: **a host grants a raw binding to a resolved import
declaration, not to namespace spelling alone**. A grant identifies the trusted package
content identity, module, host type/field (or external), wire signature and module
artifact digest. Check every required declaration before instantiation/import
coalescing; an unauthorized alias must not inherit a grant from an authorized
declaration of the same Wasm name. This is a host permission, not a compile error for
declaring that namespace. Hosts may deliberately grant another declaration; doing so
grants raw table access within that context, including operations on known live numbers.
The type system promises no unforgeability to such a grantee.

The standard Rust/native/JS factories use this general grant mechanism for raw service
declarations whose type names alone are exported; constructors, fields and live field
values stay private under §2.4. Compiler/library resolution produces a trusted binding
plan. i run/test pass it directly; native embeds it with the trusted program payload. An
ilang embedder/JS factory must approve a compiler plan tied to the module digest and
trusted package identities, or explicitly grant the needed raw bindings. A manifest's
self-asserted module name is not evidence. A hand-written Wasm/manifest is not
automatically trusted. Absent a trusted plan, the standard resource factory refuses its
bindings; arbitrary custom imports remain the embedder's explicit authority.
Re-exporting a public wrapper keeps the original grant; it does not grant a new raw
declaration. Locally supplied ordinary i fakes need no host import and still work (N4).

This does not reserve an IO namespace or teach the checker any library name. The same
policy works for plugin storage, database sessions and credential services. It changes
no set of language values or inference rule. It is an explicit host-loader extension
before resource IO, not an already-existing feature or an optional secrecy
recommendation. Keep the trusted plan outside the v3 wire/ilang.abi contract; if it is
later put into mandatory ABI metadata, version it instead of expecting older v3 hosts to
enforce it.

## 4. What is deferred

- Cooperative stealing and idle-time publication are replaced, not alternative
  implementations of D8. N7 needs no interrupt to encode.
- Moving a started continuation remains future work. Lazy frame records contain
  resume function references; moving them needs stable resume-code numbers,
  distinct from D6's language function identity. Nothing here migrates them.
- Shared GC, a second backend, component-model transport and lock-free CML are
  not needed. The review found no usable shared WasmGC heap in Wasmtime 50;
  that is dated evidence, not a prediction about future support.
- Exact locked-state service signatures, browser allocation/reclamation,
  quiescence proof and top-level worker entry are MC-D gates (multicore §7/§17).

## 5. Measurements and acceptance goals

No scaling or packaging-size measurement is available. Calibrate these proposed numbers
on the first implementation and keep the whole 1/2/4/8/16-worker curve:

| Workload | Goal |
|---|---|
| Plaintext and JSON HTTP server | At least 0.8 × N one-core throughput through 8 cores, 0.6 × N at 16 |
| Existing bench programs on one worker | Within 2% of the M:1 baseline; report epoch-artifact fallback separately |
| Chunked Mandelbrot/spectral-norm using scope.share | At least 5.5× on 8 cores |
| JSON against Node cluster and Gleam/Elixir servers | Record versions and throughput; aim not to fall below them |
| Go net/http, skynet, cross-worker ping-pong, request p99 beside pure loops | Record; do not infer success from the chosen architecture |

First measure encode/decode of a representative request task; then compare lock
contention. The coordinator measures in the plan's quiet window. No performance numbers
in this note are results.

## 6. Later choices

If the lock dominates, split locks or use a measured parallel protocol. If encoding
dominates, evaluate a shared-GC implementation. Keep the same public S7 semantics,
identity, cancellation and lifetime tests through either change.

## 7. Order of work: build now and defer deliberately

| Stage | Build now so it is not redone | What waits |
|---|---|---|
| R1 | Immutable published SplitMix and tagged paths exactly as runtime §4; no task/IO dependency | Nothing native-specific blocks R1 |
| R2 request layer | Local Request/Ticket and group-observer cells, portable group observations without Ticket enumeration, atomic subscriptions and separate program/control Root accounting; one root hand-off, consuming take, borrowed await and ordinary await_owned; no resource recognition for Int | Native packaging, worker transport |
| R2 scheduler | Complete task records, detached routing services, retained environment frames, silent pinning, typed keys and single Scheduler boundary; FIFO/CML semantics, pure fresh-key events, stateful-leaf validation, owned-plus-borrowed waitable sets, combined go/run counters and R4a joins | Queue encoding/claim ownership, locked transport implementation and worker entry |
| IO clock, before Fs | General §2.4 nominal field-privacy fix; host ResourceContext owner/controller, non-reusing Int allocator and trusted binding-plan/grant plumbing on Rust and JS; entry exit/stop/dispose hooks | Worker count, packaging layout, browser multicore |
| IO fs/net/process | Private nominal resource values and raw Int signatures from first implementation; owning await_owned/await_resource wrappers, no raw-number injection exports, kind checks, leases, provisional/Ready transfer and host teardown; process supervisor | No intermediate externref services or fd-as-number adapter |
| IO stdio | Per-source broker with context subscribers, copied output buffers/leases, compatibility printer, complete target matrix/tests | Preview-1 adapter project |
| Native packaging after R2/IO | Split compile/load host configuration; compiler-free runner, authenticated payload/plan, exact artifact manifest, N4 validation and default build | Cross-target distribution/signing choices must be fixed before release |
| MC after MC-D/G5 | N7 queue, N8 typed locked transactions, identity/cycle-safe codec, host transport/wake, claim/cancel reclamation, N12 domains/controller, program/control quiescence and browser stop-safe entry | Started-task migration, lock-free optimization, shared GC |

R1 and R2 do not wait for native packaging. Resource-bearing IO must not publish
externref signatures and replace them later. Host lifetime/grant plumbing is IO work
before Fs; it is not deferred to packaging or assigned silently to C. R2 needs one
replaceable Scheduler boundary now, not a Rust scheduler or the old parallel CML
protocol. The plan's hot-file order still controls merge order.

## 8. Adopted replacements

1. multicore D8: shared queue encoded at go replaces cooperative stealing.
2. runtime D9: private nominal Int wrappers and raw Int wire replace IO host
   references, with explicit codecs allowed by unchanged spec §10.3.
3. multicore §7/MC wave: one lock replaces parallel-CML CAS claims.
4. i build defaults to native; explicit ilang/wasi remain.

These were adopted by the user, not waiting for another approval. N4 rejects only
required unsupplied imports. The detailed rules above resolve revision 2's unreviewed
questions; remaining implementation gates are listed in §12.

## 9. Evidence: read versus run

Code facts below were read in this lane at base 051f064. Line numbers refer to that
commit. Older code citations in runtime/multicore retain their stated base.

| Read fact | File and lines |
|---|---|
| Wasmtime is pinned to 50.0.0 with cranelift | crates/ilang-host/Cargo.toml:17 |
| from_bytes builds a compiler-configured Engine and calls Module::new | crates/ilang-host/src/lib.rs:653–681 |
| Host imports are keyed only by module/name; callbacks take arguments without an invocation context; instantiation binds from that map | crates/ilang-host/src/lib.rs:533–562, :708–734 |
| Metadata records signatures, not trusted declaration provenance | crates/ilang-wasm/src/lower.rs:2649–2686 |
| Host lowering has declaration module/span before making imports | crates/ilang-wasm/src/lower.rs:971–1001 |
| Field inference currently enumerates constructors/labels without a visibility test; exported schemes require their module types to be exported | crates/ilang-check/src/types/infer.rs:1538–1634, :566–595 |
| Host boundary rejects ordinary custom types with constructors | crates/ilang-check/src/resolve.rs:1651–1673 |
| Namespace/field duplicate checking is program-wide, not ownership of a namespace | crates/ilang-check/src/resolve.rs:2502–2533 |
| Every test creates a fresh instance but uses its Program host | crates/ilang-host/src/lib.rs:803–825 |
| Futures drop outside the slot lock; drop_all clears all outstanding work | crates/ilang-host/src/requests.rs:174–191 |
| Async abandonment marks instance unusable and drops requests; traps drop requests too | crates/ilang-host/src/lib.rs:1265–1285, :1372–1382 |
| Host run shutdown does not join detached blocking work | crates/ilang-host/src/run.rs:15–24, :28–68 |
| Current standard service only prints; resources here are still a design | crates/ilang-host/src/services.rs:44–70; lib/i/fs.i:1, lib/i/net.i:1, lib/i/process.i:1 |
| fail.map_error accepts a thunk then mapper, used by explicit wrappers | lib/i/fail.i:27–32 |

Wasmtime 50 source was also read in the local registry at
`/home/agent/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/wasmtime-50.0.0/`:
`src/config.rs:679–737` documents engine-global epochs, required deadlines and lack of
interruption in blocking host code; `src/runtime/store.rs:1058–1116` defines relative
deadlines and trapping. These are local source readings, not new runtime probes. One
Engine per live interrupted entry context avoids shared-epoch cross-talk and
overflow/rearming questions; caching across domains is MC work.

I read the independent revision-1 report and its probes under /tmp/review-native,
including wasmtime-probe/src/main.rs and wasmtime-results.txt. That reviewer ran
runtime-only .cwasm loading, actual i start/main, idle-Store transfer and negative
shared-GC probes. I did not rerun those builds. Neither that report nor this reading
verifies N12, grants, real IO or the future scheduler. The concrete programs below are
required implementation fixtures; they cannot execute on the still-empty IO/task
modules. Only the small abstract sequence model described below was run in this
adoption.

## 10. Replacement spec text and implementation regions

Apply with runtime §10 and multicore §13, without renumbering spec sections. The other
accepted text stays; these instructions supersede their older backend and resource
clauses. They do not edit spec.md in this lane.

**§2.4/§6.7 nominal privacy.** Keep the rule that a type in an exported function
environment must itself be exported; exporting T without constructors does not export
its fields. Add to §6.7:

> Selecting a field of a nominal value requires the same constructor visibility
> as taking it apart by a constructor pattern (§2.4). Exporting T without its
> constructors permits neither field selection nor constructor update outside
> its module. Structural record fields retain their own rules.

This makes §6.7 explicitly honor §2.4, without a special rule for host services.

**§9.7 and §10.1, general host permissions.** Add:

> A host grants the implementation of an import; a namespace does not grant
> authority by itself. A host may restrict a binding to trusted resolved
> declarations and reject other declarations or external aliases of that name.
> Declarations and ordinary i replacements remain legal under the usual rules.
> A binding policy does not change the type of a service. The standard resource
> hosts use trusted declaration grants as well as context and resource-kind
> checks; raw resource numbers are not secret capabilities.

**§9.7.1–§9.7.5.** Use runtime §7's resource definitions, private raw fields, public
wrapper transformations, type-only service exports, field privacy, no-injection rules
and stable errors. Include runtime §8.1's context, lease, provisional/Ready and teardown
behavior. In particular File/Listener/Stream/Child wrap Int privately; close is
idempotent, competing same-direction operations give busy, and a close winning the
commit order settles pending operations with closed. Do not copy old opaque types.

**§9.9.** Use runtime §6 and multicore §18's reconciled API rules. Add:

> go does not reject a non-portable task. Its complete record is classified
> silently at publication; a non-portable record stays on its home worker.
> A started task and a host request never move. A local scope still pins all
> its children even when their results are portable IO values.

**§10.3.** Keep the value/error lists unchanged. Add:

> A nominal resource value with an Int field does not cross the host boundary.
> Its ordinary library wrapper passes only the Int to the raw service. The
> outer Request/Result declaration rules add no ABI value type.

**§11.2, §11.4 and §11.8.** Keep Int arithmetic and ordinary nominal-value
identity/hash/text. No secret-number or special IO display rule is added. S7's hidden
scheduler ids apply to task handles, not arbitrary host data.

**§11.7/§11.9.** Retain first-stop, no-finals and S7 portability rules. Add:

> A whole-run stop terminates all its workers even if a worker has no scheduler
> call. The host reports the first stop once, prevents new work or delivery,
> and tears down its requests and resource context independently of i final
> functions. Another live run is unaffected. Deferred OS cleanup owns no
> language instance; child processes are terminated and reaped under the
> resource host's cleanup contract.

**Appendix B.** Keep ABI v3 wire and H1–H14. The trusted binding plan is outside
ilang.abi, bound to exact artifact bytes by the host/build trust path (§3.1). Native
uses private linear memory, browser MC uses shared memory; N8 uses ordinary service
imports, not new runtime exports. Resource-context teardown must cover status 0,
stop/trap, host failure, async abandonment and disposal. §B.7 retains blocking imports
and no new preview-1 adapters (runtime §8.2). An Int-based adapter still needs buffer
access; a reusable fd is not its number.

**CLI build target text.** Replace build's default target with native. Document explicit
ilang/wasi, N4 required-import diagnostics and N2's artifact contract; i run/test remain
in-process. No language entry typing changes for packaging.

Implementation work (assign to the named stages; no code is changed here):

| Stage | Files and regions anchored in the base |
|---|---|
| IO before Fs | General nominal field-privacy enforcement in crates/ilang-check/src/types/infer.rs:1538–1634 and tests; new ResourceContext/allocator/binding-plan modules beside crates/ilang-host/src/services.rs:44–70; context-aware registration adapters at lib.rs:552–582, options/instantiation grants at :611–634/:708–747; entry/trap/WaitGuard lifecycle at :803–834/:1265–1285/:1372–1382; cancellation slots at requests.rs:174–191 |
| IO i values | lib/i/{fs,net,process}.i:1 replaced by private nominal wrappers/raw Int services, wrapper guards and no number-injection exports; clock.i/stdio.i:1 and runtime §11's IO assignments |
| IO trust plumbing | Compiler resolved declaration sites in lower.rs:971–1001 and import/metadata construction at :2649–2686; create a separate trusted binding plan before import coalescing, not self-authenticating ABI fields; JS loader binding at bindings/javascript/ilang.mjs:784–849 and teardown at :496–556/:881–925; new standard factories/tests |
| Native package | Factor host lib.rs:653–681 into compiler/runtime configurations; crates/ilang-host/Cargo.toml:17 feature split; crates/ilang-cli/src/lib.rs:89–115 build path and crates/ilang-cli/src/main.rs:44–51 target options; new runner/payload manifest and package tests |
| R2/MC | runtime §11 and multicore §14 regions: keep one Scheduler boundary and complete records now; add shared byte services/codec/claims/one-lock transactions only in MC; no direct cross-Store GC access |

## 11. Concrete attacks and required tests

Each expected result follows a named rule. These are design fixtures, not claims that
the unimplemented modules compile today. Use host gates, OS counters and a watchdog for
hangs; do not use sleep timing as the semantic oracle.

**Stop beside a spin.** On two workers force the two children to start remotely:

```i
(i/task)

spin = () { spin() }

@main
main = () {
  task.scope((s) {
    task.go(s, spin)
    task.go(s, () { panic("first") })
    Nil
  })
}
```

N12 requires one “first” message and no i finals. Native subprocess exits after
host teardown without joining spin; a continuing host traps/joins only this
run. A host fixture must test startup racing stop and forbid rearming after
stop. A concurrent run doing 7+1 still returns 8; a subsequent test also returns
8. Shared Engine deadlines and resetting an already-stopped worker fail this.

**Stop with outstanding work and a never-claimed reply.** Start a gated accept and
spawn. Release spawn to provisional, then stop before reply delivery; repeat after
decode Ready and before take, after take with its wrapper dropped, and after normal
entry return. Use a process fixture sleeping until signalled. N9.4/N9.5 require exactly
one close/termination request, eventual reaping, zero new delivery after stop and no i
final. A late successful accept closes its OS socket rather than resurrecting the table.
Native exit occurs after child reaping; the live-host result may precede host-only
supervisor completion.

**Close while another worker waits.** Arrange a read to own its lease, then close from
another worker. Repeat for accept, File read and Child wait. Close wins:
Error(NetError("read", "", "closed")) for the read; close gives Nil; a second close
gives Nil. Completion wins first: retain the original bytes. A second concurrent read
gives busy; a simultaneous write may proceed. These are N9.3/runtime §7 results, not
arbitrary byte distribution under D1. Concurrent close callers cannot both own the
cleanup; an already-removed alias gives Nil.

**Forged number and namespace.** This module imports no standard net module:

```i
(i/request)

@host("i:net/v1", "request")
Pretend { Pretend(read: (stream: Int, max_bytes: Int) -> Request(Result(BitArray, String))) }

@main
main = (raw ! Pretend) {
  request.await(raw.read(1, 1))
}
```

Private exports alone do not block it. §3.1 denies automatic binding before any service
start, whether 1 is absent or a live Stream number. A host may explicitly grant Pretend;
then unknown/foreign/wrong-kind 1 gives closed, while a matching live number acts on
that resource. That is the explicit raw grant, not a claim of unforgeability. Repeat
with an external alias and two declarations coalesced to one import: every required
declaration must be authorized. Also bind Pretend to ordinary fake functions: it runs
with no import under N4. Public wrappers remain ordinary code; File(1) outside i/fs
fails constructor privacy, and using a Listener as Stream fails nominal typing before
any host call.

**Two runs, exhaustion and hidden state.** In run A open number n, retain n in a
host-only fixture, then end A; in run B create n+1 and issue raw operations on n. Expect
closed/Nil, never the new resource (N9.1/N9.2). Repeat with two tests sharing standard
host options and with a second export call on one instance. Set the allocator near an
injected limit and create two provisional resources: the first gets the final fresh
number; the second closes any OS result and stops for memory exhaustion. Nothing wraps
to zero/one. Test JS above 2^53 and Rust above 2^64: exact Int round-trips and equality
survive encoding. A restarted host is a different domain, not a persistence format for
raw numbers.

**Cannot be built / source conflict attacks.** A raw field returning File(Int) still
fails §10.3; raw Int plus wrapper is required. No service field may need forall A;
public wrappers are ordinary generalized bindings. Hiding a raw service type used in an
exported environment would fail §2.4. Export only that type name, and reject `(raw !
net.Net) { raw.read(1, 1) }` outside i/net by general constructor/field privacy; the
current field checker needs the fix. An exported helper accepting a fake raw accept
would forge Stream(guessed); the no-injection export rule in N9 removes it. A
local-scope child returning Stream still stays local because the complete record
includes that scope; a capture containing a Ticket pins it even when its Stream is
portable. Key identity, cyclic closures, scope cancellation after claim/before start,
self-rendezvous in choose, a losing mapper that panics and an inactive scope after
encoding must retain S7/runtime results. Prepare bytes outside the lock, revalidate
liveness inside, and never invoke the mapper under it. N8's typed prototype remains a
real buildability gate.

**Attack/fix record.** Initial attacks found real gaps: privacy did not prevent raw
redeclarations; immediate native exit skipped resource/child cleanup; store-only
teardown could not act while another worker spun; a public raw fake could launder a
guessed number through a wrapper; in-flight spawns could escape exit and concurrent
closes needed a shared cleanup receipt; public wrapper environments required type
exports, and the current field checker could then expose their raw fields despite §2.4;
bounded/resetting numbers and shared Engine epochs could target the next or another live
run. Fixed by declaration grants, no public number-injection path, registered spawn
jobs/cleanup receipts, type-only service exports with general field privacy, independent
context teardown before exit, host-wide arbitrary Int allocation and isolated epoch
domains. The browser main-thread spin was another real gap: MC must use terminable
dedicated workers or fall back.

Pass 1 after those fixes revisited forged live numbers, external aliases, public fake
laundering and raw-field projection through an opaque exported service type, publication
after cancellation, payload lifetime through decoding, provisional/Ready/Taken
abandonment, duplicate close receipts, late spawn ownership, startup stop and nested
Owner escape. No further serious defect was found in the revised rules. Pass 2
independently varied two concurrent contexts, sequential export/test reuse, allocator
exhaustion, synchronous blocked imports, local-scope complete records and cancellation/
commit ordering. No further serious defect was found. These are manual sequence and
code-reading attacks, not executable IO/multicore/OS-cleanup verification.

An abstract Python sequence model was run through stdin in this lane: 1,154 reachable
lifecycle states and 3,652 transitions passed the invariants of unique numbers, no
insertion after context end, one cleanup owner and no native exit while a
creation/reaping job remains. It models two contexts and at most one spawn per context,
with two issued numbers and depth 14; it is not a proof for unbounded executions.
Separate models passed all 120 serial orders of two reads/write/completion/close, six
claim/cancel/start orders and six startup/stop orders. Rejected-rule witnesses included
resetting number 1 into another run, exiting with an empty table but a still-creating
job, and laundering a live number through a public raw fake. These checks validate only
the abstractions; no new Rust, JS, i compiler, epoch or OS probe was run.

The revision-3 documentation audit checked the three allowed paths, 20 i code blocks, 18
Markdown tables, balanced fences, table widths and an old-keyword scan. It also
searched every stealing/CAS/host-reference/resource signature reference and
checked the native/runtime/multicore section targets. This is a text check,
not a parser/type-check of the proposed APIs. The sentence ledger's coverage
and git diff --check are checked again before committing.

### 11.1 Revision-4 review repairs and adversarial cases

Source: the coordinator's N-R1–N-R6 rulings and independent review
`report-review-native3.md`; its full cases are under `/tmp/review-native3/`.
These rulings retain packaging, one Store per worker, publication at go, one
lock, nominal Int resources and all four encapsulation rules. The following
are design tests to implement, not programs executed against the placeholder
request/task/IO modules in this lane.

**Choice for N-R1.** Compare three designs by locality, placement-independent
answers, principal types and the amount R2 must change later: remote/proxy
Tickets need a new management-handle protocol; pinning Root/Owner pins nearly
every task; group observations keep Root/Owner portable and never expose another
worker's cells. Choose the third as ruled. Keep typed local cells from the first
and local implementation indexes from the second, but expose neither as a public
enumeration. No worker-local enumeration API remains, so no new static
non-portability type is needed. Group waits use Await with private local
observation cells; they cannot stop the entire scheduler while waiting for a
sibling. Readiness is count zero, counts are atomic snapshots, cancel-all affects
current Waiting members without closing the group, and wait-none returns on its
next zero observation (runtime §5.1). These operations observe sets of work,
not worker placement, and add no inference choice. Their uses include batches,
timeouts and plugin cleanup, not only tasks.

**N-R1 enumeration replacement.** Hold the host gate until after the count:

```i
(i/task, i/request)
@host("review:gate/v1", "request")
Gate { Gate(wait: () -> Request(Nil)) }
@main
main = (gate ! Gate, root ! request.Root) {
  pending = gate.wait()
  task.scope((s) {
    child = task.go(s, () { request.count_root(root) })
    count = task.join(child)
    request.cancel(request.ticket(pending))
    assert(count == 1)
    assert(request.settled_root(root))
    request.wait_none_root(root)
  })
}
```

Expected: count is 1 whether child runs on the maker or another worker; after
cancel the count is 0, readiness true and wait-none returns Nil. No operation
returns the maker's Ticket. Also run two groups on separate workers: cancel-all
of one leaves the other's count 1; cancel-all-root makes both zero but permits
later starts; close-root forbids later starts. Complete between check and
subscription: the waiter must still finish. A group waiter and a sibling that
sets the gate must finish on one worker too, since Await suspends only the
waiter. Cancelling a group waiter removes its observation without cancelling
the producer. The observer itself never adds a unit to the group it waits for.
After a remote cancel-all returns, query state on the maker before it processes
its notification: it must already observe Cancelled. Then start a second read
on the same Stream: the cancelled request's lease must already be revoked.
Host cancellation handles apply that action without a Store borrow; local cell
operations reconcile shared terminal state and never re-cancel a retired ABI id.
Merely queuing a notification failed this attack, so runtime §5.1 now requires
these synchronization points before the MC implementation gate.

**N-R2 race then read.** File/Stream/Child non-creators, including close, use
request.await_owned (runtime §5.4/§7). Resource creators use await_resource;
request.await remains borrowed. Test with a live Stream and a host fixture
that keeps the first read pending until the second branch wins:

```i
(i/net, i/task)
twice = (stream: net.Stream) {
  task.race(() { net.read(stream, 1) }, () { <<>> })
  net.read(stream, 1)
}
```

After the race has returned, deliver byte 97 only to the second read. Expected:
`<<97>>`, with no busy error and no abandoned first read consuming that byte.
The test scheduler forces the first read to acquire its lease before selecting
the second branch; no sleeps decide the order. Repeat at 1/2/4 workers. For
File, cancellation of a blocking read instead makes the alias closed, as the
existing cursor rule requires. For Child, cancelled wait releases the wait lease
and a subsequent wait returns its cached/eventual status without busy; it does
not terminate the child. Cancelling a borrowed request.await waiter leaves its
producer live. Close cancellation never undoes independent host cleanup.

**N-R3 wake-only deadlock.** Replay the review trace: worker 0 waits with one
wake request; the last runnable task on A registers sync(never()); there is no
program IO. Both driver wakes, if present, belong to ordinary control Roots,
scoped to their driver and closed at its exit. Program Root count = 0; explicit
borrowed producer count = 0; runnable/running/claimed work and pending program
delivery = 0 after registration. Record panic("task: deadlock") under the
shared lock even when control count is 1 or 2. The stop closes/wakes the drivers.
If a program request is still Waiting, postpone deadlock until it settles;
if its answer is ready to deliver, count that delivery until the local scheduler
rescans. Repeated wake-only notifications must not mask the zero snapshot.
The same ordinary roots and selection policy can separate maintenance work in
a batch dispatcher, a timeout manager or a plugin loop; no compiler rule names
a library, namespace or wake operation. All actual host requests still count
for H4/status 1, independently of this program-progress predicate.

**N-R4 pure events.** Runtime §6.1/§6.4 wins; multicore §4 and §18 rules 5,
11–12 now distinguish stateful handles from pure event descriptions:

```i
(i/task, i/control, i/core)
event = task.always(1)
@main
main = () {
  task.run(() { assert(task.sync(event) == 1) })
  task.run(() { assert(task.sync(event) == 1) })
  control.boundary((b) {
    e = task.map(task.always(Nil), (_) { b })
    assert(core.not(core.portable(e)))
  })
}
```

Expected: both syncs give 1 without a creating Scheduler or run for event; the
boundary capture makes e non-portable by the ordinary capture walk. Two fresh
always(1) constructions compare unequal; copying one preserves equality.
A receive/join leaf from another run still stops at sync, even when it loses
to always(1); validate all stateful leaves before registration. A pure event
has no invented task path, and user callbacks may capture any ordinary value.

**N-R5 borrowed waitable set.** A raw outer-root gated request may be borrowed
by a nested run on the same worker:

```i
(i/task, i/request)
@host("review:gate/v1", "request")
Gate { Gate(wait: () -> Request(Nil)) }
@main
main = (gate ! Gate) {
  pending = gate.wait()
  task.run(() { request.await(pending) })
}
```

Expected: after the fixture answers, the nested run returns Nil, without a
deadlock stop. Its logical waitable set is owned program-root work union
explicitly awaited borrowed Waiting work, deduplicated by identity; group
observers contribute underlying members, not themselves. Repeat with an
unrelated outer request and sync(never()) inside: now the nested run deadlocks.
A captured Request pins a child normally; no cross-worker borrow is introduced.

**N-R6 combined counter.** The review's go/run/go case is:

```i
(i/task)
@main
main = () {
  task.scope((s) {
    a = task.go(s, task.path)
    nested = task.run(task.path)
    b = task.go(s, task.path)
    assert((task.join(a), nested, task.join(b)) == ([1], [2], [3]))
  })
}
```

Expected: ([1], [2], [3]) on all worker counts, with the second child's generator
derived at P(3), not P(2) (runtime §4/§6.1; multicore D10/§5/§18.11).

**Additional decisions and sources.** The group-operation names and private
observer implementation make N-R1 concrete using existing Await/typed-cell
rules. No observer receives a host id or registers as its own producer; this
follows N-R3 and prevents self-counting deadlock. A first draft made wait-none
a direct root wait; attack with a sibling-controlled gate showed that this could
pause the sibling on one worker, so the final rule routes it through Await.
Atomic check/subscription and atomic request-to-delivery accounting follow N-R1's
placement rule, N-R3's quiescence rule and N8's one-lock transaction discipline;
they close lost-wakeup and false-zero races, not new public capabilities.
Synchronous group-cancel acceptance plus host lease revocation and local terminal
state reconciliation are also forced by N-R1: notification delay must not expose
Waiting or busy after the same cancellation has already returned on another worker.
N9.3/N12 already require nonblocking host cancellation/cleanup independent of
worker progress; MC extends that existing ownership seam to group operations.
All observers and control Roots are bounded by structured waiter/driver lifetimes.
No new detached task, library-name privilege, host wire type or inference rule
is introduced. The helper uses existing resource.ensure (spec §9.5;
lib/i/resource.i:8–13); current host cancellation drops work outside its slot lock
(crates/ilang-host/src/requests.rs:161–184). That code supports the intended
ownership seam but does not yet implement the proposed resource lease behavior.

**Revision-4 checks and two final attack passes.** After the observer and
cancellation fixes, pass 1 replayed the six review cases and varied group close
versus start, zero versus subscribe, cancel-all versus late answer, borrowed
waiter cancellation, and pending/Ready/Taken IO cancellation. It checked that
private observer dependencies cannot count themselves, group operations reveal
no foreign cell, and wake-only deliveries cannot defer deadlock. No further
serious defect was found. Pass 2 checked the same contracts against spec §2.4,
§9.5/§9.7/§10.3 and ABI H4/H13, runtime's exact replacement text, multicore's
invariants and §18, top-level events/captured boundaries, and the combined path
example. No further serious contradiction was found; the intentional spec
migration below is still required.

Executed an abstract Python model through stdin: 2,880 group action-order/home
assignments, six observer subscribe/finish/cancel orders and 192 quiescence
snapshots with varying control counts passed their stated invariants. Two lease
outcomes checked cancellation/completion before a subsequent read. This model
assumes the required atomic transitions; it tests the design accounting, not
the unbuilt notification/lease implementation. The documentation audit checked
26 i code blocks and 18 tables in the active notes, balanced fences, table
widths, accepted-syntax keyword scans, cross-reference targets and stale API
phrases. It is not parsing or type-checking. The existing ensure and cancellation
seams were read at the file/line citations above; no compiler, IO, OS, JS or
multicore execution was performed for this revision. The revision-4 ledger
records every changed paragraph/code/table block against 60f37f2 verbatim,
including every changed sentence; historical revision-3 records are preserved.

**Work order and spec reconciliation.** R2 must implement these group/observer,
ownership, event, waitable-set and combined-counter contracts now, including the
separate control-root seam; IO must use await_owned from its first non-creating
wrapper. General field privacy, binding grants and ResourceContext plumbing still
precede Fs. MC-D must prove typed notification routing and the quiescence snapshot
before MC-I; worker transport, encoding and packaging remain later stages. The
six semantic defects are resolved; no known API replacement is deferred. This
is not a promise that an unbuilt MC prototype will need no implementation changes.

The three notes now agree on these six findings. Spec edits remain intentional
pending work: runtime §10 supplies Request/Root/Owner/Await (§9.11), request-mode
host services (§9.7), contextual entries (§10.2), task/events (§9.9), and their
primitive declarations; native §10 supplies packaging and host-policy text and
the general §2.4/§6.7 field-privacy clarification. The unchanged §10.3 still
excludes nominal resource wrappers from the wire. Appendix B still counts all
actual host requests for status 1; it is not the task deadlock predicate.
No unresolved contradiction among these three active contracts was found in the
revision-4 passes; historical old/new quotes are not active rules.

## 12. Risks and coordinator questions

- **Who owns field privacy and binding-plan trust, and which plans does the SDK trust?**
  Assign this before Fs, not
  mistaken for private exports or an ABI field anybody can forge. Recommended:
  IO owns the general §2.4 field fix and host plumbing in the plan's merge order;
  use a compiler-to-host plan for CLI/native, digest-checked approved plan or explicit
  grants for SDK; fail closed when absent.
- **Which typed MC transaction/wake interface?** Recommended: ordinary request-mode byte
  transaction/wake services, typed prototype and full CML/claim/quiescence review
  at MC-D/G5. A mutex alone is not approval to implement an untyped Rust executor.
- **May native exit before child reaping to meet a stop deadline?** Recommended: no; preserve explicit close's
  reaping contract; native waits for reaping before exit, live hosts retain a
  host-only supervisor. Do not promise a strict stop deadline for OS cleanup.
- **Which browser worker-entry route supports forced stop?** Recommended: controller
  on the main thread with all Wasm in dedicated terminable workers; until then
  one-worker fallback, retaining S7's semantic rules.
- **Which payload layout, signing/distribution policy and worker-count override?**
  Recommended: one authenticated manifest/payload for the first host target,
  packaged runners before fetch, and one documented host option shared by CLI
  and embedder. Set release policy in native packaging; no R1/R2/IO API depends
  on these names.
- Encoding cost, lock contention, compiler-free runner size, exact epoch-domain
  caching cost and all scaling goals remain unmeasured. Shared GC and started
  continuation transfer remain future work.

## 13. Sentence change ledger

The entries below preserve each replaced sentence/block verbatim (including code/table
lines), with its replacement and reason. A block may contain several sentences that
changed together; nothing inside the old/new quotations is a second active rule. This
revision-3 ledger was generated from the lane base to 60f37f2, excluding the
ledger itself; the appended revision-4 ledger records subsequent changes. It includes native's adoption/status corrections as well as runtime/multicore
changes.

<!-- change-ledger-start -->
Base: `051f064fcdb0925b007d9e15cf341e342a573d7e`. Changed sentences are grouped only with the paragraph, code
block or table they replace. Empty old/new text means addition/deletion.
The coverage check reconstructs every quoted changed block from this ledger.

### C001 — docs/design/runtime.md, base lines 3–8

- Old sentence(s):

> Status: proposed joint design for R1, R2, and IO, revised after the coordinator
> review of aa20548. Code references remain at `1e0d910` unless another commit is
> named. The coordinator has accepted contextual top and blocking WASI, rejected
> universal async entries, and required the join/Owner corrections recorded below.
> This note changes no implementation. Examples use the accepted syntax, including
> capital type parameters, imports in parentheses, and clause blocks.

- New sentence(s):

> Status: proposed joint design for R1, R2, and IO, revised after the coordinator
> review of aa20548. Code references remain at `1e0d910` unless another commit is
> named. The coordinator has accepted contextual top and blocking WASI, rejected
> universal async entries, and required the join/Owner corrections recorded below.
> Adopted native.md N7–N12 replace the resource representation and future
> multicore backend here; other accepted runtime rules remain. This note changes
> no implementation. Examples use the accepted syntax, including
> capital type parameters, imports in parentheses, and clause blocks.

- Reason: Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Make the implementation/spec handoff explicit: prerequisites now, packaging and MC mechanisms in their own stages. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C002 — docs/design/runtime.md, base lines 177–181

- Old sentence(s):

> **D9. IO services start requests; ordinary wrappers await and translate errors.**
> Opaque files, listeners, streams and children are host references. The portable
> combinators cannot return these references; servers use local scopes. Stdio has
> its own `i:stdio/v1` service. Keep the normative `i/io` printer for compatibility;
> it does not take over or rename stdio. Reason: spec §9.7, §10.3, S7 P4, plan IO.

- New sentence(s):

> **D9. IO resources are private nominal wrappers over Int.** File, Listener,
> Stream and Child each have one private constructor containing an Int. Only that
> Int crosses the raw boundary. Ordinary i wrappers unwrap arguments, await,
> wrap successful resource numbers and translate String errors. These values
> are portable by S7's general rules; Request/Ticket remain local. No compiler
> codec or portability exception names an IO type. One host resource context
> owns a kind-checked table shared by all workers, separate from request Roots.
> It closes leftovers at entry end, stop, host failure or disposal, including
> provisional replies. Native N9 and §8.1 define leases, non-reuse and teardown.
> Raw service constructors/fields are private; type names alone are exported
> for public environments (§2.4). Namespace spelling grants no host binding
> (native.md §3.1). Stdio keeps i:stdio/v1; legacy i/io keeps its contract.
> Reason: adopted native N9, spec §4.3/§9.7/§10.3, ledger adoption decision.

- Reason: Honor spec §2.4 for exported environments and nominal privacy, using one general field-visibility fix before Fs. Close N9’s forged-Int paths with general declaration grants and no public raw-number injection; ordinary accessible service fakes stay legal. Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C003 — docs/design/runtime.md, base lines 708–713

- Old sentence(s):

> The i loop threads immutable State through tail recursion. It keeps a FIFO
> ready queue, tables of scopes/channels/tasks/shares, and ticket waiters. It
> removes a continuation from State before resuming it once. Resuming can yield
> another Turn; State is changed only after that call returns. No loop boundary
> is captured in a task. The loop starts each task with a new control.boundary;
> all tasks execute from the loop rather than from their parent task's stack.

- New sentence(s):

> All mutable scheduler state stays behind one Scheduler boundary: R2 exposes no
> worker queue or stealing/commit protocol in public handles. Native N7/N8 later
> replace publication and shared-state storage; continuations remain local.
> The i loop threads immutable State through tail recursion. It keeps a FIFO
> ready queue, tables of scopes/channels/tasks/shares, and ticket waiters. It
> removes a continuation from State before resuming it once. Resuming can yield
> another Turn; State is changed only after that call returns. No loop boundary
> is captured in a task. The loop starts each task with a new control.boundary;
> all tasks execute from the loop rather than from their parent task's stack.

- Reason: Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Adopt N8’s one-lock backend while retaining the i scheduler, typed local storage and the accepted CML transaction semantics. Make the implementation/spec handoff explicit: prerequisites now, packaging and MC mechanisms in their own stages.

### C004 — docs/design/runtime.md, base lines 857–876

- Old sentence(s):

> All five modules export the ordinary wrappers below, their service type with
> its constructor, opaque resource types, and error types with constructors.
> Each wrapper needs its module service, request.Await, and request.Owner, unless its row says
> otherwise. All host failures expected from normal OS operations become the
> listed typed errors; malformed host values and implementation failures remain
> host faults under ABI v3. No IO primitive implicitly imports a std package.
>
> Host services below use request mode. The tables give full source fields:
> write them inside `@host("namespace", "request") Type { Type(...) }`. The request attribute adds request.Owner to each callable field's environment;
> these tables give their written boundary declarations. Thus all fields return
> Request, never lists, tuples, records or user error constructors
> on the ABI. The host sees their unwrapped result/error ABI types from §5.2.
>
> | Module/service/namespace | Public opaque types and errors |
> |---|---|
> | clock / Clock / `i:clock/v1` | `ClockError { ClockError(operation: String, code: String) }` |
> | fs / Fs / `i:fs/v1` | `File {}`; `FsError { FsError(operation: String, path: String, code: String) }` |
> | net / Net / `i:net/v1` | `Listener {}`; `Stream {}`; `NetError { NetError(operation: String, endpoint: String, code: String) }` |
> | process / Process / `i:process/v1` | `Child {}`; `ProcessError { ProcessError(operation: String, code: String) }` |
> | stdio / Stdio / `i:stdio/v1` | `StdioError { StdioError(operation: String, code: String) }` |

- New sentence(s):

> All five modules export ordinary wrappers and error types with constructors.
> Fs, Net and Process export only their type names, with private constructors,
> fields and live field functions. Their names occur in public wrapper environments,
> so §2.4 requires exporting them. Public resource names also hide constructors.
> Clock and Stdio retain exported service constructors: they carry no resource
> numbers. Each wrapper needs its module service, request.Await and request.Owner
> unless stated otherwise. The exported service type names may be used in inferred or written
> environments; their constructors and fields remain inaccessible outside the module. Normal OS failures become typed errors;
> malformed host values remain ABI faults. No IO primitive imports std.
>
> Tables give written fields inside `@host("namespace", "request") Type { Type(...) }`.
> The attribute adds request.Owner to callable fields (§5.2). Unwrapped wire
> results/errors obey §10.3; resources cross only as Int. Definitions include:
>
> ```i
> File { File(Int) }
> Listener { Listener(Int) }
> Stream { Stream(Int) }
> Child { Child(Int) }
> ```
>
> Each definition belongs to its respective module. Before resource IO, enforce
> §2.4's existing “may not make or take apart” rule for nominal field projection,
> not just constructor names. The current field checker enumerates all constructors
> and labels without testing visibility (base 051f064,
> crates/ilang-check/src/types/infer.rs:1538–1634); exported_type already requires
> exposed environment types to be exported (:566–595). This is a general compiler
> privacy fix, not an IO-specific field exception. Structural record fields keep
> their accepted rules. A user may request an opaque Fs environment value but
> cannot select its raw fields. Test that at O0/O2 before Fs lands. Export the type without
> `(..)`, so its constructor stays private (§2.4). A wrapper matches e.g.
> `Stream(number)` to unwrap an argument and calls Stream on a successful number.
> No File/Listener/Stream/Child itself is admissible at a host export (§10.3).
> Equality, hash and text follow ordinary custom values over Int (§11). Numbers
> are host data, not secret capabilities, fds or i/task path ids; S7 §18 rules
> 11/25 concern task handles. Native §3.1's binding grants and §8.1's context/kind
> checks provide isolation.
>
> Resource modules export no raw-number injection helper, raw service value,
> or function accepting a caller-created raw service. Such a helper could wrap a
> forged live number returned by a fake accept, then pass that Stream to the real
> read/close wrappers, bypassing declaration grants. Resource wrapper constructors
> are used only on validated replies of the module's private service. Resource
> integration tests override the raw host binding explicitly, which grants raw
> authority to that fixture. Ordinary i code may inject public wrapper functions
> at its own call sites, or replace any accessible user service under §9.7.
> Clock/Stdio retain ordinary constructor fakes; opaque resource services are not
> public raw test APIs. No language rule forbids constructing or replacing services.
>
> | Module/service/namespace | Public resource types and errors |
> |---|---|
> | clock / Clock / `i:clock/v1` | `ClockError { ClockError(operation: String, code: String) }` |
> | fs / Fs (type only) / `i:fs/v1` | File, constructor private; `FsError { FsError(operation: String, path: String, code: String) }` |
> | net / Net (type only) / `i:net/v1` | Listener and Stream, constructors private; `NetError { NetError(operation: String, endpoint: String, code: String) }` |
> | process / Process (type only) / `i:process/v1` | Child, constructor private; `ProcessError { ProcessError(operation: String, code: String) }` |
> | stdio / Stdio / `i:stdio/v1` | `StdioError { StdioError(operation: String, code: String) }` |

- Reason: Honor spec §2.4 for exported environments and nominal privacy, using one general field-visibility fix before Fs. Close N9’s forged-Int paths with general declaration grants and no public raw-number injection; ordinary accessible service fakes stay legal. Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Adopt N1–N4 and the reviewed artifact/loading constraints, keeping explicit ilang/WASI and rejecting only required unsupplied imports. Make the implementation/spec handoff explicit: prerequisites now, packaging and MC mechanisms in their own stages. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C005 — docs/design/runtime.md, base lines 913–937

- Old sentence(s):

> | Raw Fs field | Type |
> |---|---|
> | `open` | `(path: String, mode: String) -> Request(Result(File,String))` |
> | `read` | `(file: File, max_bytes: Int) -> Request(Result(BitArray,String))` |
> | `write` | `(file: File, bytes: BitArray) -> Request(Result(Nil,String))` |
> | `flush` | `(file: File) -> Request(Result(Nil,String))` |
> | `close` | `(file: File) -> Request(Nil)` |
> | `remove` | `(path: String) -> Request(Result(Nil,String))` |
> | `rename` | `(from: String, to: String) -> Request(Result(Nil,String))` |
> | `read_file` | `(path: String) -> Request(Result(BitArray,String))` |
> | `write_file` | `(path: String, bytes: BitArray) -> Request(Result(Nil,String))` |
>
> Public open takes OpenMode and encodes `read`, `truncate`, `append`, or
> `read_write`; write modes create the file if absent, read_write does not
> truncate. Public read/write/flush/remove/rename/read_file/write_file otherwise
> use exactly these parameter and success types, with `? FsError`. Public close
> has no errors and is idempotent. A read requires max_bytes > 0, returns up to
> that many bytes, and returns `<<>>` only at EOF. All bytes parameters must have
> a whole number of bytes; otherwise give invalid before an OS operation. write
> writes all bytes or raises an error; an error/cancel may have written a prefix.
> flush reports data-flush failures. close releases resources even when flush
> would fail; it does not promise durability. read_file opens, reads to EOF and
> closes; write_file truncates, writes and closes, and is not atomic replacement.
> rename errors use the from path; no path normalization or sandboxing is hidden
> in the i wrapper. Expected access denial is code denied.

- New sentence(s):

> | Raw Fs field | Type |
> |---|---|
> | `open` | `(path: String, mode: String) -> Request(Result(Int,String))` |
> | `read` | `(file: Int, max_bytes: Int) -> Request(Result(BitArray,String))` |
> | `write` | `(file: Int, bytes: BitArray) -> Request(Result(Nil,String))` |
> | `flush` | `(file: Int) -> Request(Result(Nil,String))` |
> | `close` | `(file: Int) -> Request(Nil)` |
> | `remove` | `(path: String) -> Request(Result(Nil,String))` |
> | `rename` | `(from: String, to: String) -> Request(Result(Nil,String))` |
> | `read_file` | `(path: String) -> Request(Result(BitArray,String))` |
> | `write_file` | `(path: String, bytes: BitArray) -> Request(Result(Nil,String))` |
>
> Public open returns File by wrapping the successful Int, takes OpenMode and encodes `read`, `truncate`, `append`, or
> `read_write`; write modes create the file if absent, read_write does not
> truncate. Public read/write/flush/remove/rename/read_file/write_file otherwise
> use File in place of resource Int parameters, with `? FsError`; other success
> types stay the same. Unwrap File(number) before each raw call. Public close
> has no errors and is idempotent. A read requires max_bytes > 0, returns up to
> that many bytes, and returns `<<>>` only at EOF. All bytes parameters must have
> a whole number of bytes; otherwise give invalid before an OS operation. write
> writes all bytes or raises an error; an error/cancel may have written a prefix.
> flush reports data-flush failures. close releases resources even when flush
> would fail; it does not promise durability. read_file opens, reads to EOF and
> closes; write_file truncates, writes and closes, and is not atomic replacement.
> rename errors use the from path; no path normalization or sandboxing is hidden
> in the i wrapper. Expected access denial is code denied. One read/write/flush
> holds the cursor lease; a competing operation gives busy. Missing, closed,
> foreign-context or wrong-kind numbers give closed; close gives Nil. Close wakes
> pending operations with closed unless their answer already committed.
>
> The resource-bearing parts of the File wrappers are explicit ordinary i:
>
> ```i
> mode_string = {
>   ReadOnly -> "read"
>   WriteTruncate -> "truncate"
>   Append -> "append"
>   ReadWrite -> "read_write"
> }
>
> open = (path, mode, raw ! Fs) {
>   File(fail.map_error(
>     () { await_resource(raw.open(path, mode_string(mode)), (n) {
>       request.await(raw.close(n))
>     }) }
>     (code) { FsError("open", path, code) }
>   ))
> }
>
> read = (File(number), max_bytes, raw ! Fs) {
>   request.await(raw.read(number, max_bytes)) |> {
>     Ok(bytes) -> bytes
>     Error(code) -> fail.raise(FsError("read", "", code))
>   }
> }
>
> close = (File(number), raw ! Fs) { request.await(raw.close(number)) }
> ```

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3.

### C006 — docs/design/runtime.md, base lines 941–966

- Old sentence(s):

> | Raw Net field | Type |
> |---|---|
> | `listen` | `(address: String, port: Int) -> Request(Result(Listener,String))` |
> | `local_port` | `(listener: Listener) -> Request(Result(Int,String))` |
> | `accept` | `(listener: Listener) -> Request(Result(Stream,String))` |
> | `connect` | `(address: String, port: Int) -> Request(Result(Stream,String))` |
> | `read` | `(stream: Stream, max_bytes: Int) -> Request(Result(BitArray,String))` |
> | `write` | `(stream: Stream, bytes: BitArray) -> Request(Result(Nil,String))` |
> | `shutdown_write` | `(stream: Stream) -> Request(Result(Nil,String))` |
> | `close_stream` | `(stream: Stream) -> Request(Nil)` |
> | `close_listener` | `(listener: Listener) -> Request(Nil)` |
>
> The same names are public wrappers with the same parameters and success types,
> `? NetError`, and no errors for the two close functions. Address is a numeric
> IPv4/IPv6 address in v1, not a URL or a DNS name. Ports are 0..65535 for listen
> (0 asks the OS to choose) and 1..65535 for connect. local_port returns the bound
> port. Empty address, bad addresses/ports, nonpositive read bounds and partial-byte
> writes give invalid. read and write follow fs's EOF/all-bytes rules. TCP is a
> byte stream: reads need not correspond to writes. shutdown_write sends EOF in
> one direction and leaves reads usable; it is idempotent. It gives busy if a
> write is pending. Later writes on the shut-down direction give closed. Close releases the
> whole stream/listener, is idempotent, and settles other pending operations on
> that object with closed unless their answer already committed. A concurrent
> read and write are allowed. A second concurrent read, write, or accept on the
> same direction/object gives busy, not an unspecified interleaving. There is no
> TLS, UDP, HTTP, DNS, or cross-worker reference transfer in v1.

- New sentence(s):

> | Raw Net field | Type |
> |---|---|
> | `listen` | `(address: String, port: Int) -> Request(Result(Int,String))` |
> | `local_port` | `(listener: Int) -> Request(Result(Int,String))` |
> | `accept` | `(listener: Int) -> Request(Result(Int,String))` |
> | `connect` | `(address: String, port: Int) -> Request(Result(Int,String))` |
> | `read` | `(stream: Int, max_bytes: Int) -> Request(Result(BitArray,String))` |
> | `write` | `(stream: Int, bytes: BitArray) -> Request(Result(Nil,String))` |
> | `shutdown_write` | `(stream: Int) -> Request(Result(Nil,String))` |
> | `close_stream` | `(stream: Int) -> Request(Nil)` |
> | `close_listener` | `(listener: Int) -> Request(Nil)` |
>
> Public listen returns Listener; accept/connect return Stream. Public resource
> parameters use Listener or Stream as appropriate. Other result types stay the
> same, with NetError, except the error-free close functions. Explicit wrappers
> inside i/net include (await_resource is defined in §8.1):
>
> ```i
> accept = (Listener(number), raw ! Net) {
>   Stream(fail.map_error(
>     () { await_resource(raw.accept(number), (n) {
>       request.await(raw.close_stream(n))
>     }) }
>     (code) { NetError("accept", "", code) }
>   ))
> }
>
> read = (Stream(number), max_bytes, raw ! Net) {
>   request.await(raw.read(number, max_bytes)) |> {
>     Ok(bytes) -> bytes
>     Error(code) -> fail.raise(NetError("read", "", code))
>   }
> }
>
> close_stream = (Stream(number), raw ! Net) {
>   request.await(raw.close_stream(number))
> }
> ```
>
> Raw accept returns Int; neither Stream nor NetError crosses the ABI.
> The listen address is numeric IPv4/IPv6. Connect accepts a numeric address
> or a host name, and resolve(name) returns its addresses. Native uses the system
> resolver and tries addresses in order without IPv4/IPv6 racing. Ports are 0..65535 for listen
> (0 asks the OS to choose) and 1..65535 for connect. local_port returns the bound
> port. Empty address, bad addresses/ports, nonpositive read bounds and partial-byte
> writes give invalid. read and write follow fs's EOF/all-bytes rules. TCP is a
> byte stream: reads need not correspond to writes. shutdown_write sends EOF in
> one direction and leaves reads usable; it is idempotent. It gives busy if a
> write is pending. Later writes on the shut-down direction give closed. Close releases the
> whole stream/listener, is idempotent, and settles other pending operations on
> that object with closed unless their answer already committed. A concurrent
> read and write are allowed. A second concurrent read, write, or accept on the
> same direction/object gives busy, not an unspecified interleaving. DNS is part
> of i/net; TLS is i/tls (a later task); UDP is not in v1. Missing, closed, foreign-context or wrong-kind
> numbers give closed; close gives Nil. Stream values can cross workers, while
> their pending Request values cannot.

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3.

### C007 — docs/design/runtime.md, base lines 970–989

- Old sentence(s):

> | Raw Process field | Type |
> |---|---|
> | `args` | `() -> Request(Result(BitArray,String))` |
> | `env` | `(name: String) -> Request(Result(String,String))` |
> | `cwd` | `() -> Request(Result(String,String))` |
> | `entropy` | `(bytes: Int) -> Request(Result(BitArray,String))` |
> | `spawn` | `(program: String, args: BitArray) -> Request(Result(Child,String))` |
> | `wait` | `(child: Child) -> Request(Result(Int,String))` |
> | `terminate` | `(child: Child) -> Request(Result(Nil,String))` |
> | `close` | `(child: Child) -> Request(Nil)` |
>
> Public args gives List(String); spawn takes `(program: String, args: List(String))`;
> other wrappers keep raw parameters/success values, with `? ProcessError` except
> close, which has no errors. Program args exclude the interpreter/program name;
> spawn's args exclude argv[0]. Each child inherits cwd, environment, and standard
> streams. There is no shell expansion, pipeline, or implicit command parsing.
> Strings containing NUL give invalid; malformed UTF-8 OS args/env/cwd give
> not_utf8. env of an absent name gives not_found. entropy accepts 0..2^24 bytes
> and returns exactly that many bytes from the OS source, or an error. It does
> not seed task.run automatically. Use entropy for secrets, not random.Generator.

- New sentence(s):

> | Raw Process field | Type |
> |---|---|
> | `args` | `() -> Request(Result(BitArray,String))` |
> | `env` | `(name: String) -> Request(Result(String,String))` |
> | `cwd` | `() -> Request(Result(String,String))` |
> | `entropy` | `(bytes: Int) -> Request(Result(BitArray,String))` |
> | `spawn` | `(program: String, args: BitArray) -> Request(Result(Int,String))` |
> | `wait` | `(child: Int) -> Request(Result(Int,String))` |
> | `terminate` | `(child: Int) -> Request(Result(Nil,String))` |
> | `close` | `(child: Int) -> Request(Nil)` |
>
> Public args gives List(String); spawn takes `(program: String, args: List(String))`
> and returns Child; wait/terminate/close take Child. Match Child(number)
> to unwrap arguments and wrap the successful spawn Int with Child. Other wrappers
> keep raw parameters/success values, with `? ProcessError` except
> close, which has no errors. Program args exclude the interpreter/program name;
> spawn's args exclude argv[0]. Each child inherits cwd, environment, and standard
> streams. There is no shell expansion, pipeline, or implicit command parsing.
> Strings containing NUL give invalid; malformed UTF-8 OS args/env/cwd give
> not_utf8. env of an absent name gives not_found. entropy accepts 0..2^24 bytes
> and returns exactly that many bytes from the OS source, or an error. It does
> not seed task.run automatically. Use entropy for secrets, not random.Generator.

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics.

### C008 — docs/design/runtime.md, base lines 997–1006

- Old sentence(s):

> wait gives an exit status 0..255 (normal OS exit codes are reduced modulo 256);
> termination by signal s gives 128+(s % 128) on Unix,
> and platforms without Unix signals map forced termination to 255. Repeated
> wait calls give the same cached status. A second simultaneous wait gives busy.
> terminate requests termination, is idempotent for an ended child, and does not
> wait; close terminates if still running and reaps, and is idempotent/error-free.
> An expected OS error from terminate raises ProcessError; close waits for reaping, even if it must wait for natural exit after a
> failed termination request. A raw OS process may be slow to terminate, so close/final
> completion has no bounded duration. Pipes and changed cwd/env are later API
> extensions, not unspecified behavior of this version.

- New sentence(s):

> wait gives an exit status 0..255 (normal OS exit codes are reduced modulo 256);
> termination by signal s gives 128+(s % 128) on Unix,
> and platforms without Unix signals map forced termination to 255. Repeated
> wait calls give the same cached status. A second simultaneous wait gives busy.
> terminate requests termination, is idempotent for an ended child, and does not
> wait; close terminates if still running and reaps, and is idempotent/error-free.
> An expected OS error from terminate raises ProcessError; close waits for reaping, even if it must wait for natural exit after a
> failed termination request. A raw OS process may be slow to terminate, so close/final
> completion has no bounded duration. Pipes and changed cwd/env are later API
> extensions, not unspecified behavior of this version. Missing, closed,
> foreign-context or wrong-kind numbers give closed; close gives Nil. Close wakes
> a pending wait/terminate with closed unless its answer already committed.
>
> Inside i/process, encode_args is the ordinary codec above (it raises
> ProcessError("spawn", "invalid") for bad input before starting work). The
> resource-bearing wrapper parts are:
>
> ```i
> spawn = (program, args, raw ! Process) {
>   bytes = encode_args(args)
>   Child(fail.map_error(
>     () { await_resource(raw.spawn(program, bytes), (n) {
>       request.await(raw.close(n))
>     }) }
>     (code) { ProcessError("spawn", code) }
>   ))
> }
>
> wait = (Child(number), raw ! Process) {
>   request.await(raw.wait(number)) |> {
>     Ok(status) -> status
>     Error(code) -> fail.raise(ProcessError("wait", code))
>   }
> }
>
> close = (Child(number), raw ! Process) { request.await(raw.close(number)) }
> ```

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C009 — docs/design/runtime.md, base lines 1032–1047

- Old sentence(s):

> `i run`/`i test` register the new namespaces themselves in standard HostImports.
> The CLI user writes no Rust, JS, import map, or WASI wrapper. Preserve the
> existing services::standard(print) hook and legacy stdout behavior; add the new
> service modules behind it, with explicit testing imports able to override the
> standard defaults before instantiation. Add production tokio net/io/fs/process
> support as needed, without changing the one-thread-per-run executor contract.
> Service ownership of request futures remains in requests.rs, not tokio::spawn.
>
> Split `services.rs` into registration plus `services/{clock,fs,net,process,stdio}.rs`.
> Clock captures deadlines in start closures. Net uses reactor readiness; never
> hold a synchronous mutex across an await. Resource cells contain a typed OS
> object, closed flag, operation leases and a close wakeup. A close takes and
> releases the resource and wakes pending operations. If close races completion,
> whichever commits first determines the result. Request cancellation drops its
> future and registration immediately. OS side effects already committed may
> remain; no error or cancellation promises rollback.

- New sentence(s):

> `i run`/`i test` register the new namespaces themselves in standard HostImports.
> The CLI user writes no Rust, JS, import map, or WASI wrapper. Preserve the
> existing services::standard(print) hook and legacy stdout behavior; add the new
> service modules behind it, with explicit testing imports able to override the
> standard defaults before instantiation. Add production tokio net/io/fs/process
> support as needed, without changing the one-thread-per-run executor contract.
> Service ownership of request futures remains in requests.rs, not tokio::spawn.
> Before Fs, enforce general nominal field privacy (§7), then implement native.md
> §3.1's general binding grants, including external
> aliases, trusted compiler provenance and JS manifest verification. The standard
> factory grants raw bindings to their resolved declarations. Ordinary i service
> substitutions need no host binding; resource fixtures that override raw host
> functions receive that authority explicitly.
> Use a general context-aware host-registration adapter: create a fresh invocation
> context before each entry, and pass it to each host start. A legacy registration
> may ignore it. Futures capture the immutable context at start, not a mutable
> current-context slot. This permits reuse of HostOptions/service factories across
> tests without retargeting old work into the next test's table. Rust and JS need
> the same per-invocation hook; this is IO plumbing, not an existing ABI feature.
>
> One ResourceContext belongs to each live entry invocation and all its workers.
> Nested task.run, Root and Owner do not create new resource tables: resources may
> outlive request groups. A later export invocation, test or instance gets a fresh
> context. Resource wrappers cannot be host-exported under §10.3; no boundary
> promises to retain them between invocations. The context owns provisional and
> delivered resources, a live/ending/ended state and an independent teardown
> controller. A positive arbitrary-precision Int indexes each entry, which also
> records its resource kind. One host-wide allocator never reissues a number,
> including provisional/failed creations, across contexts, tests or instances.
> Unknown, zero/negative, foreign-context and wrong-kind numbers give closed;
> close gives Nil. Checked allocation failure stops for memory exhaustion, never
> wraps or truncates to u64/fd. A host restart begins another identity domain;
> retaining raw numbers across host restarts is unsupported.
>
> Split `services.rs` into registration plus `services/{clock,fs,net,process,stdio}.rs`.
> Clock captures deadlines in start closures. Net uses reactor readiness; never
> hold a synchronous mutex across an await. Table entries contain typed OS
> objects, operation leases and close wakeups.
> Lookup, lease acquisition, completion commit and removal serialize under a short
> host lock; no OS wait or i/Wasm callback runs under it. A completion committed
> first keeps its answer. Close committed first removes the entry, invalidates
> all aliases and settles pending operations with closed. An operation retains a
> lease/private OS reference, never a bare pointer into a removed entry, and tests
> its lease before publishing success. Release happens once after references are
> safe. Close returns Nil when cleanup completes (Child also awaits reaping);
> it never waits for a lock held by a pending read/wait. Cancelling close cannot
> restore the entry or cancel its host-owned cleanup. A close already in progress
> has a context/kind-checked cleanup receipt;
> concurrent closes join that receipt (in particular Child reaping), then give Nil.
> Teardown includes these jobs even after their live table entries were removed;
> completed receipts are freed, not retained as permanent tombstones.
> Request cancellation drops its future and registration immediately. OS side effects already committed may
> remain; no error or cancellation promises rollback.

- Reason: Honor spec §2.4 for exported environments and nominal privacy, using one general field-visibility fix before Fs. Close N9’s forged-Int paths with general declaration grants and no public raw-number injection; ordinary accessible service fakes stay legal. Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Adopt N1–N4 and the reviewed artifact/loading constraints, keeping explicit ilang/WASI and rejecting only required unsupplied imports. Make the implementation/spec handoff explicit: prerequisites now, packaging and MC mechanisms in their own stages. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C010 — docs/design/runtime.md, base lines 1075–1101

- Old sentence(s):

> Child ownership is independent of its wait future. A cancelled wait removes its
> wait lease; Child.close requests termination and reaping independently. Cleanup
> must not need a mutex locked by that cancelled future. Spawn failures are typed
> errors. A provisional child needs a host-owned supervisor that can receive a
> terminate request and reap without its calling Store or run reactor staying
> alive. Dropping an undelivered reply sends that termination request. A native
> worker that supervises an abandoned child may finish independently, like private
> blocking work permitted by C D6; service futures themselves are not tokio-spawned
> tasks. Explicit close awaits the supervisor's reaping completion, including when
> termination cannot be delivered and natural exit must be awaited. Merely dropping
> a default tokio Child does not implement this contract. Abandoning host work
> never promises that the OS has already completed reaping when the instance
> returns status 0. Verify this supervisor in the process phase before claiming
> that cancelled spawn leaves no orphan. Host shutdown does not wait for detached
> blocking work, as C requires.
>
> For open/listen/accept/connect/spawn the host reply owns a new resource until
> it is decoded into a request cell. Dropping host work or an undelivered reply
> releases that provisional resource. After decoding, the i wrapper owns the
> unclaimed Ready result. Its error-free final inspects request.state: for
> Ready(Ok(resource)), close it using the appropriate service, then cancel the
> ticket; for Waiting, cancel; for Taken/Cancelled/Error, just cancel. request.await
> calls take with no scheduler step between its wait returning and the transfer. A cancelled
> scheduler await jumps to the final before taking, so that final closes an already
> decoded resource. On success take removes the cell's answer and marks Taken;
> the final does not close the resource just handed to the caller. The caller
> then uses resource.acquire/ensure to own its lifetime.

- New sentence(s):

> Before starting an OS spawn, register its host-owned supervisor/creation job
> with the ResourceContext. Ending the context cancels that job even if the child
> has not been created yet. A late-created child is terminated/reaped by that
> supervisor, not dropped into a dead reactor. Native's controller waits for every
> registered spawn job to acknowledge no remaining possible child before exit;
> waiting only for children already present in the table misses this race.
> Child ownership is independent of its wait future. A cancelled wait removes its
> wait lease; Child.close requests termination and reaping independently. Cleanup
> must not need a mutex locked by that cancelled future. Spawn failures are typed
> errors. A provisional child needs a host-owned supervisor that can receive a
> terminate request and reap without its calling Store or run reactor staying
> alive. Dropping an undelivered reply sends that termination request. A native
> worker that supervises an abandoned child may finish independently, like private
> blocking work permitted by C D6; service futures themselves are not tokio-spawned
> tasks. Explicit close awaits the supervisor's reaping completion, including when
> termination cannot be delivered and natural exit must be awaited. Merely dropping
> a default tokio Child does not implement this contract. Abandoning host work
> never promises that the OS has already completed reaping when the instance
> returns status 0. Verify this supervisor in the process phase before claiming
> that cancelled spawn leaves no orphan. Host shutdown does not wait for unrelated detached blocking work, as C requires;
> the native executable specifically waits for registered spawn/reaping jobs before
> process exit, so it cannot kill its cleanup owner.
>
> For open/listen/accept/connect/spawn, insert a new number as provisional before
> publishing the reply. The host reply holds a drop guard: abandonment removes and
> closes that entry (Child requests supervised termination/reaping). Successful
> validation/publication of the decoded Int into Ready disarms the guard. The
> context still owns the resource;
> the i wrapper now owns the unclaimed Ready result. Its error-free final closes
> Ready(Ok(number)) and cancels the ticket; Waiting is cancelled; other states
> only cancel. There is no scheduler step from await returning through take and
> nominal wrapping. Cancellation before take therefore closes the Ready resource;
> successful take transfers explicit close responsibility to the caller, without
> removing the context's teardown ownership. A forgotten decoded Ready answer
> stays in the table until explicit close or teardown; no i GC timing is involved.

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C011 — docs/design/runtime.md, base lines 1124–1137

- Old sentence(s):

> This guard is ordinary i, parameterized by the module's close function and its
> kept service environment. It may suspend while closing. It adds no ABI ack or
> resource-specific compiler rule. Raw service users must install the same guard
> if they start resource-creating requests manually: request.cancel alone drops
> host work, not arbitrary i-owned resources. On a stop, i finals do not run, but
> destroying the instance drops its host resources. On GC of a forgotten File or
> Stream, host ownership may also release it; GC timing is not the specified way
> to close a resource. Expected close failures are swallowed only by the explicit
> error-free close contract; flush/terminate expose errors separately.
>
> Resource objects are host references, which remain local as S7 requires. On Rust
> this can use the existing Arc-backed HostReference (`crates/ilang-host/src/lib.rs:372–382`).
> The request table's generation token solves stale i aliases, not a malicious
> host's late callback after id reuse: ABI H13 already forbids that callback.

- New sentence(s):

> This guard is ordinary i, parameterized by the raw close function and its
> service environment, then followed by error translation and nominal wrapping.
> It may suspend during close in a shielded final. No ABI ack or resource-specific
> compiler rule is added. Raw callers inside the module must use the same guard:
> request.cancel cannot infer that an arbitrary Int answer denotes a resource.
> Expected close errors are swallowed only by close's error-free contract;
> flush/terminate expose errors separately.
>
> **Teardown is a host action, not an i final or GC finalizer.** At normal entry
> end, stop, trap, host failure, abandoned async call or explicit dispose:
> (1) mark the context ending and refuse new starts/deliveries; (2) detach/cancel
> request work and wake idle workers; (3) remove every table entry, including
> provisional and forgotten Ready/Taken answers, revoke leases and schedule each
> OS cleanup once; (4) mark ended. These steps require no Store borrow or i code. User imports, including
> synchronous services/externals, reject new host actions once the stop gate closes;
> controller cleanup remains allowed. An operation admitted before stop may finish
> its already-started OS effect, without delivering a new i callback.
> An operation either commits before ending or discards its result. A late creator
> closes its new OS object instead of inserting into an ending table. Finite
> critical sections never wait for a worker that may spin in Wasm.
> In a live host, native N12 interrupts and joins stopped workers before returning
> the run result. Child supervisors and safe detached blocking work may outlive
> that result, owning only host data, never a Store or context. A native process
> must finish child termination/reaping before its final process exit: exit would
> kill an in-process supervisor. This may await natural child exit if termination
> fails; no bounded OS-cleanup time is promised. Unrelated detached file/stdin work
> is not joined, as C requires. Stdin unsubscribes per context; the source broker
> survives sequential tests. Integer resources have no GC finalizers. Request-slot
> generations and ABI H13's late-callback prohibition still apply independently.

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C012 — docs/design/runtime.md, base lines 1141–1173

- Old sentence(s):

> | Module | `i run` Rust host | JS SDK standard service factory: Node | JS factory: browser | WASI preview-1 target in this track |
> |---|---|---|---|---|
> | random | Pure i | Same pure i | Same pure i | Same pure i |
> | task | i FIFO loop over v3 | Same loop; promises, AbortSignal, macrotask polling | Same loop on calling thread; never block it | Same i loop; raw host calls complete synchronously; no status hand-off |
> | clock | Reactor timers and OS clocks | Monotonic/system clocks and abortable timers | Performance/system clocks and abortable timers | Ordinary blocking service imports; preview-1 command supplies no i:clock adapter |
> | fs | OS files, with the cancellation rule above | Node file operations, closed/busy state, explicit close | Every raw operation gives unsupported, except close gives Nil | Ordinary blocking imports; opaque File needs an export-aware embedder, not plain preload |
> | net | Reactor TCP | Node TCP, cancellation destroys provisional sockets and removes listeners | All operations give unsupported, close gives Nil; browser fetch is not raw TCP | Ordinary blocking imports; no default listen/connect adapter |
> | process | OS args/env/cwd/entropy/child processes | Node process data, entropy, spawn/wait/terminate/close | args gives an encoded empty list; env gives denied; cwd/spawn/wait/terminate give unsupported; entropy uses platform secure entropy; close gives Nil | Ordinary blocking imports; no default process/entropy adapter or spawn implementation |
> | stdio | Per-source input broker/subscribers and byte output | Node standard byte streams with cancelable subscribers | Default byte read/write/flush operations give unsupported; an embedder can give explicit byte sources/sinks | Ordinary blocking imports; no new i:stdio adapter; legacy i/io continues to use fd_write |
>
> The JS factory is an explicit SDK option, not a language name-resolution rule.
> It installs all fields including unsupported stubs, so a module can instantiate
> and catch a typed unsupported error. Embedders can override any whole service
> with ordinary imports. Browser APIs never silently treat URLs as files or a
> WebSocket as Stream. Node and browser async starters read requestSignal()
> synchronously and register cancellation there; the helper and status-2 macrotask
> path already exist (`bindings/javascript/ilang.mjs:34–58`, `:894–902`). New service
> factories and tests are IO work, not claims about today's SDK.
>
> WASI keeps the current §B.7 blocking-import contract. It is not the `i run` host:
> no standalone promise for the new IO imports is made here. Each request-mode
> blocking import returns an already-Ready request, so neither ABI cancel nor
> resume/continue is used, and poll is a no-op. Ordinary channel tasks still run
> cooperatively, but a blocking TCP read/accept supplied by a custom WASI embedder
> blocks the worker and cannot meet the concurrent-server acceptance criterion.
> `i build --target wasi` must clearly report required non-WASI service imports;
> it must not claim that plain wasmtime run supplies them. Preserve general custom
> blocking imports, as §B.7 already permits. Implementing a preview-1 adapter set
> is a separate target project, not an unreviewed second IO host in this track.
> The current hostref representation requires an externref and reference_id
> (`crates/ilang-wasm/src/runtime/mod.rs:240`, `:249–268`), so “just return an fd”
> would not implement File under the existing ABI. That unsupported shortcut is
> specifically rejected.

- New sentence(s):

> | Module | `i run` and native Rust host | JS SDK standard service factory: Node | JS factory: browser | WASI preview-1 target in this track |
> |---|---|---|---|---|
> | random | Pure i | Same pure i | Same pure i | Same pure i |
> | task | i FIFO loop over v3 | Same loop; promises, AbortSignal, macrotask polling | Same loop on calling thread; never block it | Same i loop; raw host calls complete synchronously; no status hand-off |
> | clock | Reactor timers and OS clocks | Monotonic/system clocks and abortable timers | Performance/system clocks and abortable timers | Ordinary blocking service imports; preview-1 command supplies no i:clock adapter |
> | fs | OS files, with the cancellation rule above | Node context-owned Int table, closed/busy leases, explicit close | Every raw operation gives unsupported, except close gives Nil | Blocking Int imports; buffer-aware embedder/linked adapter needed, no default fs adapter |
> | net | Reactor TCP | Node context-owned Int table; provisional cleanup and close wakeups | All operations give unsupported, close gives Nil; browser fetch is not raw TCP | Ordinary blocking imports; no default listen/connect adapter |
> | process | OS args/env/cwd/entropy/child processes | Node process data, entropy, spawn/wait/terminate/close | args gives an encoded empty list; env gives denied; cwd/spawn/wait/terminate give unsupported; entropy uses platform secure entropy; close gives Nil | Ordinary blocking imports; no default process/entropy adapter or spawn implementation |
> | stdio | Per-source input broker/subscribers and byte output | Node standard byte streams with cancelable subscribers | Default byte read/write/flush operations give unsupported; an embedder can give explicit byte sources/sinks | Ordinary blocking imports; no new i:stdio adapter; legacy i/io continues to use fd_write |
>
> The JS factory is an explicit SDK option, not a language name-resolution rule.
> It installs all fields including unsupported stubs, so a module can instantiate
> and catch a typed unsupported error. Embedders can override any whole service with ordinary imports and explicit
> binding grants. The standard factories share one ResourceContext across workers
> and make a fresh one per invocation, even when the factory is reused for tests.
> JS resource numbers/counters use BigInt, never Number. Stop/dispose removes
> listeners, destroys sockets and owns child cleanup, including promises settling
> after cancellation. Browser APIs never silently treat URLs as files or a
> WebSocket as Stream. Node and browser async starters read requestSignal()
> synchronously and register cancellation there; the helper and status-2 macrotask
> path already exist (`bindings/javascript/ilang.mjs:34–58`, `:894–902`). New service
> factories and tests are IO work, not claims about today's SDK.
>
> WASI keeps the current §B.7 blocking-import contract. It is not the `i run` host:
> no standalone promise for the new IO imports is made here. Each request-mode
> blocking import returns an already-Ready request, so neither ABI cancel nor
> resume/continue is used, and poll is a no-op. Ordinary channel tasks still run
> cooperatively, but a blocking TCP read/accept supplied by a custom WASI embedder
> blocks the worker and cannot meet the concurrent-server acceptance criterion.
> `i build --target wasi` must clearly report required non-WASI service imports;
> it must not claim that plain wasmtime run supplies them. Preserve general custom
> blocking imports, as §B.7 already permits. Implementing a preview-1 adapter set
> is a separate target project, not an unreviewed second IO host in this track.
> Resource wrappers are ordinary Int-bearing values, not opaque ABI references.
> A WASI adapter passes Int and keeps the same context-owned kind-checked table,
> leases and teardown. An OS fd is private implementation data, never the language
> number: fds are reused. Int buffers still require an export-aware embedder or
> static linking (§B.7); switching from externref does not make plain --preload
> sufficient. No resource wrapper becomes a host-export type.

- Reason: Close N9’s forged-Int paths with general declaration grants and no public raw-number injection; ordinary accessible service fakes stay legal. Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Adopt N1–N4 and the reviewed artifact/loading constraints, keeping explicit ilang/WASI and rejecting only required unsupplied imports. Make the implementation/spec handoff explicit: prerequisites now, packaging and MC mechanisms in their own stages. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C013 — docs/design/runtime.md, base lines 1177–1240

- Old sentence(s):

> This complete example needs only i modules. It deliberately uses local_scope:
> accepted sockets are host references. Several connections overlap on one worker;
> this example does not promise multicore speedup for socket-owning handlers.
>
> ```i
> (i/task, i/net, i/fail, i/resource)
>
> copy = (stream) {
>   net.read(stream, 65536) |> {
>     <<>> -> Nil
>     bytes -> {
>       net.write(stream, bytes)
>       copy(stream)
>     }
>   }
> }
>
> serve_next = (scope, listener) {
>   resource.acquire(
>     () { net.accept(listener) }
>     (stream) { net.close_stream(stream) }
>     (stream) {
>       task.go(scope, () { serve_next(scope, listener) })
>       fail.catch(() { copy(stream) })
>       Nil
>     }
>   )
> }
>
> @main
> main = () {
>   resource.acquire(
>     () { net.listen("127.0.0.1", 8080) }
>     (listener) { net.close_listener(listener) }
>     (listener) {
>       task.local_scope((scope) {
>         task.go(scope, () { serve_next(scope, listener) })
>         Nil
>       })
>     }
>   )
> }
> ```
>
> Each accepting task acquires its own socket and installs its final before
> starting the next accepting task. Thus there is one pending accept while older
> tasks serve their own sockets; an unstarted child never owns an accepted socket.
> The handler's catch isolates expected read/write errors to that connection;
> listener errors still fail main and cancel all handlers. A production bounded
> server can use a local channel of permits, but a limit is not needed to show
> concurrency. The example's co-recursive types must infer from ordinary module
> functions; there are no constructors imported implicitly from net/task.
>
> Acceptance fixture: use port 0 and local_port in the harness variant. Connect
> clients A and B. Send one byte from A but leave A open without EOF; send a
> multi-chunk payload containing zero bytes from B. Require B to receive its exact
> payload while A remains open. Then require A's byte echoed. Half-close B and
> check its handler ends and closes its stream; connect C while A is still open.
> Finally a finite harness injects a failure from a sentinel child in the
> accepting scope, catches that error outside the scope, and checks that listener,
> A/C handlers, and all started host requests end. This distinguishes overlap
> from an implementation that accepts one client and reads it to EOF first.
> No sleep-based assertion decides readiness. Run the fixture 200 times at O0/O2
> and once through the JS Node factory; Rust is the standalone acceptance host.

- New sentence(s):

> This complete example needs only i modules. It uses a portable scope: Listener
> and Stream are nominal Int values. Connections overlap on one worker; in MC
> the next acceptor may be claimed by another worker. A started handler and each
> of its Request values still stay on their original worker.
>
> ```i
> (i/task, i/net, i/fail, i/resource)
>
> copy = (stream) {
>   net.read(stream, 65536) |> {
>     <<>> -> Nil
>     bytes -> {
>       net.write(stream, bytes)
>       copy(stream)
>     }
>   }
> }
>
> serve_next = (scope, listener) {
>   resource.acquire(
>     () { net.accept(listener) }
>     (stream) { net.close_stream(stream) }
>     (stream) {
>       task.go(scope, () { serve_next(scope, listener) })
>       fail.catch(() { copy(stream) })
>       Nil
>     }
>   )
> }
>
> @main
> main = () {
>   resource.acquire(
>     () { net.listen("127.0.0.1", 8080) }
>     (listener) { net.close_listener(listener) }
>     (listener) {
>       task.scope((scope) {
>         task.go(scope, () { serve_next(scope, listener) })
>         Nil
>       })
>     }
>   )
> }
> ```
>
> Each accepting task acquires its own socket and installs its final before
> starting the next accepting task. Thus there is one pending accept while older
> tasks serve their own sockets; an unstarted child never owns an accepted socket.
> The handler's catch isolates expected read/write errors to that connection;
> listener errors still fail main and cancel all handlers. A production bounded
> server can use a portable channel of permits, but a limit is not needed to show
> concurrency. The example's co-recursive types must infer from ordinary module
> functions; there are no constructors imported implicitly from net/task.
>
> Acceptance fixture: use port 0 and local_port in the harness variant. Connect
> clients A and B. Send one byte from A but leave A open without EOF; send a
> multi-chunk payload containing zero bytes from B. Require B to receive its exact
> payload while A remains open. Then require A's byte echoed. Half-close B and
> check its handler ends and closes its stream; connect C while A is still open.
> Finally a finite harness injects a failure from a sentinel child in the
> accepting scope, catches that error outside the scope, and checks that listener,
> A/C handlers, and all started host requests end. This distinguishes overlap
> from an implementation that accepts one client and reads it to EOF first.
> No sleep-based assertion decides readiness. Run the fixture 200 times at O0/O2
> and once through the JS Node factory; Rust is the standalone acceptance host.
> In MC repeat at 1/2/4 workers and force remote claims of the next acceptor.
> Listener/Stream remain equal after transport; their wrappers do not pin tasks.

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Make the implementation/spec handoff explicit: prerequisites now, packaging and MC mechanisms in their own stages.

### C014 — docs/design/runtime.md, base lines 1313–1313

- Old sentence(s):

> **§9.7 full replacement.**

- New sentence(s):

> **§2.4/§6.7 privacy clarification.** Retain §2.4's export requirement and add
> to §6.7: “Taking a nominal value apart by field selection requires the same
> constructor visibility as a constructor pattern (§2.4). Exporting T without
> constructors exports neither field access nor update. Structural record fields
> follow their own rules.” This enforces the existing abstraction boundary.
>
> **§9.7 full replacement.** Copy native.md §10's general binding-permission
> paragraph after this replacement; it applies to every namespace and §10.1 alias.
> It is host policy, not a reserved library namespace or a new type rule.

- Reason: Honor spec §2.4 for exported environments and nominal privacy, using one general field-visibility fix before Fs. Close N9’s forged-Int paths with general declaration grants and no public raw-number injection; ordinary accessible service fakes stay legal.

### C015 — docs/design/runtime.md, base lines 1356–1362

- Old sentence(s):

> **New §9.7.1–§9.7.5.** Copy the service/type/error definitions, stable error-code
> paragraphs, raw field tables, public wrapper type transformations, wire codecs,
> and behavior in §7.1–§7.5 of this note, in that order. Prepend §7's shared
> introductory API/error paragraphs and service/type table to §9.7.1. Include §8.1's
> resource-transfer/cancellation paragraphs from “For open/listen/accept/connect/spawn”
> through “specified way to close a resource”. They define the ownership visible
> to programs; Rust implementation choices elsewhere in §8.1 do not enter the spec.

- New sentence(s):

> **New §9.7.1–§9.7.5.** Copy the service/type/error definitions, stable error-code
> paragraphs, raw field tables, public wrapper type transformations, wire codecs,
> and behavior in §7.1–§7.5 of this note, in that order. Prepend §7's shared
> introductory API/error paragraphs and service/type table to §9.7.1. Include the resource value, privacy, binding, lease, transfer and teardown
> behavior from §7 and §8.1 through its end. Omit wrapper implementation examples
> and Rust API choices; these behavioral rules define ownership visible to programs.

- Reason: Close N9’s forged-Int paths with general declaration grants and no public raw-number injection; ordinary accessible service fakes stay legal. Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Make the implementation/spec handoff explicit: prerequisites now, packaging and MC mechanisms in their own stages.

### C016 — docs/design/runtime.md, base lines 1370–1387

- Old sentence(s):

> > i/task is a library implemented using i/control, i/key and i/request. A run
> > owns its tasks and scheduler state. Its implicit root permits local values.
> > Scopes constrain the lifetime of spawned work: none outlives its scope.
> > Values pass between tasks only through go captures/kept services, channels,
> > child results/errors and shares. Portable paths check core.portable even
> > with one worker: sends at the call, send events at construction, children
> > at completion, scope body errors at failure, shares at creation. Failure
> > stops with panic("task: value is not portable"). go itself does not check;
> > unstarted tasks with non-portable captures stay on their home worker.
> > Local scopes/channels hold the maker's boundary only as a non-portability
> > witness. A Task/join event from a local scope holds that scope too. Every
> > handle checks run identity. Channels belong to the first scope in their
> > maker's chain, or the run; channels and shares are freed at scope exit.
> > Every run has the result of some serial order of task steps. Timing may
> > choose a sender, a race/select winner, which failures precede cancellation,
> > or whether a handle is used before its owner ends. These order-dependent
> > results may change between runs or worker counts. Tests must either avoid
> > that dependence or list the allowed outcomes. No task API observes workers.

- New sentence(s):

> > i/task is a library implemented using i/control, i/key and i/request. A run
> > owns its tasks and scheduler state. Its implicit root permits local values.
> > Scopes constrain the lifetime of spawned work: none outlives its scope.
> > Values pass between tasks only through go captures/kept services, channels,
> > child results/errors and shares. Portable paths check core.portable even
> > with one worker: sends at the call, send events at construction, children
> > at completion, scope body errors at failure, shares at creation. Failure
> > stops with panic("task: value is not portable"). go never rejects for
> > non-portability; on multiple workers it silently classifies the complete task
> > record at go and publishes portable records. Other records stay local.
> > Local scopes/channels hold the maker's boundary only as a non-portability
> > witness. A Task/join event from a local scope holds that scope too. Every
> > handle checks run identity. Channels belong to the first scope in their
> > maker's chain, or the run; channels and shares are freed at scope exit.
> > Every run has the result of some serial order of task steps. Timing may
> > choose a sender, a race/select winner, which failures precede cancellation,
> > or whether a handle is used before its owner ends. These order-dependent
> > results may change between runs or worker counts. Tests must either avoid
> > that dependence or list the allowed outcomes. No task API observes workers.

- Reason: Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C017 — docs/design/runtime.md, base lines 1440–1442

- Old sentence(s):

> > The outer Request/Result types in a request-mode host field describe how
> > host work is obtained (§9.7); they do not add ABI parameter/result types.
> > Requests, tickets, tasks, scopes, channels and events cannot cross the ABI.

- New sentence(s):

> > The outer Request/Result types in a request-mode host field describe how
> > host work is obtained (§9.7); they do not add ABI parameter/result types.
> > Requests, tickets, tasks, scopes, channels and events cannot cross the ABI.
> > Custom IO wrappers with Int fields are not ABI types; raw resource services
> > pass only Int. No library-specific exception is added.

- Reason: Keep this sentence consistent with the adopted native decisions and its referenced accepted runtime/multicore rules.

### C018 — docs/design/runtime.md, base lines 1470–1475

- Old sentence(s):

> **§B.4–B.6.** Keep ABI v3 and all H rules. In §B.5 replace only “a module made by
> this compiler has no outstanding request at that point” with “structured entry
> cleanup normally leaves no outstanding request; a deliberately unowned raw
> request is dropped by the host at status 0”. Request-mode and async declarations
> produce the same v3 async signature; metadata lists the unwrapped signature and
> derives each entry's async flag from its inferred environment as §10.2 says. A completed cell is not an outstanding id.

- New sentence(s):

> **§B.4–B.6.** Keep ABI v3 and all H rules. Trusted binding plans and
> ResourceContext teardown are loader/host contracts (native §3.1), not new
> runtime imports or metadata fields that a v3 host silently trusts. In §B.5 replace only “a module made by
> this compiler has no outstanding request at that point” with “structured entry
> cleanup normally leaves no outstanding request; a deliberately unowned raw
> request is dropped by the host at status 0”. Request-mode and async declarations
> produce the same v3 async signature; metadata lists the unwrapped signature and
> derives each entry's async flag from its inferred environment as §10.2 says. A completed cell is not an outstanding id.

- Reason: Close N9’s forged-Int paths with general declaration grants and no public raw-number injection; ordinary accessible service fakes stay legal. Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C019 — docs/design/runtime.md, base lines 1491–1492

- Old sentence(s):

> Only runtime.md is edited in this lane. When integrating these coordinator-
> approved rules, apply the following exact replacements in multicore.md too.

- New sentence(s):

> Native adoption applies these coordinator-approved join corrections in
> multicore.md now. The quotations below record their source, not pending edits.
> Native N7/N8 also replace D8 and the parallel CML plan.

- Reason: Adopt N8’s one-lock backend while retaining the i scheduler, typed local storage and the accepted CML transaction semantics.

### C020 — docs/design/runtime.md, base lines 1538–1544

- Old sentence(s):

> Rule 17's unawaited-request/deadlock guarantee is unchanged. “Of the run” counts
> all Owner groups registered to its Root, not one default cancellation group or
> the whole instance. A nested run establishes another Root. The §5 counter-based
> PRF implementation sketch is replaced by the published SplitMix split-tree
> choice in §4 of this note, following the coordinator's preference; the tagged
> P/S stream promise of §18 rule 11 remains. R1 must reconcile that sketch when
> landing its algorithm text rather than leave two algorithm prescriptions.

- New sentence(s):

> Rule 17's unawaited-request/deadlock guarantee is unchanged. “Of the run” counts
> all Owner groups registered to its Root, not one default cancellation group or
> the whole instance. A nested run establishes another Root. The §5 counter-based
> PRF implementation sketch is replaced by the published SplitMix split-tree
> choice in §4 of this note, following the coordinator's preference; the tagged
> P/S stream promise of §18 rule 11 remains. This adoption reconciles the sketch now; R1 copies the one published algorithm
> when landing its spec text, rather than leaving two prescriptions.

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Make the implementation/spec handoff explicit: prerequisites now, packaging and MC mechanisms in their own stages.

### C021 — docs/design/runtime.md, base lines 1548–1560

- Old sentence(s):

> Only this note is changed in the design lane. The following is an implementation
> assignment, not permission for a worker to edit outside its lane. Line references
> anchor current regions; implementations re-find them after earlier tracks land.
>
> | Track | Files and regions to change |
> |---|---|
> | R1 | Replace `lib/i/random.i:1`; register no Rust primitive. Add R1 spec text and `tests/spec/9.10/` |
> | R2 prerequisites from S7 | New `lib/i/key.i`; `lib/i/control.i:3–28` keep_env; `lib/i/core.i:3–33` portable; primitive type/variance/name tables in `crates/ilang-check/src/types/solve.rs:1285–1291` and `hir.rs:210–257`; closure flags/walk stamps and portable walk per S7 §3.6; keep_env re-entry per S7 §4 in emit/control.rs after B |
> | R2 request layer | New `lib/i/request.i`; primitive Request/Ticket and operations in checker/runtime/IR; host declaration validation and expansion in `crates/ilang-check/src/resolve.rs:664–679`, `:1470–1555`, mode in `hir.rs:182–186`, and post-expansion interface display after syntax/T work; `crates/ilang-wasm/src/lower.rs:969–1054`, `:1140–1202`; replace the entire request/table/result region in `emit/host.rs:23–165`, `:172–246`, `:335–358`; add continue export. No edits to `ilang-host/src/requests.rs` are needed for many outstanding requests |
> | R2 scheduler and entry | Replace `lib/i/task.i:1`; `infer.rs:376–429` resolved allowed-entry-service set and diagnostics; `lower.rs:828–938` entry wrappers/metadata; `emit/mod.rs:805–870` service setup and wrapper invocation. The wrapper calls ordinary i implementation definitions; it does not implement the queue in Rust |
> | IO service phases | Replace the five reserved `lib/i/{clock,fs,net,process,stdio}.i`; new `crates/ilang-host/src/services/{clock,fs,net,process,stdio}.rs`; registration in `services.rs:44–70`; tokio features in `crates/ilang-host/Cargo.toml:15`; CLI/default host imports at `crates/ilang-cli/src/lib.rs:151–152`; new JS Node/browser service factories and their tests under `bindings/javascript/` and `tests/browser/` |
> | Shared registration | `crates/ilang-package/src/library.rs:7–26` adds request/key once; existing seven reserved entries become real without duplicate paths. Add Request/Ticket to compiler built-in type names, like Boundary/Continuation; their ordinary helper types are not prelude names. S7 Key/Keyed retains its assignment |
> | Spec/test ownership | R1 §9.10; R2 §9.2, §9.4–9.5, §9.8–9.9, §9.11, §10.2 and request additions in §4/§10.3/§11/§B.5; IO §9.7, §B.7, §0/§2.1/§12 lists. Use the exact section splits in §10; one owner reconciles each shared paragraph |

- New sentence(s):

> This adoption changes only native.md, runtime.md and multicore.md. The following
> is an implementation assignment, not permission for a worker to edit outside its lane. Line references
> anchor current regions; implementations re-find them after earlier tracks land.
>
> | Track | Files and regions to change |
> |---|---|
> | R1 | Replace `lib/i/random.i:1`; register no Rust primitive. Add R1 spec text and `tests/spec/9.10/` |
> | R2 prerequisites from S7 | New `lib/i/key.i`; `lib/i/control.i:3–28` keep_env; `lib/i/core.i:3–33` portable; primitive type/variance/name tables in `crates/ilang-check/src/types/solve.rs:1285–1291` and `hir.rs:210–257`; closure flags/walk stamps and portable walk per S7 §3.6; keep_env re-entry per S7 §4 in emit/control.rs after B |
> | R2 request layer | New `lib/i/request.i`; primitive Request/Ticket and operations in checker/runtime/IR; host declaration validation and expansion in `crates/ilang-check/src/resolve.rs:664–679`, `:1470–1555`, mode in `hir.rs:182–186`, and post-expansion interface display after syntax/T work; `crates/ilang-wasm/src/lower.rs:969–1054`, `:1140–1202`; replace the entire request/table/result region in `emit/host.rs:23–165`, `:172–246`, `:335–358`; add continue export. The existing host table already supports many outstanding requests; IO adds teardown-owner access independent of a Store borrow |
> | R2 scheduler and entry | Replace `lib/i/task.i:1`; `infer.rs:376–429` resolved allowed-entry-service set and diagnostics; `lower.rs:828–938` entry wrappers/metadata; `emit/mod.rs:805–870` service setup and wrapper invocation. The wrapper calls ordinary i implementation definitions; it does not implement the queue in Rust |
> | IO service phases | Before Fs, generic nominal field visibility in current `crates/ilang-check/src/types/infer.rs:1538–1634` and its tests (spec §2.4); replace the five reserved `lib/i/{clock,fs,net,process,stdio}.i`; new `crates/ilang-host/src/services/{clock,fs,net,process,stdio}.rs`; registration in `services.rs:44–70`; tokio features in `crates/ilang-host/Cargo.toml:15`; CLI/default host imports at `crates/ilang-cli/src/lib.rs:151–152`; new JS Node/browser factories/tests under `bindings/javascript/` and `tests/browser/`; context-aware host registrations, shared ResourceContext/Int allocator and trusted binding grants before Fs (native §3.1/§10) |
> | Shared registration | `crates/ilang-package/src/library.rs:7–26` adds request/key once; existing seven reserved entries become real without duplicate paths. Add Request/Ticket to compiler built-in type names, like Boundary/Continuation; their ordinary helper types are not prelude names. S7 Key/Keyed retains its assignment |
> | Spec/test ownership | R1 §9.10; R2 §9.2, §9.4–9.5, §9.8–9.9, §9.11, §10.2 and request additions in §4/§10.3/§11/§B.5; IO §9.7, §B.7, §0/§2.1/§12 lists. Use the exact section splits in §10; one owner reconciles each shared paragraph |

- Reason: Honor spec §2.4 for exported environments and nominal privacy, using one general field-visibility fix before Fs. Close N9’s forged-Int paths with general declaration grants and no public raw-number injection; ordinary accessible service fakes stay legal. Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Adopt N1–N4 and the reviewed artifact/loading constraints, keeping explicit ilang/WASI and rejecting only required unsupplied imports. Make the implementation/spec handoff explicit: prerequisites now, packaging and MC mechanisms in their own stages. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C022 — docs/design/runtime.md, base lines 1639–1651

- Old sentence(s):

> 1. [ ] Clock: deadlines fixed at start, both clocks, zero/negative/huge durations,
>    abortable timers and no timer keeping an ended Node run alive.
> 2. [ ] Fs: codecs/errors, whole-byte validation, all-byte writes, explicit flush,
>    close, cancelled blocking-file state, safe detached operations.
> 3. [ ] Net: numeric endpoints, ephemeral bind, read/write/half-close, busy leases,
>    cancellation/provisional socket ownership and concurrent connection fixture.
> 4. [ ] Process: bounded entropy, UTF-8/framing, no shell expansion, inherited
>    streams, wait caching, terminate and cleanup/reaping on every ownership edge.
> 5. [ ] Stdio: cancellable subscriber input, exact byte output, flush/error stream,
>    legacy i/io compatibility; register all services and finish spec lists/B.7.
> 6. [ ] Run permission/error/resource/security and concurrency reviews; include
>    unsupported browser/preview-1 behavior and user-supplied host fakes.
> 7. [ ] Run end-to-end echo, then combined verification in the integration lane.

- New sentence(s):

> 1. [ ] Clock: before Fs, add the shared ResourceContext, host-wide non-reusing Int
>    allocator and trusted binding grants (native §3.1) on Rust/JS; enforce §2.4 field privacy; test sequential
>    invocations, late creators, disposal and forced exhaustion. Clock deadlines fixed at start, both clocks, zero/negative/huge durations,
>    abortable timers and no timer keeping an ended Node run alive.
> 2. [ ] Fs: codecs/errors, whole-byte validation, all-byte writes, explicit flush,
>    private File(Int) codecs, raw Int wrappers, no number-injection exports, closed/busy leases,
>    cancelled blocking-file state, safe detached work and context teardown.
> 3. [ ] Net: numeric endpoints, ephemeral bind, read/write/half-close, busy leases,
>    private Listener/Stream(Int) codecs, provisional ownership, close-vs-wait
>    across workers and a portable-scope concurrent connection fixture.
> 4. [ ] Process: bounded entropy, UTF-8/framing, no shell expansion, inherited
>    streams, private Child(Int) codec, wait caching, terminate and reaping on every
>    ownership edge, including native exit after child cleanup.
> 5. [ ] Stdio: cancellable subscriber input, exact byte output, flush/error stream,
>    legacy i/io compatibility and context turnover; finish registration/spec lists
>    and B.7 with blocking Int adapters, not fd/externref shortcuts.
> 6. [ ] Run permission/error/resource/security and concurrency reviews; include
>    unsupported browser/preview-1 behavior and user-supplied host fakes.
> 7. [ ] Run end-to-end echo, then combined verification in the integration lane.

- Reason: Honor spec §2.4 for exported environments and nominal privacy, using one general field-visibility fix before Fs. Close N9’s forged-Int paths with general declaration grants and no public raw-number injection; ordinary accessible service fakes stay legal. Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Make the implementation/spec handoff explicit: prerequisites now, packaging and MC mechanisms in their own stages. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C023 — docs/design/runtime.md, base lines 1662–1707

- Old sentence(s):

> | Test | Expected result and rule |
> |---|---|
> | Re-evaluate word on the same Generator, and split twice without a draw | Equal (word,next) pairs and equal pairs of generators; D1/§4 immutable data |
> | int(g,0), int(g,-1), at_path(g,[0]) | InvalidBound(0), InvalidBound(-1), InvalidPath([0]); §4 validation, no draw |
> | int(g,1) | 0 and one word consumed; §4 requires at least one word even for bound 1 |
> | Stub words for n=10: W-1, then 7 | Reject W-1, give 7 with two draws; §4 rejection rule |
> | Split initial root vs child path [1] | Their histories contain S(1) vs P(1), not equal generators; §4 domain tags (not an assertion that words must differ) |
> | `task.random() == task.random()` in one task | True; D1 and S7 rule 11; no draw counter in Scheduler |
> | Published SplitMix seed 1337, first three words | 0xb5c19e300e8b07b3, 0xd600e0e216c0ac76, 0xc54efc3b3cc5af29; §4.1 pinned source vectors |
> | Request ready(7); state, take, state, take | Ready(7), 7, Taken, panic request: taken; §5.1 one transfer |
> | Take Int and String request handles stored together only as tickets | Read each original typed handle correctly; Ticket has no answer operation; D2 |
> | Cancel an old cell, reuse its slot, cancel the old alias again | New request remains Waiting and receives its own answer; D4 cell identity/generation |
> | Two same-import requests answered in reverse order | Each original typed handle returns its own value; §5.3 per-cell decoding |
> | Wait on A, host answers B first | B remains Ready, wait on A continues; §5.1 requested-set semantics |
> | Cancel after decode but before an IO await resumes | One close of the provisional socket/file, zero user-body uses; §8.1 Ready/Taken guard |
> | Local socket result vs portable scope child returning that socket | Local join works; portable result stops task: value is not portable; D7 |
> | Portable send_event of a continuation in a choose with always(1) | Stops at event construction, even though it would lose; S7 rule 2 |
> | `sync(choose([send_event(c,1), receive_event(c)]))` with no other task | Deadlock; §6.4 same registration cannot rendezvous with itself |
> | Two ready alternatives with side-effecting map callbacks | First alternative commits; exactly its callback runs in the syncing task; §6.4 |
> | `task.scope((s) { task.go(s, () { 1 })  2 })` | 2, only after child completion; R3; body result is independent of child's result |
> | `both(() { 1 }, () { "x" })`; map_concurrently([3,1], id) | (1,"x") and [3,1]; §6.1 typed separate handles/argument order |
> | Failed child while body joins an unstarted sibling | Error of failed child, body final ran, no deadlock; R2/R4 |
> | Two failures already observed before cancel, paths [1,1] and [1] | Error at [1]; R2 prefix/path order, not first observed error |
> | go A(yield; raise A), go B(raise B), body waits | B on production FIFO; §6.4 go does not yield parent, A yields behind B |
> | `task.scope((s) { s })` then go on returned s | task: go outside its scope; R1 |
> | Join from outer-scope sibling, or channel from another run | task: join outside its scope; task: handle of another run; R1 |
> | Channel returned from a finished scope then receive | task: channel is gone; R3; also test an outer waiter present at scope end |
> | Share returned then get after scope exit | task: shared value is gone; R3 |
> | Cancelled final yields, awaits fake clock, opens scope and rendezvous | Final completes, then original scope error; never boundary is not active; R4 |
> | Parent fake Clock used after child's third resume and in its final | Fake result survives; keep_env plus retained frame, D16 workaround |
> | Nested wait_flag while an outer sibling sets the flag | Both complete, no global hang; §6.5 Runner wait delegates to outer task |
> | Rebind Runner with top(seed) inside a task, then nested run | New seed, preserved nested prefix/outer wait route; §6.5 contextual top |
> | Nested raw request in request.within, with no task waiters; tasks block on channel | Wait for the request in that run's Root, then deadlock; §6.4 all-group root enumeration |
> | Infinite yield task plus an outstanding accept | Conditional status-2 polls for the waiting accept task deliver completion and its handler gets a turn; §6.4 |
> | Pure task path/yield/join operations with no host requests | No host poll, status 2, or macrotask yield; task entry can return status 0 immediately; D5/§6.4 |
> | test with script selecting nonexistent path [99] | NotRunnable([99]) of type ScheduleError after cleanup; §6.5 |
> | test with real pending timer rather than a fake | task: test needs fake IO; D10; fixed seed alone does not fix host time |
> | Net read(0); fs write with partial byte; process NUL argument | Typed invalid errors with the named operation; §7 validation |
> | Stream write of N bytes followed by peer EOF | Peer receives the exact N-byte prefix before EOF; §7.3 write-all/half-close |
> | Two concurrent stream reads; read concurrent with write | busy for second read; opposite directions work; §7.3 leases |
> | Close File/Stream/Child twice | Nil both times, underlying release once; §7/§8 idempotence |
> | Process args codec for ["a","é"] | Bytes `<<2,0,0,0,1,0,0,0,97,2,0,0,0,195,169>>`; §7.4 UTF-8 framing |
> | Missing env variable; browser raw TCP | ProcessError(...,"not_found"); NetError(...,"unsupported"); §7 and target matrix |
> | WASI pure tasks/random; WASI module requiring Fs | Pure calculations work; report ordinary blocking imports requiring an embedder; §B.7, no fake fd-as-externref |
> | @main needs user State with no binding vs standard Scheduler/Runner/Await/Root/Owner | State is an entry compile error; standard needs are provisioned without changing the type; async classification follows D5, not mere use of any request helper |
> | Existing async user service, not an IO namespace, used in two children | Overlap via Await, same request table as IO; D3, no namespace rule |

- New sentence(s):

> | Test | Expected result and rule |
> |---|---|
> | Re-evaluate word on the same Generator, and split twice without a draw | Equal (word,next) pairs and equal pairs of generators; D1/§4 immutable data |
> | int(g,0), int(g,-1), at_path(g,[0]) | InvalidBound(0), InvalidBound(-1), InvalidPath([0]); §4 validation, no draw |
> | int(g,1) | 0 and one word consumed; §4 requires at least one word even for bound 1 |
> | Stub words for n=10: W-1, then 7 | Reject W-1, give 7 with two draws; §4 rejection rule |
> | Split initial root vs child path [1] | Their histories contain S(1) vs P(1), not equal generators; §4 domain tags (not an assertion that words must differ) |
> | `task.random() == task.random()` in one task | True; D1 and S7 rule 11; no draw counter in Scheduler |
> | Published SplitMix seed 1337, first three words | 0xb5c19e300e8b07b3, 0xd600e0e216c0ac76, 0xc54efc3b3cc5af29; §4.1 pinned source vectors |
> | Request ready(7); state, take, state, take | Ready(7), 7, Taken, panic request: taken; §5.1 one transfer |
> | Take Int and String request handles stored together only as tickets | Read each original typed handle correctly; Ticket has no answer operation; D2 |
> | Cancel an old cell, reuse its slot, cancel the old alias again | New request remains Waiting and receives its own answer; D4 cell identity/generation |
> | Two same-import requests answered in reverse order | Each original typed handle returns its own value; §5.3 per-cell decoding |
> | Wait on A, host answers B first | B remains Ready, wait on A continues; §5.1 requested-set semantics |
> | Cancel after decode but before an IO await resumes | One close of the provisional socket/file, zero user-body uses; §8.1 Ready/Taken guard |
> | Local/portable scope child returns File/Listener/Stream/Child | Both join; portable is True under D9; a Request/Ticket result still stops under D7 |
> | Portable send_event of a continuation in a choose with always(1) | Stops at event construction, even though it would lose; S7 rule 2 |
> | `sync(choose([send_event(c,1), receive_event(c)]))` with no other task | Deadlock; §6.4 same registration cannot rendezvous with itself |
> | Two ready alternatives with side-effecting map callbacks | First alternative commits; exactly its callback runs in the syncing task; §6.4 |
> | `task.scope((s) { task.go(s, () { 1 })  2 })` | 2, only after child completion; R3; body result is independent of child's result |
> | `both(() { 1 }, () { "x" })`; map_concurrently([3,1], id) | (1,"x") and [3,1]; §6.1 typed separate handles/argument order |
> | Failed child while body joins an unstarted sibling | Error of failed child, body final ran, no deadlock; R2/R4 |
> | Two failures already observed before cancel, paths [1,1] and [1] | Error at [1]; R2 prefix/path order, not first observed error |
> | go A(yield; raise A), go B(raise B), body waits | B on production FIFO; §6.4 go does not yield parent, A yields behind B |
> | `task.scope((s) { s })` then go on returned s | task: go outside its scope; R1 |
> | Join from outer-scope sibling, or channel from another run | task: join outside its scope; task: handle of another run; R1 |
> | Channel returned from a finished scope then receive | task: channel is gone; R3; also test an outer waiter present at scope end |
> | Share returned then get after scope exit | task: shared value is gone; R3 |
> | Cancelled final yields, awaits fake clock, opens scope and rendezvous | Final completes, then original scope error; never boundary is not active; R4 |
> | Parent fake Clock used after child's third resume and in its final | Fake result survives; keep_env plus retained frame, D16 workaround |
> | Nested wait_flag while an outer sibling sets the flag | Both complete, no global hang; §6.5 Runner wait delegates to outer task |
> | Rebind Runner with top(seed) inside a task, then nested run | New seed, preserved nested prefix/outer wait route; §6.5 contextual top |
> | Nested raw request in request.within, with no task waiters; tasks block on channel | Wait for the request in that run's Root, then deadlock; §6.4 all-group root enumeration |
> | Infinite yield task plus an outstanding accept | Conditional status-2 polls for the waiting accept task deliver completion and its handler gets a turn; §6.4 |
> | Pure task path/yield/join operations with no host requests | No host poll, status 2, or macrotask yield; task entry can return status 0 immediately; D5/§6.4 |
> | test with script selecting nonexistent path [99] | NotRunnable([99]) of type ScheduleError after cleanup; §6.5 |
> | test with real pending timer rather than a fake | task: test needs fake IO; D10; fixed seed alone does not fix host time |
> | Net read(0); fs write with partial byte; process NUL argument | Typed invalid errors with the named operation; §7 validation |
> | Stream write of N bytes followed by peer EOF | Peer receives the exact N-byte prefix before EOF; §7.3 write-all/half-close |
> | Two concurrent stream reads; read concurrent with write | busy for second read; opposite directions work; §7.3 leases |
> | Close File/Stream/Child twice | Nil both times, underlying release once; §7/§8 idempotence |
> | Close while another worker awaits read/accept/wait | closed if close commits first; prior committed answer stays; §8.1 leases |
> | Stop with outstanding accept and forgotten Ready spawn, then run another test | No i finals; empty new table, fresh numbers and host-owned child cleanup; §8.1/native N12 |
> | Redeclare raw namespace or external alias | No automatic binding; local fake still works; native §3.1 |
> | `(raw ! net.Net) { raw.read(1, 1) }` outside i/net | Constructor/field visibility error under §2.4/§7, despite the exported type name |
> | Force a small allocator limit | Memory stop before insertion, never reuse; native N9.2 |
> | Process args codec for ["a","é"] | Bytes `<<2,0,0,0,1,0,0,0,97,2,0,0,0,195,169>>`; §7.4 UTF-8 framing |
> | Missing env variable; browser raw TCP | ProcessError(...,"not_found"); NetError(...,"unsupported"); §7 and target matrix |
> | WASI pure tasks/random; WASI requiring private Fs | Pure calculations work; report blocking Int imports requiring a buffer-aware adapter; §8.2/B.7, no fd identity reuse |
> | @main needs user State with no binding vs standard Scheduler/Runner/Await/Root/Owner | State is an entry compile error; standard needs are provisioned without changing the type; async classification follows D5, not mere use of any request helper |
> | Existing async user service, not an IO namespace, used in two children | Overlap via Await, same request table as IO; D3, no namespace rule |

- Reason: Honor spec §2.4 for exported environments and nominal privacy, using one general field-visibility fix before Fs. Close N9’s forged-Int paths with general declaration grants and no public raw-number injection; ordinary accessible service fakes stay legal. Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Adopt N1–N4 and the reviewed artifact/loading constraints, keeping explicit ilang/WASI and rejecting only required unsupplied imports. Make the implementation/spec handoff explicit: prerequisites now, packaging and MC mechanisms in their own stages. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C024 — docs/design/runtime.md, base lines 1736–1747

- Old sentence(s):

> ```i
> leak = (listener, n ! net.Net) {
>   r = n.accept(listener)
>   request.await(r)
> }
> ```
>
> Let the host decode Ready(Ok(stream)), then cancel this task before it resumes.
> A plain cancel would discard the only i owner without installing close. Fixed
> by taken/ready distinction and the ordinary error-free resource guard in §8.1.
> The program above is deliberately low-level and must install that guard; the
> public net.accept wrapper always does. No extra host ack is smuggled into ABI v3.

- New sentence(s):

> ```i
> // Inside i/net; its raw Net service is private.
> leak = (Listener(number), n ! Net) {
>   r = n.accept(number)
>   request.await(r)
> }
> ```
>
> Let the host decode Ready(Ok(number)), then cancel this task before it resumes.
> A plain cancel would discard the only i owner without installing close. Fixed
> by taken/ready distinction and the ordinary error-free resource guard in §8.1.
> The program above is deliberately low-level and must install that guard; the
> public net.accept wrapper always does. No extra host ack is smuggled into ABI v3.

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics.

### C025 — docs/design/runtime.md, base lines 1873–1878

- Old sentence(s):

> Try implementing WASI File with an i32: it cannot satisfy externref/reference_id,
> so this note does not require that adapter. Try browser TCP via fetch: unsupported
> is a typed result, not a false TCP implementation. Try a Node timer or Rust
> blocking stdin after entry cancellation: abortable timers and a detached broker
> must not keep the run waiting for abandoned work. Those are explicit host gates,
> not an inference that cancellation stops all OS activity.

- New sentence(s):

> The historical File was an externref. Adopted N9 now requires File(Int) and
> an explicit raw Int codec; a bare reusable fd still violates non-reuse/context
> ownership. This track still supplies no preview-1 adapter. Try browser TCP via fetch: unsupported
> is a typed result, not a false TCP implementation. Try a Node timer or Rust
> blocking stdin after entry cancellation: abortable timers and a detached broker
> must not keep the run waiting for abandoned work. Those are explicit host gates,
> not an inference that cancellation stops all OS activity.

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Make the implementation/spec handoff explicit: prerequisites now, packaging and MC mechanisms in their own stages. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C026 — docs/design/runtime.md, base lines 1902–1909

- Old sentence(s):

> For the echo server, cancel before the initial acceptor starts (no accepted
> socket exists), during accept (host drops provisional work), after decode but
> before take (the wrapper closes Ready), and while serving (ensure closes the
> stream and the scope cancels the next acceptor). A queued next acceptor owns
> only the listener capability whose outer acquire outlives all children. This
> removes the original unstarted-resource gap without weakening S7's cancel rule.
> A stop is still allowed to skip i finals and relies on instance destruction
> for host cleanup.

- New sentence(s):

> For the echo server, cancel before the initial acceptor starts (no accepted
> socket exists), during accept (host drops provisional work), after decode but
> before take (the wrapper closes Ready), and while serving (ensure closes the
> stream and the scope cancels the next acceptor). A queued next acceptor owns
> only the listener capability whose outer acquire outlives all children. This
> removes the original unstarted-resource gap without weakening S7's cancel rule.
> A stop skips i finals and relies on explicit ResourceContext teardown, including
> forgotten/provisional integer resources (§8.1), for host cleanup.

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts.

### C027 — docs/design/runtime.md, base lines 2143–2148

- Old sentence(s):

> The proposed i scheduler, request/Root primitives, revised source programs,
> O0/O2 WAT, JS callback fixture, OS cleanup and real echo fixture remain unrun in
> this design lane. The statistical gate remains unverified. Reading the existing
> entry/host code establishes the baseline and needed edit regions, not that the
> new implementation exists. No implementation or other note is changed here. No cargo or host integration
> tests were run for this documentation-only revision.

- New sentence(s):

> The proposed i scheduler, request/Root primitives, revised source programs,
> O0/O2 WAT, JS callback fixture, OS cleanup and real echo fixture remain unrun in
> this design lane. The statistical gate remains unverified. Reading the existing
> entry/host code establishes the baseline and needed edit regions, not that the
> new implementation exists. That earlier runtime revision changed no implementation. This native adoption
> also reconciles native.md and multicore.md; new checks are recorded in native §11. No cargo or host integration
> tests were run for this documentation-only revision.

- Reason: Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Make the implementation/spec handoff explicit: prerequisites now, packaging and MC mechanisms in their own stages. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C028 — docs/design/multicore.md, base lines 3–3

- Old sentence(s):

> Status: draft for review (study S7 of the redesign plan). It is reviewed at gate G2. It fixes what the first `i/task` (track R) and the language must guarantee now, so that the multicore wave (MC-U, MC-D, G5, MC-I) can add more workers later without changing what any program does. It does not design the multicore runtime; it lists the invariants that runtime must keep. Code facts are from commit c69515c. Line numbers are at that commit.

- New sentence(s):

> Status: draft for review (study S7 of the redesign plan). It is reviewed at gate G2. It fixes what the first `i/task` (track R) and the language must guarantee now, so that the multicore wave (MC-U, MC-D, G5, MC-I) can add more workers later without changing what any program does. It does not design the multicore runtime; it lists the invariants that runtime must keep. Native adoption replaces D8, the MC channel commit backend and native transport
> with native.md N7/N8/N11/N12. Runtime.md supplies the accepted request, join and
> Runner refinements. Other rules remain. Code facts are from commit c69515c. Line numbers are at that commit.

- Reason: Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C029 — docs/design/multicore.md, base lines 11–14

- Old sentence(s):

> - One module, **N instances**. Each instance is a **worker**. All workers import one shared linear memory.
> - Each worker has its own task deque. Only **unstarted** tasks are stolen, and a stolen task is **copied**: encoded into the shared memory by its home worker and decoded by the thief.
> - `scope.share` puts large immutable data in the shared memory once. It is freed when its scope ends.
> - Later, a shared GC heap takes the place of most copies.

- New sentence(s):

> - One module, **N instances**. Each instance is a **worker**. Native transports bytes in runner memory; browsers use shared linear memory (native N11).
> - At `go`, the maker silently classifies the complete task record. A portable record is encoded into one shared queue; other records stay local. A worker claims an unstarted record once. The maker runs its retained original; another worker decodes the bytes.
> - `scope.share` stores immutable encoded data once in the host transport. It is freed when its scope ends.
> - Later, a shared GC heap takes the place of most copies.

- Reason: Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Adopt N11: native uses runner byte storage/private memories; only the browser transport needs shared Wasm memory. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics.

### C030 — docs/design/multicore.md, base lines 22–28

- Old sentence(s):

> 1. **Function identity.** §11.4 says a function is the same only as itself. After a copy, the copy is a different GC object, and `ref.eq` says "different".
> 2. **Values that cannot move.** A boundary and a continuation are tied to one Wasm stack and to the function references of one instance. A host reference is rooted in one wasmtime `Store` or one JS realm. A host request (host-concurrency.md §6.4) has an id in the request table of one instance. None of them can be copied.
> 3. **No static rule.** A function type does not show its captures, width subtyping hides record fields, and a type variable hides everything. A `Send`-like constraint would be a qualified type and would lose principal inference.
> 4. **The environment.** §9.2 closures do not capture the environment, so a task started by `go` would get the services of the scheduler loop, not those of the place of `go`. And if the task did hold the scheduler's own boundary, no task could ever move.
> 5. **Order and timing.** Error choice, results, random numbers and handle ids must not depend on which worker ran first.
> 6. **Sharing and cycles.** WasmGC has no identity hash, and every `$obj` field is immutable (`runtime/mod.rs:234`). So a naive copy turns a DAG of depth d into a tree of 2^d nodes, and a naive walk never ends on a local recursive function, whose closure captures itself (part 3.6).
> 7. **Hosts.** wasmtime needs one `Store` per thread and a shared memory. Browsers need `SharedArrayBuffer`, which needs cross-origin isolation (COOP and COEP), and the main thread must never block.

- New sentence(s):

> 1. **Function identity.** §11.4 says a function is the same only as itself. After a copy, the copy is a different GC object, and `ref.eq` says "different".
> 2. **Values that cannot move.** A boundary and a continuation are tied to one Wasm stack and to the function references of one instance. A host reference is rooted in one wasmtime `Store` or one JS realm. A host request (host-concurrency.md §6.4) has an id in the request table of one instance. None of them can be copied.
> 3. **No static rule.** A function type does not show its captures, width subtyping hides record fields, and a type variable hides everything. A `Send`-like constraint would be a qualified type and would lose principal inference.
> 4. **The environment.** §9.2 closures do not capture the environment, so a task started by `go` would get the services of the scheduler loop, not those of the place of `go`. And if the task did hold the scheduler's own boundary, no task could ever move.
> 5. **Order and timing.** Error choice, results, random numbers and handle ids must not depend on which worker ran first.
> 6. **Sharing and cycles.** WasmGC has no identity hash, and every `$obj` field is immutable (`runtime/mod.rs:234`). So a naive copy turns a DAG of depth d into a tree of 2^d nodes, and a naive walk never ends on a local recursive function, whose closure captures itself (part 3.6).
> 7. **Hosts.** wasmtime needs one `Store` per executing worker; native transport needs no shared Wasm memory. Browsers need `SharedArrayBuffer`, which needs cross-origin isolation (COOP and COEP), and the main thread must never block.

- Reason: Adopt N11: native uses runner byte storage/private memories; only the browser transport needs shared Wasm memory. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Adopt N1–N4 and the reviewed artifact/loading constraints, keeping explicit ilang/WASI and rejecting only required unsupplied imports.

### C031 — docs/design/multicore.md, base lines 34–34

- Old sentence(s):

> **D2. Portable values, checked at run time, on every worker count.** A value is **portable** when nothing it holds is a boundary, a continuation, a host reference, a host request or a `Syntax` value (part 3.1). Every path on which a value leaves its task and may reach another worker checks this, also on one worker, and stops with `panic: task: value is not portable` when the check fails. *Reason:* a static rule cannot work (hard part 3). A check that only ran on N workers would make placement visible. A check that runs always gives the same stop everywhere.

- New sentence(s):

> **D2. Portable values, checked at run time, on every worker count.** A value is **portable** when nothing it holds is a boundary, a continuation, a host reference, a host request or a `Syntax` value (part 3.1). Portable send/result/error/share paths check this, also on one worker, and stop with `panic: task: value is not portable` on failure. Go publication instead silently classifies the complete task record and pins on failure (D8/P1). *Reason:* a static rule cannot work (hard part 3). A check that only ran on N workers would make placement visible. A check that runs always gives the same stop everywhere.

- Reason: Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C032 — docs/design/multicore.md, base lines 48–48

- Old sentence(s):

> **D5. The scheduler reaches a task only as an argument from the worker loop.** Each worker runs its own scheduler loop. For each task start, the loop makes a **new boundary for that task**, outside the loop's own frames (`control.boundary((b) { start(scheduler_for(b)) })`), and gives the task a `Scheduler` service whose suspends go to `b`, as the argument of the kept function. When the task suspends, `$boundary_run` (`emit/control.rs:195-232`) runs `f(k)` in the boundary's place, so control comes back to the loop with the continuation `k`, and `b` is not active. `control.resume(k, v)`, called from the loop, makes `b` active again for the rest of the task. There is no long-lived boundary of the worker: a suspend to a boundary that encloses the loop would capture the loop itself into `k`. No task closure, kept service, channel handle, scope handle or event that `i/task` makes ever holds a `Scheduler` that the task did not get this way (a kept function that user code makes inside a task does hold its task's `Scheduler`, D4). Every task runs directly from its run's loop, never on another task's stack. The loop never calls `control.cancel` on a task's continuation: a cancel resumes the task, and the task unwinds itself under an inner boundary, so that `b` stays active while its final functions run (part 6). *Reason:* if task closures held the scheduler's boundary, every task would be non-portable, nothing could be stolen, and every send of a closure would stop. Running every task from the loop under its own boundary makes "a boundary of another task is never active" true on any number of workers. A nested run (part 4) runs its loop inside the boundary body of its calling task; that is the one case where a task of one run is on the stack of a task of another run, and part 4 gives its rules.

- New sentence(s):

> **D5. The scheduler reaches a task only as an argument from the worker loop.** Each worker runs its own scheduler loop. For each task start, the loop makes a **new boundary for that task**, outside the loop's own frames (`control.boundary((b) { start(scheduler_for(b)) })`), and gives the task a `Scheduler` service whose suspends go to `b`, as the argument of the kept function. When the task suspends, `$boundary_run` (`emit/control.rs:195-232`) runs `f(k)` in the boundary's place, so control comes back to the loop with the continuation `k`, and `b` is not active. `control.resume(k, v)`, called from the loop, makes `b` active again for the rest of the task. There is no long-lived boundary of the worker: a suspend to a boundary that encloses the loop would capture the loop itself into `k`. No task closure, kept service, channel handle, scope handle or event that `i/task` makes ever holds a `Scheduler` that the task did not get this way (a kept function that user code makes inside a task does hold its task's `Scheduler`, D4). Every task runs directly from its run's loop, never on another task's stack. The loop never calls `control.cancel` on a task's continuation: a cancel resumes the task, and the task unwinds itself under an inner boundary, so that `b` stays active while its final functions run (part 6). *Reason:* if task closures held the scheduler's boundary, every task would be non-portable, nothing could be remotely claimed, and every send of a closure would stop. Running every task from the loop under its own boundary makes "a boundary of another task is never active" true on any number of workers. A nested run (part 4) runs its loop inside the boundary body of its calling task; that is the one case where a task of one run is on the stack of a task of another run, and part 4 gives its rules.

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Adopt N8’s one-lock backend while retaining the i scheduler, typed local storage and the accepted CML transaction semantics.

### C033 — docs/design/multicore.md, base lines 52–54

- Old sentence(s):

> **D7. Every worker works out the top-level values itself, and only portable ones are allowed on N workers.** `ilang_start` runs on every worker, with a fixed id stream, so every worker gets equal top-level values with equal ids. Top-level names are not captures, so top-level values never need to move. Two conditions keep this from being seen, and when either fails the run uses one worker (a load-time choice, D12): (a) after start-up on worker 0, every top-level value must be portable; (b) a **host external** (one that is not of the library package `i`; part 3.5) called during start-up must be declared pure by the host. *Reason:* a host reference made at start-up would be a different object on each worker, and a top-level function that uses it is a constant closure that a thief would run against its own object (part 3.5). Condition (a) catches every such value with the walk that `portable` already needs.
>
> **D8. Steal is cooperative: the victim encodes.** A worker cannot read another worker's GC objects. So a thief only asks, through a request cell in shared memory, and the victim encodes its oldest unstarted task at its next poll point. If the encode meets a value that is not portable, that task stays home and the victim tries the next one or answers "none". Started tasks never move. *Reason:* this is the only way that works with per-instance GC heaps, and it costs one atomic load per poll point when nobody steals.

- New sentence(s):

> **D7. Every worker works out the top-level values itself, and only portable ones are allowed on N workers.** `ilang_start` runs on every worker, with a fixed id stream, so every worker gets equal top-level values with equal ids. Top-level names are not captures, so top-level values never need to move. Two conditions keep this from being seen, and when either fails the run uses one worker (a load-time choice, D12): (a) after start-up on worker 0, every top-level value must be portable; (b) a **host external** (one that is not of the library package `i`; part 3.5) called during start-up must be declared pure by the host. *Reason:* a host reference made at start-up would be a different object on each worker, and a top-level function that uses it is a constant closure that a remote claimant would run against its own object (part 3.5). Condition (a) catches every such value with the walk that `portable` already needs.
>
> **D8. Publish at go into one shared queue.** On multiple workers the maker
> silently classifies the complete immutable task record at `go`: kept start
> function, captures, every kept service, scope handle, path and chain. If portable,
> it encodes once and publishes bytes, retaining its original. Publication registers
> the child in its scope and rechecks cancellation under the same lock; a cancel
> during encoding drops the unstarted record and frees prepared bytes, with
> runtime R4a applied to the caller. Otherwise a non-portable record queues
> locally, with no rejection. A claim under N8's lock changes Ready to Taken(owner)
> once. The maker uses its original; a remote owner decodes. Cancel/drop and claim
> use the same transaction: cancel before claim drops the record; claim before
> cancel routes cancellation to the claimant. Claimed-but-unstarted records can
> still be dropped and wake joiners under runtime R4a; started tasks never move.
> The maker drops its retained original after remote claim/drop acknowledgement;
> queue entries are freed after claim/drop; a remote claimant owns payload bytes
> until decoding finishes. Queue entries never retain cancelled
> scopes indefinitely. One worker uses its FIFO originals and does no encode.
> *Reason:* native N7 is adopted; only the maker can read its GC heap, and encoding
> at go avoids waiting for a busy maker to poll. Failed portability classification
> pins silently on every worker count; it never becomes P3's visible stop.

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Adopt N8’s one-lock backend while retaining the i scheduler, typed local storage and the accepted CML transaction semantics. Adopt N11: native uses runner byte storage/private memories; only the browser transport needs shared Wasm memory. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Adopt N1–N4 and the reviewed artifact/loading constraints, keeping explicit ilang/WASI and rejecting only required unsupplied imports.

### C034 — docs/design/multicore.md, base lines 60–62

- Old sentence(s):

> **D11. A stop anywhere stops the whole run, and the first stop wins.** *Reason:* §9.5 already says no final functions run at a stop. A stop cell in shared memory, written once by compare-and-swap, keeps the message to one, as on one worker. When two tasks may stop, which one is first is part of the order of D1, so the message may differ; with at most one task that may stop, it is fixed.
>
> **D12. The worker count is the host's choice and cannot be seen.** A browser without cross-origin isolation, a module that fails a condition of D7, and any host that just wants one worker, all run one worker. No `i/task` API shows worker indices, the worker count, or steals. *Reason:* D1.

- New sentence(s):

> **D11. A stop anywhere stops the whole run, and the first stop wins.** Native N12 supplies forced termination of non-polling workers and host teardown; ordinary task scheduling/cancellation remain cooperative. *Reason:* §9.5 already says no final functions run at a stop. The shared stop cell, written once under N8's lock, keeps one message, as on one worker. When two tasks may stop, which one is first is part of the order of D1, so the message may differ; with at most one task that may stop, it is fixed.
>
> **D12. The worker count is the host's choice and cannot be seen.** A browser without cross-origin isolation, a module that fails a condition of D7, and any host that just wants one worker, all run one worker. No `i/task` API shows worker indices, the worker count, or queue claims. *Reason:* D1.

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Adopt N11: native uses runner byte storage/private memories; only the browser transport needs shared Wasm memory.

### C035 — docs/design/multicore.md, base lines 70–73

- Old sentence(s):

> - A **worker** runs tasks. A program run has one or more workers. In the multicore wave, each worker is one instance of the module, all sharing one linear memory.
> - The **captures** of a function value made by a function expression are the local names that its body uses and that are bound outside the function expression, after macro expansion. A name counts when it is written in the body, also in a part that never runs. Top-level names are not captures. A constructor, a built-in, a library function and an `@external` have no captures. A function made by `control.keep_env` has the captures of its argument and the services it keeps.
> - The values a value **holds**: the fields of a tuple, list, custom value or record (its parts, §11.4); the values of the captures of a function; the value in a `Keyed`. Ints, Floats, Strings, codepoints and bit arrays hold nothing. The word is new because §11.4 says a function has no parts that may be looked at, and that stays true: `==` never looks at captures.
> - A value is **portable** when it is not a boundary, a continuation, a host reference, a host request or a `Syntax` value (part 11), and every value it holds is portable. A value may hold itself (a local recursive function, part 3.6); the rule then asks only about the other values on the way, so such a function is portable when they are.

- New sentence(s):

> - A **worker** runs tasks. A program run has one or more workers. In the multicore wave, each worker is one instance; native uses runner byte storage and browsers shared linear memory (native N11).
> - The **captures** of a function value made by a function expression are the local names that its body uses and that are bound outside the function expression, after macro expansion. A name counts when it is written in the body, also in a part that never runs. Top-level names are not captures. A constructor, a built-in, a library function and an `@external` have no captures. A function made by `control.keep_env` has the captures of its argument and the services it keeps.
> - The values a value **holds**: the fields of a tuple, list, custom value or record (its parts, §11.4); the values of the captures of a function; the value in a `Keyed`. Ints, Floats, Strings, codepoints and bit arrays hold nothing. The word is new because §11.4 says a function has no parts that may be looked at, and that stays true: `==` never looks at captures.
> - A value is **portable** when it is not a boundary, a continuation, a host reference, a host request or a `Syntax` value (part 11), and every value it holds is portable. A value may hold itself (a local recursive function, part 3.6); the rule then asks only about the other values on the way, so such a function is portable when they are.

- Reason: Adopt N11: native uses runner byte storage/private memories; only the browser transport needs shared Wasm memory. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics.

### C036 — docs/design/multicore.md, base lines 98–108

- Old sentence(s):

> | Path | Checked with `portable` | Why |
> |---|---|---|
> | Captures and kept services of a `go` body | no | Checked at steal time instead (P1) |
> | A send on a channel made by `channel.new` | yes, at the call of `send` | The receiver may be on another worker |
> | A send event on a channel made by `channel.new` | yes, when the event is made (`channel.send_event(c, v)`), not when `sync` starts or when it commits | The same; checking when the event is made makes the stop independent of which branch of a `choose` commits, and an event that holds a value that is not portable can never be built for such a channel |
> | The result and the error a child gives to a scope made by `scope`, also inside `both`, `race` and `map_concurrently`, which are built on `scope` | yes, when the child ends | The scope's task may be on another worker |
> | The argument of `scope.share` | yes | It goes to shared memory |
> | A join of a Task handle (or a join event) of a child of a scope made by `scope` | no, the result was checked when the child ended | Any task whose chain holds that scope (part 6) may join; another task stops with `task: join outside its scope` |
> | A send on a local channel; the result and the error a child gives to a local scope (also in the local forms `local_both`, `local_race` and `local_map_concurrently`, if R adds them); a join of the Task handle of a child of a local scope | no | Every end is on the home worker (P4) |
>
> **P1. Steal (multicore wave).** A thief may take only an unstarted task. The victim encodes the whole task record (D8): the kept start function with its captures and kept services, and the scope handle. If the encode meets tag 10, 11 or 12, or a host request, the encode stops, the task stays home, and nothing can be seen. So `go` itself does no check.

- New sentence(s):

> | Path | Checked with `portable` | Why |
> |---|---|---|
> | Complete task record of a `go` body | silent classification, no stop | At go on N workers; non-portable records stay local (P1) |
> | A send on a channel made by `channel.new` | yes, at the call of `send` | The receiver may be on another worker |
> | A send event on a channel made by `channel.new` | yes, when the event is made (`channel.send_event(c, v)`), not when `sync` starts or when it commits | The same; checking when the event is made makes the stop independent of which branch of a `choose` commits, and an event that holds a value that is not portable can never be built for such a channel |
> | The result and the error a child gives to a scope made by `scope`, also inside `both`, `race` and `map_concurrently`, which are built on `scope` | yes, when the child ends | The scope's task may be on another worker |
> | The argument of `scope.share` | yes | It goes to the shared host byte transport |
> | A join of a Task handle (or a join event) of a child of a scope made by `scope` | no, the result was checked when the child ended | Any task whose chain holds that scope (part 6) may join; another task stops with `task: join outside its scope` |
> | A send on a local channel; the result and the error a child gives to a local scope (also in the local forms `local_both`, `local_race` and `local_map_concurrently`, if R adds them); a join of the Task handle of a child of a local scope | no | Every end is on the home worker (P4) |
>
> **P1. Publication (multicore wave).** At go the maker classifies the whole
> record described in §3.3, never just the closure. A non-portable record stays
> local without error (D8). A portable record is encoded once and put in the
> shared queue, and any worker may claim it only while it is unstarted. The maker
> uses its retained original if it claims; a remote claimant decodes. Every claim,
> cancel and drop has the single owner/transition rule of D8. On one worker the
> same pinning outcome follows from using only the original, with no encoding.

- Reason: Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Adopt N11: native uses runner byte storage/private memories; only the browser transport needs shared Wasm memory. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C037 — docs/design/multicore.md, base lines 114–114

- Old sentence(s):

> **P4. Local channels and local scopes.** For values that must stay home, such as a host reference to a socket given to a handler task, `i/task` gives local channels and local scopes. A local handle holds the boundary of the task that made it (D5), which it gets from that task's `Scheduler` service when it is made. The boundary is there **only to make the handle non-portable**; it is never used for a suspend (a suspend to it from another task would stop with `control: boundary is not active`, and from the making task would suspend that task, which the scheduler never wants). So the handle is itself not portable, by the general rule, with no flag and no special case. A boundary is a value like any other: it may be stored in data that outlives the body of `control.boundary` (§9.4 only limits `suspend` to active boundaries), also after its task has ended. **A child of a local scope is never stolen**: its task record holds the local scope handle, so the encode of P1 fails. **The Task handle of a child of a local scope, and every join event made from it, hold the local scope's handle too**, so they are not portable either: they can be sent only on local channels, and only a task on the home worker can join them. This holds whatever the child's body captures; `l.go(() { open_socket() })` in a local scope `l` stays home and gives its host reference back on its own worker, to tasks on that worker only.

- New sentence(s):

> **P4. Local channels and local scopes.** For values that must stay home, such as a user host reference given to a handler task (standard IO resources are Int wrappers, native N9), `i/task` gives local channels and local scopes. A local handle holds the boundary of the task that made it (D5), which it gets from that task's `Scheduler` service when it is made. The boundary is there **only to make the handle non-portable**; it is never used for a suspend (a suspend to it from another task would stop with `control: boundary is not active`, and from the making task would suspend that task, which the scheduler never wants). So the handle is itself not portable, by the general rule, with no flag and no special case. A boundary is a value like any other: it may be stored in data that outlives the body of `control.boundary` (§9.4 only limits `suspend` to active boundaries), also after its task has ended. **A child of a local scope is never remotely claimed**: its task record holds the local scope handle, so P1 classifies it as non-portable. **The Task handle of a child of a local scope, and every join event made from it, hold the local scope's handle too**, so they are not portable either: they can be sent only on local channels, and only a task on the home worker can join them. This holds whatever the child's body captures; `l.go(() { open_socket() })` in a local scope `l` stays home and gives its host reference back on its own worker, to tasks on that worker only.

- Reason: Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Adopt N8’s one-lock backend while retaining the i scheduler, typed local storage and the accepted CML transaction semantics.

### C038 — docs/design/multicore.md, base lines 122–123

- Old sentence(s):

> 1. In the task's record at `go`: its closure, its kept services or its scope handle. The task is unstarted until it runs. If it is stolen, P1 walks all of it, so a task whose record reaches a non-portable value is never stolen.
> 2. By a receive, a join, or a `shared.get` while the task runs. The task is then started, and by P2 it never moves. If it gives such a value to a new child with `go`, the child's record holds it, and case 1 applies to the child.

- New sentence(s):

> 1. In the task's record at `go`: its closure, its kept services or its scope handle. The task is unstarted until it runs. Before publication, P1 walks all of it, so a task whose record reaches a non-portable value is never remotely claimed.
> 2. By a receive, a join, or a `shared.get` while the task runs. The task is then started, and by P2 it never moves. If it gives such a value to a new child with `go`, the child's record holds it, and case 1 applies to the child.

- Reason: Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Adopt N8’s one-lock backend while retaining the i scheduler, typed local storage and the accepted CML transaction semantics.

### C039 — docs/design/multicore.md, base lines 127–129

- Old sentence(s):

> - A send on a channel made by `channel.new`, a result or error given to a scope made by `scope`, or `share`: P3 stops it unless it is portable.
> - A send on a local channel: the sending task holds the local handle. By cases 1 and 2, a task that holds a non-portable handle is on that handle's home worker, so the value stays there.
> - A result or error given to a local scope: the child's record holds the local scope handle, so by P4 the child was never stolen and runs on the scope's home worker. The result is read by the scope's own task, or by a join of the child's Task handle. That handle holds the local scope handle (P4), so by cases 1 and 2 every task that holds it is on the home worker.

- New sentence(s):

> - A send on a channel made by `channel.new`, a result or error given to a scope made by `scope`, or `share`: P3 stops it unless it is portable.
> - A send on a local channel: the sending task holds the local handle. By cases 1 and 2, a task that holds a non-portable handle is on that handle's home worker, so the value stays there.
> - A result or error given to a local scope: the child's record holds the local scope handle, so by P4 the child was never remotely claimed and runs on the scope's home worker. The result is read by the scope's own task, or by a join of the child's Task handle. That handle holds the local scope handle (P4), so by cases 1 and 2 every task that holds it is on the home worker.

- Reason: Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Adopt N8’s one-lock backend while retaining the i scheduler, typed local storage and the accepted CML transaction semantics.

### C040 — docs/design/multicore.md, base lines 133–133

- Old sentence(s):

> So a stolen task holds no local handle of its old worker and belongs to no local scope of it. Every local handle it gets later is made on, or received from, a local path of its new worker. H holds.

- New sentence(s):

> So a remotely claimed task holds no local handle of its old worker and belongs to no local scope of it. Every local handle it gets later is made on, or received from, a local path of its new worker. H holds.

- Reason: Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership.

### C041 — docs/design/multicore.md, base lines 152–158

- Old sentence(s):

> - §2.5 says top-level values need no environment, so they never call host services. The only ways a top-level value can be non-portable are: an `@external` that gives a host reference (an opaque custom type, §10.3), and a boundary or continuation made by `control.boundary` during start-up.
> - Such a value is a problem even though it never moves. Top-level names are not captures, so `counter = ext_counter_new()` and `bump = () { ext_counter_inc(counter) }` make `bump` a portable constant closure. Each worker's start-up makes its own counter (`reference_id` is per instance, `lib.rs:602`), and a stolen `bump` would increment the thief's counter: `scope((s) { list.map(range(0, 100), (i) { s.go(bump) }) })` would leave less than 100 in the counter that worker 0 reads. A top-level continuation is the same: each worker would have its own copy to resume.
> - So every worker runs `ilang_start` (D7), and the run uses N workers only when both conditions hold:
>   - (a) **Portable top level.** After start-up on worker 0, the runtime walks every top-level value with the `portable` walk (`ilang_toplevel_portable`, multicore wave). If one is not portable, the run uses one worker. Then `counter` above gives 100, as on one worker.
>   - (b) **Pure start-up externals.** A **host external** is an `@external` that is not of the library package `i`. The `@external` declarations of `i/core` (`not`, `to_float`, `inspect`, `hash`, …; `@external("ilang:core", …)` and `ilang:prelude`'s `panic`) are operations of the runtime, which `lower.rs:1379` and the name tables near `lower.rs:120` map to ops; they do not count. The compiler knows statically which host externals the full needs (§2.5) of the top-level bindings **that are not direct values** reach, and records them in the metadata (§B.6, multicore wave). These are the only bindings whose expressions start-up runs; the body of a top-level function such as `main` is not run at start-up, so an external that only function bodies use does not count. A module with such externals runs N workers only when the host declares those externals pure: each call with equal arguments gives an equal result, and calling them again on each worker has no effect that the program or the user can see. Otherwise it runs one worker. This is the same line that `meta.md` §8 draws for macros (the full needs of top-level values that are not a direct value).
> - Both are choices made when the module is loaded or started, and they cannot be seen. `docs/performance.md` notes the lost parallelism.
> - Per-instance runtime state is set up on every worker as today: label globals, `$constructors_install`, the shadow stack and the site table.

- New sentence(s):

> - §2.5 says top-level values need no environment, so they never call host services. The only ways a top-level value can be non-portable are: an `@external` that gives a host reference (an opaque custom type, §10.3), and a boundary or continuation made by `control.boundary` during start-up.
> - Such a value is a problem even though it never moves. Top-level names are not captures, so `counter = ext_counter_new()` and `bump = () { ext_counter_inc(counter) }` make `bump` a portable constant closure. Each worker's start-up makes its own counter (`reference_id` is per instance, `lib.rs:602`), and a remotely claimed `bump` would increment the remote claimant's counter: `scope((s) { list.map(range(0, 100), (i) { s.go(bump) }) })` would leave less than 100 in the counter that worker 0 reads. A top-level continuation is the same: each worker would have its own copy to resume.
> - So every worker runs `ilang_start` (D7), and the run uses N workers only when both conditions hold:
>   - (a) **Portable top level.** After start-up on worker 0, the runtime walks every top-level value with the `portable` walk (`ilang_toplevel_portable`, multicore wave). If one is not portable, the run uses one worker. Then `counter` above gives 100, as on one worker.
>   - (b) **Pure start-up externals.** A **host external** is an `@external` that is not of the library package `i`. The `@external` declarations of `i/core` (`not`, `to_float`, `inspect`, `hash`, …; `@external("ilang:core", …)` and `ilang:prelude`'s `panic`) are operations of the runtime, which `lower.rs:1379` and the name tables near `lower.rs:120` map to ops; they do not count. The compiler knows statically which host externals the full needs (§2.5) of the top-level bindings **that are not direct values** reach, and records them in the metadata (§B.6, multicore wave). These are the only bindings whose expressions start-up runs; the body of a top-level function such as `main` is not run at start-up, so an external that only function bodies use does not count. A module with such externals runs N workers only when the host declares those externals pure: each call with equal arguments gives an equal result, and calling them again on each worker has no effect that the program or the user can see. Otherwise it runs one worker. This is the same line that `meta.md` §8 draws for macros (the full needs of top-level values that are not a direct value).
> - Both are choices made when the module is loaded or started, and they cannot be seen. `docs/performance.md` notes the lost parallelism.
> - Per-instance runtime state is set up on every worker as today: label globals, `$constructors_install`, the shadow stack and the site table.

- Reason: Close N9’s forged-Int paths with general declaration grants and no public raw-number injection; ordinary accessible service fakes stay legal. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Adopt N1–N4 and the reviewed artifact/loading constraints, keeping explicit ilang/WASI and rejecting only required unsupplied imports.

### C042 — docs/design/multicore.md, base lines 175–175

- Old sentence(s):

> `go` then looks like this (a sketch for track R, not its final API):

- New sentence(s):

> The following sketch shows only Scheduler routing. Runtime.md §6.3 supplies
> its exact accepted extension: detach Scheduler, Runner and Await before keep_env,
> rebind all three from the task's real Scheduler in a retained frame, and keep
> Root/Owner unchanged. This is required at go and task.keep; keeping a real parent
> Runner/Await would pin every child. The sketch is not the final API or a second
> prescription for entry bootstrap:

- Reason: Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics.

### C043 — docs/design/multicore.md, base lines 194–208

- Old sentence(s):

> - `scheduler.detached` is a top-level record of constant functions that stop with `panic: task: no scheduler`. It is portable. Binding it before `keep_env` puts it, not the parent's scheduler, in the kept array, so the parent's scheduler boundary is never kept or captured. Inside, `! Scheduler = own` binds the real one, so `detached` is never called.
> - `s.spawn(start)` makes the task record (part 3.3), with the handle of the scope `s`.
> - The worker loop calls `start(own)` inside the task's new boundary (D5), with the `Scheduler` whose suspends go to that boundary. The environment statement is inside the task, in a frame that is kept, so it stays with the task's paused part (§9.4 rule 3).
> - Kept services may still be non-portable, for example a generator's `Yield` service, or the `Assert` of `assertion.catch`, which hold a boundary. Every service bound at the place is kept, also one that `body` does not need, so a task started under such a service is pinned by P1. That costs parallelism only, and it cannot be seen. When `body` uses that service, suspending to that boundary from the task stops with `control: boundary is not active` on any number of workers, because the task runs from the loop (D5), and the loop does not run inside that boundary's body.
> - **Runs and nested runs.** A **run** is one call of `task.run`: one loop, its tasks, and the handles they make. Nothing stops i code from calling `task.run` where a run is already going on, and the type of `task.run` cannot forbid it. So the rules are these, and they are the same on every host and every worker count:
>   - **Which run uses N workers.** Only the run at the program's entry (open question 4) may use more than one worker. Every other run is a **nested run**: a one-worker FIFO loop on the stack of the code that called `task.run`. Its tasks are never stolen. When it is called from inside a task, it runs inside that task's boundary body, so the whole nested run is part of the calling task, which is started and never moves (P2).
>   - **The run service.** `task.run` needs one service of `i/task` (its name is R's; here `Runner`), and that is how it learns where it is, with no test for "is a service bound". Inside every task, the scheduler binds `Runner` to one that belongs to that task. At the top, outside any task, the program binds it with an ordinary environment statement from a portable value that `i/task` gives (`task.top(seed)`, part 5), or the entry kind of open question 4 does. A run takes three things from `Runner`: its **path prefix**, its **root seed**, and its **wait**.
>   - **Paths.** At the top the prefix is `[]`. In a task, it is the task's path plus the index of this `task.run` among the task's `go` calls and runs, counted together, so the paths of two runs never meet, and two sibling tasks that each start a nested run get different paths and different generators (part 5).
>   - **Waits.** When a run has no task that can run but has outstanding host requests, it waits through `Runner`. At the top, that is the one root wait of host-concurrency §6.4 (status 1). In a task, the nested run hands its outstanding requests to the outer run and suspends **only its calling task** to the outer `Scheduler`, so the whole nested run pauses as one task, and the outer run's other tasks go on, on one worker as on N; the outer run resumes the calling task when one of those requests ends. So a nested run that waits for a host flag that another outer task sets ends on one worker too. A nested run never returns status 1 from the caller's stack.
>   - **Deadlock.** A nested run with no task that can run and no outstanding request of its own stops with `panic: task: deadlock`. No outer task can wake it, because outer handles cannot be used in it (next point).
>   - **Handles belong to their run.** Every channel, scope, Task, event and `Shared` handle holds the key of the run that made it. Using a handle in a task of another run (a send, a receive, a `sync`, a `go`, a join, a `get`) stops with `panic: task: handle of another run`, also when the handle is portable. So `task.run(() { scope((s) { c = channel.new()  s.go(() { c.send(1) })  task.run(() { c.receive() }) }) })` stops at `c.receive()` on any number of workers. Values that are not handles pass into and out of a nested run as captures and as its result, as with any function call.
>   - **Boundaries.** A nested run inside a generator body at the top (`collect(() { task.run(() { scope((s) { s.go(() { yield(1) }) }) }) })`) runs inside that boundary body, so by §9.4 rule 1 a `yield` from its tasks succeeds and pauses the whole nested run. That is the same on every host, because that run is not the entry run and uses one worker. A kept function that an outer task made (D4), called inside its own nested run, suspends the outer task with the nested run inside it, which is also the same everywhere. Track R documents both.
> - Host services given at an entry point are records of constant functions (static ids), so they are portable, and a stolen task uses the thief's own imports. The root environment's `Assert` closure must be a constant with a static id too (part 3.4); at c69515c it is not. §9.7 gets the rule that a host service behaves the same whichever worker calls it.
>
> All shared mutable scheduler state (deques, channel state, wake-ups, steal requests, cancel requests, the stop cell) is behind the one `Scheduler` service. Values of many types are kept there as `Keyed` values (D13). In the M:1 version it is written in i on `i/control`. The multicore wave replaces its implementation without changing the `i/task` API.

- New sentence(s):

> - `scheduler.detached` is a top-level record of constant functions that stop with `panic: task: no scheduler`. It is portable. Binding it before `keep_env` puts it, not the parent's scheduler, in the kept array, so the parent's scheduler boundary is never kept or captured. Inside, `! Scheduler = own` binds the real one, so `detached` is never called.
> - `s.spawn(start)` makes the task record (part 3.3), with the handle of the scope `s`.
> - The worker loop calls `start(own)` inside the task's new boundary (D5), with the `Scheduler` whose suspends go to that boundary. The environment statement is inside the task, in a frame that is kept, so it stays with the task's paused part (§9.4 rule 3).
> - Kept services may still be non-portable, for example a generator's `Yield` service, or the `Assert` of `assertion.catch`, which hold a boundary. Every service bound at the place is kept, also one that `body` does not need, so a task started under such a service is pinned by P1. That costs parallelism only, and it cannot be seen. When `body` uses that service, suspending to that boundary from the task stops with `control: boundary is not active` on any number of workers, because the task runs from the loop (D5), and the loop does not run inside that boundary's body.
> - **Runs and nested runs.** A **run** is one call of `task.run`: one loop, its tasks, and the handles they make. Nothing stops i code from calling `task.run` where a run is already going on, and the type of `task.run` cannot forbid it. So the rules are these, and they are the same on every host and every worker count:
>   - **Which run uses N workers.** Only the run at the program's entry (open question 4) may use more than one worker. Every other run is a **nested run**: a one-worker FIFO loop on the stack of the code that called `task.run`. Its tasks are never remotely claimed. When it is called from inside a task, it runs inside that task's boundary body, so the whole nested run is part of the calling task, which is started and never moves (P2).
>   - **The run service.** `task.run` needs one service of `i/task` (its name is R's; here `Runner`), and that is how it learns where it is, with no test for "is a service bound". Inside every task, the scheduler binds `Runner` to one that belongs to that task. The entry supplies the initial Runner from inferred needs (runtime D5); task.top(seed) uses that current Runner and replaces only its seed, retaining route and prefix. A run takes three things from `Runner`: its **path prefix**, its **root seed**, and its **wait**.
>   - **Paths.** At the top the prefix is `[]`. In a task, it is the task's path plus the index of this `task.run` among the task's `go` calls and runs, counted together, so the paths of two runs never meet, and two sibling tasks that each start a nested run get different paths and different generators (part 5).
>   - **Waits.** When a run has no task that can run but has outstanding host requests, it waits through `Runner`. At the top, that is the one root wait of host-concurrency §6.4 (status 1). In a task, the nested run hands its outstanding requests to the outer run and suspends **only its calling task** to the outer `Scheduler`, so the whole nested run pauses as one task, and the outer run's other tasks go on, on one worker as on N; the outer run resumes the calling task when one of those requests ends. So a nested run that waits for a host flag that another outer task sets ends on one worker too. A nested run never returns status 1 from the caller's stack.
>   - **Deadlock.** A nested run with no task that can run and no outstanding request of its own stops with `panic: task: deadlock`. No outer task can wake it, because outer handles cannot be used in it (next point).
>   - **Handles belong to their run.** Every channel, scope, Task, event and `Shared` handle holds the key of the run that made it. Using a handle in a task of another run (a send, a receive, a `sync`, a `go`, a join, a `get`) stops with `panic: task: handle of another run`, also when the handle is portable. So `task.run(() { scope((s) { c = channel.new()  s.go(() { c.send(1) })  task.run(() { c.receive() }) }) })` stops at `c.receive()` on any number of workers. Values that are not handles pass into and out of a nested run as captures and as its result, as with any function call.
>   - **Boundaries.** A nested run inside a generator body at the top (`collect(() { task.run(() { scope((s) { s.go(() { yield(1) }) }) }) })`) runs inside that boundary body, so by §9.4 rule 1 a `yield` from its tasks succeeds and pauses the whole nested run. That is the same on every host, because that run is not the entry run and uses one worker. A kept function that an outer task made (D4), called inside its own nested run, suspends the outer task with the nested run inside it, which is also the same everywhere. Track R documents both.
> - Host services given at an entry point are records of constant functions (static ids), so they are portable, and a remotely claimed task uses the remote claimant's own imports. The root environment's `Assert` closure must be a constant with a static id too (part 3.4); at c69515c it is not. §9.7 gets the rule that a host service behaves the same whichever worker calls it.
>
> All shared mutable scheduler state (the published queue, channel state, wake-ups, cancel requests and stop cell) is behind the one `Scheduler` service. Worker-local values of many types are kept as Keyed (D13); shared transport carries encoded bytes/stable ids, never another worker's GC Keyed objects (N8). In the M:1 version it is written in i on `i/control`. The multicore wave replaces its implementation without changing the `i/task` API.

- Reason: Close N9’s forged-Int paths with general declaration grants and no public raw-number injection; ordinary accessible service fakes stay legal. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Adopt N8’s one-lock backend while retaining the i scheduler, typed local storage and the accepted CML transaction semantics. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Make the implementation/spec handoff explicit: prerequisites now, packaging and MC mechanisms in their own stages. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C044 — docs/design/multicore.md, base lines 212–218

- Old sentence(s):

> - `task.path()` gives the task's path: a list of `go` indices from the root task. The path is portable data. It does not depend on time, worker or steals.
> - `task.path()` needs no suspend: the `Scheduler` that the loop gives a task holds the task's path as data, so reading it is a field read.
> - i has no mutable values (§11.1), so a per-task draw counter would have to live in the scheduler loop, and every draw would be a suspend to the scheduler; `i/random` would need `! Scheduler`, and code with no scheduler, such as `main = () { … }` with no `task.run`, a top-level value or a macro, could not draw. So Random is a **value**, passed explicitly: a generator is immutable data (a key plus a counter: SplitMix64 or a counter-based PRF); `random.int(g, 10)` gives a number and the next generator, and `random.split(g)` gives two independent generators. `i/random` does not need `i/task`.
> - `task.random()` gives the generator of the current task, a pure function of the root seed and `task.path()`. So a child's numbers do not depend on when or where it runs, and a `go` does not change the parent's generator. Calling `task.random()` twice in one task gives the same generator; code that needs more splits it. `go` itself does not know about Random.
> - **The root seed** is part of the run service (part 4). At the top, the program gives it: `task.top(seed)` takes a generator, made from a host entropy service when the program has one, or from a fixed seed (`random.from_seed(n)`), which needs no service. There is no hidden default and no entropy that `task.run` reaches by itself, so the type of `task.run` needs no host service, and a macro, which has no host services (`meta.md` §8), gives a fixed seed and gets the same expansion on every run. A nested run in a task takes the seed of its outer run, and its paths start with the calling task's path (part 4), so its generators differ from every other task's.
> - **Split and paths never meet.** A generator is a key and a **stream**: a list of steps, where a step is either a path step `P(i)` or a split step `S(i)`, and a path step is never equal to a split step. `task.random()` at path `[i1, …, in]` gives the stream `[P(i1), …, P(in)]`; `random.split(g)` gives the streams of `g` followed by `S(1)` and by `S(2)`; numbers come from a counter-based PRF of the key, the stream and the draw counter. So the first half of `random.split(task.random())` in the task at path `[]` is the stream `[S(1)]`, and the child at path `[1]` has `[P(1)]`: they differ. Any two different streams give independent numbers. The exact encoding is R's; this rule is not.
> - Channel, scope and task handles are made of a path-based id (the path of the task that made them and a counter in that task) and the keys that type their state (D13). So `==` on handles is "the same channel" and survives copies, and nothing in them depends on the worker except key ids, which only `==` sees.

- New sentence(s):

> - `task.path()` gives the task's path: a list of `go` indices from the root task. The path is portable data. It does not depend on time, worker or queue claims.
> - `task.path()` needs no suspend: the `Scheduler` that the loop gives a task holds the task's path as data, so reading it is a field read.
> - i has no mutable values (§11.1), so a per-task draw counter would have to live in the scheduler loop, and every draw would be a suspend to the scheduler; `i/random` would need `! Scheduler`, and code with no scheduler, such as `main = () { … }` with no `task.run`, a top-level value or a macro, could not draw. So Random is a **value**, passed explicitly: a generator is immutable data (the pinned SplitMix state and tagged derivation history of runtime §4); `random.int(g, 10)` gives a number and the next generator, and `random.split(g)` gives two derived generators under that published algorithm. `i/random` does not need `i/task`.
> - `task.random()` gives the generator of the current task, a pure function of the root seed and `task.path()`. So a child's numbers do not depend on when or where it runs, and a `go` does not change the parent's generator. Calling `task.random()` twice in one task gives the same generator; code that needs more splits it. `go` itself does not know about Random.
> - **The root seed** is part of the run service (part 4). At the top, the program gives it: `task.top(seed)` takes a generator, made from a host entropy service when the program has one, or from a fixed seed (`random.from_seed(n)`), which needs no service. The contextual entry default is random.from_seed(0) (runtime D5), and task.top(seed) replaces only that seed while preserving the Runner route/prefix. There is no automatic entropy draw, so the type of `task.run` needs no host service, and a macro, which has no host services (`meta.md` §8), gives a fixed seed and gets the same expansion on every run. A nested run in a task takes the seed of its outer run, and its paths start with the calling task's path (part 4), so its generators differ from every other task's.
> - **Split and paths never meet.** A generator is a key and a **stream**: a list of steps, where a step is either a path step `P(i)` or a split step `S(i)`, and a path step is never equal to a split step. `task.random()` at path `[i1, …, in]` gives the stream `[P(i1), …, P(in)]`; `random.split(g)` gives the streams of `g` followed by `S(1)` and by `S(2)`; numbers come from the pinned published SplitMix split tree in runtime.md §4, which replaces the earlier counter-based PRF sketch. So the first half of `random.split(task.random())` in the task at path `[]` is the stream `[S(1)]`, and the child at path `[1]` has `[P(1)]`: they differ. Different tagged derivation addresses remain distinct histories; no promise of disjoint outputs or perfect statistical independence is made. Runtime.md §4 fixes the exact algorithm and encoding.
> - Channel, scope and task handles are made of a path-based id (the path of the task that made them and a counter in that task) and the keys that type their state (D13). So `==` on handles is "the same channel" and survives copies, and nothing in them depends on the worker except key ids, which only `==` sees.

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics.

### C045 — docs/design/multicore.md, base lines 222–236

- Old sentence(s):

> - A scope returns only after all its children have ended, including children that are being unwound.
> - **Chains.** The **chain** of a scope `r` is `r` followed by the chain of the task that opened `r`, as it was when `r` was opened. The **chain** of a task, at a moment, is the scopes it has opened whose bodies it is still running (innermost first), followed by the chain of the scope it was started in (empty for a run's root task). A task's chain changes only when that task itself enters or leaves a scope body, so it is fixed by the task's own steps, not by time or placement. Every scope in a task's chain has not ended: the scopes it has opened are still running their bodies, the scope it was started in waits for it, and each scope in that scope's chain was in the opener's chain when it was opened, so by the same argument it cannot end before the opener leaves it. The scheduler keeps each task's chain as a list of scope ids, so the check is a list search.
> - **Who may `go`.** `s.go` may be called only by a task whose chain holds `s`. Otherwise it stops with `panic: task: go outside its scope`, whether or not `s` has ended. So a `go` never meets an ended scope. Examples that stop: `h = scope((s) { s })  h.go(() { 1 })` (after the body returns, `s` is no longer in the root task's chain); `scope((u) { scope((s) { u.go(() { task.yield()  s.go(() { 1 }) }) }) })` (the `u` child was started in `u`, whose chain does not hold `s`); `scope((outer) { c = channel.new()  outer.go(() { h = c.receive()  h.go(() { 1 }) })  scope((s1) { c.send(s1) }) })` (the same). Each stops on any number of workers. A scope handle is portable and may be sent anywhere, but only tasks inside the scope may use it to start tasks. A `go` on a scope that is cancelling makes an unstarted task that is dropped at once; whether a `go` comes before or after the cancel is part of the order of D1.
> - **Who may join.** A join of the Task handle of a child of `s` (and a `sync` of a join event made from it) may be done only by a task whose chain holds `s`. Otherwise it stops with `panic: task: join outside its scope`. Every such joiner is cancelled when `s` is cancelled (below), so no join ever waits for a task that was dropped or cancelled. If R later adds a way to cancel one task while its scope goes on, a join of that task must complete with a value its type shows (for example a `Cancelled` case); no join may be left waiting.
> - **Errors.** When a child fails, the scope cancels the other children. When all have ended, the scope gives the error of the failed task with the lowest path (D10) among the scope's body and its children; the body comes first. Later errors are dropped. Error types stay unions; there is no aggregate error type, so inference does not change. Every error given to a scope made by `scope` is portable (P3). For `scope((s) { s.go(() { s.go(() { fail.raise(A) })  fail.raise(C) }) })`, if both fail, the error is `C` (path `[1]` comes before `[1, 1]`), whichever failed first. Which children fail before the cancel reaches them depends on the order of D1: for `s.go(() { task.yield()  fail.raise(A) })` and `s.go(() { fail.raise(B) })`, one FIFO worker gives `B`, and N workers may give `A`. The `i/task` docs say so.
> - **Results.** `both`, `race` and `map_concurrently` are ordinary i code built on `scope`, so they are portable paths: each result and error of their children is checked when the child ends (P3), and `both(() { file.open("a") }, () { file.open("b") })`, where both results are host references, stops with `panic: task: value is not portable`. R may add local forms built on `local_scope` (`local_both`, `local_race`, `local_map_concurrently`), which do not check and whose children stay home (P4); without them, the program writes the local scope itself. `both` and `map_concurrently` give results in argument order. `race` gives the first result that commits; timing chooses, as with a CML `choose`. With one worker the FIFO scheduler makes it fixed, but that is not a promise for N workers.
> - **What a cancel of a scope reaches.** When a scope `s` is cancelled (a child failed, or a task whose chain holds `s` is itself cancelled), the cancel reaches every child of `s` **and the body of `s`**. The body is unwound back to the `scope` call: `scope` runs its body under its own boundary in the body's task, and the cancel suspends to that boundary and cancels the part up to it, so the body's final functions run and the body gives no value; `scope` then waits for its children and gives the error of D10. A cancelled task cancels every scope it is running a body of, so the cancel goes down the whole tree inside `s`. A cancelled body is not a failure; it does not take part in the choice of the error. So `scope((s) { s.go(() { fail.raise(A) })  t = s.go(() { 1 })  t.join() })` gives error `A` on the one-worker FIFO scheduler: child `[1]` fails, `t` is dropped, the body waiting in `t.join()` is unwound, and nothing waits forever.
> - **Cancellation** is cooperative and goes to the task's home worker. An unstarted task is dropped; it has no final functions. A started task is cancelled when the cancel reaches it: when it is paused (waiting or runnable) at that time, the loop does not give it the answer it waits for and cancels it as the next point says; when it is running, at its next suspend point. When the cancel reaches a task is a step in the order of D1: on one FIFO worker it reaches every paused task at once, and on N workers it reaches a task on another worker at that worker's next poll point. A task that is being unwound is not cancelled again, and its final functions run to their end, with the scopes they open, which that cancel does not reach.
> - **How a started task is cancelled.** The loop never calls `control.cancel(k)` on the task's continuation `k`: under §9.4 rule 4 the task's boundary `b` would stay not active while the final functions run (`$control_cancel` and `$cancel_run`, `emit/control.rs:270-300`, never set field 8 of the boundary), and a final function that uses `i/task` (a send, a receive, a join, a sleep, an async host call through the scheduler) would suspend to `b` and stop with `control: boundary is not active`. Instead the loop **resumes** `k` with a cancel answer. `b` is then active again. The `Scheduler` call inside the task that sees the answer suspends to the task's inner boundary (made by `task.unwind_inside`, part 4) or, for a scope body, to that scope's boundary, and its handler calls `control.cancel` on that inner part only. So the final functions run with `b` active, and they may suspend to `b`; `$cancel_run` already catches a capture through it and gives it the record `$cancel_reenter`, so the rest of the unwind resumes later. They get the environment of §9.4 rule 4 as part 13 amends it: the services kept by the task's kept function and the environment statements inside the task, not the loop's services. This is all ordinary i code on `i/control`.
> - **Stops.** A stop on any worker stops the run (D11). The worker writes the stop cell once with compare-and-swap; the others stop at their next poll point or epoch check, and their messages are dropped. No final functions run (§9.5).
> - **Deadlock.** When no task can run and no host request is outstanding on any worker, nothing can ever go on. The run then stops with `panic: task: deadlock`, on one worker as on N. On one worker the scheduler sees this directly: its queue is empty and its request table is empty. It must not return status 1 then, which host-concurrency.md H4 makes a fault of the module. On N workers it needs quiescence detection: a shared count of runnable and running tasks and of outstanding requests; the worker that sees it reach zero with every deque empty writes the stop cell. In a macro, which has no host services (`meta.md` §8), every wait with nothing to run is a deadlock.
> - **Idle.** A worker with nothing to run, while others still work or requests are outstanding, waits. By host-concurrency H5, a host answers a request only through `ilang_resume:…` after the instance has returned status 1 or 2, never while it is inside Wasm. So a worker that has **outstanding requests of its own** (its pinned tasks wait on them, P2) must never park inside Wasm: it would never see its answers, and a task that waits on a 10 ms sleep would hang until something else woke the worker. The rule:
>   - A worker with outstanding requests of its own idles only by returning status 1, on every host. Its host then waits for either an answer to one of its requests or a **wake** from another worker, whichever comes first. The wake is a host request that the multicore wave adds as an import of `ilang:host/v1` (not a service): it resolves when another worker writes the worker's wake word (`Atomics.waitAsync` in JS; in wasmtime, a wait on that word in the host thread, raced with the reactor). The worker keeps one wake request outstanding while it idles, so status 1 is never given with no outstanding request (H4).
>   - A worker with no outstanding request of its own may park inside Wasm with `memory.atomic.wait32`, except browser worker 0, which never blocks: it idles like a worker with requests, with only the wake request outstanding.
>   - A one-worker run never uses the wake request: on one worker, idle means status 1 with the run's own requests (C-D's "wait for any"), and with none it is a deadlock.

- New sentence(s):

> - A scope returns only after all its children have ended, including children that are being unwound.
> - **Chains.** The **chain** of a scope `r` is `r` followed by the chain of the task that opened `r`, as it was when `r` was opened. The **chain** of a task, at a moment, is the scopes it has opened whose bodies it is still running (innermost first), followed by the chain of the scope it was started in (empty for a run's root task). A task's chain changes only when that task itself enters or leaves a scope body, so it is fixed by the task's own steps, not by time or placement. Every scope in a task's chain has not ended: the scopes it has opened are still running their bodies, the scope it was started in waits for it, and each scope in that scope's chain was in the opener's chain when it was opened, so by the same argument it cannot end before the opener leaves it. The scheduler keeps each task's chain as a list of scope ids, so the check is a list search.
> - **Who may `go`.** `s.go` may be called only by a task whose chain holds `s`. Otherwise it stops with `panic: task: go outside its scope`, whether or not `s` has ended. So a `go` never meets an ended scope. Examples that stop: `h = scope((s) { s })  h.go(() { 1 })` (after the body returns, `s` is no longer in the root task's chain); `scope((u) { scope((s) { u.go(() { task.yield()  s.go(() { 1 }) }) }) })` (the `u` child was started in `u`, whose chain does not hold `s`); `scope((outer) { c = channel.new()  outer.go(() { h = c.receive()  h.go(() { 1 }) })  scope((s1) { c.send(s1) }) })` (the same). Each stops on any number of workers. A scope handle is portable and may be sent anywhere, but only tasks inside the scope may use it to start tasks. A `go` on a scope that is cancelling makes an unstarted task that is dropped at once; whether a `go` comes before or after the cancel is part of the order of D1.
> - **Who may join.** A join of the Task handle of a child of `s` (and a `sync` of a join event made from it) may be done only by a task whose chain holds `s`. Otherwise it stops with `panic: task: join outside its scope`. A joiner still cancellable by `s` is unwound before observing a dropped/cancelled outcome; a shielded joiner, including protected final work, stops with `panic: task: join of a cancelled task` immediately when drop/cancellation is accepted, without waiting for target cleanup (runtime R4a). If R later adds a way to cancel one task while its scope goes on, a join of that task must complete with a value its type shows (for example a `Cancelled` case); no join may be left waiting.
> - **Errors.** When a child fails, the scope cancels the other children. When all have ended, the scope gives the error of the failed task with the lowest path (D10) among the scope's body and its children; the body comes first. Later errors are dropped. Error types stay unions; there is no aggregate error type, so inference does not change. Every error given to a scope made by `scope` is portable (P3). For `scope((s) { s.go(() { s.go(() { fail.raise(A) })  fail.raise(C) }) })`, if both fail, the error is `C` (path `[1]` comes before `[1, 1]`), whichever failed first. Which children fail before the cancel reaches them depends on the order of D1: for `s.go(() { task.yield()  fail.raise(A) })` and `s.go(() { fail.raise(B) })`, one FIFO worker gives `B`, and N workers may give `A`. The `i/task` docs say so.
> - **Results.** `both`, `race` and `map_concurrently` are ordinary i code built on `scope`, so they are portable paths: each result and error of their children is checked when the child ends (P3), and a result containing a user host reference stops with `panic: task: value is not portable`. Standard File/Listener/Stream/Child values are portable nominal Int wrappers (native N9); returning them through these combinators succeeds. R may add local forms built on `local_scope` (`local_both`, `local_race`, `local_map_concurrently`), which do not check and whose children stay home (P4); without them, the program writes the local scope itself. `both` and `map_concurrently` give results in argument order. `race` gives the first result that commits; timing chooses, as with a CML `choose`. With one worker the FIFO scheduler makes it fixed, but that is not a promise for N workers.
> - **What a cancel of a scope reaches.** When a scope `s` is cancelled (a child failed, or a task whose chain holds `s` is itself cancelled), the cancel reaches every child of `s` **and the body of `s`**. The body is unwound back to the `scope` call: `scope` runs its body under its own boundary in the body's task, and the cancel suspends to that boundary and cancels the part up to it, so the body's final functions run and the body gives no value; `scope` then waits for its children and gives the error of D10. A cancelled task cancels every scope it is running a body of, so the cancel goes down the whole tree inside `s`. A cancelled body is not a failure; it does not take part in the choice of the error. So `scope((s) { s.go(() { fail.raise(A) })  t = s.go(() { 1 })  t.join() })` gives error `A` on the one-worker FIFO scheduler: child `[1]` fails, `t` is dropped, the body waiting in `t.join()` is unwound, and nothing waits forever.
> - **Cancellation** is cooperative and goes to the task's home worker. An unstarted task is dropped; it has no final functions. A started task is cancelled when the cancel reaches it: when it is paused (waiting or runnable) at that time, the loop does not give it the answer it waits for and cancels it as the next point says; when it is running, at its next suspend point. When the cancel reaches a task is a step in the order of D1: on one FIFO worker it reaches every paused task at once, and on N workers it reaches a task on another worker at that worker's next poll point. A task that is being unwound is not cancelled again, and its final functions run to their end, with the scopes they open, which that cancel does not reach.
> - **How a started task is cancelled.** The loop never calls `control.cancel(k)` on the task's continuation `k`: under §9.4 rule 4 the task's boundary `b` would stay not active while the final functions run (`$control_cancel` and `$cancel_run`, `emit/control.rs:270-300`, never set field 8 of the boundary), and a final function that uses `i/task` (a send, a receive, a join, a sleep, an async host call through the scheduler) would suspend to `b` and stop with `control: boundary is not active`. Instead the loop **resumes** `k` with a cancel answer. `b` is then active again. The `Scheduler` call inside the task that sees the answer suspends to the task's inner boundary (made by `task.unwind_inside`, part 4) or, for a scope body, to that scope's boundary, and its handler calls `control.cancel` on that inner part only. So the final functions run with `b` active, and they may suspend to `b`; `$cancel_run` already catches a capture through it and gives it the record `$cancel_reenter`, so the rest of the unwind resumes later. They get the environment of §9.4 rule 4 as part 13 amends it: the services kept by the task's kept function and the environment statements inside the task, not the loop's services. This is all ordinary i code on `i/control`.
> - **Stops.** A stop on any worker closes the run's start/claim/delivery gate and records one message under the lock. Stopping invokes no i finals. Every user import checks the stopped-context gate, so spinning native workers cannot start new host effects while reaping delays process exit. Native's independent controller tears down host work/resources before process exit; it does not join a spinning Wasm worker. A continuing host uses stop-only epoch traps in that run's interruption domain, wakes idle reactors and joins workers, then returns the first stop (native N12). Epochs do not interrupt synchronous host code; multicore hosts must supply stop-aware synchronous imports or choose one worker/process isolation. Child cleanup follows runtime §8.1, including reaping before native exit.
> - **Deadlock.** When no task can run and no host request is outstanding on any worker, nothing can ever go on. The run then stops with `panic: task: deadlock`, on one worker as on N. On one worker the scheduler sees this directly: its queue is empty and its request table is empty. It must not return status 1 then, which host-concurrency.md H4 makes a fault of the module. On N workers it needs quiescence detection: a shared count of runnable and running tasks and of outstanding requests; the worker that sees it reach zero with shared and local queues empty records the stop under the same lock. Queue claims, local running counts, registrations and completion publication must participate in that snapshot; idle workers alone do not prove deadlock. In a macro, which has no host services (`meta.md` §8), every wait with nothing to run is a deadlock.
> - **Idle.** A worker with nothing to run, while others still work or requests are outstanding, waits. By host-concurrency H5, a host answers a request only through `ilang_resume:…` after the instance has returned status 1 or 2, never while it is inside Wasm. So a worker that has **outstanding requests of its own** (its pinned tasks wait on them, P2) must never park inside Wasm: it would never see its answers, and a task that waits on a 10 ms sleep would hang until something else woke the worker. The rule:
>   - A worker with outstanding requests of its own idles only by returning status 1, on every host. Its host then waits for either an answer to one of its requests or a **wake** from another worker, whichever comes first. The wake is a request-mode ordinary host service added in the multicore wave (native N8): it resolves when another worker writes the worker's wake word (`Atomics.waitAsync` in JS; in wasmtime, a wait on that word in the host thread, raced with the reactor). The worker keeps one wake request outstanding while it idles, so status 1 is never given with no outstanding request (H4).
>   - A native worker with no requests may park in the host queue wait; a browser worker may use memory.atomic.wait32, except browser worker 0, which never blocks: it idles like a worker with requests, with only the wake request outstanding.
>   - A one-worker run never uses the wake request: on one worker, idle means status 1 with the run's own requests (C-D's "wait for any"), and with none it is a deadlock.

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Adopt N8’s one-lock backend while retaining the i scheduler, typed local storage and the accepted CML transaction semantics. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Adopt N1–N4 and the reviewed artifact/loading constraints, keeping explicit ilang/WASI and rejecting only required unsupplied imports. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C046 — docs/design/multicore.md, base lines 240–244

- Old sentence(s):

> - A channel is a handle as in part 5. `==` means "the same channel".
> - **Lifetime.** A channel **belongs to a scope**: the first scope in the chain of the task that made it (part 6), or the run itself when that chain is empty. Its state (buffers, waiting senders and receivers, sync records) lives until that scope ends and is then freed, in M:1 from the scheduler's `Keyed` table and in MC from the scope's region of shared memory, as for `scope.share`. A send, a receive or a `sync` on it after that scope has ended stops with `panic: task: channel is gone`, in every build. A task that is waiting on it when the scope ends gets the same stop. Whether a step comes before or after that end is part of the order of D1, as for `shared.get`. So `r = scope((s) { channel.new() })  r.send(1)` stops on any number of workers. *Reason:* i has no weak references and the shared memory has no GC, so "a channel lives while a handle to it is reachable" cannot be kept in M:1 or in MC; tying the state to a scope gives a bound that both can keep, and fixing it now means MC changes no program. The cost: a long-lived scope that makes a channel for each request keeps every one of them until it ends. The `i/task` docs say to make such channels inside a scope per request (`scope((r) { c = channel.new() … })`), which frees them when the request is done; `docs/performance.md` says so too. A local channel follows the same rule.
> - Events are values. `choose` and `map` are pure. `sync` is the only commit point, and exactly one event of a `choose` commits. `map` functions run in the syncing task after the commit, never in the partner task. Nack and guard are not in v1; they are kept for later.
> - **M:1 now.** Channel state lives in the scheduler state, as `Keyed` buffers and waiting continuations (D13). A commit is trivial on one thread.
> - **Multicore wave.** Channel state, sync records and message buffers live in shared memory. A commit uses the parallel CML protocol of Reppy, Russo and Xiao (ICFP 2009): each sync record has a state word (WAITING, CLAIMED, SYNCED) set by compare-and-swap, and claims are taken in one global order to avoid deadlock. A message is encoded by the sender's worker (I4) and decoded by the receiver. A hand-off on the same worker passes the reference.

- New sentence(s):

> - A channel is a handle as in part 5. `==` means "the same channel".
> - **Lifetime.** A channel **belongs to a scope**: the first scope in the chain of the task that made it (part 6), or the run itself when that chain is empty. Its state (buffers, waiting senders and receivers, sync records) lives until that scope ends and is then freed, in M:1 from the scheduler's `Keyed` table and in MC from the scope's host-transport region, as for `scope.share`. A send, a receive or a `sync` on it after that scope has ended stops with `panic: task: channel is gone`, in every build. A task that is waiting on it when the scope ends gets the same stop. Whether a step comes before or after that end is part of the order of D1, as for `shared.get`. So `r = scope((s) { channel.new() })  r.send(1)` stops on any number of workers. *Reason:* i has no weak references and the shared memory has no GC, so "a channel lives while a handle to it is reachable" cannot be kept in M:1 or in MC; tying the state to a scope gives a bound that both can keep, and fixing it now means MC changes no program. The cost: a long-lived scope that makes a channel for each request keeps every one of them until it ends. The `i/task` docs say to make such channels inside a scope per request (`scope((r) { c = channel.new() … })`), which frees them when the request is done; `docs/performance.md` says so too. A local channel follows the same rule.
> - Events are values. `choose` and `map` are pure. `sync` is the only commit point, and exactly one event of a `choose` commits. `map` functions run in the syncing task after the commit, never in the partner task. Nack and guard are not in v1; they are kept for later.
> - **M:1 now.** Channel state lives in the scheduler state, as `Keyed` buffers and waiting continuations (D13). A commit is trivial on one thread.
> - **Multicore wave: one lock (native N8).** Channel/scope state, registrations,
> queue claims, cancellation, Root/Owner membership and cross-worker request
> counts, and the stop cell live in the runner's transport
> behind one lock. A sync transaction validates all leaves (run, scope, chain),
> then registers one identity with source-ordered alternatives. Choose the first
> ready alternative and oldest eligible partner, never two leaves of the same
> registration. In the same critical section mark both registrations committed,
> remove every losing leaf, transfer one payload and queue both wakeups. A cancel
> before commit unregisters all leaves; after commit it cannot roll back a message.
> No i callback, encoder/decoder, continuation, OS wait or Wasm call runs under
> the lock. Prepare portable payload bytes on the sender before entering it; a
> local payload remains in its owner's heap and has only a worker-local token in
> the transaction. `map` callbacks run only after commit in their syncing tasks,
> including the same cancellation-priority rules as runtime §6.4. Run/channel/scope
> liveness must be rechecked after preparation and before commit. A winning local
> handoff uses its retained reference; a remote receiver decodes the bytes.
> - **Implementation gate.** Shared payloads/typed keys are encoded bytes and stable
> ids; continuations and Keyed i heap objects stay in worker-local typed tables.
> The i scheduler computes descriptions and applies returned decisions. The runner
> supplies general locked byte-state transactions and async wake/wait services,
> not a Rust task executor. A lock, queue and wait alone are not a complete typed
> interface: MC-D must type-check the operation signatures and demonstrate two
> worker rendezvous, cancellation and no callback under the lock before MC-I.
> All request-mode fields use ordinary service imports; no new v3 runtime import,
> status or host-called export is assumed. Changing that requires an ABI version
> review under host-concurrency.md. One worker retains runtime §6.4's FIFO commit.

- Reason: Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Adopt N8’s one-lock backend while retaining the i scheduler, typed local storage and the accepted CML transaction semantics. Adopt N11: native uses runner byte storage/private memories; only the browser transport needs shared Wasm memory. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Adopt N1–N4 and the reviewed artifact/loading constraints, keeping explicit ilang/WASI and rejecting only required unsupplied imports. Make the implementation/spec handoff explicit: prerequisites now, packaging and MC mechanisms in their own stages. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C047 — docs/design/multicore.md, base lines 248–252

- Old sentence(s):

> - `scope.share(value) -> Shared(A)` belongs to the scope that makes it. `value` must be portable (P3).
> - A `Shared(A)` handle is (scope id, index, key), so sending it costs O(1).
> - `shared.get(s)` gives the value. On N workers, the value is encoded once into a region of the scope in shared memory, and each worker decodes it once and caches it for the life of the scope. Values from two workers are `==` (content plus ids, I1).
> - A `get` after the scope has ended stops with `panic: task: shared value is gone`, in every build, also M:1, where the scheduler keeps the set of live shares. So the stop does not depend on placement. The region is freed when the scope ends.
> - Considered and rejected: turning a `Shared` into a full copy when it leaves its scope. Finding handles inside arbitrary values at scope exit needs a walk, and width subtyping and type variables hide them from the types.

- New sentence(s):

> - `scope.share(value) -> Shared(A)` belongs to the scope that makes it. `value` must be portable (P3).
> - A `Shared(A)` handle is (scope id, index, key), so sending it costs O(1).
> - `shared.get(s)` gives the value. On N workers, the value is encoded once into a scope-owned region of the host byte transport, and each worker decodes it once and caches it for the life of the scope. Values from two workers are `==` (content plus ids, I1).
> - A `get` after the scope has ended stops with `panic: task: shared value is gone`, in every build, also M:1, where the scheduler keeps the set of live shares. So the stop does not depend on placement. The region is freed when the scope ends.
> - Considered and rejected: turning a `Shared` into a full copy when it leaves its scope. Finding handles inside arbitrary values at scope exit needs a walk, and width subtyping and type variables hide them from the types.

- Reason: Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N11: native uses runner byte storage/private memories; only the browser transport needs shared Wasm memory. Adopt N1–N4 and the reviewed artifact/loading constraints, keeping explicit ilang/WASI and rejecting only required unsupplied imports.

### C048 — docs/design/multicore.md, base lines 277–282

- Old sentence(s):

> **wasmtime (multicore wave).** Turn on the `threads` feature and `Config::wasm_threads(true)`. The module imports `(memory … shared)` instead of defining one. One Engine and Module serve all workers. Each worker is one long-lived OS thread that owns one `Store` and one instance; this replaces the per-call thread. The GC heap size is per worker. `ilang_alloc` and `ilang_free` take chunks from a shared bump word with an atomic add; a buffer freed by another worker goes on its owner's remote-free list. `$heap`, `$free_head`, `$shadow`, `$sp` and `$root_env` stay per-instance globals. The bridge uses `SharedMemory`. Idle workers follow part 6: a worker with no outstanding request parks with `memory.atomic.wait32`; a worker with outstanding requests returns status 1, and its thread waits on its reactor and its wake word together, so its answers still come (H5). When the stop cell is set, the host makes the other Stores stop promptly (for example with epoch interruption, only in threaded runs). Rust host functions are `Send + Sync` and are linked into every Store.
>
> **Browsers (multicore wave).** N workers only when `crossOriginIsolated` is true and `SharedArrayBuffer` exists; otherwise one worker, with the same meaning (D12). The `WebAssembly.Module` and the shared memory are posted to Web Workers. Worker 0 runs on the thread that called the entry, which in a page is the main thread.
>
> - **Imports.** JS functions cannot be posted to a Web Worker, so the main-thread `imports` object of today cannot serve worker k > 0. The SDK gets a new option for multicore runs: an **imports module URL**, an ES module that each Web Worker imports to build its own imports object. A host function that needs the main thread (the DOM, for example) is written there as an async function that posts to the main thread and waits for the answer as a host request; it is never synchronous. A synchronous `@external` cannot be proxied: worker k would have to block with `Atomics.wait` until the main thread is free, while the main thread may itself be running worker 0's `ilang_start`. So a host that gives only main-thread functions, with no imports module URL, runs one worker. A host reference made on one thread stays there (H), so it never needs to cross as an `externref`.
> - **Worker 0 never blocks.** It never uses `Atomics.wait`. Its scheduler idles only by returning status 1 with an outstanding request (part 6, Idle), so it returns to the event loop.

- New sentence(s):

> **wasmtime (multicore wave).** One Store/instance per executing worker, with
> private GC and linear memory. Native bytes, queues, shares, channels and wakeups
> live in runner memory (native N11); no shared Wasm allocator or threads feature
> is required for that transport. Each worker has its local typed continuation
> and request tables. Host service state and the resource table are shared, with
> short locks and no callback under them. Outstanding requests idle via status 1
> and the general wake service, raced with reactor readiness. Continuing hosts use
> an entry-context-specific epoch interruption domain, shared by its nested scheduler runs; one run's stop cannot trap another
> run on a shared Engine. Native N12 fixes setup, re-entry gates and teardown.
>
> **Browsers (multicore wave).** N workers require crossOriginIsolated,
> SharedArrayBuffer, an imports module URL and a stop-safe dedicated-worker entry
> route (native N12); otherwise one worker, with the same meaning (D12). Post the
> module and shared memory to dedicated Web Workers for every executing instance,
> including worker 0. The calling/main thread is the host controller, which can
> terminate all workers without waiting for Wasm poll points and tear down shared
> host resources. The earlier worker-0-on-main route cannot stop a pure infinite
> loop there and must not enable multicore. Synchronous one-worker host exports
> retain runtime D5's callback contract; no universal async entries are introduced.
>
> - **Imports.** JS functions cannot be posted to a Web Worker, so the main-thread `imports` object of today cannot serve worker k > 0. The SDK gets a new option for multicore runs: an **imports module URL**, an ES module that each Web Worker imports to build its own imports object. A host function that needs the main thread (the DOM, for example) is written there as an async function that posts to the main thread and waits for the answer as a host request; it is never synchronous. A synchronous `@external` that requires main-thread execution is not proxied by this design; use an async service or one worker. All synchronous imports, host future polls and destructors must satisfy native N12's stop-aware host rule. So a host that gives only main-thread functions, with no imports module URL, runs one worker. A host reference made on one thread stays there (H), so it never needs to cross as an `externref`.
> - **Worker 0 never blocks.** It never uses `Atomics.wait`. Its scheduler idles only by returning status 1 with an outstanding request (part 6, Idle), so it returns to the event loop.

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N8’s one-lock backend while retaining the i scheduler, typed local storage and the accepted CML transaction semantics. Adopt N11: native uses runner byte storage/private memories; only the browser transport needs shared Wasm memory. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Adopt N1–N4 and the reviewed artifact/loading constraints, keeping explicit ilang/WASI and rejecting only required unsupplied imports. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C049 — docs/design/multicore.md, base lines 294–298

- Old sentence(s):

> 1. **Now (R):** M:1 scheduler in i, with every rule of this note. `core.portable`, `control.keep_env` and keys (D13) land.
> 2. **Wave 5 (MC):** N instances, one shared memory, private deques, cooperative steal of unstarted tasks with a sharing-keeping copy, `scope.share` regions, CAS-committed CML, ids in `$clo` field 2, quiescence detection, the imports module URL in JS.
> 3. **Later:** with shared-everything threads, immutable data gets shared types (`$obj`, `$values`, `$cons`), copies go away for it, ids allow direct sharing, and boundaries and continuations stay per thread.
> 4. **Later:** with Go-style per-frame resume, frame records that carry their point index could be encoded, so started tasks could move.
> 5. **Later: moving host references.** By H, a host reference never leaves its worker, so a task that holds one runs where the reference was made. The plan's own end-to-end target (E2E-echo, `i/net` with `i/task`) then uses one core: worker 0 accepts each socket and hands it to a handler in a local scope, so every handler runs on worker 0 and the other workers sit idle, unless the handlers send their pure work to children of a portable scope. A later, compatible way out is a **host-mediated transfer**: a value `transfer(r)` that the host takes back from the sending instance and gives to the receiving instance as a new host reference (in wasmtime, `HostReference` is already `Arc<dyn Any + Send + Sync>`, `lib.rs:349`; in browsers only for transferable objects). It would be an explicit operation with its own `==` rule (the new reference is a different value), so it does not break H or §11.4. It is not designed here. `docs/performance.md` records the one-core limit of host-reference-heavy servers until then.

- New sentence(s):

> 1. **Now (R):** M:1 scheduler in i, with every rule of this note. `core.portable`, `control.keep_env` and keys (D13) land.
> 2. **Wave 5 (MC):** N instances, native runner bytes or browser shared memory, one queue encoded at go, silent local pinning, sharing-keeping copies, scope.share regions, one-lock CML, ids in `$clo` field 2, quiescence detection, the imports module URL in JS.
> 3. **Later:** with shared-everything threads, immutable data gets shared types (`$obj`, `$values`, `$cons`), copies go away for it, ids allow direct sharing, and boundaries and continuations stay per thread.
> 4. **Later:** with Go-style per-frame resume, frame records that carry their point index could be encoded, so started tasks could move.
> 5. **IO values now; arbitrary host references later.** Standard IO resources
> are portable private Int wrappers (native N9, runtime §7/§8). They need no host
> reference transfer; the portable-scope echo server in runtime §9 can use more
> cores. User-defined opaque host references still obey H and pin their holders.
> Any future explicit transfer of those references must define its own equality
> and ownership rules; it is not needed for standard sockets.

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Adopt N8’s one-lock backend while retaining the i scheduler, typed local storage and the accepted CML transaction semantics. Adopt N11: native uses runner byte storage/private memories; only the browser transport needs shared Wasm memory. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Adopt N1–N4 and the reviewed artifact/loading constraints, keeping explicit ilang/WASI and rejecting only required unsupplied imports.

### C050 — docs/design/multicore.md, base lines 302–308

- Old sentence(s):

> - shared-everything threads (shared structs and arrays, thread-local globals, `thread.spawn`); wasmtime support for shared structs;
> - stack switching (WasmFX), for cheaper continuations;
> - JSPI, for browser host calls;
> - component-model async and WASI 0.3 (wasi-threads is deprecated, so the WASI target stays single-worker);
> - `Atomics.waitAsync`, for worker 0 on the main thread;
> - memory-control and custom page sizes, to give `scope.share` memory back;
> - COOP/COEP `credentialless`, which makes isolation easier to deploy.

- New sentence(s):

> - shared-everything threads (shared structs and arrays, thread-local globals, `thread.spawn`); wasmtime support for shared structs;
> - stack switching (WasmFX), for cheaper continuations;
> - JSPI, for browser host calls;
> - component-model async and WASI 0.3 (wasi-threads is deprecated, so the WASI target stays single-worker);
> - `Atomics.waitAsync`, for nonblocking browser wake delivery;
> - memory-control and custom page sizes, to give `scope.share` memory back;
> - COOP/COEP `credentialless`, which makes isolation easier to deploy.

- Reason: Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N1–N4 and the reviewed artifact/loading constraints, keeping explicit ilang/WASI and rejecting only required unsupplied imports. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C051 — docs/design/multicore.md, base lines 419–419

- Old sentence(s):

> *(multicore wave)* The module imports its memory as a shared memory and exports `ilang_worker(index: i32)` and `ilang_toplevel_portable`; `ilang_start` is called on every worker.

- New sentence(s):

> *(multicore wave)* Browser modules import shared memory; native keeps private linear memory. Worker entry and the top-level portability probe are MC-D interfaces; `ilang_start` still runs on every worker. The earlier `ilang_worker(index: i32)`/`ilang_toplevel_portable` export sketch is not an addition to ABI v3: new runtime exports require a reviewed version bump (native N3). Prefer ordinary configured host exports/services if they suffice; settle this before MC-I.

- Reason: Adopt N11: native uses runner byte storage/private memories; only the browser transport needs shared Wasm memory. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C052 — docs/design/multicore.md, base lines 433–446

- Old sentence(s):

> - `docs/spec.md`: §2.5, §4.2, §4.5, §9.2, §9.4 (rules 3, 4 and 7), §9.7, new §9.8 (`i/key`, `Key`, `Keyed`), §10.1, §11.1, §11.4, §11.7, §11.8, new §11.9, §12, §13, §B.1, §B.4 (part 13).
> - `docs/design/meta.md` §8: one sentence that a `Syntax` value is not portable (§11.9), whatever its form.
> - `lib/i/core.i`: export `portable` as an `@external("ilang:core", …)` declaration, like `not` and `hash`.
> - `lib/i/key.i` (new): `new`, `wrap` and `unwrap` as `@external("ilang:core", …)` declarations.
> - `lib/i/control.i`: `keep_env`.
> - `crates/ilang-wasm/src/lower.rs`: add `portable` and the key functions to the `CORE` and `CORE_FUNCS` name tables near `:120` and `:130`; the `keep_env` op, which keeps the `$env` array as it is (part 4); bit 0 of field 1 for closures with patched captures (part 3.6). `captures_of` (`:389`) is the definition of captures; it does not change, and a comment points at §11.9.
> - `crates/ilang-wasm/src/emit/mod.rs`: `closure()` writes the patch flag in field 1 and gives a patched closure's captures array its last element, the walk stamp (part 3.6). The `$clo{n}` types do not change.
> - `crates/ilang-wasm/src/emit/control.rs`: `$keep_reenter` and the frame record of a kept function (part 4).
> - The checker's built-in types for `portable`, `keep_env` and keys (`crates/ilang-check`): `BuiltinType::Key` and `BuiltinType::Keyed`, and a `Key: [Inv]` row in `builtin_variance` (`crates/ilang-check/src/types/solve.rs:1285-1292`). Without that row the argument of `Key` is never compared, and `unwrap` is an unchecked cast.
> - `crates/ilang-wasm/src/runtime/values.rs`: `$core_portable`, a walk with an explicit stack like `$value_compare_i32`, with the walk stamp of part 3.6 (one new token for each walk). It gives False at tags 10, 11 and 12 and at a host request, and walks field 5 of `$obj` (tuples, custom values, closure captures, records), `$cons` heads and tails, `Keyed` values, and kept services. The key functions.
> - `crates/ilang-wasm/src/runtime/mod.rs`: the tag table (`:46`) gains a "portable" note, the patch flag, the key tag, and the planned `$clo` layout of part 3.4.
> - `crates/ilang-check/tests/library.rs`: the core and control export lists.
> - `lib/i/task.i`, `lib/i/random.i`: following part 18.
> - `docs/performance.md`: channel state kept until the channel's scope ends (part 7), the cost of `portable` checks (also on DAGs, part 3.6), pinned tasks under boundary services (part 4), the start-up conditions of part 3.5, and the one-core limit of servers whose handlers hold host references (part 12).

- New sentence(s):

> - `docs/spec.md`: §2.5, §4.2, §4.5, §9.2, §9.4 (rules 3, 4 and 7), §9.7, new §9.8 (`i/key`, `Key`, `Keyed`), §10.1, §11.1, §11.4, §11.7, §11.8, new §11.9, §12, §13, §B.1, §B.4 (part 13).
> - `docs/design/meta.md` §8: one sentence that a `Syntax` value is not portable (§11.9), whatever its form.
> - `lib/i/core.i`: export `portable` as an `@external("ilang:core", …)` declaration, like `not` and `hash`.
> - `lib/i/key.i` (new): `new`, `wrap` and `unwrap` as `@external("ilang:core", …)` declarations.
> - `lib/i/control.i`: `keep_env`.
> - `crates/ilang-wasm/src/lower.rs`: add `portable` and the key functions to the `CORE` and `CORE_FUNCS` name tables near `:120` and `:130`; the `keep_env` op, which keeps the `$env` array as it is (part 4); bit 0 of field 1 for closures with patched captures (part 3.6). `captures_of` (`:389`) is the definition of captures; it does not change, and a comment points at §11.9.
> - `crates/ilang-wasm/src/emit/mod.rs`: `closure()` writes the patch flag in field 1 and gives a patched closure's captures array its last element, the walk stamp (part 3.6). The `$clo{n}` types do not change.
> - `crates/ilang-wasm/src/emit/control.rs`: `$keep_reenter` and the frame record of a kept function (part 4).
> - The checker's built-in types for `portable`, `keep_env` and keys (`crates/ilang-check`): `BuiltinType::Key` and `BuiltinType::Keyed`, and a `Key: [Inv]` row in `builtin_variance` (`crates/ilang-check/src/types/solve.rs:1285-1292`). Without that row the argument of `Key` is never compared, and `unwrap` is an unchecked cast.
> - `crates/ilang-wasm/src/runtime/values.rs`: `$core_portable`, a walk with an explicit stack like `$value_compare_i32`, with the walk stamp of part 3.6 (one new token for each walk). It gives False at tags 10, 11 and 12 and at a host request, and walks field 5 of `$obj` (tuples, custom values, closure captures, records), `$cons` heads and tails, `Keyed` values, and kept services. The key functions.
> - `crates/ilang-wasm/src/runtime/mod.rs`: the tag table (`:46`) gains a "portable" note, the patch flag, the key tag, and the planned `$clo` layout of part 3.4.
> - `crates/ilang-check/tests/library.rs`: the core and control export lists.
> - `lib/i/task.i`, `lib/i/random.i`: following part 18.
> - `docs/performance.md`: channel state kept until the channel's scope ends (part 7), the cost of `portable` checks (also on DAGs, part 3.6), pinned tasks under boundary services (part 4), the start-up conditions of part 3.5, and the distinction between portable IO wrappers and pinned arbitrary host references (part 12).

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Adopt N8’s one-lock backend while retaining the i scheduler, typed local storage and the accepted CML transaction semantics. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C053 — docs/design/multicore.md, base lines 452–458

- Old sentence(s):

> - `crates/ilang-wasm/src/emit/mod.rs`: `closure()` with flags and arity in field 1 and id in field 2; static ids, also for the root `Assert` closure, which becomes a `$c` constant; the `$funcs` table, with `$entry_assert_fail`; the id counter and worker globals.
> - `crates/ilang-wasm/src/runtime/values/inspect.rs:262`: arity from field 1.
> - `crates/ilang-wasm/src/runtime/values.rs`: id compare for tag 7 and keys.
> - New runtime modules for the copy, request cells, the stop cell, quiescence, share regions and `ilang_toplevel_portable`; the shared-memory allocator in `runtime/mod.rs`; the walk mark of D9 if MC picks it.
> - `crates/ilang-host`: the `threads` feature, `SharedMemory`, long-lived worker threads, per-worker GC heap, the stop path, the start-up conditions of part 3.5.
> - `bindings/javascript/ilang.mjs`: the isolation check, Workers, the imports module URL, worker 0 rules, the wake request.
> - `tests/browser`: COOP/COEP headers and a fallback test.

- New sentence(s):

> - `crates/ilang-wasm/src/emit/mod.rs`: `closure()` with flags and arity in field 1 and id in field 2; static ids, also for the root `Assert` closure, which becomes a `$c` constant; the `$funcs` table, with `$entry_assert_fail`; the id counter and worker globals.
> - `crates/ilang-wasm/src/runtime/values/inspect.rs:262`: arity from field 1.
> - `crates/ilang-wasm/src/runtime/values.rs`: id compare for tag 7 and keys.
> - New runtime modules for copy/identity and the top-level portability probe; worker-local request/continuation tables; shared byte-state transactions, queue ownership, stop/quiescence and share storage. Only browser transport needs the shared-memory allocator in runtime/mod.rs; add the walk mark if MC selects it. Settle worker entry/ABI version at MC-D.
> - `crates/ilang-host`: runner-owned byte transport and one-lock transactions, long-lived worker threads and private memories/heaps, resource contexts, wake/reactor integration, N12 stop domains/controller and the startup conditions of part 3.5. Native transport needs neither SharedMemory nor the Wasm threads feature.
> - `bindings/javascript/ilang.mjs`: the isolation check, Workers, the imports module URL, worker 0 rules, the wake request.
> - `tests/browser`: COOP/COEP headers and a fallback test.

- Reason: Close N9’s forged-Int paths with general declaration grants and no public raw-number injection; ordinary accessible service fakes stay legal. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N8’s one-lock backend while retaining the i scheduler, typed local storage and the accepted CML transaction semantics. Adopt N11: native uses runner byte storage/private memories; only the browser transport needs shared Wasm memory. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C054 — docs/design/multicore.md, base lines 464–481

- Old sentence(s):

> - **T1** `tests/spec/13/portable.i`. True for: an Int, a big Int, a Float, a String, a bit array, `[]`, a tuple and a list of these, a top-level function, a constructor, a closure that captures only data, a closure that captures another such closure, a local recursive function (it holds itself), and two local functions that call each other. False for: a boundary `b` (inside `control.boundary`), a continuation (inside `suspend`), a closure that captures `b`, a local recursive function that also captures `b`, a list holding such a closure, a custom value with a boundary field. After records land: `{ k: b }` is False.
> - **T2** `tests/spec/9.7`, with a host fixture as in `tests/spec/11/equality_externals.i`: `portable` of a host reference is False, and of a tuple that holds one too.
> - **T3** `tests/spec/11/equality.i` stays unchanged; it guards that §11.4 did not change.
> - **T4** Captures are exact: the example of part 3.1 is True; a closure whose only use of `b` is in a clause that never runs is False. Both run at O0 and O2 and give the same answers.
> - **T5** `keep_env`: a kept function called under a different environment statement still sees the kept service, also after it suspends and is resumed under another environment (`$keep_reenter`). A kept function made where a non-portable service is bound is not portable, also when its argument does not need that service, and also when `keep_env` is called inside a generic wrapper; the answer is the same at O0 and O2. A kept function made after that service is bound again to a portable value is portable.
> - **T6** A runtime unit test runs `$core_portable` on every tag, on a list of 10^6 elements (no Wasm stack overflow), on a closure that holds itself (it ends), on a list of 10^5 local recursive closures (time linear in the size; a quadratic walk fails the time limit), and on a marked closure walked twice (the second walk does not skip it because of the first walk's stamp: a closure that holds a boundary is False both times).
> - **T7** An emit test pins today's `$clo` layout (field 1 is the patch flag, field 2 is the arity, field 3 is 0) and names the planned layout in its message. A unit test pins the list of readers of `$obj` field 3 of part 3.4: `$float_get` and the readers behind a tag-2 test.
> - **T8** Track R: a send of a host reference on `channel.new` stops with `panic: task: value is not portable` on one worker; the same send on a local channel works; a child giving back a host reference stops in a portable scope and works in a local scope; a send of the Task handle of a child of a local scope on `channel.new` stops with `panic: task: value is not portable`, and the same handle sent on a local channel can be joined; `portable` of that Task handle, and of a join event made from it, is False; a `get` after the scope ended stops with `panic: task: shared value is gone`; `r = scope((s) { channel.new() })  r.send(1)` stops with `panic: task: channel is gone`; `channel.send_event(c, host_ref)` with `c = channel.new()` stops when the event is made, also when it would sit in a `choose` whose other branch commits; `both(() { file.open("a") }, () { file.open("b") })` stops, and `map_concurrently` over children that give host references stops.
> - **T9** Track R: with a fixed seed, `task.random()` gives the same generator in each task under two scheduling orders (a test scheduler service picks the order); `random.int` with `random.from_seed` works in a program with no `task.run`, in a top-level value and in a macro; a macro that runs `task.run` under `task.top(random.from_seed(0))` gives the same expansion twice; the first half of `random.split(task.random())` in the root task and the generator of the child at path `[1]` give different numbers; two sibling tasks that each start a nested run and call `task.random()` in its root task get different numbers.
> - **T10** Track R: a scope returns only after every child ended; final functions of cancelled children run once; with children at go indices 1 and 3 that both fail before any other suspend point, and the test scheduler set to run 3 first and to deliver the scope's cancel only after both have failed (an order that N workers allow: the cancel reaches a task on another worker at its next poll point), the error is that of index 1; with the same order but the cancel delivered at once (as one FIFO worker does), index 1 is dropped unstarted and the error is that of index 3 (D10 fixes the error only when the same children fail); in `scope((s) { s.go(() { s.go(() { fail.raise(A) })  fail.raise(C) }) })`, with the test scheduler set so that both fail before the cancel reaches either, the error is `C` whichever fails first; `both` and `map_concurrently` give argument order.
> - **T11** Track R: a task started inside a generator body, under the entry's scheduler loop, keeps the generator's `Yield` service; when it yields, it stops with `control: boundary is not active`. A nested run inside a generator body (`collect(() { task.run(…) })`, part 4) yields with the meaning of §9.4, the same on every host.
> - **T25** Track R, chains (part 6): `h = scope((s) { s })  h.go(() { 1 })`, `scope((u) { scope((s) { u.go(() { task.yield()  s.go(() { 1 }) }) }) })` and `scope((outer) { c = channel.new()  outer.go(() { h = c.receive()  h.go(() { 1 }) })  scope((s1) { c.send(s1) }) })` each stop with `panic: task: go outside its scope`, under every order the test scheduler picks; a child of `s` that calls `s.go`, and a grandchild in a scope opened inside a child of `s` that calls `s.go`, work; a join of a child of `s` from a task started in an outer scope stops with `panic: task: join outside its scope`.
> - **T26** Track R, cancels (part 6): `scope((s) { s.go(() { fail.raise(A) })  t = s.go(() { 1 })  t.join() })` gives error `A` on the FIFO scheduler and never `task: deadlock`, and the body's final functions run; `scope((s) { never = channel.new()  s.go(() { resource.ensure(() { never.receive() }, () { task.yield()  scope((f) { c = channel.new()  f.go(() { c.send(1) })  c.receive() }) }) })  s.go(() { fail.raise(E) }) })` gives error `E`, not `control: boundary is not active`: the final function of the cancelled child yields, starts a scope, sends, receives and waits for its child; the same with a fake `Clock` and a sleep in the final function; a second cancel of a task that is unwinding does nothing.
> - **T27** Track R, nested runs (part 4): with a host fixture that has an async `wait_flag()` and a sync `set_flag()`, `task.run(() { scope((s) { s.go(() { task.yield()  host.set_flag() })  task.run(() { host.wait_flag() }) }) })` ends on one worker; `task.run(() { scope((s) { c = channel.new()  s.go(() { c.send(1) })  task.run(() { c.receive() }) }) })` stops with `panic: task: handle of another run`; a nested run with nothing to run and no request stops with `panic: task: deadlock`; the paths in a nested run start with the calling task's path.
> - **T24** Track R: after a task suspends and is resumed by the loop, and in the final functions of a cancelled task, a service bound before `go` (a fake `Clock`) is still the kept one (§9.4 rules 3 and 4 as amended). `s.go(() { task.yield()  task.yield()  c.send(1) })` works: the second and third scheduler calls after a resume reach the task's own `Scheduler`, not `scheduler.detached` (part 4, the call that is not a tail call), at O0 and O2, and a WAT check at O2 finds no tail call after `! Scheduler = own`. A kept function made inside a task with `control.keep_env` that sends on a channel works when its task calls it and stops with `control: boundary is not active` when another task calls it; the same callback made with `task.keep` works from both.
> - **T12** Track R: `i/task` in a macro program gives the same expansion twice. In a test under `i test`, `map_concurrently` over children that give `Syntax` values stops with `panic: task: value is not portable`, and the same work in a local scope works.
> - **T20** Track R: `scope((s) { c = channel.new()  s.go(() { c.receive() }) })` stops with `panic: task: deadlock`, and the module never returns status 1 with no request outstanding. The same in a macro program.
> - **T21** Track R: keys: `unwrap` with the key that made a `Keyed` gives `Found`, with another key `Missing`; `k = key.new()` is not generalized: after `a = key.wrap(k, 1)` and `b = key.wrap(k, "x")`, `key.unwrap(k, b) |> { Found(n) -> n + 1  Missing -> 0 }` is a compile error (the found value is `Int | String`), and a control line with a second key `k2 = key.new()` used only at `Int` (`key.unwrap(k2, key.wrap(k2, 1)) |> { Found(n) -> n + 1  Missing -> 0 }`) compiles and gives 2, so the error comes from the type check, not from the parser; `Key` is invariant: a `Key(Int)` given where a `Key(Int | String)` is wanted is a compile error, and a checker unit test reads the `Key: [Inv]` row; a `Keyed` is portable when its value is; `g = (f) { control.keep_env(f) }` gives `g(h) == g(h)` False.

- New sentence(s):

> - **T1** `tests/spec/13/portable.i`. True for: an Int, a big Int, a Float, a String, a bit array, `[]`, a tuple and a list of these, a top-level function, a constructor, a closure that captures only data, a closure that captures another such closure, a local recursive function (it holds itself), and two local functions that call each other. False for: a boundary `b` (inside `control.boundary`), a continuation (inside `suspend`), a closure that captures `b`, a local recursive function that also captures `b`, a list holding such a closure, a custom value with a boundary field. After records land: `{ k: b }` is False.
> - **T2** `tests/spec/9.7`, with a host fixture as in `tests/spec/11/equality_externals.i`: `portable` of a host reference is False, and of a tuple that holds one too.
> - **T3** `tests/spec/11/equality.i` stays unchanged; it guards that §11.4 did not change.
> - **T4** Captures are exact: the example of part 3.1 is True; a closure whose only use of `b` is in a clause that never runs is False. Both run at O0 and O2 and give the same answers.
> - **T5** `keep_env`: a kept function called under a different environment statement still sees the kept service, also after it suspends and is resumed under another environment (`$keep_reenter`). A kept function made where a non-portable service is bound is not portable, also when its argument does not need that service, and also when `keep_env` is called inside a generic wrapper; the answer is the same at O0 and O2. A kept function made after that service is bound again to a portable value is portable.
> - **T6** A runtime unit test runs `$core_portable` on every tag, on a list of 10^6 elements (no Wasm stack overflow), on a closure that holds itself (it ends), on a list of 10^5 local recursive closures (time linear in the size; a quadratic walk fails the time limit), and on a marked closure walked twice (the second walk does not skip it because of the first walk's stamp: a closure that holds a boundary is False both times).
> - **T7** An emit test pins today's `$clo` layout (field 1 is the patch flag, field 2 is the arity, field 3 is 0) and names the planned layout in its message. A unit test pins the list of readers of `$obj` field 3 of part 3.4: `$float_get` and the readers behind a tag-2 test.
> - **T8** Track R: a send of a host reference on `channel.new` stops with `panic: task: value is not portable` on one worker; the same send on a local channel works; a child giving back a host reference stops in a portable scope and works in a local scope; a send of the Task handle of a child of a local scope on `channel.new` stops with `panic: task: value is not portable`, and the same handle sent on a local channel can be joined; `portable` of that Task handle, and of a join event made from it, is False; a `get` after the scope ended stops with `panic: task: shared value is gone`; `r = scope((s) { channel.new() })  r.send(1)` stops with `panic: task: channel is gone`; `channel.send_event(c, host_ref)` with `c = channel.new()` stops when the event is made, also when it would sit in a `choose` whose other branch commits; both returning user opaque host references stops, while both returning standard File values succeeds, and `map_concurrently` over children that give host references stops.
> - **T9** Track R: with a fixed seed, `task.random()` gives the same generator in each task under two scheduling orders (a test scheduler service picks the order); `random.int` with `random.from_seed` works in a program with no `task.run`, in a top-level value and in a macro; a macro that runs `task.run` under `task.top(random.from_seed(0))` gives the same expansion twice; the first half of `random.split(task.random())` in the root task and the generator of the child at path `[1]` give different numbers; two sibling tasks that each start a nested run and call `task.random()` in its root task get different numbers.
> - **T10** Track R: a scope returns only after every child ended; final functions of cancelled children run once; with children at go indices 1 and 3 that both fail before any other suspend point, and the test scheduler set to run 3 first and to deliver the scope's cancel only after both have failed (an order that N workers allow: the cancel reaches a task on another worker at its next poll point), the error is that of index 1; with the same order but the cancel delivered at once (as one FIFO worker does), index 1 is dropped unstarted and the error is that of index 3 (D10 fixes the error only when the same children fail); in `scope((s) { s.go(() { s.go(() { fail.raise(A) })  fail.raise(C) }) })`, with the test scheduler set so that both fail before the cancel reaches either, the error is `C` whichever fails first; `both` and `map_concurrently` give argument order.
> - **T11** Track R: a task started inside a generator body, under the entry's scheduler loop, keeps the generator's `Yield` service; when it yields, it stops with `control: boundary is not active`. A nested run inside a generator body (`collect(() { task.run(…) })`, part 4) yields with the meaning of §9.4, the same on every host.
> - **T25** Track R, chains (part 6): `h = scope((s) { s })  h.go(() { 1 })`, `scope((u) { scope((s) { u.go(() { task.yield()  s.go(() { 1 }) }) }) })` and `scope((outer) { c = channel.new()  outer.go(() { h = c.receive()  h.go(() { 1 }) })  scope((s1) { c.send(s1) }) })` each stop with `panic: task: go outside its scope`, under every order the test scheduler picks; a child of `s` that calls `s.go`, and a grandchild in a scope opened inside a child of `s` that calls `s.go`, work; a join of a child of `s` from a task started in an outer scope stops with `panic: task: join outside its scope`.
> - **T26** Track R, cancels (part 6): `scope((s) { s.go(() { fail.raise(A) })  t = s.go(() { 1 })  t.join() })` gives error `A` on the FIFO scheduler and never `task: deadlock`, and the body's final functions run; `scope((s) { never = channel.new()  s.go(() { resource.ensure(() { never.receive() }, () { task.yield()  scope((f) { c = channel.new()  f.go(() { c.send(1) })  c.receive() }) }) })  s.go(() { fail.raise(E) }) })` gives error `E`, not `control: boundary is not active`: the final function of the cancelled child yields, starts a scope, sends, receives and waits for its child; the same with a fake `Clock` and a sleep in the final function; a second cancel of a task that is unwinding does nothing.
> - **T27** Track R, nested runs (part 4): with a host fixture that has an async `wait_flag()` and a sync `set_flag()`, `task.run(() { scope((s) { s.go(() { task.yield()  host.set_flag() })  task.run(() { host.wait_flag() }) }) })` ends on one worker; `task.run(() { scope((s) { c = channel.new()  s.go(() { c.send(1) })  task.run(() { c.receive() }) }) })` stops with `panic: task: handle of another run`; a nested run with nothing to run and no request stops with `panic: task: deadlock`; the paths in a nested run start with the calling task's path.
> - **T24** Track R: after a task suspends and is resumed by the loop, and in the final functions of a cancelled task, a service bound before `go` (a fake `Clock`) is still the kept one (§9.4 rules 3 and 4 as amended). `s.go(() { task.yield()  task.yield()  c.send(1) })` works: the second and third scheduler calls after a resume reach the task's own `Scheduler`, not `scheduler.detached` (part 4, the call that is not a tail call), at O0 and O2, and a WAT check at O2 finds no tail call after `! Scheduler = own`. A kept function made inside a task with `control.keep_env` that sends on a channel works when its task calls it and stops with `control: boundary is not active` when another task calls it; the same callback made with `task.keep` works from both.
> - **T12** Track R: `i/task` in a macro program gives the same expansion twice. In a test under `i test`, `map_concurrently` over children that give `Syntax` values stops with `panic: task: value is not portable`, and the same work in a local scope works.
> - **T20** Track R: `scope((s) { c = channel.new()  s.go(() { c.receive() }) })` stops with `panic: task: deadlock`, and the module never returns status 1 with no request outstanding. The same in a macro program.
> - **T21** Track R: keys: `unwrap` with the key that made a `Keyed` gives `Found`, with another key `Missing`; `k = key.new()` is not generalized: after `a = key.wrap(k, 1)` and `b = key.wrap(k, "x")`, `key.unwrap(k, b) |> { Found(n) -> n + 1  Missing -> 0 }` is a compile error (the found value is `Int | String`), and a control line with a second key `k2 = key.new()` used only at `Int` (`key.unwrap(k2, key.wrap(k2, 1)) |> { Found(n) -> n + 1  Missing -> 0 }`) compiles and gives 2, so the error comes from the type check, not from the parser; `Key` is invariant: a `Key(Int)` given where a `Key(Int | String)` is wanted is a compile error, and a checker unit test reads the `Key: [Inv]` row; a `Keyed` is portable when its value is; `g = (f) { control.keep_env(f) }` gives `g(h) == g(h)` False.

- Reason: Close N9’s forged-Int paths with general declaration grants and no public raw-number injection; ordinary accessible service fakes stay legal. Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C055 — docs/design/multicore.md, base lines 485–493

- Old sentence(s):

> - **T13** Differential: every `tests/spec` case and every std test runs single-core, threaded with 1 worker, and threaded with 4 workers (the last 200 times). Results, stdout and stop messages must be the same, except in tests marked as order-dependent (D1). A marked test lists every outcome it allows: a set of (stdout, stop message) pairs, or, when the set is large, a checker in i that is given the output and the stop and says whether some order of steps gives them.
> - **T14** Copy round-trip for each kind: `==`, `hash` and text form equal; constant closures, `True`, `False`, `[]` and labels come back `ref.eq` to the receiver's globals.
> - **T15** Sharing: `x0 = Leaf`, `x(i+1) = Node(x(i), x(i))` to depth 40, and the same with list cells: the copy has 41 nodes. A closure that holds itself copies to a closure that holds itself.
> - **T16** Closure and key identity across a steal, also inside records and tuples; a task whose record holds the root `Assert` closure is stolen, and its `Assert` is `ref.eq` to the thief's. A child of a local scope, a task whose record holds the Task handle of such a child, and a task whose record holds a host request, are never stolen.
> - **T17** A stop on worker 3 while the others loop: exactly one message, all threads join, no final functions run.
> - **T18** Cross-worker select under stress, 200 runs: no message lost or duplicated, one commit per sync.
> - **T19** Browser: with COOP/COEP and an imports module URL, 4 Workers and a main thread that stays responsive; without isolation, or without the URL, one worker with the same output; strings through the shared memory decode right.
> - **T22** Start-up: a module with a top-level host reference (`counter` of part 3.5) runs one worker and gives 100; a module whose start-up calls a host external that the host has not declared pure runs one worker; a module that only uses `i/core` externals at start-up runs 4, and so does a module whose host externals are called only from the body of `main`.
> - **T23** Deadlock and idle on 4 workers: T20 gives the same stop; a run where worker 0 has nothing to do while workers 1 to 3 work ends normally, and in a browser worker 0 waits only with the wake request outstanding; on wasmtime, a stolen task on worker 1 that sleeps 10 ms while worker 0 runs a long pure loop wakes after about 10 ms (worker 1 returns status 1 and does not park in Wasm), and a cancel or a steal request from worker 0 still reaches worker 1 while it waits (the wake request).

- New sentence(s):

> - **T13** Differential: every `tests/spec` case and every std test runs single-core, threaded with 1 worker, and threaded with 4 workers (the last 200 times). Results, stdout and stop messages must be the same, except in tests marked as order-dependent (D1). A marked test lists every outcome it allows: a set of (stdout, stop message) pairs, or, when the set is large, a checker in i that is given the output and the stop and says whether some order of steps gives them.
> - **T14** Copy round-trip for each kind: `==`, `hash` and text form equal; constant closures, `True`, `False`, `[]` and labels come back `ref.eq` to the receiver's globals.
> - **T15** Sharing: `x0 = Leaf`, `x(i+1) = Node(x(i), x(i))` to depth 40, and the same with list cells: the copy has 41 nodes. A closure that holds itself copies to a closure that holds itself.
> - **T16** Closure and key identity across a remote claim, also inside records and tuples; a task whose record holds the root `Assert` closure is remotely claimed, and its `Assert` is `ref.eq` to the remote claimant's. A child of a local scope, a task whose record holds the Task handle of such a child, and a task whose record holds a host request, are never remotely claimed.
> - **T17** Stop on worker 3 while others spin without polling: one message and no i finals. The native subprocess tears down resources/children then exits without joining spinning workers. A continuing host interrupts/joins only that run; a concurrent and a subsequent run continue normally. Repeat with outstanding requests and a late provisional reply (native N12/§11).
> - **T18** Cross-worker select under stress, 200 runs: no message lost or duplicated, one commit per sync.
> - **T19** Browser: with COOP/COEP, an imports module URL and the dedicated-worker entry route, 4 Workers and a main thread that stays responsive; stop while worker 0 spins terminates all executing workers and tears down resources; without isolation, the URL or a stop-safe entry route, one worker with the same output; strings through the shared memory decode right.
> - **T22** Start-up: a module with a top-level host reference (`counter` of part 3.5) runs one worker and gives 100; a module whose start-up calls a host external that the host has not declared pure runs one worker; a module that only uses `i/core` externals at start-up runs 4, and so does a module whose host externals are called only from the body of `main`.
> - **T23** Deadlock and idle on 4 workers: T20 gives the same stop; a run where worker 0 has nothing to do while workers 1 to 3 work ends normally, and in a browser worker 0 waits only with the wake request outstanding; on wasmtime, a remotely claimed task on worker 1 that sleeps 10 ms while worker 0 runs a long pure loop wakes after about 10 ms (worker 1 returns status 1 and does not park in Wasm), and a cancel or a queue publication from worker 0 still reaches worker 1 while it waits (the wake request).

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Adopt N8’s one-lock backend while retaining the i scheduler, typed local storage and the accepted CML transaction semantics. Adopt N11: native uses runner byte storage/private memories; only the browser transport needs shared Wasm memory. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Adopt N1–N4 and the reviewed artifact/loading constraints, keeping explicit ilang/WASI and rejecting only required unsupplied imports. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C056 — docs/design/multicore.md, base lines 497–510

- Old sentence(s):

> - **`portable` makes captures visible.** An optimizer that drops or adds a capture changes `portable(f)`. The spec definition and T4 at O0 and O2 guard it; any later closure optimization must keep the capture set.
> - **The check costs a walk.** Every portable send, result and error walks the value, also on one worker, and a DAG is walked once for each path (part 3.6). Large data should go through `scope.share` or local channels. A later mark (D9) or a "known portable" bit can make repeat checks O(1).
> - **Stopping on a non-portable result may surprise.** `s.go(open_socket)` in a portable scope `s` stops. The local scope API and a clear message are needed, or people will route around the rule.
> - **`keep_env` keeps every bound service.** A task started under a boundary-holding service that it does not use (a generator's `Yield`, the `Assert` of `assertion.catch`) is pinned. So tests that run under `assertion.catch` get no parallelism. It cannot be seen, but `docs/performance.md` must say it. Changing `keep_env` later to keep fewer services would change `portable`'s answers.
> - **Kept functions made inside a task are tied to it.** A callback made with `control.keep_env` inside a task holds that task's `Scheduler` (D4); another task that calls it stops when it uses `i/task`. `task.keep` is the way out, and the docs must say so clearly.
> - **Stricter scope rules than a naive scheduler.** `go` and joins check the chain, channels die with their scope, and handles check their run (part 6, part 7, part 4). Each stop is fixed by the program, the same on every worker count, but programs that pass scope handles or channels out of their scope will meet them.
> - **`keep_env` and keys are new language surface.** G2 must accept them as general.
> - **Order still chooses some things.** Anything that depends on the order of steps may change on N workers (D1): which sender a receive gets, which `race` branch wins, which children fail before the cancel, whether a `shared.get` comes before its scope ends, which stop is reported, and the order of output. Tests must not depend on the order, or must be marked with their allowed outcomes (T13); the M:1 order is fixed but is not a promise for N workers.
> - **Servers that hold host references use one core.** By H, handlers that hold sockets run on the worker that accepted them (part 12, item 5). Until a host-mediated transfer exists, the E2E-echo target gets no speedup from more workers except for pure work given to portable scopes.
> - **Start-up runs N times.** It costs N times the start-up work, in parallel. Modules that make host references at start-up, or call host externals the host has not declared pure, run one worker.
> - **Host limits.** One pending async import per instance today blocks overlapping I/O; the request table of C/R must come first. N Stores each reserve a GC heap. In browsers, host functions must come from an imports module URL, and main-thread functions are async proxies with latency.
> - **Ids count every closure.** The 56-bit per-worker counter (part 3.4) does not run out in practice, but it counts closures made, not live ones; lazy ids (with the mutable field of D9) remove this cost.
> - **Field 1 and field 2 move.** If other work reads field 1 or 2 of closures before MC-I, the move breaks it. T7 and the tag-table note guard it.
> - **Left to MC.** The CAS protocol, the allocator, the mark or table of D9, quiescence detection, poll-point placement, epoch stops and the memory model are not settled here. A long computation with no poll point delays steals and cancellation, as it blocks the worker in M:1 today.

- New sentence(s):

> - **`portable` makes captures visible.** An optimizer that drops or adds a capture changes `portable(f)`. The spec definition and T4 at O0 and O2 guard it; any later closure optimization must keep the capture set.
> - **The check costs a walk.** Every portable send, result and error walks the value, also on one worker, and a DAG is walked once for each path (part 3.6). Large data should go through `scope.share` or local channels. A later mark (D9) or a "known portable" bit can make repeat checks O(1).
> - **Stopping on a non-portable result may surprise.** A child returning a user opaque host reference in a portable scope stops; standard IO wrappers do not. The local scope API and a clear message are needed, or people will route around the rule.
> - **`keep_env` keeps every bound service.** A task started under a boundary-holding service that it does not use (a generator's `Yield`, the `Assert` of `assertion.catch`) is pinned. So tests that run under `assertion.catch` get no parallelism. It cannot be seen, but `docs/performance.md` must say it. Changing `keep_env` later to keep fewer services would change `portable`'s answers.
> - **Kept functions made inside a task are tied to it.** A callback made with `control.keep_env` inside a task holds that task's `Scheduler` (D4); another task that calls it stops when it uses `i/task`. `task.keep` is the way out, and the docs must say so clearly.
> - **Stricter scope rules than a naive scheduler.** `go` and joins check the chain, channels die with their scope, and handles check their run (part 6, part 7, part 4). Each stop is fixed by the program, the same on every worker count, but programs that pass scope handles or channels out of their scope will meet them.
> - **`keep_env` and keys are new language surface.** G2 must accept them as general.
> - **Order still chooses some things.** Anything that depends on the order of steps may change on N workers (D1): which sender a receive gets, which `race` branch wins, which children fail before the cancel, whether a `shared.get` comes before its scope ends, which stop is reported, and the order of output. Tests must not depend on the order, or must be marked with their allowed outcomes (T13); the M:1 order is fixed but is not a promise for N workers.
> - **User host references still pin.** Standard sockets are portable Int wrappers; arbitrary opaque references still stay home by H (part 12, item 5). Capturing a Request/Ticket still pins a handler, even if its socket is portable.
> - **Start-up runs N times.** It costs N times the start-up work, in parallel. Modules that make host references at start-up, or call host externals the host has not declared pure, run one worker.
> - **Host limits.** One pending async import per instance today blocks overlapping I/O; the request table of C/R must come first. N Stores each reserve a GC heap. In browsers, host functions must come from an imports module URL, and main-thread functions are async proxies with latency.
> - **Ids count every closure.** The 56-bit per-worker counter (part 3.4) does not run out in practice, but it counts closures made, not live ones; lazy ids (with the mutable field of D9) remove this cost.
> - **Field 1 and field 2 move.** If other work reads field 1 or 2 of closures before MC-I, the move breaks it. T7 and the tag-table note guard it.
> - **Left to MC.** The typed locked-state interface, browser allocator, mark or table of D9, quiescence proof and transport memory model remain MC-D work. A long computation delays cooperative task cancellation and local started tasks; it cannot delay remote queue claims, and run stop uses native N12.

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Adopt N8’s one-lock backend while retaining the i scheduler, typed local storage and the accepted CML transaction semantics. Adopt N1–N4 and the reviewed artifact/loading constraints, keeping explicit ilang/WASI and rejecting only required unsupplied imports. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C057 — docs/design/multicore.md, base lines 514–521

- Old sentence(s):

> 1. Is `core.portable` accepted as a public built-in (D3), or should G2 ask for a narrower form? This note argues for public, because `i/task` is ordinary i code.
> 2. `control.keep_env` takes a function of one parameter. Is that enough, or should there be a form for functions with no parameter as well? The number of parameters is part of a function type, and function types with different numbers of parameters cannot be put together (spec §4.4, §4.6), so `() -> A` is not `(x: Nil) -> A`, and a zero-parameter body such as the `go` body `() -> A ? E ! N` cannot be passed to `keep_env` as it is. `go` wraps it in a one-parameter function (part 4), and so can any other caller; a second form would save that wrapper.
> 3. Per-task storage in general (a `task.local` facility built on keys) is not needed for Random any more (part 5): Random is a value. Should `i/task` still offer per-task storage for other uses? Any such storage lives in the scheduler, so reading it is a call to the `Scheduler` service.
> 4. How does worker k > 0 enter `i/task`'s scheduler loop without a special case? One answer: the program's entry runs `task.run`, which, on N workers, is an ordinary host export or entry kind that every worker calls. Only that entry run uses N workers; a `task.run` from inside other code is a nested run with one worker (part 4). MC-D decides.
> 5. How long may a victim wait before it answers a steal request? Poll points only, or a heartbeat in function prologues later?
> 6. Should a copy encode a message at offer time or only when a commit pairs it with another worker (part 7)? MC-D decides.
> 7. Does G2 accept keys (D13) as a public primitive in a new library module `i/key`? Without them, or another safe way to keep values of many types in one service, the scheduler cannot be written in i, and rules 12 and 15 cannot hold.
> 8. Should `Syntax` later get ids and become portable, so that tests and tools can pass code between tasks of a portable scope (part 11)?

- New sentence(s):

> 1. Is `core.portable` accepted as a public built-in (D3), or should G2 ask for a narrower form? This note argues for public, because `i/task` is ordinary i code.
> 2. `control.keep_env` takes a function of one parameter. Is that enough, or should there be a form for functions with no parameter as well? The number of parameters is part of a function type, and function types with different numbers of parameters cannot be put together (spec §4.4, §4.6), so `() -> A` is not `(x: Nil) -> A`, and a zero-parameter body such as the `go` body `() -> A ? E ! N` cannot be passed to `keep_env` as it is. `go` wraps it in a one-parameter function (part 4), and so can any other caller; a second form would save that wrapper.
> 3. Per-task storage in general (a `task.local` facility built on keys) is not needed for Random any more (part 5): Random is a value. Should `i/task` still offer per-task storage for other uses? Any such storage lives in the scheduler, so reading it is a call to the `Scheduler` service.
> 4. How does worker k > 0 enter `i/task`'s scheduler loop without a special case? One answer: the program's entry runs `task.run`, which, on N workers, is an ordinary host export or entry kind that every worker calls. Only that entry run uses N workers; a `task.run` from inside other code is a nested run with one worker (part 4). MC-D decides.
> 5. Which general typed byte-state transaction interface implements part 7 without running i code under a lock? Recommended: ordinary request-mode services; require the MC-D prototype before MC-I.
> 6. Message bytes are prepared before a transaction under part 7. MC-D chooses caching/reclamation of those immutable bytes without changing offer-time portability checks.
> 7. Does G2 accept keys (D13) as a public primitive in a new library module `i/key`? Without them, or another safe way to keep values of many types in one service, the scheduler cannot be written in i, and rules 12 and 15 cannot hold.
> 8. Should `Syntax` later get ids and become portable, so that tests and tools can pass code between tasks of a portable scope (part 11)?

- Reason: Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Adopt N8’s one-lock backend while retaining the i scheduler, typed local storage and the accepted CML transaction semantics. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C058 — docs/design/multicore.md, base lines 525–549

- Old sentence(s):

> 1. Values pass between tasks only through `go` captures and kept services, channel and event sends, task results and errors (read by the scope or by a join of the Task handle), and `scope.share`. There is no other shared state.
> 2. Every portable path calls `core.portable` and stops with `panic("task: value is not portable")` when it is False: a `send` on a `channel.new` channel (checked at the call), a send event on such a channel (checked when the event is made, never at `sync` or at commit), results and errors given to a scope made by `scope`, also inside `both`, `race` and `map_concurrently`, which are built on `scope`, and `share`. This happens always, also on one worker. Local forms of the combinators, if R adds them, are built on `local_scope` and do not check.
> 3. `go` never checks portability. Whether a task can move is decided only at steal time, from its whole record (kept start function and scope handle); a failed encode leaves the task home and cannot be seen.
> 4. `go` binds `Scheduler` to the portable `scheduler.detached` and then keeps the parent's services with `control.keep_env`, and takes the real `Scheduler` as the parameter of the kept function. So the parent's scheduler is never kept or captured. The kept function binds `! Scheduler = own` and then calls the body **not** in tail position (`r = …  r`), so that the statement stays on a frame record after a resume (`lazy-reentry.md` D16, section 7a); R checks at O2 that it stays a frame. `task.keep(f)` is built the same way for callbacks that other tasks call; a function that user code keeps with `control.keep_env` inside a task is tied to that task (D4), and the docs say so.
> 5. Every task runs directly from its run's loop, never on another task's stack, under a new boundary that the loop makes for it at its start (D5); `control.resume` makes it active again. The loop has no long-lived boundary of its own. Nested scopes, `both`, `race` and `map_concurrently` never run a loop on the caller's stack. Only the entry's `task.run` may use more than one worker; any other `task.run` is a nested run: one worker, FIFO, on the caller's stack, on every host. `task.run` needs the run service (part 4), which gives it its path prefix, root seed and wait. A nested run inside a task has paths that start with the calling task's path and its run index, waits by suspending only its calling task to the outer run (never by status 1 from the caller's stack), and stops with `panic("task: deadlock")` when it has nothing to run and no request of its own. Every handle holds the key of its run; using it in another run stops with `panic("task: handle of another run")`.
> 6. Local channels and local scopes hold the boundary of the task that made them, only to be non-portable; no suspend ever goes to it. The Task handle of a child of a local scope, and every join event made from it, hold the local scope handle, so they are not portable either. Every task record holds the handle of its scope, so a child of a local scope is never stolen. Local paths do not check. There is no flag and no special case.
> 7. A started task never moves. A task that waits on an async host call resumes on the worker that started the call. A host request value (R2) is not portable, so only tasks on its home worker wait on it, poll it or cancel it.
> 8. A scope returns only after all its children have ended. On a failure it cancels the rest **and its own body**, which is unwound back to the `scope` call, and then gives the error of the failed task with the lowest path in path order (D10; its own body first, `[1] < [1, 1] < [2]`); a cancelled body is not a failure. Every task has a chain of scopes (part 6), changed only by its own scope entries and exits. `s.go`, and a join of a child of `s`, may be done only by a task whose chain holds `s`; otherwise they stop with `panic("task: go outside its scope")` and `panic("task: join outside its scope")`, whether or not `s` has ended. A `go` on a cancelling scope makes a task that is dropped at once. No join is ever left waiting for a dropped or cancelled task. Error types stay unions.
> 9. `both` and `map_concurrently` give results in argument order. The docs state the rule of D1: every run gives the result of some order of steps, so a plain receive with several senders, `race`, `choose`, and anything else that depends on the order may differ between runs and worker counts.
> 10. Cancellation is cooperative: an unstarted task is dropped; a started task is cancelled on its home worker when the cancel reaches it (at once when it is paused, at its next suspend point when it runs; when that is, is a step of D1). The loop never calls `control.cancel` on a task's continuation: it resumes the task with a cancel answer, and the task cancels its own inner part, so its boundary stays active and final functions may use `i/task`. A task being unwound is not cancelled again.
> 11. Every task has a path (the path of the task that called `go` plus the index of that `go` among its `go` calls and nested runs), given by `task.path()` with no suspend. Random is a value: `task.random()` is a pure function of the root seed and the path, and `i/random` does not need `i/task` or `Scheduler`. The root seed comes from the run service; at the top the program gives it (`task.top(seed)`), with no hidden default. Path steps and split steps are different kinds of stream step, so `random.split` never gives the generator of a task. Handle ids and per-task names come from the path, never from time, a global counter, or the worker.
> 12. Task, scope, channel, event and `Shared` handles are made of path-based ids and keys (D13), and hold no boundary, closure over scheduler state, or other non-portable value (except local handles and the handles of children of local scopes, rule 6). `==` on them means "the same handle". No API needs an order or a hash on functions, keys, boundaries or continuations; scheduler bookkeeping uses Int ids.
> 13. Events are values. `choose` and `map` are pure, `sync` commits exactly one event, and `map` functions run in the syncing task after the commit. Nack and guard are left for later.
> 14. `scope.share(value)` needs a portable value. A `Shared` belongs to its scope, and a `get` after the scope ends stops with `panic("task: shared value is gone")` in every build. A channel belongs to the first scope in its maker's chain (or to the run); its state is freed when that scope ends, and a later send, receive or `sync` on it, or a wait on it at that end, stops with `panic("task: channel is gone")` in every build.
> 15. All shared mutable scheduler state is behind the one `Scheduler` service, kept as `Keyed` values where it has many types, so the multicore wave can replace its implementation without changing the API. No API needs an unsafe cast.
> 16. The scheduler has poll points (`go`, every `sync`, joins and scope ends, `yield`, async host calls, every switch of the loop) written as one call that does nothing on one worker.
> 17. On one worker the scheduler is FIFO and fixed, with no time unless a `Clock` service is given. It idles only by waiting for an outstanding host request (C-D's "wait for any"), never by a busy loop or a blocking import, and never returns status 1 with no request outstanding. When no task can run and no request is outstanding, it stops with `panic("task: deadlock")`. On N workers (MC), a worker with outstanding requests of its own never parks inside Wasm; it returns status 1 with a wake request outstanding (part 6, Idle).
> 18. Many host calls may wait at once, through the request table. No code relies on one pending import per instance.
> 19. A stop in any task stops the program. It is never caught or counted as a task error.
> 20. No API takes a label-generic callback (records L5), and no API needs a static `Send`-like type.
> 21. No API shows worker indices, the worker count, or steals.
> 22. Tests of `i/task` and of programs that use it must not depend on the order of steps (D1: which sender a receive gets, which `race` branch, which children fail before a cancel, whether a step comes before a scope ends, which of two stops, the order of output), or must be marked with the outcomes they allow (T13).
> 23. `Key(A)` is invariant in `A`, and keys are only used through `i/key`; no API casts.
> 24. `s.go` and joins check the chain, channels and `Shared` values are freed with their scope, and handles check their run, all in M:1 already, so that the multicore wave changes no program.
> 25. Only `==` reads function and key ids. The text form of a function, a key, a `Keyed` value or any handle never shows an id, a worker index or anything else that depends on placement, and no `i/task` API gives one out.

- New sentence(s):

> 1. Values pass between tasks only through `go` captures and kept services, channel and event sends, task results and errors (read by the scope or by a join of the Task handle), and `scope.share`. There is no other shared state.
> 2. Every portable path calls `core.portable` and stops with `panic("task: value is not portable")` when it is False: a `send` on a `channel.new` channel (checked at the call), a send event on such a channel (checked when the event is made, never at `sync` or at commit), results and errors given to a scope made by `scope`, also inside `both`, `race` and `map_concurrently`, which are built on `scope`, and `share`. This happens always, also on one worker. Local forms of the combinators, if R adds them, are built on `local_scope` and do not check.
> 3. `go` never rejects a task for non-portability. On N workers it silently classifies and encodes the complete record at go (D8/P1); failure pins locally. A shared entry has one claim/drop owner, coordinated with scope cancellation. One worker keeps its FIFO originals without encoding.
> 4. As runtime §6.3 requires, `go` also detaches/rebinds Runner and Await while preserving Root/Owner. `go` binds `Scheduler` to the portable `scheduler.detached` and then keeps the parent's services with `control.keep_env`, and takes the real `Scheduler` as the parameter of the kept function. So the parent's scheduler is never kept or captured. The kept function binds `! Scheduler = own` and then calls the body **not** in tail position (`r = …  r`), so that the statement stays on a frame record after a resume (`lazy-reentry.md` D16, section 7a); R checks at O2 that it stays a frame. `task.keep(f)` is built the same way for callbacks that other tasks call; a function that user code keeps with `control.keep_env` inside a task is tied to that task (D4), and the docs say so.
> 5. Every task runs directly from its run's loop, never on another task's stack, under a new boundary that the loop makes for it at its start (D5); `control.resume` makes it active again. The loop has no long-lived boundary of its own. Nested scopes, `both`, `race` and `map_concurrently` never run a loop on the caller's stack. Only the entry's `task.run` may use more than one worker; any other `task.run` is a nested run: one worker, FIFO, on the caller's stack, on every host. `task.run` needs the run service (part 4), which gives it its path prefix, root seed and wait. A nested run inside a task has paths that start with the calling task's path and its run index, waits by suspending only its calling task to the outer run (never by status 1 from the caller's stack), and stops with `panic("task: deadlock")` when it has nothing to run and no request of its own. Every handle holds the key of its run; using it in another run stops with `panic("task: handle of another run")`.
> 6. Local channels and local scopes hold the boundary of the task that made them, only to be non-portable; no suspend ever goes to it. The Task handle of a child of a local scope, and every join event made from it, hold the local scope handle, so they are not portable either. Every task record holds the handle of its scope, so a child of a local scope is never remotely claimed. Local paths do not check. There is no flag and no special case.
> 7. A started task never moves. A task that waits on an async host call resumes on the worker that started the call. A host request value (R2) is not portable, so only tasks on its home worker wait on it, poll it or cancel it.
> 8. A scope returns only after all its children have ended. On a failure it cancels the rest **and its own body**, which is unwound back to the `scope` call, and then gives the error of the failed task with the lowest path in path order (D10; its own body first, `[1] < [1, 1] < [2]`); a cancelled body is not a failure. Every task has a chain of scopes (part 6), changed only by its own scope entries and exits. `s.go`, and a join of a child of `s`, may be done only by a task whose chain holds `s`; otherwise they stop with `panic("task: go outside its scope")` and `panic("task: join outside its scope")`, whether or not `s` has ended. A `go` on a cancelling scope makes an unstarted task dropped at once; a shielded caller receives its Task handle, while a cancellable caller unwinds before observing it (runtime R4a). At a valid join/selected join event, applicable cancellation first unwinds a cancellable joiner; a shielded joiner stops with `panic("task: join of a cancelled task")` as soon as target drop/cancellation is accepted, without waiting for cleanup (runtime R4a). Error types stay unions.
> 9. `both` and `map_concurrently` give results in argument order. The docs state the rule of D1: every run gives the result of some order of steps, so a plain receive with several senders, `race`, `choose`, and anything else that depends on the order may differ between runs and worker counts.
> 10. Cancellation is cooperative: an unstarted task is dropped; a started task is cancelled on its home worker when the cancel reaches it (at once when it is paused, at its next suspend point when it runs; when that is, is a step of D1). The loop never calls `control.cancel` on a task's continuation: it resumes the task with a cancel answer, and the task cancels its own inner part, so its boundary stays active and final functions may use `i/task`. A task being unwound is not cancelled again.
> 11. Every task has a path (the path of the task that called `go` plus the index of that `go` among its `go` calls and nested runs), given by `task.path()` with no suspend. Random is a value: `task.random()` is a pure function of the root seed and the path, and `i/random` does not need `i/task` or `Scheduler`. The root seed comes from the run service; the entry default is random.from_seed(0), and task.top(seed) changes only the current Runner's seed/route-preserving value, with no entropy draw (runtime D5). Path steps and split steps are different kinds of stream step, so `random.split` never gives the generator of a task. Task/scope/channel/event/share ids and per-task names come from the path, never from time, a global counter, or the worker.
> 12. Task, scope, channel, event and `Shared` handles are made of path-based ids and keys (D13), and hold no boundary, closure over scheduler state, or other non-portable value (except local handles and the handles of children of local scopes, rule 6). `==` on them means "the same handle". No API needs an order or a hash on functions, keys, boundaries or continuations; scheduler bookkeeping uses Int ids.
> 13. Events are values. `choose` and `map` are pure, `sync` commits exactly one event, and `map` functions run in the syncing task after the commit. Nack and guard are left for later.
> 14. `scope.share(value)` needs a portable value. A `Shared` belongs to its scope, and a `get` after the scope ends stops with `panic("task: shared value is gone")` in every build. A channel belongs to the first scope in its maker's chain (or to the run); its state is freed when that scope ends, and a later send, receive or `sync` on it, or a wait on it at that end, stops with `panic("task: channel is gone")` in every build.
> 15. All shared mutable scheduler state is behind the one `Scheduler` service, kept as `Keyed` values where it has many types, so the multicore wave can replace its implementation without changing the API. No API needs an unsafe cast.
> 16. The scheduler has poll points (`go`, every `sync`, joins and scope ends, `yield`, async host calls, every switch of the loop) written as one call that does nothing on one worker.
> 17. On one worker the scheduler is FIFO and fixed, with no time unless a `Clock` service is given. It idles only by waiting for an outstanding host request (C-D's "wait for any"), never by a busy loop or a blocking import, and never returns status 1 with no request outstanding. When no task can run and no request is outstanding, it stops with `panic("task: deadlock")`. On N workers (MC), a worker with outstanding requests of its own never parks inside Wasm; it returns status 1 with a wake request outstanding (part 6, Idle).
> 18. Many host calls may wait at once, through the request table. No code relies on one pending import per instance.
> 19. A stop in any task stops the program. It is never caught or counted as a task error.
> 20. No API takes a label-generic callback (records L5), and no API needs a static `Send`-like type.
> 21. No API shows worker indices, the worker count, or queue claims.
> 22. Tests of `i/task` and of programs that use it must not depend on the order of steps (D1: which sender a receive gets, which `race` branch, which children fail before a cancel, whether a step comes before a scope ends, which of two stops, the order of output), or must be marked with the outcomes they allow (T13).
> 23. `Key(A)` is invariant in `A`, and keys are only used through `i/key`; no API casts.
> 24. `s.go` and joins check the chain, channels and `Shared` values are freed with their scope, and handles check their run, all in M:1 already, so that the multicore wave changes no program.
> 25. Only `==` reads function and key ids. The text form of a function, a key, a `Keyed` value or any task/scope/channel/event/share handle never shows an id, a worker index or anything else that depends on placement, and no `i/task` API gives one out.

- Reason: Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Adopt N8’s one-lock backend while retaining the i scheduler, typed local storage and the accepted CML transaction semantics. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Adopt N1–N4 and the reviewed artifact/loading constraints, keeping explicit ilang/WASI and rejecting only required unsupplied imports. Make the implementation/spec handoff explicit: prerequisites now, packaging and MC mechanisms in their own stages. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

### C059 — docs/design/native.md, base lines 1–135

- Old sentence(s):

> # Design proposal: native packages and multicore workers
>
> Status: proposal. It comes from a discussion with the user on 2026-11-10, and the user agreed to its direction. It is not reviewed. Nothing in it is implemented or measured. Design notes were read on `redesign` at `1e0d910` (`multicore.md`, `runtime.md`, `host-concurrency.md`, `lazy-reentry.md`), and code on `main` at `229e3a4`.
>
> Revision 2 (2026-11-10) takes the findings of the independent review that the redesign coordinator ran: N4 is narrower, N9 has a representation that the host boundary allows and a clean-up rule, N12 is new, the sentence on moving continuations is corrected, and part 9 says what the review checked.
>
> This note changes four things that the redesign plan lists as settled. Part 8 lists them. Each needs the user's decision in the plan's own process.
>
> ## 1. Goal
>
> - `i build` makes **one executable file** by default: the program, compiled ahead of time, with the Wasmtime runtime.
> - A program uses **all cores**, with multicore speed that is not bad.
> - i stays small and easy to understand, in what the programmer sees and in how it is made.
>
> ## 2. What the programmer sees
>
> Three rules. The language has no worker count.
>
> 1. `go` starts a task. The task may run on any core.
> 2. Values do not change, so a program cannot see if a value was copied or shared.
> 3. A task that holds a boundary, a continuation or a host reference stays on the core where it was made.
>
> These are the rules of `multicore.md` (S7). This note keeps all of them: D1 (one semantic rule), D2 and D3 (portable values, checked on every worker count), D6 (stable ids), D11 (first stop wins), and `scope.share`.
>
> ## 3. Decisions
>
> **N1. `native` is a new target, and the default of `i build`.** `--target ilang` and `--target wasi` stay. `i run` and `i test` do not change: they compile and run in the process. *Proposed default, which the user has not objected to:* only `i build` changes.
>
> **N2. A native package is a runner with the program inside.** The runner is a program built ahead of time for each target triple. It has the Wasmtime runtime with no compiler (no Cranelift), the standard host services, and the reactor. `i build` compiles the module to a `.cwasm` with the Cranelift that `i` already has (`ilang-host` uses Wasmtime 50.0.0 with the `cranelift` feature), and puts it into a copy of the runner.
>
> - A `.cwasm` belongs to one Wasmtime version, one target triple and one engine configuration. The runner and `i build` must use exactly the same three. A test must check this.
> - Loading a `.cwasm` is unsafe in Wasmtime's API, because the bytes are trusted. The runner loads only the bytes inside its own file.
> - To build for another target, `i` needs the runner of that target. The release archive has them, or `i` fetches one. This is open (part 9).
>
> **N3. The native target uses the i binary interface (ABI v3), not WASI.** On the WASI target an async host function is a blocking call, so a program cannot wait for many requests.
>
> **N4. On the native target, an import that the program needs and the runner cannot give is a compile error.** A native package has no embedder. The error names the import and `--target ilang`. A program that declares a host service and replaces it with ordinary i code (§9.7) needs no import, and builds. *Reason (review):* the first form of this rule refused every user host service, and so refused programs that run today.
>
> **N5. Workers.** The runner starts N threads. Each thread has one Wasmtime `Store` with one instance of the module, and runs the same scheduler, which is ordinary i code (the M:1 scheduler of R2). N is the number of cores by default. The runner reads another number from its environment or from `i.toml` (open, part 9).
>
> **N6. A started task does not move, and nothing is preempted.** A task that computes for a long time without a scheduler call delays the other started tasks of its worker. Other workers go on. The docs say this.
>
> **N7. One shared queue of tasks, encoded at `go`.** This takes the place of cooperative stealing (`multicore.md` D8).
>
> - At `go`, a portable task is encoded into bytes and put in a queue that the runner holds. The worker also keeps the task itself.
> - A task that is not portable goes only to the worker's own queue (rule 3 of part 2).
> - An idle worker claims an entry of the shared queue. One atomic step changes the entry from "ready" to "taken", so only one worker gets it.
> - When the worker that made the task claims it, it runs the task it kept, and decodes nothing. Another worker decodes the bytes.
> - The check for "portable" runs once, at `go`, so its answer does not depend on timing.
>
> *Reason:* only the worker that owns a task can encode it, because no other worker can read its GC heap. Encoding later ("when stolen", or "when a worker is idle") needs the owner to reach a poll point, and an owner in a long computation does not. Encoding at `go` needs no protocol, no interrupt and no help from a busy worker. The cost is one encode for each `go`, also when the same worker runs the task.
>
> **N8. Shared scheduler state is in the runner, behind one lock.** Channel state, wake-ups, the stop cell and the shared queue are in the runner. A message between workers is encoded bytes. This takes the place of the lock-free parallel CML protocol that `multicore.md` part 7 plans for the multicore wave. The scheduler stays i code: the runner gives a small set of general operations (a byte queue, a lock, a wait). Their exact form is open (part 9). *Reason:* one lock is the easiest to get right. It is replaced only when a measurement shows that it is the limit.
>
> **N9. IO resources are integer handles, not host references.** A socket, a listener, a file and a child process are values of a private nominal type that holds one `Int`. The raw host service takes and gives that `Int`, and ordinary i code in the library module wraps and unwraps it. This changes `runtime.md` D9.
>
> - *Reason for this form (review):* §10.3 lets only `Nil`, `Bool`, `Int`, `Float`, `String`, `BitArray` and custom types with no constructors go to the host. A custom type with an integer field gives E0320, and a type with no constructors is a host reference, which is not portable. A private type around an ABI `Int` needs no special case in the compiler.
> - Such a handle is portable by the S7 rules, with no special case. So one accept loop with `go` for each connection uses all cores, as in Go.
> - The number is never used again, so the table is a map from numbers to resources, and a use after close finds nothing. (The first form of this note had an index and a generation; one number that is not reused is simpler.)
> - An `Int` can be made by any code, where a host reference cannot. So the raw services are not exported from the library modules; only the wrapped functions are. If this is enough is a question for the IO design (part 9).
> - A handle has no GC lifetime. It is closed explicitly, as `resource.ensure` already does.
> - **The table belongs to one run, and the end of the run closes everything left in it.** Final functions do not run at a stop (§9.5), so this rule, not a final function, frees host resources then. On native, a run is the process. Under `i run`, `i test` and an embedder, the table is dropped with the run, as dropping the instance frees host references today.
> - Two workers that read one socket get a result that depends on the order. D1 of S7 allows this. The docs say it.
> - A host request (`Request`, `Ticket`) stays not portable.
>
> *Reason:* host references are not portable because of JS realms, and a browser has no listening socket.
>
> **N10. With one worker, nothing of this runs.** The shared queue and the lock are skipped. The behavior and the cost are those of M:1 today.
>
> **N11. One encoded format, and a transport for each host.** On native, the bytes are in the runner's memory, and the module needs no shared linear memory and no thread-safe `ilang_alloc`. In a browser, the same bytes go through a shared linear memory, as the plan says.
>
> **N12. A stop ends every worker.** D11 of S7 says that a stop anywhere stops the whole run. A worker in a loop with no scheduler call never sees the stop cell. On native, the runner prints the message of the first stop and exits the process. In a host that lives on (an embedder with more than one worker), the host uses Wasmtime's epoch interruption so that the other stores trap. That is the plain use of epochs (a trap at the deadline, with no callback), and it is turned on only when there is more than one worker, because the checks cost time in all code. *Reason (review):* without this, a run with a stop may never return.
>
> ## 4. What is not done, and why
>
> - **Cooperative stealing, or publishing only when a worker is idle.** Both need the owner to reach a poll point (N7).
> - **An interrupt for encoding** (Wasmtime's epoch interruption). It closes that gap, but the task queue must then be safe against an interrupt at any point, and two facts about Wasmtime are not checked (part 9). N7 makes it unnecessary.
> - **Moving a started task by copying its continuation.** After lazy re-entry a paused continuation is heap data (frame records with a function reference and saved values), so this is possible later. Its cost is the size of everything the task can reach. This note asks only that nothing blocks it. The stable ids of D6 are for the sameness of function values; they are not enough here. A frame record holds a reference to resume code, and moving it needs a stable number for each such code place, which is a different thing and is not designed.
> - **A second back end on linear memory with Wasm threads.** It gives a real shared heap, but i must then have its own collector for many threads, and two back ends and two runtimes.
> - **A fork of Wasmtime with shared-everything threads.** Rough estimate: 1 to 2 person-months for a trial with the null collector, and 1 to 2 person-years until it can be trusted, plus the cost of keeping a fork. Wasmtime 50 has the feature flag and the validation only.
> - **The component model.** Components share nothing and copy only WIT types, so it does not help with cores. The runner may use Wasmtime's WASI implementation in Rust for its own IO; that is a choice of the IO stage.
>
> ## 5. Targets
>
> The numbers are proposals. i has no HTTP measurements today, so the first run calibrates them.
>
> | Group | Workload | Target |
> |---|---|---|
> | Server | TechEmpower Plaintext and JSON (the definitions; the project is reported as archived since March 2026) | Throughput on N cores at least 0.8 × N of one core for N ≤ 8, and 0.6 × N for 16. **Must hold.** |
> | One worker | The programs in `bench/` | Within 2% of M:1 today. **Must hold.** |
> | Parallel computing | Mandelbrot or spectral-norm in chunks, with the input in `scope.share` | Speedup at least 5.5 on 8 cores. **Must hold.** |
> | Absolute | JSON on 8 cores | Not below Node.js `cluster`, and Gleam (mist) or Elixir (Bandit). Wanted. |
> | Distance to Go | The same, against Go `net/http` | Recorded only. |
> | Weak points | skynet (10^6 very small tasks); ping-pong between workers; p99 of requests while one long computation runs | Recorded only. They decide part 6. |
>
> Run each with 1, 2, 4, 8 and 16 workers, and keep the curve. The server group needs `i/net` and the `http` module. Before that, measure the time to encode and decode the task of a usual request; this is the first number that tests N7.
>
> ## 6. Later steps
>
> None of these changes a program, because the S7 rules hold.
>
> 1. The lock is the limit: use finer locks, or a lock-free protocol.
> 2. Encoding is the limit: build the Wasmtime trial with the null collector, and measure what sharing gives.
> 3. Wasmtime gets a shared GC heap: pass references, and remove the encoding.
>
> ## 7. Stages
>
> 1. **Native package with one worker.** A runner crate, the `.cwasm` inside it, the new default of `i build`. After R2 (`i/task` and the request table).
> 2. **Workers.** In the multicore wave (MC-D, gate G5), with N5 to N11 as the proposal.
> 3. **Measure** (part 5), and choose from part 6.
>
> ## 8. Changes to what the plan calls settled
>
> 1. **`multicore.md` D8:** cooperative stealing becomes the shared queue, encoded at `go` (N7).
> 2. **`runtime.md` D9:** IO resources become integer handles (N9). That note is in review now.
> 3. **`multicore.md` part 7, multicore wave:** the parallel CML protocol becomes one lock (N8).
> 4. **The default target of `i build`** becomes `native` (N1), and the plan gets the native package as new work.
>
> ## 9. Not checked, and open
>
> Checked by the review, with probe programs on Wasmtime 50: a Wasmtime runtime with no compiler loads and runs a `.cwasm` that this compiler made; one `Store` for each worker works, and an idle `Store` can be called from another thread; a shared GC heap is not there. The review also found that `Program::from_bytes` in `ilang-host` calls the compiler, so the runner cannot use it as it is; it must be split first.
>
> Still open:
>
> - The cost of one encode and one decode. Nothing is measured.
> - The size of a runner built with no Cranelift.
> - If keeping the raw IO services private is enough against made-up handle numbers (N9), or the numbers must also be hard to guess.
> - How the `.cwasm` is put into the runner (bytes after the end of the file, or a section), and what code signing on macOS and Windows needs.
> - Where the runners of other targets come from.
> - The name and the place of the worker count.
> - The exact operations that the runner gives the i scheduler (N8), and that `choose` and `sync` across workers are right under one lock.
> - Deadlock detection and the stop cell across workers (`multicore.md` parts 6 and D11) with the state in the runner.
> - A handle after close while another worker waits on it (N9).
> - For the interrupt of part 4 only: that an epoch callback may call into the same instance, and that it may allocate GC objects.
> - When Wasmtime will have a shared GC heap. Its tracking issue is #9466. The last clear statement found (November 2024) says that nobody had started it for GC.

- New sentence(s):

> # Design: native packages and multicore workers
>
> Status: adopted, revision 3 (2026-11-10). The ledger's “Native proposal ADOPTED” records
> the user's adoption of revision 2, including the four replacements in §8. This revision
> reconciles runtime.md and multicore.md and closes the lifetime, stop and forged-number
> gaps. It changes no implementation. Performance targets are acceptance goals, not
> measurements.
>
> Controlling sources, in order: spec.md (§4.1/§4.7, §9.5/§9.7, §10.3, §11.2/§11.7,
> Appendix B); accepted syntax.md; runtime.md, multicore.md, host-concurrency.md and
> lazy-reentry.md; the plan's “已定的设计” and R/IO/MC stages; the ledger's subsequent adoption
> decision. All examples use the accepted syntax. Runtime's accepted contextual entry,
> Root/Owner, shielded-final and join corrections stay.
>
> ## 1. Problem and goal
>
> i is a standalone language: i run supplies its host; i build should make one executable
> containing the program and Wasmtime. One portable accept loop should be able to use all
> cores. No keyword, compiler rule for an IO library, static Send constraint or change to
> principal inference is needed.
>
> Separate three lifetimes. A task/scope belongs to an i scheduler run. A Request belongs
> to a worker and its Root/Owner group. An OS resource belongs to the host context of the
> enclosing entry invocation, across its workers and nested scheduler runs. Closing a
> request group does not close resources already given to callers. Ending the entry
> context does. Confusing these lifetimes leaks provisional children or lets the next test
> reuse a stale number.
>
> ## 2. Alternatives, criteria and choice
>
> Choose by the controlling spec, set-of-values typing, principal inference, ordinary
> libraries, precise ownership, isolated sequential/concurrent runs, finite host critical
> sections, and the amount R1/R2/IO would have to redo. These alternatives were considered
> before selecting the rules below:
>
> 1. **Keep opaque host references and cooperative stealing.** References cannot
>    be fabricated as Ints and have existing host ownership. But they pin socket
>    handlers and cooperative stealing waits for a busy maker. Keep their locality
>    for user opaque references, and keep the existing request cancellation seam.
> 2. **Sequential numbers, private raw bindings, immediate native exit.** This
>    is small and makes ordinary resource values portable. Privacy alone does not
>    authorize a host import; immediate exit kills an in-process child supervisor.
>    Keep nominal Int wrappers, checked non-reuse and process exit as the final
>    native action, after host teardown.
> 3. **Random Int tokens or a new unforgeable portable capability type.** Random
>    tokens reduce guessing but cannot make equal Ints different authorities.
>    A new primitive adds typing/ABI/copy work; opaque references would reintroduce
>    locality. Reject both. Keep kind checks and explicit host permissions, which
>    are needed even with secret tokens.
> 4. **Run-owned Int table, authenticated binding grants and one locked queue.**
>    Choose this. It needs no new value type: File is the set of values made by
>    its private constructor over Int; the compiler sees ordinary nominal data.
>    Host permissions are a general binding policy, useful for IO, plugin stores
>    and application credential services. They add no typing constraint or
>    attribute. Keep choice 1's local Request/Ticket and choice 2's simple counter.
>
> ## 3. Decisions and exact rules
>
> **N1. Native is the default target of i build.** Explicit --target ilang and --target
> wasi remain. i run/i test continue to compile and execute in a host process. Native
> packaging requires the program entry; a package with only host exports uses --target
> ilang, with a clear missing-entry diagnostic. Only build's default changes. Source:
> adopted revision 2/ledger.
>
> **N2. A native package is a prebuilt runner plus AOT program bytes.** Build precompiles
> with its compiler-enabled Wasmtime; the runner has the runtime, standard services and
> reactor, without a compiler backend. The artifact/runner contract pins Wasmtime version,
> target triple, CPU feature baseline, collector, engine tunables and
> epoch-instrumentation mode. Do not infer build-machine CPU features for a distributed
> baseline. Check a manifest before unsafe deserialize; load only the trusted payload
> packaged for that runner, never an arbitrary user .cwasm. Program::from_bytes must be
> factored, not reused unchanged (§9/§10). Reason: the review's runtime-only probe and
> current constructor, cited in §9.
>
> **N3. Native uses the i ABI v3 wire, not WASI.** WASI async imports are blocking under
> §B.7 and cannot provide overlapping accept/read. Ordinary new host services can use v3.
> New runtime imports/statuses/host-called exports need an ABI review and version bump
> under host-concurrency.md; N8 does not silently add them.
>
> **N4. Reject only required imports the runner cannot supply.** Report the
> namespace/label and suggest --target ilang for embedding. Apply this after inferred
> needs and ordinary i service substitution; a declared service replaced with a local fake
> needs no import. Permission withheld under §3.1 means the runner cannot supply that
> binding. Merely declaring a host type is never a native packaging error. This is the
> user's adopted N4, not a namespace ban.
>
> **N5. One Store and instance per executing worker.** Each runs the ordinary i M:1
> scheduler, with all mutable scheduler state behind Scheduler. Only the entry run may use
> multiple workers; nested runs stay one-worker FIFO as runtime §6.5 requires. Top-level
> initialization, static ids and one-worker fallback remain multicore D7. Starting N
> copies of main is not the worker entry protocol. N defaults to cores; where its override
> lives remains open (§12).
>
> **N6. Started tasks never move; task scheduling is cooperative.** A long pure
> computation delays other started tasks on its worker. Other workers can claim
> already-published tasks. Whole-run stop is an explicit exception (N12), not task
> preemption or migrating a continuation. Keep complete task records, silent pinning, kept
> environments, identity and local witnesses (S7 D2–D7/P1–P5).
>
> **N7. One shared queue, encoded at go.** On N workers the maker classifies the complete
> immutable record: kept start function, captures, every kept service, scope handle, path
> and chain. If portable it encodes once, publishes bytes and retains its original;
> otherwise it queues locally, without a stop. Publication registers the child with its
> scope and rechecks cancellation under the same lock: a cancel during encoding drops the
> unstarted record, frees the prepared bytes and applies R4a to its caller. One locked
> transition claims a Ready entry as Taken(owner). The maker uses its original; a remote
> owner decodes. Claim/cancel/drop serialize with scope cancellation: cancel first drops
> it; claim first routes cancellation to that owner. A claimed but unstarted task may
> still be dropped. A started task stays there. Reclaim the queue entry after claim/drop;
> a remote claimant owns the payload bytes until decoding ends. Reclaim the maker's
> retained original after acknowledgement. Keep outcome versus cleanup completion and
> runtime R4a's join rules. Reason: only the maker reads its GC heap; eager encoding needs
> no cooperation from it later. Cost: one encode per portable spawn even if it eventually
> runs locally.
>
> **N8. Shared scheduler state uses one lock.** Queue ownership, scope/channel state, sync
> records, wakeups and the stop cell live in the runner transport. Worker continuations,
> Request/Ticket and GC Keyed objects stay in local typed tables; shared state contains
> bytes, stable ids and owner tokens. The i scheduler computes commands and applies
> decisions. No i code, callback, codec, Wasm call or OS wait executes under the host
> lock. CML's exact single transaction is in multicore §7, retaining runtime §6.4's
> validation, source/FIFO order, no self-pair, one commit, losing-leaf removal and
> post-commit mapper placement. General request-mode byte-state/wake services keep ABI v3;
> their exact typed interface and progress proof are an MC-D implementation gate, not a
> claim that a mutex alone implements the scheduler. Replace the lock only after
> measurements.
>
> **N9. IO values are private nominal wrappers over Int.** File, Listener, Stream and
> Child each have one private constructor containing an Int, defined in their ordinary
> modules. Raw request services take/return Int; wrappers unwrap/wrap and translate errors
> explicitly (runtime §7). §10.3 remains unchanged: a wrapper itself is not an ABI value.
> A user opaque host reference stays non-portable. The wrappers are portable by ordinary
> S7 rules, while Request/Ticket stay local. Export raw service type names alone when
> public function environments use them (spec §2.4); hide their constructors, field access
> and live field values. The existing general field-privacy rule must be enforced before
> Fs (§9/§10). Public exports must not construct wrappers from caller numbers or accept
> caller-made raw services. Otherwise a fake raw producer could return a guessed number,
> have the module wrap it and pass the forged resource to live wrappers. Resource fixtures
> override host bindings explicitly; applications can inject ordinary public wrapper
> functions at their call sites. Accessible user services and Clock/Stdio still support
> ordinary i constructor fakes. The resource values use ordinary custom
> equality/hash/text. An IO number can therefore appear in text; it is not a secret or a
> scheduler path id.
>
> **N9.1. One host ResourceContext per live entry invocation.** It is shared by all
> workers of that invocation. Nested task.run, Root and Owner keep that same resource
> context. A subsequent export call/test/instance gets a fresh one. A host-wide allocator
> is shared across those contexts. Each map entry records its context, kind, OS resource
> and operation leases; a number is never an OS fd, pointer or request-table slot.
> Unknown, nonpositive, closed, wrong-kind or foreign-context numbers give code closed;
> close on them gives Nil. Checking arguments never downcasts a resource merely because
> its number exists.
>
> **N9.2. Numbers are never reissued within one live host.** Allocate positive
> arbitrary-precision Ints monotonically across all contexts and workers, including
> provisional replies and failed creations. JS uses BigInt. Free entries, not numbers; no
> tombstone for every old number is needed. Never wrap, truncate, reset between tests, or
> fall back to a reused fd. Allocation/bookkeeping failure is a memory-exhaustion stop
> before exposure; dispose of any already-created OS resource. Tests may inject a small
> counter limit: the next allocation stops, it never recycles. A host restart begins a new
> identity domain. Raw numbers retained outside it carry no promise; public wrappers
> cannot cross §10.3 anyway. Reason: spec §11.2's Int does not overflow; a bounded host
> counter cannot silently change that contract.
>
> **N9.3. Lookup, leases and close have one order.** Under a short host lock, test
> context/kind/liveness and acquire the operation's lease. Socket directions each have one
> lease; read+write may overlap, two reads/writes/accepts give busy. Files have one
> read/write/flush cursor lease, children one wait lease. Removal on close revokes leases
> and wakes pending operations with closed. A completion committed first retains its
> answer; close committed first prevents success publication. In-flight code holds a safe
> reference/private OS duplicate, never a pointer to a removed table entry. Release
> happens once when references are safe. Close is idempotent/error-free; cancelling its
> request cannot undo host cleanup. Child close awaits reaping. A context/kind-checked
> cleanup receipt survives removal; concurrent closes join that receipt, including
> reaping, and teardown includes already-closing jobs. Free receipts on completion without
> permanent tombstones. Preserve the blocking-file cancellation, stdin broker,
> partial-write and supervisor rules of runtime §8.1.
>
> **N9.4. Provisional resources have an owner before a reply exists.** The context owns
> them as soon as created. The host reply's drop guard removes and closes an undelivered
> resource. Successful validation and Ready publication disarm that guard, but the table
> retains teardown ownership. The ordinary i Ready/Taken guard closes an unclaimed
> resource if its awaiting task unwinds. No scheduler step occurs between taking the
> number and wrapping it. A forgotten decoded Ready reply or a discarded Taken wrapper
> stays in the table until explicit close/context end. Register a host-owned spawn
> supervisor/creation job before starting OS spawn. Teardown reaches that job even before
> a child exists. Any late-created child is terminated/reaped; native exit waits for every
> such job's completion, not only children already entered in the map. Request
> cancellation cannot recognize arbitrary Int resources, so it adds no hidden
> resource-specific compiler hook or v3 acknowledgement.
>
> **N9.5. Teardown is independent of i finals, GC and worker polling.** Normal entry end,
> stop, trap, host failure, abandoned async invocation and dispose all run the same
> idempotent host controller: close the start/delivery gate; cancel and detach request
> work; remove entries and revoke leases; schedule each OS cleanup once; mark ended. No
> Store borrow or i code is required. A creator racing teardown either commits before
> ending or closes its late result; it cannot insert into an ending table. Idle reactor
> waits are woken. Finite critical sections never wait for a worker executing Wasm.
> Resources outlive request Roots when appropriate, but cannot escape entry teardown
> through an independent Root/Owner. Deferred cleanup holds host data only, never a
> Store/context. General context-aware host registrations receive the invocation context
> at start; legacy callbacks may ignore it. Each future captures that immutable context,
> never a mutable “current run” pointer that a later invocation can retarget. Registration
> factories/options may be reused across tests; resource maps may not.
>
> **N10. One worker keeps the FIFO fast path.** No shared task queue, encode or scheduler
> lock. Portability stops at sends/results/shares still run; go never rejects for
> non-portability. N9 table ownership/binding rules apply on every worker count.
> Epoch-instrumented artifacts still contain checks on fallback to one worker; budget
> those checks separately rather than promise zero cost.
>
> **N11. One encoded format, host-specific byte transport.** Native keeps bytes in runner
> memory with private Wasm linear memories; it needs no shared Wasm allocator or threads
> feature for this. Browser workers use shared linear memory and the S7
> isolation/import-module gates. Same-worker paths retain references; remote decoding
> preserves identity, sharing and cycles (S7 D6/D9/I1–I4).
>
> **N12. A stop closes the whole run, including non-polling workers.** The first stop is
> recorded once with its original message. Close the start/claim/delivery gate in that
> transaction; later traps/stop attempts cannot replace it. Stopping invokes no i finals and resumes no pending i
> callbacks. An independent host controller performs N9.5, so a worker spinning with
> pending requests cannot defer host cleanup.
>
> Every user import, including synchronous services and externals, checks the entry
> context's stop gate before starting its host action. An import admitted before
> stop may finish its already-started OS effect; no later user action is admitted.
> Controller cleanup bypasses user admission and never calls i. This matters on
> native while child reaping delays final process exit: remaining Wasm may compute,
> but cannot print, start IO or deliver results after the stop. Such code is
> abandoned, not unwound through i final functions.
>
> - **Native executable:** report the first stop once, perform host teardown,
>   finish every registered spawn/reap job, then exit the process with failure. Do not
>   join spinning Wasm workers before exit. Their remaining private heaps and
>   process-local OS references die with the process. Immediate process exit
>   before teardown would skip Rust destructors and kill the child supervisor;
>   it is not the implementation. Detached file/stdin work is not joined (C D6).
>   Child cleanup may await natural exit if termination fails, as runtime §7.4
>   already permits; no bounded OS-cleanup duration is promised.
> - **A continuing Wasmtime host:** use epoch-instrumented code for multicore,
>   with a separate Engine interruption domain per simultaneously live entry
>   context, including its nested scheduler runs (an Engine clone is not a new domain). Before any Wasm, set trap-on-deadline
>   and deadline current+1, with no periodic ticks. Deadline setup and start
>   admission serialize with stop under the run gate, releasing it before
>   Wasm. The first stop advances only
>   that domain's epoch and wakes all reactors. A stopped-context gate forbids
>   fresh instantiation, deadline reset, resume/continue or any later Wasm call.
>   Thus a worker racing startup cannot arm itself after the stop. Join all
>   stopped workers and dispose their Stores before returning the run result.
>   A later run gets a fresh domain. Top-level initialization is under the same
>   gate and deadline; a startup stop tears down the partially made worker set.
>   Host-only child supervisors may outlive that result; they terminate/reap without the old run reactor or Store.
> - **Host calls:** epochs interrupt Wasm, not synchronous host code. A multicore
>   continuing host must give nonblocking or stop-aware synchronous imports,
>   host future polls and destructors;
>   otherwise choose one worker or a process boundary for that invocation.
>   Request-mode IO starters are short and their waits are cancellable host work.
>   Finite lock holds may delay stop; arbitrary user blocking imports cannot be
>   advertised as promptly interruptible. Native's controller can exit without
>   joining such a worker once resource teardown is safe.
> - **Browser:** do not claim Wasmtime epochs work in JS. Multicore requires
>   terminable dedicated workers for every executing store, with a host controller
>   and resource cleanup outside them. If worker 0 is the calling/main thread as
>   in the initial S7 route, a pure infinite loop cannot be terminated there;
>   that route must stay one-worker until the dedicated-worker entry route is
>   implemented. No callback/epoch re-entry or allocation from an interrupt is
>   needed for any of these rules.
>
> Reason: S7 D11/T17, spec §9.5/§11.7, and Wasmtime 50 epoch limitations (§9). Only
> scheduling and structured cancellation remain cooperative. Logical end of an entry and
> physical OS cleanup are separate; explicit Child.close waits, while a live host may
> retain a host-only reaper after a stopped entry returns.
>
> ### 3.1 General host-binding permissions (the forged-Int answer)
>
> Private raw constructors/fields are necessary API privacy, but insufficient authority.
> §9.7 permits another declaration of a namespace when it has no duplicate field in the
> compiled program. A module need not import i/net at all to ask for `i:net/v1.read`. An
> external alias is another potential path. Equal Ints are equal values: a kind check
> cannot distinguish a forged live Stream number from the original. Random numbers cannot
> make a mathematical guarantee here.
>
> Use the smallest exact closure: **a host grants a raw binding to a resolved import
> declaration, not to namespace spelling alone**. A grant identifies the trusted package
> content identity, module, host type/field (or external), wire signature and module
> artifact digest. Check every required declaration before instantiation/import
> coalescing; an unauthorized alias must not inherit a grant from an authorized
> declaration of the same Wasm name. This is a host permission, not a compile error for
> declaring that namespace. Hosts may deliberately grant another declaration; doing so
> grants raw table access within that context, including operations on known live numbers.
> The type system promises no unforgeability to such a grantee.
>
> The standard Rust/native/JS factories use this general grant mechanism for raw service
> declarations whose type names alone are exported; constructors, fields and live field
> values stay private under §2.4. Compiler/library resolution produces a trusted binding
> plan. i run/test pass it directly; native embeds it with the trusted program payload. An
> ilang embedder/JS factory must approve a compiler plan tied to the module digest and
> trusted package identities, or explicitly grant the needed raw bindings. A manifest's
> self-asserted module name is not evidence. A hand-written Wasm/manifest is not
> automatically trusted. Absent a trusted plan, the standard resource factory refuses its
> bindings; arbitrary custom imports remain the embedder's explicit authority.
> Re-exporting a public wrapper keeps the original grant; it does not grant a new raw
> declaration. Locally supplied ordinary i fakes need no host import and still work (N4).
>
> This does not reserve an IO namespace or teach the checker any library name. The same
> policy works for plugin storage, database sessions and credential services. It changes
> no set of language values or inference rule. It is an explicit host-loader extension
> before resource IO, not an already-existing feature or an optional secrecy
> recommendation. Keep the trusted plan outside the v3 wire/ilang.abi contract; if it is
> later put into mandatory ABI metadata, version it instead of expecting older v3 hosts to
> enforce it.
>
> ## 4. What is deferred
>
> - Cooperative stealing and idle-time publication are replaced, not alternative
>   implementations of D8. N7 needs no interrupt to encode.
> - Moving a started continuation remains future work. Lazy frame records contain
>   resume function references; moving them needs stable resume-code numbers,
>   distinct from D6's language function identity. Nothing here migrates them.
> - Shared GC, a second backend, component-model transport and lock-free CML are
>   not needed. The review found no usable shared WasmGC heap in Wasmtime 50;
>   that is dated evidence, not a prediction about future support.
> - Exact locked-state service signatures, browser allocation/reclamation,
>   quiescence proof and top-level worker entry are MC-D gates (multicore §7/§17).
>
> ## 5. Measurements and acceptance goals
>
> No scaling or packaging-size measurement is available. Calibrate these proposed numbers
> on the first implementation and keep the whole 1/2/4/8/16-worker curve:
>
> | Workload | Goal |
> |---|---|
> | Plaintext and JSON HTTP server | At least 0.8 × N one-core throughput through 8 cores, 0.6 × N at 16 |
> | Existing bench programs on one worker | Within 2% of the M:1 baseline; report epoch-artifact fallback separately |
> | Chunked Mandelbrot/spectral-norm using scope.share | At least 5.5× on 8 cores |
> | JSON against Node cluster and Gleam/Elixir servers | Record versions and throughput; aim not to fall below them |
> | Go net/http, skynet, cross-worker ping-pong, request p99 beside pure loops | Record; do not infer success from the chosen architecture |
>
> First measure encode/decode of a representative request task; then compare lock
> contention. The coordinator measures in the plan's quiet window. No performance numbers
> in this note are results.
>
> ## 6. Later choices
>
> If the lock dominates, split locks or use a measured parallel protocol. If encoding
> dominates, evaluate a shared-GC implementation. Keep the same public S7 semantics,
> identity, cancellation and lifetime tests through either change.
>
> ## 7. Order of work: build now and defer deliberately
>
> | Stage | Build now so it is not redone | What waits |
> |---|---|---|
> | R1 | Immutable published SplitMix and tagged paths exactly as runtime §4; no task/IO dependency | Nothing native-specific blocks R1 |
> | R2 request layer | Local Request/Ticket cells, rooted Owner accounting, one root hand-off, consuming take and cancellation seam; no resource recognition for Int | Native packaging, worker transport |
> | R2 scheduler | Complete task records, detached routing services, retained environment frames, silent pinning, typed keys and single Scheduler boundary; FIFO/CML semantics and R4a joins | Queue encoding/claim ownership, locked transport implementation and worker entry |
> | IO clock, before Fs | General §2.4 nominal field-privacy fix; host ResourceContext owner/controller, non-reusing Int allocator and trusted binding-plan/grant plumbing on Rust and JS; entry exit/stop/dispose hooks | Worker count, packaging layout, browser multicore |
> | IO fs/net/process | Private nominal resource values and raw Int signatures from first implementation; explicit wrappers, no raw-number injection exports, kind checks, leases, provisional/Ready transfer and host teardown; process supervisor | No intermediate externref services or fd-as-number adapter |
> | IO stdio | Per-source broker with context subscribers, copied output buffers/leases, compatibility printer, complete target matrix/tests | Preview-1 adapter project |
> | Native packaging after R2/IO | Split compile/load host configuration; compiler-free runner, authenticated payload/plan, exact artifact manifest, N4 validation and default build | Cross-target distribution/signing choices must be fixed before release |
> | MC after MC-D/G5 | N7 queue, N8 typed locked transactions, identity/cycle-safe codec, host transport/wake, claim/cancel reclamation, N12 domains/controller, quiescence and browser stop-safe entry | Started-task migration, lock-free optimization, shared GC |
>
> R1 and R2 do not wait for native packaging. Resource-bearing IO must not publish
> externref signatures and replace them later. Host lifetime/grant plumbing is IO work
> before Fs; it is not deferred to packaging or assigned silently to C. R2 needs one
> replaceable Scheduler boundary now, not a Rust scheduler or the old parallel CML
> protocol. The plan's hot-file order still controls merge order.
>
> ## 8. Adopted replacements
>
> 1. multicore D8: shared queue encoded at go replaces cooperative stealing.
> 2. runtime D9: private nominal Int wrappers and raw Int wire replace IO host
>    references, with explicit codecs allowed by unchanged spec §10.3.
> 3. multicore §7/MC wave: one lock replaces parallel-CML CAS claims.
> 4. i build defaults to native; explicit ilang/wasi remain.
>
> These were adopted by the user, not waiting for another approval. N4 rejects only
> required unsupplied imports. The detailed rules above resolve revision 2's unreviewed
> questions; remaining implementation gates are listed in §12.
>
> ## 9. Evidence: read versus run
>
> Code facts below were read in this lane at base 051f064. Line numbers refer to that
> commit. Older code citations in runtime/multicore retain their stated base.
>
> | Read fact | File and lines |
> |---|---|
> | Wasmtime is pinned to 50.0.0 with cranelift | crates/ilang-host/Cargo.toml:17 |
> | from_bytes builds a compiler-configured Engine and calls Module::new | crates/ilang-host/src/lib.rs:653–681 |
> | Host imports are keyed only by module/name; callbacks take arguments without an invocation context; instantiation binds from that map | crates/ilang-host/src/lib.rs:533–562, :708–734 |
> | Metadata records signatures, not trusted declaration provenance | crates/ilang-wasm/src/lower.rs:2649–2686 |
> | Host lowering has declaration module/span before making imports | crates/ilang-wasm/src/lower.rs:971–1001 |
> | Field inference currently enumerates constructors/labels without a visibility test; exported schemes require their module types to be exported | crates/ilang-check/src/types/infer.rs:1538–1634, :566–595 |
> | Host boundary rejects ordinary custom types with constructors | crates/ilang-check/src/resolve.rs:1651–1673 |
> | Namespace/field duplicate checking is program-wide, not ownership of a namespace | crates/ilang-check/src/resolve.rs:2502–2533 |
> | Every test creates a fresh instance but uses its Program host | crates/ilang-host/src/lib.rs:803–825 |
> | Futures drop outside the slot lock; drop_all clears all outstanding work | crates/ilang-host/src/requests.rs:174–191 |
> | Async abandonment marks instance unusable and drops requests; traps drop requests too | crates/ilang-host/src/lib.rs:1265–1285, :1372–1382 |
> | Host run shutdown does not join detached blocking work | crates/ilang-host/src/run.rs:15–24, :28–68 |
> | Current standard service only prints; resources here are still a design | crates/ilang-host/src/services.rs:44–70; lib/i/fs.i:1, lib/i/net.i:1, lib/i/process.i:1 |
> | fail.map_error accepts a thunk then mapper, used by explicit wrappers | lib/i/fail.i:27–32 |
>
> Wasmtime 50 source was also read in the local registry at
> `/home/agent/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/wasmtime-50.0.0/`:
> `src/config.rs:679–737` documents engine-global epochs, required deadlines and lack of
> interruption in blocking host code; `src/runtime/store.rs:1058–1116` defines relative
> deadlines and trapping. These are local source readings, not new runtime probes. One
> Engine per live interrupted entry context avoids shared-epoch cross-talk and
> overflow/rearming questions; caching across domains is MC work.
>
> I read the independent revision-1 report and its probes under /tmp/review-native,
> including wasmtime-probe/src/main.rs and wasmtime-results.txt. That reviewer ran
> runtime-only .cwasm loading, actual i start/main, idle-Store transfer and negative
> shared-GC probes. I did not rerun those builds. Neither that report nor this reading
> verifies N12, grants, real IO or the future scheduler. The concrete programs below are
> required implementation fixtures; they cannot execute on the still-empty IO/task
> modules. Only the small abstract sequence model described below was run in this
> adoption.
>
> ## 10. Replacement spec text and implementation regions
>
> Apply with runtime §10 and multicore §13, without renumbering spec sections. The other
> accepted text stays; these instructions supersede their older backend and resource
> clauses. They do not edit spec.md in this lane.
>
> **§2.4/§6.7 nominal privacy.** Keep the rule that a type in an exported function
> environment must itself be exported; exporting T without constructors does not export
> its fields. Add to §6.7:
>
> > Selecting a field of a nominal value requires the same constructor visibility
> > as taking it apart by a constructor pattern (§2.4). Exporting T without its
> > constructors permits neither field selection nor constructor update outside
> > its module. Structural record fields retain their own rules.
>
> This makes §6.7 explicitly honor §2.4, without a special rule for host services.
>
> **§9.7 and §10.1, general host permissions.** Add:
>
> > A host grants the implementation of an import; a namespace does not grant
> > authority by itself. A host may restrict a binding to trusted resolved
> > declarations and reject other declarations or external aliases of that name.
> > Declarations and ordinary i replacements remain legal under the usual rules.
> > A binding policy does not change the type of a service. The standard resource
> > hosts use trusted declaration grants as well as context and resource-kind
> > checks; raw resource numbers are not secret capabilities.
>
> **§9.7.1–§9.7.5.** Use runtime §7's resource definitions, private raw fields, public
> wrapper transformations, type-only service exports, field privacy, no-injection rules
> and stable errors. Include runtime §8.1's context, lease, provisional/Ready and teardown
> behavior. In particular File/Listener/Stream/Child wrap Int privately; close is
> idempotent, competing same-direction operations give busy, and a close winning the
> commit order settles pending operations with closed. Do not copy old opaque types.
>
> **§9.9.** Use runtime §6 and multicore §18's reconciled API rules. Add:
>
> > go does not reject a non-portable task. Its complete record is classified
> > silently at publication; a non-portable record stays on its home worker.
> > A started task and a host request never move. A local scope still pins all
> > its children even when their results are portable IO values.
>
> **§10.3.** Keep the value/error lists unchanged. Add:
>
> > A nominal resource value with an Int field does not cross the host boundary.
> > Its ordinary library wrapper passes only the Int to the raw service. The
> > outer Request/Result declaration rules add no ABI value type.
>
> **§11.2, §11.4 and §11.8.** Keep Int arithmetic and ordinary nominal-value
> identity/hash/text. No secret-number or special IO display rule is added. S7's hidden
> scheduler ids apply to task handles, not arbitrary host data.
>
> **§11.7/§11.9.** Retain first-stop, no-finals and S7 portability rules. Add:
>
> > A whole-run stop terminates all its workers even if a worker has no scheduler
> > call. The host reports the first stop once, prevents new work or delivery,
> > and tears down its requests and resource context independently of i final
> > functions. Another live run is unaffected. Deferred OS cleanup owns no
> > language instance; child processes are terminated and reaped under the
> > resource host's cleanup contract.
>
> **Appendix B.** Keep ABI v3 wire and H1–H14. The trusted binding plan is outside
> ilang.abi, bound to exact artifact bytes by the host/build trust path (§3.1). Native
> uses private linear memory, browser MC uses shared memory; N8 uses ordinary service
> imports, not new runtime exports. Resource-context teardown must cover status 0,
> stop/trap, host failure, async abandonment and disposal. §B.7 retains blocking imports
> and no new preview-1 adapters (runtime §8.2). An Int-based adapter still needs buffer
> access; a reusable fd is not its number.
>
> **CLI build target text.** Replace build's default target with native. Document explicit
> ilang/wasi, N4 required-import diagnostics and N2's artifact contract; i run/test remain
> in-process. No language entry typing changes for packaging.
>
> Implementation work (assign to the named stages; no code is changed here):
>
> | Stage | Files and regions anchored in the base |
> |---|---|
> | IO before Fs | General nominal field-privacy enforcement in crates/ilang-check/src/types/infer.rs:1538–1634 and tests; new ResourceContext/allocator/binding-plan modules beside crates/ilang-host/src/services.rs:44–70; context-aware registration adapters at lib.rs:552–582, options/instantiation grants at :611–634/:708–747; entry/trap/WaitGuard lifecycle at :803–834/:1265–1285/:1372–1382; cancellation slots at requests.rs:174–191 |
> | IO i values | lib/i/{fs,net,process}.i:1 replaced by private nominal wrappers/raw Int services, wrapper guards and no number-injection exports; clock.i/stdio.i:1 and runtime §11's IO assignments |
> | IO trust plumbing | Compiler resolved declaration sites in lower.rs:971–1001 and import/metadata construction at :2649–2686; create a separate trusted binding plan before import coalescing, not self-authenticating ABI fields; JS loader binding at bindings/javascript/ilang.mjs:784–849 and teardown at :496–556/:881–925; new standard factories/tests |
> | Native package | Factor host lib.rs:653–681 into compiler/runtime configurations; crates/ilang-host/Cargo.toml:17 feature split; crates/ilang-cli/src/lib.rs:89–115 build path and crates/ilang-cli/src/main.rs:44–51 target options; new runner/payload manifest and package tests |
> | R2/MC | runtime §11 and multicore §14 regions: keep one Scheduler boundary and complete records now; add shared byte services/codec/claims/one-lock transactions only in MC; no direct cross-Store GC access |
>
> ## 11. Concrete attacks and required tests
>
> Each expected result follows a named rule. These are design fixtures, not claims that
> the unimplemented modules compile today. Use host gates, OS counters and a watchdog for
> hangs; do not use sleep timing as the semantic oracle.
>
> **Stop beside a spin.** On two workers force the two children to start remotely:
>
> ```i
> (i/task)
>
> spin = () { spin() }
>
> @main
> main = () {
>   task.scope((s) {
>     task.go(s, spin)
>     task.go(s, () { panic("first") })
>     Nil
>   })
> }
> ```
>
> N12 requires one “first” message and no i finals. Native subprocess exits after
> host teardown without joining spin; a continuing host traps/joins only this
> run. A host fixture must test startup racing stop and forbid rearming after
> stop. A concurrent run doing 7+1 still returns 8; a subsequent test also returns
> 8. Shared Engine deadlines and resetting an already-stopped worker fail this.
>
> **Stop with outstanding work and a never-claimed reply.** Start a gated accept and
> spawn. Release spawn to provisional, then stop before reply delivery; repeat after
> decode Ready and before take, after take with its wrapper dropped, and after normal
> entry return. Use a process fixture sleeping until signalled. N9.4/N9.5 require exactly
> one close/termination request, eventual reaping, zero new delivery after stop and no i
> final. A late successful accept closes its OS socket rather than resurrecting the table.
> Native exit occurs after child reaping; the live-host result may precede host-only
> supervisor completion.
>
> **Close while another worker waits.** Arrange a read to own its lease, then close from
> another worker. Repeat for accept, File read and Child wait. Close wins:
> Error(NetError("read", "", "closed")) for the read; close gives Nil; a second close
> gives Nil. Completion wins first: retain the original bytes. A second concurrent read
> gives busy; a simultaneous write may proceed. These are N9.3/runtime §7 results, not
> arbitrary byte distribution under D1. Concurrent close callers cannot both own the
> cleanup; an already-removed alias gives Nil.
>
> **Forged number and namespace.** This module imports no standard net module:
>
> ```i
> (i/request)
>
> @host("i:net/v1", "request")
> Pretend { Pretend(read: (stream: Int, max_bytes: Int) -> Request(Result(BitArray, String))) }
>
> @main
> main = (raw ! Pretend) {
>   request.await(raw.read(1, 1))
> }
> ```
>
> Private exports alone do not block it. §3.1 denies automatic binding before any service
> start, whether 1 is absent or a live Stream number. A host may explicitly grant Pretend;
> then unknown/foreign/wrong-kind 1 gives closed, while a matching live number acts on
> that resource. That is the explicit raw grant, not a claim of unforgeability. Repeat
> with an external alias and two declarations coalesced to one import: every required
> declaration must be authorized. Also bind Pretend to ordinary fake functions: it runs
> with no import under N4. Public wrappers remain ordinary code; File(1) outside i/fs
> fails constructor privacy, and using a Listener as Stream fails nominal typing before
> any host call.
>
> **Two runs, exhaustion and hidden state.** In run A open number n, retain n in a
> host-only fixture, then end A; in run B create n+1 and issue raw operations on n. Expect
> closed/Nil, never the new resource (N9.1/N9.2). Repeat with two tests sharing standard
> host options and with a second export call on one instance. Set the allocator near an
> injected limit and create two provisional resources: the first gets the final fresh
> number; the second closes any OS result and stops for memory exhaustion. Nothing wraps
> to zero/one. Test JS above 2^53 and Rust above 2^64: exact Int round-trips and equality
> survive encoding. A restarted host is a different domain, not a persistence format for
> raw numbers.
>
> **Cannot be built / source conflict attacks.** A raw field returning File(Int) still
> fails §10.3; raw Int plus wrapper is required. No service field may need forall A;
> public wrappers are ordinary generalized bindings. Hiding a raw service type used in an
> exported environment would fail §2.4. Export only that type name, and reject `(raw !
> net.Net) { raw.read(1, 1) }` outside i/net by general constructor/field privacy; the
> current field checker needs the fix. An exported helper accepting a fake raw accept
> would forge Stream(guessed); the no-injection export rule in N9 removes it. A
> local-scope child returning Stream still stays local because the complete record
> includes that scope; a capture containing a Ticket pins it even when its Stream is
> portable. Key identity, cyclic closures, scope cancellation after claim/before start,
> self-rendezvous in choose, a losing mapper that panics and an inactive scope after
> encoding must retain S7/runtime results. Prepare bytes outside the lock, revalidate
> liveness inside, and never invoke the mapper under it. N8's typed prototype remains a
> real buildability gate.
>
> **Attack/fix record.** Initial attacks found real gaps: privacy did not prevent raw
> redeclarations; immediate native exit skipped resource/child cleanup; store-only
> teardown could not act while another worker spun; a public raw fake could launder a
> guessed number through a wrapper; in-flight spawns could escape exit and concurrent
> closes needed a shared cleanup receipt; public wrapper environments required type
> exports, and the current field checker could then expose their raw fields despite §2.4;
> bounded/resetting numbers and shared Engine epochs could target the next or another live
> run. Fixed by declaration grants, no public number-injection path, registered spawn
> jobs/cleanup receipts, type-only service exports with general field privacy, independent
> context teardown before exit, host-wide arbitrary Int allocation and isolated epoch
> domains. The browser main-thread spin was another real gap: MC must use terminable
> dedicated workers or fall back.
>
> Pass 1 after those fixes revisited forged live numbers, external aliases, public fake
> laundering and raw-field projection through an opaque exported service type, publication
> after cancellation, payload lifetime through decoding, provisional/Ready/Taken
> abandonment, duplicate close receipts, late spawn ownership, startup stop and nested
> Owner escape. No further serious defect was found in the revised rules. Pass 2
> independently varied two concurrent contexts, sequential export/test reuse, allocator
> exhaustion, synchronous blocked imports, local-scope complete records and cancellation/
> commit ordering. No further serious defect was found. These are manual sequence and
> code-reading attacks, not executable IO/multicore/OS-cleanup verification.
>
> An abstract Python sequence model was run through stdin in this lane: 1,154 reachable
> lifecycle states and 3,652 transitions passed the invariants of unique numbers, no
> insertion after context end, one cleanup owner and no native exit while a
> creation/reaping job remains. It models two contexts and at most one spawn per context,
> with two issued numbers and depth 14; it is not a proof for unbounded executions.
> Separate models passed all 120 serial orders of two reads/write/completion/close, six
> claim/cancel/start orders and six startup/stop orders. Rejected-rule witnesses included
> resetting number 1 into another run, exiting with an empty table but a still-creating
> job, and laundering a live number through a public raw fake. These checks validate only
> the abstractions; no new Rust, JS, i compiler, epoch or OS probe was run.
>
> The documentation audit checked the three allowed paths, 20 i code blocks, 18
> Markdown tables, balanced fences, table widths and an old-keyword scan. It also
> searched every stealing/CAS/host-reference/resource signature reference and
> checked the native/runtime/multicore section targets. This is a text check,
> not a parser/type-check of the proposed APIs. The sentence ledger's coverage
> and git diff --check are checked again before committing.
>
> ## 12. Risks and coordinator questions
>
> - **Who owns field privacy and binding-plan trust, and which plans does the SDK trust?**
>   Assign this before Fs, not
>   mistaken for private exports or an ABI field anybody can forge. Recommended:
>   IO owns the general §2.4 field fix and host plumbing in the plan's merge order;
>   use a compiler-to-host plan for CLI/native, digest-checked approved plan or explicit
>   grants for SDK; fail closed when absent.
> - **Which typed MC transaction/wake interface?** Recommended: ordinary request-mode byte
>   transaction/wake services, typed prototype and full CML/claim/quiescence review
>   at MC-D/G5. A mutex alone is not approval to implement an untyped Rust executor.
> - **May native exit before child reaping to meet a stop deadline?** Recommended: no; preserve explicit close's
>   reaping contract; native waits for reaping before exit, live hosts retain a
>   host-only supervisor. Do not promise a strict stop deadline for OS cleanup.
> - **Which browser worker-entry route supports forced stop?** Recommended: controller
>   on the main thread with all Wasm in dedicated terminable workers; until then
>   one-worker fallback, retaining S7's semantic rules.
> - **Which payload layout, signing/distribution policy and worker-count override?**
>   Recommended: one authenticated manifest/payload for the first host target,
>   packaged runners before fetch, and one documented host option shared by CLI
>   and embedder. Set release policy in native packaging; no R1/R2/IO API depends
>   on these names.
> - Encoding cost, lock contention, compiler-free runner size, exact epoch-domain
>   caching cost and all scaling goals remain unmeasured. Shared GC and started
>   continuation transfer remain future work.
>
> ## 13. Sentence change ledger
>
> The entries below preserve each replaced sentence/block verbatim (including code/table
> lines), with its replacement and reason. A block may contain several sentences that
> changed together; nothing inside the old/new quotations is a second active rule. This
> ledger is generated from the lane base to the final three notes, excluding the ledger
> itself. It includes native's adoption/status corrections as well as runtime/multicore
> changes.

- Reason: Honor spec §2.4 for exported environments and nominal privacy, using one general field-visibility fix before Fs. Close N9’s forged-Int paths with general declaration grants and no public raw-number injection; ordinary accessible service fakes stay legal. Adopt N9’s ordinary nominal Int wire values and explicit context, lease and cleanup ownership, preserving closed/busy and §10.3. Complete N12: first-stop gates, independent host teardown, native exit after child cleanup and isolated interruption in continuing hosts. Adopt N7: encode/classify the complete task record at go, pin silently, and serialize publication/claim/cancel ownership. Adopt N8’s one-lock backend while retaining the i scheduler, typed local storage and the accepted CML transaction semantics. Adopt N11: native uses runner byte storage/private memories; only the browser transport needs shared Wasm memory. Reconcile already-accepted runtime D5/R4a/Root–Owner and SplitMix rules; do not reopen their public semantics. Adopt N1–N4 and the reviewed artifact/loading constraints, keeping explicit ilang/WASI and rejecting only required unsupplied imports. Make the implementation/spec handoff explicit: prerequisites now, packaging and MC mechanisms in their own stages. Record adoption, concrete adversarial expectations and verification limits separately from implementation claims.

## 14. Revision-4 sentence change ledger

Base: 60f37f2260bf604e426ecb1869291ad9db2af96c. Each record keeps the complete
changed sentence block, including code/table lines, with old text, new text and
reason; additions/deletions use (none). These quotations are history, not active
rules. This ledger excludes its own generated records.

### R4-001 — docs/design/runtime.md, 60f37f2 line 301

Old sentence(s)/block:

> | Function | Type | Rule |
> |---|---|---|
> | `open_root` | `() -> Root ! Root` | Open an independent request run within the current entry context |
> | `close_root` | `(root: Root) -> Nil` | Close every live group of this root and the root itself; idempotent |
> | `open_owner` | `() -> Owner ! Root` | Register a group at the current root; changing Owner does not change that root |
> | `close_owner` | `(owner: Owner) -> Nil` | Cancel its outstanding work and remove the live group; idempotent |
> | `outstanding` | `(owner: Owner) -> List(Ticket)` | Current outstanding requests in this group, in start order |
> | `outstanding_root` | `(root: Root) -> List(Ticket)` | Outstanding requests of every group of this root, in start order |
> | `within(A,E,N)` | `(body: () -> A ? E ! N) -> A ? E ! Root \| N - Owner` | Fresh Owner environment; ensure close_owner on exit |
> | `ready(A)` | `(value: A) -> Request(A)` | A fresh completed request; available in pure fakes |
> | `ticket(A)` | `(r: Request(A)) -> Ticket` | The same underlying request identity; no answer access |
> | `state(A)` | `(r: Request(A)) -> RequestState(A)` | Reads do not consume the answer |
> | `take(A)` | `(r: Request(A)) -> A` | Ready becomes Taken atomically; other states stop |
> | `settled` | `(r: Ticket) -> Bool` | True for Ready, Cancelled or Taken |
> | `cancel` | `(r: Ticket) -> Nil` | Idempotent; cancels only Waiting, discards a Ready answer |
> | `await(A)` | `(r: Request(A)) -> A ! Await` | Wait, then take the Ready value |
> | `wait_any` | `(rs: List(Ticket)) -> Nil` | Root wait; return when any listed ticket settles |
> | `poll` | `() -> Nil` | Root hand-off with status 2, even with no requests |

New sentence(s)/block:

> | Function | Type | Rule |
> |---|---|---|
> | `open_root` | `() -> Root ! Root` | Open an independent request run within the current entry context |
> | `close_root` | `(root: Root) -> Nil` | Close every live group of this root and the root itself; idempotent |
> | `open_owner` | `() -> Owner ! Root` | Register a group at the current root; changing Owner does not change that root |
> | `close_owner` | `(owner: Owner) -> Nil` | Cancel its outstanding work and remove the live group; idempotent |
> | `count` | `(owner: Owner) -> Int` | Number of outstanding requests in this group |
> | `count_root` | `(root: Root) -> Int` | Number of outstanding requests in all groups of this root |
> | `settled_owner` | `(owner: Owner) -> Bool` | True exactly when count(owner) is zero |
> | `settled_root` | `(root: Root) -> Bool` | True exactly when count_root(root) is zero |
> | `cancel_all` | `(owner: Owner) -> Nil` | Cancel the group's outstanding work at this step; leave the group open |
> | `cancel_all_root` | `(root: Root) -> Nil` | Cancel outstanding work in all its groups at this step; leave them open |
> | `wait_none` | `(owner: Owner) -> Nil ! Await` | Await until this group's count is zero |
> | `wait_none_root` | `(root: Root) -> Nil ! Await` | Await until this root's count is zero |
> | `within(A,E,N)` | `(body: () -> A ? E ! N) -> A ? E ! Root \| N - Owner` | Fresh Owner environment; ensure close_owner on exit |
> | `ready(A)` | `(value: A) -> Request(A)` | A fresh completed request; available in pure fakes |
> | `ticket(A)` | `(r: Request(A)) -> Ticket` | The same underlying request identity; no answer access |
> | `state(A)` | `(r: Request(A)) -> RequestState(A)` | Reads do not consume the answer |
> | `take(A)` | `(r: Request(A)) -> A` | Ready becomes Taken atomically; other states stop |
> | `settled` | `(r: Ticket) -> Bool` | True for Ready, Cancelled or Taken |
> | `cancel` | `(r: Ticket) -> Nil` | Idempotent; cancels only Waiting, discards a Ready answer |
> | `await(A)` | `(r: Request(A)) -> A ! Await` | Borrow the producer, wait, then take the Ready value |
> | `await_owned(A)` | `(r: Request(A)) -> A ! Await` | Own a non-resource-producing operation; ensure ticket cancellation on exit |
> | `wait_any` | `(rs: List(Ticket)) -> Nil` | Root wait; return when any listed ticket settles |
> | `poll` | `() -> Nil` | Root hand-off with status 2, even with no requests |

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization. N-R2: distinguish owned non-creating operations from borrowed waits and ensure cancellation releases their leases; retain creator guards.

### R4-002 — docs/design/runtime.md, 60f37f2 line 345

Old sentence(s)/block:

> Every raw host start records its Owner group and that group's immutable root.
> close_owner cancels only its own group. close_root closes all groups registered
> at that root; no subgroup or old Owner binding escapes it. outstanding(owner)
> enumerates one cancellation group; outstanding_root(root) enumerates every group
> of the run, including groups made inside request.within and groups retained by
> go. Enumeration of a closed group/root gives []; opening a group under a closed
> Root stops with request: root is gone; a host start under a closed Owner stops
> with request: owner is gone, before making the ABI import. The registries retain
> only live roots/groups and outstanding cells; absence supplies the gone check.

New sentence(s)/block:

> Every raw host start records its Owner group and that group's immutable root.
> close_owner cancels only its own group. close_root closes all groups registered
> at that root; no subgroup or old Owner binding escapes it. All group operations
> include every worker, including groups made inside request.within and retained
> by go. Counts and readiness are atomic snapshots; a later start can change them.
> A closed group/root has count zero and is settled. cancel_all cancels the Waiting
> members at its linearization step, not subsequent starts or detached Ready values;
> close additionally refuses future starts. Opening a group under a closed Root
> stops with request: root is gone; a host start under a closed Owner stops with
> request: owner is gone, before making the ABI import. Only live registrations
> and outstanding cells are retained; absence supplies the gone check.
>
> There is no public outstanding/outstanding_root enumeration, including no
> worker-local variant. A Root/Owner operation never returns a Request or Ticket.
> A worker's private typed cell index is implementation storage, not a source API.
> Portable groups hold shared counts, revisions and cancellation state; completion
> and cancellation notifications carry stable ids to the home worker, which alone
> updates its cells and uses their ABI ids. No foreign GC cell or Ticket crosses go.
> For the same order of starts/answers/cancels, every group's answer is identical
> on every worker. Group cancellation is accepted atomically; it forbids later
> success publication for its selected members. Before returning, it applies each
> member's host cancellation action, including lease revocation, through a
> pre-registered host cancellation handle that needs no Store borrow or progress
> by the home worker. Run those actions outside the shared lock; they must obey
> the same nonblocking contract as local cancel. Detached OS effects/cleanup need
> not have finished. The home cell can reflect that transition lazily, but every
> state/settled/take/cancel/wait/delivery operation first reconciles shared terminal
> state, so a later local query observes Cancelled and cannot take a late answer.
> A home worker must not issue ABI cancel for an id the group action already
> retired. This is group control metadata, not a remotely usable Ticket. MC-D must
> prove the start-registration/cancel race and home-table retirement protocol;
> without this, it cannot claim placement-independent group operations.
>
> wait_none/wait_none_root use Await, so a task waits without pausing runnable
> siblings. They return immediately on zero, including for a closed group.
> Otherwise a private local Request(Nil) observation cell subscribes to the group
> and settles on its next zero observation. It has no host id, producer operation
> or Owner membership: a wait is not work that can complete itself. Its Ticket
> never leaves the observer's worker. Check-and-subscribe shares the registration
> lock so completion cannot be lost between checking and sleeping. A concurrent
> start before the zero observation is included; one after it does not undo the
> return. An ordinary ensure unregisters/cancels this local observer on waiter
> exit without cancelling the group's producers. These private cells use the
> same typed state/await machinery as requests; no foreign Ticket is manufactured.
> For progress accounting, an observation cell depends on the actual outstanding
> members of its target group, not on itself; add those members as explicitly
> borrowed work and deduplicate identities (§6.4). In particular it cannot keep
> an otherwise deadlocked empty group alive. Remote members are driven on their
> home workers; group changes reach the local observer through its wake route.
> The private Runner idle operation subscribes to group revisions for any change,
> combined with explicitly borrowed local tickets (§6.4), without enumerating
> foreign tickets. Its observation/check/subscription must be atomic too.

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization. N-R2: distinguish owned non-creating operations from borrowed waits and ensure cancellation releases their leases; retain creator guards. N-R3: count program producers and pending program delivery for deadlock, separate control wakes through ordinary Root/Owner accounting, and preserve ABI H4. N-R5: retain the owned-plus-explicitly-borrowed waitable set, without counting unrelated roots. N-R6: use the shared go/nested-run counter for paths and random derivation. Record the review source, exact regression expectations, phase/spec handoff and verification limits; preserve revision-3 history.

### R4-003 — docs/design/runtime.md, 60f37f2 line 385

Old sentence(s)/block:

> Root and Owner are ordinary services, not primitive types or built-in names.
> They use Key(Nil) and are portable; outstanding request values remain non-portable.
> Root/group registration and enumeration require no ABI import; closing live
> work uses only v3 cancel. Generic
> request groups also serve batch cancellation, request timeouts, and host plugin
> cleanup, independently of tasks. Request operations introduce no new error-row
> or environment polymorphism. Ticket
> cannot recover A; state only reads through the original Request(A). The existing
> value restriction prevents a fresh request from being generalized at multiple
> unrelated answer types. A cell's state transitions are Waiting -> Ready or
> Cancelled, and Ready -> Taken or Cancelled; there is no write of another A.

New sentence(s)/block:

> Root and Owner are ordinary services, not primitive types or built-in names.
> They use Key(Nil) and are portable; outstanding request values remain non-portable.
> Root/group registration and observation use internal accounting. Local Ticket
> cancel uses v3 cancel on its home worker; MC group cancellation uses the
> registered host cancellation actions above and retires the home id once. MC
> routes notifications through ordinary request-mode wake services, without new
> runtime imports. Generic
> request groups also serve batch cancellation, request timeouts, and host plugin
> cleanup, independently of tasks. Request operations introduce no new error-row
> or environment polymorphism. Ticket
> cannot recover A; state only reads through the original Request(A). The existing
> value restriction prevents a fresh request from being generalized at multiple
> unrelated answer types. A cell's state transitions are Waiting -> Ready or
> Cancelled, and Ready -> Taken or Cancelled; there is no write of another A.

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization. N-R3: count program producers and pending program delivery for deadlock, separate control wakes through ordinary Root/Owner accounting, and preserve ABI H4. N-R4: preserve runtime’s pure fresh-key events, validate stateful leaves, and derive portability from payloads/captures.

### R4-004 — docs/design/runtime.md, 60f37f2 line 401

Old sentence(s)/block:

> * `@host("ns", "async")` writes its fields with wire type `(P...) -> A ? E`.
>   Its actual i field type is `(P...) -> A ? E ! Await | Owner`. Conceptually its body
>   starts Request(Result(A,E)), calls request.await, then fail.from_result.
>   With no errors it starts Request(A) and awaits it. The attribute's type
>   expansion happens before ordinary inference and interface display. A user
>   declaration may not add a different field environment to this form.
> * Add `@host("ns", "request")`. Every field is `(P...) -> Request(T)`, with
>   no written errors or environment; its callable type adds `! Owner`. T is one ABI result type, or Result(A,E) with
>   A and E ABI-admissible as in §10.3. Its ABI signature is the same as the async
>   function with result A/errors E; no Request or Result is sent across the wire.
>   The call starts and returns a handle immediately. A raw field returning
>   `Request(Result(Nil,Never))` is allowed, though `Request(Nil)` is simpler.
> * No other occurrence of Request/Result is allowed as a boundary type. A
>   synchronous function, external, or host export cannot take or return Request.
>   Opaque references inside its A are valid, because only the decoded answer is
>   a host reference; Request itself never crosses the boundary.

New sentence(s)/block:

> * `@host("ns", "async")` writes its fields with wire type `(P...) -> A ? E`.
>   Its actual i field type is `(P...) -> A ? E ! Await | Owner`. Conceptually its body
>   starts Request(Result(A,E)), calls request.await_owned, then fail.from_result.
>   With no errors it starts Request(A) and uses the same owning helper. The attribute's type
>   expansion happens before ordinary inference and interface display. A user
>   declaration may not add a different field environment to this form.
> * Add `@host("ns", "request")`. Every field is `(P...) -> Request(T)`, with
>   no written errors or environment; its callable type adds `! Owner`. T is one ABI result type, or Result(A,E) with
>   A and E ABI-admissible as in §10.3. Its ABI signature is the same as the async
>   function with result A/errors E; no Request or Result is sent across the wire.
>   The call starts and returns a handle immediately. A raw field returning
>   `Request(Result(Nil,Never))` is allowed, though `Request(Nil)` is simpler.
> * No other occurrence of Request/Result is allowed as a boundary type. A
>   synchronous function, external, or host export cannot take or return Request.
>   Opaque references inside its A are valid, because only the decoded answer is
>   a host reference; Request itself never crosses the boundary.

Reason: N-R2: distinguish owned non-creating operations from borrowed waits and ensure cancellation releases their leases; retain creator guards.

### R4-005 — docs/design/runtime.md, 60f37f2 line 426

Old sentence(s)/block:

> Replace the C region as one change, including every caller identified in
> `emit/host.rs:23–52`. The module keeps a growable table and a free list. Each
> outstanding slot contains import index, ABI id, a fresh GC generation token,
> Owner group key, immutable Root key, and a reference to its request cell.
> Root/group indexes must remove a request on answer or cancel; completed cells
> are not retained by either index. Root enumeration spans every live group,
> while closing a group touches only its own slots. Normal entry-context exit
> retires every root/group registered in that context, including an independent
> root its caller forgot to close. Clearing requests at status 0 must clear those
> registrations too; a synchronous fake-only context uses the same registry
> epilogue without a host hand-off. Detached Ready handles retain their answers. Each import also keeps its source answer shape (plain A or
> Result(A,E)), independently of whether the ABI error list happens to be empty;
> Request(Result(A,Never)) must still decode Ok(A). A cell contains identity, generation, Waiting/Ready/Cancelled/Taken,
> and a decoded eqref answer. The id is a nonnegative i32 slot index; the table
> cannot have more than 2^31 outstanding entries. Exhaustion is a memory stop.
> A cell survives freeing its table slot. Language operations use the cell, and
> compare slot generation before cancel; they never act on a new occupant.

New sentence(s)/block:

> Replace the C region as one change, including every caller identified in
> `emit/host.rs:23–52`. The module keeps a growable table and a free list. Each
> outstanding slot contains import index, ABI id, a fresh GC generation token,
> Owner group key, immutable Root key, and a reference to its request cell.
> Root/group indexes must remove a request on answer or cancel; completed cells
> are not retained by either index. Root accounting spans every live group,
> while closing a group touches only its own slots. Keep count/revision and local
> cell indexing separate from the start; MC adds shared metadata and home-worker
> notifications without exposing an enumeration API. Normal entry-context exit
> retires every root/group registered in that context, including an independent
> root its caller forgot to close. Clearing requests at status 0 must clear those
> registrations too; a synchronous fake-only context uses the same registry
> epilogue without a host hand-off. Detached Ready handles retain their answers. Each import also keeps its source answer shape (plain A or
> Result(A,E)), independently of whether the ABI error list happens to be empty;
> Request(Result(A,Never)) must still decode Ok(A). A cell contains identity, generation, Waiting/Ready/Cancelled/Taken,
> and a decoded eqref answer. The id is a nonnegative i32 slot index; the table
> cannot have more than 2^31 outstanding entries. Exhaustion is a memory stop.
> A cell survives freeing its table slot. Ready fakes and group-observation cells
> have no host slot or ABI id; cancelling an observer only unregisters its wait.
> Group notifications settle it locally, and it counts only its producer
> dependencies for progress, never itself (§5.1). Language operations use the cell, and
> compare slot generation before cancel; they never act on a new occupant.

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization. Record the review source, exact regression expectations, phase/spec handoff and verification limits; preserve revision-3 history.

### R4-006 — docs/design/runtime.md, 60f37f2 line 495

Old sentence(s)/block:

> (none)

New sentence(s)/block:

> The ordinary helper for an owned operation that produces no resource is:
>
> ```i
> await_owned(A) = (r: Request(A)) -> A ! Await {
>   resource.ensure(
>     () { await(r) }
>     () { cancel(ticket(r)) }
>   )
> }
> ```
>
> This definition lives in i/request and uses its local function names. The final
> runs on success, error and task cancellation; after take, cancel is a no-op.
> request.await itself remains a borrowed wait and never takes producer ownership.
> File/Stream/Child wrappers and other non-creating IO wrappers use await_owned,
> including their close operations; resource creators use await_resource (§8.1).
> Cancelling a pending Stream read revokes its read lease before the cancelled
> wrapper finishes unwinding, so a following read cannot see busy from that
> abandoned request. Already consumed bytes or committed effects are not rolled
> back. A cancelled Child wait releases its wait lease without closing the Child;
> a cancelled blocking File operation instead closes that File as §8.1 requires.

Reason: N-R2: distinguish owned non-creating operations from borrowed waits and ensure cancellation releases their leases; retain creator guards. N-R5: retain the owned-plus-explicitly-borrowed waitable set, without counting unrelated roots.

### R4-007 — docs/design/runtime.md, 60f37f2 line 814

Old sentence(s)/block:

> When no task is runnable, scan terminal tickets first. Union
> outstanding_root(the_run_root) with Waiting tickets its tasks explicitly await,
> including borrowed tickets from another root. Run affiliation comes from every
> group's registered root, not from the current Owner or just the default group.
> If the union is nonempty, Runner.wait waits for one and then rescans. An
> unawaited raw request in any custom group prevents deadlock until it settles.
> Its detached Ready answer is retained only by its handle. If the set is empty,
> stop with `panic("task: deadlock")`, even if an unrelated root has work. Nested
> runs therefore neither count all instance requests nor miss their subgroups.
> Never return status 1 with zero outstanding requests, or busy-poll when no task
> can run. CPU code without suspension can monopolize M:1; a go-only loop may
> postpone children. No bounded wall-time guarantee is made for final functions.

New sentence(s)/block:

> When no task is runnable, scan terminal tickets first. The logical waitable set
> is the union of outstanding requests in every group of the run's stored program
> Root and Waiting tickets its tasks explicitly await, including borrowed tickets
> from another root on the same worker. Group-observer tickets contribute their
> underlying outstanding members, not the observer cell itself (§5.1). Deduplicate
> identities, including an owned request also explicitly awaited. This is a set of work, not a List(Ticket) that
> can be exported across workers: observe owned groups through their counts and
> revision notifications (§5.1); borrowed Ticket values stay local. If this union
> is nonempty, Runner.wait waits for a member to settle and then rescans. An
> unawaited raw request in any custom group prevents deadlock until it settles.
> Its detached Ready answer is retained only by its handle. If the set is empty
> and no task or completion delivery can run, stop with panic("task: deadlock"),
> even if an unrelated root has work. A nested run therefore includes an explicitly
> borrowed outer request, but not unrelated outer work.
>
> A dispatcher may own several ordinary Roots with separate Owners. It chooses
> which root's work it drives; no Root or request has a magic module-based class.
> The scheduler driver keeps a separate control Root/Owner for its wake requests,
> scoped to the run's driver lifetime and closed on every driver exit. It does not
> bind that Owner for user tasks or expose its wake tickets. The stored program
> Root plus explicit borrowed waits above alone justify program progress; the
> control root does not. This same separation serves a batch dispatcher, a timeout
> manager and a plugin notification loop. Both kinds still use the same Root/Owner
> registration, count, cancel and teardown rules. The distinction is which roots
> the ordinary driver selects, not a host import name or a compiler exception.
>
> Quiescence snapshots count runnable/claimed-but-unstarted/running tasks and
> pending completion-to-task or group-notification processing as work. Requests
> stay counted until their answer/cancel transition publishes that work atomically;
> there is no gap where both counts are zero. With no such work, zero members of
> the program waitable set means deadlock, whatever the control count. MC takes
> that snapshot and records the stop under the shared scheduler lock; a wake-only
> control delivery does not count as program work. At the ABI level every actual
> outstanding request, including a wake, still satisfies status 1's requirement.
> Never return status 1 with zero actual outstanding requests or busy-poll with no
> runnable task. A one-worker run needs no wake request. CPU code without suspension
> can monopolize M:1; a go-only loop may postpone children. No bounded wall-time
> guarantee is made for final functions.

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization. N-R3: count program producers and pending program delivery for deadlock, separate control wakes through ordinary Root/Owner accounting, and preserve ABI H4. N-R4: preserve runtime’s pure fresh-key events, validate stateful leaves, and derive portability from payloads/captures. N-R5: retain the owned-plus-explicitly-borrowed waitable set, without counting unrelated roots. Record the review source, exact regression expectations, phase/spec handoff and verification limits; preserve revision-3 history.

### R4-008 — docs/design/runtime.md, 60f37f2 line 992

Old sentence(s)/block:

> The resource-bearing parts of the File wrappers are explicit ordinary i:

New sentence(s)/block:

> The resource-bearing parts of the File wrappers are explicit ordinary i; all
> non-creating raw operations use request.await_owned (§5.4):

Reason: N-R2: distinguish owned non-creating operations from borrowed waits and ensure cancellation releases their leases; retain creator guards.

### R4-009 — docs/design/runtime.md, 60f37f2 line 1002

Old sentence(s)/block:

> open = (path, mode, raw ! Fs) {
>   File(fail.map_error(
>     () { await_resource(raw.open(path, mode_string(mode)), (n) {
>       request.await(raw.close(n))
>     }) }
>     (code) { FsError("open", path, code) }
>   ))
> }
>
> read = (File(number), max_bytes, raw ! Fs) {
>   request.await(raw.read(number, max_bytes)) |> {
>     Ok(bytes) -> bytes
>     Error(code) -> fail.raise(FsError("read", "", code))
>   }
> }
>
> close = (File(number), raw ! Fs) { request.await(raw.close(number)) }
> ```

New sentence(s)/block:

> open = (path, mode, raw ! Fs) {
>   File(fail.map_error(
>     () { await_resource(raw.open(path, mode_string(mode)), (n) {
>       request.await_owned(raw.close(n))
>     }) }
>     (code) { FsError("open", path, code) }
>   ))
> }
>
> read = (File(number), max_bytes, raw ! Fs) {
>   request.await_owned(raw.read(number, max_bytes)) |> {
>     Ok(bytes) -> bytes
>     Error(code) -> fail.raise(FsError("read", "", code))
>   }
> }
>
> close = (File(number), raw ! Fs) { request.await_owned(raw.close(number)) }
> ```

Reason: N-R2: distinguish owned non-creating operations from borrowed waits and ensure cancellation releases their leases; retain creator guards.

### R4-010 — docs/design/runtime.md, 60f37f2 line 1040

Old sentence(s)/block:

> ```i
> accept = (Listener(number), raw ! Net) {
>   Stream(fail.map_error(
>     () { await_resource(raw.accept(number), (n) {
>       request.await(raw.close_stream(n))
>     }) }
>     (code) { NetError("accept", "", code) }
>   ))
> }
>
> read = (Stream(number), max_bytes, raw ! Net) {
>   request.await(raw.read(number, max_bytes)) |> {
>     Ok(bytes) -> bytes
>     Error(code) -> fail.raise(NetError("read", "", code))
>   }
> }
>
> close_stream = (Stream(number), raw ! Net) {
>   request.await(raw.close_stream(number))
> }
> ```

New sentence(s)/block:

> ```i
> accept = (Listener(number), raw ! Net) {
>   Stream(fail.map_error(
>     () { await_resource(raw.accept(number), (n) {
>       request.await_owned(raw.close_stream(n))
>     }) }
>     (code) { NetError("accept", "", code) }
>   ))
> }
>
> read = (Stream(number), max_bytes, raw ! Net) {
>   request.await_owned(raw.read(number, max_bytes)) |> {
>     Ok(bytes) -> bytes
>     Error(code) -> fail.raise(NetError("read", "", code))
>   }
> }
>
> close_stream = (Stream(number), raw ! Net) {
>   request.await_owned(raw.close_stream(number))
> }
> ```

Reason: N-R2: distinguish owned non-creating operations from borrowed waits and ensure cancellation releases their leases; retain creator guards.

### R4-011 — docs/design/runtime.md, 60f37f2 line 1127

Old sentence(s)/block:

> ```i
> spawn = (program, args, raw ! Process) {
>   bytes = encode_args(args)
>   Child(fail.map_error(
>     () { await_resource(raw.spawn(program, bytes), (n) {
>       request.await(raw.close(n))
>     }) }
>     (code) { ProcessError("spawn", code) }
>   ))
> }
>
> wait = (Child(number), raw ! Process) {
>   request.await(raw.wait(number)) |> {
>     Ok(status) -> status
>     Error(code) -> fail.raise(ProcessError("wait", code))
>   }
> }
>
> close = (Child(number), raw ! Process) { request.await(raw.close(number)) }
> ```

New sentence(s)/block:

> ```i
> spawn = (program, args, raw ! Process) {
>   bytes = encode_args(args)
>   Child(fail.map_error(
>     () { await_resource(raw.spawn(program, bytes), (n) {
>       request.await_owned(raw.close(n))
>     }) }
>     (code) { ProcessError("spawn", code) }
>   ))
> }
>
> wait = (Child(number), raw ! Process) {
>   request.await_owned(raw.wait(number)) |> {
>     Ok(status) -> status
>     Error(code) -> fail.raise(ProcessError("wait", code))
>   }
> }
>
> close = (Child(number), raw ! Process) { request.await_owned(raw.close(number)) }
> ```

Reason: N-R2: distinguish owned non-creating operations from borrowed waits and ensure cancellation releases their leases; retain creator guards.

### R4-012 — docs/design/runtime.md, 60f37f2 line 1158

Old sentence(s)/block:

> Public wrappers unwrap those results, giving `? StdioError ! Stdio | request.Await | request.Owner`.
> read's bound/EOF and write's whole-byte/all-bytes rules are those above. stdout
> and stderr are distinct directions; one write per direction may be pending,
> with busy for a competing write. A cancelled write may have written a prefix.
> Flush functions are explicit. Also export `print(text: String)` and
> `print_error(text: String)`, which UTF-8 encode text plus one LF, write all bytes,
> and flush their stream; both can raise StdioError. No implicit text decoding,
> line buffering or newline translation is added. Legacy i/io still prints a
> String plus LF with the old no-error contract under `i:io/v1`.

New sentence(s)/block:

> Public wrappers use request.await_owned to unwrap those results, giving
> `? StdioError ! Stdio | request.Await | request.Owner`.
> read's bound/EOF and write's whole-byte/all-bytes rules are those above. stdout
> and stderr are distinct directions; one write per direction may be pending,
> with busy for a competing write. A cancelled write may have written a prefix.
> Flush functions are explicit. Also export `print(text: String)` and
> `print_error(text: String)`, which UTF-8 encode text plus one LF, write all bytes,
> and flush their stream; both can raise StdioError. No implicit text decoding,
> line buffering or newline translation is added. Legacy i/io still prints a
> String plus LF with the old no-error contract under `i:io/v1`.

Reason: N-R2: distinguish owned non-creating operations from borrowed waits and ensure cancellation releases their leases; retain creator guards.

### R4-013 — docs/design/runtime.md, 60f37f2 line 1586

Old sentence(s)/block:

> > i/task is a library implemented using i/control, i/key and i/request. A run
> > owns its tasks and scheduler state. Its implicit root permits local values.
> > Scopes constrain the lifetime of spawned work: none outlives its scope.
> > Values pass between tasks only through go captures/kept services, channels,
> > child results/errors and shares. Portable paths check core.portable even
> > with one worker: sends at the call, send events at construction, children
> > at completion, scope body errors at failure, shares at creation. Failure
> > stops with panic("task: value is not portable"). go never rejects for
> > non-portability; on multiple workers it silently classifies the complete task
> > record at go and publishes portable records. Other records stay local.
> > Local scopes/channels hold the maker's boundary only as a non-portability
> > witness. A Task/join event from a local scope holds that scope too. Every
> > handle checks run identity. Channels belong to the first scope in their
> > maker's chain, or the run; channels and shares are freed at scope exit.
> > Every run has the result of some serial order of task steps. Timing may
> > choose a sender, a race/select winner, which failures precede cancellation,
> > or whether a handle is used before its owner ends. These order-dependent
> > results may change between runs or worker counts. Tests must either avoid
> > that dependence or list the allowed outcomes. No task API observes workers.

New sentence(s)/block:

> > i/task is a library implemented using i/control, i/key and i/request. A run
> > owns its tasks and scheduler state. Its implicit root permits local values.
> > Scopes constrain the lifetime of spawned work: none outlives its scope.
> > Values pass between tasks only through go captures/kept services, channels,
> > child results/errors and shares. Portable paths check core.portable even
> > with one worker: sends at the call, send events at construction, children
> > at completion, scope body errors at failure, shares at creation. Failure
> > stops with panic("task: value is not portable"). go never rejects for
> > non-portability; on multiple workers it silently classifies the complete task
> > record at go and publishes portable records. Other records stay local.
> > Local scopes/channels hold the maker's boundary only as a non-portability
> > witness. A Task/join event from a local scope holds that scope too. Every
> > stateful handle checks run identity; events have fresh keys and sync checks
> > only their stateful leaves, including losing alternatives. Channels belong
> > to the first scope in their maker's chain, or the run; channels and shares are freed at scope exit.
> > Every run has the result of some serial order of task steps. Timing may
> > choose a sender, a race/select winner, which failures precede cancellation,
> > or whether a handle is used before its owner ends. These order-dependent
> > results may change between runs or worker counts. Tests must either avoid
> > that dependence or list the allowed outcomes. No task API observes workers.

Reason: N-R4: preserve runtime’s pure fresh-key events, validate stateful leaves, and derive portability from payloads/captures. Record the review source, exact regression expectations, phase/spec handoff and verification limits; preserve revision-3 history.

### R4-014 — docs/design/runtime.md, 60f37f2 line 1612

Old sentence(s)/block:

> **New §9.11.** Copy §5.1's public definitions, tables, state transitions,
> cancellation/wait/alias rules, and dynamic-context rule, including Owner groups. Copy §5.4's implicit
> adapter ownership rule, with the consuming take rule and reference to §9.7's
> resource guards. Exclude the discussion of compiler frame/table layout.

New sentence(s)/block:

> **New §9.11.** Copy §5.1's public definitions, tables, state transitions,
> cancellation/wait/alias rules, and dynamic-context rule, including portable
> Root/Owner count/readiness/cancel-all/wait-none operations and local observer
> semantics, with no Ticket enumeration. Copy §5.4's borrowed await and ordinary
> await_owned rules, with consuming take and reference to §9.7's resource guards.
> Exclude the discussion of compiler frame/table layout.

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization. N-R2: distinguish owned non-creating operations from borrowed waits and ensure cancellation releases their leases; retain creator guards. N-R5: retain the owned-plus-explicitly-borrowed waitable set, without counting unrelated roots.

### R4-015 — docs/design/runtime.md, 60f37f2 line 1800

Old sentence(s)/block:

> 1. K1 has landed on redesign at 578a17d; require it before R2 and preserve its
>    display.rs order K1 -> A2 Namer -> T1 -> T3. Do not work around the invariant
>    generalization fault by weakening key invariance or a public type scheme.
>    A1+A2 provides new syntax. B's implementation precedes R2 and its keep_env
>    re-entry changes. C implementation is already the base for this note.
> 2. R1 is independent of the new request primitives. Land it early on lane b
>    after B's retained work, for Std-3/property; it must not import Std-3.
> 3. Keep the ledger's hot-file order: lower.rs named regions C -> T1 -> R2 ->
>    T3 -> STR -> M2; infer.rs and type tables T1 -> T2 -> T3 -> R2 -> M2 -> M4.
>    Reconcile those by splitting R2's lower and infer changes into separate
>    commits; the overall R2 merge follows T3. No worker overwrites T3's new
>    lower regions. emit/host.rs, host ABI areas and §10.3 follow
>    C -> T1 (where assigned) -> R2 -> IO -> M2.
> 4. IO can prepare Rust services after C and draft i wrappers after A1+A2,
>    but its request-returning i modules merge after R2's request layer. Lane c
>    keeps clock -> fs -> net -> process -> stdio serial. JS factories are part
>    of the matching phase. Final registration/list edits happen once, in stdio.
> 5. R2 and IO then gate E2E-echo. Std-2 adds real time.now after clock lands.
>    M2 waits for the completed R2 behavior; compiler sandbox runs use a fixed
>    test Runner and no host services. MC keeps Root/Owner keys portable and gives
>    every worker the run's live-root/group registrations; closing a group coordinates
>    its requests across workers. No host request itself becomes portable.

New sentence(s)/block:

> 1. K1 has landed on redesign at 578a17d; require it before R2 and preserve its
>    display.rs order K1 -> A2 Namer -> T1 -> T3. Do not work around the invariant
>    generalization fault by weakening key invariance or a public type scheme.
>    A1+A2 provides new syntax. B's implementation precedes R2 and its keep_env
>    re-entry changes. C implementation is already the base for this note.
> 2. R1 is independent of the new request primitives. Land it early on lane b
>    after B's retained work, for Std-3/property; it must not import Std-3.
> 3. Keep the ledger's hot-file order: lower.rs named regions C -> T1 -> R2 ->
>    T3 -> STR -> M2; infer.rs and type tables T1 -> T2 -> T3 -> R2 -> M2 -> M4.
>    Reconcile those by splitting R2's lower and infer changes into separate
>    commits; the overall R2 merge follows T3. No worker overwrites T3's new
>    lower regions. emit/host.rs, host ABI areas and §10.3 follow
>    C -> T1 (where assigned) -> R2 -> IO -> M2.
> 4. IO can prepare Rust services after C and draft i wrappers after A1+A2,
>    but its request-returning i modules merge after R2's request layer. Lane c
>    keeps clock -> fs -> net -> process -> stdio serial. JS factories are part
>    of the matching phase. Final registration/list edits happen once, in stdio.
> 5. R2 and IO then gate E2E-echo. Std-2 adds real time.now after clock lands.
>    M2 waits for the completed R2 behavior; compiler sandbox runs use a fixed
>    test Runner and no host services. MC keeps Root/Owner keys portable and gives
>    every worker shared live-root/group counts, revisions and cancellation state;
>    closing a group coordinates home-worker cleanup without exporting Tickets.
>    Driver control roots remain separate from the program waitable set (§6.4).

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization. N-R2: distinguish owned non-creating operations from borrowed waits and ensure cancellation releases their leases; retain creator guards. N-R3: count program producers and pending program delivery for deadlock, separate control wakes through ordinary Root/Owner accounting, and preserve ABI H4. N-R5: retain the owned-plus-explicitly-borrowed waitable set, without counting unrelated roots. Record the review source, exact regression expectations, phase/spec handoff and verification limits; preserve revision-3 history.

### R4-016 — docs/design/runtime.md, 60f37f2 line 1836

Old sentence(s)/block:

> 1. [ ] Commit R2 replacement spec text; merge S7 primitive text, preserving §9.8.
> 2. [ ] Verify K1's invariant-generalization repair, then implement keys/variance/
>    value restriction, portable including cycles, and
>    keep_env with retained environment frames; run all S7 single-worker tests.
> 3. [ ] Implement Request/Ticket cells, Root/group registries and table replacement together with
>    per-cell answer decoding, raw/async host adapters, cancel and continue.
> 4. [ ] Prove the monomorphic keyed prototype by type-checking two channel element
>    types and heterogeneous child results. Reject any need for a cast or forall field.
> 5. [ ] Implement the i loop, per-task boundaries, rooted Owner groups, Runner/top,
>    task.keep and inferred-needs entry provisioning. Preserve synchronous exports
>    and suppress host progress polls when no task waits for IO. Include nested
>    progress-interest forwarding and guaranteed registration cleanup. Check O0/O2 WAT for retained frames.
> 6. [ ] Add scope chains/local witnesses, joins, body cancellation, shielded finals,
>    run-key checks, channel/share lifetime and error arbitration; add R4a
>    cancellation-priority joins and shielded dropped/cancelled-join stops.
> 7. [ ] Add CML registration/commit/map, combinators and deterministic script policy.
> 8. [ ] Integrate every async user host service through Await; require review
>    tests J/E/O, nested subgroup and borrowed-ticket cases, root poll fairness,
>    and synchronous callback/first-turn no-yield tests. Audit the host/control seam.
> 9. [ ] Run deadlock/cancel/fairness/alias/IO stress cases 200 times on Rust and JS,
>    related spec tests and full required verification; inspect WAT changes.

New sentence(s)/block:

> 1. [ ] Commit R2 replacement spec text; merge S7 primitive text, preserving §9.8.
> 2. [ ] Verify K1's invariant-generalization repair, then implement keys/variance/
>    value restriction, portable including cycles, and
>    keep_env with retained environment frames; run all S7 single-worker tests.
> 3. [ ] Implement Request/Ticket cells, Root/group registries and table replacement together with
>    per-cell answer decoding, raw/async host adapters, await_owned, cancel and continue.
>    Implement §5.1 group observations without public Ticket enumeration; atomic
>    subscription, local observer dependencies, terminal reconciliation and separate
>    program/control roots are part of this seam now; MC adds the remote host actions.
> 4. [ ] Prove the monomorphic keyed prototype by type-checking two channel element
>    types and heterogeneous child results. Reject any need for a cast or forall field.
> 5. [ ] Implement the i loop, per-task boundaries, rooted Owner groups, Runner/top,
>    task.keep and inferred-needs entry provisioning. Preserve synchronous exports
>    and suppress host progress polls when no task waits for IO. Include nested
>    progress-interest forwarding and guaranteed registration cleanup. Check O0/O2 WAT for retained frames.
> 6. [ ] Add scope chains/local witnesses, joins, body cancellation, shielded finals,
>    run-key checks, channel/share lifetime and error arbitration; add R4a
>    cancellation-priority joins and shielded dropped/cancelled-join stops.
> 7. [ ] Add CML registration/commit/map, combinators and deterministic script policy.
> 8. [ ] Integrate every async user host service through Await; require review
>    tests J/E/O, native §11.1 N-R1–N-R6, nested subgroup and borrowed-ticket cases, root poll fairness,
>    and synchronous callback/first-turn no-yield tests. Audit the host/control seam.
> 9. [ ] Run deadlock/cancel/fairness/alias/IO stress cases 200 times on Rust and JS,
>    related spec tests and full required verification; inspect WAT changes.

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization. N-R2: distinguish owned non-creating operations from borrowed waits and ensure cancellation releases their leases; retain creator guards. N-R3: count program producers and pending program delivery for deadlock, separate control wakes through ordinary Root/Owner accounting, and preserve ABI H4. N-R5: retain the owned-plus-explicitly-borrowed waitable set, without counting unrelated roots. Record the review source, exact regression expectations, phase/spec handoff and verification limits; preserve revision-3 history.

### R4-017 — docs/design/runtime.md, 60f37f2 line 1861

Old sentence(s)/block:

> 1. [ ] Clock: before Fs, add the shared ResourceContext, host-wide non-reusing Int
>    allocator and trusted binding grants (native §3.1) on Rust/JS; enforce §2.4 field privacy; test sequential
>    invocations, late creators, disposal and forced exhaustion. Clock deadlines fixed at start, both clocks, zero/negative/huge durations,
>    abortable timers and no timer keeping an ended Node run alive.
> 2. [ ] Fs: codecs/errors, whole-byte validation, all-byte writes, explicit flush,
>    private File(Int) codecs, raw Int wrappers, no number-injection exports, closed/busy leases,
>    cancelled blocking-file state, safe detached work and context teardown.
> 3. [ ] Net: numeric endpoints, ephemeral bind, read/write/half-close, busy leases,
>    private Listener/Stream(Int) codecs, provisional ownership, close-vs-wait
>    across workers and a portable-scope concurrent connection fixture.
> 4. [ ] Process: bounded entropy, UTF-8/framing, no shell expansion, inherited
>    streams, private Child(Int) codec, wait caching, terminate and reaping on every
>    ownership edge, including native exit after child cleanup.
> 5. [ ] Stdio: cancellable subscriber input, exact byte output, flush/error stream,
>    legacy i/io compatibility and context turnover; finish registration/spec lists
>    and B.7 with blocking Int adapters, not fd/externref shortcuts.
> 6. [ ] Run permission/error/resource/security and concurrency reviews; include
>    unsupported browser/preview-1 behavior and user-supplied host fakes.
> 7. [ ] Run end-to-end echo, then combined verification in the integration lane.

New sentence(s)/block:

> 1. [ ] Clock: before Fs, add the shared ResourceContext, host-wide non-reusing Int
>    allocator and trusted binding grants (native §3.1) on Rust/JS; enforce §2.4 field privacy; test sequential
>    invocations, late creators, disposal and forced exhaustion. Clock deadlines fixed at start, both clocks, zero/negative/huge durations,
>    abortable timers and no timer keeping an ended Node run alive.
> 2. [ ] Fs: codecs/errors, whole-byte validation, all-byte writes, explicit flush,
>    private File(Int) codecs, raw Int wrappers, no number-injection exports, closed/busy leases,
>    await_owned on non-creators, cancelled blocking-file state, safe detached work
>    and context teardown; verify the cancelled File alias gives closed.
> 3. [ ] Net: numeric endpoints, ephemeral bind, read/write/half-close, busy leases,
>    private Listener/Stream(Int) codecs, provisional ownership, close-vs-wait
>    across workers, the race-then-read lease test (native §11.1), and a
>    portable-scope concurrent connection fixture.
> 4. [ ] Process: bounded entropy, UTF-8/framing, no shell expansion, inherited
>    streams, private Child(Int) codec, wait caching, terminate and reaping on every
>    ownership edge, await_owned for wait/terminate/close, including native exit after child cleanup.
> 5. [ ] Stdio: cancellable subscriber input, exact byte output, flush/error stream,
>    legacy i/io compatibility and context turnover; finish registration/spec lists
>    and B.7 with blocking Int adapters, not fd/externref shortcuts.
> 6. [ ] Run permission/error/resource/security and concurrency reviews; include
>    unsupported browser/preview-1 behavior and user-supplied host fakes.
> 7. [ ] Run end-to-end echo, then combined verification in the integration lane.

Reason: N-R2: distinguish owned non-creating operations from borrowed waits and ensure cancellation releases their leases; retain creator guards. N-R6: use the shared go/nested-run counter for paths and random derivation. Record the review source, exact regression expectations, phase/spec handoff and verification limits; preserve revision-3 history.

### R4-018 — docs/design/runtime.md, 60f37f2 line 1890

Old sentence(s)/block:

> | Test | Expected result and rule |
> |---|---|
> | Re-evaluate word on the same Generator, and split twice without a draw | Equal (word,next) pairs and equal pairs of generators; D1/§4 immutable data |
> | int(g,0), int(g,-1), at_path(g,[0]) | InvalidBound(0), InvalidBound(-1), InvalidPath([0]); §4 validation, no draw |
> | int(g,1) | 0 and one word consumed; §4 requires at least one word even for bound 1 |
> | Stub words for n=10: W-1, then 7 | Reject W-1, give 7 with two draws; §4 rejection rule |
> | Split initial root vs child path [1] | Their histories contain S(1) vs P(1), not equal generators; §4 domain tags (not an assertion that words must differ) |
> | `task.random() == task.random()` in one task | True; D1 and S7 rule 11; no draw counter in Scheduler |
> | Published SplitMix seed 1337, first three words | 0xb5c19e300e8b07b3, 0xd600e0e216c0ac76, 0xc54efc3b3cc5af29; §4.1 pinned source vectors |
> | Request ready(7); state, take, state, take | Ready(7), 7, Taken, panic request: taken; §5.1 one transfer |
> | Take Int and String request handles stored together only as tickets | Read each original typed handle correctly; Ticket has no answer operation; D2 |
> | Cancel an old cell, reuse its slot, cancel the old alias again | New request remains Waiting and receives its own answer; D4 cell identity/generation |
> | Two same-import requests answered in reverse order | Each original typed handle returns its own value; §5.3 per-cell decoding |
> | Wait on A, host answers B first | B remains Ready, wait on A continues; §5.1 requested-set semantics |
> | Cancel after decode but before an IO await resumes | One close of the provisional socket/file, zero user-body uses; §8.1 Ready/Taken guard |
> | Local/portable scope child returns File/Listener/Stream/Child | Both join; portable is True under D9; a Request/Ticket result still stops under D7 |
> | Portable send_event of a continuation in a choose with always(1) | Stops at event construction, even though it would lose; S7 rule 2 |
> | `sync(choose([send_event(c,1), receive_event(c)]))` with no other task | Deadlock; §6.4 same registration cannot rendezvous with itself |
> | Two ready alternatives with side-effecting map callbacks | First alternative commits; exactly its callback runs in the syncing task; §6.4 |
> | `task.scope((s) { task.go(s, () { 1 })  2 })` | 2, only after child completion; R3; body result is independent of child's result |
> | `both(() { 1 }, () { "x" })`; map_concurrently([3,1], id) | (1,"x") and [3,1]; §6.1 typed separate handles/argument order |
> | Failed child while body joins an unstarted sibling | Error of failed child, body final ran, no deadlock; R2/R4 |
> | Two failures already observed before cancel, paths [1,1] and [1] | Error at [1]; R2 prefix/path order, not first observed error |
> | go A(yield; raise A), go B(raise B), body waits | B on production FIFO; §6.4 go does not yield parent, A yields behind B |
> | `task.scope((s) { s })` then go on returned s | task: go outside its scope; R1 |
> | Join from outer-scope sibling, or channel from another run | task: join outside its scope; task: handle of another run; R1 |
> | Channel returned from a finished scope then receive | task: channel is gone; R3; also test an outer waiter present at scope end |
> | Share returned then get after scope exit | task: shared value is gone; R3 |
> | Cancelled final yields, awaits fake clock, opens scope and rendezvous | Final completes, then original scope error; never boundary is not active; R4 |
> | Parent fake Clock used after child's third resume and in its final | Fake result survives; keep_env plus retained frame, D16 workaround |
> | Nested wait_flag while an outer sibling sets the flag | Both complete, no global hang; §6.5 Runner wait delegates to outer task |
> | Rebind Runner with top(seed) inside a task, then nested run | New seed, preserved nested prefix/outer wait route; §6.5 contextual top |
> | Nested raw request in request.within, with no task waiters; tasks block on channel | Wait for the request in that run's Root, then deadlock; §6.4 all-group root enumeration |
> | Infinite yield task plus an outstanding accept | Conditional status-2 polls for the waiting accept task deliver completion and its handler gets a turn; §6.4 |
> | Pure task path/yield/join operations with no host requests | No host poll, status 2, or macrotask yield; task entry can return status 0 immediately; D5/§6.4 |
> | test with script selecting nonexistent path [99] | NotRunnable([99]) of type ScheduleError after cleanup; §6.5 |
> | test with real pending timer rather than a fake | task: test needs fake IO; D10; fixed seed alone does not fix host time |
> | Net read(0); fs write with partial byte; process NUL argument | Typed invalid errors with the named operation; §7 validation |
> | Stream write of N bytes followed by peer EOF | Peer receives the exact N-byte prefix before EOF; §7.3 write-all/half-close |
> | Two concurrent stream reads; read concurrent with write | busy for second read; opposite directions work; §7.3 leases |
> | Close File/Stream/Child twice | Nil both times, underlying release once; §7/§8 idempotence |
> | Close while another worker awaits read/accept/wait | closed if close commits first; prior committed answer stays; §8.1 leases |
> | Stop with outstanding accept and forgotten Ready spawn, then run another test | No i finals; empty new table, fresh numbers and host-owned child cleanup; §8.1/native N12 |
> | Redeclare raw namespace or external alias | No automatic binding; local fake still works; native §3.1 |
> | `(raw ! net.Net) { raw.read(1, 1) }` outside i/net | Constructor/field visibility error under §2.4/§7, despite the exported type name |
> | Force a small allocator limit | Memory stop before insertion, never reuse; native N9.2 |
> | Process args codec for ["a","é"] | Bytes `<<2,0,0,0,1,0,0,0,97,2,0,0,0,195,169>>`; §7.4 UTF-8 framing |
> | Missing env variable; browser raw TCP | ProcessError(...,"not_found"); NetError(...,"unsupported"); §7 and target matrix |
> | WASI pure tasks/random; WASI requiring private Fs | Pure calculations work; report blocking Int imports requiring a buffer-aware adapter; §8.2/B.7, no fd identity reuse |
> | @main needs user State with no binding vs standard Scheduler/Runner/Await/Root/Owner | State is an entry compile error; standard needs are provisioned without changing the type; async classification follows D5, not mere use of any request helper |
> | Existing async user service, not an IO namespace, used in two children | Overlap via Await, same request table as IO; D3, no namespace rule |

New sentence(s)/block:

> | Test | Expected result and rule |
> |---|---|
> | Re-evaluate word on the same Generator, and split twice without a draw | Equal (word,next) pairs and equal pairs of generators; D1/§4 immutable data |
> | int(g,0), int(g,-1), at_path(g,[0]) | InvalidBound(0), InvalidBound(-1), InvalidPath([0]); §4 validation, no draw |
> | int(g,1) | 0 and one word consumed; §4 requires at least one word even for bound 1 |
> | Stub words for n=10: W-1, then 7 | Reject W-1, give 7 with two draws; §4 rejection rule |
> | Split initial root vs child path [1] | Their histories contain S(1) vs P(1), not equal generators; §4 domain tags (not an assertion that words must differ) |
> | `task.random() == task.random()` in one task | True; D1 and S7 rule 11; no draw counter in Scheduler |
> | Published SplitMix seed 1337, first three words | 0xb5c19e300e8b07b3, 0xd600e0e216c0ac76, 0xc54efc3b3cc5af29; §4.1 pinned source vectors |
> | Request ready(7); state, take, state, take | Ready(7), 7, Taken, panic request: taken; §5.1 one transfer |
> | Take Int and String request handles stored together only as tickets | Read each original typed handle correctly; Ticket has no answer operation; D2 |
> | Cancel an old cell, reuse its slot, cancel the old alias again | New request remains Waiting and receives its own answer; D4 cell identity/generation |
> | Two same-import requests answered in reverse order | Each original typed handle returns its own value; §5.3 per-cell decoding |
> | Wait on A, host answers B first | B remains Ready, wait on A continues; §5.1 requested-set semantics |
> | Cancel after decode but before an IO await resumes | One close of the provisional socket/file, zero user-body uses; §8.1 Ready/Taken guard |
> | Local/portable scope child returns File/Listener/Stream/Child | Both join; portable is True under D9; a Request/Ticket result still stops under D7 |
> | Portable send_event of a continuation in a choose with always(1) | Stops at event construction, even though it would lose; S7 rule 2 |
> | `sync(choose([send_event(c,1), receive_event(c)]))` with no other task | Deadlock; §6.4 same registration cannot rendezvous with itself |
> | Two ready alternatives with side-effecting map callbacks | First alternative commits; exactly its callback runs in the syncing task; §6.4 |
> | `task.scope((s) { task.go(s, () { 1 })  2 })` | 2, only after child completion; R3; body result is independent of child's result |
> | `both(() { 1 }, () { "x" })`; map_concurrently([3,1], id) | (1,"x") and [3,1]; §6.1 typed separate handles/argument order |
> | Failed child while body joins an unstarted sibling | Error of failed child, body final ran, no deadlock; R2/R4 |
> | Two failures already observed before cancel, paths [1,1] and [1] | Error at [1]; R2 prefix/path order, not first observed error |
> | go A(yield; raise A), go B(raise B), body waits | B on production FIFO; §6.4 go does not yield parent, A yields behind B |
> | `task.scope((s) { s })` then go on returned s | task: go outside its scope; R1 |
> | Join from outer-scope sibling, or channel from another run | task: join outside its scope; task: handle of another run; R1 |
> | Channel returned from a finished scope then receive | task: channel is gone; R3; also test an outer waiter present at scope end |
> | Share returned then get after scope exit | task: shared value is gone; R3 |
> | Cancelled final yields, awaits fake clock, opens scope and rendezvous | Final completes, then original scope error; never boundary is not active; R4 |
> | Parent fake Clock used after child's third resume and in its final | Fake result survives; keep_env plus retained frame, D16 workaround |
> | Nested wait_flag while an outer sibling sets the flag | Both complete, no global hang; §6.5 Runner wait delegates to outer task |
> | Rebind Runner with top(seed) inside a task, then nested run | New seed, preserved nested prefix/outer wait route; §6.5 contextual top |
> | Nested raw request in request.within, with no task waiters; tasks block on channel | Wait for the request in that run's Root, then deadlock; §6.4 all-group root accounting |
> | Infinite yield task plus an outstanding accept | Conditional status-2 polls for the waiting accept task deliver completion and its handler gets a turn; §6.4 |
> | Pure task path/yield/join operations with no host requests | No host poll, status 2, or macrotask yield; task entry can return status 0 immediately; D5/§6.4 |
> | test with script selecting nonexistent path [99] | NotRunnable([99]) of type ScheduleError after cleanup; §6.5 |
> | test with real pending timer rather than a fake | task: test needs fake IO; D10; fixed seed alone does not fix host time |
> | Net read(0); fs write with partial byte; process NUL argument | Typed invalid errors with the named operation; §7 validation |
> | Stream write of N bytes followed by peer EOF | Peer receives the exact N-byte prefix before EOF; §7.3 write-all/half-close |
> | Two concurrent stream reads; read concurrent with write | busy for second read; opposite directions work; §7.3 leases |
> | Close File/Stream/Child twice | Nil both times, underlying release once; §7/§8 idempotence |
> | Close while another worker awaits read/accept/wait | closed if close commits first; prior committed answer stays; §8.1 leases |
> | Stop with outstanding accept and forgotten Ready spawn, then run another test | No i finals; empty new table, fresh numbers and host-owned child cleanup; §8.1/native N12 |
> | Redeclare raw namespace or external alias | No automatic binding; local fake still works; native §3.1 |
> | `(raw ! net.Net) { raw.read(1, 1) }` outside i/net | Constructor/field visibility error under §2.4/§7, despite the exported type name |
> | Force a small allocator limit | Memory stop before insertion, never reuse; native N9.2 |
> | Process args codec for ["a","é"] | Bytes `<<2,0,0,0,1,0,0,0,97,2,0,0,0,195,169>>`; §7.4 UTF-8 framing |
> | Missing env variable; browser raw TCP | ProcessError(...,"not_found"); NetError(...,"unsupported"); §7 and target matrix |
> | WASI pure tasks/random; WASI requiring private Fs | Pure calculations work; report blocking Int imports requiring a buffer-aware adapter; §8.2/B.7, no fd identity reuse |
> | @main needs user State with no binding vs standard Scheduler/Runner/Await/Root/Owner | State is an entry compile error; standard needs are provisioned without changing the type; async classification follows D5, not mere use of any request helper |
> | Existing async user service, not an IO namespace, used in two children | Overlap via Await, same request table as IO; D3, no namespace rule |

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization. N-R2: distinguish owned non-creating operations from borrowed waits and ensure cancellation releases their leases; retain creator guards. N-R3: count program producers and pending program delivery for deadlock, separate control wakes through ordinary Root/Owner accounting, and preserve ABI H4. N-R4: preserve runtime’s pure fresh-key events, validate stateful leaves, and derive portability from payloads/captures. N-R5: retain the owned-plus-explicitly-borrowed waitable set, without counting unrelated roots. N-R6: use the shared go/nested-run counter for paths and random derivation. Record the review source, exact regression expectations, phase/spec handoff and verification limits; preserve revision-3 history.

### R4-019 — docs/design/runtime.md, 60f37f2 line 2029

Old sentence(s)/block:

> **Raw request deadlock and nesting (initial assessment).** Ignoring unawaited requests would declare
> deadlock while host work was outstanding, against S7 rule 17. Enumerating the
> whole instance would make a nested run wait for unrelated outer work, against
> rule 5. The first draft tried to fix this with Owner groups and per-run
> enumeration, but missed custom groups as the later review showed. The corrected
> Root affiliation and all-group enumeration are in §5.1/§6.4. The group capability is portable; request cells remain non-portable. This is
> also useful to ordinary non-task batch/timeout libraries.

New sentence(s)/block:

> **Raw request deadlock and nesting (initial assessment).** Ignoring unawaited requests would declare
> deadlock while host work was outstanding, against S7 rule 17. Enumerating the
> whole instance would make a nested run wait for unrelated outer work, against
> rule 5. The first draft tried to fix this with Owner groups and per-run
> enumeration, but missed custom groups as the later review showed. The corrected
> Root affiliation and all-group count/notification rules are in §5.1/§6.4. The group capability is portable; request cells remain non-portable. This is
> also useful to ordinary non-task batch/timeout libraries.

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization. N-R3: count program producers and pending program delivery for deadlock, separate control wakes through ordinary Root/Owner accounting, and preserve ABI H4. N-R5: retain the owned-plus-explicitly-borrowed waitable set, without counting unrelated roots.

### R4-020 — docs/design/runtime.md, 60f37f2 line 2129

Old sentence(s)/block:

> For service inference, a go body needing only Scheduler needs no parent Await/
> Runner, a body needing a custom Clock keeps that need, and a test nested under
> a caller still needs Runner. The revised schemes express exactly these cases.
> A public ordinary function needing Request(Int) needs no privileged module
> import or type declaration. RequestState's unlabeled Ready pattern agrees with
> spec §7.1. Owner's Key(Nil) is portable while a ticket returned by outstanding
> is not. There is no static conversion that can read a Ticket as a Request(Int).

New sentence(s)/block:

> For service inference, a go body needing only Scheduler needs no parent Await/
> Runner, a body needing a custom Clock keeps that need, and a test nested under
> a caller still needs Runner. The revised schemes express exactly these cases.
> A public ordinary function needing Request(Int) needs no privileged module
> import or type declaration. RequestState's unlabeled Ready pattern agrees with
> spec §7.1. Owner's Key(Nil) is portable while Request/Ticket values are not; group
> operations expose no ticket enumeration. There is no static conversion that can read a Ticket as a Request(Int).

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization.

### R4-021 — docs/design/runtime.md, 60f37f2 line 2157

Old sentence(s)/block:

> 1. **Dropped join in a shielded final: confirmed.** At aa20548, R2 dropped the
>    child, R4 shielded its joining final, and no completion arm could produce
>    its A/E. The control code only reactivates the scheduling boundary
>    (`emit/control.rs:260–295`); it supplies no task completion. R4a now resolves
>    that protocol hole and §10.1 gives the exact S7 sentence changes.
> 2. **Universal async and first-turn polling: confirmed.** The inferred env is
>    already available to entry_point (`types/infer.rs:405–429`) and lower's
>    entry_async (`lower.rs:916–938`). The JS call/callAsync guards explicitly
>    separate synchronous and async signatures (`bindings/javascript/ilang.mjs:965–981`);
>    status 2 takes a macrotask at `:894–902`. My old D5 and unconditional
>    first-turn poll would therefore break synchronous callbacks. D5 and §6.4
>    now preserve them and gate host polling on outstanding requests. The review
>    executed the comparator baseline; I independently read those paths, but did
>    not repeat that execution against a new implementation.
> 3. **Custom groups missed by idle accounting: confirmed.** The old within made
>    a group absent from the default Owner's table; the old idle union counted
>    neither it nor an unawaited request in it. Root is now separate from Owner,
>    and the idle union enumerates all groups of the run. Progress-interest tickets
>    affect polling only, never substitute for all-group deadlock accounting.
> 4. **Request-mode Owner effect omitted from replacement §9.7: confirmed.** The
>    old §5.2 added Owner while the replacement paragraph did not. Both now give
>    the callable type `(P...) -> Request(T) ! request.Owner` before inference;
>    ordinary field-function copying/display therefore sees the same effect.

New sentence(s)/block:

> 1. **Dropped join in a shielded final: confirmed.** At aa20548, R2 dropped the
>    child, R4 shielded its joining final, and no completion arm could produce
>    its A/E. The control code only reactivates the scheduling boundary
>    (`emit/control.rs:260–295`); it supplies no task completion. R4a now resolves
>    that protocol hole and §10.1 gives the exact S7 sentence changes.
> 2. **Universal async and first-turn polling: confirmed.** The inferred env is
>    already available to entry_point (`types/infer.rs:405–429`) and lower's
>    entry_async (`lower.rs:916–938`). The JS call/callAsync guards explicitly
>    separate synchronous and async signatures (`bindings/javascript/ilang.mjs:965–981`);
>    status 2 takes a macrotask at `:894–902`. My old D5 and unconditional
>    first-turn poll would therefore break synchronous callbacks. D5 and §6.4
>    now preserve them and gate host polling on outstanding requests. The review
>    executed the comparator baseline; I independently read those paths, but did
>    not repeat that execution against a new implementation.
> 3. **Custom groups missed by idle accounting: confirmed.** The old within made
>    a group absent from the default Owner's table; the old idle union counted
>    neither it nor an unawaited request in it. Root is now separate from Owner,
>    and the idle union accounts for all groups of the run without returning Tickets. Progress-interest tickets
>    affect polling only, never substitute for all-group deadlock accounting.
> 4. **Request-mode Owner effect omitted from replacement §9.7: confirmed.** The
>    old §5.2 added Owner while the replacement paragraph did not. Both now give
>    the callable type `(P...) -> Request(T) ! request.Owner` before inference;
>    ordinary field-function copying/display therefore sees the same effect.

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization. N-R3: count program producers and pending program delivery for deadlock, separate control wakes through ordinary Root/Owner accounting, and preserve ABI H4. Record the review source, exact regression expectations, phase/spec handoff and verification limits; preserve revision-3 history.

### R4-022 — docs/design/runtime.md, 60f37f2 line 2304

Old sentence(s)/block:

> Attack O with two custom groups, then close only one. outstanding_root still
> counts the other. Close the whole root and no group escapes its cancellation.
> After a result becomes Ready, closing the group/root does not discard the
> handle's answer. Rebind an outer Owner inside a nested run, then call within:
> open_owner uses the nested Root, so the new subgroup cannot inherit the outer
> Owner's affiliation. Deliberately starting under an already-made outer Owner
> still uses that original group, whose tickets may be borrowed explicitly.
> None of these cases uses instance-wide request count to decide nested deadlock.
> Also make two shielded finals join one another after both cancellations are
> accepted: each join is ready to stop, rather than waiting for the other unwind.
> Keeping outcome separate from cleanup supplies that result without returning a
> value early. No serious new rule conflict found in this pass.

New sentence(s)/block:

> Attack O with two custom groups, then close only one. count_root still
> counts the other. Close the whole root and no group escapes its cancellation.
> After a result becomes Ready, closing the group/root does not discard the
> handle's answer. Rebind an outer Owner inside a nested run, then call within:
> open_owner uses the nested Root, so the new subgroup cannot inherit the outer
> Owner's affiliation. Deliberately starting under an already-made outer Owner
> still uses that original group, whose tickets may be borrowed explicitly.
> None of these cases uses instance-wide request count to decide nested deadlock.
> Also make two shielded finals join one another after both cancellations are
> accepted: each join is ready to stop, rather than waiting for the other unwind.
> Keeping outcome separate from cleanup supplies that result without returning a
> value early. No serious new rule conflict found in this pass.

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization. N-R3: count program producers and pending program delivery for deadlock, separate control wakes through ordinary Root/Owner accounting, and preserve ABI H4. N-R5: retain the owned-plus-explicitly-borrowed waitable set, without counting unrelated roots.

### R4-023 — docs/design/multicore.md, 60f37f2 line 78

Old sentence(s)/block:

> **D10. Order comes from the task tree, never from time.** Each task has a **path**: the path of the task that called `go` plus the index of that `go` among all the `go` calls of that task (1, 2, …), whatever scope each went to. It is an ordinary `i/task` value (`task.path()`). Random splitting, handle ids, logs and traces use it. Paths are compared in **path order**: element by element, and a path comes before every longer path that starts with it (`[1] < [1, 1] < [2]`). A scope's error is the failure of the task with the **lowest path** among the scope's body and the children of the scope that failed. The body's path is a prefix of the path of every child, so the body comes first. `s.go`, and a join of a child of `s`, may be called only by a task whose **chain** holds `s` (part 6); the chain of a task changes only by its own steps, so this is fixed by the program, not by time. `both` and `map_concurrently` give results in argument order. *Reason:* these make results depend only on which children failed, not on which failed first, so one worker and N workers agree whenever the same children fail. Paths are unique, so no two children compare equal, also when children of `s` call `s.go` themselves.

New sentence(s)/block:

> **D10. Order comes from the task tree, never from time.** Each task has a **path**: the path of the task that called `go` plus the index of that `go` among that task’s `go` calls and nested runs counted together (1, 2, …), whatever scope each went to. It is an ordinary `i/task` value (`task.path()`). Random splitting, handle ids, logs and traces use it. Paths are compared in **path order**: element by element, and a path comes before every longer path that starts with it (`[1] < [1, 1] < [2]`). A scope's error is the failure of the task with the **lowest path** among the scope's body and the children of the scope that failed. The body's path is a prefix of the path of every child, so the body comes first. `s.go`, and a join of a child of `s`, may be called only by a task whose **chain** holds `s` (part 6); the chain of a task changes only by its own steps, so this is fixed by the program, not by time. `both` and `map_concurrently` give results in argument order. *Reason:* these make results depend only on which children failed, not on which failed first, so one worker and N workers agree whenever the same children fail. Paths are unique, so no two children compare equal, also when children of `s` call `s.go` themselves.

Reason: N-R5: retain the owned-plus-explicitly-borrowed waitable set, without counting unrelated roots. N-R6: use the shared go/nested-run counter for paths and random derivation.

### R4-024 — docs/design/multicore.md, 60f37f2 line 144

Old sentence(s)/block:

> **P5. Host requests stay home.** A host request value (host-concurrency.md §6.4, designed by R2) holds an id in the request table of its instance (H1 there). It is not portable (part 3.1). So only tasks on its home worker can wait on it, poll it or cancel it, and a send of it on a portable path stops. On one worker the same send stops in the same way, so a program cannot see that ids are per instance.

New sentence(s)/block:

> **P5. Host requests stay home.** A host request value (host-concurrency.md §6.4, designed by R2) holds an id in the request table of its instance (H1 there). It is not portable (part 3.1). Only tasks on its home worker can use that Request/Ticket to wait, poll or cancel; a send on a portable path stops. Portable Root/Owner operations instead act on group metadata, with host cancellation actions and home-cell reconciliation as runtime §5.1 specifies; they do not expose or move a Ticket. On one worker the same send stops in the same way, so a program cannot see that ids are per instance.

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization.

### R4-025 — docs/design/multicore.md, 60f37f2 line 226

Old sentence(s)/block:

> - `scheduler.detached` is a top-level record of constant functions that stop with `panic: task: no scheduler`. It is portable. Binding it before `keep_env` puts it, not the parent's scheduler, in the kept array, so the parent's scheduler boundary is never kept or captured. Inside, `! Scheduler = own` binds the real one, so `detached` is never called.
> - `s.spawn(start)` makes the task record (part 3.3), with the handle of the scope `s`.
> - The worker loop calls `start(own)` inside the task's new boundary (D5), with the `Scheduler` whose suspends go to that boundary. The environment statement is inside the task, in a frame that is kept, so it stays with the task's paused part (§9.4 rule 3).
> - Kept services may still be non-portable, for example a generator's `Yield` service, or the `Assert` of `assertion.catch`, which hold a boundary. Every service bound at the place is kept, also one that `body` does not need, so a task started under such a service is pinned by P1. That costs parallelism only, and it cannot be seen. When `body` uses that service, suspending to that boundary from the task stops with `control: boundary is not active` on any number of workers, because the task runs from the loop (D5), and the loop does not run inside that boundary's body.
> - **Runs and nested runs.** A **run** is one call of `task.run`: one loop, its tasks, and the handles they make. Nothing stops i code from calling `task.run` where a run is already going on, and the type of `task.run` cannot forbid it. So the rules are these, and they are the same on every host and every worker count:
>   - **Which run uses N workers.** Only the run at the program's entry (open question 4) may use more than one worker. Every other run is a **nested run**: a one-worker FIFO loop on the stack of the code that called `task.run`. Its tasks are never remotely claimed. When it is called from inside a task, it runs inside that task's boundary body, so the whole nested run is part of the calling task, which is started and never moves (P2).
>   - **The run service.** `task.run` needs one service of `i/task` (its name is R's; here `Runner`), and that is how it learns where it is, with no test for "is a service bound". Inside every task, the scheduler binds `Runner` to one that belongs to that task. The entry supplies the initial Runner from inferred needs (runtime D5); task.top(seed) uses that current Runner and replaces only its seed, retaining route and prefix. A run takes three things from `Runner`: its **path prefix**, its **root seed**, and its **wait**.
>   - **Paths.** At the top the prefix is `[]`. In a task, it is the task's path plus the index of this `task.run` among the task's `go` calls and runs, counted together, so the paths of two runs never meet, and two sibling tasks that each start a nested run get different paths and different generators (part 5).
>   - **Waits.** When a run has no task that can run but has outstanding host requests, it waits through `Runner`. At the top, that is the one root wait of host-concurrency §6.4 (status 1). In a task, the nested run hands its outstanding requests to the outer run and suspends **only its calling task** to the outer `Scheduler`, so the whole nested run pauses as one task, and the outer run's other tasks go on, on one worker as on N; the outer run resumes the calling task when one of those requests ends. So a nested run that waits for a host flag that another outer task sets ends on one worker too. A nested run never returns status 1 from the caller's stack.
>   - **Deadlock.** A nested run with no task that can run and no outstanding request of its own stops with `panic: task: deadlock`. No outer task can wake it, because outer handles cannot be used in it (next point).
>   - **Handles belong to their run.** Every channel, scope, Task, event and `Shared` handle holds the key of the run that made it. Using a handle in a task of another run (a send, a receive, a `sync`, a `go`, a join, a `get`) stops with `panic: task: handle of another run`, also when the handle is portable. So `task.run(() { scope((s) { c = channel.new()  s.go(() { c.send(1) })  task.run(() { c.receive() }) }) })` stops at `c.receive()` on any number of workers. Values that are not handles pass into and out of a nested run as captures and as its result, as with any function call.
>   - **Boundaries.** A nested run inside a generator body at the top (`collect(() { task.run(() { scope((s) { s.go(() { yield(1) }) }) }) })`) runs inside that boundary body, so by §9.4 rule 1 a `yield` from its tasks succeeds and pauses the whole nested run. That is the same on every host, because that run is not the entry run and uses one worker. A kept function that an outer task made (D4), called inside its own nested run, suspends the outer task with the nested run inside it, which is also the same everywhere. Track R documents both.
> - Host services given at an entry point are records of constant functions (static ids), so they are portable, and a remotely claimed task uses the remote claimant's own imports. The root environment's `Assert` closure must be a constant with a static id too (part 3.4); at c69515c it is not. §9.7 gets the rule that a host service behaves the same whichever worker calls it.

New sentence(s)/block:

> - `scheduler.detached` is a top-level record of constant functions that stop with `panic: task: no scheduler`. It is portable. Binding it before `keep_env` puts it, not the parent's scheduler, in the kept array, so the parent's scheduler boundary is never kept or captured. Inside, `! Scheduler = own` binds the real one, so `detached` is never called.
> - `s.spawn(start)` makes the task record (part 3.3), with the handle of the scope `s`.
> - The worker loop calls `start(own)` inside the task's new boundary (D5), with the `Scheduler` whose suspends go to that boundary. The environment statement is inside the task, in a frame that is kept, so it stays with the task's paused part (§9.4 rule 3).
> - Kept services may still be non-portable, for example a generator's `Yield` service, or the `Assert` of `assertion.catch`, which hold a boundary. Every service bound at the place is kept, also one that `body` does not need, so a task started under such a service is pinned by P1. That costs parallelism only, and it cannot be seen. When `body` uses that service, suspending to that boundary from the task stops with `control: boundary is not active` on any number of workers, because the task runs from the loop (D5), and the loop does not run inside that boundary's body.
> - **Runs and nested runs.** A **run** is one call of `task.run`: one loop, its tasks, and the handles they make. Nothing stops i code from calling `task.run` where a run is already going on, and the type of `task.run` cannot forbid it. So the rules are these, and they are the same on every host and every worker count:
>   - **Which run uses N workers.** Only the run at the program's entry (open question 4) may use more than one worker. Every other run is a **nested run**: a one-worker FIFO loop on the stack of the code that called `task.run`. Its tasks are never remotely claimed. When it is called from inside a task, it runs inside that task's boundary body, so the whole nested run is part of the calling task, which is started and never moves (P2).
>   - **The run service.** `task.run` needs one service of `i/task` (its name is R's; here `Runner`), and that is how it learns where it is, with no test for "is a service bound". Inside every task, the scheduler binds `Runner` to one that belongs to that task. The entry supplies the initial Runner from inferred needs (runtime D5); task.top(seed) uses that current Runner and replaces only its seed, retaining route and prefix. A run takes three things from `Runner`: its **path prefix**, its **root seed**, and its **wait**.
>   - **Paths.** At the top the prefix is `[]`. In a task, it is the task's path plus the index of this `task.run` among the task's `go` calls and runs, counted together, so the paths of two runs never meet, and two sibling tasks that each start a nested run get different paths and different generators (part 5).
>   - **Waits.** When a run has no task that can run but its owned-plus-borrowed program waitable set is nonempty, it waits through `Runner`. At the top, that is the one root wait of host-concurrency §6.4 (status 1). In a task, the nested run registers its owned group interests and explicitly borrowed local tickets with the outer run and suspends **only its calling task** to the outer `Scheduler`, so the whole nested run pauses as one task, and the outer run's other tasks go on, on one worker as on N; the outer run resumes the calling task when one of those requests ends. So a nested run that waits for a host flag that another outer task sets ends on one worker too. A nested run never returns status 1 from the caller's stack.
>   - **Deadlock.** Use runtime §6.4's owned-plus-borrowed waitable set: outstanding work in every group of the run's stored program Root, union Waiting tickets explicitly awaited by its tasks, including borrowed outer-root requests on the same worker, deduplicated by identity; private group-observer tickets expand to their outstanding members (runtime §5.1). With no runnable task or pending program delivery and an empty set, stop with `panic: task: deadlock`; unrelated outer-root work does not prevent it. A borrowed outer request can wake a nested run. Observe owned groups by counts/revisions, not by returning foreign Tickets (runtime §5.1).
>   - **Stateful handles belong to their run; events are descriptions.** Every channel, scope, Task and `Shared` handle holds its run key. Using those stateful handles in another run stops with `panic: task: handle of another run`, also when portable. An Event has a fresh identity key and need not have a creating run: always/never/map/choose need no Scheduler. At sync, validate the run of every leaf that refers to a channel, Task or scope, including losing alternatives; a pure leaf has no run restriction. Payloads and callback captures determine event portability, including local witnesses; there is no blanket portable-event promise (runtime §6.1/§6.4). So `task.run(() { scope((s) { c = channel.new()  s.go(() { c.send(1) })  task.run(() { c.receive() }) }) })` stops at `c.receive()` on any number of workers. Values that are not handles pass into and out of a nested run as captures and as its result, as with any function call.
>   - **Boundaries.** A nested run inside a generator body at the top (`collect(() { task.run(() { scope((s) { s.go(() { yield(1) }) }) }) })`) runs inside that boundary body, so by §9.4 rule 1 a `yield` from its tasks succeeds and pauses the whole nested run. That is the same on every host, because that run is not the entry run and uses one worker. A kept function that an outer task made (D4), called inside its own nested run, suspends the outer task with the nested run inside it, which is also the same everywhere. Track R documents both.
> - Host services given at an entry point are records of constant functions (static ids), so they are portable, and a remotely claimed task uses the remote claimant's own imports. The root environment's `Assert` closure must be a constant with a static id too (part 3.4); at c69515c it is not. §9.7 gets the rule that a host service behaves the same whichever worker calls it.

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization. N-R3: count program producers and pending program delivery for deadlock, separate control wakes through ordinary Root/Owner accounting, and preserve ABI H4. N-R4: preserve runtime’s pure fresh-key events, validate stateful leaves, and derive portability from payloads/captures. N-R5: retain the owned-plus-explicitly-borrowed waitable set, without counting unrelated roots. N-R6: use the shared go/nested-run counter for paths and random derivation. Record the review source, exact regression expectations, phase/spec handoff and verification limits; preserve revision-3 history.

### R4-026 — docs/design/multicore.md, 60f37f2 line 244

Old sentence(s)/block:

> - `task.path()` gives the task's path: a list of `go` indices from the root task. The path is portable data. It does not depend on time, worker or queue claims.
> - `task.path()` needs no suspend: the `Scheduler` that the loop gives a task holds the task's path as data, so reading it is a field read.
> - i has no mutable values (§11.1), so a per-task draw counter would have to live in the scheduler loop, and every draw would be a suspend to the scheduler; `i/random` would need `! Scheduler`, and code with no scheduler, such as `main = () { … }` with no `task.run`, a top-level value or a macro, could not draw. So Random is a **value**, passed explicitly: a generator is immutable data (the pinned SplitMix state and tagged derivation history of runtime §4); `random.int(g, 10)` gives a number and the next generator, and `random.split(g)` gives two derived generators under that published algorithm. `i/random` does not need `i/task`.
> - `task.random()` gives the generator of the current task, a pure function of the root seed and `task.path()`. So a child's numbers do not depend on when or where it runs, and a `go` does not change the parent's generator. Calling `task.random()` twice in one task gives the same generator; code that needs more splits it. `go` itself does not know about Random.
> - **The root seed** is part of the run service (part 4). At the top, the program gives it: `task.top(seed)` takes a generator, made from a host entropy service when the program has one, or from a fixed seed (`random.from_seed(n)`), which needs no service. The contextual entry default is random.from_seed(0) (runtime D5), and task.top(seed) replaces only that seed while preserving the Runner route/prefix. There is no automatic entropy draw, so the type of `task.run` needs no host service, and a macro, which has no host services (`meta.md` §8), gives a fixed seed and gets the same expansion on every run. A nested run in a task takes the seed of its outer run, and its paths start with the calling task's path (part 4), so its generators differ from every other task's.
> - **Split and paths never meet.** A generator is a key and a **stream**: a list of steps, where a step is either a path step `P(i)` or a split step `S(i)`, and a path step is never equal to a split step. `task.random()` at path `[i1, …, in]` gives the stream `[P(i1), …, P(in)]`; `random.split(g)` gives the streams of `g` followed by `S(1)` and by `S(2)`; numbers come from the pinned published SplitMix split tree in runtime.md §4, which replaces the earlier counter-based PRF sketch. So the first half of `random.split(task.random())` in the task at path `[]` is the stream `[S(1)]`, and the child at path `[1]` has `[P(1)]`: they differ. Different tagged derivation addresses remain distinct histories; no promise of disjoint outputs or perfect statistical independence is made. Runtime.md §4 fixes the exact algorithm and encoding.
> - Channel, scope and task handles are made of a path-based id (the path of the task that made them and a counter in that task) and the keys that type their state (D13). So `==` on handles is "the same channel" and survives copies, and nothing in them depends on the worker except key ids, which only `==` sees.

New sentence(s)/block:

> - `task.path()` gives the task's path: a list of combined `go`/nested-run indices from the root task. The path is portable data. It does not depend on time, worker or queue claims.
> - `task.path()` needs no suspend: the `Scheduler` that the loop gives a task holds the task's path as data, so reading it is a field read.
> - i has no mutable values (§11.1), so a per-task draw counter would have to live in the scheduler loop, and every draw would be a suspend to the scheduler; `i/random` would need `! Scheduler`, and code with no scheduler, such as `main = () { … }` with no `task.run`, a top-level value or a macro, could not draw. So Random is a **value**, passed explicitly: a generator is immutable data (the pinned SplitMix state and tagged derivation history of runtime §4); `random.int(g, 10)` gives a number and the next generator, and `random.split(g)` gives two derived generators under that published algorithm. `i/random` does not need `i/task`.
> - `task.random()` gives the generator of the current task, a pure function of the root seed and `task.path()`. So a child's numbers do not depend on when or where it runs, and a `go` does not change the parent's generator. Calling `task.random()` twice in one task gives the same generator; code that needs more splits it. `go` itself does not know about Random.
> - **The root seed** is part of the run service (part 4). At the top, the program gives it: `task.top(seed)` takes a generator, made from a host entropy service when the program has one, or from a fixed seed (`random.from_seed(n)`), which needs no service. The contextual entry default is random.from_seed(0) (runtime D5), and task.top(seed) replaces only that seed while preserving the Runner route/prefix. There is no automatic entropy draw, so the type of `task.run` needs no host service, and a macro, which has no host services (`meta.md` §8), gives a fixed seed and gets the same expansion on every run. A nested run in a task takes the seed of its outer run, and its paths start with the calling task's path (part 4), so its generators differ from every other task's.
> - **Split and paths never meet.** A generator is a key and a **stream**: a list of steps, where a step is either a path step `P(i)` or a split step `S(i)`, and a path step is never equal to a split step. `task.random()` at path `[i1, …, in]` gives the stream `[P(i1), …, P(in)]`; `random.split(g)` gives the streams of `g` followed by `S(1)` and by `S(2)`; numbers come from the pinned published SplitMix split tree in runtime.md §4, which replaces the earlier counter-based PRF sketch. So the first half of `random.split(task.random())` in the task at path `[]` is the stream `[S(1)]`, and the child at path `[1]` has `[P(1)]`: they differ. Different tagged derivation addresses remain distinct histories; no promise of disjoint outputs or perfect statistical independence is made. Runtime.md §4 fixes the exact algorithm and encoding.
> - Channel, scope and task handles are made of a path-based id (the path of the task that made them and a counter in that task) and the keys that type their state (D13). So `==` on handles is "the same channel" and survives copies, and nothing in them depends on the worker except key ids, which only `==` sees.

Reason: N-R5: retain the owned-plus-explicitly-borrowed waitable set, without counting unrelated roots. N-R6: use the shared go/nested-run counter for paths and random derivation. Record the review source, exact regression expectations, phase/spec handoff and verification limits; preserve revision-3 history.

### R4-027 — docs/design/multicore.md, 60f37f2 line 254

Old sentence(s)/block:

> - A scope returns only after all its children have ended, including children that are being unwound.
> - **Chains.** The **chain** of a scope `r` is `r` followed by the chain of the task that opened `r`, as it was when `r` was opened. The **chain** of a task, at a moment, is the scopes it has opened whose bodies it is still running (innermost first), followed by the chain of the scope it was started in (empty for a run's root task). A task's chain changes only when that task itself enters or leaves a scope body, so it is fixed by the task's own steps, not by time or placement. Every scope in a task's chain has not ended: the scopes it has opened are still running their bodies, the scope it was started in waits for it, and each scope in that scope's chain was in the opener's chain when it was opened, so by the same argument it cannot end before the opener leaves it. The scheduler keeps each task's chain as a list of scope ids, so the check is a list search.
> - **Who may `go`.** `s.go` may be called only by a task whose chain holds `s`. Otherwise it stops with `panic: task: go outside its scope`, whether or not `s` has ended. So a `go` never meets an ended scope. Examples that stop: `h = scope((s) { s })  h.go(() { 1 })` (after the body returns, `s` is no longer in the root task's chain); `scope((u) { scope((s) { u.go(() { task.yield()  s.go(() { 1 }) }) }) })` (the `u` child was started in `u`, whose chain does not hold `s`); `scope((outer) { c = channel.new()  outer.go(() { h = c.receive()  h.go(() { 1 }) })  scope((s1) { c.send(s1) }) })` (the same). Each stops on any number of workers. A scope handle is portable and may be sent anywhere, but only tasks inside the scope may use it to start tasks. A `go` on a scope that is cancelling makes an unstarted task that is dropped at once; whether a `go` comes before or after the cancel is part of the order of D1.
> - **Who may join.** A join of the Task handle of a child of `s` (and a `sync` of a join event made from it) may be done only by a task whose chain holds `s`. Otherwise it stops with `panic: task: join outside its scope`. A joiner still cancellable by `s` is unwound before observing a dropped/cancelled outcome; a shielded joiner, including protected final work, stops with `panic: task: join of a cancelled task` immediately when drop/cancellation is accepted, without waiting for target cleanup (runtime R4a). If R later adds a way to cancel one task while its scope goes on, a join of that task must complete with a value its type shows (for example a `Cancelled` case); no join may be left waiting.
> - **Errors.** When a child fails, the scope cancels the other children. When all have ended, the scope gives the error of the failed task with the lowest path (D10) among the scope's body and its children; the body comes first. Later errors are dropped. Error types stay unions; there is no aggregate error type, so inference does not change. Every error given to a scope made by `scope` is portable (P3). For `scope((s) { s.go(() { s.go(() { fail.raise(A) })  fail.raise(C) }) })`, if both fail, the error is `C` (path `[1]` comes before `[1, 1]`), whichever failed first. Which children fail before the cancel reaches them depends on the order of D1: for `s.go(() { task.yield()  fail.raise(A) })` and `s.go(() { fail.raise(B) })`, one FIFO worker gives `B`, and N workers may give `A`. The `i/task` docs say so.
> - **Results.** `both`, `race` and `map_concurrently` are ordinary i code built on `scope`, so they are portable paths: each result and error of their children is checked when the child ends (P3), and a result containing a user host reference stops with `panic: task: value is not portable`. Standard File/Listener/Stream/Child values are portable nominal Int wrappers (native N9); returning them through these combinators succeeds. R may add local forms built on `local_scope` (`local_both`, `local_race`, `local_map_concurrently`), which do not check and whose children stay home (P4); without them, the program writes the local scope itself. `both` and `map_concurrently` give results in argument order. `race` gives the first result that commits; timing chooses, as with a CML `choose`. With one worker the FIFO scheduler makes it fixed, but that is not a promise for N workers.
> - **What a cancel of a scope reaches.** When a scope `s` is cancelled (a child failed, or a task whose chain holds `s` is itself cancelled), the cancel reaches every child of `s` **and the body of `s`**. The body is unwound back to the `scope` call: `scope` runs its body under its own boundary in the body's task, and the cancel suspends to that boundary and cancels the part up to it, so the body's final functions run and the body gives no value; `scope` then waits for its children and gives the error of D10. A cancelled task cancels every scope it is running a body of, so the cancel goes down the whole tree inside `s`. A cancelled body is not a failure; it does not take part in the choice of the error. So `scope((s) { s.go(() { fail.raise(A) })  t = s.go(() { 1 })  t.join() })` gives error `A` on the one-worker FIFO scheduler: child `[1]` fails, `t` is dropped, the body waiting in `t.join()` is unwound, and nothing waits forever.
> - **Cancellation** is cooperative and goes to the task's home worker. An unstarted task is dropped; it has no final functions. A started task is cancelled when the cancel reaches it: when it is paused (waiting or runnable) at that time, the loop does not give it the answer it waits for and cancels it as the next point says; when it is running, at its next suspend point. When the cancel reaches a task is a step in the order of D1: on one FIFO worker it reaches every paused task at once, and on N workers it reaches a task on another worker at that worker's next poll point. A task that is being unwound is not cancelled again, and its final functions run to their end, with the scopes they open, which that cancel does not reach.
> - **How a started task is cancelled.** The loop never calls `control.cancel(k)` on the task's continuation `k`: under §9.4 rule 4 the task's boundary `b` would stay not active while the final functions run (`$control_cancel` and `$cancel_run`, `emit/control.rs:270-300`, never set field 8 of the boundary), and a final function that uses `i/task` (a send, a receive, a join, a sleep, an async host call through the scheduler) would suspend to `b` and stop with `control: boundary is not active`. Instead the loop **resumes** `k` with a cancel answer. `b` is then active again. The `Scheduler` call inside the task that sees the answer suspends to the task's inner boundary (made by `task.unwind_inside`, part 4) or, for a scope body, to that scope's boundary, and its handler calls `control.cancel` on that inner part only. So the final functions run with `b` active, and they may suspend to `b`; `$cancel_run` already catches a capture through it and gives it the record `$cancel_reenter`, so the rest of the unwind resumes later. They get the environment of §9.4 rule 4 as part 13 amends it: the services kept by the task's kept function and the environment statements inside the task, not the loop's services. This is all ordinary i code on `i/control`.
> - **Stops.** A stop on any worker closes the run's start/claim/delivery gate and records one message under the lock. Stopping invokes no i finals. Every user import checks the stopped-context gate, so spinning native workers cannot start new host effects while reaping delays process exit. Native's independent controller tears down host work/resources before process exit; it does not join a spinning Wasm worker. A continuing host uses stop-only epoch traps in that run's interruption domain, wakes idle reactors and joins workers, then returns the first stop (native N12). Epochs do not interrupt synchronous host code; multicore hosts must supply stop-aware synchronous imports or choose one worker/process isolation. Child cleanup follows runtime §8.1, including reaping before native exit.
> - **Deadlock.** When no task can run and no host request is outstanding on any worker, nothing can ever go on. The run then stops with `panic: task: deadlock`, on one worker as on N. On one worker the scheduler sees this directly: its queue is empty and its request table is empty. It must not return status 1 then, which host-concurrency.md H4 makes a fault of the module. On N workers it needs quiescence detection: a shared count of runnable and running tasks and of outstanding requests; the worker that sees it reach zero with shared and local queues empty records the stop under the same lock. Queue claims, local running counts, registrations and completion publication must participate in that snapshot; idle workers alone do not prove deadlock. In a macro, which has no host services (`meta.md` §8), every wait with nothing to run is a deadlock.
> - **Idle.** A worker with nothing to run, while others still work or requests are outstanding, waits. By host-concurrency H5, a host answers a request only through `ilang_resume:…` after the instance has returned status 1 or 2, never while it is inside Wasm. So a worker that has **outstanding requests of its own** (its pinned tasks wait on them, P2) must never park inside Wasm: it would never see its answers, and a task that waits on a 10 ms sleep would hang until something else woke the worker. The rule:
>   - A worker with outstanding requests of its own idles only by returning status 1, on every host. Its host then waits for either an answer to one of its requests or a **wake** from another worker, whichever comes first. The wake is a request-mode ordinary host service added in the multicore wave (native N8): it resolves when another worker writes the worker's wake word (`Atomics.waitAsync` in JS; in wasmtime, a wait on that word in the host thread, raced with the reactor). The worker keeps one wake request outstanding while it idles, so status 1 is never given with no outstanding request (H4).
>   - A native worker with no requests may park in the host queue wait; a browser worker may use memory.atomic.wait32, except browser worker 0, which never blocks: it idles like a worker with requests, with only the wake request outstanding.
>   - A one-worker run never uses the wake request: on one worker, idle means status 1 with the run's own requests (C-D's "wait for any"), and with none it is a deadlock.

New sentence(s)/block:

> - A scope returns only after all its children have ended, including children that are being unwound.
> - **Chains.** The **chain** of a scope `r` is `r` followed by the chain of the task that opened `r`, as it was when `r` was opened. The **chain** of a task, at a moment, is the scopes it has opened whose bodies it is still running (innermost first), followed by the chain of the scope it was started in (empty for a run's root task). A task's chain changes only when that task itself enters or leaves a scope body, so it is fixed by the task's own steps, not by time or placement. Every scope in a task's chain has not ended: the scopes it has opened are still running their bodies, the scope it was started in waits for it, and each scope in that scope's chain was in the opener's chain when it was opened, so by the same argument it cannot end before the opener leaves it. The scheduler keeps each task's chain as a list of scope ids, so the check is a list search.
> - **Who may `go`.** `s.go` may be called only by a task whose chain holds `s`. Otherwise it stops with `panic: task: go outside its scope`, whether or not `s` has ended. So a `go` never meets an ended scope. Examples that stop: `h = scope((s) { s })  h.go(() { 1 })` (after the body returns, `s` is no longer in the root task's chain); `scope((u) { scope((s) { u.go(() { task.yield()  s.go(() { 1 }) }) }) })` (the `u` child was started in `u`, whose chain does not hold `s`); `scope((outer) { c = channel.new()  outer.go(() { h = c.receive()  h.go(() { 1 }) })  scope((s1) { c.send(s1) }) })` (the same). Each stops on any number of workers. A scope handle is portable and may be sent anywhere, but only tasks inside the scope may use it to start tasks. A `go` on a scope that is cancelling makes an unstarted task that is dropped at once; whether a `go` comes before or after the cancel is part of the order of D1.
> - **Who may join.** A join of the Task handle of a child of `s` (and a `sync` of a join event made from it) may be done only by a task whose chain holds `s`. Otherwise it stops with `panic: task: join outside its scope`. A joiner still cancellable by `s` is unwound before observing a dropped/cancelled outcome; a shielded joiner, including protected final work, stops with `panic: task: join of a cancelled task` immediately when drop/cancellation is accepted, without waiting for target cleanup (runtime R4a). If R later adds a way to cancel one task while its scope goes on, a join of that task must complete with a value its type shows (for example a `Cancelled` case); no join may be left waiting.
> - **Errors.** When a child fails, the scope cancels the other children. When all have ended, the scope gives the error of the failed task with the lowest path (D10) among the scope's body and its children; the body comes first. Later errors are dropped. Error types stay unions; there is no aggregate error type, so inference does not change. Every error given to a scope made by `scope` is portable (P3). For `scope((s) { s.go(() { s.go(() { fail.raise(A) })  fail.raise(C) }) })`, if both fail, the error is `C` (path `[1]` comes before `[1, 1]`), whichever failed first. Which children fail before the cancel reaches them depends on the order of D1: for `s.go(() { task.yield()  fail.raise(A) })` and `s.go(() { fail.raise(B) })`, one FIFO worker gives `B`, and N workers may give `A`. The `i/task` docs say so.
> - **Results.** `both`, `race` and `map_concurrently` are ordinary i code built on `scope`, so they are portable paths: each result and error of their children is checked when the child ends (P3), and a result containing a user host reference stops with `panic: task: value is not portable`. Standard File/Listener/Stream/Child values are portable nominal Int wrappers (native N9); returning them through these combinators succeeds. R may add local forms built on `local_scope` (`local_both`, `local_race`, `local_map_concurrently`), which do not check and whose children stay home (P4); without them, the program writes the local scope itself. `both` and `map_concurrently` give results in argument order. `race` gives the first result that commits; timing chooses, as with a CML `choose`. With one worker the FIFO scheduler makes it fixed, but that is not a promise for N workers.
> - **What a cancel of a scope reaches.** When a scope `s` is cancelled (a child failed, or a task whose chain holds `s` is itself cancelled), the cancel reaches every child of `s` **and the body of `s`**. The body is unwound back to the `scope` call: `scope` runs its body under its own boundary in the body's task, and the cancel suspends to that boundary and cancels the part up to it, so the body's final functions run and the body gives no value; `scope` then waits for its children and gives the error of D10. A cancelled task cancels every scope it is running a body of, so the cancel goes down the whole tree inside `s`. A cancelled body is not a failure; it does not take part in the choice of the error. So `scope((s) { s.go(() { fail.raise(A) })  t = s.go(() { 1 })  t.join() })` gives error `A` on the one-worker FIFO scheduler: child `[1]` fails, `t` is dropped, the body waiting in `t.join()` is unwound, and nothing waits forever.
> - **Cancellation** is cooperative and goes to the task's home worker. An unstarted task is dropped; it has no final functions. A started task is cancelled when the cancel reaches it: when it is paused (waiting or runnable) at that time, the loop does not give it the answer it waits for and cancels it as the next point says; when it is running, at its next suspend point. When the cancel reaches a task is a step in the order of D1: on one FIFO worker it reaches every paused task at once, and on N workers it reaches a task on another worker at that worker's next poll point. A task that is being unwound is not cancelled again, and its final functions run to their end, with the scopes they open, which that cancel does not reach.
> - **How a started task is cancelled.** The loop never calls `control.cancel(k)` on the task's continuation `k`: under §9.4 rule 4 the task's boundary `b` would stay not active while the final functions run (`$control_cancel` and `$cancel_run`, `emit/control.rs:270-300`, never set field 8 of the boundary), and a final function that uses `i/task` (a send, a receive, a join, a sleep, an async host call through the scheduler) would suspend to `b` and stop with `control: boundary is not active`. Instead the loop **resumes** `k` with a cancel answer. `b` is then active again. The `Scheduler` call inside the task that sees the answer suspends to the task's inner boundary (made by `task.unwind_inside`, part 4) or, for a scope body, to that scope's boundary, and its handler calls `control.cancel` on that inner part only. So the final functions run with `b` active, and they may suspend to `b`; `$cancel_run` already catches a capture through it and gives it the record `$cancel_reenter`, so the rest of the unwind resumes later. They get the environment of §9.4 rule 4 as part 13 amends it: the services kept by the task's kept function and the environment statements inside the task, not the loop's services. This is all ordinary i code on `i/control`.
> - **Stops.** A stop on any worker closes the run's start/claim/delivery gate and records one message under the lock. Stopping invokes no i finals. Every user import checks the stopped-context gate, so spinning native workers cannot start new host effects while reaping delays process exit. Native's independent controller tears down host work/resources before process exit; it does not join a spinning Wasm worker. A continuing host uses stop-only epoch traps in that run's interruption domain, wakes idle reactors and joins workers, then returns the first stop (native N12). Epochs do not interrupt synchronous host code; multicore hosts must supply stop-aware synchronous imports or choose one worker/process isolation. Child cleanup follows runtime §8.1, including reaping before native exit.
> - **Deadlock.** Use the program waitable set in part 4/runtime §6.4: all outstanding work owned by the run's program Root plus explicitly awaited borrowed Waiting tickets, deduplicated by identity, expanding group-observer tickets to their underlying members (runtime §5.1). Driver wake requests use a separate ordinary control Root/Owner; they are closed with the driver and never bound into user tasks or exposed as borrowed waits. General Root/Owner accounting distinguishes them by the roots selected by the driver, never by module/import name. With no runnable, claimed-but-unstarted or running task, no pending program completion/group-notification processing, and an empty program waitable set, record `panic: task: deadlock` under the shared lock. Start/claim/cancel/answer transitions and local work counts participate in that snapshot; an answer moves from the request count to pending delivery work atomically. Control requests and wake-only deliveries are excluded from that progress predicate. All actual outstanding requests still count for ABI status 1, so wake requests preserve H4 without masking deadlock. On one worker no wake is needed and the same predicate applies. In a macro without host services, nothing runnable and an empty waitable set is a deadlock.
> - **Review wake trace.** Worker 0 is idle with one control wake request. The last runnable task on A executes sync(never()); program request count is zero. Once that registration finishes, runnable/running/claimed/pending-program-delivery counts are all zero, so the locked snapshot records deadlock even though the control count is one (or two if A also installed a wake). Stop closes the control roots and wakes the drivers; it does not wait for those wakes to justify progress. If an owned or explicitly borrowed program request is pending instead, its count prevents the stop until it settles.
> - **Idle.** A worker with nothing to run, while others still work or requests are outstanding, waits. By host-concurrency H5, a host answers a request only through `ilang_resume:…` after the instance has returned status 1 or 2, never while it is inside Wasm. So a worker that has **outstanding requests of its own** (its pinned tasks wait on them, P2) must never park inside Wasm: it would never see its answers, and a task that waits on a 10 ms sleep would hang until something else woke the worker. The rule:
>   - A worker with outstanding requests of its own idles only by returning status 1, on every host. Its host then waits for either an answer to one of its requests or a **wake** from another worker, whichever comes first. The wake is a request-mode ordinary host service added in the multicore wave (native N8): it resolves when another worker writes the worker's wake word (`Atomics.waitAsync` in JS; in wasmtime, a wait on that word in the host thread, raced with the reactor). The worker keeps one wake request outstanding while it idles, so status 1 is never given with no outstanding request (H4).
>   - A native worker with no requests may park in the host queue wait; a browser worker may use memory.atomic.wait32, except browser worker 0, which never blocks: it idles like a worker with requests, with only the wake request outstanding.
>   - A one-worker run never uses the wake request: on one worker, idle waits on the owned-plus-borrowed program set from part 4, and an empty set with no pending program delivery means deadlock.

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization. N-R2: distinguish owned non-creating operations from borrowed waits and ensure cancellation releases their leases; retain creator guards. N-R3: count program producers and pending program delivery for deadlock, separate control wakes through ordinary Root/Owner accounting, and preserve ABI H4. N-R4: preserve runtime’s pure fresh-key events, validate stateful leaves, and derive portability from payloads/captures. N-R5: retain the owned-plus-explicitly-borrowed waitable set, without counting unrelated roots. N-R6: use the shared go/nested-run counter for paths and random derivation. Record the review source, exact regression expectations, phase/spec handoff and verification limits; preserve revision-3 history.

### R4-028 — docs/design/multicore.md, 60f37f2 line 601

Old sentence(s)/block:

> (none)

New sentence(s)/block:

> The N-R1–N-R6 fixtures in native §11.1 are required at 1/2/4 workers: remote
> Root count without Ticket transfer; owned read cancellation; wake-only deadlock;
> pure and captured events; borrowed nested requests; combined go/run paths.
> MC-D must demonstrate atomic group subscription, cancellation actions before
> group-cancel return, home-cell terminal reconciliation without foreign Tickets, and the
> program/control quiescence snapshot through the one Scheduler boundary before
> MC-I. These are fixed semantics, not choices left to that prototype.

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization. N-R3: count program producers and pending program delivery for deadlock, separate control wakes through ordinary Root/Owner accounting, and preserve ABI H4. N-R4: preserve runtime’s pure fresh-key events, validate stateful leaves, and derive portability from payloads/captures. N-R5: retain the owned-plus-explicitly-borrowed waitable set, without counting unrelated roots. N-R6: use the shared go/nested-run counter for paths and random derivation. Record the review source, exact regression expectations, phase/spec handoff and verification limits; preserve revision-3 history.

### R4-029 — docs/design/multicore.md, 60f37f2 line 603

Old sentence(s)/block:

> 1. Values pass between tasks only through `go` captures and kept services, channel and event sends, task results and errors (read by the scope or by a join of the Task handle), and `scope.share`. There is no other shared state.
> 2. Every portable path calls `core.portable` and stops with `panic("task: value is not portable")` when it is False: a `send` on a `channel.new` channel (checked at the call), a send event on such a channel (checked when the event is made, never at `sync` or at commit), results and errors given to a scope made by `scope`, also inside `both`, `race` and `map_concurrently`, which are built on `scope`, and `share`. This happens always, also on one worker. Local forms of the combinators, if R adds them, are built on `local_scope` and do not check.
> 3. `go` never rejects a task for non-portability. On N workers it silently classifies and encodes the complete record at go (D8/P1); failure pins locally. A shared entry has one claim/drop owner, coordinated with scope cancellation. One worker keeps its FIFO originals without encoding.
> 4. As runtime §6.3 requires, `go` also detaches/rebinds Runner and Await while preserving Root/Owner. `go` binds `Scheduler` to the portable `scheduler.detached` and then keeps the parent's services with `control.keep_env`, and takes the real `Scheduler` as the parameter of the kept function. So the parent's scheduler is never kept or captured. The kept function binds `! Scheduler = own` and then calls the body **not** in tail position (`r = …  r`), so that the statement stays on a frame record after a resume (`lazy-reentry.md` D16, section 7a); R checks at O2 that it stays a frame. `task.keep(f)` is built the same way for callbacks that other tasks call; a function that user code keeps with `control.keep_env` inside a task is tied to that task (D4), and the docs say so.
> 5. Every task runs directly from its run's loop, never on another task's stack, under a new boundary that the loop makes for it at its start (D5); `control.resume` makes it active again. The loop has no long-lived boundary of its own. Nested scopes, `both`, `race` and `map_concurrently` never run a loop on the caller's stack. Only the entry's `task.run` may use more than one worker; any other `task.run` is a nested run: one worker, FIFO, on the caller's stack, on every host. `task.run` needs the run service (part 4), which gives it its path prefix, root seed and wait. A nested run inside a task has paths that start with the calling task's path and its run index, waits by suspending only its calling task to the outer run (never by status 1 from the caller's stack), and stops with `panic("task: deadlock")` when it has nothing to run and no request of its own. Every handle holds the key of its run; using it in another run stops with `panic("task: handle of another run")`.
> 6. Local channels and local scopes hold the boundary of the task that made them, only to be non-portable; no suspend ever goes to it. The Task handle of a child of a local scope, and every join event made from it, hold the local scope handle, so they are not portable either. Every task record holds the handle of its scope, so a child of a local scope is never remotely claimed. Local paths do not check. There is no flag and no special case.
> 7. A started task never moves. A task that waits on an async host call resumes on the worker that started the call. A host request value (R2) is not portable, so only tasks on its home worker wait on it, poll it or cancel it.
> 8. A scope returns only after all its children have ended. On a failure it cancels the rest **and its own body**, which is unwound back to the `scope` call, and then gives the error of the failed task with the lowest path in path order (D10; its own body first, `[1] < [1, 1] < [2]`); a cancelled body is not a failure. Every task has a chain of scopes (part 6), changed only by its own scope entries and exits. `s.go`, and a join of a child of `s`, may be done only by a task whose chain holds `s`; otherwise they stop with `panic("task: go outside its scope")` and `panic("task: join outside its scope")`, whether or not `s` has ended. A `go` on a cancelling scope makes an unstarted task dropped at once; a shielded caller receives its Task handle, while a cancellable caller unwinds before observing it (runtime R4a). At a valid join/selected join event, applicable cancellation first unwinds a cancellable joiner; a shielded joiner stops with `panic("task: join of a cancelled task")` as soon as target drop/cancellation is accepted, without waiting for cleanup (runtime R4a). Error types stay unions.
> 9. `both` and `map_concurrently` give results in argument order. The docs state the rule of D1: every run gives the result of some order of steps, so a plain receive with several senders, `race`, `choose`, and anything else that depends on the order may differ between runs and worker counts.
> 10. Cancellation is cooperative: an unstarted task is dropped; a started task is cancelled on its home worker when the cancel reaches it (at once when it is paused, at its next suspend point when it runs; when that is, is a step of D1). The loop never calls `control.cancel` on a task's continuation: it resumes the task with a cancel answer, and the task cancels its own inner part, so its boundary stays active and final functions may use `i/task`. A task being unwound is not cancelled again.
> 11. Every task has a path (the path of the task that called `go` plus the index of that `go` among its `go` calls and nested runs), given by `task.path()` with no suspend. Random is a value: `task.random()` is a pure function of the root seed and the path, and `i/random` does not need `i/task` or `Scheduler`. The root seed comes from the run service; the entry default is random.from_seed(0), and task.top(seed) changes only the current Runner's seed/route-preserving value, with no entropy draw (runtime D5). Path steps and split steps are different kinds of stream step, so `random.split` never gives the generator of a task. Task/scope/channel/event/share ids and per-task names come from the path, never from time, a global counter, or the worker.
> 12. Task, scope, channel, event and `Shared` handles are made of path-based ids and keys (D13), and hold no boundary, closure over scheduler state, or other non-portable value (except local handles and the handles of children of local scopes, rule 6). `==` on them means "the same handle". No API needs an order or a hash on functions, keys, boundaries or continuations; scheduler bookkeeping uses Int ids.
> 13. Events are values. `choose` and `map` are pure, `sync` commits exactly one event, and `map` functions run in the syncing task after the commit. Nack and guard are left for later.
> 14. `scope.share(value)` needs a portable value. A `Shared` belongs to its scope, and a `get` after the scope ends stops with `panic("task: shared value is gone")` in every build. A channel belongs to the first scope in its maker's chain (or to the run); its state is freed when that scope ends, and a later send, receive or `sync` on it, or a wait on it at that end, stops with `panic("task: channel is gone")` in every build.
> 15. All shared mutable scheduler state is behind the one `Scheduler` service, kept as `Keyed` values where it has many types, so the multicore wave can replace its implementation without changing the API. No API needs an unsafe cast.
> 16. The scheduler has poll points (`go`, every `sync`, joins and scope ends, `yield`, async host calls, every switch of the loop) written as one call that does nothing on one worker.
> 17. On one worker the scheduler is FIFO and fixed, with no time unless a `Clock` service is given. It idles only by waiting for an outstanding host request (C-D's "wait for any"), never by a busy loop or a blocking import, and never returns status 1 with no request outstanding. When no task can run and no request is outstanding, it stops with `panic("task: deadlock")`. On N workers (MC), a worker with outstanding requests of its own never parks inside Wasm; it returns status 1 with a wake request outstanding (part 6, Idle).
> 18. Many host calls may wait at once, through the request table. No code relies on one pending import per instance.
> 19. A stop in any task stops the program. It is never caught or counted as a task error.
> 20. No API takes a label-generic callback (records L5), and no API needs a static `Send`-like type.
> 21. No API shows worker indices, the worker count, or queue claims.
> 22. Tests of `i/task` and of programs that use it must not depend on the order of steps (D1: which sender a receive gets, which `race` branch, which children fail before a cancel, whether a step comes before a scope ends, which of two stops, the order of output), or must be marked with the outcomes they allow (T13).
> 23. `Key(A)` is invariant in `A`, and keys are only used through `i/key`; no API casts.
> 24. `s.go` and joins check the chain, channels and `Shared` values are freed with their scope, and handles check their run, all in M:1 already, so that the multicore wave changes no program.
> 25. Only `==` reads function and key ids. The text form of a function, a key, a `Keyed` value or any task/scope/channel/event/share handle never shows an id, a worker index or anything else that depends on placement, and no `i/task` API gives one out.

New sentence(s)/block:

> 1. Values pass between tasks only through `go` captures and kept services, channel and event sends, task results and errors (read by the scope or by a join of the Task handle), and `scope.share`. There is no other shared state.
> 2. Every portable path calls `core.portable` and stops with `panic("task: value is not portable")` when it is False: a `send` on a `channel.new` channel (checked at the call), a send event on such a channel (checked when the event is made, never at `sync` or at commit), results and errors given to a scope made by `scope`, also inside `both`, `race` and `map_concurrently`, which are built on `scope`, and `share`. This happens always, also on one worker. Local forms of the combinators, if R adds them, are built on `local_scope` and do not check.
> 3. `go` never rejects a task for non-portability. On N workers it silently classifies and encodes the complete record at go (D8/P1); failure pins locally. A shared entry has one claim/drop owner, coordinated with scope cancellation. One worker keeps its FIFO originals without encoding.
> 4. As runtime §6.3 requires, `go` also detaches/rebinds Runner and Await while preserving Root/Owner. `go` binds `Scheduler` to the portable `scheduler.detached` and then keeps the parent's services with `control.keep_env`, and takes the real `Scheduler` as the parameter of the kept function. So the parent's scheduler is never kept or captured. The kept function binds `! Scheduler = own` and then calls the body **not** in tail position (`r = …  r`), so that the statement stays on a frame record after a resume (`lazy-reentry.md` D16, section 7a); R checks at O2 that it stays a frame. `task.keep(f)` is built the same way for callbacks that other tasks call; a function that user code keeps with `control.keep_env` inside a task is tied to that task (D4), and the docs say so.
> 5. Every task runs directly from its run's loop, never on another task's stack, under a new boundary that the loop makes for it at its start (D5); `control.resume` makes it active again. The loop has no long-lived boundary of its own. Nested scopes, `both`, `race` and `map_concurrently` never run a loop on the caller's stack. Only the entry's `task.run` may use more than one worker; any other `task.run` is a nested run: one worker, FIFO, on the caller's stack, on every host. `task.run` needs the run service (part 4), which gives it its path prefix, root seed and wait. A nested run inside a task has paths that start with the calling task's path and its run index, waits by suspending only its calling task to the outer run (never by status 1 from the caller's stack), and uses the owned-plus-explicitly-borrowed waitable set of part 4/runtime §6.4; no runnable or pending program delivery and an empty set means `panic("task: deadlock")`. Channel/Task/scope/Shared handles check their run; Event descriptions have fresh keys without a required creating run, and sync checks every stateful leaf, including losing alternatives.
> 6. Local channels and local scopes hold the boundary of the task that made them, only to be non-portable; no suspend ever goes to it. The Task handle of a child of a local scope, and every join event made from it, hold the local scope handle, so they are not portable either. Every task record holds the handle of its scope, so a child of a local scope is never remotely claimed. Local paths do not check. There is no flag and no special case.
> 7. A started task never moves. A task that waits on an async host call resumes on the worker that started the call. Request/Ticket values (R2) are not portable: only their home worker operates on their cells. Portable Root/Owner operations observe or cancel whole groups through shared metadata and home-worker notifications; they never return foreign Tickets (runtime §5.1).
> 8. A scope returns only after all its children have ended. On a failure it cancels the rest **and its own body**, which is unwound back to the `scope` call, and then gives the error of the failed task with the lowest path in path order (D10; its own body first, `[1] < [1, 1] < [2]`); a cancelled body is not a failure. Every task has a chain of scopes (part 6), changed only by its own scope entries and exits. `s.go`, and a join of a child of `s`, may be done only by a task whose chain holds `s`; otherwise they stop with `panic("task: go outside its scope")` and `panic("task: join outside its scope")`, whether or not `s` has ended. A `go` on a cancelling scope makes an unstarted task dropped at once; a shielded caller receives its Task handle, while a cancellable caller unwinds before observing it (runtime R4a). At a valid join/selected join event, applicable cancellation first unwinds a cancellable joiner; a shielded joiner stops with `panic("task: join of a cancelled task")` as soon as target drop/cancellation is accepted, without waiting for cleanup (runtime R4a). Error types stay unions.
> 9. `both` and `map_concurrently` give results in argument order. The docs state the rule of D1: every run gives the result of some order of steps, so a plain receive with several senders, `race`, `choose`, and anything else that depends on the order may differ between runs and worker counts.
> 10. Cancellation is cooperative: an unstarted task is dropped; a started task is cancelled on its home worker when the cancel reaches it (at once when it is paused, at its next suspend point when it runs; when that is, is a step of D1). The loop never calls `control.cancel` on a task's continuation: it resumes the task with a cancel answer, and the task cancels its own inner part, so its boundary stays active and final functions may use `i/task`. A task being unwound is not cancelled again.
> 11. Every task has a path (the path of the task that called `go` plus the index of that `go` among its `go` calls and nested runs), given by `task.path()` with no suspend. Random is a value: `task.random()` is a pure function of the root seed and the path, and `i/random` does not need `i/task` or `Scheduler`. The root seed comes from the run service; the entry default is random.from_seed(0), and task.top(seed) changes only the current Runner's seed/route-preserving value, with no entropy draw (runtime D5). Path steps and split steps are different kinds of stream step, so `random.split` never gives the generator of a task. Task/scope/channel/share ids and per-task names come from the path, never from time, a global counter, or the worker. Event identity uses a fresh key independent of any task path or run; only equality observes it, and copies retain it (runtime §6.4).
> 12. Task, scope, channel and `Shared` handles use path-based ids and keys (D13), with local witnesses for local handles and children of local scopes (rule 6). Events instead contain a fresh identity key and ordinary descriptions, payloads and callbacks; their captures may contain boundaries or other non-portable values, which pin a capturing task under the normal complete-record test. Stateful handle implementations add no hidden closure over a live scheduler; user event callbacks may capture one and are then non-portable. `==` on these values means "the same handle", and event copies preserve their key. No API needs an order or a hash on functions, keys, boundaries or continuations; scheduler bookkeeping uses Int ids.
> 13. Events are values. `choose` and `map` are pure, `sync` commits exactly one event, and `map` functions run in the syncing task after the commit. Nack and guard are left for later.
> 14. `scope.share(value)` needs a portable value. A `Shared` belongs to its scope, and a `get` after the scope ends stops with `panic("task: shared value is gone")` in every build. A channel belongs to the first scope in its maker's chain (or to the run); its state is freed when that scope ends, and a later send, receive or `sync` on it, or a wait on it at that end, stops with `panic("task: channel is gone")` in every build.
> 15. All shared mutable scheduler state is behind the one `Scheduler` service, kept as `Keyed` values where it has many types, so the multicore wave can replace its implementation without changing the API. No API needs an unsafe cast.
> 16. The scheduler has poll points (`go`, every `sync`, joins and scope ends, `yield`, async host calls, every switch of the loop) written as one call that does nothing on one worker.
> 17. On one worker the scheduler is FIFO and fixed, with no time unless a `Clock` service is given. It idles only by waiting for an outstanding host request (C-D's "wait for any"), never by a busy loop or a blocking import, and never returns status 1 with no request outstanding. When no task or pending program delivery can run and the owned-plus-borrowed program waitable set is empty, it stops with `panic("task: deadlock")`; control-root wakes do not prevent this (part 6). On N workers (MC), a worker with outstanding requests of its own never parks inside Wasm; it returns status 1 with a wake request outstanding (part 6, Idle).
> 18. Many host calls may wait at once, through the request table. No code relies on one pending import per instance.
> 19. A stop in any task stops the program. It is never caught or counted as a task error.
> 20. No API takes a label-generic callback (records L5), and no API needs a static `Send`-like type.
> 21. No API shows worker indices, the worker count, or queue claims.
> 22. Tests of `i/task` and of programs that use it must not depend on the order of steps (D1: which sender a receive gets, which `race` branch, which children fail before a cancel, whether a step comes before a scope ends, which of two stops, the order of output), or must be marked with the outcomes they allow (T13).
> 23. `Key(A)` is invariant in `A`, and keys are only used through `i/key`; no API casts.
> 24. `s.go` and joins check the chain, channels and `Shared` values are freed with their scope, and handles check their run, all in M:1 already, so that the multicore wave changes no program.
> 25. Only `==` reads function and key ids. The text form of a function, a key, a `Keyed` value or any task/scope/channel/event/share handle never shows an id, a worker index or anything else that depends on placement, and no `i/task` API gives one out.

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization. N-R3: count program producers and pending program delivery for deadlock, separate control wakes through ordinary Root/Owner accounting, and preserve ABI H4. N-R4: preserve runtime’s pure fresh-key events, validate stateful leaves, and derive portability from payloads/captures. N-R5: retain the owned-plus-explicitly-borrowed waitable set, without counting unrelated roots. N-R6: use the shared go/nested-run counter for paths and random derivation. Record the review source, exact regression expectations, phase/spec handoff and verification limits; preserve revision-3 history.

### R4-030 — docs/design/native.md, 60f37f2 line 3

Old sentence(s)/block:

> Status: adopted, revision 3 (2026-11-10). The ledger's “Native proposal ADOPTED” records
> the user's adoption of revision 2, including the four replacements in §8. This revision
> reconciles runtime.md and multicore.md and closes the lifetime, stop and forged-number
> gaps. It changes no implementation. Performance targets are acceptance goals, not
> measurements.

New sentence(s)/block:

> Status: adopted, revision 4 (2026-11-10). The ledger's “Native proposal ADOPTED” records
> the user's adoption of revision 2, including the four replacements in §8. This revision
> reconciles runtime.md and multicore.md and closes the lifetime, stop and forged-number
> gaps. Revision 4 applies coordinator rulings N-R1–N-R6 after the independent
> review of 60f37f2; §11.1 records the repairs and regression cases. It changes no implementation. Performance targets are acceptance goals, not
> measurements.

Reason: Record the review source, exact regression expectations, phase/spec handoff and verification limits; preserve revision-3 history.

### R4-031 — docs/design/native.md, 60f37f2 line 128

Old sentence(s)/block:

> (none)

New sentence(s)/block:

> N-R1/N-R3 refine that boundary: Root/Owner are portable group capabilities,
> with shared count/readiness/cancel-all/wait-none semantics and no Ticket
> enumeration (runtime §5.1). Request/Ticket cells and observer cells stay local;
> metadata notifications route changes to their home workers. Group waits use
> Await and count only their underlying producers, never their own observation
> cells. The driver owns separate ordinary program and control Roots, selected
> by its accounting policy, not by any module/import rule. Only the program Root
> and explicit borrowed producer interests enter quiescence; wake requests in the
> control Root still count for ABI status 1. Snapshot runnable/running/claimed work,
> pending program deliveries and that program waitable set under the same lock
> (runtime §6.4; multicore §6). Atomic delivery publication prevents a false zero.

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization. N-R3: count program producers and pending program delivery for deadlock, separate control wakes through ordinary Root/Owner accounting, and preserve ABI H4. N-R4: preserve runtime’s pure fresh-key events, validate stateful leaves, and derive portability from payloads/captures. N-R5: retain the owned-plus-explicitly-borrowed waitable set, without counting unrelated roots.

### R4-032 — docs/design/native.md, 60f37f2 line 165

Old sentence(s)/block:

> **N9.3. Lookup, leases and close have one order.** Under a short host lock, test
> context/kind/liveness and acquire the operation's lease. Socket directions each have one
> lease; read+write may overlap, two reads/writes/accepts give busy. Files have one
> read/write/flush cursor lease, children one wait lease. Removal on close revokes leases
> and wakes pending operations with closed. A completion committed first retains its
> answer; close committed first prevents success publication. In-flight code holds a safe
> reference/private OS duplicate, never a pointer to a removed table entry. Release
> happens once when references are safe. Close is idempotent/error-free; cancelling its
> request cannot undo host cleanup. Child close awaits reaping. A context/kind-checked
> cleanup receipt survives removal; concurrent closes join that receipt, including
> reaping, and teardown includes already-closing jobs. Free receipts on completion without
> permanent tombstones. Preserve the blocking-file cancellation, stdin broker,
> partial-write and supervisor rules of runtime §8.1.

New sentence(s)/block:

> **N9.3. Lookup, leases and close have one order.** Under a short host lock, test
> context/kind/liveness and acquire the operation's lease. Socket directions each have one
> lease; read+write may overlap, two reads/writes/accepts give busy. Files have one
> read/write/flush cursor lease, children one wait lease. Removal on close revokes leases
> and wakes pending operations with closed. A completion committed first retains its
> answer; close committed first prevents success publication. In-flight code holds a safe
> reference/private OS duplicate, never a pointer to a removed table entry. Release
> happens once when references are safe. Close is idempotent/error-free; cancelling its
> request cannot undo host cleanup. Child close awaits reaping. A context/kind-checked
> cleanup receipt survives removal; concurrent closes join that receipt, including
> reaping, and teardown includes already-closing jobs. Free receipts on completion without
> permanent tombstones. Preserve the blocking-file cancellation, stdin broker,
> partial-write and supervisor rules of runtime §8.1. Non-creating raw operations
> use the ordinary request.await_owned helper with an ensure/cancel final
> (runtime §5.4); a borrowed request.await does not cancel its producer. Creators
> retain the separate await_resource guard.

Reason: N-R2: distinguish owned non-creating operations from borrowed waits and ensure cancellation releases their leases; retain creator guards. N-R3: count program producers and pending program delivery for deadlock, separate control wakes through ordinary Root/Owner accounting, and preserve ABI H4. N-R4: preserve runtime’s pure fresh-key events, validate stateful leaves, and derive portability from payloads/captures. N-R5: retain the owned-plus-explicitly-borrowed waitable set, without counting unrelated roots. Record the review source, exact regression expectations, phase/spec handoff and verification limits; preserve revision-3 history.

### R4-033 — docs/design/native.md, 60f37f2 line 351

Old sentence(s)/block:

> | Stage | Build now so it is not redone | What waits |
> |---|---|---|
> | R1 | Immutable published SplitMix and tagged paths exactly as runtime §4; no task/IO dependency | Nothing native-specific blocks R1 |
> | R2 request layer | Local Request/Ticket cells, rooted Owner accounting, one root hand-off, consuming take and cancellation seam; no resource recognition for Int | Native packaging, worker transport |
> | R2 scheduler | Complete task records, detached routing services, retained environment frames, silent pinning, typed keys and single Scheduler boundary; FIFO/CML semantics and R4a joins | Queue encoding/claim ownership, locked transport implementation and worker entry |
> | IO clock, before Fs | General §2.4 nominal field-privacy fix; host ResourceContext owner/controller, non-reusing Int allocator and trusted binding-plan/grant plumbing on Rust and JS; entry exit/stop/dispose hooks | Worker count, packaging layout, browser multicore |
> | IO fs/net/process | Private nominal resource values and raw Int signatures from first implementation; explicit wrappers, no raw-number injection exports, kind checks, leases, provisional/Ready transfer and host teardown; process supervisor | No intermediate externref services or fd-as-number adapter |
> | IO stdio | Per-source broker with context subscribers, copied output buffers/leases, compatibility printer, complete target matrix/tests | Preview-1 adapter project |
> | Native packaging after R2/IO | Split compile/load host configuration; compiler-free runner, authenticated payload/plan, exact artifact manifest, N4 validation and default build | Cross-target distribution/signing choices must be fixed before release |
> | MC after MC-D/G5 | N7 queue, N8 typed locked transactions, identity/cycle-safe codec, host transport/wake, claim/cancel reclamation, N12 domains/controller, quiescence and browser stop-safe entry | Started-task migration, lock-free optimization, shared GC |

New sentence(s)/block:

> | Stage | Build now so it is not redone | What waits |
> |---|---|---|
> | R1 | Immutable published SplitMix and tagged paths exactly as runtime §4; no task/IO dependency | Nothing native-specific blocks R1 |
> | R2 request layer | Local Request/Ticket and group-observer cells, portable group observations without Ticket enumeration, atomic subscriptions and separate program/control Root accounting; one root hand-off, consuming take, borrowed await and ordinary await_owned; no resource recognition for Int | Native packaging, worker transport |
> | R2 scheduler | Complete task records, detached routing services, retained environment frames, silent pinning, typed keys and single Scheduler boundary; FIFO/CML semantics, pure fresh-key events, stateful-leaf validation, owned-plus-borrowed waitable sets, combined go/run counters and R4a joins | Queue encoding/claim ownership, locked transport implementation and worker entry |
> | IO clock, before Fs | General §2.4 nominal field-privacy fix; host ResourceContext owner/controller, non-reusing Int allocator and trusted binding-plan/grant plumbing on Rust and JS; entry exit/stop/dispose hooks | Worker count, packaging layout, browser multicore |
> | IO fs/net/process | Private nominal resource values and raw Int signatures from first implementation; owning await_owned/await_resource wrappers, no raw-number injection exports, kind checks, leases, provisional/Ready transfer and host teardown; process supervisor | No intermediate externref services or fd-as-number adapter |
> | IO stdio | Per-source broker with context subscribers, copied output buffers/leases, compatibility printer, complete target matrix/tests | Preview-1 adapter project |
> | Native packaging after R2/IO | Split compile/load host configuration; compiler-free runner, authenticated payload/plan, exact artifact manifest, N4 validation and default build | Cross-target distribution/signing choices must be fixed before release |
> | MC after MC-D/G5 | N7 queue, N8 typed locked transactions, identity/cycle-safe codec, host transport/wake, claim/cancel reclamation, N12 domains/controller, program/control quiescence and browser stop-safe entry | Started-task migration, lock-free optimization, shared GC |

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization. N-R2: distinguish owned non-creating operations from borrowed waits and ensure cancellation releases their leases; retain creator guards. N-R3: count program producers and pending program delivery for deadlock, separate control wakes through ordinary Root/Owner accounting, and preserve ABI H4. N-R4: preserve runtime’s pure fresh-key events, validate stateful leaves, and derive portability from payloads/captures. N-R5: retain the owned-plus-explicitly-borrowed waitable set, without counting unrelated roots. N-R6: use the shared go/nested-run counter for paths and random derivation. Record the review source, exact regression expectations, phase/spec handoff and verification limits; preserve revision-3 history.

### R4-034 — docs/design/native.md, 60f37f2 line 630

Old sentence(s)/block:

> The documentation audit checked the three allowed paths, 20 i code blocks, 18
> Markdown tables, balanced fences, table widths and an old-keyword scan. It also
> searched every stealing/CAS/host-reference/resource signature reference and
> checked the native/runtime/multicore section targets. This is a text check,
> not a parser/type-check of the proposed APIs. The sentence ledger's coverage
> and git diff --check are checked again before committing.

New sentence(s)/block:

> The revision-3 documentation audit checked the three allowed paths, 20 i code blocks, 18
> Markdown tables, balanced fences, table widths and an old-keyword scan. It also
> searched every stealing/CAS/host-reference/resource signature reference and
> checked the native/runtime/multicore section targets. This is a text check,
> not a parser/type-check of the proposed APIs. The sentence ledger's coverage
> and git diff --check are checked again before committing.
>
> ### 11.1 Revision-4 review repairs and adversarial cases
>
> Source: the coordinator's N-R1–N-R6 rulings and independent review
> `report-review-native3.md`; its full cases are under `/tmp/review-native3/`.
> These rulings retain packaging, one Store per worker, publication at go, one
> lock, nominal Int resources and all four encapsulation rules. The following
> are design tests to implement, not programs executed against the placeholder
> request/task/IO modules in this lane.
>
> **Choice for N-R1.** Compare three designs by locality, placement-independent
> answers, principal types and the amount R2 must change later: remote/proxy
> Tickets need a new management-handle protocol; pinning Root/Owner pins nearly
> every task; group observations keep Root/Owner portable and never expose another
> worker's cells. Choose the third as ruled. Keep typed local cells from the first
> and local implementation indexes from the second, but expose neither as a public
> enumeration. No worker-local enumeration API remains, so no new static
> non-portability type is needed. Group waits use Await with private local
> observation cells; they cannot stop the entire scheduler while waiting for a
> sibling. Readiness is count zero, counts are atomic snapshots, cancel-all affects
> current Waiting members without closing the group, and wait-none returns on its
> next zero observation (runtime §5.1). These operations observe sets of work,
> not worker placement, and add no inference choice. Their uses include batches,
> timeouts and plugin cleanup, not only tasks.
>
> **N-R1 enumeration replacement.** Hold the host gate until after the count:
>
> ```i
> (i/task, i/request)
> @host("review:gate/v1", "request")
> Gate { Gate(wait: () -> Request(Nil)) }
> @main
> main = (gate ! Gate, root ! request.Root) {
>   pending = gate.wait()
>   task.scope((s) {
>     child = task.go(s, () { request.count_root(root) })
>     count = task.join(child)
>     request.cancel(request.ticket(pending))
>     assert(count == 1)
>     assert(request.settled_root(root))
>     request.wait_none_root(root)
>   })
> }
> ```
>
> Expected: count is 1 whether child runs on the maker or another worker; after
> cancel the count is 0, readiness true and wait-none returns Nil. No operation
> returns the maker's Ticket. Also run two groups on separate workers: cancel-all
> of one leaves the other's count 1; cancel-all-root makes both zero but permits
> later starts; close-root forbids later starts. Complete between check and
> subscription: the waiter must still finish. A group waiter and a sibling that
> sets the gate must finish on one worker too, since Await suspends only the
> waiter. Cancelling a group waiter removes its observation without cancelling
> the producer. The observer itself never adds a unit to the group it waits for.
> After a remote cancel-all returns, query state on the maker before it processes
> its notification: it must already observe Cancelled. Then start a second read
> on the same Stream: the cancelled request's lease must already be revoked.
> Host cancellation handles apply that action without a Store borrow; local cell
> operations reconcile shared terminal state and never re-cancel a retired ABI id.
> Merely queuing a notification failed this attack, so runtime §5.1 now requires
> these synchronization points before the MC implementation gate.
>
> **N-R2 race then read.** File/Stream/Child non-creators, including close, use
> request.await_owned (runtime §5.4/§7). Resource creators use await_resource;
> request.await remains borrowed. Test with a live Stream and a host fixture
> that keeps the first read pending until the second branch wins:
>
> ```i
> (i/net, i/task)
> twice = (stream: net.Stream) {
>   task.race(() { net.read(stream, 1) }, () { <<>> })
>   net.read(stream, 1)
> }
> ```
>
> After the race has returned, deliver byte 97 only to the second read. Expected:
> `<<97>>`, with no busy error and no abandoned first read consuming that byte.
> The test scheduler forces the first read to acquire its lease before selecting
> the second branch; no sleeps decide the order. Repeat at 1/2/4 workers. For
> File, cancellation of a blocking read instead makes the alias closed, as the
> existing cursor rule requires. For Child, cancelled wait releases the wait lease
> and a subsequent wait returns its cached/eventual status without busy; it does
> not terminate the child. Cancelling a borrowed request.await waiter leaves its
> producer live. Close cancellation never undoes independent host cleanup.
>
> **N-R3 wake-only deadlock.** Replay the review trace: worker 0 waits with one
> wake request; the last runnable task on A registers sync(never()); there is no
> program IO. Both driver wakes, if present, belong to ordinary control Roots,
> scoped to their driver and closed at its exit. Program Root count = 0; explicit
> borrowed producer count = 0; runnable/running/claimed work and pending program
> delivery = 0 after registration. Record panic("task: deadlock") under the
> shared lock even when control count is 1 or 2. The stop closes/wakes the drivers.
> If a program request is still Waiting, postpone deadlock until it settles;
> if its answer is ready to deliver, count that delivery until the local scheduler
> rescans. Repeated wake-only notifications must not mask the zero snapshot.
> The same ordinary roots and selection policy can separate maintenance work in
> a batch dispatcher, a timeout manager or a plugin loop; no compiler rule names
> a library, namespace or wake operation. All actual host requests still count
> for H4/status 1, independently of this program-progress predicate.
>
> **N-R4 pure events.** Runtime §6.1/§6.4 wins; multicore §4 and §18 rules 5,
> 11–12 now distinguish stateful handles from pure event descriptions:
>
> ```i
> (i/task, i/control, i/core)
> event = task.always(1)
> @main
> main = () {
>   task.run(() { assert(task.sync(event) == 1) })
>   task.run(() { assert(task.sync(event) == 1) })
>   control.boundary((b) {
>     e = task.map(task.always(Nil), (_) { b })
>     assert(core.not(core.portable(e)))
>   })
> }
> ```
>
> Expected: both syncs give 1 without a creating Scheduler or run for event; the
> boundary capture makes e non-portable by the ordinary capture walk. Two fresh
> always(1) constructions compare unequal; copying one preserves equality.
> A receive/join leaf from another run still stops at sync, even when it loses
> to always(1); validate all stateful leaves before registration. A pure event
> has no invented task path, and user callbacks may capture any ordinary value.
>
> **N-R5 borrowed waitable set.** A raw outer-root gated request may be borrowed
> by a nested run on the same worker:
>
> ```i
> (i/task, i/request)
> @host("review:gate/v1", "request")
> Gate { Gate(wait: () -> Request(Nil)) }
> @main
> main = (gate ! Gate) {
>   pending = gate.wait()
>   task.run(() { request.await(pending) })
> }
> ```
>
> Expected: after the fixture answers, the nested run returns Nil, without a
> deadlock stop. Its logical waitable set is owned program-root work union
> explicitly awaited borrowed Waiting work, deduplicated by identity; group
> observers contribute underlying members, not themselves. Repeat with an
> unrelated outer request and sync(never()) inside: now the nested run deadlocks.
> A captured Request pins a child normally; no cross-worker borrow is introduced.
>
> **N-R6 combined counter.** The review's go/run/go case is:
>
> ```i
> (i/task)
> @main
> main = () {
>   task.scope((s) {
>     a = task.go(s, task.path)
>     nested = task.run(task.path)
>     b = task.go(s, task.path)
>     assert((task.join(a), nested, task.join(b)) == ([1], [2], [3]))
>   })
> }
> ```
>
> Expected: ([1], [2], [3]) on all worker counts, with the second child's generator
> derived at P(3), not P(2) (runtime §4/§6.1; multicore D10/§5/§18.11).
>
> **Additional decisions and sources.** The group-operation names and private
> observer implementation make N-R1 concrete using existing Await/typed-cell
> rules. No observer receives a host id or registers as its own producer; this
> follows N-R3 and prevents self-counting deadlock. A first draft made wait-none
> a direct root wait; attack with a sibling-controlled gate showed that this could
> pause the sibling on one worker, so the final rule routes it through Await.
> Atomic check/subscription and atomic request-to-delivery accounting follow N-R1's
> placement rule, N-R3's quiescence rule and N8's one-lock transaction discipline;
> they close lost-wakeup and false-zero races, not new public capabilities.
> Synchronous group-cancel acceptance plus host lease revocation and local terminal
> state reconciliation are also forced by N-R1: notification delay must not expose
> Waiting or busy after the same cancellation has already returned on another worker.
> N9.3/N12 already require nonblocking host cancellation/cleanup independent of
> worker progress; MC extends that existing ownership seam to group operations.
> All observers and control Roots are bounded by structured waiter/driver lifetimes.
> No new detached task, library-name privilege, host wire type or inference rule
> is introduced. The helper uses existing resource.ensure (spec §9.5;
> lib/i/resource.i:8–13); current host cancellation drops work outside its slot lock
> (crates/ilang-host/src/requests.rs:161–184). That code supports the intended
> ownership seam but does not yet implement the proposed resource lease behavior.
>
> **Revision-4 checks and two final attack passes.** After the observer and
> cancellation fixes, pass 1 replayed the six review cases and varied group close
> versus start, zero versus subscribe, cancel-all versus late answer, borrowed
> waiter cancellation, and pending/Ready/Taken IO cancellation. It checked that
> private observer dependencies cannot count themselves, group operations reveal
> no foreign cell, and wake-only deliveries cannot defer deadlock. No further
> serious defect was found. Pass 2 checked the same contracts against spec §2.4,
> §9.5/§9.7/§10.3 and ABI H4/H13, runtime's exact replacement text, multicore's
> invariants and §18, top-level events/captured boundaries, and the combined path
> example. No further serious contradiction was found; the intentional spec
> migration below is still required.
>
> Executed an abstract Python model through stdin: 2,880 group action-order/home
> assignments, six observer subscribe/finish/cancel orders and 192 quiescence
> snapshots with varying control counts passed their stated invariants. Two lease
> outcomes checked cancellation/completion before a subsequent read. This model
> assumes the required atomic transitions; it tests the design accounting, not
> the unbuilt notification/lease implementation. The documentation audit checked
> 26 i code blocks and 18 tables in the active notes, balanced fences, table
> widths, accepted-syntax keyword scans, cross-reference targets and stale API
> phrases. It is not parsing or type-checking. The existing ensure and cancellation
> seams were read at the file/line citations above; no compiler, IO, OS, JS or
> multicore execution was performed for this revision. The revision-4 ledger
> records every changed paragraph/code/table block against 60f37f2 verbatim,
> including every changed sentence; historical revision-3 records are preserved.
>
> **Work order and spec reconciliation.** R2 must implement these group/observer,
> ownership, event, waitable-set and combined-counter contracts now, including the
> separate control-root seam; IO must use await_owned from its first non-creating
> wrapper. General field privacy, binding grants and ResourceContext plumbing still
> precede Fs. MC-D must prove typed notification routing and the quiescence snapshot
> before MC-I; worker transport, encoding and packaging remain later stages. The
> six semantic defects are resolved; no known API replacement is deferred. This
> is not a promise that an unbuilt MC prototype will need no implementation changes.
>
> The three notes now agree on these six findings. Spec edits remain intentional
> pending work: runtime §10 supplies Request/Root/Owner/Await (§9.11), request-mode
> host services (§9.7), contextual entries (§10.2), task/events (§9.9), and their
> primitive declarations; native §10 supplies packaging and host-policy text and
> the general §2.4/§6.7 field-privacy clarification. The unchanged §10.3 still
> excludes nominal resource wrappers from the wire. Appendix B still counts all
> actual host requests for status 1; it is not the task deadlock predicate.
> No unresolved contradiction among these three active contracts was found in the
> revision-4 passes; historical old/new quotes are not active rules.

Reason: N-R1: use placement-independent group observations/cancellation, keep all cells local, and remove foreign-Ticket enumeration; specify observer and cancellation synchronization. N-R2: distinguish owned non-creating operations from borrowed waits and ensure cancellation releases their leases; retain creator guards. N-R3: count program producers and pending program delivery for deadlock, separate control wakes through ordinary Root/Owner accounting, and preserve ABI H4. N-R4: preserve runtime’s pure fresh-key events, validate stateful leaves, and derive portability from payloads/captures. N-R5: retain the owned-plus-explicitly-borrowed waitable set, without counting unrelated roots. N-R6: use the shared go/nested-run counter for paths and random derivation. Record the review source, exact regression expectations, phase/spec handoff and verification limits; preserve revision-3 history.

### R4-035 — docs/design/native.md, 60f37f2 line 665

Old sentence(s)/block:

> The entries below preserve each replaced sentence/block verbatim (including code/table
> lines), with its replacement and reason. A block may contain several sentences that
> changed together; nothing inside the old/new quotations is a second active rule. This
> ledger is generated from the lane base to the final three notes, excluding the ledger
> itself. It includes native's adoption/status corrections as well as runtime/multicore
> changes.

New sentence(s)/block:

> The entries below preserve each replaced sentence/block verbatim (including code/table
> lines), with its replacement and reason. A block may contain several sentences that
> changed together; nothing inside the old/new quotations is a second active rule. This
> revision-3 ledger was generated from the lane base to 60f37f2, excluding the
> ledger itself; the appended revision-4 ledger records subsequent changes. It includes native's adoption/status corrections as well as runtime/multicore
> changes.

Reason: Record the review source, exact regression expectations, phase/spec handoff and verification limits; preserve revision-3 history.
