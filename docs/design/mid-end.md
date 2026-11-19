# Mid-end design: the next compiler project

Status: proposal. Pass 1 (type-directed equality) is merged. Pass 2 and pass 3
were probed; the results are in their sections. All numbers use the `docs/performance.md` protocol: one machine,
one run, five alternating rounds, median. Each run records `free -g` and
`/proc/loadavg`; local builds use `CARGO_BUILD_JOBS=2` and one process at a
time on a shared development machine, scaled to the host's limits; a round
with load above half the cores is discarded. Benchmarks here
ran on the 4-vCPU CI runners (load was 0-1 throughout).

## What the measurements say

| Benchmark | Paired time | Distribution |
|---|---:|---|
| `list_sort` | 1.23 s | ~65% `std` cell allocation (2 cells/element/pass, ~19 passes); comparator now a direct call; rest is list/pattern/shadow-stack dispatch |
| `equality` | 0.245 s | Int lists 39.5 ns/element; `(Int,String)` tuples 90 ns; the ~50 ns gap is per-element tag/cast/scan, no allocation left |
| `dict_insert` | 0.43 s | hash 4.6% (95 ns/hash); 200k lookups alone cost +0.17 s (850 ns each); 200k overwrites +0.40 s (2 µs each). The rest is per-level trie nodes, `Option`/result tuples and generic dispatch |
| `list_merge` | 0.53 s | inputs (two 1M maps) 0.29 s (55%); accumulation cons +0.03 s; final `reverse`+`append` +0.21 s (40%); the compare walk is not measurable |
| service env | K=4 → K=32 | +20-22% on a hot update loop (`$env_with` copies every slot) |

## Candidate passes, ranked by expected gain per unit of risk

### 1. Type-directed equality (recommended first)

- **What**: where the static type of `==` is a closed structural type (Int,
  Float, String, Bool, lists, tuples, records with those fields), emit a
  specialized structural walk instead of `$value_compare_i32`: direct cons and
  field access, scalar compares, no `value_tag`, no casts, no frames. Anything
  with functions, host refs, opaque or unknown types falls back to the generic
  function.
- **Basis**: equality is 0.245 s; per tuple element 90 ns against a 39.5 ns Int
  floor. The specialized walk should land near 50-60 ns.
- **Target**: `equality` ≤ 0.16 s (-35%) in a paired run; every other benchmark
  within +2%; corpus output identical; std build compile time +≤2%.
- **Cost/risk**: medium. Needs the operand's type id in the IR `Equal`
  condition and a small shape-driven WAT generator in emit (depth-capped).
  No new IR layer; O0 untouched; diagnostics/ABI/stop stack unchanged; no user
  code or suspension inside the walk.

### 2. Known-call inlining at the ANF level

- **What**: inline small program functions at O2/O3 before Binaryen, so calls
  through known closures and small helpers disappear and later passes see the
  bodies. Complements `ALWAYS_INLINE=40`, which is Wasm-level only.
- **Basis**: raising Binaryen's threshold 20→40 gave -18% on `list_sort` and
  -15% on `list_map`; HIR-level inlining can reach larger bodies and calls that
  Binaryen refuses, and removes the suspendable wrapper around known pure calls.
- **Target**: `list_sort` ≤ 1.10 s (-10%) from 1.23; `list_map` no regression;
  std build compile time +≤10%.
- **Cost/risk**: medium-high. Body cloning exists; inlining must not cross a
  real suspension point, must keep frames/sites, and needs a size budget.

**Probe (2026-11-20, no code).** A probe package calls `list.sort`,
`list.map`, the benchmark's `merge` and `dict.insert`; it was compiled with
`--opt 2` and the pre-Binaryen WAT was counted (call sites, callee body size in
instruction lines as a proxy for Binaryen expressions). Of 6331 direct call
sites, 4729 (74.7%) target bodies of at most 20 instruction lines and 942
(14.9%) 21-40 lines; 241 (3.8%) are 41-100 and 179 (2.8%) over 100. That is,
89.5% of sites are already within `ALWAYS_INLINE = 40`. The sort/merge chain
is typical: every callee there is 8-18 lines. After Binaryen, 3427 calls
remain and only 10.5% of them target a body of at most 40 lines; 89.5% target
larger bodies. Binaryen has therefore already inlined the small callees an ANF
inliner with a similar threshold would target, and the direct gain from pass 2
is small. It would matter only as an enabler (exposing known calls and types
before specialization) or with a threshold well above Binaryen's. Ranked below
pass 3 on current evidence.

### 3. Aggregate returns / escape (trie results and `Option`)

- **What**: return result tuples and `Option` payloads as multiple Wasm values
  instead of allocating a value per call; start with functions whose result is
  immediately destructured and whose tuple does not escape. General closure
  escape analysis is a later, separate step.
- **Basis**: dict lookups are 850 ns and inserts 2.15 µs; each trie level
  returns an aggregate (`(Node, Bool)`, `Option`). Removing those allocations
  estimates at 20-30% of `dict_insert` (0.43 → 0.30-0.34).
- **Target**: `dict_insert` ≤ 0.34 s (-20%); no other regression; corpus
  unchanged.
- **Cost/risk**: high. New IR value/statement form and emit support; a result
  tuple can cross a suspension, so those cases must stay allocated; workers
  must see the same representation.


**Pass 3 probe (2026-11-20, no code).** A copy of `std/dict`'s insert path
with the per-level result tuples removed (`put` returns only the new node and
`put_entry` only the new list) was compared with `std/dict` on 200k random
keys in one CI job, five alternating rounds (Xeon 6975P-C, load 0.5):
`dict.insert` cost 0.385 s with the tuples and 0.328 s without, a **14.8%**
difference (1.9 µs to 1.65 µs per insert, about 250 ns each). The trie shape
and node allocations are identical; the difference is the removable result
tuples. That is the upper bound for pass 3 on this workload, and it is below
the 20% target set for the highest-cost pass. **Pass 3 is therefore not
proposed now**; the shape is recorded below in case a later change needs it
as an enabler.

**Shape of the change (if revisited).**

- IR: a multi-result call form used only when the immediately following
  statement destructures the result of a direct/known call, e.g.
  `Stmt::Let2 { dst_a, dst_b, value: Value::Call, .. }` (generalized to at
  most four results). Calls through a function value and calls whose result
  is stored, returned or held across a suspension keep the tuple.
- Emit: clone the callee as a multi-value function returning
  `(result eqref eqref ...)`; the call site stores the results directly into
  locals. Other callers keep the tuple-returning function; a function used as
  a value keeps its `$fn_n` single-result form.
- Must stay allocated: a call that may suspend (the split `Suspendable`
  points), a tuple that escapes its destructuring or crosses a suspension,
  exports and worker-visible functions, and any result consumed as a value.
- Workers: the multi-result clone is internal, never a constant closure or a
  worker entry, so the codec never encodes a multi-value result.
- `--opt 0`: this pass, like the earlier ones, runs only at O2/O3. O0 keeps
  the tuple; the O0/O2 differential corpus stays independent. No ABI, export
  or metadata change; source sites and stop traces stay at the same
  positions.
- Tests: immediate destructuring removes the tuple from the generated code;
  escaping, suspending, worker-visible and value-position cases keep it;
  results stay identical.

### 4. Monomorphization of generic library calls

- **What**: instantiate `dict`, `fold`, `map` bodies for concrete type
  arguments so `eqref` boxing leaves the hot path.
- **Basis**: remaining generic dispatch in dict/equality; exact share needs a
  dedicated probe.
- **Cost/risk**: very high (type instantiation across modules, compile-cache
  keys, ABI metadata, test explosion). Not proposed now.

## Not doing now

- **Wasm stack switching / suspend-resume runtime**: resume cost is flat in
  depth after lazy capture and is not a benchmark bottleneck; it touches the
  runtime ABI and engine configuration.
- **GC/collector tuning**: copying with 256 MiB is within ~10% of the
  no-collector run on sort; DRC is the only slow collector and not the default.
- **Worker/cross-worker representation**: highest risk of a race, and no
  benchmark is dominated by it.
- **Reviving `$plain`**: its +8% equality regression is explained (one guarded
  type test per element), and pass 1 makes the guard unnecessary; do not add
  another representation layer before it.
- **`std` algorithm work** (sort/merge cells, dict path): owned by the
  standard-library line. Re-measure `list_sort`/`list_merge` after its cell
  reduction lands before choosing between passes 1 and 2.

## First pass and acceptance

Type-directed equality. It has the clearest headroom (50 ns per tuple element
over a 39.5 ns floor), a bounded surface, and no suspension/ABI interaction.

Acceptance in one paired run (same machine, five alternating rounds):
- `equality` ≤ 0.16 s from 0.245 s, at least -35%;
- `list_sort`, `list_map`, `dict_insert`, `list_merge` within +2%;
- corpus/spec/std green; O0 output equals O2 for the corpus;
- std test-build compile time within +2%.

If the first measurement is under -15%, stop and revisit; do not grow the pass
into general monomorphization.

## Verification before merge

The merge criterion is one: the head commit of the pull request has a complete
successful CI run. A documentation commit pushed on top of a verified code
commit does not extend that verification. In particular, `[skip ci]` skips the
workflow for that commit but the commit still joins the workflow's concurrency
group and cancels the run of the code commit still in flight; the head then has
no check at all. Either rebase or amend documentation into the code commit
before pushing, or let the documentation commit trigger its own full run and
wait for it to finish.
