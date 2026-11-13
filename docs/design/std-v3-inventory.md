# Standard library v3: inventory and source audit

Part A. Phase 1. Read-only audit on `lane/stdA`.
Source baseline: `68b720e9ebc0d643a3a7831a32a1b97c7ab3c915`.

## Scope and evidence

This report inventories every module in `lib/i/*.i` and `std/src/*.i`.
That is 22 `i/` modules and 27 `std/` modules. It also includes
`std/src/test.is`, the Lisp-surface test module. The report changes no code.
No build, test, formatter, compiler type query or benchmark was run.

The language authority is `docs/spec.md`. The accepted migration rules are
`docs/design/syntax-3.md`, especially §§2.8, 2.10 and 7.2 E3–E6.
Errors below are services, `Fail(E)`. Existing `Result` return types are
recorded as inventory facts. They are not recommendations to preserve them.
E4, not E3, changes these public APIs. E4 also adds resource `Scope`.
E5 renames `task.Scope(E)` to `task.Group(E)` and replaces provider plans.
These stages must not be silently counted as already present.

A computation is an ordinary function `(P) <- (S) -> R`. Its typed failure is
part of S. Services are ordinary values supplied with `<-`.
Effect-style capabilities should use those language features. They do not
need a second Effect, Context, Layer, error channel or mutable collection.
Tasks and event selection already have their own structured semantics.

### How to read the API inventories

Each module section cites its source. Type definitions are copied from that
source with whitespace compressed. Hidden constructors are still shown to
explain representation; the export-visibility line says which can be used.
`T(..)` expands a union export to its member types and constructors. Expanded
member definitions appear too. Built-ins such as Int, Result, Request, Key,
Boundary and Continuation are not new module exports just because a signature
uses them. `lib/prelude.i` is outside this directory inventory.

Function types for `i/` come from the checked-in
`crates/ilang-check/tests/types/library.out`, filtered by the explicit export
list. That file includes private helpers; they are not public API.
It is evidence from this checkout, not a new verification run. It still has
transitional Boundary/Continuation E parameters and old union notation in
some need rows. Those parameters are removed by E3; they must not become a
second error mechanism. The E3 control types are listed in the migration
findings below.

For `std/`, types come from the written source heads. Parameter binder names
are removed. Immediate callback needs are included, as required by spec §9.2
and syntax-3.md §2.10.3 R2. Returning a deferred function or stream does not
execute its callback. Source-derived types for unannotated clock/IO adapters
are reconstructed from their calls and service fields, not from compiler
output. Free type letters are implicitly universal. Module-qualified imports
retain the local source aliases. `request` in time/log/cli type explanations
means `i/request`; `clock` means `i/clock`.

HTTP server signatures are schematic source constraints. They use the local
abbreviation `BodyReader` for the closed `(read = ..., trailers = ...)` data
made in `connection_loop`. Its read callback returns BitArray, needs Net,
Await, Owner and Scheduler, and can raise HTTP framing or network errors.
Its trailers callback needs Scheduler and returns Headers. The recursive
server has no normal completion path. A fresh principal checker type for its
recursive return and all inferred row differences is unavailable without a
compiler query; none was run. The listed Never result describes that control
flow, not a claimed snapshot of inference. `ClientFunctions` below is the
closed stream/send/close data in the source, with the same field types as
Transport. It is a source name, not an explicit export.

### Counting tests, exports and documentation

A test count is the number of literal `test(...)` registrations in the named
files. It is not an assertion count, generated sample count or passing count.
The standard test counts include all 31 `.i` files in `std/test/`.
Some tests contain hundreds of cases or property checks. Counts are therefore
useful for inventory, not a coverage percentage.

For `i/`, the section also gives the number of `tests/spec/**/*.i` fixtures
whose first import group names that module path. This is an import-based
count. It includes negative fixtures and expected stops, and it misses
operator-based and indirect use. It is not a total of all repository tests.
Relevant examples and Rust integration suites are named separately. Their
counts must not be added as disjoint runtime executions.

A “no direct reference” gap means no qualified public value reference in the
module's designated std test files. Helpers and integration tests can still
exercise it. Per-item documentation means a `///` comment attached to that
value; a long module header is not per-item documentation. Documentation gaps
below list every undocumented public lowercase value. Type/constructor docs
are assessed in prose. Lines include blanks and comments. Export entries
count names written in the export header; expanded names count a union's
implicit public members once, with type and same-named constructor counted
as one API name. Constants count as values.

## Most important findings

1. **There is already one native effect model.** Core services, Fail, tasks,
   requests and guaranteed cleanup exist. The main API migration is removing
   Result-based operational sequencing from std, not inventing an effect
   wrapper. Preserve Option for ordinary absence and Result for caught data.
2. **Resource lifetime is the largest composition gap.** `i/resource` has
   ensure and a three-argument bracket, but no Scope/scoped/registered acquire.
   Successful file/socket/response handles still need explicit lifetime work.
   Native HTTP “streaming” buffers the whole response first.
3. **The library contains substantial reusable implementations.** Dict,
   sequence, streams, regex, CSV, crypto, big integers and float text are not
   mere ports of host operations. Restructuring should retain their algorithms
   while simplifying duplicate APIs and correcting contracts.
4. **Public API consistency is uneven.** Schema, regex, CSV decode and HTTP
   helpers use different data positions. Variant suffixes replace shape-group
   APIs. Failure bridges, query codecs and intrinsic facades have several
   public homes. Source still contains migration-era type/name conflicts.
5. **Tests and docs are uneven at the integration boundary.** Collection tests
   reference nearly all exports. Several HTTP body/stream adapters, log file
   output and CLI service-failure paths have much thinner direct tests. Some
   of these have Rust integration coverage, which is listed explicitly.

## Reference comparison from files on this machine

Paths in this section are relative to `/home/agent/Programming/i/.rd/ref/`.
These are source comparisons. They do not rely on remembered APIs or live web
content. The local Effect package says `4.1.3` in
`effect/packages/effect/package.json`. Individual stability tags still matter:
FileSystem, HttpClient and BrowserHttpClient in the rows below are marked
unstable even though the package version is stable.

| Capability | Reference evidence read | Consequence for this inventory |
|---|---|---|
| Computation and typed errors | `effect/packages/effect/src/Effect.ts` defines lazy Effect values with A/E/R and structured fiber interaction. | In i, use an ordinary function and its Fail/service need. Existing map/fold callback rows already carry this. |
| Service construction | `effect/packages/effect/src/Context.ts` defines service keys and contexts. `effect/packages/effect/src/Layer.ts` describes acquisition, dependency wiring, scoped release and sharing. | Use labelled service values, ordinary constructor functions and `<-`. Resource sharing and lifetime policy still need implementation, but no second Layer type is required. |
| Scoped cleanup | `effect/packages/effect/src/Effect.ts` has scoped/acquireRelease, and `effect/packages/effect/src/Scope.ts` registers cleanup for a lifetime boundary. | Current ensure/acquire is only the bracket foundation. E4 resource Scope fills a real missing capability. |
| Retry, repetition and timing | `effect/packages/effect/src/Schedule.ts` describes stepwise retry/repeat/pacing policies. `effect/packages/effect/src/Effect.ts` exposes retry/repeat/timeout and concurrency options. | There is no equivalent reusable schedule API in these 49 modules. HTTP has a local timeout built with task.race. map_concurrently has no public concurrency bound. |
| Lazy streams and queues | `effect/packages/effect/src/Stream.ts` covers lifecycle, buffering, timing, errors and consumption. `effect/packages/effect/src/Queue.ts` defines bounded, sliding, dropping and unbounded constructors. | Current std/stream is a sound sequential replayable pull foundation. Buffered queues, bounded stream concurrency and timing policies are missing. Build them through scoped tasks and immutable transitions. |
| Stateful services | `effect/packages/effect/src/Ref.ts` includes mutable Ref operations; `effect/packages/effect/src/Scope.ts` also has mutable runtime scope state. | Do not copy mutation into i values. Existing task.Ref is an immutable service handle with explicit scheduler effects; document that state protocol. |
| Schemas | `effect/packages/effect/src/Schema.ts` covers validation, codecs, refinements, transformations, derived test data and representation tooling. | Current JSON-only codecs are a useful subset. They lack native Fail/need-polymorphic transforms and derivation. Derive through i/syntax, not compiler recognition of std/schema. |
| Deterministic time | `effect/packages/effect/src/testing/TestClock.ts` advances virtual time for sleeps, timeouts and schedules. | Current clock fake only returns ready requests. A reusable controllable Clock provider is a functional gap. |
| Host services | `effect/packages/effect/src/FileSystem.ts` defines file operations, streams, sinks and watching. `effect/packages/effect/src/http/HttpClient.ts` supplies outgoing HTTP policy. `effect/packages/platform/browser/src/BrowserHttpClient.ts` supplies browser adapters. Each is marked unstable in these files. | Compare capability breadth, without presenting every platform API as a stable model. i/fs and i/http can share ordinary byte operations and service substitution. |
| List port and error style | `gleam-stdlib/src/gleam/list.gleam` contains group, index_map, prepend, fold and try_fold with the same broad roles and argument order. `gleam-stdlib/src/gleam/int.gleam` and `gleam-stdlib/src/gleam/float.gleam` use Result for parse/divide. | The family resemblance is visible in std/list/int/float. i already has callback needs and Fail, so try_* sequencing and operational Result returns should not be kept merely because the port has them. |
| Absence | `gleam-stdlib/src/gleam/option.gleam` recommends Result for fallible functions and reserves Option for optional arguments or stored data. | i intentionally differs: list.first/dict.get already return Option. Keep ordinary absence distinct from Fail. Do not import that Gleam style rule unchanged. |
| Small data helpers | `gleam-stdlib/src/gleam/pair.gleam` and `gleam-stdlib/src/gleam/order.gleam` contain comparable projections, maps and ordering helpers. | Keep useful higher-order adapters, but recognize pair.new/first/second as syntax facades. |
| Ported convenience surface | `gleam-stdlib/src/gleam/bool.gleam`, `gleam-stdlib/src/gleam/bit_array.gleam`, `gleam-stdlib/src/gleam/dict.gleam`, `gleam-stdlib/src/gleam/set.gleam`, `gleam-stdlib/src/gleam/result.gleam`, `gleam-stdlib/src/gleam/string.gleam`, and `gleam-stdlib/src/gleam/uri.gleam` show the related operation families. | Resemblance is API lineage, not proof of copied implementation. i's hash trie, scalar text and validated URI contracts need their own docs. |
| HTTP data model | `gleam-http/src/gleam/http/request.gleam` parameterizes body data and normalizes header names. | i/http already adds framing and service-backed clients. Its buffered bodies and separate response-reader records need an explicit coherent contract. |
| Actors and processes | `gleam-otp/src/gleam/otp/actor.gleam` describes stateful message-processing actors; `gleam-erlang/src/gleam/erlang/process.gleam` exposes linked and unlinked spawning, subjects and selectors. | Consult message typing and request/reply ergonomics. Do not copy detached spawning or mutable actor objects; i requires structured task lifetimes. |
| Small IO contracts | `go/src/io/io.go` defines Reader/Writer/Closer and precise partial-read/write/EOF rules. `go/src/io/fs/fs.go` defines filesystem interfaces and path rules. | i/net and i/tls duplicate a byte-stream record, while fs/stdio lack one common public contract. Copy clarity about short reads, EOF, ownership and cancellation into immutable BitArray APIs. Do not copy mutable buffers. |
| Cancellation and iteration | `go/src/context/context.go` carries cancellation/deadline state across calls. `go/src/iter/iter.go` defines callback iteration with an early-stop signal. | Needs and structured scopes already carry i's environment/lifetime. fold_until and stream demand already provide stopping. No Go-like context parameter or keyword is needed. |
| HTTP reuse | `go/src/net/http/client.go` separates Client/Transport and documents reuse, redirects and sensitive headers. | i/http already implements pooling and redirect rules. Tests should preserve those contracts while separating buffering and resource lifetime. |
| Paths and CSV | `go/src/path/path.go` explicitly handles slash-separated lexical paths. `go/src/encoding/csv/reader.go` specifies whitespace, blank lines, CRLF and byte-based error columns. | std/path fits a pure lexical module. std/csv deliberately uses scalar columns instead; document that difference rather than implying exact Go parity. |
| Time and regex | `go/src/time/time.go` distinguishes wall-time and monotonic uses. `go/src/regexp/regexp.go` documents search and repeated-match variants with byte indices. | Keep i's explicit Instant/Duration and monotonic measurement. i/regex uses scalar offsets and documents a different repeated-search bound. |
| Timing-sensitive comparison | `go/src/crypto/subtle/constant_time.go` documents content-independent comparison with public length. | std/crypto promises only source-level control flow. A compiled host timing guarantee needs separate evidence; it is not established by ordinary vector tests. |

## Module inventory

### i/assertion

Source: `lib/i/assertion.i`. Turns assertion failures into ordinary result data.

Exported types: none.

```text
catch : (() <- (Fail(E), ..N) -> A) <- (Fail(E), N - Assert) -> Result(A, AssertionFailure)
```

**Language use.** Uses Assert provision, a control boundary, suspension and cancellation. Other Fail needs pass through. Use escape for an escape-only catch. No task, macro or shape API is needed.

**Old style, overlap and thin surface.** The type comment still uses ^ and !. Its Result is intentional caught data. It overlaps the boundary pattern in i/failure.

**Tests.** The four importing spec fixtures and property adapter exercise assertion separation. No dedicated std assertion test file exists; an outlived assertion-handler test is not identified here.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### i/big

Source: `lib/i/big.i`. Implements large Int arithmetic and radix printing in i.

Exported types: none.

```text
compare : (Int, Int) -> Int
add : (Int, Int) -> Int
subtract : (Int, Int) -> Int
multiply : (Int, Int) -> Int
divide : (Int, Int) -> Int
remainder : (Int, Int) -> Int
to_text : (Int, Int) -> String
```

**Language use.** Uses immutable arithmetic, list matching, positional data and a private opaque construction handle. No services, Fail, provision, macros, tasks or resources are needed for valid internal operands.

**Old style, overlap and thin surface.** This is substantial code, not a renamed intrinsic. Private limb/build primitives support the implementation. The module comment explicitly says the runtime calls these functions; that coupling needs review under the owner rule against special library treatment.

**Tests.** No dedicated std tests or importing spec fixtures. crates/ilang-cli/tests/big_int.rs declares 3 tests for arithmetic, bitwise operations and wide segments. Direct invalid-radix and divide-by-zero contracts for this internal API are not documented as recoverable errors.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### i/bits

Source: `lib/i/bits.i`. Reads and writes integer bit segments wider than the runtime small-segment path.

Exported types: none.

```text
read_int : (BitArray, Int, Int, Bool, Bool) -> Int
from_int : (Int, Int, Bool) -> BitArray
```

**Language use.** Uses bit syntax, exact Int arithmetic, matching and immutable chunk lists. Two private primitives handle narrow chunks. Keep it pure. It needs neither a task service nor resources. A public checked entry would need defined invalid-width and bounds errors.

**Old style, overlap and thin surface.** The algorithms are real i code. The module comment describes a runtime call hook. std/bit_array owns user codecs; this module owns integer segment representation.

**Tests.** No dedicated std tests or importing spec fixtures. The wide_integer_segments_match_their_encoding test in crates/ilang-cli/tests/big_int.rs tests the language path. Direct negative-width and out-of-range read behavior is not specified here.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### i/clock

Source: `lib/i/clock.i`. Reads monotonic and Unix nanoseconds and waits through a replaceable clock service.

Export visibility: `Clock(..)`, `ClockError(..)`. Hidden representations below are descriptive only.

```text
Clock = ( monotonic_ns = () -> Request(Result(Int, String)), unix_ns = () -> Request(Result(Int, String)), sleep_ns = (Int) -> Request(Result(Nil, String)), )
ClockError = (operation = String, code = String)
```

```text
monotonic_ns : () <- (Clock, Fail(ClockError), request.Await, request.Owner) -> Int
unix_ns : () <- (Clock, Fail(ClockError), request.Await, request.Owner) -> Int
sleep_ns : (Int) <- (Clock, Fail(ClockError), request.Await, request.Owner) -> Nil
```

**Missing attached type/constructor docs.** `Clock`, `ClockError`. Module prose may still describe them.

**Language use.** Uses a labelled host service, structured ClockError, requests and Fail. Each wrapper awaits an owned operation. A reusable deterministic clock provider could build on this ordinary service. It does not need a second effect type or Layer.

**Old style, overlap and thin surface.** The wrappers translate host Result data into Fail correctly. They add ownership and structured errors, so they are not mere renames. Units are explicit in names; the functions lack item docs.

**Tests.** std/test/clock_test.i has 1 named fake-clock test. Two spec fixtures import the module. examples/io-clock/src/main.i checks real sleep. There is no reusable virtual-clock test API, nor a named fake test for monotonic/unix error mapping.

**Missing item documentation.** `monotonic_ns`, `unix_ns`, `sleep_ns`.

### i/control

Source: `lib/i/control.i`. Provides boundaries, escape, suspension, resumption, cancellation and environment capture.

Exported types: none.

```text
boundary : ((Boundary(A, B, C)) <- (Fail(B), ..C) -> A) <- (Fail(B), ..C) -> A
suspend : (Boundary(A, B, C), (Continuation(D, A, B, C)) <- (Fail(B), ..C) -> A) -> D
escape : (Boundary(A, B, C), A) -> Never
resume : (Continuation(A, B, C, D), A) <- (Fail(C), ..D) -> B
cancel : (Continuation(A, B, C, D)) <- D -> Nil
keep_env : ((A) <- (Fail(E), ..N) -> B) <- N -> (A) <- (Fail(E)) -> B
```

**Language use.** Uses open need rows and Fail in primitive signatures. It is the control foundation for library effects. After E3, Boundary and Continuation need only their ordinary need row; see syntax-3.md §2.10.7. keep_env stays until E6, per §2.10.8.

**Old style, overlap and thin surface.** These are genuine control primitives, not general library combinators to duplicate. The snapshot still exposes separate E parameters on Boundary/Continuation. Keep this migration fact distinct from a proposed second error mechanism.

**Tests.** 60 spec fixtures import i/control. No dedicated std file. The fixture count includes compiler rejection cases and stops, not just runtime successes. Future public row-only Boundary/Continuation signatures are not established by the old snapshot.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### i/core

Source: `lib/i/core.i`. Exposes the small primitive operations that cannot be written in i.

Exported types: none.

```text
not : (Bool) -> Bool
to_float : (Int) -> Float
truncate : (Float) -> Int
floor : (Float) -> Float
ceiling : (Float) -> Float
sqrt : (Float) -> Float
float_to_bits : (Float) -> Int
float_from_bits : (Int) -> Float
bit_and : (Int, Int) -> Int
bit_or : (Int, Int) -> Int
bit_xor : (Int, Int) -> Int
bit_not : (Int) -> Int
shift_left : (Int, Int) -> Int
shift_right : (Int, Int) -> Int
concat : (List(String)) -> String
byte_size : (String) -> Int
byte_slice : (String, Int, Int) -> Result(String, Nil)
codepoints : (String) -> List(UtfCodepoint)
from_codepoints : (List(UtfCodepoint)) -> String
codepoint_to_int : (UtfCodepoint) -> Int
int_to_codepoint : (Int) -> Result(UtfCodepoint, Nil)
to_upper : (String) -> String
to_lower : (String) -> String
index_of : (String, String, Int) -> Result(Int, Nil)
bit_size : (BitArray) -> Int
bit_array_to_string : (BitArray) -> Result(String, Nil)
bit_array_concat : (List(BitArray)) -> BitArray
inspect : (A) -> String
hash : (A) -> Int
portable : (A) -> Bool
```

**Language use.** Uses external declarations and polymorphic values; Result represents raw conversion and search outcomes. Keep the intrinsic boundary small. E4 should define which fallible public operations become Fail; raw ABI result data can remain private.

**Old style, overlap and thin surface.** All 30 explicit value exports are external declarations. Most are justified primitives. std/bool, std/int, std/float, std/string and std/bit_array publish many forwarding wrappers. No explicit type exports appear in this header, although spec §4.3 describes qualified built-in types.

**Tests.** 82 spec fixtures import core. Standard numeric, text and bit-array tests exercise it indirectly. There is no standalone std/core test file; primitive coverage must be read with language operator tests, not inferred from std wrapper counts.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### i/fail

Source: `lib/i/fail.i`. Raises, catches and translates typed errors through Fail(E).

Exported types: none.

```text
raise : (A) <- (Fail(A)) -> Never
catch : (() <- (Fail(E), ..N) -> A) <- N -> Result(A, E)
from_result : (Result(A, B)) <- (Fail(B)) -> A
map_error : (() <- (Fail(E), ..N) -> A, (E) <- N -> B) <- (Fail(B), ..N) -> A
```

**Language use.** Uses Fail, open need rows, matching and Result as caught data. Use the ordinary-service implementation on E3 landing. Add ordinary function combinators only where they add policy; no Effect wrapper or Layer type.

**Old style, overlap and thin surface.** raise and catch are still @external intrinsics in this checkout. i/failure duplicates both in i. from_result and map_error are useful bridges; std/result republishes bridges under other names.

**Tests.** 54 spec fixtures import i/fail. std/test/list_test.i and std/test/stream_test.i check callback failures. No dedicated std/fail file; the E3 ordinary-service replacement still needs its existing boundary and unwind contracts carried over.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### i/failure

Source: `lib/i/failure.i`. Implements raise and catch with ordinary service values and control boundaries.

Exported types: none.

```text
raise : (A) <- (Fail(A)) -> Never
catch : (() <- (Fail(B), ..N) -> A) <- N -> Result(A, B)
```

**Language use.** Uses Fail as a service, explicit provision, escape and Result data. Consolidate with i/fail on E3 landing. No independent service protocol is needed.

**Old style, overlap and thin surface.** Syn3ServiceFail aliases Fail, and catch writes both Fail(Never) and the alias. This is migration scaffolding. Public names and behavior duplicate i/fail.

**Tests.** 8 spec fixtures import i/failure, including light boundaries, resumption, finalizers, directed rows and an outlived handler. No dedicated std file. A separate permanent public compatibility module is not justified by these fixtures.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### i/fetch

Source: `lib/i/fetch.i`. Adapts a host HTTP fetch capability to typed request operations.

Export visibility: `Fetch`, `Response`. Hidden representations below are descriptive only.

```text
Fetch = ( start = (String, String, String, BitArray) -> Request(Result(Int, String)), status = (Int) -> Request(Result(Int, String)), headers = (Int) -> Request(Result(String, String)), read = (Int, Int) -> Request(Result(BitArray, String)), close = (Int) -> Request(Nil), )
Response = (Int)
```

```text
open : (String, String, String, BitArray) <- (Fail(String), Fetch, request.Await, request.Owner) -> Response
status : (Response) <- (Fail(String), Fetch, request.Await, request.Owner) -> Int
headers : (Response) <- (Fail(String), Fetch, request.Await, request.Owner) -> String
read : (Response, Int) <- (Fail(String), Fetch, request.Await, request.Owner) -> BitArray
close : (Response) <- (Fetch, request.Await, request.Owner) -> Nil
```

**Missing attached type/constructor docs.** `Fetch`, `Response`. Module prose may still describe them.

**Language use.** Uses a labelled service, a hidden response handle, owned requests and Fail(String). Use structured fetch errors and attach response cleanup to resource.Scope when E4 lands. A fake service is ordinary provision.

**Old style, overlap and thin surface.** The wrappers do ownership and error translation. Errors remain unstructured strings; headers use a String transport encoding. std/http implements the user-facing HTTP policy.

**Tests.** No dedicated std/fetch file or importing spec fixture. HTTP integration is indirect. Browser provider response cancellation, early close and partial reads need explicit adapter contract coverage.

**Missing item documentation.** `open`, `status`, `headers`, `read`, `close`.

### i/float_text

Source: `lib/i/float_text.i`. Prints the shortest round-tripping decimal form of a finite Float.

Exported types: none.

```text
to_string : (Float) -> String
```

**Language use.** Uses exact integer intervals, tuples, list matching and core float bits. It appropriately uses no service, Fail, macro, task or resource API.

**Old style, overlap and thin surface.** It is a substantial formatter. std/float.to_string reaches it indirectly through core.inspect. Small reverse/append/digit helpers overlap other modules but this implementation cannot recurse through core.inspect.

**Tests.** No dedicated std file or importing spec fixture. crates/ilang-cli/tests/float_text.rs declares 4 tests, including an independent reference and many finite values. No claim is made here about performance or passing status.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### i/fs

Source: `lib/i/fs.i`. Adapts a filesystem service and provides whole-file helpers.

Export visibility: `Fs`, `File`, `FsError(..)`, `OpenMode(..)`, `Metadata(..)`. Hidden representations below are descriptive only.

```text
Fs = ( open = (String, String) -> Request(Result(Int, String)), read = (Int, Int) -> Request(Result(BitArray, String)), write = (Int, BitArray) -> Request(Result(Nil, String)), flush = (Int) -> Request(Result(Nil, String)), close = (Int) -> Request(Nil), remove = (String) -> Request(Result(Nil, String)), rename = (String, String) -> Request(Result(Nil, String)), mkdir = (String) -> Request(Result(Nil, String)), list = (String) -> Request(Result(String, String)), size = (String) -> Request(Result(Int, String)), kind = (String) -> Request(Result(String, String)), temp_dir = () -> Request(Result(String, String)), )
File = (Int)
FsError = (operation = String, path = String, code = String)
OpenMode = ReadOnly | WriteTruncate | Append | ReadWrite
ReadOnly = ()
WriteTruncate = ()
Append = ()
ReadWrite = ()
Metadata = (size = Int, kind = String)
```

```text
open : (String, OpenMode) <- (Fail(FsError), Fs, request.Await, request.Owner) -> File
read : (File, Int) <- (Fail(FsError), Fs, request.Await, request.Owner) -> BitArray
write : (File, BitArray) <- (Fail(FsError), Fs, request.Await, request.Owner) -> Nil
flush : (File) <- (Fail(FsError), Fs, request.Await, request.Owner) -> Nil
close : (File) <- (Fs, request.Await, request.Owner) -> Nil
remove : (String) <- (Fail(FsError), Fs, request.Await, request.Owner) -> Nil
rename : (String, String) <- (Fail(FsError), Fs, request.Await, request.Owner) -> Nil
mkdir : (String) <- (Fail(FsError), Fs, request.Await, request.Owner) -> Nil
list : (String) <- (Fail(FsError), Fs, request.Await, request.Owner) -> List(String)
metadata : (String) <- (Fail(FsError), Fs, request.Await, request.Owner) -> Metadata
temp_dir : () <- (Fail(FsError), Fs, request.Await, request.Owner) -> String
read_file : (String) <- (Fail(FsError), Fs, request.Await, request.Owner) -> BitArray
write_file : (String, BitArray) <- (Fail(FsError), Fs, request.Await, request.Owner) -> Nil
read_lines : (String) <- (Fail(FsError), Fs, request.Await, request.Owner) -> List(String)
```

**Missing attached type/constructor docs.** `Append`, `File`, `Fs`, `FsError`, `Metadata`, `OpenMode`, `ReadOnly`, `ReadWrite`, `WriteTruncate`. Module prose may still describe them.

**Language use.** Uses hidden handles, labelled Metadata/FsError, Fail, request ownership and resource.ensure/acquire. E4 Scope should make file lifetime composable. A byte-reader abstraction could share whole-input and line handling with other IO.

**Old style, overlap and thin surface.** Host-operation wrappers add ownership and errors. read_chunks recursively buffers and concatenates; read_lines buffers the whole file. Metadata makes two separate host calls for size and kind. Most public operations lack item docs.

**Tests.** examples/io-fs/test/fs_test.i has 1 named test; no importing spec fixture. examples/io-fs/src/main.i checks rename, mkdir, list and metadata. There is no dedicated std/fs suite for cancellation at each acquisition point, streaming reads, or metadata consistency.

**Missing item documentation.** `open`, `read`, `write`, `flush`, `close`, `remove`, `rename`, `mkdir`, `list`, `metadata`, `temp_dir`, `read_file`, `write_file`, `read_lines`.

### i/io

Source: `lib/i/io.i`. Defines the simple host text-printing service Io.

Export visibility: `Io(..)`. Hidden representations below are descriptive only.

```text
Io = (print = (String) -> Nil)
```

Exported functions: none.

**Language use.** Uses an ordinary labelled @host service. There are no exported functions. A replacement is supplied with <- like any service. No special compiler recognition should be needed for printing.

**Old style, overlap and thin surface.** Io.print overlaps i/stdio.print but has no typed IO failure or flush contract. Its module comment still describes entry-provider behavior; E5 changes that plan.

**Tests.** No dedicated std/io tests or importing spec fixtures. examples/hello/src/main.i uses Io. crates/ilang-cli/tests/wasi.rs and package_commands.rs contain integration coverage. A common documented choice between Io and Stdio is missing.

**Documentation.** There are no public lowercase values. The service has module/type documentation.

### i/key

Source: `lib/i/key.i`. Creates typed identity keys and safely opens heterogeneous keyed values.

Export visibility: `Lookup(..)`. Hidden representations below are descriptive only.

```text
Lookup(A) = Found(A) | Missing
Found(A) = (A)
Missing = ()
```

```text
new : () -> Key(A)
wrap : (Key(A), A) -> Keyed
unwrap : (Key(A), Keyed) -> Lookup(A)
```

**Missing attached type/constructor docs.** `Found`, `Lookup`, `Missing`. Module prose may still describe them.

**Language use.** Uses invariant Key, Keyed, matching and the Found/Missing union. No services, tasks or resources are required. Keep absence as data; a missing key is not automatically a failure.

**Old style, overlap and thin surface.** unwrap converts the private raw Result into Lookup. This is a useful semantic wrapper, but its private read has a different public vocabulary from option-based lookups elsewhere.

**Tests.** 6 spec fixtures import i/key. tests/spec/9.8/keys.i covers same/different keys, equality, hash and portability. No dedicated std file. Counts alone do not establish all cross-worker lifetime cases.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### i/net

Source: `lib/i/net.i`. Adapts TCP, DNS and UDP operations through a network service.

Export visibility: `Net`, `Listener`, `Stream`, `Datagram`, `NetError(..)`. Hidden representations below are descriptive only.

```text
Net = ( bind = (String, Int) -> Request(Result(Int, String)), send_to = (Int, String, Int, BitArray) -> Request(Result(Nil, String)), receive_from = (Int, Int) -> Request(Result(BitArray, String)), local_address = (Int) -> Request(Result(String, String)), datagram_port = (Int) -> Request(Result(Int, String)), close_datagram = (Int) -> Request(Nil), listen = (String, Int) -> Request(Result(Int, String)), local_port = (Int) -> Request(Result(Int, String)), accept = (Int) -> Request(Result(Int, String)), connect = (String, Int) -> Request(Result(Int, String)), resolve = (String) -> Request(Result(String, String)), read = (Int, Int) -> Request(Result(BitArray, String)), write = (Int, BitArray) -> Request(Result(Nil, String)), shutdown_write = (Int) -> Request(Result(Nil, String)), close_stream = (Int) -> Request(Nil), close_listener = (Int) -> Request(Nil), )
Listener = (Int)
Stream = (Int)
Datagram = (Int)
NetError = (operation = String, endpoint = String, code = String)
```

```text
bind : (String, Int) <- (Fail(NetError), Net, request.Await, request.Owner) -> Datagram
send_to : (Datagram, String, Int, BitArray) <- (Fail(NetError), Net, request.Await, request.Owner) -> Nil
receive_from : (Datagram, Int) <- (Fail(NetError), Net, request.Await, request.Owner) -> (address = String, data = BitArray, port = Int)
local_address : (Datagram) <- (Fail(NetError), Net, request.Await, request.Owner) -> String
close_datagram : (Datagram) <- (Net, request.Await, request.Owner) -> Nil
listen : (String, Int) <- (Fail(NetError), Net, request.Await, request.Owner) -> Listener
local_port : (Listener | Datagram) <- (Fail(NetError), Net, request.Await, request.Owner) -> Int
accept : (Listener) <- (Fail(NetError), Net, request.Await, request.Owner) -> Stream
connect : (String, Int) <- (Fail(NetError), Net, request.Await, request.Owner) -> Stream
resolve : (String) <- (Fail(NetError), Net, request.Await, request.Owner) -> List(String)
read : (Stream, Int) <- (Fail(NetError), Net, request.Await, request.Owner) -> BitArray
write : (Stream, BitArray) <- (Fail(NetError), Net, request.Await, request.Owner) -> Nil
shutdown_write : (Stream) <- (Fail(NetError), Net, request.Await, request.Owner) -> Nil
close_stream : (Stream) <- (Net, request.Await, request.Owner) -> Nil
close_listener : (Listener) <- (Net, request.Await, request.Owner) -> Nil
stream_number : (Stream) -> Int
byte_stream : (Stream) -> (close = () <- (Net, request.Await, request.Owner) -> Nil, read = (Int) <- (Fail(NetError), Net, request.Await, request.Owner) -> BitArray, shutdown_write = () <- (Fail(NetError), Net, request.Await, request.Owner) -> Nil, write = (BitArray) <- (Fail(NetError), Net, request.Await, request.Owner) -> Nil)
```

**Missing attached type/constructor docs.** `Datagram`, `Listener`, `Net`, `NetError`, `Stream`. Module prose may still describe them.

**Language use.** Uses hidden handles, Fail(NetError), labelled packet data, owned requests and resource cleanup. local_port accepts Listener | Datagram. Use a shared byte-stream data contract and E4 Scope for lifetimes. local_port already demonstrates useful argument-type selection; close variants could share an audited API.

**Old style, overlap and thin surface.** byte_stream duplicates the record shape in i/tls. Close functions have resource-kind suffixes. Raw handle escape stream_number exists for TLS. Most operations lack item docs.

**Tests.** examples/io-net/test/net.i has 3 tests; examples/io-udp/test/udp.i has 5 through io_udp/checks. crates/ilang-cli/tests/udp.rs has 2 integration tests; tls.rs and http.rs exercise sockets too. No importing spec fixtures. Direct net.byte_stream and resolver cancellation contracts are not isolated in these named tests.

**Missing item documentation.** `send_to`, `local_address`, `close_datagram`, `listen`, `local_port`, `accept`, `connect`, `resolve`, `read`, `write`, `shutdown_write`, `close_stream`, `close_listener`.

### i/process

Source: `lib/i/process.i`. Separates arguments, environment, working directory, entropy and child-process capabilities.

Export visibility: `Arguments(..)`, `Environment(..)`, `WorkingDirectory(..)`, `Entropy(..)`, `Process`, `Child`, `Stdio(..)`, `EnvironmentMode(..)`, `Directory(..)`, `ProcessError(..)`. Hidden representations below are descriptive only.

```text
Arguments = (args = () -> Request(Result(BitArray, String)))
Environment = (env = (String) -> Request(Result(String, String)))
WorkingDirectory = (cwd = () -> Request(Result(String, String)))
Entropy = (entropy = (Int) -> Request(Result(BitArray, String)))
Process = ( spawn_piped = (String, BitArray, String, String, String) -> Request( Result(Int, String), ), spawn_config = ( String, BitArray, String, String, String, String, BitArray, String, String, ) -> Request(Result(Int, String)), own = (Int) -> Request(Nil), read_stdout = (Int, Int) -> Request(Result(BitArray, String)), read_stderr = (Int, Int) -> Request(Result(BitArray, String)), write_stdin = (Int, BitArray) -> Request(Result(Nil, String)), close_stdin = (Int) -> Request(Result(Nil, String)), spawn = (String, BitArray) -> Request(Result(Int, String)), wait = (Int) -> Request(Result(Int, String)), terminate = (Int) -> Request(Result(Nil, String)), close = (Int) -> Request(Nil), )
Child = (Int)
Stdio = Inherit | Null | Piped
Inherit = ()
Null = ()
Piped = ()
EnvironmentMode = InheritEnvironment | ReplaceEnvironment
InheritEnvironment = (List(String))
ReplaceEnvironment = (List(String))
Directory = (String)
Directory = InheritDirectory | Directory
InheritDirectory = ()
ProcessError = (operation = String, code = String)
```

```text
spawn_piped : (String, List(String), Stdio, Stdio, Stdio) <- (Fail(ProcessError), Process, request.Await, request.Owner) -> Child
spawn_config : (String, List(String), Stdio, Stdio, Stdio, EnvironmentMode, Directory) <- (Fail(ProcessError), Process, request.Await, request.Owner) -> Child
read_stdout : (Child, Int) <- (Fail(ProcessError), Process, request.Await, request.Owner) -> BitArray
read_stderr : (Child, Int) <- (Fail(ProcessError), Process, request.Await, request.Owner) -> BitArray
write_stdin : (Child, BitArray) <- (Fail(ProcessError), Process, request.Await, request.Owner) -> Nil
close_stdin : (Child) <- (Fail(ProcessError), Process, request.Await, request.Owner) -> Nil
args : () <- (Arguments, Fail(ProcessError), request.Await, request.Owner) -> List(String)
env : (String) <- (Environment, Fail(ProcessError), request.Await, request.Owner) -> String
cwd : () <- (Fail(ProcessError), WorkingDirectory, request.Await, request.Owner) -> String
entropy : (Int) <- (Entropy, Fail(ProcessError), request.Await, request.Owner) -> BitArray
spawn : (String, List(String)) <- (Fail(ProcessError), Process, request.Await, request.Owner) -> Child
wait : (Child) <- (Fail(ProcessError), Process, request.Await, request.Owner) -> Int
terminate : (Child) <- (Fail(ProcessError), Process, request.Await, request.Owner) -> Nil
close : (Child) <- (Process, request.Await, request.Owner) -> Nil
```

**Missing attached type/constructor docs.** `Arguments`, `Child`, `Directory`, `Entropy`, `Environment`, `EnvironmentMode`, `Inherit`, `InheritDirectory`, `InheritEnvironment`, `Null`, `Piped`, `Process`, `ProcessError`, `ReplaceEnvironment`, `Stdio`, `WorkingDirectory`. Module prose may still describe them.

**Language use.** Uses ordinary capability types, hidden Child, unions, labelled errors, Fail, owned requests and cleanup. A labelled spawn configuration could replace the seven-position spawn_config call. Scope should own Child lifetime. No second effect environment is needed.

**Old style, overlap and thin surface.** The capability split is useful. spawn_piped has five arguments and spawn_config seven. Directory is declared as both a labelled type and a union name in this source. Env/argv use private binary framing.

**Tests.** examples/io-process/test/process_test.i has 2 tests. No importing spec fixtures. examples/io-process/src/main.i adds a demonstration. Dedicated piped/configured child IO, termination/cancellation, environment replacement and working-directory error tests are not present in that file.

**Missing item documentation.** `spawn_piped`, `read_stdout`, `read_stderr`, `write_stdin`, `close_stdin`, `args`, `env`, `cwd`, `entropy`, `spawn`, `wait`, `terminate`, `close`.

### i/random

Source: `lib/i/random.i`. Implements immutable splittable pseudo-random generators with reproducible paths.

Export visibility: `Generator`, `RandomError(..)`. Hidden representations below are descriptive only.

```text
Generator = (Int, State, List(Step), Int)
RandomError = InvalidBound | InvalidPath
InvalidBound = (Int)
InvalidPath = (List(Int))
```

```text
from_seed : (Int) -> Generator
word : (Generator) -> (Int, Generator)
int : (Generator, Int) <- (Fail(RandomError)) -> (Int, Generator)
float : (Generator) -> (Float, Generator)
split : (Generator) -> (Generator, Generator)
at_path : (Generator, List(Int)) <- (Fail(RandomError)) -> Generator
```

**Missing attached type/constructor docs.** `Generator`, `InvalidBound`, `InvalidPath`, `RandomError`. Module prose may still describe them.

**Language use.** Uses a hidden one-constructor Generator, internal unions, explicit successor state and Fail(RandomError). Purity is useful here. Tasks can supply generators without introducing a global mutable RNG service. Do not confuse this with process.Entropy.

**Old style, overlap and thin surface.** This is substantial i code. Invalid bounds/paths already use Fail. Helper names int and float are consistent with constructors in other modules. Its seed/path protocol deserves a stability statement.

**Tests.** 14 spec fixtures import i/random. std/test/property_test.i exercises path-derived generators indirectly. No dedicated std/random file. Statistical quality and cross-version reproducibility are not proven by fixture counts.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### i/request

Source: `lib/i/request.i`. Tracks typed host work, cancellation ownership and replaceable waits.

Export visibility: `Await(..)`, `Root(..)`, `Owner(..)`, `RequestState(..)`. Hidden representations below are descriptive only.

```text
Await = (wait = (Ticket) -> Nil)
Root = (key = Key(Nil))
Owner = (key = Key(Nil), root = Root)
RequestState(A) = Waiting | Ready(A) | Cancelled | Taken
Waiting = ()
Ready(A) = (A)
Cancelled = ()
Taken = ()
```

```text
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
within : (() <- (Fail(E), ..N) -> A) <- (Fail(E), N - Owner, Root) -> A
ready : (A) -> Request(A)
ticket : (Request(A)) -> Ticket
state : (Request(A)) -> RequestState(A)
take : (Request(A)) -> A
settled : (Ticket) -> Bool
cancel : (Ticket) -> Nil
await : (Request(A)) <- (Await) -> A
await_owned : (Request(A)) <- (Await) -> A
wait_any : (List(Ticket)) -> Nil
poll : () -> Nil
```

**Missing attached type/constructor docs.** `Await`, `Cancelled`, `Owner`, `Ready`, `RequestState`, `Root`, `Taken`, `Waiting`. Module prose may still describe them.

**Language use.** Uses RequestState unions, Root/Owner/Await services, provision, resource.ensure and raw request primitives. Keep request ownership underneath tasks and IO. E4 Scope and E5 entry services should share one lifetime account, not another scheduler or context type.

**Old style, overlap and thin surface.** Most wrappers translate typed handles and add ownership. await and await_owned differ in cancellation semantics. Root/Owner count/cancel/wait families repeat structure; private _provider_* entries are E5 migration material.

**Tests.** 29 spec fixtures import i/request. Clock/stdio fake tests use ready, and IO examples use request ownership. No dedicated std/request file. Post-E5 entry lifetimes and complete public state-transition coverage cannot be inferred from those mixed fixtures.

**Missing item documentation.** `open_root`, `close_root`, `open_owner`, `close_owner`, `count`, `count_root`, `settled_owner`, `settled_root`, `cancel_all`, `cancel_all_root`, `wait_none`, `wait_none_root`, `within`, `ticket`, `state`, `take`, `settled`, `cancel`.

### i/resource

Source: `lib/i/resource.i`. Runs guaranteed cleanup around computations and acquired values.

Exported types: none.

```text
ensure : (() <- (Fail(E), ..N) -> A, () <- N -> Nil) <- (Fail(E), ..N) -> A
acquire : (() <- (Fail(E), ..N) -> A, (A) <- N -> Nil, (A) <- (Fail(E), ..N) -> B) <- (Fail(E), ..N) -> B
```

**Language use.** Uses Fail/open need rows and an ensure primitive. acquire is a three-function bracket. Implement the planned Scope service, scoped and two-argument acquire in E4 (§2.10.9). Preserve ensure as the low-level unwind contract.

**Old style, overlap and thin surface.** The current bracket is useful but cannot register several independently acquired resources in one surrounding scope. task.Scope(E) is a different concept and is slated for Group in E5.

**Tests.** 50 spec fixtures import i/resource. std/test/stream_test.i includes stream_resource_scope. No dedicated std/resource file. Scope registration order, nested scopes and early-exit lifetime tests cannot exist for the absent E4 API.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### i/stdio

Source: `lib/i/stdio.i`. Reads and writes standard IO with typed errors and owned requests.

Export visibility: `Stdio(..)`, `StdioError(..)`. Hidden representations below are descriptive only.

```text
Stdio = ( read = (Int) -> Request(Result(BitArray, String)), write = (BitArray) -> Request(Result(Nil, String)), write_error = (BitArray) -> Request(Result(Nil, String)), flush = () -> Request(Result(Nil, String)), flush_error = () -> Request(Result(Nil, String)), )
StdioError = (operation = String, code = String)
```

```text
read : (Int) <- (Fail(StdioError), Stdio, request.Await, request.Owner) -> BitArray
write : (BitArray) <- (Fail(StdioError), Stdio, request.Await, request.Owner) -> Nil
write_error : (BitArray) <- (Fail(StdioError), Stdio, request.Await, request.Owner) -> Nil
flush : () <- (Fail(StdioError), Stdio, request.Await, request.Owner) -> Nil
flush_error : () <- (Fail(StdioError), Stdio, request.Await, request.Owner) -> Nil
print : (String) <- (Fail(StdioError), Stdio, request.Await, request.Owner) -> Nil
print_error : (String) <- (Fail(StdioError), Stdio, request.Await, request.Owner) -> Nil
```

**Missing attached type/constructor docs.** `Stdio`, `StdioError`. Module prose may still describe them.

**Language use.** Uses Stdio/StdioError, Fail and request ownership; print appends LF and encodes UTF-8. A shared byte reader/writer contract could avoid separate buffering helpers. It needs no new effect wrapper.

**Old style, overlap and thin surface.** Wrappers add failure translation. print overlaps Io.print. The _error suffix means standard error, whereas map_error elsewhere means a failure payload. All public functions lack item docs.

**Tests.** std/test/stdio_test.i has 1 fake test for read, print and print_error, including invalid read. flush and flush_error are provided but never called there. Pending cancellation and output-failure mapping are not tested in that named case.

**Missing item documentation.** `read`, `write`, `write_error`, `flush`, `flush_error`, `print`, `print_error`.

### i/syntax

Source: `lib/i/syntax.i`. Exposes source syntax and compiler query services to macro programs.

Export visibility: `Syntax`, `Node(..)`, `TypeHandle`, `Position(..)`, `SyntaxError(..)`, `Definitions(..)`, `Items(..)`, `Diagnostics(..)`, `Types(..)`, `Truth(..)`. Hidden representations below are descriptive only.

```text
Syntax = Opaque
Node = TypeView | Name | Discard | Up | Symbol | Int | Float | Text | Round | Square | Curly | Bits | Quote | Unquote | Splice
TypeView = (TypeHandle)
Name = (String)
Discard = (String)
Up = (String)
Symbol = (String)
Int = (Int)
Float = (Float)
Text = (String)
Round = (List(Syntax))
Square = (List(Syntax))
Curly = (List(Syntax))
Bits = (List(Syntax))
Quote = (Syntax)
Unquote = (Syntax)
Splice = (Syntax)
TypeHandle = Opaque
Position = ImportItem | ExportItem | Item | Attribute | Constructor | Field | Type | FunctionResult | Parameter | Expression | Statement | CallArgument | Clause | ClauseHead | Pattern | Segment | Format
ImportItem = ()
ExportItem = ()
Item = ()
Attribute = ()
Constructor = ()
Field = ()
Type = ()
FunctionResult = ()
Parameter = ()
Expression = ()
Statement = ()
CallArgument = ()
Clause = ()
ClauseHead = ()
Pattern = ()
Segment = ()
Format = ()
SyntaxError = (message = String, at = Syntax)
Definitions = (definition = (Syntax) <- (Fail(SyntaxError)) -> Syntax)
Items = ( add = (Syntax, Syntax, List((String, String)), Syntax) <- ( Fail(SyntaxError), ) -> Nil, expose = (Syntax) <- (Fail(SyntaxError)) -> Nil, )
Diagnostics = (note = (String) -> Nil)
Types = ( type_of = (Syntax) <- (Fail(SyntaxError)) -> Syntax, signature = () -> Syntax, subtype = (Syntax, Syntax) <- (Fail(SyntaxError)) -> Truth, members = (Syntax) <- (Fail(SyntaxError)) -> List(Syntax), )
Truth = Yes | No | NotKnown
Yes = ()
No = ()
NotKnown = ()
```

```text
definition : (Syntax) <- (Definitions, Fail(SyntaxError)) -> Syntax
node : (Syntax) -> Node
make : (Node, Syntax) <- (Fail(SyntaxError)) -> Syntax
same_code : (Syntax, Syntax) -> Bool
show : (Syntax) -> String
show_as : (Syntax, Position) <- (Fail(SyntaxError)) -> String
type_of : (Syntax) <- (Fail(SyntaxError), Types) -> Syntax
signature : () <- (Types) -> Syntax
subtype : (Syntax, Syntax) <- (Fail(SyntaxError), Types) -> Truth
members : (Syntax) <- (Fail(SyntaxError), Types) -> List(Syntax)
```

**Missing attached type/constructor docs.** `Attribute`, `Bits`, `CallArgument`, `Clause`, `ClauseHead`, `Constructor`, `Curly`, `Definitions`, `Diagnostics`, `Discard`, `ExportItem`, `Expression`, `Field`, `Float`, `Format`, `FunctionResult`, `ImportItem`, `Int`, `Item`, `Items`, `Name`, `No`, `Node`, `NotKnown`, `Parameter`, `Pattern`, `Position`, `Quote`, `Round`, `Segment`, `Splice`, `Square`, `Statement`, `Symbol`, `Syntax`, `SyntaxError`, `Text`, `Type`, `TypeHandle`, `TypeView`, `Types`, `Unquote`, `Up`, `Yes`. Module prose may still describe them.

**Language use.** Uses opaque handles, labelled node unions, Definitions/Items/Diagnostics/Types services and Fail(SyntaxError). Macro consumers can derive closed-data codecs and tests in ordinary library macros. No runtime type reflection or special std schema rule is needed.

**Old style, overlap and thin surface.** Source still declares Int(Int), Float(Float), a plain Node union and old Position members Clause/ClauseHead. syntax-3.md §2.8 requires built-in numeric members and a labelled Node. Raw host wrappers are substantial boundary plumbing. Some public query wrappers lack item docs.

**Tests.** No dedicated std test or importing spec fixture. crates/ilang-cli/tests/syntax_runtime.rs declares 10 tests; typed_macros.rs and macro_run.rs add broader coverage. New Node and Position shapes need migration-specific round-trip coverage; old tests do not establish that future surface.

**Missing item documentation.** `make`, `type_of`, `signature`, `subtype`, `members`.

### i/task

Source: `lib/i/task.i`. Implements structured tasks, task groups, events, rendezvous channels and scheduler-owned state.

Export visibility: `Scheduler`, `DetachedScheduler`, `Runner`, `DetachedRunner`, `Scope`, `Task`, `Channel`, `Event`, `Shared`, `Ref`, `ScheduleError(..)`. Hidden representations below are descriptive only.

```text
Scheduler = ( run = Key(Nil), root = request.Root, path = Keyed, seed = () -> rng.Generator, witness = Witness, step = (Command) -> StepAnswer, runner = Runner, waiter = request.Await, )
Scheduler = DetachedScheduler | Scheduler
DetachedScheduler = ()
Runner = ( seed = rng.Generator, context = request.Root, policy = Policy, allocate = () -> List(Int), active = (Key(Nil), Active) -> RunControl, wait = (Key(Nil), request.Root, List(Ticket)) -> RunControl, poll = (Key(Nil), List(Ticket), Bool) -> RunControl, forget = (Key(Nil)) -> Nil, )
Runner = DetachedRunner | Runner
DetachedRunner = ()
Scope(E) = (info = ScopeInfo, errors = Key(E))
Task(A, E) = (info = TaskInfo, result = Key(Result(A, E)), owner = Scope(E))
Channel(A) = (info = ChannelInfo, values = Key(A))
Event(A, E, N) = (identity = Key(Nil), branches = List(Branch(A, E, N)))
Shared(A) = (run = Key(Nil), owner = Keyed, id = Keyed, values = Key(A))
Ref(A) = (run = Key(Nil), id = Keyed, values = Key(A))
ScheduleError = NotRunnable
NotRunnable = (path = List(Int))
```

```text
top : (rng.Generator) <- (Runner) -> Runner
run : (() <- (Fail(E), ..N) -> A) <- (Fail(E), N - request.Await - Runner - Scheduler - request.Owner - request.Root, Runner) -> A
test : (rng.Generator, List(List(Int)), () <- (Fail(E), ..N) -> A) <- (Fail(E | ScheduleError), N - request.Await - Runner - Scheduler - request.Owner - request.Root, Runner) -> A
scope : ((Scope(A)) <- (Fail(A), ..N) -> B) <- (Fail(A), N - request.Owner, Scheduler) -> B
local_scope : ((Scope(A)) <- (Fail(A), ..N) -> B) <- (Fail(A), N - request.Owner, Scheduler) -> B
go : (Scope(A), () <- (Fail(A), ..N) -> B) <- (N - request.Await - Runner - Scheduler, Scheduler) -> Task(B, A)
join : (Task(A, B)) <- (Fail(B), Scheduler) -> A
yield : () <- (Scheduler) -> Nil
path : () <- (Scheduler) -> List(Int)
random : () <- (Scheduler) -> rng.Generator
keep : ((A) <- (Fail(E), ..N) -> B) <- (N - request.Await - Runner - Scheduler, Scheduler) -> (A) <- (Fail(E), Scheduler) -> B
channel : () <- (Scheduler) -> Channel(A)
local_channel : () <- (Scheduler) -> Channel(A)
send : (Channel(A), A) <- (Scheduler) -> Nil
receive : (Channel(A)) <- (Scheduler) -> A
send_event : (Channel(A), A) -> Event(Nil, Never, Never)
receive_event : (Channel(A)) -> Event(A, Never, Never)
join_event : (Task(A, B)) -> Event(A, B, Never)
always : (A) -> Event(A, Never, Never)
never : () -> Event(Never, Never, Never)
choose : (List(Event(A, B, C))) -> Event(A, B, C)
map : (Event(A, B, C), (A) <- (Fail(E), ..N) -> D) -> Event(D, B | E, C | N)
sync : (Event(A, B, C)) <- (Fail(B), Scheduler, ..C) -> A
share : (Scope(A), B) <- (Scheduler) -> Shared(B)
get : (Shared(A)) <- (Scheduler) -> A
ref : (A) <- (Scheduler) -> Ref(A)
update : (Ref(A), (A) -> (A, B)) <- (Scheduler) -> B
both : (() <- (Fail(E), ..N) -> A, () <- (Fail(E), ..N) -> B) <- (Fail(E), N - request.Await - Runner - Scheduler - request.Owner, Scheduler) -> (A, B)
race : (() <- (Fail(E), ..N) -> A, () <- (Fail(E), ..N) -> A) <- (Fail(E), N - request.Await - Runner - Scheduler - request.Owner, Scheduler) -> A
map_concurrently : (List(A), (A) <- (Fail(E), ..N) -> B) <- (Fail(E), N - request.Await - Runner - Scheduler - request.Owner, Scheduler) -> List(B)
```

**Missing attached type/constructor docs.** `Channel`, `DetachedRunner`, `DetachedScheduler`, `Event`, `NotRunnable`, `Ref`, `Runner`, `ScheduleError`, `Scheduler`, `Scope`, `Shared`, `Task`. Module prose may still describe them.

**Language use.** Uses services/provision, Fail, keys, labelled internal state, continuations, resources, scopes and first-class event values. General retry/repeat/timeout and bounded concurrency can be ordinary function policies over these operations. Resource Scope must remain distinct from task groups. General function composition already carries effects.

**Old style, overlap and thin surface.** This is the largest module, not a thin facade. Public Event is deferred selection data, so it does not itself justify duplicating ordinary function types. Ref/update represent state through a scheduler service; public values remain immutable, but the state semantics need explicit documentation. Most public entries lack item docs.

**Tests.** 77 spec fixtures import i/task. Task fixtures cover structured joins, cancellation, race, events, scheduling, locality and failures. No dedicated std/task file. Bounded map limits, retry schedules and buffered-queue contracts are absent APIs, so they have no public tests.

**Missing item documentation.** `top`, `run`, `test`, `scope`, `local_scope`, `go`, `join`, `yield`, `path`, `random`, `keep`, `channel`, `local_channel`, `send`, `receive`, `send_event`, `receive_event`, `join_event`, `always`, `never`, `choose`, `map`, `sync`, `share`, `get`, `update`, `both`, `race`, `map_concurrently`.

### i/tls

Source: `lib/i/tls.i`. Upgrades connected network streams to TLS and exposes their byte operations.

Export visibility: `Tls`, `Stream`, `TlsError(..)`. Hidden representations below are descriptive only.

```text
Tls = ( connect = (Int, String, String, BitArray, BitArray, BitArray) -> Request( Result(Int, String), ), accept = (Int, BitArray, BitArray, String) -> Request(Result(Int, String)), alpn = (Int) -> Request(Result(String, String)), read = (Int, Int) -> Request(Result(BitArray, String)), write = (Int, BitArray) -> Request(Result(Nil, String)), shutdown_write = (Int) -> Request(Result(Nil, String)), close = (Int) -> Request(Nil), )
Stream = (Int)
TlsError = (stage = String, reason = String)
```

```text
options : () -> (alpn = List(String), cert = BitArray, key = BitArray, roots = BitArray)
identity : (BitArray, BitArray) -> (cert = BitArray, key = BitArray)
connect : (net.Stream, String, (alpn = List(String), cert = BitArray, key = BitArray, roots = BitArray)) <- (Fail(TlsError), Tls, request.Await, request.Owner) -> Stream
accept : (net.Stream, (cert = BitArray, key = BitArray), (alpn = List(String), cert = BitArray, key = BitArray, roots = BitArray)) <- (Fail(TlsError), Tls, request.Await, request.Owner) -> Stream
dial : (String, Int, (alpn = List(String), cert = BitArray, key = BitArray, roots = BitArray)) <- (Fail(TlsError | net.NetError), Tls, net.Net, request.Await, request.Owner) -> Stream
alpn : (Stream) <- (Fail(TlsError), Tls, request.Await, request.Owner) -> String
read : (Stream, Int) <- (Fail(TlsError), Tls, request.Await, request.Owner) -> BitArray
write : (Stream, BitArray) <- (Fail(TlsError), Tls, request.Await, request.Owner) -> Nil
shutdown_write : (Stream) <- (Fail(TlsError), Tls, request.Await, request.Owner) -> Nil
close : (Stream) <- (Tls, request.Await, request.Owner) -> Nil
byte_stream : (Stream) -> (close = () <- (Tls, request.Await, request.Owner) -> Nil, read = (Int) <- (Fail(TlsError), Tls, request.Await, request.Owner) -> BitArray, shutdown_write = () <- (Fail(TlsError), Tls, request.Await, request.Owner) -> Nil, write = (BitArray) <- (Fail(TlsError), Tls, request.Await, request.Owner) -> Nil)
```

**Missing attached type/constructor docs.** `Stream`, `Tls`, `TlsError`. Module prose may still describe them.

**Language use.** Uses hidden handles, labelled option data, Fail(TlsError), requests and acquisition cleanup. Scope should own successful streams. Share the byte-stream shape with net. Expose typed options only if they add invariant checks over the existing closed data.

**Old style, overlap and thin surface.** The options data is a useful native-language feature. byte_stream duplicates net. TLS error strings are split at a colon before becoming labelled errors. accept takes opts but forwards only opts.alpn; roots/cert/key in that argument do not configure the host accept call.

**Tests.** No dedicated std/tls file or importing spec fixture. crates/ilang-cli/tests/tls.rs has 1 test for round trip, ALPN, trust and host-name rejection. Direct shutdown_write, cancellation at handshake completion and all option-field behavior lack isolated named cases there.

**Missing item documentation.** `options`, `identity`, `connect`, `accept`, `dial`, `alpn`, `read`, `write`, `shutdown_write`, `close`, `byte_stream`.

### std/bit_array

Source: `std/src/bit_array.i`. Provides immutable bit-array operations and Base16/Base64 codecs.

Exported types: none.

```text
from_string : (String) -> BitArray
to_string : (BitArray) -> Result(String, Nil)
is_utf8 : (BitArray) -> Bool
bit_size : (BitArray) -> Int
byte_size : (BitArray) -> Int
append : (BitArray, BitArray) -> BitArray
concat : (List(BitArray)) -> BitArray
slice : (BitArray, Int, Int) -> Result(BitArray, Nil)
starts_with : (BitArray, BitArray) -> Bool
pad_to_bytes : (BitArray) -> BitArray
compare : (BitArray, BitArray) -> Order
inspect : (BitArray) -> String
base16_encode : (BitArray) -> String
base16_decode : (String) -> Result(BitArray, Nil)
base64_encode : (BitArray, Bool) -> String
base64_decode : (String) -> Result(BitArray, Nil)
base64_url_encode : (BitArray, Bool) -> String
base64_url_decode : (String) -> Result(BitArray, Nil)
```

**Language use.** Uses bit patterns, ordinary matching, Order and Result data; no explicit service reads. E4 should move invalid encodings, UTF-8 conversion and checked slices to Fail. Pure encoders need no services. Padding defaults could use shape groups.

**Old style, overlap and thin surface.** bit_size, concat, to_string and inspect wrap core; from_string wraps bit syntax. Many codecs and comparison algorithms are real i code. Boolean padding arguments and six codec names need a consistent convention.

**Tests.** 147 named tests: `std/test/bit_array_test.i` (147). All public values are referenced in the dedicated file. It covers invalid encodings, alignment, UTF-8, slicing and round trips. It does not test the planned Fail interface or optional-argument shapes.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### std/bool

Source: `std/src/bool.i`. Provides Boolean algebra, conversions and conditional callback helpers.

Exported types: none.

```text
not : (Bool) -> Bool
and : (Bool, Bool) -> Bool
or : (Bool, Bool) -> Bool
xor : (Bool, Bool) -> Bool
nand : (Bool, Bool) -> Bool
nor : (Bool, Bool) -> Bool
exclusive_nor : (Bool, Bool) -> Bool
to_string : (Bool) -> String
to_int : (Bool) -> Int
compare : (Bool, Bool) -> Order
guard : <A, N>(Bool, A, () <- N -> A) <- N -> A
lazy_guard : <A, N>( Bool, () <- N -> A, () <- N -> A ) <- N -> A
```

**Language use.** Uses branch functions, scalar comparisons and callbacks with an open need N. The callbacks already carry arbitrary services including Fail. No dedicated Result or task API is needed. A shared name would require distinct labelled shapes, such as value versus make; their current arity is identical.

**Old style, overlap and thin surface.** not only forwards core.not; and/or are eager functions, while language &&/|| short-circuit. xor versus exclusive_nor is inconsistent vocabulary.

**Tests.** 34 named tests: `std/test/bool_test.i` (34). Every exported function appears in the dedicated tests. The source is small; new shape-overloaded lazy/eager forms have no tests because they do not exist.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### std/calendar

Source: `std/src/calendar.i`. Converts and formats validated Gregorian dates, civil times and fixed offsets.

Export visibility: `Date`, `TimeOfDay`, `Offset`, `Civil(..)`, `Weekday(..)`, `CalendarError(..)`. Hidden representations below are descriptive only.

```text
Date = (Int, Int, Int)
TimeOfDay = (Int, Int, Int, Int)
Offset = (Int)
Civil = (date = Date, time = TimeOfDay)
Weekday = Monday | Tuesday | Wednesday | Thursday | Friday | Saturday | Sunday
Monday = ()
Tuesday = ()
Wednesday = ()
Thursday = ()
Friday = ()
Saturday = ()
Sunday = ()
CalendarError = BadMonth | BadDay | BadHour | BadMinute | BadSecond | BadNanosecond | BadOffset | BadText
BadMonth = ()
BadDay = ()
BadHour = ()
BadMinute = ()
BadSecond = ()
BadNanosecond = ()
BadOffset = ()
BadText = ()
```

```text
date : (Int, Int, Int) -> Result(Date, CalendarError)
time_of_day : (Int, Int, Int, Int) -> Result( TimeOfDay, CalendarError, )
offset : (Int) -> Result(Offset, CalendarError)
utc : () -> Offset
date_parts : (Date) -> (Int, Int, Int)
time_parts : (TimeOfDay) -> (Int, Int, Int, Int)
offset_minutes : (Offset) -> Int
is_leap_year : (Int) -> Bool
days_in_month : (Int, Int) -> Result(Int, CalendarError)
day_number : (Date) -> Int
from_day_number : (Int) -> Date
weekday : (Date) -> Weekday
add_days : (Date, Int) -> Date
days_between : (Date, Date) -> Int
at : (time.Instant, Offset) -> Civil
to_instant : (Civil, Offset) -> time.Instant
parse : (String) -> Result((Civil, Offset), CalendarError)
format : (Civil, Offset) -> String
now : () <- (clock.Clock, Fail(clock.ClockError), request.Await, request.Owner) -> Civil
format_now : () <- (clock.Clock, Fail(clock.ClockError), request.Await, request.Owner) -> String
```

**Missing attached type/constructor docs.** `BadDay`, `BadHour`, `BadMinute`, `BadMonth`, `BadNanosecond`, `BadOffset`, `BadSecond`, `BadText`, `CalendarError`, `Civil`, `Date`, `Friday`, `Monday`, `Offset`, `Saturday`, `Sunday`, `Thursday`, `TimeOfDay`, `Tuesday`, `Wednesday`, `Weekday`. Module prose may still describe them.

**Language use.** Uses hidden validated Date/TimeOfDay/Offset, labelled Civil and error/weekday unions. now forwards clock needs. Validation and parsing should use Fail under E4. Shape groups could supply zero nanoseconds or UTC defaults. Keep pure calendar arithmetic independent of Clock.

**Old style, overlap and thin surface.** Most APIs lack item docs despite detailed module laws. Parts are positional tuples. Fixed offsets are not time-zone rules. now/format_now duplicate composition of std/time and UTC formatting.

**Tests.** 5 named tests: `std/test/calendar_test.i` (5). Five named tests contain validation, known dates, broad day laws, boundaries and text cases. clock_test adds now/format_now. No named time-zone/DST tests exist; the module offers fixed offsets only. E4 failure behavior remains untested.

**Missing item documentation.** `date`, `time_of_day`, `offset`, `utc`, `date_parts`, `time_parts`, `offset_minutes`, `is_leap_year`, `days_in_month`, `day_number`, `from_day_number`, `weekday`, `add_days`, `days_between`, `at`, `to_instant`, `parse`, `format`.

### std/cli

Source: `std/src/cli.i`. Builds composable argument parsers and runs commands with process arguments.

Export visibility: `Parser`, `Names(..)`, `DefinitionError(..)`, `CliError(..)`. Hidden representations below are descriptive only.

```text
Parser(A) = ( List(Field), (List(List(Token))) -> Result((A, List(List(Token))), CliError), )
Names = (long = String, short = Option(String))
DefinitionError = InvalidName | DuplicateName | MisplacedRest
InvalidName = (String)
DuplicateName = (String)
MisplacedRest = ()
CliError = BadDefinition | UnknownOption | MissingValue | UnexpectedValue | RepeatedOption | MissingArgument | InvalidArgument | ExtraArgument
BadDefinition = (DefinitionError)
UnknownOption = (index = Int, token = String)
MissingValue = (index = Int, name = String)
UnexpectedValue = (index = Int, name = String)
RepeatedOption = (index = Int, name = String)
MissingArgument = (name = String)
InvalidArgument = (index = Int, name = String, text = String, reason = String)
ExtraArgument = (index = Int, token = String)
```

```text
pure : <A>(A) -> Parser(A)
flag : (Names, String) -> Parser(Bool)
option : <A>( Names, String, (String) -> Result(A, String) ) -> Parser(A)
optional : <A>( Names, String, (String) -> Result(A, String) ) -> Parser(Option(A))
many : <A>( Names, String, (String) -> Result(A, String) ) -> Parser(List(A))
argument : <A>( String, String, (String) -> Result(A, String) ) -> Parser(A)
rest : <A>( String, String, (String) -> Result(A, String) ) -> Parser(List(A))
map : <A, B>(Parser(A), (A) -> B) -> Parser(B)
map2 : <A, B, C>( Parser(A), Parser(B), (A, B) -> C ) -> Parser(C)
parse : <A>(Parser(A), List(String)) -> Result(A, CliError)
read : <A>(Parser(A)) <- (process.Arguments, Fail(process.ProcessError), request.Await, request.Owner) -> Result(A, CliError)
run : <A, N>(Parser(A), String, (A) <- N -> Nil) <- (process.Arguments, stdio.Stdio, Fail(CliError | process.ProcessError | stdio.StdioError), request.Await, request.Owner, ..N) -> Nil
help : <A>(Parser(A), String) -> Result( String, DefinitionError, )
text : (String) -> Result(String, String)
integer : (String) -> Result(Int, String)
```

**Missing attached type/constructor docs.** `BadDefinition`, `CliError`, `DefinitionError`, `DuplicateName`, `ExtraArgument`, `InvalidArgument`, `InvalidName`, `MisplacedRest`, `MissingArgument`, `MissingValue`, `Names`, `Parser`, `RepeatedOption`, `UnexpectedValue`, `UnknownOption`. Module prose may still describe them.

**Language use.** Uses a hidden Parser, labelled names/errors and unions. read/run compose process, stdio and Fail; parser callbacks are closed pure functions returning Result. Move operational parse/help/converter errors to Fail. Allow needs in validators and maps if effectful validation is part of the intended API. Derive parsers with ordinary macros only if useful.

**Old style, overlap and thin surface.** Pure parsers rebuild a Result pipeline even though ordinary functions can carry failure. read returns Result(CliError) while process errors escape through Fail. run bridges both, prints usage and re-raises. Most functions lack item docs.

**Tests.** 10 named tests: `std/test/cli_test.i` (10). Ten named tests cover parsing, definition checks, token ownership, errors, laws, help and long inputs. read/run have no direct calls in cli_test.i. crates/ilang-cli/tests/cli_log.rs has 2 end-to-end cases through examples/cli-log. Fake argument/stdio failure and effectful validator contracts are not covered as a public parser feature.

**Missing item documentation.** `pure`, `flag`, `option`, `optional`, `many`, `argument`, `rest`, `map`, `map2`, `parse`, `help`, `text`, `integer`.

### std/crypto

Source: `std/src/crypto.i`. Implements SHA-2, HMAC, key derivation, byte comparison and secure random bytes.

Export visibility: `CryptoError(..)`. Hidden representations below are descriptive only.

```text
CryptoError = InvalidLength | InvalidIterations | UnalignedInput
InvalidLength = ()
InvalidIterations = ()
UnalignedInput = ()
```

```text
sha256 : (BitArray) -> BitArray
sha512 : (BitArray) -> BitArray
hmac_sha256 : (BitArray, BitArray) -> Result( BitArray, CryptoError, )
hmac_sha512 : (BitArray, BitArray) -> Result( BitArray, CryptoError, )
hkdf_sha256 : ( BitArray, BitArray, BitArray, Int ) -> Result(BitArray, CryptoError)
hkdf_sha512 : ( BitArray, BitArray, BitArray, Int ) -> Result(BitArray, CryptoError)
hkdf_extract_sha256 : (BitArray, BitArray) -> Result( BitArray, CryptoError, )
hkdf_extract_sha512 : (BitArray, BitArray) -> Result( BitArray, CryptoError, )
hkdf_expand_sha256 : (BitArray, BitArray, Int) -> Result( BitArray, CryptoError, )
hkdf_expand_sha512 : (BitArray, BitArray, Int) -> Result( BitArray, CryptoError, )
pbkdf2_sha256 : ( BitArray, BitArray, Int, Int ) -> Result(BitArray, CryptoError)
constant_time_equal : (BitArray, BitArray) -> Bool
random_bytes : (Int) <- (process.Entropy, Fail(process.ProcessError), request.Await, request.Owner) -> Result(BitArray, CryptoError)
```

**Missing attached type/constructor docs.** `CryptoError`, `InvalidIterations`, `InvalidLength`, `UnalignedInput`. Module prose may still describe them.

**Language use.** Uses immutable bit/Int algorithms, CryptoError unions and Result. random_bytes also inherits Entropy, request and Fail(ProcessError) needs. Use Fail for invalid lengths, iterations and alignment in E4. Keep deterministic hashing pure and secure entropy separate from i/random.

**Old style, overlap and thin surface.** random_bytes exposes two failure styles in one call. HMAC takes key before input; extract takes salt before input; other KDF calls put input first. constant_time_equal documents a source control-flow property, not a host timing guarantee.

**Tests.** 88 named tests: `std/test/crypto_test.i` (88). 88 named tests include vectors and parameter errors. hkdf_expand_sha512 has no direct reference in crypto_test.i, although hkdf_sha512 reaches it. No runtime timing guarantee or complete cross-provider entropy cancellation proof follows from these tests.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### std/csv

Source: `std/src/csv.i`. Parses and prints CSV and lazily decodes chunk streams into rows or records.

Export visibility: `Options`, `CsvError(..)`. Hidden representations below are descriptive only.

```text
Options = (delimiter = Int, fields = Int, crlf = Bool)
CsvError = (line = Int, column = Int, reason = String)
```

```text
options : (String, Int, Bool) -> Result( Options, CsvError, )
defaults : () -> Options
parse : (String) -> Result(List(List(String)), CsvError)
parse_with : (String, Options) -> Result( List(List(String)), CsvError, )
parse_header : (String, Options) -> Result( List(dict.Dict(String, String)), CsvError, )
print : (List(List(String))) -> String
print_with : (List(List(String)), Options) -> String
rows : <E, N>( stream.Stream(String, E, N), Options ) -> stream.Stream(Result(List(String), CsvError), E, N)
records : <E, N>( stream.Stream(String, E, N), Options ) -> stream.Stream(Result(dict.Dict(String, String), CsvError), E, N)
decode : <A, E>( schema.Schema(A, E), dict.Dict(String, String) ) -> Result(A, E)
```

**Missing attached type/constructor docs.** `CsvError`, `Options`. Module prose may still describe them.

**Language use.** Uses labelled Options/CsvError, immutable parser state and Stream. Source errors use Fail, but CSV row errors become Result elements. E4 can merge CsvError into the pull function need, with attempt for callers who want error elements. parse/parse_with and print/print_with are natural shape-group pairs.

**Old style, overlap and thin surface.** The parser is substantial and uses demand carefully. decode takes the codec before the row. Its Result duplicates schema.decode. One place should own this conversion. The module documents positions and line endings well.

**Tests.** 107 named tests: `std/test/csv_test.i` (107). 107 named tests cover strict parsing, width, Unicode, many chunk boundaries, demand and source errors. All exports are referenced. There is no unified Fail(CsvError) pull API or reusable IO-reader integration test yet.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### std/dict

Source: `std/src/dict.i`. Implements immutable hash-trie dictionaries and transformations.

Export visibility: `Dict`. Hidden representations below are descriptive only.

```text
Dict(K, V) = (Int, Node(K, V))
```

```text
new : <K, V>() -> Dict(K, V)
size : <K, V>(Dict(K, V)) -> Int
is_empty : <K, V>(Dict(K, V)) -> Bool
get : <K, V>(Dict(K, V), K) -> Option(V)
has : <K, V>(Dict(K, V), K) -> Bool
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
```

**Language use.** Uses hidden nominal Dict, internal unions, equality/hash primitives, Option absence and need-polymorphic callbacks. It already fits immutable i well. More service machinery is unnecessary. Collision equality and iteration order need stable documented contracts.

**Old style, overlap and thin surface.** group is also exported by std/list, which delegates here. Private reverse_onto duplicates list helpers to avoid the dependency cycle. Structural equality can distinguish collision insertion orders, as the module comment states.

**Tests.** 97 named tests: `std/test/dict_test.i` (97). 97 named tests reference every export and cover persistence and collisions. They do not establish extensional equality for every collision history; the documented representation exception remains. No E4 error migration is needed for ordinary missing-key Option.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### std/float

Source: `std/src/float.i`. Parses finite binary64 floats and supplies arithmetic and elementary functions.

Exported types: none.

```text
parse : (String) -> Result(Float, Nil)
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
power : (Float, Int) -> Result(Float, Nil)
real_power : (Float, Float) -> Result(Float, Nil)
exponential : (Float) -> Result(Float, Nil)
logarithm : (Float) -> Result(Float, Nil)
square_root : (Float) -> Result(Float, Nil)
sum : (List(Float)) -> Float
product : (List(Float)) -> Float
divide : (Float, Float) -> Result(Float, Nil)
modulo : (Float, Float) -> Result(Float, Nil)
add : (Float, Float) -> Float
subtract : (Float, Float) -> Float
multiply : (Float, Float) -> Float
is_integer : (Float) -> Bool
max_finite : Float
min_positive : Float
pi : Float
tau : Float
sin : (Float) -> Float
cos : (Float) -> Float
tan : (Float) -> Result(Float, Nil)
sign_bit : (Float) -> Bool
copy_sign : (Float, Float) -> Float
asin : (Float) -> Result(Float, Nil)
acos : (Float) -> Result(Float, Nil)
atan : (Float) -> Float
atan2 : (Float, Float) -> Float
hypot : (Float, Float) -> Result(Float, Nil)
radians : (Float) -> Float
degrees : (Float) -> Result(Float, Nil)
```

**Language use.** Uses core primitives, exact Int helpers, pattern matching, Order and Result for domain/overflow errors. E4 should use Fail for domain checks and parsing; use meaningful errors rather than Nil where callers need a distinction. Keep pure successful arithmetic ordinary.

**Old style, overlap and thin surface.** Several functions rename core or operators. to_string delegates to core.inspect. The elementary algorithms are substantial. max_finite/min_positive/pi/tau are values, not functions. The header still says errors are Result.

**Tests.** 337 named tests: `std/test/float_test.i` (337). 337 named tests reference every export, with edge values and property checks. This audit does not establish correctly rounded transcendental functions over all finite inputs. New typed Fail errors have no tests yet.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### std/http

Source: `std/src/http.i`. Implements HTTP/1.1 framing, clients, routing and scoped servers.

Export visibility: `Client(..)`, `Transport(..)`, `ResponseStream`, `Method(..)`, `Status(..)`, `Headers(..)`, `Request(..)`, `Response(..)`, `HttpError(..)`, `Progress(..)`, `BodyError(..)`. Hidden representations below are descriptive only.

```text
Client = ( make = () <- ( requests.Await, requests.Owner, task.Scheduler, task.Runner, ) -> Transport, )
Transport = ( stream = (Request) <- ( Fail(HttpError), requests.Await, requests.Owner, task.Scheduler, task.Runner, ) -> ResponseStream, send = (Request) <- ( Fail(HttpError), requests.Await, requests.Owner, task.Scheduler, task.Runner, ) -> Response, close = () <- ( requests.Await, requests.Owner, task.Scheduler, task.Runner, ) -> Nil, )
ResponseStream = { ( head = Response, read = (Int) <- ( Fail(HttpError), requests.Await, requests.Owner, task.Scheduler, task.Runner, ) -> BitArray, close = () <- ( requests.Await, requests.Owner, task.Scheduler, task.Runner, ) -> Nil, ) }
Method = (String)
Status = (Int)
Headers = (List((String, String)))
Headers = Headers | RawHeaders
RawHeaders = (List((String, BitArray)))
Request = ( method = Method, target = String, headers = Headers, body = BitArray, trailers = Headers, )
Response = ( status = Status, reason = String, headers = Headers, body = BitArray, trailers = Headers, )
HttpError = NeedMore | BadMessage | Dns | Connect | Tls | Protocol | Timeout | StatusError
NeedMore = ()
BadMessage = (String)
Dns = (String)
Connect = (String)
Tls = (String)
Protocol = (String)
Timeout = ()
StatusError = (Int)
Progress(A) = More | Complete(A) | Invalid
More = (BitArray)
Complete(A) = (A, BitArray)
Invalid = (String)
BodyError = InvalidText | InvalidJson | InvalidSchema
InvalidText = ()
InvalidJson = (json.JsonError)
InvalidSchema = (schema.SchemaError)
```

```text
native_client : () <- (net.Net, tls.Tls) -> Client
fetch_client : () <- (fetch.Fetch) -> Client
send_stream : (Request) <- (Client, Fail(HttpError), requests.Await, requests.Owner, task.Scheduler, task.Runner) -> ResponseStream
request : (Method, String) -> Request
response : (Status) -> Response
feed_request : (BitArray, BitArray) -> Progress(Request)
feed_response : (BitArray, BitArray) -> Progress(Response)
feed_response_for : ( BitArray, BitArray, Method, Bool, Int ) -> Progress(Response)
feed_request_limit : ( BitArray, BitArray, Int ) -> Progress(Request)
header : (Headers, String) -> Option(String)
header_bytes : (Headers, String) -> Option(BitArray)
headers : (Headers, String) -> List(String)
add_header : (Headers, String, String) -> Headers
serialize_request : (Request) -> BitArray
serialize_response : (Response) -> BitArray
reason : (Int) -> String
parse_query : (String) -> Result( List((String, String)), uri.CodecError, )
build_query : (List((String, String))) -> String
serve : (net.Listener, (Request) <- (Fail(E), ..N) -> Response) <- (net.Net, Fail(net.NetError), task.Scheduler, N - requests.Await - task.Runner - task.Scheduler - requests.Owner) -> Never
serve_stream : (net.Listener, (Request, BodyReader) <- (Fail(E), ..N) -> Response) <- (net.Net, Fail(net.NetError), task.Scheduler, N - requests.Await - task.Runner - task.Scheduler - requests.Owner) -> Never
get : (String) <- (Client, Fail(HttpError), requests.Await, requests.Owner, task.Scheduler, task.Runner) -> Response
post : (String, BitArray) <- (Client, Fail(HttpError), requests.Await, requests.Owner, task.Scheduler, task.Runner) -> Response
send : (Request) <- (Client, Fail(HttpError), requests.Await, requests.Owner, task.Scheduler, task.Runner) -> Response
client : () <- (Client, requests.Await, requests.Owner, task.Scheduler, task.Runner) -> ClientFunctions
body_text : (Response) -> Result(String, Nil)
body_json : (Response) -> Result(json.Json, BodyError)
body_decode : <A>( Response, schema.Schema(A, schema.SchemaError) ) -> Result(A, BodyError)
send_with : <N>((Request) <- N -> Response, Request) <- N -> Response
send_timeout : <N>((Request) <- (Fail(HttpError), ..N) -> Response, Request, Int) <- (Fail(HttpError | clock.ClockError), clock.Clock, task.Scheduler, N - requests.Await - task.Runner - task.Scheduler - requests.Owner) -> Response
route : (Method, String, Request) -> Option( List((String, String)), )
content_length : String
content_type : String
transfer_encoding : String
host : String
connection : String
```

**Missing attached type/constructor docs.** `BadMessage`, `BodyError`, `Client`, `Complete`, `Connect`, `Dns`, `Headers`, `HttpError`, `Invalid`, `InvalidJson`, `InvalidSchema`, `InvalidText`, `Method`, `More`, `NeedMore`, `Progress`, `Protocol`, `RawHeaders`, `Request`, `Response`, `ResponseStream`, `Status`, `StatusError`, `Timeout`, `Tls`, `Transport`. Module prose may still describe them.

**Language use.** Uses labelled message data, unions, Fail, Client provision, request/task services, scheduler Ref and resources. It mixes that with Result-returning body and query decoders. Unify decoders under Fail. Give streamed response bodies an explicit resource lifetime. Reuse generic timeout/schedule policy and byte transport contracts. Keep protocol progress as data.

**Old style, overlap and thin surface.** native_client.stream calls provider.send first and stores res.body in task.ref: it buffers the entire response. send_stream runs provider.close before returning its reader. Headers is defined both nominally and as a union. ClientFunctions is a named source type returned by public client but is not explicitly exported. Many item docs are missing.

**Tests.** 66 named tests: `std/test/http_network_test.i` (2), `std/test/http_test.i` (64). 64 framing/provider tests plus 2 network tests under std/test. crates/ilang-cli/tests/http.rs adds 11 tests for servers, timeout, DNS, redirect rules, reuse, request streaming and origin separation. Body decoder exports and native/fetch streaming lifetimes lack direct dedicated std calls. Native response demand before full-body completion is not tested by the request-streaming case.

**Missing item documentation.** `native_client`, `fetch_client`, `send_stream`, `request`, `response`, `feed_request`, `feed_response`, `feed_request_limit`, `header`, `header_bytes`, `headers`, `add_header`, `serialize_request`, `serialize_response`, `reason`, `parse_query`, `build_query`, `get`, `post`, `send`, `client`, `body_text`, `body_json`, `body_decode`, `content_length`, `content_type`, `transfer_encoding`, `host`, `connection`.

### std/int

Source: `std/src/int.i`. Parses arbitrary precision integers and provides checked arithmetic and bit operations.

Exported types: none.

```text
to_string : (Int) -> String
parse : (String) -> Result(Int, Nil)
parse_base : (String, Int) -> Result(Int, Nil)
to_base : (Int, Int) -> Result(String, Nil)
to_base2 : (Int) -> String
to_base8 : (Int) -> String
to_base16 : (Int) -> String
to_base36 : (Int) -> String
to_float : (Int) -> Float
abs : (Int) -> Int
negate : (Int) -> Int
min : (Int, Int) -> Int
max : (Int, Int) -> Int
clamp : (Int, Int, Int) -> Int
compare : (Int, Int) -> Order
is_even : (Int) -> Bool
is_odd : (Int) -> Bool
power : (Int, Int) -> Result(Int, Nil)
square_root : (Int) -> Result(Float, Nil)
sum : (List(Int)) -> Int
product : (List(Int)) -> Int
digits : (Int, Int) -> Result(List(Int), Nil)
undigits : (List(Int), Int) -> Result(Int, Nil)
divide : (Int, Int) -> Result(Int, Nil)
remainder : (Int, Int) -> Result(Int, Nil)
modulo : (Int, Int) -> Result(Int, Nil)
floor_divide : (Int, Int) -> Result(Int, Nil)
add : (Int, Int) -> Int
subtract : (Int, Int) -> Int
multiply : (Int, Int) -> Int
bitwise_and : (Int, Int) -> Int
bitwise_or : (Int, Int) -> Int
bitwise_xor : (Int, Int) -> Int
bitwise_not : (Int) -> Int
bitwise_shift_left : (Int, Int) -> Int
bitwise_shift_right : (Int, Int) -> Int
range : <Acc, N>( Int, Int, Acc, (Acc, Int) <- N -> Acc ) <- N -> Acc
```

**Language use.** Uses exact Int, bit syntax, Order, Result and an open-row range callback. E4 should report parse, radix, exponent and divisor errors through Fail. Constructors and safe operations remain pure.

**Old style, overlap and thin surface.** to_float and bitwise_* rename core; add/subtract/multiply rename operators. to_string uses inspect. to_base2/8/16/36 are specialized names over to_base; optional radix shapes could reduce API repetition.

**Tests.** 180 named tests: `std/test/int_test.i` (180). 180 named tests reference every export and cover radix, signs, large values and arithmetic errors. They test Result contracts, not the planned Fail API. Range callback row composition is present in source; count is not a proof for every service combination.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### std/json

Source: `std/src/json.i`. Parses, prints and inspects an immutable JSON value tree.

Export visibility: `Json(..)`, `JsonError(..)`. Hidden representations below are descriptive only.

```text
Json = Null | Boolean | Integer | Number | Text | Array | Object
Null = ()
Boolean = (Bool)
Integer = (Int)
Number = (Float)
Text = (String)
Array = (List(Json))
Object = (List((String, Json)))
JsonError = (offset = Int, reason = String)
```

```text
parse : (String) -> Result(Json, JsonError)
print : (Json) -> String
pretty : (Json, Int) -> String
field : (Json, String) -> Option(Json)
index : (Json, Int) -> Option(Json)
as_bool : (Json) -> Option(Bool)
as_int : (Json) -> Option(Int)
as_float : (Json) -> Option(Float)
as_string : (Json) -> Option(String)
as_array : (Json) -> Option(List(Json))
as_object : (Json) -> Option(List((String, Json)))
```

**Missing attached type/constructor docs.** `Array`, `Boolean`, `Integer`, `Json`, `JsonError`, `Null`, `Number`, `Object`, `Text`. Module prose may still describe them.

**Language use.** Uses labelled variants, labelled JsonError, Option queries and internal Fail caught into Result parsing. E4 should expose parse through Fail(JsonError). Shape groups can unify print/pretty without a new runtime mechanism. Macros can derive codecs for closed labelled data through syntax services.

**Old style, overlap and thin surface.** The parser is real i code. Json wraps Bool/Int/Float/String even though their built-in types are distinguishable union members. Consider direct scalar members, following the Node migration rule in syntax-3.md §2.8; Array/Object still need distinct tags because both are list-backed. Changing those members also changes whole-sum export syntax. All public functions lack item docs. Json object data is an ordered list of pairs, not Dict, so duplicate/order policy must be explicit.

**Tests.** 78 named tests: `std/test/json_test.i` (78). 78 named tests cover parser/printer cases. as_bool/as_int/as_float/as_string/as_array/as_object have no direct references in json_test.i. Duplicate-field policy and decode-query negative cases need visible API documentation and targeted tests.

**Missing item documentation.** `parse`, `print`, `pretty`, `field`, `index`, `as_bool`, `as_int`, `as_float`, `as_string`, `as_array`, `as_object`.

### std/list

Source: `std/src/list.i`. Provides eager immutable list queries, transformations and combinatorics.

Export visibility: `ContinueOrStop(..)`. Hidden representations below are descriptive only.

```text
ContinueOrStop(A) = Continue(A) | Stop(A)
Continue(A) = (A)
Stop(A) = (A)
```

```text
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
group : <V, K, N>(List(V), (V) <- N -> K) <- N -> Dict(K, List(V))
filter : <A, N>(List(A), (A) <- N -> Bool) <- N -> List(A)
filter_map : <A, B, N>(List(A), (A) <- N -> Option(B)) <- N -> List(B)
map : <A, B, N>(List(A), (A) <- N -> B) <- N -> List(B)
map2 : <A, B, C, N>( List(A), List(B), (A, B) <- N -> C ) <- N -> List(C)
map_fold : <A, Acc, B, N>( List(A), Acc, (Acc, A) <- N -> (Acc, B) ) <- N -> (Acc, List(B))
index_map : <A, B, N>(List(A), (A, Int) <- N -> B) <- N -> List(B)
try_map : <A, B, F, N>(List(A), (A) <- N -> Result(B, F)) <- N -> Result( List(B), F, )
drop : <A>(List(A), Int) -> List(A)
take : <A>(List(A), Int) -> List(A)
new : <A>() -> List(A)
wrap : <A>(A) -> List(A)
append : <A>(List(A), List(A)) -> List(A)
prepend : <A>(List(A), A) -> List(A)
concat : <A>(List(List(A))) -> List(A)
flat_map : <A, B, N>(List(A), (A) <- N -> List(B)) <- N -> List(B)
fold : <A, Acc, N>( List(A), Acc, (Acc, A) <- N -> Acc ) <- N -> Acc
fold_right : <A, Acc, N>( List(A), Acc, (Acc, A) <- N -> Acc ) <- N -> Acc
index_fold : <A, Acc, N>( List(A), Acc, (Acc, A, Int) <- N -> Acc ) <- N -> Acc
try_fold : <A, Acc, F, N>( List(A), Acc, (Acc, A) <- N -> Result(Acc, F) ) <- N -> Result(Acc, F)
fold_until : <A, Acc, N>( List(A), Acc, (Acc, A) <- N -> ContinueOrStop(Acc) ) <- N -> Acc
find : <A, N>(List(A), (A) <- N -> Bool) <- N -> Option(A)
find_map : <A, B, N>(List(A), (A) <- N -> Option(B)) <- N -> Option(B)
all : <A, N>(List(A), (A) <- N -> Bool) <- N -> Bool
any : <A, N>(List(A), (A) <- N -> Bool) <- N -> Bool
zip : <A, B>(List(A), List(B)) -> List((A, B))
strict_zip : <A, B>(List(A), List(B)) -> Result(List((A, B)), Nil)
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
try_each : <A, B, F, N>( List(A), (A) <- N -> Result(B, F) ) <- N -> Result(Nil, F)
partition : <A, N>(List(A), (A) <- N -> Bool) <- N -> ( List(A), List(A), )
permutations : <A>(List(A)) -> List(List(A))
window : <A>(List(A), Int) -> List(List(A))
window_by_2 : <A>(List(A)) -> List((A, A))
drop_while : <A, N>(List(A), (A) <- N -> Bool) <- N -> List(A)
take_while : <A, N>(List(A), (A) <- N -> Bool) <- N -> List(A)
chunk : <A, K, N>(List(A), (A) <- N -> K) <- N -> List(List(A))
sized_chunk : <A>(List(A), Int) -> List(List(A))
reduce : <A, N>(List(A), (A, A) <- N -> A) <- N -> Option(A)
scan : <A, Acc, N>( List(A), Acc, (Acc, A) <- N -> Acc ) <- N -> List(Acc)
combinations : <A>(List(A), Int) -> List(List(A))
combination_pairs : <A>(List(A)) -> List((A, A))
interleave : <A>(List(List(A))) -> List(A)
transpose : <A>(List(List(A))) -> List(List(A))
max : <A, N>(List(A), (A, A) <- N -> Order) <- N -> Option(A)
min : <A, N>(List(A), (A, A) <- N -> Order) <- N -> Option(A)
```

**Missing attached type/constructor docs.** `Continue`, `Stop`. Module prose may still describe them.

**Language use.** Uses native list patterns, Option absence, Continue/Stop unions and callbacks whose needs propagate, including Fail. Ordinary map/fold/each can already raise through callback needs. E4 should avoid parallel try_* control mechanisms. Defaults and comparator variants are candidates for shape groups.

**Old style, overlap and thin surface.** try_map/try_fold/try_each reimplement short-circuiting for Result callbacks. group forwards dict.group. new/wrap and prepend wrap literal syntax but can help higher-order use. Comments still show ^ E ! N and an obsolete dict literal example.

**Tests.** 269 named tests: `std/test/list_test.i` (269). 269 named tests reference every export. They include million-step tail calls, callback error handling and order-sensitive cases. New consolidated Fail and shape-group APIs are not covered; public count alone does not imply every combinatorial size is stress-tested.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### std/log

Source: `std/src/log.i`. Delivers structured entries through a call-time Sink service.

Export visibility: `Level(..)`, `Entry(..)`, `Sink`. Hidden representations below are descriptive only.

```text
Level = Debug | Info | Warning | Failure
Debug = ()
Info = ()
Warning = ()
Failure = ()
Entry = ( level = Level, message = String, fields = List((String, String)), at = Option(time.Instant), )
Sink(E, N) = ((Level) -> Bool, (Entry) <- (Fail(E), ..N) -> Nil, (N) -> Nil)
```

```text
make : <E, N>( (Level) -> Bool, (Entry) <- (Fail(E), ..N) -> Nil ) -> Sink(E, N)
silent : <N>() -> Sink(Never, N)
filter : <E, N>(Sink(E, N), (Level) -> Bool) -> Sink(E, N)
minimum : <E, N>(Sink(E, N), Level) -> Sink(E, N)
enabled : <E, N>(Level) <- (Sink(E, N)) -> Bool
write : <E, N>(Entry) <- (Sink(E, N), Fail(E), ..N) -> Nil
lazy : <E, N>(Level, () <- N -> String) <- (Sink(E, N), Fail(E), ..N) -> Nil
format : (Entry) -> String
stderr : () -> Sink(stdio.StdioError, stdio.Stdio | request.Await | request.Owner)
stdout : () -> Sink(stdio.StdioError, stdio.Stdio | request.Await | request.Owner)
file : (String) -> Sink(fs.FsError, fs.Fs | request.Await | request.Owner)
timestamped : (Entry) <- (clock.Clock, Fail(clock.ClockError), request.Await, request.Owner) -> Entry
```

**Missing attached type/constructor docs.** `Debug`, `Entry`, `Failure`, `Info`, `Level`, `Sink`, `Warning`. Module prose may still describe them.

**Language use.** Uses labelled Entry/Level, a hidden Sink, need-polymorphic callbacks, Fail, lazy filtering and optional clock timestamps. File output uses resource.acquire. This is already a useful ordinary service. Capture downstream services with the language rules. Prefer one documented logging failure policy and Scope lifetime over an independent environment abstraction.

**Old style, overlap and thin surface.** Sink keeps an unused (N)->Nil variance marker. Old header types use ?/! and obsolete labelled declarations. stdout/file/timestamped are useful adapters but have little direct dedicated coverage. Failure is a log level name, not Fail.

**Tests.** 11 named tests: `std/test/log_test.i` (11). 11 named tests cover delivery, laziness, filters, errors, formatting and generic service gates. clock_test covers timestamped; cli_log.rs has 2 integration cases using stderr. stdout and file have no direct log_test.i calls. File-open/write/flush cleanup failures need isolated cases.

**Missing item documentation.** `make`, `silent`, `filter`, `minimum`, `enabled`, `write`, `lazy`, `format`.

### std/option

Source: `std/src/option.i`. Represents absence and transforms optional values.

Export visibility: `Option(..)`. Hidden representations below are descriptive only.

```text
Option(A) = Some(A) | None
Some(A) = (A)
None = ()
```

```text
is_some : <A>(Option(A)) -> Bool
is_none : <A>(Option(A)) -> Bool
map : <A, B, N>(Option(A), (A) <- N -> B) <- N -> Option(B)
then : <A, B, N>(Option(A), (A) <- N -> Option(B)) <- N -> Option(B)
flatten : <A>(Option(Option(A))) -> Option(A)
unwrap : <A>(Option(A), A) -> A
lazy_unwrap : <A, N>(Option(A), () <- N -> A) <- N -> A
or : <A>(Option(A), Option(A)) -> Option(A)
lazy_or : <A, N>(Option(A), () <- N -> Option(A)) <- N -> Option(A)
to_result : <A, E>(Option(A), E) -> Result(A, E)
from_result : <A, E>(Result(A, E)) -> Option(A)
all : <A>(List(Option(A))) -> Option(List(A))
values : <A>(List(Option(A))) -> List(A)
filter : <A, N>(Option(A), (A) <- N -> Bool) <- N -> Option(A)
map2 : <A, B, C, N>( Option(A), Option(B), (A, B) <- N -> C ) <- N -> Option(C)
```

**Missing attached type/constructor docs.** `None`, `Some`. Module prose may still describe them.

**Language use.** Uses a one-constructor Some, nullary None, their union and need-polymorphic callbacks. Keep ordinary absence as Option. Failure conversion belongs in one bridge location. Lazy/eager pairs could share shape-selected APIs where types stay clear.

**Old style, overlap and thin surface.** Most operations are useful data operations. to_result/from_result overlap std/result.from_option/to_option. A private reverse duplicates list traversal because imports would cycle. Its header still claims all fallible std calls return Result.

**Tests.** 40 named tests: `std/test/option_test.i` (40). 40 named tests reference every export and cover None/Some branches and callback behavior. No new failure policy is needed for ordinary optional data. Future unified bridge names have no tests.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### std/order

Source: `std/src/order.i`. Represents comparison results and combines ordering decisions.

Export visibility: `Order(..)`. Hidden representations below are descriptive only.

```text
Order = Less | Equal | Greater
Less = ()
Equal = ()
Greater = ()
```

```text
negate : (Order) -> Order
reverse : <A, N>((A, A) <- N -> Order) -> (A, A) <- N -> Order
to_int : (Order) -> Int
compare : (Order, Order) -> Order
max : (Order, Order) -> Order
min : (Order, Order) -> Order
break_tie : (Order, Order) -> Order
lazy_break_tie : <N>(Order, () <- N -> Order) <- N -> Order
is_less : (Order) -> Bool
is_equal : (Order) -> Bool
is_greater : (Order) -> Bool
```

**Missing attached type/constructor docs.** `Equal`, `Greater`, `Less`. Module prose may still describe them.

**Language use.** Uses the Less/Equal/Greater union, a lazy callback need and a deferred comparator wrapper. It appropriately needs no own service, task or resource system. Shape groups could combine break_tie and lazy_break_tie.

**Old style, overlap and thin surface.** The three-way comparison value is useful. max/min/compare on Order are a broad convenience surface. reverse transforms a comparator, while negate transforms an Order; that distinction needs to remain clear.

**Tests.** 30 named tests: `std/test/order_test.i` (30). 30 named tests reference every export. No missing direct export reference was found. Callback-service composition under the future merged lazy API is not tested because that API does not exist.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### std/pair

Source: `std/src/pair.i`. Constructs, reads and maps ordinary two-position data.

Exported types: none.

```text
new : <A, B>(A, B) -> (A, B)
first : <A, B>((A, B)) -> A
second : <A, B>((A, B)) -> B
swap : <A, B>((A, B)) -> (B, A)
map_first : <A, B, C, N>((A, B), (A) <- N -> C) <- N -> (C, B)
map_second : <A, B, C, N>((A, B), (B) <- N -> C) <- N -> (A, C)
```

**Language use.** Uses positional data/destructuring and callbacks with arbitrary needs. No services or nominal pair type are needed. The mapping functions are useful when passing a function value.

**Old style, overlap and thin surface.** new/first/second restate (a,b), .0 and .1. swap/map_first/map_second add composition. The API resembles `gleam-stdlib/src/gleam/pair.gleam`; it already propagates needs rather than imposing purity.

**Tests.** 14 named tests: `std/test/pair_test.i` (14). 14 named tests reference every export. No dedicated missing branch exists for this small total API. The audit does not infer arbitrary callback-service coverage from those examples.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### std/path

Source: `std/src/path.i`. Manipulates lexical slash-separated paths without host IO.

Export visibility: `RelativeError(..)`. Hidden representations below are descriptive only.

```text
RelativeError = DifferentRoots | UnknownParent
DifferentRoots = ()
UnknownParent = ()
```

```text
clean : (String) -> String
join : (List(String)) -> String
split : (String) -> (String, String)
base : (String) -> String
dir : (String) -> String
extension : (String) -> String
is_absolute : (String) -> Bool
relative : (String, String) -> Result(String, RelativeError)
```

**Missing attached type/constructor docs.** `DifferentRoots`, `UnknownParent`. Module prose may still describe them.

**Language use.** Uses String/list matching and a RelativeError union returned as Result. Use Fail for relative-path failure in E4. Keep lexical paths independent of filesystem services. POSIX lexical rules are intentional.

**Old style, overlap and thin surface.** The implementation is substantial and well documented. It overlaps URI path work conceptually but percent escapes, separators and validation have different contracts. join takes all pieces as one List rather than supporting a two-piece shape.

**Tests.** 5 named tests: `std/test/path_test.i` (5). Five named tests include examples, exhaustive small laws and long paths; every export is referenced. Windows drive semantics and filesystem resolution are absent by contract, not missing tests for promised features. The Fail version is not tested.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### std/property

Source: `std/src/property.i`. Generates reproducible values and shrinks failures through immutable lazy trees.

Export visibility: `Gen`, `Tree(..)`, `Config(..)`, `BuildError(..)`, `Verdict(..)`, `Failure(..)`, `Report(..)`. Hidden representations below are descriptive only.

```text
Gen(A) = ((Int, random.Generator) -> Option(Tree(A)))
Tree(A) = (value = A, children = () -> List(Tree(A)))
Config = ( seed = Int, cases = Int, max_size = Int, max_discards = Int, max_shrinks = Int, )
BuildError = InvalidRange | InvalidLength | EmptyChoice
InvalidRange = (Int, Int)
InvalidLength = (Int, Int)
EmptyChoice = ()
Verdict = Pass | Fail | Discard
Pass = ()
Fail = (String)
Discard = ()
Failure(A) = ( seed = Int, attempt = Int, size = Int, original = A, smallest = A, reason = String, path = List(Int), checked = Int, limited = Bool, )
Report(A) = Passed | GaveUp | Falsified(A) | InvalidConfig
Passed = (successes = Int, discards = Int)
GaveUp = (successes = Int, discards = Int)
Falsified(A) = (Failure(A))
InvalidConfig = (String)
```

```text
defaults : () -> Config
make : <A>((Int, random.Generator) -> Option(Tree(A))) -> Gen(A)
sample : <A>(Gen(A), Int, random.Generator) -> Option( Tree(A), )
tree : <A>(A, () -> List(Tree(A))) -> Tree(A)
constant : <A>(A) -> Gen(A)
map : <A, B>(Gen(A), (A) -> B) -> Gen(B)
flat_map : <A, B>(Gen(A), (A) -> Gen(B)) -> Gen(B)
pair : <A, B>(Gen(A), Gen(B)) -> Gen((A, B))
filter : <A>(Gen(A), (A) -> Bool) -> Gen(A)
sized : <A>((Int) -> Gen(A)) -> Gen(A)
resize : <A>(Gen(A), Int) -> Gen(A)
int : (Int, Int) -> Result(Gen(Int), BuildError)
bool : () -> Gen(Bool)
element : <A>(List(A)) -> Result(Gen(A), BuildError)
list : <A>(Gen(A), Int, Int) -> Result(Gen(List(A)), BuildError)
float : () -> Gen(Float)
utf_codepoint : () -> Gen(UtfCodepoint)
string : (Int) -> Result(Gen(String), BuildError)
check : <A>(Gen(A), Config, (A) -> Verdict) -> Report( A, )
replay : <A>( Gen(A), Int, Int, Int, List(Int) ) -> Option(A)
format : <A>(Report(A), (A) -> String) -> String
test : <A>( Gen(A), Config, (A) <- (Assert) -> Nil, (A) -> String ) <- (Assert) -> Nil
```

**Missing attached type/constructor docs.** `BuildError`, `Config`, `Discard`, `EmptyChoice`, `Fail`, `Failure`, `Falsified`, `GaveUp`, `Gen`, `InvalidConfig`, `InvalidLength`, `InvalidRange`, `Pass`, `Passed`, `Report`, `Tree`, `Verdict`. Module prose may still describe them.

**Language use.** Uses hidden Gen, labelled Config/Tree/Failure, Verdict/Report unions, immutable random state and Assert interception. Builder failures should use Fail in E4; Verdict, discard and Report are intentional data. Effectful properties need explicit need rows if supported. Schema-derived generators can be ordinary macros or functions.

**Old style, overlap and thin surface.** Callbacks are deliberately pure except the Assert adapter. property.Fail(String) shadows the prelude service name within the module and can confuse readers. Reports do not imply globally minimal failures. The source explains replay and budget limits well.

**Tests.** 14 named tests: `std/test/property_test.i` (14). 14 named tests reference all exports and cover laws, paths, budgets, discards and shrink order. std/test/property_failures.py defines a separate subprocess adapter check, not run here. Its embedded Config(42,1,0,0,0) still supplies positional fields to a labelled constructor and needs migration. Concurrent/effectful property protocols and schema derivation are absent APIs.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### std/regex

Source: `std/src/regex.i`. Compiles Unicode-scalar regular expressions to immutable NFA programs and searches text.

Export visibility: `Regex`, `RegexError(..)`, `Match(..)`, `Captures`. Hidden representations below are descriptive only.

```text
Regex = ( code = dict.Dict(Int, Inst), start = Int, names = dict.Dict(String, Int), )
RegexError = (position = Int, reason = String)
Match = (start = Int, end = Int, text = String)
Captures = ( slots = dict.Dict(Int, Offset), names = dict.Dict(String, Int), text = String, )
```

```text
compile : (String) -> Result(Regex, RegexError)
is_match : (Regex, String) -> Bool
find : (Regex, String) -> Option(Match)
find_all : (Regex, String) -> List(Match)
captures : (Regex, String) -> Option(Captures)
group : (Captures, Int) -> Option(Match)
named : (Captures, String) -> Option(Match)
replace : (Regex, String, String) -> String
replace_all : (Regex, String, String) -> String
split : (Regex, String) -> List(String)
```

**Missing attached type/constructor docs.** `Captures`, `Match`, `Regex`, `RegexError`. Module prose may still describe them.

**Language use.** Uses labelled errors/matches, hidden Regex/Captures, internal unions and Fail internally; compile catches back into Result. E4 should let compile expose its existing Fail. Search absence should remain Option. Shape groups can express first/all or bounded options only when the result types remain clear.

**Old style, overlap and thin surface.** This is a substantial engine with Unicode tables. Pattern object comes before input, unlike the common data-first transforms. Repeated full searches have a documented O(m*n^2) worst case; do not transfer a single-search bound to every iterator.

**Tests.** 130 named tests: `std/test/regex_test.i` (130). 130 named tests reference all exports and cover Unicode, captures, flags, zero-width results and invalid syntax. Exhaustive Unicode and all adversarial program/input sizes are not proven. A new Fail compile contract remains untested.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### std/result

Source: `std/src/result.i`. Transforms explicit success/error data and bridges caught computations.

Exported types: none.

```text
is_ok : <A, E>(Result(A, E)) -> Bool
is_error : <A, E>(Result(A, E)) -> Bool
map : <A, E, B, N>(Result(A, E), (A) <- N -> B) <- N -> Result(B, E)
map_error : <A, E, F, N>(Result(A, E), (E) <- N -> F) <- N -> Result( A, F, )
then : <A, E, B, N>( Result(A, E), (A) <- N -> Result(B, E) ) <- N -> Result(B, E)
flatten : <A, E>(Result(Result(A, E), E)) -> Result(A, E)
unwrap : <A, E>(Result(A, E), A) -> A
lazy_unwrap : <A, E, N>(Result(A, E), () <- N -> A) <- N -> A
unwrap_error : <A, E>(Result(A, E), E) -> E
unwrap_both : <A>(Result(A, A)) -> A
or : <A, E>(Result(A, E), Result(A, E)) -> Result(A, E)
lazy_or : <A, E, N>( Result(A, E), () <- N -> Result(A, E) ) <- N -> Result(A, E)
all : <A, E>(List(Result(A, E))) -> Result(List(A), E)
partition : <A, E>(List(Result(A, E))) -> (List(A), List(E))
values : <A, E>(List(Result(A, E))) -> List(A)
replace : <A, E, B>(Result(A, E), B) -> Result(B, E)
replace_error : <A, E, F>(Result(A, E), F) -> Result(A, F)
recover : <A, E, F, N>( Result(A, E), (E) <- N -> Result(A, F) ) <- N -> Result(A, F)
from_option : <A, E>(Option(A), E) -> Result(A, E)
to_option : <A, E>(Result(A, E)) -> Option(A)
to_error : <A, E>(Result(A, E)) <- (Fail(E)) -> A
catch : <A, E, N>(() <- (Fail(E), ..N) -> A) <- N -> Result(A, E)
```

**Language use.** Uses Result matching, Option conversions, open callback needs and Fail bridges. Keep data combinators for values after catch. Put raising and catching in one canonical i/fail API. E4 need not delete Result data.

**Old style, overlap and thin surface.** catch forwards fail.catch and to_error forwards fail.from_result. from_option/to_option duplicate option conversions. then/recover are meaningful for stored results but should not become the default computation sequencing protocol.

**Tests.** 55 named tests: `std/test/result_test.i` (55). 55 named tests. unwrap_both has no direct reference in result_test.i. Ordinary result values and lazy branches are covered; a consolidated canonical failure bridge API is not present yet.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### std/schema

Source: `std/src/schema.i`. Builds two-way JSON codecs for validated application data.

Export visibility: `Schema(..)`, `SchemaError(..)`, `Path(..)`, `ParseError(..)`. Hidden representations below are descriptive only.

```text
Schema(A, E) = (read = (json.Json) -> Result(A, E), write = (A) -> json.Json)
SchemaError = (path = List(Path), expected = String)
Path = Field | Index
Field = (String)
Index = (Int)
ParseError = Syntax | Decode
Syntax = (json.JsonError)
Decode = (SchemaError)
```

```text
decode : <A, E>(Schema(A, E), json.Json) -> Result(A, E)
encode : <A, E>(Schema(A, E), A) -> json.Json
parse : <A>(Schema(A, SchemaError), String) -> Result( A, ParseError, )
print : <A, E>(Schema(A, E), A) -> String
int : Schema(Int, SchemaError)
float : Schema(Float, SchemaError)
string : Schema(String, SchemaError)
bool : Schema(Bool, SchemaError)
null : Schema((), SchemaError)
literal_string : <K>(K) -> Schema(K, SchemaError)
literal_int : <K>(K) -> Schema(K, SchemaError)
optional : <A>(Schema(A, SchemaError)) -> Schema(Option(A), SchemaError)
nullable : <A>(Schema(A, SchemaError)) -> Schema(Option(A), SchemaError)
list : <A>(Schema(A, SchemaError)) -> Schema(List(A), SchemaError)
dict : <A>(Schema(A, SchemaError)) -> Schema( ds.Dict(String, A), SchemaError, )
tuple : <A, B>(Schema(A, SchemaError), Schema(B, SchemaError)) -> Schema( (A, B), SchemaError, )
object : <A>(A) -> Schema(A, SchemaError)
field : <A, V>( Schema(A, SchemaError), String, Schema(V, SchemaError), (A) -> V, (A, V) -> A ) -> Schema(A, SchemaError)
optional_field : <A, V>( Schema(A, SchemaError), String, Schema(V, SchemaError), (A) -> Option(V), (A, Option(V)) -> A ) -> Schema(A, SchemaError)
union : <A>( List(Schema(A, SchemaError)), (A) -> Int ) -> Schema(A, SchemaError)
tagged_union : <A>( String, List((String, Schema(A, SchemaError))), (A) -> String ) -> Schema(A, SchemaError)
map : <A, B>( Schema(A, SchemaError), (A) -> B, (B) -> A ) -> Schema(B, SchemaError)
transform : <A, B>( Schema(A, SchemaError), (A) -> B, (B) -> A ) -> Schema(B, SchemaError)
refine : <A>( Schema(A, SchemaError), (A) -> Bool, String ) -> Schema(A, SchemaError)
lazy : <A>(() -> Schema(A, SchemaError)) -> Schema(A, SchemaError)
```

**Missing attached type/constructor docs.** `Decode`, `Field`, `Index`, `ParseError`, `Path`, `Schema`, `SchemaError`, `Syntax`. Module prose may still describe them.

**Language use.** Uses a labelled Schema of read/write functions, labelled path errors, unions and ordinary callback composition. All codec callbacks are closed pure functions; decode uses Result. Use native Fail for decode/parse and needs for effectful transforms if supported. Macros through i/syntax can derive closed-data codecs without compiler privilege. Keep one transform name.

**Old style, overlap and thin surface.** optional aliases nullable exactly, despite different missing-field meanings implied by the names; optional_field is separate. transform aliases map. Schema and first-argument codecs make piping decoded data awkward. All 25 public values lack item docs.

**Tests.** 52 named tests: `std/test/schema_test.i` (52). 52 named tests reference all exports, including recursive codecs, tagged cases, paths and literal types. They do not cover effectful codec needs or macro derivation, which are absent. The Result-to-Fail migration needs caller and test updates.

**Missing item documentation.** `decode`, `encode`, `parse`, `print`, `int`, `float`, `string`, `bool`, `null`, `literal_string`, `literal_int`, `optional`, `nullable`, `list`, `dict`, `tuple`, `object`, `field`, `optional_field`, `union`, `tagged_union`, `map`, `transform`, `refine`, `lazy`.

### std/sequence

Source: `std/src/sequence.i`. Provides persistent indexed deques using measured finger trees.

Export visibility: `Sequence`. Hidden representations below are descriptive only.

```text
Sequence(A) = (Tree(A))
```

```text
new : <A>() -> Sequence(A)
wrap : <A>(A) -> Sequence(A)
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
equal : <A>(Sequence(A), Sequence(A)) -> Bool
```

**Language use.** Uses hidden nominal Sequence, internal unions/cached measures, Option queries and need-polymorphic callbacks. Keep this distinct from lists for end edits and indexed access. No special effects or resource layer is needed.

**Old style, overlap and thin surface.** This is substantial i code, with documented worst-case bounds. equal is elementwise, while ordinary ==/hash/inspect observe the private tree shape. This can surprise Dict/Set users; conversion to List is the documented key workaround.

**Tests.** 8 named tests: `std/test/sequence_test.i` (8). Eight named tests reference all exports and include small exhaustive cases, persistent models, large boundaries, callback rows and independent histories. They do not turn ordinary == into extensional sequence equality; that is an API design gap, not a missing passing assertion.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### std/set

Source: `std/src/set.i`. Implements immutable mathematical set operations over Dict keys.

Export visibility: `Set`. Hidden representations below are descriptive only.

```text
Set(A) = (Dict(A, Nil))
```

```text
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
```

**Language use.** Uses a hidden one-constructor Set, immutable Dict, equality/hash and need-polymorphic callbacks. The thin representation layer is useful; it enforces set semantics rather than just renaming a primitive. No service machinery is needed.

**Old style, overlap and thin surface.** Dict collision-order equality can leak through the Set representation. contains uses a different membership verb from dict.has. new/from_list and algebraic operations are appropriate domain functions.

**Tests.** 31 named tests: `std/test/set_test.i` (31). 31 named tests reference every export and cover algebraic cases. Collision-history extensional equality is not guaranteed by the underlying representation. Future equality policy needs explicit tests across independently built sets.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### std/stream

Source: `std/src/stream.i`. Represents replayable lazy sequences whose pull functions can need services and fail.

Export visibility: `Stream`, `Step(..)`. Hidden representations below are descriptive only.

```text
Stream(A, E, N) = (() <- (Fail(E), ..N) -> Step(A, E, N), (N) -> Nil)
Step(A, E, N) = Done | Next(A, E, N)
Done = ()
Next(A, E, N) = (A, Stream(A, E, N))
```

```text
new : <A, N>() -> Stream(A, Never, N)
wrap : <A, N>(A) -> Stream(A, Never, N)
from_list : <A, N>(List(A)) -> Stream(A, Never, N)
make : <A, E, N>(() <- (Fail(E), ..N) -> Step(A, E, N)) -> Stream( A, E, N, )
defer : <A, E, N>(() <- (Fail(E), ..N) -> Stream(A, E, N)) -> Stream( A, E, N, )
unfold : <S, A, E, N>( S, (S) <- (Fail(E), ..N) -> Option((A, S)) ) -> Stream(A, E, N)
pull : <A, E, N>(Stream(A, E, N)) <- (Fail(E), ..N) -> Step(A, E, N)
map : <A, B, E, F, N>( Stream(A, E, N), (A) <- (Fail(F), ..N) -> B ) -> Stream(B, E | F, N)
filter : <A, E, F, N>( Stream(A, E, N), (A) <- (Fail(F), ..N) -> Bool ) -> Stream(A, E | F, N)
filter_map : <A, B, E, F, N>( Stream(A, E, N), (A) <- (Fail(F), ..N) -> Option(B) ) -> Stream(B, E | F, N)
flat_map : <A, B, E, F, G, N>( Stream(A, E, N), (A) <- (Fail(G), ..N) -> Stream(B, F, N) ) -> Stream(B, E | F | G, N)
append : <A, E, F, N>( Stream(A, E, N), Stream(A, F, N) ) -> Stream(A, E | F, N)
zip : <A, B, E, F, N>( Stream(A, E, N), Stream(B, F, N) ) -> Stream((A, B), E | F, N)
take : <A, E, N>(Stream(A, E, N), Int) -> Stream(A, E, N)
drop : <A, E, N>(Stream(A, E, N), Int) -> Stream(A, E, N)
take_while : <A, E, F, N>( Stream(A, E, N), (A) <- (Fail(F), ..N) -> Bool ) -> Stream(A, E | F, N)
fold : <A, Acc, E, N>( Stream(A, E, N), Acc, (Acc, A) <- N -> Acc ) <- (Fail(E), ..N) -> Acc
fold_until : <A, Acc, E, N>( Stream(A, E, N), Acc, (Acc, A) <- N -> list.ContinueOrStop(Acc) ) <- (Fail(E), ..N) -> Acc
each : <A, B, E, N>(Stream(A, E, N), (A) <- N -> B) <- (Fail(E), ..N) -> Nil
to_list : <A, E, N>(Stream(A, E, N)) <- (Fail(E), ..N) -> List(A)
from_results : <A, E, N>(List(Result(A, E))) -> Stream(A, E, N)
attempt : <A, E, N>(Stream(A, E, N)) -> Stream(Result(A, E), Never, N)
map_environment : <A, E, N, M>( Stream(A, E, N), (() <- (Fail(E), ..N) -> Step(A, E, N)) <- (Fail(E), ..M) -> Step( A, E, N, ) ) -> Stream(A, E, M)
```

**Missing attached type/constructor docs.** `Done`, `Next`, `Step`, `Stream`. Module prose may still describe them.

**Language use.** Uses hidden Stream, Done/Next union, ordinary deferred functions with Fail/open needs and Result only at attempt/from_results boundaries. A stream is repeated production, not a second representation of a single effect. Add resource-safe IO sources, bounded concurrency and timing through existing task/Scope services. Whole-need-row simplification needs care rather than a cosmetic copy of Effect types.

**Old style, overlap and thin surface.** The phantom (N)->Nil marker forces invariant N. map_environment is a stream-specific callback runner; ordinary <- already provides services. There is no public file/socket source, buffer, merge or timeout. Core sequential transforms are substantial and demand-aware.

**Tests.** 13 named tests: `std/test/stream_test.i` (13). 13 named tests reference all exports and cover algebraic laws, demand, replay, call-time services, error positions, callback order, resource scope and factories. Buffered, concurrent and scheduled sources have no public tests because the APIs do not exist.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### std/string

Source: `std/src/string.i`. Provides Unicode-scalar text operations and conversions.

Exported types: none.

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
utf_codepoint : (Int) -> Result(UtfCodepoint, Nil)
utf_codepoint_to_int : (UtfCodepoint) -> Int
compare : (String, String) -> Order
inspect : <A>(A) -> String
to_option : (String) -> Option(String)
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

**Language use.** Uses core UTF-8 primitives, list matching, Option absence and need-polymorphic callbacks; split_when explicitly propagates Fail. E4 should use Fail for invalid scalar construction. Services are otherwise needed only by callbacks. Shape groups could reduce default/search and replacement variants.

**Old style, overlap and thin surface.** byte_size, concat, to_upper/to_lower, inspect and codepoint conversion wrap core. inspect is polymorphic over every value despite living in string. Core index_of uses byte offsets; std/string.index_of uses scalar offsets. These are overlapping names with different units.

**Tests.** 333 named tests: `std/test/string_additions_test.i` (5), `std/test/string_test.i` (328). 328 named tests plus 5 additions tests. is_empty has no direct reference in those files. Many Unicode and long-input cases exist. This is scalar text, not a promise of grapheme segmentation or normalization; those APIs and their tests are absent.

**Documentation.** All explicit public lowercase values have attached item docs. Constructor and module-level contract gaps are described above.

### std/time

Source: `std/src/time.i`. Represents exact durations and Unix instants and supplies clock adapters.

Export visibility: `Duration`, `Instant`, `TimeError(..)`. Hidden representations below are descriptive only.

```text
Duration = (Int)
Instant = (Int)
TimeError = DivisionByZero
DivisionByZero = ()
```

```text
nanoseconds : (Int) -> Duration
milliseconds : (Int) -> Duration
seconds : (Int) -> Duration
minutes : (Int) -> Duration
hours : (Int) -> Duration
to_nanoseconds : (Duration) -> Int
add : (Duration, Duration) -> Duration
subtract : (Duration, Duration) -> Duration
scale : (Duration, Int) -> Duration
divide : (Duration, Int) -> Result(Duration, TimeError)
compare : (Duration, Duration) -> Order
from_unix_nanoseconds : (Int) -> Instant
unix_nanoseconds : (Instant) -> Int
shift : (Instant, Duration) -> Instant
between : (Instant, Instant) -> Duration
compare_instants : (Instant, Instant) -> Order
now : () <- (clock.Clock, Fail(clock.ClockError), request.Await, request.Owner) -> Instant
sleep : (Duration) <- (clock.Clock, Fail(clock.ClockError), request.Await, request.Owner) -> Nil
measure : <A, E, N>(() <- (Fail(E), ..N) -> A) <- (clock.Clock, Fail(E | clock.ClockError), request.Await, request.Owner, ..N) -> (A, Duration)
```

**Missing attached type/constructor docs.** `DivisionByZero`, `Duration`, `Instant`, `TimeError`. Module prose may still describe them.

**Language use.** Uses distinct hidden Duration/Instant constructors, TimeError, pure exact arithmetic and inherited Clock/Fail/request needs for observation. Use Fail for zero division in E4. General timeout/retry policy should build on Clock and task.race once, instead of living only in HTTP.

**Old style, overlap and thin surface.** The nominal distinction prevents adding two instants. Unit constructors are useful, not intrinsic renames. Pure APIs have uneven item docs. measure uses monotonic time; now uses wall time.

**Tests.** 4 named tests: `std/test/time_test.i` (4). Four named tests cover units, arithmetic, instants and huge counts. clock_test.i adds observation/sleep/measure. No reusable controllable clock or schedule policy exists; multi-step virtual-time ordering and timeout policies therefore lack library-level tests.

**Missing item documentation.** `add`, `subtract`, `scale`, `compare`, `unix_nanoseconds`, `shift`, `compare_instants`.

### std/uri

Source: `std/src/uri.i`. Parses, validates, resolves and encodes URI references and query forms.

Export visibility: `Uri`, `Parts(..)`, `Authority(..)`, `Host(..)`, `UriPart(..)`, `UriProblem(..)`, `UriError(..)`, `CodecError(..)`. Hidden representations below are descriptive only.

```text
Uri = (Parts)
Parts = ( scheme = Option(String), authority = Option(Authority), path = String, query = Option(String), fragment = Option(String), )
Authority = (userinfo = Option(String), host = Host, port = Option(String))
Host = RegName | IpLiteral
RegName = (String)
IpLiteral = (String)
UriPart = WholePart | SchemePart | AuthorityPart | UserInfoPart | HostPart | PortPart | PathPart | QueryPart | FragmentPart
WholePart = ()
SchemePart = ()
AuthorityPart = ()
UserInfoPart = ()
HostPart = ()
PortPart = ()
PathPart = ()
QueryPart = ()
FragmentPart = ()
UriProblem = BadCharacter | BadPercent | BadScheme | BadAuthority | BadHost | BadPort | BadPath | RelativeBase
BadCharacter = ()
BadPercent = ()
BadScheme = ()
BadAuthority = ()
BadHost = ()
BadPort = ()
BadPath = ()
RelativeBase = ()
UriError = (part = UriPart, offset = Int, problem = UriProblem)
CodecError = BadEscape | NonAscii | InvalidUtf8 | UnalignedBits
BadEscape = (offset = Int)
NonAscii = (offset = Int)
InvalidUtf8 = ()
UnalignedBits = ()
```

```text
parse : (String) -> Result(Uri, UriError)
build : (Parts) -> Result(Uri, UriError)
parts : (Uri) -> Parts
to_string : (Uri) -> String
has_scheme : (Uri) -> Bool
resolve : (Uri, Uri) -> Result(Uri, UriError)
encode_component : (String) -> String
encode_bytes : (BitArray) -> Result(String, CodecError)
decode_component : (String) -> Result(String, CodecError)
decode_bytes : (String) -> Result(BitArray, CodecError)
parse_query : (String) -> Result(List((String, String)), CodecError)
build_query : (List((String, String))) -> String
```

**Missing attached type/constructor docs.** `Authority`, `AuthorityPart`, `BadAuthority`, `BadCharacter`, `BadEscape`, `BadHost`, `BadPath`, `BadPercent`, `BadPort`, `BadScheme`, `CodecError`, `FragmentPart`, `Host`, `HostPart`, `InvalidUtf8`, `IpLiteral`, `NonAscii`, `Parts`, `PathPart`, `PortPart`, `QueryPart`, `RegName`, `RelativeBase`, `SchemePart`, `UnalignedBits`, `Uri`, `UriError`, `UriPart`, `UriProblem`, `UserInfoPart`, `WholePart`. Module prose may still describe them.

**Language use.** Uses hidden validated Uri, labelled Parts/Authority/errors, unions, Option components, internal Fail and public Result. E4 should expose parse/build/resolve/codec failures through Fail. Keep absent URI parts as Option. Query construction should have one public home.

**Old style, overlap and thin surface.** HTTP republishes parse_query/build_query. Internal parsers catch Fail back into Result. Offsets are documented as bytes; regex/string use scalars. has_scheme and encode_bytes lack item docs. The implementation is substantially stricter than a string-splitting facade.

**Tests.** 10 named tests: `std/test/uri_test.i` (10). Ten named tests reference all exports and include errors, resolution and long inputs. They test the Result surface. Browser URL policy, DNS validity, arbitrary schemes and port-range policy are explicitly outside URI syntax validation.

**Missing item documentation.** `has_scheme`, `encode_bytes`.

### std/test (Lisp surface)

Source: `std/src/test.is`. It registers tests by expanding syntax into items.
This module is included even though the requested glob is `.i`.

Exported types: none. `tests : Nil` is the item-group anchor.
The six exported macros have syntax-transformer types, not runtime callback
execution types:

```text
test  : (Syntax, Syntax) <- (syntax.Items, Fail(syntax.SyntaxError)) -> Syntax
skip  : (Syntax, Syntax) <- (syntax.Items, Fail(syntax.SyntaxError)) -> Syntax
only  : (Syntax, Syntax) <- (syntax.Items, Fail(syntax.SyntaxError)) -> Syntax
fails : (Syntax, Syntax) <- (syntax.Items, Fail(syntax.SyntaxError)) -> Syntax
suite : (Syntax, Syntax) -> Syntax
each  : (Syntax, Syntax, Syntax) <- (syntax.Items, Fail(syntax.SyntaxError)) -> Syntax
```

These are body-derived macro signatures. `syntax.Items.add` can raise
Fail(SyntaxError); test/skip/only/fails/each therefore also need
`Fail(syntax.SyntaxError)`. `suite` rewrites child calls for later expansion.
`Syntax` arguments preserve user test code; the macros do not execute it.

**Language use.** Uses macro syntax, quotation/splicing, Items, labelled syntax
nodes and ordinary functions. It does not need a special test keyword.
The generated test uses the services required by its body at runtime.

**Old style and gaps.** The suite expander still pattern-matches the quoted
`(! ~service ~value)` environment-statement form, which syntax-3.md removes.
Its error text still says “environment statements”. New `<-` provides should
be part of the macro contract. This file has no module or per-item docs.
The implementation also has private append and syntax-list walkers.

**Tests.** There is no `std/test/test_test.i`. The 31 standard test files use
test registration. `examples/macro-tests/test/examples.i` contains 3 named
tests and `examples/tour/test/sugar_test.i` exercises test sugar.
Macro and generated-test Rust suites add integration coverage.
This usage count does not prove every skip/only/fails/suite/each selection
rule, or the migration from quoted `!` to `<-`.

## Cross-module findings

### E3/E4 boundary and migration checklist

`docs/design/syntax-3.md` §2.10.6 says fallible operations use Fail and Result
is data after catch. §7.2 E4 estimates about 140 changed signatures; that is a
planning estimate across a different migration boundary, not a count proved
by this report. The inventories above preserve every current public type.
The following groups identify the actual migration decisions in this tree.

| Group | Current public surface | Audit judgement |
|---|---|---|
| Numeric failures | int parse/parse_base/to_base/power/square_root/digits/undigits/divide/remainder/modulo/floor_divide; float parse/power/real_power/exponential/logarithm/square_root/divide/modulo/tan/asin/acos/hypot/degrees | E4 Fail candidates. Nil errors hide useful distinctions among bad text, invalid radix, domain and overflow. |
| Binary/text failures | bit_array to_string/slice/base16_decode/base64_decode/base64_url_decode; string.utf_codepoint | E4 Fail candidates. Keep is_utf8 and other predicates total. Decide whether raw core Result APIs remain private ABI helpers. |
| Parsing and validation | calendar date/time_of_day/offset/days_in_month/parse; cli parse/help/text/integer; csv options/parse/parse_with/parse_header/decode; json.parse; path.relative; property int/element/list/string; regex.compile; schema decode/parse; uri parse/build/resolve/encode_bytes/decode_component/decode_bytes/parse_query | These are computations that fail. Use Fail. Preserve validation reports and ordinary absence as data where that is the requested result. |
| Cryptographic preconditions | hmac_sha256/512, hkdf_sha256/512, hkdf_extract_sha256/512, hkdf_expand_sha256/512, pbkdf2_sha256, random_bytes | Use Fail for invalid length, alignment and iteration count. random_bytes already raises ProcessError separately; merge the error needs. |
| HTTP decoding | parse_query, body_text, body_json, body_decode | Share query codecs with uri. Body decoding failures should compose through Fail with transport failures. |
| Collection sequencing | list.try_map/try_fold/try_each and strict_zip; Result callbacks embedded in cli.Parser and schema.Schema | Ordinary callbacks can fail already. Consolidate operational sequencing through native needs. Keep intentional transformations of a list of Result values available as data operations. |
| Intentional data | fail.catch, assertion.catch, result operations, option/result conversion, stream.attempt/from_results, HTTP Progress and property Verdict/Report | Keep data semantics. A caught error, optional lookup, partial parse progress or property report is not automatically a raised error. |
| Raw host responses | Clock/Fs/Process/Net/Tls/Stdio/Fetch fields return Request(Result(...)); request and key have private raw status Result values. | The ABI response is data. Public wrappers already await and translate it into Fail. Do not mechanically replace host transport data with active handlers. |
| Deferred CSV errors | rows/records return Stream(Result(Row, CsvError), E, N). | Source E already goes through Fail while CsvError is an element. Make this an explicit capture policy, or put CsvError in the ordinary pull need and offer capture through attempt. |

E3 also removes separate error parameters from control handles. The target
control contracts from `docs/design/syntax-3.md` §2.10.7 are:

```text
boundary : <R, N>((Boundary(R, N)) <- N -> R) <- N -> R
suspend  : <A, R, N>(Boundary(R, N), (Continuation(A, R, N)) <- N -> R) -> A
resume   : <A, R, N>(Continuation(A, R, N), A) <- N -> R
cancel   : <A, R, N>(Continuation(A, R, N)) <- N -> Nil
escape   : <R, N>(Boundary(R, N), R) -> Never
```

The Fail service, including one merged Fail for all raised variants, belongs
inside N. The snapshot's older E parameters are not an alternative design.

### Source conflicts that must be resolved before stability

These are textual findings against the current spec, not compiler failures
observed in this audit. `docs/spec.md` §§2.2 and 4.3 require one definition per
name and one constructor per labelled type. `syntax-3.md` §2.8 makes the same
rule explicit.

| Source | Observed conflict or migration residue |
|---|---|
| `lib/i/task.i` | Runner and Scheduler are each first declared as labelled data and later redeclared as unions containing the same name. |
| `lib/i/process.i` | Directory is first `(String)` and later `InheritDirectory \| Directory`. |
| `std/src/http.i` | Headers is first `(List((String,String)))` and later `Headers \| RawHeaders`. Public client returns source alias ClientFunctions, absent from its export list. |
| `lib/i/syntax.i` | Int/Float wrapper constructors and the old Node/Position definitions remain, against syntax-3.md §§2.8 and 6.8. |
| `lib/i/fail.i`, `lib/i/failure.i` | Intrinsic error entry points and ordinary-service entry points coexist. E3 is the stated consolidation point. |
| `lib/i/assertion.i`, `std/src/list.i`, `std/src/log.i`, `std/src/test.is` | Old ^/!/?, old type declarations or quoted environment statements survive in comments/macros. Comments can mislead even where executable source has moved. |
| `std/test/property_failures.py` | The embedded probe uses positional `property.Config(42,1,0,0,0)` although Config has labelled fields. This probe needs the same syntax migration as ordinary .i tests. |
| `std/test/std3_interfaces.py` | This script compares 35 property/string type strings against `docs/design/std-3.md`. It is not an all-std public-interface snapshot, and its expected strings need review when E4 changes APIs. |
| `lib/i/big.i`, `lib/i/bits.i`, `lib/i/float_text.i` | Comments state runtime call hooks for library algorithms. Distinguish a necessary primitive ABI from compiler recognition of library names; the owner forbids library privilege. This audit does not change those hooks. |

Some hidden exported type representations also mention private implementation
types, such as Dict's Node, Sequence's Tree and Task's info types. Review this
against spec §2.4's public-type visibility rule as part of the migration.
The inventories intentionally show the representations rather than claiming
that a hidden constructor makes a private referenced type public.

### Module × language-feature use

This matrix covers implementation use, including private helpers.
`Y` means direct use; `I` means a need inherited from a called operation;
`-` means absent. `N` means open callback/function need rows, `S` means named
service reads or service definitions, and `S/N` means both.
`F` is Fail use, `V` is Result values, `P` is service provision with `<-`,
`G` is an exported multi-shape function group, `L` is labelled data/fields,
`T` is local type forms (`1` one-constructor, `U` union, `O` opaque),
`M` is macros (`API` means syntax support), `C` is tasks/scopes/events,
and `R` is resource ensure/acquire use. An I in R means indirect cleanup.
A type declaration alone is not evidence of runtime task creation.

All exported ordinary functions inspected here have one argument shape.
Same-arity constructor matching is common. It is not an optional-argument
shape group. The consistent absence in G identifies an opportunity, not a
requirement that every pure module use every feature.

| Module | Needs | F | V | P | G | L | T | M | C | R |
|---|---|---|---|---|---|---|---|---|---|---|
| `i/assertion` | N | I | Y | Y | - | Y | - | - | - | I |
| `i/big` | - | - | - | - | - | - | O | - | - | - |
| `i/bits` | - | - | - | - | - | - | - | - | - | - |
| `i/clock` | S | Y | Y | - | - | Y | 1 | - | - | I |
| `i/control` | N | Y | - | - | - | - | - | - | - | I |
| `i/core` | - | - | Y | - | - | - | - | - | - | - |
| `i/fail` | N | Y | Y | - | - | - | - | - | - | - |
| `i/failure` | S/N | Y | Y | Y | - | Y | - | - | - | I |
| `i/fetch` | S | Y | Y | - | - | Y | 1 | - | - | Y |
| `i/float_text` | - | - | - | - | - | - | - | - | - | - |
| `i/fs` | S | Y | Y | - | - | Y | 1/U | - | - | Y |
| `i/io` | S | - | - | - | - | Y | 1 | - | - | - |
| `i/key` | - | - | Y | - | - | - | 1/U | - | - | - |
| `i/net` | S | Y | Y | - | - | Y | 1 | - | - | Y |
| `i/process` | S | Y | Y | - | - | Y | 1/U | - | - | Y |
| `i/random` | - | Y | - | - | - | - | 1/U | - | - | - |
| `i/request` | S/N | I | Y | Y | - | Y | 1/U | - | - | Y |
| `i/resource` | N | Y | - | - | - | - | - | - | - | Y |
| `i/stdio` | S | Y | Y | - | - | Y | 1 | - | - | I |
| `i/syntax` | S | Y | - | - | - | Y | 1/U/O | API | - | - |
| `i/task` | S/N | Y | Y | Y | - | Y | 1/U | - | Y | Y |
| `i/tls` | S | Y | Y | - | - | Y | 1 | - | - | Y |
| `std/bit_array` | - | - | Y | - | - | - | - | - | - | - |
| `std/bool` | N | - | - | - | - | - | - | - | - | - |
| `std/calendar` | I | Y | Y | - | - | Y | 1/U | - | - | I |
| `std/cli` | N | Y | Y | - | - | Y | 1/U | - | - | I |
| `std/crypto` | I | I | Y | - | - | - | 1/U | - | - | I |
| `std/csv` | N | Y | Y | - | - | Y | 1 | - | - | - |
| `std/dict` | N | - | - | - | - | - | 1/U | - | - | - |
| `std/float` | - | - | Y | - | - | - | - | - | - | - |
| `std/http` | S/N | Y | Y | Y | - | Y | 1/U | - | Y | Y |
| `std/int` | N | - | Y | - | - | - | - | - | - | - |
| `std/json` | - | Y | Y | - | - | Y | 1/U | - | - | - |
| `std/list` | N | I | Y | - | - | - | 1/U | - | - | - |
| `std/log` | S/N | Y | - | - | - | Y | 1/U | - | - | Y |
| `std/option` | N | - | Y | - | - | - | 1/U | - | - | - |
| `std/order` | N | - | - | - | - | - | 1/U | - | - | - |
| `std/pair` | N | - | - | - | - | - | - | - | - | - |
| `std/path` | - | - | Y | - | - | - | 1/U | - | - | - |
| `std/property` | S | Y | Y | - | - | Y | 1/U | - | - | I |
| `std/regex` | - | Y | Y | - | - | Y | 1/U | - | - | - |
| `std/result` | N | Y | Y | - | - | - | - | - | - | - |
| `std/schema` | - | - | Y | - | - | Y | 1/U | - | - | - |
| `std/sequence` | N | I | - | - | - | - | 1/U | - | - | - |
| `std/set` | N | - | - | - | - | - | 1 | - | - | - |
| `std/stream` | N | Y | Y | - | - | - | 1/U | - | - | - |
| `std/string` | N | Y | Y | - | - | - | - | - | - | - |
| `std/time` | I | I | Y | - | - | - | 1 | - | - | I |
| `std/uri` | - | Y | Y | - | - | Y | 1/U | - | - | - |
| `std/test` | S | I | - | - | - | - | - | Y | - | - |

A Fail entry in a callback type is real use of the native need mechanism,
although the module may not itself raise. Likewise, deferred stream errors
need no service at construction time. Some `Y`/`I` distinctions describe the
public contract rather than an explicit service binder.

### Duplicated or overlapping functionality

| Modules / names | Nature of overlap | One-place direction |
|---|---|---|
| i/fail.raise/catch; i/failure.raise/catch | Two public implementations of failure entry points. | Consolidate at E3 into ordinary-service i/fail. |
| i/fail.catch; std/result.catch | Exact forwarding facade. | One canonical catch; document compatibility separately. |
| i/fail.from_result; std/result.to_error | Same active failure bridge under different names. | One bridge with an action-revealing name. |
| std/option.to_result; std/result.from_option | Same conversion implementation. | Choose one canonical conversion home. |
| std/option.from_result; std/result.to_option | Same conversion implementation. | Choose one canonical conversion home. |
| std/list.group; std/dict.group | list.group forwards to dict.group. | Keep the algorithm in one place and decide whether an alias earns its public cost. |
| i/core.not; std/bool.not | One-call intrinsic wrapper. | Separate primitive ABI from discoverable public facade explicitly. |
| i/core.to_float/bit_* /shift_*; std/int.to_float/bitwise_* | Same primitive operations, longer public names. | One documented public vocabulary; avoid independent contracts. |
| i/core.ceiling/floor/truncate; std/float.ceiling/floor/truncate | One-call primitive wrappers. | Same judgement as numeric facades. |
| i/core.concat/byte_size/to_upper/to_lower/from_codepoints/codepoint_to_int; std/string equivalents | Mostly direct forwarding. | Keep policy such as units in the public layer; avoid duplicate implementations. |
| i/core.inspect; std/string.inspect, std/bit_array.inspect, std/int.to_string, std/float.to_string | Several text-formatting doors; string.inspect accepts any type. | Put generic inspection in one clear home. Specialized formatting may remain discoverable aliases. |
| i/float_text.to_string; core.inspect(Float); std/float.to_string | One algorithm reached through several API layers. | Preserve the single formatter; document the layering and runtime coupling. |
| i/core.bit_size/bit_array_concat/bit_array_to_string; std/bit_array.bit_size/concat/to_string | Direct primitive facades. | Do not mistake codec algorithms for these wrappers. |
| std/int/float add/subtract/multiply; std/bool and/or; pair.new/first/second; list.new/wrap/prepend | Restate syntax, operators or field reads. | Keep only where higher-order use or discoverability justifies it. Boolean evaluation differs from short-circuit operators. |
| std/list map/fold/each versus try_map/try_fold/try_each | Two sequencing protocols, despite ordinary callback Fail support. | Prefer native needs; keep Result collection as explicit data handling. |
| std/schema.optional/nullable; transform/map | Literal aliases, with optional/nullable suggesting different meanings. | One name per operation; preserve missing-field semantics in optional_field. |
| std/http.parse_query/build_query; std/uri.parse_query/build_query | Exact public query-codec forwarding. | URI/query codec home, used by HTTP. |
| std/csv.decode; std/schema.decode plus row-to-JSON conversion | Overlapping codec dispatch with codec-first order. | Define the row representation adapter once. |
| i/net.byte_stream; i/tls.byte_stream; HTTP private transport adapters | Similar closed data for read/write/close, with differing error/lifetime contracts. | One ordinary immutable byte-operation contract; adapters translate errors once. |
| i/io.Io.print; i/stdio.print | Both print text with LF, but failure and host contracts differ. | Document or consolidate the compatibility boundary. |
| i/clock sleep_ns/unix_ns; std/time.sleep/now; std/calendar.now/format_now | Raw versus domain-typed clock operations. | Useful layering, not duplicate algorithms. Keep one clock service and explicit units. |
| std/list, std/sequence, std/stream | Eager linked lists, persistent indexed deques and lazy effectful pulls. | Keep distinct semantics and shared naming; these are not interchangeable duplicate containers. |
| std/dict, std/set | Set is Dict(A,Nil) with set semantics. | Useful abstraction; no independent hashing implementation. |
| Private list reversals in i/bits, i/float_text, i/task, i/syntax, std/option, std/result, std/dict | Repeated elementary traversal, often to avoid lib→std dependency or std import cycles. | Audit dependency direction before deduplicating. The foundation cannot import std/list. |
| std/path splitting; std/uri path normalization; std/http route splitting | Similar text operations with different escaping/validation semantics. | Share only primitives whose units and escaping rules match. |
| std/stream.map_environment; control.keep_env; task.keep | All adapt deferred environments but have different lifetime/return contracts. | Use ordinary provision as the base; preserve stream-tail rebuilding and scheduler ownership where required. |

### Public names and argument orders that diverge from common conventions

The common pattern in list/dict/string/sequence/stream is data first,
configuration next, callback last. `x |> f(a)` inserts x as position zero
(`docs/spec.md` §6.6; `syntax-3.md` §2.6.2). The entries below are exceptions
or ambiguities, not a claim that the language reserves any name.

| Public names | Divergence | Concrete consequence |
|---|---|---|
| schema.decode/encode/parse/print; csv.decode | Codec before the data. | `value \|> schema.decode(codec)` passes value as the codec. |
| regex.is_match/find/find_all/captures/replace/replace_all/split | Regex before input. | A pipeline of input text needs a lambda or a different chosen subject convention. |
| http.send_with/send_timeout | Send function before Request. | Request pipelines need a lambda. Timeout also takes raw ns while std/time has Duration. |
| http.route | Request is third, after method and pattern. | A pipeline of requests cannot feed it directly. |
| crypto.hmac_sha256/512; hkdf_extract_sha256/512; hkdf_expand_sha256/512 | HMAC puts key first, extract puts salt first, expand puts key first; combined hkdf puts input first. | The meaning of piped bytes changes across the same family. Decide key-as-subject versus input-as-subject once. |
| bool.xor versus exclusive_nor | Abbreviated and expanded names in one small family. | Users cannot predict the complement name. |
| dict.has; set.contains; list.contains; regex.is_match | Different membership verbs. | Some reflect domain semantics; dict.has versus contains deserves a deliberate rule. |
| list/sequence/stream.wrap; option.Some; pair.new | Single-element construction has several vocabularies. | Clarify constructor versus collection singleton naming. |
| int.to_base2/8/16/36; bit_array.base16_encode/decode | Base formatting and codecs use different naming directions. | Similar tasks are hard to discover by one pattern. |
| int.bitwise_* versus core.bit_*/shift_* | Prefix and word-order changes for the same primitives. | Public vocabulary is larger than the capability. |
| string.inspect | Generic `(A)->String` operation in a text module. | Its apparent subject is narrower than its type. |
| result.to_error | Returns a success value or raises. | The conversion name hides active failure; from_result in i/fail is clearer in context. |
| list.try_map/try_fold/try_each | Prefix marks Result sequencing, while ordinary map/fold/each can already fail. | Two apparent effect systems for callers. |
| schema.optional versus nullable | Same function, different implied semantics. | JSON null and missing fields are easy to confuse. |
| schema.transform versus map | Same function under two names. | No extra capability. |
| csv.parse_with/print_with; json.pretty; option/result.lazy_unwrap/lazy_or; order.lazy_break_tie; bool.lazy_guard | Separate variant names replace shape-selected calls. | Optional/configured/lazy behavior does not use the available shape-group feature. Same-shape type overloads need careful inference; do not merge blindly. |
| net.close_stream/close_listener/close_datagram versus fs/tls/process/fetch.close | Kind-specific suffixes in one module only. | Argument unions or type selection could provide one close, if the contracts agree. |
| std/string.index_of and i/core.index_of | Same base name, scalar versus byte index. | A result cannot safely be reused across the two APIs without conversion. |
| regex.Match offsets; csv.CsvError positions; uri/json parse offsets | Regex/scalar and CSV/scalar positions differ from URI/JSON byte offsets. | Position units belong in public docs and types where valuable. The inventory does not propose treating all Int positions as interchangeable. |
| std/log.Failure; property.Fail; i/failure; prelude Fail(E) | Nearby words mean a level, verdict, module and error service. | Use qualification and reduce migration-only aliases; no keyword restriction is needed. |
| task.Scope(E); planned resource.Scope | Same word for task-group ownership and resource lifetime. | Follow the documented E5 Group rename; do not conflate the services. |

### Size, public surface and dedicated test counts

The first two columns count the source, not generated artifacts.
“Expanded” includes union members implicitly exported with `(..)`.
A type with a hidden constructor still counts as one exported type name.
The test count column uses each module's designated std test files only;
clock/stdio use their standard fake test. Other i/ modules show zero here
and retain their spec/example/Rust evidence in the module sections.

| Module | Lines | Export entries | Expanded names | Public lowercase values | Named std tests |
|---|---:|---:|---:|---:|---:|
| `i/assertion` | 25 | 1 | 1 | 1 | 0 |
| `i/big` | 893 | 7 | 7 | 7 | 0 |
| `i/bits` | 128 | 2 | 2 | 2 | 0 |
| `i/clock` | 46 | 5 | 5 | 3 | 1 |
| `i/control` | 44 | 6 | 6 | 6 | 0 |
| `i/core` | 153 | 30 | 30 | 30 | 0 |
| `i/fail` | 28 | 4 | 4 | 4 | 0 |
| `i/failure` | 28 | 2 | 2 | 2 | 0 |
| `i/fetch` | 67 | 7 | 7 | 5 | 0 |
| `i/float_text` | 347 | 1 | 1 | 1 | 0 |
| `i/fs` | 235 | 19 | 23 | 14 | 0 |
| `i/io` | 11 | 1 | 1 | 0 | 0 |
| `i/key` | 27 | 4 | 6 | 3 | 0 |
| `i/net` | 321 | 22 | 22 | 17 | 0 |
| `i/process` | 327 | 24 | 30 | 14 | 0 |
| `i/random` | 201 | 8 | 10 | 6 | 0 |
| `i/request` | 197 | 27 | 31 | 23 | 0 |
| `i/resource` | 23 | 2 | 2 | 2 | 0 |
| `i/stdio` | 80 | 9 | 9 | 7 | 1 |
| `i/syntax` | 639 | 20 | 55 | 10 | 0 |
| `i/task` | 4735 | 41 | 42 | 30 | 0 |
| `i/tls` | 182 | 14 | 14 | 11 | 0 |
| `std/bit_array` | 332 | 18 | 18 | 18 | 147 |
| `std/bool` | 78 | 12 | 12 | 12 | 34 |
| `std/calendar` | 472 | 26 | 41 | 20 | 5 |
| `std/cli` | 836 | 19 | 30 | 15 | 10 |
| `std/crypto` | 678 | 14 | 17 | 13 | 88 |
| `std/csv` | 517 | 12 | 12 | 10 | 107 |
| `std/dict` | 531 | 23 | 23 | 22 | 97 |
| `std/float` | 1560 | 44 | 44 | 44 | 337 |
| `std/http` | 2012 | 46 | 61 | 35 | 66 |
| `std/int` | 391 | 37 | 37 | 37 | 180 |
| `std/json` | 555 | 13 | 20 | 11 | 78 |
| `std/list` | 1147 | 68 | 70 | 67 | 269 |
| `std/log` | 223 | 15 | 19 | 12 | 11 |
| `std/option` | 148 | 16 | 18 | 15 | 40 |
| `std/order` | 92 | 12 | 15 | 11 | 30 |
| `std/pair` | 40 | 6 | 6 | 6 | 14 |
| `std/path` | 187 | 9 | 11 | 8 | 5 |
| `std/property` | 760 | 29 | 39 | 22 | 14 |
| `std/regex` | 1077 | 14 | 14 | 10 | 130 |
| `std/result` | 202 | 22 | 22 | 22 | 55 |
| `std/schema` | 438 | 29 | 33 | 25 | 52 |
| `std/sequence` | 652 | 24 | 24 | 23 | 8 |
| `std/set` | 161 | 21 | 21 | 20 | 31 |
| `std/stream` | 302 | 25 | 27 | 23 | 13 |
| `std/string` | 763 | 53 | 53 | 53 | 333 |
| `std/time` | 148 | 22 | 23 | 19 | 4 |
| `std/uri` | 829 | 20 | 43 | 12 | 10 |
| `std/test` (.is) | 111 | 7 | 7 | 7 | 0 |
| **Total** | 23979 | 912 | 1070 | 790 | 2170 |

The standard test total above counts each of the 31 files once. It is not
inflated by assigning clock_test's calls to time/calendar/log as extra tests.
Types and value names in this table follow explicit exports, not every line
in library.out. Repeated source type definitions count once as a public name.

The phase-2 order suggested by this inventory is: settle the E3 source
boundary, implement E4 Fail and resource Scope coherently, consolidate public
homes and argument conventions, then add capability breadth on that base.
No code change, test execution or performance claim is part of this phase.
