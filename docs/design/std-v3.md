# Standard library v3: decisions and delivery plan

14 November 2026. Phase 1 synthesis. Branch `lane/stdv3`.

Authority, in order: [specification](../spec.md),
[syntax three §2.10](syntax-3.md#210-services-errors-and-the-environment),
then this design. Evidence: the complete [inventory](std-v3-inventory.md),
[Effect capability audit](std-v3-effect.md), and
[style and stability audit](std-v3-style.md). Their source baseline is
`68b720e9`; this synthesis starts at `aa03523e6`. The audits describe source,
not passing tests. This document changes no implementation.

Effect 4.1.3 is the capability reference. Its package version does not make
its platform APIs stable. Reference paths below name the supplied checkouts
under `.rd/ref/`, as recorded by the audits. They are source references,
not a requirement to download a moving upstream version.

## 1. Principles

The standard library makes i a useful standalone language. A program should
read files, validate input, call and serve HTTP, manage concurrent work, and
explain its failures using the shipped distribution. Gleam supplies the model
for small, readable modules. Go supplies precise, composable IO contracts.
Effect supplies the breadth and the lifetime discipline. i supplies the means.

A computation is an ordinary function `(P) <- (S) -> R`. Errors are the need
`Fail(E)`. A service is an ordinary labelled value. Service construction,
sharing and provision are ordinary calls, bindings and `<-`. All values stay
immutable. Scheduler-owned state is accessed through immutable handles and
pure `task.update` transitions. Every child belongs to a task group. Every
resource has an enclosing lifetime. Cleanup may wait; a timeout waits for
cleanup too. A stop is not a recoverable error and runs no i finalizers.

The library is the language's best textbook. Write the most natural code
available in the language now. Every public function has a doc comment with
its purpose, needs, errors and a runnable example. Every module starts with
how to use it and why its design has this shape. Tests are executable examples
of observable behavior. State units, callback order, costs, laziness and
ownership where they matter. Prefer one function fewer to an inelegant one.
Do not preserve an alias merely to keep an export count high. Preserve useful
algorithms and their regression examples. Implement each policy in one place.

The following decisions cover every exclusion in the Effect audit, §20.

| Do not copy | Reason and i replacement |
|---|---|
| Effect wrapper, `gen`, effect-map/run DSL | Function calls and native needs already sequence computations. |
| Context tags, string service keys, Layer graph or automatic memoization | Nominal service types, explicit construction order, bind-once sharing, `<-`. Scoped factories must still supply Layer's useful lifetime guarantees. |
| Separate error channel or tag-specific recovery family | `Fail(E)`, union narrowing, ordinary matching, one `i/fail`. |
| Full Cause/Exit capture of defects and interruption | Preserve stops, cancelled-join rules and least-task-path failure selection. Diagnostics are data; catch cannot observe every outcome. |
| Detached fibers, general interruption masks, worker placement API | Structured groups, cancellation-safe handoffs, host-selected workers. |
| Mutable Ref/collections, synchronized mutation protocol | Existing scheduler state and pure atomic replacement; owner tasks for effectful protocols. |
| Match builder, Data classes, tagged-error hierarchy | Branches, guards, labelled constructors and unions are language features. |
| Custom equality/hash protocol | One language equality and its compatible hash. Domain comparisons are explicit functions. |
| Either, second BigInt, Array/Chunk/Record replicas | Existing Result, arbitrary Int, List, Sequence, Dict and closed data. Decimal has different semantics and earns a type. |
| Schema classes or compiler-known derivation | Immutable schema descriptions, ordinary interpreters and optional ordinary macros. |
| Second streaming evaluator, seven-parameter public Channel, EOF encoded as failure | Ordinary pull functions, explicit end data, scoped consumption and small sinks. Task channels retain their separate rendezvous meaning. |

No standard module gains compiler privilege. Existing `big`, `bits` and
`float_text` runtime hooks must use ordinary linking to checked definitions,
with any unavoidable primitive selected through the generic primitive ABI.
A and the compiler migration owner must reject path-name recognition, bypassed
checking or module-specific optimization. Renaming files does not settle it.
The baseline gate covers this; this plan adds no new privileged hook.

## 2. Conventions: settled rules

### 2.1 Names and calls

Use snake_case for modules, functions, bindings and fields. Use UpperCamelCase
for types and constructors. Use nouns for observations (`size`, `keys`) and
verbs for work (`read`, `decode`). Predicates use `is_`, `has_`, or `contains`:
`is_empty`, `has_scheme`, `contains(collection, item)`. Use `new` for empty
collections and `singleton` for one element. Keep `then` for Option/Result
data; use `flat_map` for sequences. Do not introduce `try_` or `map_effect`.

Put the subject first, configuration next, callback last. Fold callbacks take
the accumulator first. A builder is the subject while being built. Input data
is the subject while being decoded. Thus `schema.decode(value, codec)` and
`regex.find(text, pattern)`. Cryptographic input bytes precede key/salt/options.
Use `from_` and `to_` for representation boundaries. Name index units in docs;
raw clock names retain `_ns`, while policy APIs take `Duration`.

Use distinct arities for default/configured shapes of one operation. Use a
label to distinguish eager and lazy alternatives of equal arity: `default =`
versus `make =`. They are shapes, not optional parameters or type overloads.
All shapes must have one coherent result and need contract. Do not merge a
pure function with an IO function under one group.

This current-library example demonstrates naming, the pipe's position-zero
rule, and callback-last order. It does not depend on any new API:

```i
(std/list)

doubled = (values: List(Int)) -> List(Int) {
  values |> list.map((n) { n * 2 })
}
example = () { assert(doubled([2, 3]) == [4, 6]) }
```

### 2.2 Errors and data

Use `Fail(ModuleError)` whenever an operation cannot deliver its requested
answer, including pure parsing and validation. Use Option for expected
absence. Use Result for caught outcomes that are stored, inspected or
collected. `fail.catch` handles typed failure, not Assert or a stop. Catch at
recovery/reporting/storage boundaries. Do not catch each internal call.
Protocol progress and validation reports remain explicit domain data.

Each module owns one public error family, named `ModuleError`; its cases have
recovery meaning and useful fields. Existing `FsError`, `NetError`, etc. keep
those qualified names. Supporting reason/path enums are not extra error
channels. Consolidate today's CLI definition errors and URI codec errors into
their module family. Unrelated module errors compose as `Fail(A | B)`.
Wrap a lower error only to add domain context; retain it in a typed field.
Never turn an upstream error into an English string for recovery. Adding a
case to a stable closed error family is breaking.

This complete example already uses syntax three and today's failure API:

```i
(i/fail, std/list, std/option)

BadPort = (text = String)
PortError = BadPort
parse_port = (text: String) -> Int {
  text |> ("80") { 80 } | ("443") { 443 }
    | (_) { fail.raise(BadPort(text = text)) }
}
ports = () { list.map(["80", "443"], parse_port) }
example = () {
  assert(fail.catch(ports) == Ok([80, 443]))
  assert(fail.catch(() { parse_port("bad") }) == Error(BadPort(text = "bad")))
  assert(list.first([]) == option.None)
}
```

To compose errors, call both operations normally. To recover selectively,
match `Error(e: WantedError)` and re-raise the narrowed remainder. No generic
`Error` hierarchy, universal Cause service or error-code string parser is added.

### 2.3 Services, defaults and tests

A service is a nominal labelled type with the smallest useful function fields.
Name its responsibility: `Clock`, `Client`, `ConfigProvider`, `Tracer`.
Two independently replaceable roles have two constructors. Provider functions
name their choice: `native_client`, `fetch_client`, `stderr`, `memory`, `fixed`.
Return an ordinary service or positional group of services. Bind once to share;
call twice to isolate. State captured services versus call-time needs explicitly.
Never capture a live Fail handler in a long-lived provider.

Choose defaults explicitly in program entry code. Platform services are only
those of spec §9.7. Imports and package files install no other services.
This follows syntax-three §2.10.11 over the style audit's §4.5 provider-plan
suggestion: that suggestion describes removed machinery. Test replacement is
ordinary provision. Resource-backed factories require Scope around all uses.

```i
Settings = (port = () -> Int)
port = () <- (settings: Settings) { settings.port() }
local = () -> Settings { Settings(port = () { 8080 }) }
example = () {
  live = local()
  assert(port() == 8080) <- (live)
  assert(port() == 9000) <- (Settings(port = () { 9000 }))
}
```

### 2.4 Module boundary

`i/…` holds specified language/runtime support, host contracts and minimal
owned-request adapters. `std/…` holds data structures, protocols, codecs,
policies and reusable algorithms, including those with host needs. `i/` never
imports `std/`. Shared small traversals may live in an ordinary internal
foundation where an import cycle forbids reuse of `std/list`; that does not
create a second public API. Keep specified `i/random` in place. Do not move
specified primitives or break host namespaces as a cosmetic cleanup.

Generic inspection and hashing have one public home, `i/core.inspect/hash`.
Type-specific number formatting remains in int/float because it is a
representation operation. Generic string/byte inspection aliases are deleted.
`i/io.Io` remains the specified minimal best-effort print capability;
`i/stdio` remains fallible byte IO. Their different error promises are explicit.

```i
((Io) = i/io)
main = () <- (io: Io) { io.print("hello") }
```

### 2.5 Doc and test form

Use `////` for the module introduction. It gives the normal composition first,
then the design reason, units, lifetime and stability. Use `///` on every
exported value, type and visible constructor. Avoid a handwritten duplicate
signature catalogue in source comments. The checker owns signatures.

```i
//// Integer list transformations. Ordinary callbacks keep their native needs.
//// Stability: experimental. Since: next. Tracking: docs/design/std-v3.md.
(std/list)

/// Doubles each integer, preserving order.
/// Needs: none. Errors: none.
/// Example: doubled([2, 3]) == [4, 6].
doubled = (values: List(Int)) -> List(Int) {
  values |> list.map((n) { n * 2 })
}
```

Each `Example:` expression is checked in an ordinary test body with the
module's declared example imports. Multi-statement examples use fenced i
blocks, an explicit `example = () { … }`, and assertions. The docs runner must
execute them, not merely parse the comments. Deferred APIs state when callbacks
run. Resources state who closes them and what happens on early exit. Examples
must work on main with the documented package dependency; proposed APIs in
§3 are contracts for implementation, not claimed current exports.

Tests use the existing macros and Assert. Name the observable behavior.
Table-driven examples cover related operations. Add laws where useful, not a
new test per trivial wrapper. Need-bearing callbacks, failures, empty inputs,
limits, early exit and cancellation need visible examples. Assert failures
stay separate from Fail. No test keyword or special standard-library runner.

```i
(testlib = (test) = std/test, std/list)

test("mapping preserves order", () {
  assert(list.map([2, 3], (n) { n * 2 }) == [4, 6])
  Nil
})
```

### 2.6 Decisions where audits differ

| Question | Decision and reason |
|---|---|
| Keep Result traversals from Gleam? | Delete list.try_map/try_fold/try_each. `fail.catch(() { list.map(xs, f) })` supplies control; `result.all` handles stored data. Style's optional retention adds a synonym. |
| Keep provider plans? | No. Syntax-three §2.10.11 and spec §9.7 require explicit program choices. |
| Expose Effect Cause/Exit? | Diagnostic Cause data only, no general exit observer. Spec cancellation and stop semantics take precedence. |
| Canonicalize Dict and Sequence to make `==` extensional? | No rushed representation rewrite. Document structural equality, add explicit content comparisons, and keep these containers experimental until reviewed. There is still one language equality. Arbitrary collision keys have no universal order for canonicalization. |
| Replace JSON scalar constructors by built-ins now? | Keep existing JSON tags. This preserves explicit wire-number intent and limits migration work; Node's numeric syntax migration does not mandate JSON's representation. |
| Move every pure `i/` algorithm? | Keep specified runtime support in place; review hooks separately. No name-based compiler privilege is legitimized. |
| Stream CSV errors as Result elements? | Pull raises CsvError. `stream.attempt` is the explicit capture boundary. End remains data. |
| Copy all Go packages / all Effect exports? | No. Finish the paths in §5. Specialized wire formats, named zones, transactions and host-worker interop are named extensions, with no fictitious implementation in today's estimate. |

## 3. Finished module map

### 3.1 Reading the catalogue

This is the release surface, including retained items. An omitted old export
is deleted only when its deletion is stated below. Each entry gives its
purpose, public types/functions, reference and migration. `private` means a
nominal type with hidden constructor, not the language's Opaque foreign type.
Visible constructor definitions are public items too. Prelude types such as
Result and BitArray are used, never redeclared. Free letters are universal.
Aliases in these contracts are `ifs = i/fs`, `iprocess = i/process`,
`fs = std/fs`, `process = std/process`, `net = i/net`, `tls = i/tls`,
`fetch = i/fetch`, `stdio = i/stdio`, `clock = i/clock`, `random = i/random`,
and otherwise the matching std module. Unqualified Scheduler is i/task's.
Types local to a module keep their unqualified name. Blocks labelled `text`
describe contracts; they are not source declarations.

To keep host rows readable, `IO[S,E]` below expands to the need
`(S, Fail(E), request.Await, request.Owner)`. `IO[S]` omits Fail. Commas in S
mean several services. `Tasks[N,E]` means `(..N, Fail(E), task.Scheduler)`;
a child callback explicitly receives Scheduler, Runner and request.Await as
specified in §9.9. Owner and Root are never silently removed. `ClockNeeds`
means `(time.Clock, Fail(time.TimeError), task.Scheduler, request.Await,
request.Owner)` for the high-level clock. These abbreviations are documentation only, not new row syntax or
language aliases. A bare need N can contain Fail; when Fail(E) is written
separately, the residual N excludes Fail by syntax-three rule B. Fixed-key
rows in these tables are merged needs. Implementations must extract one
checked residual row and preserve the lack constraints, not concatenate
service unions or silently capture Fail. Validate these heads before bodies
are written, including callbacks whose state services require Scheduler. Deferred containers' E
parameters below occur only in their ordinary `Fail(E)` callback need; they
are not another error channel or an evaluator.

References use these exact local roots:

| Prefix | Source root |
|---|---|
| E | `effect/packages/effect/src/` at audited Effect 4.1.3 |
| G | `gleam-stdlib/src/gleam/` |
| GH | `gleam-http/src/gleam/http/` |
| Go | `go/src/` |

All changed/new contracts start experimental. A stable candidacy is given in
§4; maturity never substitutes for the tests in §5. The catalogue includes
small complete initial APIs, not unimplemented aliases for every reference
operation. Explicit extensions at the end are outside this day's release.

### 3.2 Language support and native effects — lane A

| Module | Purpose, reference and today's disposition |
|---|---|
| `i/fail` | The sole active typed-error API. E `Effect.ts`; native translation in spec §9.3. Keep all four names; replace intrinsic raise/catch by the ordinary service implementation. |
| `i/failure` | Delete the entire transitional module. Move its implementation into i/fail; migrate imports. No compatibility execution engine. |
| `i/assertion` | Capture Assert only for test/report data. Spec §9.6; Go `testing/testing.go`. Keep catch. |
| `i/control` | Native control primitives. Spec §9.4. Keep names; remove separate E from Boundary/Continuation. Keep keep_env until E6's library-keep gate passes. |
| `i/resource` | Guaranteed cleanup and one resource lifetime service. E `Scope.ts`, `Effect.ts`. Keep ensure; change acquire from bracket to registered two-function acquisition; add Scope/scoped. Existing use callbacks become calls inside scoped. |
| `i/key` | Typed identity and heterogeneous values. Spec §9.8. Keep Lookup/Found/Missing and all three functions; do not replace the specified sum by Option. |
| `i/request` | Host-operation identity, ownership and waits. Spec §9.11. Keep all current public entries, normalized to one need row. Requests are not Deferred or logical batches. |

```text
// i/fail
raise : <E>(E) <- (Fail(E)) -> Never
catch : <A,E,N>(() <- (..N, Fail(E)) -> A) <- N -> Result(A,E)
from_result : <A,E>(Result(A,E)) <- (Fail(E)) -> A
map_error : <A,E,F,N>(() <- (..N, Fail(E)) -> A, (E) <- N -> F)
  <- (..N, Fail(F)) -> A
// i/assertion
catch : <A,N>(() <- (..N, Assert) -> A) <- N -> Result(A,AssertionFailure)
// i/control; Boundary and Continuation remain built-in types.
boundary : <R,N>((Boundary(R,N)) <- N -> R) <- N -> R
suspend : <A,R,N>(Boundary(R,N), (Continuation(A,R,N)) <- N -> R) -> A
resume : <A,R,N>(Continuation(A,R,N), A) <- N -> R
cancel : <A,R,N>(Continuation(A,R,N)) <- N -> Nil
escape : <R,N>(Boundary(R,N), R) -> Never
keep_env : <A,B,E,N>((A) <- (..N, Fail(E)) -> B) <- N -> (A) <- (Fail(E)) -> B
// i/resource
Scope = private service
ensure : <A,N>(() <- N -> A, () <- N -> Nil) <- N -> A
acquire : <A,N>(() <- N -> A, (A) -> Nil) <- (..N, Scope) -> A
scoped : <A,N>(() <- (..N, Scope) -> A) <- N -> A
// i/key
Found(A) = (A)
Missing = ()
Lookup(A) = Found(A) | Missing
new : <A>() -> Key(A)
wrap : <A>(Key(A),A) -> Keyed
unwrap : <A>(Key(A),Keyed) -> Lookup(A)
// i/request
Await = (wait = (Ticket) -> Nil)
Root = (key = Key(Nil))
Owner = (key = Key(Nil), root = Root)
Waiting = (); Ready(A) = (A); Cancelled = (); Taken = ()
RequestState(A) = Waiting | Ready(A) | Cancelled | Taken
open_root : () <- (Root) -> Root
close_root : (Root) -> Nil
open_owner : () <- (Root) -> Owner
close_owner : (Owner) -> Nil
count : (Owner) -> Int
count_root : (Root) -> Int
settled_owner : (Owner) -> Bool
settled_root : (Root) -> Bool
cancel_all : (Owner) -> Nil
cancel_all_root : (Root) -> Nil
wait_none : (Owner) <- (Await) -> Nil
wait_none_root : (Root) <- (Await) -> Nil
within : <A,N>(() <- (..N, Owner) -> A) <- (..N, Root) -> A
ready : <A>(A) -> Request(A)
ticket : <A>(Request(A)) -> Ticket
state : <A>(Request(A)) -> RequestState(A)
take : <A>(Request(A)) -> A
settled : (Ticket) -> Bool
cancel : (Ticket) -> Nil
await : <A>(Request(A)) <- (Await) -> A
await_owned : <A>(Request(A)) <- (Await) -> A
wait_any : (List(Ticket)) -> Nil
poll : () -> Nil
```

Scope registers releases in reverse acquisition order. Use kept non-Fail
services in release. Catch fallible flush explicitly before close; release
itself must not raise. Acquiring and registering form a cancellation-safe
handoff. Streams, servers, caches and provider factories use this Scope.
Task groups keep their own joining lifetime and never become resource Scope.

`ensure`'s final callback has no Fail in its actual need, even when the body
has Fail. The shared N notation in the spec does not grant permission to raise
from a finalizer. A's checking/examples must enforce the stated finalizer rule.

| Module | Public contract, reference and migration |
|---|---|
| `i/big` | Keep the seven support functions below. Spec Int; E `BigInt.ts` for capability comparison. No second public integer type. |
| `i/bits` | Keep wide segment algorithms. Spec §8, Go `encoding/binary/binary.go`. Checked user codecs live in std/bytes. |
| `i/float_text` | Keep shortest round-trip formatter. Spec §11.8, Go `strconv/ftoa.go`. It is the single formatting algorithm. |
| `i/core` | Keep necessary primitives and canonical inspect/hash/portable. Spec §13; E `Equal.ts`, `Hash.ts`. Raw conversion/search statuses become private adapters; public checked conversions/search use CoreError below. std/string maps a missing-search error to its ordinary Option answer. std wrappers translate domain errors, never reimplement the primitive. |

```text
// i/big: operands obey private runtime representation invariants.
compare, add, subtract, multiply, divide, remainder : (Int,Int) -> Int
to_text : (Int,Int) -> String
// i/bits: internal segment preconditions remain specified at the caller.
read_int : (BitArray,Int,Int,Bool,Bool) -> Int
from_int : (Int,Int,Bool) -> BitArray
// i/float_text
to_string : (Float) -> String
// i/core
CoreError = (operation = String, offset = Int, reason = CoreProblem)
CoreProblem = Bounds | InvalidScalar | InvalidUtf8 | NotFound
Bounds = (); InvalidScalar = (); InvalidUtf8 = (); NotFound = ()
not : (Bool) -> Bool
to_float : (Int) -> Float
truncate : (Float) -> Int
floor, ceiling, sqrt : (Float) -> Float
float_to_bits : (Float) -> Int
float_from_bits : (Int) -> Float
bit_and, bit_or, bit_xor, shift_left, shift_right : (Int,Int) -> Int
bit_not : (Int) -> Int
concat : (List(String)) -> String
byte_size : (String) -> Int
byte_slice : (String,Int,Int) <- (Fail(CoreError)) -> String
codepoints : (String) -> List(UtfCodepoint)
from_codepoints : (List(UtfCodepoint)) -> String
codepoint_to_int : (UtfCodepoint) -> Int
int_to_codepoint : (Int) <- (Fail(CoreError)) -> UtfCodepoint
to_upper, to_lower : (String) -> String
index_of : (String,String,Int) <- (Fail(CoreError)) -> Int
bit_size : (BitArray) -> Int
bit_array_to_string : (BitArray) <- (Fail(CoreError)) -> String
bit_array_concat : (List(BitArray)) -> BitArray
inspect : <A>(A) -> String
hash : <A>(A) -> Int
portable : <A>(A) -> Bool
```

The byte-offset `core.index_of` is a primitive interface; std/string's search
uses scalar offsets. Neither offset may be reused in the other API. Compiler
support declarations must remain target-neutral primitive operations. Changing
public conversion heads requires the E4 primitive adapter migration; it must
not become an intrinsic recognizer for a new std module.

`i/syntax` keeps ordinary macro support. Reference: spec §§14.4–14.8, not an
Effect reflection API. Keep all ten functions, Syntax/TypeHandle, query services
and Truth. Node becomes one labelled wrapper containing built-in Int/Float
members. Delete duplicate Int/Float constructors. Replace old Position members
Constructor/Field/Parameter/CallArgument/Clause/ClauseHead by the spec positions.
Types and private-constructor checks remain ordinary compiler query contracts.

```text
Syntax = Opaque; TypeHandle = Opaque
TypeView = (TypeHandle)
Name = (String); Discard = (String); Up = (String); Symbol = (String); Text = (String)
Round = (List(Syntax)); Square = (List(Syntax)); Curly = (List(Syntax))
Bits = (List(Syntax)); Quote = (Syntax); Unquote = (Syntax); Splice = (Syntax)
Node = (TypeView | Name | Discard | Up | Symbol | Int | Float | Text |
        Round | Square | Curly | Bits | Quote | Unquote | Splice)
Position = ImportItem | ExportItem | Item | Attribute | Type | DataTypeItem |
           FunctionResult | Expression | DataItem | Statement | Pattern |
           DataPatternItem | Segment | Format
// Each Position member has data ().
SyntaxError = (message = String, at = Syntax)
Definitions = (definition = (Syntax) <- (Fail(SyntaxError)) -> Syntax)
Items = (add = (Syntax,Syntax,List((String,String)),Syntax) <- (Fail(SyntaxError)) -> Nil,
         expose = (Syntax) <- (Fail(SyntaxError)) -> Nil)
Diagnostics = (note = (String) -> Nil)
Yes = (); No = (); NotKnown = (); Truth = Yes | No | NotKnown
Types = (type_of = (Syntax) <- (Fail(SyntaxError)) -> Syntax,
         signature = () -> Syntax,
         subtype = (Syntax,Syntax) <- (Fail(SyntaxError)) -> Truth,
         members = (Syntax) <- (Fail(SyntaxError)) -> List(Syntax))
node : (Syntax) -> Node
make : (Node,Syntax) <- (Fail(SyntaxError)) -> Syntax
same_code : (Syntax,Syntax) -> Bool
show : (Syntax) -> String
show_as : (Syntax,Position) <- (Fail(SyntaxError)) -> String
definition : (Syntax) <- (Definitions,Fail(SyntaxError)) -> Syntax
type_of : (Syntax) <- (Types,Fail(SyntaxError)) -> Syntax
signature : () <- (Types) -> Syntax
subtype : (Syntax,Syntax) <- (Types,Fail(SyntaxError)) -> Truth
members : (Syntax) <- (Types,Fail(SyntaxError)) -> List(Syntax)
```

### 3.3 Core data — lane D

Types with hidden constructors below retain their algorithms and current cost
bounds. Dict/Set traversal order is unspecified. Sequence uses a measured tree.
`same_entries`/`same_elements` are explicit content predicates, not overloads of
`==` or a second hash law. Structural equality/hash may observe representation.
Use a canonical application projection, normally a list with a domain comparator,
when content-based keys are wanted. These three containers stay experimental.

#### std/option

Ordinary absence and its transformations. Reference: E `Option.ts`; G `option.gleam`.

Keep all items. Merge lazy_unwrap into unwrap(make = …), lazy_or into or(make = …); eager shapes use default = … and other = …. Keep both Option↔Result conversions here only.

```text
Some(A) = (A); None = (); Option(A) = Some(A) | None
is_some : <A>(Option(A)) -> Bool
is_none : <A>(Option(A)) -> Bool
map : <A, B, N>(Option(A), (A) <- N -> B) <- N -> Option(B)
then : <A, B, N>(Option(A), (A) <- N -> Option(B)) <- N -> Option(B)
flatten : <A>(Option(Option(A))) -> Option(A)
unwrap : <A>(Option(A), default = A) -> A
unwrap : <A, N>(Option(A), make = () <- N -> A) <- N -> A
or : <A>(Option(A), other = Option(A)) -> Option(A)
or : <A, N>(Option(A), make = () <- N -> Option(A)) <- N -> Option(A)
to_result : <A, E>(Option(A), E) -> Result(A, E)
from_result : <A, E>(Result(A, E)) -> Option(A)
all : <A>(List(Option(A))) -> Option(List(A))
values : <A>(List(Option(A))) -> List(A)
filter : <A, N>(Option(A), (A) <- N -> Bool) <- N -> Option(A)
map2 : <A, B, C, N>( Option(A), Option(B), (A, B) <- N -> C ) <- N -> Option(C)

```

#### std/result

Stored outcomes after catch. Reference: E `Result.ts`; G `result.gleam`.

Keep data functions. Delete catch/to_error (use i/fail), from_option/to_option (use option conversions). Merge lazy_unwrap/lazy_or into labelled shapes; no active failure bridge here.

```text
Result(A,E), Ok(A), Error(E) are prelude types; no new type.
is_ok : <A, E>(Result(A, E)) -> Bool
is_error : <A, E>(Result(A, E)) -> Bool
map : <A, E, B, N>(Result(A, E), (A) <- N -> B) <- N -> Result(B, E)
map_error : <A, E, F, N>(Result(A, E), (E) <- N -> F) <- N -> Result( A, F, )
then : <A, E, B, N>( Result(A, E), (A) <- N -> Result(B, E) ) <- N -> Result(B, E)
flatten : <A, E>(Result(Result(A, E), E)) -> Result(A, E)
unwrap : <A, E>(Result(A, E), default = A) -> A
unwrap : <A, E, N>(Result(A, E), make = () <- N -> A) <- N -> A
unwrap_error : <A, E>(Result(A, E), E) -> E
unwrap_both : <A>(Result(A, A)) -> A
or : <A,E>(Result(A,E), other = Result(A,E)) -> Result(A,E)
or : <A, E, N>( Result(A, E), make = () <- N -> Result(A, E) ) <- N -> Result(A, E)
all : <A, E>(List(Result(A, E))) -> Result(List(A), E)
partition : <A, E>(List(Result(A, E))) -> (List(A), List(E))
values : <A, E>(List(Result(A, E))) -> List(A)
replace : <A, E, B>(Result(A, E), B) -> Result(B, E)
replace_error : <A, E, F>(Result(A, E), F) -> Result(A, F)
recover : <A, E, F, N>( Result(A, E), (E) <- N -> Result(A, F) ) <- N -> Result(A, F)

```

#### std/list

Eager linked sequences, traversal and search. Reference: E `Array.ts`; G `list.gleam`.

Keep all listed algorithms. wrap→singleton, sized_chunk→chunks, strict_zip→zip_exact. Delete try_map/try_fold/try_each, new and prepend syntax facades. Delete group forwarding alias; dict.group is canonical. zip_exact raises ListError.

```text
Continue(A) = (A); Stop(A) = (A)
ContinueOrStop(A) = Continue(A) | Stop(A)
ListError = UnequalLengths
UnequalLengths = (left = Int, right = Int)
length : <A>(List(A)) -> Int
count : <A, N>(List(A), (A) <- N -> Bool) <- N -> Int
reverse : <A>(List(A)) -> List(A)
is_empty : <A>(List(A)) -> Bool
contains : <A>(List(A), A) -> Bool
first : <A>(List(A)) -> Option(A)
rest : <A>(List(A)) -> Option(List(A))
last : <A>(List(A)) -> Option(A)
at : <A>(List(A), Int) -> Option(A)
index_of : <A>(List(A), A) -> Option(Int)
filter : <A, N>(List(A), (A) <- N -> Bool) <- N -> List(A)
filter_map : <A, B, N>(List(A), (A) <- N -> Option(B)) <- N -> List(B)
map : <A, B, N>(List(A), (A) <- N -> B) <- N -> List(B)
map2 : <A, B, C, N>( List(A), List(B), (A, B) <- N -> C ) <- N -> List(C)
map_fold : <A, Acc, B, N>( List(A), Acc, (Acc, A) <- N -> (Acc, B) ) <- N -> (Acc, List(B))
index_map : <A, B, N>(List(A), (A, Int) <- N -> B) <- N -> List(B)
drop : <A>(List(A), Int) -> List(A)
take : <A>(List(A), Int) -> List(A)
singleton : <A>(A) -> List(A)
append : <A>(List(A), List(A)) -> List(A)
concat : <A>(List(List(A))) -> List(A)
flat_map : <A, B, N>(List(A), (A) <- N -> List(B)) <- N -> List(B)
fold : <A, Acc, N>( List(A), Acc, (Acc, A) <- N -> Acc ) <- N -> Acc
fold_right : <A, Acc, N>( List(A), Acc, (Acc, A) <- N -> Acc ) <- N -> Acc
index_fold : <A, Acc, N>( List(A), Acc, (Acc, A, Int) <- N -> Acc ) <- N -> Acc
fold_until : <A, Acc, N>( List(A), Acc, (Acc, A) <- N -> ContinueOrStop(Acc) ) <- N -> Acc
find : <A, N>(List(A), (A) <- N -> Bool) <- N -> Option(A)
find_map : <A, B, N>(List(A), (A) <- N -> Option(B)) <- N -> Option(B)
all : <A, N>(List(A), (A) <- N -> Bool) <- N -> Bool
any : <A, N>(List(A), (A) <- N -> Bool) <- N -> Bool
zip : <A, B>(List(A), List(B)) -> List((A, B))
zip_exact : <A, B>(List(A), List(B)) <- (Fail(ListError)) -> List((A, B))
unzip : <A, B>(List((A, B))) -> (List(A), List(B))
intersperse : <A>(List(A), A) -> List(A)
unique : <A>(List(A)) -> List(A)
sort : <A, N>(List(A), (A, A) <- N -> Order) <- N -> List(A)
range : (Int, Int) -> List(Int)
repeat : <A>(A, Int) -> List(A)
split : <A>(List(A), Int) -> (List(A), List(A))
split_while : <A, N>(List(A), (A) <- N -> Bool) <- N -> ( List(A), List(A), )
key_find : <K, V>(List((K, V)), K) -> Option(V)
key_filter : <K, V>(List((K, V)), K) -> List(V)
key_pop : <K, V>(List((K, V)), K) -> Option((V, List((K, V))))
key_set : <K, V>(List((K, V)), K, V) -> List((K, V))
each : <A, B, N>(List(A), (A) <- N -> B) <- N -> Nil
partition : <A, N>(List(A), (A) <- N -> Bool) <- N -> ( List(A), List(A), )
permutations : <A>(List(A)) -> List(List(A))
window : <A>(List(A), Int) -> List(List(A))
window_by_2 : <A>(List(A)) -> List((A, A))
drop_while : <A, N>(List(A), (A) <- N -> Bool) <- N -> List(A)
take_while : <A, N>(List(A), (A) <- N -> Bool) <- N -> List(A)
chunk : <A, K, N>(List(A), (A) <- N -> K) <- N -> List(List(A))
chunks : <A>(List(A), Int) -> List(List(A))
reduce : <A, N>(List(A), (A, A) <- N -> A) <- N -> Option(A)
scan : <A, Acc, N>( List(A), Acc, (Acc, A) <- N -> Acc ) <- N -> List(Acc)
combinations : <A>(List(A), Int) -> List(List(A))
combination_pairs : <A>(List(A)) -> List((A, A))
interleave : <A>(List(List(A))) -> List(A)
transpose : <A>(List(List(A))) -> List(List(A))
max : <A, N>(List(A), (A, A) <- N -> Order) <- N -> Option(A)
min : <A, N>(List(A), (A, A) <- N -> Order) <- N -> Option(A)

```

#### std/dict

Persistent hash-trie dictionaries. Reference: E `HashMap.ts`, `Record.ts`; G `dict.gleam`; Go `maps/maps.go`.

Keep all algorithms; has→contains. Add same_entries, using lookup rather than representation equality. group stays here.

```text
Dict(K,V) = private
new : <K, V>() -> Dict(K, V)
size : <K, V>(Dict(K, V)) -> Int
is_empty : <K, V>(Dict(K, V)) -> Bool
get : <K, V>(Dict(K, V), K) -> Option(V)
contains : <K, V>(Dict(K, V), K) -> Bool
insert : <K, V>(Dict(K, V), K, V) -> Dict(K, V)
delete : <K, V>(Dict(K, V), K) -> Dict(K, V)
upsert : <K, V, N>( Dict(K, V), K, (Option(V)) <- N -> V ) <- N -> Dict(K, V)
update : <K, V, N>(Dict(K, V), K, (V) <- N -> V) <- N -> Dict(K, V)
from_list : <K, V>(List((K, V))) -> Dict(K, V)
to_list : <K, V>(Dict(K, V)) -> List((K, V))
keys : <K, V>(Dict(K, V)) -> List(K)
values : <K, V>(Dict(K, V)) -> List(V)
fold : <K, V, Acc, N>( Dict(K, V), Acc, (Acc, K, V) <- N -> Acc ) <- N -> Acc
each : <K, V, A, N>(Dict(K, V), (K, V) <- N -> A) <- N -> Nil
map_values : <K, V, W, N>(Dict(K, V), (K, V) <- N -> W) <- N -> Dict( K, W, )
filter : <K, V, N>(Dict(K, V), (K, V) <- N -> Bool) <- N -> Dict(K, V)
take : <K, V>(Dict(K, V), List(K)) -> Dict(K, V)
drop : <K, V>(Dict(K, V), List(K)) -> Dict(K, V)
merge : <K, V>(Dict(K, V), Dict(K, V)) -> Dict(K, V)
combine : <K, V, N>( Dict(K, V), Dict(K, V), (V, V) <- N -> V ) <- N -> Dict(K, V)
group : <V, K, N>(List(V), (V) <- N -> K) <- N -> Dict(K, List(V))
same_entries : <K,V>(Dict(K,V),Dict(K,V)) -> Bool
```

#### std/set

Persistent membership and set algebra. Reference: E `HashSet.ts`; G `set.gleam`.

Keep all items; add same_elements. Storage stays Dict(A,Nil); no duplicate hashing engine.

```text
Set(A) = private
new : <A>() -> Set(A)
size : <A>(Set(A)) -> Int
is_empty : <A>(Set(A)) -> Bool
insert : <A>(Set(A), A) -> Set(A)
delete : <A>(Set(A), A) -> Set(A)
contains : <A>(Set(A), A) -> Bool
to_list : <A>(Set(A)) -> List(A)
from_list : <A>(List(A)) -> Set(A)
fold : <A, Acc, N>(Set(A), Acc, (Acc, A) <- N -> Acc) <- N -> Acc
each : <A, B, N>(Set(A), (A) <- N -> B) <- N -> Nil
map : <A, B, N>(Set(A), (A) <- N -> B) <- N -> Set(B)
filter : <A, N>(Set(A), (A) <- N -> Bool) <- N -> Set(A)
take : <A>(Set(A), List(A)) -> Set(A)
drop : <A>(Set(A), List(A)) -> Set(A)
union : <A>(Set(A), Set(A)) -> Set(A)
intersection : <A>(Set(A), Set(A)) -> Set(A)
difference : <A>(Set(A), Set(A)) -> Set(A)
symmetric_difference : <A>(Set(A), Set(A)) -> Set(A)
is_subset : <A>(Set(A), Set(A)) -> Bool
is_disjoint : <A>(Set(A), Set(A)) -> Bool
same_elements : <A>(Set(A),Set(A)) -> Bool
```

#### std/sequence

Persistent indexed deque and concatenation. Reference: E `Chunk.ts`; Go `slices/slices.go`.

Keep measured-tree algorithms. wrap→singleton; equal→same_elements to state the observation. No new Chunk or mutable array module.

```text
Sequence(A) = private
new : <A>() -> Sequence(A)
singleton : <A>(A) -> Sequence(A)
from_list : <A>(List(A)) -> Sequence(A)
to_list : <A>(Sequence(A)) -> List(A)
length : <A>(Sequence(A)) -> Int
is_empty : <A>(Sequence(A)) -> Bool
first : <A>(Sequence(A)) -> Option(A)
last : <A>(Sequence(A)) -> Option(A)
push_front : <A>(Sequence(A), A) -> Sequence(A)
push_back : <A>(Sequence(A), A) -> Sequence(A)
pop_front : <A>(Sequence(A)) -> Option((A, Sequence(A)))
pop_back : <A>(Sequence(A)) -> Option((A, Sequence(A)))
at : <A>(Sequence(A), Int) -> Option(A)
set : <A>(Sequence(A), Int, A) -> Option(Sequence(A))
append : <A>(Sequence(A), Sequence(A)) -> Sequence(A)
split : <A>(Sequence(A), Int) -> (Sequence(A), Sequence(A))
take : <A>(Sequence(A), Int) -> Sequence(A)
drop : <A>(Sequence(A), Int) -> Sequence(A)
reverse : <A>(Sequence(A)) -> Sequence(A)
map : <A, B, N>(Sequence(A), (A) <- N -> B) <- N -> Sequence(B)
fold : <A, Acc, N>( Sequence(A), Acc, (Acc, A) <- N -> Acc ) <- N -> Acc
each : <A, B, N>(Sequence(A), (A) <- N -> B) <- N -> Nil
same_elements : <A>(Sequence(A), Sequence(A)) -> Bool

```

#### std/order

Ordering results and comparator composition. Reference: E `Order.ts`; G `order.gleam`.

Keep all functions; add by/combine. Merge lazy_break_tie into break_tie(make = …), eager shape next = …. Deferred comparator construction does not call its callbacks.

```text
Less = (); Equal = (); Greater = (); Order = Less | Equal | Greater
negate : (Order) -> Order
reverse : <A, N>((A, A) <- N -> Order) -> (A, A) <- N -> Order
to_int : (Order) -> Int
compare : (Order, Order) -> Order
max : (Order, Order) -> Order
min : (Order, Order) -> Order
break_tie : (Order, next = Order) -> Order
break_tie : <N>(Order, make = () <- N -> Order) <- N -> Order
is_less : (Order) -> Bool
is_equal : (Order) -> Bool
is_greater : (Order) -> Bool
by : <A,B,N>((A,A) <- N -> Order, (B) <- N -> A) -> (B,B) <- N -> Order
combine : <A,N>((A,A) <- N -> Order, (A,A) <- N -> Order) -> (A,A) <- N -> Order
```

#### std/pair

Higher-order adapters for anonymous pairs. Reference: G `pair.gleam`.

Keep swap/map_first/map_second. Delete new/first/second; tuple syntax and .0/.1 already express them.

```text
No public types; use ordinary (A,B) data.
swap : <A, B>((A, B)) -> (B, A)
map_first : <A, B, C, N>((A, B), (A) <- N -> C) <- N -> (C, B)
map_second : <A, B, C, N>((A, B), (B) <- N -> C) <- N -> (A, C)

```

#### std/bool

Boolean conversion and useful function values. Reference: G `bool.gleam`; E `Boolean.ts`.

Keep not/xor/to_string/to_int/compare; exclusive_nor→xnor. Delete and/or/nand/nor and guard/lazy_guard; native branches and short-circuit operators are clearer.

```text
Bool is a prelude type; no new type.
not : (Bool) -> Bool
xor : (Bool, Bool) -> Bool
xnor : (Bool, Bool) -> Bool
to_string : (Bool) -> String
to_int : (Bool) -> Int
compare : (Bool, Bool) -> Order

```

### 3.4 Numbers, text and bytes — lane E

Operational failures in these modules move from Result to the named Fail.
No parse error uses Nil. Existing total predicates and absence queries remain
total. Shared error shapes:

```text
IntError = (operation = String, input = String, reason = IntProblem)
IntProblem = BadText | BadRadix | BadExponent | ZeroDivisor | NegativeRoot | BadDigit | OutOfRange
// Every IntProblem case has data ().
FloatError = (operation = String, input = String, reason = FloatProblem)
FloatProblem = BadText | Domain | NonFinite | ZeroDivisor
// Every FloatProblem case has data ().
StringError = InvalidScalar
InvalidScalar = (value = Int)
BitArrayError = (operation = String, byte_offset = Int, reason = BitProblem)
BitProblem = Bounds | InvalidUtf8 | BadEncoding | Unaligned
// Every BitProblem case has data ().
```

IntError and FloatError are owned in their respective modules. Repeated case
spellings above are qualified identities, not a global shared error enum.

#### std/int

Exact integers, checked arithmetic, radices and useful number algorithms. Reference: E `BigInt.ts`, `Number.ts`; G `int.gleam`; Go `strconv/atoi.go`.

Keep all listed algorithms; checked Result operations become Fail(IntError). Delete add/subtract/multiply operator facades. Replace parse_base by the two-argument parse shape. Keep pure decimal to_string and checked to_base separate: merging them would give decimal formatting an unnecessary Fail need. Delete the fixed-radix aliases in favor of to_base(value, radix). Rename bitwise_* to bit_*/shift_* to agree with the primitive vocabulary. Add gcd/lcm.

```text
to_string : (Int) -> String
parse : (String) <- (Fail(IntError)) -> Int
parse : (String, Int) <- (Fail(IntError)) -> Int
to_base : (Int, Int) <- (Fail(IntError)) -> String
to_float : (Int) <- (Fail(IntError)) -> Float
abs : (Int) -> Int
negate : (Int) -> Int
min : (Int, Int) -> Int
max : (Int, Int) -> Int
clamp : (Int, Int, Int) -> Int
compare : (Int, Int) -> Order
is_even : (Int) -> Bool
is_odd : (Int) -> Bool
power : (Int, Int) <- (Fail(IntError)) -> Int
square_root : (Int) <- (Fail(IntError)) -> Float
sum : (List(Int)) -> Int
product : (List(Int)) -> Int
digits : (Int, Int) <- (Fail(IntError)) -> List(Int)
undigits : (List(Int), Int) <- (Fail(IntError)) -> Int
divide : (Int, Int) <- (Fail(IntError)) -> Int
remainder : (Int, Int) <- (Fail(IntError)) -> Int
modulo : (Int, Int) <- (Fail(IntError)) -> Int
floor_divide : (Int, Int) <- (Fail(IntError)) -> Int
bit_and : (Int, Int) -> Int
bit_or : (Int, Int) -> Int
bit_xor : (Int, Int) -> Int
bit_not : (Int) -> Int
shift_left : (Int, Int) -> Int
shift_right : (Int, Int) -> Int
range : <Acc, N>( Int, Int, Acc, (Acc, Int) <- N -> Acc ) <- N -> Acc
gcd, lcm : (Int,Int) -> Int
```

| Today's item | Final spelling |
|---|---|
| bitwise_and / bitwise_or / bitwise_xor / bitwise_not | bit_and / bit_or / bit_xor / bit_not |
| bitwise_shift_left / bitwise_shift_right | shift_left / shift_right |
| to_base2 / to_base8 / to_base16 / to_base36 | to_base(value, 2 / 8 / 16 / 36), checked radix API |


#### std/float

Finite binary64 parsing, checked math and formatting. Reference: E `Number.ts`; G `float.gleam`; Go `math/`, `strconv/ftoa.go`.

Keep all listed algorithms and constants. Result operations become Fail(FloatError). Delete add/subtract/multiply facades. Preserve finite-only semantics and exact shortest formatting; no NaN/infinity. Approximate transcendental accuracy remains documented, not silently upgraded to correctly rounded.

```text
parse : (String) <- (Fail(FloatError)) -> Float
to_string : (Float) -> String
compare : (Float, Float) -> Order
loosely_compare : (Float, Float, Float) -> Order
loosely_equals : (Float, Float, Float) -> Bool
min : (Float, Float) -> Float
max : (Float, Float) -> Float
clamp : (Float, Float, Float) -> Float
abs : (Float) -> Float
negate : (Float) -> Float
ceiling : (Float) -> Float
floor : (Float) -> Float
round : (Float) -> Int
truncate : (Float) -> Int
to_precision : (Float, Int) -> Float
power : (Float, Int) <- (Fail(FloatError)) -> Float
real_power : (Float, Float) <- (Fail(FloatError)) -> Float
exponential : (Float) <- (Fail(FloatError)) -> Float
logarithm : (Float) <- (Fail(FloatError)) -> Float
square_root : (Float) <- (Fail(FloatError)) -> Float
sum : (List(Float)) -> Float
product : (List(Float)) -> Float
divide : (Float, Float) <- (Fail(FloatError)) -> Float
modulo : (Float, Float) <- (Fail(FloatError)) -> Float
is_integer : (Float) -> Bool
max_finite : Float
min_positive : Float
pi : Float
tau : Float
sin : (Float) -> Float
cos : (Float) -> Float
tan : (Float) <- (Fail(FloatError)) -> Float
sign_bit : (Float) -> Bool
copy_sign : (Float, Float) -> Float
asin : (Float) <- (Fail(FloatError)) -> Float
acos : (Float) <- (Fail(FloatError)) -> Float
atan : (Float) -> Float
atan2 : (Float, Float) -> Float
hypot : (Float, Float) <- (Fail(FloatError)) -> Float
radians : (Float) -> Float
degrees : (Float) <- (Fail(FloatError)) -> Float
```

#### std/string

Unicode-scalar text. Reference: E `String.ts`; G `string.gleam`; Go `strings/strings.go`.

Keep all listed functions. Delete generic inspect (i/core.inspect) and to_option (empty-text policy belongs to the caller). utf_codepoint raises StringError. Scalar indexes and slicing remain unchanged. Share Unicode tables with regex, not two independent copies.

```text
length : (String) -> Int
byte_size : (String) -> Int
is_empty : (String) -> Bool
concat : (List(String)) -> String
append : (String, String) -> String
join : (List(String), String) -> String
repeat : (String, Int) -> String
split : (String, String) -> List(String)
split_once : (String, String) -> Option((String, String))
lines : (String) -> List(String)
trim : (String) -> String
trim_start : (String) -> String
trim_end : (String) -> String
starts_with : (String, String) -> Bool
ends_with : (String, String) -> Bool
contains : (String, String) -> Bool
replace : (String, String, String) -> String
slice : (String, Int, Int) -> String
drop_start : (String, Int) -> String
drop_end : (String, Int) -> String
first : (String) -> Option(String)
last : (String) -> Option(String)
pop_codepoint : (String) -> Option((String, String))
crop : (String, String) -> String
remove_prefix : (String, String) -> String
remove_suffix : (String, String) -> String
to_upper : (String) -> String
to_lower : (String) -> String
capitalise : (String) -> String
reverse : (String) -> String
pad_left : (String, Int, String) -> String
pad_right : (String, Int, String) -> String
to_codepoints : (String) -> List(UtfCodepoint)
from_codepoints : (List(UtfCodepoint)) -> String
to_codepoint_strings : (String) -> List(String)
utf_codepoint : (Int) <- (Fail(StringError)) -> UtfCodepoint
utf_codepoint_to_int : (UtfCodepoint) -> Int
compare : (String, String) -> Order
at : (String, Int) -> Option(String)
index_of : (String, String, Int) -> Option(Int)
last_index_of : (String, String, Int) -> Option(Int)
count : (String, String) -> Int
take_start : (String, Int) -> String
take_end : (String, Int) -> String
split_once_last : (String, String) -> Option((String, String))
replace_n : (String, String, String, Int) -> String
split_when : <E, N>( String, (UtfCodepoint) <- (Fail(E), ..N) -> Bool ) <- (Fail(E), ..N) -> List(String)
words : (String) -> List(String)
map : <N>(String, (UtfCodepoint) <- N -> UtfCodepoint) <- N -> String
filter : <N>(String, (UtfCodepoint) <- N -> Bool) <- N -> String
fold : <A, N>(String, A, (A, UtfCodepoint) <- N -> A) <- N -> A
```

#### std/bit_array

Arbitrary-bit operations and existing text/base codecs. Reference: G `bit_array.gleam`; Go `encoding/base64/base64.go`, `encoding/hex/hex.go`.

Keep all listed items except inspect. Move Result validation to Fail(BitArrayError). Keep bit units for slice and explicit zero-padding behavior of existing codecs. Whole-byte IO uses std/bytes. Padding-Boolean encoders gain a one-argument default shape; explicit Bool shape remains, with documented meaning.

```text
from_string : (String) -> BitArray
to_string : (BitArray) <- (Fail(BitArrayError)) -> String
is_utf8 : (BitArray) -> Bool
bit_size : (BitArray) -> Int
byte_size : (BitArray) -> Int
append : (BitArray, BitArray) -> BitArray
concat : (List(BitArray)) -> BitArray
slice : (BitArray, Int, Int) <- (Fail(BitArrayError)) -> BitArray
starts_with : (BitArray, BitArray) -> Bool
pad_to_bytes : (BitArray) -> BitArray
compare : (BitArray, BitArray) -> Order
base16_encode : (BitArray) -> String
base16_decode : (String) <- (Fail(BitArrayError)) -> BitArray
base64_encode : (BitArray, Bool) -> String
base64_decode : (String) <- (Fail(BitArrayError)) -> BitArray
base64_url_encode : (BitArray, Bool) -> String
base64_url_decode : (String) <- (Fail(BitArrayError)) -> BitArray
base64_encode, base64_url_encode : (BitArray) -> String
```

#### std/crypto

SHA-2, HMAC, KDF and secure entropy. Reference: Go `crypto/sha256/`, `crypto/sha512/`, `crypto/subtle/constant_time.go`; E platform capability comparison.

Keep all names and algorithms. Fail(CryptoError) replaces Result. Input comes first in HMAC/extract; expand keeps derived key first as its input, then info/length. Host entropy ProcessError remains composed. constant_time_equal promises only the documented source-level property until compiled timing is separately established.

```text
CryptoError = InvalidLength | InvalidIterations | UnalignedInput
InvalidLength = (); InvalidIterations = (); UnalignedInput = ()
sha256 : (BitArray) -> BitArray
sha512 : (BitArray) -> BitArray
hmac_sha256 : (BitArray, BitArray) <- (Fail(CryptoError)) -> BitArray  // input, key/salt
hmac_sha512 : (BitArray, BitArray) <- (Fail(CryptoError)) -> BitArray  // input, key/salt
hkdf_sha256 : ( BitArray, BitArray, BitArray, Int ) <- (Fail(CryptoError)) -> BitArray
hkdf_sha512 : ( BitArray, BitArray, BitArray, Int ) <- (Fail(CryptoError)) -> BitArray
hkdf_extract_sha256 : (BitArray, BitArray) <- (Fail(CryptoError)) -> BitArray  // input, key/salt
hkdf_extract_sha512 : (BitArray, BitArray) <- (Fail(CryptoError)) -> BitArray  // input, key/salt
hkdf_expand_sha256 : (BitArray, BitArray, Int) <- (Fail(CryptoError)) -> BitArray
hkdf_expand_sha512 : (BitArray, BitArray, Int) <- (Fail(CryptoError)) -> BitArray
pbkdf2_sha256 : ( BitArray, BitArray, Int, Int ) <- (Fail(CryptoError)) -> BitArray
constant_time_equal : (BitArray, BitArray) -> Bool
random_bytes : (Int) <- (iprocess.Entropy, Fail(iprocess.ProcessError), request.Await, request.Owner) <- (Fail(CryptoError)) -> BitArray
```

#### std/bytes — new

Whole-byte algorithms, assembly and bounded binary cursors. Reference:
G `bytes_tree.gleam`; Go `bytes/bytes.go`, `encoding/binary/varint.go`.
BitArray stays the language type. Do not add a compiler-known Bytes primitive.
`check` rejects unaligned bits; padding is explicit in bit_array.pad_to_bytes.
Keep codecs in bit_array, UTF-8 in unicode; do not repeat their implementation.

```text
BytesError = (byte_offset = Int, reason = BytesProblem)
BytesProblem = Unaligned | Bounds | BadVarint
Unaligned = (); Bounds = (); BadVarint = ()
Builder = private
Cursor = private
check : (BitArray) <- (Fail(BytesError)) -> BitArray
size : (BitArray) <- (Fail(BytesError)) -> Int
slice : (BitArray,Int,Int) <- (Fail(BytesError)) -> BitArray
at : (BitArray,Int) <- (Fail(BytesError)) -> Option(Int)
find : (BitArray,BitArray) <- (Fail(BytesError)) -> Option(Int)
split : (BitArray,BitArray) <- (Fail(BytesError)) -> List(BitArray)
replace : (BitArray,BitArray,BitArray) <- (Fail(BytesError)) -> BitArray
new : () -> Builder
append : (Builder,BitArray) <- (Fail(BytesError)) -> Builder
build : (Builder) -> BitArray
cursor : (BitArray) <- (Fail(BytesError)) -> Cursor
remaining : (Cursor) -> Int
read : (Cursor,Int) <- (Fail(BytesError)) -> (BitArray,Cursor)
read_varint : (Cursor) <- (Fail(BytesError)) -> (Int,Cursor)
write_varint : (Int) <- (Fail(BytesError)) -> BitArray
```

Byte algorithms reject unaligned input even when the answer would be empty.
Bounds are nonnegative, checked, and measured in bytes. A cursor is immutable:
reusing it repeats the same read. Varint is unsigned base-128, at most ten bytes
for the interoperable 64-bit wire form; negative/overlong/non-minimal values fail.
Builder assembly concatenates chunks once; repeated append does not flatten.

#### std/unicode — new

One Unicode table and incremental UTF-8 decoder. Reference: Go
`unicode/letter.go`, `unicode/utf8/utf8.go`; G `string.gleam`. Move regex's
Unicode 16 property/fold data here; E's case-insensitive behavior is unaffected.
`version` is part of the release manifest. Casing stays simple scalar casing.
No grapheme or normalization promise is hidden under string.length.

```text
UnicodeError = (byte_offset = Int, reason = String)
Decoder = private
Decoded = (text = String, next = Decoder)
version : String
is_letter, is_digit, is_space, is_upper, is_lower : (UtfCodepoint) -> Bool
simple_fold : (UtfCodepoint) -> UtfCodepoint
property : (UtfCodepoint,String) -> Bool
decoder : () -> Decoder
feed : (Decoder,BitArray) <- (Fail(UnicodeError)) -> Decoded
finish : (Decoder) <- (Fail(UnicodeError)) -> String
decode : (BitArray) <- (Fail(UnicodeError)) -> String
encode : (String) -> BitArray
```

`feed` emits only complete scalars and retains at most three trailing bytes.
`finish` rejects an incomplete scalar. Unknown property names return False;
regex compile validates names separately. Error offsets count bytes from the
start of this decoder, including earlier chunks. No replacement-character
policy silently masks malformed input.

#### std/decimal — new, experimental

Exact base-ten arithmetic for application data. Reference: E `BigDecimal.ts`.
Normalize coefficient/scale together so ordinary `==` and hash agree with
numeric decimal equality. Zero has coefficient and scale zero. Int remains the
only integer type. Division always names precision and rounding.

```text
Decimal = private
DecimalError = (operation = String, reason = DecimalProblem)
DecimalProblem = BadText | ZeroDivisor | BadPrecision
BadText = (); ZeroDivisor = (); BadPrecision = ()
Rounding = HalfEven | TowardZero | Floor | Ceiling
HalfEven = (); TowardZero = (); Floor = (); Ceiling = ()
from_parts : (Int,Int) -> Decimal
parts : (Decimal) -> (coefficient = Int, scale = Int)
parse : (String) <- (Fail(DecimalError)) -> Decimal
to_string : (Decimal) -> String
compare : (Decimal,Decimal) -> Order
add, subtract, multiply : (Decimal,Decimal) -> Decimal
divide : (Decimal,Decimal,Int,Rounding) <- (Fail(DecimalError)) -> Decimal
round : (Decimal,Int,Rounding) <- (Fail(DecimalError)) -> Decimal
```

Precision means fractional decimal places, and must be nonnegative. Negative
scale is valid for exact integers with trailing zeros. No implicit Float or
currency conversion is added. Resource bounds belong to the caller when it
accepts arbitrary exponents from untrusted input.

### 3.5 Structured tasks and coordination — lane B

#### i/task

Structured ownership, channels/events and the sole state authority. Reference:
E `Fiber.ts`, `Effect.ts`, `Ref.ts`; spec §9.9 is authoritative for behavior.
Keep all existing public operations. Scope(E)→Group(E); hide DetachedRunner
and DetachedScheduler implementation sentinels as the spec requires. Keep
Event(A,E,N): E is used by sync's ordinary Fail need only. Add a bounded
map shape, Ref get/set, and revision notifications plus the test-driver quiescence operation. `get`'s
Shared and Ref cases have the same scheduler need and one result family.
No separate std/ref implementation is created.

```text
Scheduler, Runner, Group(E), Task(A,E), Channel(A), Event(A,E,N), Shared(A), Ref(A)
  = private nominal types
ScheduleError = NotRunnable | InvalidLimit
NotRunnable = (path = List(Int)); InvalidLimit = (limit = Int)
top : (random.Generator) <- (Runner) -> Runner
run : <A,N>(() <- (..N,Scheduler,Runner,request.Await,request.Owner,request.Root) -> A)
  <- (..N,Runner) -> A
test : <A,E,N>(random.Generator,List(List(Int)),
  () <- (..N,Fail(E),Scheduler,Runner,request.Await,request.Owner,request.Root) -> A)
  <- (..N,Fail(E | ScheduleError),Runner) -> A
scope, local_scope : <A,E,N>((Group(E)) <- (..N,Fail(E)) -> A)
  <- (..N,Fail(E),Scheduler) -> A
go : <A,E,N>(Group(E),() <- (..N,Fail(E),Scheduler,Runner,request.Await) -> A)
  <- (..N,Scheduler) -> Task(A,E)
join : <A,E>(Task(A,E)) <- (Fail(E),Scheduler) -> A
yield : () <- (Scheduler) -> Nil
path : () <- (Scheduler) -> List(Int)
random : () <- (Scheduler) -> random.Generator
keep : <A,B,E,N>((A) <- (..N,Fail(E),Scheduler,Runner,request.Await) -> B)
  <- (..N,Scheduler) -> (A) <- (Fail(E),Scheduler) -> B
channel, local_channel : <A>() <- (Scheduler) -> Channel(A)
send : <A>(Channel(A),A) <- (Scheduler) -> Nil
receive : <A>(Channel(A)) <- (Scheduler) -> A
send_event : <A>(Channel(A),A) -> Event(Nil,Never,Never)
receive_event : <A>(Channel(A)) -> Event(A,Never,Never)
join_event : <A,E>(Task(A,E)) -> Event(A,E,Never)
always : <A>(A) -> Event(A,Never,Never)
never : () -> Event(Never,Never,Never)
choose : <A,E,N>(List(Event(A,E,N))) -> Event(A,E,N)
map : <A,B,E,F,N>(Event(A,E,N),(A) <- (..N,Fail(F)) -> B) -> Event(B,E | F,N)
sync : <A,E,N>(Event(A,E,N)) <- (..N,Fail(E),Scheduler) -> A
share : <A,E>(Group(E),A) <- (Scheduler) -> Shared(A)
get : <A>(Shared(A)) <- (Scheduler) -> A
get : <A>(Ref(A)) <- (Scheduler) -> A
ref : <A>(A) <- (Scheduler) -> Ref(A)
set : <A>(Ref(A),A) <- (Scheduler) -> Nil
update : <A,B>(Ref(A),(A) -> (A,B)) <- (Scheduler) -> B
both : <A,B,E,N>(() <- (..N,Fail(E),Scheduler,Runner,request.Await) -> A,
  () <- (..N,Fail(E),Scheduler,Runner,request.Await) -> B) <- Tasks[N,E] -> (A,B)
race : <A,E,N>(() <- (..N,Fail(E),Scheduler,Runner,request.Await) -> A,
  () <- (..N,Fail(E),Scheduler,Runner,request.Await) -> A) <- Tasks[N,E] -> A
map_concurrently : <A,B,E,N>(List(A),
  (A) <- (..N,Fail(E),Scheduler,Runner,request.Await) -> B)
  <- Tasks[N,E | ScheduleError] -> List(B)
map_concurrently : <A,B,E,N>(List(A),Int,
  (A) <- (..N,Fail(E),Scheduler,Runner,request.Await) -> B)
  <- Tasks[N,E | ScheduleError] -> List(B)
Revision(A) = private
snapshot : <A>(Ref(A)) <- (Scheduler) -> (value = A, revision = Revision(A))
changed : <A>(Revision(A)) -> Event(A,Never,Never)
quiesce : () <- (Scheduler) -> Nil
```

The two-argument map defaults to 32 workers; the explicit limit must be
positive. Both allocate O(limit) live child workers and preserve input order.
`race` keeps spec R5: an observed failure wins over a later success, and a
success waits for loser cleanup. No first-success-despite-failure variant.

`snapshot` atomically reads a Ref and its revision. `changed` is a non-consuming
native event that becomes ready once that revision changes, including a change
before sync registers. Every successful replacement advances the revision,
even if its value compares equal. A losing/cancelled subscription consumes no
state. These ordinary scheduler operations let coordination keep aggregate
immutable state in Ref and sleep on changes without polling or detached owner
tasks. FIFO waiter tickets are stored in that state and removed on cancellation.
This extends the private i/task scheduler protocol, never the compiler/runtime.

`quiesce` parks the calling test driver until other tasks in the run have
finished runnable steps and completion processing and are blocked or complete.
It observes no worker count. A private ordinary scheduler command implements
it; it is not compiler syntax or a host operation. It does not advance clocks
or make an external request complete. An endlessly runnable task can prevent
it returning. It is for deterministic drivers, not a promise of program termination.

#### Coordination family

All new handles belong to their creation Scope and run. Constructors require
resource.Scope and task.Scheduler; operations require task.Scheduler. State is
an immutable value replaced through Ref or a scoped owner loop. The table's
`C` need means `(resource.Scope, task.Scheduler)`, not a new language alias.
All handles require portable stored payloads and enforce run/liveness rules.
Invalid handle use keeps the task stop contract; input capacity errors are Fail.

| Module | Purpose and exact reference | Public types and functions |
|---|---|---|
| `std/deferred` | One completion, many readers. E `Deferred.ts`. | `Deferred(A,E)` private; `new : () <- C -> Deferred(A,E)`; `complete : (Deferred(A,E),Result(A,E)) <- (Scheduler) -> Bool`; `await : (Deferred(A,E)) <- (Scheduler,Fail(E)) -> A`; `poll : (Deferred(A,E)) <- (Scheduler) -> Option(Result(A,E))`; `is_done : (Deferred(A,E)) <- (Scheduler) -> Bool`. |
| `std/semaphore` | FIFO admission with safe release. E `Semaphore.ts`; Go `sync/`. | `Semaphore` private; `SemaphoreError = InvalidPermits`, `InvalidPermits = (Int)`; `new : (Int) <- (C,Fail(SemaphoreError)) -> Semaphore`; `with_permits : (Semaphore,Int,() <- N -> A) <- (..N,Scheduler,Fail(SemaphoreError)) -> A`; `with_permit : (Semaphore,() <- N -> A) <- (..N,Scheduler) -> A`; `available : (Semaphore) <- (Scheduler) -> Int`. |
| `std/queue` | Bounded/unbounded FIFO handoff. E `Queue.ts`. | `Queue(A,E)` private; `QueueError = InvalidCapacity | Closed`, `InvalidCapacity = (Int)`, `Closed = ()`; `bounded, dropping, sliding : (Int) <- (C,Fail(QueueError)) -> Queue(A,E)`; `unbounded : () <- C -> Queue(A,E)`; `offer : (Queue(A,E),A) <- (Scheduler,Fail(QueueError)) -> Bool`; `take : (Queue(A,E)) <- (Scheduler,Fail(E)) -> Option(A)`; `poll : (Queue(A,E)) <- (Scheduler,Fail(E)) -> Option(A)`; `size : (Queue(A,E)) <- (Scheduler) -> Int`; `end : (Queue(A,E)) <- (Scheduler) -> Nil`; `fail : (Queue(A,E),E) <- (Scheduler) -> Nil`; `shutdown : (Queue(A,E)) <- (Scheduler) -> Nil`. |
| `std/pubsub` | Scoped broadcast to current subscribers. E `PubSub.ts`. | `PubSub(A)`, `Subscription(A)` private; `PubSubError = InvalidCapacity | Closed`, cases as above; `bounded, dropping, sliding : (Int) <- (C,Fail(PubSubError)) -> PubSub(A)`; `subscribe : (PubSub(A)) <- (C,Fail(PubSubError)) -> Subscription(A)`; `publish : (PubSub(A),A) <- (Scheduler,Fail(PubSubError)) -> Bool`; `take : (Subscription(A)) <- (Scheduler) -> Option(A)`; `unsubscribe : (Subscription(A)) <- (Scheduler) -> Nil`; `end, shutdown : (PubSub(A)) <- (Scheduler) -> Nil`. |
| `std/latch` | Reusable open/closed gate. E `Latch.ts`. | `Latch` private; `new : (Bool) <- C -> Latch`; `open, close, release, await : (Latch) <- (Scheduler) -> Nil`; `is_open : (Latch) <- (Scheduler) -> Bool`. |
| `std/fiber_set` | Dynamic jobs within one structured group. E `FiberSet.ts`. | `FiberSet(E,N)` private; `scoped : ((FiberSet(E,N)) <- (..N,Fail(E),Scheduler) -> A) <- Tasks[N,E] -> A`; `spawn : (FiberSet(E,N),() <- (..N,Fail(E),Scheduler,Runner,request.Await) -> Nil) <- (..N,Scheduler) -> Nil`; `size : (FiberSet(E,N)) <- (Scheduler) -> Int`; `await_empty : (FiberSet(E,N)) <- (Scheduler,Fail(E)) -> Nil`. |

`Deferred.complete` accepts caught outcome data. First completion returns True;
later ones return False and change nothing. Cancelling one waiter does not
cancel the producer or other waiters. There is no recoverable cancelled outcome.

Queue `end` refuses new offers, drains buffered values, then returns None.
`fail` also drains, then raises E. The first terminal transition wins.
`shutdown` discards pending values, wakes waiters and returns None; resource
scope exit performs shutdown. A dropping offer returns False at capacity;
a sliding offer removes the oldest buffered item and returns True. `poll`
returns None when empty or ended; inspect size only for observation, never
for a check-then-take protocol. The next take/poll after a drained failure raises.

PubSub has no replay: new subscribers see future publications. Bounded mode
waits for every current subscriber's capacity; dropping rejects the publication
for all when any is full; sliding drops the oldest item in each full subscription.
No-subscriber publication succeeds. Subscription scope exit unsubscribes before
its waiting state is freed. Latch.open admits current and future waiters;
close blocks future ones; release admits only those waiting now.

Semaphore capacity is positive. with_permits accepts 0..capacity; zero runs
its body immediately, while negative or oversized requests raise InvalidPermits.
Permit acquisition and release registration are one safe handoff. Cancelling
a waiting operation removes it before its callback can run. No Queue/Semaphore
Event constructors are exposed: native task events supply atomic selection;
a fabricated poll-then-act event would violate the language's commit rules.
FiberSet has no keyed replacement, clear/interrupt, or detached spawn. Failure
cancels its owning group by ordinary rules; scoped return joins every job.
Those restrictions avoid inventing arbitrary per-task interruption.

### 3.6 Time, clocks, random and schedules — lane C

#### i/clock and i/random

Keep the raw Clock adapter and pure splittable generator. Reference:
E `Clock.ts`, `Random.ts`; Go `time/time.go`; spec §§9.10, 9.12.
Existing Clock/ClockError and all three functions remain. RandomError,
Generator and all six functions remain, including exact seed/path reproducibility.
A seeded Generator is already the deterministic test version. No ambient mutable
Random service or duplicate TestRandom execution mechanism is added.

```text
// i/clock: raw Request(Result) fields are host reply data, not public sequencing.
Clock = (monotonic_ns = () -> Request(Result(Int,String)),
         unix_ns = () -> Request(Result(Int,String)),
         sleep_ns = (Int) -> Request(Result(Nil,String)))
ClockError = (operation = String, code = String)
monotonic_ns, unix_ns : () <- IO[Clock,ClockError] -> Int
sleep_ns : (Int) <- IO[Clock,ClockError] -> Nil
// i/random
Generator = private
InvalidBound = (Int); InvalidPath = (List(Int))
RandomError = InvalidBound | InvalidPath
from_seed : (Int) -> Generator
word : (Generator) -> (Int,Generator)
int : (Generator,Int) <- (Fail(RandomError)) -> (Int,Generator)
float : (Generator) -> (Float,Generator)
split : (Generator) -> (Generator,Generator)
at_path : (Generator,List(Int)) <- (Fail(RandomError)) -> Generator
```

#### std/time and std/test_clock

Keep Duration/Instant and all pure time operations. Add duration text and a
high-level ordinary Clock. Reference: E `Duration.ts`, `DateTime.ts`,
`testing/TestClock.ts`; Go `time/time.go`. Both need-polymorphic policy code
and virtual time use this same service. i/clock remains the raw host contract.

```text
// std/time
Duration, Instant = private
DivisionByZero = (); BadText = (String); ClockFailure = (clock.ClockError)
TimeError = DivisionByZero | BadText | ClockFailure
Clock = (now = () <- (Fail(TimeError),Scheduler,request.Await,request.Owner) -> Instant,
         monotonic = () <- (Fail(TimeError),Scheduler,request.Await,request.Owner) -> Duration,
         sleep = (Duration) <- (Fail(TimeError),Scheduler,request.Await,request.Owner) -> Nil)
nanoseconds, milliseconds, seconds, minutes, hours : (Int) -> Duration
to_nanoseconds : (Duration) -> Int
add, subtract : (Duration,Duration) -> Duration
scale : (Duration,Int) -> Duration
divide : (Duration,Int) <- (Fail(TimeError)) -> Duration
compare : (Duration,Duration) -> Order
from_unix_nanoseconds : (Int) -> Instant
unix_nanoseconds : (Instant) -> Int
shift : (Instant,Duration) -> Instant
between : (Instant,Instant) -> Duration
compare_instants : (Instant,Instant) -> Order
parse : (String) <- (Fail(TimeError)) -> Duration
format : (Duration) -> String
native : () <- (clock.Clock) -> Clock
now : () <- (Clock,Fail(TimeError),Scheduler,request.Await,request.Owner) -> Instant
monotonic : () <- (Clock,Fail(TimeError),Scheduler,request.Await,request.Owner) -> Duration
sleep : (Duration) <- (Clock,Fail(TimeError),Scheduler,request.Await,request.Owner) -> Nil
measure : <A,E,N>(() <- (..N,Fail(E)) -> A)
  <- (..N,Clock,Scheduler,request.Await,request.Owner,Fail(E | TimeError)) -> (A,Duration)
// std/test_clock
TestClock = private
TestClockError = NegativeAdvance
NegativeAdvance = (time.Duration)
new : (time.Instant) <- (resource.Scope,Scheduler) -> TestClock
clock : (TestClock) -> time.Clock
advance : (TestClock,time.Duration) <- (Scheduler,Fail(TestClockError)) -> Nil
set_time : (TestClock,time.Instant) <- (Scheduler) -> Nil
pending : (TestClock) <- (Scheduler) -> List(time.Duration)
```

`native()` captures only the raw clock choice; Await/Owner are from each call.
The high-level functions' extra Scheduler need permits the same service to
implement virtual sleeps using ordinary channels and Ref, without constructing
user-completable host Requests. `test_clock.new` owns timer state in Scope.
`advance` quiesces, wakes deadlines in chronological/FIFO order, quiesces again,
and repeats until the target time; new sleeps registered after several hops
are included. Zero advance drains due work. Negative advance raises.
`set_time` changes wall time only. Scope exit removes sleepers by structured
cancellation; no fake host outstanding request keeps a run alive.

Nonpositive sleep returns immediately. Signed exact Duration remains useful
for differences. Duration text accepts signed integer quantities with `ns`,
`us`, `ms`, `s`, `m`, `h`, summing adjacent terms; format emits an exact ns term
for guaranteed round-trip. Named zones and leap seconds are not promised.

#### std/calendar

Keep validated Gregorian Date/TimeOfDay/Offset, Civil and weekday cases.
Reference: Go `time/time.go`; E `DateTime.ts`. All Result validation becomes
Fail(CalendarError). Delete now/format_now composition aliases: use time.now,
calendar.at(value, calendar.utc()), and format. Calendar operations stay pure.

```text
Date, TimeOfDay, Offset = private
Civil = (date = Date, time = TimeOfDay)
Weekday = Monday | Tuesday | Wednesday | Thursday | Friday | Saturday | Sunday
// Each weekday has data ().
CalendarError = BadMonth | BadDay | BadHour | BadMinute | BadSecond |
                BadNanosecond | BadOffset | BadText
// Each CalendarError member has data ().
date : (Int, Int, Int) <- (Fail(CalendarError)) -> Date
time_of_day : (Int, Int, Int, Int) <- (Fail(CalendarError)) -> TimeOfDay
offset : (Int) <- (Fail(CalendarError)) -> Offset
utc : () -> Offset
date_parts : (Date) -> (Int, Int, Int)
time_parts : (TimeOfDay) -> (Int, Int, Int, Int)
offset_minutes : (Offset) -> Int
is_leap_year : (Int) -> Bool
days_in_month : (Int, Int) <- (Fail(CalendarError)) -> Int
day_number : (Date) -> Int
from_day_number : (Int) -> Date
weekday : (Date) -> Weekday
add_days : (Date, Int) -> Date
days_between : (Date, Date) -> Int
at : (time.Instant, Offset) -> Civil
to_instant : (Civil, Offset) -> time.Instant
parse : (String) <- (Fail(CalendarError)) -> (Civil, Offset)
format : (Civil, Offset) -> String
```

#### std/random — new collection algorithms; deterministic testing in place

Reference: E `Random.ts`; reuse i/random's state and errors. These are pure
algorithms with explicit successor generators, not ambient service calls.
`std/test_random` is deliberately not added: use i/random.from_seed/at_path,
task.test's script and test_clock as three separate reproducibility controls.
This is the requested test version without duplicating random state machinery.

```text
shuffle : <A>(List(A),random.Generator) -> (List(A),random.Generator)
choice : <A>(List(A),random.Generator) -> (Option(A),random.Generator)
```

Empty choice returns None without drawing. Shuffle uses an unbiased bounded
integer draw at each Fisher–Yates step; the private indexed workspace is an
immutable Sequence. Secure bytes remain crypto.random_bytes with Entropy.

#### std/concurrent and std/schedule — new

Reference: E `Effect.ts`, `Schedule.ts`, `FiberSet.ts`. `concurrent` owns generic
time limits; `i/task` owns race and bounded traversal. Do not duplicate them as
HTTP-only helpers. No new race semantics are introduced.

```text
// std/concurrent
ConcurrentError = Timeout
Timeout = (after = time.Duration)
timeout : <A,E,N>(() <- (..N,Fail(E),Scheduler,Runner,request.Await) -> A,time.Duration)
  <- (..N,time.Clock,Scheduler,request.Owner,Fail(E | ConcurrentError | time.TimeError)) -> A
// std/schedule
Schedule(I,O,E,N), Decision(I,O,E,N) = private
Metadata = (attempt = Int, elapsed = time.Duration)
ScheduleError = InvalidCount | InvalidDelay | InvalidJitter
InvalidCount = (Int); InvalidDelay = (time.Duration); InvalidJitter = (Float,Float)
Stop(O) = (output = O)
Continue(I,O,E,N) = (output = O, delay = time.Duration, next = Schedule(I,O,E,N))
// Decision's representation is Stop(O) | Continue(I,O,E,N), with constructors hidden.
make : ((I,Metadata) <- (..N,Fail(E)) -> Decision(I,O,E,N)) -> Schedule(I,O,E,N)
stop : (O) -> Decision(I,O,E,N)
next : (O,time.Duration,Schedule(I,O,E,N)) <- (Fail(ScheduleError)) -> Decision(I,O,E,N)
step : (Schedule(I,O,E,N),I,Metadata) <- (..N,Fail(E)) -> Decision(I,O,E,N)
recurs : (Int) <- (Fail(ScheduleError)) -> Schedule(I,Int,Never,N)
spaced : (time.Duration) <- (Fail(ScheduleError)) -> Schedule(I,Int,Never,N)
exponential : (time.Duration,Int) <- (Fail(ScheduleError)) -> Schedule(I,Int,Never,N)
cap : (Schedule(I,O,E,N),time.Duration) <- (Fail(ScheduleError)) -> Schedule(I,O,E,N)
while : (Schedule(I,O,E,N),(I) <- N -> Bool) -> Schedule(I,O,E,N)
map : (Schedule(I,O,E,N),(O) <- N -> B) -> Schedule(I,B,E,N)
both : (Schedule(I,O,E,N),Schedule(I,B,E,N)) -> Schedule(I,(O,B),E,N)
either : (Schedule(I,O,E,N),Schedule(I,O,E,N)) -> Schedule(I,O,E,N)
and_then : (Schedule(I,O,E,N),Schedule(I,O,E,N)) -> Schedule(I,O,E,N)
jitter : (Schedule(I,O,E,N),Float,Float,random.Generator)
  <- (Fail(ScheduleError)) -> Schedule(I,O,E,N)
retry : <A,E,F,N>(() <- (..N,Fail(E)) -> A,Schedule(E,O,F,N))
  <- (..N,time.Clock,Scheduler,request.Await,request.Owner,Fail(E | F | time.TimeError)) -> A
repeat : <A,E,F,N>(() <- (..N,Fail(E)) -> A,Schedule(A,O,F,N))
  <- (..N,time.Clock,Scheduler,request.Await,request.Owner,Fail(E | F | time.TimeError)) -> (A,O)
```

A schedule is immutable stepping policy, not a computation interpreter.
`make` permits a custom effectful step. No callback runs during composition.
The runner calls the body once before consulting the schedule. Metadata.attempt
is zero for that first outcome; `recurs(n)` permits n further calls. Retry
feeds caught errors, returns success immediately, and re-raises the last body
error on Stop. Schedule failure propagates as F. Repeat feeds successful values,
returns the last value and Stop output, and propagates body failure immediately.

Sleep follows each Continue and precedes the next body call. `both` takes the
larger delay and stops when either stops; `either` takes the smaller delay and
continues while either can continue. `and_then` activates the second after the
first stops. Exhaustion creates no extra error type. Jitter multiplies a delay
by a seeded draw in the finite nonnegative range [low,high], using an exact
integer nanosecond result rounded down. Equal bounds are allowed; zero-delay
steps remain scheduler-cooperative. Exponential uses an integer factor ≥1.

### 3.7 Formats, schema and configuration — lane F

#### std/json

Ordered JSON values and strict syntax. Reference: Go `encoding/json/encode.go`,
`encoding/json/stream.go`; E `Schema.ts`. Keep all tagged scalar constructors,
Array/Object, queries and accessors. parse raises JsonError. pretty becomes
print(value, indent); compact print(value) remains. New incremental state is
the same parser used by parse, not a second JSON implementation.

```text
Null = (); Boolean = (Bool); Integer = (Int); Number = (Float); Text = (String)
Array = (List(Json)); Object = (List((String,Json)))
Json = Null | Boolean | Integer | Number | Text | Array | Object
JsonError = (offset = Int, reason = String)
Decoder = private
More = (next = Decoder)
Complete = (value = Json, rest = BitArray)
Progress = More | Complete
parse : (String) <- (Fail(JsonError)) -> Json
print : (Json) -> String
print : (Json,Int) -> String
field : (Json,String) -> Option(Json)
index : (Json,Int) -> Option(Json)
as_bool : (Json) -> Option(Bool)
as_int : (Json) -> Option(Int)
as_float : (Json) -> Option(Float)
as_string : (Json) -> Option(String)
as_array : (Json) -> Option(List(Json))
as_object : (Json) -> Option(List((String,Json)))
decoder : (max_bytes = Int, max_depth = Int) <- (Fail(JsonError)) -> Decoder
feed : (Decoder,BitArray) <- (Fail(JsonError)) -> Progress
finish : (Decoder) <- (Fail(JsonError)) -> Json
```

parse rejects trailing non-whitespace and uses the same Unicode/number rules
as the incremental parser. Object keeps order and duplicate entries; `field`
returns the last occurrence. Schema object decoding rejects duplicates before
field selection. Integer syntax stays exact Int; fractional/exponent syntax
must produce finite Float or fail. Compact output preserves object order and
Integer versus Number intent. Indent at or below zero selects compact output; positive indent selects pretty
output. Both shapes remain pure and share one need. Freeze
these acceptance and wire policies before stability. Decoder byte/depth limits
must be positive. Partial input keeps immutable parser state, never rescans
an ever-growing prefix on every feed.

#### std/schema

One inspectable description supplies codecs, validation, JSON Schema and
pure generators. Reference: E `Schema.ts`, `SchemaAST.ts`,
`SchemaRepresentation.ts`, `JsonSchema.ts`, `Arbitrary.ts`; G
`dynamic/decode.gleam`. Preserve existing structural builder algorithms.
Schema(A,E) becomes Schema(A,E,D,W); D/W are ordinary decode/encode needs,
E is a custom error type used only inside Fail. There is no schema evaluator
for language computations. User code without macros remains first-class.

Keep primitive/literal/object/field/optional_field/list/dict/tuple/union/
tagged_union/lazy/refine names. optional was a nullable alias: delete it.
Keep nullable for JSON null, optional_field for absence. transform was an alias
of map: keep map for total bidirectional functions; give transform actual
fallible, service-using semantics. Delete old ParseError/Syntax/Decode wrappers;
parse composes JsonError, SchemaError and custom E directly. Decode, encode,
parse and print all take their input first.

```text
Schema(A,E,D,W) = private
Field = (String); Index = (Int); Path = Field | Index
Issue = (path = List(Path), expected = String, actual = Option(json.Json), message = String)
SchemaError = (issues = List(Issue))
Valid(A) = (A); Invalid = (List(Issue)); Validation(A) = Valid(A) | Invalid
Description = private
Kind = Scalar | Literal | Array | Object | Union | Reference | Custom
Scalar = (); Literal = (); Array = (); Object = (); Union = (); Reference = (); Custom = ()
int : Schema(Int,Never,Never,Never)
float : Schema(Float,Never,Never,Never)
string : Schema(String,Never,Never,Never)
bool : Schema(Bool,Never,Never,Never)
null : Schema((),Never,Never,Never)
literal_string : <K>(K) -> Schema(K,Never,Never,Never)
literal_int : <K>(K) -> Schema(K,Never,Never,Never)
decode : (json.Json,Schema(A,E,D,W)) <- (..D,Fail(E | SchemaError)) -> A
encode : (A,Schema(A,E,D,W)) <- (..W,Fail(E | SchemaError)) -> json.Json
validate : (json.Json,Schema(A,E,D,W)) <- (..D,Fail(E)) -> Validation(A)
parse : (String,Schema(A,E,D,W)) <- (..D,Fail(E | SchemaError | json.JsonError)) -> A
print : (A,Schema(A,E,D,W)) <- (..W,Fail(E | SchemaError)) -> String
nullable : (Schema(A,E,D,W)) -> Schema(Option(A),E,D,W)
list : (Schema(A,E,D,W)) -> Schema(List(A),E,D,W)
dict : (Schema(A,E,D,W)) -> Schema(dict.Dict(String,A),E,D,W)
tuple : (Schema(A,E,D,W),Schema(B,E,D,W)) -> Schema((A,B),E,D,W)
object : (A) -> Schema(A,Never,Never,Never)
field : (Schema(A,E,D,W),String,Schema(V,E,D,W),(A) -> V,(A,V) -> A) -> Schema(A,E,D,W)
optional_field : (Schema(A,E,D,W),String,Schema(V,E,D,W),
  (A) -> Option(V),(A,Option(V)) -> A) -> Schema(A,E,D,W)
union : (List(Schema(A,E,D,W)),(A) -> Int) -> Schema(A,E,D,W)
tagged_union : (String,List((String,Schema(A,E,D,W))),(A) -> String) -> Schema(A,E,D,W)
map : (Schema(A,E,D,W),(A) -> B,(B) -> A) -> Schema(B,E,D,W)
transform : (Schema(A,E,D,W),(A) <- (..D,Fail(E | SchemaError)) -> B,
  (B) <- (..W,Fail(E | SchemaError)) -> A) -> Schema(B,E,D,W)
refine : (Schema(A,E,D,W),(A) -> Bool,String) -> Schema(A,E,D,W)
lazy : (() -> Schema(A,E,D,W)) -> Schema(A,E,D,W)
lazy : (String,() -> Schema(A,E,D,W)) -> Schema(A,E,D,W)
describe : (Schema(A,E,D,W)) -> Description
kind : (Description) -> Kind
fields : (Description) -> List((name = String, value = Description, optional = Bool))
children : (Description) -> List(Description)
constraints : (Description) -> List((String,json.Json))
annotations : (Description) -> List((String,json.Json))
reference : (Description) -> Option(String)
annotate : (Schema(A,E,D,W),String,json.Json) -> Schema(A,E,D,W)
to_json_schema : (Schema(A,E,D,W)) <- (Fail(SchemaError)) -> json.Json
generator : (Schema(A,Never,Never,Never)) <- (Fail(SchemaError)) -> property.Gen(A)
```

Literal builders retain the current generic literal-preserving contract.
Their checked implementation constrains K to String or Int respectively;
canonical interface extraction must preserve that constraint. No bounded-type
parameter syntax or special schema inference rule is introduced.

Decode reports the first structural error; validate accumulates independent
structural issues in field/input order. Custom E propagates, since it is not
a validation issue by fiat. Encode checks the same refinements and field
contracts; an invalid value must not bypass validation through serialization.
Missing, explicit null and unknown fields have distinct rules. Object defaults
are used only as construction seeds; required fields remain required.
Unknown fields are rejected by default. Field annotations may explicitly
permit them in the description; this changes neither parser nor JSON data.

The named lazy shape supplies a stable recursive reference name. An unnamed
recursive callback remains usable for codecs; external derivation fails at that
path if no finite reference description can be obtained. Description nodes
record encoded structure, recursive references, field names,
constraints and annotations. Typed functions remain alongside this data.
Opaque custom predicates/transforms are marked Custom: JSON Schema/generator
derivation fails with an Issue naming the unsupported path. An arbitrary
predicate is not translated to a made-up external constraint. Recursive
pure generation uses property size bounds; it must stop or discard when the
budget is exhausted. Macro derivation is an optional later convenience through
i/syntax; no compiler or private-constructor bypass is included today.

#### std/csv

Strict rows and incremental records. Reference: Go `encoding/csv/reader.go`,
`writer.go`; current algorithms. Keep Options/CsvError. parse_with and
print_with merge into explicit Options shapes. defaults/options stay. rows
and records raise CsvError from pull rather than returning Result elements.
Delete decode forwarding; `to_json` is the single row representation adapter,
then ordinary schema.decode handles it.

```text
Options = private
CsvError = (line = Int, column = Int, reason = String)
options : (String,Int,Bool) <- (Fail(CsvError)) -> Options
defaults : () -> Options
parse : (String) <- (Fail(CsvError)) -> List(List(String))
parse : (String,Options) <- (Fail(CsvError)) -> List(List(String))
parse_header : (String,Options) <- (Fail(CsvError)) -> List(dict.Dict(String,String))
print : (List(List(String))) -> String
print : (List(List(String)),Options) -> String
rows : (stream.Stream(String,E,N),Options) -> stream.Stream(List(String),E | CsvError,N)
records : (stream.Stream(String,E,N),Options) -> stream.Stream(dict.Dict(String,String),E | CsvError,N)
to_json : (dict.Dict(String,String)) -> json.Json
```

Line/column positions remain one-based scalar positions, unlike JSON's byte
offsets. Existing delimiter/CRLF/width/blank-line behavior and chunk-demand
regressions remain binding. Empty/malformed headers have explicit CsvError.
`stream.attempt(rows(...))` captures row outcomes when that is the requested data.

#### std/regex

Immutable scalar NFA programs and search. Reference: Go `regexp/regexp.go`.
Keep Regex, RegexError, Match, Captures and all ten functions. compile raises
RegexError; search input goes first. Move shared Unicode property tables to
unicode. Repeated-search complexity remains the documented O(m*n²) bound;
do not advertise the single-search bound for find_all.

```text
Regex, Captures = private
RegexError = (position = Int, reason = String)
Match = (start = Int, end = Int, text = String)
compile : (String) <- (Fail(RegexError)) -> Regex
is_match : (String,Regex) -> Bool
find : (String,Regex) -> Option(Match)
find_all : (String,Regex) -> List(Match)
captures : (String,Regex) -> Option(Captures)
group : (Captures,Int) -> Option(Match)
named : (Captures,String) -> Option(Match)
replace, replace_all : (String,Regex,String) -> String
split : (String,Regex) -> List(String)
```

#### std/uri

Validated URI references and the only query codec home. Reference: G
`uri.gleam`; GH `request.gleam`; Go `net/url/url.go`. Keep Uri/Parts/Authority/
Host/UriPart/UriProblem and all twelve functions. Merge CodecError's cases
into the UriError family; delete CodecError as a competing public failure
family. HTTP's query forwarding functions are deleted. All Result operations
become Fail(UriError). Offsets remain bytes, not scalar positions.

```text
Uri = private
Parts = (scheme = Option(String), authority = Option(Authority), path = String,
         query = Option(String), fragment = Option(String))
Authority = (userinfo = Option(String), host = Host, port = Option(String))
RegName = (String); IpLiteral = (String); Host = RegName | IpLiteral
UriPart = WholePart | SchemePart | AuthorityPart | UserInfoPart | HostPart |
          PortPart | PathPart | QueryPart | FragmentPart
// Each UriPart member has data ().
UriProblem = BadCharacter | BadPercent | BadScheme | BadAuthority | BadHost |
             BadPort | BadPath | RelativeBase | BadEscape | NonAscii | InvalidUtf8 | UnalignedBits
// Each problem member has data (); offset belongs to UriError, once.
UriError = (part = UriPart, offset = Int, problem = UriProblem)
parse : (String) <- (Fail(UriError)) -> Uri
build : (Parts) <- (Fail(UriError)) -> Uri
parts : (Uri) -> Parts
to_string : (Uri) -> String
has_scheme : (Uri) -> Bool
resolve : (Uri,Uri) <- (Fail(UriError)) -> Uri
encode_component : (String) -> String
encode_bytes : (BitArray) <- (Fail(UriError)) -> String
decode_component : (String) <- (Fail(UriError)) -> String
decode_bytes : (String) <- (Fail(UriError)) -> BitArray
parse_query : (String) <- (Fail(UriError)) -> List((String,String))
build_query : (List((String,String))) -> String
```

URI syntax validation does not validate DNS, port availability or filesystem
containment. Preserve ordered repeated query pairs. No dynamic map substitutes
for closed Parts data.

#### std/cli

Typed argument parsing, help and command execution. Reference: Go `flag/flag.go`;
G's plain data/call conventions. Keep Parser/Names and fifteen operations.
DefinitionError becomes cases of CliError; delete the redundant BadDefinition wrapper; Parser stores ordinary needs and
Fail-bearing converters. parse takes argument data first. read/run remain
host-backed composition. Pure text converter is identity, not a Result facade.

```text
Parser(A,E,N) = private
Names = (long = String, short = Option(String))
CliError = InvalidName | DuplicateName | MisplacedRest | UnknownOption | MissingValue |
           UnexpectedValue | RepeatedOption | MissingArgument | InvalidArgument | ExtraArgument
InvalidName = (String); DuplicateName = (String); MisplacedRest = ()
UnknownOption = (index = Int, token = String)
MissingValue = (index = Int, name = String)
UnexpectedValue = (index = Int, name = String)
RepeatedOption = (index = Int, name = String)
MissingArgument = (name = String)
InvalidArgument = (index = Int, name = String, text = String, reason = String)
ExtraArgument = (index = Int, token = String)
pure : (A) -> Parser(A,Never,Never)
flag : (Names,String) -> Parser(Bool,Never,Never)
option : (Names,String,(String) <- (..N,Fail(E)) -> A) -> Parser(A,E,N)
optional : (Names,String,(String) <- (..N,Fail(E)) -> A) -> Parser(Option(A),E,N)
many : (Names,String,(String) <- (..N,Fail(E)) -> A) -> Parser(List(A),E,N)
argument : (String,String,(String) <- (..N,Fail(E)) -> A) -> Parser(A,E,N)
rest : (String,String,(String) <- (..N,Fail(E)) -> A) -> Parser(List(A),E,N)
map : (Parser(A,E,N),(A) <- (..N,Fail(E)) -> B) -> Parser(B,E,N)
map2 : (Parser(A,E,N),Parser(B,E,N),(A,B) <- (..N,Fail(E)) -> C) -> Parser(C,E,N)
parse : (List(String),Parser(A,E,N)) <- (..N,Fail(E | CliError)) -> A
read : (Parser(A,E,N)) <- (..N,IO[iprocess.Arguments,E | CliError | iprocess.ProcessError]) -> A
run : (Parser(A,E,N),String,(A) <- N -> Nil)
  <- (..N,IO[iprocess.Arguments,stdio.Stdio,E | CliError | iprocess.ProcessError | stdio.StdioError]) -> Nil
help : (Parser(A,E,N),String) <- (Fail(CliError)) -> String
text : (String) -> String
integer : (String) <- (Fail(int.IntError)) -> Int
```

Preserve typed converter errors E instead of stringifying them into
InvalidArgument. That existing case remains for parser-owned token validation.
Usage/help failures do not swallow bad user data or failed output. Fake Args
and Stdio tests exercise read/run, not just parser internals.

#### std/config — new

Typed sources with explicit precedence. Reference: E `Config.ts`,
`ConfigProvider.ts`. Schema owns structure/validation; Config does not invent
a parallel field description DSL. Read returns absence as data; invalid values
and source failures remain errors. A provider may be passed explicitly when
combining sources, then provided to the application.

```text
ConfigProvider(N) = (read = (List(String)) <- (..N,Fail(ConfigError)) -> Option(json.Json))
ConfigError = (path = List(String), reason = ConfigProblem)
ConfigProblem = SourceFailure | InvalidPath | Missing
SourceFailure = (operation = String, cause = iprocess.ProcessError | ifs.FsError |
                 io.IoError | json.JsonError | unicode.UnicodeError)
InvalidPath = (); Missing = ()
Secret = private
memory : (json.Json) -> ConfigProvider(Never)
environment : (prefix = String) -> ConfigProvider((iprocess.Environment,request.Await,request.Owner))
file : (String,max_bytes = Int) <- (fs.FileSystem(N),..N,Fail(ConfigError)) -> ConfigProvider(Never)
fallback : (ConfigProvider(N),ConfigProvider(N)) -> ConfigProvider(N)
nested : (ConfigProvider(N),List(String)) -> ConfigProvider(N)
read : (List(String)) <- (ConfigProvider(N),..N,Fail(ConfigError)) -> Option(json.Json)
load : (List(String),schema.Schema(A,E,N,W))
  <- (ConfigProvider(N),..N,Fail(ConfigError | schema.SchemaError | E)) -> A
optional : (List(String),schema.Schema(A,E,N,W))
  <- (ConfigProvider(N),..N,Fail(ConfigError | schema.SchemaError | E)) -> Option(A)
secret : (String) -> Secret
reveal : (Secret) -> String
redacted : (Secret) -> String
```

`fallback(first, second)` consults second only for None. Empty strings are
present. Invalid data/source failure never falls back. Paths are explicit
segments, not ambiguous dotted strings. Environment uses std/process.env for its absence classification, then prefix plus
uppercase segments joined by `__`; missing environment means None, other
process failures become ConfigError. Its default values are JSON Text; the
application schema uses an explicit transform for text-to-port/bool conversion.
File sources parse one bounded JSON file at acquisition and return immutable
snapshots. CLI values enter through memory after ordinary CLI parsing.
Construct each source once in main, then write the precedence explicitly.

Secret holds its text behind a private closure, so generic inspection does
not print the underlying text. `redacted` returns `[redacted]`; typed log/schema
renderers use that value. `reveal` is explicit. It is not a claim of memory
erasure or protection after the caller reveals/copies the string.

### 3.8 Streams, sinks and common byte IO — lane G

#### std/stream

Lazy production with ordinary need-bearing pulls. Reference: E `Stream.ts`,
`Channel.ts`, `Pull.ts`; the existing replayable implementation is retained.
Keep Stream/Step/Done/Next and all sequential operations listed below.
wrap→singleton. Delete map_environment: use call-time services or an explicitly
kept factory, with its lifetime visible. Keep attempt/from_results as intentional
caught-data boundaries. The explicit E parameter is just Fail(E) in pull.

```text
Stream(A,E,N) = private
Done = (); Next(A,E,N) = (A,Stream(A,E,N))
Step(A,E,N) = Done | Next(A,E,N)
new : <A,N>() -> Stream(A,Never,N)
singleton : <A,N>(A) -> Stream(A,Never,N)
from_list : <A,N>(List(A)) -> Stream(A,Never,N)
make : (() <- (..N,Fail(E)) -> Step(A,E,N)) -> Stream(A,E,N)
defer : (() <- (..N,Fail(E)) -> Stream(A,E,N)) -> Stream(A,E,N)
unfold : (S,(S) <- (..N,Fail(E)) -> Option((A,S))) -> Stream(A,E,N)
pull : (Stream(A,E,N)) <- (..N,Fail(E)) -> Step(A,E,N)
map : (Stream(A,E,N),(A) <- (..N,Fail(F)) -> B) -> Stream(B,E | F,N)
filter : (Stream(A,E,N),(A) <- (..N,Fail(F)) -> Bool) -> Stream(A,E | F,N)
filter_map : (Stream(A,E,N),(A) <- (..N,Fail(F)) -> Option(B)) -> Stream(B,E | F,N)
flat_map : (Stream(A,E,N),(A) <- (..N,Fail(G)) -> Stream(B,F,N)) -> Stream(B,E | F | G,N)
append : (Stream(A,E,N),Stream(A,F,N)) -> Stream(A,E | F,N)
zip : (Stream(A,E,N),Stream(B,F,N)) -> Stream((A,B),E | F,N)
take, drop : (Stream(A,E,N),Int) -> Stream(A,E,N)
take_while : (Stream(A,E,N),(A) <- (..N,Fail(F)) -> Bool) -> Stream(A,E | F,N)
fold : (Stream(A,E,N),B,(B,A) <- N -> B) <- (..N,Fail(E)) -> B
fold_until : (Stream(A,E,N),B,(B,A) <- N -> list.ContinueOrStop(B)) <- (..N,Fail(E)) -> B
each : (Stream(A,E,N),(A) <- N -> B) <- (..N,Fail(E)) -> Nil
to_list : (Stream(A,E,N)) <- (..N,Fail(E)) -> List(A)
from_results : (List(Result(A,E))) -> Stream(A,E,N)
attempt : (Stream(A,E,N)) -> Stream(Result(A,E),Never,N)
run : (() <- (..N,resource.Scope,Fail(E)) -> Stream(A,E,N),
  (Stream(A,E,N)) <- (..N,Fail(E)) -> B) <- (..N,Fail(E)) -> B
from_reader : (io.Reader(E,N),Int) -> Stream(BitArray,E | io.IoError,N)
utf8 : (Stream(BitArray,E,N)) -> Stream(String,E | unicode.UnicodeError,N)
lines : (Stream(String,E,N),max_scalars = Int) -> Stream(String,E | StreamError,N)
chunks : (Stream(A,E,N),Int) -> Stream(List(A),E | StreamError,N)
StreamError = InvalidLimit | TokenTooLarge
InvalidLimit = (Int); TokenTooLarge = (limit = Int)
```

Pure streams remain replayable: repeating a pull repeats the work. A reader
stream observes external progress and is consumed once inside its owner's
Scope. `run` opens the source and calls its consumer inside a fresh resource
scope. It returns only the result; it cannot return a reader tail after closing
its owner. This factory argument makes resource acquisition explicit and
avoids a second Source wrapper. A caller can run a pure stream by passing
`() { values }`. A sink consumer calls sink.run(input, consumer).value.
This dependency direction is deliberate: sink imports stream; stream accepts
an ordinary consumer function and never imports sink. `sink.run` exposes leftovers only inside a caller's
existing lifetime. A bare `fold` does not secretly become a resource manager.

UTF-8 and line scanning retain partial input; line limits prevent unbounded
single-token buffering. CRLF is one line separator; a final unterminated line
is emitted, with no extra empty line after a final separator. Chunk size must
be positive. Empty steps cannot create busy loops.

Concurrent operations have callback lifetimes. `use` consumes their temporary
stream; returning from use cancels producers and waits for cleanup. This is
ordinary structured `task.race` composition, with the producer branch owning
its children and the consumer branch deciding normal completion. Producers
wait after sending end instead of winning the race themselves.

```text
buffer : (Stream(A,E,N),Int,(Stream(A,E,N)) <- (..N,Fail(E)) -> R)
  <- (..N,resource.Scope,Scheduler,Fail(E | queue.QueueError)) -> R
merge : (List(Stream(A,E,N)),Int,(Stream(A,E,N)) <- (..N,Fail(E)) -> R)
  <- (..N,resource.Scope,Scheduler,Fail(E | queue.QueueError)) -> R
map_concurrently : (Stream(A,E,N),Int,
  (A) <- (..N,Fail(E),Scheduler,Runner,request.Await) -> B,
  (Stream(B,E,N)) <- (..N,Fail(E)) -> R)
  <- (..N,resource.Scope,Scheduler,Fail(E | task.ScheduleError | queue.QueueError)) -> R
throttle : (Stream(A,E,N),time.Duration,
  (Stream(A,E | time.TimeError,N)) <- (..N,Fail(E | time.TimeError)) -> R)
  <- (..N,time.Clock,Scheduler,request.Await,request.Owner,Fail(E | time.TimeError)) -> R
```

Limit bounds queued elements and active producer work, not just a semaphore
around an unbounded list of task handles. Parallel map preserves input order;
merge intentionally preserves only each input's order. Captured non-Fail timer
services in throttle stay inside use; errors still go to the caller's catch.
No stream callback outlives use. Buffer/map/merge need no public Channel IR.
An input Reader and its response/file/connection owner close at the enclosing
`run` boundary, including early take, decode error and cancellation.

#### std/sink — new

Reusable consumers with explicit leftovers. Reference: E `Sink.ts`. A sink
contains an ordinary consuming function. It does not own a hidden scheduler.
All transforms and callbacks obey native needs.

```text
Sink(A,B,E,N) = private
End(A,B,E,N) = (value = B, rest = stream.Stream(A,E,N))
make : ((stream.Stream(A,E,N)) <- (..N,Fail(E)) -> End(A,B,E,N)) -> Sink(A,B,E,N)
run : (stream.Stream(A,E,N),Sink(A,B,E,N)) <- (..N,Fail(E)) -> End(A,B,E,N)
fold : (B,(B,A) <- (..N,Fail(E)) -> B) -> Sink(A,B,E,N)
fold_until : (B,(B,A) <- (..N,Fail(E)) -> list.ContinueOrStop(B)) -> Sink(A,B,E,N)
collect : () -> Sink(A,List(A),E,N)
take : (Int) <- (Fail(stream.StreamError)) -> Sink(A,List(A),E,N)
head : () -> Sink(A,Option(A),E,N)
each : ((A) <- (..N,Fail(E)) -> Nil) -> Sink(A,Nil,E,N)
map : (Sink(A,B,E,N),(B) <- (..N,Fail(E)) -> C) -> Sink(A,C,E,N)
zip : (Sink(A,B,E,N),Sink(A,C,E,N)) -> Sink(A,(B,C),E,N)
```

zip runs the first sink, then the second on its remaining input. It does not
broadcast or run concurrently. take accepts zero and leaves the stream untouched.
head on empty input returns None. fold_until consumes the stopping element
and returns the following stream. `stream.run` consumers return End.value and deliberately leave End.rest
inside the resource scope. Leftovers are never pulled speculatively.

#### std/io — new

Small byte protocols, precise progress and bounded composition. Reference:
Go `io/io.go`, `bufio/bufio.go`, `bufio/scan.go`. Keep i/io separate: it is the
specified best-effort printer, not this module. No mutable caller buffer or
interface keyword is added.

```text
IoError = (operation = String, transferred = Int, reason = IoProblem)
IoProblem = InvalidLimit | Unaligned | UnexpectedEnd | NoProgress | BadProgress | TooLarge
// Each IoProblem member has data ().
Reader(E,N) = (read = (Int) <- (..N,Fail(E | IoError)) -> Option(BitArray))
Writer(E,N) = (write = (BitArray) <- (..N,Fail(E | IoError)) -> Int)
read_exact : (Reader(E,N),Int) <- (..N,Fail(E | IoError)) -> BitArray
read_all : (Reader(E,N),max_bytes = Int) <- (..N,Fail(E | IoError)) -> BitArray
write_all : (BitArray,Writer(E,N)) <- (..N,Fail(E | IoError)) -> Nil
copy : (Reader(E,N),Writer(E,N),max_bytes = Int) <- (..N,Fail(E | IoError)) -> Int
limit : (Reader(E,N),Int) <- (Fail(IoError)) -> Reader(E,N)
tee : (Reader(E,N),Writer(E,N)) -> Reader(E,N)
from_bytes : (BitArray) <- (Scheduler,Fail(IoError)) -> Reader(Never,Scheduler)
collect : () <- (Scheduler) -> (writer = Writer(Never,Scheduler), bytes = () <- (Scheduler) -> BitArray)
```

Read takes a positive byte bound. None is EOF; Some contains 1..bound whole
bytes. It may return fewer than requested. Zero/negative bound fails IoError.
A reader that receives both bytes and a later failure emits those bytes first
and raises the retained failure on the next read; adapters cannot fabricate
partial data a host never reports. A pure parser cursor is instead explicit
successor state in std/bytes; from_bytes uses a scheduler Ref for IO-style reads.

Writer.write returns the accepted prefix length, 0..input byte count. It may
raise after external partial effects. An empty write returns zero. write_all
handles short writes, rejects zero progress on nonempty input, and never
claims atomicity after an error. Today's Nil-returning host writes map success
to the full supplied byte count; their documented full-write promise must be
tested. A host that can short-write must loop or report progress in its adapter.
This chooses the style audit's counted-progress option; it is not a new ABI.

read_all/copy reject growth beyond max_bytes before retaining excess data;
the source may already have advanced by the bounded probe that discovered it.
read_exact(0) returns empty without reading. limit ends at its byte budget;
tee writes a successfully read chunk before returning it. These values borrow
their handle lifetime; none pretends a dropped reader is a destructor.

### 3.9 Native IO, files, paths, commands and terminal — lane H

#### Native capability contracts

These modules retain the existing host namespaces, requests, creation guards,
handle representations and cancellation/lease rules. Reference: Go `os/file.go`,
`net/net.go`, `crypto/tls/`, `os/exec/exec.go`; E `FileSystem.ts`,
`process/ChildProcessSpawner.ts`, `socket/Socket.ts`, `Terminal.ts`.
Host Result replies remain boundary data; public operations raise native needs.
No new host operation or unsupported-only platform provider is added.

| Module | Public types and disposition |
|---|---|
| `i/io` | Keep the specified public `Io = (print = (String) -> Nil)` service and no free functions. Best-effort newline printing intentionally reports no error. |
| `i/stdio` | Keep public Stdio constructor and all seven functions. Stdio's existing read/write/write_error/flush/flush_error Request fields stay. StdioError gains a typed reason; raw code remains diagnostic. |
| `i/fs` | Keep private raw Fs/File constructors, OpenMode and Metadata. Keep minimal operations below. Move read_file/write_file/read_lines algorithms to std/fs; update spec §9.12's convenience-operation location in the migration, rather than leaving two implementations. |
| `i/process` | Keep Arguments/Environment/WorkingDirectory/Entropy public constructors, raw Process and Child private. Keep all fourteen operations. Directory's value constructor becomes AtDirectory, fixing the duplicate name; the union stays Directory. |
| `i/net` | Keep private Net/Listener/Stream/Datagram, NetError and all seventeen operations. Keep specified close_stream/close_listener/close_datagram rather than prematurely merging their different contracts. byte_stream retains a low-level lib-only byte record; std/io adaptation lives above it. |
| `i/tls` | Keep Tls/Stream, TlsError and all eleven functions. Keep options/identity as closed data. accept's identity carries cert/key; split server options to alpn only, deleting ignored roots/cert/key fields from that argument. |
| `i/fetch` | Keep Fetch/Response and five functions. Replace Fail(String) with one FetchError. Scope-owning HTTP construction is above this raw adapter. |

Native error details are never parsed by application code. Each module owns
its own reason enum; shared case spellings below are qualified constructors.
Map host codes exactly once and retain the original code for diagnostics.

```text
FsError = (operation = String, path = String, reason = FsProblem, code = String)
FsProblem = NotFound | Denied | Exists | Closed | Busy | Invalid | Unsupported | Other
NetError = (operation = String, endpoint = String, reason = NetProblem, code = String)
NetProblem = NotFound | Denied | Closed | Busy | Invalid | Unreachable | Other
ProcessError = (operation = String, reason = ProcessProblem, code = String)
ProcessProblem = NotFound | Denied | Closed | Busy | Invalid | Other
StdioError = (operation = String, reason = StdioProblem, code = String)
StdioProblem = Closed | Busy | Invalid | Other
TlsError = (stage = String, reason = TlsProblem, detail = String)
TlsProblem = Trust | Hostname | Handshake | Closed | Busy | Other
FetchError = (operation = String, reason = FetchProblem, detail = String)
FetchProblem = Invalid | Network | Closed | Other
// All reason-enum members have data (). Raw namespace/code strings are not expanded error sums.
```

Public native functions use `IO[S,E]` from §3.1:

```text
// i/stdio
read : (Int) <- IO[Stdio,StdioError] -> BitArray
write, write_error : (BitArray) <- IO[Stdio,StdioError] -> Nil
flush, flush_error : () <- IO[Stdio,StdioError] -> Nil
print, print_error : (String) <- IO[Stdio,StdioError] -> Nil
// i/fs
Fs, File = private
ReadOnly = (); WriteTruncate = (); Append = (); ReadWrite = ()
OpenMode = ReadOnly | WriteTruncate | Append | ReadWrite
Metadata = (size = Int, kind = String)
open : (String,OpenMode) <- IO[Fs,FsError] -> File
read : (File,Int) <- IO[Fs,FsError] -> BitArray
write : (File,BitArray) <- IO[Fs,FsError] -> Nil
flush : (File) <- IO[Fs,FsError] -> Nil
close : (File) <- IO[Fs] -> Nil
remove, mkdir : (String) <- IO[Fs,FsError] -> Nil
rename : (String,String) <- IO[Fs,FsError] -> Nil
list : (String) <- IO[Fs,FsError] -> List(String)
metadata : (String) <- IO[Fs,FsError] -> Metadata
temp_dir : () <- IO[Fs,FsError] -> String
// i/process
Arguments = (args = () -> Request(Result(BitArray,String)))
Environment = (env = (String) -> Request(Result(String,String)))
WorkingDirectory = (cwd = () -> Request(Result(String,String)))
Entropy = (entropy = (Int) -> Request(Result(BitArray,String)))
Process, Child = private
Inherit = (); Null = (); Piped = (); Stdio = Inherit | Null | Piped
InheritEnvironment = (List(String)); ReplaceEnvironment = (List(String))
EnvironmentMode = InheritEnvironment | ReplaceEnvironment
InheritDirectory = (); AtDirectory = (String)
Directory = InheritDirectory | AtDirectory
args : () <- IO[Arguments,ProcessError] -> List(String)
env : (String) <- IO[Environment,ProcessError] -> String
cwd : () <- IO[WorkingDirectory,ProcessError] -> String
entropy : (Int) <- IO[Entropy,ProcessError] -> BitArray
spawn : (String,List(String)) <- IO[Process,ProcessError] -> Child
spawn_piped : (String,List(String),Stdio,Stdio,Stdio) <- IO[Process,ProcessError] -> Child
spawn_config : (String,List(String),Stdio,Stdio,Stdio,EnvironmentMode,Directory)
  <- IO[Process,ProcessError] -> Child
read_stdout, read_stderr : (Child,Int) <- IO[Process,ProcessError] -> BitArray
write_stdin : (Child,BitArray) <- IO[Process,ProcessError] -> Nil
close_stdin : (Child) <- IO[Process,ProcessError] -> Nil
wait : (Child) <- IO[Process,ProcessError] -> Int
terminate : (Child) <- IO[Process,ProcessError] -> Nil
close : (Child) <- IO[Process] -> Nil
// i/net
Net, Listener, Stream, Datagram = private
bind : (String,Int) <- IO[Net,NetError] -> Datagram
send_to : (Datagram,String,Int,BitArray) <- IO[Net,NetError] -> Nil
receive_from : (Datagram,Int) <- IO[Net,NetError] -> (address = String, port = Int, data = BitArray)
local_address : (Datagram) <- IO[Net,NetError] -> String
close_datagram : (Datagram) <- IO[Net] -> Nil
listen : (String,Int) <- IO[Net,NetError] -> Listener
local_port : (Listener | Datagram) <- IO[Net,NetError] -> Int
accept : (Listener) <- IO[Net,NetError] -> Stream
connect : (String,Int) <- IO[Net,NetError] -> Stream
resolve : (String) <- IO[Net,NetError] -> List(String)
read : (Stream,Int) <- IO[Net,NetError] -> BitArray
write : (Stream,BitArray) <- IO[Net,NetError] -> Nil
shutdown_write : (Stream) <- IO[Net,NetError] -> Nil
close_stream : (Stream) <- IO[Net] -> Nil
close_listener : (Listener) <- IO[Net] -> Nil
stream_number : (Stream) -> Int
byte_stream : (Stream) -> (read = (Int) <- IO[Net,NetError] -> BitArray,
  write = (BitArray) <- IO[Net,NetError] -> Nil,
  shutdown_write = () <- IO[Net,NetError] -> Nil, close = () <- IO[Net] -> Nil)
// i/tls
Tls, Stream = private
options : () -> (alpn = List(String), cert = BitArray, key = BitArray, roots = BitArray)
identity : (BitArray,BitArray) -> (cert = BitArray, key = BitArray)
connect : (net.Stream,String,(alpn = List(String),cert = BitArray,key = BitArray,roots = BitArray))
  <- IO[Tls,TlsError] -> Stream
accept : (net.Stream,(cert = BitArray,key = BitArray),(alpn = List(String)))
  <- IO[Tls,TlsError] -> Stream
dial : (String,Int,(alpn = List(String),cert = BitArray,key = BitArray,roots = BitArray))
  <- IO[Tls,net.Net,TlsError | net.NetError] -> Stream
alpn : (Stream) <- IO[Tls,TlsError] -> String
read : (Stream,Int) <- IO[Tls,TlsError] -> BitArray
write : (Stream,BitArray) <- IO[Tls,TlsError] -> Nil
shutdown_write : (Stream) <- IO[Tls,TlsError] -> Nil
close : (Stream) <- IO[Tls] -> Nil
byte_stream : (Stream) -> (read = (Int) <- IO[Tls,TlsError] -> BitArray,
  write = (BitArray) <- IO[Tls,TlsError] -> Nil,
  shutdown_write = () <- IO[Tls,TlsError] -> Nil, close = () <- IO[Tls] -> Nil)
// i/fetch
Fetch, Response = private
open : (String,String,String,BitArray) <- IO[Fetch,FetchError] -> Response
status : (Response) <- IO[Fetch,FetchError] -> Int
headers : (Response) <- IO[Fetch,FetchError] -> String
read : (Response,Int) <- IO[Fetch,FetchError] -> BitArray
close : (Response) <- IO[Fetch] -> Nil
```

The lib-only net/TLS byte records remain different because their nominal
errors differ. Their algorithms are not duplicated: std/io's one contract
and G's native adapter functions handle EOF/progress. They cannot import
std/io from lib/i. The native/fetch HTTP providers translate transport errors
once into HttpError while retaining typed underlying details.

#### std/fs — new high-level home

Scoped files and replaceable filesystem policy. Reference: E `FileSystem.ts`;
Go `io/fs/fs.go`, `os/file.go`. Move today's i/fs.read_file/write_file/read_lines
here and implement them through std/io and one acquisition path. The migration
updates spec's module location with those moves. `FileSystem(N)` is an ordinary
service so memory tests do not need mutable raw host fields or a second host ABI.
It reuses i/ifs.OpenMode/Metadata/FsError; it defines no second filesystem error.

```text
File(N) = (reader = io.Reader(ifs.FsError,N), writer = io.Writer(ifs.FsError,N),
           flush = () <- (..N,Fail(ifs.FsError)) -> Nil, close = () <- N -> Nil)
FileSystem(N) = (
  open = (String,ifs.OpenMode) <- (..N,resource.Scope,Fail(ifs.FsError)) -> File(N),
  remove = (String) <- (..N,Fail(ifs.FsError)) -> Nil,
  rename = (String,String) <- (..N,Fail(ifs.FsError)) -> Nil,
  mkdir = (String) <- (..N,Fail(ifs.FsError)) -> Nil,
  list = (String) <- (..N,Fail(ifs.FsError)) -> List(String),
  metadata = (String) <- (..N,Fail(ifs.FsError)) -> ifs.Metadata,
  temp_dir = () <- (..N,Fail(ifs.FsError)) -> String)
native : () <- (ifs.Fs) -> FileSystem((request.Await,request.Owner))
memory : (dict.Dict(String,BitArray)) <- (resource.Scope,Scheduler) -> FileSystem(Scheduler)
open : (String,ifs.OpenMode) <- (FileSystem(N),..N,resource.Scope,Fail(ifs.FsError)) -> File(N)
read_file : (String,max_bytes = Int)
  <- (FileSystem(N),..N,Fail(ifs.FsError | io.IoError)) -> BitArray
write_file : (String,BitArray) <- (FileSystem(N),..N,Fail(ifs.FsError | io.IoError)) -> Nil
read_lines : (String,max_bytes = Int)
  <- (FileSystem(N),..N,Fail(ifs.FsError | io.IoError | unicode.UnicodeError)) -> List(String)
stream : (String,chunk_bytes = Int)
  <- (FileSystem(N),..N,resource.Scope,Fail(ifs.FsError)) -> stream.Stream(BitArray,ifs.FsError | io.IoError,N)
write_sink : (String) <- (FileSystem(N),..N,resource.Scope,Fail(ifs.FsError))
  -> sink.Sink(BitArray,Nil,ifs.FsError | io.IoError,N)
copy : (String,String,max_bytes = Int)
  <- (FileSystem(N),..N,Fail(ifs.FsError | io.IoError)) -> Int
walk : (String,max_depth = Int) <- (FileSystem(N),..N,resource.Scope,Fail(ifs.FsError))
  -> stream.Stream((path = String, metadata = ifs.Metadata),ifs.FsError,N)
```

Whole-file helpers create their own scoped use; streaming helpers borrow the
surrounding scope. write_file flushes explicitly with Fail before non-failing
close. copy does not promise atomic destination replacement. walk is lazy
preorder with a required nonnegative depth bound. Native metadata currently
follows links; walk inherits that provider behavior and can revisit linked
paths until max_depth. It is not containment or a no-follow traversal. Native
directory list still buffers one directory; do not claim constant space for
that provider. Exclusive temporary creation, lstat and watcher operations
need a future host contract; they have no speculative export here. The existing
temp_dir observation stays available.

File acquisition registers close before exposing reader/writer fields. Memory
is the one model filesystem provider reused by test_services. Unsupported
operations fail FsError(Unsupported), not a fake platform installation that
pretends an absent host capability exists. Service fakes themselves may model
capabilities their tests choose; platform target tables remain authoritative.

#### std/path

Pure slash-separated lexical paths. Reference: Go `path/path.go`.
Keep all eight functions and their algorithms. RelativeError→PathError;
DifferentRoots/UnknownParent remain its cases; relative now raises Fail.
There is no target-dependent change to path meaning.

```text
DifferentRoots = (); UnknownParent = (); PathError = DifferentRoots | UnknownParent
clean : (String) -> String
join : (List(String)) -> String
split : (String) -> (String,String)
base, dir, extension : (String) -> String
is_absolute : (String) -> Bool
relative : (String,String) <- (Fail(PathError)) -> String
```

Lexical clean is not filesystem containment. Native Windows path rules and
file-URL conversion are separate extensions, never inferred from the target.

#### std/process — new

Immutable commands and scoped pipelines. Reference: E
`process/ChildProcess.ts`, `process/ChildProcessSpawner.ts`; Go `os/exec/exec.go`.
Reuse i/process's capability split and ProcessError. A Spawner is an ordinary
high-level service for tests; it captures the raw Process choice deliberately.
No shell interpolation or detached child is added.

```text
Command = (program = String, args = List(String), directory = iprocess.Directory,
           environment = iprocess.EnvironmentMode, stdin = iprocess.Stdio,
           stdout = iprocess.Stdio, stderr = iprocess.Stdio)
Child(N) = (stdin = io.Writer(iprocess.ProcessError,N),
            stdout = io.Reader(iprocess.ProcessError,N), stderr = io.Reader(iprocess.ProcessError,N),
            close_stdin = () <- (..N,Fail(iprocess.ProcessError)) -> Nil,
            wait = () <- (..N,Fail(iprocess.ProcessError)) -> Int,
            terminate = () <- (..N,Fail(iprocess.ProcessError)) -> Nil,
            close = () <- N -> Nil)
Spawner(N) = (spawn = (Command) <- (..N,resource.Scope,Fail(iprocess.ProcessError)) -> Child(N))
Output = (status = Int, stdout = BitArray, stderr = BitArray)
command : (String,List(String)) -> Command
native : () <- (iprocess.Process) -> Spawner((request.Await,request.Owner))
spawn : (Command) <- (Spawner(N),..N,resource.Scope,Fail(iprocess.ProcessError)) -> Child(N)
collect : (Command,max_bytes = Int)
  <- (Spawner(N),..N,Scheduler,Fail(iprocess.ProcessError | io.IoError)) -> Output
pipeline : (List(Command),max_bytes = Int)
  <- (Spawner(N),..N,Scheduler,Fail(iprocess.ProcessError | io.IoError)) -> Output
env : (String) <- IO[iprocess.Environment,iprocess.ProcessError] -> Option(String)
```

command defaults to inherited directory/environment, null stdin and piped
stdout/stderr. collect and pipeline concurrently drain both output streams,
close stdin when copying finishes, and wait for child completion. Nonzero exit
is Output.status data, not automatically Fail. Parent cancellation terminates
and waits before closing handles. Spawn failure closes all earlier pipeline
children. env maps only the native missing-variable condition to None; empty
is Some(""). This is the shared absence adapter used by config.environment.

#### std/terminal — new cooked IO

Line-oriented interaction on stdin/stdout, useful without a raw terminal.
Reference: E `Terminal.ts`; Go `bufio/scan.go`. This first API needs only the
existing Stdio capability. No unsupported size/key-event methods are exported.

```text
TerminalError = LineTooLong
LineTooLong = (limit = Int)
read_line : (max_bytes = Int)
  <- IO[stdio.Stdio,TerminalError | stdio.StdioError | unicode.UnicodeError] -> Option(String)
display : (String) <- IO[stdio.Stdio,stdio.StdioError] -> Nil
prompt : (String,max_bytes = Int)
  <- IO[stdio.Stdio,TerminalError | stdio.StdioError | unicode.UnicodeError] -> Option(String)
```

read_line excludes the line terminator; EOF before any bytes is None, a final
unterminated line is Some. prompt displays and flushes before reading.
display adds no newline; stdio.print is the explicit newline operation.
Raw mode, terminal size, keys and mode restoration need a future optional host
contract; ordinary byte stdio remains usable on every supported target.

### 3.10 HTTP data, client and server — lane I

#### std/http

Pure HTTP message data, framing and buffered-body decoding. Reference:
E `http/HttpClientRequest.ts`, `http/HttpClientResponse.ts`; GH `request.gleam`,
`response.gleam`; Go `net/http/`. Keep one parser/serializer implementation.
Request/Response become body-parameterized. Move service/network operations
to http_client/http_server. Do not re-export them from this module.

Resolve the duplicate Headers declaration as one private byte-header value.
Keep ordered repeated headers and case-insensitive names. RawHeaders and its
constructor disappear; from_header_bytes preserves raw values explicitly.
header_bytes stays, headers→header_values, add_header stays. BodyError's cases
move into HttpError; BodyError disappears. NeedMore is progress, not an error.
Timeout moves to std/concurrent. Query codecs live only in uri.

```text
Method = (String); Status = (Int); Headers = private
Request(B) = (method = Method, target = String, headers = Headers, body = B, trailers = Headers)
Response(B) = (status = Status, reason = String, headers = Headers, body = B, trailers = Headers)
HttpError = BadMessage | Dns | Connect | Tls | Fetch | Protocol | StatusError |
            BodyTooLarge | InvalidText | InvalidJson | InvalidSchema
BadMessage = (String); Dns = (net.NetError); Connect = (net.NetError)
Tls = (tls.TlsError); Fetch = (fetch.FetchError); Protocol = (String)
StatusError = (Int); BodyTooLarge = (limit = Int); InvalidText = ()
InvalidJson = (json.JsonError); InvalidSchema = (schema.SchemaError)
More = (BitArray); Complete(A) = (A,BitArray); Invalid = (HttpError)
Progress(A) = More | Complete(A) | Invalid
ParseOptions = (method = Option(Method), head_only = Bool, max_header_bytes = Int, max_body_bytes = Int)
parse_options : () -> ParseOptions
request : (Method,String) -> Request(BitArray)
response : (Status) -> Response(BitArray)
from_headers : (List((String,String))) <- (Fail(HttpError)) -> Headers
from_header_bytes : (List((String,BitArray))) <- (Fail(HttpError)) -> Headers
header : (Headers,String) <- (Fail(HttpError)) -> Option(String)
header_bytes : (Headers,String) -> Option(BitArray)
header_values : (Headers,String) <- (Fail(HttpError)) -> List(String)
add_header : (Headers,String,String) <- (Fail(HttpError)) -> Headers
feed_request : (BitArray,BitArray) -> Progress(Request(BitArray))
feed_request : (BitArray,BitArray,ParseOptions) -> Progress(Request(BitArray))
feed_response : (BitArray,BitArray) -> Progress(Response(BitArray))
feed_response : (BitArray,BitArray,ParseOptions) -> Progress(Response(BitArray))
serialize_request : (Request(BitArray)) <- (Fail(HttpError)) -> BitArray
serialize_response : (Response(BitArray)) <- (Fail(HttpError)) -> BitArray
reason : (Int) -> String
body_text : (Response(BitArray)) <- (Fail(HttpError)) -> String
body_json : (Response(BitArray)) <- (Fail(HttpError)) -> json.Json
body_decode : (Response(BitArray),schema.Schema(A,E,D,W)) <- (..D,Fail(E | HttpError)) -> A
route : (Request(B),Method,String) -> Option(List((String,String)))
content_length, content_type, transfer_encoding, host, connection : String
```

feed_response_for/feed_request_limit merge into ParseOptions shapes. `route`
puts Request first. Buffered decoders raise InvalidText/InvalidJson/InvalidSchema;
custom schema error E remains typed. Header text access rejects invalid UTF-8
instead of confusing bad data with absence. Header insertion validates names
and rejects CR/LF. Serialization validates method, target, status and framing.
No function silently rewrites an invalid outgoing message into a valid one.

Default parser limits are 64 KiB headers and 8 MiB buffered body. Options must
be positive; invalid options produce Invalid(BadMessage(...)). Stream drivers
use the same head/framing logic incrementally. More remains ordinary parser
progress, not a second computation failure protocol.

#### std/http_client — new home for existing client work

Scoped pooled clients and transport substitution. Reference: E
`http/HttpClient.ts`; Go `net/http/client.go`, `transport.go`.
Move native_client/fetch_client/get/post/send/send_stream here. Today's
Client.make/Transport split collapses into one explicitly constructed service.
Transport and ClientFunctions disappear from the public API. ResponseStream
becomes Response(io.Reader(...)) with its release registered in Scope.
Delete client() forwarding factory and send_with: construct once, provide once,
then use an ordinary function. Delete send_timeout: use concurrent.timeout.

`H` below is `(request.Await, request.Owner, task.Scheduler, task.Runner)`.
E is the request-body source error type; it is part of ordinary Fail only.
A shared client chooses the application's body-error union once. Empty and
buffered bodies fit through pure readers/streams and do not introduce errors.

```text
Client(E,N) = (send = (http.Request(io.Reader(E,N)))
  <- (..N,resource.Scope,Fail(E | http.HttpError | io.IoError)) -> http.Response(io.Reader(http.HttpError,N)))
Options = (max_body_bytes = Int, redirects = Int)
options : () -> Options
native_client : () <- (net.Net,tls.Tls,resource.Scope,Scheduler) -> Client(E,H)
tcp_client : () <- (net.Net,resource.Scope,Scheduler) -> Client(E,H)
fetch_client : () <- (fetch.Fetch,resource.Scope,Scheduler) -> Client(E,H)
send_stream : (http.Request(io.Reader(E,N)))
  <- (Client(E,N),..N,resource.Scope,Fail(E | http.HttpError | io.IoError)) -> http.Response(io.Reader(http.HttpError,N))
send : (http.Request(BitArray)) <- (Client(E,N),..N,Fail(E | http.HttpError | io.IoError)) -> http.Response(BitArray)
send : (http.Request(BitArray),Options) <- (Client(E,N),..N,Fail(E | http.HttpError | io.IoError)) -> http.Response(BitArray)
get : (String) <- (Client(E,N),..N,Fail(E | http.HttpError | io.IoError)) -> http.Response(BitArray)
post : (String,BitArray) <- (Client(E,N),..N,Fail(E | http.HttpError | io.IoError)) -> http.Response(BitArray)
expect_success : (http.Response(B)) <- (Fail(http.HttpError)) -> http.Response(B)
```

Factories capture the chosen raw transport services and own pool cleanup in
Scope. Each streaming response registers its release in the consuming scope;
closing a factory before returning a live body is forbidden. send creates a
child response scope, collects within the configured bound, then closes that
body. The pool's outer scope remains live across calls. tcp_client is the
explicit HTTP-only option on targets with Net but no Tls; HTTPS fails with a
protocol error rather than inventing a Tls service.

Native send_stream reads the socket as demanded; its first chunk must arrive
before the complete body exists. Fetch obeys the same close/early-exit contract.
An unread body removes or closes its connection; only a completely consumed
reusable body returns a connection to the pool. Redirect policy preserves
existing sensitive-header/origin rules. Streaming request bodies are not
replayed automatically; a retrying application must supply a fresh-body factory
and an idempotency policy. Default send bound is 8 MiB; redirects default to 10.

Non-2xx status is ordinary response data until expect_success is called.
Timeout/retry compose ordinary functions from C, with no HTTP-specific timing
engine. Tracing middleware takes an ordinary send function and returns one;
no package import installs logging, tracing or retry policy.

#### std/http_server — new home for existing server work

Bounded, structured HTTP/1.1 serving and testable handlers. Reference: E
`http/HttpServer.ts`, `http/HttpRouter.ts`, `http/HttpServerResponse.ts`;
Go `net/http/server.go`. Move serve/serve_stream here. route remains pure in
http. Middleware is a function on handler functions; it earns no new class.

```text
Options = (concurrency = Int, max_header_bytes = Int, max_body_bytes = Int)
options : () -> Options
serve : (net.Listener,(http.Request(BitArray)) <- (..N,Fail(E)) -> http.Response(BitArray))
  <- (net.Net,Scheduler,..N,Fail(E | net.NetError | http.HttpError | io.IoError)) -> Never
serve : (net.Listener,Options,
  (http.Request(BitArray)) <- (..N,Fail(E)) -> http.Response(BitArray))
  <- (net.Net,Scheduler,..N,Fail(E | net.NetError | http.HttpError | io.IoError)) -> Never
serve_stream : (net.Listener,Options,
  (http.Request(io.Reader(http.HttpError,N))) <- (..N,Fail(E))
    -> http.Response(stream.Stream(BitArray,E,N)))
  <- (net.Net,Scheduler,..N,Fail(E | net.NetError | http.HttpError | io.IoError)) -> Never
test_client : ((http.Request(BitArray)) <- (..N,Fail(E)) -> http.Response(BitArray))
  -> (http.Request(BitArray)) <- (..N,Fail(E)) -> http.Response(BitArray)
```

Default admission is 128 live connections, with the same 64 KiB/8 MiB bounds.
Validate Options before accepting work, using HttpError.BadMessage for invalid
configuration. Admission precedes per-connection task allocation. Slow/partial
bodies have bounded buffering and stream demand, and handlers own request/response
resources through completion. Parent cancellation waits for every handler and
connection finalizer. Streaming responses write chunks incrementally and close
sources after write failure. The pure test_client invokes the same handler;
separate loopback tests exercise framing and transport. Malformed peer messages
close that peer; unhandled application E follows task-group failure rules.

### 3.11 Diagnostics, observability, caching, batching and tests — lane J

#### std/cause — new diagnostic data

Reference: E `Cause.ts`, `Exit.ts`; Go `errors/join.go`. This is explicit
collected diagnostic data, not a catch-all runtime outcome. It contains only
errors the caller actually caught or received. Do not add Die/Interrupt,
recoverable panic, a changed join result or an Exit service.

```text
Cause(E) = (errors = List(E), annotations = List((String,String)))
from_error : (E) -> Cause(E)
combine : (Cause(E),Cause(E)) -> Cause(E)
map : (Cause(E),(E) <- N -> F) <- N -> Cause(F)
annotate : (Cause(E),String,String) -> Cause(E)
pretty : (Cause(E),(E) -> String) -> String
```

combine preserves left-to-right order. It does not claim that task groups
report all concurrent failures; their least-path rule remains unchanged.
No matcher module is added. Branches, guards, type tests and exhaustiveness
already cover matching, selective recovery and domain dispatch.

#### std/log

Structured, replaceable logging with explicit policy. Reference: E `Logger.ts`;
Go `log/slog/handler.go`. Sink(E,N)→Logger(E,N) avoids confusion with stream
consumers. Failure level→Error; use qualification to distinguish prelude Error.
Keep make/silent/filter/minimum/enabled/write/lazy/format/stdout/stderr/file/
timestamped. Drop the unused variance marker. Add JSON formatting, fan-out and
immutable contextual fields. File factory acquires once in Scope, not per entry.

```text
Debug = (); Info = (); Warning = (); Error = (); Level = Debug | Info | Warning | Error
Value = json.Json | config.Secret
Entry = (level = Level, message = String, fields = List((String,Value)), at = Option(time.Instant))
Logger(E,N) = private service
make : ((Level) -> Bool,(Entry) <- (..N,Fail(E)) -> Nil) -> Logger(E,N)
silent : () -> Logger(Never,Never)
filter : (Logger(E,N),(Level) -> Bool) -> Logger(E,N)
minimum : (Logger(E,N),Level) -> Logger(E,N)
enabled : (Level) <- (Logger(E,N)) -> Bool
write : (Entry) <- (Logger(E,N),..N,Fail(E)) -> Nil
lazy : (Level,() <- N -> String) <- (Logger(E,N),..N,Fail(E)) -> Nil
format, format_json : (Entry) -> String
stdout, stderr : () -> Logger(stdio.StdioError,(stdio.Stdio,request.Await,request.Owner))
file : (String) <- (fs.FileSystem(N),..N,resource.Scope,Fail(ifs.FsError))
  -> Logger(ifs.FsError | io.IoError,N)
timestamped : (Entry) <- ClockNeeds -> Entry
fanout : (List(Logger(E,N))) -> Logger(E,N)
with_fields : (List((String,Value)),() <- (Logger(E,N),..M) -> A)
  <- (Logger(E,N),..M) -> A
```

Filtered messages do not evaluate their lazy callbacks or timestamps. fanout
writes in list order and propagates the first failure. with_fields provides an
annotated Logger value to its body using ordinary `<-`; no global context map.
Fields on the entry override contextual fields of the same name. Secret values
always render through config.redacted. File flushing errors occur in normal
write calls; scope close follows a documented non-failing close policy.
No background logger queue or silent drop is installed by default. Users may
compose queue/batch explicitly when their overflow and failure policy is known.

#### std/metric — new

Scoped measurement state and snapshots. Reference: E `Metric.ts`. One ordinary
Registry service uses a Ref; there is no process-global mutable registry.
Start with counter, gauge and histogram. Export is explicit snapshot data.

```text
Counter, Gauge, Histogram = private metric descriptions
MetricError = InvalidName | InvalidBuckets | InvalidDelta | Capacity
InvalidName = (String); InvalidBuckets = (); InvalidDelta = (); Capacity = (Int)
Count = (Int); Level = (Float); Buckets = (bounds = List(Float), counts = List(Int))
Value = Count | Level | Buckets
Sample = (name = String, labels = List((String,String)), value = Value)
Measurement = Add | Set | Observe
Add = (Counter,Int); Set = (Gauge,Float); Observe = (Histogram,Float)
Registry = (record = (Measurement) <- (Scheduler,Fail(MetricError)) -> Nil,
            snapshot = () <- (Scheduler) -> List(Sample))
registry : (max_series = Int) <- (resource.Scope,Scheduler,Fail(MetricError)) -> Registry
counter : (String,List((String,String))) <- (Fail(MetricError)) -> Counter
gauge : (String,List((String,String))) <- (Fail(MetricError)) -> Gauge
histogram : (String,List((String,String)),List(Float)) <- (Fail(MetricError)) -> Histogram
add : (Counter,Int) <- (Registry,Scheduler,Fail(MetricError)) -> Nil
set : (Gauge,Float) <- (Registry,Scheduler,Fail(MetricError)) -> Nil
observe : (Histogram,Float) <- (Registry,Scheduler,Fail(MetricError)) -> Nil
snapshot : () <- (Registry,Scheduler) -> List(Sample)
```

Counter deltas are nonnegative exact Ints. Gauges are finite Float. Histogram
bounds are strictly increasing; each observation increments exactly one bucket,
including a final unbounded bucket. Counts remain exact Int; no overflowing
floating sum is promised. Labels are normalized by name and duplicate labels
are rejected. Capacity bounds distinct name/label series, not observation count.
Creating a description does not register global state. Snapshot order is stable
by name and label list for deterministic tests and exporters.

#### std/trace — new

Operation spans and ordinary parent provision. Reference: E `Tracer.ts`.
Keep needs visible. A tracer records completion without raising from cleanup.
No full Effect Exit observation or hidden propagation service is claimed.

```text
SpanContext = (trace_id = String, span_id = String)
ParentSpan = (value = Option(SpanContext))
Succeeded = (); Failed = (); Abandoned = (); SpanStatus = Succeeded | Failed | Abandoned
Span = (context = SpanContext, parent = Option(SpanContext), name = String,
        attributes = List((String,json.Json)), started = time.Duration,
        ended = Option(time.Duration), status = SpanStatus,
        cause = Option(cause.Cause(String)))
Tracer(N) = private service
make : (random.Generator,(Span) <- N -> Nil) <- (resource.Scope,Scheduler) -> Tracer(N)
root : () -> ParentSpan
with_span : (String,List((String,json.Json)),() <- (..N,ParentSpan,Fail(E)) -> A)
  <- (Tracer(N),ParentSpan,time.Clock,Scheduler,request.Await,request.Owner,
      ..N,Fail(E | time.TimeError)) -> A
```

make's recorder must have no Fail need; it may suspend. Its non-Fail services
are explicit. with_span derives IDs from its scoped generator, provides the new
ParentSpan to the body, catches typed E only to record diagnostic text, and
re-raises E unchanged. It records Succeeded or Failed on normal/error return.
A finalizer records Abandoned on escape or structured cancellation; it cannot
distinguish those without a language outcome API. A stop records no guaranteed
completion. An unavailable final clock sample is ended=None; it cannot turn
cleanup into a raised error. No correctness claim depends on real-time ordering.
HTTP wire propagation/export backends are later adapters over SpanContext.

#### std/cache — new

Bounded LRU, monotonic TTL and shared in-progress loads. Reference: E `Cache.ts`.
A scoped callback owns all loader jobs. Do not return a Cache from a short-lived
task group or let the first waiting caller own shared work.

```text
Cache(K,A,E) = private
Options = (capacity = Int, success_ttl = time.Duration, error_ttl = time.Duration)
CacheError = InvalidOptions | Closed | RecursiveLoad
InvalidOptions = (); Closed = (); RecursiveLoad = ()
scoped : ((K) <- (..N,Fail(E),Scheduler,Runner,request.Await) -> A,Options,
  (Cache(K,A,E)) <- (..N,Fail(F),Scheduler) -> R)
  <- (..N,time.Clock,Scheduler,request.Await,request.Owner,Fail(F | CacheError | time.TimeError)) -> R
get : (Cache(K,A,E),K) <- (Scheduler,Fail(E | CacheError | time.TimeError)) -> A
get_cached : (Cache(K,A,E),K) <- (Scheduler,Fail(CacheError | time.TimeError)) -> Option(Result(A,E))
set : (Cache(K,A,E),K,A) <- (Scheduler,Fail(CacheError | time.TimeError)) -> Nil
refresh : (Cache(K,A,E),K) <- (Scheduler,Fail(E | CacheError | time.TimeError)) -> A
invalidate : (Cache(K,A,E),K) <- (Scheduler) -> Nil
invalidate_all : (Cache(K,A,E)) <- (Scheduler) -> Nil
size : (Cache(K,A,E)) <- (Scheduler) -> Int
```

Capacity is positive; TTLs are nonnegative. TTL starts at completion and uses
the factory's captured monotonic clock. Error TTL zero does not retain a settled
failure, but concurrent waiters still share that attempt. The factory captures
loader non-Fail needs once; values cannot silently cross provider identities.
The caller's Fail remains live at each get. Loader recursion for the same key
fails explicitly rather than deadlocking. LRU uses access order, independent
of Dict traversal; expired/settled entries are eviction candidates.

In-flight keys count toward capacity. When all entries are loading, new-key
admission waits rather than allocating unbounded entries. Cancelling one waiter
removes that waiter; the shared loader stays owned by scoped's background group.
Invalidation advances a generation: existing waiters may get the old attempt,
but its late completion cannot repopulate the invalidated entry. refresh keeps
the old settled value until the new attempt succeeds. scoped uses the same
producer/consumer race pattern as stream helpers: callback return cancels loader
work and waits for cleanup. The cache handle never extends that lifetime.

#### std/batch — new logical batching

Combine application lookups without overloading i/request. Reference:
E `Request.ts`, `RequestResolver.ts`. Resolver inputs/answers are immutable
ordinary data, and per-item Result values represent caught outcomes.
No logical Request base class, dispatch DSL or compiler optimizer is added.

```text
Batcher(I,O,E) = private
Item(I) = (id = Int, input = I)
Answer(O,E) = (id = Int, outcome = Result(O,E))
Options = (max_batch = Int, max_pending = Int, flush_after = time.Duration)
BatchError = InvalidOptions | Closed | MissingAnswer | DuplicateAnswer | UnknownAnswer
InvalidOptions = (); Closed = (); MissingAnswer = (Int); DuplicateAnswer = (Int); UnknownAnswer = (Int)
scoped : ((List(Item(I))) <- (..N,Fail(E),Scheduler,Runner,request.Await) -> List(Answer(O,E)),
  Options,(Batcher(I,O,E)) <- (..N,Fail(F),Scheduler) -> R)
  <- (..N,time.Clock,Scheduler,request.Await,request.Owner,Fail(F | BatchError | time.TimeError)) -> R
submit : (Batcher(I,O,E),I) <- (Scheduler,Fail(E | BatchError | time.TimeError)) -> O
flush : (Batcher(I,O,E)) <- (Scheduler,Fail(BatchError | time.TimeError)) -> Nil
pending : (Batcher(I,O,E)) <- (Scheduler) -> Int
```

Dispatch FIFO on max_batch or the first queued item's deadline. flush starts
all current batches, but does not claim their answers have completed. Pending
admission is bounded by max_pending and waits at capacity. Different grouping
keys use different explicit Batcher values, constructed at the owner scope.
Missing/duplicate/unknown answer IDs fail every affected waiter with BatchError;
none remain asleep. Resolver failure E is shared by its current batch. Cancelling
one waiter does not cancel a dispatched batch needed by others. Scoped return
cancels dispatch/loads and waits for cleanup. Compose cache's loader with submit
when caching is wanted; there is no second cache implementation in batch.

#### std/kv — new

A small ordinary key/value service, schema views and explicit persistence.
Reference: E `persistence/KeyValueStore.ts`. Memory and file providers share
one contract; neither imports a hidden global store.

```text
Store(N) = (get = (String) <- (..N,Fail(KvError)) -> Option(BitArray),
            set = (String,BitArray) <- (..N,Fail(KvError)) -> Nil,
            remove = (String) <- (..N,Fail(KvError)) -> Nil,
            keys = () <- (..N,Fail(KvError)) -> List(String))
KvError = (operation = String, key = String, reason = KvProblem)
KvProblem = InvalidKey | FileFailure | IoFailure | InvalidValue
InvalidKey = (); FileFailure = (ifs.FsError); IoFailure = (io.IoError)
InvalidValue = (String)
memory : () <- (resource.Scope,Scheduler) -> Store(Scheduler)
file_store : (String,max_bytes = Int)
  <- (fs.FileSystem(N),..N,resource.Scope,Scheduler,Fail(KvError)) -> Store((N,Scheduler))
get : (String) <- (Store(N),..N,Fail(KvError)) -> Option(BitArray)
set : (String,BitArray) <- (Store(N),..N,Fail(KvError)) -> Nil
remove : (String) <- (Store(N),..N,Fail(KvError)) -> Nil
keys : () <- (Store(N),..N,Fail(KvError)) -> List(String)
prefix : (Store(N),String) -> Store(N)
get_schema : (String,schema.Schema(A,E,N,W))
  <- (Store(N),..N,Fail(KvError | schema.SchemaError | json.JsonError | unicode.UnicodeError | E)) -> Option(A)
set_schema : (String,A,schema.Schema(A,E,D,N))
  <- (Store(N),..N,Fail(KvError | schema.SchemaError | E)) -> Nil
```

Only a missing key yields None. A corrupt stored value raises its typed decode
error. Schema views share JSON/UTF-8 implementations. prefix changes the key
namespace and strips its prefix in keys. Memory uses one Ref; providers return
lexically sorted key lists. File keys use reversible UTF-8 hex filenames so
keys cannot become paths. Empty/NUL-containing keys fail InvalidKey.

File set replaces via a flushed intermediate file and rename, serialized within
this Store instance. The backend must document whether rename replacement is
atomic on its target. No cross-instance compare-and-swap, multi-key transaction,
crash durability or secure exclusive temporary-file guarantee is promised.
The store directory is explicitly owned by this service; do not mix unrelated
writers into it. Unsupported backend guarantees cannot be inferred from Dict.

#### std/property

Pure generators, lazy shrink trees and reproducible reports. Reference:
E `Arbitrary.ts`; Go `testing/testing.go`. Keep all listed operations and
representations. BuildError→PropertyError; builders raise it. property.Fail→
Falsify avoids confusion with the Fail service. Verdict/Report are intentional
data, including InvalidConfig. Allow native needs in check/test callbacks;
generation and shrinking stay pure and bounded. No arbitrary IO is replayed
as if a single seed controlled external arrival order.

```text
Gen(A) = private
Tree(A) = (value = A, children = () -> List(Tree(A)))
Config = (seed = Int, cases = Int, max_size = Int, max_discards = Int, max_shrinks = Int)
PropertyError = InvalidRange | InvalidLength | EmptyChoice
InvalidRange = (Int,Int); InvalidLength = (Int,Int); EmptyChoice = ()
Pass = (); Falsify = (String); Discard = (); Verdict = Pass | Falsify | Discard
Failure(A) = (seed = Int, attempt = Int, size = Int, original = A, smallest = A,
              reason = String, path = List(Int), checked = Int, limited = Bool)
Passed = (successes = Int, discards = Int)
GaveUp = (successes = Int, discards = Int)
Falsified(A) = (Failure(A)); InvalidConfig = (String)
Report(A) = Passed | GaveUp | Falsified(A) | InvalidConfig
defaults : () -> Config
make : ((Int,random.Generator) -> Option(Tree(A))) -> Gen(A)
sample : (Gen(A),Int,random.Generator) -> Option(Tree(A))
tree : (A,() -> List(Tree(A))) -> Tree(A)
constant : (A) -> Gen(A)
map : (Gen(A),(A) -> B) -> Gen(B)
flat_map : (Gen(A),(A) -> Gen(B)) -> Gen(B)
pair : (Gen(A),Gen(B)) -> Gen((A,B))
filter : (Gen(A),(A) -> Bool) -> Gen(A)
sized : ((Int) -> Gen(A)) -> Gen(A)
resize : (Gen(A),Int) -> Gen(A)
int : (Int,Int) <- (Fail(PropertyError)) -> Gen(Int)
bool : () -> Gen(Bool)
element : (List(A)) <- (Fail(PropertyError)) -> Gen(A)
list : (Gen(A),Int,Int) <- (Fail(PropertyError)) -> Gen(List(A))
float : () -> Gen(Float)
utf_codepoint : () -> Gen(UtfCodepoint)
string : (Int) <- (Fail(PropertyError)) -> Gen(String)
check : (Gen(A),Config,(A) <- N -> Verdict) <- N -> Report(A)
replay : (Gen(A),Int,Int,Int,List(Int)) -> Option(A)
format : (Report(A),(A) -> String) -> String
test : (Gen(A),Config,(A) <- (..N,Assert) -> Nil,(A) -> String) <- (..N,Assert) -> Nil
```

check repeats a callback while shrinking. Effectful properties must install
fresh scoped test state per trial, explicitly. Fail in a property propagates
unless the property catches it and returns Falsify as data. The assertion
adapter catches Assert only and does not swallow typed failure. A shrink result
is bounded/local, never advertised as the globally smallest counterexample.
Fix the embedded Python Config probe to labelled construction.

#### std/test and std/test_services

Reference: Go `testing/example.go`, `testing/testing.go`; E
`testing/TestClock.ts`, `testing/TestConsole.ts`, `testing/TestSchema.ts`,
`packages/vitest/src/index.ts` (the last path relative to effect/).
Keep ordinary test macros and migrate quoted suite provision to `<-`.
Do not create a special test keyword, test Layer mechanism or second runner.

```text
// std/test (test.is)
tests : Nil
test, skip, only, fails : (Syntax,Syntax) <- (syntax.Items,Fail(syntax.SyntaxError)) -> Syntax
suite : (Syntax,Syntax) -> Syntax
each : (Syntax,Syntax,Syntax) <- (syntax.Items,Fail(syntax.SyntaxError)) -> Syntax
// std/test_services, new: recorders are immutable handles with scheduler-backed state.
Console = (input = io.Reader(Never,Scheduler), stdout = io.Writer(Never,Scheduler),
           stderr = io.Writer(Never,Scheduler), output = () <- (Scheduler) -> BitArray,
           errors = () <- (Scheduler) -> BitArray)
LogRecorder = (logger = log.Logger(Never,Scheduler), entries = () <- (Scheduler) -> List(log.Entry))
TraceRecorder = (tracer = trace.Tracer(Scheduler), spans = () <- (Scheduler) -> List(trace.Span))
console : (BitArray) <- (resource.Scope,Scheduler) -> Console
logs : () <- (resource.Scope,Scheduler) -> LogRecorder
traces : (random.Generator) <- (resource.Scope,Scheduler) -> TraceRecorder
round_trip : (A,schema.Schema(A,E,N,N)) <- (..N,Assert,Fail(E | schema.SchemaError)) -> Nil
```

Raw Stdio request fields cannot call Scheduler. Console exposes the
high-level std/io reader/writer contract so stateful recording remains ordinary
scheduler-owned state. Use fixed ready-reply Stdio fakes for raw adapter tests;
use Console for stateful common-IO tests. Never run task effects inside a closed
pure host Request field.

Reuse fs.memory for filesystem fakes, config.memory for config, metric.registry
for metric snapshots, i/random.from_seed for random, and test_clock for time.
Do not add aliases for those factories in test_services. For HTTP, construct the public Client service value directly.
Log/trace recorders add observation, so they earn functions. round_trip encodes,
decodes, and asserts language equality; representation-sensitive types use an
explicit projection property rather than a false universal law.

### 3.12 Deliberate boundaries and coverage ledger

Every current module has a disposition in §§3.2–3.11. The list below is a
cross-check against all 22 i modules and all 27 `.i` std modules plus test.is.
It is not an additional alias surface.

| Current family | Coverage and final home |
|---|---|
| assertion, big, bits, clock, control, core, fail, failure, fetch, float_text | Retained/reconciled in i, except failure deleted into fail; §§3.2, 3.6, 3.9. |
| fs, io, key, net, process, random, request, resource, stdio, syntax, task, tls | Minimal/spec support in i; fs convenience algorithms move to std/fs; §§3.2, 3.5, 3.6, 3.9. |
| option, result, list, dict, set, sequence, order, pair, bool | Existing data families retained, duplicate syntax/bridges removed; §3.3. |
| int, float, string, bit_array, crypto | Existing algorithms retained and failures normalized; §3.4. bytes/unicode/decimal add distinct capabilities. |
| calendar, time | Existing pure units/civil algorithms retained; §3.6. Clock/test_clock/random/schedule/concurrent complete policy paths. |
| json, schema, csv, regex, uri, cli | Existing algorithms retained, operational Result removed, one source of representations; §3.7. config reuses these. |
| stream | Replayable sequential base plus explicit scoped/concurrent consumption; §3.8. sink/io add shared consumer and byte contracts. |
| path | Pure lexical path retained; §3.9. New fs/process/terminal modules are high-level ordinary libraries. |
| http | Data/framing kept in http; clients and servers move into http_client/http_server; §3.10. |
| log, property, test.is | Retained and extended; §3.11. cause/metric/trace/cache/batch/kv/test_services add the missing application/test capabilities. |

These extension boundaries have no phantom exports or eight-hour estimates:

| Capability | Home when implemented; decision fixed now |
|---|---|
| Richer filesystem metadata, seek, links/permissions, exclusive temporary creation, watch | Extend i/fs host contract with explicit target support, then std/fs. Do not fake it with lexical checks or invent a platform capability. |
| Raw terminal, size and key events | Optional i/terminal capability plus std/terminal policy; scope restores mode. Byte stdio stays available. |
| Windows/native paths, Unicode normalization/graphemes/UTF-16 | Separate explicit path-style and unicode operations, with data versions. Existing scalar/POSIX meanings remain fixed. |
| Named timezone data, gap/fold policy, cron | calendar/timezone data extension then schedule cron. Fixed offsets and monotonic policy ship first. |
| WebSocket, Unix sockets, HTTP2, cookies/multipart/proxies/compression | Extend the corresponding protocol/transport modules, with scoped adapters and streaming tests. HTTP/1.1 and existing TCP/UDP/TLS are the release contract here. |
| Keyed job replacement, synchronized effectful refs, transactions | First use FiberSet/owner tasks and one aggregate Ref. New semantics need a separate design; no mutable public object or automatic replay of IO. |
| XML/ASN.1/PEM/base32/ASCII85/native binary schema | Independent codecs over bytes/io/schema when a concrete format is required. No gob-style private-runtime serialization. |
| Metric/tracing exporters and propagation | Ordinary bounded, scoped adapters over snapshots/spans and HTTP functions; explicit delivery/error/overflow policy. |
| External workers | Explicit scoped transport only for interop; normal parallel execution remains task scheduling with unobservable worker placement. |

The completion claim is the complete surface and application paths specified
here. It is not a claim that the current tree already implements them, nor that
all specialized capabilities in the reference packages can be built today.

## 4. Stability policy and its machine check

### 4.1 Marks and promises

Use ordinary module/item doc metadata. These lines add no language attribute:

```text
Stability: stable. Since: 1.0.0.
Stability: experimental. Since: 1.0.0. Tracking: docs/design/std-v3.md#37-formats-schema-and-configuration--lane-f.
```

A module supplies its default. Every export inherits it or has one explicit
override. Missing/contradictory marks fail CI. New or changed items remain
experimental until reviewed. A stable signature's complete reachable public
closure must be stable: types, aliases, visible constructors, all shape branches,
callback types, error cases, service fields and host contracts. Imports alone
do not make an unrelated experimental helper part of that closure.

Stable means correct existing clients keep both their types and documented
behavior throughout the package major. It includes argument order, labels,
shape selection, needs, errors, lazy/eager timing, units, encoding, callback
order, ownership, cancellation and stated target support. It excludes private
representation, unspecified iteration order, exact scheduling choices and
undocumented performance. A new error case is not a compatible addition to a
stable closed sum. No version-zero or experimental exception overrides spec §2.7.

| Change | Release rule |
|---|---|
| Internal fix preserving documented interface/behavior | Patch with regression evidence. |
| Independent export | At least minor; check import conflicts and dependencies. |
| Changed/removed value type, need, error, field, constructor visibility, parameter order/label/shape | Major under the existing conservative comparator. |
| Changed shape selection, accepted input, wire format, equality observation, demand or cleanup | Behavioral review; major when documented clients can break, even if type strings are equal. |
| Deprecation | Existing `@deprecated("use …; migration …")`, working replacement and migration example. Keep the old export throughout its stable major. Publish replacement/warning at least one minor before later-major removal. |
| Security fix requiring a documented break | Explicit release decision, advisory and migration. Never call it an ordinary compatible patch. |

The first migration changes many public heads; choose the release major before
baselining. Do not mark the whole library stable just because Effect's package
is 4.1.3. Initial candidates after gates: option/result/order/pair/bool, list's
retained native-need traversal, and validated pure scalar/codec contracts.
Dict/Set/Sequence remain experimental while representation equality is reviewed.
New lifetime, clock, concurrent, schema-description, platform and application
services start experimental until their full composition examples pass.

### 4.2 One semantic comparison engine

Current machinery is `interface_of`, `earlier_interface`, `compare_with`,
`Diff::new` and `Diff::version_step` in
`crates/ilang-cli/src/commands/package.rs`. It already compares exported values,
type identities, constructors/fields, aliases and required host namespaces.
It normalizes type variables and proves supported semantic scheme equivalence.
Proof-budget exhaustion fails; uncertain representation change requires major.
It does not read maturity comments or prove behavior. The current
`std/test/std3_interfaces.py` checks only 35 selected type strings; it is not
an all-library compatibility check and must not become another type-diff engine.

Lane L exposes a read-only package command, decided CLI spelling for L:

```sh
i interface --project std --against vMAJOR.MINOR.PATCH --version NEXT --json
```

The command calls the same extraction/comparison functions as publish. It
creates no tag, edits no source/lock file and never fetches a moving baseline.
The command name is tooling work to implement, not a command available now.
Expose it as a PackageCommand variant in package.rs; main's existing flattened
package subcommands need no second dispatch engine. A library-mode option
extracts the shipped i/ exports and host contracts through the same code;
compiler release baselines cover i/ separately from the std package major.

Baseline is the newest published version tag for the package, pinned to its
commit in the CI report. The migration's first checked baseline is pinned too.
Do not compare only HEAD~1: several individually additive commits may combine
to break an older supported client. Do not silently skip an unreadable old
interface. Current earlier_interface has a pre-A3-syntax path that prints a
manual-review message and returns no baseline. CI treats that as blocked until
a recorded migration review and checked baseline exist; green means compared.
Absence of the package at an old tag is a distinct first-publication condition.

### 4.3 Generated list, behavior notes and gates

`docs/design/std-v3-api.json` is generated from checked exports, not parsed
signature prose. It records format/compiler version, package and baseline
commit, module path, qualified nominal identity, public name, canonical scheme,
all shapes/labels, visible constructors/fields, stability/since/tracking,
reachable dependencies, required host contracts/targets, and doc-example/test
links. Normalized hidden representations are recorded only as needed by the
existing comparator, never exposed as public constructors.

`docs/design/std-v3-behavior.md` lists reviewed contracts and changes by item:
parser acceptance/duplicates, numeric rounding, Unicode version/offset units,
traversal order, callback timing, queue terminal/overflow, stream demand,
resource handoff, cancellation, target availability and wire encodings.
A changed stable implementation needs either a contract-preserving explanation
or a reviewed behavioral change note with tests. This is review evidence, not
a claim that a script can prove semantic equivalence.

`scripts/std-api-check` runs the following in order and fails closed:

1. Check std and shipped i/; generate the current interface and maturity list.
   Reject missing docs, missing example links, focus-only tests and invalid marks.
2. Reuse the semantic comparator against the pinned release tag, enforce the
   minimum version step, and report removals/changes/additions by item. No tag
   is created and no string comparison substitutes for type identity.
3. Reject a stable export that reaches an experimental contract, or silently
   loses stable status. Check public constructors and deferred callback needs.
   Require replacement/migration notes for a new deprecation.
4. Check reviewed behavior notes and run linked examples/laws. Compile/run
   representative clients from the previous supported release. Those clients
   exercise service provision and resource lifetimes, not just import names.
5. Run the target matrix below and existing release checks. Fail an unexpected
   provider/foreign namespace change or a skipped compatibility baseline.

| Target | Required release evidence |
|---|---|
| native | All four K programs, loopback HTTP, true streaming, IO cancellation/cleanup, existing native host tests. Explicit native client/FileSystem/Spawner/time.Clock choices in main. |
| nodejs | Pure library and provided stdio/fs/process/network paths from the platform table. HTTP-only tcp_client where Tls is absent; no hidden TLS installation. |
| wasi | Pure library, preopened filesystem and byte stdio paths. No Net/Process/WorkingDirectory promise where the platform table omits them. |
| js | Pure library, clock/entropy and explicit fetch-adapter tests only when the program supplies Fetch. No invented platform Fs/Net/Stdio provider. |
| host | Contract fakes and required-import metadata; the embedder explicitly registers services. |

The stable public closure cannot depend on an experimental fake merely because
its test does. Test-only dependencies are outside the released public interface.
Capability availability is checked after the application's explicit provision;
catching every error does not satisfy a missing service.

Final existing release gate remains `i publish NEXT --dry-run --project std`
with the intended major. It checks a clean committed tree, version progression,
package/tests and interface compatibility. It creates no tag in dry-run mode.
The first actual publication/tagging is separate release work; this synthesis
neither publishes nor changes CI. L's read-only comparison and metadata gates
must pass before any item is promoted from experimental.

## 5. Work breakdown and acceptance

### 5.1 Eight-hour execution window

Use ten implementation workers, one per lane A–J. Worker eleven does examples
K for hours 0–4 and stability CI L for hours 4–7. Hour 7–8 is integration and
regression reserve for everyone. Thus twelve file-disjoint lanes use eleven
workers. Estimates below include docs and tests; they are focused worker-hours,
not a claim that every future reference capability fits in one day.

Freeze the contracts below at hour 0. Each lane publishes its exported heads
and test doubles by hour 1, then implements bodies. Consumers may write against
those exact heads before the provider merges. Merge usable foundations by hour
2, policy/codec slices by hour 4, application paths by hour 6. Test after each
merge. No lane waits until hour 7 to reveal an interface mismatch.

The hard prerequisite is the syntax-three E3/E5 language baseline: ordinary
Fail, row-only control types, Group, explicit entries and Scope provisioning.
The audits and this worktree still show migration residue. The standard-library
budget does not hide a compiler migration. A owns library-side reconciliation;
the existing syntax migration owner must deliver compiler support in parallel.
If that baseline or the ordinary primitive-linking gate is unavailable by hour 2, report the deadline risk immediately.
Do not restore provider plans or a second error mechanism to make a build pass.

File sets below are exclusive. A lane owns its current and destination files,
its matching `std/test/<name>_test.i`, and a matching new
`std/test/examples/<name>_examples.i`. Existing multi-module test files belong
to the lane explicitly listed. Only that owner edits them. Consumers report
needed changes to the owner. Use one isolated branch/worktree per lane. Merge
complete slices into the integration branch at the stated checkpoints; no lane
rebases or commits another lane's pending work. No lane edits shared spec,
workflow or compiler files unless explicitly assigned. K owns application
callers and example manifests. L alone edits std/i.toml, shared release lists
and CI. Existing unrelated examples are
migrated by K, never by module lanes.

Existing integration files have the same exclusive ownership:
A owns `crates/ilang-cli/tests/{big_int,float_text,syntax_runtime}.rs`;
B owns `tests/spec/9.9/**`, `tests/spec/9/task_ref.i`, and
`crates/ilang-cli/tests/{determinism,multicore,r2_runtime}.rs`;
H owns `crates/ilang-cli/tests/{tls,udp,native,wasi}.rs`;
I owns `crates/ilang-cli/tests/http.rs`;
J owns `crates/ilang-cli/tests/{cli_log,many_tests,macro_run}.rs`.
L owns `crates/ilang-check/tests/types/library.out`, `tests/r2-{scale.py,interface.md}`,
`tests/review/cases/**/expected.json`, and any other library-consumer golden
update under `crates/*/tests` not assigned above. Those are mechanical consumer
updates only; L does not alter compiler semantics to accept an API. Other spec
fixture migrations belong to the prerequisite syntax baseline. No module lane
regenerates L's snapshots or edits another lane's integration file.

### 5.2 Lanes

| Lane / budget | Exclusive source files and modules | Order and assumed cross-lane interfaces | Required tests and examples |
|---|---|---|---|
| A. Native effects and lifetime; 6 h | `lib/i/{fail,failure,assertion,control,resource,key,request,syntax,core,big,bits,float_text}.i`; `tests/spec/9/{keep_env,control_lazy_env,control_lazy_env_unwind}.i`; `docs/spec.md` library API/location clauses only, coordinated with the syntax owner; new `std/test/{fail,resource,request,syntax,key}_test.i` | Consolidate fail → reconcile row-only control → resource Scope/scoped/acquire → request handoff → syntax docs/hooks review. Exact primitive contracts are spec §§9.3–9.5, 9.8, 9.11 and syntax-three §2.10.7–9. No dependency on std. Coordinate E3/E5 compiler work with its existing owner. | Catch separation, narrowed rethrow, outlived handler rejection, reverse cleanup, nested/empty scope, cancel between acquisition and registration, unclaimed Ready release, no finalizer Fail. Preserve existing primitive regression suites. |
| B. Task coordination; 7 h | `lib/i/task.i`; new `std/src/{deferred,semaphore,queue,pubsub,latch,fiber_set}.i`; matching new tests | Group rename/ref snapshots/change events/quiescence and bounded traversal → Deferred → Semaphore → bounded Queue → PubSub/Latch → scoped FiberSet. Assume A's Scope and ordinary Fail. Use task events and owner loops; no public scheduler protocol extension. Publish capacity/termination heads first. | FIFO/backpressure, losing select consumes nothing, cancelled waiter removal, exactly-once completion, permit handoff, scope closure, O(limit) live traversal children, no leaked handles. Existing task Rust/spec fixtures run unchanged apart from Group naming owned through integration. |
| C. Time and policy; 6 h | `lib/i/{clock,random}.i`; `std/src/{time,calendar,random,schedule,concurrent,test_clock}.i`; `std/test/{clock,time,calendar}_test.i` and new matching tests | Keep exact Duration/Instant → task-level Clock service and virtual sleepers → timeout → schedule/retry/repeat → random test helpers. Assume B's Ref/channel/scopes and A's resource lifetime. Existing i/clock remains raw adapter. | Quiescent virtual time, sleepers registered after yields, cancellation removes sleeps, wall jumps do not affect duration, timeout waits for cleanup, exact attempt/delay vectors, bounded seeded jitter, seed/path replay. No real sleeping in policy unit tests. |
| D. Collections and data; 6 h | `std/src/{option,result,list,dict,set,sequence,order,pair,bool}.i`; their current tests | Delete duplicate bridges → normalize names/shapes → preserve need propagation → content comparison laws. Assume core equality/hash from A, unchanged. Preserve collection algorithms; no canonical-tree project today. | Existing collection regression/property examples; different insertion/tree histories, hash collisions, stable sorting, empty lookup, effectful callbacks and first failure. Show explicit conversion to list for value keys. |
| E. Numbers, text and binary data; 7 h | `std/src/{int,float,string,bit_array,bytes,unicode,decimal,crypto}.i`; matching tests plus `string_additions_test.i`; Unicode tables currently included by regex move only through F's coordinated deletion | Typed numeric/byte errors → subject-first codecs → shared Unicode data → incremental UTF-8 → byte assembly/search → small normalized Decimal. Assume A's core/entropy boundary; D's Order/List. Keep i/float_text algorithm. | Existing vectors and float-text regressions; invalid radix/domain/UTF-8 offsets; every UTF-8 split; whole-byte enforcement; byte search; Decimal normalization/rounding; no new timing-security claim. |
| F. Formats, schema and configuration; 7 h | `std/src/{json,schema,csv,regex,uri,cli,config}.i`; matching current/new tests | Fail migration and argument order → retain parser algorithms → schema description + decode/encode/validate → config provider precedence. Assume E's UTF-8 and numeric errors, D's collections, H's environment/file adapters; describe schema metadata before consumers use it. | Round trips, duplicate JSON fields, unknown/missing/null distinctions, encode validation, unsupported custom derivation, CSV chunk boundaries and Fail, regex zero-width demand, URI byte offsets, CLI fake args, config missing/invalid/source-error precedence. |
| G. Stream, sink and common IO; 7 h | `std/src/{stream,sink,io}.i`; their tests | Freeze Reader/Writer → scoped run → incremental adapters → Sink leftovers → bounded map/merge/buffer and time operators. Assume A Scope, B queues/events, C clock and E UTF-8. No std import in lib/i adapters: high-level adapters live here. | Pure replay/demand; scoped single-consumption IO; early take closes; decoder Fail/cancel closes; bounded memory and producers; leftovers are not read twice; every chunk boundary; short/zero-progress writes and exact-read EOF. |
| H. Host IO and tools; 7 h | `lib/i/{fs,process,net,tls,stdio,io,fetch}.i`; `std/src/{fs,path,process,terminal}.i`; matching tests including `stdio_test.i`; existing IO example tests stay K-owned | Preserve raw ABI and creation guards → scoped high-level handles/adapters → bounded file streams/copy/walk → immutable commands with concurrent output drains → cooked terminal. Assume A Scope, G Reader/Writer/source contract, C timeout, F typed env/config errors. No new host imports in this day. | File cleanup at every handoff; injected read/write/flush errors; TLS option behavior; UDP empty packets; resolver/handshake cancel; child termination+wait and both pipe drains; bounded walk/link behavior; native/node/wasi capability differences. |
| I. HTTP; 7 h | `std/src/{http,http_client,http_server}.i`; `std/test/{http,http_network}_test.i`; new client/server tests | Pure messages/framing → scoped Client factory → true native/fetch response pulls → streaming server response + bounded admission → generic timeout/retry adapters. Assume A Scope, B admission, C policies, F codecs, G IO, H net/TLS/fetch. | A response chunk arrives before body completion; early-close at every stage; pool survives body use; size limits; redirects never replay unsafe bodies; cross-origin sensitive headers; framing, request/response streams, cancellation waits for handlers. |
| J. Application services and testing; 7 h | `std/src/{cause,log,metric,trace,cache,batch,kv,property,test_services}.i`, `std/src/test.is`; matching tests, `property_failures.py` | Cause data/test macros/properties and recorders → contextual log/metric/trace → scoped single-flight cache → batch dispatcher → memory/file KV. Assume A Scope, B Deferred/Queue, C monotonic clock, D Dict, F schema/config, H files. Start recorders/cache state machines before dependencies merge. | Lazy filtered logs, output failure policy, deterministic metric/span records, early/cancelled spans, cache capacity/TTL/invalidation/load races, cancellation of one shared waiter, partial/missing batch answers, KV round trips, property seed/shrink replay and suite `<-` handling. |
| K. Programs and caller migration; 4 h, worker 11 first | All `examples/**` files that import affected modules; four new packages `examples/std-v3-{http,files,crawl,config}/**`; acceptance guide `docs/examples/std-v3.md` | Start with fakes against frozen heads. HTTP retry+timeout → scoped streaming file CSV processing → crawl with input-order bounded results → schema-validated config. Assume contracts A–J; module owners never edit these callers. | Each package runs and has a fake-service test. HTTP: transient failure then success and deadline cleanup. Files: early stop, malformed UTF-8/CSV, no leak. Crawl: measured max in-flight ≤ limit, fail/cancel cleanup. Config: env overrides file only when present, invalid override fails, secret rendered redacted. |
| L. Stability list and CI; 3 h, worker 11 second | `std/i.toml`; `docs/design/std-v3-api.json`; `docs/design/std-v3-behavior.md`; `scripts/std-api-check`; `std/test/std3_interfaces.py`; CI workflow files under `.github/workflows/`; `crates/ilang-cli/src/commands/package.rs` and its `package_commands.rs` tests only for read-only comparator exposure | Extract final heads → metadata/dependency closure → read-only existing semantic comparator → tag and compiled-client gate. Assume all lanes' item docs and example links; coordinate compiler owner before touching package.rs. No handwritten second type-diff engine. | Deliberate removal/need/error/constructor/shape/host-namespace changes fail; independent export requires minor; dropped stable mark fails; experimental reachability fails; pre-A3 baseline skip is a hard review gate; examples compile. |

B and J are the largest new-code lanes. Deliver their minimal complete
contracts first: bounded FIFO queue, one-shot Deferred, fixed Semaphore;
then PubSub/Latch and cache/batch. Exclude resizing semaphores, replaying hubs,
key replacement, transactions and exporter ecosystems from today's work.
Do not substitute a semaphore around an unbounded spawn-per-input traversal.

### 5.3 Interfaces frozen at hour zero

| Producer → consumers | Contract consumers may assume |
|---|---|
| A → all | `fail.raise/catch/from_result/map_error` exactly spec §9.3; `resource.ensure/acquire/scoped` exactly §9.5. Resource Scope is different from task Group. Release callbacks cannot raise. |
| B → C/G/I/J | Existing task event and Ref contracts plus snapshot/changed notifications and quiesce; bounded queue with FIFO, explicit end after draining, failure terminal; Deferred many-read exactly-once completion; Semaphore bracket releases once on all structured exits. No arbitrary per-task interrupt. |
| C → G/I/J/K | Signed exact Duration; distinct Unix Instant; monotonic deadline arithmetic; provided task-level Clock with `now`, `monotonic`, `sleep`; deterministic `test_clock.advance`; timeout returns after loser cleanup; retries count retries after initial attempt. |
| E → F/G/H | BitArray stays the language type. `bytes` requires alignment. UTF-8 decoder keeps partial codepoints between chunks and reports byte offsets. No Unicode index reinterpretation. |
| F → H/I/J/K | JSON retains tagged scalars and ordered object pairs. Schema decode/encode both raise SchemaError; validate returns accumulated issues. ConfigProvider read returns Option(Json), source failure raises ConfigError. |
| G → H/I/K | Reader.read positive bound → Option(BitArray), None = EOF; Some is nonempty and within bound. Writer.write accepts immutable bytes and returns accepted prefix count; io.write_all handles short/zero progress. Partial external effects may precede failure. Scope encloses the entire use. Sink returns result and remaining Stream. |
| H → I/J/K | Existing raw ABI and request ownership remain. High-level file/socket/child acquisition registers release in Scope before exposing handle. No fabricated support on targets without the host service. |
| I → K | Explicit scoped Client; buffered send is built on streaming send plus bounded collection; native streaming reads live connection, not a buffered whole response. |
| J → all/K/L | Test recorder values are ordinary provided services. Cache captures loader non-Fail needs deliberately, owns work in Scope, and shares in-flight outcomes. `std/test` remains ordinary item macros. |

### 5.4 Integration and size control

At 07:00–08:00 Hong Kong time, freeze heads, allocate the files, and publish
fakes. At 09:00 merge A and each lane's smallest usable slice. At 11:00 run the
four programs with fakes. At 13:00 require true HTTP streaming and native IO
cleanup tests. At 14:00 freeze implementations; finish CI, examples and target
checks. Merge to main by 15:00 only after the acceptance gates pass. If work
starts at another time, preserve the same eight-hour offsets and the final
hour of reserve. Do not treat a failing slice as complete to hit the clock.

The ten estimates total 67 worker-hours; K/L add 7. The nominal capacity is
88 worker-hours. The difference is integration and uneven-lane reserve, not
permission to add host subsystems. New capability families are experimental;
complete behavior and executable examples are required even for experimental
items. Existing 23,979 source lines and 2,170 named std tests are migration
assets, not work to rewrite from scratch or proof that new behavior works.

No new native ABI operations, timezone database, XML/ASN.1/gob, interactive
raw terminal, filesystem watcher, WebSocket/HTTP2 stack, transaction journal,
or external worker transport is included in the eight-hour estimate. They
are explicit extension boundaries in §3. A complete first release supplies
all requested everyday paths; reference breadth is not a promise to reproduce
every specialized upstream export today.

### 5.5 Owner decisions only

| Decision | Default proposed here; decision needed before |
|---|---|
| Which commit supplies the E3/E5 compiler baseline? | Name the syntax migration integration commit and its owner at hour 0. Without it the eight-hour std integration promise is conditional. |
| Which package major is the migration release? | Use a new major if any existing published interface breaks. Experimental does not waive spec §2.7. Decide before L creates the baseline. |
| Which targets must pass today? | Native full path; nodejs and wasi their available IO paths; js pure data and fetch only when explicitly supplied; host contract fakes. Approve the matrix at hour 0. |

API naming, error style, resource lifetime, default provision and lane ownership
are decided here. They need no further owner polling. A failed critical gate
is reported with its exact test and impact. Scope cuts require an explicit
release decision; do not silently replace resource safety with documentation.


### 5.6 Checks on this synthesis

The catalogue accounts for all 50 current module source files. Every inventoried
public function spelling appears as a retained item or an explicit migration.
Fences, section presence and whitespace were checked. This is coverage of the
plan, not proof that the proposed implementation is correct.

The five ordinary §2 code blocks type-check with this worktree's checker and
library sources, which match main's language/library baseline `180e00cf7`.
Their assertion/printing entries also ran successfully with the available
syntax-three CLI. The sixth block expanded and passed as an ordinary test
(1 passed, 0 failed) with that CLI. It is the local `convfix` executable, not a
new binary built from this branch: a fresh full CLI build encountered an
uninitialized Binaryen submodule. No implementation or compatibility test
result is inferred from these documentation checks. Only this design file is
changed and committed by the synthesis.
