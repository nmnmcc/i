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
An optional `--cpu N` pins both binaries equally. The coordinator can use
this with a quiet CPU/window. The supplied new-syntax baseline is
`/home/agent/Programming/i/.rd/cache/bin/i-1df2fe0`. It checks current std.
The note's 1865366 compiler predates the accepted syntax, so the supplied
new-syntax baseline is a substitute for that old revision, explicitly
identified in reports. The actual pre-stage-7 binary is separately kept
at `target/t1/continued/i-before-stage7` for the corpus comparison.

`gen-stress.py DIRECTORY N PART` makes keys, pick, calls, alias or nested.
The timed keys/pick sizes are 250/500/1000. Pick has a catch-all and a
written String input so both compilers accept exactly the same program.
The calls sizes are 2500/5000. The local `f` is indirect (§6.10), with one
shared monotype, rather than a generalized function at every call:

```i
calls = () {
    f = { g = (x) { Nil }
        g }
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
