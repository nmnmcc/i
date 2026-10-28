# Wleam compatibility and validation

Wleam's language baseline is Gleam **v1.19.2**
(`ed41eace2dd89054eb2d744c33925261d1eff32e`); its standard library baseline is
gleam_stdlib **v1.0.5** (`5b0642a77a38f984db5913548b84668652ca8b5c`).
Source provenance and licenses are recorded in [UPSTREAM.md](../UPSTREAM.md).
The preserved test inputs have a checked SHA-256 inventory in
[`upstream-manifest.json`](../tests/conformance/upstream-manifest.json).

The compatibility contract is one Core Wasm module requiring WasmGC, typed
function references and tail calls. Native compilation, formatting and LSP do
not require Wasmtime. The runner and browser adapter implement the synchronous
host ABI. Applications use the package module's zero-argument `main`; libraries
use `build --lib` with explicit `[wasm].exports`.

The following coverage is required by the test harness. A row describes the
acceptance test, not a claim that an unexecuted build passed it. Actual results
are recorded in `target/conformance/report.json` and browser reports.

| Behavior | Acceptance evidence |
| --- | --- |
| Closures, higher-order functions, generics, `use`, pipes and labels | `functions` and the complete upstream language corpus |
| Records, opaque types, updates, exhaustive patterns and guards | `records`, `patterns`, `error_exhaustive`, upstream language corpus |
| Arbitrary-precision integers, signed division, remainder and bit operations | `arithmetic`, `bigint`, stdlib integer tests and additional integer regressions |
| Tail calls and recursion | `recursion`, upstream language stress tests, stdlib list stress tests |
| Bit offsets, integer/float segments, UTF encodings and dynamic sizes | `bit_arrays`, upstream language bit-array suites, stdlib bit-array tests |
| Structural equality, hashing, persistent dictionaries and hash collisions | `collections`, stdlib dictionary tests, HAMT regression corpus, production runtime WAT tests |
| Strings, graphemes and case conversion | `unicode`, full stdlib string tests, generated Unicode 17 vector corpus |
| Dynamic decoding, URI parsing and decimal float conversion | Full stdlib corpus and additional stdlib regressions |
| Local dependency resolution | `local_packages` and frontend manifest/checker integration tests |
| Source errors and runtime failures | Compile-error, assert/panic/todo and legacy-target migration fixtures |
| Test discovery | Exact inventories: 490 core + 1,520 stdlib + 120 Unicode + 250 additional regressions |
| Cache invalidation and formatting | Frontend checker and CLI integration tests; LSP formatting tests |
| Cross-module editor operations | LSP protocol tests, including unsaved source and local dependencies |
| Same module on multiple hosts | Wasmtime results followed by Chromium, Firefox and WebKit on the copied Wasm artifact |
| Cargo/Blaze agreement | Byte-for-byte artifact comparison, diagnostic and execution checks with both compiler builds |

The compiler uses arbitrary-precision `Int` values. Integer division truncates
toward zero; integer division and remainder by zero return zero. Non-finite
floating-point results are runtime errors. The small oracle programs compare
deterministic values and output with the pinned Erlang backend.

`exp`, `log` and `power` use the same implementation on all Wasm hosts. Their
last bit may differ from Erlang's platform math library. The port's 60,042-case
reference comparison observed a maximum difference of one ULP; this is a measured
sample result, not an error bound for every input. See the
[stdlib notes](../stdlib/README.md). Integer and decimal-conversion tests retain
exact comparisons.

Unicode data is fixed at **17.0.0**. The generated suite includes all 766 official
grapheme-break vectors, 3,037 default scalar casing cases and 18 contextual casing
cases, grouped into 120 test functions. These tests assert actual strings and
cluster boundaries. Unicode 17 contextual case conversion can differ from the
pinned Erlang host's string implementation: for example, final Greek sigma in
`"ΟΣ"` lowercases to `"ος"` under the Unicode contract, while Erlang 28's
`string:lowercase` returns `"οσ"`. That behavior is checked against the Unicode
data, rather than treating the host's casing result as the Unicode oracle.

Upstream migration is explicit:

- The core corpus retains 490 language tests. Two TypeScript output-file copying
  probes are preserved as reference-only cases; the harness checks generated
  Wasm artifacts directly. The
  [core migration inventory](../tests/conformance/projects/upstream_language/migration.json)
  records every source file and adaptation.
- The stdlib corpus has 1,545 original declarations representing 1,538 unique
  tests. Seven duplicate host branches are consolidated. Eighteen probes inspect
  Erlang process/reference/atom/improper-list values or JavaScript object
  internals; the Wasm ABI exposes these only as opaque host references. Their
  originals and individual reasons remain in the
  [stdlib migration inventory](../stdlib/test/migration.json). Legacy foreign
  declarations are required to produce migration diagnostics. The remaining
  1,520 tests retain their assertions and execute on Wasm.
- Active test functions end in `Nil` to meet the test discovery contract. This
  adaptation preserves each preceding expression and assertion.

Foreign functions use `@external(wasm, "module", "function")`. The synchronous
ABI transports primitive values, explicit sign/limb integers, copied UTF-8 and
bit buffers, and opaque `externref` values. Complex records and collections need
explicit application encoding. Allocation/free and reference lifetime tests are
part of the host-adapter suites.

Hex resolution, old-platform code generation and publication, WASI Component
Model, compilation inside the browser and asynchronous foreign calls are outside
this version's scope. Old target configuration and external declarations fail
with migration diagnostics. There are no expected-success placeholders for core
language or standard library behavior: unsupported lowering, missing imports,
empty test results and compiler crashes fail conformance.

Run [`scripts/dev-verify`](../scripts/dev-verify) in the pinned Devenv environment
for the complete required checks. See the
[conformance guide](../tests/conformance/README.md) for focused execution and the
[performance guide](performance.md) for actual measurement procedures. A focused
test report or benchmark does not replace the complete acceptance run.
