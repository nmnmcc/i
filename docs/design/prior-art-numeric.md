# Numeric prior art: representation, functional code, and rounded math

Source study, 2026-11-11. Proposals for the coordinator, not an amendment to the
accepted numeric design or permission to ship the withheld float library.
Only this note is committed. No reference source is copied into i.

The main result is that `numeric-lowering.md` has the right architecture.
Worker/wrapper, local unboxing, bounded specialization, and scalar products all
have direct precedents below. The missing emphasis is on profitable joins and
call boundaries, worker argument count, and independently verified math tables.
For full-range mathematics, start from CORE-MATH's reduction and certification
methods. A literal C-to-i translation would invalidate important assumptions.

## 1. Sources, method, and alternatives

The controlling sources read were `docs/spec.md` §§4.1, 4.7–4.8, 6.2, 9.4–9.5,
11.1–11.8 and 13; `docs/design/syntax.md`; all of
`docs/design/numeric-lowering.md`; the float sections of `docs/design/std-3.md`,
including §13 on `lane/std3-float`; the plan's “已定的设计” and Std track; and
`.rd/cache/ledger.md`, especially the Std-3 profile and numeric-review rulings.
The ledger preserves correct rounding, the ordinary <=20x native-libm gate,
and the full-range under-1-ms gate. Float remains withheld. This study does not
reopen those decisions.

All i code citations refer to the lane's starting `redesign` commit
`c2f41b734f2af530a82e3510eb778f8e1402075b`. The float branch is pinned to
`de4d58d04c39539cdcb20ad68e7d93e5f5453795`. `F:` below means a file at that
commit, read with `git show`, not the older file on redesign. `W:` means
`crates/ilang-wasm/src/`; `B:` means `crates/ilang-binaryen/binaryen/`.
Reference paths start at `/home/agent/Programming/i/.rd/ref/`.
A reference citation gives the file, line or region, and the function or named
code block read. It describes this snapshot, not every release of the language.

| Reference | Commit read |
|---|---|
| GHC | `19ff36f2aede0ca2e554016ff22c71e73aa9803b` |
| OCaml | `75c151eb70092b6dac7aed6cc01a1a21870a0e70` |
| Go | `cd98b3b9e2f0a79dc207049202f09748e8afe580` |
| wasm_of_ocaml, in js_of_ocaml | `8982a1e112df986f107759d173f26aeedd2172f9` |
| Flambda2, in flambda-backend | `f81c7cb7fa317607e3bdab02906b83a59e453ac0` |
| CORE-MATH | `018fa97cbdacf59924b899a9dfa422d532cb2bb7` |
| musl | `7813e9119eb5fe681b6750b8a033a0490bca0941` |
| CPython | `44ff6c9c3c64c880bfef3288e57fa1a8a724883a` |
| Zarith | `7bbc77f0080bc66acf49daff5a9a170266f0b616` |
| Binaryen, repository submodule | `fba5005a132d85d578be5bd010e0487ed8e2ce3e` |

The pre-existing references were listed first. I added shallow clones of
`https://github.com/ocsigen/js_of_ocaml.git`,
`https://github.com/ocaml-flambda/flambda-backend.git` (sparse checkout of
`middle_end/flambda2`), and `https://github.com/ocaml/Zarith.git` under `.rd/ref/`.
They were not edited or built in place. Binaryen was read from the main
checkout's populated submodule, read-only; this lane's submodule is empty.
No claim below depends on memory-only recollection of an implementation.

This is a study, so the three alternatives are implementation strategies to
compare, not three proposed language extensions. Criteria, in order: unchanged
values, stops and tail space; ordinary libraries and principal inference;
measured work removed; implementation effort; code size and proof burden.

| Strategy considered before choosing | Useful part | Limitation |
|---|---|---|
| A. Expose local boxes and let Binaryen do all optimization | Smallest first experiment; reuse Heap2Local | Surviving reference calls still require boxes; reference-type refinement is not a scalar ABI conversion (§5) |
| B. Adopt a full native functional compiler architecture now | Demand, result-shape and specialization analyses are well studied | GHC's laziness machinery and OCaml's native stack layouts solve problems i does not have (§§2–3) |
| C. Keep the accepted scalar-worker plan and adopt specific proven analyses | Changes only internal encodings; incremental and measurable | Requires explicit join, call, suspension and trace verification (§§2–5) |

Choose C, starting with A as N0 already requires. Take B's worker argument
budget, result summaries and benefit accounting, without introducing source
unboxed types, layout polymorphism, or a new library privilege. Independently,
compare three math choices: retain the current interval implementation as the
production engine; translate CORE-MATH literally; or adapt its tables/reduction
with new i-specific bounds and retain interval fallback. Choose the third:
the first misses the recorded gate, and the second assumes FMA and C integer
semantics that ordinary i operations do not have (§6).

Evidence labels: **read** means source inspection; **measured here** means the
native experiment in §6.6; **reported** means earlier i measurements, not rerun;
**proposal/inference** means an expected consequence, not an implemented result.

## 2. GHC: workers, demand, product results, and specialization

### 2.1 What the code does

The representation distinction is explicit in the source:
`ghc/libraries/ghc-internal/src/GHC/Internal/Types.hs : 553`, `Double`, wraps
`Double#`; `RuntimeRep` (:798–814) includes a 64-bit floating representation
and tuple/sum representations. `ghc/compiler/GHC/Types/RepType.hs : 402–440`,
“RuntimeRep and PrimRep”, explains that an unboxed product may need several
machine components. Take internal scalar/product representations, not the
source spelling or the assumption that an unboxed value is always one word.

GHC analyzes how arguments are used, separately from their source types.
`ghc/compiler/GHC/Core/Opt/DmdAnal.hs : 63–100` (`DmdAnalOpts`,
`dmdAnalProgram`) carries strictness/boxity settings and a maximum worker
argument count. Its application case (`dmdAnal`, around :472) analyzes the
function under a call demand and transfers the resulting argument demand to
the actual argument. `finaliseArgBoxities` (:2090–2190) trims unboxing facts to
what worker/wrapper will actually implement and spends a budget while expanding
products. This is more than “the argument has a numeric type.”

The consuming transformation is concrete:
`ghc/compiler/GHC/Core/Opt/WorkWrap/Utils.hs : 212–287`, `mkWwBodies`, builds
argument unpacking, a worker, result unpacking, and a wrapper that rebuilds the
source result. `isWorkerSmallEnough` immediately below bounds expanded arity.
The note “Worker/wrapper for CPR” (:1736–1777) explains
`mkWWcpr_entry`: a pair of boxed integers can travel as two raw machine results,
then be rebuilt only for a caller needing the boxed pair. That is the direct
precedent for i's Double pairs; it does not require exposing unboxed tuples to
i programmers.

Constructed product result analysis is a separate forward fact. In
`ghc/compiler/GHC/Core/Opt/CprAnal.hs : 182–207`, `cprAnalProgram` processes
bindings and uses `cprFix` for recursive groups. `cprTransform` (:338–365)
consults function signatures and constructor information.
`cprTransformDataConWork` (:380–399) rejects existential or definitely recursive
constructor expansion and limits size. Nested fields need strictness or a
termination fact. Take the separation of result shape from argument demand;
leave the lazy-field termination machinery out of i's initial implementation.
In i, operands have already run in source order (§6.2), but evaluating an unused
operand can still stop (§11.7).

`ghc/compiler/GHC/Core/Opt/SpecConstr.hs`, “Specialising for constant
parameters” (:283–325), explains why a recursive higher-order call with a known
function argument becomes worth specializing: the callback can become a direct
call and then inline. “Limit recursive specialisation” (:584–618) gives the
opposite example: specializing on increasingly long list constructors never
finishes. The `specRec` policy limits that process. Take known callback code
and stable representation keys; leave arbitrary value/shape specialization
out of the first i pass. Different captured Float values must not create new
workers.

Inlining is guided by the use site. In
`ghc/compiler/GHC/Core/Opt/Simplify/Inline.hs : 302–442`, `tryUnfolding` checks
saturation, argument discounts, result context, work-free status and adjusted
size; deep case nesting adds a penalty. A known constructor argument or a
caller immediately inspecting a result gives information that plain callee
size misses. Take these signals; do not transplant GHC's tuning constants or
its source pragmas into i.

Unboxing does not abolish representation constraints at polymorphic calls.
`ghc/compiler/GHC/Core/Lint.hs : 764–780`, “Checking for representation
polymorphism”, explains why compulsory internal unfoldings are checked after
expansion; binder lint at :1838–1842 requires a fixed runtime representation
apart from stated exceptions. A raw Double representation cannot simply be
substituted into every erased polymorphic entry. Take an internal representation
verifier. Do not add GHC-style representation kinds or unboxed source types:
those would be a separate language proposal, unnecessary for this work.

### 2.2 Comparison and ordered recommendations

**Already alike.** i's accepted R1/R3/R5 separate value types from representation,
retain generic entries, bound worker variants, specialize known callbacks and
flatten small products after inlining. Its current ANF fixes evaluation order
(`W:ir.rs:1–19`, module contract); this is a useful starting IR.

**Different.** The current emitter gives every source function eqref arguments
and result (`W:emit/mod.rs:284–295`, `header`), and `Func` has no explicit
result representation (`W:ir.rs:497–508`). These are implementation choices,
not consequences of set types or principal inference. i's strict evaluation,
observable stops and bounded tail calls *do* require different legality rules
from a direct import of GHC's demand transformation (spec §§6.2, 11.6–11.7).

**Recommended, highest gain per effort first:**

1. Keep N1/N2 scalar workers; a full lazy-language demand analysis is unnecessary
   for eager i. Before expanding product signatures later, cap generated scalar
   argument count, allowing the original source arity. Variant counts and small
   individual products do not bound total signature expansion. This extra budget
   is not a blocker for scalar Float N1. It belongs in R3's future selector,
   `W:ir.rs:499–508` and `W:emit/mod.rs:284–295`.
2. In N2, score known callback calls and construction/projection cancellation
   before increasing a global inlining limit. Add reason counters to R3;
   the call-selection seam is `W:lower.rs:1294–1405` and
   `W:emit/mod.rs:443–457`. Use the original closure and environment.
3. Keep N3's accepted inlining-first scalar products. If pair-returning calls
   still dominate afterward, propose bounded result-shape summaries across
   known functions, including recursive groups. Accepted R5 explicitly defers
   out-of-line multivalue returns: this would be a later extension, not an N3
   prerequisite. Start with one known product shape and retain R3's backward
   tail-result constraints. Do not import all of CPR.

## 3. OCaml: local unboxing, flat storage, Flambda, and effects

### 3.1 What the code does

`ocaml/asmcomp/cmmgen.ml : 1263–1320`, `transl_let`, selects an unboxed local
when the bound expression contains removable numeric allocation; it avoids
unboxing a mere constant reference when that saves nothing. Generic values
require stricter checking that returning branches agree. The supporting
`join_unboxed_number_kind` and `is_unboxed_number_cmm` (:271–350) merge branch
facts, allow nonreturning branches, and distinguish known numeric kinds from
generic branches. `add_unboxed_id` (:79) records the alternate local.
`transl_catch` (:744–792) similarly gathers argument information for static
handler joins. Take representation joins and profitability; do not interpret
“strict” in that helper as a new i source strictness annotation.

Heap storage is a separate decision. `ocaml/lambda/translcore.ml : 1022–1050`,
`transl_record`, maps `Record_float` to a float-array constant or construction.
`ocaml/asmcomp/cmmgen.ml : 795–805`, `transl_make_array`, emits unboxed elements
for `Pfloatarray`, but ordinary pointer/int arrays use the general allocation
path. `transl_unop`'s `Pfloatfield` case (:858–860) loads the raw field and boxes
it at a generic value boundary. Local unboxing and flat persistent storage are
therefore distinct opportunities. Take the distinction, not a special rule for
an i library named float or an assumption that all polymorphic lists become flat.

Flambda's `ocaml/middle_end/flambda/unbox_specialised_args.ml : 30–108`,
`Transform.what_to_specialise`, extracts projections from specialized arguments
and propagates them through invariant parameter relationships in recursive
functions. `inlining_cost.ml : 229–240,311–320`, `Benefit` and `evaluate`, account
for removed calls, allocations, primitives, branches, and indirect calls made
direct. Take those concrete benefit categories; ordinary user functions should
qualify exactly as numeric helpers do.

Flambda2 adds a particularly useful join discipline.
`flambda-backend/middle_end/flambda2/simplify/unboxing/unbox_continuation_params.ml : 21–149`,
`refine_decision_based_on_arg_types_at_uses` and `make_decisions`, first proposes
unboxing, then checks arguments at each continuation use and tracks invalid
rewrites. `is_unboxing_beneficial.ml : 22–103`,
`is_unboxing_beneficial_for_epa` and `filter_non_beneficial_decisions`, favor
already available components instead of newly inserted loads; recursive loops
receive different treatment in `make_decisions`. `unboxers.ml : 21–94` represents
numeric unboxing with explicit kinds and proof queries.
`optimistic_unboxing_decision.ml : 72–105` caps recursive depth and rejects
unhelpful symbol aliases. Take explicit edge conversions, proof facts and
bounded depth. These are compiler IR continuations, **not** captured user effect
continuations.

Flambda2 also tries simplification before judging an inline:
`flambda-backend/middle_end/flambda2/simplify/inlining/call_site_inlining_decision.ml : 45–98,288–308`,
`speculative_inlining` and `might_inline`, analyze a trial body and compare its
cost with a threshold. Useful as a later model; a full speculative simplifier
is not a prerequisite for i N1/N2.

Native effects do not inherently prohibit raw numeric locals. The save path is
visible across compiler and runtime: `ocaml/asmcomp/cmmgen.ml : 917–923,1214–1218`
lowers `Pperform`/`Presume` to calls; `asmcomp/amd64/proc.ml : 307–313`,
`destroyed_at_oper`, marks all physical registers clobbered by OCaml calls;
`asmcomp/spill.ml : 161–176`, `reload`, requires all live-across-call registers
to be spilled. `ocaml/runtime/amd64.S:272–285`, `SWITCH_OCAML_STACKS`, saves the
old stack pointer and restores the new one. `caml_perform` (:1127–1207) attaches
that stack to the continuation; `caml_resume` (:1227–1277) switches back. Thus
raw numeric spill slots can stay in the saved stack without becoming GC roots.
`ocaml/asmcomp/amd64/emit.mlp : 239–254`, `record_frame_label`, includes `Val`
registers/stack slots in GC root metadata, not Float slots.
`ocaml/runtime/fiber.c : 262–337`, `scan_stack_frames` and `caml_scan_stack`,
scan listed roots and stack-chain handlers. `caml_continuation_use_noexc` and
`caml_continuation_use` (:688–724) take the saved stack and enforce one use.
Take precise live-value representation and one-use ownership. Leave native
stack scanning to OCaml: i's explicit WasmGC frame records need their own save
and restore encoding; an OCaml native stack is not an implementation recipe
for serializing a Wasm local.

### 3.2 Comparison and ordered recommendations

**Already alike.** R1/R2 and R7 intend local unboxing plus explicit boxing at
escapes. i already distinguishes `Value`, `I64`, `I32` and `Env` locals
(`W:ir.rs:113–120`), and its control IR records saved locals
(`W:ir.rs:510` onward, `Point`).

**Different.** There is no F64 local in that enum. Source parameters are allocated
as `Value` (`W:lower.rs:1352–1368`, `function`). Suspension's destination block
and try expression are hardcoded eqref (`W:emit/control.rs:622–646`,
`suspendable`); segment headers also return eqref (:760–780). Flat records and
lists are not present: fields are `$values` references and list heads are
references (`W:runtime/values.rs:62–73`, `list_cons`, `record`, `tuple`). The
existing representation is not forced by i's language rules. Preserving saved
values, environments, identity and stop positions is forced by §§9.4 and 11.

**Recommended, highest gain per effort first:**

1. Make joins and conversions explicit in N1's representation verifier. Count
   eliminated allocations and newly inserted boxes per branch/loop, borrowing
   `transl_let` and Flambda2's benefit check. Amend R1/R2 implementation guidance;
   change the future numeric pass, not source inference.
2. Keep conservative N1: leave values live across suspension boxed until R7
   save/restore is implemented. Later validate every predecessor and saved
   field; never infer a captured-frame solution from Flambda2's IR joins.
3. Do N3 pair/projection elimination before flat heap layouts. A typed immutable
   Float array could later help audio, geometry and tables, but it requires a
   separate general storage design and visitor/ABI review. Keep §9's deferral;
   no float-only heap-layout exception in this study.

## 4. Go: escape summaries, inlining cost, SSA order, and Wasm shape

### 4.1 What the code does

`go/src/cmd/compile/internal/escape/escape.go : 18–85`, package algorithm, builds
weighted location edges, checks that stack pointers neither escape to heap nor
outlive their frame, and exports parameter-to-result/heap summaries for known
calls. It deliberately does not distinguish every field or array element.
`escape/solve.go : 61–140`, `walkAll` and `walk`, use a work queue, grouped root
states and bounded dereference propagation to a fixed point. Take the idea of
small summaries and monotone worklists. Leave pointer-address analysis and
stack allocation of escaping GC objects out of i's initial numeric pass.
Removing a box entirely is the useful operation here.

`go/src/cmd/compile/internal/inline/inl.go : 50–85,201–240`, `inlineBudget`, has
a base budget of 80, extra call cost 57, a larger hot-function budget and a
once-called closure allowance. `hairyVisitor.doNode` (:490 onward) charges
operations and applies legality exclusions; `inlineCostOK` (:985 onward)
makes the call-site decision. The same code contains package/name exceptions,
including `internal/runtime/maps` and `internal/abi`. Take bounded cost,
call-site benefit and diagnostics. Leave these exceptions: i explicitly gives
no library privileged optimization rules. The numbers 80 and 57 are not
portable performance laws.

The current Go snapshot moved its pass list:
`go/src/cmd/compile/internal/ssacompile/compile.go : 414–460`, `passes`, performs
copy/dead-code cleanup, decomposition, generic optimization/CSE, proof,
call expansion, more decomposition, mem2reg, and later target lowering. It
places dead-code cleanup before transformations whose escape reasoning would
otherwise see dead address uses. `ssacompile/prove.go : 913–940,1231–1268`,
`factsTable.checkpoint`, `restore` and `prove`, maintains branch-scoped facts
and induction facts. Take explicit pass prerequisites and branch-local range
proofs. Do not copy integer rules into floating arithmetic: i still rounds each
Float operation and stops immediately on nonfinite results.

Go's Wasm implementation is not WasmGC. Its backend source describes a linear
memory Go stack, resumable block IDs, a cached SP, and a uniform `(i32)->i32`
control convention (`go/src/cmd/compile/internal/wasm/ssa.go : 30–124`, backend
calling-convention comment; `ssaGenValue` :220 onward emits the calls).
Arithmetic still lowers to raw target operations:
`go/src/cmd/compile/internal/ssa/_gen/Wasm.rules : 7–48`, arithmetic rules, and
`wasm/ssa.go : 426–468`, `ssaGenValueOnStack`, emit Float arithmetic/conversions.
Take raw scalar arithmetic and inspect resume overhead separately. Leave the
linear stack and uniform control ABI: i already has a WasmGC continuation
protocol. Go/Wasm timing would not isolate the cost of i Float boxes.

### 4.2 Comparison and ordered recommendations

**Already alike.** i has an ordered ANF, direct versus value calls, and a separate
control split (`W:ir.rs:1–19,249–255`; `W:lower.rs:494–504`,
`lower_with_bindings`). R3 already requires recomputing control facts after
eligible inlining.

**Different.** i emits reference arithmetic helper calls
(`W:emit/mod.rs:512` onward, `op`; `W:runtime/core.rs:11–24`). Go's scalar
arithmetic is not a reason to adopt Go's stack ABI. i's transitive may-suspend
exclusion and logical trace requirements remain binding (R3/R9, spec §11.7).

**Recommended, highest gain per effort first:**

1. Record a pass contract: solved facts → known-call selection and legal
   inlining → representation/escape repair → control split → verification →
   emission → Binaryen. Refresh invalidated facts explicitly. Reserve
   `W:lower.rs:494–504` and `W:control.rs` for the numeric implementation owner.
2. For N4, use branch-scoped integer interval/known-bit facts with exact generic
   fallback. Reuse the worklist method, not Go's fixed-width overflow semantics.
3. Postpone profile-guided inlining and broad escape analysis. First count
   surviving calls/boxes and use simple general benefits. No copied library
   exceptions, runtime name whitelist or new optimization annotation.

## 5. WasmGC and Binaryen: what the downstream optimizer can recover

### 5.1 wasm_of_ocaml

`js_of_ocaml/compiler/lib-wasm/gc_target.ml : 58–73`, `Type.float_type` and
`float_array_type`, use a one-f64 GC struct for a boxed Float and an f64 GC
array for flat storage. `Value.val_int`/`int_val` (:532–534) use i31 references;
`Memory.box_float`/`unbox_float` (:712–719) emit struct construction and field
read. `generate.ml : 450–477`, `convert`, explicitly bridges inferred scalar and
reference representations. That is a close WasmGC precedent for R1/R2.

The compiler performs its own cross-call work, rather than asking Binaryen to
discover every scalar signature. `compiler/lib-wasm/typing.ml : 27–41`,
`can_unbox_parameters` and `can_unbox_return_value`, require direct-only known
calls. `generate.ml : 2709,2985–3009`, `translate_function` signature construction, then selects
raw parameter/result types. The return helper explicitly discusses a limited
loss of tail calls. Take known-call analysis, but retain i's stricter R3 result
constraints: i cannot adopt that explanation as a blanket tail-space proof.

Its Binaryen configuration is also informative:
`compiler/lib-wasm/binaryen.ml : 65–94`, `optimizes`, `optimization_options` and
`optimize`, use O2/O3, skip `inlining-optimizing`, and pass
`--traps-never-happen`. The checked function does not add an explicit GUFA pass.
This is evidence for doing language-aware optimization before Binaryen, not a
recommendation to copy the switches. i's observable stop protocol forbids
assuming every trap away (spec §11.7; accepted R9).

### 5.2 Binaryen itself

`B : src/passes/Heap2Local.cpp : 18–110`, pass contract, identifies the exact useful
shape: allocation and all accesses inside one function, with exclusive uses.
`getParentChildInteraction` (:317 onward) treats returns as escapes and defaults
to escaping unless a use is recognized. Its `visitArrayGet`/`visitArraySet`
(:447–460) require constant indexes. A Float `struct.new`, stored in one local
and read by `struct.get`, is a good input. A box returned across a surviving
reference call or mixed with another object at a join is not the same case.

GUFA is flow-based refinement, not a replacement calling convention.
`B : src/passes/GUFA.cpp : 18–43,117–167,413–426`, pass description,
`GUFAOptimizer::visitExpression` and `GUFAPass::run`, use a content oracle to
replace known contents/unreachable expressions; the cast-all variant inserts
more precise casts. It can expose work for later cleanup. It does not make an
f64 a subtype of eqref. `SignatureRefining.cpp : 18–27,45–65`,
`SignatureRefining::run`, refines shared signature types and declines modules
with tables in this revision. `TypeRefining.cpp : 18–26,63–78`,
`FieldInfoScanner::noteExpression`, refines field subtypes from writes. Take
reference narrowing/devirtualization opportunities; do not claim these passes
unbox an arbitrary surviving eqref parameter into f64.

The pinned default pipeline already includes `heap2local`, cast optimization
and local subtyping at O2, with closed-world type/signature refinement
(`B : src/passes/pass.cpp : 682–724,776–803`, `PassRunner` default pass lists).
Local CSE's later gate is O3 or shrink >=1 (:725–728); GUFA is registered but
not added in this default sequence. i already selects closed world, observable
traps, speed level from its option and an always-inline limit of 20
(`crates/ilang-binaryen/src/lib.rs:40–47,106–112`, optimization setup).

### 5.3 Comparison and ordered recommendations

**Already alike.** i already uses i31ref, f64 payloads in GC objects, and
Binaryen's O2 GC cleanup (`W:runtime/bigint.rs:87–108`, `int_from_i64`;
`W:runtime/values.rs:52–59`, `float_new`/`float_get`). R3 has a more general
worker/wrapper scheme than the direct-only restriction read in wasm_of_ocaml.

**Different.** i's Float object currently uses the common multi-field `$obj`
constructor, and source Float literals allocate that object
(`W:emit/mod.rs:491–495`, `atom`). More importantly, ordinary functions and
arithmetic helpers return references. These are removable implementation costs,
not requirements of value-set types. Its finite checks and trace fidelity are
required differences from the reference's permissive optimizer flags.

**Recommended, highest gain per effort first:**

1. Keep N0's local construction/consumption experiment under the existing
   pipeline. Read actual optimized WAT, not only pre-Binaryen output. Do not
   spend weeks writing a duplicate local Heap2Local pass.
2. When calls survive, implement N2 scalar signatures in i. Distinguish local
   boxes eliminated by Binaryen from boxes avoided by the calling convention.
   Change `W:ir.rs:249–267,499–508`, `W:emit/mod.rs:284–295,443–457`.
3. Only after measuring survivors, compare `gufa-optimizing` plus cleanup and
   targeted CSE with the default pipeline. Preserve traps. Keep global heap
   layout changes deferred; a smaller Float box is less valuable than no box.

## 6. Correct rounding: CORE-MATH, Go, musl, and plain i

### 6.1 Start from the actual CORE-MATH algorithms

Do not describe every function as the same double-double algorithm. In this
snapshot sine has two fast reductions and two accurate paths:
`core-math/src/binary64/sin/sin.c : 883–951`, `cr_sin_moderate` and
`cr_sin_large`, reduce to a very small remainder, reconstruct from two tables,
evaluate short sine/cosine corrections, and test rounded endpoints
`fh + (fl - eps)` and `fh + (fl + eps)`. Agreement returns immediately;
disagreement calls the accurate path. `muldd` (:276–285) uses an FMA residual.
Take the small reduced domain, mixed precision, table reconstruction and an
explicit error radius. Leave its numeric radius unchanged only if the entire
operation model and its proof really are unchanged.

`sin.c : 229–262`, `reduce_large`, selects a short window of reciprocal-pi words
using the input exponent and multiplies it by the significand. It avoids
computing all the irrelevant integer bits of a huge quotient.
`reduce_large_acc` (:572–617) computes a finer remainder with an explicit
truncation bound. `sin_large_accurate` (:801–856) uses 128-bit fixed-point table
values and polynomial products, then `u128_tod` (:100 onward) rounds to double.
`sin_small_accurate` (:861–876) instead uses compensated Float arithmetic.
There is no general “keep doubling an arbitrary precision Taylor series” loop
in these functions. Take the bounded reduction and function-specific accurate
path; do not assume that 128 bits alone prove rounding for every function.

Cosine is not quite identical: `core-math/src/binary64/cos/cos.c : 725–830`,
`u128_tod`, `exceptions` and `cos_large_accurate`, include explicit difficult
inputs. Its `cr_cos_moderate` and `cr_cos_large` (:831–939) have endpoint tests.
Tangent's `reduce_fast` (`tan/tan.c : 1747` onward) documents the reduction error
term by term; `cr_tan` (:2246–2296) tests a fast enclosure then calls
`tan_accurate`. In the latter's final stage (:2200–2242), an integer error bound
is checked around the high bits, with two explicit exception inputs. Atan uses
an index approximation, an angle table and a short polynomial in `cr_atan`
(`atan/atan.c : 231–285`), then `as_atan_refine2` (:160–229) with compensated
operations and exceptional-input corrections. Take per-function proof and hard
case data. Leave the notion that one generic “accurate enough” pair format
settles every hard case.

### 6.2 What has and has not been proved here

The sources provide executable arithmetic, stated bounds, generators and
hard-case inputs. I read `sin/sin.sage` table generators `computeT`,
`computeS1u`, `computeS2u`, `computeC2u` (:2–11,74–98,141 onward), and
`sin/sin_fast.sollya`'s Remez/fpminimax/error-estimation script. Those are much
better starting points than handwritten constants. They are not a proof that
an independently rewritten i expression has the same error.

There is an important completeness limit: current `sin.c` cites `sin.pdf`,
`sinmoderate.sollya` and `sinsmall_acc.sollya` (notably :118,864,881,907), but
those files were not present in the checked-out tree. The available
`sin_fast.sollya` describes a different polynomial interval/degree from
`cr_sin_moderate`. I did not reconstruct the missing proof package or rerun
Sage, Sollya or an exhaustive hard-case search. The tan/atan exception tables
and `.wc` files were read as evidence, not independently certified complete.
`core-math/src/binary64/sin/check_special.c : 89–110`, `check`, compares result
bits with a reference under the selected rounding mode. Take that independent
bit comparison as a test pattern, not as a substitute for the bounds.
A production port needs a source revision with matching proof artifacts, plus
new bounds for each changed operation. Pin it and archive generator versions,
coefficient bits, domain partitions, proof outputs and tests together.

### 6.3 The non-correctly-rounded baselines

Go's `go/src/math/trig_reduce.go : 22–104`, `trigReduce` and `mPi4`, implement
Payne–Hanek reduction from a 1217-bit 4/pi table. The exponent selects a small
window; `bits.Mul64` and `bits.Add64` produce the fractional part and octant.
`go/src/math/sin.go : 189` onward, `Sin`/`sin`, dispatches to architecture code
when available; the pure-Go path uses split pi/4 reduction below its threshold
and fixed polynomial evaluation. No correct-rounding endpoint certificate or
accurate fallback is in that pure-Go path. Take the portable table-window
method, not its result as a rounding oracle. A comparison must force the
pure-Go path if that is the baseline intended.

musl's `musl/src/math/sin.c : 40–78`, `sin`, explicitly describes “nearly rounded”
results and dispatches reduced arguments to `__sin`/`__cos`.
`src/math/__rem_pio2_large.c : 13–66`, `__rem_pio2_large` algorithm, skips quotient
bits irrelevant modulo eight and uses 24-bit chunks held in doubles. This is
especially useful for a no-FMA i implementation: bounded chunk products can
be exact binary64 operations. It is a reduction technique, not a correct
rounding certificate for the final sine. Neither baseline justifies relaxing
Std-3 F1.

### 6.4 Differences forced by i

The current plain-i library already uses TwoSum, a split-product residual,
fast endpoint agreement, and outward integer intervals
(`F:std/src/float.i:1333–1371`, `certify`, `two_sum`, `two_product`;
:1478–1545, `fast_round`, `fast_reduce`, `fast_sincos`). It is already in the
same broad certified-fast/accurate-fallback family. The choice is not between
certification and CORE-MATH: CORE-MATH supplies better studied reductions,
polynomials and accurate-path engineering within that family.

The difference with the largest full-range consequence is explicit in the
code: `fixed_pi` recomputes Machin-series bounds (:1144–1167), `reduce_angle`
calls it at the requested working precision (:1223–1251), and `refine_math`
doubles precision after failed enclosure or certification (:1789–1802).
`fast_reduce` rejects angles above 2^20 (:1413,1492–1495). Those choices are
not required by ordinary libraries or principal inference. Immutable generated
tables and fixed-window reduction can be ordinary i data and functions.

Three differences *are* required by today's contract:

- CORE-MATH's explicit FMA residuals cannot be rewritten as ordinary `a *. b +.
  c`. i rounds the multiply first. Conversely, the optimizer cannot fuse i's
  separate operators. Use a proven split-product routine on a bounded/scaled
  domain and derive new bounds, or propose an explicit general FMA operation
  separately. This study recommends the no-FMA route first (R2, Std-3 F1).
- C unsigned arithmetic wraps, while i Int is unbounded. A 128-bit product
  truncation must be an explicit mask/remainder; high-half products and shifts
  must retain their stated bit meaning. `sin.c : 179–190`, `mhUU`, and
  `reduce_large` are algorithms to rewrite, not expressions to transliterate.
  Exact Int works initially; machine-limb optimization needs proven ranges.
- NaN/infinity/errno and directed rounding are not i's result model. Preserve
  signed zero and gradual underflow; prevent intermediate overflow. Domain or
  rounded-overflow errors follow Std-3 F1–F4. Library code cannot catch a core
  nonfinite stop and turn it into Error afterward (spec §§9.3, 11.3, 11.7).

Table access itself needs a cost plan. There is no indexed numeric array in the
current value paths read here: list heads and tuple/record fields are references
(`W:runtime/values.rs:62–73`). A linear list scan through a table can lose the
algorithmic gain. Compare ordinary balanced selection code, constant-field
records, and existing byte/bit storage access. Require indexed-table benchmarks
before proposing new general storage. A typed-array redesign is not a hidden
prerequisite for the first reduction experiment.

### 6.5 Reported i costs and realistic expectations

These are **reported**, not rerun: `F:docs/design/std-3.md:1390–1433` records
15.212 microseconds and 289 Float allocations for revised fast sin(0.5), among
313 total allocations. Its scalar controls (:1504–1537) give 131 ns without
redundant finite checks, 250 ns with checks, 130 ns native C for the same
algorithm, and about 10 ns native libm. Twelve input comparisons do not prove
a compiler transformation or all-input accuracy. The numeric note's separate
kernel measurements are not interchangeable with this harness.

The same report (:1480–1502) records huge-input median 1.608315 ms, hard-input
median 18.821662 ms, and a subnormal group containing an observed 453.732-ms
maximum. These mixed groups are not worst-case bounds; their presence rules
out claiming that unboxing the ordinary path alone releases the library.

**Inference:** scalar workers and pair elimination have a demonstrated
same-algorithm control in the low hundreds of nanoseconds. Use 250 ns as the
checked sin(0.5) control, not 10 ns as an implementation promise. Reaching
<=200 ns may need fewer checked operations or proven check removal. A smaller
CORE-MATH-style reduced polynomial may help, but split-product replacements,
table selection, i's traces and checks can consume the gain. There is no
measured no-FMA plain-i CORE-MATH port here and therefore no defensible precise
nanoseconds-per-call estimate for it. The right experimental target is first
below the 250-ns checked control, then the existing 20x gate, measured per
function and domain. Native FMA costs below cannot be assigned to Wasm i.

### 6.6 Native reference experiment

**Measured here.** A native experiment was built outside the reference tree,
in this lane's ignored `target/prior-art-numeric/`. An original timing harness
includes the unchanged CORE-MATH `sin.c` and links system libm. The include
allows a wrapper to call its static accurate path directly, without editing or
copying the reference. This does not build i or establish an i speedup.

Machine: x86-64 AMD EPYC-Genoa virtual CPU, 16 exposed CPUs; Clang 21.1.8;
libm from glibc 2.44-57 (checked with `ldd`). The final commands, from this lane:

```sh
direnv exec /home/agent/Programming/i clang -O3 -march=x86-64-v3 -fno-fast-math -ffp-contract=off -fno-builtin-sin target/prior-art-numeric/native.c -lm -o target/prior-art-numeric/native
/home/agent/Programming/i/.rd/cache/quiet direnv exec /home/agent/Programming/i bash -c 'target/prior-art-numeric/native > target/prior-art-numeric/native-final.txt'
```

An earlier `-march=native` attempt was explicitly ignored by the Nix wrapper;
the measured builds instead used `-march=x86-64-v3`. `objdump -d` confirmed
native FMA instructions and indirect calls in the final executable. Explicit
FMA remains enabled; `-ffp-contract=off` prevents incidental contraction of
separate operations. Rounding is FE_TONEAREST, without fast-math.

For each of five rounds, each input and each function receives 200 warmups and
10,001 paired single-call measurements. `CLOCK_MONOTONIC_RAW` brackets the
function call, a volatile result store, and the identity control separately;
target/control order alternates. A volatile function pointer prevents replacing
the timed call by a constant. Subtract signed identity duration per pair; sort
only for median/p99/maximum. Compilation, warmup, sorting and output are outside
the timed intervals. The quiet wrapper holds all three heavy slots; it cannot
control VM frequency or preemption. Only per-round summaries were saved, not
all raw samples. There was no GC in this native probe.

| Input | Function | Five round medians, ns | Median of medians, ns | Range of round p99, ns |
|---|---|---|---:|---:|
| 0.5 | cr_sin | 1, 1, 1, 1, 1 | below reliable timer floor | 11–11 |
| 0.5 | accurate path, forced | 29, 29, 29, 29, 29 | 29 | 32–41 |
| 0.5 | system sin | 10, 10, 10, 10, 10 | about 10 | 12–12 |
| 1e300 | cr_sin | 10, 10, 10, 10, 10 | about 10 | 20–20 |
| 1e300 | accurate path, forced | 29, 29, 29, 29, 29 | 29 | 32–41 |
| 1e300 | system sin | 89, 89, 89, 89, 89 | 89 | 108–118 |
| 0x1.54a9ad28f0a25p+225 | cr_sin | 10, 10, 10, 10, 10 | about 10 | 20–79 |
| same | accurate path, forced | 29, 29, 29, 29, 30 | 29 | 32–50 |
| same | system sin | 80, 80, 80, 80, 80 | 80 | 90–110 |

The last input is the regression input mentioned in `sin.c : 947`, not a claim
that the normal dispatcher takes its accurate path. Forced accurate calls
measure `sin_large_accurate` on inputs satisfying its domain; they do not measure
fallback probability. All three variants returned identical bits for each of
these three inputs. That is a consistency check against libm, not independent
correct-rounding validation. The largest final observed signed difference was
413,151 ns on forced accurate sin(1e300), showing why a median is not a bound.

The preliminary two-function probe (`native.txt`) was noisier: cr_sin(0.5)
round medians 20,10,20,10,9 ns, and cr_sin(1e300) 29,21,29,10,10 ns. It does not
support an exact ratio between CORE-MATH and libm. The final tiny-input 1-ns
number is also not a credible one-nanosecond latency claim: the observed timer
steps are roughly 10 ns, and subtraction magnifies alignment/overhead effects.
The useful conclusion is limited: this bounded accurate C path costs tens of
nanoseconds on selected inputs here, rather than requiring millisecond adaptive
bignum work. Translating that result to no-FMA i remains unmeasured.

Final harness SHA-256:
`306d8f0d2c9fd74360402a6d0eca24e445aceafe9fa38885620e6e47bebee454`.
Final executable:
`cb5b01df84ec1476e1b8be999bfb64dc40482ba0b15a36eb63a4ca688c0f2519`.
Final summaries:
`07232bfc9bc45166535ec1d604bf4d0c825a928ac1f32ff66c8dabe7603ab3aa`.

For reproducibility after the ignored directory is gone, this is the complete
original harness (save as `target/prior-art-numeric/native.c`); it contains no
copied reference implementation:

```c
#define _POSIX_C_SOURCE 200809L
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <fenv.h>
#include "/home/agent/Programming/i/.rd/ref/core-math/src/binary64/sin/sin.c"
__attribute__((noinline)) static double accurate(double x) { return sin_large_accurate(x); }
static volatile double sink;
__attribute__((noinline)) static double identity(double x) { return x; }
static uint64_t now(void) { struct timespec t; clock_gettime(CLOCK_MONOTONIC_RAW,&t); return (uint64_t)t.tv_sec*1000000000+t.tv_nsec; }
static double sample(double (*volatile f)(double), double x) { uint64_t a=now(); sink=f(x); return (double)(now()-a); }
static int cmp(const void *a,const void *b) { double x=*(const double*)a,y=*(const double*)b; return (x>y)-(x<y); }
int main(void) {
  fesetround(FE_TONEAREST);
  double xs[]={0.5,1e300,0x1.54a9ad28f0a25p225};
  double (*fs[])(double)={cr_sin,accurate,sin}; const char *names[]={"core","accurate-forced","libm"};
  for(int run=0;run<5;run++) for(int k=0;k<3;k++) for(int j=0;j<3;j++) {
    double d[10001]; for(int i=0;i<200;i++) sink=fs[j](xs[k]);
    for(int i=0;i<10001;i++) {double a,b; if(i&1){a=sample(fs[j],xs[k]);b=sample(identity,xs[k]);}else{b=sample(identity,xs[k]);a=sample(fs[j],xs[k]);}d[i]=a-b;}
    qsort(d,10001,sizeof(double),cmp);
    printf("%d %s %a %.0f %.0f %.0f %a\n",run,names[j],xs[k],d[5000],d[9900],d[10000],fs[j](xs[k]));
  }
}
```

### 6.7 Comparison and ordered recommendations

**Already alike.** The certified fast path plus rigorous fallback architecture,
bit-based interval agreement, and no-FMA compensation already exist in the
float branch (source citations in §6.4). Preserve those useful parts.

**Different.** Recomputing pi, a broad reduced interval and adaptive arbitrary
precision fallback are implementation choices. Finite intermediates,
nearest-even semantics, zero signs and ordinary library code are required
constraints. FMA/table-dependent proofs must be adapted to those constraints.

**Recommended, highest gain per effort first:**

1. Keep compiler N0–N3 first on its own track. In the float track, first replace
   repeated pi construction with generated, certified reciprocal-pi/pi bounds
   and a windowed reducer. Target `F:std/src/float.i:1144–1251` and Std-3 F7.
   Retain an exact outward error bound and refinement fallback. Do not promise
   every hard input is solved by a fixed table without its proof.
2. Next trial sin/cos table-assisted small-domain polynomials with i-specific
   no-FMA bounds, changing `fast_reduce`, `reduced_value`, polynomial helpers
   and F6. Regenerate constants rather than copying source tables. Keep the
   certificate and instrument fallback rate by input family.
3. Replace slow paths one function at a time with bounded accurate methods and
   fully justified hard-case handling. Prioritize full-range/subnormal failures,
   not only ordinary sine. Update F5/F7–F9 and independent vectors. Keep the
   current interval algorithm as an oracle/fallback until coverage is proved.
4. Treat explicit FMA as a separate coordinator question, not a prerequisite
   or silent compiler contraction. It could serve compensated arithmetic,
   geometry and dot products, but portable deterministic implementation and
   the four feature criteria need their own design.

## 7. Arbitrary precision: a small-value path is not one universal design

### 7.1 What the code does

GHC's numeric implementation moved in this snapshot. The `ghc-bignum` package
reexports `ghc-internal` (`ghc/libraries/ghc-bignum/ghc-bignum.cabal : 27–47`,
`library`). Read `ghc/libraries/ghc-internal/src/GHC/Internal/Bignum/Integer.hs`:
`Integer` (:172–187) is a small signed `Int#` payload or positive/negative
`BigNat#`; the invariant requires the big constructors only when the value
does not fit small. `integerAdd` (:554–580) checks `addIntC#` overflow;
`integerMul` (:583–612) uses a two-word product and falls back to big arithmetic.
Take exact promotion and normalization. The `IS` constructor is not evidence
that every escaping Haskell Integer is an immediate pointer-free value.

Zarith is a closer representation precedent. `zarith/caml_z.c : 120–131`,
representation contract, requires an OCaml immediate int when the number fits;
otherwise a signed limb block. `zarith/z.ml : 28–72`, `add`, `sub`, `mul`, use the
small path and overflow checks before C fallback. `caml_z.c : 405–423`,
`ml_z_reduce`, trims zero limbs and returns an immediate when possible. Take
canonical small/big transitions. Leave the OCaml external primitives and GMP
interface as implementation-specific details, not i library privileges.

CPython's compact path is **not** immediate tagging.
`cpython/Objects/longobject.c : 61–77,312–322`, `get_small_int`,
`maybe_small_long`, `_PyLong_FromSTwoDigits`, select cached objects, a compact
object, or a larger object. `long_add` (:3832–3863) and `long_mul`
(:4338–4352) perform native arithmetic on two compact inputs, then normalize
through those constructors. `Include/cpython/longintrepr.h : 11–44` describes
30- or 15-bit limbs and a double-width product type. Take a cheap arithmetic
branch and clear limb invariants; do not call Python's small cache an i31-like
representation or assume compact arithmetic always avoids allocation.

Go's `math/big` uses a third approach: `go/src/math/big/int.go : 20–37`, `Int`,
is a sign plus a magnitude slice, with pointer receiver/result conventions.
`Add` (:142–159) updates the receiver; `Mul`/`mul` (:184–205) use magnitude
multiplication or squaring. `nat.go : 56–73`, `make`/`setWord`, reuse capacity and
avoid over-allocation for one limb; `nat.add` (:97–124) handles zero operands,
adds limbs and normalizes. The `mul` comment specifically avoids returning a
pointer so temporary Ints do not escape. Take scratch-storage reuse only when
i can prove it unobservable. Public mutable big integers would conflict with
spec §11.1; reuse behind uniqueness/liveness proofs need not.

### 7.2 Comparison and ordered recommendations

**Already alike.** i already has the Zarith-style small/big distinction:
`W:runtime/bigint.rs:87–108`, `int_from_i64`, returns signed i31 for
[-2^30,2^30-1], otherwise limbs. `int_add`, `int_sub`, `int_mul`
(:220–268,304–315) test two i31 operands, calculate in i64 and repack. These
small operations do not allocate limb arrays when their result stays i31.
The accepted numeric note's integer profile correctly targets repeated packing
and control overhead, not nonexistent small-Int heap boxes.

**Different.** The 31-bit immediate boundary follows this WasmGC encoding;
it does not bound Int's value set. i uses exact unbounded arithmetic and a
partial checked `int_to_i64` (:110–132). A blanket i64 substitution is unsound.
Go's receiver mutation is also not a source-level model i can adopt. Neither
difference requires numeric type annotations or specialized std functions.

**Recommended, highest gain per effort first:**

1. Keep R4/N4 after the Float path. Carry proved/guarded small values through
   a whole loop in machine locals, and repack at escapes. Test overflow at
   i31 and i64 boundaries and never replay preceding effects on fallback.
   Change the future integer analysis and `W:emit/mod.rs:512` onward; retain
   `runtime/bigint.rs` as the exact fallback.
2. For math reduction, first use existing exact Int operations for fixed-width
   limb algorithms, with explicit masks. Then measure general small-limb
   multiply/shift helpers and common allocation sites. A 128-bit unsigned
   algorithm is not automatically covered by signed-i64 scalarization.
3. Consider scratch-buffer reuse only with a separate uniqueness proof and
   benchmarks. Do not start by changing the limb radix, adopting GMP, caching
   arbitrary integers, or adding public mutation. The source study supplies no
   measurement showing those changes beat the earlier work.

## 8. Consolidated order, files, and spec disposition

These are the ten most valuable proposals, in approximate gain per effort.
The order is a recommendation across two tracks; it does not remove their
existing ownership dependencies.

| Order | Proposal | Reference anchor | i change or gate |
|---|---|---|---|
| 1 | Expose temporary boxes locally before inventing a pass | Binaryen `Heap2Local.cpp`, `getParentChildInteraction` | N0; `W:emit/mod.rs:482–554`, `runtime/core.rs:11–24` |
| 2 | Scalar locals and known-call workers with generic wrappers | GHC `WorkWrap/Utils.hs`, `mkWwBodies`; wasm_of_ocaml `generate.ml`, signature construction | N1/N2; `ir.rs:113–120,499–508`, `emit/mod.rs:284–295` |
| 3 | Remove pair construction across helper calls | GHC `CprAnal.hs`, `cprTransformDataConWork` | N3/R5 inlining first; out-of-line result summaries remain a later proposal |
| 4 | Judge inlining by eliminated work and cap scalar arity | GHC `tryUnfolding`, `finaliseArgBoxities`; OCaml `Benefit.evaluate` | R3; worker selector and general diagnostics |
| 5 | Replace repeated high-precision pi evaluation with bounded table reduction | CORE-MATH `reduce_large_acc`; Go `trigReduce` | Std-3 F7; `F:std/src/float.i:1144–1251` |
| 6 | Port math proof/data generation with the algorithm | CORE-MATH `cr_sin_moderate`, `computeT`, fpminimax scripts | Std-3 F6–F9; regenerated tables, operation-specific bounds |
| 7 | Validate every representation edge, including saved locals | OCaml `transl_let`; Flambda2 `make_decisions`; native `scan_stack_frames` | R1/R7; checker facts, control saves and verifier |
| 8 | Optimize small Int regions, keeping exact promotion | GHC `integerAdd`/`integerMul`; Zarith `add`/`ml_z_reduce` | N4/R4; guards and existing big fallback |
| 9 | Follow simplification with cleanup and scoped facts | Go `passes`, `factsTable.restore` | Numeric pass ordering; `lower_with_bindings`, control split |
| 10 | Tune Binaryen only on residual measured work | GUFA `visitExpression`; wasm_of_ocaml `optimization_options` | N6; retain observable traps; no blind flag copying |

The narrow first implementation can use local expression/operator facts, but
cross-call workers need reliable solved facts. Today's `Typed` exposes scheme
trees, field access, type tests, and optional editor locals, not a complete
mandatory expression representation table
(`crates/ilang-check/src/types/mod.rs:347–402`, `Typed`). Coordinate the export
with checker owners; the accepted note's baseline line numbers have moved after
T1/T2. Do not parse display strings or silently treat editor data as always
available. This is a file-map correction, not a new inference algorithm.

No replacement normative spec text is proposed by this study:

| Spec section | Disposition |
|---|---|
| §4.1, §§4.7–4.8 | Keep value sets and principal inference; optimization failure uses a generic encoding |
| §6.2 | Keep evaluation order, including unused operands that stop |
| §§9.4–9.5 | Keep one-use continuations, environments and final behavior |
| §§11.1–11.5 | Keep immutable values, unbounded Int, finite binary64 and equality |
| §§11.6–11.8 | Keep bounded tail stack, logical stop positions and signed-zero text |
| §13 and Appendix B | No new primitive or ABI proposed; any explicit FMA proposal needs separate review |

Proposed amendments belong first in `numeric-lowering.md` R1/R3/R5 and §7
(argument budget, benefit accounting, bounded result facts, joins), and
`F:docs/design/std-3.md` F6–F9 (proof-backed reduction/table plan). The files
named above are change regions for their owners. This lane changes none of them.

## 9. Self-attack: concrete programs and repairs

These are source reasoning tests of recommendations, not tests of an
implemented optimizer. Snippets use accepted syntax. Expected results follow
the cited language/design rule; no machine timing is a correctness oracle.

### 9.1 First pass: real holes in an over-eager recommendation

**FMA substitution breaks the proof and changes results.** Consider:

```i
probe = () {
	a = 1.0000000074505806
	b = 0.9999999925494194
	{ a *. b } -. 1.0
}
```

Here a=1+2^-27 and b=1-2^-27. Their exact product is 1-2^-54, the midpoint
below 1, so the rounded product is 1 and this program returns +0. A fused
multiply-subtract returns -2^-54. Repair: explicit no-contraction rule, and no
reuse of FMA residual error bounds for a split-product rewrite (§6.4; R2).

**An unused component can still stop.**

```i
pair = (x : Float) { (0.0, x *. x) }
probe = () {
	(first, _) = pair(1.0e308)
	first
}
```

Expected: the Float nonfinite stop at multiplication, not 0.0 (spec §6.2,
§11.3/§11.7; R2/R5). Repair: remove allocation and projections, but retain the
checked computation in its original order. GHC's treatment of unevaluated
fields is not i's rule.

**“Unbox continuation parameters” can refer to two different things.**

```i
(
	i/core
	i/control
)
probe = () {
	control.boundary((b) {
		x = -.0.0
		y = control.suspend(b, (k) { control.resume(k, 2.0) })
		(core.float_to_bits(x), y)
	})
}
```

Expected: (9223372036854775808,2.0), preserving the saved zero bits (R7;
spec §9.4). Flambda2's internal join rewrite alone cannot implement this on i's
eqref frame protocol. Repair: distinguish IR joins from captured effects;
retain conservative N1 and typed save/restore gates (§3.2).

**A machine integer is not the whole Int type.**

```i
probe = (x : Int) { x + 1 }
```

At 1073741823, expect 1073741824; at 9223372036854775807, expect
9223372036854775808. Repair: keep exact promotion, explicit unsigned-width
masking for imported algorithms, and no blanket use of partial `int_to_i64`
(spec §11.2; R4; `W:runtime/bigint.rs:110–132`).

**Accepted R5 defers out-of-line multivalue results.** Repair: keep its
inlining-first N3; label bounded cross-call product-return summaries as a later
proposal, not an approved requirement (§2.2).

**Native FMA timings cannot predict a no-FMA Wasm port.** Repair: separate
reported i controls from the native experiment and give a test target, not a
claimed port latency (§6.5–6.6). Also distinguish the missing sine proof files
from the scripts actually present (§6.2).

### 9.2 Second pass: no further serious finding

Attack generic encoding and recursive result adaptation:

```i
id = (x) { x }
probe = () { (id(2.5), id("a")) }
relay = (f : (Float) -> Float, n : Int, x : Float) -> Float {
	{ n == 0 } |> (True) { f(x) } | (False) { relay(f, n - 1, x) }
}
```

Expected: (2.5,"a"), with one principal generic scheme; relay(id,1000000,0.5)
returns 0.5 in bounded tail stack (spec §§4.7,11.6; R1/R3). Workers do not
replace the generic entry, and unknown tail results force the compatible
reference convention before emission. The recommendations keep this gate.

Attack the claim that complete source scalar types already exist. Reading
`Typed` (:347–402) and `header` (:284–295) shows that both solved-fact export and
explicit worker signatures are needed. The file plan includes both. Attack
“just add GUFA”: its actual transformation and Heap2Local's escape default do
not justify a cross-call scalar ABI rewrite. §5 states the narrower claim.

Attack numeric library privilege by moving the pair, polynomial or fold into
an unrelated application module. Nothing recommended inspects its module name.
Unboxing helps audio, geometry and integration; pair elimination also helps
parsers and state machines; known callback specialization also helps codecs and
collection folds. No source type or new feature is required.

### 9.3 Third pass: no further serious finding

Attack endpoint equality near zero: i equates ±0, so numeric endpoint equality
alone is not enough to certify a signed underflow. Existing `fast_round` rejects
zero endpoints and `certify` compares bits (`F:std/src/float.i:1478–1485,1333–1340`).
The proposed port keeps those conditions and exact tiny-input shortcuts; test
sin(-0.0) and the least negative subnormal by bits, not only `==` (F1–F5).

Attack quotient-zero and stop motion:

```i
probe = (x : Float) { { x *. 2.0 } /. 0.0 }
```

At x=1.0e308 the multiplication stops; replacing the whole expression by zero
is wrong. At x=1.0, it returns positive zero (spec §§6.2,11.3; R2/R6).
The note preserves the finite check and leaves traps observable.

Attack bounded tables on maximum finite inputs: copying Go's one-double
remainder cannot certify a CORE-MATH result. §6 retains outward error bounds
and fallback and does not claim the hard-case corpus is exhaustive. Attack
128-bit arithmetic: signed i64 overflow is not unsigned truncation; §7 requires
exact Int/masks first and measured, proved limb lowering afterward. No new
serious hole found in the corrected proposals. These two clean passes are
self-review, not independent approval or a proof of a future implementation.

## 10. Coordinator questions and verification limits

1. **Should NUM keep its sequence?** Recommend yes: N0, conservative N1, N2,
   N3, then N4/N5 by measured survivors. Out-of-line product returns remain
   deferred. Add benefit accounting and an arity budget before product expansion;
   do not require a full demand solver, full CPR, speculative simplifier or
   new heap layout before first gains. Keep the transitive may-suspend ban.
2. **Should std/float follow CORE-MATH?** Recommend yes, method and verified
   data generation, one function at a time. First fix full-range reduction;
   retain the existing certified interval path until each replacement has a
   complete no-FMA proof. Acquire matching proof artifacts before adoption.
3. **Should FMA or typed numeric arrays be added now?** Recommend no. Neither
   is established as necessary by these measurements. Benchmark the ordinary-i
   no-FMA/table path first; revisit as separate general proposals if it cannot
   meet the gate. Do not weaken rounding or special-case a library.

Not verified: the proposed compiler changes, their cross-target output and
speed; a rewritten CORE-MATH library; complete hard-case searches; missing
sine proof artifacts; all mathematical functions beyond the sin/cos/tan/atan
source slices read here; Kotlin, Dart or Scala WasmGC backends; GHC/OCaml kernel
performance on this machine; full-range under-1-ms performance after any change.
Only one WasmGC language backend was studied, deliberately in depth.
No full repository test run is appropriate for this note-only change. The
native experiment is in §6.6. Source claims were checked against the pinned
files. All six i code fences parsed without diagnostics using the existing
`/home/agent/Programming/i/.rd/integ/target/debug/examples/parse-snippets`
(length-prefixed UTF-8 input, as in `scripts/old-syntax-check : 130–136`);
this checks syntax, not inference or execution. Final staged diff checks also
passed. The counterexample results above are reasoned from
the specification, not reported as executions of future lowering.
