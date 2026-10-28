# WasmGC backend

`compile` lowers checked Gleam modules to the serializable IR in `ir.rs`, keeps
definitions reachable from entry points and exports, and emits Core Wasm with
`wasm-encoder`. `wasmparser` validates every completed artifact. The compiler has
no execution-engine dependency.

Program functions, patterns, constants, closures, public ABI wrappers, and source
metadata are encoded directly in `binary/`. The invariant runtime is maintained
as WAT, assembled once per compiler process, and linked as cached binary sections.
When a program introduces foreign imports, the linker adjusts runtime function
indices. Runtime code is reused without instruction decoding otherwise.

All language functions share a typed function-reference signature accepting an
environment and argument array. Generic values use `eqref`; small integers use
`i31`, larger integers use signed 32-bit limb arrays. Records, lists, environments,
UTF-8 strings, and bit arrays use GC arrays and structures. Tail-position calls
use Wasm tail-call instructions. Host objects use a separate `externref` wrapper.
Linear memory is used only for the copying host ABI and allocator bookkeeping.

`compile_cached` stores reachable definitions per module. Its versioned key
includes typed definitions, dependency interfaces, constructor identities,
compiler fingerprint, and ABI schema. Canonical binary hashing accepts all AST
map keys; bitcode payloads preserve floating-point bits and nested IR. Checksums,
atomic replacement, and best-effort reads make invalid cache entries misses.

The `wleam.abi` custom section describes host imports, exports, entry points, and
tests. `wleam.source` and qualified Wasm function names retain source locations;
explicit panics and failed assertions include one-based line and UTF-16 column.
The `wleam:runtime/v1` intrinsic namespace uses the internal boxed convention,
while ordinary `@external(wasm, ...)` declarations use the public copying ABI.

Tests cover IR lowering and caching, actual source-to-Wasm execution, numerical
oracles, bit-array encodings, deep structural operations, ABI ownership, and
direct binary linking. Run them through the repository development scripts or
the pinned Cargo toolchain with Node and Wasmtime available in Devenv.
