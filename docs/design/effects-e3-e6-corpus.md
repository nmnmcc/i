# E3–E6 corpus integration audit

Base: main `1bdea69406471b72e6d1df3a5592c075688478b8`, merged through
`gate/g23` (`bddbb4b59`). The coordinator's 1,973-case run classified 1,009
observations as fingerprint-only, 919 as display-only, and 45 otherwise.
Sixteen of the 45 have resource timeouts; the remaining 29 are listed below.
The timeout reruns retain each saved `bound_seconds`; no bound is raised.

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

**R3 — Row/value separation.** Spec §4.7: “A type parameter used where a
need is written (`<- N`, `<- (..N, …)`) is a **row variable**: it stands for a
whole set of services. Any other use makes it an ordinary type variable.
Using one name both ways is a compile error.” Spec §9.4: “A value of a row
type may be bound, captured and provided after `<-`, alone or as a spread
in provided data; it may not be spread into ordinary data, compared,
printed or matched.” Returning an environment row as ordinary data is
likewise outside the stated row-value operations.

**R4 — Universal service forms.** Spec §9.2: “An entry that every value of
its type constructor satisfies, as `Fail(Never)`, is removed.” Spec §4.7:
“A type is made simpler before it is shown: equal parts are put together,
and type variables that have no effect on the meaning are taken out. This
never changes which programs are correct or what they do.” For an unused
head `s: S(N)` where `S(N) = (() <- N -> Nil)`, the unconstrained one-sided
row argument reaches the universal form of `S`; the callback is never
called. Both `min_main` cases therefore simplify to `() -> Nil`. Calling
the service, as in `min_main_6_0`, retains `S`; the regression checks this
negative control. This is general universal-entry removal, not a special
case granting an application service to the platform.

**R5 — Removed channel obligations.** R1 removes the extra error-channel
constraint, not an incompatible service constraint. Spec §9.2 still says:
“A provided value of type `P` satisfies the need entry `S` when `P <: S`
(§4.5 rule 4).” Its preceding paragraph still requires E0515 for an empty
service intersection. The `b10` recursive definition retains E0505 and its
provider retains E0515; the secondary cycle through the deleted carrier
does not exist. The `z3`/`z5` concrete provider errors all remain; the
additional summary derived from that carrier disappears. The earlier
[E6 audit](e6-main-corpus-audit.md) describes their exact remaining sites.
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
| `review-k4d/_loose/p/b1` | E0515 ×5 | same five + E0225 for duplicate explicit `S` | a, R2 |
| `review-k4d/proj/b1` | E0515 ×5 | same five + E0225 for duplicate explicit `S` | a, R2 |
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
| `review-k4s/min_main_3_0` | entry E0523 | `main : () -> Nil`, entry runs; unused universal service form removed | a, R4 |
| `review-k4s/min_main_5_0` | entry E0523, W0504 | `main : () -> Nil`, entry runs; same W0504 remains | a, R4 |
| `review-k4s/mixed_recursive_0_0` | accepted, W0504 | E0225: `N` is both a callback need and the ordinary parameter type | a, R3 |
| `review-k4s/mixed_recursive_0_17` | accepted, W0504 | E0225: same mixed row/value parameter | a, R3 |
| `review-k4s/mixed_recursive_1_0` | accepted, W0504 | E0225: same mixed row/value parameter | a, R3 |
| `review-k4s/mixed_recursive_1_17` | accepted, W0504 | E0225: same mixed row/value parameter | a, R3 |
| `review-k4s/std` | checker accepts; entry tests resource-censored | migrate the unused `Sink` marker from `(N) -> Nil` to `(() <- N -> Nil) -> Nil`, retaining its original invariant row parameter; write the message callback's `Fail(F)` explicitly; port grouped suite collection. Preserve every test body and saved bound. | c, R1/R3/R7 |
| `review-std2/replacement_unused` | accepted, W0504 | E0225: two explicit `S` items remain invalid even when unused | a, R2 |

## Corpus runner repair

The existing `b10` requirements separately require E0505 and E0515. The
runner unconditionally excluded E0505 even when explicitly required, making
both requirements impossible to satisfy. Allow a cycle only when this case
has an explicit cycle requirement; still require the independent provider
error, and continue rejecting an unexpected cycle in every other case.
Five Python regressions cover both errors, either missing error, unexpected
cycles, and warning-only output. No semantic requirement is removed.

## Verification and display groups

The release rebuild, targeted checker regressions, same-bound timeout
reruns, per-group update runs, exact display classifications and
fingerprint-only restoration are recorded here when complete. Until then
this section is an audit in progress, not a claim that the corpus gate is green.
