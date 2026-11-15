# E6 integration with converted main: filtered corpus observations

The 111 cases selected by `review-k4d/_loose/p` were run after merging main
`13488d07` into E6. Every phase exit and every runtime stdout is unchanged.
All existing semantic requirements passed before updating observations.
62 saved observations differ from converted main; no source case is changed.
The 15 error/warning lines of the checker soundness golden remain verbatim
from main, including all four Reader(String)/Reader(Int) E0515 diagnostics.

The observations change for these reasons:

* E3 removes the synthetic error channel. A callback with opaque need `N`
  keeps that single need; it does not acquire a separately generalized
  `Fail(E)`. Most differences are this display and the resulting type-variable
  numbering. Spec §4.6: “Errors are the need entry `Fail(E)` (§9.3), so
  `Effect<A, E, R>` of Effect-TS is `() <- (..R, Fail(E)) -> A`.”
  Syntax-three §6.1: “**One need row.** `err` is removed (stage E3): errors
  are the `Fail(E)` entry of `env`.”
* E6's directed occurrence simplification removes redundant service-argument
  intersections, for example `State(B & A)` becomes `State(A)` for a read-only
  state. These cases are `b8`, `b9`, `d2`, `e1`, `e2`, `p1`, `q1`, and `q6`.
  Spec §4.7: “A type is made simpler before it is shown: equal parts are put together,
  and type variables that have no effect on the meaning are taken out.” Spec §9.2: “A provided value of type `P` satisfies
  the need entry `S` when `P <: S` (§4.5 rule 4).”
* E5 replaces provider-plan wording with the target's missing-service error.
  All twelve affected cases retain E0523, its source location, and failure
  in every corresponding O0/O2 run/test phase. Syntax-three §2.10.11:
  target services are supplied by the entry stub; other needs must be provided
  by i code. No target promises an application-defined `S` or `Dep`.
* `b1` and `r6` additionally report E0225 for two explicit `S` values in one
  written provide. E6 preserves the written provide group rather than treating
  its entries as nested overrides. All prior diagnostics remain. Spec §9.2:
  “An explicit item wins over an item from a spread with the same key; two
  explicit items, or two spreads, with one key are a compile error.”
* `b10` retains E0505 at the recursive `bad` definition and E0515 at its
  incompatible provider. The secondary E0505 propagated to `main` through the
  old error-channel representation disappears. The source's `Fail(Never)` is
  an ordinary vacuous need, not an independent error-channel constraint.
  Spec §9.2: “An entry that every value of its type constructor satisfies,
  as `Fail(Never)`, is removed.” Spec §4.7: “A type that has itself inside it,
  through a type variable, is a compile error.” Both rejection codes now have
  explicit independent semantic requirements.
* `z3` and `z5` retain their actual incompatible-service E0515 diagnostics;
  the additional unexpressed-handler summary at the `wrap` definition is no
  longer emitted. `z3` still reports the provider mismatch at `wrap` (6:1)
  and the outer provide (19:8). `z5` still reports the inner callback mismatch
  (11:14), its call (20:5), and the outer provider (24:8). The single-row
  schemes already express the service obligation; they do not also need an
  error-channel-derived hidden obligation. Spec §9.2's provided-value subtype
  rule is the rejection criterion. Explicit E0515 rejection requirements
  were added for both cases; no incompatible provider becomes accepted.
* `g_ann_Never` and `g_inf_Never` retain E0501 at 16:2. The unknown form is
  displayed as `_` instead of `S(_)` after inference through the single need
  row; the explicit `Never` contract still rejects the provider.

Observation updates retain every phase, saved time budget and input file.
Independent requirements were strengthened for `b1`, `b10`, `r6`, `z3`, and
`z5`. The complete before/after type diff and machine-checked exit/stdout audit
are preserved with the lane's verification logs under `target/e6-main-merge`.
