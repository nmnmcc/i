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
ilang-host 0.1.0; abi 3; wasmtime 50.0.0; target x86_64-unknown-linux-musl; cpu x86-64-v2;
collector copying; gc heap 268435456; wasm stack …; thread stack …; epochs off; fuel off
```

The manifest also binds the trusted binding plan to the module digest (`with_binding_plan`
validates it) and the code to its digest. The runner grants the standard services to the
declarations of the trusted library package exactly as `i run` does.

## Measurements

Measured 2026-11-13 on x86_64 Linux (16 cores shared with other builds; load average
about 30 for the first set, about 55 for the static set, so times are upper bounds),
release profile (thin LTO, one codegen unit, debug info stripped, symbols kept),
`--opt 2`.

### Static runner (current: `x86_64-unknown-linux-musl`, static-pie)

`file`: "ELF 64-bit LSB pie executable, x86-64, static-pie linked"; `ldd`: "statically
linked"; no `PT_INTERP`, no `DT_NEEDED` (checked by `packages_are_statically_linked`).

| Executable | Wasm module (bytes) | Executable size (bytes) |
|---|---|---|
| runner alone (`i-runner`) | — | 6,911,408 |
| examples/hello | 17,167 | 7,161,401 |
| examples/cli-log | 99,440 | 8,085,192 |
| examples/echo | 227,587 | 9,542,175 |
| examples/http-get | (host target has no HTTP provider) | 11,673,302 |

The payload (executable minus runner) is the same as before: 250 KB for hello, 4.76 MB
for http-get; it is mostly machine code, roughly 10–20 times the optimized Wasm.

Start-up (20 runs each, empty environment, wall time from spawn to exit, load ~55):

| Command | Median | Minimum |
|---|---|---|
| bare runner (rejects: no payload) | 1.0 ms | 0.5 ms |
| examples/hello executable | 10.4 ms | 4.7 ms |
| examples/cli-log executable | 30.2 ms | 16.4 ms |

### First, dynamically linked runner (glibc, superseded)

Runner 7,136,368 bytes; hello 7,386,360; cli-log 8,310,151; echo 9,767,134; http-get
11,898,261. Start-up at load ~30: hello median 5.6 ms (min 2.8), cli-log 14.5 ms (min
6.9), bare runner 4.1 ms (min 1.5). `i run --project examples/hello` (compiles first)
takes about 1,000 ms. `i` itself is about 62 MB, of which the embedded runner is 6.9 MB.

The static minimums are of the same order as the dynamic ones; the medians were taken
at almost twice the load. musl's allocator is known to be slower than glibc's for
allocation-heavy hosts; a measured comparison on a quiet machine (and, if needed, a
bundled allocator such as mimalloc, as many musl Rust tools ship) is follow-up work.

## Static linking

Owner decision: the executables `i build` writes are statically linked. As Deno and
Bun ship a prebuilt runner per target, and Go and Zig link statically, `build.rs` builds
`i-runner` for `<arch>-unknown-linux-musl` on Linux (`rust-toolchain.toml` adds the
x86-64 musl target; CI's `devenv shell` gets it from there). Rust's musl targets link
musl statically by default (`crt-static`), and produce a static PIE.

- Wasmtime supports musl (its signal-based trap handling uses `sigaction`/`ucontext`,
  which musl provides); every packaged example runs, including traps and stops.
- TLS is rustls with ring; ring's C and assembly are compiled for the musl target by the
  ordinary C compiler, with no system library. Root certificates are read from the
  system's certificate files at run time (rustls-native-certs), not linked.
- Name resolution: `i/net` resolves through the standard library's `getaddrinfo`,
  which in musl reads `/etc/hosts` and `/etc/resolv.conf` itself and does not load
  glibc's NSS modules; the http-get example resolves `example.com` this way. (glibc
  `+crt-static` was rejected: its `getaddrinfo` needs NSS shared objects at run time.)
- The AOT code is compiled for the runner's triple (`ILANG_RUNNER_TARGET`,
  `ilang_cli::RUNNER_TARGET`), not for `i`'s own `x86_64-unknown-linux-gnu`; the
  contract names that triple, and the runner refuses any other.
- aarch64 Linux: add `aarch64-unknown-linux-musl` to `rust-toolchain.toml` on that
  machine (build.rs already selects `<arch>-unknown-linux-musl`); ring and Wasmtime
  support it. Nothing else changes.
- macOS cannot be fully static (Apple supports only dynamic linking to libSystem).
  There `build.rs` builds the runner for `i`'s own target, and the rule is "depends
  only on system libraries" (libSystem and the Security framework that
  rustls-native-certs uses); a test like `packages_are_statically_linked` would check
  `otool -L` lists only `/usr/lib/` and `/System/Library/` paths.
- Cross-target packages (one `i` making executables for other systems) still need
  per-target runners, as Deno downloads `denort` per target; still open.

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
- `--target host` (ilang) now has a host cell for the standard host services in
  lib/provider-plan.toml (Io, Clock, Stdio, Fs, Net, Tls and the process services): an
  embedding host of ABI v3 gives them as `i run` does, so `i build --target host` of
  examples/hello writes its module again. This table is expected to be replaced by the
  fixed platform-service table; packaging does not depend on it.
- The runner is a separate package built by a nested Cargo invocation (as Substrate's
  wasm-builder does), and `i` embeds it; `i build` needs no other file next to `i`.
