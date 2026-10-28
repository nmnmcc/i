# Wleam runner

`wleam-runner` is the isolated Wasmtime process used by `wleam run` and
`wleam test`. The compiler and language server do not link Wasmtime.

```sh
wleam-runner build/app.wasm
wleam-runner build/app.wasm --invoke example
wleam-runner build/app.wasm --all-tests
wleam-runner build/app.wasm --test module.test_one --test module.test_two --json
wleam-runner build/app.wasm --list-tests --json
```

The default entry comes from the module's `wleam.abi` custom section, falling
back to `_start` for a module without an entry. `__wleam_init`, when exported,
runs after instantiation and before the entry or test. Entry and test exports
must have no parameters or results. Use the `wleam-host` SDK for typed calls
and custom foreign imports.

Each test runs in a fresh instance of the already compiled module. A trap fails
that test and execution continues with the next test. This isolates mutable
guest state, including state partially changed by a failing assertion.
`--list-tests` reads metadata directly without compiling or instantiating the
module.

Use the release runner for execution benchmarks and large test suites:
`cargo build --release -p wleam-runner`. A debug build also leaves the JIT
compiler's own Rust code unoptimized, which increases module loading time.

Ordinary execution streams guest stdout and stderr. Test execution captures
them and prints them with the test's result. `--json` captures both streams
inside one report and produces no guest text outside the JSON:

```json
{
  "module": "build/app.wasm",
  "mode": "test",
  "passed": 1,
  "failed": 0,
  "results": [{
    "name": "module.test_one",
    "status": "passed",
    "duration_ms": 1.25,
    "stdout": "",
    "stderr": ""
  }]
}
```

A failed case also contains an `error` string. A failure before execution
produces a report with `mode: "error"`, an `error` string, and empty `results`.
Exit status is zero when execution succeeds or every selected test passes,
and nonzero on errors or failed tests. Selecting the same test more than once
runs it once, preserving selection order. `--all-tests` with no discovered
tests reports zero tests successfully.

The built-in imports are `wleam:host/v1.write`, `random`, `panic`, and
`reference_id`. The last assigns stable per-instance identities to opaque host
references for guest equality and hashing, with identity zero for null.
Output channels are 1 for stdout and 2 for stderr. Strings are UTF-8; panic
messages become Wasmtime errors. Custom imports require an embedding program
using `wleam-host`; a missing import is reported during instantiation.
