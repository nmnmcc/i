# Design: JS bindings, target providers, and checked WASI adapters

Status: revision 8, after the confirmation review of e821f9a, 2026-11-10.
This note applies J1–J8, K1–K5 and L1–L4. Track JA has installed the owned source amendments listed in §9; that table records the items left out. This is not an implementation claim. Revision 8 merged redesign at 681ad65; JA starts at 7b3cea3. The current compiler printer remains until the provider implementation and parity tests land. Removing externals, merging host forms, portable
arbitrary foreign values and callbacks remain outside this revision.

Sources, in order: the user's rulings; `docs/spec.md`; accepted `syntax.md`,
`runtime.md`, `meta.md`, and the now-landed `native.md`/`multicore.md`; the
reviews `.rd/cache/gpt/report-review-js.md`, `report-review-js3.md` and
`report-review-js4.md` (full last report `/tmp/review-js4/review.md`); the old
note. The plan's “已定的设计”, IO/native tracks and the coordinator ledger were
read. The ledger accepts the WASI exception, provider/contract spellings,
F2 certification and P1's separate compiler track; these are not open questions.
Code anchors below were checked against merged redesign 681ad65. Historical
probe results are labelled separately. All i examples use accepted syntax.

## 1. Problem and criteria

i runs on its own under `i run`. A browser or Wasm embedder is one place to
run a program, not the definition of the language. A JS build should supply
its loader and bindings. A WASI build should retain its command behavior and
custom imports. Application and standard-library code should use ordinary
values, functions and environments on both.

The old note left six contracts inconsistent: whole-service requirements
versus partial platform support; JS objects versus integer IO resources;
other-target foreign bindings; request result/error encodings; memory
ownership; and macro execution. It also claimed a kernel only moved values,
although invocation, validation, identity, cancellation and cleanup need
fixed implementations.

Choose by sound sets of values, principal inference, ordinary library code,
standalone behavior, compatibility with Appendix B, complete lifetimes,
structured concurrency and a small implementation that can be tested before
IO integration. Considered designs:

| Design | Benefit | Reason not to choose it unchanged |
|---|---|---|
| Large host services with unsupported stubs and opaque IO objects | Closest to the older runtime table and SDK | Violates J1 and J2; requesting entropy also requests child processes |
| One raw root per target, integer pointers, arbitrary promise results | Small-looking surface, libraries write all protocols | Double free and stale addresses are unsound; constructed errors cannot cross §10.3; native has no accepted universal root |
| Infer only the fields used, erase unavailable values, optimize away other targets | Fewer apparent requirements | Changes whole-service inference and reference identity; optimization cannot justify an observable null |
| Capability-sized providers, checked foreign kernels and ordinary wrappers | Keeps inference and ABI value sets; makes missing capabilities explicit | Needs a specified link pass and checked adapters; chosen |

Keep the first design's explicit SDK factories for resource operations, the
second's ordinary library composition, and the third's removal of *proven*
unreachable code. Do not infer a different type on each target.

For this confirmation revision, three construction designs were compared
before choosing the replacement:

| Design | Criterion and decision |
|---|---|
| Fixed Root/Owner/Await prefix, then other factories | Small wrapper, but Config -> Await's construction prerequisite cannot be supplied; reject (review 4.1/L1) |
| Let adapters supply final services and automatically compose adapters | Hides overridden factories and leaves order/lifetime ambiguous; reject (L2, spec §9.2) |
| One graph with typed adapter seeds and explicit factory arguments | Chosen: every dependency is visible, overrides still execute, one adapter has one guarded lifetime |

Keep the first design's dependency-free primitive admission and the second's
ordinary scoped function. Use existing List(S) for optional seeds, not a new
primitive value type. Resolve only build provisioning after inference; never
specialize source schemes per target. T9 supplies the exact choice rather than
leaving these alternatives to implementers.

The new mechanisms are general. Foreign values serve DOM, cryptography and
numerical packages. Raw outcomes serve rejected promises, synchronous foreign
exceptions and third-party job APIs. Checked buffers serve file output,
packet codecs and compression. Entry providers serve clocks, entropy and
application logging. None requires the checker to recognize a library name.
A nominal type is still a set of constructor values or opaque host values;
all functions keep ordinary value/error/environment types. There is no new
subtyping, field-sensitive service inference, keyword or unstructured task.

## 2. Target provisioning and capability split

**T1 — Entry providers are ordinary values. [J1]** A target build has an
explicit provider plan mapping resolved service type identities to ordinary
i factory functions or existing host registrations. This is build/entry
provisioning, not name resolution or a special lowering for `i/io`. Apply the
same mechanism to a user-supplied provider plan. The shipped plan supplies
standard defaults; it does not discover arbitrary package services by name.
The compiler checks every factory's ordinary type and its dependencies;
provider dependency cycles are build errors. No provider is a new language
attribute. T6 gives the public per-target selection format required by K3.

Infer source types first. An environment statement removes a requirement in
the usual way (spec §9.2). For remaining entry requirements, instantiate a
whole service, never selected fields. Construct each T9 graph node once per
entry invocation, after top-level initialization, and capture the root/host
values it uses. An adapter has outer and body scopes; a service needed in both
has one instance in each scope. Replacing `Js` later does not change an `Io`
already constructed.
A function that reads `Js` at call time still gets the caller's replacement.
Provider construction cannot introduce async work outside the R2 entry
context. Factories that need requests run inside that context.

A required standard capability with no default and no explicit provider is
a compile error naming the target, service and requiring entry. The user
accepted this cost verbatim: **“接受这个代价” (J1)**. For example:

```i
(i/fs, i/fail)


main = () {
  fail.catch(() { fs.read_file("settings.txt") })
  Nil
}
```

On `js`, the missing `Fs` default is a compile error even though this code
catches every `FsError`, including `unsupported`. Catching an error never
removes a service requirement. Choose an application-service provider per target (T6), use a Target branch
whose payload can actually supply its needs, or give an explicit service
before it reaches the entry. Smaller services
make entropy usable; they do **not** restore this fallback pattern.

**T2 — Split by capability, not module. [J1]** Use the following exact split
of runtime §7.4; keep the functions in `i/process` and their existing result,
error, argument-codec and cancellation rules. A wrapper requires only the
service in its row, plus R2 `Await | Owner` when it waits.

| Service / namespace | Raw fields, with runtime §7.4 signatures except J2's Int resources |
|---|---|
| `Entropy` / `i:entropy/v1` | `entropy` |
| `Arguments` / `i:arguments/v1` | `args` |
| `Environment` / `i:environment/v1` | `env` |
| `WorkingDirectory` / `i:working-directory/v1` | `cwd` |
| `Process` / `i:process/v1` | `spawn`, `wait`, `terminate`, `close` |

Keep `Clock`, `Fs`, `Net`, `Stdio` and legacy `Io` separate. `Arguments` and
`Environment` do not drag in `cwd`: preview 1 has no general cwd query.
Entropy, Arguments, Environment and WorkingDirectory have public service
constructors for ordinary fakes. Process follows N9's private raw fields.
There is no aggregate Process requirement on `process.entropy` or
`process.args`. Keep `ProcessError` for these wrappers; changing error names
is unnecessary. This decomposition supersedes runtime §7.4's single service.

**T3 — Defaults describe shipped implementations, not theoretical platform
support. [J1, J4]** The initial synchronous milestone supplies only `Io`,
`Target` and the relevant root. Add the following planned defaults only when
their implementations and tests ship:

| Target / CLI | Root available through Target | Defaults after IO integration |
|---|---|---|
| `Js` / `js` | `Js` | Io, Clock, Entropy |
| `NodeJS` / `nodejs` | `Js` | Io, Clock, Entropy, Arguments, Environment, WorkingDirectory, Fs, Net, Process, Stdio |
| `Wasi` / `wasi` | `Wasi` | Io initially; Clock, Entropy, Arguments, Environment, Fs, Stdio require separately approved preview-1 adapters |
| `Native` / `native` | None in this revision | Accepted native runner's ordinary standard registrations, split as T2 |
| `Host` / `host` | None | Explicit embedder/provider plan; no claim of arbitrary defaults |

A supported JS environment must supply the declared runtime feature profile
(WasmGC, tail calls, exception handling, console, clocks/timers and, at the IO
milestone, secure entropy). This is not a claim about every JS engine or every
browser context. The generated loader checks its required profile before
starting. No insecure random fallback. Deno/Bun compatibility is unverified;
Node compatibility is not inferred from their names. Preview 2 is outside
this note. `--target ilang` remains an alias of `--target host` with no removal
scheduled; emitted legacy host artifacts keep the current Appendix B format.
This preserves native N1's explicit `ilang` usage while adopting the user's
preferred spelling.

**Command selection. [L4]** Execution location and provider selection are
separate. The native column does not require an AOT executable.

| Command | Provider column / Target value | Validation and execution |
|---|---|---|
| `i run` | native / Native | Validate the selected entry plan; execute in process |
| `i test` | native / Native | Validate test entry plans; execute in process with the normal fresh test instances |
| `i build` | selected target; default native | Validate that target's entry plans; emit its artifacts |
| `i check` | none | Parse, resolve and infer all source, including test bodies; no target plan selection |
| `i check --target t` | t, with ilang normalized to host | Check source and the plans for that target's main, configured exports and eligible tests; emit and execute nothing |

WASI has no compiled test entries under B.7: its check still checks test source,
but constructs no test wrapper/plan. Targeted checking validates supplies,
adapter types/conflicts, cycles, synchronous contracts and F2's foreign
availability. Untargeted checking promises none of these: success does not
promise that any target can build or run the program. Neither check form
promises actual host grants, engine features, successful initializers, or
runtime results. Malformed manifest syntax is still a package-load error.
The target-independent source checker does not reject an entry merely for
having an ordinary service requirement (P2). Settings therefore reads the
file under run/test, rather than silently selecting the host fallback.

**T4 — Target is an ordinary nominal service value.** Proposed `i/target`:

```i
(js = i/js, wasi = i/wasi)

(Target(..))

Target {
  Js(js: js.Js)
  NodeJS(js: js.Js)
  Wasi(wasi: wasi.Wasi)
  Native
  Host
}
```

Its configured entry provider constructs the actual target. Native is
nullary here: an invented `native.Native` root would contradict accepted
native N3/N4 and runtime §8.1. A future provider bundle needs its own design;
packaging does not depend on it. Host is nullary because an arbitrary
embedder's services cannot be enumerated by the compiler.

This complete example covers all five constructors and needs only Target:

```i
(target = i/target)


main = (t ! target.Target) {
  t |> {
    target.Js(_) -> Nil
    target.NodeJS(_) -> Nil
    target.Wasi(_) -> Nil
    target.Native -> Nil
    target.Host -> Nil
  }
}
```

In a JS clause, bind `! js.Js = j` from its payload before calling JS code;
in a WASI clause, bind `! wasi.Wasi = w`. These branches can discharge their
root requirements. Native and Host clauses cannot discharge arbitrary needs
from their tags. Any such unsupplied requirement remains in the principal
entry type on **all** targets. A test may supply any constructed Target. F2 treats such a value as unknown;
it never assumes every environment-bound Target is the build default. F2
recognizes only the certified entry binding and its lexical aliases/captures.

The printing example has its required import:

```i
((Io) = i/io)


main = (io ! Io) { io.print("Hello from i!") }
```

It works where Io is provided, including the standalone runner and WASI.
An arbitrary Host embedder still has to provide it (spec §9.7).

**T5 — Custom imports are not missing standard defaults. [J4]** Preserve
spec B.7: required custom WASI services and externals stay imports, with their
requirements reported at build time. A suitable embedder may supply them;
plain preload is sufficient only for the scalar types listed in B.7.
Request-mode custom imports remain blocking and return Ready requests under
R2, with no async hand-off. An explicit host provider plan can also satisfy a
standard service; absent that plan, J1 applies. Do not extend native N4's
closed executable rule to WASI. JS generated artifacts, in contrast, must
supply all their required imports or fail the build; arbitrary embedding uses
Host. None of these policies changes legal service substitution in i.

**T6 — Application providers are selected before checking their needs. [K3]**
The root package may select one provider per service and target in `i.toml`.
The compact form uses an ordinary fully qualified type name as key and an
accessible factory name as value. T9 specifies the complete validated schema.
The shipped capability registry is data in the plan:
registered capabilities with no provider for this target do not fall through
to an implicit host import. This applies to user-registered capabilities too;
open custom-host bindings follow T5. No module spelling determines either policy. Resolve names through the package graph, never by
recognizing a module spelling. In the compact form factories have no ordinary
arguments. In the full form
T9 lists each generated argument and its type. All factories return the selected
service type, have no uncaught typed errors, and may have ordinary environment
needs. They may stop. Error-producing setup belongs in an ordinary
service operation or is caught by the factory; provider construction cannot
silently add an error to a source entry's host-export signature (§10.3).
Their bodies and types are checked on every target. Only the chosen factory's
inferred environment is recursively provisioned on that target. A reference
to an unchosen factory in ordinary executable code is still governed by F2;
a provider row alone is not such a reference.

A root-package target cell overrides the shipped factory cell for that
resolved service constructor, retaining the row role and adapter (T9). An
absent cell uses the shipped cell if one exists; otherwise
report an unsupplied service. Duplicate rows and dependency cycles are errors.
There is no wildcard, implicit service discovery, dependency-package override,
or inferred effect deletion. Ordinary local environment substitutions keep
§9.2's meaning. The plan checks the full chosen provider graph, before F2;
link pruning cannot remove provider dependencies or change entry classification.
The same selection rule works for storage, configuration, telemetry and clocks.

Complete proposed package, `i.toml`:

```toml
name = "app"

[providers."app/main.Settings"]
js = "app/main.fallback"
nodejs = "app/main.filesystem"
native = "app/main.filesystem"
wasi = "app/main.fallback"
host = "app/main.fallback"
```

`src/main.i`:

```i
(i/fs, i/request)

(Settings(..), filesystem, fallback)

Settings {
  Settings(read: () -> BitArray ^ fs.FsError ! request.Await | request.Owner)
}

filesystem = (f ! fs.Fs) {
  Settings(read: () {
    ! fs.Fs = f
    fs.read_file("settings.txt")
  })
}

fallback = () { Settings(read: () { <<>> }) }


main = (settings ! Settings) { settings.read() }
```

The principal environment of main is Settings | Await | Owner on every
target. The filesystem factory needs Fs; the fallback needs nothing. NodeJS
and Native select filesystem and supply the real granted Fs registration.
Their closure is async under T7. The other three targets select fallback;
their closure contains no Fs and no request service, so it is synchronous.
The same code reads the real file on Node/Native and computes the empty bit
array in the browser (main's value is ignored as usual). It type-checks and
links on all five targets under this proposal when the named standard Fs
implementation has shipped on Node/Native. It does not promise runtime file
existence: ordinary FsError still stops main. No dummy Fs, raw-number exposure,
handwritten JS, Native payload or new effect inference is involved.

This uses T1's already-general provider mechanism; K3 makes its user-facing
selection mandatory. The new package keys require the explicit §2.6 amendment
in §9; today's reader ignores unknown root keys
(`crates/ilang-package/src/manifest.rs:114–129`). It is a design fixture, not a
claim that this package builds with the current compiler.

**T7 — Entry classification follows the complete provider graph. [K1]**
For each entry, seed the graph with every service in its inferred environment.
Choose providers for the build target using T6. A factory node has edges to
all services in its own inferred environment, even if it only captures them;
continue until terminal host or entry-context providers. Sharing a node does
not remove any dependency. Reject unresolved needs and cycles before lowering.
The entry is async exactly when this transitive closure contains a request-mode
service provider, Scheduler provider or Runner provider. Normalize the existing
async host declaration to its request-and-await provider before classification.
Await, Owner and Root alone do not qualify. Each service node's declared
request/async host mode counts even when an ordinary factory supplies that
service: a fake provider for Remote itself still classifies async. A pure
Bundle provider with no Remote dependency does not qualify merely because its
function type returns Request values. A local environment substitution that
removes Remote from inferred needs creates no Remote provider node. There is
no inspection of library names, function bodies, selected fields, or whether
an operation will actually wait.

The last two provider categories are general plan roles, not tests for the
names Scheduler/Runner or their modules. A service row with the scheduler or
runner role names a typed structured
adapter under T9; user services may use the same roles/adapters. A factory
override retains that row role and must satisfy its service type. Two distinct
adapters reaching one entry are an error naming both rows, not a composition.
[L2] The shipped
Scheduler/Runner providers select the accepted FIFO adapter, path [], seed 0,
with no entropy draw. A request leaf supplies its mode from the host declaration;
no ordinary provider can erase that leaf's classification. Merely naming a
service Scheduler is irrelevant. Providers do not change source value sets
or principal types.

Explicit substitutions contribute only the requirements left by ordinary
inference. A factory that locally constructs a fake raw service has no raw
provider dependency; one that captures a provisioned raw service retains it.
A caller constructing a bundle manually needs whatever the ordinary construction
and calls require. Low-level code with no qualifying provider that tries to
hand off still gets runtime's `request: no active entry` stop. This is not an
unsound guess that a pure-looking closure can never call a low-level wait.

For an async entry, open the primitive admission context before executing
any chosen factory. Opening it is not a service factory, cannot be replaced,
and has no service dependencies. It supplies no chosen Root, Owner or Await.
Construct **all** replaceable service values in T9's single topological order,
including context providers and prerequisites supplied inside the adapter.
No special service prefix is allowed. Close the primitive context on every
exit; a factory stop still runs no i finals. A synchronous entry has its
primitive registry but no async admission and
may use completed fakes. [L1]

This deliberately classifies a whole bundle: after the request milestone,
even a converter-only Js use or a Target payload containing Js can make an
entry async. No selected-field exception is allowed.

A previously synchronous export that needs such a bundle becomes async, even
if its body only returns 7. Metadata says `async: true`; it has a status-returning
entry and an `ilang_result:` export. To request a build-time guarantee, a root
package may list exports in `wasm.synchronous` (a subset of `wasm.exports`).
A listed export whose provider closure is async is a compile error naming a
qualifying dependency path. There is no implicit synchronous promise in the
ordinary source type, no silent blocking adapter, and no new source attribute.

The coordinator accepted the B.7 exception: this replaces D5 on async-protocol
targets (Host, JS/NodeJS and native v3). WASI retains blocking imports and
synchronous metadata, including custom imports. No async WASI ABI is proposed.

**T8 — Wrapper and metadata examples. [K1]** The wrapper order for each async
example below is fixed (the following is algorithm notation, not an i API):

```text
open the dependency-free primitive admission context
execute the one topological order of the complete chosen graph (T9)
  construct prerequisites before consumers, including context providers
  enter the sole adapter when its outer prerequisites are available
  construct seed-dependent providers inside its guarded continuation
call the unchanged source entry
finish structured children/finals; close outstanding request groups
save the entry result/error and return status 0
```

A wait inside construction or the source call returns status 1/2 and resumes
this one wrapper through the v3 root continuation; suspension does not end
the adapter continuation's dynamic extent. Error/stop behavior remains
that of the entry; a stop skips i finals. No provider executes before admission.
For the promise example in §10,
main needs Js | Await | Owner. The Js provider depends on JsSync and
JsRequests, so it is async. `app/main.main() -> i32` returns the v3 status;
`ilang_result:app/main.main() -> ()` has no result because main's value is
ignored. The observe reply enters through
`ilang_resume:i:js-requests/v1:observe(id: i32, outcome: externref) -> i32`. The generated wrapper opens a direct request
context, builds JsSync/JsRequests and their bundle, then runs main; there is
no Scheduler/Runner dependency and therefore no task scheduler. A possible
metadata fragment is:

```json
{"entry":{"name":"app/main.main","async":true}}
```

An equivalent user bundle, with no JS or standard-service names in the rule:

```i
(i/request, i/resource)

(Bundle(..), Remote(..), bundle)

@host("app:remote/v1", "request")
Remote { Remote(fetch: () -> Request(Int)) }

Bundle { Bundle(fetch: () -> Int ! request.Await | request.Owner) }

bundle = (remote ! Remote) {
  Bundle(fetch: () {
    r = remote.fetch()
    resource.ensure(
      () { request.await(r) }
      () { request.cancel(request.ticket(r)) }
    )
  })
}


main = (b ! Bundle) { b.fetch() }
```

On Host, select Bundle's factory `app/main.bundle` and explicitly register
Remote with the embedder. The graph is Bundle -> bundle -> Remote(request),
plus Await/Owner. The wrapper opens the same direct context, binds Remote,
constructs Bundle, runs main and cleans up. Its main/status and result exports
have the same shapes as the promise example. Its fetch reply export takes
`(id: i32, sign: i32, address: i32, count: i32) -> i32`, the ordinary Int wire
result after its request id. Its entry metadata is the same
`async: true` shape above; the service metadata has
`app:remote/v1.fetch = {params: [], result: "int", errors: [], async: true}`.
Remote is exported because it occurs in bundle's inferred environment
(spec §2.4). It is a normal public service for fakes, not a private
resource-number service. Its provider selection is:

```toml
name = "app"

[providers."app/main.Bundle"]
host = "app/main.bundle"
```

For a synchronous-use export in that module, add this ordinary exported binding:

```i
answer = (b ! Bundle) { 7 }
```

Include answer in the exports and configure
`wasm.exports = ["app/main.answer"]`. Its explicit environment parameter
makes Bundle a requirement even though b is unused. With the same provider,
the wrapper opens the context and constructs Bundle before returning 7; it
may finish with status 0 immediately. The ABI entry is `() -> i32` (status),
and `ilang_result:app/main.answer` returns the Int's sign/address/count.
Metadata is:

```json
{"exports":{"app/main.answer":{"params":[],"result":"int","errors":[],"async":true}}}
```

If also configured with `wasm.synchronous = ["app/main.answer"]`, reject the
build; do not emit a misleading synchronous signature. With a pure Bundle
provider instead, it remains synchronous, returns Int directly through the
ABI, and its metadata says `async: false`. No recognition of Bundle is involved.

**T9 — One typed plan schema and one construction graph. [L1, L2]** This
is the full version-1 schema; T6's string cells remain its nullary shorthand.
The package plan is data, not executable macro code. A plan is resolved with
its declaring package's authority: it may name that package's private bindings;
references into dependencies must be exported. Inherited cells keep their
original declaring authority. Copying a dependency's private path into a root
cell is an access error. This gives every package the same plan-only access,
without making a private factory callable from ordinary outside source. It
preserves runtime §6.5's private root-runner constructor. The spec §2.6
amendment includes this rule; it does not change source visibility (§2.4).
Unknown keys, duplicate resolved rows/cells, unknown
targets, ill-kinded type arguments and unresolved names are errors. The five
columns are js, nodejs, native, wasi and host; ilang is a CLI alias only.

```text
providers."<resolved service type constructor>":
  role = "none" | "scheduler" | "runner"                 # default none
  adapter = "<accessible ordinary function>"               # iff role != none
  discriminant = {js=C, nodejs=C, native=C, wasi=C, host=C} # optional
  <target> = "<factory>"                                 # compact form
           | {factory="<factory>", arguments=[...], seed_required=false}
           | {host=true}
provider_adapters."<accessible ordinary function>":
  supplies = ["<closed service type>", ...]
  requires = ["<closed service type>", ...]
wasm.exports = ["<exported entry>", ...]
wasm.synchronous = ["<member of wasm.exports>", ...]
```

Exactly one variant occupies a target cell. `arguments` defaults to []; its
only elements are `"seed"` and `"entry-root-key"`, each at most once, in the
factory's argument order. A string is `{factory=..., arguments=[]}`.
`seed_required` defaults to false and is legal only with a seed argument.
A host cell is legal only for a declared host service; T5's existing open
custom-host registration is its implicit cell. A registered ordinary row
never falls through to host registration. A discriminant row may omit cells;
its default cell constructs `discriminant[current_target]`, provisioning
constructor fields as services. Each constructor must belong to that row's
type, and each field type must be a closed valid service type. Constructor
aliases resolve before validation; maps must cover all five targets but need
not be injective. An explicit cell replaces generation and disables F2's
certification for that row. Generated payloads are dependency edges too.

A row key is a nominal service constructor identity, not its spelling. A
parameterized row's factory scheme is instantiated at each required set of
type arguments, by ordinary type checking; no holes or free variables may
remain in a chosen instance. Distinct `Cell(Int)`/`Cell(String)` nodes stay
distinct. Each factory invocation binds only its own checked environment in
its lexical scope. It does not union incompatible instantiations into one
environment; spec §9.2's same-constructor restrictions still apply to each
actual function environment. This changes no source inference.

A factory for service S has type `(generated arguments) -> S ^ Never ! R`,
with R its ordinary inferred environment. The selected arguments have these
exact types and meanings:

| Argument | Type | Value and dependency |
|---|---|---|
| seed | `List(S)` | [] outside the adapter; inside it, [s] when supplies contains S, otherwise []; a nonempty list depends on the adapter entering |
| entry-root-key | `Key(Nil)` | The initial registered root key of this entry's primitive context, available before factories; no service dependency |

`seed_required=true` requires exactly [s] at this construction site, or the
plan is rejected; it is not a runtime missing-service fallback. A seed is an
ordinary value; the factory decides how to use it. It may return it, wrap it,
or ignore it. Even with a seed, **the selected factory is called** and its
result is the chosen S. A nullary override receives no seed and still replaces
the chosen value. This prevents an adapter from silently undoing an Await,
Root or user-service override. Supplying a seed is not a second provider row.

The root-key argument serializes runtime §5.1's narrow initial-root bootstrap:
open the primitive entry registry, register its initial key when this argument
is present in the graph, then let an ordinary factory construct the nominal
Root value. A graph needing no key registers no initial root. It is not a
public unscoped
root allocator. Any ordinary factory with this argument type can receive the
key; no rule recognizes Root or its module. All uses in one entry receive the
same initial key. The registry exists for synchronous fake-only entries too;
only T7-async entries have async admission. The primitive opening is fixed,
dependency-free and constructs no replaceable service value. On every exit
it retires the entry's registered roots/groups as runtime §5.1 requires.

The **role belongs to the row**. A root-package target cell replaces only the
provider cell; the inherited role, adapter and discriminant are unchanged.
Redeclaring inherited metadata identically is allowed; changing it is an
error. New application rows may declare any role with the same rules. A row
with a role must name a declared adapter, and a row with role none must not.
An override must still type-check as a provider of S; supplying a pure fake
does not erase its role. Request mode likewise remains the declared host mode.
All row/adapter declarations are schema-checked in a target plan; only chosen
factory dependencies need supplies. Source-only check does no plan validation.

An adapter declaration names an accessible ordinary generalized function with
this checked type (schema notation, R and S are the declared service unions):

```text
adapter(A,E,N): (body: () -> A ? E ! N) -> A ? E ! R | N - S
```

It is generalized over arbitrary A, E and N; no extra result/error restriction
or requirement is permitted. The declared closed supplies and requires lists
each form a valid ordinary environment. They may overlap **across** lists:
the required outer Runner and supplied inner Runner, for example, are different
lexical values. A candidate's inferred scheme must satisfy this entire type,
not only the current entry instantiation. There is no rank-two field or new
function kind: the plan names an ordinary top-level generalized function.
The adapter supplies S by calling body under ordinary environment bindings;
those bindings are captured as seeds by the generated continuation. It must
not assume its seeds are the final services used by the source entry.

The continuation has entry lifetime. Generated guards allow exactly one
invocation, wholly within the adapter invocation's dynamic extent. Re-entry
or a second invocation stops with `provider: body already called`; invocation
after adapter exit stops with `provider: body is not active`; returning normally
without a completed invocation stops with `provider: body was not completed`.
A stop may abandon the invocation; it runs no i finals and the primitive
context still retires. Suspension is not exit. Adapters may use ordinary
structured control and resource.ensure around body; they cannot detach it.
The guard is required for every adapter, not trusted library code. Ordinary
service values can escape normally; their later operations obey their own
lifetime rules. This is not a new linear type system.

Resolve and validate the graph as follows, with no choice left to an optimizer:

1. Resolve body providers transitively from the entry's full inferred needs.
   Each service uses its one target cell. Include all factory requirements,
   generated-constructor payloads and declared request modes, whether used at
   runtime or not. Choose roles from these rows, not their factories' names.
   Expansion must be finite: on any dependency path within one scope,
   revisiting a service constructor is rejected even at different type
   arguments (`provider: recursive dependency`). This conservative rule also
   rejects a finitely decreasing chain such as Box(Box(Config)) -> Box(Config)
   -> Config. Independent sibling instances remain legal. It prevents an
   expanding Cell(A) -> Cell(List(A)) plan from hanging the compiler, without
   adding type-level termination analysis. Completed nodes may be shared;
   sharing must not hide a repeated constructor on any graph path.
2. If a role is reached, include its adapter and resolve that adapter's required
   services in an **outer** scope using the same target cells. Include all roles
   and requirements reached there too. Repeat until closed. No adapter means
   no outer scope. More than one resolved adapter identity is an error naming
   the entry and both originating service rows and their adapter names. Repeated
   references to the same adapter are shared. Adapter references must resolve
   to a top-level function binding, allowing only direct lexical binding
   aliases; arbitrary computed function values cannot be plan adapter names.
   There is no nested composition.
3. Make one node per (outer/body scope, fully instantiated service). An edge
   to each required service stays in that scope. An adapter-enter node depends
   on all outer requires. Seed nodes depend on adapter-enter; a body factory
   with a seed argument depends on the appropriate seed if one is supplied.
   Outer factories always receive []; an unsatisfied required seed is an error.
   The primitive context/key is a leaf, never a service-provider dependency.
   Two scopes do not share provider instances, even if their factories match.
4. Check all supplied/required types, missing cells and cycles over this **one**
   graph, including adapter and seed edges. A factory cannot obtain an inner
   seed while constructing an outer prerequisite. Such a requirement is a
   plan error, not permission for a temporary default service. Source factories
   and adapters have already undergone ordinary type checking.
5. Execute a stable topological order. First materialize primitive arguments
   and host-registration leaves in canonical order; these run no i factory.
   Then, among ready service nodes, order lexicographically by canonical
   package/module/type path and normalized type arguments, then outer before
   body. Expand aliases and sort/deduplicate union members recursively when
   serializing closed type arguments; never sort by allocated TypeIds. Prefer
   all ready provider nodes to adapter-enter. Enter the adapter
   once its prerequisites are ready; finish the same order in its guarded
   continuation, then invoke the source entry with its chosen body services.
   No provider may execute before primitive admission is opened when async.

This ordering may construct a body service independent of seeds before adapter
entry; it is then an ordinary lexical capture in the continuation. It constructs
seed-dependent providers after entry. This is still one topological order, not
Root/Owner/Await followed by another sort. No second provider is substituted
to break a cycle. Classification includes both scopes and the adapter's
requirements; linkage includes the adapter and all chosen factory bodies.
A generated wrapper reference is not a *source* reference in F2's certification.

Shipped task rows use the same schema: scheduler/runner roles select one
ordinary adapter with runtime §6.1's run type and supplied
Scheduler/Runner/Await/Owner/Root, requiring outer Runner. Seed-aware ordinary
factories return the supplied values inside that adapter; outside it they
construct the accepted default Runner through its private ordinary constructor
and the direct context services. The plan names that constructor with its
own package authority, never via a public root-runner API. The outer
Runner factory has no required seed and no dependency cycle. Root's direct
factory takes entry-root-key; Owner's direct factory needs Root. Seed-aware
factories still declare their full ordinary environments on both paths; those
requirements are provisioned even when a seed is present. The seeded default
run remains FIFO, path [], seed 0, with no entropy draw. No special factory
elision, role erasure or hidden context provider is permitted.

**T10 — Config/Await, completely elaborated. [L1]** Proposed `src/main.i`:

```i
(i/request, i/resource)

(Config(..), Remote(..), Bundle(..), make_config, make_await, make_root, make_bundle)

Config { Config(label: String) }
@host("app:configured/v1", "request")
Remote { Remote(fetch: () -> Request(Int)) }
Bundle { Bundle(fetch: () -> Int ! request.Await | request.Owner) }

make_config = () -> Config { Config("configured") }
make_await = (cfg ! Config) -> request.Await {
  request.Await((ticket) {
    _ = cfg
    request.wait_any([ticket])
  })
}
make_root = (k: Key(Nil)) -> request.Root { request.Root(k) }
make_bundle = (remote ! Remote) -> Bundle {
  Bundle(fetch: () { request.await_owned(remote.fetch()) })
}


main = (b ! Bundle) -> Int ! request.Await | request.Owner { b.fetch() }
```

Complete explicit host cells (standard-library source names are ordinary
accessible binding references here, not names recognized by the planner):

```toml
name = "app"
[providers."app/main.Config"]
host = "app/main.make_config"
[providers."app/main.Remote"]
host = {host=true}
[providers."app/main.Bundle"]
host = "app/main.make_bundle"
[providers."i/request.Root"]
host = {factory="app/main.make_root", arguments=["entry-root-key"]}
[providers."i/request.Owner"]
host = "i/request.open_owner"
[providers."i/request.Await"]
host = "app/main.make_await"
```

The graph has Await -> Config, Owner -> Root -> key, Bundle -> Remote(request).
No row has a role, so there is no structured adapter. T7 sets async true. The
wrapper first opens primitive admission and obtains the registered key and
host Remote registration. The following is its complete ordinary-i body,
with every factory requirement explicitly bound; the two parameters are the
already-typed primitive leaves. It has **no** inferred environment or error:

```i
entry_body = (remote: Remote, entry_key: Key(Nil)) -> Int {
  b = {
    ! Remote = remote
    make_bundle()
  }
  cfg = make_config()
  aw = {
    ! Config = cfg
    make_await()
  }
  root = make_root(entry_key)
  owner = {
    ! request.Root = root
    request.open_owner()
  }
  resource.ensure(
    () {
      ! Bundle = b
      ! request.Await = aw
      ! request.Owner = owner
      main()
    }
    () {
      request.close_owner(owner)
      request.close_root(root)
    }
  )
}
```

The ordering is dependency-first; in canonical order the chosen root's key
leaf precedes its constructor and Owner follows Root. Await is never built
before Config. If a factory stops before resource.ensure is installed, the
primitive context still retires the registrations; no i final runs. The
outer ABI wrapper discards main's Int, records success and returns status 0,
or suspends/resumes through the same continuation with status 1/2. Metadata
is `{"entry":{"name":"app/main.main","async":true}}`; the status export
is `() -> i32`, the main result export has no result, and the Remote reply
uses the ordinary Int wire after its request id. This is typed elaboration
against the accepted API, not an executed future-R2 fixture.

**T11 — A user role and adapter, with no shipped service. [L2]** A complete
`src/main.i` uses only ordinary nominal values and functions:

```i
(Config(..), Session(..), config_from_seed, make_session, with_session)

Config { Config(label: String) }
Session { Session(read: () -> String) }

config_from_seed = (values: List(Config)) -> Config {
  values |> {
    [value] -> value
    _ -> panic("provider: missing seed")
  }
}
make_session = (cfg ! Config) -> Session {
  Session(read: () { cfg.label })
}
with_session(A, E, N) = (body: () -> A ^ E ! N) -> A ^ E ! N - Config {
  ! Config = Config("inside")
  body()
}


main = (s ! Session) { s.read() }
```

```toml
name = "app"
[providers."app/main.Session"]
role = "runner"
adapter = "app/main.with_session"
native = "app/main.make_session"
[providers."app/main.Config"]
native = {factory="app/main.config_from_seed", arguments=["seed"], seed_required=true}
[provider_adapters."app/main.with_session"]
supplies = ["app/main.Config"]
requires = []
```

The one graph is adapter-enter -> Config seed -> chosen Config -> Session ->
main. Primitive admission precedes it. The typed ordinary body of its guarded
continuation is:

```i
body = (seed ! Config) -> String {
  cfg = config_from_seed([seed])
  service = {
    ! Config = cfg
    make_session()
  }
  ! Session = service
  main()
}
```

Call `with_session(body)` through T9's single-invocation guard. Its type is
`() -> String`, with no remaining requirements or errors. Main computes
"inside" (discarded at the ABI); metadata says async true because Session's
row has the runner role, even though the first ABI call completes with status
0. Replacing Session's factory with a nullary fake still keeps that role and
adapter. Replacing Config with a nullary factory returning Config("override")
causes main to compute "override": the adapter seed does not replace it.
A second row naming a different adapter is rejected with both row names.
The same contract serves transaction scopes, tracing sessions and dispatch
loops; no compiler rule identifies a library, allocator or scheduler module.

## 3. Foreign declarations, linking, and loader lifecycle

**F1 — A declaration holds an opaque JS value. [J3]** The new form is
`@js("expression") name: T`, where T expands to one closed nominal type with
no constructors. No holes, type variables, unions, functions or constructed
resource types. The annotation is required. It creates no i constructor or
cast between nominal types. `i/js.Value {}` is one ordinary use; packages can
define `Element {}` or other opaque types. JS is specific to this attribute;
it is not a target-neutral foreign language feature.

Each retained declaration becomes a zero-argument value import returning
`externref`, with a cache in the loader, rather than an imported mutable
global. Its first and only initialization call is emitted in `ilang_start`,
in §2.5 dependency/module/source order alongside ordinary initializers.
The host parses the text as one parenthesized expression in strict ESM scope.
It may name globals but not generated loader locals, other declarations or i
bindings. The compiler does not type-check JS. Syntax failure is a loader
error before startup; an initializer exception stops startup and no entry
runs. Initializers may have host side effects: the old assertion that *only*
Js acts was false. Use function expressions to defer work to an explicit Js
call. The source listing exposes these initializers as foreign code to audit.
No initializer or synchronous call may re-enter i; callbacks are a later gate.

**F2 — One reference-reachability algorithm. [J3, K2]** This defines both
acceptance and foreign initializer execution. Run it after ordinary type
checking and provider selection, before optimization, with the same result
at O0/O2/O3. There is no per-input specialization, constructor propagation,
constant folding, Boolean evaluation, type-directed branch elimination or
implementation-chosen widening. There is exactly one clause-pruning rule below.

The finite graph has one node for each ordinary binding's initializer, each
source function/clause-block body, each chosen provider body/terminal, and each
foreign declaration. References are resolved lexical definition identities.
Start with all ordinary top-level initializers, emitted entries/exports/tests,
and chosen provider factories. Scanning an initializer preserves §2.5: making
a function literal does not scan its body. A reference to an existing binding,
including an alias or a function passed as data, follows its definition and
activates all function bodies syntactically contained in that definition.
Scanning an activated body visits all expressions and guards, both sides of
conditionals/Boolean operators and every clause except F2's sole pruning case.
A function literal reached in an activated body is itself activated, even if
it is never called. A non-direct initializer scans its nested function bodies
too, since it may execute them. Reads of foreign declarations retain them.

Iterate these fixed reference edges to the least fixed point. Every source
body has one node shared by all callers; a second call adds no specialized
copy. Recursion and mutually recursive aliases terminate by marking nodes
once. References to aggregate bindings activate all function literals in their
definitions, not only a selected field. Higher-order parameters add no inferred
callee set: all supplied function expressions/aliases are already activated
where they are referenced, stored, returned or passed. This closed program
cannot receive an i function from the host (§10.3). Function values supplied
by chosen providers are traversed in the same way. Thus an unknown function
parameter cannot hide an unvisited source closure. No function-value analysis
or optimizer discovery changes this graph.

**The only pruning case.** The provider plan may designate a nominal service
as a build-discriminant service by giving a constructor map for every target.
Its default provider is a generated constructor application from that map,
with independently provisioned payloads. Target is one ordinary instance of
this general mechanism. The compiler uses resolved type/constructor identities
from the map, never the spelling Target, Js, Wasi, or a module name.

A subject is certified only when it is the environment parameter of an actual
emitted entry with that generated default provider, or a lexical alias made
solely by `alias = certified_name` (an annotation/group does not change this).
The entry binding must have **no source value reference anywhere in the
compiled program**, including as a higher-order value or recursive call,
and cannot also be selected as a provider factory. Import/export lists and
entry configuration are declarations, not source value references. Only its
emitted entry designation may invoke it. This syntactic restriction prevents
an ordinary i call from supplying another environment to the same entry body.
An explicit provider override, ordinary parameter, function result, tuple/field
projection or constructor expression is unknown. A helper's own environment
parameter is unknown, even if every apparent caller uses the default. There
is no propagation through actual parameters or return values.

A closure's direct lexical capture of a certified name or alias remains
certified. A newly bound/shadowing name is a different definition and gains
certification only through the alias rule. `! D = value` does not mutate a
previously bound D value; the captured old name remains certified, but later
calls fetching D see the substitution and their own environment parameters
are unknown. `keep_env(f)` activates f by ordinary reference reachability;
it grants no certification to f's environment parameters. A direct capture
of the original certified value remains a capture, including inside keep_env.
These rules use lexical identity, not runtime environment/value analysis.

For the source form `certified_name |> { clauses }` (allowing a one-expression
`{ }` group around the subject), omit a clause's guard and body only
if its pattern selects resolved constructor C and
`C != map[current_target]`, allowing field subpatterns and `name = pattern`.
For a choice, this inequality must hold for **every** alternative, recursively;
any other alternative prevents pruning. Compare resolved constructor identities,
not names or target labels. Maps need not be injective: mapping js and nodejs
to the same Web constructor retains Web on both. [L3] A wildcard, type
pattern or any other pattern is retained. A matching-target guarded clause
is always possibly executed, even if its guard is literally False; retain
later clauses too. A constructor unequal to the actual mapped constructor cannot match
regardless of its guard. No first-match or exhaustiveness reasoning removes any other clause.
Calling a named clause block or passing the subject to a function gives no
pruning, even if that function is an alias. Type-check all clauses first.
Suppressing an impossible clause's references does not remove pattern-failure
Assert behavior for a one-clause binding, or any other source semantics.
This rule changes no inferred requirements. The compiler preserves this
source match marker until the analysis is done; optimizer-introduced matches
cannot gain certification.

Every retained foreign declaration is initialized exactly once in F1 order,
even if an optimizer removes its reads. An unretained declaration is never
initialized. On an unavailable target a retained declaration is a compile
error at its declaration and a requiring reference, never null or a late trap.
Ordinary top-level `forced = foreign` always retains it. Two retained
`@js("({})")` declarations produce distinct objects; reads of one preserve
its identity within its nominal type (P1).

Documented rejections: the review's choose(True)/choose(False) example retains
unused, rejects on WASI and initializes (then throws) on JS. Passing Target to
a helper, fetching it in a helper's environment, or manufacturing the correct
constructor also gives no pruning. An uncalled closure referenced in retained
code still retains its foreign reads. This loss is deliberate and predictable.
T4's direct entry match remains expressible under the certified-entry rule.
The broader rule “any environment-bound Target is the build target” is unsound;
§10 gives the substitution/keep_env counterexample. Do not broaden F2 to accept
it or silently specialize its helper.

**F3 — Artifacts and versioning.** `name.wasm`, `name.mjs` and `name.d.ts` are
the generated JS artifact. The mjs contains the fixed bridge, retained source
expressions and loader; each instance has its own declaration caches and
entry resource contexts. It starts the module once, then calls main or the
requested host export. JS source may use `import("node:...")` inside a
function and await its opaque promise through §5. There are no static import
declarations inside an expression, implicit Node module variables, or
compiler-generated user callbacks. Standard Node factories may use their
own ESM imports. Their source is fixed support code, not user handwritten JS.

Keep the v3 wire for ordinary service/request calls. Put foreign declaration
text, stable package/module/name identity, generated import name, nominal type
identity, source span and availability in a separate versioned target section
`ilang.js` (initial version 1). A matching generated loader requires that
section and `target: "js"` or `"nodejs"`; old Appendix B hosts reject the
non-host target instead of accidentally loading it. The new mandatory target
metadata is an explicit B.6 extension, not an undocumented v3 runtime export.
The value getters are target imports listed there, not additions to v3's
`ilang:host/v1`. A digest binds the JS manifest to its Wasm artifact; it is
not self-authenticating permission for raw IO (N9 §3.1 still applies).

`bindings/javascript` remains the Host SDK. Generated-target and explicit SDK
usage share encoding and request lifecycle code. Host exports always use the
real SDK conversions and d.ts representations, never a dynamically replaced
Js service; a custom outgoing converter cannot change an export's ABI.

## 4. Synchronous bridge and conversion contract

**F4 — Lower operations use admissible boundary types. [J7]** Public calls
need Js. At the request milestone it is an ordinary bundle:

```i
Js { Js(sync: JsSync, requests: JsRequests) }
```

JsSync is the synchronous raw host service (`i:js/v1`); JsRequests is the
request-mode raw service (`i:js-requests/v1`). The target provider builds the
bundle from their ordinary values. Before the request milestone only the sync
part exists. There is no mixed-mode host declaration. Merely constructing
request function values starts no work; only calling a starter needs Owner.
Public wrappers read the bundle; callers do not separately require its raw
services. Thus the Target payload can discharge both sync and async JS use.
The pure `with_int` helper copies this bundle and its sync service with the
outgoing converter replaced. A fake can reuse opaque seed values or replace
an application service above Js; no constructor for Value is added.

`call(f, args)` gives Value or stops for a thrown exception;
`try_call(f, args)` gives Value with error `Thrown { Thrown(Value) }`.
The fixed bridge returns opaque `Outcome {}` for invocation. Its checked
`is_error: (Outcome) -> Bool` and `value: (Outcome) -> Value` accessors let
ordinary i construct/raise Thrown *after* crossing. Thrown never crosses
§10.3 as an ABI error. Non-callable f produces a caught JS TypeError by the
same rule. `call` uses `panic("js: call threw")` with source positions;
`try_call` preserves the exact thrown value, including null and undefined.

A private argument builder uses `Args {}` with `new_args: () -> Args`,
`push_arg: (Args, Value) -> Nil`, `invoke: (Value, Args) -> Outcome`.
Build a fresh array per call, copy the completed argument sequence at invoke,
and never expose the mutable builder through public i/js functions. List
traversal, result decoding and error construction are ordinary i. An Args
value used by a low-level client remains opaque; its identity, not its
contents, is the i value. Host state can change without mutating an i list.
No List or arbitrary constructed error is added to the wire. Forged opaque
outcomes/builders from a foreign host fail brand validation and stop as a
host fault. This is a dynamic host-protocol check, not a new nominal cast.

A synchronous result declared externref is opaque data, even if its JS value
is a Promise or has a `then` property. Do not inspect or await that property.
The SDK currently rejects thenable synchronous results without this type
qualification (`bindings/javascript/ilang.mjs:836–839`); adjust that general
boundary check for **every** synchronous externref result, not just i/js.
Otherwise `value(outcome)` could not return a Promise for A1 to observe, and
a user object's `then` getter could run outside the explicit call. Ordinary
async service starters still adopt their returned promise as work. The raw
Outcome itself is a private branded object without a then property.

Public `nil`, `bool`, `int`, `float`, `string`, `bits` convert into Value.
`to_nil`, `to_bool`, `to_int`, `to_float`, `to_string`, `to_bits` give the
corresponding i type with error `Mismatch { Mismatch }`. Implement each raw
extractor with a primitive error, e.g. `(Value) -> Float ^ Nil`; validate in
the JS bridge *before* returning the Float. The ordinary wrapper maps that
Nil error to Mismatch. This is implementable by §10.3 and avoids using the
SDK's permissive `ofType` predicate as a conversion test.

Exactly one outcome applies to each case below. “Mismatch” means the public
conversion error; “stop” means §11.7, not a catchable mismatch. These are the
real Js defaults; `with_int` and analogous ordinary service-copy functions
may replace outgoing conversions for application calls.

| Case | Outcome | Representation or reason |
|---|---|---|
| i Nil to JS | convert | undefined |
| JS undefined or null through to_nil | convert | Nil |
| i Bool / JS boolean in the matching conversion | convert | boolean / Bool |
| i Int / JS bigint in the matching conversion | convert | Exact arbitrary-precision bigint / Int; never Number |
| i Float / finite JS number in the matching conversion | convert | binary64; negative zero follows existing SDK/spec equality |
| i String / well-formed JS string in the matching conversion | convert | Unicode scalar sequence, UTF-8 at the wire |
| i BitArray to JS, including non-byte lengths | convert | Fresh `{ bytes: Uint8Array, bitLength: number }`, zero padding |
| Correct SDK bit-array object through to_bits | convert | Snapshot copy of exactly ceil(bitLength/8) bytes and exact bit length |
| Wrong JS primitive kind, including Number through to_int | Mismatch | No coercion |
| NaN or either infinity through to_float | Mismatch | Not in the set Float |
| Lone surrogate through to_string | Mismatch | Not in the set String |
| Bare Uint8Array through to_bits | Mismatch | Missing bit length |
| Bad bit length, wrong byte count, nonzero padding or detached bytes | Mismatch | Not a canonical SDK BitArray |
| Getter/proxy throws while inspecting a conversion input | Mismatch | No valid snapshot was obtained |
| Shared-memory-backed byte view through to_bits | Mismatch | No stable snapshot contract; caller must make a private copy |
| Arbitrary JS result or thrown value entering Value | convert | Opaque reference; inspect only with explicit converters |
| i lists, records, functions or File as a declared ABI value | reject at compile time | §10.3; use ordinary codecs or wrappers |
| Constructed Thrown/Mismatch as an ABI error | reject at compile time | §10.3 error list |
| Invalid value from a typed ABI import/export path | stop | Existing `host: value is not correct`; converters do not weaken B.2 |
| Allocation/stack exhaustion while copying or encoding | stop | §11.7 |
| JS call throws and the public operation is call | stop | F4's panic; try_call instead decodes Thrown |

For bit conversion read properties once, validate the private snapshot, then
encode that snapshot; do not validate one getter result and encode another.
Mutation of JS bytes afterwards cannot mutate an i BitArray. Outgoing bytes
are also copies; no view aliases guest linear memory. No operation silently
pads a bit array, rounds an Int, repairs Unicode or maps a nonfinite number.
A plain `@js` value may be a promise or object; it is not thereby an i Float,
String or IO resource. Foreign identity within one nominal type uses reference_id. Distinct nominal
types remain distinct under spec §4.1/§11.4; the current representation does
not preserve that distinction. Prerequisite P1 in §8 must land before this
bridge, for every opaque boundary, not only JS getters. [K4]

## 5. Requests, cancellation and IO ownership

**A1 — Await an opaque outcome, then decode in i. [J5]** The raw request
service has `observe: (Value) -> Request(Outcome)` and
`start: (Value, Args) -> Request(Outcome)`, with Owner added by runtime §5.2.
There is no `Request(Result(Value, Thrown))` import and no general request map.
`observe` adopts an already-started promise/thenable; fulfilled and rejected
values both settle as an Outcome success at the ABI level. Accessor decoding
from F4 runs after `request.await`. Public `await(promise)` therefore has
value Value, error Thrown, and environment `Js | Await | Owner`. It obtains
JsRequests from the ordinary Js bundle (F4), not from another entry requirement.
Use distinct raw JsSync and request-mode JsRequests service types;
do not put mixed host modes in one declaration. The public starter similarly
builds Args in i and invokes the request service. Arbitrary promises may keep
working after cancellation; observe only withdraws delivery.

**A2 — Standard IO starts with cancellation already installed. [J5]** Use
runtime §8.2's standard request factories. The request slot/id and generation
exist before the host starter runs. During that import, the starter reads
`requestSignal()` synchronously, records the immutable entry ResourceContext,
registers abort cleanup and copies input buffers before returning. It then
starts the OS/JS operation. The general start operation calls its JS function
with `(signal, ...args)` under the same protocol, capturing synchronous throws
as rejected outcomes. A function may ignore the signal, but is then only an
uncancellable foreign operation, not a standard IO implementation.

Abort removes the outstanding registration before invoking listeners. Late
completion is ignored using request identity/generation, even after id reuse;
resource-producing late results must also be closed. Completion delivery is
serialized through R2's one root continuation. Status 2 yields a macrotask;
no microtask-only spin. Public waiting wrappers install an **individual request
guard** immediately after start and before any wait. Non-resource-producing
operations use runtime §5.4's ordinary `request.await_owned`, implemented with
resource.ensure; creators use §8.1's await_resource. Borrowed request.await
never cancels its producer merely because its waiter exits. Its final
cancels that handle on normal exit, error or unwind; take the answer before
normal cleanup, so cancelling Taken does nothing. Closing an Owner/root
cancels only outstanding Waiting requests. A Ready cell has already left its
group, belongs to its handle, and survives Owner/root closure. Raw callers
receive that ownership; no implicit per-request guard is installed for them.
This preserves runtime §§5.1/5.4 exactly. [K5] A raw outcome has no implicit IO-resource cleanup:
standard resource factories use the explicit guards and table below. No
promise is a detached i task, and callbacks/re-entry are not authorized here.

**A3 — All IO targets use native N9. [J2]** File, Listener, Stream and Child
have private constructors with one Int field, e.g. inside i/fs:

```i
(File)

File { File(Int) }
```

Raw resource services pass Int. No JS object or fd itself is an i IO resource.
Public wrappers wrap validated returned numbers and unwrap arguments in
ordinary i. Export the service type names needed in public environments,
but hide raw constructors, fields and live field functions. No public helper
may wrap caller numbers or accept a caller-created raw resource service;
otherwise a fake can launder a guessed live number. General nominal field
privacy and N9 §3.1's trusted declaration grants are prerequisites, including
external aliases and import coalescing. A namespace spelling is not a grant.
Use application-level service fakes or explicitly granted host test fixtures.

One ResourceContext per entry invocation owns the resources across its workers,
Roots and nested task runs. A host-wide positive arbitrary Int allocator
never reissues numbers, including failed/provisional numbers; JS uses BigInt.
Fresh tests/export invocations get fresh contexts. Check context, kind and
liveness on every operation; unknown/closed/foreign/wrong-kind gives `closed`,
close gives Nil. OS descriptors may be reused privately, language numbers may
not. Allocation failure stops before exposure and disposes of the OS result.

Lookup, operation leases, completion and close have one commit order. Same
direction competitors give `busy`; completion before close retains its answer;
close before completion wakes it with `closed`. File cursor operations share
one lease; Child has one wait lease. Idempotent close joins the cleanup receipt;
child close waits for reaping. Cancelling close cannot undo cleanup. Keep
runtime's blocking-file cancellation, stdin broker and partial-write rules.

The context owns provisional resources before delivery. A host drop guard
closes an undelivered result; after Ready publication an ordinary i guard
handles abandonment before Take. Take-to-wrapper conversion has no scheduler
step. Forgotten Ready or Taken resources remain owned until close/context end.
Register pending spawn jobs before OS creation so teardown catches late
children. Entry end, stop, trap, host failure, abandonment and disposal close
the start/delivery gate and run host cleanup independently of i finals or GC.
These are N9.1–N9.5, not new JS-specific resource semantics.

**A4 — Keep resource factories, share their implementation. [J2, J5]**
Generated Node builds package the same granted raw factories as the SDK;
ordinary i wrappers still define public service composition, errors and
protocol codecs. A generic Js root cannot safely manufacture Fs from an
arbitrary user-created Js and expose its private raw numbers. Withdraw the
old “every service is built only on Js” promise for resource services. Likewise
native retains its accepted raw namespaces; no Native root redesign is a
prerequisite. Host SDK embedders may explicitly choose unsupported stubs;
a generated target's automatic plan must omit absent capabilities under J1.
This reconciles the factory mechanism with the new default policy without
pretending that OS cleanup can be ordinary i code after a stop.

## 6. Checked WASI memory and the first adapter

**W1 — Buffers are checked tokens, never addresses. [J6]** The public root's
memory fields below are synchronous. Token is an Int internally naming a live
allocation in this instance's dedicated checked-buffer table. The table never
contains ABI argument/result buffers, runtime scratch, GC data or allocator
metadata. Export no operation giving a token's address. Allocation produces
zeroed bytes and a fresh positive token, never reusing one in that instance.
End of entry/startup failure/stop frees remaining checked buffers. A token
from another instance is not a portable memory reference; no library IO value
wraps it. Root APIs expose operations, not the table.

| Field | Type |
|---|---|
| alloc | `(size: Int) -> Int ^ String` |
| free | `(token: Int) -> Nil ^ String` |
| store | `(token: Int, offset: Int, bytes: BitArray) -> Nil ^ String` |
| load | `(token: Int, offset: Int, size: Int) -> BitArray ^ String` |
| fd_write | `(fd: Int, token: Int, offset: Int, size: Int) -> Int ^ String` |
| proc_exit | `(code: Int) -> Nil ^ String` |

This deliberately replaces the old four raw-address fields and the promise
of an unchecked field for every preview-1 function. `Wasi` is an ordinary
service type with this checked signature. Its compiler-provided implementation
is a new adapter, not the existing generic Int ABI lowering. Library helpers
may wrap buffer tokens privately and add structured cleanup, as ordinary code.
Possession of the root grants use of its buffer table; tokens are not secrets.

Validation order is fixed: validate scalar ranges/whole-byte inputs, then
look up a live token, then check its extent. Negative sizes/offsets, an fd or
exit code outside unsigned 32 bits, and non-whole-byte store input give
`invalid`. Unknown/nonpositive/freed tokens (including double free) give
`closed`. A valid token with offset or size outside its allocation gives
`bounds`. Use mathematical checked addition, never wrapping pointer arithmetic.
For empty loads/stores require `0 <= offset <= allocation size`; they do not
bypass liveness. An allocation size above memory32's addressable range gives
`invalid`; exhaustion within that range stops for memory exhaustion.
Successful free removes exactly one table entry before freeing its allocation;
there is no size argument or interior-pointer free. Load returns a copy.

Tokens are identifiers, so `token + 1` is **not** a pointer into token. It may
name another allocation only if that token is live in this authorized table;
there is no promise of unforgeability. To access a subrange pass token and
checked offset. Stale tokens cannot become valid after allocator address
reuse. A call pins its allocation until it returns. These synchronous fields
do not yield; no i callback can free a pinned buffer inside a host call.
For future async adapters, retain a lease or copy before returning; they
cannot inherit raw addresses from this synchronous contract.

**W2 — The kernel understands checked layouts. [J6]** For fd_write, validate
W1 inputs; allocate private scratch for one iovec and a u32 count; translate
the checked slice to the real pointer; call the runtime-owned preview-1 import;
then free scratch on every returning path. On errno != 0, raise String
`errno:<unsigned decimal>`. On success return the number of bytes written,
0..size; a larger count is an invalid host value and stops. No user-controlled
pointer-bearing bytes are ever passed as an iovec. Future adapters require
reviewed pointer/length/output layouts; a scalar-width table alone is not
sufficient. Use a table for the raw i32/i64 signatures, checked signedness and
result shape, with checked layout adapters where necessary. No promise that
every function needs identical code remains.

proc_exit validates its unsigned code, runs context teardown, then calls the
existing raw `(i32) -> ()` import. Its i signature is Nil because §10.3 has no
Never result; successful execution never returns an i Nil. If the host returns
unexpectedly, trap and retire the instance. Invalid code raises `invalid`
before teardown or exit. There is no fabricated errno/result from a void
Wasm call. Teardown does not run i finals; explicit process exit is termination,
not a way to detach work. WASI resource cleanup must finish before this call.

**W3 — Ordinary printing, with existing behavior preserved.** The proposed
Io factory captures Wasi and makes a closure that writes the UTF-8 text, then separately attempts LF even if the text write gave up. For each write it allocates/stores a buffer and repeats fd_write on the remaining slice. Positive short writes advance the offset; zero progress or a reported write error gives up that write; release the buffer on every returning path. Give Nil, never
an IO error, as legacy Io requires. Stdio's error-reporting API stays distinct.
This moves encoding, looping and output policy into i, while W2 keeps unsafe
layout construction inside the kernel. The review's raw-iovec prototype is
evidence for the loop, not verification of this checked adapter.

**W4 — Runtime stops and custom imports stay. [J4, J6]** Keep the independent
runtime fd_write/proc_exit stop path, including startup stops, source positions,
partial writes and exit 1. It cannot call a replaceable i Io. Retain `_start`,
no compiled tests, and blocking custom imports. User declarations of the
runtime-reserved fd_write/proc_exit imports remain errors under B.7; W2 reuses
those internal imports rather than declaring duplicates. Do not remove the
shared runtime write loop when retiring the special Io lowering.

## 7. Macro boundary

**M1 — Extend the existing rule, not macro authority. [J8]** In meta §4.3,
add `js` to built-in attribute names and `@js` to the caller-origin restriction
beside external/host/main/test. A dependency macro cannot invent it; a caller
may hand its own unmarked attribute to a macro that preserves it. JS text as
quoted syntax data is not an executable JS binding.

In meta §8, a compiler-run macro program holds no executable `@js` binding,
value getter, Js/JsRequests provider or JS initializer. Compute executable
needs through macro entries and all ordinary top-level initializers; if they
need one, reject before instantiation, even when no macro entry needs Target.
Unused runtime functions containing such declarations can be excluded from
the macro program, but cannot leave an initializer behind. Macro quotation of
a declaration is data and does not execute it. Caller-origin checks still
apply when that data becomes an expansion. No file-reading/d.ts import tool,
callback bridge or JS evaluator is part of the compiler's macro sandbox.

## 8. Evidence from code and implementation regions

**P1 — Preserve nominal provenance at every opaque boundary. [K4]** This is
a prerequisite compiler track, not a spec relaxation. Retain each resolved
opaque TypeId from the checked HostSignature through internal import/export
signatures, import coalescing and decode sites. The wire still carries one
externref; no nominal tag supplied by the host is trusted. On ingress attach
the *declared* nominal identity to the foreign reference/id in the guest value.
On egress unwrap the foreign reference without losing the identity of the
guest value that remains in i. Aliases share their resolved nominal identity;
different type definitions never do. Within one nominal type preserve the
host's reference_id identity, including null's id 0. Equality compares the
pair (nominal identity, foreign identity), not the latter alone. Never
canonicalize two types' wrappers to the same guest object. Hashing must remain
consistent with equality; ordinary opaque display stays unchanged.

Cover external results, synchronous host replies, request/async replies,
host-export arguments and results, generated getters, wrappers reused through
coalesced imports, and all intervening generic i code. One Wasm externref
import may be shared, but each i declaration's decoder retains its own nominal
identity. Ordinary type checks still prevent applying an A function to B.
Host re-entry/export round trips reattach the receiving declaration's type;
the wire does not promise to transport guest nominal authority. Opaque
references remain non-portable; N9's Int wrappers are unaffected.

The current checker retains `HostType::Opaque(TypeId)`
(`crates/ilang-check/src/types/mod.rs:314–332`), but lowering erases it
(`crates/ilang-wasm/src/lower.rs:2587–2595`). `$hostref` has only reference/id
(`runtime/mod.rs:240`); its constructor and equality have no nominal component
(`runtime/values.rs:28–34`, `:146–148`, under `crates/ilang-wasm/src/`). These
are read-code facts. The independent review's same-object A/B and null probe
returned True; that execution was reproduced in revision 7 using the review's existing
Wasm fixture and this lane's SDK. It is an existing compiler discrepancy,
not permission to change spec §4.1/§11.4.

P1 implementation scope: `crates/ilang-wasm/src/ir.rs:584–616` keeps nominal
provenance alongside the wire shape; `lower.rs:2587–2595` and import merging
preserve it; `emit/mod.rs:980–1005`, `emit/host.rs:231–242` and the R2 reply
decoder pass it on ingress. Update `runtime/abi.rs:138–141`,
`runtime/mod.rs:240`, `runtime/values.rs:28–34` and `:146–148` for construction,
unwrap and equality; audit equality-compatible hashing and opaque tests.
All abbreviated paths in this paragraph are under `crates/ilang-wasm/src/`.
The existing wire type spelling externref and v3 signatures stay unchanged.

Required host-fixture tests through imports **and** exports: the same raw
object under A and B is False; under A twice is True; distinct objects under
A are False. Null/A versus null/B is False, null/A twice True, and null/A
versus non-null/A False. Repeat through a generic equality helper, collections,
coalesced imports, a host round trip and an async Ready reply. Test aliases
of A as A and two separate definitions of A as distinct. Keep SDK identity
behavior within a nominal type. Place compiled boundary regressions in
`crates/ilang-wasm/tests/codegen.rs`, `bindings/javascript/test/compiled.test.mjs`
and compiled browser fixtures, plus runtime representation tests in
`crates/ilang-wasm/tests/runtime_values.rs`. Keep existing SDK identity tests
(`bindings/javascript/test/host.test.mjs:76–81`, `tests/browser/host.spec.mjs:66–75`).
These are prerequisite tests before F1/F4.

**P2 — Move entry supply validation to provider resolution. [K1, K3]** The
checker currently rejects non-host/non-Assert environment members before
lowering (`crates/ilang-check/src/types/infer.rs:405–429`). Retain ordinary
inference, named-service validity, entry arity/result checks and §10.3 export
checks there. Record the complete inferred environment, not only host TypeIds;
move only the “is there a supply for this entry service?” test to the checked
provider-plan phase, after type checking all declarations and before F2 and
Wasm lowering. An unsupplied user service still gives an entry diagnostic,
now with the entry -> provider -> missing service path. No synthetic provider
may hide ill-typed source, and no library-name whitelist remains in this test.

Replace `lower.rs:916–938`'s direct-service async calculation with T7's resolved
provider closure, before constructing the wrapper at `:943–965` and emitting
entry/export metadata at `:880–913`. All these paths are under
`crates/ilang-wasm/src/`; they are existing implementation regions read here.
The checker/plan interface must carry both the original environment and the
chosen dependency graph. The package reader adds T6's providers and T7's
synchronous contract at `crates/ilang-package/src/manifest.rs:110–129` and
its wasm table parser at `:274`, with storage propagated through
`crates/ilang-package/src/lib.rs:219` and `:497`, followed by CLI build
orchestration at `crates/ilang-cli/src/lib.rs:89–119`. Check a provider cycle
or missing Fs before artifact emission, not during instantiation.

These are **read-code facts**, not observations of the proposed implementation.
Every implementation statement in this note refers to the following anchors.
Historical review-3 probe reproduction is recorded in §10; review-4 probes
are read evidence unless explicitly listed in revision 8 verification below.

| Fact checked in this lane | File and lines |
|---|---|
| Host provisioning constructs all fields; current WASI Io selection is special | `crates/ilang-wasm/src/lower.rs:969–1005`, `:2600–2605` |
| Ordinary top-level initializers run in program order | `crates/ilang-wasm/src/lower.rs:776–807` |
| Metadata is v3, lists services/externals, omits WasiStdout and marks WASI | `crates/ilang-wasm/src/lower.rs:2649–2690` |
| Current IR targets are Ilang/Wasi; special print import kind exists | `crates/ilang-wasm/src/ir.rs:642–665` |
| Checker recognizes external/host attributes, not js | `crates/ilang-check/src/resolve.rs:500–551` |
| ABI checker admits only built-ins and constructorless types | `crates/ilang-check/src/resolve.rs:1655–1684`; `crates/ilang-check/src/types/infer.rs:528–537` |
| Nominal field selection enumerates constructors without a visibility check in that enumeration | `crates/ilang-check/src/types/infer.rs:1538–1614` |
| SDK bits retain length/padding; correct validates finite/scalar values; ofType is weaker | `bindings/javascript/ilang.mjs:324–378` |
| SDK wire lowering/lifting copies buffers and retains bit length | `bindings/javascript/ilang.mjs:560–608` |
| SDK currently rejects thenable synchronous results before lowering | `bindings/javascript/ilang.mjs:836–839` |
| SDK checks reference identity separately from value encoding | `bindings/javascript/ilang.mjs:757–770` |
| Signal is available during start; registration captures request identity; cancel deletes before abort | `bindings/javascript/ilang.mjs:26–36`, `:794–825`, `:744–755` |
| Late settlement checks entry identity; dropping clears before abort; status 2 yields macrotask | `bindings/javascript/ilang.mjs:494–507`, `:884–925` |
| Allocator free links the given address with no ownership validation | `crates/ilang-wasm/src/runtime/mod.rs:369–374` |
| WASI fd_write returns i32, proc_exit has no result; stops use a private short-write loop | `crates/ilang-wasm/src/runtime/mod.rs:255–256`, `:273–304` |
| Custom WASI services, blocking async and host references have regression tests | `crates/ilang-cli/tests/wasi.rs:313–355` |
| CLI checks before compiling/optimizing; options used by run/test select Ilang | `crates/ilang-cli/src/lib.rs:89–119`; `crates/ilang-cli/src/main.rs:160–167`, `:229–270` |
| Legacy Io writes a line and ignores output errors in the Rust host | `crates/ilang-host/src/services/mod.rs:53–77`; declaration `lib/i/io.i:11–14` |

Implementation handoff, with changed regions (new files are explicitly new):

| Stage | Files/regions and required change |
|---|---|
| Front end | `crates/ilang-syntax/src/ast.rs:49–63`, `crates/ilang-check/src/resolve.rs:500–551`: represent/validate foreign declarations; preserve spans and resolved nominal identity; accepted-syntax parser/formatter work must use their landed owners' current regions |
| Provider prerequisites | P1 opaque provenance across all ABI paths; P2 checker entry validation and `lower.rs:916–965` async wrappers; these precede JS/IO integration |
| Provider/link pass | New target-provider and foreign-reachability modules beside `crates/ilang-wasm/src/lower.rs`; integrate before `:776–807` and `:969–1005`; do not modify principal inference |
| Target metadata | `crates/ilang-wasm/src/ir.rs:642–675`, `lower.rs:2649–2690`; new JS artifact emitter and versioned target manifest |
| JS bridge | `bindings/javascript/ilang.mjs:324–378`, `:560–608`, `:784–849`: share validators, add snapshot converters, target getters/outcomes/Args and loader; treat synchronous externref results as opaque, including promises; new i/js and i/target modules |
| WASI prototype | `crates/ilang-wasm/src/runtime/mod.rs:255–304`, `:369–374`: new checked table and adapter beside allocator, keep runtime stops; new i/wasi; retire `WasiStdout` in `lower.rs:989–990` and `ir.rs:642–644` only after §9 amendments and parity tests |
| CLI | `crates/ilang-cli/src/main.rs:160–167`, `:229–270`, `:323–348`, `src/lib.rs:89–123`: target names/alias, provider diagnostics and three-file output |
| R2 then IO | Request fields and lifecycle per runtime §§5/8; shared JS start/cancel regions above; split process services before IO uses them; new standard factories obey native N9; general field privacy in `infer.rs:1538–1614` is a prerequisite |
| Macro integration | M2/M4 owners add M1 to the attribute origin checks and executable-needs validator described in meta §§4.3/8; no existing macro implementation is claimed here |
| Regression tests | Extend `crates/ilang-cli/tests/wasi.rs:313–355`; new JS-target/compiler diagnostics and generated-artifact tests; SDK and browser fixtures in their existing suites |

Order: land the source reconciliations and P1/P2 first. On landed C and accepted syntax,
prototype synchronous JS/linking and the checked WASI printer. Integrate
promises only after R2's typed Request/Owner layer; integrate standard IO only
after T2/A3 and binding grants. Preview-1 adapters beyond the printer are a
separate target stage, not implicit IO scope. Native packaging remains after
R2 as accepted and needs no invented Native root. Typed wrappers wait for M4
and M1. Callbacks require a separate re-entry/environment/lifetime design and
ABI review; they do not block ordinary WASI IO.

### 8.1 Ordered implementation tracks and ownership

The existing CLI options helper selects Ilang; run/test call it and construct
Program in process (`crates/ilang-cli/src/main.rs:160–167`, `:229–270`). Check
loads tests and calls the source checker only (`:323–348`). L4 therefore needs
an explicit target-plan phase, not a renamed default in that shared helper.
P2 moves only supply validation from `infer.rs:405–429`; source-only checking
must retain entry arity/ABI checks while allowing ordinary service needs.

| Order / track | Owned files or regions | Dependency and completion condition |
|---|---|---|
| 0. Source integration | `docs/spec.md` §§2.6/9.7/10/B; `docs/design/runtime.md` D5/§5.1/§10; `docs/design/meta.md` §§4.3/8 | Apply §9 before claiming spec conformance; preserve landed native/multicore contracts |
| 1a. P1 opaque provenance (existing compiler owner) | `crates/ilang-wasm/src/ir.rs` HostSignature; `lower.rs:2587–2595`; `emit/mod.rs:980–1005`; `emit/host.rs:231–242`; `runtime/abi.rs:138–141`, `runtime/mod.rs:240`, `runtime/values.rs:28–34`, `:146–148`; boundary/runtime tests listed under P1 | Can proceed independently; same-object A/B and null tests must pass before JS/IO boundary integration |
| 1b. General field privacy (existing FP owner) | `crates/ilang-check/src/types/infer.rs:1538–1614` and nominal visibility tests | Parallel to P1; required before public Fs/File wrappers can claim private raw Int fields |
| 1c. Plan/check/CLI | `crates/ilang-package/src/manifest.rs:110–129`, `:274–317`, `src/lib.rs:219`, `:497`; checker `types/infer.rs:379–429` and checked-program metadata; new `crates/ilang-check/src/providers.rs`; CLI `main.rs:160–167`, `:229–270`, `:323–348`, `lib.rs:89–123`; new plan/command tests | After 0; implement T3/T6/T9 validation, origin authority, instantiated identities, one-adapter diagnostic and P2; parallel to 1a/1b with disjoint checker regions |
| 1d. R2 runtime integration | R2-owned `lib/i/request.i`, `lib/i/task.i` and request/control/key primitives; `crates/ilang-wasm/src/emit/host.rs` continuation/lifecycle regions; host request contexts | After accepted runtime dependencies; serialize host decoder edits with 1a. Expose only primitive admission/registered key plumbing to planner; implement owned waits, guards and shared group operations |
| 2. Wrapper and link | new provider elaboration/link modules beside `crates/ilang-wasm/src/lower.rs`; `lower.rs:776–807`, `:880–1005`; IR entry/target metadata; `emit/host.rs` generated adapter guards, coordinated with R2; compiler integration tests | After 1a/1c/1d; one whole graph/topological order, T10/T11, F2 including L3, async metadata and synchronous diagnostics. Do not edit P1's opaque lowering independently |
| 3a. JS target | syntax AST/foreign declaration and checker `resolve.rs:500–551`; target IR/metadata and new JS artifact emitter; `bindings/javascript/ilang.mjs` converters/getters/loader; new `lib/i/js.i`, `lib/i/target.i`; SDK/browser/CLI tests | After 2 for full integration; synchronous bridge work can start earlier against a fixed plan interface. Reuse ABI/SDK representations and existing request factories |
| 3b. Checked WASI | `crates/ilang-wasm/src/runtime/mod.rs:255–304`, `:369–374`; new `lib/i/wasi.i`; printer replacement in `lower.rs:989–990`, `ir.rs:642–644`; `crates/ilang-cli/tests/wasi.rs` | After 0/plan interface; parallel to JS in these regions. Keep B.7/custom imports; checked slice/proc_exit tests before removing WasiStdout |
| 3c. Standard IO/target factories | standard library service/provider declarations; `crates/ilang-host/src/services/`; SDK standard factories and IO tests | After 1b/1d and T2 service split; generated JS defaults additionally need 3a. Keep native N9 grants/private Ints, await_owned/await_resource and shared context teardown |
| 4. Macro/packaging follow-through | M2/M4-owned macro origin/executable-needs checks; native packaging files assigned by native.md | M1 after macro prerequisites; native packaging after its accepted R2/IO gates. Callback re-entry and MC-D remain separate work |

These are ownership boundaries for implementation assignment, not files changed
in this lane. Sequential handoffs on lower.rs/emit/host.rs avoid competing
edits to nominal decoders, continuation state and provider wrappers. The new
plan/link track is ready to assign; no library identity or adapter-composition
choice is left for that track to invent.

## 9. Replacement source text and conflicts

This section records the precise amendment handoff and its JA installation. Installed locations refer to this lane; the coordinator still lands these commits. Items marked “left out, conflict” remain unchanged at the named source. The review and old proposal cannot override the spec. No ruling requires
widening the §10.3 ABI value/error lists.

| Source section | Replacement/addition | JA installation |
|---|---|---|
| Spec §2.6 | Insert T9's full schema, validation, graph, ordering and lifetime rules as the normative provider-plan subsection; retain T6's shorthand and T7's wasm.synchronous contract. Add T3's command table (L4). These keys were previously unused; do not claim compatibility with old readers that ignore them. | Installed: `docs/spec.md:292`, `docs/spec.md:483`. |
| Spec §2.2/§2.5; syntax §2.5 | Add js to built-in attributes. Replace the external-only declaration restriction with: “A declaration requires external or js, or expansion by a macro. A js declaration has the closed opaque type and foreign link semantics of §10.1. It is initialized once during start only if retained by the target link pass; ordinary bindings retain this section's initialization rules.” Add F1's ordering and F2's roots/conservative analysis verbatim as the foreign-declaration subsection. | Spec installed: `docs/spec.md:178`, `docs/spec.md:247`, `docs/spec.md:1404`. Syntax companion installed: `docs/design/syntax.md:15`, `:69`, `:138`, `:144`, `:202`. |
| Spec §4.3/§4.7 | Keep sets, nominal identity and principal inference. Add js to the mandatory-full-annotation exceptions. Explicitly say target linking does not change inferred types. No general cast or request-map rule. | Installed: `docs/spec.md:606`, `docs/spec.md:656`. |
| Spec §9.2 | No semantic change. Add the captured-provider versus call-site replacement example described in T1. | Installed: `docs/spec.md:1097`; example gives (1, 2). |
| Spec §9.7/§10.2 | Keep general host imports and substitution. Replace entry's host/Assert-only restriction with: “An entry may also require a service supplied by its explicit target provider plan. The plan constructs ordinary service values; missing required standard capabilities are build errors, including when all operation errors are caught. Providers do not alter inferred source types.” Use T1–T5 for Target, capability tables and explicit host plans. Retain legacy Io's line/no-error behavior. | Installed: `docs/spec.md:1231`, `docs/spec.md:1505`. |
| Spec §10.1 | Retain externals. Add F1–F3 as “JavaScript value declarations”: full nominal annotation, supported targets, source scope/order, retained getter imports, compile-time unavailable errors, loader failures and no re-entry. | Installed: `docs/spec.md:1374`, `docs/spec.md:1404`, `docs/spec.md:1775`. Parenthesized-subject allowance left out, conflict with spec §6.4; braces retained. |
| Spec §10.3 | Keep the lists unchanged. Cross-reference R2's existing outer Request/Result declaration forms. “IO wrappers over Int and constructed Thrown/Mismatch are not boundary types; their libraries pass Int or opaque Outcome and decode in i.” | Installed: `docs/spec.md:1521`; boundary lists unchanged; R2 declaration forms explicitly cross-referenced. |
| Spec §11.7 | Keep host-value, memory and stack stops. A bridge call that does not expose exceptions uses F4's panic; malformed raw outcome/host reply uses the existing host-value stop. Do not convert these into Mismatch. | Installed: `docs/spec.md:1582`. |
| Spec B.1/B.3 | Keep ABI allocation ownership. “Checked Wasi buffers are separate table-owned allocations and are not addresses supplied to ilang_free by i code.” Use W1/W2 as the checked root contract; ABI exports remain host machinery. | Installed: `docs/spec.md:1678`, `docs/spec.md:1711`, `docs/spec.md:1816`. |
| Spec B.4–B.6 | Keep v3 ids, cancel, statuses and import-time re-entry restrictions. Add F3's JS target marker/ilang.js v1 metadata and retained getter imports, without changing the host artifact contract. Mandatory changes to runtime imports/statuses/host-called exports still require a version bump. | Installed: `docs/spec.md:1717`, `docs/spec.md:1775`; B.5 unchanged. |
| Spec B.7 | Replace only the special Io implementation bullet after adoption: “The target's Io provider writes UTF-8 text plus LF through the checked Wasi adapter; positive short writes are retried, zero progress/errors give up, and print gives Nil.” Keep it absent from external required-import metadata because internally provided. Keep custom imports, runtime-reserved import names, stops, `_start`, blocking calls and no tests. Add the no-result proc_exit adapter rule W2. | Installed: `docs/spec.md:1807`, `docs/spec.md:1870`; separate LF attempt preserves the existing failure behavior. |
| Runtime D5 and §10 | Replace entry provisioning/classification and ABI async-flag text with §9.1 below. | Installed exact §9.1 replacements: `docs/design/runtime.md:126`, `docs/design/runtime.md:1768`, `docs/design/runtime.md:1843`. |
| Runtime §§5.1/5.4 | Keep their existing ownership rules; use corrected A2 and the Ready-survives-Owner test in §10. | Bootstrap installed: `docs/design/runtime.md:378`; ownership unchanged; Ready test at `docs/design/runtime.md:2098`. |
| Runtime §§7/7.4 | Keep the landed native N9/runtime resource text, group operations, await_owned/await_resource and program/control-root accounting. Split process fields by T2, preserving other signatures/errors. | Split installed: `docs/design/runtime.md:1225`; raw signatures, codecs, errors and resource rules preserved. |
| Runtime §8.2 | Generated js/nodejs defaults follow T1–T3, with no automatic unsupported stubs. Explicit Host factories remain configurable. Generated builds share A4's factories. Keep WASI custom imports; list printer as this target project's separate adapter stage, and do not promise later adapters before they land. | Installed: `docs/design/runtime.md:1488`, `docs/design/runtime.md:1849`. |
| Meta §4.3 | Add js to the built-in names and @js to the caller-origin list, exactly as M1. | Installed in the two named rows: `docs/design/meta.md:371`, `docs/design/meta.md:379`; the declaration companion is installed in §7.1 at `:496`. Meta remains a draft whose review is unfinished. |
| Meta §8 | Add: “A macro program contains no executable JS binding or initializer. Reject required js declarations before instantiation; quoted syntax remains data.” Apply M1's full-needs rule. | Installed: `docs/design/meta.md:581`. Executable JS needs include macro entries and all ordinary initializers; quoted declarations remain data. The preceding external-needs rule is unchanged. |
| Native N1/N3/N4/N9 | Keep runner ABI, registrations, packaging order and N9. Document host as the preferred ilang alias, and the nullary Native tag; do not add a Native root. | Preserved without editing native.md: N1/N3/N4/N9 remain controlling. Alias and nullary tag installed at `docs/spec.md:487` and `docs/spec.md:1246`. |

The older spec restrictions on declarations, entry services, the special WASI printer and target metadata have been replaced at the locations above. Runtime's automatic browser stubs are replaced under J1. Native N9, custom WASI imports, request ownership and the multicore gate are unchanged.

**Companion installation.** The coordinator extended JA ownership to syntax.md and Meta §§7.1/8. Syntax's declaration rules and their summary/keyword restatements now admit `@js`; its built-in attribute list adds the closed-opaque declaration form. Meta §7.1 adds `@js` to the declaration exceptions, and §8 excludes executable JS bindings, getters, providers and initializers from macro programs. Its existing external-needs rule is unchanged. Quoted declarations remain data, subject to §4.3's caller-origin rule when expanded. Meta is still a draft with unfinished review: only the passages named by this handoff were changed, and the Meta commit records that restriction. No companion item remains pending for lack of ownership; the source changes do not claim a macro implementation.

F2 originally said “allowing parentheses or a one-expression group around the subject”. Spec §6.4 says “`(x)` is a tuple of one part” and uses `{ }` for grouping. The parenthesis allowance was left out, and that sentence in F2 was corrected. This changes no type or tuple rule.

**Adjoining sentences corrected with installation.** Spec opening/§2.1 now name target modules and distinguish checking test source from compiling test entries; §1.4 no longer reserves `@js` for macros; §2.2/§2.5 allow its declaration and initialization; §2.6's key introduction includes plans; §4.3/§4.7 require its annotation and preserve inference; §9.7 distinguishes chosen factories from imported services and keeps Io's Nil result; §10.2 replaces the host/Assert-only and direct-async rules; B.1/B.3 separate checked buffers, B.6 lists required host registrations and JS markers, and B.7 applies its reserved-import rule to required graph imports. Runtime §10's old host-supply/browser/WASI sentences, entry-dependency instructions and related test expectations now follow D5 and J1. Syntax's summary, §1.4, §2.2 and §2.5 no longer reserve `@js` declarations for macros; its attribute-list instruction now adds `js`. Meta §7.1 no longer says externals are the sole non-macro declarations. Meta §8 gains the executable-JS restriction without removing the external restriction. These are consequences of the named amendments, not additional feature choices.

**Older-sentence inventory (JA, including the companion follow-up).** The list below quotes the older sentences removed or reworded in the amended source passages. Locations name the paragraph or table row in baseline `7b3cea3`, not the shifted line numbers of this lane. Replacements are located in the amendment map above. To make the inventory complete, it also includes sentences restated with unchanged meaning inside a replaced paragraph and changed table rows. It does not claim those preserved rules were false. Pure additions, such as Meta §8's JS ban, have no older sentence to quote; Meta's external-needs sentence remains unchanged.

<details>
<summary>Full older-sentence and table-row list</summary>

**docs/spec.md**

- Baseline paragraph/row 14: The paths `i/task`, `i/clock`, `i/fs`, `i/process`, `i/net`, and `i/stdio` are kept for library modules that are not defined yet: each is a module with no names, and a program may not define a module with one of these paths.
- Baseline paragraph/row 14: Other library modules, the command-line tools, and the way test results are shown are not part of the language.
- Baseline paragraph/row 67: Some forms are kept for macros, which are not yet part of i: an attribute that is not built in, such as `@m` or `@m(args)` before an item (§2.2); a top-level item that is a call, `m(args)` or `a.m(args)` (§2.2); a declaration with no `@external` (§2.2); and, in a type, a lowercase name or `a.m` followed by `(` (§3).
- Baseline paragraph/row 133: Modules in `test/` are only compiled when the tests of their package are run.
- Baseline paragraph/row 137: `i`, for the seven **library modules** (§9, §13), which every compiler gives, and the kept paths named at the start of this document.
- Baseline paragraph/row 178: A declaration with `@external` names a value of the host (§10.1).
- Baseline paragraph/row 178: A declaration with no `@external` is kept for macros, where it goes with a binding of the same name.
- Baseline paragraph/row 178: It is a compile error, and the message says that a declaration with no `@external` is kept for macros.
- Baseline paragraph/row 246: A declaration with no `@external` is kept for macros, and is a compile error (§2.2).
- Baseline paragraph/row 266: These keys have a meaning; other keys are not used:
- Baseline paragraph/row 390: They only come from the host (§10).
- Baseline paragraph/row 445: Type annotations may be left out everywhere for inference, but not on `@external` declarations.
- Baseline paragraph/row 1008: A host service in the environment of an entry point (§10.2) is given by the host.
- Baseline paragraph/row 1008: `i run` and `i test` give it to every program, and on the WASI target the module gives it itself (§B.7).
- Baseline paragraph/row 1008: Any other host, such as an embedder or the JavaScript SDK, gives it like any other host service, or the program cannot run there.
- Baseline paragraph/row 1108: An entry point may have environment parameters and an environment, but only of host services and `Assert`; the host and the runtime give them.
- Baseline paragraph/row 1108: Any other service must be given before it gets to the entry point.
- Baseline paragraph/row 1112: An entry point that may call a function of an async host service (§9.7) is **async**.
- Baseline paragraph/row 1112: The host has to wait for it (Appendix B).
- Baseline paragraph/row 1352: `services` lists every host service function that the entry points need, and `externals` every external function.
- Baseline paragraph/row 1352: Other modules have no `target`.
- Baseline paragraph/row 1363: `print` of the standard service, `Io` of `i/io` (§9.7), is not an import: the module writes the text and a line feed to file descriptor 1.
- Baseline paragraph/row 1363: A write that fails is given up.
- Baseline paragraph/row 1363: The metadata does not list it.
- Baseline paragraph/row 1363: Such an import is an external function, or a field of a host service in the environment of an entry point (§9.7).
- Baseline paragraph/row 1363: Every other host service and external function is an import as in §B.4, so a host that has them can give them.

**docs/design/runtime.md**

- Baseline paragraph/row 126: “Provision entry services from the inferred environment.” Let N be the entry function's ordinary inferred environment.
- Baseline paragraph/row 126: On the ilang target the entry is async exactly when N contains an async/request host service or Scheduler or Runner from i/task.
- Baseline paragraph/row 126: Otherwise it remains synchronous.
- Baseline paragraph/row 126: No reachability guess, attribute on the entry, or change to its principal type is needed.
- Baseline paragraph/row 126: Task services in N request the default FIFO task run, with root path [] and seed random.from_seed(0).
- Baseline paragraph/row 126: Async host services without task needs use a direct root Await handler and request context; they do not set up a scheduler.
- Baseline paragraph/row 126: Pure entries set up neither a task run nor a host hand-off.
- Baseline paragraph/row 126: Await/Owner/Root alone, for example for completed fakes, do not turn an entry async; a low-level wait/poll that tries to hand off there stops with request: no active entry.
- Baseline paragraph/row 138: This is one narrow entry-point rule for the resolved standard services, authorized by the coordinator and ledger G2(S7).
- Baseline paragraph/row 138: It is a deliberate provision for i/task and request services; an arbitrary user scheduler is not implicitly bootstrapped.
- Baseline paragraph/row 138: IO service values are given only as the inferred needs require, like Assert and Io today.
- Baseline paragraph/row 138: Channel and scheduling operations remain ordinary i.
- Baseline paragraph/row 138: The host-import and request primitives themselves have no IO namespace rule.
- Baseline paragraph/row 138: A pure task run can have async metadata yet finish with status 0 on its first call: scheduler switches alone never cause a host yield.
- Baseline paragraph/row 138: Progress polling below requires an actual task waiting for host work; idle waiting also counts unawaited requests.
- Baseline paragraph/row 138: There is no automatic entropy draw.
- Baseline paragraph/row 138: Contextual top(seed) keeps the current Runner's route/prefix; it changes only the seed.
- Baseline paragraph/row 338: The runtime creates the initial Root only for an entry whose inferred needs require request/task support.
- Baseline paragraph/row 338: The initial bootstrap is part of the narrow entry provision, not a public unscoped root allocator.
- Baseline paragraph/row 338: Entry request contexts and each task.run open a fresh Root and bind it for their whole lifetime.
- Baseline paragraph/row 1030: | process / Process (type only) / `i:process/v1` | Child, constructor private; `ProcessError { ProcessError(operation: String, code: String) }` |
- Baseline paragraph/row 1188: | Raw Process field | Type |
- Baseline paragraph/row 1449: | Module | `i run` and native Rust host | JS SDK standard service factory: Node | JS factory: browser | WASI preview-1 target in this track |
- Baseline paragraph/row 1451: | random | Pure i | Same pure i | Same pure i | Same pure i |
- Baseline paragraph/row 1452: | task | i FIFO loop over v3 | Same loop; promises, AbortSignal, macrotask polling | Same loop on calling thread; never block it | Same i loop; raw host calls complete synchronously; no status hand-off |
- Baseline paragraph/row 1453: | clock | Reactor timers and OS clocks | Monotonic/system clocks and abortable timers | Performance/system clocks and abortable timers | Ordinary blocking service imports; preview-1 command supplies no i:clock adapter |
- Baseline paragraph/row 1454: | fs | OS files, with the cancellation rule above | Node context-owned Int table, closed/busy leases, explicit close | Every raw operation gives unsupported, except close gives Nil | Blocking Int imports; buffer-aware embedder/linked adapter needed, no default fs adapter |
- Baseline paragraph/row 1455: | net | Reactor TCP | Node context-owned Int table; provisional cleanup and close wakeups | All operations give unsupported, close gives Nil; browser fetch is not raw TCP | Ordinary blocking imports; no default listen/connect adapter |
- Baseline paragraph/row 1456: | process | OS args/env/cwd/entropy/child processes | Node process data, entropy, spawn/wait/terminate/close | args gives an encoded empty list; env gives denied; cwd/spawn/wait/terminate give unsupported; entropy uses platform secure entropy; close gives Nil | Ordinary blocking imports; no default process/entropy adapter or spawn implementation |
- Baseline paragraph/row 1457: | stdio | Per-source input broker/subscribers and byte output | Node standard byte streams with cancelable subscribers | Default byte read/write/flush operations give unsupported; an embedder can give explicit byte sources/sinks | Ordinary blocking imports; no new i:stdio adapter; legacy i/io continues to use fd_write |
- Baseline paragraph/row 1459: The JS factory is an explicit SDK option, not a language name-resolution rule.
- Baseline paragraph/row 1459: It installs all fields including unsupported stubs, so a module can instantiate and catch a typed unsupported error.
- Baseline paragraph/row 1459: Embedders can override any whole service with ordinary imports and explicit binding grants.
- Baseline paragraph/row 1472: Implementing a preview-1 adapter set is a separate target project, not an unreviewed second IO host in this track.
- Baseline paragraph/row 1640: A host service required at an entry point is given by the host.
- Baseline paragraph/row 1640: i code can replace it with an ordinary value, including completed fake requests.
- Baseline paragraph/row 1640: Each field is imported under the namespace and its label (Appendix B).
- Baseline paragraph/row 1640: i run and i test provide it; the WASI target implements it as §B.7 says.
- Baseline paragraph/row 1640: Other hosts provide the imports or cannot instantiate the program; supported hosts may supply typed unsupported errors for unavailable capabilities.
- Baseline paragraph/row 1730: Entry environment parameters and needs may contain host services, Assert, request.Await, request.Root, request.Owner, task.Scheduler, and task.Runner.
- Baseline paragraph/row 1730: No other unsupplied service is allowed.
- Baseline paragraph/row 1730: The runtime binds only the defaults requested by the ordinary inferred environment.
- Baseline paragraph/row 1730: Inferred Scheduler or Runner needs request the default structured FIFO run, fresh identity/path [], seed random.from_seed(0), and its internal request context.
- Baseline paragraph/row 1730: Async/request host needs without task needs get a direct request context/Await handler, without a task scheduler.
- Baseline paragraph/row 1730: Await/Root/Owner-only needs may use completed fakes in a synchronous entry.
- Baseline paragraph/row 1730: These resolved standard-service entry defaults are the one narrow library provision; another library gets no implicit bootstrap.
- Baseline paragraph/row 1730: Entry wrapping does not change a function's principal type.
- Baseline paragraph/row 1730: The default values can be replaced by ordinary environment statements.
- Baseline paragraph/row 1730: task.top(seed) changes a Runner's seed while retaining its route and prefix.
- Baseline paragraph/row 1730: An unsupplied user State service remains an entry error.
- Baseline paragraph/row 1730: Main/test errors stop after cleanup; host-export errors reach the host after cleanup.
- Baseline paragraph/row 1730: Stops run no finals.
- Baseline paragraph/row 1730: On the ilang target, let N be the entry's inferred environment: the entry is async exactly when N contains an async or request-mode host service, or task.Scheduler or task.Runner.
- Baseline paragraph/row 1730: Otherwise it is a synchronous export, including a pure comparator usable by a synchronous host callback.
- Baseline paragraph/row 1730: No body-reachability analysis or universal async wrapping is used.
- Baseline paragraph/row 1730: Even an async entry may finish with status 0 on its first call.
- Baseline paragraph/row 1730: Scheduler progress polling yields to the host only for actual host waiters, including nested waiters; idle waits also count unawaited work.
- Baseline paragraph/row 1730: Pure task switches alone do not yield there.
- Baseline paragraph/row 1730: On WASI, all entry calls and host imports remain synchronous (§B.7).
- Baseline paragraph/row 1800: Request-mode and async declarations produce the same v3 async signature; metadata lists the unwrapped signature and derives each entry's async flag from its inferred environment as §10.2 says.
- Baseline paragraph/row 1800: A completed cell is not an outstanding id.
- Baseline paragraph/row 1811: The new standard service namespaces i:clock/v1, i:fs/v1, i:net/v1, i:process/v1 and i:stdio/v1 remain ordinary blocking imports on this target.
- Baseline paragraph/row 1811: This version supplies no preview-1 adapters for them.
- Baseline paragraph/row 1811: A required adapter is not supplied by plain wasmtime run; builds report these required imports.
- Baseline paragraph/row 1811: An async adapter awaits its already-completed blocking result.
- Baseline paragraph/row 1811: A request adapter returns a completed request value.
- Baseline paragraph/row 1811: The task loop's root poll is a no-op; no request uses a resume export or ABI cancel.
- Baseline paragraph/row 1811: i/random is pure and i/task can interleave computations at its explicit scheduling points, but a blocking host operation blocks that worker.
- Baseline paragraph/row 1811: Legacy i/io is unchanged.
- Baseline paragraph/row 1895: Generated host-field references add ordinary dependency edges to i/request; entry wrappers add i/task dependencies only when N requests task services; request-only wrappers depend on i/request.
- Baseline paragraph/row 1895: Add these edges before ordering/ reachability, using inferred service needs rather than scanning function bodies.
- Baseline paragraph/row 1948: [ ] Implement the i loop, per-task boundaries, rooted Owner groups, Runner/top, task.keep and inferred-needs entry provisioning.
- Baseline paragraph/row 1948: Preserve synchronous exports and suppress host progress polls when no task waits for IO.
- Baseline paragraph/row 1976: [ ] Run permission/error/resource/security and concurrency reviews; include unsupported browser/preview-1 behavior and user-supplied host fakes.
- Baseline paragraph/row 2054: | Missing env variable; browser raw TCP | ProcessError(...,"not_found"); NetError(...,"unsupported"); §7 and target matrix |
- Baseline paragraph/row 2055: | WASI pure tasks/random; WASI requiring private Fs | Pure calculations work; report blocking Int imports requiring a buffer-aware adapter; §8.2/B.7, no fd identity reuse |
- Baseline paragraph/row 2056: | @main needs user State with no binding vs standard Scheduler/Runner/Await/Root/Owner | State is an entry compile error; standard needs are provisioned without changing the type; async classification follows D5, not mere use of any request helper |
- Baseline paragraph/row 2476: Inferred entry-service classification, pure/fake-only synchronous cases, task/host-only async cases, and no host poll for pure or raw-only runnable work.

**docs/design/meta.md**

- Baseline paragraph/row 368: An attribute whose name is one of the built-in attributes (`external`, `host`, `main`, `test`, `deprecated`, `macro`) is that attribute; a macro with such a name is used with its module name.
- Baseline paragraph/row 378: The attributes `@external`, `@host`, `@main`, and `@test` may be in a result only when the attribute came from the code of the caller (it has no marks, §5).
- Baseline paragraph/row 494: This is the only case where two items have one name, and the only use of a declaration other than `@external`.

**docs/design/syntax.md**

- Baseline paragraph/row 7: The forms of macros in `docs/design/meta.md` are kept: `@m` and `@m(args)` before an item, a top-level item that is a call, a declaration with no `@external`, and a call `m(…)` in a type.
- Baseline paragraph/row 69: Some forms are kept for macros, which are not yet part of i: an attribute that is not built in, such as `@m` or `@m(args)` before an item (§2.2); a top-level item that is a call, `m(args)` or `a.m(args)` (§2.2); a declaration with no `@external` (§2.2); and, in a type, a lowercase name or `a.m` followed by `(` (§3).
- Baseline paragraph/row 127: A declaration with `@external` names a value of the host (§10.1).
- Baseline paragraph/row 127: A declaration with no `@external` is kept for macros, where it goes with a binding of the same name.
- Baseline paragraph/row 127: It is a compile error, and the message says that a declaration with no `@external` is kept for macros.
- Baseline paragraph/row 144: The list of the attributes keeps its text.
- Baseline paragraph/row 194: A declaration with no `@external` is kept for macros, and is a compile error (§2.2).

**docs/design/js.md (rule corrections)**

- F2, baseline lines 947–948 (grouping clause): “For the source form `certified_name |> { clauses }` (allowing parentheses or a one-expression group around the subject)”. Parentheses were left out because they construct tuples; `{ }` grouping remains.
- W3, baseline lines 1278–1281: “Io factory captures Wasi and makes a closure that UTF-8 encodes text and LF, allocates/stores a buffer, and repeats fd_write on its remaining slice. Positive short writes advance the offset; zero progress or a reported write error gives up; release the buffer on every returning path.” The text and LF attempts are now separate, preserving failure behavior.

Status sentences saying amendments await another owner or remain entirely uninstalled were also replaced by the installation record above; they are progress records, not language rules.

</details>

**Companion validation.** Commits `6962322` (syntax.md) and `de7de49` (draft meta.md) each change one source. After them, `git merge refs/heads/redesign` reported “Already up to date”; the merged ref was `7b3cea3ed2602744bcbb8c747ebffe177864f1a2`, already an ancestor. The repeated `scripts/old-syntax-check` passed all 1,164 snippets with 19 unchanged future-track entries. The repeated `cargo test -p ilang-cli --test spec --locked`, run through the heavy wrapper, passed 168/168 cases at O0/O2. No new failure or source conflict was found. Inspection checked that initializer-only JS needs are rejected, unused runtime functions leave no executable initializer, quoted declarations remain data, and caller-origin restrictions still apply when they are expanded; these are source-rule checks, not executed macro tests.

**Installation checks and attacks.** The CLI spec suite passed all 168 cases at O0/O2; the syntax spec test passed parsing and print round trips. The old-syntax checker passed with its 19 future-track expectations unchanged. No new snippet needed a future-parser entry, and no check was weakened. The first CLI suite attempt stopped before tests because Binaryen was absent; initializing this lane's pinned submodule fixed it. No remaining suite failure was found.

The new capture example was run and its assertion of (1, 2) passed. A probe comparing `(t)` with t was rejected as disjoint with E0520, confirming the grouping conflict. Current source-only checking of a State entry still gives E0522 (`crates/ilang-check/src/types/infer.rs:405–429`), and the @js probe gives E0301/E0308/E0315 (`crates/ilang-check/src/resolve.rs:500–551` handles the attribute list); these are verified implementation gaps, not failures of installed target behavior. No provider/foreign implementation was added.

Reading the current `$wasi_line` showed it always tries LF after the text write (`crates/ilang-wasm/src/runtime/mod.rs:298–304`); W3's sentence and spec B.7 now preserve that behavior. The short-write loop is at `:282–297`. The ordinary printer and checked table remain future work, so their memory/error behavior is checked against W1/W2 by inspection, not claimed executed.

After these corrections, two further inspection passes found no serious new issue. Pass 1 traced `Cell(A) -> Cell(List(A))` (recursive-dependency error), the T10 Config-before-Await graph, the T11 seed override ("override", never silently replaced), and the existing substituted-Target/keep_env fixtures (retain unknown foreign references). Pass 2 traced body-twice and body-after-exit adapters (the specified provider stops), `free(token); load(token, 0, 0)` (closed), oversized fd_write results (host-value stop), and Ready surviving Owner close (original answer once). Those future results follow the installed rules; they were not run. A byte comparison confirmed that spec §10.3's boundary lists, B.2's wire table, B.5's protocol and runtime §§5.4/8.1 ownership/resource rules are unchanged. R2/IO type and operation definitions still need their own installation; JA cross-references those accepted contracts rather than inventing them.

### 9.1 Exact runtime replacement (accepted WASI exception)

**Replace D5 in full with the following text. [K1, L1, L2]**

> Let N be an entry's ordinary inferred environment. Resolve the target's
> chosen provider for every service in N, and transitively for every service
> in each chosen factory's inferred environment, generated constructor payloads
> and the structured adapter's outer requirements. Use the complete validated
> provider-plan schema of spec §2.6, including its outer/body scopes and seed
> edges. A role belongs to the service row and survives a factory override.
> At most one distinct adapter may reach an entry; otherwise report both
> originating rows and their adapter names. No composition is defined.
> A request host declaration
> is a request-mode provider; an async host declaration is normalized to its
> request-and-await provider. Scheduler and Runner are provider roles with
> typed structured-run entry adapters, usable by any service; no rule tests
> a module or type spelling. Reject unresolved supplies and provider cycles.
>
> On targets using the v3 asynchronous entry protocol, the entry is async
> exactly when that closure contains a request-mode service provider, a
> Scheduler provider or a Runner provider. This is computed from the full
> chosen graph, without source-body reachability or field-use analysis, and
> does not change the principal source type. Await/Owner/Root providers alone
> do not qualify. A service node's declared request/async mode counts even if
> a factory supplies a completed fake for that service. A pure bundle provider
> without such a dependency may remain synchronous. An ordinary user bundle
> follows exactly the same rule as a shipped bundle.
>
> First open the primitive entry context; enable async admission exactly for
> an async entry. Opening is fixed, cannot be replaced, and has no service
> dependencies. It constructs no replaceable service value. The initial root
> key argument, when needed, is registered there as part of narrow bootstrap.
> Construct every chosen service, including context services, in the one
> topological order of the complete plan graph in spec §2.6. An adapter-enter
> node follows its outer prerequisites; seeds become available inside its
> guarded continuation. A seed-dependent factory executes there. Other ready
> factories precede adapter-enter according to that order. No Root/Owner/Await
> prefix and no temporary fallback service may bypass a dependency.
>
> Invoke the unchanged source entry with the chosen body services. Finish the
> adapter and its ordinary structured cleanup, then retire the primitive
> context on every exit. Provider factories have no uncaught typed errors;
> construction cannot add to the source entry's ABI error row. A stop skips
> i finals; context retirement still occurs. Enforce the adapter's single-call,
> active-extent guard from spec §2.6; a suspension does not end that extent.
> The shipped task adapter retains FIFO, fresh run identity/path [], seed
> random.from_seed(0), and no entropy draw. With no role there is no structured
> adapter: ordinary chosen Await/Owner/Root factories suffice. Synchronous
> fake-only contexts cannot hand off: a low-level wait requiring hand-off or
> poll stops with request: no active entry.
>
> A host export can therefore become async when its provider closure changes,
> even if its source type is unchanged or its body never waits. The artifact's
> async metadata and wrapper follow the closure. If it is listed in the root
> package's wasm.synchronous contract, such a build is an error showing the
> qualifying dependency path. Do not silently block or emit synchronous
> metadata. Otherwise the caller must use the async entry protocol. Status 0
> on the first call is still possible; pure scheduler switches alone do not
> yield to the host. Existing progress-polling and contextual Runner seed
> rules stay unchanged.
>
> On WASI preview 1, spec B.7 continues to require blocking imports,
> synchronous entry metadata and no async hand-off. This is the explicit
> target-ABI exception accepted by the coordinator; custom imports remain.

**Runtime §10, §10.2 replacement. [K1, K3, L1, L2, L4]** Keep its first paragraph defining
main, tests and host exports and their ordinary signatures. Replace all its
following entry-service and async-classification paragraphs by:

> Any ordinary named service may appear in an entry's inferred environment
> when the chosen target provider plan supplies it. Source inference and
> boundary/entry signature checks run first. The provider phase then checks
> supplies for the full inferred environment and selected factory dependencies;
> the checker no longer restricts entry services to a built-in list. A missing
> required service is an entry compile error, even when operation errors are
> caught. Select user providers per target by spec §2.6; the dependencies of
> an unchosen provider are not supplied on that target. Do not delete any
> inferred requirement through target-branch pruning.
>
> Source-only i check stops after ordinary source validation; it promises no
> provider or target availability. i check --target validates the selected
> target plan without emission or execution. i run and i test execute in
> process using native providers; i build uses its selected target, default
> native. All use the command/entry selection table of spec §2.6.
>
> Apply D5's transitive-provider rule, wrapper order, synchronous-export
> contract and WASI exception exactly. This is the sole classification rule;
> directly examining N for host/task service names is insufficient. Ordinary
> environment statements still replace values at call sites, and selected
> factories are ordinary functions with ordinary types. A provider can capture
> its dependencies; later substitutions do not mutate that captured value.
> The original source type stays unchanged. Main/test errors stop after
> cleanup; host-export errors reach the host after cleanup; stops run no i finals.

**Runtime §10, B.4–B.6 replacement sentence. [K1]** Replace the sentence saying
metadata derives async flags from the inferred environment with:

> Metadata derives each entry's async flag and status/result exports from
> D5's complete chosen provider closure, subject to B.7's blocking WASI ABI.
> This calculation precedes linking and optimization. A requested synchronous
> export that classifies async is a build error. Request-mode and async host
> declarations retain v3's existing wire; completed cells are not outstanding ids.

Keep runtime §5.1/§5.4's cancellation rules unchanged: close_owner/close_root
cancel outstanding work,
not detached Ready answers. Keep native N9, runtime §8.1 and J4's custom WASI
imports. P1 implements the existing nominal-equality spec, requiring no new
ABI type or weakening of identity.

**Exact adjoining bootstrap sentence (runtime §5.1). [L1]** Replace the
sentence that says the runtime creates the initial Root based on inferred
needs with:

> The primitive entry context registers the initial root key only when the
> checked provider graph needs the entry-root-key argument. An ordinary chosen
> factory constructs the Root service value from that key. Opening admission
> has no service dependencies and does not construct a chosen Root, Owner or
> Await. This is narrow entry provisioning, not a public unscoped root allocator.

Keep the rest of runtime §5.1/§5.4: shared portable Root/Owner capabilities,
local Request/Ticket cells, count/settled/cancel_all/wait_none operations,
Ready survival, borrowed await, await_owned and await_resource. Keep the
landed program/control-root distinction: only program work and explicit
borrowed producer interests determine quiescence, while all actual requests
count for ABI status 1. No Ticket enumeration or native root bundle is added.
Multicore's pure Event algebra, shared spawn/run counter, non-migrating live
continuations and MC-D gate remain unchanged. These provider changes do not
approve or implement multicore.

The earlier K1/B.7 question is closed by the ledger. The table above records installed source changes and the one tuple/grouping conflict left out; implementation conformance still requires those handoffs and the compiler/runtime work. No new spec/native/multicore
conflict is resolved by silently weakening their rules.

## 10. Required tests and adversarial revision

Expected results below are tied to rules. These are implementation fixtures,
not claims that future APIs compile on merged redesign. Use gates and counters
for cancellation/cleanup, not sleep timing. Run availability tests at O0/O2/O3.

| Test | Expected result and rule |
|---|---|
| Promise example and equivalent user bundle on async-protocol target | Async context opens before factories; async true metadata and result export (T7/T8) |
| answer export with request-backed Bundle / pure Bundle / synchronous contract | async true / async false / compile error for the request-backed plan (T7/T8) |
| Settings fixture with file contents `<<1, 2>>` on Node/Native | read returns `<<1, 2>>`; real Fs provider chosen, private numbers remain hidden (T6) |
| Settings fixture on Js/Wasi/Host | read returns `<<>>`, no Fs provider/import, synchronous classification (T6/T7) |
| User State without any provider; provider dependency cycle | Entry-supply diagnostic after inference / provider-cycle diagnostic (P2/T6) |
| choose example below | WASI rejects retained unused; JS initializes it and stops with unused, at O0/O2/O3 (F2) |
| Certified Target alias captured directly; helper environment Target; substituted Target through keep_env | Prune only the direct capture's other-target constructor clauses; helper cases retain foreign binding (F2) |
| Recursive or higher-order reference to an entry | Its Target binder loses certification; retain all clauses (F2) |
| Matching-target clause with literal False guard | Retain guard/body and later clauses; no guard/value evaluation (F2) |
| Same raw object under A/B, A/A; null under A/B, A/A | False, True; False, True through import/export/generic equality (P1) |
| Ready answer returned from within, then take | Returns original opaque outcome, then Taken; Owner close sent no cancel for it (A2/K5) |
| Explicit request.cancel after Ready, then take | request: cancelled stop; individual guard differs from Owner close (runtime §§5.1/5.4) |
| process.entropy(32) on supported JS profile | 32 bytes; only Entropy plus request services required, no Process import (T2) |
| process.args on WASI without/with explicit adapter plan | Missing Arguments diagnostic / decoded arguments; no spawn/cwd requirement (T2/T3) |
| fs.read_file inside fail.catch on js | Compile error naming Fs; no executable unsupported handler (T1/J1) |
| Same nominal service used through one field | Whole service type; no target-dependent principal type (T1) |
| Capture real Js in Io, then substitute Js | Old Io keeps captured implementation; a later direct JS call uses substitute (T1) |
| Five-constructor Target match | Exhaustive; entry needs Target only (T4) |
| Unavailable @js only in another-target constructor clause on a certified entry Target | No declaration getter/import on WASI (F2) |
| Unavailable @js in unknown call, ordinary initializer or explicit fake Target branch | Compile error, no null, at all optimization levels (F2) |
| Two @js object expressions and repeated reads | Distinct objects compare False, same declaration True (F1/F2) |
| Custom WASI Float service and existing greet fixture | Import retained; supplied host result preserved (T5, wasi.rs:313) |
| One-bit round trip; mutate JS copy afterwards | `<<1:u1>>`; original i value remains that one bit (F4) |
| Nonfinite number, lone surrogate, malformed bits | Mismatch at explicit converters; typed invalid ABI reply still stops (F4) |
| Conversion getter returns good value once, bad later | One snapshot used; no unchecked second read (F4) |
| Sync call returns a Promise or an object with a throwing then getter | Return that opaque Value without touching then; only explicit observe adopts it (F4/A1) |
| Promise rejects with null or arbitrary object | await raises Thrown of that same raw value after wait (A1) |
| Started IO cancelled, id reused, old completion arrives | Old response cannot resume new request; provisional resource closed once (A2/A3) |
| Stop with waiting IO and late spawn | No i finals, no new delivery, registered job cleans/reaps late child (A3/N9) |
| Close races a gated read | Close first gives closed; completion first preserves bytes; competing read gives busy (A3) |
| Attempt File(1), raw field projection, or raw alias without grant | Privacy error / permission denial; no live table access (A3) |
| Resource number above 2^53, stale number from previous entry | Exact BigInt round trip / closed; never new resource (A3) |
| alloc/free/free, then two allocations | Second free Error("closed"); new tokens differ; live allocations never overlap (W1) |
| Freed token reused after physical address reuse | closed; no access to new buffer (W1) |
| Negative offset, oversized slice, empty slice at end, one-bit store | invalid, bounds, empty BitArray, invalid respectively (W1) |
| Attempt to store at an ABI/runtime address interpreted as a token | closed unless it independently names a checked allocation; never arbitrary memory (W1) |
| Printer with two-byte host writes | Exact UTF-8 text plus LF, zero remaining allocations (W2/W3) |
| fd_write reports count above size | host-value stop; no underflow/repeat loop (W2) |
| proc_exit valid/invalid, malicious host returns | Raw import has no result / invalid error without exit / trap (W2) |
| Startup panic with no Io and three-byte writes | Full reason/source positions on stderr, LF, exit 1 (W4) |
| Dependency macro invents @js; compiler-run initializer needs it | Expansion error / macro-program compile error (M1) |
| Caller supplies @js to wrapper macro; quote stores its text | Origin rule permits preservation / syntax data does not execute (M1) |

### Confirmation-review fixtures [L1–L4]

The shared-constructor package is complete ordinary source plus the new map:

```i
(Family(..))

Foreign {}
@js("(() => { throw Error('must initialize'); })()")
foreign: Foreign

Family { Web NonWeb }


main = (f ! Family) {
  f |> {
    Web -> foreign == foreign
    NonWeb -> True
  }
}
```

```toml
name = "app"
[providers."app/main.Family"]
discriminant = {js="app/main.Web", nodejs="app/main.Web", native="app/main.NonWeb", wasi="app/main.NonWeb", host="app/main.NonWeb"}
```

On js **and** nodejs, Web equals map[current_target]. The declaration is
retained; its initializer executes once and fails with "must initialize"
before main. An optimizer cannot erase it. On native/wasi/host, the Web clause
alone is pruned and no foreign declaration is retained (F2/L3). A choice
`Web | NonWeb` is retained on every target because one alternative equals the
actual constructor. A guarded Web clause remains retained on js even with
False as guard. An alias of Web resolves to Web and has the same result.

| Test / concrete variation | Required result and rule |
|---|---|
| T10 with reply Int 17 | Config is constructed before Await; main computes 17; wrapper has no unfilled Config and metadata async true (L1/T9/T10) |
| T10 make_config reads Await during construction | Await -> Config -> Await cycle diagnostic, no bootstrap/default substitution (T9) |
| T11 as written | "inside"; one adapter, no missing Config; first status 0 with async true metadata (L2/T11) |
| T11 Config override returns Config("override") | "override"; override executes and the seed is not installed as the chosen service (T9) |
| T11 Session override is a pure constant | Still runner role and async true; same adapter remains selected (L2/T9) |
| Entry needs A and B, row A names with_a and row B names with_b | Compile error names A/with_a and B/with_b plus entry; no ordering/composition choice (L2) |
| Same two rows name aliases of with_a | One resolved adapter, called once (T9); aliases must be direct binding aliases resolving to that function |
| Adapter requires Config outside; Config factory requires a supplied seed | Compile error at outer Config, even though a seed exists inside (T9) |
| Parameterized Cell(Int)/Cell(String) used by separate scoped factories | Two nodes with checked type arguments; no accidental union in one environment (spec §9.2/T9) |
| Cell(A) factory needs Cell(List(A)); finitely nested Box requires its parameter | Both reject a repeated constructor on an active dependency path; sibling Cell instances remain legal (T9 finite expansion) |
| Selected factory error type String | Provider type diagnostic before linking; no widened export error row (T6/T9) |
| Adapter calls body twice in consecutive statements | Second invocation stops: provider: body already called (T9 lifetime) |
| Adapter returns without invoking body | provider: body was not completed; no fabricated successful entry (T9 lifetime) |
| Adapter stores body, returns, then a later entry calls saved body | First return stops; later invocation, if reachable after host handling that stop, stops: provider: body is not active (T9 lifetime) |
| Copy private root-runner function path from dependency into root plan | Access diagnostic; declaring package's own plan can reference it without a public API (T9) |
| Shared-Web fixture on js and nodejs, every optimization level | Initializer retained and throws before main (L3/F2) |
| Settings via run/test with an available file `<<1, 2>>` | Reads those bytes using native, still in process (L4/T6) |
| Missing service under check, check --target native, build, run | Source-only success if source is well typed; other three reject the unsupplied entry (L4/P2) |
| JS-only binding under check then check --target native | Source-only success; targeted check reports retained unavailable declaration (L4/F2) |
| await_owned exits while Waiting / Owner closes after raw Ready | Individual request cancelled / Ready answer survives and can be taken (A2, landed runtime §§5.1/5.4) |

Adapter conflicts, role overrides and noninjective maps are plan behavior, not
additional source inference. Executable implementation tests must run these
fixtures after the plan and R2 tracks land; parsing alone proves none of the
future runtime results.

### Attack that changed the draft

These are concrete source/sequence attacks; declarations of future services
are fixtures. They exposed issues beyond merely copying the review fixes.

**Soundness: address reuse and malicious conversion.**

```i
(i/fail, wasi = i/wasi)

probe = (w: wasi.Wasi) {
  p = w.alloc(8)
  w.free(p)
  again = fail.catch(() { w.free(p) })
  q = w.alloc(8)
  stale = fail.catch(() { w.load(p, 0, 1) })
  (again, stale, p == q)
}
```

Checking only physical bounds still accepts stale access if q reuses p's
address. W1 now separates never-reused tokens from physical allocations:
(Error("closed"), Error("closed"), False). A JS bit object whose `bytes` getter
changes between validation and encoding similarly defeats a shallow check;
F4 now snapshots once and rejects shared/detached storage.

**Cannot be built: constructed errors and mixed request modes.**

```i
(i/request)

Value {}
Thrown { Thrown(Value) }

@host("probe:js/v1", "request")
Bad { Bad(wait: (Value) -> Request(Result(Value, Thrown))) }
```

§10.3 and runtime §5.2 reject this declaration. A1 now returns only opaque
Outcome, with separate synchronous accessors and i decoding. JsRequests is
separate from synchronous JsSync, with both inside the ordinary Js bundle.
Otherwise a mixed raw host declaration would impose request semantics on
converters, or a second public environment requirement would escape a guarded
Target branch. Constructing the bundle performs no request. W2 likewise uses a Nil
i signature for the raw void proc_exit instead of an inadmissible Never ABI
result. Raw ABI fields returning File(Int) also remain rejected under A3.

A further concrete buildability probe is:

```i
(js = i/js)

@js("() => Promise.resolve(7n)")
seven: js.Value


main = () { js.to_int(js.await(js.call(seven, []))) }
```

The current SDK's unconditional synchronous thenable rejection would reject
the promise when the Outcome accessor returns it as Value. F4 now requires a
general fix: synchronous externref results are data; only request starters
adopt promises. The analogous `@js("() => ({ get then() { throw 7; } })")`
result must remain opaque until an explicit operation inspects it. This is
an implementation finding from the cited SDK source, not a run of new APIs.

**Source conflict and reachability: shadowed Target and startup use.**

```i
(js = i/js, target = i/target)

@js("({})")
object: js.Value

uses = (t: target.Target) {
  t |> {
    target.Js(_) -> { object == object }
    target.NodeJS(_) -> { object == object }
    target.Wasi(_) -> True
    target.Native -> True
    target.Host -> True
  }
}


main = (t ! target.Target) { uses(t) }
```

This program now rejects on WASI: the ordinary parameter of uses is unknown,
even though main passes its certified entry service. Moving the match into
main, or into a closure that lexically captures main's t, permits only F2's
other-target constructor pruning. Adding `forced = object` at top level
retains it independently. This is a documented rejection, not a demand for
smarter analysis. Treating every Target as the build target or deleting ordinary
initializers by unused-name analysis was wrong. Its foreign-only retention exception is an
explicit spec amendment. The factory attack `make_fs(fake_js)` returning a
live guessed Int also defeats N9; A4 removes public root-to-resource factories
and preserves granted raw factories instead.

### Review-3 attacks and fixed expectations

The deterministic-analysis witness from the review is retained in full:

```i
(js = i/js)

@js("(() => { throw Error('unused'); })()")
unused: js.Value

choose = (b) {
  b |> {
    True -> True
    False -> False
  }
}


main = () {
  _ = choose(True)
  choose(False) |> {
    True -> unused == unused
    False -> True
  }
}
```

F2 visits both result clauses, with one choose body node for both calls.
There is no certified build-discriminant subject. WASI rejects; JS runs the
retained initializer and stops before main, at every optimization level.
An implementation that proves the branch False may optimize the reads only;
it must still initialize unused. The old specialisation choice is removed.

The smallest substitution witness for an overly broad Target rule is:

```i
(js = i/js, target = i/target)

@js("globalThis")
foreign: js.Value

inspect = (t ! target.Target) {
  t |> {
    target.Native -> foreign == foreign
    _ -> True
  }
}


main = () {
  ! target.Target = target.Native
  inspect()
}
```

On WASI this must reject, because inspect really reads foreign. Its environment
binder is not a certified entry binder; F2 retains it. Capturing inspect with
`control.keep_env((_: Nil) { inspect() })` inside that substitution and invoking
it later has the same answer. Conversely, a closure directly capturing an
entry's certified t still sees that old value after a substitution: its only
pruning is justified by lexical capture, not by keep_env. If an entry is also
called recursively or passed to another i function, F2's whole-program source-
reference check disables its certification, preventing the same bug there.
This is the smallest sound syntactic boundary chosen for K2, not a hidden
value-flow analysis. It expresses T4 but deliberately rejects the helper forms.

The Ready-owner test from finding 5, inside an async A1 host fixture, is:

```i
(js = i/js, i/request)

survives = (promise: js.Value, raw ! js.JsRequests) {
  r = request.within(() {
    r = raw.observe(promise)
    request.wait_any([request.ticket(r)])
    r
  })
  answer = request.take(r)
  (answer, request.state(r))
}
```

The fixture's chosen raw request provider makes the entry async under T7.
Gate the host completion, deliver a known Outcome, then let within return.
Expected `(that_outcome, request.Taken)` and no ABI cancel for the completed
id. Closing the Owner has removed only outstanding registrations, not r's
Ready value. Explicitly cancelling r before take instead stops with
`request: cancelled`. Public await wrappers have individual guards and take
before their final cancels; this raw-handle test intentionally has no such guard.

Three designs were reconsidered for this revision: expose every raw request
service in public environments; keep bundle APIs and classify their chosen
provider closure; or infer async behavior from body/value reachability. K1
chooses the second, preserving ordinary bundles and deterministic metadata at
the cost of conservative async exports. For linking, specialisation and merged
value-flow analyses both have more acceptance rules; K2 chooses reference
reachability plus the certified-discriminant clause rule. For filesystem
fallback, a Native root or public raw factory would reopen N9; K3's ordinary
application provider avoids both. Criteria remain §1's generality, principal
inference, predictable acceptance and buildability.

### Verification and final attacks

The revision-6 memory-model results remain historical evidence, not validation
of the new provider/link rules. For revision 7, the review's existing
`/tmp/review-js3/opaque.mjs` probe was run against its cached Wasm fixture and
this lane's SDK. It reproduced True for the same object under A/B, False for
different objects, and True for null under A/B. This verifies the P1 defect;
it does not test a fix. No compiler rebuild or new JS/R2 implementation was run.

All 16 i blocks parsed with cached accepted-syntax compiler i-1df2fe0, using
fmt --check on in-memory files (formatting differences allowed). TOML and JSON
blocks parsed, and fence/table/citation/diff checks passed. This verifies
syntax, not future provider keys or API types. An abstract provider model
checked the same closure rule on three differently named bundles and all
five Settings target selections. A separate six-order model checked
completion/Owner-close/take, including Ready survival and explicit handle
cancellation. These finite models are not compiler or scheduler implementations.

Initial revision-7 attacks exposed why certification must exclude a helper's
environment binder and an entry with source callers; F2 now states those
conditions. A provider factory raising an undeclared host-export error was
another real hole; T6 now requires its ordinary error type to be Never.
That preserves the export's source error row without a hidden metadata union.

Pass 1 after those fixes traced direct aliases, shadowing, lexical captures,
keep_env, higher-order references, recursive entries, guarded clauses, direct
versus non-direct initializers, and unchosen provider functions through the
fixed reference graph. It traced both provider branches of Settings and the
Js/user-bundle/export wrappers, including a pure replacement provider and an
explicit synchronous contract. No further serious defect was found.

Pass 2 independently checked coalesced opaque imports, nulls, external/async
replies and export round trips against P1; Ready escape versus individual
request guards; provider cycles, missing supplies and error-free construction;
and the retained custom-WASI, checked-buffer, conversion, macro and N9 rules.
The K1/B.7 conflict remains explicitly flagged for the coordinator. No further
serious defect was found by manual analysis. Neither pass is a soundness proof
or an execution of the unimplemented provider/linker/nominal-provenance changes.

### Revision 8: attacks, fixes and verification

The first attack used T10's Config-dependent Await and T11's Config override.
A prefix wrapper leaves Config unsupplied; blindly binding an adapter's Config
would change "override" back to "inside". T9 removes the prefix and makes the
adapter's value an explicit seed, so the chosen factory always decides the
final service. The shipped run type also needs Runner outside and supplies it
inside: collapsing those values would invent a cycle. Separate lexical scope
nodes solve it without changing the source scheme. Plan references to private
owned factories preserve runtime's private root-runner constructor; importing
that private path from another package is rejected.

A concrete termination attack replaces a package with:

```i
Cell(A) { Cell }
make_cell(A) = (next ! Cell(List(A))) -> Cell(A) {
  _ = next
  Cell
}

main = (c ! Cell(Int)) { Nil }
```

Selecting make_cell for Cell produces infinitely many distinct instantiated
needs. T9 now rejects a repeated constructor on a dependency path, including
through shared nodes. This costs finite nested Box plans too; it does not
reject independent Cell(Int)/Cell(String) constructions. No optimizer or
type-level termination proof is allowed to change that acceptance rule.

A concrete lifetime attack replaces T11's with_session body with this
well-typed ordinary function:

```i
with_session(A, E, N) = (body: () -> A ^ E ! N) -> A ^ E ! N - Config {
  ! Config = Config("inside")
  _ = body()
  body()
}
```

Its ordinary type alone cannot promise one invocation. T9's generated guard
stops the second call, and rejects a call after the adapter extent. This is a
runtime invariant on every adapter, not a trusted-library exemption.

After these fixes, two further passes found no serious issue:

1. Soundness/build pass: traced T10 with no seed, T11 with a required seed,
   an override ignoring its seed, same/different adapters, outer versus inner
   Runner, missing outer seeds, duplicate instantiated services and expanding
   type arguments. Verified each factory's arguments/remaining environment and
   the resulting wrapper's empty environment. Traced suspension and error
   exits against the single-call guard; no scope can fetch an inner prerequisite
   before adapter entry. Rechecked that constructor recursion is rejected even
   when a dependency was previously memoized. These are manual derivations,
   not a compiler execution of the future plan.
2. Source-conflict pass: traced shared Web, constructor aliases and choices,
   False guards, captured/rebound Target and the retained choose rejection;
   checked source-only versus targeted validation and the Settings columns.
   Compared the resulting contract to landed runtime/native/multicore: private
   Runner bootstrap, nullary Native, private IO Ints, owned versus borrowed
   waits, Ready survival, program/control accounting and the MC-D gate remain.
   The explicit §9 amendments cover changed provisioning/metadata; blocking
   WASI/custom imports and principal source inference are unchanged.

Run/read evidence for this revision: all 23 i fences parsed with cached
accepted-syntax `i-1df2fe0 fmt --check` through memory file descriptors; five
TOML and two JSON examples parsed with Python's standard parsers. Formatting
is not type checking. The four existing review-4 packages (bootstrap, Settings,
generic scopes, rebinding) ran successfully with that compiler; no new files
were written for these probes. They test ordinary language behavior, not R2.
An abstract Python check covered 8,505 constructor-map/choice cases, including
shared constructors, and asserted dependency ordering, cycle rejection,
outer/inner scopes and adapter conflicts. This model is not the future linker.
Code-region reads and git diff --check supplement these checks. No cargo build
or test of unimplemented JS/provider features is claimed.

## 11. Risks and coordinator questions

No unresolved semantic choice blocks handing §§8/9 to implementers. Source
amendments in §9 are an explicit prerequisite, not a claim that today's spec
already permits ordinary entry services or @js. The ledger's WASI exception,
provider/contract spellings, F2 certification, P1 ownership and native N9 are
retained. L1–L4 now have one executable plan contract; competing adapters are
a compile error rather than an open composition question.

Coordinator scheduling question: assign the new plan/CLI track and serialize
its wrapper/host-emitter changes with P1 and R2. Recommended answer: use §8's
track order and region ownership; P1 and field privacy already have owners.
No additional language decision is requested. General callback re-entry and
future adapter composition remain outside this handoff, not hidden blockers.

Unverified: future provider-plan/checker/link implementations, full type-checking
of request-era fixtures, generated artifacts, browser profiles, real request/
resource cleanup and performance. The complete packages above target the
specified new schema; today's manifest reader does not implement it. Primitive
root-key plumbing and adapter lifetime guards need R2/compiler integration.
The retained conversion, checked-memory, N9, custom-import and macro rules
remain required. Native packaging and MC-D keep their accepted separate gates.

## 12. Review disposition

| Review finding | Changed rule and location |
|---|---|
| 1 blocker: service granularity and caught unsupported | T1/T2/T3 (§2), exact split and accepted cost; §9 runtime handoff |
| 2 blocker: resource representation and cleanup | A3/A4 (§5), native N9 on every target; W1/W2 (§6) never expose OS fds as File |
| 3 major: other-target foreign imports/null | F1/F2/F3 (§3), required target linking and compile errors |
| 4 major: removal of custom WASI imports | T5 (§2), W4 (§6), B.7 handoff (§9) |
| 5 major: inadmissible promise errors and late cancellation | F4 (§4), A1/A2 (§5), opaque outcomes and start-time signal |
| 6 major: memory lifetime and proc_exit | W1/W2 (§6), checked token table and void-result case |
| 7 major: incomplete conversions | F4 (§4), exhaustive outcome table and copy semantics |
| 8 major: macro escape | M1 (§7), exact meta §§4.3/8 additions in §9 |
| 9 minor: target coverage and missing Io import | T4 (§2), both corrected complete examples |
| Kernel/provisioning claims | §1, T1, F4, A4, W2: actual fixed duties and ordinary provider composition |
| Factory/WASI/native scope, ABI and callbacks | A4, T3/T5, F3, implementation order (§8), conflicts (§9) |
| CLI aliases, exports, initializer scope/order, Node loading | T3, F1–F3 (§§2–3) |

Decisions beyond rulings, with reasons/sources: T1's general provider plan
implements the user's defaults without library-specific lowering (spec §9.2);
T2's five service names separate preview-1 cwd and child capabilities (review
1/runtime §7.4); T3/T4 keep ilang as an alias and Native nullary (native N1–N4,
review reconciliation); F1/F2 choose getter initialization and conservative
foreign-only retention (spec §2.5/§4.7, review 3); F3 versions target metadata
outside v3 and uses global ESM scope/dynamic imports (B.6, review minors);
F4 uses synchronous opaque outcomes and primitive conversion errors (spec
§10.3, review 5/7), a Js bundle to discharge both raw service requirements
(spec §9.2), snapshot validation to preserve immutable values (§11.1), and
opaque synchronous externref results despite thenable payloads (§10.3/B.2;
SDK `ilang.mjs:836–839`); A4 shares accepted factories (N9/runtime §8.2); W1/W2 reduce the first
adapter to a checked slice (J6, allocator evidence); §8 keeps callbacks behind
a separate gate (B.3–B.5/runtime R2). These are design choices, not additional
claims of user rulings or implemented features.

### Review of 9c28dd1

| Finding | Changed rule and place |
|---|---|
| 1 blocker: async hidden behind bundles | T7/T8 (§2), P2 (§8), exact D5/§10 replacement (§9.1); promise, user bundle and synchronous export wrappers/metadata |
| 2 blocker: implementation-dependent retention | F2 (§3), fixed reference graph and certified-discriminant pruning; choose, aliases, closure, substitution and recursion tests (§10) |
| 3 major: resource fallback cannot discharge Fs | T6 (§2), complete ordinary-i Settings package and per-target provider plan; no effect deletion or raw-number injection |
| 4 major: opaque nominal identity erased | P1 (§8), general compiler prerequisite and all-boundary tests; F4 no longer claims the current bridge suffices |
| 5 major: Owner closure discards Ready | A2 (§5), runtime §§5.1/5.4 unchanged, survivors test (§10) |
| Handoff: checker rejects ordinary services first | P2 (§8), move supply validation after inference and before linking/lowering; keep ordinary entry/ABI checks |

New choices beyond the rulings: error-free provider construction preserves
source export error rows (spec §10.3); root-only provider-table precedence and
`wasm.synchronous` spellings serialize T1/K3 and K1's diagnostic option (spec
§2.6 amendment); provider roles and a generated discriminant constructor map
make K1/K2 independent of library names; F2's entry-only certification and
source-reference exclusion follow §9.2 and the substitution counterexample;
activating referenced closures without callee-set analysis fixes the finite
context-insensitive graph required by K2; P1's guest-side nominal/ref pair
implements §4.1/§11.4 while keeping B.2's externref wire; the accepted K1 target
qualification preserves higher-priority B.7.
Earlier decisions and their sources in this section remain, except the old
F2 value analysis and direct-environment async rule, which K1/K2 supersede.

### Confirmation review of e821f9a

| Finding / ruling | Changed rule and place |
|---|---|
| 1 major / L1: context prefix violates factory dependencies | T7/T8 primitive admission, T9 complete graph/order, T10 typed Config/Await elaboration (§2); exact runtime D5/§5.1 replacement (§9.1) |
| 2 major / L2: incomplete plan and adapter contract | T9 schema, types, seed inputs, scopes, lifetime guard and roles; T11 complete user role/adapter (§2); P2/ordered ownership (§8.1); spec §2.6 handoff (§9) |
| 3 major / L3: shared constructors pruned | F2 resolved C != map[current_target] for every choice alternative (§3); shared-Web source/map and retained initializer test (§10) |
| 4 minor / L4: command provider target/check promise | T3 command table (§2), current CLI anchors and plan track (§8.1), exact runtime §10 replacement (§9.1) |

Decisions beyond L1–L4 and their sources:

- Typed seed lists let ordinary adapters supply prerequisites while chosen
  factories still determine services. Separate outer/body instances avoid
  a self-cycle for runtime §6.1's required/supplied Runner; concrete type
  arguments and lexical bindings preserve spec §9.2. Factory environments
  are not specialized by seed contents (K1/K2 and principal inference).
- `entry-root-key` is an explicit typed compiler argument for runtime §5.1's
  already-required narrow bootstrap; admission itself supplies no service
  value (L1). Registration remains entry-scoped, with no public unscoped
  allocator and no IO raw numbers exposed (runtime §5.1/native N9).
- Plan references carry declaring-package authority, including own private
  bindings, so runtime §6.5's private root-runner constructor stays private.
  Cross-package references still require exports. This is a general §2.6
  plan visibility amendment, not a source privacy exception for a library.
- Stable topological tie-breaking makes factory side-effect order predictable
  (spec §2.5's ordered evaluation and L1); full closed requirements and row
  identity make overrides checkable (L2). A seed-required flag rejects an
  impossible outer bootstrap instead of inventing a provider. Constructor
  recursion on one dependency path is conservatively rejected, including
  finite nested Box plans, to bound generic expansion without a new type-level
  termination analysis (principal-inference requirement and a finite build).
- An ordinary generalized adapter plus runtime single-call/active-extent guard
  gives a structured lifetime without adding linear or rank-two source types
  (runtime structured concurrency, spec §4.7, L2). Direct aliases identify one
  adapter; arbitrary function-value equivalence is not analyzed (K2).
- Targeted check validates all eligible entry/test plans, with B.7's no-test
  exception; source-only check makes no availability promise (L4). Command
  execution retains native N1's in-process run/test and native build default.

Settings, principal inference, T7 classification and synchronous-export
contracts, the accepted WASI exception, F2 certification, P1 and P2 are
retained. Remaining source amendments and implementation risks are explicit
in §§8/9/11; no remaining native/runtime/multicore conflict is silently waived.
