# Stage 3 full-corpus audit

The coordinator checked 1,973 cases on gate `cc8031eb` (main `1bdea694`
plus stage 3). This follow-up preserves case intent and phase sets. The
coordinator owns the O2 budget timeouts; none of those observations is
updated here. Every re-recording uses `UPDATE=1`, the shared heavy wrapper,
an exact per-case `--filter`, and `--keep-bounds`. Metadata-only changes
are restored to the pre-run file.

## Service migrations

| Case | Old | New | Controlling paragraph |
| --- | --- | --- | --- |
| `review-js3/capability` | Accepted before stage 3; a union alias is now E0225 as a service. | `TargetKind` retains all five variants; `Target = (TargetKind)` is the single-constructor service. `Target(NodeJS(_))` and `Target(Native)` retain capability selection; `TargetKind(..)` retains variant visibility. Original check and both test phases remain. | Syntax-three §2.8 kinds 3/4 and Constructors; spec §4.3, §9.2. |
| `review-js4/rebinding` | Accepted and ran; union-as-service now gives E0225. | Same named wrapper; captured `Target(Web)` remains Web while the call receives `Target(Native)`. Keep the assertion `(1, 2)` and both run phases plus both test phases. | Same type rules; spec §9.2 Calls/Providing and captured-service paragraph. |

Syntax-three §2.8 says: “The left side is a new nominal type and its one
constructor, with the same name.” For other right sides: “A name is a
transparent alias, as today.” And: “A labelled type with data `D` has the
constructor function `(D) -> T(…)`.” Thus the former sum alias is not one
nominal service, while the new data wrapper is.

Spec §9.2: “A **service** is an ordinary value of a labelled type (§4.3),
normally one whose data holds functions. Two services of one form are made
by defining two types. A service is a normal value: it may be stored and
given to functions, and code that has it may call its functions directly.”
The Providing paragraph requires: “A provided item is keyed by its known
type (§4.8), which must be one named service type: not a union, a type
variable, or unknown.” Its Calls paragraph says: “So a function made in
one place and called in another gets its services from the place where it
is called.” The captured-service paragraph says: “A service value can be
captured. Providing the service later changes calls that take it from their
caller, not the captured value.” The rebinding test retains that distinction.

## Newly provable unused clause

| Case | Old | New | Controlling paragraph |
| --- | --- | --- | --- |
| `review-k4/cases/wrapper_10_False_False` | Two W0504; no W0501. | Add W0501 for `(False) { use() }` in `f`, after the literal `True`. All exits and runtime output retained. | Syntax-three §2.8; spec §4.3, §4.5 rule 3, §7.4. |
| `review-k4/cases/wrapper_10_False_True` | Same. | Same clause and warning in every phase. | Same. |
| `review-k4/cases/wrapper_10_True_False` | Same. | Same clause and warning in every phase. | Same. |
| `review-k4/cases/wrapper_10_True_True` | Same. | Same clause and warning in every phase. | Same. |

Spec §7.4: “The branches of a shape group whose parameter has the type `A`
**cover** `A` when, for every value of `A`, some branch without a guard has
a parameter group that matches it. A branch with a guard is not counted.
A branch is **never used** when every value that its pattern matches is
matched by an earlier branch without a guard.”

The domain is now the known type `True`, whose only constructor is True;
`False` is a distinct type (§4.3), disjoint by §4.5 rule 3. The preceding
True arm covers that entire domain, leaving no input for the False arm.
Before stage 3, the temporary multi-constructor lowering gave the True
constructor the family type Bool, which also contained False. The type
checker therefore did not prove the False arm useless. The arm was already
dynamically unreachable; this is new static precision, not changed runtime
selection. Branch bodies and shared needs remain checked (§9.2 Branches).

## Equality warnings

| Case | Old | New | Controlling paragraph |
| --- | --- | --- | --- |
| `review-k4d/_loose/spec/tests_spec_11_equality` | Four scalar W0503 per phase. | Seven: add Some/None, Ok/Error, True/False at lines 57/59/62. Already recorded in `7a7d7df01`; verify again and restore fingerprint-only changes. | Syntax-three §2.8; spec §4.3, §4.5 rule 3, §6.9. |
| `review-t1/original-equality` | Four scalar W0503 per phase. | The same three added warnings at lines 57/59/62; all five equality tests still pass at both optimizations. | Same. |

Spec §6.9: “When both sides of `==` or `!=` have known types (§4.8), and
these are disjoint (§4.5 rule 3), the compiler gives warning W0503, showing
both types: the two sides can never be equal.”

Before stage 3 each pair shared a family type constructor (Option, Result,
or Bool); the top-member disjointness test could not distinguish its
constructors. Now Some and None, Ok and Error, True and False are distinct
nominal types, even where their data is identical. Their constructor values
have those own types (§2.8), so the three comparisons are provably disjoint.
The comparisons were already false; the warnings add no narrowing fact and
their Bool results and runtime output are unchanged (§6.9, §11.4).

## Display-only groups

The guard reports 39 display-only cases, with unchanged exits, diagnostic
code multisets and runtime output. The guard text omits their names. A saved
case list/diff has been requested because the coordinator worktrees now
contain restored observations; these cases must be enumerated and explained
before updating their records.
