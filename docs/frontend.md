# Frontend and semantic caches

Wleam vendors Gleam v1.19.2's parser, type inference, exhaustiveness checker,
pattern decision trees, diagnostics, formatter and language-server engine. The
baseline commit and licences are recorded in [UPSTREAM.md](../UPSTREAM.md).
The vendored Rust crates form a separate workspace under `vendor/gleam`.

The frontend has one target, `wasm`. An omitted `target` in `gleam.toml` selects
it. Erlang and JavaScript targets and configuration tables are rejected.
External functions use `@external(wasm, "module", "function")`; module and
function strings are Wasm import names and do not inherit JavaScript identifier
restrictions. Argument and return annotations remain mandatory on externals.

The old code generators, BEAM/native-file copying, platform subprocesses,
online package client, package publication and documentation generators are
removed. The former Hex client crate supplies only the baseline version-range
parser under the package name `gleam-version`. Legacy dependency declarations
remain representable so project loading can issue migration diagnostics.

## Project loading

`wleam_frontend::Project::load(path)` opens a project directory or `gleam.toml`.
`Project::check(&CheckOptions)` resolves local path dependencies, parses and
checks modules, and returns a `CheckedProject`. Its shared typed modules are in
dependency order. Its source map retains paths, source text, package ownership
and test-module origins for code generation and diagnostics.

Dependencies must use `{ path = "..." }`. Resolution checks package names,
duplicate names from different directories, cycles, duplicate dependency
sections and unsupported online requirements. Development dependencies apply
only to the root package when tests are requested. The bundled `gleam_stdlib`
is an implicit dependency; an explicit standard-library path overrides it.
`WLEAM_STDLIB` selects a development copy. Installed binaries can find an
adjacent distribution or materialise their embedded sources in the user cache.

`CheckOptions` controls test inclusion, cache use and open-buffer source
overrides. `CheckedProject.exports` contains the configured `[wasm].exports`
entries, written as `module.function`; checking verifies that each entry names
an existing public function. The backend owns entry-point and foreign ABI
validation.

## Cache behaviour

Typed modules and their importable interfaces are cached under
`build/.wleam/frontend-v1`. A reusable `Project` also retains them in memory.
Cache keys include source contents, source paths, package configuration,
dependency-interface hashes, compiler revision and ABI schema. Timestamps do
not determine semantic validity. A payload checksum is verified before decoding
cached typed trees. Corrupt or incompatible entries are ignored.

Interface hashes include public types, functions, constructor information,
record accessors and constant values. They exclude source positions and
function bodies, and normalise inferred generic-variable identifiers. The
encoding distinguishes compiler structures from user mappings, so names such as
`location`, `start` and `end` retain their meaning. Editing
a function body therefore checks that module while permitting callers with an
unchanged imported interface to reuse their typed trees. Public type or
constant changes invalidate callers. Cached type-variable identifier ranges
are reserved before new inference to prevent identifiers from colliding.

`CheckedProject.stats` exposes parsed/checked module counts, memory and disk
hits, and cache writes. Use these counters alongside wall-clock measurements
to distinguish successful reuse from merely faster filesystem reads.

The LSP retains the upstream semantic engine and checking-only project compiler.
It uses local dependency manifests and source fingerprints, preserving semantic
editor operations without invoking either legacy backend.

## Checks

The root workspace tests cover project resolution, source overrides and
semantic-cache reuse. The nested workspace retains upstream parser, formatter,
type inference, exhaustiveness, metadata, version-range and editor tests:

```sh
cargo test -p wleam-frontend
cargo test --manifest-path vendor/gleam/Cargo.toml --workspace
```

Migration fixtures exercise Wasm attributes and explicit rejection of legacy
targets. Snapshot changes record the altered target and diagnostics while
retaining language-semantics assertions.
