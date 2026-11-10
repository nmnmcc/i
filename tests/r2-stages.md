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
- runtime.md D5: "On the ilang target the entry is async exactly when N contains
  an async/request host service or Scheduler or Runner from i/task."
  Entries follow inferred needs; the unlanded provider-closure design is excluded.
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
4. FIFO loop, entries, nested routing and retained task environments.
5. Structured lifetimes, cancellation, shielding, channels and shares.
6. Event registration, combinators and scripted tests.
7. Rust/Node/browser stress, public interface, WAT and benchmark audits.
8. Regenerate derived files.

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
