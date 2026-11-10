# Testing the checker

Run the existing gates through the shared heavy slot in redesign worktrees:

```sh
ILANG_BROWSER_PORT=4187 /home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i ./scripts/dev-verify
```

The spec suite remains the language/runtime runner (`cargo test -p ilang-cli
--test spec --locked`). It checks both optimization levels and stop/output
headers. Checker fixtures remain the source of shown-type and diagnostic
baselines. `tests/review` adds provenance and explicit known defects to projects
recovered from temporary independent reviews. Its runner calls the same public
`check --types`, `test`, and `run` commands; it does not implement another host
or type checker. A missing expectation or empty selection is an error.

```sh
scripts/review-corpus run --filter review-t2
scripts/test-generation --mode short
scripts/test-generation --mode long --seed 412 --count 180 --output generation.json
```

Generated tests combine the T1 finite-domain model, literal/value-type/guard
interactions, T2 alias dependencies, and K4's equal-argument service family.
They bound each checker/run child, compare O0/O2 outputs, permute only pure
declarations, and remove a redundant helper annotation. They never permute
clauses or effectful initializers: spec §2.5 gives those a meaningful order.
Finite coverage counts only unguarded rows (§7.4); execution uses the first
matching row (§6.12). Literal/value sets follow §§4.2–4.3. A difference caused by
removing an expected type is not universally a bug (§4.8); the chosen helper
annotation has no such role. Failures preserve source, seed, model and observed
output under `tests/review/generated-failures`. Clause families are minimized
by deletion with model recomputation; other families state their limitation.

Captured outputs are observations, not semantic oracles. In particular,
agreement between O0 and O2 can preserve a shared checker defect. Requirements
are separate from `expected.json`, with source sentences, and known failures
must be promoted when their required result is achieved. E0520 observations
are marked for W520's error-to-warning transition. Future record cases remain
held for T3 rather than becoming passing parser-error tests.

Regeneration is explicit and belongs in the final `Regenerate derived files`
commit. `UPDATE=1 scripts/review-corpus import` reimports review sources;
`UPDATE=1 scripts/review-corpus run` captures current outputs. Review semantic
requirements independently before accepting a new output. Historical tools
are preserved unchanged and are not executed: some refer to other lanes or
compiler mutations. Reimporting is a recovery tool, not part of normal CI.

Measure checker workloads separately from the runtime `scripts/bench`:

```sh
scripts/checker-bench --runs 5 --output baseline.json
scripts/checker-bench --binary /path/to/baseline --candidate /path/to/candidate --runs 5 --output interleaved.json
scripts/checker-bench --compare baseline.json candidate.json
```

Use the coordinator's `.rd/cache/quiet` wrapper (it acquires all heavy slots)
for baseline/publication runs. Do not nest `quiet` inside `heavy`. The harness
records the actual command, compiler/binary hashes, complete source inputs,
wall/user/system CPU time, per-child peak RSS, return status, output hashes and
load averages. Timings include startup, package load, parsing, inference and
type display; they are not isolated solver times. Hardware instruction counts
are unavailable and recorded as null. No numeric gate is installed. Record
families are deferred until T3, with its models retained as tools.

Prior art read before implementation (paths under `.rd/ref`):

- `tsrs/bench/release.mjs`: `release`, `command`, `waitQuiet` — pinned inputs,
  preflight, repeated/interleaved measurements, raw data and visible noise.
- `tsrs/tasks/determinism/src/runner/regressions.rs`: `selection`, `run` — reject
  empty selection, check the saved baseline, then vary schedules.
- `tsrs/notes/fuzz-derived-variance.md`: Generator, Findings, Verdict — retain
  counterexamples even after removing an optimization. Its old fuzzer was
  removed upstream; this lane does not claim to run it.
- `tsrs/docs/PORTING.md`: porting contract; `docs/PORT_AUTOMATION.md`: freeze
  oracle hashes, keep failed work; `AGENTS.md`: Reproductions and Performance.
  i takes provenance and repeatability, with its spec as the oracle. TypeScript
  assignability/history, widening and reference behavior do not define i.
- `gleam/compiler-core/src/type_/tests.rs`: `assert_infer`,
  `assert_module_error` — readable source beside inferred types/full errors.
- `go/test/README.md`, `go/test/alias.go`: black-box regressions and errorcheck
  annotations — use the existing runner and name the diagnostic requirement.
- `ghc/testsuite/driver/testlib.py`: `expect_broken`, unexpected-pass reporting;
  `tests/typecheck/should_fail/all.T` — named expected failures remain visible.
  i keeps the required result separately, rather than blessing an observed trap.

No reference implementation was edited, built, cloned or copied. The proposed
method follows `prior-art-compiler.md` findings 2–4 and working-method table.
The note was read at commit `3cc1def`: the ledger names it as landed but this
lane's starting redesign commit `1a8fee2` does not contain it.
