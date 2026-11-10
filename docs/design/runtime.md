# Runtime: random, tasks, and standalone IO

Status: proposed joint design for R1, R2, and IO, revised after the coordinator
review of aa20548. Code references remain at `1e0d910` unless another commit is
named. The coordinator has accepted contextual top and blocking WASI, rejected
universal async entries, and required the join/Owner corrections recorded below.
This note changes no implementation. Examples use the accepted syntax, including
capital type parameters, imports in parentheses, and clause blocks.

## 1. Problem, sources, and evidence

A server needs several pending accepts, reads, and writes without suspending its
whole scheduler. Cancellation must release its sockets and run i final functions.
The language must also support pure, repeatable random computations and tests
that control task order. These facilities must work under `i run`, without a
user-written host or a Rust implementation of the scheduler.

The controlling language rules are spec §4.1 (sets of values), §4.7 (principal
inference and the value restriction), §9.2 (call-site services), §9.4–9.5
(one-shot continuations and final functions), §10.2, and Appendix B. The syntax
is that of `syntax.md`, especially its replacements for §2.2–2.5 and §6.12.
`multicore.md` §18 rules 1–25 are requirements throughout this note; its §13
already reserves §9.8 for keys and §9.9 for tasks. `host-concurrency.md` §6.4
requires request values and one root continuation. `lazy-reentry.md` D16 and
§7a require a retained frame around task environment statements. The plan's
“已定的设计”, R1/R2/IO stages, and hot-file merge order apply. The ledger's
G2(S7) decision overrides the older request for an explicit top-level Runner:
entry points supply it, while `task.top(seed)` remains available.

Read facts at the base commit (line references below always mean this commit):

| Fact read in code | Evidence |
|---|---|
| Seven runtime modules are reserved, included, and empty | `crates/ilang-package/src/library.rs:14–23`; `lib/i/task.i:1`, and the single reservation comment at line 1 of each of `lib/i/{random,clock,fs,net,process,stdio}.i` |
| Libraries are compiled into the package loader | `crates/ilang-package/src/library.rs:7–26` |
| Entry environments currently allow host services and Assert only | `crates/ilang-check/src/types/infer.rs:376–429` |
| Async metadata currently follows entry host-service needs | `crates/ilang-wasm/src/lower.rs:916–938` |
| Async imports currently start, suspend to the root, then read a reply | `crates/ilang-wasm/src/lower.rs:1140–1202` |
| The compiler table still has one slot; the hand-off identifies everything R2 must replace | `crates/ilang-wasm/src/emit/host.rs:23–52`, `:58–64`, `:90–121` |
| Results still use per-import globals; resumes write them before resuming the root | `crates/ilang-wasm/src/emit/host.rs:228–246`, `:335–358` |
| The async-live check is separate from outstanding work | `crates/ilang-wasm/src/emit/host.rs:123–165` |
| Root initialization gives Assert and the needed host services | `crates/ilang-wasm/src/emit/mod.rs:805–836` |
| Resume activates a task boundary; cancel does not | `crates/ilang-wasm/src/emit/control.rs:260–295` |
| The host already stores many requests, with immediate cancellation and stale-wrapper rejection | `crates/ilang-host/src/requests.rs:72–192` |
| Status 2 gives the reactor a turn and uses continue if no answer is ready | `crates/ilang-host/src/lib.rs:1137–1183` |
| One big-stack thread owns an enabled current-thread reactor | `crates/ilang-host/src/run.rs:28–68`; tokio production features are currently rt/time, `crates/ilang-host/Cargo.toml:15` |
| Only the old text printer is currently standard IO | `crates/ilang-host/src/services.rs:44–70`; `lib/i/io.i:8–14` |
| Integer bit operations and bit-array operations needed by R1 and wire codecs exist | `lib/i/core.i:67–89`, `:131–141` |
| A final function cannot raise; acquire is already ordinary i over ensure | `lib/i/resource.i:8–20` |

These are reading results, not claims that the proposed APIs already compile.
The scheduler, request-value representation, new service implementations, and
examples below have not been run. K1 has landed on redesign at `578a17d`.
I read its first-parent patch: `crates/ilang-check/src/types/display.rs:993–1040`
now counts invariant arguments on both sides during normalization. This lane
still has the older compiler; R2 must consume K1 rather than weaken key
invariance. The old generalization path is `types/infer.rs:2104–2120` and
`types/display.rs:558–575`. K1's coordinator-run tests are not tests run here.
Section 12 separates model checks from implementation evidence.

## 2. Alternatives and choice

Choose by sound typing without casts or rank-two fields; compliance with all
25 task rules; ability to implement it in the code above; small general language
surface; explicit cancellation ownership; and standalone IO on each target.

1. **A Rust executor and one continuation per host request.** This needs little
   i code and fast mutable queues. It fails the i-scheduler requirement, and a
   root continuation per request captures separate scheduler histories
   (`host-concurrency.md` §6.4). Reject it. Keep Rust's reactor and its existing
   cancellation protocol.
2. **Explicit request-returning IO only.** Every operation returns a typed
   request; libraries explicitly await it. This has the smallest primitive
   layer, and permits batches without tasks. Used alone, existing async host
   calls could still capture a running scheduler. Keep its request API and
   raw service form, but give ordinary async calls the same wait route.
3. **Typed requests, a replaceable wait service, and an i scheduler.** Requests
   describe host work. A normal service routes an await to the calling task;
   only the run loop uses the root wait/poll operations. Typed keys store the
   loop's heterogeneous state. Choose this design. Keep ordinary blocking-looking
   function calls from design 1, and explicit starts/batches from design 2.

No worker count, unsafe cast, static Send constraint, keyword, generic record
label, or special treatment of any IO namespace is added. The compiler knows
host requests and entry wrapping; it does not know channel protocols, task
states, error arbitration, or event commits.

## 3. Decisions

**D1. Random is immutable explicit data.** `i/random` needs no service and does
not import `i/task`. Splitting and task paths use different stream tags.
Reason: `multicore.md` §5 and §18 rule 11; this also makes random useful in
property testing, simulation, and procedural generation without a scheduler.

**D2. Add primitive Request and Ticket types and operations in `i/request`.** `Request(A)` is the
set of handles whose successful completion value belongs to A. Host errors are
values too: a fallible operation returns `Request(Result(A, E))`. `Ticket`
erases the answer type and supports readiness/cancellation only; it cannot read
an answer. Request is covariant; Ticket has no parameter. Each handle is the
same only as itself, and both are non-portable even after completion. Reason:
C §6.4, S7 P5; useful for a scheduler, batched file reads without tasks, and a
first-answer timeout without tasks. There are no linear types or type tests of
A. Cancellation changes the host work associated with a handle, as using a
continuation changes whether it is usable; it does not mutate an i value.

**D3. Await is an ordinary public service, not a task hook.** Its definition is
`Await { Await(wait: (ticket: Ticket) -> Nil) }`. `request.await` asks this
service to wait, then reads the typed result. The compiler's async-host adapter
uses exactly this route. The general async-host declaration rule adds `Await | Owner`
to the callable field's environment before inference; the wire signature has
no environment. This is an explicit replacement of §9.7's old “no environment”
rule, not a silent inference exception for a library. A request-mode declaration
starts work without Await. Every namespace, including user namespaces, follows
the same rules. Await is declared by ordinary i in `i/request`; only Request,
Ticket and the low-level operations need compiler primitives. A fake Await that
returns early causes a defined stop, never an ill-typed answer.

**D4. One root hand-off, with typed answers stored per request.** Each task owns
its ordinary control continuation. Root wait/poll pauses the loop once, and a
resume updates one request cell before resuming that one loop. Reason: C §6.4
and `emit/host.rs:23–48`. A slot index alone is not a language handle: cells
have identity and a fresh GC identity token for their generation, so an old alias cannot cancel a reused slot.

**D5. Provision entry services from the inferred environment.** Let N be the
entry function's ordinary inferred environment. On the ilang target the entry
is async exactly when N contains an async/request host service or Scheduler or
Runner from i/task. Otherwise it remains synchronous. No reachability guess,
attribute on the entry, or change to its principal type is needed. Task services
in N request the default FIFO task run, with root path [] and seed
random.from_seed(0). Async host services without task needs use a direct root
Await handler and request context; they do not set up a scheduler. Pure entries
set up neither a task run nor a host hand-off. Await/Owner/Root alone, for
example for completed fakes, do not turn an entry async; a low-level wait/poll
that tries to hand off there stops with request: no active entry.

This is one narrow entry-point rule for the resolved standard services,
authorized by the coordinator and ledger G2(S7). It is a deliberate provision
for i/task and request services; an arbitrary user scheduler is not implicitly
bootstrapped. IO service values are given only as the inferred needs require,
like Assert and Io today. Channel and scheduling operations remain ordinary i.
The host-import and request primitives themselves have no IO namespace rule.
A pure task run can have async metadata yet finish with status 0 on its first
call: scheduler switches alone never cause a host yield. Progress polling below
requires an actual task waiting for host work; idle waiting also counts
unawaited requests. There is no automatic entropy draw. Contextual top(seed)
keeps the current Runner's route/prefix; it changes only the seed.

**D6. Scopes use module functions, not polymorphic methods.** Write
`task.go(s, body)`, `task.join(t)`, `task.share(s, x)`, and `task.send(c, x)`.
S7's `s.go`, `t.join`, and `channel.new` name operations, not required field
spellings. A Scope(E) record cannot store `go` at every A under §4.7's ordinary
monomorphic field types. Public module bindings can be generalized normally.
Keys, keep_env, portable, and the S7 closure-cycle walk land with R2; no scheduler
state is stored using a forged host reference or an untyped cast.

**D7. Preserve the exact S7 lifetime, portability, and failure rules.** Portable
sends check at the send call; send events check when constructed, even for a
losing branch. Child results/errors check at completion, scope body errors check
before arbitration, and shares check at creation. A scope body's own successful
return need not cross workers and is not checked. Local scopes/channels carry
the maker's boundary solely as a non-portability witness. Scope chains, run keys,
path-order errors, and scope-owned channel storage apply on one worker already.
The implicit run root permits local values and host-export references; explicit
portable scopes constrain their children. Reason: S7 §3.3, §6–8, §18 rules 2–14.

**D8. Cancel by resuming the task with a cancellation answer.** The task then
unwinds its own inner boundary, leaving its scheduling boundary active. Finals
may yield, await IO, and open scopes; they cannot raise, and they are shielded
from a second cancellation. Reason: S7 §6 and the active-boundary behavior read
at `emit/control.rs:260–295`. There is no public cancel-one-task operation in v1.
join keeps its type: cancellation takes priority for a joiner it can unwind; a
shielded joiner of a dropped/cancelled child stops with
`panic("task: join of a cancelled task")` as soon as cancellation is accepted.
Shielded finals invalidate the older argument that every joiner can be
cancelled. Race uses a private scope-success signal and structured cancellation, not a public task kill.

**D9. IO services start requests; ordinary wrappers await and translate errors.**
Opaque files, listeners, streams and children are host references. The portable
combinators cannot return these references; servers use local scopes. Stdio has
its own `i:stdio/v1` service. Keep the normative `i/io` printer for compatibility;
it does not take over or rename stdio. Reason: spec §9.7, §10.3, S7 P4, plan IO.

**D10. The test scheduler is explicit and has no real time.** `task.test` chooses
specified runnable paths, then FIFO when the script ends. Pending real host
requests stop the test scheduler; fakes use completed requests. Host integration
tests separately drive deterministic completion gates. Determinism requires
fixed fake inputs, not just a fixed random seed. Reason: S7 rules 17 and 22.

## 4. R1: the public random module and exact algorithm

The following is the normative API table. Signatures are schemes in the new i
syntax; implementations are bindings, not bare non-external declarations.
Generator's constructor is private. Its values contain a normalized seed,
SplitMix state, tagged derivation history, and nonnegative draw index; equality
is ordinary structural equality. No service or i/task import is needed.

| Name | Type |
|---|---|
| `from_seed` | `(seed: Int) -> Generator` |
| `word` | `(g: Generator) -> (Int, Generator)` |
| `int` | `(g: Generator, below: Int) -> (Int, Generator) ? RandomError` |
| `float` | `(g: Generator) -> (Float, Generator)` |
| `split` | `(g: Generator) -> (Generator, Generator)` |
| `at_path` | `(g: Generator, path: List(Int)) -> Generator ? RandomError` |

`RandomError { InvalidBound(Int) InvalidPath(List(Int)) }` is exported with its
constructors. int requires below > 0, including arbitrarily large bounds;
at_path requires strictly positive path indices. Invalid arguments raise the
shown error without a draw. at_path(g, []) is identity. task.random derives
from the root's complete generator, including previous draws/splits.

### 4.1 Published generator

Use **SplitMix**, Steele, Lea and Flood's published splittable PRNG
([OOPSLA 2014](https://gee.cs.oswego.edu/dl/papers/oopsla14.pdf)), specifically the
init/nextWord64/splitSMGen variant pinned by the maintained Haskell
[splitmix-0.1.3.1 source](https://hackage-content.haskell.org/package/splitmix-0.1.3.1/docs/src/System.Random.SplitMix.html).
Replace the earlier custom Threefry compression completely. Pinning a version
matters: the two mixers' roles differ between this variant and Java's.

Here is the numerical contract. W=2^64; shifts are unsigned and additions and
multiplications reduce modulo W. M(x) applies xor-shift 33, multiply by
0xff51afd7ed558ccd, xor-shift 33, multiply by 0xc4ceb9fe1a85ec53, xor-shift 33.
V(x) uses shifts 30/27/31 and multipliers 0xbf58476d1ce4e5b9/0x94d049bb133111eb.
G(x) sets V(x)'s low bit; if popcount(z xor (z >> 1)) < 24, xor z with
0xaaaaaaaaaaaaaaaa. For n reduced to 0..W-1, initial state is
(M(n),G(n+0x9e3779b97f4a7c15)). A state (s,gamma) draws M(s+gamma), then becomes
(s+gamma,gamma). Native splitting gives continuation (s+2*gamma,gamma) and
new child (M(s+gamma),G(s+2*gamma)). These are the pinned variant's operations.

from_seed normalizes an arbitrary Int by Euclidean remainder modulo W;
seeds differing by W intentionally give the same generator. word uses that
published next operation and increments the logical draw index. Reusing the
same immutable input repeats the same output; callers thread the next state.
This is not a cryptographic generator. The published family's known gamma
weaknesses are discussed by the authors in their
[2017 follow-up](https://labs.oracle.com/pls/apex/f?p=94065%3A10%3A4081129589733%3A5391).
Choosing a published algorithm does not waive the independent statistical gate.

### 4.2 Tagged streams by splitting, not a new hash

The only adapter is deterministic navigation of the published split tree.
Define fork(h) by native-splitting h into (rest,a), then native-splitting rest
into (_,b); return the two new children (a,b). Thus an edge 0/1 selects a fresh
published child, not the continuation of the parent's draw sequence. Each fork
uses two published splits and introduces no mixer or key derivation.

Give S(1) code 00 and S(2) code 01. For a positive index i, give P(i) code 1
followed by Elias gamma(i): write the binary representation of i, with k-1 zeros
before its k bits. Examples: P(1)=11, P(2)=1010, P(3)=1011. These code words are
prefix-free. Select the fork child for each bit, using the entire current state.
Public split returns the states at S(1) and S(2); at_path traverses P(i) for
successive list elements. Append the corresponding tagged steps to the logical
history, recording the previous draw index at each derivation; reset the new
index to zero. Deriving after a draw therefore differs from deriving its input.
Consecutive path components are handled identically whether supplied by one
at_path call or by several. Task paths use only P steps; explicit splits use S
steps. Prefix-free concatenation keeps those addresses distinct at every depth,
including arbitrarily large Int indices. Runtime scheduling never selects bits.

This adapter only chooses nodes made by published splitting. It makes no new
statistical theorem: distinct logical streams can give equal words or, with
finite-state PRNGs, colliding states. The R1 review must test this exact traversal,
weak-gamma cases, interleaved tasks, deep split trees and rejection sampling.
Use stable cross-implementation vectors, and do not land R1 if the independent
review rejects the chosen variant or traversal.

For int(g,n), use the least k with W^k >= n, at least one. Draw k words in
least-significant-first order to form u; let limit = W^k - (W^k % n). Reject
u >= limit and draw another group. Return u % n and the generator after all
accepted and rejected draws. This avoids modulo bias under the word model.
float draws one word u and returns floor(u/2^11) / 2^53 as a Float, with its
next generator; the result is in [0,1). At most 53 bits enter the conversion.

## 5. R2a: requests, Await, and the ABI hand-off

### 5.1 Public request API

Request(A) and Ticket are compiler-defined names available in every module,
like Boundary and Continuation. They have no public constructors. i/request
exports the following ordinary status/service types, their constructors, and
functions; it does not redeclare or re-export the primitive types.

```i
Await { Await(wait: (ticket: Ticket) -> Nil) }
Root { Root(key: Key(Nil)) }
Owner { Owner(key: Key(Nil), root: Root) }
RequestState(A) { Waiting Ready(A) Cancelled Taken }
```

| Function | Type | Rule |
|---|---|---|
| `open_root` | `() -> Root ! Root` | Open an independent request run within the current entry context |
| `close_root` | `(root: Root) -> Nil` | Close every live group of this root and the root itself; idempotent |
| `open_owner` | `() -> Owner ! Root` | Register a group at the current root; changing Owner does not change that root |
| `close_owner` | `(owner: Owner) -> Nil` | Cancel its outstanding work and remove the live group; idempotent |
| `outstanding` | `(owner: Owner) -> List(Ticket)` | Current outstanding requests in this group, in start order |
| `outstanding_root` | `(root: Root) -> List(Ticket)` | Outstanding requests of every group of this root, in start order |
| `within(A,E,N)` | `(body: () -> A ? E ! N) -> A ? E ! Root \| N - Owner` | Fresh Owner environment; ensure close_owner on exit |
| `ready(A)` | `(value: A) -> Request(A)` | A fresh completed request; available in pure fakes |
| `ticket(A)` | `(r: Request(A)) -> Ticket` | The same underlying request identity; no answer access |
| `state(A)` | `(r: Request(A)) -> RequestState(A)` | Reads do not consume the answer |
| `take(A)` | `(r: Request(A)) -> A` | Ready becomes Taken atomically; other states stop |
| `settled` | `(r: Ticket) -> Bool` | True for Ready, Cancelled or Taken |
| `cancel` | `(r: Ticket) -> Nil` | Idempotent; cancels only Waiting, discards a Ready answer |
| `await(A)` | `(r: Request(A)) -> A ! Await` | Wait, then take the Ready value |
| `wait_any` | `(rs: List(Ticket)) -> Nil` | Root wait; return when any listed ticket settles |
| `poll` | `() -> Nil` | Root hand-off with status 2, even with no requests |

`cancel` makes every alias observe Cancelled, even for a previously Ready cell.
A Taken request stays Taken on cancel. It invokes ABI cancel only for a still-outstanding host id. Cancelling a Ready
request never invokes that import. `await` or take on Cancelled stops with
`panic("request: cancelled")`; on Taken it stops with `panic("request: taken")`;
take on Waiting stops with `panic("request: not ready")`; if Await.wait returns with Waiting it stops with
`panic("request: wait returned early")`. `wait_any([])` stops with
`panic("request: empty wait")`. A nonempty list may contain aliases; deduplicate
by identity. Terminal tickets make it return immediately. It must keep waiting
through answers of unlisted requests, which are stored for later use. All-listed
cancelled tickets count as settled. No root continuation is stored on a cell.

Cancellation ownership and run affiliation are separate. Root identifies a
request run and its enclosing entry context. The runtime creates the initial
Root only for an entry whose inferred needs require request/task support.
open_root needs an existing live Root to retain that entry context; it does
not require the new root to share the old root's request count. A dispatcher
can thus open an independent root without inventing a second entry. The
initial bootstrap is part of the narrow entry provision, not a public unscoped
root allocator. Entry request contexts and each task.run open a fresh Root and
bind it for their whole lifetime. open_owner uses that Root, not the currently
bound Owner, to register its group. request.within opens/binds a fresh Owner and
ensures close_owner on exit; it leaves Root unchanged. Thus custom groups of a
run have the same root, and a nested task.run has a different root. Both Root
and Owner are ordinary portable key-bearing service values, not host requests.

Every raw host start records its Owner group and that group's immutable root.
close_owner cancels only its own group. close_root closes all groups registered
at that root; no subgroup or old Owner binding escapes it. outstanding(owner)
enumerates one cancellation group; outstanding_root(root) enumerates every group
of the run, including groups made inside request.within and groups retained by
go. Enumeration of a closed group/root gives []; opening a group under a closed
Root stops with request: root is gone; a host start under a closed Owner stops
with request: owner is gone, before making the ABI import. The registries retain
only live roots/groups and outstanding cells; absence supplies the gone check.

Root is kept at go sites as a normal service. Changing Owner alone cannot move
a group/request to another run. open_owner under a nested run's Root makes a
nested group even if user code temporarily rebinds an outer Owner. Explicitly
using a previously made outer Owner for a raw start still charges that outer
group/root; this is a capability use, not a transfer of the group. Borrowed
request tickets may be awaited by another run on the same worker. Opening or
rebinding an independent Root is the public low-level operation for writing a
separate request dispatcher, like calling root wait directly; it does not
silently reaffiliate an existing i/task run's stored root or its old groups.
The default task scheduler counts the root it owns, not whichever Owner is
bound at an idle point. An independent request dispatcher must drive its root.

A Ready cell has left its group and belongs to its handle. Closing a group/root
does not discard such an answer; raw callers must own it just as they own files
returned by open. Completed cells retain decoded i values only while handles
are reachable, not through the outstanding table. Low-level cancellation cannot
roll back a host side effect or run i final functions. Closing a root after its
structured tasks/finals end cancels any still-outstanding unowned raw work.

Root wait/poll require an active async entry context when they must hand off;
otherwise they stop with `panic("request: no active entry")`. Test async-live,
not only the non-null host-root pointer: the old pointer can survive a completed
async call (`emit/host.rs:78–80`, `:125–165`). A later synchronous export must
not hand off through that stale boundary. An immediate wait
on completed fake requests works without that context. They are public primitives
for writing dispatchers, batch IO, or request races. Direct use inside a task
pauses that entire run until it returns; normal await uses Await instead. This
is explicit control use, like suspending to an outer boundary, and does not
create multiple scheduler histories.

Root and Owner are ordinary services, not primitive types or built-in names.
They use Key(Nil) and are portable; outstanding request values remain non-portable.
Root/group registration and enumeration require no ABI import; closing live
work uses only v3 cancel. Generic
request groups also serve batch cancellation, request timeouts, and host plugin
cleanup, independently of tasks. Request operations introduce no new error-row
or environment polymorphism. Ticket
cannot recover A; state only reads through the original Request(A). The existing
value restriction prevents a fresh request from being generalized at multiple
unrelated answer types. A cell's state transitions are Waiting -> Ready or
Cancelled, and Ready -> Taken or Cancelled; there is no write of another A.

### 5.2 Host service declaration forms

Keep synchronous `@host("ns")` as spec §9.7 defines it. For every namespace:

* `@host("ns", "async")` writes its fields with wire type `(P...) -> A ? E`.
  Its actual i field type is `(P...) -> A ? E ! Await | Owner`. Conceptually its body
  starts Request(Result(A,E)), calls request.await, then fail.from_result.
  With no errors it starts Request(A) and awaits it. The attribute's type
  expansion happens before ordinary inference and interface display. A user
  declaration may not add a different field environment to this form.
* Add `@host("ns", "request")`. Every field is `(P...) -> Request(T)`, with
  no written errors or environment; its callable type adds `! Owner`. T is one ABI result type, or Result(A,E) with
  A and E ABI-admissible as in §10.3. Its ABI signature is the same as the async
  function with result A/errors E; no Request or Result is sent across the wire.
  The call starts and returns a handle immediately. A raw field returning
  `Request(Result(Nil,Never))` is allowed, though `Request(Nil)` is simpler.
* No other occurrence of Request/Result is allowed as a boundary type. A
  synchronous function, external, or host export cannot take or return Request.
  Opaque references inside its A are valid, because only the decoded answer is
  a host reference; Request itself never crosses the boundary.

This adds no attribute that names a library or scheduler. The host sees only v3
async functions. Namespace/label uniqueness applies across all three forms.
Fakes construct the same service values with ordinary functions; request.ready
allows immediate fake results. Existing async programs keep their normal-call
behavior, with Await and Owner now visible in their inferred service requirements.

### 5.3 Replacing the compiler request table

Replace the C region as one change, including every caller identified in
`emit/host.rs:23–52`. The module keeps a growable table and a free list. Each
outstanding slot contains import index, ABI id, a fresh GC generation token,
Owner group key, immutable Root key, and a reference to its request cell.
Root/group indexes must remove a request on answer or cancel; completed cells
are not retained by either index. Root enumeration spans every live group,
while closing a group touches only its own slots. Normal entry-context exit
retires every root/group registered in that context, including an independent
root its caller forgot to close. Clearing requests at status 0 must clear those
registrations too; a synchronous fake-only context uses the same registry
epilogue without a host hand-off. Detached Ready handles retain their answers. Each import also keeps its source answer shape (plain A or
Result(A,E)), independently of whether the ABI error list happens to be empty;
Request(Result(A,Never)) must still decode Ok(A). A cell contains identity, generation, Waiting/Ready/Cancelled/Taken,
and a decoded eqref answer. The id is a nonnegative i32 slot index; the table
cannot have more than 2^31 outstanding entries. Exhaustion is a memory stop.
A cell survives freeing its table slot. Language operations use the cell, and
compare slot generation before cancel; they never act on a new occupant.

`$req_start` allocates a cell and slot before calling the import. The raw stub
returns that typed cell: allocate the cell, read its id into the import's leading
ABI argument, call the import, free input buffers, and return the cell instead
of HostStart's current Nil. Replace make_import_func accordingly. `$req_answer(import,id)` checks outstanding status and
import identity, decodes/validates the result into that cell, frees all ABI result
buffers, changes it to Ready, and removes the slot from the outstanding table.
Keep T1's checks of refined result/error value sets on this direct decode path;
an invalid host value must stop, not enter a Ready cell as the wrong A or E.
An ABI error is stored as the Error(e) value in Result(A,E), not thrown through
the resume export; the awaiting adapter raises it later in the calling task.
Only then may it resume `$host_k`. Per-import `$hr` globals may be scratch during
one resume, but cannot retain a pending answer across a resume/continue. Wrong
id/import remains a bare host-fault trap. Copy inputs before freeing buffers.

There is exactly one `$host_k`, for whichever root wait or poll currently handed
control to the host. Both use the existing `$host_boundary`/`$host_resume_k` seam.
The latter may resume with Nil: the loop queries cells, so the answer id need
not become an i Int. Store last-status separately from async-live. Continue
requires async-live and last-status=2, clears that hand-off, and resumes the
same root k. Resume exports may follow status 1 or 2 and consume that same k.
Before calling control, clear `$host_k` so another wait replaces it once.

A root wait returns status 1 only with an outstanding request; no-ready/no-work
is a library deadlock, not status 1. Poll returns status 2. A response to an
unlisted request is decoded and retained; the resumed wait scans its requested
set and hands off again if necessary. `$req_clear` detaches remaining cells as
Cancelled at status 0 and clears outstanding roots; the host drops remaining
work under H7. Structured entry wrapping normally leaves none. Keep the
async-live guards of `emit/host.rs:123–165`; zero outstanding does not mean ended.
No ABI version bump, new import, or new host-called export is needed: continue,
cancel, and typed resume exports are already v3.

### 5.4 Ordinary waiting and cancellation ownership

In a task, Await.wait submits a WaitRequest(ticket) command to its Scheduler,
with the task's inner-unwind target registered privately. The loop records the
waiting task and ticket and queues another task. On each switch it scans settled
tickets; a ticket may have several waiters. Await consumes the answer once; multiple
waiters deliberately competing for it may therefore meet request: taken.
The plain async adapter owns its just-started request through resource.ensure,
and cancels it on exit, including while unwinding. A raw starter gives ownership
to the caller, who can use the same pattern. Cancelling one waiter of a manually
shared raw request does not cancel the producer unless its owner's final function
does so; cancelling a ticket wakes all waiters, whose await then stops as above.

Because request.cancel also discards Ready answers, an implicit adapter must
take its A before its final cancels the cell. For resource-creating imports,
IO wrappers additionally transfer resource ownership as section 8 specifies;
discarding an unclaimed Ready answer releases its resource. These are two
separate ownership transfers, not a promise that all raw results are resources.

## 6. R2b: the task API and scheduler

### 6.1 Public task types and functions

Export types Scheduler, Runner, Scope(E), Task(A,E), Channel(A), Event(A,E,N),
Shared(A), and ScheduleError; export only ScheduleError's constructors. The
other constructors and scheduler protocol are private ordinary custom types,
not host opaque types. Scope, Task, Channel, and Shared contain invariant keys;
their parameters are therefore invariant. Event's answer/error/environment
parameters are covariant: its private representation keeps typed decoding
functions, rather than Key(A) in an output field. A public Event can hold user
mapping functions and the local handles of its leaves. Such contents affect
portability by the normal walk. The library creates no event that captures its
Scheduler or mutable loop state. A join event keeps its task's scope handle.

Schemes below declare their capital variables on their bindings. N is the body
service requirement; supplying Scheduler/Runner/Await/Owner/Root discharges those members.
Assert and user host services remain in N where the body needs them.

| Binding | Type |
|---|---|
| `top` | `(seed: random.Generator) -> Runner ! Runner` |
| `run(A,E,N)` | `(body: () -> A ? E ! N) -> A ? E ! Runner \| N - Scheduler - Runner - request.Await - request.Owner - request.Root` |
| `test(A,E,N)` | `(seed: random.Generator, choices: List(List(Int)), body: () -> A ? E ! N) -> A ? E \| ScheduleError ! Runner \| N - Scheduler - Runner - request.Await - request.Owner - request.Root` |
| `scope(A,E,N)` | `(body: (s: Scope(E)) -> A ? E ! N) -> A ? E ! N \| Scheduler` |
| `local_scope(A,E,N)` | Same scheme as scope |
| `go(A,E,N)` | `(s: Scope(E), body: () -> A ? E ! N) -> Task(A,E) ! Scheduler \| N - Scheduler - Runner - request.Await` |
| `join(A,E)` | `(t: Task(A,E)) -> A ? E ! Scheduler` |
| `yield` | `() -> Nil ! Scheduler` |
| `path` | `() -> List(Int) ! Scheduler` |
| `random` | `() -> random.Generator ! Scheduler` |
| `keep(S,A,E,N)` | `(f: (S) -> A ? E ! N) -> { (S) -> A ? E ! Scheduler } ! Scheduler \| N - Scheduler - Runner - request.Await` |
| `channel(A)` | `() -> Channel(A) ! Scheduler` |
| `local_channel(A)` | `() -> Channel(A) ! Scheduler` |
| `send(A)` | `(c: Channel(A), value: A) -> Nil ! Scheduler` |
| `receive(A)` | `(c: Channel(A)) -> A ! Scheduler` |
| `send_event(A)` | `(c: Channel(A), value: A) -> Event(Nil,Never,Never)` |
| `receive_event(A)` | `(c: Channel(A)) -> Event(A,Never,Never)` |
| `join_event(A,E)` | `(t: Task(A,E)) -> Event(A,E,Never)` |
| `always(A)` | `(value: A) -> Event(A,Never,Never)` |
| `never` | `() -> Event(Never,Never,Never)` |
| `choose(A,E,N)` | `(events: List(Event(A,E,N))) -> Event(A,E,N)` |
| `map(A,B,E,F,N,M)` | `(e: Event(A,E,N), f: (A) -> B ? F ! M) -> Event(B,E \| F,N \| M)` |
| `sync(A,E,N)` | `(e: Event(A,E,N)) -> A ? E ! N \| Scheduler` |
| `share(A,E)` | `(s: Scope(E), value: A) -> Shared(A) ! Scheduler` |
| `get(A)` | `(s: Shared(A)) -> A ! Scheduler` |
| `both(A,B,E,N)` | `(left: () -> A ? E ! N, right: () -> B ? E ! N) -> (A,B) ? E ! Scheduler \| N - Scheduler - Runner - request.Await` |
| `race(A,E,N)` | `(left: () -> A ? E ! N, right: () -> A ? E ! N) -> A ? E ! Scheduler \| N - Scheduler - Runner - request.Await` |
| `map_concurrently(A,B,E,N)` | `(xs: List(A), f: (A) -> B ? E ! N) -> List(B) ? E ! Scheduler \| N - Scheduler - Runner - request.Await` |

In tables, `\|` is only Markdown escaping; the i operator is `|`. The schemes
for combinators use one common E and N; subtyping admits either child's error
and needs. No union of invariant Task(Int,E) and Task(String,E) is required:
`both` keeps two separate handles. No local variants of these three combinators
are exported in v1. Local callers write a local_scope explicitly.

The differences in the go/keep/combinator environments matter: their wrappers
supply Scheduler/Runner/Await themselves, so requiring all of N at their call
sites would overstate their principal needs. Owner is kept, and is not subtracted
there. Root is kept there too. run/test supply a fresh Root and default Owner,
so they subtract both. test still needs
the calling Runner for its nested path/wait route. Attribute expansion for host
fields happens before ordinary constraint generation; it adds fixed ordinary
service types, not a type-dependent constraint. Keys are invariant and obey the
existing value restriction; Request and Event add no qualified types or higher
rank quantification. Thus the design reduces inference to the existing rules,
with the new primitive variance rows. The prototype gate checks that argument
against the implementation rather than assuming it from the notation.

No public spawn detaches a task. `go` never checks its closure's portability.
It increments the calling task's spawn/run counter, beginning at 1; the child
path is the parent's path with that index appended. A nested run takes an index
from that same counter. `path` and `random` are data reads without a suspend.
A top seed that was already split or drawn keeps that entire derivation state.

A scope waits for all children even when its body returns without joining them.
Returning a Task handle does not extend the right to join it. Handles check run
identity before other checks, then chain membership or liveness. `share` checks
membership as `go` does (same outside-scope stop), including local scopes; values
shared through either scope kind must be portable. get checks run then share
liveness. Handle equality means the same object of the same run, never equal
numeric ids in two runs. Text uses constructors without hidden ids: a private
identity-bearing key is printed as `//key` per S7, and implementation ids/paths
are kept under Keyed fields whose text is //keyed, not exposed as plain fields. `path()` alone exposes the semantic task path.

### 6.2 Scopes, errors, and cancellation

R1. A task's chain is its active scope bodies, inner first, followed by its
starting scope's chain. A scope's chain is itself plus its opener's chain at
opening. `go` and join require the relevant scope in this chain. Run-key mismatch
stops with `task: handle of another run`; failed membership stops with
`task: go outside its scope` or `task: join outside its scope`. These messages
are passed to panic, so the stop prefix is `panic: `.

R2. A scope body executes in the opener task under an inner scope boundary;
it does not start another scheduler. A child executes from the run loop under
its own task boundary. On body/child failure, mark the scope cancelling, cancel
its body and every child, and propagate down their active scope trees. Wait for
all unwind operations, including scopes created by finals. Choose the least
path among actual failures; a prefix sorts before its extensions and the scope
body has the opener's path. A cancellation is not a failure. Types accumulate
ordinary error unions; stops cannot be caught. A go accepted while cancelling
creates an unstarted, immediately dropped child and its ordinary Task(A,E)
handle. A shielded final receives that handle; its body never runs. If that
scope's cancellation can unwind the caller, the command delivers cancellation
instead of returning the handle. This priority also applies at a valid join.
A cancelling scope never starts new user work just to manufacture a join value.

R3. On a normal scope exit, join every live child before returning its body
value. Before either successful or failing return, free the scope's channels,
shares, completed-task table entries, and registration records. The owning
Scope/Task handle can survive but does not retain its result/state after exit.
Reads needing dead state stop by the defined checks. Channel ownership is the
first scope in the maker's chain, or the run. A channel wait still present when
that owner ends stops with `task: channel is gone`, including a waiter in an
outer scope; it is not silently dropped. share get after exit stops with
`task: shared value is gone`. Scope ends are scheduling/commit steps.

R4. A paused or runnable started task receives Cancel at the next switch after
cancellation reaches it. A running task sees it at its next scheduler suspend.
An unstarted task is dropped without calling its body. The Scheduler command
which receives Cancel suspends to the innermost cancellation delimiter being
cancelled; that handler calls control.cancel only on that inner continuation.
The outer task boundary stays active. Finals use the task's saved services;
all awaits and sends they perform go to that still-active task boundary. An
unwind flag excludes that task from a second cancel, including cancellation
of scopes it creates while unwinding. Such new scopes still obey their own
internal failure/normal-exit rules. The outer cancel waits for them to end.
A final that waits forever can keep cancellation waiting forever.

R4a. Keep outcome and cleanup completion separate. A task outcome is pending,
success, failure or cancelled; drop sets cancelled immediately, and a started
task sets cancelled when its home scheduler accepts cancellation, before running
its unwind. A mere cancellation request not yet accepted is not a cancelled
outcome: that task can still finish first. Normal success/failure is recorded
after its final functions end. Scope exit still waits for cleanup completion,
independently of the outcome. Keep dropped/cancelled records until scope exit.
A cancelled outcome retains its causing scope and cancellation epoch, so a join
can apply that cause even before its caller receives a cancel message. Do not
use only a worker-local “cancel delivered” flag to decide whether to stop.
join's type remains `(Task(A,E)) -> A ? E ! Scheduler`. After the run/chain checks, process any applicable scope cancellation
before observing a target's cancellation outcome. A joiner still cancellable by
that cancellation is unwound; it never races between this path and a join panic.
This includes a final executing a fresh scope body that the fresh scope can
cancel: unwind that inner body, not the already-shielded outer final. A cleanup
scope and its children are exempt from the older cancellation as R4 requires.

A joiner shielded from the target scope's cancellation cannot obtain A or E
from a dropped/cancelled child. It stops immediately with
`panic("task: join of a cancelled task")` when that outcome is known; it does
not wait for the cancelled target's final functions. If it joined while the
outcome was still pending, accepting cancellation wakes that join immediately.
This also avoids cycles between shielded finals joining one another. The stop
ends the run and, as with any stop, may abandon remaining i finals. In a valid
structured exit without that stop, scopes still wait for all cleanup to finish.
An already recorded success or failure gives ordinary A or E. No Cancelled
error value, aggregate error, or changed join signature is introduced. A join
can wait for genuinely live work, but never for an already cancelled outcome.

A join_event has the same rule when selected/committed. A known cancelled
outcome makes that alternative ready, even if cleanup is not complete; it is not a prevalidation fault of a losing
alternative. A cancellable syncing task processes cancellation first. A shielded
syncing task stops only if that join alternative wins; losing map functions
still do not execute. This preserves CML selection and prevents an invalid
stop from racing cancellation of an ordinary joiner.

R5. A race commits the first successful child completion observed by its
scope. Commit atomically records the winner and initiates the ordinary scope
cancel of the loser and body wait. Return only after their finals end. Failure
before a success commits invokes R2; a failure already observed wins over a
later success. Once success has committed, cancellation suppresses later
unobserved errors. Thus race is first completion unless that completion was a
failure; errors use the same scope rules as other combinators. It is not “first
successful result despite failures”. Both/map_concurrently give argument/input
order and R2's errors. All three use portable scopes and check child results
and errors before recording a completion, including race losers that finish
before cancellation reaches them.

R6. The runtime entry wrapper and every explicit run own a local root task,
which they run to completion with structured children. An entry error is
reported only after the run has unwound; a host export receives its ordinary
error. A stop immediately traps and runs no finals, as §9.5 requires. Host
trap/failure/drop also invalidates the instance under ABI H8–H11; it cannot
promise to resume i cleanup. “Cancellation runs finals” refers to i structured
cancellation, not destruction of a trapped instance by its host.

### 6.3 Typed storage and one loop

A feasible monomorphic internal protocol is:

```i
StepAnswer { Proceed(value: Keyed) Cancel }
Scheduler { Scheduler(step: (command: Keyed) -> StepAnswer) }
```

The real representation also carries task path/seed, the current task boundary
witness, and the task's Runner/Await. These are private fields, never serialized
as a portable Scheduler. Commands use ordinary private custom types; their
erasure through Keyed is intentional. Different typed operations do not cast:
a Channel(A) has its Key(A); send wraps with that key, receive unwraps with the
same key. A Task(A,E) has a Key(Result(A,E)); finishing wraps its result, joining
unwraps it. Scope(E) keeps a Key(E) and a list of task ids, not List(Task(A,E))
at different A. Shared(A) uses Key(A) in the same way. Event leaves carry pure
adapter functions which accept Keyed and unwrap their own key after selection.
Mismatched keys stop as an internal protocol fault; they never fabricate A.

The scheduler table holds only Keyed, Int-indexed records, and continuations at
one fixed protocol type. Generic operations create typed decoders at their
call sites, where A/E/N are known to inference. Erasure does not generalize a
key made by `key.new()`: that call is not a direct value (§4.7). `both` erases its
two completions separately and decodes each with its own key. No service field
needs `forall A`, and no tuple projection recovers an existential type.

Each switch returns a Turn, whose arms contain a command/continuation or an
erased terminal result. The start wrapper catches all body errors into its
known completion key before they cross the protocol boundary. control.keep_env
removes N from the start function's external needs; its task boundary consequently
has one fixed result/error/environment type. A suspended task's protocol return
is StepAnswer, with the loop's common Turn as the boundary result. All scope and
unwind delimiters are inside that boundary. This is a construction argument,
not a checked implementation; the first R2 implementation gate is to type-check
this two-result-type/channel prototype before building the full scheduler.

The i loop threads immutable State through tail recursion. It keeps a FIFO
ready queue, tables of scopes/channels/tasks/shares, and ticket waiters. It
removes a continuation from State before resuming it once. Resuming can yield
another Turn; State is changed only after that call returns. No loop boundary
is captured in a task. The loop starts each task with a new control.boundary;
all tasks execute from the loop rather than from their parent task's stack.

Before keep_env, go replaces Scheduler, Runner and Await with portable detached
stand-ins. It then keeps every service, including user services it does not use.
Its one parameter is the new Scheduler, whose private fields supply that task's
Runner and Await. Inside the retained frame it binds all three real routing services. It keeps Root and Owner unchanged as normal
portable user services; a user request.within group at the go site must not be
overridden by the run's default group.
This extends S7's Scheduler substitution to the two routing services introduced
here: otherwise they would retain the parent's boundary and pin every child.
No user service is cleared or filtered. task.keep uses the same transformation
but takes the caller's Scheduler when invoked. The detached services stop with
`panic("task: no scheduler")` if used.

Use a non-tail call after those environment statements, through a surviving
identity frame when necessary. `r = body()  r` is only a source sketch; check
O0 and O2 WAT and the third resume. If optimization removes that frame, use a
separate clause-block call that survives optimization and test it. This is the
D16 workaround, not a claim that B2's tail-environment fix has landed. Apply it
to entry wrappers, nested run wrappers, scope-body delimiters and task.keep too,
where their environment statements would otherwise disappear before a suspend.

### 6.4 Events, FIFO behavior, and fairness

Channels are unbuffered rendezvous channels in v1. No close-channel operation is
needed: owner-scope exit frees their state. An event is a reusable immutable
description. choose([]) behaves as never(); sync(always(x)) gives x. New event constructions
have fresh identity keys, independent of A/E/N, so equality means the same
event handle rather than a comparison of event meanings. map/choose
perform no Scheduler operation. send_event checks portable payloads immediately;
receive_event and join_event do not test liveness until sync. A send_event can
also be built without Scheduler; its handle records whether it is local.

One sync creates one registration identity with ordered alternatives. Validate
run identities, join chains and channel liveness of all leaves before registering
any. An invalid losing alternative is still an invalid sync. Register without
running map callbacks. Among ready alternatives use source order; among partner
registrations use FIFO arrival order. Distinct leaves from the same registration
must never rendezvous with one another. A registration may commit once. Commit
atomically marks both partner registrations, removes all their losing leaves,
transfers one value, then queues the tasks. Apply only the winning branch's map
functions, in composition order, in the syncing task after commit. A mapper can
raise, await, spawn, or stop; it cannot roll back the message. No guard or nack
operation is exported. Cancelling a sync unregisters all leaves before unwinding.

go appends the child to the FIFO queue and returns to the parent in the same
turn, after cancellation checks and a host poll only if a task needs host
progress; it does not yield the parent. yield ends the current turn and queues the caller at the tail. A blocking command removes
its task until ready; a nonblocking sync/join still makes a scheduling point.
A completion queues waiters FIFO. Cancellation bookkeeping runs before another
normal task turn; a task already unwinding is queued normally. Scope ends,
async awaits, and each loop switch are poll points too. path/random are not.

At every switch, first run cancellation bookkeeping and scan terminal ticket
waiters. Build the progress-interest set from tasks actually suspended in
Await.wait on Waiting tickets, including interests forwarded by nested runs.
Root-level Runner.poll calls request.poll once only when this set is nonempty.
Otherwise skip it, including before the first task turn. Pure go/yield/join/
scope operations and merely starting an unawaited raw request do not by
themselves return status 2. A task/run which never blocks, and makes no explicit
root wait/poll call, therefore never yields to the host. A pure Scheduler-only
entry can return async status 0 immediately. A runnable pure task can of course
share its run with a sibling needing host progress: that sibling's wait causes
the run's hand-off. This is service of an existing wait, not an implicit yield
added to the pure task.

While there is progress interest and a task is runnable, poll once before its
next turn, using status 2. Scan settled tickets, then run a task even if further
host answers might be ready. This keeps an infinite yielding task from starving
an IO-waiting sibling, without making pure task switches pay an event-loop yield.
Unawaited raw work alone does not force progress hand-offs during a runnable
loop; its caller must eventually wait/poll explicitly or let the run become
idle. S7 rule 17 still counts it at idle as specified next.

A nested loop's private Runner.poll passes its current interest tickets to the
outer Scheduler with its nested-run id, then yields only its calling outer task.
The outer loop keeps that interest registration while the nested run lives and
forwards it in turn if nested again. Thus an inner IO waiter alongside an inner
CPU-yielding task can receive host answers even when the outer caller is runnable.
Terminal tickets are filtered; each poll replaces that run's prior interests,
and an error-free final removes the registration on every nested-run exit.
Cancellation removes a nested registration before completing the calling task's
unwind; borrowed tickets themselves are not cancelled by unregistering interest.
No new ABI operation is needed: these are private ordinary Scheduler commands.
The root Runner's poll takes the aggregated tickets; it never scans all instance
requests to invent a wait. Explicit low-level request.poll still intentionally
hands off, and stops request: no active entry from a synchronous entry.

When no task is runnable, scan terminal tickets first. Union
outstanding_root(the_run_root) with Waiting tickets its tasks explicitly await,
including borrowed tickets from another root. Run affiliation comes from every
group's registered root, not from the current Owner or just the default group.
If the union is nonempty, Runner.wait waits for one and then rescans. An
unawaited raw request in any custom group prevents deadlock until it settles.
Its detached Ready answer is retained only by its handle. If the set is empty,
stop with `panic("task: deadlock")`, even if an unrelated root has work. Nested
runs therefore neither count all instance requests nor miss their subgroups.
Never return status 1 with zero outstanding requests, or busy-poll when no task
can run. CPU code without suspension can monopolize M:1; a go-only loop may
postpone children. No bounded wall-time guarantee is made for final functions.

### 6.5 Runner, nested runs, and testing

The runtime supplies an initial portable Runner: prefix, full seed generator,
policy, the enclosing request-context Root, and constant functions implementing
root wait/poll. run uses that Root while opening its fresh run Root, so its
public environment still needs Runner rather than an extra Root argument. `top(seed)` replaces
only the seed in the current Runner and preserves its path allocator and wait
route. This is why its scheme needs Runner: rebinding a seed inside a task must
not reset its nested-run prefix or make its nested wait bypass the outer loop.
Entry provisioning uses the private ordinary root-runner constructor, not top.
The ledger supplies this initial service, so a fixed-seed test can write
`! task.Runner = task.top(random.from_seed(7))` without a bootstrap problem. In each task its Runner
routes wait/poll to the outer Scheduler and takes paths from that task's shared
spawn/run counter. That per-task Runner is non-portable; go's detached substitution
above prevents accidental retention in children. Explicit run inside the entry's
default root task is nested, as S7 requires. Each run opens a fresh request.Root and its default Owner around its entire
loop, in a retained frame. Tasks keep Root and their go-site Owner. Custom
within groups remain registered to that root. The run closes the entire root
after all tasks/finals end, not just its default group.
Only the runtime-created entry run
is eligible for future multicore execution; v1 has one worker throughout.

A nested loop owns fresh run identity and FIFO state on the caller task's stack.
Nested children do not execute an outer task's channel commands: all handle
uses check their run key. Its wait hands tickets to the outer task's Scheduler;
its poll yields that calling task without directly yielding to the host. Only that task is paused; other outer tasks
continue. When nested work ends it cancels remaining requests/children before
return. A nested deadlock is detected from that nested run's own tasks and
waitable requests. It does not return status 1 from a task stack. Chains do not
cross run identities.

`task.test(seed, choices, body)` supplies a Runner for a fresh nested run. Before
a turn, take the next script path, if any; it must be runnable, otherwise raise
`ScheduleError { NotRunnable(path: List(Int)) }` after structured cleanup. When
the script ends choose FIFO. Do not root-poll or use real clocks. Completed fake
requests are allowed; reaching a required wait on an unsettled host request stops
with `panic("task: test needs fake IO")`. Fixed inputs and script give the same
values, task paths, selected alternatives and trace on every run. Tests for real
host overlap use ABI completion gates instead. The testing policy changes only
which legal runnable step is chosen; it cannot commit an unready event.

## 7. IO public APIs

All five modules export the ordinary wrappers below, their service type with
its constructor, opaque resource types, and error types with constructors.
Each wrapper needs its module service, request.Await, and request.Owner, unless its row says
otherwise. All host failures expected from normal OS operations become the
listed typed errors; malformed host values and implementation failures remain
host faults under ABI v3. No IO primitive implicitly imports a std package.

Host services below use request mode. The tables give full source fields:
write them inside `@host("namespace", "request") Type { Type(...) }`. The request attribute adds request.Owner to each callable field's environment;
these tables give their written boundary declarations. Thus all fields return
Request, never lists, tuples, records or user error constructors
on the ABI. The host sees their unwrapped result/error ABI types from §5.2.

| Module/service/namespace | Public opaque types and errors |
|---|---|
| clock / Clock / `i:clock/v1` | `ClockError { ClockError(operation: String, code: String) }` |
| fs / Fs / `i:fs/v1` | `File {}`; `FsError { FsError(operation: String, path: String, code: String) }` |
| net / Net / `i:net/v1` | `Listener {}`; `Stream {}`; `NetError { NetError(operation: String, endpoint: String, code: String) }` |
| process / Process / `i:process/v1` | `Child {}`; `ProcessError { ProcessError(operation: String, code: String) }` |
| stdio / Stdio / `i:stdio/v1` | `StdioError { StdioError(operation: String, code: String) }` |

The common stable wire error codes are `invalid`, `not_found`, `denied`,
`exists`, `closed`, `busy`, `interrupted`, `unsupported`, `io`. Net additionally
uses `address_in_use`, `refused`, `reset`, `unreachable`; clock uses `range`;
process uses `not_utf8`. Each wrapper records the operation name shown below,
and the input path or endpoint (empty for resource-only operations). Codes are
String so a platform-specific future code is preserved, not misclassified or
silently turned into a stop. Human-readable OS messages are not the wire format.
There is no aggregate IoError that would erase the module's error identity.

### 7.1 Clock

Raw fields of Clock:

| Field | Type |
|---|---|
| `monotonic_ns` | `() -> Request(Result(Int,String))` |
| `unix_ns` | `() -> Request(Result(Int,String))` |
| `sleep_ns` | `(duration: Int) -> Request(Result(Nil,String))` |

Wrappers have those parameter/result types without Request/Result and with
`? ClockError ! Clock | request.Await | request.Owner`. monotonic_ns measures nanoseconds since
the instance's clock origin and never decreases; unix_ns counts signed
nanoseconds since 1970-01-01T00:00:00Z with platform clock adjustments visible.
sleep_ns requires a nonnegative duration; zero still performs an await/poll
point. Set the deadline in the host start closure, not on first future poll.
A duration outside the host's representable range gives code range. No promise
of precise wake time is made. There is no ambient clock in task.test; fakes use
request.ready. Calendar/time conversion is Std-2's job.

### 7.2 Files

```i
OpenMode { ReadOnly WriteTruncate Append ReadWrite }
```

| Raw Fs field | Type |
|---|---|
| `open` | `(path: String, mode: String) -> Request(Result(File,String))` |
| `read` | `(file: File, max_bytes: Int) -> Request(Result(BitArray,String))` |
| `write` | `(file: File, bytes: BitArray) -> Request(Result(Nil,String))` |
| `flush` | `(file: File) -> Request(Result(Nil,String))` |
| `close` | `(file: File) -> Request(Nil)` |
| `remove` | `(path: String) -> Request(Result(Nil,String))` |
| `rename` | `(from: String, to: String) -> Request(Result(Nil,String))` |
| `read_file` | `(path: String) -> Request(Result(BitArray,String))` |
| `write_file` | `(path: String, bytes: BitArray) -> Request(Result(Nil,String))` |

Public open takes OpenMode and encodes `read`, `truncate`, `append`, or
`read_write`; write modes create the file if absent, read_write does not
truncate. Public read/write/flush/remove/rename/read_file/write_file otherwise
use exactly these parameter and success types, with `? FsError`. Public close
has no errors and is idempotent. A read requires max_bytes > 0, returns up to
that many bytes, and returns `<<>>` only at EOF. All bytes parameters must have
a whole number of bytes; otherwise give invalid before an OS operation. write
writes all bytes or raises an error; an error/cancel may have written a prefix.
flush reports data-flush failures. close releases resources even when flush
would fail; it does not promise durability. read_file opens, reads to EOF and
closes; write_file truncates, writes and closes, and is not atomic replacement.
rename errors use the from path; no path normalization or sandboxing is hidden
in the i wrapper. Expected access denial is code denied.

### 7.3 TCP networking

| Raw Net field | Type |
|---|---|
| `listen` | `(address: String, port: Int) -> Request(Result(Listener,String))` |
| `local_port` | `(listener: Listener) -> Request(Result(Int,String))` |
| `accept` | `(listener: Listener) -> Request(Result(Stream,String))` |
| `connect` | `(address: String, port: Int) -> Request(Result(Stream,String))` |
| `read` | `(stream: Stream, max_bytes: Int) -> Request(Result(BitArray,String))` |
| `write` | `(stream: Stream, bytes: BitArray) -> Request(Result(Nil,String))` |
| `shutdown_write` | `(stream: Stream) -> Request(Result(Nil,String))` |
| `close_stream` | `(stream: Stream) -> Request(Nil)` |
| `close_listener` | `(listener: Listener) -> Request(Nil)` |

The same names are public wrappers with the same parameters and success types,
`? NetError`, and no errors for the two close functions. Address is a numeric
IPv4/IPv6 address in v1, not a URL or a DNS name. Ports are 0..65535 for listen
(0 asks the OS to choose) and 1..65535 for connect. local_port returns the bound
port. Empty address, bad addresses/ports, nonpositive read bounds and partial-byte
writes give invalid. read and write follow fs's EOF/all-bytes rules. TCP is a
byte stream: reads need not correspond to writes. shutdown_write sends EOF in
one direction and leaves reads usable; it is idempotent. It gives busy if a
write is pending. Later writes on the shut-down direction give closed. Close releases the
whole stream/listener, is idempotent, and settles other pending operations on
that object with closed unless their answer already committed. A concurrent
read and write are allowed. A second concurrent read, write, or accept on the
same direction/object gives busy, not an unspecified interleaving. There is no
TLS, UDP, HTTP, DNS, or cross-worker reference transfer in v1.

### 7.4 Processes and entropy

| Raw Process field | Type |
|---|---|
| `args` | `() -> Request(Result(BitArray,String))` |
| `env` | `(name: String) -> Request(Result(String,String))` |
| `cwd` | `() -> Request(Result(String,String))` |
| `entropy` | `(bytes: Int) -> Request(Result(BitArray,String))` |
| `spawn` | `(program: String, args: BitArray) -> Request(Result(Child,String))` |
| `wait` | `(child: Child) -> Request(Result(Int,String))` |
| `terminate` | `(child: Child) -> Request(Result(Nil,String))` |
| `close` | `(child: Child) -> Request(Nil)` |

Public args gives List(String); spawn takes `(program: String, args: List(String))`;
other wrappers keep raw parameters/success values, with `? ProcessError` except
close, which has no errors. Program args exclude the interpreter/program name;
spawn's args exclude argv[0]. Each child inherits cwd, environment, and standard
streams. There is no shell expansion, pipeline, or implicit command parsing.
Strings containing NUL give invalid; malformed UTF-8 OS args/env/cwd give
not_utf8. env of an absent name gives not_found. entropy accepts 0..2^24 bytes
and returns exactly that many bytes from the OS source, or an error. It does
not seed task.run automatically. Use entropy for secrets, not random.Generator.

Encode the argument list as u32 little-endian count, then for each string a u32
little-endian byte length and those UTF-8 bytes, with no padding/trailing bytes.
Lengths/count above 2^32-1 give invalid. Decode with checked arithmetic; bad
framing is a host fault for host-produced args and invalid for spawn input.
These are ordinary bit-array codecs in i/process, not new ABI types.

wait gives an exit status 0..255 (normal OS exit codes are reduced modulo 256);
termination by signal s gives 128+(s % 128) on Unix,
and platforms without Unix signals map forced termination to 255. Repeated
wait calls give the same cached status. A second simultaneous wait gives busy.
terminate requests termination, is idempotent for an ended child, and does not
wait; close terminates if still running and reaps, and is idempotent/error-free.
An expected OS error from terminate raises ProcessError; close waits for reaping, even if it must wait for natural exit after a
failed termination request. A raw OS process may be slow to terminate, so close/final
completion has no bounded duration. Pipes and changed cwd/env are later API
extensions, not unspecified behavior of this version.

### 7.5 Standard streams

| Raw Stdio field | Type |
|---|---|
| `read` | `(max_bytes: Int) -> Request(Result(BitArray,String))` |
| `write` | `(bytes: BitArray) -> Request(Result(Nil,String))` |
| `write_error` | `(bytes: BitArray) -> Request(Result(Nil,String))` |
| `flush` | `() -> Request(Result(Nil,String))` |
| `flush_error` | `() -> Request(Result(Nil,String))` |

Public wrappers unwrap those results, giving `? StdioError ! Stdio | request.Await | request.Owner`.
read's bound/EOF and write's whole-byte/all-bytes rules are those above. stdout
and stderr are distinct directions; one write per direction may be pending,
with busy for a competing write. A cancelled write may have written a prefix.
Flush functions are explicit. Also export `print(text: String)` and
`print_error(text: String)`, which UTF-8 encode text plus one LF, write all bytes,
and flush their stream; both can raise StdioError. No implicit text decoding,
line buffering or newline translation is added. Legacy i/io still prints a
String plus LF with the old no-error contract under `i:io/v1`.

## 8. Host implementation, targets, and resource ownership

### 8.1 Standalone Rust services

`i run`/`i test` register the new namespaces themselves in standard HostImports.
The CLI user writes no Rust, JS, import map, or WASI wrapper. Preserve the
existing services::standard(print) hook and legacy stdout behavior; add the new
service modules behind it, with explicit testing imports able to override the
standard defaults before instantiation. Add production tokio net/io/fs/process
support as needed, without changing the one-thread-per-run executor contract.
Service ownership of request futures remains in requests.rs, not tokio::spawn.

Split `services.rs` into registration plus `services/{clock,fs,net,process,stdio}.rs`.
Clock captures deadlines in start closures. Net uses reactor readiness; never
hold a synchronous mutex across an await. Resource cells contain a typed OS
object, closed flag, operation leases and a close wakeup. A close takes and
releases the resource and wakes pending operations. If close races completion,
whichever commits first determines the result. Request cancellation drops its
future and registration immediately. OS side effects already committed may
remain; no error or cancellation promises rollback.

Filesystem blocking work must own a duplicate OS handle and its private input/
output buffers. Cancelling an outstanding operation marks that File closed and releases the language
resource; detached work may finish using its duplicate but cannot be followed
by another read/write on the same File with an unknowable cursor. New operations
on that alias return closed. Close does not wait for detached work or acquire a
mutex it holds. Unrelated files, network requests, and the run may go on.
Path-only operations may finish after cancellation; document that rename/remove/
write_file are not rolled back. Avoid detached work that owns a resource another
live i operation must reacquire. This follows C D6 and services.rs:23–36 rather
than pretending that dropping tokio::fs cancels a syscall.

Stdin needs one broker per host input source: the default OS stdin broker is
shared across instances, with an instance-owned cancelable subscriber. A
per-instance detached reader could consume bytes intended for a later instance.
One dedicated reader owns private buffers; cancellation removes the subscriber,
and bytes received later remain for the next subscriber. No second blocking OS
read starts while one is pending. The reader never holds a lock needed by the
program while blocked. Instance destruction unsubscribes without dropping a
pending source read or retaining a Store. Bound buffered data. For sockets/pipes
with reactor readiness, use it instead. On host-source destruction an OS read
which cannot be interrupted may remain on a detached thread with private data;
ordinary default-stdin instance turnover does not create more reader threads.
Stdout/stderr writes own copied buffers and an operation lease; if detached, the
lease remains busy until that write ends, so the next write cannot interleave
with its remaining prefix. Cancellation may leave a written prefix.

Child ownership is independent of its wait future. A cancelled wait removes its
wait lease; Child.close requests termination and reaping independently. Cleanup
must not need a mutex locked by that cancelled future. Spawn failures are typed
errors. A provisional child needs a host-owned supervisor that can receive a
terminate request and reap without its calling Store or run reactor staying
alive. Dropping an undelivered reply sends that termination request. A native
worker that supervises an abandoned child may finish independently, like private
blocking work permitted by C D6; service futures themselves are not tokio-spawned
tasks. Explicit close awaits the supervisor's reaping completion, including when
termination cannot be delivered and natural exit must be awaited. Merely dropping
a default tokio Child does not implement this contract. Abandoning host work
never promises that the OS has already completed reaping when the instance
returns status 0. Verify this supervisor in the process phase before claiming
that cancelled spawn leaves no orphan. Host shutdown does not wait for detached
blocking work, as C requires.

For open/listen/accept/connect/spawn the host reply owns a new resource until
it is decoded into a request cell. Dropping host work or an undelivered reply
releases that provisional resource. After decoding, the i wrapper owns the
unclaimed Ready result. Its error-free final inspects request.state: for
Ready(Ok(resource)), close it using the appropriate service, then cancel the
ticket; for Waiting, cancel; for Taken/Cancelled/Error, just cancel. request.await
calls take with no scheduler step between its wait returning and the transfer. A cancelled
scheduler await jumps to the final before taking, so that final closes an already
decoded resource. On success take removes the cell's answer and marks Taken;
the final does not close the resource just handed to the caller. The caller
then uses resource.acquire/ensure to own its lifetime.

The core of that guard is ordinary i (Request is a built-in type; status constructors are
qualified here; no new syntax or polymorphic service field is needed):

```i
await_resource(A, E, N) = (
  r: Request(Result(A, E))
  release: (A) -> Nil ! N
) -> A ? E ! N | request.Await {
  resource.ensure(
    () { fail.from_result(request.await(r)) }
    () {
      request.state(r) |> {
        request.Ready(Ok(value)) -> release(value)
        _ -> Nil
      }
      request.cancel(request.ticket(r))
    }
  )
}
```

This guard is ordinary i, parameterized by the module's close function and its
kept service environment. It may suspend while closing. It adds no ABI ack or
resource-specific compiler rule. Raw service users must install the same guard
if they start resource-creating requests manually: request.cancel alone drops
host work, not arbitrary i-owned resources. On a stop, i finals do not run, but
destroying the instance drops its host resources. On GC of a forgotten File or
Stream, host ownership may also release it; GC timing is not the specified way
to close a resource. Expected close failures are swallowed only by the explicit
error-free close contract; flush/terminate expose errors separately.

Resource objects are host references, which remain local as S7 requires. On Rust
this can use the existing Arc-backed HostReference (`crates/ilang-host/src/lib.rs:372–382`).
The request table's generation token solves stale i aliases, not a malicious
host's late callback after id reuse: ABI H13 already forbids that callback.

### 8.2 Target behavior, operation by operation family

| Module | `i run` Rust host | JS SDK standard service factory: Node | JS factory: browser | WASI preview-1 target in this track |
|---|---|---|---|---|
| random | Pure i | Same pure i | Same pure i | Same pure i |
| task | i FIFO loop over v3 | Same loop; promises, AbortSignal, macrotask polling | Same loop on calling thread; never block it | Same i loop; raw host calls complete synchronously; no status hand-off |
| clock | Reactor timers and OS clocks | Monotonic/system clocks and abortable timers | Performance/system clocks and abortable timers | Ordinary blocking service imports; preview-1 command supplies no i:clock adapter |
| fs | OS files, with the cancellation rule above | Node file operations, closed/busy state, explicit close | Every raw operation gives unsupported, except close gives Nil | Ordinary blocking imports; opaque File needs an export-aware embedder, not plain preload |
| net | Reactor TCP | Node TCP, cancellation destroys provisional sockets and removes listeners | All operations give unsupported, close gives Nil; browser fetch is not raw TCP | Ordinary blocking imports; no default listen/connect adapter |
| process | OS args/env/cwd/entropy/child processes | Node process data, entropy, spawn/wait/terminate/close | args gives an encoded empty list; env gives denied; cwd/spawn/wait/terminate give unsupported; entropy uses platform secure entropy; close gives Nil | Ordinary blocking imports; no default process/entropy adapter or spawn implementation |
| stdio | Per-source input broker/subscribers and byte output | Node standard byte streams with cancelable subscribers | Default byte read/write/flush operations give unsupported; an embedder can give explicit byte sources/sinks | Ordinary blocking imports; no new i:stdio adapter; legacy i/io continues to use fd_write |

The JS factory is an explicit SDK option, not a language name-resolution rule.
It installs all fields including unsupported stubs, so a module can instantiate
and catch a typed unsupported error. Embedders can override any whole service
with ordinary imports. Browser APIs never silently treat URLs as files or a
WebSocket as Stream. Node and browser async starters read requestSignal()
synchronously and register cancellation there; the helper and status-2 macrotask
path already exist (`bindings/javascript/ilang.mjs:34–58`, `:894–902`). New service
factories and tests are IO work, not claims about today's SDK.

WASI keeps the current §B.7 blocking-import contract. It is not the `i run` host:
no standalone promise for the new IO imports is made here. Each request-mode
blocking import returns an already-Ready request, so neither ABI cancel nor
resume/continue is used, and poll is a no-op. Ordinary channel tasks still run
cooperatively, but a blocking TCP read/accept supplied by a custom WASI embedder
blocks the worker and cannot meet the concurrent-server acceptance criterion.
`i build --target wasi` must clearly report required non-WASI service imports;
it must not claim that plain wasmtime run supplies them. Preserve general custom
blocking imports, as §B.7 already permits. Implementing a preview-1 adapter set
is a separate target project, not an unreviewed second IO host in this track.
The current hostref representation requires an externref and reference_id
(`crates/ilang-wasm/src/runtime/mod.rs:240`, `:249–268`), so “just return an fd”
would not implement File under the existing ABI. That unsupported shortcut is
specifically rejected.

## 9. End-to-end acceptance: concurrent TCP echo

This complete example needs only i modules. It deliberately uses local_scope:
accepted sockets are host references. Several connections overlap on one worker;
this example does not promise multicore speedup for socket-owning handlers.

```i
(i/task, i/net, i/fail, i/resource)

copy = (stream) {
  net.read(stream, 65536) |> {
    <<>> -> Nil
    bytes -> {
      net.write(stream, bytes)
      copy(stream)
    }
  }
}

serve_next = (scope, listener) {
  resource.acquire(
    () { net.accept(listener) }
    (stream) { net.close_stream(stream) }
    (stream) {
      task.go(scope, () { serve_next(scope, listener) })
      fail.catch(() { copy(stream) })
      Nil
    }
  )
}

@main
main = () {
  resource.acquire(
    () { net.listen("127.0.0.1", 8080) }
    (listener) { net.close_listener(listener) }
    (listener) {
      task.local_scope((scope) {
        task.go(scope, () { serve_next(scope, listener) })
        Nil
      })
    }
  )
}
```

Each accepting task acquires its own socket and installs its final before
starting the next accepting task. Thus there is one pending accept while older
tasks serve their own sockets; an unstarted child never owns an accepted socket.
The handler's catch isolates expected read/write errors to that connection;
listener errors still fail main and cancel all handlers. A production bounded
server can use a local channel of permits, but a limit is not needed to show
concurrency. The example's co-recursive types must infer from ordinary module
functions; there are no constructors imported implicitly from net/task.

Acceptance fixture: use port 0 and local_port in the harness variant. Connect
clients A and B. Send one byte from A but leave A open without EOF; send a
multi-chunk payload containing zero bytes from B. Require B to receive its exact
payload while A remains open. Then require A's byte echoed. Half-close B and
check its handler ends and closes its stream; connect C while A is still open.
Finally a finite harness injects a failure from a sentinel child in the
accepting scope, catches that error outside the scope, and checks that listener,
A/C handlers, and all started host requests end. This distinguishes overlap
from an implementation that accepts one client and reads it to EOF first.
No sleep-based assertion decides readiness. Run the fixture 200 times at O0/O2
and once through the JS Node factory; Rust is the standalone acceptance host.

## 10. Replacement spec text, section by section

Apply after A1+A2 and the S7 replacement text, without renumbering any existing
section. The following replacements are exact editing instructions. API tables
are normative text, not pseudocode. Copy their signatures with Markdown's pipe
escaping, which renders as i's ordinary union operator.

**Opening list and §0 terminology.** Replace the opening effects/library bullet
with “effects and the library modules defined in §9 and §13”. Replace the whole
paragraph beginning “Every compiler must give the six library modules” with:

> Every compiler gives the library modules `i/fail`, `i/control`, `i/resource`,
> `i/assertion`, `i/io`, `i/core`, `i/key`, `i/request`, `i/random`, `i/task`,
> `i/clock`, `i/fs`, `i/net`, `i/process`, and `i/stdio`. They are ordinary modules;
> only operations specified as primitives need a compiler implementation. The
> host services those modules declare are given by the host as §9.7 says. No
> path in this list is a reserved empty module. Other library modules, CLI
> commands, and test result presentation are not part of the language.

Add to §0 after “Error”:

> A request is a handle to work whose answer may arrive later (§9.11). A task
> is a computation scheduled within a structured run (§9.9). An outstanding
> host request is work not yet answered or dropped (Appendix B); a request
> with a decoded answer is no longer outstanding.

In §0 “Not fixed”, add:

> Order-dependent task steps and observations of host services need not give
> the same result in repeated runs (§9.9). A fixed pure input and a fixed test
> schedule do give the same result.

**§2.1.** Replace only the `i` package bullet:

> - `i`, for the library modules listed at the start of this document (§9,
>   §13), which every compiler gives. Their primitives may be built into the
>   compiler and are not host imports (§10.1); their declared host services
>   are host imports (§9.7).

**§4.2.** After the S7 sentence about Key/Keyed, add:

> Request(A) and Ticket are built-in types, with operations in i/request (§9.11). Request(A) contains request handles whose answer belongs
> to A. Ticket is an answer-erased handle. Neither has public constructors.

**§4.5 rule 7.** Add “Request(A) goes in the same direction as A.” No rule for
Task/Scope/Channel/Event/Shared is added: their directions follow their ordinary
custom definitions under rule 6, as specified by the API in §9.9.

**§9.2.** Retain the S7 keep_env addition. Add:

> Await, Root and Owner (§9.11), Scheduler and Runner (§9.9) are normal service values.
> Replacing a host service in i uses an environment statement as for any
> service. Replacing a random seed with task.top keeps the run route of the
> current Runner. Entry defaults do not change call-site service lookup.

**§9.4.** Apply S7 §13's keep_env row and additions to rules 3, 4 and 7 exactly.
Add after them:

> A structured task cancel resumes its scheduling continuation with a cancel
> answer; i code then cancels an inner continuation (§9.9). It does not change
> control.cancel's rule that the boundary of the cancelled continuation is
> not active during unwinding.

**§9.5.** Add:

> A structured scope waits for cancelled children and their final functions
> before it returns (§9.9). Those final functions may suspend and resume and
> use services, but may not raise errors. An unstarted child has no installed
> final functions. A host failure or a destroyed instance cannot run i final
> functions; the host instead releases its own work and resources (Appendix B).

**§9.7 full replacement.**

> A type definition with `@host("namespace")`,
> `@host("namespace", "async")`, or `@host("namespace", "request")` declares a
> host service. It has exactly one constructor and labeled function fields.
> Two host services may not have the same namespace and field label.
>
> In the synchronous form, every field's parameters, value and errors must be
> types the host can take (§10.3), with no environment. Its call completes
> during the import.
>
> In the async form, the written field type is `(P...) -> A ? E`, with no
> written environment. Before inference, its callable type becomes
> `(P...) -> A ? E ! request.Await | request.Owner`. Parameters, A and E must be types the host
> can take. A call starts a request, awaits it through the Await service, and
> returns its value or raises its error. The call therefore waits at its call
> site; other tasks in the run may proceed meanwhile. Interface display shows
> both service requirements. No particular scheduler is built into this rule.
>
> In request form, every field has type `(P...) -> Request(T)` with no written errors
> or written environment. Before inference, its callable type becomes
> `(P...) -> Request(T) ! request.Owner`; interface display shows that effect.
> T is an ABI result type, or Result(A,E) for an ABI result type A and ABI error
> type E. The ABI signature is the corresponding async
> signature, without Request/Result. Its call returns the request handle
> immediately. Only this outer Request and optional Result have this rule;
> Request is not an ABI value type.
>
> A host service required at an entry point is given by the host. i code can
> replace it with an ordinary value, including completed fake requests. Each
> field is imported under the namespace and its label (Appendix B). A service
> behaves the same whichever worker calls it; requests and host references
> stay on their worker (§11.9).
>
> The standard service of i/io remains
> `@host("i:io/v1") Io { Io(print: (text: String) -> Nil) }`. print writes the
> text plus LF and gives up a failed write. i run and i test provide it;
> the WASI target implements it as §B.7 says. i/stdio is a separate byte-IO
> service, i:stdio/v1. The clock, files, TCP, process and stdio APIs are given
> in §9.7.1–§9.7.5. i run and i test implement all of them. Other hosts
> provide the imports or cannot instantiate the program; supported hosts may
> supply typed unsupported errors for unavailable capabilities.

**New §9.7.1–§9.7.5.** Copy the service/type/error definitions, stable error-code
paragraphs, raw field tables, public wrapper type transformations, wire codecs,
and behavior in §7.1–§7.5 of this note, in that order. Prepend §7's shared
introductory API/error paragraphs and service/type table to §9.7.1. Include §8.1's
resource-transfer/cancellation paragraphs from “For open/listen/accept/connect/spawn”
through “specified way to close a resource”. They define the ownership visible
to programs; Rust implementation choices elsewhere in §8.1 do not enter the spec.

**§9.8.** Use S7's Keys text unchanged. R2 supplies its implementations.

**New §9.9.** Copy the public API/type text in §6.1, R1–R6 including R4a in §6.2, and the event,
FIFO, fairness, nested-run and test-scheduler rules in §6.4–§6.5. Do not copy the
implementation argument of §6.3. Prepend:

> i/task is a library implemented using i/control, i/key and i/request. A run
> owns its tasks and scheduler state. Its implicit root permits local values.
> Scopes constrain the lifetime of spawned work: none outlives its scope.
> Values pass between tasks only through go captures/kept services, channels,
> child results/errors and shares. Portable paths check core.portable even
> with one worker: sends at the call, send events at construction, children
> at completion, scope body errors at failure, shares at creation. Failure
> stops with panic("task: value is not portable"). go itself does not check;
> unstarted tasks with non-portable captures stay on their home worker.
> Local scopes/channels hold the maker's boundary only as a non-portability
> witness. A Task/join event from a local scope holds that scope too. Every
> handle checks run identity. Channels belong to the first scope in their
> maker's chain, or the run; channels and shares are freed at scope exit.
> Every run has the result of some serial order of task steps. Timing may
> choose a sender, a race/select winner, which failures precede cancellation,
> or whether a handle is used before its owner ends. These order-dependent
> results may change between runs or worker counts. Tests must either avoid
> that dependence or list the allowed outcomes. No task API observes workers.

**New §9.10.** Copy §4's API, numerical contract, tagged split-tree adapter,
validation, rejection sampling and float rules. Retain the published SplitMix
variant/version and reference, but omit the review commentary about replacing
the old draft and statistical acceptance. Include the seed-modulo-W rule and
state that distinct streams are not guaranteed to have disjoint outputs.

**New §9.11.** Copy §5.1's public definitions, tables, state transitions,
cancellation/wait/alias rules, and dynamic-context rule, including Owner groups. Copy §5.4's implicit
adapter ownership rule, with the consuming take rule and reference to §9.7's
resource guards. Exclude the discussion of compiler frame/table layout.

**§10.2 full replacement.**

> The program entry is the @main function of the compiled package. It has no
> ordinary parameters and its value is ignored. More than one is a compile
> error; dependencies' @main attributes do not add entries. Tests are @test
> functions in test/ of the compiled package; each has no ordinary parameters
> and returns Nil. A test passes by returning Nil and fails on a stop. Each
> runs in its own freshly started instance. Host exports are the functions
> named in the compiled package's wasm.exports; they must be exported by
> their module and take/return only the types of §10.3, including their errors.
>
> Entry environment parameters and needs may contain host services, Assert,
> request.Await, request.Root, request.Owner, task.Scheduler, and task.Runner.
> No other unsupplied service is allowed. The runtime binds only the defaults
> requested by the ordinary inferred environment. Inferred Scheduler or Runner
> needs request the default structured FIFO run, fresh identity/path [], seed
> random.from_seed(0), and its internal request context. Async/request host
> needs without task needs get a direct request context/Await handler, without
> a task scheduler. Await/Root/Owner-only needs may use completed fakes in a
> synchronous entry. These resolved standard-service entry defaults are the
> one narrow library provision; another library gets no implicit bootstrap.
>
> Entry wrapping does not change a function's principal type. The default
> values can be replaced by ordinary environment statements. task.top(seed)
> changes a Runner's seed while retaining its route and prefix. An unsupplied
> user State service remains an entry error. Main/test errors stop after
> cleanup; host-export errors reach the host after cleanup. Stops run no finals.
>
> On the ilang target, let N be the entry's inferred environment: the entry is
> async exactly when N contains an async or request-mode host service, or
> task.Scheduler or task.Runner. Otherwise it is a synchronous export, including
> a pure comparator usable by a synchronous host callback. No body-reachability
> analysis or universal async wrapping is used. Even an async entry may finish
> with status 0 on its first call. Scheduler progress polling yields to the host only for actual host waiters,
> including nested waiters; idle waits also count unawaited work. Pure task
> switches alone do not yield there.
> On WASI, all entry calls and host imports remain synchronous (§B.7).

**§10.3.** Keep the existing boundary value/error lists, and append:

> The outer Request/Result types in a request-mode host field describe how
> host work is obtained (§9.7); they do not add ABI parameter/result types.
> Requests, tickets, tasks, scopes, channels and events cannot cross the ABI.

**§11.1, §11.4, §11.7–§11.9, §13.** Apply S7's existing replacements, including
portable, keys, cycles, and identity. Add requests/tickets to §11.1's value list.
Add to §11.4: “A request or ticket compares by its underlying request identity,
which does not change on answer, take or cancel. Ticket identity is not a
fresh value on each call of ticket.” They are different type
constructors, so a Request and Ticket are not equal to one another. Add to
§11.8: Request displays `//request` and Ticket displays `//ticket`, without
ids, state or answers. For task handles, keep Int ids under Keyed fields (text
`//keyed`), so inspect cannot expose bookkeeping ids. Add all panic reasons
spelled in §§5–6 as library panic cases, not new catchable errors. Keep Request
and Ticket non-portable in §11.9, including completed/fake handles.

**§12 module list.** The final paragraph becomes:

> These are normal functions and types. The modules i/fail (§9.3), i/control
> (§9.4), i/resource (§9.5), i/assertion (§9.6), i/io (§9.7), i/clock, i/fs,
> i/net, i/process, i/stdio (§9.7.1–§9.7.5), i/key (§9.8), i/task (§9.9),
> i/random (§9.10), i/request (§9.11), and i/core (§13) give the other names.

Add this row to §12's built-in name table, beside the S7 Key/Keyed row:

> | Request Ticket | Built-in types operated on by i/request | §9.11 |

Await, Root, Owner, Scheduler, Runner and IO types require imports, using qualified
names when wanted; the entry rule refers to their resolved definitions, not spellings.

**§B.4–B.6.** Keep ABI v3 and all H rules. In §B.5 replace only “a module made by
this compiler has no outstanding request at that point” with “structured entry
cleanup normally leaves no outstanding request; a deliberately unowned raw
request is dropped by the host at status 0”. Request-mode and async declarations
produce the same v3 async signature; metadata lists the unwrapped signature and
derives each entry's async flag from its inferred environment as §10.2 says. A completed cell is not an outstanding id.

**§B.7.** Keep the existing target rules, and append:

> The new standard service namespaces i:clock/v1, i:fs/v1, i:net/v1,
> i:process/v1 and i:stdio/v1 remain ordinary blocking imports on this target.
> This version supplies no preview-1 adapters for them. A required adapter is
> not supplied by plain wasmtime run; builds report these required imports.
> An async adapter awaits its already-completed blocking result. A request
> adapter returns a completed request value. The task loop's root poll is a
> no-op; no request uses a resume export or ABI cancel. i/random is pure and
> i/task can interleave computations at its explicit scheduling points, but
> a blocking host operation blocks that worker. Legacy i/io is unchanged.

### 10.1 Coordinated edits to the accepted multicore note

Only runtime.md is edited in this lane. When integrating these coordinator-
approved rules, apply the following exact replacements in multicore.md too.

In §18 rule 8, old sentence:

> No join is ever left waiting for a dropped or cancelled task.

New sentence:

> At a valid join or selected join event for a dropped or cancelled task, an
> applicable scope cancellation first unwinds a joiner it can still cancel;
> a joiner shielded from that cancellation stops with
> `panic("task: join of a cancelled task")` immediately once the target is
> dropped or its cancellation is accepted, without waiting for its unwind,
> so no join remains waiting for a cancelled outcome without a result.

In that same rule, old sentence:

> A `go` on a cancelling scope makes a task that is dropped at once.

New sentence:

> A `go` on a cancelling scope makes an unstarted task that is dropped at
> once; a caller shielded from that scope's cancellation receives its
> `Task(A,E)` handle, while a caller still cancellable by it is unwound before
> it can observe the handle.

The supporting argument in §6 “Who may join” must agree. Its old sentence is:

> Every such joiner is cancelled when `s` is cancelled (below), so no join ever
> waits for a task that was dropped or cancelled.

Replace it with:

> A joiner still cancellable by `s` is unwound before it can observe a dropped
> or cancelled result; a joiner shielded from that cancellation, including a
> final function or its protected cleanup work, stops with
> `panic: task: join of a cancelled task` immediately once the target is
> dropped or its cancellation is accepted.

The unchanged public join type needs no Cancelled case. This is smaller than
adding a typed cancellation result to every join and combinator; a stop is
already allowed for a function with no corresponding error type (§11.7).
Do not stop an ordinary joiner merely because cancellation delivery lost a race
with the target's drop. R4a's applicable-cancellation check is part of processing
the command, including when a new scope body inside a final is cancellable.

Rule 17's unawaited-request/deadlock guarantee is unchanged. “Of the run” counts
all Owner groups registered to its Root, not one default cancellation group or
the whole instance. A nested run establishes another Root. The §5 counter-based
PRF implementation sketch is replaced by the published SplitMix split-tree
choice in §4 of this note, following the coordinator's preference; the tagged
P/S stream promise of §18 rule 11 remains. R1 must reconcile that sketch when
landing its algorithm text rather than leave two algorithm prescriptions.

## 11. Files, ownership, merge order, and ordered implementation checklists

Only this note is changed in the design lane. The following is an implementation
assignment, not permission for a worker to edit outside its lane. Line references
anchor current regions; implementations re-find them after earlier tracks land.

| Track | Files and regions to change |
|---|---|
| R1 | Replace `lib/i/random.i:1`; register no Rust primitive. Add R1 spec text and `tests/spec/9.10/` |
| R2 prerequisites from S7 | New `lib/i/key.i`; `lib/i/control.i:3–28` keep_env; `lib/i/core.i:3–33` portable; primitive type/variance/name tables in `crates/ilang-check/src/types/solve.rs:1285–1291` and `hir.rs:210–257`; closure flags/walk stamps and portable walk per S7 §3.6; keep_env re-entry per S7 §4 in emit/control.rs after B |
| R2 request layer | New `lib/i/request.i`; primitive Request/Ticket and operations in checker/runtime/IR; host declaration validation and expansion in `crates/ilang-check/src/resolve.rs:664–679`, `:1470–1555`, mode in `hir.rs:182–186`, and post-expansion interface display after syntax/T work; `crates/ilang-wasm/src/lower.rs:969–1054`, `:1140–1202`; replace the entire request/table/result region in `emit/host.rs:23–165`, `:172–246`, `:335–358`; add continue export. No edits to `ilang-host/src/requests.rs` are needed for many outstanding requests |
| R2 scheduler and entry | Replace `lib/i/task.i:1`; `infer.rs:376–429` resolved allowed-entry-service set and diagnostics; `lower.rs:828–938` entry wrappers/metadata; `emit/mod.rs:805–870` service setup and wrapper invocation. The wrapper calls ordinary i implementation definitions; it does not implement the queue in Rust |
| IO service phases | Replace the five reserved `lib/i/{clock,fs,net,process,stdio}.i`; new `crates/ilang-host/src/services/{clock,fs,net,process,stdio}.rs`; registration in `services.rs:44–70`; tokio features in `crates/ilang-host/Cargo.toml:15`; CLI/default host imports at `crates/ilang-cli/src/lib.rs:151–152`; new JS Node/browser service factories and their tests under `bindings/javascript/` and `tests/browser/` |
| Shared registration | `crates/ilang-package/src/library.rs:7–26` adds request/key once; existing seven reserved entries become real without duplicate paths. Add Request/Ticket to compiler built-in type names, like Boundary/Continuation; their ordinary helper types are not prelude names. S7 Key/Keyed retains its assignment |
| Spec/test ownership | R1 §9.10; R2 §9.2, §9.4–9.5, §9.8–9.9, §9.11, §10.2 and request additions in §4/§10.3/§11/§B.5; IO §9.7, §B.7, §0/§2.1/§12 lists. Use the exact section splits in §10; one owner reconciles each shared paragraph |

The key/request modules, checker host-form expansion, runtime primitive types,
JS factories and target-import diagnostic are explicit extensions to the plan's
short file list. Assign them to R2/IO as above before implementation begins;
none is quietly charged to C. S7 already requires the key/keep_env/portable work.
Request-mode host fields and Owner/Await expansion add checker changes to R2;
those are a real prerequisite, not something i/task alone can express today.
Generated host-field references add ordinary dependency edges to i/request;
entry wrappers add i/task dependencies only when N requests task services;
request-only wrappers depend on i/request. Add these edges before ordering/
reachability, using inferred service needs rather than scanning function bodies.
Assign the corresponding edge construction in the checker/driver to R2 (after
T2's ordering changes), so dead-code elimination or module ordering cannot drop
the runtime definitions merely because user source did not import i/task.

Merge constraints:

1. K1 has landed on redesign at 578a17d; require it before R2 and preserve its
   display.rs order K1 -> A2 Namer -> T1 -> T3. Do not work around the invariant
   generalization fault by weakening key invariance or a public type scheme.
   A1+A2 provides new syntax. B's implementation precedes R2 and its keep_env
   re-entry changes. C implementation is already the base for this note.
2. R1 is independent of the new request primitives. Land it early on lane b
   after B's retained work, for Std-3/property; it must not import Std-3.
3. Keep the ledger's hot-file order: lower.rs named regions C -> T1 -> R2 ->
   T3 -> STR -> M2; infer.rs and type tables T1 -> T2 -> T3 -> R2 -> M2 -> M4.
   Reconcile those by splitting R2's lower and infer changes into separate
   commits; the overall R2 merge follows T3. No worker overwrites T3's new
   lower regions. emit/host.rs, host ABI areas and §10.3 follow
   C -> T1 (where assigned) -> R2 -> IO -> M2.
4. IO can prepare Rust services after C and draft i wrappers after A1+A2,
   but its request-returning i modules merge after R2's request layer. Lane c
   keeps clock -> fs -> net -> process -> stdio serial. JS factories are part
   of the matching phase. Final registration/list edits happen once, in stdio.
5. R2 and IO then gate E2E-echo. Std-2 adds real time.now after clock lands.
   M2 waits for the completed R2 behavior; compiler sandbox runs use a fixed
   test Runner and no host services. MC keeps Root/Owner keys portable and gives
   every worker the run's live-root/group registrations; closing a group coordinates
   its requests across workers. No host request itself becomes portable.

Ordered R1 checklist:

1. [ ] Commit §9.10 spec text; resolve the stream-derivation quality question.
2. [ ] Implement the pinned published SplitMix variant and tagged split-tree
   navigation, with immutable Generator and explicit 64-bit seed normalization.
3. [ ] Implement draw/split/path and arbitrary-bound rejection; no task dependency.
4. [ ] Independently check published SplitMix vectors, prefix-code traversal,
   weak-gamma cases and rejection tests; retain the statistical review gate.
5. [ ] Run spec tests and the appropriate i/Rust verification through heavy,
   at O0/O2; compare replay across targets; release to Std-3.

Ordered R2 checklist:

1. [ ] Commit R2 replacement spec text; merge S7 primitive text, preserving §9.8.
2. [ ] Verify K1's invariant-generalization repair, then implement keys/variance/
   value restriction, portable including cycles, and
   keep_env with retained environment frames; run all S7 single-worker tests.
3. [ ] Implement Request/Ticket cells, Root/group registries and table replacement together with
   per-cell answer decoding, raw/async host adapters, cancel and continue.
4. [ ] Prove the monomorphic keyed prototype by type-checking two channel element
   types and heterogeneous child results. Reject any need for a cast or forall field.
5. [ ] Implement the i loop, per-task boundaries, rooted Owner groups, Runner/top,
   task.keep and inferred-needs entry provisioning. Preserve synchronous exports
   and suppress host progress polls when no task waits for IO. Include nested
   progress-interest forwarding and guaranteed registration cleanup. Check O0/O2 WAT for retained frames.
6. [ ] Add scope chains/local witnesses, joins, body cancellation, shielded finals,
   run-key checks, channel/share lifetime and error arbitration; add R4a
   cancellation-priority joins and shielded dropped/cancelled-join stops.
7. [ ] Add CML registration/commit/map, combinators and deterministic script policy.
8. [ ] Integrate every async user host service through Await; require review
   tests J/E/O, nested subgroup and borrowed-ticket cases, root poll fairness,
   and synchronous callback/first-turn no-yield tests. Audit the host/control seam.
9. [ ] Run deadlock/cancel/fairness/alias/IO stress cases 200 times on Rust and JS,
   related spec tests and full required verification; inspect WAT changes.

Ordered IO checklist (each phase writes its i module and Rust service, compiles
because lib/i is included, and adds matching target/JS cases):

1. [ ] Clock: deadlines fixed at start, both clocks, zero/negative/huge durations,
   abortable timers and no timer keeping an ended Node run alive.
2. [ ] Fs: codecs/errors, whole-byte validation, all-byte writes, explicit flush,
   close, cancelled blocking-file state, safe detached operations.
3. [ ] Net: numeric endpoints, ephemeral bind, read/write/half-close, busy leases,
   cancellation/provisional socket ownership and concurrent connection fixture.
4. [ ] Process: bounded entropy, UTF-8/framing, no shell expansion, inherited
   streams, wait caching, terminate and cleanup/reaping on every ownership edge.
5. [ ] Stdio: cancellable subscriber input, exact byte output, flush/error stream,
   legacy i/io compatibility; register all services and finish spec lists/B.7.
6. [ ] Run permission/error/resource/security and concurrency reviews; include
   unsupported browser/preview-1 behavior and user-supplied host fakes.
7. [ ] Run end-to-end echo, then combined verification in the integration lane.

## 12. Tests, counterexamples, and verification limits

Every expected value below follows the named rule; implementation tests should
assert that rule, not snapshot private layouts. New-syntax examples below are
small function bodies with imports of the named modules unless stated otherwise.
The acceptance server in §9 is a complete module.

### 12.1 Rule-based tests

| Test | Expected result and rule |
|---|---|
| Re-evaluate word on the same Generator, and split twice without a draw | Equal (word,next) pairs and equal pairs of generators; D1/§4 immutable data |
| int(g,0), int(g,-1), at_path(g,[0]) | InvalidBound(0), InvalidBound(-1), InvalidPath([0]); §4 validation, no draw |
| int(g,1) | 0 and one word consumed; §4 requires at least one word even for bound 1 |
| Stub words for n=10: W-1, then 7 | Reject W-1, give 7 with two draws; §4 rejection rule |
| Split initial root vs child path [1] | Their histories contain S(1) vs P(1), not equal generators; §4 domain tags (not an assertion that words must differ) |
| `task.random() == task.random()` in one task | True; D1 and S7 rule 11; no draw counter in Scheduler |
| Published SplitMix seed 1337, first three words | 0xb5c19e300e8b07b3, 0xd600e0e216c0ac76, 0xc54efc3b3cc5af29; §4.1 pinned source vectors |
| Request ready(7); state, take, state, take | Ready(7), 7, Taken, panic request: taken; §5.1 one transfer |
| Take Int and String request handles stored together only as tickets | Read each original typed handle correctly; Ticket has no answer operation; D2 |
| Cancel an old cell, reuse its slot, cancel the old alias again | New request remains Waiting and receives its own answer; D4 cell identity/generation |
| Two same-import requests answered in reverse order | Each original typed handle returns its own value; §5.3 per-cell decoding |
| Wait on A, host answers B first | B remains Ready, wait on A continues; §5.1 requested-set semantics |
| Cancel after decode but before an IO await resumes | One close of the provisional socket/file, zero user-body uses; §8.1 Ready/Taken guard |
| Local socket result vs portable scope child returning that socket | Local join works; portable result stops task: value is not portable; D7 |
| Portable send_event of a continuation in a choose with always(1) | Stops at event construction, even though it would lose; S7 rule 2 |
| `sync(choose([send_event(c,1), receive_event(c)]))` with no other task | Deadlock; §6.4 same registration cannot rendezvous with itself |
| Two ready alternatives with side-effecting map callbacks | First alternative commits; exactly its callback runs in the syncing task; §6.4 |
| `task.scope((s) { task.go(s, () { 1 })  2 })` | 2, only after child completion; R3; body result is independent of child's result |
| `both(() { 1 }, () { "x" })`; map_concurrently([3,1], id) | (1,"x") and [3,1]; §6.1 typed separate handles/argument order |
| Failed child while body joins an unstarted sibling | Error of failed child, body final ran, no deadlock; R2/R4 |
| Two failures already observed before cancel, paths [1,1] and [1] | Error at [1]; R2 prefix/path order, not first observed error |
| go A(yield; raise A), go B(raise B), body waits | B on production FIFO; §6.4 go does not yield parent, A yields behind B |
| `task.scope((s) { s })` then go on returned s | task: go outside its scope; R1 |
| Join from outer-scope sibling, or channel from another run | task: join outside its scope; task: handle of another run; R1 |
| Channel returned from a finished scope then receive | task: channel is gone; R3; also test an outer waiter present at scope end |
| Share returned then get after scope exit | task: shared value is gone; R3 |
| Cancelled final yields, awaits fake clock, opens scope and rendezvous | Final completes, then original scope error; never boundary is not active; R4 |
| Parent fake Clock used after child's third resume and in its final | Fake result survives; keep_env plus retained frame, D16 workaround |
| Nested wait_flag while an outer sibling sets the flag | Both complete, no global hang; §6.5 Runner wait delegates to outer task |
| Rebind Runner with top(seed) inside a task, then nested run | New seed, preserved nested prefix/outer wait route; §6.5 contextual top |
| Nested raw request in request.within, with no task waiters; tasks block on channel | Wait for the request in that run's Root, then deadlock; §6.4 all-group root enumeration |
| Infinite yield task plus an outstanding accept | Conditional status-2 polls for the waiting accept task deliver completion and its handler gets a turn; §6.4 |
| Pure task path/yield/join operations with no host requests | No host poll, status 2, or macrotask yield; task entry can return status 0 immediately; D5/§6.4 |
| test with script selecting nonexistent path [99] | NotRunnable([99]) of type ScheduleError after cleanup; §6.5 |
| test with real pending timer rather than a fake | task: test needs fake IO; D10; fixed seed alone does not fix host time |
| Net read(0); fs write with partial byte; process NUL argument | Typed invalid errors with the named operation; §7 validation |
| Stream write of N bytes followed by peer EOF | Peer receives the exact N-byte prefix before EOF; §7.3 write-all/half-close |
| Two concurrent stream reads; read concurrent with write | busy for second read; opposite directions work; §7.3 leases |
| Close File/Stream/Child twice | Nil both times, underlying release once; §7/§8 idempotence |
| Process args codec for ["a","é"] | Bytes `<<2,0,0,0,1,0,0,0,97,2,0,0,0,195,169>>`; §7.4 UTF-8 framing |
| Missing env variable; browser raw TCP | ProcessError(...,"not_found"); NetError(...,"unsupported"); §7 and target matrix |
| WASI pure tasks/random; WASI module requiring Fs | Pure calculations work; report ordinary blocking imports requiring an embedder; §B.7, no fake fd-as-externref |
| @main needs user State with no binding vs standard Scheduler/Runner/Await/Root/Owner | State is an entry compile error; standard needs are provisioned without changing the type; async classification follows D5, not mere use of any request helper |
| Existing async user service, not an IO namespace, used in two children | Overlap via Await, same request table as IO; D3, no namespace rule |

Preserve all S7 single-worker tests T1–T12/T20/T21/T24–T27 as assigned there,
especially the closure cycle walk, invariant key error, Syntax/local variants,
callback environment and inactive-boundary cases. Do not silently change their
expectations to fit a scheduler queue policy. IO integration adds request gates,
OS resource counters and a child-reaping probe. Use a bounded wall-time watchdog
only to detect a hang; never make timing thresholds the semantic oracle.

### 12.2 Attack round 1: real defects fixed

**Typed heterogeneity.** Attempted program:

```i
bad_methods = (scope) {
  a = scope.go(() { 1 })
  b = scope.go(() { "x" })
  (a, b)
}
```

A fixed Scope field cannot have two independently instantiated result types.
Likewise a scheduler cannot recover an Int and String from an untyped universal
queue with a type test. Fixed by module-level go plus invariant typed keys and
erased decoder functions (§6.1/§6.3); the prototype gate must actually type-check.
This is an inference/buildability attack, not a rejected syntax spelling alone.

**Decoded-resource cancellation.** Program shape (Net is a raw request service):

```i
leak = (listener, n ! net.Net) {
  r = n.accept(listener)
  request.await(r)
}
```

Let the host decode Ready(Ok(stream)), then cancel this task before it resumes.
A plain cancel would discard the only i owner without installing close. Fixed
by taken/ready distinction and the ordinary error-free resource guard in §8.1.
The program above is deliberately low-level and must install that guard; the
public net.accept wrapper always does. No extra host ack is smuggled into ABI v3.

**Unstarted resource hand-off.** The first echo draft used:

```i
unsafe_handoff = (scope, listener) {
  stream = net.accept(listener)
  task.go(scope, () {
    resource.ensure(() { copy(stream) }, () { net.close_stream(stream) })
  })
}
```

If the scope cancels before the child starts, that child's ensure was never
installed. S7 says unstarted tasks have no finals. Fixed the acceptance server:
each handler accepts/acquires its own stream, then starts the next acceptor.
The new queued task captures no newly accepted stream. This is simpler than a
new ownership-transfer primitive or a special “final of an unstarted task”.

**Nested-run routing under a fixed seed.** A pure top(seed) that manufactures a
new root Runner makes this program incorrectly reset paths and wait globally:

```i
seeded_nested = () {
  ! task.Runner = task.top(random.from_seed(7))
  task.run(() { clock.sleep_ns(1) })
}
```

Fixed top's type to require the current Runner and replace only its seed. The
runtime supplies the initial Runner, as the ledger now requires. This retains
one run service and makes rebinding a seed safe inside any task. The exact
signature is an explicit coordinator confirmation item below because S7's prose
predated runtime-provided Runner.

**D16 environment loss and hidden pinning.** A tail call after binding a real
Scheduler loses that statement on a resume. Keeping the parent's Await or
Runner would separately retain its boundary, pinning all children. Fixed by
retained frames, portable detached routing stand-ins, and the three-service
parameter rebinding. Owner remains ordinary portable data and must be kept,
not replaced: a custom request.within at the spawn site still owns its requests.
Test the second and third scheduler calls, not just initial entry.

**FIFO/spec conflict.** Making go yield its parent made the S7 example with
A(yield; raise A) and B(raise B) give A. Fixed go to start the child in the queue
without ending the parent's turn; cancellation/host polls still occur. The
required FIFO example now gives B.

**Raw request deadlock and nesting (initial assessment).** Ignoring unawaited requests would declare
deadlock while host work was outstanding, against S7 rule 17. Enumerating the
whole instance would make a nested run wait for unrelated outer work, against
rule 5. The first draft tried to fix this with Owner groups and per-run
enumeration, but missed custom groups as the later review showed. The corrected
Root affiliation and all-group enumeration are in §5.1/§6.4. The group capability is portable; request cells remain non-portable. This is
also useful to ordinary non-task batch/timeout libraries.

### 12.3 Attack round 2: adversarial programs after those fixes

**Erased type confusion and reuse.** Try reading two different request types
through List(Ticket), or cancel an Int alias after its ABI slot is reused by a
String request. Ticket exposes no typed read; original typed cells/generation
tokens prevent aliasing a new occupant. Test take/cancel/answer in every valid
order. A response with the wrong ABI id/import remains a host fault; the design
does not promise to tolerate a host violating H13 after id reuse.

**Self-select and losing effects.** `sync(choose([send_event(c,1),
receive_event(c)]))` has no partner, so must deadlock. A non-portable losing send
still stops at construction. Losing map callbacks do not execute. The one
registration/one commit rule and eager payload check cover all three attacks.

**Cancellation after rendezvous.** Cancel a sender just after its event commits
and before its map callback. The receiver keeps the committed value. The sender
unwinds; no callback is promised for a task cancelled before running it. This
is a legal serial order, not message rollback or a double send.

**Wrong scope and final re-entry.** Send a scope to an outer sibling, join a local
Task from a different chain, or cancel a final that has just opened a scope.
Run/chain checks reject the first two before waiting. Shielded unwind prevents
a second cancellation; a new final scope still completes or fails under its
own rules. A nonterminating final can legitimately prevent return.

**Ordinary host service bypass.** Replace the IO service with an unrelated
user-defined async host namespace and call it in siblings. The declaration's
Await/Owner expansion is namespace-independent, so it cannot fall back to a
root capture per task. Explicit root wait inside a task intentionally pauses
that run as §5 says; it still has only one root history.

**Outer/nested capability escape.** An outer Channel given to a nested run stops
with handle of another run. An outer raw request may deliberately be awaited
there: its Ticket is a request capability on the same worker, not a task/run
handle; the nested Runner forwards that wait to the outer loop. Closing the
nested Root must not cancel an outer-owned request. The union-of-own-and-awaited
wait set gives exactly that behavior.

**Source and boundary conflicts.** Returning Request from a host export still
fails §10.3. An arbitrary user service at main still fails §10.2. A stop still
skips finals. i/io still has its original namespace. WASI blocking imports still
have no ids/resumes. This pass also found three specification/buildability defects: go/keep and
portable combinator schemes initially overstated the principal environment by
retaining service requirements their wrappers supply; test initially omitted
the Runner it needs for nested routing; and a module-scoped primitive Request
would need an unexplained compiler-only type declaration. Fixed the schemes
using the existing environment difference, kept Runner in test, and made Request/
Ticket built-in type names like Boundary/Continuation. A labeled Ready(value: A)
constructor also conflicted with the positional Ready(Ok(x)) pattern: Ready
now has an unlabeled A field. The host check also caught the per-instance stdin
broker consuming a later instance's input: the default broker is now per source,
with per-instance subscribers. A child supervisor rather than a destructor
alone owns provisional reaping. This pass therefore does not count as clean.

### 12.4 Attack round 3: repeat with representation and target cases

Try a portable handle containing an Int task id and inspect it; that would expose
bookkeeping. Required representation puts ids under Keyed, whose text is opaque,
and only path() exposes the semantic path. A mapped Event's decoder may capture
a local Channel, but its type stores only the decoder function; portable walks
its captures, so the local boundary is still found. Variance does not hide it.

Try a recursive closure or a record that hides a request: S7's tag-directed walk
sees the request through captures/fields and terminates on recursive closures.
Try a Ready error containing a host reference through a portable child: the
completion check runs on errors as on values. Try killing an async close with
the same scope cancel: shielded unwind lets it complete. Try receiving a raw
Ready answer after its Owner exits: settled cells belong to their handles and
have left the group; the answer is not incorrectly cancelled by group cleanup.

Try implementing WASI File with an i32: it cannot satisfy externref/reference_id,
so this note does not require that adapter. Try browser TCP via fetch: unsupported
is a typed result, not a false TCP implementation. Try a Node timer or Rust
blocking stdin after entry cancellation: abortable timers and a detached broker
must not keep the run waiting for abandoned work. Those are explicit host gates,
not an inference that cancellation stops all OS activity.

This was an initial pass at aa20548. The independent review later found the
join, entry and subgroup defects despite this earlier clean assessment. The buildability gate and actual OS tests remain necessary;
reasoning cannot certify their implementation.

### 12.5 Attack round 4: final typing, ownership and entry pass

Concrete negative cases: pass an early-returning fake Await to await(ready(1))
and to await of a Waiting host request; the first can take 1, the second stops
request: wait returned early. Call take twice; the second stops request: taken.
Close an Owner, rebind that exact Owner, and start a host request; it stops
request: owner is gone before making an ABI import. Complete a request, close
its Owner, and take its still-owned Ready answer; it succeeds once. No GC or
slot reuse changes any of these outcomes.

For service inference, a go body needing only Scheduler needs no parent Await/
Runner, a body needing a custom Clock keeps that need, and a test nested under
a caller still needs Runner. The revised schemes express exactly these cases.
A public ordinary function needing Request(Int) needs no privileged module
import or type declaration. RequestState's unlabeled Ready pattern agrees with
spec §7.1. Owner's Key(Nil) is portable while a ticket returned by outstanding
is not. There is no static conversion that can read a Ticket as a Request(Int).

For the echo server, cancel before the initial acceptor starts (no accepted
socket exists), during accept (host drops provisional work), after decode but
before take (the wrapper closes Ready), and while serving (ensure closes the
stream and the scope cancels the next acceptor). A queued next acceptor owns
only the listener capability whose outer acquire outlives all children. This
removes the original unstarted-resource gap without weakening S7's cancel rule.
A stop is still allowed to skip i finals and relies on instance destruction
for host cleanup.

This was the second initial clean assessment at aa20548, superseded by the
independent review and the new attacks below. These are source/type/ownership arguments; the mandatory
implementation prototype and OS tests remain unverified.

### 12.6 Independent review: findings checked and required examples

I read the whole report at
`/home/agent/Programming/i/.rd/cache/gpt/report-review-runtime.md`, including its
retained decisions, implementation limits and echo assessment. Each finding
was checked separately before changing the rules:

1. **Dropped join in a shielded final: confirmed.** At aa20548, R2 dropped the
   child, R4 shielded its joining final, and no completion arm could produce
   its A/E. The control code only reactivates the scheduling boundary
   (`emit/control.rs:260–295`); it supplies no task completion. R4a now resolves
   that protocol hole and §10.1 gives the exact S7 sentence changes.
2. **Universal async and first-turn polling: confirmed.** The inferred env is
   already available to entry_point (`types/infer.rs:405–429`) and lower's
   entry_async (`lower.rs:916–938`). The JS call/callAsync guards explicitly
   separate synchronous and async signatures (`bindings/javascript/ilang.mjs:965–981`);
   status 2 takes a macrotask at `:894–902`. My old D5 and unconditional
   first-turn poll would therefore break synchronous callbacks. D5 and §6.4
   now preserve them and gate host polling on outstanding requests. The review
   executed the comparator baseline; I independently read those paths, but did
   not repeat that execution against a new implementation.
3. **Custom groups missed by idle accounting: confirmed.** The old within made
   a group absent from the default Owner's table; the old idle union counted
   neither it nor an unawaited request in it. Root is now separate from Owner,
   and the idle union enumerates all groups of the run. Progress-interest tickets
   affect polling only, never substitute for all-group deadlock accounting.
4. **Request-mode Owner effect omitted from replacement §9.7: confirmed.** The
   old §5.2 added Owner while the replacement paragraph did not. Both now give
   the callable type `(P...) -> Request(T) ! request.Owner` before inference;
   ordinary field-function copying/display therefore sees the same effect.

The report's three programs are required implementation tests, not merely
illustrations. Use these exact accepted-syntax forms and a host completion gate
where specified.

**Review test J: shielded final joins its dropped child.**

```i
(i/task, i/resource, i/fail)

probe = () {
  fail.catch(() {
    task.scope((s) {
      gate = task.channel()
      parked = task.channel()

      task.go(s, () {
        resource.ensure(
          () {
            task.send(gate, Nil)
            task.receive(parked)
            Nil
          }
          () {
            t = task.go(s, () { Nil })
            fail.catch(() { task.join(t) })
            Nil
          }
        )
      })

      task.receive(gate)
      fail.raise("boom")
    })
  })
}
```

Call probe from a test harness. The rendezvous installs the final before boom.
go in that shielded final returns a dropped Task(Nil,E), without running its
body; join stops with **panic: task: join of a cancelled task**. Neither
fail.catch catches that stop, and the result is not Error("boom") or deadlock.
This follows R2/R4a even though join's public type is unchanged. Run at O0/O2,
including scheduler orders that delay cancellation delivery to ordinary tasks.

**Review test E: a pure synchronous export remains a comparator.**

```i
(cmp)

cmp = (a: Float, b: Float) { a -. b }
```

Compile/export cmp through wasm.exports. Its inferred environment is Never;
metadata says async=false. The following supported JS use returns [1,2,3]
synchronously, with no task run, status 2 or macrotask yield:

```js
[3, 1, 2].sort((a, b) => guest.call("app/main.cmp", [a, b]));
```

Also test a pure main, a Scheduler-only entry doing path/yield/join with no host
requests, and a host async-only entry. Expected: pure main is synchronous;
Scheduler-only is async in metadata but completes with status 0 without a host
yield; host-only awaits its request without initializing the task scheduler.
An imported but unused task module must not change any classification. Replacing
an async service with completed fake functions removes that service from N;
Await/Owner/Root alone do not make such an entry async. These are D5/§6.4 tests.

**Review test O: unawaited request in a custom Owner.**

```i
(i/request, i/task)

@host("review:work", "request")
Work { Work(finish: () -> Request(Nil)) }

@main
main = (w ! Work) {
  request.within(() {
    r = w.finish()
    c: task.Channel(Nil) = task.channel()
    task.receive(c)
    request.take(r)
  })
}
```

The fixture holds finish pending beyond the next host poll and then completes
it on command. The run first waits for that request (status 1 once no task is
runnable), even though it is unawaited and belongs to a custom cancellation
group. After finish completes, it stops with **panic: task: deadlock** because
nothing can send on c. request.take(r) is unreachable. It must not stop before
finish completes. Repeat inside a nested task.run with an unrelated outer
request: the nested root counts every nested subgroup, but not the outer work.
The outer request cannot postpone a genuine nested deadlock. This is the
unchanged S7 rule 17 interpreted through Root affiliation.

The changed parts were attacked again. A candidate fix that waited for a
cancelled target's cleanup before stopping was rejected: two shielded finals
joining each other could wait forever. The chosen rule records cancellation
at acceptance and stops that join immediately; cleanup completion remains a
separate scope-exit condition. The following two passes were repeated with
that fix and the refined progress-interest polling rule.

### 12.7 New self-attack, pass 1: cancellation and nested ownership

Attack J with an ordinary, still-cancellable joiner instead of a final: let one
worker mark the target dropped before another worker delivers the scope cancel.
The join command must see the applicable cancellation and unwind its caller;
it never tests “target cancelled” first and stops. A shielded final can still
stop because it cannot be unwound by that cancel. Attack a final which opens a
fresh cleanup scope whose child fails while the final joins a sibling: the new
scope can cancel its body, so R4a unwinds that inner body before a join panic.
The older outer cancellation still cannot reach the new cleanup scope.

Attack select with always(1) before a cancelled join alternative: cancelled is a
ready completion, not an invalid handle; the first ready alternative wins and
gives 1. Reverse them in a shielded final and the selected join stops. In an
ordinary task cancellable by the target scope, scope cancellation wins before
selection, in either order. Attack go on a cancelling scope: no child body runs;
a shielded final receives a dropped handle, whereas a cancellable body unwinds
before observing it. No missing A/E remains and no synthetic error is introduced.

Attack O with two custom groups, then close only one. outstanding_root still
counts the other. Close the whole root and no group escapes its cancellation.
After a result becomes Ready, closing the group/root does not discard the
handle's answer. Rebind an outer Owner inside a nested run, then call within:
open_owner uses the nested Root, so the new subgroup cannot inherit the outer
Owner's affiliation. Deliberately starting under an already-made outer Owner
still uses that original group, whose tickets may be borrowed explicitly.
None of these cases uses instance-wide request count to decide nested deadlock.
Also make two shielded finals join one another after both cancellations are
accepted: each join is ready to stop, rather than waiting for the other unwind.
Keeping outcome separate from cleanup supplies that result without returning a
value early. No serious new rule conflict found in this pass.

### 12.8 New self-attack, pass 2: effects, polling and split addresses

Attack E through a wrapper `cmp2 = (a: Float, b: Float) { cmp(a,b) }` and through
an unrelated imported task module: N is still Never and both exports stay sync.
A higher-order helper called only with pure functions keeps a pure instantiated
environment; used with a task/async operation, its inferred environment carries
that need. No body scan is required. An explicit low-level poll from a sync
entry stops request: no active entry rather than silently changing its ABI.
Also call that synchronous export after an earlier async call ended: the stale
host-root pointer cannot authorize polling when async-live is zero.
A fake-only Await can complete synchronously; the default root handler checks
settled tickets before attempting any host hand-off.

Attack a pure Scheduler-only root on its first turn: its progress-interest set
is empty, so it returns status 0 without status 2. Even raw unawaited work does
not force host progress polls in a runnable task which never waits. Once the
run is idle, that same work prevents deadlock and becomes its explicit wait.
Attack a nested IO waiter alongside a CPU-yielding sibling: nested Runner.poll
forwards that waiter ticket, and the outer root services the interest while
still running other tasks. Cancel/exit the nested run while it borrowed an outer
ticket: remove its interest registration without cancelling the borrowed work.
Only actual live waiter registrations authorize progress polls; idle/deadlock
still counts every group in each Root. On WASI, blocking results are already
Ready and host polling remains a no-op.

Attack the SplitMix adapter with S(1), S(2), P(1), P(2), and mixed multi-step
paths. Their bit codes are 00, 01, 11, 1010; prefix-free concatenation gives
unambiguous typed addresses. Each edge selects a newly spawned published child,
so it does not merely advance the parent's continuation stream by two words.
Native odd gamma, 64-bit masking and the corrected low-transition gamma test
must match the pinned variant, including negative and oversized seed reduction.
These are functional/address checks, not statistical independence evidence.
No serious new rule conflict found in this second consecutive clean pass.

### 12.9 What was actually checked

Read the full independent report and checked the four old-text/source findings
as above. Read K1's first-parent patch at 578a17d without changing this lane's
base. Earlier checks at aa20548 covered a Threefry model; those vectors are
superseded and supply no verification of SplitMix. This revision's independent
Python models passed:

- The pinned source's first three seed-1337 words and seed-42 state, 64-bit
  masking, odd gamma, seed normalization, rejection boundary and float range.
- 1,026 prefix-free S/P codes, two-step address uniqueness and a 201-bit path
  index. Model word vectors for seed 0 are root 0x9474f0eb06d79fd8,
  S(1) 0x5223018439128278, P(1) 0xdbcefe01be55b171; these are model vectors,
  not i executions or statistical evidence.
- Inferred entry-service classification, pure/fake-only synchronous cases,
  task/host-only async cases, and no host poll for pure or raw-only runnable
  work. Nested waiter interests forward, update and unregister without
  cancelling borrowed requests.
- Cancellation-priority joins, immediate shielded cancelled-join stops,
  fresh cancellable cleanup bodies, dropped go handles and selected join events.
- All custom groups counted by Root, group-close isolation, nested exclusion,
  borrowed tickets and review O's wait-before-deadlock outcome; scoped subroot
  allocation and retirement at entry-context exit.
- Thirteen Markdown tables, balanced fences, accepted-syntax token checks,
  the exact old S7 sentence anchors, all three review programs, and the Owner
  effect in replacement §9.7. git diff --check passed.

The proposed i scheduler, request/Root primitives, revised source programs,
O0/O2 WAT, JS callback fixture, OS cleanup and real echo fixture remain unrun in
this design lane. The statistical gate remains unverified. Reading the existing
entry/host code establishes the baseline and needed edit regions, not that the
new implementation exists. No implementation or other note is changed here. No cargo or host integration
tests were run for this documentation-only revision.


## 13. Coordinator decisions and remaining gates

The coordinator has settled the five questions: contextual top is accepted;
universal async entries are rejected; blocking-import WASI is accepted;
independent statistical review is mandatory with a named published splittable
generator preferred; R2 owns requests/compiler work and IO owns JS factories/
target reporting. This revision chooses SplitMix as pinned in §4. K1 landed on
redesign at 578a17d. There is no outstanding permission request for these edits.

The implementation still must validate principal inference with the keyed
prototype, retained environments at O0/O2, cancellation-priority joins including
new cleanup scopes, rooted-group accounting, synchronous JS callbacks and real
IO overlap/cleanup. SplitMix's exact traversal and known weak-gamma cases still
need independent statistical review before R1 lands. Keep the existing risks:
long-lived scope storage, portability walks, cooperative fairness, unbounded
final duration, detached host cleanup and real unsupported-target stubs. A
published generator does not prove those gates or supply cryptographic security.
