# Numeric step 3: shadow-stack cost, regression and remaining allocations

No new language or library state surface.

Owner's rule kept: values, effects and complete stop traces are identical at
O0/O2/O3. The oracle is `cargo test -p ilang-cli --test numeric_lowering`
(the 455-program base plus boundary and loop cases, reference O0 against
visible O2 and scalar O2/O3), plus the
spec chapters with stop traces and suspension (`-- 11/ 9/`).

## Method

`tests/perf/numeric` with the paired driver
(`cargo run --release -p ilang-cli --example numeric_measure -- <dir>`).
`NUMERIC_BASELINE` is the previous step's optimized Wasm, so every pair runs
on the same Wasmtime build and timer, alternating order per batch. New in this
step: `NUMERIC_CANDIDATE` replaces the current compiler's module (to pair two
saved stages), `NUMERIC_KERNELS` selects kernels, and every run reports
**fuel**: Wasmtime's count of executed Wasm operators for one call, on the
instrumented module. `perf` is not installed on this machine, so fuel stands in
for `instructions:u`; like allocation counts it does not depend on load. Fuel
includes eight counter operators per allocation of the instrumentation.

Machine: 16 cores shared by about 15 workers, load average 23-40 during every
run below (`uptime` printed around each run). Times are five batch medians of
1001 paired samples each; they are not quiet-machine claims. Allocation
counts are raw (the entry/root wrapper has 11 nonnumeric allocations; the
`-11` column follows the step-2 report).

## 1. The N4 integer regression

There is one N4 commit (087ae08e); the bisection is between its parent N3
(1a3f30bb) and it, with the stage modules the step-2 author saved
(`target/numeric-step2/n3/n1.wasm`, `n4/n1.wasm` in lane jp), paired with
`NUMERIC_BASELINE`/`NUMERIC_CANDIDATE`, integer kernel only:

| Pair (baseline / candidate) | baseline ns | candidate ns | fuel |
|---|---|---|---|
| N3 / N4 as built | 1049-1071 | 1231-1261 | 14122 / 10799 |
| N4 / N4 with one tail-call site (guard fallback removed) | 1194-1212 | 1242-1261 | 10799 / 10031 |
| N3 / N4, both loops without the two shadow calls | 531-550 | 361-391 | 10005 / 6682 |
| N3 / N4, only `$shadow_leave_tail` written inline | 952-1582 | 832-1192 | 12970 / 9647 |
| N3 / N4, only `$shadow_enter` written inline | 811-831 | 661-681 | 12961 / 9638 |

N4's i64 worker executes 24% fewer Wasm operators than N3's eqref loop and is
still 18% slower, but only while each iteration makes both out-of-line calls
`$shadow_enter` (entry) and `$shadow_leave_tail` (before `return_call`).
Remove either and N4 is faster; remove both and it is 30% faster. The guard
and its second tail-call site are not the cause. So the regression is not in
the integer code: it is the interaction of the N4 loop with the two runtime
calls per iteration. Without `perf` the machine-level mechanism is not
measured; the likely one is that Wasmtime's tail calling convention has no
callee-saved registers, so every value live across those calls is spilled and
reloaded, and the N4 loop keeps more values live across them (two i64
scalars and two nullable big-Int fallbacks).

Fix (commit ab9bb55d): functions with numeric code write the shadow-stack
actions out instead of calling the runtime: `$shadow_enter_call` (push
`$tail`, grow in a cold call), `$shadow_leave`, `$shadow_leave_tail` with a
known site, and `$shadow_site` inside a frame (whose slot `sp - 1` always
exists). The stack holds the same values at every instruction. Reference
(O0) code, metered code and functions without numeric code keep the calls,
so programs without numeric code keep byte-identical WAT (§7 gate 2).

| Kernel | landing ns / allocs / fuel | step 1 ns / allocs / fuel |
|---|---|---|
| fast | 3995 / 39 / 9133 | 3396 / 39 / 8079 |
| horner | 50 / 2 / 1477 | 10 / 2 / 1419 |
| dot | 1413 / 2 / 6603 | 1273 / 2 / 5970 |
| integer | 1222 / 3 / 10831 | 631 / 3 / 9149 |
| tuples | 49 / 2 / 1481 | 29 / 2 / 1415 |
| closures | 1452 / 5 / 6485 | 1201 / 5 / 5626 |

Integer against N3 directly (paired): 1063 -> 621 ns. The regression is
gone: the N4 representation is now 42% faster than N3.

## 2. Lazy site stores and self tail loops

What mature implementations do: keep the hot path free of frame bookkeeping
and reconstruct frames only when they are observed (GHC and Go map return
addresses to frame info with pc tables; wasm_of_ocaml and OCaml record
nothing per call and rebuild backtraces at raise). Here, without access to
Wasm return addresses, the equivalent is to keep the site of the running
operation as compile-time state and write it only on a path where the stack
can be read. Two changes (commit below), both in numeric functions only:

- **Self tail calls become loops.** A function with a frame whose body tail
  calls itself (and has no point or join) gets `(loop $self_loop ...)` after
  its entry. The back edge evaluates every argument first, sets the
  parameters, resets every other local to its entry value (scalar caches use
  null as "not boxed yet"), stores the tail site in the frame's slot and sets
  `$tail` to -1: exactly what `$shadow_leave_tail` + `$shadow_enter_call`
  did to the stack, with no call and no frame change. The Int-guard fallback
  still leaves and tail-calls the ordinary entry. Binaryen can then inline
  the single-caller loop (fold's callback variant now inlines into the kernel
  and Heap2Local removes the closure object: closures 5 -> 4 allocations).
- **Site stores wait.** The emitter keeps the pending store per frame offset
  (`pending[0]` is slot `sp - 1`). A statement whose only way to stop is a
  Float check leaves its site store pending; the check's failure branch
  writes every pending store before `$stop_float`. Logical enter/leave of
  inlined calls shift the pending stores with the frame; a leave drops the
  callee's; a back edge overwrites its own. Anything else that could read
  the stack (any call, throw, point, `if`) writes them first, in the eager
  order. Block labels are joins: if the pending stores on the branches into a
  label and its fall-through differ, that label is re-emitted as a written
  join (the set of such labels only grows; emission repeats until stable).

Every reader of the shadow stack (`$stop`, `$shadow_top` for frame records,
`$shadow_snapshot` for host requests) runs after a call or a stop branch, so
it sees exactly the eager contents: stop traces stay byte-identical. The
corpus (455 programs with complete stop traces, including stops inside
inlined helpers one and three frames deep, tail calls out of inlined calls,
and stops after suspend/resume) and spec chapters 11 and 9 pass.

| Kernel | step 1 ns / allocs / fuel | step 2 ns / allocs / fuel |
|---|---|---|
| fast | 3486 / 39 / 8079 | 3364 / 39 / 6444 |
| horner | 21 / 2 / 1419 | 20 / 2 / 1335 |
| dot | 1290 / 2 / 5970 | 1140 / 2 / 5073 |
| integer | 621 / 3 / 9149 | 361 / 3 / 6973 |
| tuples | 30 / 2 / 1415 | 20 / 2 / 1373 |
| closures | 1211 / 5 / 5626 | 770 / 4 / 4585 |

The hot loops now make no shadow-stack call; dot, fold and integer pay one
inline store per iteration (the tail site on the back edge).

## 3. Unboxed Float results at the Int/Float boundary; R5 not done

`core.to_float` of an i31, or of a scalar small Int, is now an
`f64.convert` and a Float-representation definition (a worker can keep it
in f64 or return it as its f64 result); other Ints still call the runtime,
which stops when the result is not finite. While looking at the integer
kernel's sites, its third allocation turned out to be the box made only to
pass `x` to `core.truncate`; a scalar Float below 2^30 in magnitude now
truncates straight to the i31. The corpus gained both edges of each fast
path, the runtime paths and the `to_float` stop.

| Kernel | step 2 ns / allocs / fuel | step 3 ns / allocs / fuel |
|---|---|---|
| integer | 369 / 3 / 6973 | 350 / 2 / 6931 |
| others | unchanged | unchanged (same fuel and allocations) |

Time gain about 5%; the allocation target for integer (<=2) is met.

The second half of this step, out-of-line multivalue returns for helpers
that return records/pairs (R5), is **not done**. The fast-sine allocation
sites show where it would matter: of 39 allocations, 12 come from one
out-of-line helper returning a `Double(hi, lo)` record three times (record,
values array and two Float boxes each) and 8 from another returning two
records. Removing them needs a worker result convention of two f64 values,
a call statement with several destinations in the IR (every walker in
control.rs, numeric/*.rs and the emitter), and a materialization rule at
escaping uses. That did not fit in this step's time with ~20-minute build
queues, and it was not started, to keep every commit working.

## 4. Fold closure allocations (R7)

After step 2 the fold callback variant is a loop with one caller, so
Binaryen inlines it into the kernel and its Heap2Local pass removes the
closure object (5 -> 4). The capture array and the box of the captured `x`
stayed: each was stored into the allocation before it, so it escaped when
Heap2Local ran, and the default pipeline runs that pass only once after
inlining. Binaryen's escape analysis is the R7 analysis needed here
("capture arrays that do not escape the call"); it just needs another
round per nesting level. `ilang-binaryen` now runs `heap2local,
simplify-locals, heap2local, simplify-locals, vacuum` after the default
O2/O3 pipeline. Heap2Local only rewrites allocations whose every use it
sees; closure identity, portability walks and environment keeping all use
the reference, so a closure that any of them can observe still escapes and
stays. No change in the compiler's own IR.

| Kernel | step 3 ns / allocs / fuel | step 4 ns / allocs / fuel |
|---|---|---|
| closures | 771 / 4 / 4585 | 742 / 2 / 4433 |
| others | unchanged | unchanged (same fuel and allocations) |

Closures now allocate only the wrapper's input and result Float boxes
(target <=4 met). Time gain about 4% (fuel 3%), below 5%; this step was not
taken further.

## 5. List tag tests

A list pattern lowered to `Tag(v, 6)` and then cell tests. The tag test is
`$value_tag`, which Binaryen inlines as a null test, an i31 test and a
`ref.cast` to `$val`, the common supertype, before reading the tag. On a
`$cons` cell that cast is not an exact-type match, and in Wasmtime it is by
far the most expensive part of the loop: a hand edit of the step-2 dot module
that only deleted the four tag tests per iteration took dot from 1132 to
260 ns. Only lists are `$cons` cells, so a pattern with at least one item
needs no tag test: its first cell test proves the tag. `[]` now tests for a
cell first (which fails at once on every cell of a recursion) and the tag
only after that, for a value that is not a cell. Semantics are unchanged for
every value, list or not; this is in the shared lowering, so O0 changes the
same way.

| Kernel | step 4 ns / allocs / fuel | step 5 ns / allocs / fuel |
|---|---|---|
| dot | 1142 / 2 / 5073 | 271 / 2 / 3217 |
| closures | 701 / 2 / 4433 | 260 / 2 / 3505 |
| integer | 341 / 2 / 6931 | 329 / 2 / 6931 |
| others | unchanged | unchanged (same fuel and allocations) |

## Final table

Paired run of the landing module (main before this step) against the final
module, same driver, load average 40 (09:13 HKT). ns is the median of five
batch medians; allocations subtract the 11 wrapper allocations as in the
step-2 report; fuel is executed Wasm operators per call.

| Kernel | Target ns / allocs | Before (landing) | After (final) | Met? |
|---|---|---|---|---|
| fast sine | <=500 / <=8 | 4116 / 39 / 9133 fuel | 3305 / 39 / 6444 fuel | **no** (both) |
| Horner | <=40 / <=2 | 49 / 2 / 1477 | 10 / 2 / 1335 | yes (timer resolution is 10 ns) |
| dot | <=300 / <=2 | 1432 / 2 / 6603 | 261 / 2 / 3217 | yes |
| integer | <=500 / <=2, no Int objects | 1243 / 3 / 10831 | 330 / 2 / 6931 | yes |
| tuples | <=40 / <=2, no tuple objects | 40 / 2 / 1481 | 21 / 2 / 1373 | yes |
| closures (fold) | <=400 / <=4 | 1433 / 5 / 6485 | 260 / 2 / 3505 | yes |

The N4 integer regression is explained and fixed (section 1): the
integer kernel is now 3.2x faster than N3's.

## What is still unmet, and why

- **Fast sine: 3305 ns and 39 allocations against 500 ns and 8.** Its cost
  is allocation: of the 50 raw allocations, 16 are in the certified path's
  main body, 20 in two out-of-line helpers that return `Double(hi, lo)`
  records (each record is an object, a values array and two Float boxes),
  and the rest in the wrapper and small helpers. A hand edit giving
  `$value_tag` an exact-`$obj` fast path (the cast that made list loops
  slow) gained only 2% here, so it was not made. These helpers are above the inlining bound, and returning
  two f64 values out of line is R5's deferred multivalue convention, which
  this step did not implement (section 3). Steps 1-2 removed 29% of its
  executed operators (9133 -> 6444) and 20% of its time; the next gain
  needs that result convention, not more frame work.
- Timings are from a loaded machine (load average 23-55); fuel and
  allocation counts are exact. The 10 ns timer step makes Horner and tuples
  targets met with little margin to spare in absolute terms.

## Interpretations

- "Stop when a step's gain is below 5%": read per step, as "do not tune that
  step further". Step 3 (5% time, -1 allocation in integer) and step 4 (4%
  time, -2 allocations in closures) are below or at that line in time and
  were not taken further; step 5 was measured independently (it gives 4x on
  dot) and was done because its cost was already known from a hand edit.
- Frame work is limited to functions with numeric code (and O2/O3), so
  §7 gate 2 (no numeric code -> byte-identical WAT) and O0 as the untouched
  oracle both hold. The list-pattern change (section 5) is in the shared
  lowering and changes O0 too; it changes no result.
- The extra Heap2Local rounds (section 4) apply to every O2/O3 module, not
  only numeric ones: a Binaryen pass is semantics-preserving by construction,
  and doing it per function would need a second pipeline.
- `perf` is not installed; fuel is reported instead of `instructions:u`.

## Verification

The previous worker recorded after each step:
`cargo test -p ilang-cli --locked --test numeric_lowering`
(the 455-program corpus plus the cases added here), `cargo test -p
ilang-wasm --locked`, and `cargo test -p ilang-cli --locked --test spec --
11/ 9/` (50 of 50 at --opt 0 and --opt 2). For step 4 the corpus ran
together with step 5 (the first step-4 run failed to compile a test case
added in step 3, fixed in that commit); ilang-wasm and spec passed for
step 4 alone. The reboot recovery reruns the required checks below; the
previous report's unfinished `STD_RESULT` placeholder did not establish a
passing std run at either optimization level.

## Reboot recovery

The remaining uncommitted change raised the numeric inline limit from 32
to 96 statements for helpers detected as returning a tuple or record. It
had no recorded measurements or dedicated regression test, and cloned
each callee body to decide eligibility. It was read and removed from the
landing candidate, with its patch saved in the ignored
`target/num3-deferred-inline.patch`. The existing 32-statement limit and
growth budgets are retained. Numeric-lowering R5 says "Expose cross-call
pairs first by R3 inlining. Defer out-of-line multivalue aggregate workers
and typed heap layouts"; R3 starts with "at most 32 source IR statements
and 2x per-caller growth", and coordinator decision 9.3 makes budgets
internal heuristics whose fallback never changes source acceptance or
principal inference. Recovery retains the established heuristic without
claiming that the discarded experiment passed or failed a performance gate.

The six-kernel table above is now also in `docs/performance.md`. These are
the earlier worker's measurements; the reboot recovery does not claim a
new quiet baseline/candidate comparison. The historical before/after Wasm
artifacts are absent from this worktree's `target/`, so that paired run
cannot be repeated directly. The fast-sine targets and the quiet timing,
std check/build cost and Wasm-size acceptance gates remain unverified or
unmet as described there. No source acceptance, runtime meaning, reported
error or test expectation was changed during recovery.

Recovery checks run sequentially in this worktree through
`/home/agent/Programming/i/.rd/cache/heavy`. Cargo and measurement commands
also used `direnv exec /home/agent/Programming/i` after that prefix.

| Command after the prefix | Result |
|---|---|
| `cargo test -p ilang-wasm --locked` | 70 tests passed; doc tests passed (0 tests) |
| `cargo test -p ilang-cli --test numeric_lowering --locked` | 4 tests passed, including the differential corpus and exact resumed-stop fixture |
| `cargo test -p ilang-cli --test spec --locked -- 11/ 9/` | 50/50 cases passed at O0 and O2 |
| `cargo clippy -p ilang-wasm -p ilang-binaryen -p ilang-cli --all-targets --locked -- -D warnings` | passed |
| `cargo fmt --all -- --check` | passed |
| `cargo build --release -p ilang-cli --bin i --example numeric_measure --locked` | passed |
| `./target/release/examples/numeric_measure target/num3-measure-current` | passed; expected results checked, allocations and fuel equal the recorded final module |
| `./target/release/i test --project std --opt 0` (heavy prefix only) | 2170 passed, 0 failed |

All commands exited 0. `git diff --check` passed, as did
`git diff --exit-code -- crates lib std examples tests`: recovery leaves
compiler, library, examples and test files unchanged from `cc73cf90`.

The fresh five batch medians and counts are in `docs/performance.md`;
raw samples and modules are under `target/num3-measure-current/`. Verification
logs are `target/num3-*-verification.log`.

The full workspace test run, unfiltered spec suite, review corpus and
`scripts/dev-verify` were left to the coordinator as requested. Recovery
did not rerun std O2, JS/WASI probes, the independent rounding corpus or
byte comparisons against the pre-lane compiler. The representation-mode
byte-equality test passed, but it is not a substitute for those historical
comparisons. No claim is made that all numeric-lowering acceptance gates
are complete.

