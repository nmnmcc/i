# Numeric steps 2–5

No new language or library state surface.

N2 uses separate internal worker signatures, with f64 parameters and proved
f64 results. Source closures retain their original code references and capture
sets. Known-code calls pass the original closure and the current environment.
Callback variants key on code identity and representations, never capture values.
Tail-result constraints propagate backward to a fixed point; unknown tails and
suspension force Value results. Four workers per source binding and twice the
source statement count bound additional worker code, including inline growth.

Inlining binds each actual once, renames locals/labels/facts, and retains eager
logical enter/site/leave/tail actions. It excludes the frozen transitive
may-suspend set, recursive helpers and bodies over 32 statements, with twice the
caller size as the growth limit (three times for a caller that projects fields,
see the final pass below). The exact review fixture checks both its
eligibility dump and complete O0/O2/O3 resumed-stop output. Suspension is
recomputed afterward. Scalar captures box only in the capture handler; segments
and joins restore the original scalar representations from canonical values.
Their result and saved-frame protocols remain unchanged.

Prior art read: pinned Binaryen Heap2Local.cpp (escape/exclusive-use proofs),
Inlining.cpp (size budget and exception exclusions), OCaml asmcomp/cmmgen.ml
(strict unboxed joins), js_of_ocaml compiler/lib-wasm/gc_target.ml (Float
box/unbox), Flambda2 simplify/inlining/inlining_transforms.ml (bind arguments and
rename), simplify/unboxing/unboxers.ml (proof-bearing unboxing), GHC
GHC/Internal/Bignum/Integer.hs integerAdd (exact small-to-big promotion).

Interpretations: retain the N1 emitter's separate canonical-reference cache and
scalar locals instead of changing semantic LocalType. R7 still saves canonical
Floats/Ints and restores their selected scalar encodings, without changing the
continuation ABI. This follows numeric-lowering R2/R7's storage requirements.

Measurements use tests/perf/numeric and the existing numeric_measure example.
NUMERIC_BASELINE selects the previous stage's optimized Wasm, paired against the
current compiler, so all variants use the same Wasmtime build and timer. Raw
samples, five batch medians, optimized WAT and allocation instrumentation are in
target/numeric-step2/. Tables show raw allocation counts. The entry/root wrapper
has 11 nonnumeric allocations; input/result Float boxes remain in the allocation
gates after subtracting those 11. N1 identity has 12 raw allocations, while the
scalar-entry adapter currently gives N2–N5 identity 13.

N2 first paired measurement (five batch medians, ns; not a quiet timing claim):

| Kernel | N1 baseline | N2 | N2 allocations including wrapper |
|---|---|---|---:|
| fast | [4518, 5448, 4497, 4507, 4539] | [4286, 4207, 4186, 4176, 4297] | 52 |
| horner | [100, 41, 80, 100, 90] | [50, 40, 60, 60, 39] | 13 |
| dot | [6470, 5628, 4607, 4348, 4377] | [3275, 3256, 3226, 3184, 1814] | 13 |
| integer | [1041, 1101, 1072, 1180, 1101] | [1032, 1081, 1641, 1622, 1152] | 14 |
| tuples | [180, 170, 160, 181, 161] | [60, 60, 60, 51, 42] | 13 |
| closures | [4327, 4487, 5679, 4216, 4298] | [2954, 1603, 1593, 1603, 1603] | 16 |

N2 differential corpus, exact transitive-suspension acceptance and changed-crate
Clippy pass. Earlier measurement showed unreachable workers behind generic
recursive entries; reference adapters corrected that call-selection gap.

N3 replaces up to four tuple, nominal-record or structural-record fields,
including compatible joins. Actual constructor/type/label metadata governs
projection and test folding. Every observing use materializes original fields;
unused field arithmetic retains its checks. Dominated semantic list and record
reads are commoned across calls; branch joins discard uncertain provenance.
Environment, closure-patch, request, host and bigint scratch arrays are excluded.

N3 paired batch medians (ns), compared with N2:

| Kernel | N2 baseline | N3 | N3 allocations including wrapper |
|---|---|---|---:|
| fast | [4296, 4175, 4155, 4227, 4247] | [4315, 4316, 4076, 4156, 4266] | 51 |
| horner | [60, 72, 60, 60, 50] | [70, 71, 60, 50, 51] | 13 |
| dot | [2163, 2003, 2002, 2001, 2004] | [2112, 2063, 1993, 2031, 2084] | 13 |
| integer | [1072, 1131, 1150, 1112, 1072] | [1053, 1082, 1143, 1131, 1111] | 14 |
| tuples | [50, 40, 59, 60, 50] | [49, 49, 49, 40, 50] | 13 |
| closures | [1792, 1723, 1723, 1793, 1753] | [1733, 1743, 1823, 1812, 1744] | 16 |

The tuple allocation target is met: 13 raw allocations, two after subtracting
the nonnumeric wrapper, with no tuple/backing-array allocation. The sine survivor still
crosses larger record-returning helpers; out-of-line multivalue is deferred by R5.

N4 adds signed-i31 worker entry guards. Parameter aliases feed widened i64
arithmetic; only i31 values enter the scalar signature. A failed edge passes
current canonical operands directly to the ordinary entry, including exact
promotion results, without restarting any source computation. Arbitrary Int
never uses int_to_i64. Conservative integer intervals remove redundant small
operand/result tests, while numeric equality and exhaustive Bool branches use
machine conditions. The differential effect loop crosses the i31 boundary and
checks every step occurs once; scalar Int capture is tested across suspension.

N4 paired batch medians (ns), compared with N3:

| Kernel | N3 baseline | N4 | N4 allocations including wrapper |
|---|---|---|---:|
| fast | [4226, 4156, 4354, 4147, 4147] | [4286, 4557, 4477, 4537, 4186] | 50 |
| horner | [60, 51, 60, 60, 59] | [61, 60, 40, 51, 40] | 13 |
| dot | [2644, 2593, 2945, 1852, 1843] | [1823, 1783, 3445, 3455, 1792] | 13 |
| integer | [1154, 1112, 1121, 1242, 1182] | [1392, 1372, 1343, 1282, 1362] | 14 |
| tuples | [69, 49, 60, 59, 70] | [39, 51, 40, 40, 51] | 13 |
| closures | [1644, 1662, 1644, 2946, 2775] | [1604, 1653, 1653, 1684, 2675] | 16 |

N4 differential corpus and changed-crate Clippy pass. These batch spreads
require a quiet confirmation before a performance pass/fail claim.

N5 uses exact BigInt endpoints in units of 2^-1074. Add/subtract/multiply
and division bounds use exact integer/rational transfers, then round endpoints
outward to binary64. Signed-zero possibilities are retained separately. Every
removed check has an exact magnitude bound at most max_finite; zero division
still returns positive zero. Unknown/recursive inputs widen to all finite values.
Dominating comparisons refine branch successors, including failure blocks;
per-definition versions prevent stale comparison facts after a join or rewrite.
There are no range-specialized clones or caller-dependent callee summaries.
The instruction itself and eager source-site actions remain. Proof exhaustion
retains checks. CORE-MATH sin.c u128_tod and its explicit rounding-bound argument
were also read before implementing the exact dyadic enclosure.

N5 first paired batch medians (ns), compared with N4:

| Kernel | N4 baseline | N5 | N5 allocations including wrapper |
|---|---|---|---:|
| fast | [4707, 4597, 4537, 4777, 4446] | [4808, 4886, 4397, 4618, 4376] | 50 |
| horner | [40, 49, 40, 60, 40] | [30, 59, 41, 61, 42] | 13 |
| dot | [1853, 1842, 1842, 1882, 1913] | [1841, 1862, 1813, 1842, 1823] | 13 |
| integer | [1291, 1262, 1244, 1232, 1293] | [1292, 1261, 1231, 1242, 1291] | 14 |
| tuples | [52, 60, 50, 50, 60] | [40, 49, 30, 60, 31] | 13 |
| closures | [1702, 1683, 1682, 1743, 1752] | [1793, 1732, 1783, 1912, 1693] | 16 |

N5 differential corpus and the ilang-wasm tests pass, including the exact
subnormal/overflow enclosure tests. A final budget audit counts argument binds,
logical actions and aggregate expansion in actual extra worker statements,
and narrows expansion if it would exceed the bound.

Final corpus: 399 exported programs, with an assertion requiring at least 200
different source bodies, plus the exact standalone resumed-stop project. All
four reference/visible/scalar configurations agree, including complete stop
traces and final effects. The million-call unknown-tail relay remains bounded.
The final worker denominator includes only numeric-eligible original code;
callback roots are included, and every extra statement is counted after expansion.
Two callback code variants can each gain one scalar entry within the four-worker
per-binding limit. Additional keys use the ordinary entry.

The task's “at redesign” is interpreted as the starting revision: work stays on
the existing lane/jp branch, as the shared rule forbids edits on redesign.

## Final pass and targets

A second worker reviewed the four stages and the unfinished last change. Kept:
callers that project tuple/record fields may expand to three times their size
(still inside the global 2x worker-statement bound; §9.3 makes budgets
heuristics). Without it the final growth audit had narrowed the tuple kernel
back to 16 raw allocations; with it the tuple kernel is again 13. Copy
forwarding inside one inline expansion was tightened: the copy and its source
must each have at most one definition there, so a join can never change the
forwarded value. The measurement driver now records per-site allocation
counts (target/numeric-step2/*/allocation-sites.json), and
tests/perf/numeric/compare-js.mjs replays the exported corpus on Node.

The corpus gained 56 programs: inlined helpers whose multiplication stops (one
and three nested inlined frames), an inlined call that tail-calls a recursive
loop, mixed Int/Float workers that leave the i31 region in the middle of a loop
and stop later, Int loops that cross both i31 ends, closure factories passed to
a known higher-order helper, an unused record field whose arithmetic must still
stop, the binary64 overflow tie (2^970 + max_finite) under a dominating bound,
and bounded division by both zeros and by subnormal divisors. Total: 455
programs, at least 200 distinct bodies, values, final effects and complete stop
traces equal for reference O0, visible O2, scalar O2 and scalar O3. A separate
19-project probe (main programs run with `i run` at O0/O2/O3, comparing exit
status, stdout and complete stderr traces, including an Int worker and an
inlined stopping helper after a suspend/resume) also agreed at all levels.

Final paired measurement against the N5-stage Wasm
(NUMERIC_BASELINE=target/numeric-step2/n5/n1.wasm),
five batch medians (ns), one heavy slot, other workers' builds running (not a
quiet window):

| Kernel | N5 baseline | Final | Final allocations including wrapper |
|---|---|---|---:|
| fast | [4406, 4317, 4317, 4326, 6259] | [4477, 4295, 4237, 4266, 4245] | 50 |
| horner | [40, 41, 31, 40, 41] | [60, 62, 50, 60, 52] | 13 |
| dot | [1811, 1793, 1795, 2885, 1914] | [1814, 1844, 1853, 2484, 1934] | 13 |
| integer | [1301, 1280, 1290, 1254, 1304] | [1332, 1302, 1312, 1262, 1312] | 14 |
| tuples | [49, 50, 60, 50, 50] | [40, 49, 51, 50, 40] | 13 |
| closures | [1623, 1683, 1632, 1643, 1631] | [1612, 1623, 1624, 1682, 1653] | 16 |

Targets of numeric-lowering §7 (gate 4 and the combined table) against the
median of the five batch medians of each stage's own paired run. Allocation
counts subtract the 11 nonnumeric entry/root allocations; the input and result
Float boxes of the f64 ABI wrapper remain in them (identity is 2 on this scale).

| Kernel | Early-N2 gate | Combined target ns / allocs | N2 | N3 | N4 | N5 | Final | Met |
|---|---:|---|---:|---:|---:|---:|---:|---|
| fast sine | <=3,000 | <=500 / <=8 | 4207 / 41 | 4266 / 40 | 4477 / 39 | 4618 / 39 | 4266 / 39 | no |
| Horner | <=70 | <=40 / <=2 | 50 / 2 | 60 / 2 | 51 / 2 | 42 / 2 | 60 / 2 | early gate and allocations yes; 40 ns no in the final run (N5 run: 42) |
| dot | <=700 | <=300 / <=2 | 3226 / 2 | 2063 / 2 | 1823 / 2 | 1841 / 2 | 1853 / 2 | allocations yes; time no |
| integer | <=1.05x baseline until N4, then <=2,500 | <=500 / <=2, no Int objects | 1152 / 3 | 1111 / 3 | 1362 / 3 | 1261 / 3 | 1312 / 3 | 1.05x (N2 1.046x, N3 1.00x) and 2,500 yes; no Int objects yes; 500 ns and 2 allocations no |
| tuples | <=70 | <=40 / <=2, no tuple/array objects | 60 / 2 | 49 / 2 | 40 / 2 | 40 / 2 | 49 / 2 | early gate, allocations and no tuple objects yes; 40 ns met at N4/N5, not in the final run (49) |
| closures (fold) | <=800 | <=400 / <=4 | 1603 / 5 | 1744 / 5 | 1653 / 5 | 1783 / 5 | 1624 / 5 | no |

Reasons for the misses, from the optimized WAT and per-site counts:

- Every source call and every self tail iteration keeps the eager logical
  frame actions (`shadow_enter_call`, a site store and `shadow_leave_tail`),
  because cold stop-stack reconstruction is deferred (§9.5). In dot, fold and
  integer these are two calls and two bounds-checked array stores per element
  or step, plus the list pattern's tag test chain (`ref.test`/`ref.cast`
  twice per list). That, not boxing, is the remaining per-iteration cost:
  dot is about 57 ns per element with no Float box in the loop.
- fast sine keeps 39 allocations: the certified path crosses record- and
  pair-returning helpers above the 32-statement inline bound, and out-of-line
  multivalue returns are deferred by R5 (sites in allocation-sites.json).
- fold: the closure object, its capture array and the canonical box of the
  captured `x` are allocated per call (R7: stored closures keep ordinary
  captures), plus the wrapper boxes; 5 against <=4.
- integer: `core.to_float` is a runtime helper that returns a boxed Float, the
  third Float box. The N4 i64 worker is not faster than the N3 loop, which
  re-encoded i31 values in eqref parameters: about +15% in the N4 paired run
  (1154 -> 1362 ns), repeated in later runs (~1260-1310 against N3's
  ~1110-1150). Both loops are dominated by the same shadow-stack calls; the
  cost gate "no unexplained >5% regression" is therefore not met for this
  kernel, and the cause is not established.
- Horner and tuples are within one or two steps of the 10 ns timer
  resolution of the 40 ns target; the runs were not quiet, so these are not
  pass/fail claims (gate 4 requires another quiet paired comparison when the
  spread exceeds 20%, which it does for both).
