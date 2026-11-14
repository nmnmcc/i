# E3–E6 corpus integration audit

Base: main `1bdea69406471b72e6d1df3a5592c075688478b8`, merged through
`gate/g23` (`bddbb4b59`). The coordinator's 1,973-case run classified 1,009
observations as fingerprint-only, 919 as display-only, and 45 otherwise.
Sixteen of the 45 have resource timeouts; the remaining 29 are listed below.
The timeout reruns retain each saved `bound_seconds`; no bound is raised.

## Owner rulings, 2026-11-14

The merge base for the new gate is main `9b8061137`, merged at `e357d7449`.
The historical table below is being re-evaluated against the owner decisions in
`decisions-2026-11-14.md`; its R3/R4 outcomes are not approved new baselines.
Decision 1: “A need is a set of environments (EF01/T5/F7).”
Decision 2: “Drop the Fail(Never) universal-entry removal (EF03/T4).”
Spec §9.2's safety promise remains: “Getting it never fails at run time.”
A greatest form within one constructor does not supply that constructor's key.
The solver and printed types now retain all such entries, including explicitly
written `Fail(Never)`, contravariant `Sink(Never)` and unconstrained unused `S`.
The two `review-k4s/min_main_{3,5}_0` entries must again reject a missing `S`;
the old R4 acceptance justification is withdrawn. The revised unit regression
keeps both unused cases and the used-head control. Three §9.2 fixtures check
missing/provided `Sink(Never)` and a pure function missing `Fail(Never)`.
The wide-row and `review-std2/unsafe` display judgments that erased keys also
need re-recording only after the new full corpus run. Verification is pending.

Decision 1 also requires judging the intersection of the entire service value,
not rejecting an empty covariant argument alone. Spec §4.4 intersects the
arguments of one constructor; §11.7 says “The program stops” when “`panic(m)`
is called”. Thus `Read(A) = (read = () -> A)` admits a `Read(Never)` whose
method stops. The checker now applies its ordinary coverage/inhabitance test
to a provisional empty-argument meet before reporting E0515. A service with a
required value field of type `Never` still gives E0515. Provider-subtyping
failures and invariant-argument errors are independently retained. New unit
and §9.2 runtime regressions pin both sides; merged verification is pending.

Decision 1 / EF09 removes the exclusive ordinary/row-variable classifier.
A variable may occur in a `Boundary` argument, an ordinary parameter type and a
need. The old mixed-use rejection test now retains its source as an acceptance
test and separately checks the surviving §9.2 rule: “A provided item is keyed
by its known type … which must be one named service type: not a union, a type
variable, or unknown.” The new §9.4 fixture also checks the published control
signature structure. This does not yet redefine the written §9.4 restrictions
on captured-row operations; the broader ordinary-environment model and catch
migration scope have been queried. R3's old exclusive-kind paragraph is no
longer a valid justification for any corpus rejection.

The stored-callback audit found and repairs an additional R3-dependent defect:
the port made `Stream` and `Sink` covariant in their residual need and made the
stream error label phantom. Restore main's stream signatures and ordinary
negative markers. Std-1 §3.1: “A/E are covariant; N is invariant.” Std-2 §6.3 L5:
“E is covariant; N is invariant through the unused `(N)->Nil` marker”. The new
mixed-variable ruling allows this original representation again. The vendored
stream source and its four error-label annotations follow the same restoration;
every test assertion and algorithm is unchanged. A checker regression reads
the actual shipped declarations and checks error covariance plus both negative
need-coercion cases. The E4 Result-data choices remain unchanged.

The final-AST main merge is `5e08939f0` (fa `35f0e0588`). Production
resolves the native tree; E3's single need and E6's rest/provide metadata now
live in its prepass and direct resolver. The old AST remains only in main's
dev-only migration oracle, whose E3/E6 assertions remain exercised. The
snippet inventory is the union, and main's three changed mixed-install
records remain the baseline until the full effects corpus is observed.

Decision 2 exposed five library errors in the first row-scheme run, not 41
independent case defects: all checked programs also check `i/task`. §9.5
writes `resource.ensure` as `<A, N>(() <- N -> A, () <- N -> Nil) <- N -> A`;
restore that exact signature instead of manufacturing `Fail(Never)` for a
pure body. §9.4's written `keep` type still has `Fail(E)` on its result, so
internal closed scheduler callbacks supply an actual `Fail(Never)` value,
whose argument is uninhabited. They do not erase a key or retain a caller's
handler. No final is installed or run on a stop (decision 15). The focused
row run completed in 143 seconds; the attempted process stop found it already
finished. Rest-service regressions passed 13/13 before the final-AST merge;
the merged workspace all-targets check is green, with runtime gates pending.

Fresh std checking found three conversion issues and now passes after their
repair. Main's `redirect_headers` takes and returns `AnyHeaders`, preserving
raw byte values; restore those types. Std-1 §3.1's written signatures give
`fold`, `fold_until`, and `each` independent callback error `F`, with result
errors `E | F`; the restored Stream representation had lost those explicit
Fail heads, so put them back and keep every callback-error assertion. The two
map-environment runner tests now state the genuine `Fail(Never)` callback
need (Decision 2), with all pull/replay assertions unchanged. Finally state
the concrete stdout/stderr sink result, including Stdio, Await and Owner,
to prevent an unresolved callback row from recursively swallowing its own
Sink requirement (std-2 §7: finite, closed instantiations). The vendored copy
receives the same migration. The release build, workspace check and std
check are green; full runtime gates are next.

The first final-AST row run passed 45/47 tests. Its two failures were a stale
total diagnostic count (main already includes the extra duplicate-key error)
and handler-hole display: a removed entry denotes the supplied value, so
coalescing its arguments must follow ordinary value variance, not required
service variance (§9.2). Keep the original exact-witness equality assertion
and repair the display walk. Restore main's separate four-site provider
regression inadvertently omitted during conflict resolution; every E0515
location remains pinned. The fourteen-project guard now starts with main's
actual total counts while retaining all independently pinned provider counts.

Decision 2 also requires removing conversion-only ghost keys from fixtures.
`73f725713^` shows the captured-row coalescing source wrote `() <- N -> Nil`;
that commit inserted `Fail(Never)` only to prevent the old lowerer inventing
`N%E`. Restore the original annotation now that E3 removes that carrier. The
spec-example generator likewise originally wrote only Yield and N (before
`947cc442d`), so its pure callback does not gain a new Fail service. This
does not erase any actual written key: genuine `Fail(Never)` requirements
remain in the missing-provider and returned-callback regressions.

Decision 1 makes the two original covariant Sink-error meet sources valid:
`Sink(Never, Never)` contains a callback and is inhabited. Retain both sources
as positive cases; their negative controls add a required E value field,
whose Never intersection is empty (§4.1). Preserve the original invariant
need-coercion rejection. The display-error fixture similarly uses a required
State value field; the bottom-returning-method positive remains independently
covered. These changes preserve each negative test's incompatibility intent.

The first gate-equivalent workspace run completed 118 binaries: 101 passed
and 17 failed. Every failure is retained in `target/e4-ptest-logs` for the
subsequent fixes/reruns; this checkpoint is not a green landing claim. A
revised `run` captures its caller's residual N explicitly and passes it to
the scheduled body. Thus actual Fail entries come from the caller, while
internal completion is `Result(Result(A, Never), ScheduleError)` and does not
manufacture an outward Fail need. `execute` supplies the genuine Fail(Never)
needed by its pure kept bookkeeping callback. This implements the exact
§9.9 run contract under Decision 2 instead of relying on erasure.

Fresh checks confirm `only_runner` has exactly Runner and the test wrapper
has exactly Fail(ScheduleError) and Runner. Native R2 passes all nine tests,
including accounting, ancestor cancellation/finals and 200 seeded replays at
each optimization level. The original events program prints
`app/main.main : () <- (Assert, Fail(Never), task.Runner) -> Nil`: Scheduler is
removed, while its genuine pure-event Fail key remains under Decision 2. The
CLI test pins that exact type. Callback fixtures supply real Fail(Never) keys
where a closed function is required, or state the key on a boundary/callback
annotation; every prior cancellation, ordering, error and portability
assertion is unchanged. The R2 export helper likewise installs that real key
instead of its old NoFail filter. The generator/catch fixture drops only its
proven conversion ghost key. The remaining old three-argument Boundary use
becomes the same two-argument Boundary under E3.

Owner decision 9, “Finals may raise”, changes the old final-error rejection
fixture into an execution test asserting the identical Error("final") through
fail.catch. Decision 15 still forbids running finals on a stop; no stop
handler or stop-final path is added. The first full spec run passed its stop
fixtures. Full reruns of the changed fixtures remain pending. Required i
formatting is applied to the 30 reported files without changing assertions.

The fresh fourteen-project probe retains every concrete provider-subtyping
failure. In each `b1` copy only the summary at `6:1` disappears. This is a
Decision 1 / §4.1 correction, not a lost provider check: “A function field
requires a function value, not a value of its result type; a function that
stops or diverges can inhabit `() -> Never`.” For `S(A,N) = (() <- N -> A)`,
the intersection of the two S forms can therefore be inhabited. §9.2 only
requires E0515 “When the intersection for one service of a need is `Never`
while neither side is `Never`”. All four concrete E0515 locations (`6:11`,
`12:12`, `12:21`, `15:8`) and the duplicate-key E0225 remain. A two-parameter
positive regression uses this exact constructor shape, with a required-value
field negative control. An inhabited meet still cannot supply a missing key
(Decision 2). The fresh m32 probe adds the specified duplicate-key E0225 and
retains all eight E0515 plus E0501; the other eleven projects retain their
full landed diagnostic sets. Recording and corpus verification are pending.

The exact annotation-hole witness regression also passes after a display
repair. The trace proved the local hole has both lower and upper bound Int;
outer polarity simplification had incorrectly substituted Never inside its
removed-provider mask. §4.5 says “Two types are equal when each is a subtype
of the other.” Resolve that proven closed equality in the mask before
simplifying, leaving one-sided covariant bounds open. All eight focused
handler tests pass, including the original exact equality assertion and a
new negative control: Svc(Int) cannot provide Svc(String) through Svc(_).
The ordinary value-difference display is unchanged. The environment golden
retains its empty-meet error by giving Source a required A value field
(§4.1, R8); function-only inhabited meets retain independent positive tests.
Both std levels pass 2,171 tests each. Full Rust and corpus gates follow.

## Controlling paragraphs

**R1 — E3, ordinary error values.** Syntax-three §6.1: “**One need row.**
`err` is removed (stage E3): errors are the `Fail(E)` entry of `env`.”
Spec §9.3: “An error is raised through the prelude service `Fail(E)`”, with
the definition `Fail(E) = (raise = (E) -> Never)`. The argument of this
ordinary declared type is an ordinary value type, not a second row kind.
There is no separately generalized or constrained error carrier.

**R2 — Duplicate written keys.** Spec §9.2, Providing: “An explicit item
wins over an item from a spread with the same key; two explicit items, or
two spreads, with one key are a compile error.” This applies also when the
key is not used by the body. Three explicit items of one key give two
duplicate sites; a second duplicated key gives its own site.

**R3 — Mixed variable use; captured-row operations.** Owner decision 1
(EF01/EF09): “A need is a set of environments.” The exclusive classification
in the old §4.7 sentence is dropped. One parameter may occur in a need and
an ordinary type position. Binding a value to another parameter of that same
row type is forwarding; the checker uses the actual formal type for that
permission, never an F1 structural hint. The four `mixed_recursive` sources
are acceptance regressions again, retaining their `S` needs. The written
§9.4 operation restrictions remain: “A value of a row type may be bound,
captured and provided after `<-`, alone or as a spread in provided data; it
may not be spread into ordinary data, compared, printed or matched.” Existing
negative tests for those operations remain. The full ordinary-environment
model and Decision 8 migration scope have been queried separately; no new
public environment type or syntax is invented by this repair.

**R4 — Universal removal withdrawn.** Owner decision 2 (EF03/T4): “Drop
the Fail(Never) universal-entry removal.” This overrides the former §9.2
removed-entry sentence. The section's safety promise still says: “Type
checking makes certain that every service is there when a call takes it.
Getting it never fails at run time.” Simplifying arguments cannot supply a
missing constructor key. Both unused `min_main` cases therefore retain `S`
and reject at an entry again. Explicit `Fail(Never)` and `Consumer(Never)`
keys remain real needs, and the previous D9 display justification is withdrawn.
The two old unit tests that accepted an unused `Fail(E)` without a provider
now retain those original sources as rejection cases, alongside the real-raise
controls. Positive controls supply an actual `Fail(Never)` or use a genuinely
service-free callback. The filtered-callback spec fixture also checks that
`N - Fail(Never)` cannot satisfy an unused Fail key. This rewrites tests of the
dropped removal rule under Decision 2; no negative case or assertion is lost.

**R5 — Removed channel obligations.** R1 removes the extra error-channel
constraint, not an incompatible service constraint. Spec §9.2 still says:
“A provided value of type `P` satisfies the need entry `S` when `P <: S`
(§4.5 rule 4).” Its preceding paragraph still requires E0515 for an empty
service intersection. The `b10` recursive definition retains E0505 and its
provider retains E0515; the secondary cycle through the deleted carrier
does not exist. The `z3`/`z5` concrete provider errors all remain; the
additional summary derived from that carrier disappears. The earlier
[E6 audit](e6-main-corpus-audit.md) describes their exact remaining sites.
`b10` and `z3` explicitly write `Fail(Never)`: the key is now retained
under the revised R4. Their former R4-dependent diagnostic-count judgment
is withdrawn pending the fresh probe; an old error cannot disappear merely
because its key used to be erased. `z5` writes only
the callback's opaque `N`; its extra carrier was implicit in the old
representation and is removed by R1. Its three concrete provider errors
remain independently checked.
In `21_135`, the local provide of `T` surrounds calls of `S` and `Dep`, not
`T`; the removed carrier previously made a spurious hidden `T` obligation.
Its inferred `main` still needs `Dep`, `S` and `T`, and the platform must
reject all three before entry execution (R6).

**R6 — E5 platform entries.** Syntax-three §2.10.11: “The need of an entry,
after what the entry's own code provides, may only contain platform services
of the target. Otherwise it is the compile error ‘`T` is not provided by
the target `t`’.” Its platform-services paragraph says: “All other services
are provided in `main` by i code.” Stage E5 explicitly deletes the provider
plan. `21_160` and `21_243` used to stop at the planner's one aggregate
‘service A - app/main.Dep must be closed’ message; they now report the
distinct missing `S` and `T`. These are two different keys, not two copies
of one diagnostic. Their checker phase remains accepted and every entry
phase remains rejected.

**R7 — Keeping vendored suite provisions.** Spec §10.2: “`suite` prefixes
names and provides the services written in it to its test bodies; `each`
contributes one test for each written case.” The vendored collector must
retain complete grouped environment nodes, their temporary bindings and
block scopes, as the current standard-library collector does.

**R8 — Whole-service inhabitance.** Owner Decision 1: “A need is a set
of environments.” Spec §4.1: “A non-opaque labelled type is inhabited exactly
when its data is inhabited. Apply these rules after substituting its type
arguments.” Further: “A function field requires a function value, not a value
of its result type; a function that stops or diverges can inhabit `() -> Never`.”
Spec §9.2 reports E0515 only “When the intersection for one service of a need
is `Never` while neither side is `Never`”. Thus an empty covariant result
argument alone does not make a service with only methods empty. This makes
the conjunction valid, but does not make the unrelated invalid providers valid;
their subtyping errors remain. A required value field of type Never still
makes the whole meet empty and still reports E0515.

## Non-timeout cases

Category **a** is required by the quoted landed rules. Category **c** is a
stale vendored source migration. No expectation is justified merely by the
new compiler's output. The table records diagnostic multiplicity as well
as acceptance. The source migration of `alias_kind_error` follows the shared
rule that a test pinning a removed feature keeps its original intent.

| Case | Old | New / retained intent | Category and paragraph |
|---|---|---|---|
| `review-k4-lane/alias_kind_env` | E0515 | E0515 + E0225: returning the row as ordinary data | a, R3 |
| `review-k4-lane/alias_kind_error` | E0515 on the old error-row alias | `Fail(N)` correctly has an ordinary argument after E3. Migrate `Row(N)` to `() <- N -> Nil`, keeping the original negative row-kind test and its E0515 requirement; it also reports the R3 row-value error. The ordinary `Fail(N)` form is a positive compiler regression. | a, R1/R3; source migration |
| `review-k4d/_loose/fuzz/e0505/21_135` | checker E0515 | checker accepts the unused local `T` provide; entry still rejects missing `Dep`, `S`, `T` | a, R1/R5/R6 |
| `review-k4d/_loose/fuzz/e0505/21_145` | E0501 | E0225 + the same E0501; two explicit `S` values | a, R2 |
| `review-k4d/_loose/fuzz/e0505/21_160` | one aggregate E0523 | two E0523, for `S` and `T`; all entry phases still reject | a, R6 |
| `review-k4d/_loose/fuzz/e0505/21_243` | one aggregate E0523 | two E0523, for `S` and `T`; all entry phases still reject | a, R6 |
| `review-k4d/_loose/p/b1` | E0515 ×5 | four concrete E0515 + E0225; the method-only meet is inhabited, so remove only summary `6:1` | a, R2 and R8 |
| `review-k4d/proj/b1` | E0515 ×5 | four concrete E0515 + E0225; remove only the inhabited-meet summary `6:1` | a, R2 and R8 |
| `review-k4d/_loose/p/b10` | E0505 ×2 + E0515 | E0505 + E0515; recursive definition and incompatible provider both remain | a, R1/R5 |
| `review-k4d/proj/b10` | E0505 ×2 + E0515 | E0505 + E0515; recursive definition and incompatible provider both remain | a, R1/R5 |
| `review-k4d/_loose/p/r6` | E0515 ×3 | same three + E0225 for duplicate explicit `S` | a, R2 |
| `review-k4d/proj/r6` | E0515 ×3 | same three + E0225 for duplicate explicit `S` | a, R2 |
| `review-k4d/_loose/p/z3` | E0515 ×3 | two concrete E0515; obsolete carrier summary removed | a, R1/R5 |
| `review-k4d/proj/z3` | E0515 ×3 | two concrete E0515; obsolete carrier summary removed | a, R1/R5 |
| `review-k4d/_loose/p/z5` | E0515 ×4 | three concrete E0515; obsolete carrier summary removed | a, R1/R5 |
| `review-k4d/proj/z5` | E0515 ×4 | three concrete E0515; obsolete carrier summary removed | a, R1/R5 |
| `review-k4d/fuzz/m32` | E0501 + E0515 ×8 | same errors + E0225 | a, R2 |
| `review-k4d/fuzz/pl0` | E0501 ×3 | same three + E0225 for two explicit `Cov` values | a, R2 |
| `review-k4d/fuzz/pl41` | E0501 ×3 + E0515 | same errors + E0225 ×3: three explicit `Cov`, two explicit `Svc` | a, R2 |
| `review-k4d/fuzz/pl42` | E0501 ×3 | same errors + E0225 ×2: three explicit `Svc` | a, R2 |
| `review-k4d/fuzz/pl43` | E0501 ×2 | same errors + E0225: two explicit `Cov` | a, R2 |
| `review-k4s/min_main_3_0` | entry E0523 | retain the required `S` key; entry rejects again (fresh verification pending) | b, universal-erasure defect fixed; R4 / Decision 2 and §9.2 safety |
| `review-k4s/min_main_5_0` | entry E0523, W0504 | retain `S`, E0523 and the original warning (fresh verification pending) | b, universal-erasure defect fixed; R4 / Decision 2 and §9.2 safety |
| `review-k4s/mixed_recursive_0_0` | accepted, W0504 | forwarding a value to the same row-typed formal accepts again; retain `S` need (fresh verification pending) | b, overbroad mixed-use rejection repaired; R3 / Decision 1 and §9.4 bound values |
| `review-k4s/mixed_recursive_0_17` | accepted, W0504 | forwarding a value to the same row-typed formal accepts again; retain `S` need (fresh verification pending) | b, overbroad mixed-use rejection repaired; R3 / Decision 1 and §9.4 bound values |
| `review-k4s/mixed_recursive_1_0` | accepted, W0504 | forwarding a value to the same row-typed formal accepts again; retain `S` need (fresh verification pending) | b, overbroad mixed-use rejection repaired; R3 / Decision 1 and §9.4 bound values |
| `review-k4s/mixed_recursive_1_17` | accepted, W0504 | forwarding a value to the same row-typed formal accepts again; retain `S` need (fresh verification pending) | b, overbroad mixed-use rejection repaired; R3 / Decision 1 and §9.4 bound values |
| `review-k4s/std` | checker accepts; entry tests resource-censored | restore main’s stream signatures, stream test type annotations and ordinary negative `Sink` marker under Decision 1; preserve E covariance and N invariance, explicit message `Fail(F)`, and grouped suite collection. Every test body, error assertion and saved bound remains. | c, R1/R7; std-1 §3.1 and std-2 L5 variance contracts |
| `review-std2/replacement_unused` | accepted, W0504 | E0225: two explicit `S` items remain invalid even when unused | a, R2 |

## Corpus runner repair

The existing `b10` requirements separately require E0505 and E0515. The
runner unconditionally excluded E0505 even when explicitly required, making
both requirements impossible to satisfy. Allow a cycle only when this case
has an explicit cycle requirement; still require the independent provider
error, and continue rejecting an unexpected cycle in every other case.
Five Python regressions cover both errors, either missing error, unexpected
cycles, and warning-only output. No semantic requirement is removed.

## Library defect found while uncensoring a native case

`review-native3/_loose/design-cases/events` completes with the optimizer
thread limit, but exposes a separate **b** defect. Its inferred main need
changes from `(Assert, task.Runner)` to `(Assert, task.Runner,
task.Scheduler)`. This is not justified by the published library contract.
Spec §9.9 gives `run` the type
`<A, N>(() <- (..N, Scheduler, Runner, request.Await, request.Owner, request.Root) -> A) <- (..N, Runner) -> A`:
the callback's Scheduler is supplied inside run, not required from its
caller. E6's `make_start` annotation now propagates opaque N to its caller,
and the current `run` implementation leaks Scheduler through it.
`review-r2/never_yields` exposes the same defect through `task.test`: its
main gains Scheduler beside the existing Fail(ScheduleError), Io and Runner.
`review-r2/determinism`, one of the original 919 display cases, has the same
extra Scheduler need. Its apparent display change is **b**, not an allowed
simplification: the baseline is restored pending this repair.
The adjacent §9.9 `test` signature likewise lists Scheduler, Runner, Await,
Owner and Root among the services provided to the callback, leaving residual
N, Fail(E | ScheduleError) and Runner at the caller. All three old baselines
are retained. Events and never_yields belonged to the original 16 timeout
cases; determinism belonged to the original 919 display-only cases.

An isolated checker probe confirms the contract failure (a proposed positive
regression, currently failing):

```text
(i/task)
only_runner: (() <- (task.Scheduler) -> Nil) <- (task.Runner) -> Nil =
    (body) <- (runner: task.Runner) { task.run(body) }
```

Before the repair this rejected contrary to §9.9 with E0501,
`task.Scheduler is not a subtype of task.Runner`. The corresponding test
wrapper also rejects with E0501 against Fail(ScheduleError) | Runner.
The coordinator authorized the library repair on 2026-11-14. The four-signature
repair is now applied to `lib/i/task.i`, retaining main's stage-three Runner and
Scheduler representations. Three §9.9 fixtures check the Runner-only run/test
contracts, rejection of an unrelated unprovided `Dep`, and the actual events
program. A CLI regression checks the latter's printed entry type exactly:
`app/main.main : () <- (Assert, Fail(Never), task.Runner) -> Nil`
(Decision 2 retains the genuine pure-event Fail key).
The original [proposal](effects-e3-e6-task-run.patch) is retained as an audit
artifact. Verification of the merged compiler and these regressions is pending
the heavy runs after 20:30; the older results below describe the previous hash.

## Verification and display groups

The release rebuild passes. All 42 checker row-scheme tests pass, including
four new regressions for the ordinary Fail argument, unused universal need
forms with a used-service negative control, exact distinct entry-service
errors, and preservation of the vendored marker's invariant row.
All five corpus-runner regression tests pass. Changed i files and Rust test
formatting pass.

Commands used for this audit (H is `/home/agent/Programming/i/.rd/cache/heavy`):

| Command | Result |
|---|---|
| `CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 $H direnv exec /home/agent/Programming/i cargo build --release -p ilang-cli --locked` | pass |
| `CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 $H direnv exec /home/agent/Programming/i cargo test --release -p ilang-check --test row_schemes --locked -- --test-threads 2` | 42 pass |
| `python3 -I scripts/qa/corpus-test` | 5 pass |
| `python3 -m py_compile scripts/qa/corpus.py` | pass |
| `direnv exec /home/agent/Programming/i rustfmt --check --edition 2024 crates/ilang-check/tests/row_schemes.rs` | pass |
| `./target/release/i fmt --check tests/review/cases/review-k4-lane/alias_kind_error/src/main.i tests/review/cases/review-k4s/std/src/log.i tests/review/cases/review-k4s/std/test/stream_test.i` | pass |
| `git diff --check` | pass; patch context whitespace was corrected using a zero-context artifact |
| `git apply --check --unidiff-zero docs/design/effects-e3-e6-task-run.patch` | pass; unapplied proposal only |
| `./target/release/i fmt --check target/e4-task-run-candidate.i target/e4-proposed-regressions` | pass |
| `prlimit --cpu=10:11 ./target/release/i check --offline --project target/e4-task-contract-actual --types` | rejects with two E0501; confirms unfixed run/test contract |
| `prlimit --cpu=10:11 ./target/release/i check --offline --project target/e4-task-run-contract --types` | copied module: run/test and Dep-present positives pass; Dep-absent control rejects E0501; privileged imports require the documented type-only stubs |

Corpus updates use `UPDATE=1 python3 scripts/qa/corpus.py run --binary
./target/release/i --case-list GROUP.json --filter PREFIX --jobs JOBS
--keep-bounds`, with all groups serialized inside one H invocation. Semantic
groups use the 29 cases above; original timeout groups use their 16 cases,
one worker at a time. Display groups use four workers, except scheduler
groups use one. The case inventory below gives the selected identities.
The prefixes are `review-k4-lane/`, `review-k4/`, `review-k4c/`,
`review-k4d/`, `review-k4s/`, `review-native3/`, `review-num/`, `review-r2/`,
`review-std2/`, `review-t1/`, and the four additional runtime-message probes
include `review-js/`. A second pass of resource-censored cases uses a
one-element case list, the complete case ID as filter, and one worker.
No saved CPU bound is raised. Each completed observation is additionally
checked for phase sets, exits, diagnostic multiplicity, and runtime stdout;
fingerprint-only updates are restored byte for byte from the base.

All 29 non-timeout cases pass the bounded per-group update runs and an
additional audit of their exact required error multisets and entry results.
The first vendored rerun exposed four remaining stale stream annotations;
only their written Stream/Step need arguments were corrected to include
Fail(E), as in the current std tests. Every one of that file's 94 assertions
and every string literal is unchanged. The migrated vendored package now
checks successfully; its original O0/O2 resource-censored test outcomes
remain at the same 10-second bound. The suite collector produces 1,446
generated test bindings both before and after migration.

All 919 original display-only cases have complete bounded reruns, exact
diagnostic-multiset/runtime-output audits, and one-line explanations below.
There are 918 justified display updates and one held library-contract
defect, determinism. No expectation is recorded for that defect.
All 1,009 original fingerprint-only files retain their original bytes.
Across all 1,973 records, every saved bound is unchanged. The original
16 timeout cases were rerun alone; 25 cases censored during those runs or
the display runs received a second isolated attempt with Binaryen limited
to one worker. Sixteen clear completely, six remain CPU-censored, and three
complete but expose the library-contract defect above. The original
baselines of those nine cases remain intact. No third diagnostic retry or
bound increase is made. The initial default-pool scheduler group returned
failure for CPU-censored accept requirements; its completed results were
audited individually rather than treating UPDATE mode's exit as proof of
successful execution. The full requested integration is not green until
the library owner repairs the contract and the six remaining resource
cases can be verified at the same bounds.

<!-- resource-audit -->

## Resource reruns

All 16 cases identified as timeouts in the coordinator run were rerun
alone, at the same saved CPU bounds. Additional censored cases from
the display runs were retried alone as well. Binaryen defaults to a
worker for every available core, independently of RAYON_NUM_THREADS
(`crates/ilang-binaryen/binaryen/src/support/threads.cpp:156-176`).
The final isolated retry sets BINARYEN_CORES=1; no CPU limit,
optimization level, source assertion or semantic requirement changes.
CORPUS_BOUND_SCALE is unset (1). The 10-second CPU limit remains
`prlimit --cpu=10:11`, with the existing wall-time backstop.

CPU-censored cases after that retry: 6. The three library-contract
defects are listed separately. A censored phase
is never recorded as a new passing expectation.

| Case | Final same-bound result |
|---|---|
| `review-k4/perf/wide_2048` | all original exits preserved; fingerprint-only file restored |
| `review-native3/_loose/design-cases/borrowed-request` | test2: exit 1 -> timeout; old baseline retained |
| `review-native3/_loose/design-cases/events` | §9.9 run/test contract defect; original baseline retained; library-owner repair pending |
| `review-native3/_loose/design-cases/paths` | run2: exit 0 -> timeout; old baseline retained |
| `review-r2/cancel_send` | test2: exit 0 -> timeout; old baseline retained |
| `review-r2/dag_16` | all original exits preserved; fingerprint-only file restored |
| `review-r2/dag_20` | all original exits preserved; fingerprint-only file restored |
| `review-r2/deep_50` | all original exits preserved; fingerprint-only file restored |
| `review-r2/join_failure` | all original exits preserved; fingerprint-only file restored |
| `review-r2/lifetime` | test2: exit 0 -> timeout; old baseline retained |
| `review-r2/never_yields` | §9.9 run/test contract defect; original baseline retained; library-owner repair pending |
| `review-r2/senders_128` | all original exits preserved; task.Scope(E) → task.Group(E), syntax-three §2.10.9 (D8) |
| `review-r2/senders_16` | all original exits preserved; task.Scope(E) → task.Group(E), syntax-three §2.10.9 (D8) |
| `review-r2/shield_nested` | test2: exit 0 -> timeout; old baseline retained |
| `review-r2/tasks_5000` | all original exits preserved; task.Scope(E) → task.Group(E), syntax-three §2.10.9 (D8) |
| `review-t1/stress-keys-2000` | test2: exit 0 -> timeout; old baseline retained |
| `review-k4-lane/w10-Dep` | all original exits preserved; shown-type change covered in the display inventory |
| `review-k4/cases/provision_helper_True` | all original exits preserved; shown-type change covered in the display inventory |
| `review-k4d/proj/q1` | all original exits preserved; shown-type change covered in the display inventory |
| `review-k4d/proj/q10` | all original exits preserved; shown-type change covered in the display inventory |
| `review-k4-lane/unused-True` | all original exits preserved; fingerprint-only file restored |
| `review-k4-lane/w14-Never` | all original exits preserved; fingerprint-only file restored |
| `review-k4-lane/w9-Dep` | all original exits preserved; fingerprint-only file restored |
| `review-r2/deadlock` | all original exits preserved; fingerprint-only file restored |
| `review-r2/determinism` | §9.9 run/test contract defect; original baseline retained; library-owner repair pending |
| `review-r2/nonportable_result` | all original exits preserved; fingerprint-only file restored |
| `review-r2/request_groups` | all original exits preserved; fingerprint-only file restored |
| `review-r2/senders_1000` | all original exits preserved; task.Scope(E) → task.Group(E), syntax-three §2.10.9 (D8) |
| `review-r2/senders_64` | all original exits preserved; task.Scope(E) → task.Group(E), syntax-three §2.10.9 (D8) |
| `review-r2/tasks_1000` | all original exits preserved; task.Scope(E) → task.Group(E), syntax-three §2.10.9 (D8) |

The vendored std case separately retains its pre-existing O0/O2 test
censors. Its successful checker and unchanged 1,446 generated tests
do not establish that those runtime tests finish within ten seconds.

<!-- display-inventory -->

## Original display inventory: 918 recorded and one held defect

Every listed observation has equal phase sets, phase exits, diagnostic code
multisets (including duplicate counts), and runtime stdout. Only the checker
types or diagnostic text change. Groups overlap. The original coordinator
run has 919 display-only cases; 918 justified updates are recorded after
complete bounded reruns; the remaining case, determinism, completed too but
is held as the §9.9 library-contract defect B1. All 919 are listed below.
CPU-censored observations retain their old baselines and are not counted
as completed display-only reruns.

| Group | Cases | Exact displayed change and controlling paragraph |
|---|---:|---|
| B1 | 1 | determinism gains task.Scheduler through run/test contrary to the explicit spec §9.9 callback-supplied service types. Old baseline retained; requires the library-owner fix and regression above. |
| D1 | 856 | Opaque callbacks keep one need: `(Fail(E), ..N)` becomes `N`; known service keys remain. Merging two callbacks combines their whole rows, without a separate `Fail(E | E2)` carrier. R1 (syntax-three §6.1); subsequent variable renumbering follows §4.7. |
| D2 | 111 | Directed bounds are shown without redundant variables: read-only `S(N & A)` becomes `S(N)`; a consumer of argument `A` needs `S(A)` rather than a separate bound `S(B)`; `cur : (A & B) <- (State(A & B)) -> B` becomes `(A) <- (State(A)) -> A`; a pure stored callback closes its residual N to Never. Inferred E/A labels can change. Spec §4.7 simplification, §4.5 rule 4 needs direction, §9.2 provider subtyping; R1 removes the independent error bounds. |
| D3 | 9 | Only canonical type-variable labels change after the independent error carrier disappears; the shown type is equivalent by renaming. R1 and spec §4.7. |
| D4 | 258 | E5 replaces planner messages such as `missing service S(Never)` or `service A must be closed` with the concrete constructor-key message `S is not provided by the target native`. Same E0523 multiset, source sites and failing phases. R6 (§2.10.11). |
| D5 | 3 | Recovery diagnostics retain a known String argument: `S(_, _)` becomes `S(String, _)` against the same incompatible `S(1, Never)`. Same E0515 site and invariant mismatch; ordinary incoming bounds survive single-row inference. R1, spec §4.7 monomorphic variables show their incoming types, §4.5 rule 4 and §9.2. |
| D6 | 2 | The obsolete independent error parameter disappears from `Boundary(R, E, N)`, now `Boundary(R, N)` (and likewise Continuation). Syntax-three §2.10.7 explicitly gives these two- and three-parameter forms, and R1 puts any Fail entry in N. |
| D7 | 5 | The single-row solver reports the incompatible leaf instead of the enclosing recovery summary: g_ann_Never/g_inf_Never show `_` against Never instead of `S(_)`; pl44 shows `_` against the required Int instead of `Cov(_)` against Never. Each E0501 remains at the same callback and annotation sites, with the same count; every separate provider error also remains. R1 changes the constraint graph, while spec §4.5 rule 4 still forbids the needed effect in a pure callback; §4.7 governs displayed incoming bounds. These are recovery projections of the same rejection, not newly accepted annotations. |
| D8 | 5 | The task scope handle is shown as `task.Group(E)` instead of `task.Scope(E)`. Syntax-three §2.10.9 says the structured-concurrency handle is renamed `task.Group(E)` in stage E5, so Scope in a need always means the resource service; §7.2 row E5 and the spec-change table repeat the rename. Spec §9.9 uses Group(E); task.scope remains the same operation. Runtime output and error counts are unchanged. |
| D9 (withdrawn) | 3 historical cases | Decision 2 drops universal-entry removal. Retain the 1,024-service keys and `Consumer(Never)` instead; simplifying an argument cannot provide a service (§9.2: “Getting it never fails at run time”). The old explanation is not approval to erase these keys; fresh records remain pending. |

One line per reviewed case:

| Case | Display explanation |
|---|---|
| `review-k4-lane/closed_handler_hole` | D1 |
| `review-k4-lane/handler-hole-False-False` | D1 |
| `review-k4-lane/handler-hole-True-False` | D1 |
| `review-k4-lane/handler-hole-True-True` | D1 |
| `review-k4-lane/previous-0-False` | D4 |
| `review-k4-lane/previous-0-True` | D4 |
| `review-k4-lane/previous-1-False` | D1, D4 |
| `review-k4-lane/previous-2-False` | D1, D4 |
| `review-k4-lane/previous-2-True` | D1, D4 |
| `review-k4-lane/previous-3-False` | D1, D4 |
| `review-k4-lane/previous-3-True` | D1, D4 |
| `review-k4-lane/previous-4-False` | D2, D4 |
| `review-k4-lane/previous-4-True` | D2, D4 |
| `review-k4-lane/previous-5-False` | D4 |
| `review-k4-lane/previous-5-True` | D4 |
| `review-k4-lane/previous-6-False` | D1, D4 |
| `review-k4-lane/previous-6-True` | D1, D4 |
| `review-k4-lane/previous-chain-64` | D1, D4 |
| `review-k4-lane/w1-Dep` | D1 |
| `review-k4-lane/w1-Never` | D1 |
| `review-k4-lane/w10-Dep` | D1 |
| `review-k4-lane/w10-Never` | D1 |
| `review-k4-lane/w12-Dep` | D1 |
| `review-k4-lane/w12-Never` | D1 |
| `review-k4-lane/w13-Dep` | D1 |
| `review-k4-lane/w13-Never` | D1 |
| `review-k4-lane/w2-Dep` | D1 |
| `review-k4-lane/w2-Never` | D1 |
| `review-k4-lane/w3-Dep` | D1 |
| `review-k4-lane/w3-Never` | D1 |
| `review-k4-lane/w4-Dep` | D1 |
| `review-k4-lane/w4-Never` | D1 |
| `review-k4-lane/w5-Dep` | D1 |
| `review-k4-lane/w5-Never` | D1 |
| `review-k4/cases/conflict_2046229799906290556` | D1 |
| `review-k4/cases/conflict_3995797194752417745` | D1 |
| `review-k4/cases/conflict_4688151140569818353` | D1 |
| `review-k4/cases/conflict_4759503553685934015` | D1 |
| `review-k4/cases/conflict_5003292184701381503` | D1 |
| `review-k4/cases/conflict_7401424894349461812` | D1 |
| `review-k4/cases/finite` | D1 |
| `review-k4/cases/generic_conflict` | D1, D2 |
| `review-k4/cases/hidden_conflict` | D1 |
| `review-k4/cases/invariant_args` | D1 |
| `review-k4/cases/local_instances_toprun` | D1 |
| `review-k4/cases/matrix_False_False_False` | D1, D2 |
| `review-k4/cases/matrix_False_False_False_reverse` | D1, D2 |
| `review-k4/cases/matrix_False_False_True` | D1, D2 |
| `review-k4/cases/matrix_False_False_True_reverse` | D1, D2 |
| `review-k4/cases/matrix_False_True_False` | D1, D2 |
| `review-k4/cases/matrix_False_True_False_reverse` | D1, D2 |
| `review-k4/cases/matrix_False_True_True` | D1, D2 |
| `review-k4/cases/matrix_False_True_True_reverse` | D1, D2 |
| `review-k4/cases/matrix_True_False_False` | D1 |
| `review-k4/cases/matrix_True_False_False_reverse` | D1 |
| `review-k4/cases/matrix_True_False_True` | D1 |
| `review-k4/cases/matrix_True_False_True_reverse` | D1 |
| `review-k4/cases/matrix_True_True_False` | D1 |
| `review-k4/cases/matrix_True_True_False_reverse` | D1 |
| `review-k4/cases/matrix_True_True_True` | D1 |
| `review-k4/cases/matrix_True_True_True_reverse` | D1 |
| `review-k4/cases/multi` | D1 |
| `review-k4/cases/new_unsound_annotated` | D1, D2, D5 |
| `review-k4/cases/new_unsound_closure` | D1, D2, D5 |
| `review-k4/cases/new_unsound_direct` | D1, D2, D5 |
| `review-k4/cases/provision_closure_False` | D1 |
| `review-k4/cases/provision_closure_True` | D1 |
| `review-k4/cases/provision_direct_False` | D1 |
| `review-k4/cases/provision_direct_True` | D1 |
| `review-k4/cases/provision_explicit_contract` | D1 |
| `review-k4/cases/provision_helper_False` | D1 |
| `review-k4/cases/provision_helper_True` | D1 |
| `review-k4/cases/provision_inferred_False` | D1 |
| `review-k4/cases/provision_inferred_True` | D1 |
| `review-k4/cases/provision_marker` | D1 |
| `review-k4/cases/wrapper_0_False_False` | D1 |
| `review-k4/cases/wrapper_0_False_True` | D1 |
| `review-k4/cases/wrapper_0_True_False` | D1 |
| `review-k4/cases/wrapper_0_True_True` | D1 |
| `review-k4/cases/wrapper_10_False_False` | D1 |
| `review-k4/cases/wrapper_10_False_True` | D1 |
| `review-k4/cases/wrapper_10_True_False` | D1 |
| `review-k4/cases/wrapper_10_True_True` | D1 |
| `review-k4/cases/wrapper_11_False_False` | D1 |
| `review-k4/cases/wrapper_11_False_True` | D1 |
| `review-k4/cases/wrapper_11_True_False` | D1 |
| `review-k4/cases/wrapper_11_True_True` | D1 |
| `review-k4/cases/wrapper_1_False_False` | D1 |
| `review-k4/cases/wrapper_1_False_True` | D1 |
| `review-k4/cases/wrapper_1_True_False` | D1 |
| `review-k4/cases/wrapper_1_True_True` | D1 |
| `review-k4/cases/wrapper_2_False_False` | D1 |
| `review-k4/cases/wrapper_2_False_True` | D1 |
| `review-k4/cases/wrapper_2_True_False` | D1 |
| `review-k4/cases/wrapper_2_True_True` | D1 |
| `review-k4/cases/wrapper_3_False_False` | D1 |
| `review-k4/cases/wrapper_3_False_True` | D1 |
| `review-k4/cases/wrapper_3_True_False` | D1 |
| `review-k4/cases/wrapper_3_True_True` | D1 |
| `review-k4/cases/wrapper_4_False_False` | D1 |
| `review-k4/cases/wrapper_4_False_True` | D1 |
| `review-k4/cases/wrapper_4_True_False` | D1 |
| `review-k4/cases/wrapper_4_True_True` | D1 |
| `review-k4/cases/wrapper_5_False_False` | D1 |
| `review-k4/cases/wrapper_5_False_True` | D1 |
| `review-k4/cases/wrapper_5_True_False` | D1 |
| `review-k4/cases/wrapper_5_True_True` | D1 |
| `review-k4/cases/wrapper_6_False_False` | D1 |
| `review-k4/cases/wrapper_6_False_True` | D1 |
| `review-k4/cases/wrapper_6_True_False` | D1 |
| `review-k4/cases/wrapper_6_True_True` | D1 |
| `review-k4/cases/wrapper_7_False_False` | D1 |
| `review-k4/cases/wrapper_7_False_True` | D1 |
| `review-k4/cases/wrapper_7_True_False` | D1 |
| `review-k4/cases/wrapper_7_True_True` | D1 |
| `review-k4/cases/wrapper_8_False_False` | D1 |
| `review-k4/cases/wrapper_8_False_True` | D1 |
| `review-k4/cases/wrapper_8_True_False` | D1 |
| `review-k4/cases/wrapper_8_True_True` | D1 |
| `review-k4/cases/wrapper_9_False_False` | D1 |
| `review-k4/cases/wrapper_9_False_True` | D1 |
| `review-k4/cases/wrapper_9_True_False` | D1 |
| `review-k4/cases/wrapper_9_True_True` | D1 |
| `review-k4/perf/generic_chain_1024` | D1 |
| `review-k4/perf/generic_chain_128` | D1 |
| `review-k4/perf/generic_chain_2048` | D1 |
| `review-k4/perf/generic_chain_256` | D1 |
| `review-k4/perf/generic_chain_512` | D1 |
| `review-k4/perf/generic_chain_64` | D1 |
| `review-k4/perf/plain_chain_1024` | D1 |
| `review-k4/perf/plain_chain_128` | D1 |
| `review-k4/perf/plain_chain_2048` | D1 |
| `review-k4/perf/plain_chain_256` | D1 |
| `review-k4/perf/plain_chain_512` | D1 |
| `review-k4/perf/plain_chain_64` | D1 |
| `review-k4/perf/wide_generic_1024` | D1, D9 |
| `review-k4/perf/wide_generic_128` | D1, D2 |
| `review-k4/perf/wide_generic_2048` | D1, D2 |
| `review-k4/perf/wide_generic_256` | D1, D2 |
| `review-k4/perf/wide_generic_512` | D1, D2 |
| `review-k4/perf/wide_generic_64` | D1, D2 |
| `review-k4c/cases/annotation_0` | D2 |
| `review-k4c/cases/annotation_1` | D2 |
| `review-k4c/cases/annotation_2` | D2 |
| `review-k4c/cases/annotation_3` | D2 |
| `review-k4c/cases/closure_statement` | D1, D4 |
| `review-k4c/cases/closure_statement_ok` | D1 |
| `review-k4c/cases/co_callback` | D1, D2 |
| `review-k4c/cases/co_deep` | D2 |
| `review-k4c/cases/co_direct` | D2 |
| `review-k4c/cases/co_escape` | D2 |
| `review-k4c/cases/co_higher` | D1, D2 |
| `review-k4c/cases/co_local` | D2 |
| `review-k4c/cases/co_nested` | D1, D2 |
| `review-k4c/cases/contra_callback` | D1 |
| `review-k4c/cases/contra_deep` | D2 |
| `review-k4c/cases/contra_escape` | D2 |
| `review-k4c/cases/contra_exploit` | D2 |
| `review-k4c/cases/contra_higher` | D1, D2 |
| `review-k4c/cases/contra_local` | D2 |
| `review-k4c/cases/contra_local_good` | D2 |
| `review-k4c/cases/contra_nested` | D1 |
| `review-k4c/cases/effects_co_0_0` | D1 |
| `review-k4c/cases/effects_co_0_1` | D1 |
| `review-k4c/cases/effects_co_1_0` | D1 |
| `review-k4c/cases/effects_co_1_1` | D1 |
| `review-k4c/cases/effects_co_2_0` | D1 |
| `review-k4c/cases/effects_co_2_1` | D1 |
| `review-k4c/cases/effects_co_3_0` | D1 |
| `review-k4c/cases/effects_co_3_1` | D1 |
| `review-k4c/cases/effects_co_4_0` | D1 |
| `review-k4c/cases/effects_co_4_1` | D1 |
| `review-k4c/cases/effects_co_5_0` | D1 |
| `review-k4c/cases/effects_co_5_1` | D1 |
| `review-k4c/cases/effects_co_6_0` | D1 |
| `review-k4c/cases/effects_co_6_1` | D1 |
| `review-k4c/cases/effects_co_7_0` | D1 |
| `review-k4c/cases/effects_co_7_1` | D1 |
| `review-k4c/cases/effects_inv_0_0` | D1 |
| `review-k4c/cases/effects_inv_0_1` | D1 |
| `review-k4c/cases/effects_inv_1_0` | D1 |
| `review-k4c/cases/effects_inv_1_1` | D1 |
| `review-k4c/cases/effects_inv_2_0` | D1 |
| `review-k4c/cases/effects_inv_2_1` | D1 |
| `review-k4c/cases/effects_inv_3_0` | D1 |
| `review-k4c/cases/effects_inv_3_1` | D1 |
| `review-k4c/cases/effects_inv_4_0` | D1 |
| `review-k4c/cases/effects_inv_4_1` | D1 |
| `review-k4c/cases/effects_inv_5_0` | D1 |
| `review-k4c/cases/effects_inv_5_1` | D1 |
| `review-k4c/cases/effects_inv_6_0` | D1 |
| `review-k4c/cases/effects_inv_6_1` | D1 |
| `review-k4c/cases/effects_inv_7_0` | D1 |
| `review-k4c/cases/effects_inv_7_1` | D1 |
| `review-k4c/cases/effects_two_0_0` | D1 |
| `review-k4c/cases/effects_two_0_1` | D1 |
| `review-k4c/cases/effects_two_1_0` | D1 |
| `review-k4c/cases/effects_two_1_1` | D1 |
| `review-k4c/cases/effects_two_2_0` | D1 |
| `review-k4c/cases/effects_two_2_1` | D1 |
| `review-k4c/cases/effects_two_3_0` | D1 |
| `review-k4c/cases/effects_two_3_1` | D1 |
| `review-k4c/cases/effects_two_4_0` | D1 |
| `review-k4c/cases/effects_two_4_1` | D1 |
| `review-k4c/cases/effects_two_5_0` | D1 |
| `review-k4c/cases/effects_two_5_1` | D1 |
| `review-k4c/cases/effects_two_6_0` | D1 |
| `review-k4c/cases/effects_two_6_1` | D1 |
| `review-k4c/cases/effects_two_7_0` | D1 |
| `review-k4c/cases/effects_two_7_1` | D1 |
| `review-k4c/cases/finite_field_0` | D1 |
| `review-k4c/cases/finite_field_1` | D1 |
| `review-k4c/cases/finite_field_2` | D1 |
| `review-k4c/cases/finite_field_3` | D1 |
| `review-k4c/cases/finite_field_4` | D1 |
| `review-k4c/cases/finite_field_annot_0` | D1 |
| `review-k4c/cases/finite_field_annot_1` | D1 |
| `review-k4c/cases/finite_field_annot_2` | D1 |
| `review-k4c/cases/finite_field_annot_3` | D1 |
| `review-k4c/cases/finite_field_annot_4` | D1 |
| `review-k4c/cases/finite_pattern_0` | D1 |
| `review-k4c/cases/finite_pattern_1` | D1 |
| `review-k4c/cases/finite_pattern_2` | D1 |
| `review-k4c/cases/finite_pattern_3` | D1 |
| `review-k4c/cases/finite_pattern_4` | D1 |
| `review-k4c/cases/inv_callback` | D1 |
| `review-k4c/cases/inv_higher` | D1 |
| `review-k4c/cases/inv_nested` | D1 |
| `review-k4c/cases/missing_0_0_0_0` | D1, D4 |
| `review-k4c/cases/missing_0_0_0_1` | D1, D4 |
| `review-k4c/cases/missing_0_0_0_2` | D1, D4 |
| `review-k4c/cases/missing_0_0_1_0` | D1, D4 |
| `review-k4c/cases/missing_0_0_1_1` | D1, D4 |
| `review-k4c/cases/missing_0_0_1_2` | D1, D4 |
| `review-k4c/cases/missing_0_1_0_0` | D1, D4 |
| `review-k4c/cases/missing_0_1_0_1` | D1, D4 |
| `review-k4c/cases/missing_0_1_0_2` | D1, D4 |
| `review-k4c/cases/missing_0_1_1_0` | D1, D4 |
| `review-k4c/cases/missing_0_1_1_1` | D1, D4 |
| `review-k4c/cases/missing_0_1_1_2` | D1, D4 |
| `review-k4c/cases/missing_0_2_0_0` | D1, D4 |
| `review-k4c/cases/missing_0_2_0_1` | D1, D4 |
| `review-k4c/cases/missing_0_2_0_2` | D1, D4 |
| `review-k4c/cases/missing_0_2_1_0` | D1, D4 |
| `review-k4c/cases/missing_0_2_1_1` | D1, D4 |
| `review-k4c/cases/missing_0_2_1_2` | D1, D4 |
| `review-k4c/cases/missing_0_3_0_0` | D1, D4 |
| `review-k4c/cases/missing_0_3_0_1` | D1, D4 |
| `review-k4c/cases/missing_0_3_0_2` | D1, D4 |
| `review-k4c/cases/missing_0_3_1_0` | D1, D4 |
| `review-k4c/cases/missing_0_3_1_1` | D1, D4 |
| `review-k4c/cases/missing_0_3_1_2` | D1, D4 |
| `review-k4c/cases/missing_0_4_0_0` | D1, D2, D4 |
| `review-k4c/cases/missing_0_4_0_1` | D1, D2, D4 |
| `review-k4c/cases/missing_0_4_0_2` | D1, D2, D4 |
| `review-k4c/cases/missing_0_4_1_0` | D1, D4 |
| `review-k4c/cases/missing_0_4_1_1` | D1, D4 |
| `review-k4c/cases/missing_0_4_1_2` | D1, D4 |
| `review-k4c/cases/missing_0_5_0_0` | D1, D4 |
| `review-k4c/cases/missing_0_5_0_1` | D1, D4 |
| `review-k4c/cases/missing_0_5_0_2` | D1, D4 |
| `review-k4c/cases/missing_0_5_1_0` | D1, D4 |
| `review-k4c/cases/missing_0_5_1_1` | D1, D4 |
| `review-k4c/cases/missing_0_5_1_2` | D1, D4 |
| `review-k4c/cases/missing_0_6_0_0` | D1, D4 |
| `review-k4c/cases/missing_0_6_0_1` | D1, D4 |
| `review-k4c/cases/missing_0_6_0_2` | D1, D4 |
| `review-k4c/cases/missing_0_6_1_0` | D1, D4 |
| `review-k4c/cases/missing_0_6_1_1` | D1, D4 |
| `review-k4c/cases/missing_0_6_1_2` | D1, D4 |
| `review-k4c/cases/missing_0_7_0_0` | D1, D4 |
| `review-k4c/cases/missing_0_7_0_1` | D1, D4 |
| `review-k4c/cases/missing_0_7_0_2` | D1, D4 |
| `review-k4c/cases/missing_0_7_1_0` | D1, D4 |
| `review-k4c/cases/missing_0_7_1_1` | D1, D4 |
| `review-k4c/cases/missing_0_7_1_2` | D1, D4 |
| `review-k4c/cases/missing_1_0_0_0` | D1, D4 |
| `review-k4c/cases/missing_1_0_0_1` | D1, D4 |
| `review-k4c/cases/missing_1_0_0_2` | D1, D4 |
| `review-k4c/cases/missing_1_0_1_0` | D1, D4 |
| `review-k4c/cases/missing_1_0_1_1` | D1, D4 |
| `review-k4c/cases/missing_1_0_1_2` | D1, D4 |
| `review-k4c/cases/missing_1_1_0_0` | D1, D4 |
| `review-k4c/cases/missing_1_1_0_1` | D1, D4 |
| `review-k4c/cases/missing_1_1_0_2` | D1, D4 |
| `review-k4c/cases/missing_1_1_1_0` | D1, D4 |
| `review-k4c/cases/missing_1_1_1_1` | D1, D4 |
| `review-k4c/cases/missing_1_1_1_2` | D1, D4 |
| `review-k4c/cases/missing_1_2_0_0` | D1, D4 |
| `review-k4c/cases/missing_1_2_0_1` | D1, D4 |
| `review-k4c/cases/missing_1_2_0_2` | D1, D4 |
| `review-k4c/cases/missing_1_2_1_0` | D1, D4 |
| `review-k4c/cases/missing_1_2_1_1` | D1, D4 |
| `review-k4c/cases/missing_1_2_1_2` | D1, D4 |
| `review-k4c/cases/missing_1_3_0_0` | D1, D4 |
| `review-k4c/cases/missing_1_3_0_1` | D1, D4 |
| `review-k4c/cases/missing_1_3_0_2` | D1, D4 |
| `review-k4c/cases/missing_1_3_1_0` | D1, D4 |
| `review-k4c/cases/missing_1_3_1_1` | D1, D4 |
| `review-k4c/cases/missing_1_3_1_2` | D1, D4 |
| `review-k4c/cases/missing_1_4_0_0` | D1, D4 |
| `review-k4c/cases/missing_1_4_0_1` | D1, D4 |
| `review-k4c/cases/missing_1_4_0_2` | D1, D4 |
| `review-k4c/cases/missing_1_4_1_0` | D1, D4 |
| `review-k4c/cases/missing_1_4_1_1` | D1, D4 |
| `review-k4c/cases/missing_1_4_1_2` | D1, D4 |
| `review-k4c/cases/missing_1_5_0_0` | D1, D4 |
| `review-k4c/cases/missing_1_5_0_1` | D1, D4 |
| `review-k4c/cases/missing_1_5_0_2` | D1, D4 |
| `review-k4c/cases/missing_1_5_1_0` | D1, D4 |
| `review-k4c/cases/missing_1_5_1_1` | D1, D4 |
| `review-k4c/cases/missing_1_5_1_2` | D1, D4 |
| `review-k4c/cases/missing_1_6_0_0` | D1, D4 |
| `review-k4c/cases/missing_1_6_0_1` | D1, D4 |
| `review-k4c/cases/missing_1_6_0_2` | D1, D4 |
| `review-k4c/cases/missing_1_6_1_0` | D1, D4 |
| `review-k4c/cases/missing_1_6_1_1` | D1, D4 |
| `review-k4c/cases/missing_1_6_1_2` | D1, D4 |
| `review-k4c/cases/missing_1_7_0_0` | D1, D4 |
| `review-k4c/cases/missing_1_7_0_1` | D1, D4 |
| `review-k4c/cases/missing_1_7_0_2` | D1, D4 |
| `review-k4c/cases/missing_1_7_1_0` | D1, D4 |
| `review-k4c/cases/missing_1_7_1_1` | D1, D4 |
| `review-k4c/cases/missing_1_7_1_2` | D1, D4 |
| `review-k4c/cases/missing_2_0_0_0` | D1, D4 |
| `review-k4c/cases/missing_2_0_0_1` | D1, D4 |
| `review-k4c/cases/missing_2_0_0_2` | D1, D4 |
| `review-k4c/cases/missing_2_0_1_0` | D1, D4 |
| `review-k4c/cases/missing_2_0_1_1` | D1, D4 |
| `review-k4c/cases/missing_2_0_1_2` | D1, D4 |
| `review-k4c/cases/missing_2_1_0_0` | D1, D4 |
| `review-k4c/cases/missing_2_1_0_1` | D1, D4 |
| `review-k4c/cases/missing_2_1_0_2` | D1, D4 |
| `review-k4c/cases/missing_2_1_1_0` | D1, D4 |
| `review-k4c/cases/missing_2_1_1_1` | D1, D4 |
| `review-k4c/cases/missing_2_1_1_2` | D1, D4 |
| `review-k4c/cases/missing_2_2_0_0` | D1, D4 |
| `review-k4c/cases/missing_2_2_0_1` | D1, D4 |
| `review-k4c/cases/missing_2_2_0_2` | D1, D4 |
| `review-k4c/cases/missing_2_2_1_0` | D1, D4 |
| `review-k4c/cases/missing_2_2_1_1` | D1, D4 |
| `review-k4c/cases/missing_2_2_1_2` | D1, D4 |
| `review-k4c/cases/missing_2_3_0_0` | D1, D4 |
| `review-k4c/cases/missing_2_3_0_1` | D1, D4 |
| `review-k4c/cases/missing_2_3_0_2` | D1, D4 |
| `review-k4c/cases/missing_2_3_1_0` | D1, D4 |
| `review-k4c/cases/missing_2_3_1_1` | D1, D4 |
| `review-k4c/cases/missing_2_3_1_2` | D1, D4 |
| `review-k4c/cases/missing_2_4_0_0` | D1, D2, D4 |
| `review-k4c/cases/missing_2_4_0_1` | D1, D2, D4 |
| `review-k4c/cases/missing_2_4_0_2` | D1, D2, D4 |
| `review-k4c/cases/missing_2_4_1_0` | D1, D4 |
| `review-k4c/cases/missing_2_4_1_1` | D1, D4 |
| `review-k4c/cases/missing_2_4_1_2` | D1, D4 |
| `review-k4c/cases/missing_2_5_0_0` | D1, D4 |
| `review-k4c/cases/missing_2_5_0_1` | D1, D4 |
| `review-k4c/cases/missing_2_5_0_2` | D1, D4 |
| `review-k4c/cases/missing_2_5_1_0` | D1, D4 |
| `review-k4c/cases/missing_2_5_1_1` | D1, D4 |
| `review-k4c/cases/missing_2_5_1_2` | D1, D4 |
| `review-k4c/cases/missing_2_6_0_0` | D1, D4 |
| `review-k4c/cases/missing_2_6_0_1` | D1, D4 |
| `review-k4c/cases/missing_2_6_0_2` | D1, D4 |
| `review-k4c/cases/missing_2_6_1_0` | D1, D4 |
| `review-k4c/cases/missing_2_6_1_1` | D1, D4 |
| `review-k4c/cases/missing_2_6_1_2` | D1, D4 |
| `review-k4c/cases/missing_2_7_0_0` | D1, D4 |
| `review-k4c/cases/missing_2_7_0_1` | D1, D4 |
| `review-k4c/cases/missing_2_7_0_2` | D1, D4 |
| `review-k4c/cases/missing_2_7_1_0` | D1, D4 |
| `review-k4c/cases/missing_2_7_1_1` | D1, D4 |
| `review-k4c/cases/missing_2_7_1_2` | D1, D4 |
| `review-k4c/cases/missing_3_0_0_0` | D1, D4 |
| `review-k4c/cases/missing_3_0_0_1` | D1, D4 |
| `review-k4c/cases/missing_3_0_0_2` | D1, D4 |
| `review-k4c/cases/missing_3_0_1_0` | D1, D4 |
| `review-k4c/cases/missing_3_0_1_1` | D1, D4 |
| `review-k4c/cases/missing_3_0_1_2` | D1, D4 |
| `review-k4c/cases/missing_3_1_0_0` | D1, D4 |
| `review-k4c/cases/missing_3_1_0_1` | D1, D4 |
| `review-k4c/cases/missing_3_1_0_2` | D1, D4 |
| `review-k4c/cases/missing_3_1_1_0` | D1, D4 |
| `review-k4c/cases/missing_3_1_1_1` | D1, D4 |
| `review-k4c/cases/missing_3_1_1_2` | D1, D4 |
| `review-k4c/cases/missing_3_2_0_0` | D1, D4 |
| `review-k4c/cases/missing_3_2_0_1` | D1, D4 |
| `review-k4c/cases/missing_3_2_0_2` | D1, D4 |
| `review-k4c/cases/missing_3_2_1_0` | D1, D4 |
| `review-k4c/cases/missing_3_2_1_1` | D1, D4 |
| `review-k4c/cases/missing_3_2_1_2` | D1, D4 |
| `review-k4c/cases/missing_3_3_0_0` | D1, D4 |
| `review-k4c/cases/missing_3_3_0_1` | D1, D4 |
| `review-k4c/cases/missing_3_3_0_2` | D1, D4 |
| `review-k4c/cases/missing_3_3_1_0` | D1, D4 |
| `review-k4c/cases/missing_3_3_1_1` | D1, D4 |
| `review-k4c/cases/missing_3_3_1_2` | D1, D4 |
| `review-k4c/cases/missing_3_4_0_0` | D1 |
| `review-k4c/cases/missing_3_4_0_1` | D1 |
| `review-k4c/cases/missing_3_4_0_2` | D1 |
| `review-k4c/cases/missing_3_4_1_0` | D1 |
| `review-k4c/cases/missing_3_4_1_1` | D1 |
| `review-k4c/cases/missing_3_4_1_2` | D1 |
| `review-k4c/cases/missing_3_5_0_0` | D1, D4 |
| `review-k4c/cases/missing_3_5_0_1` | D1, D4 |
| `review-k4c/cases/missing_3_5_0_2` | D1, D4 |
| `review-k4c/cases/missing_3_5_1_0` | D1, D4 |
| `review-k4c/cases/missing_3_5_1_1` | D1, D4 |
| `review-k4c/cases/missing_3_5_1_2` | D1, D4 |
| `review-k4c/cases/missing_3_6_0_0` | D1, D4 |
| `review-k4c/cases/missing_3_6_0_1` | D1, D4 |
| `review-k4c/cases/missing_3_6_0_2` | D1, D4 |
| `review-k4c/cases/missing_3_6_1_0` | D1, D4 |
| `review-k4c/cases/missing_3_6_1_1` | D1, D4 |
| `review-k4c/cases/missing_3_6_1_2` | D1, D4 |
| `review-k4c/cases/missing_3_7_0_0` | D1, D4 |
| `review-k4c/cases/missing_3_7_0_1` | D1, D4 |
| `review-k4c/cases/missing_3_7_0_2` | D1, D4 |
| `review-k4c/cases/missing_3_7_1_0` | D1, D4 |
| `review-k4c/cases/missing_3_7_1_1` | D1, D4 |
| `review-k4c/cases/missing_3_7_1_2` | D1, D4 |
| `review-k4c/cases/multi_run` | D1 |
| `review-k4c/cases/nested_env_function` | D1 |
| `review-k4c/cases/order_hidden_conflict_0` | D1 |
| `review-k4c/cases/order_hidden_conflict_1` | D1 |
| `review-k4c/cases/order_hidden_conflict_10` | D1 |
| `review-k4c/cases/order_hidden_conflict_11` | D1 |
| `review-k4c/cases/order_hidden_conflict_12` | D1 |
| `review-k4c/cases/order_hidden_conflict_13` | D1 |
| `review-k4c/cases/order_hidden_conflict_14` | D1 |
| `review-k4c/cases/order_hidden_conflict_15` | D1 |
| `review-k4c/cases/order_hidden_conflict_16` | D1 |
| `review-k4c/cases/order_hidden_conflict_17` | D1 |
| `review-k4c/cases/order_hidden_conflict_18` | D1 |
| `review-k4c/cases/order_hidden_conflict_19` | D1 |
| `review-k4c/cases/order_hidden_conflict_2` | D1 |
| `review-k4c/cases/order_hidden_conflict_20` | D1 |
| `review-k4c/cases/order_hidden_conflict_21` | D1 |
| `review-k4c/cases/order_hidden_conflict_22` | D1 |
| `review-k4c/cases/order_hidden_conflict_23` | D1 |
| `review-k4c/cases/order_hidden_conflict_24` | D1 |
| `review-k4c/cases/order_hidden_conflict_25` | D1 |
| `review-k4c/cases/order_hidden_conflict_26` | D1 |
| `review-k4c/cases/order_hidden_conflict_27` | D1 |
| `review-k4c/cases/order_hidden_conflict_28` | D1 |
| `review-k4c/cases/order_hidden_conflict_29` | D1 |
| `review-k4c/cases/order_hidden_conflict_3` | D1 |
| `review-k4c/cases/order_hidden_conflict_30` | D1 |
| `review-k4c/cases/order_hidden_conflict_31` | D1 |
| `review-k4c/cases/order_hidden_conflict_4` | D1 |
| `review-k4c/cases/order_hidden_conflict_5` | D1 |
| `review-k4c/cases/order_hidden_conflict_6` | D1 |
| `review-k4c/cases/order_hidden_conflict_7` | D1 |
| `review-k4c/cases/order_hidden_conflict_8` | D1 |
| `review-k4c/cases/order_hidden_conflict_9` | D1 |
| `review-k4c/cases/order_multi_instance_0` | D1 |
| `review-k4c/cases/order_multi_instance_1` | D1 |
| `review-k4c/cases/order_multi_instance_10` | D1 |
| `review-k4c/cases/order_multi_instance_11` | D1 |
| `review-k4c/cases/order_multi_instance_12` | D1 |
| `review-k4c/cases/order_multi_instance_13` | D1 |
| `review-k4c/cases/order_multi_instance_14` | D1 |
| `review-k4c/cases/order_multi_instance_15` | D1 |
| `review-k4c/cases/order_multi_instance_16` | D1 |
| `review-k4c/cases/order_multi_instance_17` | D1 |
| `review-k4c/cases/order_multi_instance_18` | D1 |
| `review-k4c/cases/order_multi_instance_19` | D1 |
| `review-k4c/cases/order_multi_instance_2` | D1 |
| `review-k4c/cases/order_multi_instance_20` | D1 |
| `review-k4c/cases/order_multi_instance_21` | D1 |
| `review-k4c/cases/order_multi_instance_22` | D1 |
| `review-k4c/cases/order_multi_instance_23` | D1 |
| `review-k4c/cases/order_multi_instance_24` | D1 |
| `review-k4c/cases/order_multi_instance_25` | D1 |
| `review-k4c/cases/order_multi_instance_26` | D1 |
| `review-k4c/cases/order_multi_instance_27` | D1 |
| `review-k4c/cases/order_multi_instance_28` | D1 |
| `review-k4c/cases/order_multi_instance_29` | D1 |
| `review-k4c/cases/order_multi_instance_3` | D1 |
| `review-k4c/cases/order_multi_instance_30` | D1 |
| `review-k4c/cases/order_multi_instance_31` | D1 |
| `review-k4c/cases/order_multi_instance_4` | D1 |
| `review-k4c/cases/order_multi_instance_5` | D1 |
| `review-k4c/cases/order_multi_instance_6` | D1 |
| `review-k4c/cases/order_multi_instance_7` | D1 |
| `review-k4c/cases/order_multi_instance_8` | D1 |
| `review-k4c/cases/order_multi_instance_9` | D1 |
| `review-k4c/cases/order_mutual_0` | D1, D2 |
| `review-k4c/cases/order_mutual_1` | D1, D2 |
| `review-k4c/cases/order_mutual_10` | D1, D2 |
| `review-k4c/cases/order_mutual_11` | D1, D2 |
| `review-k4c/cases/order_mutual_12` | D1, D2 |
| `review-k4c/cases/order_mutual_13` | D1, D2 |
| `review-k4c/cases/order_mutual_14` | D1, D2 |
| `review-k4c/cases/order_mutual_15` | D1, D2 |
| `review-k4c/cases/order_mutual_16` | D1, D2 |
| `review-k4c/cases/order_mutual_17` | D1, D2 |
| `review-k4c/cases/order_mutual_18` | D1, D2 |
| `review-k4c/cases/order_mutual_19` | D1, D2 |
| `review-k4c/cases/order_mutual_2` | D1, D2 |
| `review-k4c/cases/order_mutual_20` | D1, D2 |
| `review-k4c/cases/order_mutual_21` | D1, D2 |
| `review-k4c/cases/order_mutual_22` | D1, D2 |
| `review-k4c/cases/order_mutual_23` | D1, D2 |
| `review-k4c/cases/order_mutual_24` | D1, D2 |
| `review-k4c/cases/order_mutual_25` | D1, D2 |
| `review-k4c/cases/order_mutual_26` | D1, D2 |
| `review-k4c/cases/order_mutual_27` | D1, D2 |
| `review-k4c/cases/order_mutual_28` | D1, D2 |
| `review-k4c/cases/order_mutual_29` | D1, D2 |
| `review-k4c/cases/order_mutual_3` | D1, D2 |
| `review-k4c/cases/order_mutual_30` | D1, D2 |
| `review-k4c/cases/order_mutual_31` | D1, D2 |
| `review-k4c/cases/order_mutual_4` | D1, D2 |
| `review-k4c/cases/order_mutual_5` | D1, D2 |
| `review-k4c/cases/order_mutual_6` | D1, D2 |
| `review-k4c/cases/order_mutual_7` | D1, D2 |
| `review-k4c/cases/order_mutual_8` | D1, D2 |
| `review-k4c/cases/order_mutual_9` | D1, D2 |
| `review-k4c/cases/order_mutual_annot_0` | D1 |
| `review-k4c/cases/order_mutual_annot_1` | D1 |
| `review-k4c/cases/order_mutual_annot_10` | D1 |
| `review-k4c/cases/order_mutual_annot_11` | D1 |
| `review-k4c/cases/order_mutual_annot_12` | D1 |
| `review-k4c/cases/order_mutual_annot_13` | D1 |
| `review-k4c/cases/order_mutual_annot_14` | D1 |
| `review-k4c/cases/order_mutual_annot_15` | D1 |
| `review-k4c/cases/order_mutual_annot_16` | D1 |
| `review-k4c/cases/order_mutual_annot_17` | D1 |
| `review-k4c/cases/order_mutual_annot_18` | D1 |
| `review-k4c/cases/order_mutual_annot_19` | D1 |
| `review-k4c/cases/order_mutual_annot_2` | D1 |
| `review-k4c/cases/order_mutual_annot_20` | D1 |
| `review-k4c/cases/order_mutual_annot_21` | D1 |
| `review-k4c/cases/order_mutual_annot_22` | D1 |
| `review-k4c/cases/order_mutual_annot_23` | D1 |
| `review-k4c/cases/order_mutual_annot_24` | D1 |
| `review-k4c/cases/order_mutual_annot_25` | D1 |
| `review-k4c/cases/order_mutual_annot_26` | D1 |
| `review-k4c/cases/order_mutual_annot_27` | D1 |
| `review-k4c/cases/order_mutual_annot_28` | D1 |
| `review-k4c/cases/order_mutual_annot_29` | D1 |
| `review-k4c/cases/order_mutual_annot_3` | D1 |
| `review-k4c/cases/order_mutual_annot_30` | D1 |
| `review-k4c/cases/order_mutual_annot_31` | D1 |
| `review-k4c/cases/order_mutual_annot_4` | D1 |
| `review-k4c/cases/order_mutual_annot_5` | D1 |
| `review-k4c/cases/order_mutual_annot_6` | D1 |
| `review-k4c/cases/order_mutual_annot_7` | D1 |
| `review-k4c/cases/order_mutual_annot_8` | D1 |
| `review-k4c/cases/order_mutual_annot_9` | D1 |
| `review-k4c/cases/original` | D4 |
| `review-k4c/cases/plain_0` | D1 |
| `review-k4c/cases/plain_2` | D1 |
| `review-k4c/cases/plain_4` | D1 |
| `review-k4c/cases/plain_5` | D1 |
| `review-k4c/cases/plain_6` | D1 |
| `review-k4c/cases/recursive_co_0` | D1 |
| `review-k4c/cases/recursive_co_1` | D1 |
| `review-k4c/cases/recursive_co_2` | D1 |
| `review-k4c/cases/recursive_co_3` | D1 |
| `review-k4c/cases/recursive_co_4` | D1 |
| `review-k4c/cases/recursive_co_5` | D1 |
| `review-k4c/cases/recursive_inv_0` | D1 |
| `review-k4c/cases/recursive_inv_1` | D1 |
| `review-k4c/cases/recursive_inv_2` | D1 |
| `review-k4c/cases/recursive_inv_3` | D1 |
| `review-k4c/cases/recursive_inv_4` | D1 |
| `review-k4c/cases/recursive_inv_5` | D1 |
| `review-k4c/cases/row_callback` | D1 |
| `review-k4c/cases/row_higher` | D1, D2 |
| `review-k4c/cases/row_nested` | D1 |
| `review-k4c/cases/stress_plain_1024` | D1 |
| `review-k4c/cases/stress_plain_128` | D1 |
| `review-k4c/cases/stress_plain_32` | D1 |
| `review-k4c/cases/stress_plain_512` | D1 |
| `review-k4c/cases/stress_row_1024` | D1 |
| `review-k4c/cases/stress_row_128` | D1 |
| `review-k4c/cases/stress_row_32` | D1 |
| `review-k4c/cases/stress_row_512` | D1 |
| `review-k4c/cases/stress_wide_1024` | D9 |
| `review-k4c/cases/stress_wide_128` | D2 |
| `review-k4c/cases/stress_wide_32` | D2 |
| `review-k4c/cases/stress_wide_512` | D2 |
| `review-k4c/cases/stress_wide_open_1024` | D1, D2 |
| `review-k4c/cases/stress_wide_open_128` | D1, D2 |
| `review-k4c/cases/stress_wide_open_32` | D1, D2 |
| `review-k4c/cases/stress_wide_open_512` | D1, D2 |
| `review-k4c/cases/two_run` | D1 |
| `review-k4d/_loose/fuzz/bad/run_44_188` | D1, D2 |
| `review-k4d/_loose/fuzz/bad/run_44_361` | D1, D2 |
| `review-k4d/_loose/fuzz/e0505/21_139` | D1, D4 |
| `review-k4d/_loose/fuzz/e0505/21_177` | D1, D4 |
| `review-k4d/_loose/fuzz/e0505/21_194` | D1, D4 |
| `review-k4d/_loose/fuzz/e0505/21_253` | D1 |
| `review-k4d/_loose/fuzz/e0505/21_268` | D1, D4 |
| `review-k4d/_loose/fuzz/e0505/21_269` | D1, D4 |
| `review-k4d/_loose/fuzz/e0505/21_272` | D1 |
| `review-k4d/_loose/fuzz/e0505/21_3` | D1, D4 |
| `review-k4d/_loose/fuzz/e0505/21_36` | D1, D4 |
| `review-k4d/_loose/fuzz/e0505/21_4` | D1 |
| `review-k4d/_loose/fuzz/e0505/21_55` | D1, D4 |
| `review-k4d/_loose/fuzz/e0505/21_78` | D1, D4 |
| `review-k4d/_loose/fuzz/e0505/21_84` | D1, D4 |
| `review-k4d/_loose/p/a2` | D1 |
| `review-k4d/_loose/p/a3` | D4 |
| `review-k4d/_loose/p/b2` | D1, D4 |
| `review-k4d/_loose/p/b3` | D1, D4 |
| `review-k4d/_loose/p/b4` | D1, D4 |
| `review-k4d/_loose/p/b5` | D1 |
| `review-k4d/_loose/p/b6` | D1, D4 |
| `review-k4d/_loose/p/b7` | D1, D4 |
| `review-k4d/_loose/p/b8` | D1, D2, D4 |
| `review-k4d/_loose/p/b9` | D1, D2, D4 |
| `review-k4d/_loose/p/d2` | D2 |
| `review-k4d/_loose/p/e1` | D1, D2 |
| `review-k4d/_loose/p/e2` | D1, D2 |
| `review-k4d/_loose/p/g_ann_Never` | D7 |
| `review-k4d/_loose/p/g_inf_Never` | D7 |
| `review-k4d/_loose/p/p1` | D1, D2 |
| `review-k4d/_loose/p/q1` | D1, D2 |
| `review-k4d/_loose/p/q10` | D1 |
| `review-k4d/_loose/p/q11` | D1 |
| `review-k4d/_loose/p/q12` | D1 |
| `review-k4d/_loose/p/q13` | D1 |
| `review-k4d/_loose/p/q14` | D1 |
| `review-k4d/_loose/p/q15` | D1 |
| `review-k4d/_loose/p/q16` | D1 |
| `review-k4d/_loose/p/q17` | D1 |
| `review-k4d/_loose/p/q18` | D1 |
| `review-k4d/_loose/p/q19` | D1 |
| `review-k4d/_loose/p/q2` | D1 |
| `review-k4d/_loose/p/q20` | D1 |
| `review-k4d/_loose/p/q3` | D1 |
| `review-k4d/_loose/p/q4` | D1 |
| `review-k4d/_loose/p/q5` | D1 |
| `review-k4d/_loose/p/q6` | D1, D2 |
| `review-k4d/_loose/p/q7` | D1 |
| `review-k4d/_loose/p/q8` | D1 |
| `review-k4d/_loose/p/q9` | D1 |
| `review-k4d/_loose/p/r1` | D1, D4 |
| `review-k4d/_loose/p/r2` | D1, D4 |
| `review-k4d/_loose/p/r3` | D1, D4 |
| `review-k4d/_loose/p/r4` | D1 |
| `review-k4d/_loose/p/r4b` | D1 |
| `review-k4d/_loose/p/r4c` | D1 |
| `review-k4d/_loose/p/r5` | D1, D4 |
| `review-k4d/_loose/p/w1` | D1 |
| `review-k4d/_loose/p/w10` | D1 |
| `review-k4d/_loose/p/w12` | D1 |
| `review-k4d/_loose/p/w13` | D1 |
| `review-k4d/_loose/p/w2` | D1 |
| `review-k4d/_loose/p/w3` | D1 |
| `review-k4d/_loose/p/w4` | D1 |
| `review-k4d/_loose/p/w5` | D1 |
| `review-k4d/_loose/p/x2` | D1 |
| `review-k4d/_loose/p/x3` | D1 |
| `review-k4d/_loose/p/x5` | D1 |
| `review-k4d/_loose/p/x7` | D1 |
| `review-k4d/_loose/p/x8` | D1 |
| `review-k4d/_loose/p/z2` | D1 |
| `review-k4d/_loose/spec/tests_spec_11_stop_tail_service` | D1 |
| `review-k4d/_loose/spec/tests_spec_9_generic_service_rows` | D1 |
| `review-k4d/fuzz/m0` | D1 |
| `review-k4d/fuzz/m11` | D1 |
| `review-k4d/fuzz/m13` | D1 |
| `review-k4d/fuzz/m14` | D1 |
| `review-k4d/fuzz/m15` | D1 |
| `review-k4d/fuzz/m16` | D1 |
| `review-k4d/fuzz/m21` | D1 |
| `review-k4d/fuzz/m31` | D1 |
| `review-k4d/fuzz/m34` | D1, D4 |
| `review-k4d/fuzz/m35` | D1 |
| `review-k4d/fuzz/pl44` | D1, D2, D7 |
| `review-k4d/fuzz/pl45` | D1, D2 |
| `review-k4d/fuzz/pl46` | D1, D2 |
| `review-k4d/fuzz/w1` | D1, D4 |
| `review-k4d/fuzz/w2` | D1 |
| `review-k4d/fuzz/w3` | D1 |
| `review-k4d/fuzz/w4` | D1 |
| `review-k4d/perf/chain1000` | D1 |
| `review-k4d/perf/chain200` | D1 |
| `review-k4d/perf/chain4000` | D1 |
| `review-k4d/perf/mutual300` | D1 |
| `review-k4d/perf/runchain1000` | D1 |
| `review-k4d/perf/runchain200` | D1 |
| `review-k4d/perf/runchain4000` | D1 |
| `review-k4d/perf/twice1000` | D1 |
| `review-k4d/perf/twice200` | D1 |
| `review-k4d/perf/twice4000` | D1 |
| `review-k4d/perm/p0` | D1 |
| `review-k4d/perm/p1` | D1 |
| `review-k4d/perm/p10` | D1 |
| `review-k4d/perm/p11` | D1 |
| `review-k4d/perm/p12` | D1 |
| `review-k4d/perm/p13` | D1 |
| `review-k4d/perm/p14` | D1 |
| `review-k4d/perm/p15` | D1 |
| `review-k4d/perm/p16` | D1 |
| `review-k4d/perm/p17` | D1 |
| `review-k4d/perm/p18` | D1 |
| `review-k4d/perm/p19` | D1 |
| `review-k4d/perm/p2` | D1 |
| `review-k4d/perm/p20` | D1 |
| `review-k4d/perm/p21` | D1 |
| `review-k4d/perm/p22` | D1 |
| `review-k4d/perm/p23` | D1 |
| `review-k4d/perm/p24` | D1 |
| `review-k4d/perm/p25` | D1 |
| `review-k4d/perm/p26` | D1 |
| `review-k4d/perm/p27` | D1 |
| `review-k4d/perm/p28` | D1 |
| `review-k4d/perm/p29` | D1 |
| `review-k4d/perm/p3` | D1 |
| `review-k4d/perm/p30` | D1 |
| `review-k4d/perm/p31` | D1 |
| `review-k4d/perm/p32` | D1 |
| `review-k4d/perm/p33` | D1 |
| `review-k4d/perm/p34` | D1 |
| `review-k4d/perm/p35` | D1 |
| `review-k4d/perm/p36` | D1 |
| `review-k4d/perm/p37` | D1 |
| `review-k4d/perm/p38` | D1 |
| `review-k4d/perm/p39` | D1 |
| `review-k4d/perm/p4` | D1 |
| `review-k4d/perm/p40` | D1 |
| `review-k4d/perm/p41` | D1 |
| `review-k4d/perm/p42` | D1 |
| `review-k4d/perm/p43` | D1 |
| `review-k4d/perm/p44` | D1 |
| `review-k4d/perm/p45` | D1 |
| `review-k4d/perm/p46` | D1 |
| `review-k4d/perm/p47` | D1 |
| `review-k4d/perm/p48` | D1 |
| `review-k4d/perm/p49` | D1 |
| `review-k4d/perm/p5` | D1 |
| `review-k4d/perm/p50` | D1 |
| `review-k4d/perm/p51` | D1 |
| `review-k4d/perm/p52` | D1 |
| `review-k4d/perm/p53` | D1 |
| `review-k4d/perm/p54` | D1 |
| `review-k4d/perm/p55` | D1 |
| `review-k4d/perm/p56` | D1 |
| `review-k4d/perm/p57` | D1 |
| `review-k4d/perm/p58` | D1 |
| `review-k4d/perm/p59` | D1 |
| `review-k4d/perm/p6` | D1 |
| `review-k4d/perm/p7` | D1 |
| `review-k4d/perm/p8` | D1 |
| `review-k4d/perm/p9` | D1 |
| `review-k4d/proj/a2` | D1 |
| `review-k4d/proj/a3` | D4 |
| `review-k4d/proj/b2` | D1, D4 |
| `review-k4d/proj/b3` | D1, D4 |
| `review-k4d/proj/b4` | D1, D4 |
| `review-k4d/proj/b5` | D1 |
| `review-k4d/proj/b6` | D1, D4 |
| `review-k4d/proj/b7` | D1, D4 |
| `review-k4d/proj/b8` | D1, D2, D4 |
| `review-k4d/proj/b9` | D1, D2, D4 |
| `review-k4d/proj/d2` | D2 |
| `review-k4d/proj/e1` | D1, D2 |
| `review-k4d/proj/e2` | D1, D2 |
| `review-k4d/proj/g_ann_Never` | D7 |
| `review-k4d/proj/g_inf_Never` | D7 |
| `review-k4d/proj/p1` | D1, D2 |
| `review-k4d/proj/q1` | D1, D2 |
| `review-k4d/proj/q10` | D1 |
| `review-k4d/proj/q11` | D1 |
| `review-k4d/proj/q12` | D1 |
| `review-k4d/proj/q13` | D1 |
| `review-k4d/proj/q14` | D1 |
| `review-k4d/proj/q15` | D1 |
| `review-k4d/proj/q16` | D1 |
| `review-k4d/proj/q17` | D1 |
| `review-k4d/proj/q18` | D1 |
| `review-k4d/proj/q19` | D1 |
| `review-k4d/proj/q2` | D1 |
| `review-k4d/proj/q20` | D1 |
| `review-k4d/proj/q3` | D1 |
| `review-k4d/proj/q4` | D1 |
| `review-k4d/proj/q5` | D1 |
| `review-k4d/proj/q6` | D1, D2 |
| `review-k4d/proj/q7` | D1 |
| `review-k4d/proj/q8` | D1 |
| `review-k4d/proj/q9` | D1 |
| `review-k4d/proj/r1` | D1, D4 |
| `review-k4d/proj/r2` | D1, D4 |
| `review-k4d/proj/r3` | D1, D4 |
| `review-k4d/proj/r4` | D1 |
| `review-k4d/proj/r4b` | D1 |
| `review-k4d/proj/r4c` | D1 |
| `review-k4d/proj/r5` | D1, D4 |
| `review-k4d/proj/w1` | D1 |
| `review-k4d/proj/w10` | D1 |
| `review-k4d/proj/w12` | D1 |
| `review-k4d/proj/w13` | D1 |
| `review-k4d/proj/w2` | D1 |
| `review-k4d/proj/w3` | D1 |
| `review-k4d/proj/w4` | D1 |
| `review-k4d/proj/w5` | D1 |
| `review-k4d/proj/x2` | D1 |
| `review-k4d/proj/x3` | D1 |
| `review-k4d/proj/x5` | D1 |
| `review-k4d/proj/x7` | D1 |
| `review-k4d/proj/x8` | D1 |
| `review-k4d/proj/z2` | D1 |
| `review-k4s/ann_arg0` | D1 |
| `review-k4s/ann_arg1` | D1 |
| `review-k4s/ann_capture0` | D1 |
| `review-k4s/ann_capture1` | D1 |
| `review-k4s/ann_different0` | D1 |
| `review-k4s/ann_different1` | D1 |
| `review-k4s/ann_direct0` | D1 |
| `review-k4s/ann_direct1` | D1 |
| `review-k4s/ann_install0` | D1 |
| `review-k4s/ann_install1` | D1 |
| `review-k4s/ann_local0` | D1 |
| `review-k4s/ann_local1` | D1 |
| `review-k4s/ann_local_install0` | D1 |
| `review-k4s/ann_local_install1` | D1 |
| `review-k4s/ann_nested0` | D1 |
| `review-k4s/ann_nested1` | D1 |
| `review-k4s/ann_poly0` | D1 |
| `review-k4s/ann_poly1` | D1 |
| `review-k4s/ann_twice0` | D1 |
| `review-k4s/ann_twice1` | D1 |
| `review-k4s/ann_wrap0` | D1 |
| `review-k4s/ann_wrap1` | D1 |
| `review-k4s/captured_rows` | D1 |
| `review-k4s/chain_1024` | D1 |
| `review-k4s/chain_128` | D1 |
| `review-k4s/chain_256` | D1 |
| `review-k4s/chain_512` | D1 |
| `review-k4s/chain_64` | D1 |
| `review-k4s/co_0_0` | D1, D2 |
| `review-k4s/co_0_17` | D1, D2 |
| `review-k4s/co_12_0` | D1 |
| `review-k4s/co_12_17` | D1 |
| `review-k4s/enum_case` | D1 |
| `review-k4s/extra_closed_dep` | D1 |
| `review-k4s/extra_generic_never` | D1 |
| `review-k4s/extra_genuine_mutual_fn` | D1 |
| `review-k4s/extra_genuine_mutual_value` | D1 |
| `review-k4s/extra_genuine_two_rows` | D1 |
| `review-k4s/extra_pure_helper` | D1 |
| `review-k4s/extra_servicearg` | D1 |
| `review-k4s/extra_serviceboth` | D1 |
| `review-k4s/helper_ann_tmp` | D1 |
| `review-k4s/hidden_conflict_closure` | D1 |
| `review-k4s/hidden_conflict_install` | D1 |
| `review-k4s/hidden_conflict_nested` | D1 |
| `review-k4s/inner_body_ann` | D1 |
| `review-k4s/min_main_0_0` | D1, D4 |
| `review-k4s/min_main_0_1` | D1, D4 |
| `review-k4s/min_main_1_0` | D4 |
| `review-k4s/min_main_1_1` | D4 |
| `review-k4s/min_main_2_0` | D2, D4 |
| `review-k4s/min_main_2_1` | D4 |
| `review-k4s/min_main_3_1` | D4 |
| `review-k4s/min_main_4_0` | D4 |
| `review-k4s/min_main_4_1` | D4 |
| `review-k4s/min_main_5_1` | D4 |
| `review-k4s/min_main_6_0` | D1, D2, D4 |
| `review-k4s/min_main_6_1` | D1, D4 |
| `review-k4s/min_main_7_0` | D1, D4 |
| `review-k4s/min_main_7_1` | D1, D4 |
| `review-k4s/min_main_8_0` | D4 |
| `review-k4s/min_main_8_1` | D4 |
| `review-k4s/missing_scheme_main` | D1, D4 |
| `review-k4s/mixed_install_12` | D1 |
| `review-k4s/mixed_install_3` | D1 |
| `review-k4s/mixed_install_6` | D1 |
| `review-k4s/mixed_order_0` | D1, D2 |
| `review-k4s/mixed_order_1` | D1, D2 |
| `review-k4s/outer_body_ann` | D1 |
| `review-k4s/outer_closure_ann` | D1 |
| `review-k4s/outer_closure_var` | D1 |
| `review-k4s/outer_inner_local_ann` | D1 |
| `review-k4s/outer_no_helper` | D1 |
| `review-k4s/outer_no_sparam` | D1 |
| `review-k4s/principal_missing_retann` | D1 |
| `review-k4s/principal_missing_use` | D1 |
| `review-k4s/principal_missing_use_retann` | D1 |
| `review-k4s/runtime` | D1, D2 |
| `review-k4s/simple` | D1 |
| `review-k4s/soundness` | D4 |
| `review-k4s/soundness_unannotated` | D4 |
| `review-k4s/wide_128` | D1 |
| `review-k4s/wide_256` | D1 |
| `review-k4s/wide_32` | D1 |
| `review-k4s/wide_512` | D1 |
| `review-k4s/wide_64` | D1 |
| `review-k4s/wide_plain_1024` | D1 |
| `review-k4s/wide_plain_128` | D1 |
| `review-k4s/wide_plain_256` | D1 |
| `review-k4s/wide_plain_512` | D1 |
| `review-k4s/wide_plain_64` | D1 |
| `review-num/probes` | D6 |
| `review-num/trace` | D3, D6 |
| `review-r2/missing_service` | D4 |
| `review-r2/senders_1000` | D8 |
| `review-r2/senders_256` | D8 |
| `review-r2/senders_32` | D8 |
| `review-r2/senders_64` | D8 |
| `review-r2/tasks_1000` | D8 |
| `review-std2/client_nil_error` | D3 |
| `review-std2/client_no_marker` | D3 |
| `review-std2/error` | D3 |
| `review-std2/full` | D3 |
| `review-std2/full_closed` | D3 |
| `review-std2/minimal_client` | D3 |
| `review-std2/minimal_client_final` | D1 |
| `review-std2/pure_wrap` | D1 |
| `review-std2/replacement` | D3 |
| `review-std2/replacement_scope` | D3 |
| `review-std2/unsafe` | D9 |
| `review-t1/original-stop_tail_service` | D1 |
| `review-t1/service` | D2 |
| `review-r2/determinism` | B1 — held; expectation unchanged |
