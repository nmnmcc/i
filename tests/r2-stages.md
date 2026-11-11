# R2 stages

The lane began clean at 5ec3d12 and fast-forwarded redesign to f8ba920.
There was no interrupted uncommitted work.

## Sources and interpretation

- runtime.md §11: "Commit R2 replacement spec text; merge S7 primitive text,
  preserving §9.8." Stage 1 adds the assigned spec sections before implementation.
  IO owns §9.7 and the opening/module-list migration; its existing text is retained.
- runtime.md §11: "Clock -> fs -> net -> process -> stdio" belongs to IO.
  R2 therefore leaves i/clock to lane c.
- runtime.md §6.3: "the first R2 implementation gate is to type-check this
  two-result-type/channel prototype before building the full scheduler."
  Stage 2 establishes keys, portability and retained environments, then that gate.
- Coordinator ruling after the JP amendments: "Until JP lands, the compiler
  implements the special case in which the provider graph of an entry is exactly
  its inferred environment with the shipped defaults." The complete provider
  graph remains normative; its implementation is inherited by JP below.
- runtime.md §6.1: "A nested run takes an index from that same counter."
  Spawn and nested-run paths share the task counter.
- runtime.md §5.1: "A Ready cell has left its group and belongs to its handle."
  Group close preserves decoded answers; borrowed await does not cancel producers.

## Checkpoints

Each implementation stage requires the full dev-verify gate before its checkpoint
commit. The final audit also requires old-syntax-check. Generated expectations are checked separately
against their source rules and regenerated only in the final derived-files commit.

1. Spec migration: keys, retention, tasks, requests, entry classification and values.
2. Primitives and the heterogeneous monomorphic protocol prototype.
3. Request cells, rooted groups, observers, adapters and ABI v3 seam.
4. FIFO loop, scopes, events, combinators and the first task laws.
5. Nested cancellation, shielding and completion-gated host stress.
6. Shared conformance cases, full interface/WAT audits and measurements.
7. Regenerate derived files after each validated source checkpoint.

## Stage 1 validation

`ILANG_BROWSER_PORT=4183 /home/agent/Programming/i/.rd/cache/heavy direnv exec
/home/agent/Programming/i ./scripts/dev-verify` completed successfully: Rust fmt,
clippy and workspace tests; 1,422 std, 2 hello and 12 bench tests at each opt
level; WASI smoke; 46 Node tests and 33 browser tests. Log:
`target/r2-stage1-verify.log` (local, not committed).

The first syntax-check invocation was interrupted with SIGTERM (143) while
waiting for a slot. Its outer heavy wrapper was redundant: the script already
wraps its cargo build. The corrected invocation is `direnv exec
/home/agent/Programming/i ./scripts/old-syntax-check`; it passed: 1,106 extracted literals/blocks, exact expected diagnostics, 19 future-track entries.
No source or environment setting changed during that correction.

Documentation describes the accepted contract; passing existing tests alone
does not establish the new runtime implementation.


## Stage 2: primitives and typed protocol

Implementation:

- Key is invariant, fresh and identity-based. Keyed compares key plus payload.
  The private primitive read uses Result(A, Nil); ordinary i maps it to the
  public Lookup type. This keeps Lookup constructors ordinary library values.
- Portable walks values with an explicit stack. Patched closures reserve the
  final capture slot for a fresh walk token, as multicore.md §3.6 specifies.
  Record tag 14 is reserved; keys and keyed values use tags 15 and 16.
- keep_env stores every bound slot as ordinary tuple fields visible to the
  portable walk. Its delimiter/re-entry restores the kept base on each resume
  and cancel, before inner environment statements.
- The heterogeneous prototype uses Key(Result(A,E)) for results, Key(A) for
  channel payloads, and one Turn/StepAnswer continuation type. It type-checks
  and runs with Int/String children and channels without a cast or forall field.

Interpretation checks:

- keys.i and union.i: multicore.md D13, "unwrap gives the value only when x was
  made with the same key"; its §13 says keyed equality compares keys and values.
- invariant.i and value_restriction.i: D13, "Key(A) is invariant in A" and
  "key.new() is not a direct value". The independent key stays usable at Int.
- keep_env.i: multicore.md D4, "It keeps every service bound there" and §4,
  "the records inside it get the kept services and their own environment
  statements on top". Three resumes return the kept Name, cancellation finals
  see their kept Name, and an unused boundary-bearing service pins the closure.
- portable.i: multicore.md §3.1, "a name counts when it is written in the body,
  also in a part that never runs"; §3.6 requires cycle termination and an
  explicit stack. The million-element list and recursive closure stay portable;
  boundaries, captured boundaries and keyed boundaries do not.
- keyed_protocol.i: runtime.md §6.3, "the first R2 implementation gate is to
  type-check this two-result-type/channel prototype". The typed original
  handles decode their own Int and String results after interleaved resumes.

Focused results: 4/4 key cases, keep_env, portable and keyed_protocol pass at
O0 and O2. Final stage-2 full gate passes (see below).

Failed intermediate checks, all corrected:

- First Rust compile missed the host wrapper's call of the changed closure
  helper (E0061). Its unpatched captures now pass false explicitly.
- The first runnable key tests found an overbroad edit in inspect WAT, using
  a tag local outside inspect_atom. Restricted the new cases to inspect_atom;
  all four key tests then passed at both optimization levels.
- The prototype first used an invalid labeled constructor pattern (E0510),
  then invalid colon syntax (E0237/E0201). Field access with an explicit Start
  parameter annotation satisfies spec §6.7/§4.8's known-type requirement; the
  missing annotation was reported by E0506. The corrected prototype passes.

- The first full stage-2 gate failed in front_end because an unannotated
  unwrap inferred an empty answer case and warned in every program. Declaring
  its required public scheme `unwrap(A): (Key(A), Keyed) -> Lookup(A)` removes
  the warning; all five front_end checks and all focused key/prototype cases pass.
- The second full gate reached the library tests and rejected their old lists
  of paths/exports. Updated their explicit expectations from spec §9.4's
  keep_env row, §9.8's key API, and §13's portable row; checks stay exact.

- The third gate rejected the old library type golden. Regenerated it only with
  `UPDATE=1 cargo test -p ilang-check --test types --locked -- library`.
  Its new lines follow the key, keep_env and portable spec rows above; generation
  is not used as evidence for those contracts. A stage with changed derived
  files ends in a separate `Regenerate derived files` checkpoint commit.
- The fourth full gate passed (exit 0), including Rust, O0/O2 i, 46 Node and
  33 browser tests. An additional O0/O2 keep_env check verifies an inner Name
  environment statement survives three resumes through the retained identity
  call required by runtime.md §6.3. The public unwrap parameter label is x,
  matching spec §9.8; a named-argument call tests that label independently.


Stage-2 final gate after the public-label correction:

`ILANG_BROWSER_PORT=4183 /home/agent/Programming/i/.rd/cache/heavy direnv exec
/home/agent/Programming/i ./scripts/dev-verify` passed, exit 0, with 46 Node
and 33 browser tests (`target/r2-stage2-verify5.log`). Its preceding golden
update used the generation command above. `direnv exec
/home/agent/Programming/i ./scripts/old-syntax-check` passed: 1,106 extracted
blocks, exact diagnostics, 19 unchanged future-track entries. No R2 syntax
exception was needed. No environment setting was changed.

The source-based key/retention/portability/protocol checks establish the new
primitive contracts separately from the regenerated library interface golden.
The implementation and its generated interface form the two commits of this
checkpoint; the checkpoint head is the final regeneration commit.

## Stage 3: requests, groups and ABI adapters

The request layer now implements all public functions in runtime.md §5.1 and
§5.4. Root and Owner remain ordinary portable key-bearing i service values.
Request and Ticket are built-ins, with covariant Request answers. The private
slot table has a free list and a fresh generation identity per cell. Ready
cells leave the table and groups; scratch answer/start references are cleared.

A request-mode host field returns its cell immediately. Ordinary async fields
have Await | Owner added before inference, then call ordinary i await_owned
and from_result. Namespace spelling never selects a task or IO implementation.
The wire stays ABI v3. Request(Result(A,Never)) still receives Ok(A).

Group zero/change observers are local Request(Nil) cells with no host id and
no group membership. Check/subscription and each start/answer/cancel notification
are indivisible on this one worker. Notifications implement the private revision
observation without exporting a producer-ticket enumeration. Their progress
predicate queries the actual target's producer count, never the observation.

The entry wrapper calls ordinary i request.entry only from inferred request
needs. Completed fakes remain synchronous. Root wait/poll retain one host_k;
status and async-live are separate. Continue checks both. WASI returns completed
cells after its blocking imports and checks Owner liveness before the import.
Root poll on WASI is a no-op.

Interpretation checks (independent of generated expectations):

- ready/cancel_ready/take_twice/empty_wait/no_active_entry follow §5.1's exact
  state transitions and panic sentences. Identity is independent of state;
  Ticket aliases share their underlying request and cannot read an answer.
- fake_owned follows D5: "Await/Owner/Root alone ... do not turn an entry async"
  and §5.1's closed-group zero/readiness rules. Root/Owner are portable values.
- raw/raw_cancel/groups follow §5.1: "A Ready cell has left its group and
  belongs to its handle"; "open_owner uses that Root, not ... Owner"; and
  "cancel_all cancels the Waiting members ... not subsequent starts or detached
  Ready values". Tests cover independent roots, group-close isolation, zero
  observers, reusing slots and cancelling old aliases.
- ownership follows §5.4: "request.await itself remains a borrowed wait" and
  await_owned's ensure/cancel definition. Cancelling an ordinary control waiter
  leaves the borrowed producer Waiting and makes the owned producer Cancelled.
- result_never follows §5.3's explicit Ok(A) rule even for an empty error list.
- covariance, ticket_not_answer, request_boundary, request_host_shape and
  async_field_environment follow D2 and §5.2's exact type and boundary rules.
- Source traces follow spec §11.7 and §5.3: decode/validation must precede root
  resume. Each host cell saves the initiating call's full trace, including lazy
  frames, and validates against that trace before delivery. The old async
  host-error positions pass unchanged. This is implementation evidence for
  existing trace requirements, not a newly generated position oracle.
- Root suspension/re-entry stays in emit/control.rs. The host side still calls
  control only through host_boundary and host_resume_k. The request wait record
  is a runtime record that handles normal return only (kind 5), not a generated
  code record. The existing shape test now also checks that record explicitly.

Public interface: every public request function's inferred scheme matches the
§5.1 table (including within's Root | N - Owner) and §5.4's owning helper. The
new private intrinsic/entry helpers in library.out are not exported. No public
producer-ticket enumeration or new ABI export/import protocol was added.

Validation:

- `cargo test -p ilang-cli --test spec --locked`: 190/190 cases at O0/O2,
  including all 20 request cases and all 12 legacy host-service cases.
- `cargo test -p ilang-cli --test wasi --locked`: 17/17, blocking async behavior
  preserved.
- `cargo test -p ilang-wasm --test control_lazy --locked`: initially 16/17;
  corrected runtime record classification passes the focused record_kinds
  check and the subsequent full gate.
- Full heavy-wrapped `ILANG_BROWSER_PORT=4183 ./scripts/dev-verify`: exit 0
  (`target/r2-stage3-verify2.log`), including 46 Node and 33 browser tests using
  the regenerated compiled fixture.
- `scripts/old-syntax-check`: 1,127 extracted blocks, exact diagnostics, 19
  unchanged future-track entries; exit 0.
- Generated files came only from `UPDATE=1 cargo test -p ilang-check --test
  types --locked` and `./scripts/build-abi-fixture.sh`, through heavy. Their
  checkpoint commit follows the implementation commit.

Failed intermediate checks, corrected:

- Initial request cases rejected open_root/open_owner's explicit return types
  without their required !Root annotation (spec §6.8: omitted ! means Never).
  A direct Request/Ticket comparison also correctly triggered E0520; the
  identity-law case uses generic equality, preserving §6.9's disjoint-type rule.
- Legacy host tests found the explicit Later-only continuation annotation and
  missing async validation trace. The annotation now includes the service
  expansion required by D3; the original trace expectation was preserved.
- A runtime wait record initially used the code-record kind/name prefix. The
  shape check caught its missing code slot. It now uses runtime kind 5, and
  the check covers it without loosening existing assertions.
- The first full gate rejected the environment layout assertion: Clock's
  private slot moved from 1 to 4 because Await, Root and Owner are now registered
  before it. The representation check remains exact at 4. This numeric layout
  check is not evidence for a language rule; spec environment execution cases
  independently check the lookup behavior.
- Preliminary standalone WAT shape checks caught an extra parenthesis and a
  non-defaultable local crossing branches. An early substitute used the eager
  1df2fe0 frame signature with lazy re-entry code; switching the substitute to
  the current generated module fixed that mismatch. The real compile/spec and
  host/control seam tests above supersede those preliminary checks.

The lane still needs the i scheduler, full scope/cancellation/event laws,
Rust/JS stress fixtures, WAT audit, performance measurements and final merge.
The scratch scheduler draft in target/r2-task.i is not an implementation claim.

## Upstream merge after stage 3

The required redesign merge now includes 50c44a9 (the JS/WASI source amendments),
P1 opaque nominal identity, FP field privacy, and the pure Std-2 modules. The
lane keeps its request implementation. It preserves P1's per-declaration result
decoder: an outstanding request records its initiating nominal result identity,
and answer delivery decodes using that identity before resuming the root. The
new opaque_request_answers_keep_the_declared_type test delivers B before A,
then Alias; spec §4.1/§11.4 require A and Alias to compare equal for the same
foreign reference, and B to differ. This is a behavioral check, not a new golden.
The existing generated browser fixture is retained during conflict resolution;
its final replacement is generated by scripts/build-abi-fixture.sh.

### Entry-contract conflict

The explicit R2 scope says: "Entries are async exactly as runtime.md states
(the provider-closure refinement of js.md is NOT landed; do not implement it)."
At the starting accepted runtime revision f8ba920, D5 said:

> On the ilang target the entry is async exactly when N contains an
> async/request host service or Scheduler or Runner from i/task.

The newly merged spec §10.2 instead says:

> On targets using the v3 asynchronous entry protocol, an entry is **async**
> exactly when its complete chosen provider graph (§2.6.1), including both
> scopes and adapter requirements, contains a request-mode service provider,
> a scheduler role or a runner role.

The amended runtime D5 agrees with the new spec and requires rejecting
unresolved supplies and provider cycles. The new ledger assigns the plan/checker
and CLI implementation to a separate JP track and serializes wrapper/link work
after R2. Applying that contract here would implement the refinement expressly
excluded by the assigned scope; silently retaining the old contract would fail
the new normative spec. The merged spec keeps the newer accepted text; the
existing implementation is not claimed to satisfy provider plans.

A small distinguishing package has this i.toml:

```toml
name = "app"
[wasm]
exports = ["app/main.probe"]
[providers."app/main.Bundle"]
host = "app/main.make"
```

and src/main.i:

```i
// future-track: JP -- provider-closure semantics; currently rejected by R2.
(Bundle, probe)
Bundle { Bundle }
@host("probe", "request")
Work { Work(start: () -> Request(Int)) }
make = (work ! Work) { Bundle }
probe = (bundle ! Bundle) { Nil }
```

The entry's ordinary inferred environment contains only Bundle. Under the
original entry-service whitelist it is rejected; there is no qualifying host or
task service in that environment. Under spec §2.6.1 and amended D5, the chosen
Bundle factory depends on Work, whose request-mode host registration makes the
entry async even though probe never calls start. No task scheduler, IO namespace,
field-use analysis or uncaught operation error is needed for this distinction.
This is a source-contract counterexample, not an execution-test expectation.

The task scheduler draft remains only in ignored target files. Entry-dependent
work awaits resolution of the incompatible scope and newly amended sources.

The distinguishing package was also checked with the stage-3 release binary
(`direnv exec ... target/release/i check --project target/r2-entry-conflict`).
It reports E0522 at probe: Bundle must be supplied before the entry. This is
implementation evidence of the old whitelist, not proof of either interpretation
or a claim that provider-plan conformance passes.

The first merge syntax check mistook a new test's Rust formatting literal
`ilang:host/v1={}` for an i declaration. The test now formats its import module
as a separate argument, as the existing P1 test does, producing the same host
preload argument. The scanner and its exception list were not changed.

Merge validation completed:

- Heavy-wrapped `cargo test -p ilang-wasm --test codegen --locked`: 10/10,
  including the new out-of-order opaque request behavior and the existing
  synchronous P1 checks on both targets.
- Heavy-wrapped `ILANG_BROWSER_PORT=4183 ./scripts/dev-verify`: exit 0
  (`target/r2-merge-verify.log`). Rust fmt, clippy and workspace tests pass;
  the spec harness reports 195/195 at O0/O2. Each i optimization level passes
  1,446 std, 2 hello and 12 bench tests. WASI smoke passes. Node passes 47/47
  and the three browser engines pass 36/36.
- `direnv exec ... ./scripts/old-syntax-check`: exit 0 on the corrected test
  literal (`target/r2-merge-syntax2.log`): 1,221 extracted blocks, exact expected
  diagnostics, and 19 unchanged future-track entries.
- `scripts/build-abi-fixture.sh` generated the replacement compiled.wasm and
  validated all fixtures. No other generated file changed. The binary is kept
  out of the merge commit and follows in the final Regenerate derived files
  commit. The merged request public schemes still match runtime §5.1/§5.4;
  P1 changes their representation, not those schemes.

No permanent environment setting was changed. Shared heavy slots and another
lane's quiet measurement delayed commands; their locks were not modified.
The source-contract example remains rejected under the original entry rule;
provider-plan conformance is not claimed. The task module is still empty.
All task-interface rows, structured-lifetime/event laws, scripted determinism,
200-run Rust/JS stress, scheduler/channel costs and complete WAT-difference audit
remain to be implemented or measured after the entry-contract scope is resolved.

## Coordinator resolution: R2 entry specialization

The coordinator keeps the original R2 entry scope. Until JP, the provider graph
is exactly the inferred service set with shipped defaults. The compiler's
services_require_async function takes that set; JP can pass its chosen closure
instead. Provider and adapter tables now report E0522, "not implemented yet",
rather than being ignored. providers_pending is marked future-track JP and
expects that temporary rejection. The amended normative text is retained.

JP inherits these sentences, currently not implemented by R2:

- Spec §2.6.1: "A target provider plan maps resolved service type identities to
  ordinary i factory functions or host registrations."
- Spec §2.6.1: "For each entry, supply every service left in its inferred
  environment and all dependencies of the chosen factories."
- Spec §2.6.1: "Missing required services and dependency cycles are compile
  errors naming the target, service and requiring entry."
- Spec §2.6.1's schema, outer/body graphs, adapter guards, seed and entry-root-key
  arguments, dependency ordering and wasm.synchronous contract.
- Spec §10.2 and runtime D5: classification from the complete chosen provider
  graph, including factory and structured-adapter requirements.
- Spec §2.6's source-only/target checking and target/command provider selection.

The Bundle counterexample above is a future JP semantic example. It parses
already, so adding a failing parser exception to old-syntax-check would be
incorrect. Its executable R2 test instead carries the future-track JP marker
and checks the explicit temporary manifest diagnostic. No check is weakened.

Merged BG resource contexts and binding grants at cb17adb. Existing host request
starts already receive their invocation context; group cancellation uses the
ABI cancel path that revokes each request's host lifetime/lease. The guest keeps
its per-entry Root/Owner registry inside that host invocation. Entry retirement
closes all remaining roots; host invocation teardown remains the final owner.

R2 entry-specialization checkpoint: the full heavy-wrapped gate with port 4183
passed, exit 0 (target/r2-entry-ruling-verify.log): 196 spec cases at O0/O2,
Rust checks, i tests, WASI smoke, 76 Node tests and 51 browser tests. The new
manifest unit test checks both root and dependency manifests. Its TOML literals
are recorded as non-i host data in the existing syntax-check inventory; the
provider_pending i program itself is parsed normally. The generated BG binding
plan follows in a separate Regenerate derived files commit.
The entry-specialization syntax gate also passed: 1,226 extracted blocks, exact
expected diagnostics and 19 future parser entries. The JP semantic marker does
not add a parser exception, because that example already parses.

## Stage 4: i scheduler, task interfaces and first laws

The loop, scopes, task starts, cancellation delimiters, channel/event registration,
shares, combinators, path allocation and request routing are now ordinary i code
in i/task. The compiler wraps only entries whose inferred needs request those
services, through ordinary i task.entry inside request.entry. It stores no
scheduler queue in Rust. Nested runs use the caller's allocator and wait route.
This checkpoint covers the first laws; the later cancellation/nesting and host
stress stages still have independent obligations.

Source-based interpretation checks:

- tasks/nested_paths: D10 and §6.1 say nested runs take an index from the same
  counter as go. Scope bodies run in the opener. Two initial children, two in
  both, three in map_concurrently and two in race make the following run index
  10. This expectation is counted from those source rules, not generated output.
- lifetimes follows R3's wait-for-every-child requirement. channel_fifo and
  events follow §6.4's source-order alternatives, FIFO partners, one commit,
  fresh event identity and winner-only map execution.
- cancel_cleanup follows R2/R4: wait for finals and their fresh cleanup scopes;
  inherited cancellation cannot cancel that cleanup work.
- join_cancelled_final is review test S/R4a: go on a cancelling scope returns a
  dropped handle to the shielded final, whose join must stop immediately.
- kept_services follows §6.3 and S7 D4: keep all services, replace only routing,
  and preserve services through the third resume. A portable task handle does
  not retain the child's go captures; its scope is its portability witness.
  task.keep inside that child independently exposes retention of the unused
  Marker service, after detaching routing.
- local_values, body_value_local and the nonportable cases follow D7's exact
  crossing points. A scope body's successful result need not be portable.
  Public Task/Scope/Channel/Shared invariance and Event covariance follow §6.1.
- async_tasks follows D3/§5.4 for an ordinary non-IO async namespace. It also
  checks that a status-2 polling handoff accepts a host answer.
- scripted/test_needs_fake follow §6.5: absolute script paths, FIFO fallback,
  structured NotRunnable cleanup, and no real-IO polling under test.
- self_select/never follow §6.4's no-self-rendezvous and quiescence rule.
  Gone/another-run/outside-scope cases follow R1/R3 and prevalidation of losing
  alternatives. Shared handles remain portable for local scopes (§6.1).

Corrected intermediate failures:

- The draft type checker rejected six selectors through an alias of a matched
  Runner. Binding the subject before the match preserves its known narrowed
  constructor; no Assert requirement or checker rule was weakened.
- An initial scratch counter assertion incorrectly treated a nested run as
  fresh. The source count above requires index 10; O0/O2 then passed.
- The first 14 spec cases exposed one implementation error: children created
  by a shielded final in an already-cancelling scope inherited an exemption
  and ran. Spawn now records them dropped immediately, independently of that
  exemption. The R4a stop now passes at both optimization levels.
- Three portability probes accidentally returned their own boundary or a share
  of it, creating infinite types (E0505). The probes now keep a finite outer
  Nil-returning boundary and test the specified crossing point. A separate
  incorrect Task-handle portability assertion was replaced with the scope-
  witness rule and an independent unused-service retention check.
- Scope setup originally suspended before keep_env, losing unused services at a
  tail-called scope. Retention now surrounds the first scope/run/event suspend.
  The Marker probe passes without a compiler special case.
- Invalid-script cleanup dropped an unstarted root, then checked deadlock before
  completion. The loop now rechecks completion after that transition.
- Async siblings exposed a resume guard using bitwise 1 & status, rejecting
  status 2. It now compares status > 0 before testing async-live. A direct WAT
  driver isolated the export guard; the real Rust-host regression and spec case
  replace that substitute diagnosis.
- The first Rust-host test draft had one unmatched parenthesis and assumed a
  nonexistent BuildOptions::default. Both compilation failures were corrected.
- Library assertions still called i/task reserved and expected no functions.
  Their exact list now comes from §6.1; only the five IO modules remain reserved.

Focused result: all 33 task cases pass at O0/O2 (r2-task-spec5.log). Remaining
stages include nested cancellation through retained outer callbacks, completion-
gated Rust/JS probes, all remaining traces, full public-interface and WAT audits,
and scheduler/channel cost measurements.
The first stage-4 full gate reached codegen and rejected Clock's old slot 4.
Scheduler and Runner add two preceding service slots, so the exact representation
assertion is now 6. Behavioral environment cases remain unchanged. Local/portable
paths query core.portable on the ordinary boundary witness; no compiler local flag
or special-case portability operation was added.
Stage-4 final gate passed, exit 0 (target/r2-stage4-verify2.log): Rust fmt,
clippy and workspace tests including the new real-host status-2 regression;
228/228 spec cases at O0/O2; 1,446 std tests per level, hello and bench, WASI
smoke, 76 Node and 51 browser tests. old-syntax-check passed 1,259 extracted
blocks with exact diagnostics and the same 19 future parser entries.
Library schemes and the browser fixture/plan were generated by their normal
commands and form the following separate regeneration commit. No performance
claim or complete WAT-audit claim is made by this functional checkpoint.

## Stage 5: nested cancellation, host gates and complete task laws

The source rule for nested cancellation is R4: “The outer task boundary stays
active” and “all awaits and sends they perform go to that still-active task
boundary.” A retained callback can suspend an outer scope delimiter from an
inner task. The loop therefore records the actual home task of each delimiter,
its nested-run route and cancellation cause. Forwarded cancellation is data in
the driver; it is applied only after returning to the home task continuation.
This keeps the loop outside every captured task boundary. Cancelling the parent
first drains nested runs, removes their interests, closes their roots, and then
unwinds the parent's delimiter. No second host continuation is introduced.

Cancellation causes contain run identity, scope and epoch. R4a says “A cancelled
outcome retains its causing scope and cancellation epoch”; a numeric epoch from
one run cannot exempt an unrelated cause in another run. Cleanup scopes keep a
birth anchor and their own failure domain. R4 says new scopes created while
unwinding remain protected from enclosing cancellation but still obey their own
internal failure rules. The fresh_cleanup_failure and foreign-scope probes test
both halves, including two levels of retained callbacks and blocked senders.

Scope delimiters also ensure FinishScope on arbitrary enclosing control.cancel,
not only on scheduler-generated cancellation. R3 requires “Before either
successful or failing return, free the scope's channels, shares, completed-task
table entries, and registration records.” FinishScope is idempotent when normal
retirement already ran. outer_control_cancel installs a yielding child final
before abandoning the enclosing continuation and requires that final to finish.

The root poll now takes an explicit runnable flag. §6.4 says “While there is
progress interest and a task is runnable, poll once before its next turn, using
status 2” and “Never ... busy-poll with no runnable task.” Idle roots use wait;
nested polls still forward interests and yield only the calling outer task.
Normal maintenance traverses task rows directly, skipping empty notice queues
and open scopes instead of doing a second path lookup for every task. These
changes preserve FIFO and the point at which bookkeeping is observed.

Interpretation checks and their requirement sentences:

- cancel_points and cancel_ready_await exercise every ordinary suspension:
  yield, send, receive, sync, join, scope end and Await. R4 delivers cancellation
  at “the next scheduler suspend”; R4a processes cancellation before a selected
  cancelled join. Even an already-Ready await must suspend through Await.
- cancelled_join_loses and cancelled_join_cycle follow R4a: a cancelled join
  alternative is ready, but it stops only when selected in a shielded task.
  An earlier always leaf wins; mutually joining shielded finals stop rather
  than wait for each other's cleanup.
- channel_waiter_gone and committed_message_survives separate R3's still-present
  registration from §6.4's atomic committed transfer. map_after_commit and
  event_services follow “Apply only the winning branch's map functions, in
  composition order, in the syncing task after commit.” Mapper error/suspension
  cannot roll back the peer's transferred message.
- nested_paths and the host trace use D10/§6.1's shared spawn/run counter,
  beginning at 1, and §6.5's absolute script paths. Expected random words are
  independently derived with random.at_path at [1,1] and [1,2], rather than
  copied from a task trace. The same -7 seed and script must give equal paths,
  values and trace at both optimization levels and on every host (§6.5).
- join_reusable, local_share, drop_unstarted, race_failure, send_event_stale,
  invalid_losing_run and wrong_run_before_payload check R1/R3, D7 and §6.4/R5:
  non-consuming joins while live; portable shared values in local scopes;
  no execution of an unstarted dropped child; failure before race success;
  no liveness check at event construction; all run checks before leaf/payload
  validation. Source-order/FIFO/identity/type-variance laws remain covered by
  the earlier cases, not replaced by generated expectations.

The real-host package tests/browser/fixtures/r2 is shared by the Rust integration
suite, Node and Chromium/Firefox/WebKit. Host starts capture BG's immutable
invocation ResourceContext; tests grant each compiler-authenticated declaration
explicitly. Each context is live during starts/checks and dead after return or
stop. There are no timers in completion decisions. overlap requires starts 1
and 2 before a runnable releaser can answer either; unlisted_answer answers 2
while 1's waiter remains registered, then releases 1 from another task. These
are §5.1's “keep waiting through answers of unlisted requests” and §6.4's
progress/fairness rules. Owned cancellation drops the pending Rust future or
aborts the JS signal before the scope returns; borrowed cancellation leaves it
live, as §5.4 says. race_read uses readiness waits and then takes the winner's
still-Ready answer. The losing producer survives until run retirement.

owned_idle uses a custom Owner and no waiter on its raw operation. §6.4 requires
its Root membership to defer deadlock until the producer settles; Rust observes
one future poll. unrelated_idle starts under a different Root and must stop
before any future poll; JS observes signal abortion before the first Promise
continuation. These instantiate review O/N-R3/N-R5. nested_interest races a
nested IO waiter against an infinite yielding task; the first host answer must
reach that waiter and cancel the CPU branch (§12.8). raw_only makes no Await
registration, so its runnable switches must not poll even with raw work.
Pure task switches are checked to request zero host macrotasks. cmp/cmp2 and a
settled fake stay synchronous; sync_poll after an async entry tests async-live,
not a stale host-root pointer (D5 and §12.8). The suite repeats each cancellation,
IO/fairness and deadlock probe 200 times per optimization level, separately
from 200 scripted random/path replays.

Corrected intermediate checks in this stage:

- Draft helpers had mismatched Runner wait/poll arities, a tuple/choice grouping
  error, selectors through an unrefined alias and an accidental id replacement.
  Type checking rejected them; the implementation was corrected.
- The first nested-cancel probe lost a final because a retained outer callback
  began cancellation before recording the home inner cause. ExternalCancel now
  records that cause before invoking the inner abort. Nested final scopes and
  two nested-run cases then passed.
- Initial foreign-scope probes stopped boundary-not-active or missed a child
  final. Home routes, birth anchors and idempotent FinishScope fix this.
  Deep relay replies preserve their typed value and discard flag across every
  hop. The blocked-callback probes pass without keeping a dead boundary alive.
- Two test drafts used a module-shadowing local named request and a final with
  an uncaught error row. Renaming the local and explicitly catching the mapper
  error corrected the probes; no public type or check was weakened.
- A labeled-constructor wildcard pattern and a choice inside a clause were
  rejected by the accepted parser. Positional patterns and separate clauses
  express the same implementation in accepted syntax.
- The host fixture initially omitted wasm.exports and its public service types,
  and tried to export ScheduleError/RandomError through the ABI. Explicit
  exports and exhaustive result handling corrected those compilation failures.
- Initial host assertions omitted the specified panic prefix, assumed Nil was
  JS null, and required a newline after an otherwise complete deadlock message.
  They now assert the exact first message line, use JS undefined for Nil, and
  retain separate call-position checks where §11.7 specifies positions.
- event_services initially parsed a tuple after yield as another function call.
  A named tuple binding corrected the source; O0/O2 both passed.

Clock remains IO-owned: runtime.md §11's ownership table assigns
lib/i/clock.i to IO. R2 supplies its ordinary request/Await/scheduler substrate.
No clock stub, special namespace rule or alternative host ABI was introduced.

The future-track JP marker beside spec §10.2 covers these additional exact
sentences inherited by JP: “Normalize an async host declaration to its
request-and-await provider first”; “A pure bundle with no such dependency may
be synchronous”; “Metadata and wrapper shape follow the graph before linking
and optimization; a requested synchronous export that classifies async is a
compile error (§2.6.2)”; “Construct all services in §2.6.1's one topological order,
enter the sole adapter at its place in that order, and invoke the unchanged
source entry with the chosen body services.” R2's default wrappers are ordinary
i definitions but do not yet select or construct user factories/adapters.

Performance measurement (substitute for an isolated primitive microbenchmark):
`heavy direnv exec /home/agent/Programming/i ./scripts/bench r2_` built the
release compiler and passed the three checked benchmarks. Then
`heavy direnv exec /home/agent/Programming/i bash tests/r2-bench.sh` ran five
samples with that executable at O2. Each reported duration excludes build time,
as scripts/bench specifies. This shared machine was also running verification
and WAT analysis; these are busy-machine measurements, not quiet acceptance
numbers. runtime.md supplies no numeric spawn/switch/channel latency target.

| Operation measured | Five wall times, seconds | Median aggregate cost |
| --- | --- | --- |
| 1,000 spawn + scope + join + retirement cycles | .580, .649, .686, .590, .602 | 602 microseconds/cycle |
| 1,000 yields in a live task | .019, .014, .014, .015, .016 | 15 microseconds/switch |
| 1,000 rendezvous transfers, including sender/receiver switches | .050, .077, .053, .052, .048 | 52 microseconds/transfer, or 26 per send/receive operation |

All samples checked sums (1,000 child results, 1,000 yields and 500,500 received
integers). The spawn number includes scope setup and cleanup; it is not a claim
about the isolated go instruction. The channel number includes both endpoints.
The source and five-sample script are retained for a coordinator quiet rerun.
The immutable tables favor correctness and a small plain-i implementation;
these measurements do not claim constant cost with arbitrary task population.

Stage-5 full gate passed, exit 0 (target/r2-stage5-verify.log): Rust fmt/clippy
and workspace tests; 259 spec cases at O0/O2; 1,465 std tests per level, two
hello tests and fifteen bench tests per level; WASI smoke; 78 Node tests and
54 browser tests. The host stress test passed 200 iterations of each of nine
probes at each optimization level, plus 200 seeded replays. The independent
syntax gate passed 1,301 extracted blocks, exact expected diagnostics and the
same nineteen future parser entries (target/r2-stage5-syntax.log).
The library golden and both R2 browser artifacts/plans were regenerated by
UPDATE=1 types/library and build-abi-fixture.sh, respectively, and are committed
separately. Further work adds direct JS execution of all runtime spec bodies
and finishes the detailed WAT comparison; neither is claimed by this checkpoint.

## Stage 6: shared conformance artifacts and final audits

The added conformance generator reads every .i case in §§9.9/9.11. Compile-error
cases stay in the real spec suite; every runnable case is compiled into shared
O0/O2 modules. The temporary adaptation removes @main, adds an ABI Nil wrapper
which stops on an unexpected typed error, exports its required service types,
and gives each test's host declaration a distinct namespace. It does not edit
the original spec sources. Distinct namespaces are required by the general
host declaration uniqueness rule (E0311); the first generation attempt correctly
rejected the duplicate test:io/test:async fields. No namespace exception was
added to the checker or scheduler. A second generator assertion counted host
edits as main attributes; a separate main counter corrected that failure.

Expectations remain the hand-authored // expect and // output headers, each
already tied to a source rule in the original case and this report. The first
JS conformance run found a generator interpretation error: an absent output
header had been turned into an expected empty log. The spec harness says
“With output lines” it checks output (crates/ilang-cli/tests/spec.rs:20), and
its Case.output is optional. The generator now preserves absence as null;
present output headers still require exact log equality. It does not derive
expected values by executing the implementation. The spec suite's originals
and all of their checks remain unchanged.

The last law additions make two checks explicit rather than relying on shared
implementation paths: invalid_losing_join/invalid_losing_gone prevalidate
same-run chain/liveness errors in losing leaves (§6.4), and shielded_final_io
awaits a real host answer, yields, then rendezvous in a fresh cleanup scope
before the original scope error is returned (R4). derived_seed independently
checks the full drawn-and-split top generator at [1] and [1,1], rather than
reconstructing it from its original numeric seed (§6.1/6.5).

Review O's final host probe now deliberately leaves producer 9 Pending at its
first Rust poll. Only after the invocation itself yields Pending does the
external test driver issue its release command. The second producer poll
completes it, followed by deadlock. In the unrelated-Root variant the invocation
stops before any producer poll or release. The JS driver similarly releases
the owned producer only after observing its first host handoff. This replaces
the earlier ready-at-first-poll substitute with the note's pending-then-release
trace. The 200-iteration matrix retains exact assertions (two polls versus
zero). No timer or clock decides release. The JS review E probe now also calls
the pure wrapper as an actual Array.sort comparator.

The last fairness audit found and fixed a missing go poll point. §6.4 says go
returns in the parent's same turn “after cancellation checks and a host poll
only if a task needs host progress.” Retaining State.current had also skipped
that poll. The loop now polls an existing interest even while retaining the
current parent, and treats that retained task as runnable. No extra scripted
choice or parent yield is introduced; no-interest commands still skip host
progress. go_progress starts producer 10 in a sibling, releases it from the
parent's next turn, then requires the host answer to be serviced before go
returns. It is part of the 200-iteration O0/O2 Rust/JS matrix. The focused Rust
matrix passed after this correction (target/r2-go-progress.log).

The first final syntax check rejected two incidental host strings: the generated
TOML manifest and the JavaScript diagnostic “case/export coverage differs”.
They are recorded as non-i data in the existing inventory. All original i
bodies and the added wrapper still parse normally; no future parser exception
or expected error was removed. The fixture generator's real i wrapper is parsed
again by conformance_fixture_sources_parse.

Stage-6 checkpoint: target/r2-final-verify.log passed, exit 0: Rust fmt/clippy
and all workspace tests; 263 spec cases at O0/O2; 1,465 std, two hello and
fifteen bench tests per level; WASI smoke; 79 Node and 57 browser tests.
The shared artifacts execute all 79 runnable request/task cases at both levels;
compile-error cases remain checked by the Rust spec harness. Each of ten
completion/cancellation/deadlock/fairness probes and the scripted trace runs
200 times per optimization level on Rust and all JS hosts. The corrected
syntax check passed 1,310 extracted blocks with the same nineteen future
parser entries. WAT and interface details are in r2-wat.md/r2-interface.md.

A last merge attempt found redesign had advanced to f6f9cc3 (T1 and the numeric
design). Git refused to overwrite the pending old-syntax-check inventory edits;
it changed no source and left no merge in progress. This green checkpoint is
committed first, then redesign is merged and the gates are repeated. No stash,
branch switch, root-checkout write or shared lock change was used.

## Stage 7: integrate current redesign and close failure-order laws

The merge to f6f9cc3 needed four textual resolutions. The host-field validator
keeps R2's normalization of Request/Result and adds T1's diagnostic on that
normalized wire result/error type. Spec §10.3 now says host-given Int/String
members “must have all its values”; the request result is such a host-given
value. literal_answer rejects Request("ok") with E0320, while literal_parameter
accepts a finite outgoing String parameter and checks its full String answer.
Both pass at O0/O2. The spec merge preserves that paragraph and the R2 boundary
exclusions together. The solver keeps both the invariant-key test and all T1
literal tests. The syntax inventory uses a three-way key merge, retaining T1's
consumed parser entries and R2's genuine non-source classifications.

The merge message initially retained Git's conflict comments after the trailer;
it was immediately amended so its last line is the required co-author line.
There was no persistent Git configuration change. The prior refused merge
changed no files; the four later conflicts were resolved and committed.

failed_sibling_join is R2/R4a's named trace: a child fails before its sibling
starts, while the body is joining that sibling. The body final yields and ends,
then the original error is returned; the cancelled join does not stop the body.
For the separate least-path law, the Rust integration test copies the actual
plain-i scheduler source into a temporary pure probe and seeds its private
Keyed error storage. The two unused IO observation declarations get pure stubs
only in that copy. Its calls to the unchanged lowest_error/read code check both
observation orders, prefix [1] before [1,1], and [1,1] before [2]. This is a
storage/arbitration unit test, not a claim of simultaneous M:1 failures or a
replacement scheduler. It passes on the Rust host at O0/O2.

The default initial seed follows spec §2.6.1's “run remains FIFO, path [], seed
0, with no entropy draw” and runtime D5's corresponding shipped-adapter text.
Explicit top uses §6.5's “replaces only the seed ... preserves its path allocator
and wait route.” The generator golden was rerun after the merge; the inferred
library interface did not change, including the documented row intersections.

The post-merge full Rust gate found one upstream fixture ambiguity:
types::pattern::shared_place_tests::written_occurrences_include_other_constructors_and_aliases
searches all program types by the spelling Event and selects i/task.Event rather
than the fixture's app/main.Event. The failed assertion is pattern.rs:1267.
Spec §2.3's ordinary module-qualified identities permit both types, and runtime
§6.1 requires R2's public Event name. The precise prepared fix adds the module
app/main to that test's id lookup, preserving every assertion. This file is
outside R2's owned regions, so it remains unmodified pending the requested
ownership ruling. The one-line diff is target/r2-pattern-fixture.patch.
No test was skipped or weakened to obtain a green full-Rust claim.

Independent post-merge checks completed: the full spec suite passed 322/322
cases at O0/O2; dev-verify i passed 1,465 std, two hello and fifteen bench tests
per level plus WASI smoke; dev-verify js passed 79 Node and 57 browser tests;
old-syntax-check passed 1,459 blocks with seventeen future parser entries after
T1 consumed its entries; the complete ilang-wasm test package passed. These
independent results are not a substitute for the currently failed full Rust
gate. The direct Rust R2 matrix (four tests, including all 200 repeats at O0/O2)
also passed after the merge.

The nested-interest host probe now contains both an inner CPU-yielding sibling
and an outer one. This explicitly exercises Runner.poll forwarding while the
nested loop is runnable, rather than proving only idle Runner.wait forwarding.
Its inner CPU branch stops after 32 unanswered turns, so a missing route fails
without a timer or an indefinitely hung test. The expected answer still comes
only from the explicit completion gate. The four-test Rust matrix passed this
stronger probe; the JS artifact and test repeat follow its regeneration.

Final busy-machine benchmark repetition, after the go poll correction and T1
merge, used the same five-sample script and checked sums. Wall-time samples:

| Aggregate operation | Samples, seconds | Median cost |
| --- | --- | --- |
| 1,000 spawn/scope/join/retire cycles | .527, .631, .576, .537, .543 | 543 microseconds/cycle |
| 1,000 yields | .022, .014, .018, .014, .016 | 16 microseconds/switch |
| 1,000 rendezvous transfers | .053, .057, .065, .053, .061 | 57 microseconds/transfer, 28.5 per endpoint |

These replace the earlier checkpoint numbers for the final implementation.
They remain aggregate, busy-machine substitutes for isolated primitive costs;
no quiet-machine latency or absent numeric target is claimed.

Native §5 does state a within-2% goal for existing one-worker benchmarks against
the M:1 baseline. That is a multicore implementation/epoch-fallback comparison;
its table also asks for a 1/2/4/8/16-worker curve and encode/lock measurements.
Native §7 explicitly defers queue encoding/claim ownership and transport to MC.
This R2 implementation supplies that M:1 baseline; it has no multicore variant
to compare and does not claim the 2% target. There is no prior i/task API at the
1df2fe0 executable for a task-spawn/channel before-and-after comparison.

The prepared pattern fixture fix was validated in an isolated git-archive copy
under this lane's target/r2-pattern-probe: the exact formerly failing test passed
with every assertion unchanged (target/r2-pattern-isolated.log). This is a
substitute isolated verification of the proposed fix, not a green check of the
unchanged repository file. No owned-path exception is assumed from silence.
The final stronger JS artifact passed dev-verify js: 79 Node and 57 browser
tests, including all 81 runtime spec bodies at O0/O2 and the 200-run matrix.
The O2 retention functions 70–72 were re-inspected in the final artifact; the
restoring/non-tail-call and saved-base re-entry bodies remain byte-identical to
the earlier inspected versions.

The next redesign merge, c2f41b7 (T2 value types and alias patterns), needed two
textual resolutions. Its early host-field skip for value-dependent types is
retained before R2's function destructuring and wire normalization. T2's joint
J6 sentence is “T2’s context check precedes host/service shape checks”; such
signatures are rejected with E0531, not deferred accepted ABI declarations.
The existing early-context provenance tests verify that distinction. The
syntax inventory again uses a three-way key merge, so T2's consumed entries
stay removed and R2's non-source strings remain classified.

The merge's git diff --cached --check reported upstream review cases.tsv's
empty second-column tabs as trailing whitespace, and an upstream main.i blank
line at EOF. These are T2 review fixtures outside R2's paths; the merge keeps
their bytes. This whitespace diagnostic is reported separately from the real
format/lint/test gates. The merge commit ends with the required co-author line.

After that merge, the full gate again passed fmt and Clippy and failed the
same outside-owned-path Event fixture (29 passed, one failed, four pre-existing
ignored tests). Independent checks passed all 337 spec cases at O0/O2 and the
four Rust R2 integration tests, including the 200-repeat matrix. Regenerating
the inferred library golden produced no diff. The O2 fixture printed with
wasm-tools is byte-identical to the previously inspected final O2 WAT, so the
retention inspection still describes the rebuilt artifact.

The repeated WAT export passed in 343.56 s: 1,282 files. All 1,232 files from the
post-T1 audit are byte-identical; T2 adds 50 exports. The 1df2fe0 comparison is
unchanged for all 522 reference files. See r2-wat.md for the source attribution
of every existing difference and the explicitly labeled diagnostic substitutes.

Final post-T2 independent gates passed: dev-verify i/js, exit 0, with 1,465 std,
two hello and fifteen bench tests at each optimization level, WASI smoke, 79
Node tests and 57 browser tests (target/r2-t2-i-js.log). The final fixture
generation completed before the release build reached the JS tests; the shared
artifacts contain the stronger nested-poll probe and all 81 runtime law bodies.
old-syntax-check passed, exit 0: 1,726 extracted blocks, exact diagnostics and
seventeen existing future parser entries (target/r2-t2-syntax.log). No R2
parser exemption was added. The full gate's Rust fixture failure remains an
explicit outstanding check, rather than being hidden by these independent
passes. Redesign is merged through c2f41b7.

A fresh isolated archive of the post-T2 merge was checked with that same
one-line fixture qualification. The complete checker library test target passed:
30 passed, zero failed, four pre-existing ignored tests
(target/r2-t2-pattern-isolated.log). Every assertion is retained. This remains
an explicitly labeled isolated substitute; the real pattern.rs is unmodified
because the plan's ownership table assigns types/pattern.rs to T1. The pending
ownership question is required by the user's “Change only the paths your scope
owns” instruction. No full-gate or final-stage green claim is made from this
substitute. Final-stage source/tests and generated artifacts stay uncommitted
until the real full gate passes; the earlier green stages and both redesign
integration merges are committed.

## Final follow-up: approved fixture repair and quiet measurements

The user approved the prepared fixture qualification. Commit 3b9393a changes
only that test's lookup to app/main and retains every assertion. Its commit
message states that this is test-only and explains the new library Event
collision. Redesign was already merged through c2f41b7. The earlier ownership
block is resolved; the isolated passes are not substituted for the final gate.

The quiet harness reserves all three heavy slots and executes the already
built Rust test binary, without nesting heavy inside quiet. It reuses one
real Rust-host instance, compiles at O2, checks results, alternates paired
measurement order, and subtracts both the n=0 kernel and the empty-loop delta.
There are five batches of 21 paired samples at each of n=32,64,128. Compilation,
initialization and warmup are outside timing. The in-scope variant additionally
excludes opening/retiring the enclosing scope; its two synchronous timer imports
are removed by the zero-work pair. Both methods and limitations are reported
in r2-performance.md. Instrumented binaries are counted separately, never timed.

The first quiet measurement demonstrated an algorithmic defect: retained clean
children were traversed at every switch, so empty spawn/join grew from about
328 us at 32 children to 907 us at 128. The new persistent AVL task store gives
logarithmic lookup/update while separate creation-ordered path indexes track
active tasks and unfinished cleanup. Status changes update only the main store;
index membership changes only at insertion/cleanup. This avoids duplicating
TaskState updates in both indexes. A two-list FIFO queue and queued flag avoid
linear enqueue copies and normal-turn membership scans. Scope child counts
avoid repeated child scans, and retirement removes exactly the scope's recorded
children. Child paths are consed for retirement only; no scheduling order uses
that reversed list. The 513-child task_index case checks all results and repeated
late joins before scope retirement, independently of any generated expectation.

These changes preserve §6.2's “A normal exit waits for every child” and §6.4's
“Cancellation bookkeeping runs before another normal task turn”. Creation
serials preserve the previous cancellation/message order, independently of
semantic path order. The root path and combined go/run counter are unchanged.
The interest set now uses the existing Await registration index, following
§6.4's “tasks actually suspended in Await.wait on Waiting tickets”. Pending
counts change in the same indivisible M:1 state transition as task cleanup.

The profile also found eager at_path derivation in every task start. The private
Scheduler now retains a pure thunk over the complete root generator and immutable
task path; random() computes the same at_path value on demand. Runtime §6.1 says
“path and random are data reads without a suspend”, and §4 says task.random
derives from the root's “complete generator, including previous draws/splits”.
Neither sentence requires computing unused words at spawn. The thunk makes no
Scheduler command, adds no suspension, and retains the complete generator rather
than reconstructing it from a seed integer. Public schemes and constructor
privacy are unchanged. No compiler optimization or random-module change is made.

The user examples in r2-examples run at O0/O2 with complete output assertions.
The release CLI also ran all four packages at O2: scope prints 42; race prints
loser cancelled then 7; channel prints hello; the missing sender exits 1 with
panic: task: deadlock. r2-user-guide.md contains the three complete programs.
Their expected outputs come from §§6.1/6.2/6.4, not generated observations.

Intermediate failures and corrections in this follow-up:

- The first performance fixture put imports after exports (E0115). The imports
  now precede exports. Example declarations initially used invalid () ! Io
  spelling; the actual programs use the environment parameter (io ! Io).
- The first AVL draft used positional patterns with labeled fields (E0510),
  and an ungrouped pending-count expression piped an Int into Bool clauses
  (E0501/E0516). TaskNode has positional fields; the count branches are explicit.
- New spec headers initially said ok and left a stop message unquoted. The
  harness requires pass and a quoted complete panic message. No expectation
  value or assertion was weakened. task_index's join helper also needed Assert
  in its explicit environment; a first textual correction missed the formatted
  signature, so that same failure repeated before the exact signature was fixed.
- Example code first imported a nonexistent i/io.print binding. Io is a service
  with a print field, so the programs now call io.print. They are standalone
  packages tested with the standard service, rather than custom test namespaces.
- The new Rust example test was accidentally inserted inside the arbitration
  probe's raw i string. It was moved outside the string; the i probe is unchanged.
- The lazy RNG edit first left the private field annotated Generator instead
  of () -> Generator. The annotation was corrected before functional testing.
- The profiling panic closure first returned anyhow::Result, which does not
  satisfy Wasmtime 50's IntoFunc. It now returns wasmtime::Result. Its initial
  max_wasm_stack exceeded async_stack_size; both profiler limits are explicit.
- A timer registration Result was initially unused. It is now unwrapped, so a
  registration failure fails the measurement rather than being ignored.
- Keeping debug names changed declarative elem ordering; formatting also changed
  source-site data. A whole-binary strip comparison therefore failed. Every O2
  function body was instead checked byte-for-byte, and counters were inserted
  into the actual timed artifact, using names only for attribution. This is not
  a claim of whole-binary equality. Counter runs validate all returned results
  and require status 0, so pure tasks make no progress hand-off.
- Two concurrent dev-shell entries printed a transient devenv chmod/load-exports
  race. The commands still executed; later serialized entries succeeded. No
  persistent environment or shell configuration was changed.

The preliminary full verification and WAT export began before the last path-index
refinement. They provide intermediate evidence and are superseded by the final
head's gates and fresh export, rather than being claimed as a consistent final
checkpoint. All source changes are frozen before the final regeneration/commit.

The first in-scope timer probe used Int subtraction on Float timestamps and
failed E0501 before measurement. It now uses the specified Float operator -.
(spec §6.9); the fixture passed source checking before the quiet repetition.

The first successful in-scope timer run was followed by an outer-measurement
failure: keeping the clock declaration in that module required its host import
even for the pure exports. The clock wrappers now live in intervals.i and are
appended only to the temporary interval package. The ordinary fixture and its
counted/timed artifact retain only the panic import; an unused clock callback
is not silently supplied to make the pure measurement pass.

The final hot-function review found a further algorithmic mistake: replacing
an existing AVL value called the insertion balancer up every ancestor, even
though subtree heights cannot change. update_task now preserves each height
and shape for status/value replacement. Only insertion and removal rebalance.
record_task shares the bookkeeping between insertion and replacement without
copying the tree twice. This is a scheduler algorithm correction, not a compiler
optimization. The following quiet repeat and full final-head checks include it.

A measurement-driver repeat initially chose cargo's i binary from JSON output
instead of the test executable, so it rejected --test-threads and measured
nothing. The driver now selects the r2_performance test artifact by both target
name and kind. The failed invocation is not a timing sample.

The final export was queued before the quiet runner had acquired all slots,
which delayed measurement. That premature wat_dump child was terminated with
SIGTERM, selected by its exact lane and WAT_DUMP path. No other process was
stopped. Its partial target/r2-wat-final is not used for acceptance. The quiet
run finishes first, then a fresh authoritative export runs on frozen source.

Early quiet samples used only 10/20 warmups. The first, n=32 experiments were
consistently slower than later sizes on a fresh 256 MiB heap. The final protocol
uses 200 warmups per kernel/size, matching the numeric profile's steady-state
method and allowing heap pages/collector state to warm before sampling. The
earlier measurements remain intermediate evidence, not the final cost table.

A CPU-heavy diagnostic was initially outside the shared slot wrapper. It was
briefly paused to protect the quiet window; the pause helper also matched
itself. Both owned processes were resumed, and the diagnostic was moved under
a heavy-slot lease. No other process was changed and no process remains paused.
Because that transition could overlap the first 200-warmup run, that timing is
not used as the final quiet result. A clean repetition waits for the leased
diagnostic to finish. Counter values are unaffected by timing overlap.

The all-file owner-normalizing diagnostic was stopped after more than five CPU
minutes without a result. It is replaced by an exact, unnormalized comparison
of every common runtime function in every changed compiling export. The slow
check is reported as interrupted, not passed; the replacement is a diagnostic
of unchanged compiler-runtime code, alongside the source-based library audit.
The slot lease ended and both previously paused processes were restored/ended.

The boundary callback in turn formerly captured s and t to select a few fields.
That could retain the whole scheduler State and an obsolete TaskState through
another task's continuation, even though the loop's control continuation was
outside the boundary. The fields are now projected before constructing the
callback, which captures only run key, Runner, path and body. Runtime §6.3 says
“the loop is outside the task boundary”; its plain-i state must not become an
incidental task capture either. make_scheduler receives only run key/Runner,
not State. This is an owned retention fix, not compiler field specialization.
The final export, profiles and gates include these projected captures.

The accepted 200-warmup quiet run passed all three harness tests with unchanged
source hashes (target/r2-measurement-accepted). Inside-scope costs at n=128 are
107.251 us per empty spawn+join, 13.045 us per ready-task switch and 34.870 us per
send+receive transfer. The comparable outside-only spawn/join is 109.681 us,
against the old quiet 907.413 us. These are still not Go-level costs. Full raw
samples, the separate count artifact and hot WAT are retained; r2-performance.md
states the baseline formula, scope exclusion, timer imports and limitations.

The final syntax inventory first rejected the performance harness's generated
TOML manifest literal as if it were i source. That one host-data literal now
has the existing not_source classification, with its reason. No i source or
expected parser diagnostic is exempted. The corrected precommit check parses
1,747 extracted literals/blocks with 17 future-track entries.

Redesign advanced to 1a8fee2 with the concurrency and numeric prior-art studies.
The lane merged it in fed3bda; no compiler, library or test code changed in
that merge. The final frozen-source WAT dump passed in 405.64 seconds. Its
1,286 results retain all 522 original reference files: 188 byte-equal ERROR
results, 334 explained compiling-file differences and 764 added results. The
four additions since the T2 dump are task_index at all dump configurations.
The exact runtime comparison and final private-source attribution are recorded
in r2-wat.md.

The latest user instruction supersedes the earlier precommit stopping point:
commit the source/tests/audits and then the generated artifacts, and run both
the complete dev-verify and old-syntax-check on that final committed head.
The final report records those final-head commands and outcomes; preliminary
passes and the interrupted exports are not substituted for them.

Redesign then advanced again to 82375c0 (W0503). Merge 1c1f212 retains the
approved Event fixture qualification and the R2 host-data inventory entry.
The first final-head dev-verify was still waiting for a slot; its empty log
confirmed it had run no check. Its exact owned process group was stopped with
SIGTERM (143) before this merge. This is an interrupted queued invocation,
not a failed test or a passing gate. The syntax check on 8e262d7 had passed;
both gates are repeated on the later committed head.

The W0503 merge makes known-disjoint equality advisory, per amended spec §11,
and restores ordinary literal comparisons in the inherited tests. It changes
no scheduler, request or host runtime operation. A release compiler rebuilt
from this merge produced target/r2-performance-w520.wasm. cmp against the
accepted quiet run's outer/kernels.wasm passed byte-for-byte, so those quiet
timings and dynamic counts describe the final code despite the diagnostic
change. A fresh full WAT export and both official generation commands follow
that check. No alternative compiler or substitute timing is used.

The repeated post-W0503 export passed in 373.95 s with 1,294 results. Its exact
inventory and inherited-source attribution are in r2-wat.md. The subsequent
UPDATE=1 library generation and build-abi-fixture.sh both passed and produced
no diff: all generated artifacts are already at a fixed point on the merged
code. The nine changed compiling exports have no changed common runtime
function except seven source-initializer changes. A stray slash in the first
scratch-audit shell command returned 126 before running the diagnostic; the
correct command then completed successfully. No file or environment change
resulted from that shell typo.

## Independent review corrections

The lane began this follow-up clean at 0573e88. No interrupted edits were
present. Merge 27781967 brings in the QA corpus. The syntax inventory conflict
was resolved by key-wise three-way merge, preserving both tracks' entries.
The independent report and its observations are requirements evidence to
investigate, not replacement expected outputs.

References read before the corrections (under .rd/ref; none executed/copied):

- trio/src/trio/_core/_run.py: CancelStatus.recalculate,
  parent_cancellation_is_visible_to_us, CancelScope.shield and its setter.
  Shielding blocks ancestor cancellation while retaining cancellation from
  scopes created inside it. i requires this protection automatically in finals.
- eio/lib_eio/core/cancel.ml: protect, with_cc, cancel_child; switch.ml:
  await_idle and on_release_full. Release hooks run in protected child contexts,
  and a fresh scope can still manage its own failures. i keeps one least-path
  failure rather than Eio's aggregation.
- ghc/libraries/ghc-internal/src/GHC/Internal/Control/Exception/Base.hs:
  bracket and finally; ghc/rts/RaiseAsync.c: throwToMsg's NotBlocked case and
  blockedThrowTo. GHC masks the release action, with interruptible-operation
  exceptions. i's final protection must survive scheduler suspension as R4 says.
- go/src/runtime/chan.go: hchan.sendq/recvq and waitq.enqueue/dequeue;
  select.go: selectgo passes 2/3 and waitq.dequeueSudoG. Each source owns its
  waiters; one choice owns its withdrawal links. i uses source-order choices,
  not Go's randomized polling, and preserves FIFO partners.
- cml-sml/libraries/cml/src/core-cml/channel.sml: channel, cleanAndRemove,
  cleanAndEnqueue, sendEvt; event.sml: mkFlg, syncOnBEvts. Channel-local queues
  share a transaction identity so one commit withdraws the other alternatives.
- go/src/runtime/runtime2.go: g and sudog; proc.go: newproc1/goid allocation;
  tokio/tokio/src/runtime/task/list.rs: LocalOwnedTasks.bind and its owned list.
  Stable integer identities are distinct from paths. Their mutable records
  cannot simply be copied into i: runtime §6.3 says the loop threads immutable
  State, and forbids storing its state in forged host references or untyped casts.
- otp/erts/emulator/beam/copy.c: copy_shared_calculate and
  copy_shared_perform_x. A shared object's mark/forwarding entry prevents
  traversing its expanded paths. The single-worker spawn has no publication
  consumer for portable_record, so it must not classify that record at all.

Source interpretation: spec §9.5 says “A final function may not give errors”
and “When the program stops (§11.7), final functions are not run.” Stop variants
therefore check immediate stop without finals; typed failures from a final are
negative typing tests. Normal-return and typed-error finals must complete under
ancestor cancellation, and fresh cleanup scopes must retain their own failure
and cancellation rules (runtime §12.9/R4a). No stop semantics are changed.

The WIP checkpoint initially failed checking because `address_index` used a
one-element tuple in arithmetic and `finish_scope` queried the new TaskStore
as a list. The arithmetic now uses a block for grouping and the scope lookup
uses `store_find`. A 16-field immutable page made inference of `write_slot`
take tens of seconds. Four-field pages preserve the same table operations and
reduce the debug compiler's check of the cancellation reproducer to 0.34 s.

On the four-field implementation, all eight `r2_runtime` tests passed. The
original typed-error cancellation reproducer printed both final markers at O0
and O2. One non-quiet debug-compiler scale run passed at 128, 256, 1,000 and
10,000 senders, receivers and three-way choices; the 10,000 cases took 4.14,
4.60 and 4.22 seconds respectively including startup. Shared capture depths
28 and 60 took 1.55 and 1.56 seconds. These are single samples under concurrent
load, not quiet-window measurements; raw output is in target/r2-scale-current.

The full spec suite passed 348/348 cases at O0 and O2; the 9.9 subset passed
78/78, and all 38 imported review-r2 observations and requirements passed.
A subsequent full dev-verify passed the Rust tests up through the spec suite,
then was interrupted (exit 130) during value_type_performance to free capacity
for integration's corpus run. It did not reach the i and browser phases and is
not a completed gate. The integration gate must run on the merged source.

After merging K4, all eight runtime tests and all 78 section 9.9 cases passed
again, and the regenerated fixtures passed the Node and 57 browser tests.
The first review-r2 rerun was interrupted for the concurrent integration run.
For its 32 completed changed observations, K4 canonicalized the order of
environment-union members without changing the members or execution results;
the missing-service case also gained the corpus driver's two entry probes,
both reporting the same E0522. Those observations were updated on that basis.
The remaining six showed the same order-only change; after updating those
observations, the entire 38-case review-r2 run passed, with the intentionally
divergent case retained as one censored observation.

After UNUSED was merged, W0504 identified the dead `need` helper and the two
private entry wrappers called by compiler-generated code. The dead helper was
removed; the compiler-owned wrappers use the ordinary `_` exemption and the
lowerer calls those renamed definitions. No warning rule is specialized for a
library. The 8 runtime tests, all 78 section 9.9 cases at O0/O2, and all 38
review-r2 observations passed after this change. ABI fixtures regenerated with
no diff; the library type inventory was regenerated for the renamed wrappers.

The integrated review corpus exposed nine pre-R2 observations. Three E0320
diagnostics now describe the request-aware host field rules; five native design
examples now see the implemented task/request API, with the borrowed-request
example still reporting its missing embedder host. The numeric async probe's
explicit environment formerly omitted Await and Owner: runtime.md D3 and §5.4
add both to an async host field, so its annotation now includes them while its
floating-point computation is unchanged. Its original source is retained in
the review inventory as a historical snapshot. All nine updated observations
passed again, and the inventory fingerprint audit reported zero failures. The
migrated numeric export also ran against an asynchronous JS host at O0, O2 and
O3; each call made one host handoff and returned negative zero with its sign.
