# Native packaging: report

Status: implemented (2026-11-13) on lane/pack, per docs/design/native.md N1, N2, N4,
the “Native packaging after R2/IO” row of §7 and §11. One worker; the multicore rows
are later work.

## What `i build` makes

`i build` (default target `native`) writes one executable, `build/<name>`: the package
name, or the entry `NAME` of `i build NAME`. `--target host` (alias `ilang`), `js`,
`nodejs` and `wasi` still write `build/<name>.wasm`. `i run` and `i test` are unchanged.

The executable is

```text
runner | manifest (JSON) | metadata module | precompiled code | trailer
trailer = payload length u64 | manifest length u64 | SHA-256 of the payload | "ilang-native-v1\0"
```

as in Deno `compile` (payload appended to a prebuilt runner, `denort`) and Bun
`--compile`. The runner (`crates/ilang-runner`, binary `i-runner`) has Wasmtime's
runtime without a compiler, the standard host services of `i run` and the reactor
(`ilang_host::block_on`). `crates/ilang-cli/build.rs` builds it with a separate Cargo
invocation of that package only, into `target/runner`, so that feature unification
cannot add Cranelift; `i` carries it with `include_bytes!` (`ilang_cli::RUNNER`).

`ilang_host::native` holds both sides:

- `package` (feature `compiler`) checks the program entry and N4, then
  `Engine::precompile_module` with the packaging engine;
- `load`/`run` (runtime only) read the trailer from the end of the executable, check
  magic, lengths, the payload digest, the manifest format and the contract, and only
  then call the unsafe `Module::deserialize`, which checks Wasmtime's own compiler
  settings, tunables, features and CPU flags again.

The engine configuration is factored: `runtime_config` (wasm features, stacks,
collector, GC heap) is shared by `Program::from_bytes` (in-process, unchanged
behaviour), the packaging compiler and the runner. Packaging adds an explicit target
triple (so no CPU features of the building machine are inferred) and the baseline
flags. The contract string, recorded in the manifest and compared by the runner, is

```text
ilang-host 0.1.0; abi 3; wasmtime 50.0.0; target <host triple>; cpu x86-64-v2;
collector copying; gc heap 268435456; wasm stack …; thread stack …; epochs off; fuel off
```

The manifest also binds the trusted binding plan to the module digest (`with_binding_plan`
validates it) and the code to its digest. The runner grants the standard services to the
declarations of the trusted library package exactly as `i run` does.

## Measurements

Measured 2026-11-13 on x86_64-unknown-linux-gnu (16 cores, load average about 30 from
other builds, so times are upper bounds), release profile (thin LTO, one codegen unit,
debug info stripped, symbols kept), `--opt 2`.

| Executable | Wasm module (bytes) | Payload (bytes) | Executable size (bytes) |
|---|---|---|---|
| runner alone (`i-runner`) | — | — | 7,136,368 |
| examples/hello | 17,167 | 249,992 | 7,386,360 |
| examples/cli-log | 99,440 | 1,173,783 | 8,310,151 |
| examples/echo | 227,587 | 2,630,766 | 9,767,134 |
| examples/http-get | (host target has no HTTP provider) | 4,761,893 | 11,898,261 |

The payload is mostly precompiled machine code: roughly 10–20 times the optimized Wasm
size. `i` itself is 62,348,656 bytes, of which the embedded runner is 7.1 MB.

Start-up (20 runs each, empty environment, wall time from spawn to exit):

| Command | Median | Minimum |
|---|---|---|
| bare runner (rejects: no payload) | 4.1 ms | 1.5 ms |
| examples/hello executable | 5.6 ms | 2.8 ms |
| examples/cli-log executable (parses arguments, logs) | 14.5 ms | 6.9 ms |
| `i run --project examples/hello` (compiles first) | about 1,000 ms | — |

The executables are dynamically linked against the C library of the building machine
(here a Nix store glibc path), as `i` is; a static or older-glibc runner is part of the
open distribution question.

## Decisions

- Output name: `build/<package name>`, or `build/<NAME>` for `i build NAME`, so that two
  entries do not overwrite each other; Wasm targets keep `build/<package name>.wasm`.
- The payload digest detects damage and truncation; it is not a signature. Signing and
  cross-target distribution stay open (native.md §7); `i build --help` says that only
  the host triple is built.
- CPU baseline: x86-64-v2 on x86-64 (SSE3, SSSE3, SSE4.1, SSE4.2, POPCNT), the base
  architecture elsewhere; Wasmtime refuses the code at load on an older CPU.
- The collector and GC heap come from `ILANG_GC`/`ILANG_GC_HEAP` at build time and are
  recorded in the manifest; the runner ignores those variables.
- N4 at build time: every import the module requires must be one the runner's standard
  host options give and grant; otherwise the build fails listing each namespace and label
  and suggests `--target host`. A service replaced by ordinary i code needs no import.
- Exit status as `i run`: 0 when the entry returns, 1 with the stop or error message.
- The runner reads only the payload and trailer of its file, not the whole executable.
