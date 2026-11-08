# Measuring iLang performance

The benchmark harness keeps two measurements separate:

- **Rust development:** rebuilding and testing the iLang implementation with Cargo or Blaze after an edit.
- **i compilation:** checking, building and testing an i project using an already-built iLang executable.

No speedup is assumed. A successful report records actual samples; a failed or timed-out command stops the suite, preserves its logs, marks the report unsuccessful and returns a nonzero exit status.

## Measured baseline: 2026-10-29

Blaze did not improve edit or full-test latency on this frozen iLang workload. Unchanged-build times overlapped; the measured function and type edits took about 1.19 and 1.24 times Cargo's duration with Blaze. Both engines passed all rebuild checks and executed the same 124 tests in every test cycle, including the generated revision/layout probe.

Measurements ran on Linux 6.18.56, x86_64, an AMD EPYC-Genoa processor with 16 visible CPUs and 62.8 GiB visible memory. The final Rust run covered 28 commands between 19:15:04 and 19:25:50 UTC on October 28 (October 29 in Hong Kong); the release-language run followed at 19:26 UTC. Other iLang validation jobs and their daemons had stopped. System and dependency-download caches remained populated.

Cargo and rustc were 1.99.1. Blaze identified itself as `0.20261027.0 (5d4f78f)` and used `nightly-2026-10-27`, with the documented upstream cache-path backport `7145b282`. Each engine had a separate fresh project snapshot, target directory and, for Blaze, daemon/JIT cache. The source snapshot was identical before the same generated edits were applied.

| Rust scenario | Samples per engine | Cargo median, seconds | Blaze median, seconds | Blaze / Cargo time |
| --- | ---: | ---: | ---: | ---: |
| Fresh project build | 1 | 32.536 | 81.959 | 2.52 |
| Unchanged build | 3 | 0.123 | 0.125 | 1.01 |
| Function-body edit and build | 3 | 2.130 | 2.536 | 1.19 |
| Shared-type edit and build | 3 | 2.015 | 2.490 | 1.24 |
| First test build/run after application builds | 1 | 35.873 | 216.570 | 6.04 |
| Function-body edit and full test cycle | 3 | 23.451 | 50.983 | 2.17 |

Unchanged-build ranges were 0.119–0.131 seconds for Cargo and 0.124–0.128 seconds for Blaze; their small median difference is not meaningful evidence of a speed difference. Function-edit ranges were 2.074–2.157 and 2.503–3.120 seconds respectively. Full test counts were 124 passed, zero failed, ignored or filtered in all eight test cycles. The full suite includes default workspace members and documentation tests; a package-selected development test is a different workload.

The benchmark exposed a nonexistent `rerun-if-changed` path in the frontend build script. After correction, unchanged Cargo builds stopped recompiling the frontend/LSP/CLI and the observed median fell from 1.884 to 0.123 seconds. The historical report is retained as `target/benchmarks/rust-before-watch-fix.json`; its later samples may overlap correction validation and are not used for the final comparison above.

The release iLang workload contains 32 linked computation modules, a shared record module, an application main and one arithmetic test. Each warm/edit scenario has five samples; cold check/build each have one. All 38 timed commands passed, as did 16 additional direct executions of built artifacts and all six named arithmetic-test executions.

| i scenario | Check median, milliseconds | Build median, milliseconds |
| --- | ---: | ---: |
| Fresh project | 16.004 | 34.583 |
| Unchanged source | 8.505 | 8.453 |
| Leaf function-body edit | 8.903 | 26.548 |
| Shared public type edit | 18.782 | 38.862 |

The first test cycle took 52.224 milliseconds; edited test cycles had a 49.965-millisecond median and a 46.030–51.320-millisecond range. These results describe this small dependency-chain fixture and do not establish throughput for the complete standard library or arbitrary applications.

| Rust engine | Maximum process high-water RSS, MiB | Maximum sampled aggregate RSS, MiB |
| --- | ---: | ---: |
| Cargo | 1,043.0 | 3,495.0 |
| Blaze | 1,747.8 | 21,314.7 |

The sampled aggregate includes persistent Blaze workers and may count shared pages more than once; it is not physical-memory consumption. It cannot be compared directly with a single process's RSS. The primary language run's 38 commands all hit the Python launcher's inherited 29.742-MiB RSS floor, and their short lifetimes escaped `/proc` discovery. That value does not establish iLang's peak memory.

A separate 38-command run used GNU Time 1.10 from `/nix/store/6vrl8gs1971k9p913mq5y041mpscvbss-time-1.10/bin/time` to measure the child's resource usage independently. It used identical compiler, runner and standard-library inputs and passed the same result checks. Maximum child process high-water RSS was **11.73 MiB for checking, 15.32 MiB for building and 24.65 MiB for test commands**. Test accounting includes the executor's child usage but is not a concurrent-process sum. The helper's startup cost is present only in this supplementary run's timings; the latency tables above use the original runs.

The RSS observer consumed substantial CPU during the Rust run. Initial/final process discovery happens outside the timed interval, while periodic discovery and sampling can compete with a running command. This overhead was not independently quantified or subtracted. These are instrumented measurements on a shared development host, with small sample counts; the close unchanged-build medians should not be ranked.

The first Nix realization of Blaze, including its compatibility backport, was **not timed reliably**. Its cost is therefore unmeasured. The 81.959-second figure above measures first project loading/building with an already-installed distribution and an empty private project/JIT cache. Earlier environment observations, whose caches differed, are recorded separately in [initial environment validation](development.md#initial-environment-validation).

Raw samples, exact commands, validation evidence and logs are retained locally in `target/benchmarks/rust-final.json`, `target/benchmarks/gleam-final.json` and `target/benchmarks/gleam-memory.json`. The SHA-256 identities are:

```text
Rust benchmark source snapshot: 4533b14950b12ae1329be860df4a1c288bbdecec1aa6f0b8eb0ec4761f2b487c
Release iLang executable:       6a080f4e337f2c90a408f6d90738b17667da3adb5c53c3ed1a3b1532392ca8c0
Release ilang-runner:           73689bc7779b2de0f46261f2de75c2c1f8f3b4f4fd346d11ee6ce4165b7a9356
Standard-library snapshot:     6551e0857ddcfc4497de8b579eaa08eed3f5432cca23a0843bde829f0d93e8dc
```

## Rust development measurements

Enter the pinned Devenv environment, prepare the committed Cargo lockfile and fetch dependencies before comparing engines. Tool installation, Nix realization and the first Blaze toolchain installation are preparation costs, outside these command timings.

```sh
cargo fetch --locked
python3 scripts/benchmark.py rust --rounds 5 \
  --output target/benchmarks/rust.json
```

The harness freezes one source snapshot, then copies it into separate temporary Cargo and Blaze workspaces. It excludes build outputs, Git metadata, environment caches and dependency installation directories. Both workspaces start with identical source and no project build cache. The source checkout is never edited.

Each engine measures:

1. One cold build of the `i` executable.
2. Repeated builds with unchanged source.
3. Builds after changing a reachable function body.
4. Builds after changing the layout of a type shared by two modules.
5. The first test build/run.
6. Test cycles after further function-body edits.

The edit probes are small generated modules called from the copied CLI entry point. They exercise the actual iLang dependency graph and executable rebuild, while making edits repeatable and leaving the implementation's behavior intact. They are controlled compiler-edit probes, not a claim that every real compiler change costs the same amount. `black_box` keeps the changed values and layouts observable to the optimizer. After every build, an untimed executable check asserts that the function revision and type size match runtime expectations; the generated unit test makes the same assertions. A stale executable therefore fails validation instead of producing a misleading fast sample.

Cargo uses `--locked` and `target/cargo`; Blaze uses its default target directory, a unique `BLAZE_HOME`, and a matching private `RA_JIT_CACHE`. The harness stops only its own Blaze daemon at the end. Dependency resolution and lockfile changes belong to Cargo. Blaze's nightly and the project's Rust version are captured separately in the report.

Every test sample must contain libtest summaries with a positive passed count, zero failed/ignored/measured/filtered tests, and the generated probe's named success. Counts must remain equal across all rounds and both engines; the first engine's count is the oracle (Cargo by default). Blaze's affected-test environment setting is cleared. An exit status of zero without this evidence fails the benchmark.

To measure one engine, use `--engines cargo` or `--engines blaze`. This produces a single-engine report, not a Cargo/Blaze comparison. `--cargo` and `--blaze` select explicit executable paths when needed.

## i compilation measurements

Use the same validated compiler and runner binaries throughout a comparison. For representative release timings, first run the release build, then:

```sh
python3 scripts/benchmark.py i \
  --compiler target/cargo/release/i \
  --runner target/cargo/release/ilang-runner \
  --modules 32 --rounds 5 \
  --output target/benchmarks/i.json
```

The harness snapshots the compiler and runner binaries and matching standard library before timing, so a concurrent development rebuild cannot change the executable used halfway through the run. Standard library selection is `--stdlib`, then `ILANG_STDLIB`, then `SOURCE/stdlib`; its content digest is recorded together with both binary digests.

It creates a disposable i project with a module dependency chain and a shared public record. It measures cold checking and cold building in separate fresh projects; neither benefits from the other's frontend cache. For each operation it then measures unchanged source, a leaf function-body edit and a shared public type change. The type change updates the record constructor's caller as well, keeping the program valid.

The final test cycles execute an assertion through the dependency chain. Function-edit test samples include updating the expected result. Runner JSON must report exactly the generated arithmetic test passing; a nonzero exit, missing test or empty success report fails validation.

After each build, an untimed runner invocation executes the artifact directly and checks its echoed arithmetic result. It does not invoke the compiler again. Function-body build samples therefore change only the leaf module while still rejecting stale generated code. The first test cycle is named `first-test`: its test module is new, while earlier application builds have already warmed the project cache. The Rust suite uses the same label and distinction.

`--modules` changes the dependency-chain size. The current source fixture uses arbitrary-precision integer addition, records, cross-module calls and assertions. It is deliberately deterministic. This fixture alone does not characterize Unicode processing, large integer arithmetic or all application workloads; conformance tests cover their correctness independently.

`all` runs both suites and also requires explicit `--compiler` and `--runner` paths. The language suite always uses those binaries, rather than an unvalidated binary from the development measurement.

## Reports and memory accounting

The JSON report contains:

- Schema version, UTC start/end times, platform, CPU identity/count and available host memory/load observations.
- Input parameters, executable versions, the Rust source-tree digest and the measured iLang binary digest.
- Every command's arguments, working directory, duration, exit status and timeout flag.
- Paths to unabridged stdout/stderr logs.
- Per-scenario sample counts, median/minimum/maximum time and memory observations.

There are two distinct memory measurements:

- `command_peak_process_rss_bytes` comes from Unix `wait4`. It is a process high-water mark reported for the command and its accounted-for children, not the sum of all concurrently running processes. Child launch can inherit the launcher's earlier high-water mark. Each sample therefore includes `launcher_peak_rss_bytes_at_spawn`, its measurement method and `command_rss_at_or_below_launcher_floor`; a result at that floor does not establish the command's own peak. Linux uses the current interpreter's `VmHWM` as the floor, since `RUSAGE_SELF` can include a pre-exec ancestor's high-water mark. The floor is not subtracted. Files are hashed incrementally to keep the launcher small. A short Blaze client does not include an already-running daemon's complete heap in this value.
- `sampled_engine_peak_rss_bytes` sums Linux `/proc` RSS observations for processes carrying a unique benchmark-session marker, including Blaze's daemon and workers when they inherit it. It samples at roughly 50 ms and discovers new processes roughly every 250 ms. It may miss brief peaks and double-count shared pages. `null` means no observation was available; it does not mean zero memory. This aggregate is unavailable on platforms without Linux `/proc`.

For a separate memory-focused run, pass `--gnu-time time`. GNU Time's child resource accounting adds `external_command_peak_rss_bytes`, avoiding the Python launcher's inherited floor. The report records the helper's exact path, version and per-command resource log. This is still a process high-water mark, not an aggregate or an existing daemon's memory. Keep these samples separate from ordinary latency results because helper startup is timed as well. The `/proc` observer itself uses CPU, so timings collected with it are not zero-overhead measurements.

Cold means **a fresh project cache and daemon session**, with shared system, Nix and Cargo registry caches retained. A report is not an operating-system cold-cache or network-download benchmark. Cold scenarios run once; `--rounds` controls warm/edit repetitions. Preserve raw reports when comparing results, and report their source/toolchain identities, fixture size and sample counts together with any ratio.

Run comparisons on the same machine under similar load. Do not compare debug and release compiler binaries without naming that difference. Record one-time Nix/Blaze preparation separately from repeated edit latency.

## Runtime observations and limits

Initial native conformance observations included roughly 506 seconds for the standard-library suite, 188 seconds for its dictionary random-operations test and 139 seconds for its million-element shuffle test. These are verification durations, not controlled runtime benchmarks. The dictionary test also copies association lists and performs linear traversals; host-generated random inputs differ between runs. Those durations cannot isolate dictionary implementation cost or establish a Cargo/Blaze runtime difference.

A source audit of the pinned Wasmtime 50.0.0 establishes that its initial GC heap defaults to zero and grows geometrically, while the DRC collector has a separate candidate-root threshold: the greater of 1,024 and twice the number remaining after the previous collection. DRC traces stack roots and sweeps candidate roots; these operations do not scan the entire heap. The relevant pinned crate locations are `wasmtime-50.0.0/src/config.rs:1495`, `wasmtime-50.0.0/src/runtime/store/gc.rs:354`, `wasmtime-internal-cranelift-50.0.0/src/func_environ/gc/drc.rs:157`, and `wasmtime-50.0.0/src/runtime/vm/gc/enabled/drc.rs:538`.

This audit is not a profile and does not establish that increasing the initial heap improves these workloads. The validated runtime configuration is retained. A future controlled experiment should use a fixed deterministic operation sequence and the same Wasm and runner, compare the default heap with 8/32/64 MiB initial heaps, and record collection counts, collection time, elapsed time and peak RSS. Any configuration change must also pass buffer/reference lifetime and GC-pressure regressions.

## Failure handling and inspection

`--timeout` sets a per-command deadline. Timed-out process groups are terminated, partial reports are written and project-specific Blaze sessions are stopped. Temporary source trees are removed on success and failure; `--keep-workdir` retains them for inspecting the measured edits. Output logs remain next to the JSON report.

The temporary parent (`--work-parent`, or the system temporary directory selected by `TMPDIR`) must be outside the source tree. For Blaze runs it must also have no directory component named `target`: the pinned Blaze source watcher treats such paths as immutable. Build output directories may still be named `target`. Reports must be outside the source tree or inside an ignored output directory such as `target/`. These checks prevent recursive snapshots, stale-source timings and mixing previous benchmark output into the measured source.

Use `python3 scripts/benchmark.py --help` for all options. Measurements of iLang should be published only after its correctness checks pass; the presence of a benchmark harness is not evidence that the compiler, Blaze integration or performance targets have passed.
