# Syntax three, stage 3: report

Current integration status, 2026-11-14: **verification in progress against
fetched `origin/main` at `cc26b8d9`**. The READY result below applies to the
previous `ab8178ab4` base, not yet to this new integration.

Previous base landing status, 2026-11-14: **READY for the requested gates**.
Main and the latest READY conversion fix are merged, the three provider
observations match main, and the final shard has five explained display
differences after unchanged-bound retries. The follow-up section below
supersedes earlier interim blockers. The inherited record-field solver
limitation remains documented separately.

Stage 3 of `syntax-3.md` §7.2: in today's syntax every type with two or more
constructors became one-constructor types and an alias; a definition with two
or more constructors is now E0238.

## The migrator

`cargo run -p ilang-syntax --example migrate-syntax3 -- types` rewrites in
place every `.i` file, every ```` ```i ```` fence in Markdown and every Rust
string literal with an `UpName {`; `--check types` counts what is left (0).
One module per subcommand in `crates/ilang-syntax/examples/migrate-syntax3/`,
one row per subcommand in `SUBCOMMANDS`.

- `T(A) { C1(A) C2 }` becomes `T(A) = C1(A) | C2`, `C1(A) { C1(A) }`,
  `C2 { C2 }`; each constructor keeps the parameters its fields use.
- Exports: `T(..)` becomes `T, C1(..), C2(..)`; `T` becomes `T, C1, C2`.
- A constructor whose name is already a type name (a built-in type, its own
  type, another type of the module, an imported name) gets the type name
  `<Name>Case`; uses of the constructor do not change. A wrapper of the
  built-in type of its name (`Int(Int)`) is also printed with §7.3's
  suggestion to drop it.
- Sources that contain `migrate-syntax3: keep` are skipped (the E0238
  fixture).

## By hand

- `i/syntax.Node` as in §2.8, with its users (`std/src/test.is`, Rust tests).
- `i/task` `Scheduler` and `Runner`: a service type is one named type (E0225),
  so each became a one-constructor service whose field is the union of its
  detached and attached states (`SchedulerService(state: SchedulerState)`).
- `std/http` `ResponseStream` and `ClientFunctions` became one-constructor
  types because of the solver bug below.

## Solver bug: error unions in record fields

A record field whose function type has an error union of six or more
one-constructor members fails `E <: E`; the member reported depends on the
order of the members. The same check passes for a field of a custom type
and for a function parameter, so the fault is in the records graph, which
stage 6 deletes. Lane s67 should confirm that this passes there:

```text
(i/fail)
(main)

E = A | B | C | D | Q | P
A { A }
B { B(String) }
C { C(String) }
D { D(String) }
P { P(String) }
Q { Q }

f = (n: Int) -> Int ^ E {
	n |> {
		0 -> fail.raise(A)
		_ -> n
	}
}

R = (read: (Int) -> Int ^ E)

g = () -> R {
	(read: (n) { f(n) })
}

main = () { Nil }
```

Today: `error[E0501]: \`Q\` is not a subtype of \`P | Q | { D | { C | { A | B } } }\``.
With five members, or with `R { R(read: (Int) -> Int ^ E) }`, it checks.

## Host ABI

`Bool = True | False` reaches the host as before: a host export
`(Int) -> Bool` has `"result":"bool"` in `ilang.abi`, an error `^ Bool` has
`"errors":["bool"]`, and the JavaScript bindings give `true`/`false`.

## Behaviour changes seen in the tests

- A refutable binding of an unknown value narrows it: `first = (r) { Ok(v) = r  v }`
  is `(r: Ok(A)) -> A` with no `Assert`, not `(r: Result(A, B)) -> A ! Assert`
  (lane js's D4 rule widens to `Q | r - Q` only when another constructor's
  value is known). Fixtures that pass an `Error` give the parameter the type
  `Result(…)` now (`tests/spec/9/assert.i`, `crates/ilang-check/tests/codegen.rs`);
  a recursive top-level value needs a written type on the part
  (`tests/spec/7/may_fail_group_ctor.i`).
- Types are more precise: `named : (r: Result(A, B)) -> Ok(A | 0)`,
  `not_known : (s: Shape) -> Circle`; a constructor's type is its own
  (`Some: (A) -> Some(A)`).
- A match that does not cover a member of the subject's sum gives E0516
  alone, as before (a former constructor that no pattern takes is held for
  the coverage error when a pattern takes another member of a declared union
  with it; `Checker::uncovered_member`). An earlier state of this lane also
  gave E0501 (`Point` is not a subtype of `Circle | Square`, corpus fuzz-5:
  `True` is not a subtype of `False`); that was a bug against §5.2.8 rule 11
  ("the coverage error in a match is unchanged").
- `Circle(..v, …)` with `v: Shape` gives E0501, not E0517's message.
- `field_shared` no longer joins the constructors of a former family.
- Coverage witnesses show a one-constructor type as its constructor
  (`("a", False)`).

## Decisions after the first report

- A build-discriminant service stays one nominal type; the spec's `Target`
  is `Target { Target(TargetKind) }` with `TargetKind = Js | NodeJS | Wasi |
  Native | Host`. The two `provider_graph.rs` discriminant tests were deleted:
  the provider code wants the discriminant's constructors to be the
  service's own, and the provider plan is being removed (E5).
- Display folds an alias's expansion also inside a larger union, the alias
  with the most members first: `String | True | False` is `String | Bool`,
  `Result(Int, Int) | String`.
- The order dependence of `value_types.rs` came from record graphs, not
  from alias folding: the record of std/http's request body reader showed
  its error union in checking order. That reader is now the custom type
  `BodyReader(E, N, M)`; the record solver bug above is the same family.

## Not solved

- The record-field solver bug above (stage 6 deletes records).

## After the conversion (merge of lane/conv, 2026-11-13)

The conversion landed first, so stage 3 now applies to syntax-three text:
the migrator `types` and its old-syntax output were dropped, and
`syn3/lower.rs` lowers each data definition to a one-constructor type and a
sum to the alias of the union of its members.

- `A(..)` of a sum exports the name and each labelled member with its
  constructor (§5.2.1), in the resolver.
- Sums with a member of their own name (§7.3 manual list) were renamed by
  hand: i/task's `Scheduler` and `Runner` services are
  `Scheduler = (state = SchedulerState)` with `AttachedScheduler` (likewise
  Runner); i/process's sum is `DirectoryMode`; std/http's sum is
  `AnyHeaders`; i/syntax's `Node` is the labelled type of one union item, as
  §2.8's text has it, and its users wrap values in `Node(…)`; test fixtures
  with `X = X | Y` name the sum `XSum`.
- The pattern type of a fitted `C(…)` is the smallest declared union of
  labelled types that contains `C` (§2.12, D4, "as today with the
  multi-constructor `Result`"): `first = (r) { Ok(v) = r  v }` is
  `(Result(A, B)) <- (Assert) -> A` again (types/effects.i, with the closed
  case `closed: (Result(Int, String)) -> Int = (Ok(v)) { v }`, E0501).
- A match reports a member of the subject's sum that no pattern takes with
  E0516 alone (§5.2.8 rule 11), also for a pipe into a clause-block
  literal, which is now a match (the Lisp surface's match).
- A discriminant service wraps the union of its targets' values
  (`Bundle = (value = BundleKind)`); the provider plan wraps the selected
  member (`ConstructionStep::wrap`).
- The known types of W0503 messages fold declared aliases (`Bool`).

## Landing review, 2026-11-13 evening

The three questioned expectation changes have been traced to their source:

- `projects/import_exported`: the converted library explicitly exports the
  labelled type `Hidden`, without its constructor, because `Opaque = Hidden`
  exports an alias that names it. Syntax-three §5.2.1 says, "If the type of an
  exported definition, or the expansion of an exported name, uses a type
  defined in this module, that type must be exported too." The import of the
  type therefore succeeds. Qualified construction and the bare constructor
  still fail; `Private` still checks an unexported type import. The separate
  `export_hidden` fixture still rejects an exported alias of a private type.
  Comments now explain this distinction without changing source coordinates
  or any expectation.
- `types/known`: §6.7 says, "`C(..v, label = e)` is an ordinary call whose
  data has a spread." §2.8 gives `Circle` its own nominal type and one
  constructor. The former E0517 text claimed its type had other constructors;
  it no longer applies. `not_known` still rejects the two-member spread
  (§2.3 requires exactly one known member); `wrong_ctor` still rejects the
  incompatible `Square` data (it has `side` instead of `radius`). Both errors
  are retained as E0501. No program changes acceptance here.
- `types/library.out`: `types/display.rs::tree_with`, in its `Ty::Fn` arm,
  deliberately reads labels from the parameter data's shape, rather than
  `FnTy.labels`, which also carries positional binder names. This was
  introduced by `80bc9296cd`, before the stage-3 merge; the numeric runtime
  modules had stale goldens after conversion. §2.4 says, "Function types
  drop the names and the body. The parameter part is a data type" and
  "Labels are part of the type". Thus `(x: Float)` becomes `(Float)`;
  real data labels remain `label = T`. The library golden test now documents
  the cause. Runtime module definitions and expectations are unchanged.

The worktree started clean at `682d1606d`. Wasm and macro package suites
passed. The requested CLI command reproduced the conversion's known
`a_captured_service_row_keeps_callback_dependencies` failure; its fix is
being tracked in `status-convfix.txt`. Remaining landing checks are in
progress; this paragraph does not claim the branch is green.

At the `19882c7d2` compiler tree, the remaining CLI binaries passed:
providers 18, commands 14, macro_run 57, typed_macros 36. Project goldens
(2 tests) and type goldens (5 tests) passed, including all three reviewed
expectations. A fresh release build passed; std O2 passed 2,170 tests with
zero failures. Shard 1/10 (198 cases, two jobs) is now queued. The conversion
fix candidate `d67bcf7e` restores the original written provider annotation
in the failed CLI fixture; it changes no compiler rule and is still awaiting
its own final validation before integration.

## Completed shard audit

The requested shard run completed on the same compiler tree: 198 selected,
140 accepted checks and 58 rejected checks, exit 1. All 170 output
differences are classified in [the per-case audit](syn3-stage3-corpus-audit.md):
11 message only, 144 shown type, 15 outcome/diagnostic differences. Two
additional requirement failures are recorded there. No expectation,
requirement, bound, or corpus source was rewritten.

Two diagnostic changes are explained by removed grammar. Eleven other
diagnostic/acceptance changes are classified as bugs; two O2 test timeouts
after passing stdout are under unchanged-bound retry. `proj/q35` also fails
its principal-type assertion (`S(N & A)` instead of `S(N)`). Several failures
are lost complete written needs in the conversion, including six permutation
cases. This branch is not landable while these remain unresolved.

The separately reviewed convfix `d67bcf7e` is merged at `62bc17079`: it
restores the captured-row fixture's original provider annotation and adds a
lowering comparison, changing no compiler code or expected result. Its
own CLI/lowering verification on this branch is pending. The newer
conversion head remains unlandable because of its reported `p1` correctness
blocker and has not been merged.

## Checkpoint after restoring source contracts

`022f42dc1` restores the original complete written needs in all 60
permutation cases, takes conversion's exact missing/mutual corrections,
and adopts its exact `proj/q35` forwarder. A 63-case retry preserves every
saved phase exit, diagnostic-code count, and runtime stdout, with all
semantic requirements passing. An independent audit also checks that both
original failing bindings remain: `bad` and `loop` in every permutation,
`first` and `second` in the recursive case. No observation, requirement or
bound is re-recorded. Spec §4.7's annotation paragraph is quoted in the
source-restoration commit.

After merging the isolated captured-row correction, this branch's CLI
`check` passes 10/10 and `syn3_lower` with `legacy` passes 19/19. Production
Rust, `lib/`, and `std/` are unchanged from the tested release compiler,
whose SHA-256 remains
`e0db61d40034648d64429b6361a70e324954999557b861d8263a1a8404cba719`.
The earlier Wasm, macro and std O2 results therefore apply to that compiler.

The two unchanged-bound resource retries pass every test/program phase;
only type notation differs. The optional all-forwarder verification was
stopped before starting after 20 minutes in the shared queue. Its 11
additional uncommitted backports were read and deliberately discarded;
the verified `q35` stays. They remain available on the conversion branch.

Two closed-callback probes did not restore the lost hidden E0515s; two
provider-nesting variants did not restore `b1`'s expected two E0515s. No
experimental change was applied, and those investigations stopped after
two unsuccessful variants. The two missing hidden errors and the extra
`b1` error remain unexplained blockers. The final shard is now queued on
the restored fixture inputs. The conversion head still reports its `p1`
blockers and is not marked landable, so its compiler changes remain unmerged.

## Final handoff, 23:24

The final shard completed on the restored inputs at `d402cf846`: 198 cases,
138 accepted and 60 rejected checks, zero semantic requirement failures.
Every case preserves its saved phase exits and runtime stdout (normalizing
only the property-failure source coordinates). The 63-case audit was repeated
and also confirms every original failing binding and diagnostic-code count.

The raw runner exits 1 for 170 output differences, all individually classified
in [the audit](syn3-stage3-corpus-audit.md): 11 message only, 154 shown type,
five diagnostic outcomes. Two are authorized removed-grammar diagnostics.
Three remain unexplained: the hidden-provider E0515 disappears from
`provision_closure_False` and `_loose/p/x8`, while `_loose/p/b1` gains another
E0515 at `main`. None was re-recorded. This branch is **not landable** until
these are resolved and a landable conversion head is integrated.

All heavy commands ran serially through the wrapper. Fresh results are Wasm
65, macro 9, CLI providers 18 / commands 14 / macro_run 57 / typed_macros 36 /
check 10, project goldens 2, type goldens 5, legacy-enabled lowering 19, fresh
release, and std O2 2,170/2,170. The initial combined CLI command failed on
the captured-row fixture before its restoration; its final retry passes.
The complete command/result ledger is in the shared coordinator cache,
`gpt/r2/report-s3-landing.md`, with the raw logs and both corpus snapshots.

The inherited record-field union solver limitation documented above was not
investigated or reverified in this pass. The coordinator-owned workspace,
unfiltered spec and unfiltered corpus gates were not run here. The latest
conversion status still reports unresolved anomaly groups and is not marked
landable, so no newer full conversion head was merged. The isolated captured
row fix and exact, verified fixture corrections were integrated separately;
`syn3/lower.rs` still does not absorb members.

## Follow-up after conversion landed, 2026-11-14

Merged main `68b720e9` at `a3cb1ba96`, resolving all 60 permutation source
conflicts to main. Stage 3's checker/LSP/Wasm changes and independent data
definitions remain. Main's provider contracts survive lowering. The later
READY fix `db743d5e`, merged at `2af820b34`, moves the open/closed provider
decision to resolution so transparent/imported aliases cannot hide holes;
it preserves the original annotation and value through lowering and keeps
closed function-valued providers' generalization boundary. No member
absorption is restored.

The three previously unexplained cases now match landed main exactly:
`provision_closure_False` has one E0515, `_loose/p/x8` two, and `_loose/p/b1`
three. A new checker regression separately pins these counts and compares
the complete diagnostic multisets, including source locations/messages,
against main's unchanged records. It passes after the merge. In particular,
main's `x8` source restores the full written need on `wrap`; the source
contract and provider-generalization repairs supersede the old discrepancies.
No expectation is rewritten in this follow-up.

The five-package gate passes: checker 357 (four existing ignores, requested
recovery test skipped), syntax 190, LSP 106 including all 98 protocol tests,
Wasm 65, macro 9. The new three-case regression passes separately after
compilation. Final workspace/release/runtime/static gates are in progress;
this checkpoint does not yet claim landability.

Both fresh std runs pass 2,170 tests at O0 and O2. The new shard 1/10 run
completes all 198 cases with no changed outcomes or requirement failures:
three shown types and two coverage-message witnesses are the only
differences. Every difference is explained in the November 14 section of
[the corpus audit](syn3-stage3-corpus-audit.md), including the exact alias
behind `ScheduleError` → `NotRunnable`. No expectation, requirement, source,
or timeout bound was changed. Main's exact three provider observations
match. CLI, chapter, generation, and static gates remain in progress.

The five requested CLI binaries now pass: check 10, commands 14,
macro_run 57, providers 18, typed_macros 36. The five spec filters pass at
both optimization levels: `4.3/` 17, `6/` 80, `7/` 104, `9/` 139, and
`15.7/` 28. Existing pending entries remain unchanged: 28 under `6/`, one
under `7/`, and one under `15.7/`. Short generation with the fresh release
passes 32 cases (20 accepted, 12 rejected by the model), 64 declaration
permutations, and 32 annotation removals. Its one expected report-only
annotation-removal difference cites spec §4.7; there are no failures.

The exact verification commands, exits, and log paths are recorded in the
coordinator cache's `gpt/r2/report-s3-r3.md` and JSON command ledgers.
All heavy commands are serial and wrapped. Formatting, clippy, and snippet
checks are queued for a shared slot; this checkpoint awaits those gates.

All static gates now pass: Rust formatting, full-root i formatting,
workspace/all-target/all-feature clippy with warnings denied, 4,776 normal
snippet sources, 34 Lisp files and 89 macro fragments, and eight snippet
helper tests. The first i format check listed `std/src/http.i` and
`std/src/test.is`; applying the existing formatter wraps the longer stage-3
names and adds an optional trailing comma to the HTTP result tuple. The
diff changes no body or string. Repository formatter property tests had
already checked canonical-tree preservation and idempotence.

The first snippet run found `"\nwith_int"`, the READY alias regression's
replacement search needle. Its new inventory entry supplies the exact
`Reader` prefix and `with_int` tail from `types/soundness.i`; the assembled
snippet is verified to be that file's prefix. It is parsed normally, with
no exemption or diagnostic recording. The checker regression still pins
all four Reader E0515 errors. The retry uses the snippet script's own
wrapped cargo build, avoiding a redundant outer build wrapper.

A fresh release rebuild and another shard run are in progress after this
cosmetic source cleanup, to confirm the final observation coordinates.

The fresh release rebuild passes and has the same SHA-256. The post-format
shard completes all 198 cases with the same 138 accepted / 60 rejected
checks. Besides the original five display differences, its first pass has
two O2 test budget timeouts, `cancel_choose` and `deadlock`, both after the
exact passing stdout. The unchanged-bound (10 seconds), two-job retry
passes both complete observations with no differences or requirement
failures. `deadlock` retains its expected run exit 1; its test exits are 0.
No observation, requirement, or bound is changed.

All 198 reconciled complete observations are verified identical to the
first full run. Final remaining classifications are three shown types,
two messages, and zero outcomes. The final audit preserves the failed
bounded observations as well as the successful retries. All requested
gates are complete; the exact command/result ledger and final hash are in
`gpt/r2/report-s3-r3.md` in the coordinator cache. The recovery test was
skipped as requested; four existing checker ignores, the existing spec
pending list, and the registered stress runtime limit remain. The inherited
record-field union solver bug is not fixed by this follow-up; the earlier
report defers it to stage 6's record-graph deletion. Coordinator-owned
unfiltered gates and main landing remain outside this lane's commands.

## Integration with the landed tip, 2026-11-14

Fetched origin before merging `refs/remotes/origin/main` at `cc26b8d9`;
the owner's `refs/heads/main` is not used. The single merge conflict is in
`row_schemes.rs`: preserve both alias-provider tests and the named-function
provider test as complete functions. Main's cached-handler revalidation
(`26b605121`) and named-helper regression (`58a356366`) remain intact,
alongside the resolved-alias fix already carried by this lane. Independent
data definitions remain independent; member absorption is not restored.

Read the fourteen observation changes from landed `180e00cf7`. Main now
records one, two, and five errors for the original three cases. The exact
diagnostic regression now covers sixteen cases: those three and the other
thirteen changed records. Each case independently pins its total errors
and E0515 count, then compares every complete error message and location.
The fourteen source programs and expected observations are verified
byte-for-byte identical to the fetched tip. No observation is re-recorded.
The workspace check and fresh gate matrix are in progress; the old READY
claim is not reused for this integration.
