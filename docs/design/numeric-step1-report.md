# Numeric step 1

No new language or library state surface.

## Scope and implementation

O0 keeps reference lowering. O2/O3 select numeric emission; direct codegen
library callers retain the reference default. The small CLI edits only thread
this choice through ordinary builds, collected tests, selected roots and WAT
exports, as required by numeric-lowering §5. WAT remains pre-Binaryen.

N0 emits Float arithmetic, finite checks, object construction and field reads
in the caller's body. Binaryen's unchanged O2/O3 pipeline can see construction
and consumption together. Its inline threshold remains 20. No custom Binaryen
pass, extra cleanup pipeline, helper inliner or scalar call convention was added.

N1 selects local encodings in emission, after control splitting. A sorted,
per-function representation plan operates on single definitions only. This
keeps the semantic IR and all control walkers on canonical values; explicit
raw reads and materialization are generated at emission. This is the conservative
interpretation of R1/R7 for this step, without extending continuation storage.
Multi-definition locals remain Value, so branch-local facts cannot rewrite
another incoming definition. Fact lookup uses `FuncId`, not debug names; changing
all IR function names to the same text leaves executable WAT unchanged, covered
by a regression test. Float parameters still arrive as eqref and are
unboxed inside the reference entry using final checker facts. Every result and
tail-call result convention remains eqref.

Float operations use f64 locals; each potentially overflowing operation retains
its finite check and eager source-site action, even when unused. Constants use
all 64 bits. Division evaluates both operands, handles either zero with positive
zero, and otherwise divides and checks. There is no reassociation, fused
arithmetic, finite-check removal, or fast-math option. Canonical Float inputs and
results seed a local reference cache. Scalar definitions materialize once at the
first reference escape; aliases copy an existing reference without forcing a box.
The first paired N1 measurement exposed 19 extra sine boxes (and one extra dot/
tuple box) when incoming boxes were discarded; this measured regression prompted
the cache, rather than a new optimizer pass.

An Int region has an exact signed-i31 scalar in an i64 local and a nullable
canonical fallback reference. Null marks the scalar successor, not source Nil.
Two small operands are widened before add/subtract/multiply/divide/remainder or
negation. The exact result is checked against [-2^30, 2^30-1]; otherwise the
existing exact packer receives that current result. A big operand uses existing
arbitrary-precision arithmetic once. Canonical small results reopen a scalar
region. Neither overflow nor fallback replays source calls. Zero division and
remainder return zero before the machine instruction; -2^30/-1 and negation of
-2^30 pack +2^30 exactly. General Int never uses int_to_i64. Numeric comparisons
can stay i32 conditions; escaping Bools become the existing canonical values.

Checker expression and parameter handles are retained through inference. Their
small fact tables freeze only after completion, fixation, generalization,
deferred checks and entry checks. Only closed Float terms and finite small-Int
sets produce facts, validated against the still-live solver. Unknown, recursive,
unbounded and mixed terms remain Value. This does not constrain, instantiate,
generalize, print or rename a type to decide its representation. Lowering carries
facts only for fresh expression definitions, not narrowed uses of older locals.

## Prior art actually read

- Pinned Binaryen `src/passes/Heap2Local.cpp`, pass contract and exclusivity/
  escape checks: expose construction and every read before replacing anything.
- `.rd/ref/js_of_ocaml/compiler/lib-wasm/gc_target.ml`,
  `Memory.box_float`/`unbox_float`: ordinary WasmGC construction/field reads.
- `.rd/ref/ocaml/asmcomp/cmmgen.ml`, `join_unboxed_number_kind`,
  `is_unboxed_number_cmm`, `transl_let`: local alternate encodings, strict
  agreement at joins, canonical boxing at generic uses.
- `.rd/ref/ghc/libraries/ghc-internal/src/GHC/Internal/Bignum/Integer.hs`,
  `integerAdd` and small/big constructors: exact promotion with existing operands.

## Where canonical values remain, and why

- Every reference parameter/result, direct/unknown call, stored function and tail
  edge: scalar signatures and worker cloning are N2. Closure identity and current
  call-site environment remain untouched.
- Every function containing a saved-frame point: all its values remain boxed,
  including values dead at some points. Segments, joins, restoration, environment
  deltas and lazy capture are unchanged. N0's visible arithmetic on the normal
  path still returns canonical boxes; reentry uses the reference helpers.
- Closure captures, globals, list elements, tuple/record fields, service values,
  host values, portability walks and saved continuation data: existing layouts
  and observers require canonical values.
- Generic equality/inequality, and runtime operations without a local scalar form
  (including floor, ceiling, square root, bit conversion and inspection): the existing checked runtime
  operation consumes canonical arguments and returns canonical values.
- Mixed/unknown types and multiple definitions at joins: conservative Value
  encoding; no guessed type or SSA assumption.
- Int results outside i31, or operations receiving big Int: existing exact
  packer/limb arithmetic. An i64 local is only a widened i31 operation temporary,
  never an unrestricted source Int encoding.
- Macro resource-metered execution keeps its existing reference representation
  and accounting. There are no changes to host ABI v3 or runtime metadata.

## N0 experiment, before scalar selection

`tests/perf/numeric` freezes the six kernels from numeric-lowering §2.2 plus
identity. Interpretation: the Int loop uses the ordinary literal-zero clause
rather than an intermediate Bool equality; every variant uses this same source
and the same 128-iteration sum. These are paired current-compiler measurements,
not a claim of byte-identical inputs to the historical design-note artifacts. `math.i` is a diagnostic copy of `de4d58d:std/src/float.i` with the
same fast-sine wrapper. It is benchmark code, not a change to the float library
or its release/rounding contract. The six expected answers at x=0.5 are checked.

`cargo run --locked -p ilang-cli --example numeric_measure -- OUTPUT` builds
reference, visible and scalar variants with the same Binaryen O2 settings.
Wasmtime 50.0.0, copying collector, 256 MiB initial heap, Cranelift Speed; Linux
x86-64. Timing excludes compilation, initialization and instrumentation. After
200 warmups, each of five batches alternates target/identity for 1,001 samples;
batch order alternates baseline/candidate. Values are f64 ABI calls on a reused
instance. Each time below is the median signed target-minus-identity nanoseconds.
Do not compare these absolute timings to the older note's Wasmtime 50.0.1 C timer.

Allocation instrumentation is inserted into printed **optimized** WAT at every
struct.new*/array.new*, with reset/count exports appended after all original
indices. Counts reset after initialization; instrumented answers are checked and
instrumented modules are never timed. Identity executes 12 allocations here
(the newer entry/root wrapper plus input), versus one in the older note. Raw
counts below include this fixed overhead. Subtracting 12 isolates kernel work.

First full experiment, before N1, raw five batch medians:

| Kernel | Baseline O2 ns | N0 O2 ns | Baseline / N0 allocations |
|---|---|---|---|
| certified fast sine | 14181,14170,14202,14243,14201 | 4538,4517,4539,4588,4416 | 322 / 56 |
| Horner | 2303,2432,2403,2424,2324 | 81,50,50,90,81 | 34 / 13 |
| dot | 4996,4747,4677,4717,4808 | 4547,4387,4267,4457,4278 | 76 / 44 |
| integer | 1843,1863,1914,1883,1875 | 1864,1873,1904,1853,1843 | 13 / 13 |
| tuples | 291,301,322,310,281 | 171,172,170,150,150 | 33 / 27 |
| closures | 4496,4587,4587,6261,4557 | 4137,4297,4156,5709,4136 | 79 / 47 |

Module size before: 65,522 bytes; N0: 75,679 bytes (+15.5%).
N0 alone leaves one Horner result box after subtracting identity overhead; its
21 temporary/literal allocations disappear through default Binaryen. Dot and
closures lose 32 local product boxes each; values crossing calls remain. Sine
loses 266 allocations. Tuple objects/arrays and returned fields still escape.
This supports keeping ordinary Heap2Local rather than writing a custom box
elimination pass. N1 adds explicit local encoding and guarded small-Int arithmetic,
not another whole-program pass. The first run had >20% timing spread in small
kernels/closures; quiet paired reruns follow before making timing claims.

Artifacts: ignored `target/numeric-step1/n0-full/{samples.csv,medians.csv,
*-count.wasm,*.wasm,*.wat}`; the committed harness regenerates all of them.

## Final N1 measurements

After materialization reuse, the second five-batch quiet comparison below is the
confirmation run. First corrected run and confirmation both retain all six
expected answers and the same allocation counts. No N2 latency gate is claimed;
workers, callback specialization and aggregate elimination are out of scope.

| Kernel | Baseline O2 median ns | N0 O2 median ns | N1 O2 median ns | Allocations baseline / N0 / N1 |
|---|---:|---:|---:|---|
| certified fast sine | 14331 | 4607 | 4287 | 322 / 56 / 56 |
| Horner | 2453 | 69 | 59 | 34 / 13 / 13 |
| integer | 1814 | 1861 | 1071 | 13 / 13 / 13 |
| dot | 4666 | 4337 | 4226 | 76 / 44 / 44 |
| tuples | 280 | 150 | 162 | 33 / 27 / 27 |
| closures | 4538 | 4206 | 4235 | 79 / 47 / 47 |

Module bytes: baseline 65,522; N0 75,679; final N1 75,505 (+15.24% against
baseline, below the note's 25% size ceiling). The clear additional N1 gain is the
Int path: 1,814 to 1,071 ns in the confirmation run (1.69x); no Int limbs are
allocated on its small path. Float allocation removal is primarily N0's work.
N1 makes scalar locals and materialization explicit while retaining N0's counts;
small Float timing differences between N0 and N1 are not robust speed claims.

All five final run medians (ns), including outliers, follow. Run A is the first
corrected measurement; B is the confirmation, repeated because spread exceeded
20%. C regenerated the same bytes after the function-ID refinement and serves as
the final artifact check. All use the committed `numeric_measure` driver and the
shared quiet lock.
The quiet wrapper currently reserves slots 1–3 while the shared heavy wrapper
has six slots; it does not prevent unrelated CPU work. Scheduling outliers and
10 ns timer quantization remain visible, especially in short kernels. These are
observations, not worst-case timings or a claim that later-stage targets pass.

| Run | Kernel | Baseline | N0 | N1 |
|---|---|---|---|---|
| A | certified fast sine | 14842,14722,15215,14653,14401 | 4759,4627,4707,4656,4597 | 4395,4366,4478,4427,4528 |
| A | Horner | 2493,2542,2694,2495,2433 | 120,49,60,80,109 | 100,40,29,50,70 |
| A | integer | 1873,1843,1893,2905,1923 | 1905,1834,1903,2835,2774 | 1110,1111,1352,1332,1121 |
| A | dot | 4747,4768,5007,4729,4758 | 4367,4286,4407,4347,4348 | 4316,4327,4447,4417,4337 |
| A | tuples | 281,290,281,281,261 | 150,162,151,160,181 | 140,150,141,140,161 |
| A | closures | 4678,4537,4507,4527,4536 | 4286,4195,4177,4226,4186 | 4176,4107,4207,4204,4195 |
| B | certified fast sine | 18999,14201,14331,14361,14191 | 4618,4607,4607,4597,4657 | 4257,4275,4306,4287,4347 |
| B | Horner | 2444,2464,2404,2453,2473 | 69,60,41,90,151 | 59,40,20,69,110 |
| B | integer | 1814,1791,1843,1834,1812 | 1824,1833,1911,1884,1861 | 1061,1061,1071,1092,1071 |
| B | dot | 4677,4628,4687,4666,4646 | 4377,4307,4337,4356,4296 | 4245,4226,4217,6170,4215 |
| B | tuples | 250,291,280,260,302 | 140,250,150,168,150 | 162,199,160,141,162 |
| B | closures | 4547,4507,4526,4538,4548 | 4206,4207,4176,4187,4206 | 4287,4237,4235,4206,4186 |
| C | certified fast sine | 14662,13722,13801,14492,13390 | 4446,4497,4377,4355,4337 | 4158,4138,4046,4046,4026 |
| C | Horner | 2204,2204,2396,2223,2183 | 109,40,41,80,91 | 70,29,1,49,81 |
| C | integer | 1873,1865,1903,1923,1923 | 1793,1773,1833,1833,1772 | 1022,1051,1051,1042,1013 |
| C | dot | 4427,4417,4437,4417,4436 | 4087,4065,4116,4106,4097 | 4016,4045,4126,4037,4047 |
| C | tuples | 340,271,260,250,310 | 150,152,159,152,160 | 141,130,141,130,220 |
| C | closures | 5668,4406,4376,4368,4327 | 4056,4056,4086,4026,4016 | 4056,4036,4107,4046,4026 |

Final confirmation SHA-256:

- baseline: `e64a24dada36b86cd6b469d44309d751845cf37fd8eed709339ec04b8b935c9d`
- N0: `012d85f8ac18ec42dfddcdecf16267bd26854c835a12dbc7fd2891f6c15277eb`
- N1: `f34bee4c464bd253b945806a1acd243f7c944936b6549ef0d1480f8675f1afdc`

Baseline and N0 binaries are also byte-identical between the initial N0 experiment
and the final confirmation, separating N0's effect from N1. Raw samples, WAT and
instrumented modules are under ignored `target/numeric-step1/materialized-paired`
and `confirmation-paired`; final artifact-check samples are under `identity-check`.
All reported final run medians are committed above.

Reproduce from this lane:

```sh
R=/home/agent/Programming/i
$R/.rd/cache/heavy direnv exec $R cargo build --locked -p ilang-cli --example numeric_measure
$R/.rd/cache/quiet direnv exec $R target/debug/examples/numeric_measure target/numeric-step1/reproduce
```

## Verification

The generated corpus currently contains 111 numeric programs, each with normative
expected values, Float bits or stop reasons, plus final-effect expectations.
The same corpus runs reference/O0, visible/O2, scalar/O2 and scalar/O3. It compares
entire language stop messages, including all logical positions; incidental
engine-native backtraces are not language stop traces. Each stop gets a fresh
instance. Coverage includes every integer operator at i31/i64 boundaries, mixed
sign division/remainder and zero divisors, promotion/demotion, rounded/subnormal
Float operations, signed zero through inspect and bit conversion, unknown calls,
closures, records, loops, factorial, finals, cancellation, two suspensions and
three transitively suspending helper frames. Nonfinite bit patterns and unused
operations still stop. O0 emitted bytes and nonnumeric O2/O3 WAT/bytes are checked.

Final verification (all Cargo commands used the shared heavy wrapper and
`direnv exec /home/agent/Programming/i`):

- `cargo check -p ilang-check -p ilang-wasm -p ilang-cli --all-targets`: passed;
  final backend follow-up `cargo check -p ilang-wasm -p ilang-cli --all-targets`
  also passed.
- `cargo test --locked -p ilang-check -p ilang-wasm`: passed. Four pre-existing
  ignored checker tests remain; no numeric tests are ignored.
- `cargo test --locked -p ilang-cli --test numeric_lowering`: 3 passed, 0 failed;
  includes 111 programs at reference/O0, visible/O2, scalar/O2 and scalar/O3,
  nonnumeric byte equality and code-identity invariance.
- `cargo build --release --locked -p ilang-cli`: passed on implementation
  revision `3be6ec93`.
- `cargo test --workspace --locked`: passed on that revision. Earlier complete
  workspace/acceptance runs also passed before the final identity refinement.
- The exact task acceptance check: **passed**, including a clean starting tree,
  the numeric project build and the differential/report discovery checks.
- Std: **1845 passed, 0 failed** at each of O0, O2 and O3.
- The exact task factorial/zero-divisor/BigInt fixture produced, at all three
  levels:

```text
265252859812191058636308480000000
9223372036854775807
```

- `cmp` of the saved original O0 benchmark Wasm and WAT against the final
  release compiler's O0 output: passed, byte-for-byte.
- Baseline/N0 artifacts from the first experiment versus final confirmation:
  passed, byte-for-byte. The final implementation regenerated identical baseline,
  N0 and N1 bytes after the function-ID refinement. The actual acceptance CLI
  build `tests/perf/numeric/build/numeric.wasm` also has the measured N1 hash.
- All measurement runs completed and checked expected answers separately from
  timing. `rustfmt --check` on changed Rust files and `git diff --check`: passed.

Initial harness/scaffolding failures, corrected before the final passes:

| Command | Failing line | Correction |
|---|---|---|
| `cargo check -p ilang-wasm -p ilang-cli --all-targets` | `error[E0283]: type annotations needed` | Kept the original concrete Options entry points; numeric variants are explicit. |
| same check | `error[E0063]: missing field numeric_enabled in initializer of emit::Emitter` | Completed the existing emitter test initializer. |
| `cargo check -p ilang-cli --example numeric_measure` | `error[E0308]: mismatched types` (expected wasmtime::Error, found anyhow::Error) | Used the pinned Wasmtime error type in the measurement import hook. |
| `cargo check -p ilang-cli --test numeric_lowering` | `error[E0277]: Vec<std::string::String> doesn't implement std::fmt::Display` | Rendered diagnostics with Debug in the harness assertion. |
| `cargo test -p ilang-cli --test numeric_lowering` | `error[E0105]: a block ends with an expression` | Put the following tuple on a new line rather than adding a comma. |
| same test | `error[E0313]: the host export numeric/cases.case_0 is not a function in the exports of a module numeric/cases` | Added the source module export group. |
| same test | `error[E0201]: b is not defined` | Used ordinary record field access; spec §7.1 has no record patterns. |
| same test | `error[E0115]: a group in ( ) that is not followed by = is the imports or the exports, which come first in a file` | Moved the export group into the file header. |

The failing diagnostic lines were:

```text
error[E0283]: type annotations needed
error[E0063]: missing field `numeric_enabled` in initializer of `emit::Emitter<'_>`
error[E0308]: mismatched types
error[E0277]: `Vec<std::string::String>` doesn't implement `std::fmt::Display`
error[E0105]: a block ends with an expression
error[E0313]: the host export `numeric/cases.case_0` is not a function in the exports of a module `numeric/cases`
error[E0201]: `b` is not defined
error[E0115]: a group in `( )` that is not followed by `=` is the imports or the exports, which come first in a file
```

No unresolved failure or unfinished N0/conservative-N1 behavior remains. N2+
workers, scalar parameters/results across calls, transitive inlining, typed heaps,
unrestricted i64 Int, aggregate replacement and range-based check removal were
intentionally not implemented in this step. Branch: `lane/jp`.
