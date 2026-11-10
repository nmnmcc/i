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
