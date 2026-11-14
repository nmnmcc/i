# T1 performance gate

Source: literal-types §9, “std `i check` median: at most +10%”,
“at most 2× base at n = 1000” and “no part grows more than 2.5×”.
These are checking times (parsing, resolution, inference, coverage and
interface construction), not Wasm compilation or execution times.

Run from this lane, through the dev shell, with two release binaries.
`time-std.sh` uses the same current std sources for both. Both scripts
warm each binary once, then record 11 sequential samples and compare
medians. They retain binary hashes, affinity, load and every sample in
JSON. A failed check is a failed run, including the warm-up; it is never
timed as a successful check. Do not call a busy-machine measurement a
quiet-window result.

```
direnv exec /home/agent/Programming/i tests/t1/time-std.sh BEFORE AFTER --output target/t1/perf/std.json --require-quiet
direnv exec /home/agent/Programming/i tests/t1/stress-time.sh BEFORE AFTER --output target/t1/perf/stress.json --require-quiet --ignored-tests
```

The `--require-quiet` measurement gate subtracts child CPU time from
the machine busy ticks during each block and requires background work
to use at most 5% of total capacity. This is an explicit operational
interpretation of “quiet window”; load averages are also retained.
The aggregate `perf-gate.sh BEFORE AFTER REPORT_DIR [CPU]` uses the
coordinator's `.rd/cache/quiet` wrapper to reserve all three heavy-job
slots for each timing command. It releases those locks before running
the four ignored tests through the ordinary heavy wrapper, avoiding a
nested lock wait. This is the final gate command in this lane.
An optional `--cpu N` pins both binaries equally. The coordinator can use
this with a quiet CPU/window. The supplied new-syntax baseline,
`/home/agent/Programming/i/.rd/cache/bin/i-1df2fe0`, checked Std-2 but cannot
check the later property module: its bundled i/random predates R1. The
final baseline is `target/t1/continued/i-baseline-94789e6`, a copied
pre-T1 upstream release build. All 910 tracked source blobs of its source
tree were verified against 94789e6, its binary hash was preserved, and
its build timestamp is newer than those sources. A control reports
`x = 1` as Int; it checks the same expanded std sources as T1. The build
was performed by the other lane, not by this one. Provenance is recorded
in `target/t1/continued/borrowed-baseline-provenance.json`.

Both 94789e6 and the earlier syntax baseline are explicit substitutes for
the note's 1865366 compiler, whose old syntax cannot check this source
tree. The final comparison includes all T1 changes, unlike the separate
pre-stage-7 binary (which already has the latent representation changes).
That earlier binary stays at `target/t1/continued/i-before-stage7` for
historical corpus comparisons.

`gen-stress.py DIRECTORY N PART` makes keys, pick, calls, alias or nested.
The timed keys/pick sizes are 250/500/1000. Pick has a catch-all and a
written String input so both compilers accept exactly the same program.
The calls sizes are 2500/5000. The local `f` is indirect (§6.10), with one
shared monotype, rather than a generalized function at every call:

```i
calls = () {
	f = {
		g = (x) { Nil }
		g
	}
	f("a")
	f("b")
	Nil
}
```

Four ignored Rust tests use the same generator: 2000 literal clauses,
2500 and 5000 calls, a 1000-literal alias used by 100 functions, and a
50 × 50 tuple match. A debug-only solver counter records the peak count
of concrete bounds for every head, per variable and per side, at every
insertion and copied-bound installation. Each test requires a maximum of
two. The calls test also checks that the binding is indirect and that all
N literal lower bounds reached its one shared parameter. The timing
script checks its release growth; the ignored tests print debug check
times without treating them as the release timing budget.

The scalar-row index has a separate oracle test against the full matrix,
including finite/cofinite/mixed/unknown/empty subjects, guards (only
unguarded rows enter the index), literal choices, prefixes, complements,
structural queries and a closing wildcard. Nonliteral queries retain the
ordinary matrix. The bulk finite join has an independent infinite-domain
model over every three-way combination of the four-element universe and
its outside value. The normalization walk borrows its input; it retains
K1's invariant/environment polarity condition exactly.

## Follow-up T1-P1: baseline generalized-call scaling

Open, outside T1's literal-bound work. Literal-types §9 says “calls ... is
already about 13 times slower when n doubles in the base” and “it gets
its own ticket.” Reproduce the prototype shape with a generalized local
identity, a list result, and 250/500/1000 distinct arguments:

```i
calls = () {
	id = (x) { [x] }
	[id(0), id(1), id(2)]
}
```

Expand the last list for each size; compare the supplied pre-T1 compiler
with T1. Preserve principal schemes and constraint consistency. This
reproduction is distinct from the required monomorphic 5000-call fusion
gate. The note's 13× figure is its measurement, not a result reproduced
by this lane. No change to generalization is part of T1.


## Final measured result

The final compatible-baseline gate passes: std +8.401%; keys at 1000
1.544× before and 1.170× on doubling; pick at 1000 0.194× before and
1.800× on doubling; monomorphic calls 1.797× on doubling. Every numerical
limit and the independent quiet-window check passes, with all four
ignored counter stresses green. The accepted experiment is attempt 18
under `target/t1/continued/final-verified-performance-18`; earlier noisy
windows are retained and rejected, not averaged into that result.
