# E4 checker performance repair, 2026-11-15

The e975c5e93 LANDABLE claim is withdrawn. The four wide cases were checker defects, not resource censors. No timeout is spliced in this repair. Full revalidation is pending.

## Cause and repair

`crates/ilang-check/src/types/display.rs`, `Norm::steps` and `Norm::meet_forms`: reverse service-argument variance exposes independent one-sided variables. `steps` eliminated only one per whole-type simplification pass; each pass used a linear scan of prior entries for every service in `meet_forms`. N passes times N squared comparisons gave cubic work. Memory remained about 118 MB.

`perf`, gdb, lldb and samply are unavailable. Temporary timing spans first profiled the 2048-entry checker: the measured solver operations accounted for under a second of its approximately 70 seconds. Narrowed 512-entry spans measured 2165.974 ms in `simplify_steps`, including 994.923 ms in `meet_forms`, 150.577 ms in `steps`, and 2,992,529 recursive `norm` calls. Inclusive recursive times overlap. All probes are removed from production.

The repair indexes service forms by constructor and equal value set, retaining the first output position and input merge order. It batches consecutive extremum substitutions only for variables with one singleton-group occurrence; naming substitutions, shared variables and nontrivial union/intersection members retain the existing barrier. Indexed lookup alone took 13.118 seconds at 2048, so both changes are necessary. This is an implementation optimization: no language rule or diagnostic changes.

The new unit regression asserts that 2048 independent service arguments simplify in one recorded step containing 2048 substitutions, and checks the exact resulting type. It contains no elapsed-time assertion.

## Scaling, pinned CPU 2

All commands run through `/home/agent/Programming/i/.rd/cache/heavy`, with BINARYEN_CORES=1. Each process is fresh. Main is the supplied `docs-bin/i-5e08939f`; old e4 is `docs-bin/i-e4`; fixed is this worktree release compiler. Command: `taskset -c 2 BINARY check --types --offline --project tests/review/cases/review-k4/perf/wide_generic_N`.

| N | Main before wall/user seconds | Old e4 wall/user seconds | Fixed wall/user seconds |
|---:|---:|---:|---:|
| 256 | 0.313/0.230 | 0.713/0.632 | 0.438/0.378 |
| 512 | 0.385/0.322 | 2.040/1.939 | 0.508/0.438 |
| 1024 | 0.666/0.583 | 10.319/10.085 | 0.832/0.700 |
| 2048 | 1.761/1.631 | 70.086/67.940 | 1.660/1.532 |

Old e4 approaches cubic growth (user-time doubling ratios 3.07, 5.20, 6.74). The repair removes this cubic term; whole-checker residual growth is at most quadratic on these sizes, like main. The indexed lookup and independent substitution batch themselves are linear, apart from existing normalization sorting.

## Three-run comparison

The generic cases use `check --types`; both stress cases use `test --opt 0`; every command adds `--offline --project tests/review/cases/CASE` and pins CPU 2. Entries below are wall/user seconds.

| Case | Main, three runs | Fixed, three runs | Median wall ratio |
|---|---|---|---:|
| wide_generic_1024 | 0.749/0.616, 0.702/0.604, 0.760/0.665 | 0.832/0.700, 0.856/0.733, 0.867/0.772 | 1.14x |
| wide_generic_2048 | 1.845/1.686, 1.777/1.630, 1.766/1.623 | 1.660/1.532, 1.674/1.537, 1.614/1.487 | 0.93x |
| stress_wide_1024 | 0.705/0.603, 0.721/0.618, 0.708/0.608 | 2.428/1.239, 0.693/0.604, 0.700/0.596 | 0.99x |
| stress_wide_open_1024 | 0.928/0.820, 0.913/0.782, 0.970/0.835 | 0.880/0.772, 0.819/0.719, 0.870/0.773 | 0.94x |

The first stress test in the fixed binary had a 2.428-second wall/1.239-second user outlier; subsequent fresh processes took 0.693/0.700 seconds. This observation is retained, not discarded. A final repeated comparison follows the production rebuild. Old e4 stress reproductions took 10.288 and 10.530 seconds (user 10.075/10.300).

## Fresh observations and remaining gates

The exact four-case corpus run passed every check/test0/test2 at the original 10/60-second CPU bounds, jobs 1, UPDATE=1, --keep-bounds. This replaces all six previously spliced phases and all four carried phases. Both previously carried check displays now show the actual E3/E4 `run` type rather than the old main error channel; no diagnostic or runtime assertion changes. Full 1973-case preservation audit and repeated landing gates are pending.

## Commands so far

- H `python3 -u target/e4-perf/measure.py`: all eight baseline scaling checks exit 0; raw records in `target/e4-perf/before-scaling.json`.
- H cargo release builds for timing probes 1–5: passed; probe logs under `target/e4-perf/profile*`. The first probe needed periodic output because CLI exits without main-thread TLS destruction; corrected in the next probe.
- H pinned instrumented checks at 2048 and 512: passed; timing spans identify the loops described above.
- H release build with indexed lookup only: passed. Initial `/usr/bin/time` launch failed because that executable is absent; reran using Python wait4/resource counters, check passed in 13.118s.
- H `cargo test -p ilang-check --lib --locked independent_service_arguments_are_simplified_in_one_step`: 1 passed, 84 filtered; 0.02 seconds.
- H release build with both repairs: passed.
- H `python3 -u target/e4-perf/after-measure.py`: all 30 measured processes exit 0; commands, wall/user/system/RSS recorded in `target/e4-perf/after-measurements.json`.
- H `scripts/review-corpus run --binary ./target/release/i --keep-bounds --jobs 1 --case-list target/e4-perf-cases.json`, UPDATE=1 BINARYEN_CORES=1: selected 4, checks exit 0 for all 4, no failures.
- `direnv exec /home/agent/Programming/i rustfmt --edition 2024 crates/ilang-check/src/types/display.rs`: passed.
- `git diff --check`: passed.
