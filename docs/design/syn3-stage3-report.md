# Syntax three, stage 3: report

Package-interface follow-up, 2026-11-14: **expanded CLI verification in
progress**. The earlier green CLI matrix covered `check`, `providers`,
`commands`, `typed_macros`, `macro_run`, and the requested spec filters.
`package_commands` was not in that matrix; its obsolete constructor-family
assertion is a real coverage gap. The follow-up runs its complete binary,
all other CLI integration binaries, and both CLI unit-test targets.
The coordinator main snapshot `e44514d4` is also merged so its newly added
`ispec` binary can run: 42 integration binaries are now accounted for.
The five previously tested CLI binaries are rerun after this merge, and
the requested spec chapter filters are rerun too.
The unfiltered spec suite remains coordinator-owned.

Earlier requested integration matrix, 2026-11-14: **READY against fetched
`origin/main` at `cc26b8d9`**. Every requested fresh gate passes. The full
`review-k4d` filter checks 365 projects with zero failures; all fourteen
named-provider projects match the landed error multisets exactly. One
additional equality observation records the W0503 warnings required by
spec §6.9 for stage three's distinct nominal types (§2.8), with unchanged
acceptance and runtime. The landed-tip integration and audit below
supersede earlier READY results and interim blockers.

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
  detached and attached states (`SchedulerService(state : SchedulerState)`).
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
With five members, or with `R { R(read : (Int) -> Int ^ E) }`, it checks.

## Host ABI

`Bool = True | False` reaches the host as before: a host export
`(Int) -> Bool` has `"result":"bool"` in `ilang.abi`, an error `^ Bool` has
`"errors":["bool"]`, and the JavaScript bindings give `true`/`false`.

## Behaviour changes seen in the tests

- A refutable binding of an unknown value narrows it: `first = (r) { Ok(v) = r  v }`
  is `(r : Ok(A)) -> A` with no `Assert`, not `(r : Result(A, B)) -> A ! Assert`
  (lane js's D4 rule widens to `Q | r - Q` only when another constructor's
  value is known). Fixtures that pass an `Error` give the parameter the type
  `Result(…)` now (`tests/spec/9/assert.i`, `crates/ilang-check/tests/codegen.rs`);
  a recursive top-level value needs a written type on the part
  (`tests/spec/7/may_fail_group_ctor.i`).
- Types are more precise: `named : (r : Result(A, B)) -> Ok(A | 0)`,
  `not_known : (s : Shape) -> Circle`; a constructor's type is its own
  (`Some : (A) -> Some(A)`).
- A match that does not cover a member of the subject's sum gives E0516
  alone, as before (a former constructor that no pattern takes is held for
  the coverage error when a pattern takes another member of a declared union
  with it; `Checker::uncovered_member`). An earlier state of this lane also
  gave E0501 (`Point` is not a subtype of `Circle | Square`, corpus fuzz-5:
  `True` is not a subtype of `False`); that was a bug against §5.2.8 rule 11
  ("the coverage error in a match is unchanged").
- `Circle(..v, …)` with `v : Shape` gives E0501, not E0517's message.
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
  case `closed : (Result(Int, String)) -> Int = (Ok(v)) { v }`, E0501).
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
  "Labels are part of the type". Thus `(x : Float)` becomes `(Float)`;
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

The first snippet run found the READY alias regression's replacement text
ending in `with_int`. Its inventory entry supplies the `Reader` prefix
and `with_int` tail from `types/soundness.i`. It is parsed normally, with
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
records one, two, and five errors for the original three cases. The checker
regression pins those three; a CLI provider regression checks all fourteen
changed projects, including their real manifests and macro expansion.
Together they cover sixteen distinct cases. Each independently pins its
total errors and E0515 count, then compares every complete error message
and location.
The fourteen source programs and expected observations are verified
byte-for-byte identical to the fetched tip. No observation is re-recorded.
The workspace check and fresh gate matrix are in progress; the old READY
claim is not reused for this integration.

The workspace all-target check passes at merge checkpoint `b7d8c17a0`.
The first row suite passes 31 tests; its expanded guard reaches a project
that imports `std/test`, where the in-memory checker helper has no std
dependency or macro expansion. It therefore reports E0209/E0234 rather
than the project's expected provider errors. Move the fourteen-project
guard to CLI providers tests, retaining every pin and complete diagnostic
comparison; keep the original three-case checker guard. No expected error
is changed to accommodate the missing test context. The fresh matrix is
rerunning this focused coverage before its remaining gates.

Focused verification now passes: all 32 row tests, including both sides'
unchanged alias/named-helper regressions and the corrected 1/2/5 guard;
the CLI guard checks all fourteen actual projects and matches every landed
error message, location, total, and E0515 count. No stage-3 error deficit or
different diagnostic remains among those projects. No additional checker
implementation change is needed after importing main's revalidation fix.
The remaining package/runtime/spec/corpus gates are still in progress.

Reviewing the renamed snippet gate also identifies the exact registered
fragment as `"\nIntReader = Reader(Int)\nwith_int"`, rather than its search
needle. Its completion now applies the test's following annotation
replacement too. The assembled fragment is verified byte-for-byte to be
the actual constructed alias program's prefix. This changes no parser
diagnostic expectation, exemption, or checker assertion.

The Rust matrix passes: checker 362 (four existing ignores and the requested
recovery skip), syntax 190, LSP 106, macro 9, Wasm 65; CLI check 10,
commands 14, macro_run 57, providers 19, typed_macros 36. Spec filters
`4.3/`, `6/`, `7/`, `9/`, `15.7/` pass 17/80/104/139/28 cases at O0 and O2,
with the existing pending entries unchanged. Short generation passes 32
cases, 64 permutations and 32 annotation removals. Fresh release, Rust/i
formatting, all-feature workspace clippy and the final workspace check pass.
The renamed `scripts/check-snippets` passes 4,817 sources plus 34 Lisp
files and 89 macro fragments; all eight helper tests pass. The corrected
alias-fragment context is included in that result. Std O0/O2 and the full
365-case `review-k4d` filter remain in progress; this is not yet READY.

Both fresh std runs now pass 2,170 tests each at O0 and O2. The remaining
gate is the complete requested `review-k4d` selection: 365 projects, two
jobs, with zero failures required. All verification commands remain serial
and use the shared wrapper; the snippet gate uses its internal wrapped
cargo build. No source or observation is changed during this final run.

### Required equality warnings on the landed-tip integration

The first complete `review-k4d` run checks 365 projects (213 accepted,
152 rejected); 364 observations match and one differs only in warning
messages. `review-k4d/_loose/spec/tests_spec_11_equality` gains W0503 at
`test/equality.i:57:5`, `59:5`, and `62:5`, in check, O0 and O2. The pairs
are `Some(1)` / `None`, `Ok(1)` / `Error(1)`, and `True` / `False`.
Syntax-three §2.8 makes each data definition its own nominal type and its
constructor return that type. These pairs are now known disjoint types;
spec §6.9 requires W0503 when both known types are disjoint (§4.5 rule 3).
Suppressing these warnings would violate that paragraph.

Only those nine warning lines are added to the saved observation; all
exits, stdout, commands, bounds, source programs and existing warnings are
unchanged. Each runtime phase still passes all five equality tests. The
record's compiler/hash provenance is refreshed for this observed binary.
The focused case passes after the update. This preserves and strengthens
the equality test's intent; no error is removed or weakened. All fourteen
named-provider records remain byte-identical to fetched main. The complete
first-run snapshot is `s3-r4-corpus-first-outcomes.json` in the shared
coordinator cache; its raw log is `s3-r4-corpus.log`, and the focused retry
is `s3-r4-equality.log`. A second complete filter run is required before
READY.

For the strict audit classification this is **one diagnostic outcome
difference**, since the W0503 count increases, with **zero acceptance or
runtime outcome changes**. There are zero other message or shown-type
differences in this filter. The controlling §6.9 paragraph is quoted in
commit `7a7d7df01`; the machine classification is
`s3-r4-corpus-classification.json`. The historical shard classifications
above apply to their earlier baselines.

### Final landed-tip gates

The second **complete** `review-k4d` run passes: 365 selected projects,
213 accepted checks, 152 rejected checks, zero failures. Its command is
`python3 scripts/qa/corpus.py run --filter review-k4d --jobs 2`, through the
shared heavy wrapper; log `s3-r4-corpus-final.log`. The focused equality
retry also passes. No runtime retry or changed bound is needed in this
final complete run. All fourteen named-provider source/expected pairs are
verified byte-identical to fetched `cc26b8d9` again after completion.

The first run's sole difference is classified strictly as one diagnostic
outcome change (three new W0503 warnings in each check/O0/O2 phase), with
zero acceptance/runtime changes and zero other message/shown-type changes.
Spec §6.9 requires the warnings under syntax-three §2.8; its complete
paragraph is quoted in `7a7d7df01`. The first-run snapshot retains all saved
expected records and the sole mismatching actual observation; matching
cases have no `actual.json` by runner design. The initial sparse-snapshot
audit helper was corrected for that format. The machine classification is
`s3-r4-corpus-classification.json`.

All other requested gates remain green on this compiler: workspace check;
checker/syntax/LSP/Wasm/macro and all five CLI binaries; all five requested
spec filters at O0/O2; std O0/O2 (2,170 each); snippets and helper tests;
short generation; Rust/i formatting; workspace all-feature clippy. The
full command/result ledger and initial resolved failures are in shared
cache `report-s3-r4.md`, with `s3-r4-rust-results.json`,
`s3-r4-post-results.json`, and `s3-r4-final-results.json`. Only documentation
and the explained equality observation changed after those compiler gates.
The requested recovery skip, four existing ignores, existing spec pending
entries and inherited record-field solver limitation remain as documented.
No required lane gate remains unfinished; main landing is coordinator-owned.

## Package interface comparison after independent constructor types

The coordinator's integration gate finds `publish_compares_the_interface`
expecting a change to constructors of `Shape`. Syntax-three §2.8 and spec
§4.3 make `Shape = Circle | Square | Dot` a transparent union alias, and
`Dot = ()` a separate nominal type with its own constructor. Spec §2.4
makes `Shape(..)` export that alias and each member with its constructor.
The specified four interface items therefore are changed `type api.Shape`,
changed `value api.add`, added `constructors api.Dot`, and added
`type api.Dot`. The assertion now pins all six exact lines, including both
before/after types. Spec §2.7 requires a newer version in one major to take
the place of an older one; the function-arity and union-alias changes
preserve the test's major-version rejection.

Keep every existing major-version and tag assertion. The later hiding step
compares with published v1.1.0, whose `Shape(..)` exported Circle and Square,
so pin removal of those two constructors individually. Dot was never
published, so it cannot be a removed constructor. This preserves the
constructor-visibility intent under the new type definitions. No comparison
implementation or language behavior changes. The controlling paragraphs
are quoted in the commit message. The corrected snippet context and all
fourteen provider records remain unchanged.

The first whole-package run passes the new exact four-item assertion and
26 other tests, then rejects the old hiding fixture before interface
comparison: bare `Shape` leaves its three member types private (E0227).
Spec §2.4 requires the types used by an exported alias to be exported too.
The fixture now exports `Shape, Circle, Square, Dot`, each without `(..)`,
so only the constructors are hidden and the removed-value/interface/tag
checks remain meaningful. The resolver's E0227 is retained. A complete
whole-package retry follows; no expectation is changed to accept E0227.
