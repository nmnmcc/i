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
caller size as the growth limit. The exact review fixture checks both its
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
target/numeric-step2/. Counts include the 12-allocation host-wrapper overhead;
allocation gates below subtract that overhead.

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
