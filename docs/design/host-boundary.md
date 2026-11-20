# Design: the host boundary for 0.2.0

Status: first version for approval. Branch `design/host-boundary`, base
`a21eae769`, 2026-11-21. Size evidence:
[`host-boundary-inventory.md`](host-boundary-inventory.md).

No code is changed by this document. Sections marked **Approval** have a
product-level choice; everything else is a decision with its reason.

## 0. Summary

The boundary has two directions and two products:

- **Import**: i calls the host. Syntax: one attribute, `@import`, on a service
  type. The toolchain synthesizes a typed `make_<Type>` accessor. The owner or
  a direct dependent constructs the value; a sibling cannot reach the module.
- **Export**: the host calls i. Syntax: `@export` on a top-level function of a
  **library** package. There is no manifest list.
- **Program** and **library** are two products, chosen by `product` in
  `i.toml`, not by a function's name. A program has an entry and is run; a
  library has exports and is embedded. Dependencies must be libraries.
- `@external`, `@host`, `@js`, `[wasm].exports` and `[wasm].synchronous` are
  deleted, with a dedicated "retired" error and no aliases.
- `hostcall = true` is a per-package, default-off switch that makes
  `i/hostcall` importable. It adds the three capabilities that the old
  `Request`/`Await`/`Owner` surface exposed: a custom wait, a chosen call
  group, and a hand-written dispatcher.
- `Await`, `Owner`, `Root`, `Scheduler` and `Runner` become **implicit
  services**: inferred, never written in need rows, and dropped from them in
  display; a genuine value parameter of one stays. The 106 exported signatures
  that carry them today lose the row entries. This change and `hostcall` are
  one phase: both use the same hidden-parameter handling, and splitting them
  would edit the same signatures twice.

The release gate: **0.2.0 is not tagged until phase E is green.**

## 1. Products

### 1.1 Manifest

`i.toml` gets one root key:

```toml
product = "library"   # optional; "program" is the default
```

`product = "program"` is accepted too. A missing key means a program, and the
program rules then give a clear error when they do not fit: no entry is
`E0617` ("add `main`, or set `product = \"library\"`") and an `@export` is
`E0330`. The default is safe because both mistakes name the fix; only a
library has to write the key. This keeps the 2 209 fixture manifests mostly
untouched.

`product` changes entry checking, artifacts and dependency rules, so it is
explicit where it differs. It is not a name rule: an absent or misspelled
`main` gives an error, never a silently different product.

Every package that another package depends on needs `product = "library"`;
the phase adds the key to all of them (including `std` and the dependency
fixtures) with a script and a load test that no dependency is a program.

### 1.2 Rules that make the products real

| Rule | program | library |
|---|---|---|
| Entry | exactly one selected entry: `main` in `src/main.i`, or a key of `[entries]` | no program entry; `[entries]` is `E0614` |
| `main` in `src/main.i` | the default entry | an ordinary function; not run |
| `@export` | `E0330` ("a program is run, not called; export from a library") | the only place it is legal |
| `i run` / `i test` | allowed | `i test` allowed; `i run` is `E0331` and names `i build --target host` |
| Default `i build` target | `native` (as today) | `host`; `--target native` on a library is `E0332`; `js`, `nodejs` and `wasi` are allowed |
| Dependencies | may be libraries only; a program dependency is `E0615` at the dependency key | may be libraries only |
| Artifacts | one entry artifact (`build/<name>` or the target's `.wasm` + loader) | reactor `.wasm`, `bindings.json`, generated bindings; no executable |
| `i deps` | prints `product: program` | prints `product: library` and its exports |

A library with no `@export` is a warning (`W0330`) on the compiled root
library only, not on every dependency (std has none), and not an error: a
macro-only or import-only library exists.

A program may itself be built for `host`; then it is still a command with
`ilang_start` and one entry, not a callable interface. That is what
"`main` is an executable file" means: entry selection is a property of the
product, not of the spelling `main`. In a program, `add` is dead code unless
`main` calls it; in a library, `add` is dead unless `@export`ed.

### 1.3 Alternatives

| Option | Cost | Why not |
|---|---|---|
| `product = "library"` with program default (chosen) | one key on libraries only | — |
| Required `product` on every package | 2 209 fixture manifests get a line | the default already produces safe errors; the sweep is noise |
| `[package] type = "…"` | a table in every manifest, flat style broken | no second package key is planned |

**Approval P1**: confirm optional `product`, default `"program"`, `"library"`
explicit. Alternative: require it everywhere (2 209 edits, scriptable in one
commit); `[package] kind` (same count plus a table). Cost of changing later:
the manifest reader and the library-only manifests once.

## 2. Export marking

### 2.1 Syntax

```i
@export
add : (Int, Int) -> Int = (a, b) { a + b }

@export("sync")
print : (String) -> Nil = (text) { … }
```

`@export` is legal on a top-level function item of a library package only. It
takes no argument (asynchronous entry protocol) or exactly `"sync"`. It does
not change i-module visibility: a separate `(add)` group is still needed for
another i module to import it.

Only functions: a function *value* does not cross (§10.3), but the host calls
one by name. Its parameters and result must be boundary types; its service
needs stay on the i side and the platform binds them from imports, so a
service is not passed through the ABI. Stateful host APIs return an opaque
handle and functions over it. A record or service value itself is not
exported; `@export` on a whole service type would need a host-side handle
protocol, so it is deferred (P2).

### 2.2 Rules

- The export name is derived: `<package name>/<module path after the package>.function`. For package `app`, module `app/log`, `function add` gives `app/log.add`. No string list and no duplicate-name rule can be violated; `@export` twice on one item is `E0309`.
- The function must be module-exported if it is also to be called from another i module; host binding generation reads the definition, not the module group.
- Shape rules reuse the current E0304 family: positional parameter data, host-takeable visible types (§10.3), at most `<- (Fail(E))` with host-takeable `E`, required annotation, no type variables.
- `@export("sync")` is `E0339` when the inferred need has a `"later"` import
  service or `Scheduler`/`Runner`, and the message names it. Without `"sync"`,
  the export uses the async entry protocol and may return status 0 on the
  first call.
- An export implies reachability; the unused-binding warning skips it. No host-export/program-entry conflict rule remains, because the product separates them. Only the root package's `@export`s fill `host_exports` and the ABI; a dependency's exports are checked but are not exposed.
- `@export` in a module of a dependency does not re-export through a consumer; the consumer's own `@export` set is what it exposes. Host bindings for a library list only that library's exports.

### 2.3 `[wasm]` deletion

Both keys go. `wasm.exports` is replaced by `@export`; `wasm.synchronous` by
`@export("sync")`. No `.toml` uses `synchronous` today
(inventory §2), so that deletion has no data migration. `[wasm]` in an old
manifest is `E0612`:

```
error[E0612]: the `[wasm]` table was removed in 0.2.0
  = mark a function `@export` in its module, and `@export("sync")` for a synchronous one
```

### 2.4 Alternatives

| Option | Cost | Why not |
|---|---|---|
| `@export` attribute (chosen) | one parser/checker name | — |
| `pub` keyword | no `@`; reads like Rust | i has no keywords; a `name` `pub` would become reserved, and every attribute form already exists |
| A module export group means a host export | no new syntax | conflates "another i module may import this" with "the host may call this"; makes every public helper a host entry and leaves no sync marker |

**Approval P2**: confirm `@export` / `@export("sync")` on functions only.
Alternative: also allow `@export` on a whole service type, exporting its
fields as a group (cost: an object/handle protocol for obtaining the service
value; +1 phase).

## 3. Importing the host

### 3.1 The declaration

The user writes a service type in the root package and marks it:

```i
// src/log.i
@import("app:log/v1")
Logger = (log = (String) <- (Fail(String)) -> Nil)
```

The type is ordinary otherwise: one constructor, labelled function fields
(§9.7). Its module must export it as `Logger(..)`; `@import` requires the
constructor and the field access because the accessor and callers read the
fields and a test constructs a fake. Missing it is `E0329`.

Modes:

- `@import("ns")` — default, `"later"`: the host does the call, the i caller
  waits at the call and continues with the answer. In i code the field is an
  ordinary function.
- `@import("ns", "now")` — synchronous: the answer is already there. Used for
  `i/io.print` only; every other standard service is `"later"`.
- `@import("ns", "label", mode)` — one standalone imported function, `mode`
  written. Tests use it; a service's fields do not need it.
- `@import("js", "expression")` on a declaration of an opaque type — a JS
  value getter (the `@js` replacement, §4.3).

`ilang:` namespaces belong to `@runtime`, not `@import` (`E0306`). `i:` names
are ordinary and may be declared by any package, but must not repeat a
namespace+label already in the program (`E0311`); re-declaring a standard
service therefore means using the library's own type, not a copy.

Field types are written **without** `Request`: `() -> Result(Nil, String)`,
not `() -> Request(Result(Nil, String))`. `Request` is the low-level handle
type (§5); it does not appear in an ordinary service. The mode is per service;
a service that needs both synchronous and waiting fields is split into two
service types (the import namespace may repeat with different labels).

### 3.2 The generated implementation

An `@import` service type gets a synthesized top-level accessor in its **own
module**, `make_<Type>`, appended in memory after the source items and added to
the module's exports:

```i
// src/log.i as written
@import("app:log/v1")
Logger = (log = (String) <- (Fail(String)) -> Nil)
f = () { ... }

// synthesized at the end of app/log, not written to disk
make_Logger : () -> app/log.Logger
```

`make_Logger()` constructs the service, or returns the one the **call site**
already has: it returns `Logger(log = <the import function>)`, and if the
caller's *service environment* carries a `Logger` (a `<-` binding or inferred
row in scope, including a fake or a platform value), it returns that instead.
A local variable is not an environment entry and does not count. The decision
is made at compile time.
Each field's import function value is synthesized from the service type's
`@import` namespace, label and mode; no source declarations for them exist.
The accessor has **no need**, so a host-facing signature stays free of the
service type and the host supplies only imports. A function that wants an
injectable service takes it in its own `<- (logger : Logger)` pattern instead;
callers then pass `make_Logger()` or a fake.

```i
(log = app/log)
main = () {
	{ … } <- (log.make_Logger())
}
```

- **No file, no second module, no cycle**: the accessor is in the same module
  as the type, so it can use the constructor and cannot make a module cycle.
  There is no `gen/` directory; the first path segment stays free. `i fmt` sees
  only the source. The synthesized def is a declaration with its own kind, so
  the "a declaration needs `@import`/`@runtime`" rule does not apply to it.
- **Type consistency**: the accessor's result names the service type
  directly, and the synthesized fields are typed by that type's own labels;
  they cannot differ. The resolver synthesizes the accessor in its top-level
  collection phase, from the `@import` type's name, so module order does not
  matter.
- **Identity**: each synthesized field is one top-level import value (not a
  fresh closure), so two accessor results have the same field functions and
  compare equal field by field.
- **Visibility**: it is an ordinary exported name of the service's module, so
  it is reachable by the owner and by that package's direct dependents; a
  sibling cannot reach it. A dependency that calls its own accessor keeps the
  service out of its signature; one that takes the service in its pattern
  exposes it, and a dependent calls the accessor to provide it.
- **Name collisions**: the accessor and its field values are named
  `make_<Type>` and `<type>_<label>` (first character lowercased); a user
  definition with one of those names is E0208, and the user renames.
- **Fakes**: a test in another module passes `Logger(log = …)` using the
  exported constructor. The fake reaches a function only if that function
  takes `Logger` in its own pattern; an accessor-style function builds/returns
  the platform value and cannot be injected from outside.

The accessor is the only way to get a user service value outside the type's
own module unless the function takes it as an explicit parameter. Standard
library services (`i:clock/v1`, …) are in the target's table, so the platform
always provides them and a program keeps calling `clock.monotonic_ns()`
without an accessor; an accessor returns the platform value. A user service is
not in the table: if an entry need names it (the function takes
`<- (logger : Logger)`), the platform binds it from the host's registered
imports, so an explicit parameter gets the host value (`E0334` on native,
where the runner registers none); otherwise the accessor builds it. An
explicit parameter may therefore be a fake; an accessor-only use constructs
the real one.

Because the accessor has no need, async classification can no longer read a
service type out of the entry need. It moves to a per-entry link pass: for
**each entry separately**, start from that entry and the top-level
initializers, and activate the §10.1.2 graph. The graph gains two edge kinds:
a field call of an `@import` type activates that field's import, and a read of
the field (passed as a value) activates it too. The entry is **async** exactly
when its own activated set contains a `"later"` field or `Runner`/`Scheduler`;
another entry's later import does not make it async, and a `"now"` field does
not. `platform::stub` takes `is_async` from that per-entry result, and
`@export("sync")` (E0339) is checked against it. Metadata and wrapper shape
follow before optimization, as today.

This is a new pass, not an existing one: §10.1.2's only implemented user is
`@js`, which never shipped. It runs in the checker beside foreign linking and
is consumed by all three `platform::stubs` callers (`lower.rs`,
`commands/package.rs`, `main.rs`).

### 3.3 Host side

The namespace is reserved to the declaring package in the program: two
declarations with the same namespace and label are `E0311` (existing rule).
The platform binds every host service in an entry need, including a user
service passed as an explicit parameter; the standard table decides which a
target *guarantees*, and a user service is instead guaranteed by the
embedder's registration (`E0334` on native). A missing user-service parameter
is the usual `E0523`.

- **Rust**: `Instance::service("app:log/v1", "log", closure)` and
  `context_service` / `async_service` keep their signatures; the generated
  `build/<name>.rs` adds a typed `Logger` trait and a registration helper so a
  typo is a compile error.
- **JS**: `options.imports["app:log/v1"].log`; the generated `name.d.ts`
  types the object and `name.mjs` validates that every field is a function of
  the right arity before instantiation.
- **WASI**: the module imports `("app:log/v1", "log")` directly. The generated
  `name.h` declares the import with the §B.2 layout. The embedder supplies it;
  there is no adapter.
- **`i run`, `i test`, and `i build --target native`**: user namespaces are
  not in the native standard table. They stop before running with `E0334`
  ("the program imports `app:log/v1`, which the native host does not supply;
  build it and embed it, or provide a value in a test"). A test that never
  calls the accessor and passes its own record does not need the import, so it
  runs. `i run` has no test body, so it always stops.

`@import` does **not** need `hostcall`. It is the default layer. The generated
implementation is the typed surface; `i/hostcall` only controls how a call
waits and which group it is registered in (§5). A host answer error follows
the field's `Fail(E)`; a host trap or a host work failure ends the program as
today. `hostcall` is a language switch, not a host grant: it appears in
`i deps` but the embedder grants nothing extra.

`i deps` and `i publish`'s permission diff print imports as
`import "ns" "label"` and exports as `export "pkg/module.fn"`; the old
`external "m" "f"` wording disappears. A namespace that the root itself
declares is a *requirement* of the root; a library's namespace is a
requirement it adds to whoever uses it. The synthesized accessors are ordinary
exported names and are not listed separately.

### 3.4 Alternatives

| Option | Cost | Why not |
|---|---|---|
| `@import` + same-module accessor (chosen) | synthesis in the resolver, collision rule | — |
| Keep `@host`/`@external` under new names | smallest | mechanism unchanged: two attributes and a generic `Request` in signatures |
| WIT file + manifest mapping | tool interop, one manifest map | the owner rejected manifest mapping; also duplicates the type in two languages |

Mode words (`"now"`/`"later"`) stay in the `@import` argument because the mode
is part of the declared host contract, unlike the old `"async"`/`"request"`
split which also forced `Request` into every field type. A mode is one word at
the declaration, never repeated at a call.

**Approval P3**: confirm the same-module accessor `make_<Type>` (no generated
files or modules) and that it is an ordinary exported name. Alternative: a
separate `gen/` module per source module (a visible file for editors, but a
module cycle when the service's own module calls the accessor; needs a third
module or a load pass). The name alternative is `impl_<Type>`; `make_<Type>`
wins because it reads as a constructor.

## 4. Deleting the three attributes

### 4.1 Replacement table

| Today | After |
|---|---|
| `@external("m", "f")` (user/test) | `@import("m", "f", "now"|"later")` |
| `@external("ilang:…", "f")` (library runtime ops) | `@runtime("ilang:…", "f")` |
| `@external("i:compiler/syntax/v1", "f")` | `@runtime("i:compiler/syntax/v1", "f")` |
| `@host("ns")` | `@import("ns", "now")` |
| `@host("ns", "async")`, `@host("ns", "request")` | `@import("ns")` (default `"later"`) |
| `@js("expr")` | `@import("js", "expr")` |
| `[wasm].exports = [...]` | `@export` on each function |
| `[wasm].synchronous = [...]` | `@export("sync")` |

`@runtime` is legal in the library package `i` only (`ilang:` and
`i:compiler/` namespaces), with the existing external type rules. It is the
"the compiler/runtime implements this" marker, not a host boundary. A user
package writing it is `E0306` unless it declares `ilang:`/`i:compiler/`.

Names are not aliased: a retired attribute is `E0337`, one error per item,
naming its replacement. `@host` with a mode string points at `@import`; `@js`
points at `@import("js", …)`.

### 4.2 Library conversion, exactly

`lib/i` and `lib/prelude.i` (159 attribute lines):

- 30 `ilang:core`, 12 `ilang:big`, 2 `ilang:bits`, 6 `ilang:control`,
  3 `ilang:key`, 1 `ilang:resource`, 22 `ilang:request`, 14 `ilang:worker`,
  1 `ilang:prelude`, 56 `i:compiler/syntax/v1` → `@runtime` (147 lines).
- 12 `@host` → `@import`:
  - `i:io/v1` → `@import("i:io/v1", "now")`;
  - `Clock`, `Stdio`, `Fs`, `Net`, `Tls`, `Fetch`, `Arguments`, `Environment`,
    `WorkingDirectory`, `Entropy`, `Process` → `@import("ns")`.
- Their field types drop `Request(...)`: e.g.
  `sleep_ns : (Int) -> Result(Nil, String)`.
- Wrappers drop `request.await_owned(...)` and decode the result directly.
  `i/clock.monotonic_ns` becomes `() <- (Clock, Fail(ClockError)) -> Int`
  (displayed without the implicit services).
- Wrappers that need the handle for a resource guard (`fs.open`, `fetch.open`,
  `net.listen/connect/bind`, `tls.*`, `process.spawn*`) use
  `hostcall.call(() -> raw.open(…))` and `hostcall.await`.
  The library package `i` therefore has `hostcall = true` in its synthesized
  manifest. This also makes the library the first real client of the switch.
- `lib/i/clock.i:17`, `fetch.i:20`, `fs.i:43`, `io.i:10`, `net.i:42`,
  `process.i:54,57,60,63,66`, `stdio.i:21`, `tls.i:31` are the 12 sites. The
  brief names five host services; the repository has twelve because `process`
  is split by capability, `stdio` exists beside `io`, and `tls` sits beside
  `net`. All twelve convert; none stay on `@host`.

`std/`, `examples/`, `docs/content/` have **no** attribute site. They do need
work: `std` (source and tests) and `examples/cli-log`,
`examples/io-process/test` import `i/request`, so their manifests get
`hostcall = true`; `std/src/http.i` and `std/src/log.i`
lose the `Await`/`Owner` rows in their exported signatures (inventory §4).

### 4.3 `@js`

`@import("js", "globalThis.document.body") body : Element` on a declaration
whose type expands to one opaque type. `js` is a reserved pseudo-namespace and
cannot be a service namespace. All §10.1.1 rules stay: required annotation,
one opaque type, one retained initialization in `ilang_start`, loader parse
error before startup, compile error on a target without JS, distinct objects
for two declarations. `@js` was never implemented (E0301; spec-known-gaps
DOC3), so this phase also removes its "kept for macros" state.

The `i/js` target module defines `Js` when it lands; `Value` stays an ordinary
opaque alias there.

### 4.4 Spec edits

| Section | Change |
|---|---|
| §1.4 | "a declaration with neither `@external` nor `@js`" → neither `@import` nor `@runtime`; the attribute examples become `@import`/`@export` |
| §2.2 | attribute list: `@import`, `@export`, `@runtime`, `@deprecated`; forms and places; retired names are not accepted |
| §2.1 | no reserved path segment is needed (the accessor is a normal name) |
| §2.4.1 | declarations listed by `@import`/`@runtime` |
| §2.5 | declaration rule |
| §2.6 | add `product`, `hostcall`; delete the `[wasm]` block; dependency kinds |
| §2.6.1, §2.6.2 | entries by product; `@export("sync")` classification; no `wasm.synchronous` |
| §4.3 | opaque from `@import("js", …)` |
| §4.7 | annotation required on imports |
| §9.7 | "imported services"; no mode strings; no `Request` in fields |
| §9.11 | `Await`/`Owner`/`Root` are implicit; new §10.4 for `i/hostcall` |
| §10.1 | imports: service, raw function, JS; §10.1.1 JS; §10.1.2 foreign linking unchanged except the import-field edge |
| §10.2 | "Products and entries": program vs library, export marking, platform services implicit, async from the link pass |
| §10.3 | examples use `@import`; the rule is unchanged |
| §10.4 (new) | `hostcall` switch, `i/hostcall`, groups/waits/calls, the misuse table, no-leak rules |
| §11.4 | "each import declaration" |
| §14.5, §14.9 | caller-origin attributes become the new set; any `@import` in macro needs is an error |
| §B.2 | unchanged encoding; wording |
| §B.4 | "imported functions" and "imported fields"; no `@external`; unify the wire kind |
| §B.5 | export names derived from `@export`; duplicate rules simplified |
| §B.6 | one `imports` map (`namespace` → `label` → signature + `mode`); `exports` from `@export`; no `services`/`externals` split |
| §B.6.1 | `name.d.ts`/`name.mjs` types for imports and exports |
| §B.7, §B.7.1 | generic import/resume on WASI; no adapter for user namespaces |

Active design docs `js.md`, `native.md`, `runtime.md`, `syntax.md`,
`syntax-3.md`, `meta.md`, `multicore.md`, `records.md`, `literal-types.md`,
`numeric-lowering.md` and `a1-front-end.md` get the same substitutions in the
phase that changes the underlying feature. Test-data and prose outside the
source tree also migrate: `scripts/check-snippets.json`,
`bindings/javascript/README.md`, `crates/ilang-host/README.md`,
`scripts/doc-comments-rewrites/final-tree.json`. Historical stage reports get a
one-line banner at the top: "boundary syntax predates 0.2.0; see
`host-boundary.md`".

### 4.5 Alternatives

| Option | Cost | Why not |
|---|---|---|
| Delete all three (chosen) | 509 attribute occurrences + 22 manifests + spec | required by the decision; no compatibility |
| Keep `@external` for runtime intrinsics only | 147 fewer edits | it is visible in `lib/i` and keeps the bad name; `@runtime` cleanly separates "compiler op" from "host call" |
| Keep `@js` | 17 occurrences, no implementation | the owner said it disappears; one attribute is the rule |

**Approval P4**: `@runtime` as the library-only marker. Alternative: reuse
`@import` with a `"runtime"` mode (fewer names, but runtime ops are not
imports and have no namespace registration).

## 5. `hostcall`

### 5.1 Switch

Root key in the importing package's own `i.toml`:

```toml
hostcall = true
```

Default `false`. Must be a boolean (`E0616`). Per package. No version key,
no translation of older manifests: a manifest without it is off. No root
consent: a dependency may enable it and the root needs nothing.

Access is `(hostcall = i/hostcall)`. The **original dispatcher module
`i/request` is gated the same way**: importing it also needs the key, so the
three capabilities cannot be reached with the key off. A package that only
uses `i/clock`-style wrappers never imports either and needs nothing.
`i/hostcall` re-exports the `i/request` primitives the dispatcher needs, so
most code imports one module. The library package `i` has the key on in its
synthesized manifest (its own `i/task`, `i/fs`, … use the primitives). `std`
and the `i/request` test fixtures set the key.

`i/hostcall` is a new reserved library path (beside `i/core`); its module is
always in the library graph. Without the key in the *importing package's*
manifest: `E0323` (the same error for `i/request`):

```
error[E0323]: `i/request` needs the hostcall switch
  --> src/main.i:2:1
   |
 2 | (request = i/request)
   |  ^^^^^^^
   = set `hostcall = true` at the top level of /…/i.toml
```

The same error names `i/hostcall` when that is the imported module.

Resolution stays deterministic: `i/hostcall` is always in the library graph;
the resolver checks the importing module's package flag. Transitivity is
explicit: if package B enables hostcall and the root A calls B's clean
functions, A needs no key. If A wants to wrap B's call in its own group, A
turns its own key on and imports `i/hostcall`.

### 5.2 Types

`i/hostcall` exports:

```i
Call(A)                   // an unwaited start under one group; not a boundary type
CallGroup                 // a live group; not a boundary type
Wait = (wait = (Ticket) -> Nil)   // a wait mode
```

`Ticket` and `Root` are name aliases of the `i/request` types (a module may
only export names it defines), and `RequestState` is aliased for `state`.
`Request(A)` remains the dispatcher's cell type, not a compatibility layer: a
package with the key off never sees it, because boundary-visible types are
clean and wrappers return the value. The dispatcher primitives below are thin
wrappers over `i/request`, not a second implementation.

`Call(A)` and `CallGroup` are opaque (no public constructor). `Wait` is an
ordinary record exported with its constructor, so a dispatcher can write one:
`Wait(wait = (ticket) { … })`. A `Call(A)` is the existing request cell plus
its group provenance in the type. `CallGroup` is the existing group key with a
worker/root stamp. `Wait` is the existing `Await` shape under its own name, so
the platform default and a custom value are distinguishable; `waiting` wraps
the bare function into the ambient `Await`.

### 5.3 Surface

```i
// groups
group : () -> CallGroup            // at the ambient current root
group_at : (Root) -> CallGroup
close : (CallGroup) -> Nil         // idempotent; cancels outstanding
closed : (CallGroup) -> Bool

// scopes: the main capability, per-call group without signature changes
under : <A>(CallGroup, () -> A) -> A
waiting : <A>(Wait, () -> A) -> A

// unwaited starts
call : <A>(() -> A) -> Call(A)              // ambient group; one direct imported field call
call_in : <A>(CallGroup, () -> A) -> Call(A)
await : <A>(Call(A)) -> A
state : <A>(Call(A)) -> RequestState(A)
take : <A>(Call(A)) -> A
cancel : <A>(Call(A)) -> Nil
settled : <A>(Call(A)) -> Bool
ticket : <A>(Call(A)) -> Ticket
adopt : <A>(Request(A)) -> Call(A)          // join the ambient group
request : <A>(Call(A)) -> Request(A)

// dispatcher primitives, re-exported
root : () -> Root
open_root : () -> Root
close_root : (Root) -> Nil
wait_any : (List(Ticket)) -> Nil
poll : () -> Nil
```

A group registers at a Root and its outstanding starts count in that Root's
accounting. It stays live until `close` (or a close of its Root); `close`
cancels its Waiting members, lets Ready cells belong to their handles, and
forbids later starts. A `Call` keeps its group id for cancel/retire, but a
settled Call no longer counts.

`under(g) { body }` makes `g` the group of every start in `body`, including
starts inside library wrappers. `waiting(w) { body }` makes `w` the wait of
every await in `body`. Both nest; the innermost wins; both restore on normal
return, error, panic and task cancellation, like `resource.ensure` finals.
Inside a task, `waiting` changes how this continuation waits; it does not
replace the task scheduler, so a blocking custom wait starves that run's
siblings until it returns. On WASI there is no hand-off; `poll` and a wait
that must hand off follow the existing "no active entry" rule.

`call(op)` / `call_in(g, op)` is the raw-start form. It evaluates `op` in
no-wait mode and returns the start instead of awaiting it; `call` uses the
ambient group. `op` must be **one direct field call of an `@import` service
type** at its top level (`E0335`; e.g. `() -> logger.log("x")`). It does not
accept a library wrapper, because a wrapper awaits and maps its own errors; for
a wrapper, use `under(g) { … }`, which lets it await normally under the chosen
group. A `"now"` field is called immediately: its error raises at `call`, and a
successful result is wrapped as a ready Call.

The checker resolves the closure's callee to a field of an `@import` service
type. Lowering marks the closure as no-wait; at the field call it tests the
callee's **runtime import tag**: an import field value carries the import
index (the existing closure representation gains an optional import index), so
a start is emitted; a fake closure has none, so it is called and its result
wrapped as a ready Call. A fake that raises its `Fail` raises at `call`, the
same as a `"now"` field; no Call is made.

### 5.4 Misuse: compile-time or runtime

| # | Misuse | Caught by | Result |
|---|---|---|---|
| 1 | import `i/hostcall` or `i/request` without the key | compile `E0323` | — |
| 2 | write `Call`/`CallGroup`/`Wait` without importing `i/hostcall` | name error `E0201` | — |
| 3 | export or module-export a function signature naming `Call`, `CallGroup`, `Wait`, `Request`, `Ticket` | compile `E0324` | — |
| 4 | call `call` on a non-field closure | compile `E0335` | — |
| 5 | `call_in`/`under` a closed group, or `group_at` a closed Root | runtime | panic `hostcall: group closed` / `hostcall: root closed` |
| 6 | use a group from another worker | runtime | panic `hostcall: foreign group` |
| 7 | `call` under a closed ambient Root | runtime | panic `hostcall: root closed` |
| 8 | `await`/`take` a terminal Call | runtime | panic `hostcall: taken` / `hostcall: cancelled` |
| 9 | `take` a waiting Call | runtime | panic `hostcall: not ready` |
| 10 | custom `Wait.wait` returns while the ticket is Waiting | runtime | panic `hostcall: wait returned early` |
| 11 | custom `Wait` receives a Ticket of another Root | runtime | panic `hostcall: foreign ticket` |
| 12 | a `Wait.wait` starts a request | runtime | panic `hostcall: wait reentered` |
| 13 | `root()`/`poll()` with no active async entry | runtime | panic `hostcall: no active entry` (existing text family) |
| 14 | close the group of a Call whose await is in flight | runtime | the start is cancelled; the await panics `hostcall: cancelled`; no leak |
| 15 | `cancel` a terminal Call | runtime | idempotent; no panic |
| 16 | `close` inside its own `under` | allowed | later starts in the body panic; the scope restores |
| 17 | `close_root` the ambient Root inside a body | allowed | later starts panic; settled Calls are unaffected |
| 18 | `adopt` a Request already owned by another group | allowed | the Call's group becomes the ambient one; either group's close cancels the shared ticket (idempotent) |
| 19 | use a value whose type is a hostcall type, imported through an inferred signature | allowed; the caller can pass it on without naming it | — |

Rows 15–19 are defined behavior, not misuse; row 19 is the inference case.
Naming a hostcall type requires the key; carrying one does not.

Panics are the chosen result for programmer errors in a dispatcher: a host
answer error still goes through `Fail`; a host trap or a host work failure
still ends the whole program as today; a misuse is a bug in the package, not
a recoverable answer.

### 5.5 "No leak" as checkable rules

1. **Implicit services.** `Await`, `Owner`, `Root`, `Scheduler`, `Runner` may
   not be written in a need row of program source (`E0325`). The library
   package `i` is exempt: it binds and forwards them (`waiter : Await`), since
   it is the implementation. Inference carries them everywhere; the type
   printer drops them from **need rows only** (a value parameter or result
   stays, because the caller passes it); the export ABI encoder drops the
   need entries. One predicate in the checker (a fixed list of the five
   `(module, name)` identities) drives all three so they cannot drift.
2. **Boundary-visible signatures.** Every boundary-visible **function**'s
   expanded type is checked for the mechanics types `Call`, `CallGroup`,
   `Wait`, `Request`, `Ticket`; any is `E0324` with the name. In an `@export`
   signature, a **value** position of an implicit service is also `E0324`
   (the host cannot pass it); in a dependency's module export it stays,
   because i callers can pass it. The check is by type identity, so
   `std/http.Request` (an HTTP record) is not the mechanics `Request`. The
   five implicit
   services may stay inside records and aliases because display hides them
   everywhere; they are only checked at the function's own row (they cannot be
   written there, rule 1). "Boundary-visible" is `@export` for the root
   package; for a dependency with the key it is every function in a module's
   export group. The library `i` is exempt, and `i/request`/`i/hostcall` are
   its mechanics surface.
3. **Dependencies.** Rule 2 makes an enabled dependency's exported **function
   signatures** clean, so a non-enabled caller sees no mechanics in a
   signature. An exported *type* may still expand to `Call`, so naming that
   type needs the key. If a caller wants to name a hostcall type itself, it
   must import `i/hostcall` or `i/request`, which is `E0323` without its own
   key.
4. **Generated bindings.** `build/*.rs`, `*.d.ts` and `*.h` are generated from
   the visible type only. A debug assertion in the generator rejects an
   implicit/hostcall type, so a metadata leak fails the build test.
5. **Golden proof.** `crates/ilang-check/tests/types/library.out` has no
   implicit name in a need row after phase A. A second golden
   `library.needs.out` has the full 240 occurrences, so the machinery is still
   tested even though it is hidden.

Rule 2 means an enabled package cannot accidentally leak by returning
`Call(A)` from an export. A library that wants to expose a startable call
returns `A` from an exported function; a raw caller uses `hostcall.call` on
the imported field.

### 5.6 `i deps` and `i build` output

`i deps` adds, under every package with the key:

```
  hostcall
```

For the root too. `i build` prints one line before the artifacts:

```
hostcall: <package key>
```

in package order, for each enabled package in the graph. The key is the same
as in the `i deps` report: `location@major`, `path <dir>`, or
`<this package>` for the root. `i check` prints nothing. The synthesized
accessors are not listed. Tests pin both. No output when no package enables
it.

### 5.7 Alternatives

| Option | Cost | Why not |
|---|---|---|
| `hostcall = true` root key (chosen) | one key | — |
| `[hostcall] enabled = true` | structured | one boolean does not need a table |
| Root consent key that dependencies must satisfy | explicit grants | the owner said "no consent needed"; it also breaks a library that adds the capability in a minor version |
| `Call`/`CallGroup`/`Wait` as ordinary library-only types | no flag | the three capabilities are then always visible; the owner required default-off |

**Approval P5**: the misuse split (compile vs runtime panic) and the ambient
names `under`/`waiting`. Alternative names: `in_group`/`with_wait`; compile
the closed-group impossibility by making `close` consume a linear `CallGroup`
(cost: a linearity feature; it catches #5 statically but not #10–#13).

## 6. Transparent runtime

`Await`, `Owner`, `Root` leave the 88 exported signatures, and
`Scheduler`/`Runner` with them: the full implicit set is **106 exported
signatures and 240 occurrences** (the 88/190 figure is `Await`/`Owner`/`Root`
only; `Scheduler`/`Runner` add 23 exported lines and 50 occurrences, 5 shared
with the 88, so 106/240). All of them are inferred and hidden, not removed:
they remain ordinary services at run time and can still be obtained as values
(`hostcall.root()`), but a signature neither writes nor shows them.

- The checker already infers them into the environment row. The change marks
  them implicit: source may omit them (and must, outside the exempt library
  `i`; §5.5 rule 1), display
  filters them, metadata filters them.
- Calling a function with implicit parameters passes the ambient values, so
  lowering cannot change meaning; only the printed/encoded signature changes.
- Entries still bind them as today (`platform.rs`, `lower.rs` entry stub); the
  bindings are just not named in source.
- `i/task` keeps its `i/request` primitives; only its signatures lose the
  implicit names. `Request` stays for the low-level layer and for `i/task`.
- The 18 value positions (12 exported, 6 private) stay ordinary parameters or
  results in source and display: a caller passes them. Only the need-row
  occurrences disappear.

**One phase with `hostcall`**: both need the same hidden-parameter plumbing and
edit the same files (`resolve.rs`, `infer.rs`, `platform.rs`, `lower.rs`,
`library.out`). Two phases would touch all 106 signatures twice and re-baseline
twice, for no earlier user value. Alternative: hide first, hostcall second
(+1 CI cycle, ~40 min); not chosen.

### 6.1 Migration list

The full 88 names are Appendix A. Split by file:

- `lib/i/request.i`: 15 exported functions + 3 types.
- `lib/i/task.i`: 5 exported functions; 9 more private signatures.
- `lib/i/{clock,fs,process,net,tls,fetch,stdio}.i`: 68 exported signatures
  (wrapper rows; most drop `Await, Owner` only).
- `std/src/http.i`: 3 exported types/functions; `std/src/log.i`: 2.
- `library.out` 103 lines and 9 review `expected.json` files.
- 16 `tests/spec/9.11`/`9.9` fixtures and 7 review fixtures (Appendix A.2).
- `docs/spec.md` §2.6.1, §9.7, §9.9, §9.11, §10.2; active design docs.

`lib/i/request.i` keeps `Await = …`, `Root = …`, `Owner = …` as type
definitions and keeps its need rows for binding them; display hides them, so
`library.out` is clean. Other library modules drop their written rows where
inference covers them; `i/task`'s `N - Await - …` subtractions stay and are
hidden too.

## 7. Phases

Each phase ends green on its branch, merges to the integration branch, and
runs the CI shards its change touches (`rust-test` group, `spec`, and the
relevant host tests) before landing. The full `scripts/dev-verify` and the
review corpus run once at E, not per phase. Workers are cheap; the estimates
are wall time on this machine from measured `cargo build`/test cycles, not
human hours. A cold full build measured **31 min 39 s**;
a cycle below is a rebuild of the touched crates plus the
affected test group, not a full clean build.

| Phase | User can now do | Main change | Parallel workers | Critical path |
|---|---|---|---|---:|
| 0 | nothing yet; the new forms parse and give the new errors | parser/AST forms, manifest `product`/`hostcall`, codes, `@runtime` in `lib/i`, `@import` service form | 4 | 1 cycle ≈ 40 min |
| A | choose a group per call, replace the wait, write a dispatcher, with no `Await`/`Owner` in any signature | implicit services, hostcall module/types/lowering, library conversion, `deps`/build output, 88-signature corpus | 7 | 3 cycles ≈ 120 min |
| B | build a library a host can call, exports marked where written | product rules, `@export`, `[wasm]` deletion, export stubs + Rust/JS/WASI export bindings, CLI errors | 6 | 2 cycles ≈ 80 min |
| C | call a host function from user code and embed it with typed stubs | user `@import` host stubs for Rust/JS/WASI, `@js` retirement, old-attribute deletion, corpus | 6 | 3 cycles ≈ 120 min |
| D | use JS globals | `@js`→`@import("js",…)`, JS value goldens | 4 | 2 cycles ≈ 80 min |
| E | — | spec/design/docs/corpus, full verify, review, release gate | 4 | 2 cycles + review ≈ 120 min |

Critical path: 0 → A → B → C → D → E ≈ **9 h 20 min**. Allowing one
failed-review rework (+60 min) gives **~10 h 20 min** to a 0.2.0 candidate.
Measured here: a cold `cargo build -p ilang-cli` took **31 min 39 s**, the
`ilang-check --test projects` group **86 s** on a warm target, and the spec
suite **over 20 min** (still running when measured). The 40-minute cycle is
a warm rebuild of the touched crates plus an affected test group; it is not a
clean build and not the full suite. B and C share the binding generators and
metadata encoder, so they merge B then C; D needs B's export generator.

The phases can be cut for scope only by dropping a product decision, not by
leaving two syntaxes: A+B+C are the minimum for the owner's decision; D can
ship after 0.2.0 only if 0.2.0 is allowed without JS values and non-Rust
bindings (would need an explicit waiver).

**Release gate**: 0.2.0 is tagged only after E, with:
`cargo test --workspace`, `scripts/dev-verify` and the review corpus green;
no retired name in `docs/spec.md`, `lib/`, `std/`, `examples/`, `tests/`,
`crates/`, `bindings/`, `scripts/` or `docs/content/`;
`library.out` has no implicit name; the binding goldens match.

## 8. Acceptance rules and cases

Every row is a test file and an assertion, not a promise. New fixtures live
under `tests/spec/10/` (syntax/product), `tests/spec/10.1/` (imports),
`tests/spec/10.4/` (hostcall), plus `crates/ilang-check/tests/projects/` for
project-level errors. Names are fixed here so the tests can be written in
parallel with the implementation.

1. **Product**: missing entry E0617; bad `product` value E0613; program+`@export` E0330; library+`[entries]`
   E0614; program dependency E0615; `i run` a library E0331; native build of a
   library E0332.
2. **No name rule**: a library with `main` in `src/main.i` builds as a
   library and its `main` is not invoked; a program with `add` runs only
   `main`.
3. **Export**: `@export` on a binding/local/type E0338; wrong type
   shape E0304; `@export("sync")` + Clock sleep E0339; duplicate `@export`
   E0309; export in a dependency is not in the root's metadata; `i build NAME`
   on a library is E0331.
4. **Import**: service shape E0310; `@import` on a non-service type E0327;
   namespace `js` as a service E0328; `ilang:` outside `i` E0306; two
   same-namespace+label services E0311; import in a macro need E0336; missing
   type export E0329; wrong argument count E0326.
5. **Accessor**: `make_Logger` is synthesized once and exported; a user
   `make_Logger` is E0208; a fake in another module uses `Logger(..)` and
   works when the function takes the service; a fake field through `call`
   gives a ready Call; `library.out` shows the accessor once; no files appear
   under `src/`.
6. **Hostcall**: importing `i/request` or `i/hostcall` without the key E0323
   (both modules, message names `i.toml`); all rows of §5.4; a runtime panic
   fixture asserts the exact text; `i deps`/`i build` output golden.
7. **No leak**: no implicit name appears in a `<-` need row of `library.out`
   (value parameters/results may keep them); `library.needs.out` has all 240
   occurrences; exporting a function returning `Call(Int)` E0324; an enabled
   dependency module-exporting `f : () -> Call(Int)` E0324; generated
   `.d.ts`/`.h`/`.rs` never encode an implicit need.
8. **End to end**: `tests/host/library/` is a Rust host embedding a library
   with an `@import` service and the three exports of Appendix C (one
   fallible/async, one `"sync"`); the host registers a fake, calls them, and
   checks results/errors. A separate i test compares two accessor results
   field by field. A JS twin runs the host side in Node. A WASI twin runs
   under wasmtime with a C shim.
9. **Retired syntax**: `@external`, `@host`, `@js`, `[wasm]` each give E0337/
   E0612 with the replacement; `git grep -F` proves none in `lib std examples
   tests crates bindings scripts docs/spec.md docs/content`. Historical design docs
   keep the old names under a one-line "predates 0.2.0" banner and are checked
   only for the banner.

## 9. Approval items

| # | Choice | Alternatives (cost) | If unchanged |
|---|---|---|---|
| P1 | optional `product`, default `"program"`; `"library"` explicit | required key (2 209 manifest edits); `[package] kind` (same plus a table) | §1/§7 change |
| P2 | `@export` on functions only, `"sync"` argument | export whole services too (object protocol; +1 phase) | §2/§7 change |
| P3 | same-module `make_<Type>` accessor | a separate `src/gen/` module (editors see a file, but its own module calling it makes a cycle); `impl_<Type>` name (shorter) | §3/§7 change |
| P3 name collisions | E0208 (existing duplicate definition) | a new code (one more message) | trivial |
| P4 | library-only `@runtime` for compiler ops | reuse `@import` with `"runtime"` mode (one name fewer, no registration) | §4.2 changes 147 lines differently |
| P5 | compile vs panic split of §5.4; `under`/`waiting` names | linear `CallGroup` (new feature); `in_group`/`with_wait` names | §5 tests change |
| P6 | one phase for implicit services + hostcall | two phases (106 signatures twice, +1 cycle ≈ 40 min) | §6/§7 |
| P7 | implicit services may not be written at all (E0325) | allow writing them (source stays dirty; no lint against drift; display hides them anyway) | §5.5/§6 |

No approval is needed for: deleting all old syntax, `i/hostcall` import
error, the misuse outcomes, the synthesized accessor, the retired-name
errors, the release gate.

## 10. What is not changed

- The §B.2 value encoding, §B.3 buffer ownership, opaque `externref`, error
  tags and the async entry protocol (status 0/1/2, request ids, resume export
  names) keep their bytes. Only the import/export *tables* and their metadata
  names change.
- `Fail`/host error handling, traps, stops and cancellation ownership stay as
  specified; `i/hostcall` reuses `i/request`'s cells and `resource.ensure`
  cleanup, not a second mechanism.
- Host registration is keyed by namespace and label today and stays so; the
  generated stubs are typed on top of it.
- Macros keep their expansion rules; only the caller-origin attribute set and
  the macro-need exclusion change.
- `i.lock`, minimal version selection and repository URLs do not change.
- No `edition` key, no old-syntax mode, no `i fmt --from-old` migration tool.

## Appendix A — the implicit-signature corpus

The 88 figure counts `Await`/`Owner`/`Root` exported signatures only. The full
set, with `Scheduler`/`Runner`, is 106 exported lines and 240 occurrences:
103 `Await`/`Owner`/`Root` lines minus 15 private leaves 88 exported (4 of the
88 are mixed); `Scheduler`/`Runner` add 23 exported lines and 50 occurrences,
5 of which are already in the 88, so +18 and 106.
`crates/ilang-check/tests/types/library.out`:

`i/request` 15: 26,27,28,29,30,31,32,33,34,35,37,38,39,47,48
`i/task` 5: 347,365,369,374,375
`i/clock` 3: 419,420,421
`i/fs` 14: 529–536,539–541,543,544,546
`i/process` 14: 553–560,562–567
`i/net` 16: 568–576,579,581–584,586,587
`i/tls` 9: 594–602
`i/fetch` 5: 603–607
`i/stdio` 7: 608–614

Private signatures also affected: `request.i` 53–56; `task.i` 230,260,264,266,
346,373,376,378,417; `fs.i:542`; `tls.i:593`.

### A.2 Supplied/platform sites and fixtures

Platform citations: `docs/spec.md:458-460,1340,1580,1592,2472-2482`;
`platform.rs:1-6,73-77,95-171`; `resolve.rs:1405-1497`;
`lower.rs:973-982,1040-1051,1568-1572,1611-1648,2215-2225,2637-2655`;
`emit/host.rs:9-13,218-235,601`.

Fixtures to re-bless or rewrite: the 9 `tests/spec/9.11` files and 7
`tests/spec/9.9` files that name them (`race_then_read`, `cancel_ready_await`,
`cancel_points`, `kept_services`, `async_tasks`, `nested_borrowed`,
`final_error_rejected`);
`tests/review/cases/review-js4/{settings,bootstrap}`,
`review-native3/_loose/design-cases/{enumeration,borrowed-request,cancelled-read}`,
`review-r2/{pure_fake,request_groups,wait_any_unlisted,borrow_owned,request_reuse,request_stop}`,
`tests/browser/fixtures/r2`; the 9 `expected.json:13` lines and
`tests/review/baselines/pre-w520.json`.

## Appendix B — generated binding shapes

`build/<name>.bindings.json` keeps the existing plan and changes `kind` to
`"import"` for imported fields and `"export"` for exported functions, with
`"mode": "now"|"later"`; the ABI `services`/`externals` maps become one
`imports` map keyed by namespace then label, each with `params`, `result`,
`errors` and `mode`. Existing duplicate/overlap checks collapse to
namespace+label uniqueness.

`build/<name>.rs`:

```rust
pub trait Log { fn log(&self, text: &str); }
pub fn log(instance: &mut Instance) -> LogImpl<'_> { … }
pub struct Exports<'a> { instance: &'a Instance }
impl Exports<'_> { pub fn add(&self, a: i64, b: i64) -> i64 { … } }
```

`build/<name>.d.ts`:

```ts
export interface Imports { "app:log/v1": { log(text: string): void } }
export interface Exports { "app/log.add"(a: bigint, b: bigint): bigint }
```

`build/<name>.h` uses the §B.2 layout with `ilang_str`, `ilang_bits`,
`ilang_int` and one `extern` per import; exports are `ilang_export_…` with the
status rule of §B.5.

## Appendix C — a complete example

```i
// library package app, src/lib.i
@import("app:log/v1")
Logger = (log = (String) <- (Fail(String)) -> Nil)

@export
add : (Int, Int) -> Int = (a, b) { a + b }

@export
add_logged : (Int, Int) <- (Fail(String)) -> Int = (a, b) {
	logger = make_Logger()
	logger.log("add")
	add(a, b)
}

@export("sync")
version : () -> Int = () { 1 }
```

The library module export group is `(Logger(..) add add_logged version)`;
`make_Logger` is added to it synthetically. The root:

```i
(lib = app/lib)
main = () {
	lib.add_logged(2, 3)
}
```

Rust host:

```rust
let mut imports = HostImports::new();
imports.service("app:log/v1", "log", |text: &str| println!("{text}"));
let mut instance = Instance::new(&wasm, imports)?;
let exports = Exports::new(&mut instance);
let answer = exports.add_logged(2, 3)?;
```

The synthesized `make_Logger` is an export of `app/lib`; the generated
`build/app.rs` contains the `Logger` trait, the `add_logged` wrapper and the
registration helper.

## Appendix D — retired error catalogue

| Code | Meaning |
|---|---|
| E0323 | `i/hostcall` or `i/request` imported without `hostcall = true` |
| E0324 | non-boundary type in an `@export` |
| E0325 | implicit service written in a need row |
| E0326 | wrong `@import` argument count or form |
| E0327 | wrong `@import` place, or `@import` on a non-service type |
| E0328 | `js`/`ilang:` used where not allowed |
| E0329 | `@import` service type not exported with `(..)` |
| E0330 | `@export` in a program |
| E0331 | `i run`/`i build NAME` on a library with no target entry |
| E0332 | native build of a library |
| E0334 | native run/test with an unsupplied user import |
| E0335 | `call` closure without exactly one imported field call |
| E0336 | `@import` in the needs of a macro |
| E0337 | retired attribute `@external`/`@host`/`@js` |
| E0338 | wrong `@export` place or argument |
| E0339 | `"sync"` export that needs a later service or the scheduler |
| E0612 | `[wasm]` table present |
| E0613 | invalid `product` value |
| E0614 | `[entries]` in a library |
| E0615 | program dependency |
| E0616 | non-boolean `hostcall` |
| E0617 | a program has no entry |

**Existing codes.** Kept and retargeted: E0301 (unknown attribute; its message
lists the four new built-ins), E0302, E0303, E0306, E0307, E0308, E0309, E0310,
E0311, E0312, E0315–E0319 (import types), E0320 (field type), E0322. Removed
with the mechanism: E0313 and E0314 (a `[wasm]` export could be unknown or
also the entry; `@export` cannot), E0321 (`[wasm]` duplicate; `@export` twice
is E0309). E0304 runs first; E0324 is the second pass on a shaped signature.

**Numbering.** E0323–E0339 and E0612–E0617 are unused at `a21eae769`,
confirmed by `git grep`.
