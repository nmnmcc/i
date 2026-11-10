# R2 quiet performance audit

Reproduce from the lane with:

```sh
direnv exec /home/agent/Programming/i python3 tests/r2-measure.py target/r2-measurement
```

The driver builds the Rust harness through heavy, then reserves all three heavy
slots with quiet. Cargo is never nested inside quiet. It uses i O2, Wasmtime
50.0.0 with Cranelift Speed, the standard copying collector and 256 MiB initial
GC heap, on an AMD EPYC-Genoa VM. Compilation, instance initialization and
200 warmups per kernel/size are outside timing. One instance is reused inside the real Rust host's
block_on context, avoiding a new host thread per operation. The quiet lock
excludes cooperating compiler jobs, not VM scheduling or frequency changes.

Each n=32/64/128 experiment has five batches of 21 paired samples. Order
alternates. Report the median of the five batch medians, retaining signed
samples rather than clamping negative corrections. For a kernel K and empty
loop E, subtract K(0) and L*(E(n)-E(0)) from K(n), then divide by n operations.
L is one for spawn/join and two for switch/channel because both peers count
loops. Switch uses 2*n operations: each yield transfers between two ready
tasks. Channel uses n complete send+receive rendezvous transfers; an endpoint
cost is half that. Spawn uses an empty Nil child, joins it, and reuses one scope.
Every returned result is checked. The separate 513-child spec case checks all
retained child results and repeated late joins before retirement.

Two measurements separate scope retirement from the primitive. The ordinary
export timer includes amortized retirement of its one scope; K(0) removes
fixed entry, context and scope setup. The interval variant starts inside that
scope and stops before retirement. Its two synchronous Float timer imports
are removed by the zero-work pair. It adds no service slot or scheduler command.
The paired external timer around that same invocation shows the remaining
retirement/entry portion. This subtraction is an estimate, not a hardware-cycle
bound; small negative residuals can occur with independent clocks and noise.
It does not claim to isolate bare allocation, or spawn without join.

## Final quiet timings

The accepted run is target/r2-measurement-accepted, with source hashes checked
before/after the build and measurement. All three harness tests passed. These
are microseconds per operation, after the paired baseline corrections:

| Primitive, inside one scope | n=32 | n=64 | n=128 |
| --- | ---: | ---: | ---: |
| Empty task spawn+join | 89.216 | 97.745 | 107.251 |
| One switch between two ready tasks | 13.029 | 13.199 | 13.045 |
| One channel send+receive | 34.975 | 35.982 | 34.870 |

The independent outer-only run, including amortized scope retirement, gives
92.867/99.347/109.681 us for spawn+join, 13.866/13.238/13.255 for a switch, and
40.200/34.265/35.200 for a transfer at the same sizes. The interval variant's
paired outer timer at n=128 gives 109.959, 13.177 and 35.054 us respectively.
This puts scope retirement/remaining entry work in the low-microsecond range
per task, rather than explaining most of the spawn cost. The endpoint share
of the n=128 rendezvous is about 17.435 us; this is half a complete transfer,
not a separately timed send instruction. The log-dependent spawn cost and
quiet-window variation are visible rather than collapsed into one best sample.

These costs remain far from a Go-like lightweight-spawn claim. The old quiet
n=128 aggregate was about 907 us; the final comparable outer cost is about
110 us, about 8.3 times lower. That comparison also spans warmup/heap effects,
so the structural counter evidence below is the stronger attribution of what
was removed. No sub-microsecond or worst-case claim is made.

The original quiet snapshot retains the old implementation's complete raw
samples under target/r2-perf-before. Its empty spawn/join cost grows from
328 us at 32 children to 518 us at 64 and 907 us at 128. This is not the earlier
543 us scope-per-child aggregate, and no goroutine comparison is inferred from
that differently defined measurement.

The owned scheduler fixes are:

- A persistent AVL store replaces linear task lookup/update. Clean results stay
  available until their scope retires, but do not participate in normal-turn
  scans. Separate path indexes preserve task creation order for active work and
  unfinished cleanup; ordinary status updates do not copy those indexes.
- Existing-value replacements preserve tree shape/heights. Only insertion and
  removal rebalance. The initial AVL version needlessly rebalanced replacements;
  the hot-function counts exposed that source algorithm cost.
- A two-list FIFO and queued flag replace linear enqueue copies and normal-turn
  membership scans. The script policy retains its explicit search behavior.
- Scope pending counts replace repeated child-liveness scans. Retirement removes
  its recorded children; reversing that private deletion list changes no turn.
- Ticket interest uses the actual Await registration index. Maintenance does
  not repeat unchanged pure-work scans after a no-op poll or turn selection.
- Boundary callbacks capture projected run/runner/path/body values rather than
  the whole scheduler State or an obsolete TaskState. The captured loop state
  was an incidental retention path, even though the loop continuation was outside.
- The task's random stream is derived when random() reads it, rather than for
  every empty spawn. The complete root generator and path remain captured. R1's
  algorithm and all random values are unchanged.

Cancellation still visits affected active tasks; CML partner search visits
registrations/leaves. These are algorithm costs, not claims of constant-time
operations for arbitrarily large cancellation sets or choices. The measured
switch/channel kernels have two tasks and one channel. Task storage costs grow
logarithmically with retained results. No multicore result or native.md's
future within-2% multicore/M:1 comparison is claimed.

Counters are inserted after O2, into the actual timed module printed by
wasm-tools. They count executed struct.new*/array.new* and function entries,
including tail entries. They do not count bytes, live objects, collections or
native CPU instructions. Initialization is excluded. The instrumented modules
are never timed and all results/statuses are checked. Pure kernels must return
status 0: no request wait, progress poll or event-loop hand-off occurs.

For attribution only, the C helper repeats the compiler's exact Binaryen O2
settings with debug names. Every optimized function body is compared with the
actual artifact before names are used. Declarative elem order can differ;
source formatting in the old snapshot also changed site data. Those differences
are not called whole-binary equality. The counters still run in the actual
artifact, and the saved hot WAT is uninstrumented code with separate counts.

Source interpretation: runtime.md §6.4 requires FIFO and cancellation bookkeeping
before normal turns; §6.2 requires retaining child results until structured
retirement. The index/queue changes preserve those rules. §§4/6.1 require the
same complete-generator path derivation and no suspension in random(), not eager
computation of unused words. The independent O0/O2 law and trace tests establish
those contracts; the counters do not prove language semantics.

## Executed O2 work per operation

At n=128, apply the same zero-work/loop correction to the dynamic counts. These
are signed deltas averaged per operation, not independently timed categories.
The final production code has the following counts:

| Executed work | Empty spawn+join | One ready-task switch | Send+receive transfer |
| --- | ---: | ---: | ---: |
| Function entries, including runtime helpers | 6,691.75 | 535.77 | 1,509.85 |
| All GC allocations | 1,551.90 | 152.48 | 470.99 |
| Generic object records/protocol boxes | 638.93 | 50.98 | 161.97 |
| Value arrays, including record/capture/environment storage | 710.94 | 59.48 | 192.98 |
| List cells | 45 | 8 | 29 |
| Closure objects | 50 | 3.99 | 21 |
| Captures plus continuation objects | 6 | 2 | 4 |
| Frames plus frame-list nodes | 60.03 | 17.02 | 38.05 |
| Subcontinuations plus trampolines | 36 | 11 | 24 |
| Boundary objects | 2 | 0 | 0 |
| Byte arrays | 3 | 0 | 0 |
| Float boxes | 0 | 0 | 0 |
| ABI host waits/progress hand-offs | 0 | 0 | 0 |

Value arrays share a canonical GC representation after Binaryen; the count
cannot by itself distinguish every environment tuple from a record backing
array. The separate frame/continuation counts do not include all captured-value
storage. Tiny negative limb deltas (-1/256 per switch, -1/128 per transfer)
come from subtracting extra empty-kernel argument/result conversions. They are
baseline corrections, not negative physical allocations; no limb allocation
is charged to the small loop counters.

The old n=128 spawn/join used 68,179.86 entries and 7,722.26 allocations. Its
wide-integer RNG work included 3,123.52 default limb arrays, 678.77 fixed limb
arrays and 133.24 other limb arrays per operation. Deferring unused at_path
work removes that work rather than changing the random algorithm. Completed
result scans and copied FIFO/child lists also disappear from the hot path.
The final task store retains logarithmic tree work, including immutable path
copies; that is an explicit scheduler algorithm cost.

## Attribution and compiler follow-up

The uninstrumented *-hot.wat files contain the twelve most frequently entered
functions for each kernel, with separately counted entries. They show the
remaining dynamic constructor/tuple/Bool tests, structural equality, logical
shadow-stack operations, typed tree lookups/updates and control trampoline
calls. Capturing and re-entering a suspended task still builds the frames,
frame lists, continuation and saved environment visible in the table. A channel
transfer builds CML descriptions/registrations and drives both continuations.
These operations stay entirely inside Wasm; the pure artifact imports only
panic, which is never called in a successful sample. Collector/runtime-internal
calls are not being described as user ABI calls. The interval probe alone has
two synchronous timer imports per batch, removed by its paired baseline.

The distinction is:

- Scheduler algorithm: persistent tree search/path copying, scope bookkeeping,
  queue operations, required event validation/registration/commit, and the
  chosen suspension points. The needless scans, queue copies, eager RNG,
  duplicated index updates and replacement rebalancing found by this audit
  were fixed in i/task itself.
- Compiler/code generation: erased generic values and records repeatedly cross
  helper calls; records, tuple/capture arrays and closures remain heap objects;
  generated field/type tests and logical trace helpers remain calls; captured
  environments and lazy re-entry retain their general runtime representation.
  Counts are not CPU-time percentages, and no unsupported latency attribution
  to a single category is made.

This agrees with numeric-lowering.md §2's findings: small Int arithmetic need
not allocate, while escaping aggregate/callback boundaries block scalar
replacement and helper specialization. Here the large counts are records,
arrays and control objects, not Float boxes or boxed small Int loop variables.
R3's known-call/helper work, R6's scalar aggregates and R7's capture/constant
work are general compiler follow-ups. The three per-spawn byte arrays also show
that eager error-message arguments are still materialized. No compiler was
optimized, no intrinsic recognizes i/task, and no Go-level cost is claimed.

The final projected-capture implementation has the same allocation counts as
the preceding version: projecting existing references changes what the callback
retains, not how many closure/capture-array objects it allocates. The O0 boundary
callback's array contains four projected values (path, run key, Runner, body),
with no State/TaskState operand; its surrounding re-entry frame saves no such
local. This is retention evidence, not a claimed measured reduction in peak
live heap. The final source snapshots and hashes are saved with the accepted
measurement, and the driver rejects source edits during build/measurement.
