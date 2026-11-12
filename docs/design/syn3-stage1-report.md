# Syntax three, stage 1: recursive matches over unions of one-constructor types

Stage 1 of the plan in `docs/design/syntax-3.md` §7.2 (B2 of the evaluation
in `.rd/cache/gpt/report-eval-syn3.md`), and the display part (D7).

## The shape that was slow

```
C0 { C0 }
C1(A) { C1(A, E(A)) }
…
E(A) = C0 | C1(A) | … | Cn(A)
f0 = (e) { e |> { C0 -> 0  C1(a, r) -> a + f0(r)  … } }
```

## Diagnosis

Measured with a debugger sampling the release `i check` of 9 members (most
samples in `coverage::values`, the rest in `display::Norm::walk`/`record`),
then with counters behind a temporary environment variable (removed).

The constraint graph itself is polynomial. Each clause `Ci(a, r)` has a field
variable `ai`; the recursive call gives `E(ai) <: e`, and the routing of every
`Cj(ai)` into `e`'s upper bound `… | Cj(aj) | …` makes the `ai` mutually bounded
(`ai <: aj` for all i, j). `add_bound` compares lower bounds of one head and
joins their covariant arguments into a fresh variable (`join`), so every pair
`ai, aj` gets about n join variables `w` with `ai <: w`, `aj <: w`: O(n³)
variables (dump at 6 members: 5 field variables, 125 join variables).

Three places then cost more than polynomial or a high polynomial:

1. **Coverage (`coverage::values`), exponential.** The values of a variable
   were found by a search that cuts a cycle only on the current path
   (`seen.contains` / `seen.pop()`), with no memo. On the dense graph of
   the `ai` (upper bounds in `Up` mode) the search walks every simple path.
   Calls of `values` per `i check`: 6 members 0.76 M, 7 members 7.7 M,
   8 members 88 M (×10 per member, the n! growth); 1.5 M calls for a single
   column at 8 members.
2. **Simplification (`display::simplify_steps`), O(steps × size × k²).**
   The coalesced parameter type has every `ai` with an intersection of all its
   join variables: about 0.31 n³ variables that occur only negatively (65 at 6
   members, 166 at 8, 632 at 12). Each is removed by one step, and each step
   re-walks the whole type; the walk's `record` intersects the co-occurrence
   list of every occurrence with `Vec::contains` (k² equality tests on groups
   of k ≈ n²/2 members).
3. **A wrong result above 64 such variables.** The step loop stopped after 64
   steps, so from 8 members on `f0` was shown as
   `(e: C0 | C1(Int & A & B & … & M2) | …) -> O2 | Int ^ E2 ! N` instead of the
   simplified type.

## Fix

- `coverage.rs`: the search of `values` is now `Walk`, with three exact
  shortcuts that keep the results of the path search (the same heads in the
  same order):
  - a *closed* search, one that goes from variable to variable only through
    the operation of its mode (union in `Pos`, intersection in `Up`) and whose
    other parts (for example `Never | r` in `Up` mode) lead to searches that
    never come back, visits each variable once (a variable met again adds
    the identity of the operation, which the path search also adds, as its
    first visit already contributed everything reachable). It is used only
    when none of its variables is on the current path;
  - a search that met no variable of its path (no cut) is reused wherever
    none of the variables it reached is on the path;
  - these results are kept (`Searches`, in the checker) while the solver's
    bounds are unchanged: stamp of the variable buffer, the number of
    variables, of `touched` entries, and a new `Solver::rewrites` counter
    bumped by `normalize_bounds` and `mark_inputs`.
- `display.rs`:
  - the co-occurrence members of a variable are kept as (group, index) and
    computed only when a rule asks for them (hash-set intersection);
  - `steps` takes, in one pass, the leading decisions that only drop a
    variable from groups that keep at least two members (a negative-only
    variable from intersections, a positive-only one from unions). Such a
    drop changes no other variable's counts, only removes the dropped
    variable from their members, which `decide` excludes, so the decisions
    are the ones the one-at-a-time loop makes; the first other decision ends
    the batch. `trace` replays the same batches;
  - the 64-step limit is removed (each step removes a variable, so the loop
    ends);
  - `Norm::inter` finds repeated members with a hash set (there is at most one
    headed member) instead of `Vec::contains`.

Inference is unchanged; `i check --types --project std` gives byte-identical
output before and after.

## Display by declared alias (D7)

`AliasFold` (display.rs): an alias whose expansion is a union of `Con`
members with distinct heads, with no holes and every parameter used, is
recorded in `Solver::alias_folds` when its template is assembled (alias.rs).
`prepare` (every named display: `--types`, hovers, `scheme_in`) and `show`
(error messages) replace a union that is exactly such an expansion, matching
the members by head and binding the parameters, by `Alias(args)`. When several
aliases fit, an alias whose shown name has no module path is required (in
scope), and then the one whose name needs no module qualifier, then the
shortest name, then the first declared. Interpretation: "written in the
source of the binding" is approximated by "named without a module" because
display has no access to the binding's annotations.

`first = (xs: List(Option(Int))) -> Int {…}` now shows
`first : (xs: List(Option(Int))) -> Int`; the scaling fixtures show
`f0 : (e: E(Int)) -> Int`.

Changed goldens: none (the ilang-check goldens and the spec suite contain no
union that is exactly a declared alias's expansion).

## Timings

Release `i check --project <dir>`, same machine, load average 20–35 (other
lanes building). Before: the release binary of `redesign` (`5c186400` plus
documentation only).

| Members | Before | After |
|---|---|---|
| 6 | 0.22 s | 0.12 s |
| 7 | 1.03 s | 0.11 s |
| 8 | 7.4 s | 0.11 s |
| 9 | 78 s (evaluation) | 0.13 s |
| 10 | over 100 s (evaluation) | 0.14 s |
| 12 | – | 0.14 s |
| 16 | – | 0.21 s |
| 22 | – | 0.58 s |
| `i check --project std` | 16.4 s | 15.4 s |

Debug build (the test): 16 members about 7 s, so the test bound is 60 s in a
debug build and 5 s with optimizations; an exponential regression exceeds it
by orders of magnitude.

## Tests

`crates/ilang-check/tests/alias_unions.rs`:
`nominal_union_recursive_match_scales` (generated sources for 8, 12, 16 and 22
members; time bound and the shown type `(e: E(Int)) -> Int`) and
`union_is_displayed_by_its_declared_name`.
