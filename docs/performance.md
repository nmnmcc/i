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

The environment variables `ILANG_GC` (`copying`, `drc`, `null`) and
`ILANG_GC_HEAP` (MiB) choose the garbage collector and the initial size of
the GC heap (see `crates/ilang-host/src/lib.rs`).

The numbers below are from one Linux x86-64 machine with 16 cores, shared
with other work, so they vary by about 10%. Where the machine was busy,
processor time is given.

## Results

Release build, `--opt 2`, seconds. "Before" is the head of `new-compiler`
(`f7b3964`); "after" has every change below.

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

## Type checker

A function with many suspend points and no annotations, of this shape
(`s` suspends):

```i
a1 = case a0 % 3 { 0 -> s(b, a0 + 1)  _ -> a0 + 2 }
a2 = case a1 % 3 { 0 -> s(b, a1 + 1)  _ -> a1 + 2 }
...
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
