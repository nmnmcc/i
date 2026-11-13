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
| `i check --project std` | 20.4 s | 0.13 s |
| `i test --project std --opt 2` (1376 tests, with the build) | 190 s | 34 s |
| `i test --project std --opt 0` | | 39 s |
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
| `check_std` | `i check --project std` (checks the tests too) |
| `build_std_tests_o0`, `build_std_tests_o2` | `i test --project std --opt N` with a filter that matches no test: the build of the whole test program, and no test runs (`i test` has no build-only mode) |
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
| `i test --project std` peak memory | 8.5 GB at `--opt 0`, 17 GB at `--opt 2` |
| `i check --project std` peak memory | 0.3 GB |

The memory of `i test --project std` is the backend (the Wasm module, Binaryen and Wasmtime's
code generation), not inference, which is why `i check --project std` stays small.

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
  speed, but `i test --project std` then peaked at 3.7 GB of memory, and it
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
a1 = { a0 % 3 } |> { 0 -> s(b, a0 + 1)  _ -> a0 + 2 }
a2 = { a1 % 3 } |> { 0 -> s(b, a1 + 1)  _ -> a1 + 2 }
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
`opt-level = 2` and the workspace crates at 0. `i test --project std` in a
debug build took about 50 minutes; most of that was the type checker (20 s
for `std` even in a release build) and the DRC collector, both fixed above.
It now takes under a minute.

Workspace crates at `opt-level = 1` (with `CARGO_PROFILE_DEV_OPT_LEVEL=1`,
dependencies still at 2) against 0, debug builds, wall time:

| | 0 | 1 |
|---|---|---|
| `i test --project std --opt 0` (1376 tests) | 51.6 s | 51.2 s |
| the `list_test` module of it | 39.7 s | 39.9 s |
| `cargo test -p ilang-cli --test spec`, built | 11.3 s | 11.8 s |
| `i check --project std` | 0.80 s | 0.19 s |
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
I_TRACE_STAGES=1 ILANG_COMPILE_CACHE=0 $H /run/current-system/sw/bin/time -v ./target/release/i test --project std --opt 2
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
