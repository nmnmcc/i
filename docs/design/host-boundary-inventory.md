# Inventory: the current host-boundary mechanism

Measured at commit `a21eae769` on branch `design/host-boundary`, 2026-11-21.
Read-only: no source file was changed for this inventory. This file is the
size estimate and the deletion checklist for
[`host-boundary.md`](host-boundary.md).

## 0. Headline

The old model is not a thin edge layer. It is a language feature spread over
every layer, both hosts, the ABI, and 150 fixtures:

| Surface | Count |
|---|---:|
| `@external(...)` occurrences, whole repository | **310** |
| `@host(...)` occurrences, whole repository | **182** |
| `@js(...)` occurrences, whole repository | 17 |
| `[wasm]` manifest tables | **22** |
| Library type signatures that carry `Await`/`Owner`/`Root` | **88 exported** (103 total) |
| ...with `Scheduler`/`Runner` added, the implicit set | **106 exported**, 240 occurrences |
| Distinct `@external` namespaces | 32 |
| Distinct `@host` namespaces | 42 |
| Emitter / host files touching `HostCall`/`HostStart`/`HostReply` or import kinds | 14 |

An exact replacement, with no compatibility aliases, is therefore a
multi-crate change plus a corpus migration, not a rename. The two largest
engine surfaces are the checker (`resolve.rs`, `resolve/three.rs`,
`types/infer.rs`, `platform.rs`) and the Wasm lowering (`lower.rs`,
`emit/host.rs`, `emit/wasi.rs`).

## 1. The three attributes in source

### 1.1 `lib/i` and `lib/prelude.i`

`@external` — 147, by namespace:

| Namespace | Files | Count |
|---|---|---:|
| `ilang:core` | `lib/i/core.i:36,40,44,48,52,56,60,64,68,72,76,80,84,88,92,96,100,104,108,112,116,120,124,128,132,136,140,144,148,152` | 30 |
| `i:compiler/syntax/v1` | `lib/i/syntax.i:178,194,197,200,203,206,209,212,256,259,262,265,268,271,293,296,299,302,305,308,311,314,317,320,323,326,329,332,335,338,341,344,347,350,353,356,359,362,365,368,371,374,377,380,481,601,604,607,610,613,616,619,622,625,628,631` | 56 |
| `ilang:request` | `lib/i/request.i:48,51,54,57,60,63,66,69,72,75,112,115,118,121,124,127,151,155,160,163`; `lib/i/task.i:1510,1513` | 22 |
| `ilang:worker` | `lib/i/task.i:1238,1241,1244,1247,1250,1253,1256,1259,1262,1265,1268,1271,1274,1277` | 14 |
| `ilang:big` | `lib/i/big.i:29,33,38,42,46,51,58,61,64,69,72,76` | 12 |
| `ilang:control` | `lib/i/control.i:12,17,24,28,32`; `lib/i/task.i:225` | 6 |
| `ilang:key` | `lib/i/key.i:14,18,21` | 3 |
| `ilang:bits` | `lib/i/bits.i:22,26` | 2 |
| `ilang:resource` | `lib/i/resource.i:9` | 1 |
| `ilang:prelude` | `lib/prelude.i:35` | 1 |

`@host` — 12, every one a standard host service:

| File:line | Namespace |
|---|---|
| `lib/i/clock.i:17` | `i:clock/v1` |
| `lib/i/fetch.i:20` | `i:fetch/v1` |
| `lib/i/fs.i:43` | `i:fs/v1` |
| `lib/i/io.i:10` | `i:io/v1` |
| `lib/i/net.i:42` | `i:net/v1` |
| `lib/i/process.i:54` | `i:arguments/v1` |
| `lib/i/process.i:57` | `i:environment/v1` |
| `lib/i/process.i:60` | `i:working-directory/v1` |
| `lib/i/process.i:63` | `i:entropy/v1` |
| `lib/i/process.i:66` | `i:process/v1` |
| `lib/i/stdio.i:21` | `i:stdio/v1` |
| `lib/i/tls.i:31` | `i:tls/v1` |

`@js` in the library: **0**. (`i/js`, `i/target`, `i/wasi` do not exist as
source files; the `@js` feature is not implemented, see
`docs/design/spec-known-gaps.md:23`.)

### 1.2 `std`, `examples`, `docs/content`

**0 occurrences in all three.** `std/src/*.i` and every example call the
library wrappers only. This is the smallest part of the migration and the part
that proves the intended shape: ordinary user code already never names the
boundary.

### 1.3 Tests and fixtures

| Category | `@external` | `@host` | `@js` |
|---|---:|---:|---:|
| `tests/spec/**` (16 files) | 14 | 57 | 0 |
| `tests/review/**` (24 files + 3 JSON) | 12 | 18 | 0 |
| `tests/browser/**` (5 files) | 8 | 10 | 0 |
| `tests/performance/**` | 2 | 0 | 0 |
| `crates/*/tests/**` (17 files) | 87 | 44 | 2 |
| `tests/r2-performance/intervals.i:3` | 1 | 0 | 0 |

The namespaces are dominated by fixture names (`test:io/v1` 23,
`test:async/v1` 26, `test:kv/v1` 9, `app:a/v1` 4, …), not by runtime
namespaces.

Pinning tests that must be rewritten or replaced:

- `crates/ilang-check/tests/projects/{attr_rules,attr_host,attr_reserved,attr_entry,external_types,wasm_exports,export_hidden}/*` and their `expected.out`.
- `crates/ilang-check/tests/types/kept.{i,out}`, `library.out`.
- `crates/ilang-wasm/tests/codegen.rs:156-231,364-520`.
- `crates/ilang-cli/tests/{commands.rs,package_commands.rs,wasi.rs,fmt_to_safety.rs}` host/export sections.
- `crates/ilang-package/tests/manifest.rs:140-165`.
- `bindings/javascript/test/{host,providers,bindings,wasi}.test.mjs`.
- `tests/browser/host.spec.mjs` and its fixtures.

## 2. Manifest `[wasm]`

22 `i.toml` files, all with `exports`, **none** with `synchronous`:

`crates/ilang-check/tests/projects/entry_dependency/i.toml:8-9`,
`crates/ilang-check/tests/projects/ok_dependency_manifest/u/i.toml:5-6`,
`crates/ilang-check/tests/projects/wasm_exports/i.toml:3-4`,
`crates/ilang-check/tests/types/entry_checks/i.toml:3-16`,
`tests/browser/fixtures/compiled/i.toml:3-4`,
`tests/browser/fixtures/external-fail/i.toml:3-4`,
`tests/browser/fixtures/http-fetch/i.toml:6-7`,
`tests/browser/fixtures/r2/i.toml:3-4`,
`tests/perf/numeric/i.toml:3-4`,
`tests/performance/abi/i.toml:3-4`,
`tests/performance/big-int/i.toml:3-4`,
`tests/r2-performance/i.toml:3-4`,
`tests/review/cases/review-js3/opaque/i.toml:2-3`,
`tests/review/cases/review-num/async/i.toml:2-3`,
`tests/review/cases/review-runtime/pure/i.toml:2-3`,
`tests/review/cases/review-t1/host_export_input/i.toml:2-3`,
`tests/spec/10.2/export_literal/i.toml:5-6`,
`tests/spec/10.2/export_never_param/i.toml:8-9`,
`tests/spec/10.2/export_never_value/i.toml:8-9`,
`tests/spec/10.2/export_twice/i.toml:8-9`,
`tests/spec/2.4/unused_roots/i.toml:5-6`,
`tests/spec/6/t3_record_param_export/i.toml:4-5`.

Reader/validator/consumers: `crates/ilang-package/src/manifest.rs:9-18,70-77,116-141,188-206,299-342,388-394`,
`crates/ilang-package/src/lib.rs:246-247,547-548,640-641`,
`crates/ilang-check/src/resolve.rs:2529-2613`,
`crates/ilang-check/src/resolve/legacy.rs:4723-4790`,
`crates/ilang-wasm/src/lower.rs:773-790`,
`crates/ilang-macro/src/run.rs:89-93`.

## 3. Compiler and ABI implementation

### 3.1 Attribute parsing and checking

- Generic AST attribute: `crates/ilang-syntax/src/ast.rs:213-237`; parser `parse.rs:513-612`; Lisp `rows.rs:958-1031`; printer `print/position.rs:48-54`.
- Built-in attribute contract and E0301–E0309: `crates/ilang-check/src/resolve/three.rs:964-1073` (legacy twin `resolve/legacy.rs:809-918`).
- Service shape E0310: `three.rs:1333-1363` (legacy `legacy.rs:1623-1660`).
- HIR: `DefKind::External` `hir.rs:212-220`; `TypeDef.host` `hir.rs:243-244`; `Host { namespace, is_async, is_request }` `hir.rs:248-253`; `Program.host_exports`, `host_fns`, `external_fns` `hir.rs:92-121`.
- External typing E0315–E0319: `resolve.rs:1684-1784,1540-1593,1627-1682,1826-1842`.
- Host field rules E0320: `resolve.rs:1391-1535`.
- Program/export rules: `resolve.rs:2430-2613`.
- Entry export inference E0304: `types/infer.rs:1024-1205`.
- Platform table and entry-need check E0523: `platform.rs:41-150`.
- Codes: `crates/ilang-check/src/lib.rs:49-98` (E0301–E0322).

### 3.2 Codegen and ABI

- IR values: `crates/ilang-wasm/src/ir.rs:455-480` (`HostCall`, `HostStart`, `HostReply`, `ImportReply`).
- Import kinds and records: `ir.rs:812-882` (`ImportKind::{External,Service,AsyncService,WasiStdout,WasiAdapter}`, `Import`, `HostService`).
- Import lowering: `lower.rs:1861-1865,1896-1978,1980-2027,2029-2161,2201-2319`.
- Export lowering: `lower.rs:1518-1546,1467`.
- Emitter: `emit/mod.rs:202-235,1632-1652`; `emit/host.rs:15-113,591-711,849-901`; WASI adapters `emit/wasi.rs:1-364`.
- Runtime intrinsics table: `ir.rs:78-94`; `runtime/mod.rs:333-347`.
- ABI JSON: `lower.rs:4585-4656`; ABI reader `crates/ilang-host/src/lib.rs:231-357`; JS reader `bindings/javascript/ilang.mjs:205-305`.

### 3.3 Hosts and SDK

- Rust registration API: `crates/ilang-host/src/lib.rs:554-767` (`service`, `async_service`, `external`, `fallible_external`, `context_*`).
- Import check/link: `lib.rs:1104-1150,1303-1336,2415-2519`; binding plan `bindings.rs:9-122`.
- Standard registrations: `services/mod.rs:50-229` and `services/{clock,fs,net,process,stdio,tls}.rs`.
- JS: `bindings/javascript/ilang.mjs:205-305,839-1042`; `standard.mjs:34-88`; type surface `ilang.d.ts`.
- WASI: `lower.rs:1922-1941`; `emit/wasi.rs`.

### 3.4 CLI

- `i build`/`run`/`test` host-export path: `crates/ilang-cli/src/main.rs:344-488,527-628,684-720`; `lib.rs:46-63,185-262`.
- Permission report (`i deps`): `crates/ilang-cli/src/commands/package.rs:620-836`; `deps` prints host services and `external` names, and entry stubs.

### 3.5 Spec and design text

`docs/spec.md` — 31 lines: 79, 286, 292, 294, 295, 296, 362, 379, 465, 671,
820, 1553, 1556, 1561, 1563, 2401, 2413, 2425, 2456, 2491, 2493, 2532, 2886,
3026, 3118 (plus §10.2 at 2464, §B.2 at 3078, §B.5 at 3130, §B.6 at 3147).

Active design docs: `docs/design/js.md` (3 `@host`, 8 `@js`, 6
`wasm.synchronous`, 4 `wasm.exports`), `native.md` (6/12), `runtime.md`
(8), `syntax.md` (3), `syntax-3.md` (2), `meta.md` (3), `multicore.md` (3),
`records.md` (7), `literal-types.md` (2), `numeric-lowering.md` (1),
`a1-front-end.md` (3).

Editors: `editors/vscode/test/sample.i:17,20`, `sample.i.snap:72,90`,
`grammar.test.js:132`; no grammar file names the three attributes (all use the
generic attribute rule).

## 4. `Await` / `Owner` / `Root` in public signatures

`crates/ilang-check/tests/types/library.out` is 614 lines. **103** lines
contain one of the three; **88 of those lines are exported names**, not 84.
The 84 figure is `103 − 15 non-exported − 4 mixed`; the four mixed signatures
are dropped by that arithmetic: `i/request.open_root:26`,
`open_owner:28`, `wait_none:38`, `wait_none_root:39`.

| Module | exported lines | occurrences | env position | value position |
|---|---:|---:|---:|---:|
| `i/request` | 15 | 20 | 7 | 12 |
| `i/task` | 5 | 22 | 5 | 0 |
| `i/clock` | 3 | 6 | 3 | 0 |
| `i/fs` | 14 | 28 | 14 | 0 |
| `i/process` | 14 | 28 | 14 | 0 |
| `i/net` | 16 | 38 | 16 | 0 |
| `i/tls` | 9 | 24 | 9 | 0 |
| `i/fetch` | 5 | 10 | 5 | 0 |
| `i/stdio` | 7 | 14 | 7 | 0 |
| **Total** | **88** | **190** | **80** | **12** |

Source occurrences are 90: `lib/i/request.i` 38, `lib/i/task.i` 30 (both in
types and bodies), `std/src/http.i` 18, `std/src/log.i` 4.

Exported source declarations: `lib/i/request.i` 36,38,40 (types) and 78,80,82,
86,88,90,92,94,96,98,107,109,140; `lib/i/task.i:3847,4079-4081`;
`std/src/http.i:1247-1249,1253-1254,1286-1287,1293-1300,1308-1309`;
`std/src/log.i:184,190`.

Platform supply points that change when the three become implicit:

- spec §2.6.1 `docs/spec.md:458-460`; §9.7 `:1580,1592`; §9.9 `:1340`; §10.2 `:2472-2482`.
- `crates/ilang-check/src/platform.rs:1-6,73-77,95-171`.
- `crates/ilang-check/src/resolve.rs:1405-1497` (adds `Await`/`Owner` to async service fields).
- `crates/ilang-wasm/src/lower.rs:973-982,1040-1051,1568-1572,1611-1648,2215-2225,2637-2655`.
- `crates/ilang-wasm/src/emit/host.rs:9-13,218-235,601`.

Fixtures and goldens that pin them: 16 `tests/spec/9.11` and `9.9` files, 7
review fixtures + 9 `expected.json` lines, `tests/browser/fixtures/r2`, and
`library.out` itself (all listed in `host-boundary.md` Appendix A).

## 5. What this means for the plan

- The library conversion is the large mechanical part: 159 attribute lines in
  `lib/i` + prelude, plus 88 signature edits.
- The checker/codegen change is the risk: attribute contract, import kind,
  host service mode, entry stubs, export inference, ABI.
- The fixture migration is broad but parallel (tests/spec, tests/review,
  crates/*/tests, browser fixtures, docs).
- `@js` costs a separate target because it is unimplemented; it must not be
  smuggled into the main import phase as "already working".
- No `[wasm].synchronous` TOML exists, so deleting the key costs only spec,
  reader, lowering and two tests.
