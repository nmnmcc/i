# Numeric lowering without temporary boxes

Status: compiler design with coordinator decisions recorded in §9, not a
language extension. Independent review permits N0 and conservative N1 after
checker coordination; the transitive inlining gate below is required before N2.
Only this note is committed. Measurements and probes are under this lane's ignored
`target/numeric/`. Baseline compiler:
`94789e6141d9d2213d3b909b07bcbac9bbcdb02d`. All repository code locations below
refer to that revision. Binaryen locations refer to submodule
`fba5005a132d85d578be5bd010e0487ed8e2ce3e`.

## 1. Problem and constraints

Keep Float arithmetic in f64 until a reference is needed. Keep small integer
arithmetic in machine locals only while its exact value is represented. Give
known calls scalar parameters and results; retain ordinary reference entries
for unknown calls and stored functions. No rule recognizes std/float, a
package name, or a coefficient table.

The controlling spec sections are §4.7–4.8 (principal inference), §6.2
(evaluation order), §9.4–9.5 (control and finals), and §11.1–11.8. In particular:

- Int is unbounded; division truncates toward zero, remainder has the dividend's
  sign, and division/remainder by zero give zero (§11.2).
- Each Float operation rounds to binary64, nearest with ties to even; a
  non-finite result stops immediately; division by either zero gives positive
  zero (§11.3; `runtime/core.rs:19–22`). Here `runtime/` and `emit/` abbreviate
  directories under `crates/ilang-wasm/src/`.
- Numeric and aggregate equality is by value, but functions, boundaries,
  continuations and host references have identity (§11.4). Float equality
  equates the zeros; inspection and bit conversion distinguish them
  (§11.8, §13). Preserve bits, not just equality.
- Tail calls use bounded stack (§11.6). Stops retain exact logical source call
  positions, including tail-site rules (§11.7), and do not run finals (§9.5).
  `? Never ! Never` does not prove a function cannot stop (§11.7).

Examples use accepted `syntax.md` syntax. The plan's “已定的设计” requires ordinary
libraries and general features. The ledger's Std-3 profile ruling keeps correct
rounding and withholds the float library: this work does not reopen a weaker
rounding contract. `lazy-reentry.md` §5.1, D9 and S0 govern saved frames;
`runtime.md` D4, D6–D8 and §7 govern task capture and erasure.

A type remains a set of values. These changes select encodings for subsets
already proved by the checker or a control-flow test. They introduce no source
type, annotation, overload, trait, effect or inference constraint. Float workers
serve geometry, numerical integration and audio; small Int regions serve
parsers, collection indexes and task counters; specialization serves codecs,
folds and numerical helpers; aggregate elimination serves coordinates, parser
results and state machines; range proofs serve image channels, bounded physical
quantities and polynomial evaluation; immutable commoning serves configuration,
syntax trees and numeric records.

## 2. Measurements before the choice

### 2.1 Method, artifacts and limits

Built this lane's release compiler through `cache/heavy`, after initializing
its Binaryen submodule using the plan's command-local URL override. Cargo
metadata reports this lane as workspace root. No compiler source was patched.
The six kernels compiled and ran at O0 and O2. The float source is from
`de4d58d04c39539cdcb20ad68e7d93e5f5453795:std/src/float.i`; the isolated copy
exports one additional wrapper around its existing `fast_sincos`.

Artifacts under `target/numeric/`:

- `project/src/kernels.i`, `project/src/math.i`, `project/i.toml`;
- `o0.wasm`, `o2.wasm`, and WAT printed from those actual binaries;
- `count.py`, `counts0.json`, `counts2.json`, `*-o[02]-hot.wat` (the twelve
  most executed functions for each kernel, with execution counts);
- `measure.py`, `float_profile_native.c`, `time.sh`, five
  `timing-{o0,o2,scalar}-N.json` files containing every paired sample;
- `scalar.wat`/`.wasm`, hand-written controls; `attacks/`, executable probes;
- `alternatives.md`, written after measurement and before this design choice.

Build with `target/release/i build --project target/numeric/project --opt 0
--out target/numeric/o0.wasm`, then the same with `2`; print binaries with
`wasm-tools print`. The CLI's `--wat` is pre-Binaryen, so it cannot stand in for
optimized WAT (`crates/ilang-cli/src/main.rs:214–228`,
`crates/ilang-cli/src/lib.rs:146–162,190–198`). Run `count.py 0` and `count.py 2`.
Run `cache/quiet direnv exec /home/agent/Programming/i bash
target/numeric/time.sh` from this lane. The wrapper reserves all three heavy
slots. Future implementation moves the kernels to bench/, not this commit.

The timer reuses the Std-3 Wasmtime 50.0.1 C API with copying GC, read from
`.rd/note-std3/target/std3-prof-env`; Python bytecode writes are disabled. That
upstream build selects Copying for Auto when gc-copying is compiled in
(`target/std3-c-api-tool/wasmtime-50.0.1/crates/wasmtime/src/config.rs:3708–3711`,
in the evidence lane). Its default initial heap differs from i's 256 MiB,
and its patch version differs from i's pinned 50.0.0. i selects collector/heap
at `crates/ilang-host/src/lib.rs:791–793,1534–1547`. These are embedded execution
measurements, not CLI startup timings.

Machine: Linux 6.18.56, x86-64, AMD EPYC-Genoa virtual CPU, 16 exposed CPUs.
Each kernel takes one host Float, here 0.5, and returns one host Float. After
initialization and 200 warmups, C times 1,001 calls individually, pairs each
with an identity call, and alternates order. Report the median of five run
medians of signed target-minus-identity nanoseconds. Compilation and Python
are outside timed intervals. The identity input box is included in allocation
counts but its time is subtracted. Counts reset after initialization; list and
top-level constant construction is excluded. Count executed struct.new*/
array.new*, not bytes, live objects or GC cycles. Instrumented modules are
never timed; their answers were checked against uninstrumented modules.

The timer resolves about 10 ns. O2 run medians vary: Horner 120–220 ns,
integer 4,367–8,272 ns. The quiet lock cannot control VM scheduling/frequency.
These are not worst-case promises: fast-sine O2's largest sampled difference
was 95,444 ns, and O0's was 2,808,852 ns.

| Kernel, x = 0.5 | Expected value | O0 ns/call | O2 ns/call | O0 allocations | O2 allocations | O0 / O2 function entries |
|---|---:|---:|---:|---:|---:|---:|
| certified Float fast sine | 0.479425538604203 | 25,228 | 16,896 | 317 | 311 | 1,700 / 356 |
| degree-7 Horner | 1.7509765625 | 651 | 129 | 23 | 23 | 79 / 17 |
| dot product, 32 list elements | 13.25 | 4,187 | 1,754 | 65 | 65 | 847 / 166 |
| integer sum, 128 iterations | 8256.0 | 5,839 | 4,488 | 2 | 1 | 2,077 / 1,159 |
| three tuple-returning calls | 2.875 | 591 | 121 | 22 | 22 | 78 / 18 |
| closure fold, 32 list elements | 9.0 | 4,557 | 1,803 | 68 | 68 | 847 / 230 |

Identity has one allocation at both levels and 9 / 3 function entries. The
integer kernel executes no allocating Int operation here; its O2 allocation
is a Float. This is a tagging/call/control-flow opportunity, not Int boxing.
Sine counts differ from Std-3's 313/360 because the export/call graph and probe
wrapper differ. Neither measurement replaces the other.

SHA-256 of O0: `c945519ed78d7af6161c4530964b6992fbf3e1797e038181d21ccbe4418e7f17`.
O2: `d15ddfff698e5d2a1ab66e82db89a56153814cb4e033d55d90362e07b4053c25`.
Kernel source: `f42997dda6752a74d28d5fe01d7c000c547b1915b0cbc9cd2ae5ea3cc6e41193`.
Diagnostic math source:
`d05069cf293e24f976785b515ec16deb7c1d835ddcc33e095f9213214d28709c`.

### 2.2 Exact kernels

The project is num, depends on this lane's std, and exports all seven names
below through wasm.exports. num/math is the float copy above, with fast_sin
added to its exports and this wrapper appended:

```i
fast_sin = (x: Float) -> Float {
  fast_sincos(x, False) |> { Some(v) -> v None -> x }
}
```

The fallback deliberately returns x: the measured 0.5 returns sine, confirming
the certified branch. Production sin keeps its interval fallback.

```i
(i/core, math = num/math)
(identity, fast, horner, dot, integer, tuples, closures)
identity = (x: Float) -> Float { x }
fast = (x: Float) -> Float { math.fast_sin(x) }
horner = (x: Float) -> Float {
  a = 0.125 *. x +. 0.25
  b = a *. x +. 0.375
  c = b *. x +. 0.5
  d = c *. x +. 0.625
  e = d *. x +. 0.75
  f = e *. x +. 0.875
  f *. x +. 1.0
}
xs = [0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875, 1.0,
  0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875, 1.0,
  0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875, 1.0,
  0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875, 1.0]
dot_go = (as: List(Float), bs: List(Float), sum: Float) -> Float {
  as |> {
    [] -> sum
    [a, ..at] -> bs |> {
      [] -> sum
      [b, ..bt] -> dot_go(at, bt, sum +. a *. b)
    }
  }
}
dot = (x: Float) -> Float { dot_go(xs, xs, x) }
int_go = (n: Int, acc: Int) -> Int {
  { n == 0 } |> { True -> acc False -> int_go(n - 1, acc + n) }
}
integer = (x: Float) -> Float {
  core.to_float(int_go(128, core.truncate(x)))
}
pair = (x: Float) -> (Float, Float) { (x +. 0.25, x *. 0.5) }
tuples = (x: Float) -> Float {
  (a, b) = pair(x)
  (c, d) = pair(a +. b)
  (e, f) = pair(c +. d)
  e +. f
}
fold(A, B) = (items: List(A), acc: B, f: (B, A) -> B) -> B {
  items |> { [] -> acc [a, ..rest] -> fold(rest, f(acc, a), f) }
}
closures = (x: Float) -> Float {
  fold(xs, 0.0, (acc: Float, v: Float) -> Float { acc +. v *. x })
}
```

Expected values follow source-ordered operations, list length and integer sum
1 + ... + 128. R1–R8 must preserve them. Scalar controls also agreed bit-for-bit
with production on ten inputs per non-sine kernel: -2, -1, -0.5, -0, 0, 0.125,
0.5, 1, 2, 100 (50 comparisons).

### 2.3 Hot WAT and Binaryen

| Kernel | O0 hot path | O2 survivor and obstruction |
|---|---|---|
| sine | float helpers unbox twice, check and box; fields use field helper | 356 entries, 1,394 cast/load/non-null operations; arithmetic results and records cross calls |
| Horner | 14 boxed arithmetic results, eight literal boxes, input | seven calls each to functions 22 (multiply) and 20 (add), each returns struct.new 2; all 23 allocations survive |
| dot | two list destructures, multiply, add, recursion | 64 Float results cross helper or recursive calls; input lists really escape |
| integer | add/sub, int_from_i64, equality, clause tests | function 6 tests both i31 bounds and returns ref.i31 256 times; function 27 tests Bool tags 257 times |
| tuples | pair builds two Floats, values array, object | three calls to function 30 still return a tuple across a call boundary |
| closure fold | closure/capture array, indirect callback, capture handler | function 28 keeps a try_table around callback; numeric values cross the fold/callback boundary even where Binaryen resolves a call |

For example, measured O2 Horner multiplication performs this sequence, followed
by its finite-failure branch and a fresh object containing the result:

```wat
local.get 0
ref.cast (ref 2)
struct.get 2 3
local.get 1
ref.cast (ref 2)
struct.get 2 3
f64.mul
local.tee 2
f64.abs
f64.const 0x1.fffffffffffffp+1023
f64.le
```

Full evidence: `target/numeric/horner-o2-hot.wat:2–28`;
`integer-o2-hot.wat:31–80`; tuple helper `tuples-o2-hot.wat:71`;
fold handler `closures-o2-hot.wat:132`. O0 executes runtime paths at
`runtime/core.rs:11–24`, `runtime/values.rs:52–73`,
`runtime/bigint.rs:220–228,260–268,304–311`. Each kernel has its own retained
O0/O2 hot WAT file, including sine and dot.

Binaryen already uses closed-world speed level 2, shrink level 0, observable
traps and an always-inline limit of 20 expressions
(`crates/ilang-binaryen/src/lib.rs:40–47,106–112`). Its pinned
`src/passes/pass.cpp` includes at O2:

- GC type/signature refinement, global type optimization and constant-field
  propagation, lines 776–803;
- precompute, heap2local, cast optimization and local subtyping, lines 682–723;
- inlining-optimizing and global cleanup, lines 821–843.

local-cse requires O3 or shrink level 1 (lines 725–728). GUFA is registered
(lines 205–227) but is not in the default O2 pipeline. Wasm multivalue
“tuple-optimization” (lines 698–702) does not invent multivalue signatures for
i's heap tuples.

`Heap2Local.cpp:99–110,317–332` requires nonescaping allocations and exclusive
uses; a return or unrecognized use such as a call escapes. Array accesses need
constant indexes (`:447–460`). It can remove a box when construction and all
reads become visible inside one function. It cannot generally unbox surviving
reference parameters/results. Signature refinement narrows reference types,
not eqref to f64 (`SignatureRefining.cpp:18–24,205–260`). O2 inlining stops at the 20-expression always-inline limit
for multiply-used, nontrivial helpers (`Inlining.cpp:148–195`); checking,
allocation and shadow operations increase their size. Closure calls can be
split for capture before Binaryen runs (`control.rs:273–304`); later resolving
a call does not rerun i's control analysis. These measured survivors motivate
earlier lowering; Binaryen does have useful GC optimizations.

The independent review linked a scratch driver against the pinned Binaryen
with the compiler's O2 settings (`/tmp/review-num/opt.cpp:10–16`). Its local
visibility probe constructs a Float box, reads it for multiplication, checks
finiteness, then constructs and reads a second box
(`/tmp/review-num/box-visible.wat:3–9`). Default optimization removes both boxes
and retains the multiplication and finite check
(`/tmp/review-num/box-visible-o2.wat:4–17`). This is the review's observation,
confirmed by reading those inputs/outputs; it does not prove a cross-function
representation rewrite. N0 must first try emission that makes construction and
consumption visible together, then measure on the real kernels before adding
custom optimization passes (§7).

### 2.4 Controls and gain estimates

Hand-written scalar.wat keeps each Float operation and a finite check, uses
f64 locals, and allocates nothing per call. Horner and tuples expand in source
order. Dot/fold traverse a prebuilt GC list with boxed Float heads, but use a
simpler known shape, one traversal for identical input lists, and no source
shadow bookkeeping. Fold specializes the captured x. The Int control guards
abs(x) <= 100000, truncates to i64 and sums 128 times with a bounded accumulator;
it is not a general implementation of truncate or Int.

| Checked control | ns/call, five-run median | O2 baseline |
|---|---:|---:|
| Horner | about 10, timer floor | 129 |
| dot | 60 | 1,754 |
| bounded Int | 60 | 4,488 |
| tuple calls expanded | about 10, timer floor | 121 |
| specialized closure fold | 60 | 1,803 |

These are performance controls, not compiler implementations or full proofs.
The supplied Std-3 controls, separately measured and **not rerun here**, are
131 ns scalar sine, 250 ns with finite checks after emitted arithmetic, and
130 ns native C for the same algorithm; native libm is about 10 ns. Source:
`de4d58d:docs/design/std-3.md:1318–1525` and `.rd/cache/gpt/report-std3b.md`.
They combine inlining, scalar replacement and LLVM cleanup: 15,212 / 250 =
61-fold opportunity and 250 / 131 = 1.91-fold check-removal opportunity are not
independent speedups to multiply or promise.

| Mechanism | Basis and expected gain | Main risk |
|---|---|---|
| Float locals/known calls | Horner 23 objects to <=2 wrapper objects, dot 65 to <=2; first combined latency gate 2–5x, supported by controls | wrong representation at erased edge |
| small Int regions | no Int allocation today; remove 256 packing paths and repeated tests; first target 1.5x | overflow, replaying effects on fallback |
| callback specialization/helper inlining | reduce 230 fold entries and capture scaffolding; first target 2x | identity, environment, traces, code growth |
| scalar aggregates | tuple kernel loses six aggregate/array objects plus boxed fields; target another 20% after Float workers | escape or shape mismatch |
| finite proof | supplied 250-to-131 ns control, up to about 1.9x there; none without bounds | ignoring rounding in a real-number bound |
| loads/constants | eight Horner literal boxes; Std-3 source caching/restructuring reduced 178 array reads to 33; require fewer executed reads, no separate timing multiplier | signed zero or mutable runtime storage |

## 3. Alternatives and choice

Criteria in order: exact values/stops, bounded tail stack and capture; unchanged
inference and ordinary libraries; feasibility in current code; gain across all
six kernels; bounded compile time/code size; review cost. The alternatives
record was written after measurements and before this choice.

| Design | Benefit | Why not the whole answer |
|---|---|---|
| A. Binaryen only: larger inlining, GUFA/CSE, repeat passes | least i-specific machinery, existing GC cleanup | cannot reliably change reference conventions or recover source bounds; global budget affects all code |
| B. Replace all values with tagged word/NaN boxing | compact uniform convention | unbounded Int and moving GC still need tags/roots; changes every heap visitor and control path |
| C. Whole-program monomorphization and typed heaps | greatest theoretical gain, including numeric lists | recursive shapes/generic instances can grow without bound; changes portability/heap visitors |
| D. Scalar workers inside existing reference world | local proof, bounded variants, unchanged escapes | keeps boxes for real heap data and unknown calls |

Choose D. Keep A's final cleanup, B's existing i31 fallback, C's specialized
workers and aggregate shape facts with strict budgets. Defer typed numeric
lists, unboxed closure environments and new continuation layouts. No change
to the language's value sets is needed.

## 4. Numbered rules

### R1. A scalar representation is a proof about a value, never a new type

After checking, attach conservative representation facts to numeric operands,
parameters, results and field reads: Value, FiniteF64, or an exact integer
interval. Unknown, mixed, recursive or failed analysis gives Value. A union of
Float with a non-Float is not FiniteF64; a successful type-test branch may refine
it. Bottom/unreachable does not license a cast in reachable code.

Keep solver terms or stable post-solve facts, not printed type strings. Today
Typed has exported scheme trees, field access and type-test maps, but no full
expression-type map; local solver data is optional editor output
(`crates/ilang-check/src/types/mod.rs:148–210`, `types/infer.rs:194–227`). Add a
small code-generation fact table computed from final solved types and recorded
expression/parameter handles. Coordinate this export with T1 and T2, separately
from lowering work: freeze facts only after their completed inference lifecycle,
including completion, fixation, generalization and deferred checks. Do not
snapshot an intermediate solver state or freeze types earlier to help lowering
(`types-joint.md` §2.3 and §3). It must not force generalization, instantiate a
binding again, add a bound, or change displayed types. Validate facts against
the solver before discarding it. Operators can supply result facts directly;
a consumer's desired type alone does not prove every incoming value has that
representation. This is explicit checker-output work, not a field already
available in lower.rs.

Add explicit conversions and a result representation to the IR; distinguish
F64, source SmallInt, and internal I64/I32. Existing LocalType lacks F64 and
Func lacks a result-type field (`ir.rs:113–120,497–508`). Verify every operand,
call and branch assignment. Locals can have multiple definitions in this
structured IR: use per-definition versions for analysis and convert on incoming
edges; do not assume local numbers are SSA. T1 literal Int sets can give bounds,
but are not necessary for this work.

### R2. A Float operation produces a checked f64 and boxes only at an escape

Emit ordinary f64.add/sub/mul/div/neg, preserving each operation and its order.
For add/sub/mul, compute into a raw temporary, then test
abs(result) <= max_finite; only the checked successor gets FiniteF64. For divide,
evaluate both operands first, branch on divisor equal to either zero, give
positive zero there, otherwise divide and check. Negation, floor and ceiling of
finite inputs are finite. Sqrt of a negative number still stops (spec §13; runtime/core.rs:42–44);
conversions from arbitrary Int and bit patterns retain their existing checks.
Never enable fast-math, reassociation, FMA contraction, reciprocal multiplication
or flushing subnormals.

Separate BoxFiniteFloat from CheckFinite. Boxing allocates the ordinary Float
object without a second check; only a proof-bearing value can reach it.
Unboxing a typed Float needs no finite check. Host input/raw bit conversion
remain separate checks (R8). A raw possibly non-finite f64 cannot reach a call,
field, merge, return or saved frame. An unused arithmetic result still checks
at its original position.

Escape means an ordinary heap field, list head, closure capture, service value,
global requiring Value, erased argument/result, or runtime operation without a
scalar form. Box once per dominating value where useful; local consumers stay
scalar. Preserve all bits when boxing. Numeric object identity is not observable
(§11.4). Generic helpers remain for untransformed code.

Reason/current seams: parameters are all Value (`lower.rs:1352–1405`); emitted
functions have eqref parameters/results (`emit/mod.rs:284–295`); literals
allocate (`emit/mod.rs:490–495`); arithmetic helpers combine checking and boxing
(`runtime/core.rs:11–24`, `runtime/values.rs:52–59`). Split these decisions in
i's lowering rather than asking later passes to recover them.

### R3. Known calls use bounded scalar workers; function values keep their identity

A worker key is original code identity plus argument/result representations,
and optionally known callback code identity. Specialize representations, not
arbitrary source types, values, Float bits or unbounded intervals. Keep the
ordinary reference entry for every function used as a value. Its wrapper
unboxes, calls the worker and boxes results as needed. This wrapper is not a
new language function value: closures retain their original generic entry.
Workers have separate internal Wasm function types. An f64 signature is not a
subtype of the eqref convention and must never be installed in the generic
closure's function-reference field or reached by casting that reference.

Known Float functions can call workers with f64 parameters/results. A known
closure passes the **original closure object** and current environment to its
selected code. The current Direct path passes null, so merely changing the
function id is wrong (`ir.rs:249–255`, `emit/mod.rs:443–457`). Add explicit
known-code-plus-closure calls. A generic fold may have a Float-accumulator,
known-callback worker while still receiving the callback's actual captures.
Different capture values do not create variants. Unknown functions always use
the reference convention; a function type alone does not prove code identity.

Initial budgets: four workers per source binding; extra workers collectively
contain at most 2x the original eligible IR statements. Traverse in stable
order. Reuse keys through recursive groups; never specialize recursively on
growing tuple/type shapes. Budget exhaustion chooses the generic version,
without rejecting code or changing inferred types. Internal dumps report keys,
counts and fallback reasons.

**Tail edges constrain result representations.** Tail calls remain Wasm tail
calls, with no boxing/unboxing afterward. Propagate result-convention constraints
backward over tail edges to a fixed point. Unknown tail callees anchor Value;
affected caller workers must return Value too. A reference adapter may wrap a
scalar-result worker only if that worker cannot tail back into a reference-result
path. Otherwise choose a Value-result worker and tail-call it. Converting
arguments before a tail call is safe; waiting to convert its result can create
an unbounded chain of adapter frames. An SCC-only check is insufficient: an
unknown tail edge constrains its noncyclic predecessors as well.

Functions that may suspend keep Value results initially. Reentry/trampoline
signatures remain unchanged (R7); scalar locals and arguments are independent
of this conservative result rule.

Inline known nonrecursive small helpers in numeric-eligible regions, initially
at most 32 source IR statements and 2x per-caller growth. Before any inlining,
compute the **transitive may-suspend set** to a fixed point over the candidate
call graph, including workers. A function in that set is never inlined. Known
calls propagate suspension, including tail calls; unknown calls and suspending
operations are conservative seeds, as in `control.rs:245–304`. A body-local
search for suspend/catch/ensure/unknown calls is not sufficient: a helper can
call another known function that suspends. Keep the direct catch/ensure/unknown
call exclusions too. An inline expansion must not bypass the pre-inlining
exclusion by reconsidering a previously excluded function later in the pass.

Inline before capture splitting and keep the may-suspend recomputation after
inlining too. Copy operation origins and logical call-site chains. Preserve
argument evaluation once, in written order, before the body
(`lower.rs:1933–1959`), failing patterns, function identity and environments.
This applies equally to user helpers. Leave Binaryen's global inline limit at
20 rather than forcing large runtime functions inline.

The current capture path saves one shadow slot per physical frame and restores
that one slot (`emit/control.rs:625–646,686–719,834–888`). Eager logical site
writes and post-inlining liveness cannot preserve additional logical frames
inside a captured inline expansion. A design for captured logical frame chains
is deferred. The three-function resumed-stop acceptance case in §8 checks this
restriction at O0, O2 and O3.

### R4. An Int is machine-local only while exact; leaving the region uses ordinary Int

The canonical small set is [-2^30, 2^30-1]. It already uses i31 references
without allocation; larger numbers have a sign and u32 limbs
(`runtime/bigint.rs:1–5,57–108`). Two-small add/sub/mul widen to i64 and return
through int_from_i64 (`:220–228,260–268,304–311`). The costs are tests,
casts/extracts, calls and two result-bound tests, not allocation.

First remove redundant tests when both operands are proved small. Work in i64:
sum, difference, product, negation and quotient of signed i31 values fit. A
small result remains scalar; otherwise use the existing exact packer and a
Value region. Division/remainder by zero returns zero. Negating -2^30 or
dividing it by -1 gives +2^30 and must not truncate through ref.i31.

Prove loop invariants where possible. Otherwise guard a small region's entry
and result bounds. Failure passes the **current values** to the ordinary
continuation/loop without restarting or repeating prior calls. Use a sufficiently
wide temporary for the current operation and pack that result; do not replay
already executed work. A full i64 region is allowed only with proof that every
intermediate fits, not merely source Int. General i64 multiplication and MIN/-1
division require proof or fallback *before* a wrapping/trapping instruction.
Do not use int_to_i64 as a general unboxer: it traps outside range
(`runtime/bigint.rs:110–132`).

Mixed small/big joins use Value; a later dominating small test can reopen a
region. A source SmallInt saved/captured is an ordinary i31 Int, unlike the
internal bit-array-width I64 box. The measured integer kernel's truncate(x)
is unbounded for arbitrary Float x. Obtaining the scalar control's behavior
needs a guard plus a bound for all 128 additions; x = 0.5 is not a proof.

When a numeric comparison is consumed only by an exhaustive True/False branch,
emit a machine i32 condition directly. Use i64 comparisons only with the exact
integer representation proof, and f64 comparisons only for two Float operands;
Float equality still equates signed zeros. If Bool escapes, convert 0/1 to the
existing False/True values. A generic equality or mixed union retains ordinary
equality. This removes tag tests around proved Bool results without extending
the source type system or treating every eqref as a machine condition.

Full-range fixed-point trig integers need separate limb-arithmetic measurements;
the generic path keeps arbitrary precision and has no new size limit.

### R5. A short-lived aggregate is its fields until an observer needs the aggregate

Before representation selection, replace an eligible tuple or known-constructor
record and constant projections by ordered scalar fields. Start with nonrecursive
shapes of at most four fields and no escape. All field expressions still run
left to right, including checks of unused fields. Keep constructor/type identity
and tuple arity in shape metadata. Fold tests only for a known actual constructor;
a label can denote different indexes (`lower.rs:1763–1798`).

Expose cross-call pairs first by R3 inlining. Defer out-of-line multivalue
aggregate workers and typed heap layouts, avoiding a second continuation result
protocol. Conditional aggregates with one known shape may join field by field;
different shapes/uncertain widths use Value. Materialize before unknown calls,
escaping returns, equality/inspect/hash not explicitly lowered for that shape,
closure captures, generic containers or service storage. Preserve the actual
identity-bearing field values; never recreate a function, boundary, continuation
or host reference when materializing.

Scalar fields can survive suspension through R7 if no aggregate reference
escapes. Otherwise materialize before escape. Nominal records are today's
client; T3 structural records require checked shape/field mappings before this
pass supports them. Do not infer T3 layout from today's nominal layout.
Current aggregates are a values array plus object (`emit/mod.rs:586–595`,
`runtime/values.rs:64–73`); this pass removes both before calls obscure them.
Binaryen heap2local handles remaining allocations where its local proof succeeds.

### R6. Remove a finite check only with a conservative proof for that exact operation

Run separate abstract interpretation after inlining/scalar replacement. Float
facts are intervals of **rounded binary64 values**, with signed-zero information.
Unknown inputs cover the whole finite set. Bounds come from constants, verified
branch comparisons and proved transfers, not names or benchmark inputs. Join
by hull; widen unproved loops/recursion to unknown. Unknown means retain checks.

Bound exact dyadic results using integer/rational arithmetic, then enclose their
correctly rounded binary64 results outward. For example, an exact magnitude
bound <= max_finite suffices for add/sub/mul. Rounded host arithmetic is not an
outward-bound algorithm. This sufficient rule is conservative near overflow.
Nonzero division requires a strictly positive lower bound on divisor magnitude;
split its zero branch, which returns +0.0. Handle positive/negative subnormals
and rounding into either zero. Negation/floor/ceiling of finite inputs cannot
overflow; sqrt's nonnegative-input proof is separate.

A branch fact is valid only where its test dominates. A prior successful check
proves its own result finite, not the next arithmetic result. A callee summary
needs explicit proved preconditions; no reuse of one caller's interval for
others. Initially prefer inlining to range-specific clones: R3 keys do not
contain arbitrary ranges. Proof size/time exhaustion leaves checks in place
and never rejects a source program.

Keep the original rounding instruction even if its check disappears. No
algebraic identity may change signed zero, intermediate stops or compensated
arithmetic. Run the existing independent correct-rounding corpus after every
numeric stage.

### R7. Every live scalar is saved on capture; stored closures have ordinary captures

Rewrite representations/calls **before** control::split. Today split follows
run immediately (`lower.rs:499–504`) and computes liveness, segments and
environment chains (`control.rs:41–139`). Extend every use/definition walker
for new operations before permitting scalar locals in suspendable functions.

Retain frame shape, reentry type, saved-values array, environment delta, slot,
pending-list state and lazy one-frame-at-a-time behavior. On the actual capture
catch path, save each live FiniteF64 as an ordinary Float, SmallInt as i31, and
internal I64/I32 using their existing box. Restore by saved representation;
never recompute a value. Preserve -0 bits, big-Int references and GC roots.
Normal nonsuspending execution pays no capture box.

Current record emission boxes internal I64/I32 (`emit/control.rs:686–719`) and
restores them (`:834–861`). Extend these matches exhaustively. Segments/joins
already use saved locals' types (`:752–785`), but result and suspended-call
result are hardcoded eqref (`:642–646,776–780`). Keep that protocol: receive a
suspendable call into a Value temporary and unbox in the next ordinary statement.
Point.dst stays Value. R3 therefore keeps may-suspend functions' results Value
and propagates tail-result constraints. Every function with a point still has
a frame (S0).

Closure captures stay canonical heap values, including specialized callbacks.
Recursive closure construction patches capture arrays (`lower.rs:1732–1758`,
`emit/mod.rs:417–425`); no hoisting of reads before patches. Eliminating a whole
closure needs proof that identity comparison, a portability walk, environment
keeping, stores and unknown calls cannot observe it. Known code identity alone
is not that proof.

go is ordinary i: lexical numeric values use ordinary closure capture, while
its keep_env wrapper retains the specified services (`runtime.md:829–831`).
Ordinary closure calls still receive the environment of their call site; numeric
optimization must not introduce environment capture there or remove keep_env.
Task suspension uses these frame rules. There is no scalar task encoding.
`runtime.md:820–845` erases results through Keyed and retains local continuations: materialize ordinary values at those edges. Preserve the accepted
capture set even when optimized code does not read every capture, since removing
a nonportable value could change a portability test. `runtime.md:683` says go
itself does not check closure portability; later portability/lifetime checks
must still agree. Typed closure/heap layouts require a separate codec design.

### R8. Common immutable language reads and bit-identical constants; keep boundary checks

Common field/list reads only with the same receiver version, proved shape and
physical field index, dominated by the original read. A completed read of an
immutable language value may be reused across a call; if it then lives across
suspend, R7 saves it. Do not hoist a potentially failing read/check onto a new
path. Facts after a type/constructor branch do not apply before that branch.

Exclude environment arrays, unpatched closure arrays, request cells,
continuation flags, shadow stacks, host state and bigint scratch arrays. The
values array type is shared with mutable implementation storage, including
closure patches (`emit/mod.rs:417–425`); the Wasm type alone cannot prove
immutability. Initially common semantic IR tuple/record/list
projections with established provenance.

Scalar constants are immediate f64/i64 values. Key Float constants by all 64
bits, never f64 PartialEq, so the zeros remain distinct. Reuse boxes only when
they dominate uses; do not eagerly initialize global boxes on untaken paths.
Module-wide lazy pooling is deferred. Eliminating Horner's eight literal boxes
needs no pool; current emission allocates each use (`emit/mod.rs:491–495`).
Constant folding preserves source rounding/check order; failing expressions
remain runtime stops at their original sites.

Host ABI v3 stays unchanged: Float as f64, Int as sign/address/count, existing
error tags, async status/results, ownership and metadata (spec B.2–B.6).
Synchronous adapters may validate then call workers or encode scalar results
directly. NaN/infinity still stop with `host: value is not correct`, not
`float: result is not finite` (`runtime/abi.rs:41–46,50–68`). A raw host f64 is
not FiniteF64. Async replies validate and box for the existing root/reentry
protocol. No new import, libm, target, Wasm feature or root convention is needed.

### R9. Inlining preserves the logical stop stack

Initially retain logical enter/site/leave actions for inlined calls, with
caller site, tail flag and callee operation site. Fewer native calls cannot
mean fewer language calls. Current shadow protocol: `runtime/mod.rs:72–100`;
tail emission: `emit/mod.rs:390–401`.

The coordinator defers cold stop-stack reconstruction; it is not part of N0–N5.
A later design and separate gate may defer these writes across a straight numeric
stretch to the failure branch. Its compile-time origin stack must reconstruct exactly
the same slots before stop_float. Flush before unknown calls, possible errors,
suspension, final-function handlers or other observable stops; preserve tail
sites. Start with leaf stretches, never defer across control effects. A proved
impossible check needs no site write for that check. Scalar controls omit shadow
bookkeeping; approaching their times may need this stage. This is a general
stop-metadata optimization, not permission to omit unproved checks.

## 5. Pipeline and boundaries

At O0 retain the existing path byte-for-byte. At O2/O3 pass a numeric optimization
flag to lowering, off by default for direct library callers. Options currently
contains only tests/target (`lower.rs:58–64`); CLI already knows opt level
(`crates/ilang-cli/src/lib.rs:12–20,150–162`). Thread the flag through WAT dumps,
tests and artifact construction. Do not silently redefine --wat as post-Binaryen.

Ordered pipeline:

1. Ordinary inference; freeze conservative facts (R1).
2. Existing ANF lowering, retaining facts and source origins.
3. Known-code analysis and bounded worker cloning; compute the transitive
   may-suspend set before any helper inlining and exclude every member (R3).
4. Aggregate replacement and immutable projection commoning (R5/R8).
5. Recompute may-suspend; solve tail-result conventions; select representations;
   insert conversions, guarded Int regions and checks (R2/R4/R7).
6. Range proof/check removal (R6); verify representation, dominance, sites and
   tail conventions. Cold trace paths and captured logical frame chains are
   deferred, not part of this pipeline.
7. Extended control splitting, emission, validation, unchanged Binaryen O2/O3,
   validation again.

No-op analysis must not renumber functions, constructors, globals or sites.
Append new worker ids in stable order. Keep old runtime text unless transformed
regions need additional helpers. When no numeric region transforms, emit the
old path exactly, even though unused numeric runtime is linked. CSE and aggregate
replacement initially run only in numeric-eligible regions; expanding them to
all code is a separate WAT/performance review.

| Boundary | Float / small Int | Aggregate/call behavior |
|---|---|---|
| known worker/local | f64 / proved machine scalar | virtual shape; result convention follows tail graph |
| known generic use | optional bounded worker | original scheme and generic entry remain |
| unknown callback/service | box to Value | original closure/environment; reference entry |
| heap/list/global/capture | canonical Float / i31 or big Int | materialize, no new layout |
| suspend/reentry | box only live scalars on capture | eqref result protocol, exact restoration |
| host | validated f64 / sign-limb format | same ABI and metadata |
| task/Keyed/portability | ordinary heap values | no worker layout or reduced capture set escapes |

## 6. Replacement spec text, section by section

No semantic change is needed. These clarifications can accompany the first
implementation; they do not make worker selection a language promise. Unnamed
text stays, and no section is renumbered.

- **§4.7**, append: “A compiler's choice of how a value is stored or how a call
  is made does not change its type, its type parameters, or the rules for
  generalizing a binding.”
- **§6.2**, append after the list: “An optimization preserves this order for
  every operation that can have an effect or stop. A Float operation whose
  value is unused can still stop (§11.3).”
- **§9.4**, add after rule 3: “Every value that the paused computation will use
  again is kept with that computation. This includes the exact value of an Int
  and the binary64 bits of a Float.” Environment capture in rule 3 stays.
- **§11.1**, append: “A value's storage is not part of the value. The compiler
  may keep a value in a local machine slot or omit a temporary aggregate,
  provided the behavior described here is preserved.”
- **§11.2**, retain unchanged: no small-Int type, overflow exception or bound.
- **§11.3**, retain both bullets, then append: “Each operation rounds its own
  result before that result is used by another operation. An optimization may
  omit a non-finite-result test only when it proves that test cannot fail; it
  must preserve the operation's rounded value. Division by either positive or
  negative zero gives positive zero.”
- **§11.4–11.8**, retain unchanged, including function identity, zero equality,
  negative-zero text, tail space and stop positions.
- **§B.2–B.6**, retain unchanged. Internal scalar calls do not define another
  host ABI.

Memory/stack exhaustion remains a stop (§11.7). Less allocation can change when
that resource limit is reached; it cannot replace arithmetic overflow or a bad
host value with a resource-limit stop. Preserve observable values, effects and
specified stops, not identical time/resource consumption.

## 7. Implementation order, files and gates

Order by gain per risk. Each stage has its own report and review. The coordinator
approved one combined checker/IR/control track, serialized after the current
owners T1, T2 and R2. Coordinate its two shared areas separately:

- **Checker facts:** types/mod.rs, types/infer.rs and expression-fact collection
  follow T1 and T2's completed inference lifecycle, including their completion,
  fixation and deferred-check work (`types-joint.md` §2.3 and §3). Freeze the
  fact table only after that lifecycle finishes; do not insert an early snapshot.
- **Lowering and host seams:** serialize lower.rs and adapter edits with R2 and
  JP. Coordinate T3 field-shape integration separately when those shapes are
  supported; it does not replace T1/T2 checker coordination.

Region numbers identify baseline seams, not ownership of the whole file.
Here `types/` is under `crates/ilang-check/src/`; new numeric modules are under
`crates/ilang-wasm/src/numeric/`.

| Step | Work/payoff | Files and regions to reserve |
|---|---|---|
| N0 | first try emission that exposes construction/consumption to default Binaryen; measure before custom passes; freeze kernels/counters and adversarial/verifier cases | new bench/test/numeric_test.i, bench driver, ilang-wasm/tests/numeric_lowering.rs, spec/control/host tests |
| N1 | R1/R2 scalar Float locals, checks/constants; first local box removal | after T1/T2: types/mod.rs:148–200; types/infer.rs:155–228 plus expression fact collection; new numeric modules; ir.rs:113–208,257–285,497–508; lower.rs:58–64,499–504,1352–1405,1549–1660,1802–1900; emit/mod.rs:284–295,482–554,1075–1081; conditional runtime helper at values.rs:52–59; CLI lib.rs:146–162, main.rs:214–228 |
| N2 | R3 workers/inlining, R7 scalar capture, R9 logical sites; largest combined Float gain | lower.rs:1294–1405,1732–1758,1903–2015; ir.rs:249–275,499–552; control.rs:41–155,239–305,495–619 and use walkers; emit/mod.rs:300–460,554–599; emit/control.rs:625–888; new call-graph/worker modules |
| N3 | R5 pairs/records, R8 projection commoning | new aggregate/commoning modules; field origins at lower.rs:1763–1798,1955–1977; emit/mod.rs:572–599; coordinate T3 shapes, no global layout change |
| N4 | R4 guarded small Int regions and reduced numeric/Bool tests | new integer/range modules; ir.rs:163–185; emit/mod.rs:512–540; reuse runtime/bigint.rs:87–132,220–268,304–315,408–435 |
| N5 | R6 finite proofs; cold stop-stack reconstruction deferred | new float-range module and check/site IR; emit/mod.rs:321–401; no change to lazy trampoline algorithms or captured logical frame storage |
| N6 | tune measured survivors; targeted Binaryen CSE experiment if justified | ilang-binaryen/src/lib.rs:106–112 only with separate WAT/performance gate; recommend no default pipeline change |

N0 first tests “emit so that Binaryen can see it”: place temporary Float
construction and consumption together, retaining finite checks, using the
unchanged default Binaryen pipeline. Measure time, allocation count and hot WAT
on all six real kernels against the existing baseline before introducing any
custom optimization pass. Keep the simplest emission change that meets the
gates; custom passes must address measured residual work. The independent
visibility probe in §2.3 establishes a local opportunity, not sufficient support
for cross-call worker conventions or capture rewriting.

N1 must either include R7 save/restore or conservatively leave all values live
across suspension boxed until N2. No intermediate stage may emit unsavable F64
locals. The conservative option is preferred for the first small commit.

lower.rs:841–1292 (entries/imports/providers) and :2676–2781 (ABI metadata)
remain with their active tracks; coordinate the adapter call-selection seam.
emit/host.rs keeps the control handshake and ABI; serialize any scalar sync
adapter work with R2/JP. No library file is an optimization prerequisite.
Update calling-convention comments when implemented (`ir.rs:9–19`,
`runtime/mod.rs:31–63`).

Acceptance gates:

1. **Behavior:** full scripts/dev-verify green through cache/heavy, plus syntax
   check, at each landable stage. Numeric/control/ABI probes run at O0/O2/O3 on
   native, JS and WASI where supported. Keep O0 as the differential oracle and
   test equal results, effects, specified stops and complete stop traces at all
   three levels, including the transitive-suspension acceptance program in §8.
   Normative expectations still govern; do not bless a shared pre-existing bug
   merely because levels agree. Check exact bits, errors, finals/cancellation, one-shot use,
   environments and deep tail calls. Std-3's independent rounding corpus
   stays bit-equal, including subnormal/huge/hard cases.
2. **WAT:** every O0 program stays byte-equal, numeric or not. At O2, zero
   transformations means byte-equal emitted WAT and printed optimized WAT
   against the same baseline/Binaryen revision. “No numeric code” means no
   eligible reachable numeric operations/values, not absence of i32 runtime
   bookkeeping or unused numeric helper text. Numeric changes can add workers,
   locals and conditional helpers, but host signatures and ABI metadata stay
   equal. Artifact hashes change with bytes; binding plans must be recomputed
   normally (`crates/ilang-cli/src/lib.rs:162–168`). No global renumbering waiver.
3. **Structure:** no Float box/cast/load between scalar worker arithmetic steps.
   Dot keeps input lists but no per-element product/accumulator boxes. Small
   Int loops allocate no limbs on their small path. Tuple kernel loses three
   tuple objects and backing arrays. Fold has a known numeric callback path
   and any reachable erased fallback. Every capture has matching typed save/
   restore; every tail edge uses return_call/return_call_ref with matching
   result convention.
4. **Performance:** alternate baseline/candidate across five quiet runs on the
   same harness/machine; report raw times, all medians and counts. Early N2:
   sine <=3,000 ns, Horner <=70 ns, dot <=700 ns, tuple <=70 ns, fold <=800 ns.
   Int <=1.05x baseline until N4, then <=2,500 ns. Combined targets below are
   acceptance targets, not measured implementation claims. Run-median spread
   >20% requires another quiet paired comparison before claiming pass/fail.
5. **Cost:** no unexplained >5% regression in other benchmarks; median std
   check/build <=1.15x baseline; benchmark Wasm size <=1.25x; worker budgets
   hold on recursive generic stress tests. Narrow a heuristic when a gate
   fails, never source acceptance or numeric correctness.

| Kernel | Combined O2 target ns/call | Allocation target per call | Basis |
|---|---:|---|---|
| fast sine | <=500; stretch <=200 | <=8, explain each survivor | supplied checked scalar 250; 200 is separate 20x-libm gate |
| Horner | <=40 | <=2 | scalar near 10, wrapper/trace margin |
| dot | <=300 | <=2 | scalar 60, allow two-list walk/checks |
| integer | <=500 | <=2, no Int objects on small path | scalar 60, allow guards/fallback edges |
| tuples | <=40 | <=2, no tuple/array objects | scalar near 10 |
| closures | <=400 | <=4, no per-element Float boxes | scalar 60, allow closure/capture array and boundaries |

O0 has no speed target and should retain baseline counts. A <=500 ns sine
compiler gate does **not** release std/float: the coordinator's <=20x-native
ordinary gate and under-1-ms full-range gate still apply. These kernels cannot
establish the interval slow path's worst case.

## 8. Adversarial tests and self-review

Expected values below derive from rules, not merely snapshots. Scratch attacks
include thirteen passing tests; the implementation track must add host,
task and stress cases as well as these negative cases.

| Test | Expected result and rule |
|---|---|
| bits/inspect of -.0.0; equality with 0.0 | 9223372036854775808, text -0.0, equality True (R2/R8; §11.3–11.4/11.8) |
| 1.0 divided by negative zero | bits 0, positive zero (R2) |
| sum x + 1 then subtract x, x = 10^16 | 0.0; compensated residual 1.0, no reassociation/FMA (R2/R6) |
| 2^30-1 + 1; -2^30 - 1 | 1073741824; -1073741825 (R4) |
| 2^63-1 + 1; -2^63 / -1 | 9223372036854775808, no trap/wrap (R4) |
| 7 / 0; -7 % 3 | 0; -1 (R4) |
| generic id at Float and String | 2.5 and string a, same principal scheme (R1/R3) |
| two make(2.5) calls yielding closures | f == f True, f == g False, f() = 2.5 (R3/R5/R7) |
| -0.0 and 2^30-1 live across suspend | (9223372036854775808,1073741824,2.0) (R7) |
| tail relay via unknown Float callback | 0.5 at depth 10,000 and later 1,000,000, bounded stack (R3) |
| Left(x,y)/Right(y,x), read p.x twice | 0.5 for Left(0.25,1.0) and Right(1.0,0.25) (R5/R8) |
| unused x *. x at x = 1.0e308, then return 0.0 | float non-finite stop at multiply (R2/R9) |
| x *. 2.0 then divide result by zero, x = 1.0e308 | stop at multiply, not zero result (R2/R6) |
| overflow in untaken clause | chosen finite value, no stop (R6/R8) |
| Float/String union refined by type test | correct branch, no cast of String to Float (R1/R2) |
| host NaN/infinity or malformed async Float reply | host-value stop with existing site, before worker arithmetic (R8) |
| cancel with final closure capturing -0 and big Int | exact values, final once; second continuation use stops (§9.4/R7) |
| tail cycle returning through erased closure | bounded stack for any depth, Value result convention (R3) |
| scalar record containing a function, later generic equality | original field identity and equality (R5) |
| unknown callback reads override/suspends | caller/resumer environment by §9.4 (R3/R7) |
| numerical helper copied into user module | same eligibility/optimization, no std-name test (R1–R8) |

A compiled/run saved-scalar counterexample:

```i
(i/core, i/control)
probe = () {
  control.boundary((b) {
    x = -.0.0
    n = 1073741823
    y = control.suspend(b, (k) { control.resume(k, 2.0) })
    (core.float_to_bits(x), n + 1, y +. x)
  })
}
```

A tail-adapter counterexample (scratch passes generic id as f):

```i
relay = (f: (Float) -> Float, n: Int, x: Float) -> Float {
  { n == 0 } |> { True -> f(x) False -> relay(f, n - 1, x) }
}
```

If each specialized recursive call waits to unbox a generic result, native
stack grows with n. R3 chooses Value results there, independently of scalar
locals/arguments. A scalar signature is not proof that a tail adapter is safe.

### First attack: findings and fixes

- **Cannot be built from current type output:** the shortcut of reading retained
  expression types was false (Typed locations in R1). Added post-solve fact
  export/verifier; no printed-type parsing or re-inference.
- **Cannot be built by adding only LocalType::F64:** continuation blocks/results
  are hardcoded eqref (`emit/control.rs:642–646,776–780`). R3/R7 retain that
  result protocol and explicitly save/restore live scalars.
- **Unsound Int and wrong cost premise:** int_to_i64 is partial; small Int
  already allocates nothing. Boundary arithmetic disproves blanket i64 lowering.
  R4 now uses proved/guarded regions with exact big fallback.
- **Tail-space violation:** relay exposes result-adapter frames. Added backward
  constraints across tail edges, not only within SCCs.
- **Identity/zero source conflicts:** code-address closure merging changes
  f != g; equality-based Float pooling changes -0 bits. Keep original closures
  and use bit keys.
- **Dead arithmetic still stops:** unused overflow and overflow before division
  by zero must remain. Separate check from allocation and preserve its site.
- **Probe relied on unsupported inference:** unannotated `take = (p) { p.x +. p.x }`
  gets E0506 under §4.8 rule 13. Corrected to p: Pair; the design requires no new
  inferred-field rule.

### Second pass: value, stop and boundary attacks — no serious finding

Ran the expanded eleven-test scratch suite at O0/O2: all passed. Added union
refinement (1.0/2.0), least-subnormal ties (bits 0 and negative-zero bits),
three-times-least-subnormal times 0.5 (bits 2), aggregate equality with shared
versus fresh function fields (True/False), and an unknown callback under two
Bias services (0.5/1.5). These challenge R1/R3/R5/R6/R8; the written rules
already require path facts, exact rounding, original identities and call-site
environments. No new language/type feature was needed.

Seven standalone programs in target/numeric/attacks2 tested dead multiplication,
overflow before zero division, unused overflowing tuple field, untaken overflow,
first-stop order, negative sqrt and cancellation finals. Their O0/O2 status and
complete output matched. Representative sources:

```i
f = (x: Float) -> Float {
  unused = x *. x
  0.0
}
@main
main = () { f(1.0e308) }
```

This stops at the multiplication; removing the unused box must not remove that
check. `pair = (x: Float) { (0.0, x *. x) }` followed by projection of part zero
also stops in the second field, retaining the non-tail caller in its trace.
The cancellation final sees text `-0.0, 9223372036854775808`; its closure values
cannot be replaced by truncated integers or unsigned zero. Negative sqrt stops
with the Float reason; the untaken overflow exits successfully. First-stop
order reports only `panic: before` and its source site.

Raw host probes used a fresh instance after each stop: NaN, +infinity and
-infinity at both O0/O2 give exactly `host: value is not correct`; -0.0 returns
unchanged bits. Eight results are saved in target/numeric/host-results.json.
These exercise the current synchronous boundary, not a future scalar adapter
or async reply decoder. R8 retains validation before giving an adapter the
FiniteF64 proof.

Read-back checks of R1's solver-fact seam, the explicit worker types, R4's
small bounds and R7's eqref destinations found no remaining implementation
contradiction. Minor probe corrections were decimal-point spelling
`1.0e308` and the earlier Pair annotation; neither changes a proposed rule.

### Third pass: capture, tail protocol and source audit — no serious finding

Added a second suspension with x/y live, yielding 3.5 (R7), and a recursive
local closure retaining x at depth 10,000, yielding 0.5 (R3/R7). All thirteen
scratch tests pass at O0/O2. Added a Float overflow after resumption through
outer frames: target/numeric/attacks2/resumed_stop/src/main.i:3–15. Both levels
report the Float reason and exactly these positions:

```text
attack/main:5:2
attack/main:8:6
attack/main:13:37
attack/main:14:25
```

All eight standalone attack programs now have identical O0/O2 status, stdout
and stderr. The resumed program forces the live x to survive capture and
challenges both saved values and R9's pending logical frames. It does not
pretend the proposed unboxed implementation has been tested.

Manually followed the tail-adapter counterexample through R3's backward
constraints and the closed generic-entry/worker graph: an unknown tail return
forces Value before a worker is emitted; no conversion is left after a tail
call. Followed R5 field scalars through R7's liveness/save path and checked that
Point.dst remains Value. Checked generic Float/String use, alternate record
field indexes, signed-zero constant keys, source effects before overflow, and
budget fallback against §4.7, §6.2, §9.4 and §11.2–11.7. No serious hole found.
T3 shapes, future task portability codecs and cold trace reconstruction remain
explicit implementation gates, not assumed existing facilities.

The full measured kernels and fast wrapper type-checked as part of their
project; the note's standalone control/relay snippets were separately checked.
The counter instrumentation was audited after fixing its initial matcher to
include named O0 functions: identity executes one allocation at both levels,
not zero. All published counts use the corrected matcher. Fifty scalar-control
comparisons were bit-equal; all twelve baseline kernel/optimization combinations
were executed and counted. The note's new negative snippet is the same program
as the executed dead-overflow probe, with spacing changed only.

These are two self-review passes of the corrected design, not independent
reviewer approvals or tests of an implemented optimization.

### Independent review correction: transitively suspending inlinees

The independent review in `.rd/cache/gpt/report-review-num.md` found one major
inlining/capture gap and one checker-ownership omission after the self-checks
above. R3 now excludes the full transitive may-suspend set before inlining;
R1/§7/§9 coordinate final checker facts with T1/T2. The earlier self-checks did
not establish the stronger inlining rule.

**Acceptance test before N2:** create a project with `name = "trace"` and put
exactly this source in `src/main.i`. Preserve its lines and single-space body
indentation so the expected columns below remain exact. This is the review's
three-function program (`/tmp/review-num/trace/src/main.i:1–18`):

```i
(i/control)
g = (b, x: Float) -> Float {
 y = control.suspend(b, (k) { control.resume(k, 1.0) })
 x *. x +. y
}
f = (b, x: Float) -> Float {
 y = g(b, x)
 y +. 0.25
}
h = (b, x: Float) -> Float {
 y = f(b, x)
 y +. 0.5
}
@main
main = () {
 control.boundary((b) { h(b, 1.0e308) })
 Nil
}
```

At each of O0, O2 and O3, `i run` must exit with status 1, write no stdout, and
write exactly this stderr, including the final newline:

```text
float: result is not finite
trace/main:4:2
trace/main:7:6
trace/main:11:6
trace/main:16:2
```

The value x is live through suspension in g. Both f and h belong to the
transitive may-suspend set, although neither body contains a direct suspend.
The initial i inliner must not inline f into h or any other member of that set;
check its eligibility/IR dump as well as the O0/O2/O3 output. After resume, the
failed multiplication must retain both pending caller positions (R3/R7/R9;
spec §9.4 and §11.7). One saved physical-frame slot cannot encode the extra
logical frame that the earlier eligibility rule would permit. Keep the
post-inlining recomputation, but do not treat it as a substitute for the
pre-inlining exclusion. Captured logical frame chains require a later design.

The reviewer ran this baseline program at all three levels. This lane also
copied it under `target/numeric/review-trace/` and checked the exact exit status,
stdout and stderr at all three levels after these note corrections. That tests
the acceptance fixture on the existing compiler; it does not claim that the
future inliner has passed its gate.

## 9. Coordinator decisions

These answers are binding for the implementation track, replacing the original
open questions and recommendations.

1. **One combined checker/IR/control track: approved**, serialized after current
   owners T1, T2 and R2. Coordinate checker fact export in types/mod.rs, infer.rs
   and expression facts with T1/T2, separately from lower.rs/host coordination
   with R2/JP (§7). Freeze facts after the completed inference lifecycle.
2. **O2/O3 first: approved.** Keep O0 as the differential oracle and add a test
   that results and complete stop traces agree at O0/O2/O3; normative expected
   behavior remains required. Retain the byte-equality gates in §7.
3. **Budgets are internal heuristics.** Four workers, bounded growth and small
   aggregates are initial tuning choices. Fallback never changes source
   acceptance or principal inference.
4. **Separate performance gates: approved.** Compiler kernels and the Float
   library have separate gates; correct rounding and both Std-3 performance
   requirements remain unchanged.
5. **Cold stop-stack reconstruction: deferred.** Keep eager logical site actions.
   Captured logical frame chains are also deferred; R3's pre-inlining transitive
   may-suspend exclusion is mandatory before N2.
6. **Typed heaps and unrestricted i64 Int: deferred.** No new heap visitors or
   control ABI are needed for the immediate work.

Independent review permits N0 and conservative N1 after checker coordination.
N0 must first expose construction and consumption to the existing Binaryen
pipeline and measure, before deciding that a custom pass is needed.

Not verified here: implementation speed/build-size targets, dev-verify after
compiler changes, T3 record integration, later R2 task/portability code,
all-target execution of the proposed lowering, or the full-range Float gate.
