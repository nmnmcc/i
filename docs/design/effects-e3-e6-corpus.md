# E3–E6 corpus integration audit

The final full observation uses main `5307a664e` as the pinned record base.
Main through `7c0652fad` is merged; the intervening commits change only
documentation and its gates, with identical compiler, library, corpus sources,
requirements and saved observations. The observed compiler/library tree is
`040109cb3` (release built after `62af4eac4`); the later commits add only
regression checks, ruled requirements, documentation and generated fixtures.

All **1,973 cases** ran with `--jobs 6`, `BINARYEN_CORES=1` and
`--keep-bounds`. Nine new resource censors received an isolated same-bound
retry. All nineteen revised ruling/type requirements pass their filtered
reruns. After the phase splices below, comparison with the pinned base gives:

| Observation class | Cases | Result |
|---|---:|---|
| Identical outcomes | 977 | Restore only compiler/binary fingerprint fields after proving every outcome and other metadata field equal; serialization equals the old bytes. |
| Display changes | 972 | Same phases, exits, diagnostic codes/counts/sites/predicates and runtime stdout; explanations below. This includes 260 cases with changed target-diagnostic wording. |
| Semantic changes | 24 | Every changed diagnostic or phase is covered by the case table and quoted paragraphs below. |
| Unexplained semantic changes | 0 | No unexplained acceptance, rejection or lost diagnostic is recorded. |

Every saved `bound_seconds` is unchanged. Six timed phases are restored by
the coordinator's `splice.py`. Four dependent runtime phases that could not
start after a censored checker retain their old observations individually.
These are **unverified phases**, not fresh passing runs. Completed phases of
the merged binary are retained, including when a later retry is censored.
No whole-file timeout restore is used.

The current requirement matcher, cloned from `scripts/qa/corpus.py` without
changes, rechecks all **395 applicable requirements** over the saved
observations: zero failures, the same two registered XFAILs. This is an audit
of observations after the required splices, not a claim to have uncensored
those four stress cases. Full gate commands and results are in
[the landing verification](effects-e3-e6-validation.md).

## Owner decisions and repaired defects

The controlling 2026-11-14 owner decisions are: **1**, “A need is a set of
environments”; **2**, “Drop the Fail(Never) universal-entry removal”; **9**,
finals may raise under the stated precedence; **15**, releases do not run on
a stop. R3/R4 below replace the old exclusive-kind and universal-removal
justifications. No final is installed or run on a stop.

Dropping the exclusive classifier restores all four original
`mixed_recursive` acceptance cases. Both `min_main` programs again require
`S` and reject every target entry phase. The old requirement demanding a
service-free main is replaced with an exact missing-S E0523 requirement.
The exact q25/q30/q35/q40 `fw` expectations gain only their explicitly written
`Fail(Never)`; their exact `use` types and runtime stdout assertions remain.
The other 32 q21–q40 requirements keep their original exact types. Four new
acceptance requirements independently pin mixed forwarding, and the unused
duplicate-provider case independently requires E0225.

The whole-service meet repair retains every concrete provider-subtyping
failure. Each b1 loses only the false summary at `6:1`; its four concrete
E0515 sites (`6:11`, `12:12`, `12:21`, `15:8`) and E0225 remain. Both m3 copies
lose only summary `10:2` and retain provider error `10:48`. Exact count/site
regressions, the fourteen-project provider guard and main's four-site
named-helper regression remain. A required Never value field still rejects;
a function field returning Never can be inhabited (R8).

The handler-witness repair substitutes only a proved closed equality into a
removed-provider mask before polarity simplification. The original exact
annotation-hole equality assertion remains, with a new incompatible-service
control; one-sided bounds stay open (§4.5: “Two types are equal when each is
a subtype of the other”). Ordinary value-difference display is unchanged.

The stored-callback repair restores the published Stream error covariance
and residual-need invariance, including Sink's ordinary `(N)->Nil` marker.
Std-1 §3.1: “A/E are covariant; N is invariant.” Std-2 §6.3 L5: “E is covariant;
N is invariant through the unused `(N)->Nil` marker”. The shipped-declaration
regression checks these actual types. The vendored snapshot receives the
same marker/annotation and grouped-suite-collector repairs, retaining every
test body, algorithm, error assertion and literal. It checks successfully;
its 1,446 collected test bindings and original bounded runtime censors remain.

Decision 2 also restores §9.5's exact `resource.ensure` type
`<A,N>(() <- N -> A, () <- N -> Nil) <- N -> A`. Internal pure kept scheduler
callbacks supply an actual `Fail(Never)` value; no key is erased or caller
handler captured in its place. The former final-error rejection fixture now
asserts the same `Error("final")` through `fail.catch` under Decision 9.
Decision 15's stop behavior remains unchanged.

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
negative tests for those operations remain. No new public environment type or syntax is invented by this repair;
the written captured-row operation restrictions remain separate from the
removed exclusive variable classifier.

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
`b10` and `z3` explicitly write `Fail(Never)`: the key is retained
under the revised R4. The R4-dependent justification is withdrawn. The
remaining changes are only the old independent error-channel cycle or hidden
obligation, not deletion of this real key; the separate recursive and concrete
provider failures stay. The exact fresh phase/site results are listed below. `z5` writes only
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

## Rejudged non-timeout cases

Category **a** is required by the quoted landed rules. Category **c** is a
stale vendored source migration. No expectation is justified merely by the
new compiler's output. The table records diagnostic multiplicity as well
as acceptance. It covers all 29 original non-timeout cases plus both m3
copies; repaired R3/R4 cases now agree with main and need no changed outcome. The source migration of `alias_kind_error` follows the shared
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
| `review-k4d/_loose/p/m3` | E0515 ×2 | retain concrete provider error `10:48`; retire only method-only empty-meet summary `10:2`; all check/run phases still reject | a, R8; exact site/count regression |
| `review-k4d/proj/m3` | E0515 ×2 | same retained provider error `10:48`, with check/run rejection unchanged | a, R8; exact site/count regression |
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
| `review-k4s/min_main_3_0` | entry E0523 | retain the required `S` key; entry rejects again | b, universal-erasure defect fixed; R4 / Decision 2 and §9.2 safety |
| `review-k4s/min_main_5_0` | entry E0523, W0504 | retain `S`, E0523 and the original warning | b, universal-erasure defect fixed; R4 / Decision 2 and §9.2 safety |
| `review-k4s/mixed_recursive_0_0` | accepted, W0504 | forwarding a value to the same row-typed formal accepts again; retain `S` need | b, overbroad mixed-use rejection repaired; R3 / Decision 1 and §9.4 bound values |
| `review-k4s/mixed_recursive_0_17` | accepted, W0504 | forwarding a value to the same row-typed formal accepts again; retain `S` need | b, overbroad mixed-use rejection repaired; R3 / Decision 1 and §9.4 bound values |
| `review-k4s/mixed_recursive_1_0` | accepted, W0504 | forwarding a value to the same row-typed formal accepts again; retain `S` need | b, overbroad mixed-use rejection repaired; R3 / Decision 1 and §9.4 bound values |
| `review-k4s/mixed_recursive_1_17` | accepted, W0504 | forwarding a value to the same row-typed formal accepts again; retain `S` need | b, overbroad mixed-use rejection repaired; R3 / Decision 1 and §9.4 bound values |
| `review-k4s/std` | checker accepts; entry tests resource-censored | restore main’s stream signatures, stream test type annotations and ordinary negative `Sink` marker under Decision 1; preserve E covariance and N invariance, explicit message `Fail(F)`, and grouped suite collection. Every test body, error assertion and saved bound remains. | c, R1/R7; std-1 §3.1 and std-2 L5 variance contracts |
| `review-std2/replacement_unused` | accepted, W0504 | E0225: two explicit `S` items remain invalid even when unused | a, R2 |


## Scheduler contract and resumed caller services

Spec §9.9 gives run the type
`<A,N>(() <- (..N,Scheduler,Runner,request.Await,request.Owner,request.Root) -> A) <- (..N,Runner) -> A`.
Scheduler is supplied to the body, not required of its caller. The adjacent
`test` type likewise supplies those routing services and leaves residual N,
Fail(E | ScheduleError) and Runner with the caller.

The library now captures the caller's residual N explicitly and invokes the
body through a callback with those closed routing services. Its call is
retained in a surviving identity frame. Spec §9.4 rule 3: “The paused part
keeps the services provided inside it”. Runtime §6.3: “Use a non-tail call
after those environment statements, through a surviving identity frame when
necessary.” This uses the published D16 workaround; it does not claim to fix
the compiler's general tail-provide gap. No release is run on a stop.

| Original leak case | Fresh full-corpus result |
|---|---|
| `review-native3/_loose/design-cases/events` | Every phase passes. Exact printed main is `() <- (Assert, Fail(Never), task.Runner) -> Nil`; Scheduler is gone and the genuine Fail key remains (Decision 2). |
| `review-r2/determinism` | Every phase passes; main needs `Fail(task.NotRunnable), Io, task.Runner`, with no Scheduler; outcomes match main. |
| `review-r2/never_yields` | Check and both test phases pass; both runs intentionally diverge with empty stdout at the original bound; no Scheduler; outcomes match main. |

Four §9.9 fixtures cover the run/test Runner-only contracts, rejection of an
unprovided Dep, the original events program with exact printed type, and a
Sample/Fail computation that yields three times before raising the same
`"resumed-run"` error. Targeted native checks pass O0/O2; the last case also
passes every JavaScript engine at both levels. The existing nested-run final
law retains every exact output and Error assertion. WAT evidence and all
123 Node / 69 browser results are in [the JS audit](effects-e4-js.md).

## Resource observations and phase splices

All nine newly censored cases were rerun alone at the exact saved bound.
`stress-keys-2000`, `cancel_error_final`, `nonportable_result`, `senders_1000`
and `tasks_1000` recover every required exit. Nonportable-result retains its
original run0/run2 exit 1 and passing test phases; the other four pass all
phases. No retry changes a program or an assertion.

| Remaining case | Bound | Fresh evidence and individually retained phases |
|---|---:|---|
| `review-k4/perf/wide_generic_1024` | 10s | Isolated check completes with the current principal types. test0/test2 remain censored; splice only those baseline phases. |
| `review-k4/perf/wide_generic_2048` | 60s | Both checker attempts censored. Splice the old check; carry its unstarted test0/test2 observations individually. No fresh completion is claimed. |
| `review-k4c/cases/stress_wide_1024` | 10s | Full-run check completes; test0/test2 censor. Isolated checker also censors. Keep the completed merged check and full-run censored phases, then splice only test0/test2. |
| `review-k4c/cases/stress_wide_open_1024` | 10s | Both checker attempts censored. Splice the old check and individually carry the unstarted test0/test2 observations. |

The full and isolated raw observations are retained locally before splicing.
The splice audit verifies each changed entry equals exactly the corresponding
baseline phase; every completed merged phase and every non-outcome field is
unchanged. `splice.py` restores six timed phases across these four files;
the four unstarted phases above are carried separately, never by restoring a
whole file. JSON formatting and fingerprint restoration change no observation.
The pre-existing vendored-std and std3-bench test censors remain censors;
intentional divergence cases retain their registered divergence requirements.

Commands (H is `/home/agent/Programming/i/.rd/cache/heavy`):

| Command | Result |
|---|---|
| `UPDATE=1 BINARYEN_CORES=1 $H direnv exec /home/agent/Programming/i scripts/review-corpus run --binary ./target/release/i --keep-bounds --jobs 6 --case-list target/e4-full-corpus-cases.json` | All 1,973 selected once: 554 rejected checks, 1,416 accepted, 3 checker censors, 2 baseline XFAILs. Initial failures are the subsequently replaced R4 requirements and resource censors. |
| `$H direnv exec /home/agent/Programming/i python3 -u target/e4-retry-corpus-censors.py` | Nine isolated resource retries, then 19 exact ruling/type guards; every guard passes. Child runs use `UPDATE=1`, `--jobs 1`, `--keep-bounds`, a one-case list and exact `--filter CASE`. Four stress cases remain censored as listed. |
| `python3 /home/agent/Programming/i/.rd/cache/splice.py` | Six timed phases across four records; complete phase-level provenance checked. |
| `python3 target/e4-current-corpus-audit.py --base 5307a664e --restore-fingerprints` | All bounds equal; 977 unchanged outcomes restore fingerprint fields byte-exactly; 972 display and 24 explained semantic cases. |
| `python3 target/e4-audit-saved-requirements.py` | Clone of the unmodified current matcher: all 395 applicable saved-observation requirements pass, 2 existing XFAILs. |
| `python3 target/e4-current-display-analysis.py` | All 972 display cases assigned to the groups below; no unclassified diagnostic prose or location. |

The runner's E0505 repair permits a cycle only when that case independently
requires E0505; b10 still independently requires its provider error. Five
Python regressions cover both errors, either missing error, unexpected cycles
and warning-only output. No semantic requirement is removed.

## Display groups

Each current display case has identical phase sets, exits, diagnostic code
multisets, source sites, diagnostic predicates and runtime stdout. Group D4
also covers changed diagnostic prose in rejected run/test phases. Groups overlap.
D10 describes restored real keys separately from carrier removal or bounds.

| Group | Current cases | Display change and controlling paragraph |
|---|---:|---|
| D1 | 845 | The independent error carrier disappears: opaque `(Fail(E), ..N)` callbacks become N, and complete callback rows combine directly. R1 (syntax-three §6.1: “One need row … err is removed”); §4.7 governs resulting inferred names. Written service keys remain. |
| D2 | 150 | Directed bounds and now-ordinary shared parameters simplify: S(N & A) becomes S(N), a consumer uses its incoming A, and redundant result/need variables coalesce. R1/R3; §4.7: “type variables that have no effect on the meaning are taken out”; §4.5 rule 4 and §9.2 preserve provider subtyping. D10 separately accounts for added real keys. |
| D3 | 9 | Only canonical variable names change; each scheme is equal up to renaming. R1 and §4.7 simplification; no nominal constructor changes. |
| D4 | 260 | The removed provider planner’s “missing service” / “must be closed” wording becomes the concrete constructor key “T is not provided by the target native”. Same E0523 counts, sites and rejected phases. R6 (syntax-three §2.10.11). |
| D5 | 3 | S(_, _) becomes S(String, _) in three rejected new_unsound cases. The incoming String bound is retained at the same provider E0515 site and the same mismatch with S(1, Never). R1; §4.7: a nongeneralized variable shows the union of incoming types; §9.2 still requires provider subtyping. |
| D6 | 2 | Boundary(R,E,N) becomes Boundary(R,N), with any Fail entry in the single N. Syntax-three §2.10.7 explicitly specifies the new control type arities; R1 removes the separate error parameter. |
| D8 | 8 | Printed task.Scope(E) becomes task.Group(E). Syntax-three §2.10.9 explicitly renames the concurrency handle so Scope is the resource service; §9.9 uses Group(E). Every runtime assertion and exit stays. |
| D10 | 172 | Explicit Fail(Never) appears in needs and mismatch targets instead of being erased, including E0504 `_ | _` becoming `Fail(Never) | _ | _` and E0501 `S(Dep)` becoming `Fail(Never) | S(Dep)`. R4 / Owner Decision 2; §9.2: “Getting it never fails at run time”. Same errors and sites; no service key is manufactured or removed. |

The former **D9** explanation (universal key erasure) is withdrawn. Both wide
key inventories and Consumer(Never) remain real requirements. The former **B1**
Scheduler leak is repaired as above; determinism is identical to main again.
The original nine-group labels are replaced below by their current judgments.

## Original 919-case display inventory

Of these original cases, 907 currently have explained display changes and 12
match the pinned main record (some retain explicitly listed resource phases).
Every original identity is kept. A group label refers to the exact change and
quoted rule above; resource notes identify observations not freshly completed.

| Original case | Current explanation |
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
| `review-k4/cases/generic_conflict` | D1, D10, D2 |
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
| `review-k4/cases/multi` | D1, D10 |
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
| `review-k4/perf/wide_generic_1024` | D1; resource phases retained as listed above |
| `review-k4/perf/wide_generic_128` | D1 |
| `review-k4/perf/wide_generic_2048` | Identical outcome; no expectation update; resource phases retained as listed above |
| `review-k4/perf/wide_generic_256` | D1 |
| `review-k4/perf/wide_generic_512` | D1 |
| `review-k4/perf/wide_generic_64` | D1 |
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
| `review-k4c/cases/missing_2_0_1_0` | D1, D10, D2, D4 |
| `review-k4c/cases/missing_2_0_1_1` | D10, D2, D4 |
| `review-k4c/cases/missing_2_0_1_2` | D1, D10, D2, D4 |
| `review-k4c/cases/missing_2_1_0_0` | D1, D4 |
| `review-k4c/cases/missing_2_1_0_1` | D1, D4 |
| `review-k4c/cases/missing_2_1_0_2` | D1, D4 |
| `review-k4c/cases/missing_2_1_1_0` | D1, D10, D2, D4 |
| `review-k4c/cases/missing_2_1_1_1` | D10, D2, D4 |
| `review-k4c/cases/missing_2_1_1_2` | D1, D10, D2, D4 |
| `review-k4c/cases/missing_2_2_0_0` | D1, D4 |
| `review-k4c/cases/missing_2_2_0_1` | D1, D4 |
| `review-k4c/cases/missing_2_2_0_2` | D1, D4 |
| `review-k4c/cases/missing_2_2_1_0` | D1, D10, D2, D4 |
| `review-k4c/cases/missing_2_2_1_1` | D10, D2, D4 |
| `review-k4c/cases/missing_2_2_1_2` | D1, D10, D2, D4 |
| `review-k4c/cases/missing_2_3_0_0` | D1, D4 |
| `review-k4c/cases/missing_2_3_0_1` | D1, D4 |
| `review-k4c/cases/missing_2_3_0_2` | D1, D4 |
| `review-k4c/cases/missing_2_3_1_0` | D1, D10, D2, D4 |
| `review-k4c/cases/missing_2_3_1_1` | D10, D2, D4 |
| `review-k4c/cases/missing_2_3_1_2` | D1, D10, D2, D4 |
| `review-k4c/cases/missing_2_4_0_0` | D1, D2, D4 |
| `review-k4c/cases/missing_2_4_0_1` | D1, D2, D4 |
| `review-k4c/cases/missing_2_4_0_2` | D1, D2, D4 |
| `review-k4c/cases/missing_2_4_1_0` | D1, D10, D2, D4 |
| `review-k4c/cases/missing_2_4_1_1` | D10, D2, D4 |
| `review-k4c/cases/missing_2_4_1_2` | D1, D10, D2, D4 |
| `review-k4c/cases/missing_2_5_0_0` | D1, D4 |
| `review-k4c/cases/missing_2_5_0_1` | D1, D4 |
| `review-k4c/cases/missing_2_5_0_2` | D1, D4 |
| `review-k4c/cases/missing_2_5_1_0` | D1, D10, D2, D4 |
| `review-k4c/cases/missing_2_5_1_1` | D10, D2, D4 |
| `review-k4c/cases/missing_2_5_1_2` | D1, D10, D2, D4 |
| `review-k4c/cases/missing_2_6_0_0` | D1, D4 |
| `review-k4c/cases/missing_2_6_0_1` | D1, D4 |
| `review-k4c/cases/missing_2_6_0_2` | D1, D4 |
| `review-k4c/cases/missing_2_6_1_0` | D1, D10, D2, D4 |
| `review-k4c/cases/missing_2_6_1_1` | D10, D2, D4 |
| `review-k4c/cases/missing_2_6_1_2` | D1, D10, D2, D4 |
| `review-k4c/cases/missing_2_7_0_0` | D1, D4 |
| `review-k4c/cases/missing_2_7_0_1` | D1, D4 |
| `review-k4c/cases/missing_2_7_0_2` | D1, D4 |
| `review-k4c/cases/missing_2_7_1_0` | D1, D10, D2, D4 |
| `review-k4c/cases/missing_2_7_1_1` | D10, D2, D4 |
| `review-k4c/cases/missing_2_7_1_2` | D1, D10, D2, D4 |
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
| `review-k4c/cases/missing_3_3_0_0` | D1, D10, D4 |
| `review-k4c/cases/missing_3_3_0_1` | D1, D10, D4 |
| `review-k4c/cases/missing_3_3_0_2` | D1, D10, D4 |
| `review-k4c/cases/missing_3_3_1_0` | D1, D10, D2, D4 |
| `review-k4c/cases/missing_3_3_1_1` | D10, D2, D4 |
| `review-k4c/cases/missing_3_3_1_2` | D1, D10, D2, D4 |
| `review-k4c/cases/missing_3_4_0_0` | D1, D10 |
| `review-k4c/cases/missing_3_4_0_1` | D1, D10 |
| `review-k4c/cases/missing_3_4_0_2` | D1, D10 |
| `review-k4c/cases/missing_3_4_1_0` | D1, D10 |
| `review-k4c/cases/missing_3_4_1_1` | D1, D10 |
| `review-k4c/cases/missing_3_4_1_2` | D1, D10 |
| `review-k4c/cases/missing_3_5_0_0` | D1, D4 |
| `review-k4c/cases/missing_3_5_0_1` | D1, D4 |
| `review-k4c/cases/missing_3_5_0_2` | D1, D4 |
| `review-k4c/cases/missing_3_5_1_0` | D1, D4 |
| `review-k4c/cases/missing_3_5_1_1` | D1, D4 |
| `review-k4c/cases/missing_3_5_1_2` | D1, D4 |
| `review-k4c/cases/missing_3_6_0_0` | D1, D10, D4 |
| `review-k4c/cases/missing_3_6_0_1` | D1, D10, D4 |
| `review-k4c/cases/missing_3_6_0_2` | D1, D10, D4 |
| `review-k4c/cases/missing_3_6_1_0` | D1, D10, D2, D4 |
| `review-k4c/cases/missing_3_6_1_1` | D10, D2, D4 |
| `review-k4c/cases/missing_3_6_1_2` | D1, D10, D2, D4 |
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
| `review-k4c/cases/stress_wide_1024` | Identical outcome; no expectation update; resource phases retained as listed above |
| `review-k4c/cases/stress_wide_128` | Identical outcome; no expectation update |
| `review-k4c/cases/stress_wide_32` | Identical outcome; no expectation update |
| `review-k4c/cases/stress_wide_512` | Identical outcome; no expectation update |
| `review-k4c/cases/stress_wide_open_1024` | Identical outcome; no expectation update; resource phases retained as listed above |
| `review-k4c/cases/stress_wide_open_128` | D1 |
| `review-k4c/cases/stress_wide_open_32` | D1 |
| `review-k4c/cases/stress_wide_open_512` | D1 |
| `review-k4c/cases/two_run` | D10, D2 |
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
| `review-k4d/_loose/p/g_ann_Never` | Identical outcome; no expectation update |
| `review-k4d/_loose/p/g_inf_Never` | Identical outcome; no expectation update |
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
| `review-k4d/_loose/p/x8` | D10, D2 |
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
| `review-k4d/fuzz/pl44` | D1, D2 |
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
| `review-k4d/perm/p0` | D1, D10 |
| `review-k4d/perm/p1` | D1, D10 |
| `review-k4d/perm/p10` | D1, D10 |
| `review-k4d/perm/p11` | D1, D10 |
| `review-k4d/perm/p12` | D1, D10 |
| `review-k4d/perm/p13` | D1, D10 |
| `review-k4d/perm/p14` | D1, D10 |
| `review-k4d/perm/p15` | D1, D10 |
| `review-k4d/perm/p16` | D1, D10 |
| `review-k4d/perm/p17` | D1, D10 |
| `review-k4d/perm/p18` | D1, D10 |
| `review-k4d/perm/p19` | D1, D10 |
| `review-k4d/perm/p2` | D1, D10 |
| `review-k4d/perm/p20` | D1, D10 |
| `review-k4d/perm/p21` | D1, D10 |
| `review-k4d/perm/p22` | D1, D10 |
| `review-k4d/perm/p23` | D1, D10 |
| `review-k4d/perm/p24` | D1, D10 |
| `review-k4d/perm/p25` | D1, D10 |
| `review-k4d/perm/p26` | D1, D10 |
| `review-k4d/perm/p27` | D1, D10 |
| `review-k4d/perm/p28` | D1, D10 |
| `review-k4d/perm/p29` | D1, D10 |
| `review-k4d/perm/p3` | D1, D10 |
| `review-k4d/perm/p30` | D1, D10 |
| `review-k4d/perm/p31` | D1, D10 |
| `review-k4d/perm/p32` | D1, D10 |
| `review-k4d/perm/p33` | D1, D10 |
| `review-k4d/perm/p34` | D1, D10 |
| `review-k4d/perm/p35` | D1, D10 |
| `review-k4d/perm/p36` | D1, D10 |
| `review-k4d/perm/p37` | D1, D10 |
| `review-k4d/perm/p38` | D1, D10 |
| `review-k4d/perm/p39` | D1, D10 |
| `review-k4d/perm/p4` | D1, D10 |
| `review-k4d/perm/p40` | D1, D10 |
| `review-k4d/perm/p41` | D1, D10 |
| `review-k4d/perm/p42` | D1, D10 |
| `review-k4d/perm/p43` | D1, D10 |
| `review-k4d/perm/p44` | D1, D10 |
| `review-k4d/perm/p45` | D1, D10 |
| `review-k4d/perm/p46` | D1, D10 |
| `review-k4d/perm/p47` | D1, D10 |
| `review-k4d/perm/p48` | D1, D10 |
| `review-k4d/perm/p49` | D1, D10 |
| `review-k4d/perm/p5` | D1, D10 |
| `review-k4d/perm/p50` | D1, D10 |
| `review-k4d/perm/p51` | D1, D10 |
| `review-k4d/perm/p52` | D1, D10 |
| `review-k4d/perm/p53` | D1, D10 |
| `review-k4d/perm/p54` | D1, D10 |
| `review-k4d/perm/p55` | D1, D10 |
| `review-k4d/perm/p56` | D1, D10 |
| `review-k4d/perm/p57` | D1, D10 |
| `review-k4d/perm/p58` | D1, D10 |
| `review-k4d/perm/p59` | D1, D10 |
| `review-k4d/perm/p6` | D1, D10 |
| `review-k4d/perm/p7` | D1, D10 |
| `review-k4d/perm/p8` | D1, D10 |
| `review-k4d/perm/p9` | D1, D10 |
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
| `review-k4d/proj/g_ann_Never` | Identical outcome; no expectation update |
| `review-k4d/proj/g_inf_Never` | Identical outcome; no expectation update |
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
| `review-k4d/proj/x8` | D10, D2 |
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
| `review-k4s/runtime` | D10, D2 |
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
| `review-r2/missing_service` | D10, D4 |
| `review-r2/senders_1000` | D10, D8 |
| `review-r2/senders_256` | D10, D8 |
| `review-r2/senders_32` | D10, D8 |
| `review-r2/senders_64` | D10, D8 |
| `review-r2/tasks_1000` | D10, D8 |
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
| `review-std2/unsafe` | Identical outcome; no expectation update |
| `review-t1/original-stop_tail_service` | D1 |
| `review-t1/service` | D2 |
| `review-r2/determinism` | B1 repaired; identical need, exits and output |

## Additional current display cases

The latest base and owner-ruling repairs expose these 65 further display
changes. The exact same phase/exit/error-site/runtime-output checks apply.

| Case | Display explanation |
|---|---|
| `review-k4-lane/q25` | D10, D2 |
| `review-k4-lane/q30` | D10, D2 |
| `review-k4-lane/q35` | D10, D2 |
| `review-k4-lane/q40` | D10, D2 |
| `review-k4-lane/w11-Dep` | D10, D2 |
| `review-k4-lane/w11-Never` | D10, D2 |
| `review-k4-lane/w9-Dep` | D10, D2 |
| `review-k4-lane/w9-Never` | D10, D2 |
| `review-k4c/cases/two_env` | D10 |
| `review-k4d/_loose/p/h1` | D10 |
| `review-k4d/_loose/p/q25` | D10 |
| `review-k4d/_loose/p/q30` | D10 |
| `review-k4d/_loose/p/q35` | D10 |
| `review-k4d/_loose/p/q40` | D10 |
| `review-k4d/_loose/p/w11` | D10 |
| `review-k4d/_loose/p/w9` | D10 |
| `review-k4d/_loose/p/x1` | D10, D2 |
| `review-k4d/_loose/p/x4` | D10, D2 |
| `review-k4d/_loose/p/x6` | D10, D2 |
| `review-k4d/_loose/p/x9` | D10, D2 |
| `review-k4d/_loose/p/z1` | D10 |
| `review-k4d/_loose/p/z4` | D10 |
| `review-k4d/proj/h1` | D10 |
| `review-k4d/proj/q25` | D10 |
| `review-k4d/proj/q30` | D10 |
| `review-k4d/proj/q35` | D10 |
| `review-k4d/proj/q40` | D10 |
| `review-k4d/proj/w11` | D10 |
| `review-k4d/proj/w9` | D10 |
| `review-k4d/proj/x1` | D10, D2 |
| `review-k4d/proj/x4` | D10, D2 |
| `review-k4d/proj/x6` | D10, D2 |
| `review-k4d/proj/x9` | D10, D2 |
| `review-k4d/proj/z1` | D10 |
| `review-k4d/proj/z4` | D10 |
| `review-k4s/min_main_3_0` | D4 |
| `review-k4s/min_main_5_0` | D4 |
| `review-k4s/mixed_recursive_0_0` | D1 |
| `review-k4s/mixed_recursive_0_17` | D1 |
| `review-k4s/mixed_recursive_1_0` | D1 |
| `review-k4s/mixed_recursive_1_17` | D1 |
| `review-k4s/std` | D1, D10, D2 |
| `review-native3/_loose/design-cases/events` | D10 |
| `review-native3/_loose/design-cases/paths` | D10 |
| `review-r2/cancel_choose` | D10 |
| `review-r2/choice_commit` | D10 |
| `review-r2/dag_16` | D10 |
| `review-r2/dag_20` | D10 |
| `review-r2/dag_24` | D10 |
| `review-r2/dag_28` | D10 |
| `review-r2/dag_28_local` | D10 |
| `review-r2/deadlock` | D10 |
| `review-r2/deep_1000` | D10 |
| `review-r2/deep_200` | D10 |
| `review-r2/deep_50` | D10 |
| `review-r2/escaped_task` | D10 |
| `review-r2/lifetime` | D10 |
| `review-r2/local_capture` | D10 |
| `review-r2/nonportable_result` | D10 |
| `review-r2/race_tie` | D10 |
| `review-r2/request_groups` | D10 |
| `review-r2/senders_128` | D10, D8 |
| `review-r2/senders_16` | D10, D8 |
| `review-r2/shield_nested` | D2 |
| `review-r2/tasks_5000` | D10, D8 |
