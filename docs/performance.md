# Performance

How fast the compiler and the programs it makes are, how to measure it, and
the choices made from the measurements.

## Measuring

`scripts/bench` builds the release `i` and runs the benchmarks in `bench/`
at `--opt 2`. Each benchmark is a test of the `bench` package, which uses
`std`, and checks its result. `i test --time` prints, for each test, the
time it took and the processor time of the process (user and system), both
without the build:

```sh
scripts/bench          # all of them
scripts/bench sort     # the ones whose name has `sort`
```

| Benchmark | What it does |
|---|---|
| `list_build` | `list.range(1, 1_000_000)` and `list.length` |
| `list_map` | `list.map` of `+ 1` over 1M Ints |
| `list_reverse` | `list.reverse` five times over 1M Ints |
| `list_sort` | `list.sort(by: int.compare)` of 1M Ints that look random |
| `list_merge` | merging two sorted lists of 1M Ints, written out |
| `closures` | 1M calls of 100 closures that capture a value |
| `inspect` | `core.inspect` of a list of 200k tuples with lists |
| `dict_insert` | 200k `dict.insert`s |
| `equality` | 20 `==` of two lists of 100k tuples |
| `resume_depth_10` | 100k resumes of a computation paused under 10 frames |
| `resume_depth_100` | 100k resumes of a computation paused under 100 frames |
| `resume_depth_1000` | 100k resumes of a computation paused under 1000 frames |

The environment variables `ILANG_GC` (`copying`, `drc`, `null`) and
`ILANG_GC_HEAP` (MiB) choose the garbage collector and the initial size of
the GC heap (see `crates/ilang-host/src/lib.rs`).

The numbers below are from one Linux x86-64 machine with 16 cores, shared
with other work, so they vary by about 10%. Where the machine was busy,
processor time is given.

## Results

Release build, `--opt 2`, seconds. "Before" is commit `f7b3964`; "after"
has every change below.

| Benchmark | Before | After |
|---|---|---|
| `list_build` | 0.54 | 0.15 |
| `list_map` | 0.97 | 0.24 |
| `list_reverse` | 1.90 | 0.40 |
| `list_sort` | 78.4 | 3.90 |
| `list_merge` | 5.70 | 1.13 |
| `closures` | 0.15 | 0.09 |
| `inspect` | 0.84 | 0.14 |
| `dict_insert` | 4.47 | 0.87 |
| `equality` | 1.43 | 0.39 |

| Task (release) | Before | After |
|---|---|---|
| `i check --project stdlib` | 20.4 s | 0.13 s |
| `i test --project stdlib --opt 2` (1376 tests, with the build) | 190 s | 34 s |
| `i test --project stdlib --opt 0` | | 39 s |
| type check of the suspend-point chain below, n=800 | 1.6 s | 0.8 s |

## Instruction counts and peak memory

Wall-clock time moves with the load of the machine. `scripts/bench-instructions` measures by
user-space instruction count (`perf stat -e instructions:u`; where perf is not usable it falls
back to user CPU time, and each result line says which metric it has) and by peak resident
memory (`time -v`, the largest process of the tree). It writes one JSON line per benchmark to
`target/bench-instructions/results.jsonl` and a markdown table beside it. It is run by hand and
is not a CI gate.

| Benchmark | What it runs |
|---|---|
| `check_std` | `i check --project stdlib` (checks the tests too) |
| `build_std_tests_o0`, `build_std_tests_o2` | `i test --project stdlib --opt N` with a filter that matches no test: the build of the whole test program, and no test runs (`i test` has no build-only mode) |
| `corpus_wide_256`, `corpus_wide_1024`, `corpus_wide_generic_1024` | `i check --types` of the wide-union cases `tests/review/cases/review-k4/perf/*` |
| `kernel_*` | the kernels of `tests/perf/numeric` (`identity`, `fast`, `horner`, `integer`, `dot`, `tuples`, `closures`), a million calls each, in `tests/perf/numeric-loop`; `kernel_base` is the loop alone, to subtract |
| `bench_closures`, `bench_resume_depth_1000` | tests of `bench/` at `--opt 2`, build included |

### Recorded measurements

Numbers from the optimization work of 2026-11-13 that were not written down anywhere else.
They are ratios or sizes taken on the shared 16-core machine, not instruction counts; the
script gives instruction counts for the same things from now on.

| Measurement | Result |
|---|---|
| Library `catch` against the built-in `fail.catch`, with an error caught | 1.9 to 2.1 times as slow |
| The same, with no error | 2.9 to 3.1 times as slow |
| Float to text after the runtime moved into i (`lib/i/float_text.i`), typical values | about 3 times as slow as before |
| The same, largest float (max `Float`) | 2.0 times |
| The same, `5e-324` (smallest subnormal) | 5.9 times |
| `NUM` integer kernel (`integer` in `tests/perf/numeric`) at step N4 | about 15% slower |
| `i test --project stdlib` peak memory | 8.5 GB at `--opt 0`, 17 GB at `--opt 2` |
| `i check --project stdlib` peak memory | 0.3 GB |

The memory of `i test --project stdlib` is the backend (the Wasm module, Binaryen and Wasmtime's
code generation), not inference, which is why `i check --project stdlib` stays small.

## Syntax-three front-end cost

The review recorded these measurements for
`tests/review/cases/review-t1/stress-keys-2000`, three runs per configuration,
with `RAYON_NUM_THREADS=1` and `BINARYEN_CORES=1`. These are the supplied
historical measurements, retained as reported; they are not new timings.

| `i test` level | `9b806113` wall / peak RSS | `5e08939f0` wall / peak RSS | Increase |
|---|---|---|---|
| `--opt 0` | 0.39 s / 54 MB | 0.45 s / 69 MB | 0.06 s / 15 MB |
| `--opt 2` | 0.82 s / 128 MB | 0.87 s / 145 MB | 0.05 s / 17 MB |

Repeat the timing command on the stored project directly, through the
shared heavy wrapper. The corpus driver adds a `qa_startup` test and an
`i` dependency; that is a different workload. Do not use its prepared
project for comparison with this table, or run the corpus driver for the
performance measurement.

On 2026-11-14, a release build of `9b806113` and this lane immediately
before/after the allocation changes gave the following medians. Each row
uses three timed runs after one warmup, the same stored project, `--offline`,
and the two thread limits above. RSS is GNU time's KiB; heap sizes below
are heaptrack's decimal MB. Shared-machine scheduling affected wall times,
so these runs establish the memory change, not a wall-time speedup.

| Tree | O0 wall / CPU / RSS | O2 wall / CPU / RSS |
|---|---|---|
| `9b806113` rebuilt | 0.41 s / 0.39 s / 54,212 KiB | 0.87 s / 0.84 s / 128,152 KiB |
| DOC1–5, before allocation changes | 0.95 s / 0.52 s / 66,592 KiB | 1.30 s / 0.99 s / 148,400 KiB |
| Borrow/move trees and release parsed items after resolution | 0.49 s / 0.45 s / 62,180 KiB | 0.90 s / 0.88 s / 143,916 KiB |

Heaptrack 1.5.0 (`--record-only`, O0) counted 3,585,139 allocations before
the changes and 3,394,655 afterward: 190,484 fewer. Full backtraces
(`heaptrack_print --shorten-templates 0 --merge-backtraces 0`) identify
`resolve` cloning every parsed module, followed by `facts::groups::module`
cloning each already-owned binding and item. The resolver now borrows the
input modules and group normalization moves its owned bindings. These
changes alone lowered median RSS to 62,348 KiB, but did not lower the
31.70 MB peak live heap. Releasing parsed item trees after resolution,
before test-root inference, lowered peak live heap to 24.50 MB. Imports,
scope metadata and the typed expanders' own source snapshots remain live.
The rebuilt old compiler's peak live heap was 28.13 MB.

The residual RSS difference is largely freed memory retained by glibc,
measured rather than inferred from RSS. A profiling-only exit hook using
`mallinfo2`, `/proc/self/status` and `malloc_trim(0)` measured:

| Tree | Live allocator bytes | Free bytes in arenas | RSS before / after trim |
|---|---:|---:|---:|
| Rebuilt `9b806113` | 810,656 | 30,654,816 | 53,792 / 27,060 KiB |
| After these changes | 812,816 | 37,566,704 | 61,120 / 28,552 KiB |

Thus 6,911,888 extra bytes were free inside retained arenas at exit, while
live allocations differed by only 2,160 bytes. The production compiler
does not call `malloc_trim`; the portable fix is to avoid redundant tree
allocations and shorten their lifetimes. Stage tracing independently put
the pre-fix O0 peak at 59 MiB during checking versus 45 MiB in the old
compiler; the final checking peak was 53 MiB. Stage tracing resets the
kernel peak counter, so its runs are separate from the GNU time samples.

Profiles and individual samples from this investigation are in the lane's
ignored `target/docfix-profile/` directory (`raw-before-test.zst`,
`raw-final-test.zst`, `raw-old-test.zst`, and `raw-*-{0,2}-{1,2,3}.json`).
The old source was archived inside that directory and its binary built
there with the same Binaryen source revision and release profile. An
initial corpus-prepared workload and cold first runs were excluded from
the warm, stored-project comparison above.

## Numeric lowering

The six kernels in `tests/perf/numeric` isolate arithmetic, known calls,
short-lived aggregates and list walks. The step-3 paired run compared the
landing module before these changes with the final module using the same
Wasmtime build, alternating order over five batches of 1001 samples. These
are the previous worker's recorded measurements, not a new timing run after
the reboot. The machine was shared (load average 40 in the final run), and
the timer resolves about 10 ns. Times are medians of the five batch medians.
Allocations subtract the 11 nonnumeric entry/root allocations but include
the input and result Float boxes.

| Kernel | Before ns / allocations | After ns / allocations | Target ns / allocations |
|---|---|---|---|
| fast sine | 4116 / 39 | 3305 / 39 | <=500 / <=8 |
| Horner | 49 / 2 | 10 / 2 | <=40 / <=2 |
| dot | 1432 / 2 | 261 / 2 | <=300 / <=2 |
| integer | 1243 / 3 | 330 / 2 | <=500 / <=2, no Int objects |
| tuples | 40 / 2 | 21 / 2 | <=40 / <=2, no tuple/array objects |
| closures (fold) | 1433 / 5 | 260 / 2 | <=400 / <=4 |

The recovery run at 12:34 HKT on 2026-11-13 rebuilt the current compiler
and repeated the driver without the missing historical baseline modules.
Its scalar O2 results follow. All allocation and fuel counts match the
recorded final module exactly; the optimized scalar module is 160585 bytes.
This is a fresh candidate measurement, not a quiet historical comparison.
The driver also checks the results against its fixed expected values.

| Kernel | Five batch medians, ns | Median ns | Allocations | Fuel |
|---|---|---:|---:|---:|
| fast sine | 3337, 3334, 3455, 3364, 3245 | 3337 | 39 | 6444 |
| Horner | 19, 2, 20, 10, 11 | 11 | 2 | 1335 |
| dot | 261, 260, 270, 262, 262 | 262 | 2 | 3217 |
| integer | 322, 330, 321, 321, 321 | 321 | 2 | 6931 |
| tuples | 20, 20, 10, 20, 22 | 20 | 2 | 1373 |
| closures (fold) | 260, 260, 261, 261, 260 | 260 | 2 | 3505 |

The ignored `target/num3-measure-current/` contains `samples.csv`,
`medians.csv`, `allocation-sites.json`, and the optimized and instrumented
modules. Run the driver through the heavy wrapper:

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo run --release -p ilang-cli --example numeric_measure --locked -- target/num3-measure-current
```

The changes write shadow-stack actions inline in numeric functions, turn
self tail calls into loops, delay source-site stores until the stack can be
read, keep small `to_float`/`truncate` conversions unboxed, run two extra
Heap2Local rounds, and test list cells before their common value tag.
The integer regression from N4 is removed; the list test change accounts
for most of the dot and fold gains. Values, effects and complete logical
stop traces must still agree at O0/O2/O3.

Fast sine still misses both targets: two helpers return allocated records
above the inline limit. Out-of-line multivalue aggregate workers remain
deferred by numeric-lowering R5. The other five kernels are within their
time and allocation thresholds in this run; a quiet paired comparison is
still needed for the performance acceptance gate, especially for the short
Horner and tuple kernels. These measurements do not establish the i/float
release gates or a full-range worst case. Raw batch medians from a new
quiet run, std check/build cost and benchmark Wasm size acceptance also
remain to be checked. The per-step measurements, fuel counts and surviving
allocations were recorded in a step report that is not kept in the tree.

### Numeric follow-up: list-cell cast reuse

The num4 starting revision, `fd99f9e32`, already includes the step-3 list tag
change, scalar `to_float`/`truncate` paths and two extra Heap2Local rounds.
A fresh paired run on 2026-11-13 at 18:19 HKT tested the remaining suggestion:
retain a successful cell cast and reuse it for later patterns and projections.
It used the saved starting revision's scalar O2 module as `NUMERIC_BASELINE`,
and the candidate in the same driver process, Wasmtime build and timer.
Load average after the run was 18. Times are the medians of five batch
medians of 1001 samples; allocation counts subtract the 11 wrapper objects.

| Kernel | Before ns | Cast cache ns | Before / cache allocations | Before / cache fuel |
|---|---:|---:|---|---|
| fast sine | 3364 | 3345 | 39 / 39 | 6515 / 6515 |
| Horner | 19 | 21 | 2 / 2 | 1346 / 1346 |
| dot | 281 | 281 | 2 / 2 | 3228 / 4062 |
| integer | 340 | 341 | 2 / 2 | 6942 / 6942 |
| tuples | 10 | 20 | 2 / 2 | 1384 / 1384 |
| closures (fold) | 380 | 290 | 2 / 2 | 3516 / 3934 |

The candidate used `br_on_cast_fail` to keep a nullable typed cell reference
for each receiver with one definition per self-tail iteration. The first
successful test supplied subsequent tests and both projections; null checks
retained the original casts on untested paths. Every back edge reset the
caches, and functions with suspension points were excluded. This follows
numeric-lowering R8's same-receiver-version and proved-shape requirements.

The experiment was removed: dot gained no time and executed 26% more fuel;
fold executed 12% more fuel. Fold's batch medians were
`[380, 392, 399, 280, 260]` before and `[421, 391, 251, 241, 290]` with the
cache, so its median difference does not establish a stable gain. Extra cache
branches and maintenance offset the avoided exact-cell checks. Horner and
tuples have identical fuel and allocations, and their differences are near
the timer's 10 ns resolution. The optimized module grew from 236330 to
236727 bytes. The candidate passed the wasm tests; it was dropped before
running the differential corpus. The ignored
`target/num4-deferred-cell-cache.patch` preserves the experiment, and
`target/num4-before/` and `target/num4-paired/` hold modules, raw samples,
batch medians and allocation sites.

The retained change adds 13 differential programs for list loops: failed
literal patterns, exact and short lists, a non-list union member, unequal
list lengths, and overflow stops after later matches. Compiler and optimizer
sources remain at the starting revision. The conversion and Heap2Local
changes already present need no further rounds for these kernel targets.

The saved 96-statement product-inlining idea was also measured, with a
read-only eligibility scan in place of the saved patch's body clone. It
allowed a larger helper only when its returns directly named small products,
and retained the suspension, recursion, caller-growth and global-growth
limits. The scalar optimized module was byte-identical to the baseline
(236330 bytes), with unchanged fuel and allocations in every kernel, so this
heuristic change was also removed. Its patch and paired artifacts remain in
`target/num4-deferred-inline.patch` and `target/num4-inline-paired/`.

The final retained-tree pair at 18:31 HKT checked all six expected results
again. Its scalar module is byte-identical to the starting module (236330
bytes; SHA-256 `55f56af8f56aa75a25de875effce3d90c23eef837710dd2cbe670c05d23569fc`).
The following timing differences between identical modules illustrate the
loaded-machine and timer limits; the retained change claims no speedup.
Raw samples and all five batch medians are in `target/num4-final/`.

| Kernel | Before ns | Retained ns | Allocations, both | Fuel, both |
|---|---:|---:|---:|---:|
| fast sine | 3365 | 3345 | 39 | 6515 |
| Horner | 20 | 21 | 2 | 1346 |
| dot | 270 | 271 | 2 | 3228 |
| integer | 381 | 371 | 2 | 6942 |
| tuples | 21 | 21 | 2 | 1384 |
| closures (fold) | 260 | 260 | 2 | 3516 |

Targets still not met:

- Fast sine: 3345 ns and 39 allocations against <=500 ns and <=8 allocations.
  Its out-of-line record-returning helpers still need the work deferred by R5:
  “Defer out-of-line multivalue aggregate workers and typed heap layouts.”

The other five kernels meet their thresholds in the retained-tree pair. A quiet
timing acceptance run, std check/build cost, and benchmark Wasm-size acceptance
remain unchecked; this loaded-machine run does not close those gates.

## Garbage collection

Wasmtime 50 has three collectors:

- **Copying** (`gc-copying`): a Cheney semi-space collector. Compiled code
  allocates inline with a bump pointer, and there are no read or write
  barriers. A collection copies the live objects into the other half of
  the heap; it collects cycles. Half of the heap is always unused.
- **Deferred reference counting** (`gc-drc`): every allocation is a call
  into the runtime, every load and store of a reference has a barrier that
  counts references, and cycles are never freed.
- **Null** (`gc-null`): bump allocation that never frees. The program traps
  when the heap (at most 4 GiB) is full, so it is only for measuring and
  for very short runs.

`Config::collector` picks one; `Config::gc_heap_initial_size` sets the size
that every store's GC heap starts with (by default 0, growing on demand).
The heap is only reserved: memory is used as objects are made.

Processor time in seconds, all with the same compiler (before the changes
to the generated code), only the collector and its initial heap changing:

| Collector | build | map | reverse | sort | merge | inspect | dict | equality | peak RSS |
|---|---|---|---|---|---|---|---|---|---|
| null | 0.18 | 0.28 | 0.41 | 5.75 | 1.23 | 0.15 | 0.77 | 0.32 | 1.9 GB |
| DRC | 0.34 | 0.58 | 1.07 | 59.2 | 3.52 | 0.51 | 4.53 | 1.06 | 203 MB |
| DRC, 256 MiB | 0.20 | 0.41 | 1.24 | 59.6 | 3.40 | 0.21 | 4.30 | 0.48 | 426 MB |
| copying, 0 | 0.20 | 1.23 | 2.72 | 9.51 | 6.66 | 0.49 | 2.91 | 1.00 | 183 MB |
| copying, 64 MiB | 0.18 | 1.16 | 2.61 | 9.65 | 6.33 | 0.31 | 0.94 | 0.90 | 183 MB |
| copying, 128 MiB | 0.17 | 0.36 | 0.47 | 6.25 | 5.42 | 0.17 | 0.84 | 0.64 | 182 MB |
| copying, 256 MiB | 0.18 | 0.31 | 0.51 | 6.10 | 1.36 | 0.16 | 0.85 | 0.43 | 313 MB |
| copying, 1 GiB | 0.16 | 0.28 | 0.44 | 5.56 | 1.31 | 0.15 | 1.00 | 0.41 | 1.1 GB |

DRC is slow where much is allocated and kept (`list.sort`: ten times the
copying collector), and it leaks cycles. The copying collector is as fast as
the null one, but only with a large enough heap. The reason is how Wasmtime
50 chooses between collecting and growing: it grows the heap only when the
live data after a collection fill more than half of it, and with the
copying collector half of the heap is all that can be live. So it grows
only when a half is full of live data; while the live data is a little
under that, each collection frees little and the next one comes soon. With
the merge of two 1M lists, the time goes from 1.4 s to 6.7 s.

**Choice**: the copying collector with a 256 MiB initial heap (two halves
of 128 MiB), the smallest size that did not thrash on any benchmark. A
program whose live data stays just under the size of a half (and then
under each larger size the heap doubles to) still collects often; give it
a larger `ILANG_GC_HEAP`. The null collector is never used, as it never
frees.

## Generated code and runtime

- **List cells.** A list cell was an `$obj` (tag, two numbers, a float,
  four references) with a `$values` array of the head and the tail: two
  objects, about 90 bytes. It is now a `$cons` struct of the tag, the head
  and the tail, about 32 bytes. `$obj` and `$cons` are subtypes of `$val`,
  whose field 0 is the tag, and `[]` is still an `$obj`.
- **Constructors without fields** (`None`, `Less`, ...) allocated a new
  `$obj` at each use. Each is now made once, in a global; values are
  compared by their parts, so this cannot be seen.
- **Comparison and `==`** allocated a frame for the remaining siblings each
  time they went into a part, also for list heads and small Ints. Two
  small Ints are now compared at once, list heads that are small Ints need
  no frame, and `==` of two small Ints is decided at once.
- **Inlining.** At `-O2`, Binaryen inlines only functions of 2 expressions
  or with one caller, and Wasmtime inlines nothing, so every `$list_head`,
  `$value_tag` and shadow stack function stayed a call, with spills of the
  GC references around it. `ilang-binaryen` now always inlines functions of
  up to 20 expressions:

  | Always inline up to | sort | merge | reverse | build of std's tests at `--opt 2` |
  |---|---|---|---|---|
  | 2 (Binaryen's default) | 5.17 | 1.34 | 0.49 | 2.5 s |
  | 15 | 3.87 | 1.14 | 0.41 | 3.6 s |
  | 20 | 3.94 | 1.16 | 0.40 | 3.5 s |
  | 40 | 3.26 | 0.96 | 0.33 | 5.3 s |

  Wasmtime's own inliner (`Config::compiler_inlining`) gave about the same
  speed, but `i test --project stdlib` then peaked at 3.7 GB of memory, and it
  would change `--opt 0` code as well.

What is left in the hot loops is mostly the shadow stack (a call on entry
and before a tail call, for the positions of stop messages) and the tag
checks of patterns, which test `$value_tag` again for each case.

## Resuming a continuation

A paused computation keeps one record for each of its frames, innermost
first. `control.resume` rebuilds only the innermost frame. The others are
rebuilt one at a time, when the frames inside them return. When the resumed
code suspends again, only the frames it entered since the resume are
captured, and the records still waiting are joined on in one step. So a
resume and the next suspend cost the same at any depth of the paused stack.

Some frames still cost something at each resume:

- a frame whose call is under an environment statement (`! S = v`) of its
  function: its environment is worked out again;
- a boundary that the computation is paused inside, and a cancel whose
  final function suspended: they are re-entered at once, and a later
  suspend through them packs the frames inside them.

`fail.catch` and `resource.ensure` cost nothing at a resume. An error or a
cancel goes through the waiting frames one at a time, as an unwind does. So
`control.cancel` costs time in proportion to the number of waiting frames,
as it did before.

`bench/test/control_test.i` uses frames without environment statements or
inner boundaries. At `--opt 2`, the median of five runs at depth 1000,
and at depth 100, must be at most 1.5 times the median at depth 10
(`docs/design/lazy-reentry.md`, section 10). The coordinator measures this
in a quiet window. In a lane, repeat this command five times at the eager
baseline `0e7c450`, then at the lazy implementation `56196f6`:

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i ./scripts/bench control
```

One sample of each compiler with other jobs running gave the following
wall and process CPU times: the release compiler at `1df2fe0` on the eager
baseline sources, then the release compiler at `56196f6`. These are
substitute measurements, not the five-run acceptance measurement.

| Depth | Eager wall seconds | Eager CPU seconds | Lazy wall seconds | Lazy CPU seconds |
|---|---|---|---|---|
| 10 | 0.489 | 0.45 | 0.146 | 0.15 |
| 100 | 3.420 | 2.92 | 0.162 | 0.15 |
| 1000 | 27.304 | 26.10 | 0.157 | 0.15 |

CI uses the exact fuel check of section 13.2, test 6 instead. It compiles
the benchmark as a WASI command once for each depth and round count,
precompiles it with fuel enabled, then bisects the fuel needed to finish
to a bracket narrower than 0.05%. The extra fuel per round is the
difference between 4000 and 1000 rounds, divided by 3000. Both deeper
values must be at most 1.05 times the depth-10 value. With the dev-shell
Wasmtime 49.0.2 CLI on Linux x86-64, lazy gave:

| Depth | Fuel upper bound, 1000 rounds | Fuel upper bound, 4000 rounds | Extra fuel per round | Ratio to depth 10 |
|---|---|---|---|---|
| 10 | 707840 | 2817024 | 703.0613 | 1.000000 |
| 100 | 739328 | 2848768 | 703.1467 | 1.000122 |
| 1000 | 1057792 | 3167232 | 703.1467 | 1.000122 |

Run these checks with the slot/dev-shell prefix above and
`cargo test -p ilang-wasm --test control_lazy --locked -- --nocapture`.
At the eager baseline, set `CALIBRATE_EAGER=1` and filter that command
to `eager_stack_calibration` to repeat the stack calibration.

The stack checks use a child shell with a 64 MiB native stack and a Wasm
stack at most half that size. A signal fails the check; only a clean
Wasm stack trap counts as reaching the stack limit. On the same CLI:

- The first eager captures at depths 10000 and 100000 finish with a
  32 MiB Wasm stack; the depth-10000 fallback also fits in 4 MiB.
  The lazy deep test completes 1000 rounds at depth 100000 with that stack (section 13.2, test 7).
- With a fixed 128 KiB Wasm stack, the largest complete nested generator
  walk is depth 219 on eager and 262 on lazy. The test requires at least
  80% of the eager depth (test 10). The tree has a recursive left subtree
  and a right leaf, so it checks all values in order without an exponential
  number of nodes.
- The same callback-based 50000-deep recursion is used with and without
  the resume. On eager, recursion alone needs between 5600085 and 5601109
  bytes. Test 11 fixes S11 at 8401663 bytes, 1.5 times the upper bound
  rounded down.
  Eager recursion plus a resume of 50000 paused frames gives a clean
  Wasm stack trap there; lazy finishes and counts all 100000 frames.
  Using the same recursive function matters: calibrating a one-argument
  recursion and then testing a two-argument recursion measures different
  native frame layouts.

The eager baseline keeps exactly six lazy-only tests ignored, as section
14 requires. Its other control tests pass. The 19 new spec cases pass on
eager at O0 and O2; their checked headers and values are the eager
baseline for lazy. Run tests 12 to 16, plus the error through a final
record, compare the entire eager output, stop text and exit code at O0
in `control_lazy.rs` and at O2 in `ilang-cli/tests/wasi.rs`. D15 keeps
unspecified cancel behaviour out of the spec suite. D16 preserves the
known environment-before-tail-call gap; its expected name is `resumer`.

The section 11.4 WAT audit compares the 598 exports of the eager baseline
and lazy: 380 compiled modules are byte-identical after the allowed shared
runtime patch, record checks and masking; 218 ERROR exports are identical.
It checks the saved values, shapes and positions of 5438 code records.
The O2 long-pending program also keeps its recursive calls as non-tail
calls: the recursive deep function calls itself with `call`, and the
resumed generator segment calls its generator with `call` before returning
to `keep`. This checks the growing pending list required by section 13.3.

No export is added or removed between those two commits. The generated
segments, joins, host runtime, sites, imports, exports and custom sections
have no other differences.

## Type checker

A function with many suspend points and no annotations, of this shape
(`s` suspends):

```i
a1 = { a0 % 3 } |> (0) { s(b, a0 + 1) } & (_) { a0 + 2 }
a2 = { a1 % 3 } |> (0) { s(b, a1 + 1) } & (_) { a1 + 2 }
// ...
```

Each call of `s` gives new type variables for the arguments of the boundary
`b`, which become equal to those of the first call: the boundary's
variables form one strongly connected group that grows with every binding,
and every binding's new variables reach it.

| n | before, debug | after, debug | before, release | after, release |
|---|---|---|---|---|
| 200 | 1.0 s | 0.4 s | 0.09 s | 0.05 s |
| 400 | 3.4 s | 1.6 s | 0.35 s | 0.20 s |
| 800 | 12.6 s | 6.1 s | 1.61 s | 0.80 s |
| 1600 | | | 8.3 s | 3.2 s |

- **Occurs check** (`solve.rs`). It searched the bound graph once for each
  root (the variables made or given bounds by the binding), each search
  going over the whole group. It now finds the strongly connected
  components of the graph reached from the roots once, and answers all
  roots with one pass over the components, with bit sets of the targets.
  When a root has a cycle, the old search runs for that root alone, so
  the variables marked as reported, and so every message, are the same.
- **Coalescing** (`display.rs`), which simplifies the type of every
  generalized binding, computed the cycles of variables of the whole
  program each time, and found the members of a cycle and the variables
  being expanded by scanning lists. It now looks only at the variables
  that the type reaches, with maps. This is what made `i check --project
  std` take 20 s.
- The solver and coalescing use FxHash (`rustc-hash`) for their maps and
  sets of small keys.

What is left is still quadratic in n: `add_bound` follows the bounds that
are variables to every variable of the group for each new bound (`reach`),
and the occurs check still goes over the group once per binding. Both would
need the variables of a cycle to be merged into one, which changes the
solver throughout.

## Debug builds

`[profile.dev]` builds the dependencies (Wasmtime, Cranelift) at
`opt-level = 2` and the workspace crates at 0. `i test --project stdlib` in a
debug build took about 50 minutes; most of that was the type checker (20 s
for `std` even in a release build) and the DRC collector, both fixed above.
It now takes under a minute.

Workspace crates at `opt-level = 1` (with `CARGO_PROFILE_DEV_OPT_LEVEL=1`,
dependencies still at 2) against 0, debug builds, wall time:

| | 0 | 1 |
|---|---|---|
| `i test --project stdlib --opt 0` (1376 tests) | 51.6 s | 51.2 s |
| the `list_test` module of it | 39.7 s | 39.9 s |
| `cargo test -p ilang-cli --test spec`, built | 11.3 s | 11.8 s |
| `i check --project stdlib` | 0.80 s | 0.19 s |
| type check of the suspend-point chain, n=800 | 6.1 s | 1.0 s |
| rebuild of `i` after an edit of `ilang-check/src/types/solve.rs` | 1.7 s | 4.6 s |
| ... of `ilang-wasm/src/emit.rs` | 1.3 s | 2.7 s |
| ... of `ilang-cli/src/main.rs` | 1.0 s | 1.4 s |
| `cargo test -p ilang-check` after an edit of `solve.rs` | 1.7 s | 4.2 s |
| first build after the switch | | 113 s (Binaryen's C++ is built again) |

(An edit is a line put in at the top of the file, which moves every span
after it, and taken out again.)

**Choice**: keep `opt-level = 0` for the workspace crates. The tests now
spend their time in the Wasm code of the tests, which Cranelift compiles
the same way in both, so they gain nothing; the type checker is 4 to 6
times faster at 1, but takes under a second on `std` either way, while
every rebuild after an edit takes up to 2.7 times longer. For work on the
type checker with large inputs, `CARGO_PROFILE_DEV_OPT_LEVEL=1` (or a
release build) gives the fast checker without changing the default.

## Standard-library test memory (2026-11-13)

The `lane/o2mem` baseline is `d17def11` with the stage trace from
`073255fb` and `df32d679`, built in release mode through the shared heavy
wrapper (four Cargo jobs, no LTO, 16 codegen units). The wrapper sets
`RAYON_NUM_THREADS=1`. No compile cache was used. The measurement was:

```sh
H=/home/agent/Programming/i/.rd/cache/heavy
I_TRACE_STAGES=1 ILANG_COMPILE_CACHE=0 $H /run/current-system/sw/bin/time -v ./target/release/i test --project stdlib --opt 2
```

All **2170 tests passed**, with no failures. On the shared machine the run
took 10:11.41 wall time (379.32 s user, 56.17 s system). The trace's `MB`
columns are actually MiB (1024 squared bytes):

| Stage | Wall seconds | CPU seconds | RSS at end, MiB | Stage peak, MiB |
|---|---:|---:|---:|---:|
| Load | 0.029 | 0.000 | 12 | 12 |
| Expand | 12.541 | 11.280 | 250 | 250 |
| Resolve | 5.698 | 5.550 | 250 | 250 |
| Check | 0.835 | 0.820 | 306 | 356 |
| Lower | 2.400 | 2.360 | 307 | 318 |
| Emit WAT | 0.402 | 0.390 | 362 | 399 |
| Parse WAT | 1.562 | 1.530 | 505 | 557 |
| Binaryen | 142.801 | 70.110 | 652 | 652 |
| Wasmtime compile | 243.282 | 159.900 | 19414 | 19766 |
| Run tests | 198.610 | 180.370 | 19414 | 19844 |

The large increase is in **Wasmtime compilation**, from roughly 0.6 GiB
to 19.3 GiB. The test run inherits that resident memory and adds only
about 0.1 GiB to the compilation peak. Binaryen's module is already
optimized in place and disposed before Wasmtime starts. Reclaiming all
of its measured resident memory would still leave the peak well above
the 12 GB target.

The external `time -v` reports 19,881,856 kB maximum RSS. Do not treat that
as an independent process-wide high-water measurement when tracing:
the trace resets Linux `VmHWM` at stage boundaries through
`/proc/self/clear_refs`. The maximum of the stage peaks is 19,844 MiB;
an untraced `time -v` run is needed for an independent process-wide peak.

Wasmtime 50 already enables `memory_init_cow` by default. Binaryen uses
its own thread pool, independent of Rayon's thread count. Smaller Wasm
byte buffers cannot account for the observed increase. If allocator and
Wasmtime configuration changes do not suffice, the next investigation
needs to identify which Wasmtime compilation data stays live across
functions, or reduce the size of the single compilation unit. Any split
must preserve all tests, their provider plans and fresh instances: spec
§10.2 requires that the runner compile the collection once and run each
selected test in a fresh started instance. Filtering out tests is not a
memory fix.

### Buffer-lifetime experiment

A separately built variant transferred ownership of the original Wasm bytes
into optimization, released those bytes after removing names and before
Binaryen, and released Binaryen's input buffer immediately after its reader
returned. The WAT text was explicitly dropped after parsing; the baseline
already transferred that string to the parser, so this made its existing
lifetime explicit. The same full traced command was run, without a filter:

| Variant | Binaryen peak, MiB | Wasmtime compile peak, MiB | Run peak, MiB | Wall time | Result |
|---|---:|---:|---:|---|---|
| Baseline | 652 | 19766 | 19844 | 10:11.41 | 2170 passed, 0 failed |
| Early buffer drops | 664 | 19730 | 19851 | 7:26.56 | 2170 passed, 0 failed |

Every test-result line and the final summary were identical. `i build
--target host` of `examples/hello` and `examples/echo`, at both `--opt 0`
and `--opt 2`, produced byte-identical Wasm before and after (`cmp` checked
all four files). The buffer drops did not lower the peak; the timing
variation on this shared machine is not evidence of a speed improvement.
The experiment is not retained as a compiler change.

The baseline and this experiment both exceed the 12 GB target. These
measurements do not establish whether allocator trimming or other Wasmtime
settings can meet it. A source inspection of Wasmtime 50 shows that it
calls `compiler.release_caches()` after assembling the compilation artifact;
trimming only after that point could reduce memory retained during tests,
but cannot recover a peak that has already occurred during compilation.

### Remaining measurements

At the end of the first investigation, no 12 GB fix had been validated. A Linux/glibc variant with
`malloc_trim(0)` after Binaryen and after Wasmtime compilation built
successfully, but its timed std run waited 8 minutes 37 seconds for the
shared wrapper's memory slot and was cancelled before the executable
started, to leave time for checking before the lane deadline.
There is no RSS or test result for that variant.

A further test-module-only variant was prepared with
`parallel_compilation(false)`, `generate_address_map(false)` and
`native_unwind_info(false)` (the last outside Windows). It was not built
or measured. The host's stop messages are assembled by the guest under
spec §11.7, but the requested native and spec stop-message checks still
have to validate any retained configuration change. Both variants,
including a temporary module-dump diagnostic, were discarded by reversing
the reviewed experiment patch. Only the optional stage trace and these
measurements are retained. Allocator trimming and the Wasmtime settings
remain open experiments, rather than demonstrated failures or fixes.

## Macro expansion and compiled-code cache

Building the macro registry once per expansion and reusing it for typed
macros reduced release `i check --project stdlib` from 26.2 to 18.1 CPU seconds
on the lane machine. The expanded std output was byte-identical.

The global cache retains whole compiled modules as well as individual
Cranelift functions, subject to the same size and age limits. Set
`I_CACHE_MODULE_MAX_SIZE=0` to measure function-only reuse. Its compiled-code
keys include a content hash of the executable, computed once per process.
Macro compilation itself is not cached across processes: a safe key would need
all source and macro inputs, compiler and host ABI identities, dependencies,
configuration, and every observable input to macro execution.

Release `i test --project stdlib --opt 0`, one run per cell, on the shared lane
machine, with separate initially empty private stores and one Rayon thread:

| Cache entries | Run | CPU (user + system) | Peak RSS | Wall time |
|---|---|---:|---:|---:|
| Modules and functions | cold | 353.25 s | 10.99 GiB | 631.98 s |
| Modules and functions | warm | 250.04 s | 1.00 GiB | 381.58 s |
| Functions only | cold | 345.72 s | 10.99 GiB | 437.46 s |
| Functions only | warm | 321.32 s | 11.22 GiB | 379.85 s |

All four passed 2,170 tests with byte-identical stdout and empty stderr. Wall
times include shared-machine contention and are not directly comparable.

A maintenance measurement used a private 5 GiB sparse fixture with 5,121
entries and a warm filesystem metadata cache. After a write, the cap check
walked the entries in 13.9 ms; a due collection took 17.7 ms. These are single
wall-time observations, not guarantees for other filesystems or cache sizes.

## Large Int arithmetic in i

Beyond the native fast paths, Int comparison, addition, subtraction,
multiplication, division and remainder are implemented in `lib/i/big.i`.
Operands are read by index with
`size` and `limb`, in base 2^30. Each result is written into fresh private
construction storage; that storage receives no writes after `finish`.
Arithmetic does not copy operands into lists. Text conversion still uses
digit lists.

Multiplication uses Karatsuba on ranges of the original operands when both
ranges exceed 16 digits and their lengths differ by at most a factor of two.
Small and unbalanced ranges use product scanning. Division normalizes each
operand once, estimates a quotient digit and fuses its product with remainder
subtraction. These algorithms, their loops and carry propagation are i code.
The only arithmetic intrinsics multiply-add one digit with carry (returning
its low and high parts separately), or divide two digits by one.

Measurements on 2026-11-13, Linux x86-64, on the shared 16-core host, at O2.
The i arithmetic is commit `5a2420e6`.
The baseline is the provided WAT executable at
`.rd/jp/target/release/i` (SHA-256
`e4da9d35637d98d37b7056bb4ac9615b86a12de0257933a93c6b0552702caefa`).
The baseline branch was `lane/jp`, with head `93291533`; the executable was
used directly, without rebuilding or copying it. Both modules run in the
same current Wasmtime host. Program compilation and Rust operand construction
are outside the measured interval.

Times are the median of five alternating batches, in microseconds per
operation, using Linux process CPU time
(`clock_gettime(CLOCK_PROCESS_CPUTIME_ID)`), including the guest worker thread.
Batch sizes are calibrated toward 50 ms of CPU time, capped at 1,048,576
iterations. The CSV also records wall time; scheduling on the shared host made wall-time ratios noisy. Loop and
amortized host-call overhead are included. Every batch's final result is
checked against `num-bigint`.

For bit width `n`, the dividend/first operand is `a = 2^n - 129`. The other
operand is `a - 1`, so comparison walks the whole number and ordinary division
has a one-digit quotient. `divide_half` instead uses `2^(n/2) + 123` as the
divisor, to exercise a long quotient.

| Bits | Operation | WAT µs | i µs | i / WAT |
|---|---|---:|---:|---:|
| 100 | `add` | 0.207 | 0.977 | 4.72× |
| 100 | `compare` | 0.067 | 0.355 | 5.31× |
| 100 | `multiply` | 0.236 | 6.391 | 27.09× |
| 100 | `divide` | 2.919 | 3.770 | 1.29× |
| 100 | `divide_half` | 2.701 | 4.370 | 1.62× |
| 200 | `add` | 0.208 | 1.333 | 6.39× |
| 200 | `compare` | 0.069 | 0.463 | 6.75× |
| 200 | `multiply` | 0.401 | 14.363 | 35.81× |
| 200 | `divide` | 6.177 | 4.903 | 0.79× |
| 200 | `divide_half` | 6.361 | 9.534 | 1.50× |
| 300 | `add` | 0.240 | 1.936 | 8.06× |
| 300 | `compare` | 0.078 | 0.626 | 8.06× |
| 300 | `multiply` | 0.585 | 26.142 | 44.72× |
| 300 | `divide` | 12.524 | 4.181 | 0.33× |
| 300 | `divide_half` | 9.559 | 14.314 | 1.50× |
| 8500 | `add` | 1.198 | 38.639 | 32.26× |
| 8500 | `compare` | 0.699 | 12.728 | 18.20× |
| 8500 | `multiply` | 230.717 | 9115.392 | 39.51× |
| 8500 | `divide` | 4971.423 | 117.747 | 0.024× |
| 8500 | `divide_half` | 3622.384 | 2916.085 | 0.81× |

The [recorded CSV](../tests/performance/big-int/results-2026-11-13.csv)
also preserves batch sizes and wall times.

All algorithms remain in i even where these measurements show a regression
against WAT. There is no large-Int arithmetic fallback to WAT.

Reproduce from this worktree, using the shared wrapper:

```sh
H=/home/agent/Programming/i/.rd/cache/heavy
BIG_INT_BASE=/home/agent/Programming/i/.rd/jp/target/release/i \
BIG_INT_ARTIFACTS="$PWD/target/big-int-perf" \
"$H" direnv exec /home/agent/Programming/i \
  cargo test -p ilang-cli --test big_int_performance --locked -- --ignored --nocapture
```

The fixture is `tests/performance/big-int`; the ignored Rust integration test
builds it with both compilers, retains both Wasm artifacts when
`BIG_INT_ARTIFACTS` is set, and prints the measurements as CSV.

### Metadata initialization diagnosis

The second investigation first measured the preserved `b2/std-O0.wasm`
probe artifact (6,475,874 bytes), independently of the i frontend. The
`ilang-host` examples `module_stats` and `compile_probe` retain the
measurement tools. The function names below are those in the Wasm name
section; `fNNN` names identify generated functions, not source bindings.

```text
types: 206, GC array/struct types: 37, tables: 0, element segments: 1
code bytes: 3490626
  123145 bytes      0 locals f39983: ilang_start
   16608 bytes   1120 locals f14437: f14008
    4652 bytes    174 locals f12134: f11705
    4474 bytes    158 locals f15713: f15284
    4426 bytes    229 locals f39982: start
    4352 bytes    146 locals f15718: f15289
    3925 bytes    131 locals f12407: f11978
    3813 bytes    192 locals f14387: f13958
    3789 bytes    147 locals f13058: f12629
    3715 bytes    127 locals f12374: f11945
    3629 bytes    267 locals f14398: f13969
    3350 bytes    112 locals f15703: f15274
    3166 bytes    128 locals f11509: f11080
    3165 bytes    129 locals f15846: f15417
    3082 bytes     92 locals f15660: f15231
    2982 bytes     82 locals f12150: f11721
    2954 bytes    172 locals f11511: f11082
    2896 bytes    104 locals f15777: f15348
    2783 bytes    103 locals f14328: f13899
    2678 bytes    109 locals f14621: f14192
    2621 bytes     86 locals f13104: f12675
    2604 bytes    117 locals f11459: f11030
    2531 bytes    160 locals f12486: f12057
    2512 bytes    142 locals f14517: f14088
    2465 bytes     56 locals f11969: f11540
    2462 bytes     91 locals f13097: f12668
    2441 bytes     56 locals f13145: f12716
    2411 bytes     78 locals f11495: f11066
    2407 bytes     55 locals f13117: f12688
    2407 bytes    111 locals f14649: f14220
functions: 29 imported, 42154 defined
declared in element segments: 26897
exported: 2202
named by ref.func: 19196
escaping defined functions (trampolines now): 29099
escaping if only ref.func targets were declared: 21398
```

The 42,154 defined functions contain 3,490,626 bytes of code (83 bytes
on average). Only two bodies exceed 10,000 bytes. The largest is the
exported `ilang_start`, which installs runtime metadata; it is distinct
from `start`, which executes source initializers. In particular, expanded
programs have a source-note string for every site. One giant
`array.new_fixed` evaluates all allocating string expressions before
constructing its array. Every earlier GC reference stays live across the
later allocations, making the compilation disproportionately expensive.

Both compile-only measurements used `RAYON_NUM_THREADS=1`, no compile
cache, and the shared heavy wrapper with `/run/current-system/sw/bin/time
-v`. For the second measurement only the body of function 39983
(`ilang_start`) was replaced with an empty body; the module was compiled,
never executed. All remaining functions, types and element declarations
were retained.

| Compile-only input | Peak RSS, KiB | User CPU, s | System CPU, s | Wall, s |
|---|---:|---:|---:|---:|
| Original O0 module | 11,037,164 | 80.75 | 9.02 | 91.02 |
| Only `ilang_start` stubbed | 600,112 | 18.18 | 0.66 | 19.11 |

This identifies generated metadata initialization as the dominant cost;
partitioning the test collection is unnecessary. Build large metadata
arrays by storing each value immediately, in the original order, with
64 stores per generated helper. User initializers still run in their
original order after metadata installation. The collection, provider
plans and fresh started instance per test stay the same (§10.2), and the
source positions and expansion notes stay at the same indices (§11.7).
Metered compiler-macro modules retain the original emission so their
instruction and allocation accounting does not change.

### Full std measurements with bounded metadata

On merged main `95b1b500`, with the same wrapper and release profile as
above, the full test runs give:

| Version | Optimization | Peak RSS, KiB | User CPU, s | System CPU, s | Wall | Result |
|---|---:|---:|---:|---:|---|---|
| Main emitter | 0 | 11,435,724 | 277.30 | 43.48 | 6:04.88 | 2170 passed |
| Bounded metadata | 0 | 1,418,108 | 228.18 | 37.16 | 4:58.75 | 2170 passed |
| Main emitter | 2 | 20,365,148 | 381.52 | 55.06 | 7:07.08 | 2170 passed |
| Bounded metadata | 2 | 2,149,808 | 270.52 | 37.02 | 4:19.99 | 2170 passed |

Each command was:

```sh
ILANG_COMPILE_CACHE=0 $H /run/current-system/sw/bin/time -v ./target/release/i test --project stdlib --opt N
```

The main-emitter baseline uses the emitter from `95b1b500`; both variants
use the merged source tree. The Wasmtime compilation cache is disabled.
Stdout and stderr were captured separately, without stage tracing (which
would reset the high-water mark). All four complete stdout files are byte
for byte identical: 2170 result lines, in the same order, and the summary.
All exit statuses are zero; none produces diagnostics on stderr.

Peak RSS falls from 10.91 to 1.35 GiB at O0 and from 19.42 to 2.05 GiB
at O2, below the 6 GB target at both levels. Total process CPU (user plus
system) falls from 320.78 to 265.34 seconds at O0 and from 436.58 to
307.54 seconds at O2. These are measurements on the shared machine, not a
promise of the same timing on other machines. The bounded emission was
first committed at `3bb591db` after the O2 run passed.

The 4097-note metadata regression passes. Final compatibility checking
also passes. Every heavy command below ran through `$H`; Cargo commands
used `direnv exec /home/agent/Programming/i`, and standard Rust test
harnesses used `RUST_TEST_THREADS=2`.

| Checking | Result |
|---|---|
| `cargo test --release -p ilang-host -p ilang-wasm --locked` | 165 passed |
| `cargo test --release -p ilang-cli --test many_tests --test compile_time_tests --test native --test compile_cache --test determinism --test commands --test macro_budget --locked` | 48 passed |
| `cargo test --release -p ilang-cli --test spec --locked -- FILTER` for `9/`, `10/`, `11/`, `9.7/`, `9.8/`, `10.` | 127, 6, 51, 17, 4, 14 cases passed respectively; 219 distinct cases, each at O0 and O2 |
| `i test --project examples/hello` | 2 passed |
| `i test --project bench` | 15 passed |
| `i test --project examples/macro-tests` | 9 passed, 1 skipped, as configured |
| `i test --project stdlib http` | 66 passed; complete output matches the baseline result lines selected by the same name substring, in the same order, plus the summary |
| `cargo clippy --release -p ilang-host -p ilang-wasm --all-targets --locked -- -D warnings` | passed |
| `cargo fmt --all -- --check` and `git diff --check` | passed |

The slash spec filters also match dotted sections ending with the same
number: for example, `11/` includes `9.11/`. The extra filters cover the
remaining subsections, including the test-entry cases in `10.2/`.

The first host/Wasm command exposed one pre-existing fixture mismatch:
main `95b1b500` changed the shared browser fixture to ABI 4, while the
legacy metadata unit test still explicitly asserted ABI 3. Commit
`399a7c3e` constructs that test's intended ABI 3 fixture from its WAT and
retains every assertion; other tests keep using the shared current
fixture. The complete host/Wasm rerun passes. No production ABI code or
language rules changed.

The full workspace run, unfiltered spec/corpus and CI execution remain
with the coordinator. No partitioning or per-module linking was needed;
the collection is still compiled once and every selected test still gets
its own started instance.

## ABI 5 small Ints and internal representation

ABI 5 keeps the three-i32 signature for Int and adds an inline signed-i32
value when count is zero and sign is nonzero (Appendix B.2). Zero remains
`(0, 0, 0)`; larger values still use sign/magnitude limb buffers. A counter
that calls a host function with a small Int and gets a small Int back removes
two linear-memory allocations and frees per call. The Rust and JavaScript
hosts select the encoding from each module's version, retaining limb buffers
for versions 3 and 4.

The repeatable measurement is `crates/ilang-cli/tests/abi_performance.rs` with
`tests/performance/abi`, release Rust and `--opt 2` guest code. Run it through
`cache/heavy` with `cargo test -p ilang-cli --test abi_performance --release
--locked -- --ignored --nocapture`. It checks each result, warms up once and
reports the median of five wall-clock samples, excluding compilation and
instantiation. `counter` makes 1,000,000 synchronous imports, passing the
previous result to a Rust host that adds one. `read_lengths` makes 1,000,000
imports with length 8 and an eight-byte BitArray result, isolating the encoding
cost without filesystem latency. `fs_reads` uses the real standard `fs.read`
adapter for 10,000 eight-byte reads from `/dev/zero`, opening and closing once
per sample. That case also includes requests, resource leases, scheduler turns
and filesystem work. `clock.monotonic_ns` is deliberately excluded because its
values soon exceed the inline range.

The test writes modules into `target/abi-performance`. Run the JS comparison
through `cache/heavy` with `node scripts/bench-abi.mjs
target/abi-performance/v4-O2.wasm target/abi-performance/v5-O2.wasm`; both
modules run with the same new SDK, which selects their respective encodings.

Internal Int representation was investigated without changing numeric
lowering. `runtime/bigint.rs` already uses allocation-free `i31ref` values in
`−2³⁰..2³⁰−1`; wider values use a sign and GC limb array. At O2,
`numeric/integer.rs` selects guarded small-Int parameters for numeric workers;
`emit/numeric.rs` can keep them and intermediate results in scalar `i64`
locals. A failed small-Int guard retains the general arbitrary-precision
path. The ABI's signed-i32 range is wider than the internal i31 range, so
values such as `i32::MIN` still need a GC object internally while avoiding a
host-boundary buffer.

`add` accumulates 456 a million times (result 456,000,000, within i31), and
`compare` compares the changing loop counter with 456 a million times. The
`empty` control only decrements the counter and carries a result. These are
whole-loop timings, including loop and function overhead, not isolated
instruction latencies.

Disassembly of the baseline O2 artifact confirms this path is used by these
kernels: the add worker has scalar `i64` parameters and an `i64.add` followed
by the signed-i31 range check; the compare worker uses `i64.lt_s` on scalar
parameters. Thus both i31 references and guarded scalar arithmetic already
exist before ABI 5; the boundary optimization does not introduce either one.

Measured on 2026-11-14, Linux x86-64, shared 16-core machine, release profiles
with LTO disabled and 16 codegen units (the heavy wrapper's settings).
The Rust baseline uses compiler/host `180e00cf7` with this same benchmark
source; the after run uses ABI 5. The JS comparison uses Node 24.21.0 and the same
ABI-5 SDK for both artifacts, selecting the wire encoding by module version.
All rows are seconds; each result was checked.

| Host and case | Iterations | ABI 4 | ABI 5 | Less time |
|---|---:|---:|---:|---:|
| Rust counter through a host function | 1,000,000 | 0.937546 | 0.586667 | 37% |
| Rust read length and eight-byte result | 1,000,000 | 1.029945 | 0.834817 | 19% |
| Rust real `fs.read`, eight bytes from `/dev/zero` | 10,000 | 0.522017 | 0.386503 | 26% |
| JS counter through a host function | 1,000,000 | 3.482284 | 0.490162 | 86% |
| JS read length and eight-byte result | 1,000,000 | 2.383987 | 0.784770 | 67% |

The read-length microbenchmark computes the next length from the returned
BitArray's bit size, so the return value is consumed. The real filesystem
case is noisy: its five samples ranged from 0.253–0.841 seconds before and
0.327–0.706 after. Its median is an observation, not an isolated estimate of
encoding cost. The synchronous microbenchmarks give the clearer boundary
comparison.

| Internal O2 loop, 1,000,000 iterations | ABI-4 baseline | ABI-5 run |
|---|---:|---:|
| Int add, accumulating 456 | 0.003944 | 0.005456 |
| Int compare, changing counter `< 456` | 0.003843 | 0.004802 |
| Empty control, decrement and carry | 0.005268 | 0.002811 |

These millisecond-scale internal loops have substantial scheduling noise
(the baseline empty loop ranged from 0.003443 to 0.016764 seconds). Their
ordering does not establish an arithmetic speed difference: numeric lowering
is unchanged and only one call crosses the boundary per loop. The finding is
that the existing i31/scalar fast paths are exercised, rather than allocating
a boxed arbitrary-precision Int for every small add or compare. No new internal
representation or numeric optimization was implemented in this lane.

The original ABI-4 Rust metadata reader and JavaScript SDK were also checked
against version-5 metadata; both reject it with the existing message:
`the module uses i ABI version 5, but this host reads version 4; update the host`.

## CI baseline and the always-inline threshold (2026-11-19)

These numbers come from GitHub Actions' `ubuntu-24.04` hosted runners (4 vCPU,
16 GB), a release `i` built from the stated commit, `RAYON_NUM_THREADS=1`, and
the benchmark package's own sources. `i test --time` gives each test's wall
time and the process's CPU time (user plus system) without the build;
`/usr/bin/time -v` gives the peak RSS.

**Runner speed is not constant.** The same binary measured `list_sort` in
1.98 s on an Intel Xeon 6975P-C and 2.63-2.69 s on an AMD EPYC 7763 in the same
workflow, and 2.18 s on an AMD EPYC 9V74. Absolute numbers from different
runners, including the older numbers earlier in this document, are therefore
not comparable across machines. Every comparison below alternates its
configurations inside one job (five rounds, rotated order), so both sides see
one CPU.

Each timed round records `free -g` and `/proc/loadavg` before and after the
run (`machine.txt` and `load-*.txt` in the artifacts). A round whose load
exceeds half the machine's core count is not a measurement and is discarded.
On the shared development machine, builds use `CARGO_BUILD_JOBS=2` and only
one compiler or test process runs at a time; these CI measurements ran on
4-vCPU runners and recorded load 0-1 throughout.

Commits: `current` is `a906931b9` (main when this was measured); `documented`
is `ad87d8c06`, the tree the earlier "After" numbers were recorded on; `change`
is the always-inline commit. The `documented` column runs that commit's own
benchmark sources because the current sources use newer syntax; the benchmark
algorithms are the same.

### Benchmark table

Median of five alternating rounds in one job, seconds. Process CPU time is
within 0.02 s of the wall time in every row, so only the wall time is shown.

| Benchmark | current `--opt 0` | current `--opt 2` | documented `--opt 2` | always-inline 40 `--opt 2` | 40 vs current |
|---|---:|---:|---:|---:|---:|
| `list_build` | 0.136 | 0.084 | 0.098 | 0.068 | -19% |
| `list_map` | 0.194 | 0.128 | 0.150 | 0.109 | -15% |
| `list_reverse` | 0.269 | 0.186 | 0.239 | 0.163 | -12% |
| `list_sort` | 3.166 | 1.966 | 2.849 | 1.615 | -18% |
| `list_merge` | 0.855 | 0.580 | 0.853 | 0.572 | -1% |
| `closures` | 0.109 | 0.069 | 0.064 | 0.065 | -6% |
| `inspect` | 0.145 | 0.115 | 0.094 | 0.110 | -4% |
| `dict_insert` | 0.582 | 0.452 | 0.410 | 0.450 | -0.4% |
| `equality` | 0.314 | 0.255 | 0.228 | 0.247 | -3% |

Median peak RSS of the whole `i test --project bench` over the five rounds:
421 MB on `current` at `--opt 2`, 396 MB at `--opt 0`. The 1M-element lists do not approach the
runner's memory; the 11-19 GB figures earlier in this document were the `std`
test collection's Wasmtime compilation, which this package does not have.

Against the documented tree on the same machine, `current` is faster on the
list benchmarks: `list_sort` -31%, `list_merge` -32%, `list_reverse` -22%,
`list_build` -14%, `list_map` -15%. It is slower on `equality` (+12%),
`dict_insert` (+10%), `inspect` (+22%) and `closures` (+8%); `inspect`'s
output work also changed since that tree (labels and canonical order), so only
part of its difference is code generation. The others are the representation
changes since then (unions of single-constructor types, `Nil` removal); they
are the reason a representation change is not free and why the next one must be
paired.

### Where `list_sort` spends its time

The comparison kernel in `bench/test/diag_test.i` (diagnostic branch only, never
merged) runs 10M iterations of one self tail call. Three rounds on one Xeon
6975P-C agreed to 0.002 s:

| Loop body, 10M iterations | seconds | ns per iteration |
|---|---:|---:|
| empty loop | 0.179 | 17.9 |
| `int.compare` called directly, result matched | 0.330 | 33.0 |
| a function value from a list, called the same way | 0.634 | 63.4 |
| a capturing closure, called the same way | 0.514 | 51.4 |

A call through a function value costs about 30 ns more than a direct call;
`list.sort` compares about `N log N ≈ 20M` times for 1M elements, so about
0.6 s of its 2.0-2.2 s is the indirect comparator call.

The collector is not the cost: on one runner the same `list_sort` at `--opt 2`
took 1.83 s with copying and a 1 GiB heap, 2.04 s with `ILANG_GC=null`, and
12.97 s with `ILANG_GC=drc`.

Allocation is the rest. `list.reverse` costs 0.19-0.22 s per 1M cells
(37-44 ns per new cell); the sort creates two cells per element per pass
(accumulate, then reverse) over about 19 passes, roughly 38M cells and about
1.4 s. Ints stay `i31ref` and `Order`/`Bool` are canonical globals, so there is
no per-comparison boxing. What the compiler can still remove is call overhead:
every list access, pattern test and shadow-stack position stayed a Wasm call,
because Binaryen's default inlines only functions of 2 expressions and
Wasmtime inlines nothing.

### The change: always inline up to 40 expressions

`ilang-binaryen` already called `BinaryenSetAlwaysInlineMaxSize`. The change
raises that from 20 to 40, so the small runtime helpers around list cells,
patterns and the shadow stack become part of their callers at `--opt 2`.

Measured on the same runner in the same run (Intel Xeon 6975P-C; the same
`current` binary measured 1.98-2.00 s for `list_sort` in the other paired
runs, so these numbers are reproducible): `list_sort` 1.966 -> 1.615 s
(-17.9%), `list_build` -19.0%, `list_map` -14.8%, `list_reverse` -12.4%,
`closures` -5.8%, `inspect` -4.3%, `equality` -3.1%, `list_merge` -1.4%,
`dict_insert` -0.4%. No benchmark got slower. The gain is the calls around
list cells, pattern tests and shadow-stack positions that are now inlined;
the indirect comparator call itself is not removed by this change.

The cost is compile time. The earlier measurement in this document put the
`std` test build at 3.5 s with 20 and 5.3 s with 40; that measurement was not
repeated here. The inspection threshold itself is unchanged.

### A second measured candidate, not taken

Ordinary values (Bool, Float, String, records, data) can be allocated as a new
`final` subtype of `$obj` instead of `$obj` itself. A cast to a final type is
an exact type-index comparison, while a cast to the non-final `$val` calls
Wasmtime's `is_subtype`. A paired run on one runner measured `list_sort`
2.179 -> 1.953 s (-10%), `list_build` -10%, `list_map` -6%, but `equality`
+8% (the generic comparison pays for a type test per operand). The optimized
probe module went from 34 casts to `$val` to 1. It is a real but smaller win,
and it was not landed: the plan is one change at a time, and the inline
threshold was larger and had no regression. The branch and the numbers stay in
`ci/perf-obj-final` as the record.

Verification: the PR's `CI` workflow runs the spec suite, the `ilang-check` and
`ilang-cli` groups, the review corpus and the `i` group on the change commit.

### Devirtualizing a known function argument (2026-11-19)

`list.sort(xs, int.compare)` calls the comparator through the `by` parameter
in every merge step. A function value is called with `call_ref`; the
measurement above puts that at about 30 ns more than a direct call, and the
sort compares about 20M times.

The compiler now specializes a callee for a passed `Atom::Func(g)` — the one
closure of a top-level function, a constructor, or a library function used as
a value. The callee is cloned with that parameter bound to `g`, calls through
the parameter become direct calls, and direct calls that forward the parameter
are specialized recursively. It runs after lowering and before
`control::split`, so the existing `may_suspend` fixpoint decides whether a
clone still needs suspension points; a known function that may suspend is left
indirect. `--opt 0` never runs the pass, so the reference lowering stays
independent.

**Generated-code evidence.** On a probe with only `list.sort(randoms(1000),
int.compare)` at `--opt 2`:

| Probe | `call_ref` sites | sort-chain functions with a comparator `call_ref` | bytes |
|---|---:|---:|---:|
| current | 25 | 7 functions, 10 sites | 275,986 |
| specialized | 17 | 0 in each of the 7 clones | 276,963 |

The clones are reached from `main`; the original functions remain in the
module only because their constant closures are still referenced, which is why
the module-wide count falls by 8 rather than 10.

| Benchmark | current (inline 40) | specialized | change |
|---|---:|---:|---:|
| `list_build` | 0.086 | 0.068 | -20.9% |
| `list_map` | 0.131 | 0.108 | -17.6% |
| `list_reverse` | 0.191 | 0.163 | -14.7% |
| `list_sort` | 1.990 | 1.226 | -38.4% |
| `list_merge` | 0.586 | 0.570 | -2.7% |
| `closures` | 0.069 | 0.064 | -7.2% |
| `inspect` | 0.117 | 0.111 | -5.1% |
| `dict_insert` | 0.466 | 0.440 | -5.6% |
| `equality` | 0.255 | 0.246 | -3.5% |

Median of five alternating rounds in one job on an Intel Xeon 6975P-C; process
CPU time is within 0.02 s of the wall time, so only the wall time is shown.
No benchmark got slower. `list_sort` falls by 38%; the extra beyond the
comparator-call estimate is that the direct call no longer needs the
`suspendable` handler around an unknown function value. `list_map` and
`list_build` gain because the non-capturing callbacks they pass are specialized
too.

Rejected shapes are tested as well, each asserting that `call_ref` remains: a
capturing closure, a parameter that escapes into another closure, a known
function that may suspend (directly or through an indirect call of its own),
and a function whose parameter is only forwarded indirectly. Accepted shapes
are tested too: a plain direct call, recursion through the parameter, and the
same callee passed two different known functions (two clones).

The cost is code size: clones duplicate a function body. The pass caps them at
eight times the program's function count; the probe grew 0.35%. Exports, the
ABI metadata, source positions and the inspection threshold do not change.

### Reusing a list comparison's continuation frame (2026-11-19)

`equality` (20 comparisons of two 100k-element lists of `(Int, String)`
tuples) was the slowest remaining benchmark the compiler can reach. The
isolating probes ran three times in one job on an Intel Xeon 6975P-C:

| Comparison, 20 rounds of 100k elements | seconds | ns per element |
|---|---:|---:|
| Int lists | 0.059 | 29.5 |
| `(Int, Int)` tuple lists | 0.171 | 85.5 |
| `(Int, String)` tuple lists | 0.241 | 120.5 |

Building 2M cons cells on the same runner took 0.111 s, 55.5 ns per cell. The
56 ns a two-field tuple costs over a plain Int element is therefore one
allocation per list cell: `$value_compare_i32` remembered the two list tails
in a heap `$compare_frame` while it compared the heads, and allocated a new
frame for every cell.

A popped list continuation now stays in a local and is reused for the next
cell, so a list comparison allocates one frame per nesting level instead of
one per element. The frame fields became mutable for that reuse. This is a
runtime implementation change (it applies at every `--opt` level); it does
not change the comparison result.

Two independent paired runs, five alternating rounds each, on two runners
(seconds, medians; the second run's absolute times are slower, so only the
within-run change is meaningful):

| Benchmark | Xeon Platinum 8573C | AMD EPYC 7763 |
|---|---:|---:|
| `equality` | 0.367 -> 0.246 (**-33.0%**) | 0.444 -> 0.256 (**-42.3%**) |
| `list_sort` | +0.8% | +0.4% |
| `list_merge` | +0.9% | +0.4% |
| `list_build` | -1.1% | +2.1% |
| `list_map` | -0.7% | +0.6% |
| `list_reverse` | -0.5% | 0.0% |
| `closures` | 0.0% | 0.0% |
| `inspect` | -2.3% | -0.6% |
| `dict_insert` | -0.9% | 0.0% |

The equality probes confirm the same cause: `diag_equal_20x` -33.2%,
`(Int,Int)` tuple lists -26.5%, Int lists -5.1% (noise). The other benchmarks
move by at most 2.1%, with overlapping ranges, and none of them executes the
changed comparison; `list_build`'s +2.1% is a 2 ms difference in a path that
never calls `==` on a list, so it is layout noise rather than a regression.

Compile time for the specialization committed just before this one was
measured paired on one AMD EPYC 7763: `i test --project stdlib --opt 2` with a
filter that matches no test (the whole std collection build) took 233.8 and
236.6 s of CPU before and 234.5 and 236.4 s after, and 2.76-2.80 s for the
small probe package in both. About 0.1-0.2% of the std build, not a
measurable cost. (The +1.8 s quoted earlier in this document is the
always-inline change, which is a different change.)

### Type-directed structural equality (2026-11-20)

After the method-call and frame work, `equality` was 0.245 s for 20
comparisons of two 100k `(Int, String)` lists. Int-list elements cost 39.5 ns
and tuple elements 90 ns; the ~50 ns gap was per-element tag, cast and scan,
not allocation.

The checker now records the two operand types of `==`/`!=`
(`Typed::comparison_types`). At `--opt 2/3`, when both are the same closed
structural type, lowering builds a comparison plan and emit generates a
`$eq_plan_N` function: Int through `$int_compare`, Float through `f64.eq`
(Float values are always finite, so no NaN reaches it), String through
`$string_compare`, lists walk their cells and tuples their parts. No
`value_tag`, cast or frame per element.

Everything else keeps the generic `$equal`: functions, host references,
opaque values, custom types, unions whose members need different plans, and
anything over the caps — nesting depth 6 and 32 plans. A bare scalar
comparison also stays generic so the numeric plan keeps seeing it. `--opt 0`
never specializes, so the reference lowering and the differential corpus stay
independent.

Paired, one Xeon 6975P-C, five alternating rounds, `--opt 2`:

| Benchmark | main | type-directed | |
|---|---:|---:|---:|
| `equality` | 0.180 | 0.116 | **-35.6%** |
| `list_sort` | 1.254 | 1.251 | -0.2% |
| `dict_insert` | 0.488 | 0.484 | -0.8% |
| `list_merge` | 0.561 | 0.563 | +0.4% |
| `closures` | 0.065 | 0.065 | 0.0% |
| `list_build` | 0.069 | 0.070 | +1.4% |
| `list_map` | 0.110 | 0.112 | +1.8% |
| `list_reverse` | 0.170 | 0.173 | +1.8% |
| `inspect` | 0.109 | 0.112 | +2.8% |

The equality probes agree: 20x lists -37.2%, tuple lists -38.9%, Int lists
-31.6%. The small increases are in paths that compare only Ints or Options
(build, map, reverse, inspect) and their sample ranges overlap; they are code
layout, not a plan on the path.

Compile time and size: paired on one runner, the warm `std` test build (a
filter that matches no test) took 148.1 s of CPU before and 149.9 s after
(**+1.2%**). A probe with one list-of-tuples equality grew from 271,952 to
272,189 bytes (**+0.09%**); the sort-and-map probe, which has no structural
equality, is byte-identical.

Tests: specialization and the fallbacks (function inside a structure, record,
mixed union, depth over the cap); float boundaries (`-0.0 == 0.0`,
`0.1 +. 0.2 != 0.3`); and a stop case showing that a NaN cannot become a
Float value, so `NaN == NaN` is unreachable rather than defined. The full CI
gate passes, and corpus output is unchanged at O0 and O2.
## `list.sort`: one list cell per element per merge level (2026-11-20)

The merge built each output element twice: a cons into the accumulator, then a
cons again in `reverse_onto`. `sort` now alternates the merge direction by
level. Ascending runs are merged into a descending run and descending runs into
an ascending run; both write one cons per element and need no per-level
reverse. Only a carried run and a final descending level pay for one reverse.
The sorting order and stability are unchanged; `sort_stability_test` and the
new `sort_stability_many_test` pin equal keys in their original order.

Method: release `i` from this commit, `i test --project bench --opt {0,2}
--time`, five alternating rounds (old/new rotated), median wall seconds, one
process at a time pinned to four CPUs. Same machine, same run.

| Benchmark | `--opt 0` before | `--opt 0` after | `--opt 2` before | `--opt 2` after |
|---|---:|---:|---:|---:|
| `list_sort` | 4.913 | 3.776 (-23%) | 1.609 | 1.100 (-32%) |
| `list_merge` | 1.233 | 1.242 (+1%) | 0.707 | 0.728 (+3%) |
| `dict_insert` | 1.086 | 1.090 (0%) | 0.840 | 0.899 (+7%) |

`list_merge` and `dict_insert` do not use `sort`; their differences are run
noise (individual `dict_insert` `--opt 2` runs ranged 0.78-0.97). The change is
confined to `sort`'s merge.

## Stage report card (2026-11-20)

The four retained compiler changes of this stage, the two directions rejected
by probes, and the standard library's sort-cell change. Every row is a paired
run on one runner, five alternating rounds (seven for the dict repeat), and
`--opt 2` unless stated. Different items ran on different runner models, so
only the deltas within a row are comparable.

**Cumulative.** One job on a Xeon 6975P-C, five rounds in rotating order, each
tree compiled by its own compiler: the tree before the inlining change ran
`list_sort` in 1.990 s and the current tree in 0.860 s, **-56.8%**; the tree
before the comparison-frame reuse ran `equality` in 0.260 s and the current
tree in 0.090 s, **-65.4%**. Over the same span the compiler got 1.2% slower
and the generated module 0.09% larger, with corpus output unchanged. Load
stayed between 0.8 and 2.6 on the 16-core runner.

Those two chain numbers are one measurement of three trees on one machine
(`fb81d62ee`, `12930cd64`, `d1a099c63`), so they carry no cross-machine
estimate. The per-item rows below were taken on several runner models; the
1.966 s and 0.367 s starting points quoted in earlier records sit on different
machines, which is why the chain restates both ends. The before-inlining tree
reproduced at 1.990 s against the recorded 1.966 s, and the equality chain
starts at 0.260 s rather than 0.367 s because 0.367 s is the 8573C measurement
of that same tree; on this runner the frame-reuse record's own Xeon column was
0.255 s.

| Item | Machine | Change | Result |
|---|---|---|---|
| always-inline 40 | Xeon 6975P-C | `list_sort` 1.966 -> 1.615 s; `list_build` -19.0% | -17.9%; no benchmark slower |
| known-argument devirtualization | Xeon 6975P-C | `list_sort` 1.990 -> 1.226 s; 10 comparator `call_ref` sites -> 0 in the clones | -38.4%; module +0.35% |
| comparison-frame reuse | 8573C / EPYC 7763 | `equality` 0.367 -> 0.246 s / 0.444 -> 0.256 s | -33.0% / -42.3% |
| type-directed structural equality | Xeon 6975P-C | `equality` 0.180 -> 0.116 s | -35.6%; compile +1.2%; equality probe +0.09% bytes |
| pass 2 probe (rejected) | Xeon 6975P-C | 89.5% of 6331 direct call sites already target a body within `ALWAYS_INLINE`; after Binaryen only 10.5% of the 3427 remaining calls target a body of at most 40 lines | small direct gain, not built |
| pass 3 probe (rejected) | Xeon 6975P-C | `dict_insert` 0.385 -> 0.328 s with the per-level result tuples removed | 14.8% upper bound, below the 20% target |
| standard library: alternating merge | Xeon 6975P-C, O2 / O0 | `list_sort` 1.220 -> 0.840 s / 3.090 -> 2.500 s | -31.1% / -19.1% |

The std change also moved the two neighbouring benchmarks, inconsistently:
`list_merge` -8.8% at O2 and +6.1% at O0, `dict_insert` +7.3% at O2 on this
machine and -2.8% (0.720 -> 0.700 s) in a repeat on an EPYC 7763, 0.0% at O0.
`stdlib/src/dict.i` and the compiler are identical before and after, so the
one +7% reading is run variance, not a regression. Load stayed between 0.4
and 1.6 in every round; the protocol is in the section above. All four
compiler changes passed the full gate with corpus output unchanged.
