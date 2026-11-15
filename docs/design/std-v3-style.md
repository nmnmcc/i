# Standard library v3: style and stability

Part C. Read-only audit and proposal. 14 November 2026.

The i checkout reviewed here is `68b720e9ebc0d643a3a7831a32a1b97c7ab3c915`.
No implementation, build, or test run is part of this phase. Statements about
coverage mean source-level API coverage. They do not certify correctness or
target support.

Reference paths below are relative to `/home/agent/Programming/i/.rd/ref/`.
i paths are relative to this repository. Each comparison names its source
files. The Effect checkout identifies itself as 4.1.3 in
`effect/packages/effect/package.json`.

## 1. Decisions that should guide the rewrite

1. Keep Gleam's small modules, plain names, subject-first calls, and examples.
   Keep i's own absence convention. Do not import Gleam's Result-only control
   flow. See `gleam-stdlib/src/gleam/option.gleam`,
   `gleam-stdlib/src/gleam/list.gleam`, and `std/src/list.i`.
2. Use ordinary functions and services for workflows. A computation is
   `() <- (Fail(E), ..N) -> A`. Provision is `body() <- (service)`.
   There must be no parallel Effect, Layer, Context, or error-channel system.
   This is required by `docs/spec.md` §§9.1–9.3 and
   `docs/design/syntax-3.md` §2.10.
3. Take Go's common I/O contracts seriously. i has many host operations, but
   lacks a shared reader/writer vocabulary and buffering layer. See
   `go/src/io/io.go`, `go/src/bufio/bufio.go`, `lib/i/fs.i`,
   `lib/i/stdio.i`, `lib/i/net.i`, and `lib/i/tls.i`.
4. Complete useful paths through the library before adding more synonyms.
   In particular: bounded I/O, incremental codecs, timeout/retry policies,
   Unicode operations, typed diagnostics, and deterministic test providers.
   The evidence and limits appear in §3.
5. Stabilize behavior as well as types. Reuse the interface comparison in
   `crates/ilang-cli/src/commands/package.rs`, but add a documented behavioral
   review. The current comparison cannot detect a changed parser policy or
   cleanup guarantee.

The spec is authoritative when this checkout differs from it. For example,
`std/src/result.i` and `std/src/list.i` still describe `^ E` in comments.
`lib/i/control.i` still carries an explicit error parameter on boundaries.
`lib/i/resource.i` has callback-style `acquire`, while `docs/spec.md` §9.5
defines scope-based acquisition too. These are migration facts. All proposals
below use `Fail(E)` and the specified service model.

## 2. What Gleam teaches

### 2.1 Module inventory

This is the module inventory of the supplied sources, including nested modules.
It is not a claim that every package in the Gleam ecosystem is listed.

| Source module | What it holds |
|---|---|
| `gleam-stdlib/src/gleam/bit_array.gleam` | Bits, byte sizes, slicing, UTF-8 conversion, hex and base64 codecs. |
| `gleam-stdlib/src/gleam/bool.gleam` | Boolean operations, conversion, eager and lazy guards. |
| `gleam-stdlib/src/gleam/bytes_tree.gleam` | Persistent byte assembly, cheap tree concatenation, final flattening. Byte alignment is explicit. |
| `gleam-stdlib/src/gleam/dict.gleam` | Dictionaries, lookup, updates, folds, filters, merging and grouping. |
| `gleam-stdlib/src/gleam/dynamic.gleam` | Opaque foreign data, classification, construction from typed values. |
| `gleam-stdlib/src/gleam/dynamic/decode.gleam` | Typed decoders, fields, paths, optional fields, lists, maps, composition and decode errors. |
| `gleam-stdlib/src/gleam/float.gleam` | Parsing, printing, arithmetic, rounding, comparisons, mathematical operations and random draws. Documents target differences. |
| `gleam-stdlib/src/gleam/function.gleam` | Only `identity` in the supplied source. |
| `gleam-stdlib/src/gleam/int.gleam` | Parsing, radix conversion, arithmetic, comparisons, bit operations, random draws and range folding. |
| `gleam-stdlib/src/gleam/io.gleam` | Text output to stdout and stderr, with and without a newline. |
| `gleam-stdlib/src/gleam/list.gleam` | Collection traversal, mapping, folding, search, sorting, grouping and combinatorics. |
| `gleam-stdlib/src/gleam/option.gleam` | `Some`/`None`, defaults, conversion, mapping and composition of optional data. |
| `gleam-stdlib/src/gleam/order.gleam` | `Lt`/`Eq`/`Gt`, comparator reversal and tie breaking. |
| `gleam-stdlib/src/gleam/pair.gleam` | Pair construction, projections, swapping and mapping either part. |
| `gleam-stdlib/src/gleam/result.gleam` | `Ok`/`Error` inspection, mapping, short circuit composition, recovery and collection. |
| `gleam-stdlib/src/gleam/set.gleam` | Persistent sets, membership, set algebra, filtering and folds. |
| `gleam-stdlib/src/gleam/string.gleam` | Text search, slicing, trimming, padding, graphemes, codepoints and casing. |
| `gleam-stdlib/src/gleam/string_tree.gleam` | Efficient text assembly and transformations on text trees. |
| `gleam-stdlib/src/gleam/uri.gleam` | URI data, parsing, rendering, query forms, percent encoding, origins and resolution. |

The additional packages make their boundaries visible:

| Source module | What it holds |
|---|---|
| `gleam-http/src/gleam/http.gleam` | Methods, schemes, multipart parsing, content disposition and Link headers. |
| `gleam-http/src/gleam/http/request.gleam` | Generic request bodies, URI conversion, headers, query, method, address and cookies. |
| `gleam-http/src/gleam/http/response.gleam` | Generic response bodies, headers, body mapping, redirects and cookies. |
| `gleam-http/src/gleam/http/cookie.gleam` | Cookie parsing and rendering, attributes and SameSite policy. |
| `gleam-http/src/gleam/http/service.gleam` | Deprecated service and middleware aliases and helpers. Its annotations direct users to middleware packages. |
| `gleam-otp/src/gleam/otp/actor.gleam` | Typed actors, immutable next-state descriptions, initialization, start errors, send and call. |
| `gleam-otp/src/gleam/otp/static_supervisor.gleam` | Fixed child specifications, restart strategies and shutdown policies. |
| `gleam-otp/src/gleam/otp/factory_supervisor.gleam` | Dynamic children from one template, child counts and restart settings. |
| `gleam-otp/src/gleam/otp/supervision.gleam` | Child specifications, restart policy, child kind, timeout and returned-data mapping. |
| `gleam-otp/src/gleam/otp/system.gleam` | OTP system messages, debugging, state inspection, suspend and resume. |
| `gleam-otp/src/gleam/otp/port.gleam` | A deprecated Port type; the replacement is in gleam-erlang. |
| `gleam-otp/src/gleam/otp/internal/result2.gleam` | Internal result with two success payloads. Do not treat it as public library design guidance. |
| `gleam-erlang/src/gleam/erlang/application.gleam` | Application start kinds and private resource directories. |
| `gleam-erlang/src/gleam/erlang/atom.gleam` | Atom lookup, creation, strings and dynamic decoding. |
| `gleam-erlang/src/gleam/erlang/charlist.gleam` | Conversion between Erlang charlists and strings. |
| `gleam-erlang/src/gleam/erlang/node.gleam` | Distributed Erlang node identity, visibility and connection. |
| `gleam-erlang/src/gleam/erlang/port.gleam` | Erlang Port type. |
| `gleam-erlang/src/gleam/erlang/process.gleam` | Pids, typed subjects, selectors, sends, receives, calls, links, monitors, timers and exits. |
| `gleam-erlang/src/gleam/erlang/reference.gleam` | Fresh Erlang reference identity. |

### 2.2 Naming and call style

These are observed conventions, with exceptions. They are not universal rules
proved by a separate Gleam style specification.

| Convention | Source evidence | Consequence for i |
|---|---|---|
| Lowercase snake_case; types in UpperCamelCase. Operations use short verbs such as `insert`, `delete`, `filter` and `sort`. Queries also use nouns such as `length`. | `gleam-stdlib/src/gleam/list.gleam`, `gleam-stdlib/src/gleam/dict.gleam`. | Keep this style. Module qualification supplies the domain. |
| The transformed subject normally comes first. Configuration follows; callbacks are commonly last. | `list.map(list, with fun)`, `list.fold(list, from initial, with fun)` in `gleam-stdlib/src/gleam/list.gleam`; setters in `gleam-http/src/gleam/http/request.gleam`. | This agrees with i's pipe inserting position zero, defined in `docs/spec.md` §6.9. |
| `try_` signals traversal or recovery through Result. | `try_map`, `try_fold`, `try_each` in `gleam-stdlib/src/gleam/list.gleam`; `try_recover` in `gleam-stdlib/src/gleam/result.gleam`; `try_map` in `gleam-http/src/gleam/http/response.gleam`. | Do not make every i effectful callback need a second traversal API. |
| `_map` describes a specific transformation. It does not have one universal failure meaning. | `filter_map`, `flat_map`, `index_map`, `find_map` in `gleam-stdlib/src/gleam/list.gleam`. `filter_map` discards Error results; `flat_map` concatenates lists. | Define the container and discarded cases precisely. i already uses Option for `filter_map`. |
| `from_` names an input representation; `to_` names an output representation. | `from_list`/`to_list` in `gleam-stdlib/src/gleam/dict.gleam`; `from_string`/`to_string` in `gleam-stdlib/src/gleam/bit_array.gleam`. | Keep directional names and document validation. A `to_` name does not imply totality. |
| Labels make calls read naturally. | `keeping`, `with`, `from`, `up_to` in `gleam-stdlib/src/gleam/list.gleam`. | Translate meaning, not syntax. In i, `with : T` binds a positional parameter; `with = value` supplies a data label. See `docs/spec.md` §§6.4, 6.11. |

There are exceptions to subject-first order. `dict.group(key, list)` in
`gleam-stdlib/src/gleam/dict.gleam` puts the classifier first. The corresponding
`list.group(list, by key)` in `gleam-stdlib/src/gleam/list.gleam` puts the subject
first. Do not copy an exception as the general convention.

`gleam-stdlib/src/gleam/option.gleam` explicitly reserves Option for optional
arguments and stored data. It says fallible functions return Result, with Nil
when no diagnostic exists. Thus `list.first`, `dict.get`, and string lookups
return `Result(_, Nil)` in `gleam-stdlib/src/gleam/list.gleam`,
`gleam-stdlib/src/gleam/dict.gleam`, and
`gleam-stdlib/src/gleam/string.gleam`. This differs from i's existing Option
lookups. `result.try` and `result.try_recover` compose Result values in
`gleam-stdlib/src/gleam/result.gleam`.

### 2.3 What is left outside the small stdlib

The supplied stdlib has no filesystem, sockets, HTTP transport, OTP actors,
calendar, or JSON module. This is an inventory observation, not evidence of a
promise that Gleam will never add them. Its README promises Erlang and
JavaScript support; several functions use target-specific implementations.
Sources: `gleam-stdlib/README.md`,
`gleam-stdlib/src/gleam/string.gleam`, and the inventory above.

HTTP data is in another package. Actual clients and servers are adapters in
other packages, listed in `gleam-http/README.md`. OTP and Erlang runtime
operations are also separate, as shown by
`gleam-otp/src/gleam/otp/actor.gleam` and
`gleam-erlang/src/gleam/erlang/process.gleam`. This is a useful boundary lesson.
It is too small a coverage target for the owner's standalone i distribution.

### 2.4 What the i port gets right, and what should change

| Current i evidence | Assessment and proposed direction |
|---|---|
| `std/src/list.i`, `std/src/dict.i`, `std/src/set.i`, `std/src/pair.i`, `std/src/order.i` | The small type-focused modules, short verbs, subject-first order and comparator callbacks fit i. Keep them. |
| `std/src/list.i`, `std/src/option.i`, `std/src/result.i` | Callbacks already have an open need `N`. Ordinary map and fold can propagate `Fail(E)` through that row. Make this the main composition path. |
| `std/src/list.i`: `try_map`, `try_fold`, `try_each` | These traverse explicit Result data and stop at the first Error. They are not necessary to call functions that raise through Fail. Keep only where Result data traversal is useful; document that distinction and implement any retained helper through the canonical traversal. |
| `std/src/list.i`, `std/src/dict.i`, `std/src/string.i`, `std/src/option.i` | Option-returning lookups and Option-based filter/find mapping are good adaptations. Do not restore Gleam's Result-with-Nil lookup convention. |
| `std/src/int.i`, `std/src/float.i`, `std/src/bit_array.i` | Parsing and decoding often still return `Result(_, Nil)`. Diagnostics are lost. Introduce domain errors before stability and use Fail for the ordinary validating operation (§4.3). |
| `std/src/crypto.i`: `random_bytes` | Invalid length is Result data, while `process.entropy` can raise a ProcessError. A Result return alone does not mean the call has no Fail need. Normalize the public boundary and document the complete inferred contract. |
| `std/src/result.i`, `lib/i/fail.i`, `lib/i/failure.i` | `to_error`/`catch` wrappers and two failure modules create competing entry points. E3 should leave `i/fail` as the canonical error bridge; keep Result operations about stored data. |
| `std/src/list.i`, `std/src/option.i`, `std/src/result.i` | Private reversal loops recur. Gleam's `gleam-stdlib/src/gleam/option.gleam` explains its own copy by import cycles. i should use an acyclic internal collection foundation, without new compiler privilege. |
| `std/src/json.i`, `std/src/schema.i` | Labelled sums and explicit codecs fit i better than copying foreign Dynamic reflection. The current schema builder needs empty records, getters and setters; `std/test/schema_test.i` shows the repeated reconstruction. Explore ordinary constructor callbacks with decoded fields and named data. Do not invent reflection or a `use` form. |
| `std/src/string.i` versus `gleam-stdlib/src/gleam/string.gleam` | i correctly documents scalar indexes rather than pretending to preserve Gleam's grapheme semantics. Add separately named grapheme operations if provided. |
| `std/src/csv.i`: `parse`/`parse_with`; `std/src/http.i`: `send_with` and callback-first `send_timeout`; `std/src/schema.i`: schema-first decode | Review same-operation variants for shape groups and subject-first order. Shape groups are appropriate for differing argument shapes with one result family. They are not an excuse for hidden changes of meaning. |

The Gleam decoder examples use `use name <- decode.field(...)` in
`gleam-stdlib/src/gleam/dynamic/decode.gleam`. i has no such form.
Its `<-` supplies services, and must not be explained as callback sugar.
Likewise, Erlang process APIs include `spawn_unlinked` in
`gleam-erlang/src/gleam/erlang/process.gleam`. Do not port that lifecycle.
i tasks belong to scopes under `docs/spec.md` §9.9.

## 3. What Go teaches, and the coverage gap

### 3.1 Small contracts, separate responsibilities

`go/src/io/io.go` defines Reader and Writer with one method each. It separately
defines Closer and Seeker, then composes interfaces. Its docs specify short
reads, partial data before an error, EOF, zero-size reads, short writes, and
buffer retention. Small means a small *contract*, not merely a short type.

`go/src/bufio/bufio.go` wraps the same reader/writer contracts. File and network
code can therefore use the same algorithms. `go/src/net/net.go` describes Conn
and Listener, while `go/src/net/http/client.go` separates Client policy from
RoundTripper transport. `go/src/net/http/server.go` defines Handler. These cuts
let a caller replace one part without replacing the whole stack.

For i, use ordinary labelled values containing functions. Pass a reader value
explicitly when two inputs are being handled. Use `<-` for ambient capabilities
such as a filesystem or clock. Adapters should translate each host's operation
once. Pure codecs and buffering algorithms should consume the common values.
No method dispatch, interface keyword, mutable caller buffer, or compiler-known
standard package is required.

The immutable translation must settle three things before code is written:

| Contract | Proposed i rule |
|---|---|
| Input | Take a positive byte bound and return immutable bytes plus explicit end/progress data. Represent pending partial failure so bytes are not lost by raising too early. Zero-byte requests must have a stated rule. |
| Output | State whether `write` completes all bytes or returns progress. Never imply atomic output after a failure. A `write_all` helper must handle partial progress and reject endless zero progress. |
| Ownership | A reader value does not own a resource lifetime by itself. Consumption remains inside a resource scope; early return, error and task cancellation run cleanup. Pure buffered cursors return the next cursor. A host handle's external progress must be documented. |

These are proposals, motivated by `go/src/io/io.go`. Existing i wrappers often
return Nil for writes and use empty bytes for EOF (`lib/i/fs.i`,
`lib/i/net.i`, `lib/i/stdio.i`). They do not expose Go's partial-count contract.
Do not promise an adapter can preserve information that its host API omits.

### 3.2 Package-by-package comparison

“Gap” means absent or incomplete in the inspected public i APIs. It does not
mean that i must copy every Go function. Priority A closes a common application
path. Priority B extends reach after those paths work. “Adapt” means that the Go
mechanism conflicts with i, but its user need still matters.

| Go package and files read | What it supplies | i today: files and scope | Gap and direction |
|---|---|---|---|
| io: `go/src/io/io.go` | Shared Reader/Writer/Closer/Seeker contracts and I/O composition. | `lib/i/fs.i`, `lib/i/net.i`, `lib/i/tls.i`, `lib/i/stdio.i` have separate operations. TCP and TLS provide `byte_stream` data. | A: common byte readers/writers; copy, bounded read, exact read, limit and tee behavior. Define EOF, progress, errors and scope ownership once. |
| bufio: `go/src/bufio/bufio.go`, `go/src/bufio/scan.go` | Buffering, lookahead, delimiter reads, configurable token scanning. | `lib/i/fs.i` reads whole files for `read_lines`; `std/src/csv.i` has its own incremental cursor. | A: reusable immutable cursors, bounded tokens, incremental UTF-8 and line scanning. Reuse the reader contract. |
| bytes: `go/src/bytes/bytes.go`, `go/src/bytes/buffer.go`, `go/src/bytes/reader.go` | Byte search and manipulation; buffers and readers. | `std/src/bit_array.i` has concatenation, slicing, comparison and codecs; `std/src/sequence.i` is a persistent indexed collection. | A: byte search/split/replace and cheap chunk assembly. Distinguish bytes from arbitrary bits. Adapt buffers as immutable builders/cursors. |
| strings: `go/src/strings/strings.go`, `go/src/strings/builder.go`, `go/src/strings/reader.go` | Text search, splitting, transformations, assembly and reading. | `std/src/string.i` already has a broad scalar-based API, including bounded replacement, words, map/filter/fold. | A: chunk assembly and reader adapters. Document scalar versus byte indexes at each boundary. Avoid needless copies. |
| strconv: `go/src/strconv/doc.go` | Numeric and Boolean parsing/formatting, bases, precision, quoting. | `std/src/int.i`, `std/src/float.i`, `std/src/bool.i`, `lib/i/float_text.i` cover much of conversion; int and float parse errors are Nil. | A: diagnostic parse errors and explicit formatting options. B: quoting/unquoting APIs. Keep conversions with their type rather than add a second central conversion family. |
| unicode: `go/src/unicode/letter.go`, `go/src/unicode/utf8/utf8.go`, `go/src/unicode/utf16/utf16.go` | Codepoint properties, casing and UTF-8/UTF-16 codecs. | `lib/i/core.i` supplies scalar conversion and simple casing. `std/src/string.i` has an internal whitespace test. `std/src/regex.i` has private Unicode 16 property and fold tables. | A: expose a shared Unicode foundation and table version; reuse existing data instead of adding a second set. Add incremental UTF-8. B: UTF-16 and separately specified grapheme support. Preserve scalar indexes. |
| sort: `go/src/sort/sort.go` | Collection sorting through Len/Less/Swap, stable and unstable contracts. | `std/src/list.i` provides stable comparator merge sort; `std/src/order.i` supplies comparator tools. | Mostly covered. State comparator laws. Adapt to immutable outputs; no public Swap protocol. |
| slices: `go/src/slices/slices.go`, `go/src/slices/sort.go` | Generic collection edits, equality, sort checks and binary search. | `std/src/list.i` is rich; `std/src/sequence.i` has persistent indexing, splitting and end edits. | B: sortedness and binary search for indexed sequences, and comparison helpers where useful. No need for mutable slices or clones of immutable values. |
| maps: `go/src/maps/maps.go`, `go/src/maps/iter.go` | Equality, copying, filtered mutation, iterator collection. | `std/src/dict.i`, `std/src/set.i`, `std/src/stream.i` cover persistent updates and traversals. | B: explicit semantic equality/comparator helpers and stream adapters. Resolve the documented dict collision-order equality exception before promising map value equality. No mutable Copy requirement. |
| time: `go/src/time/time.go`, `go/src/time/sleep.go`, `go/src/time/tick.go`, `go/src/time/zoneinfo.go` | Durations, clocks, timers, tickers and named zones. | `std/src/time.i` has exact durations and Unix instants; `std/src/calendar.i` has Gregorian dates and fixed offsets; `lib/i/clock.i` has monotonic/wall observations and sleep. | A: common deadline, timeout, repeat and backoff policy functions. B: named timezone data and scoped periodic streams. Keep wall instants distinct from monotonic measurement. |
| context: `go/src/context/context.go` | Cancellation trees, deadlines and request values. The Context argument goes first. | `lib/i/task.i`, `lib/i/request.i`, `lib/i/resource.i` already handle scoped work, request ownership and unwinding; `std/src/http.i` has a timeout helper. | Adapt: general scoped deadlines and typed request metadata services. Do not add a Context parameter or an untyped value bag. Needs already express capabilities. |
| errors: `go/src/errors/errors.go` | Error construction, wrapping, joining and tree inspection. | `lib/i/fail.i` provides raise/catch/conversion. Modules define labelled errors; several low-level codes are strings. | A: rich module errors and consistent operation/resource context. Typed unions and matching replace Is/As-style inspection; avoid a universal string error. |
| fmt: `go/src/fmt/doc.go` | Formatting to strings, writers or stdout; scanning. | `lib/i/core.i` has inspect; interpolation is defined in `docs/spec.md` §1.6; `std/src/int.i` and `std/src/float.i` print numbers; `lib/i/stdio.i` prints lines. | A: pure configurable formatting and common writer sinks. B: scanning if real uses justify it. Prefer typed options to a second unchecked interpolation language. |
| os: `go/src/os/file.go` | Portable file operations, metadata, structured path errors. | `lib/i/fs.i` covers open/read/write/list/metadata/temp_dir and simple whole-file helpers. `lib/i/process.i` covers arguments, env, cwd, entropy and child processes. | A: safe temporary-resource helpers, seek/random access, richer metadata, recursive traversal, permissions/links where supported, and documented host capability limits. Keep process concerns separate from filesystem contracts. |
| path: `go/src/path/path.go` | Lexical slash-separated paths. | `std/src/path.i` has clean/join/split/base/dir/extension/relative and explicitly lexical POSIX rules. | Mostly covered. B: matching/globbing rules if needed. Do not treat lexical cleanup as filesystem containment or URI decoding. |
| path/filepath: `go/src/path/filepath/path.go` | Native path conventions, distinct from URL paths. | `std/src/path.i` explicitly excludes drive conventions and filesystem access. | B: an explicit native path style/provider boundary. Do not make the meaning of existing `std/path` change with the target. |
| net: `go/src/net/net.go` | TCP, UDP, Unix sockets, address interfaces and name resolution. | `lib/i/net.i` has TCP, UDP, name resolution and half close; `lib/i/tls.i` adds TLS and ALPN. | A: typed addresses and endpoints, shared I/O and deadlines. B: Unix sockets and richer resolver results. Feature availability must be explicit by target. |
| net/http: `go/src/net/http/doc.go`, `go/src/net/http/client.go`, `go/src/net/http/transport.go`, `go/src/net/http/server.go` | Client/transport/handler boundaries, response-body ownership, pooling, proxies and policy. | `std/src/http.i` has HTTP/1.1 framing, messages, native/fetch providers, redirects, scoped servers and stream-shaped replies; `std/i.toml` declares client defaults. | A: split protocol data/framing from transport and policy; complete bounded streaming and cancellation contracts. `native_client` currently collects a response before exposing its stream reads. B: cookie jar, proxy/compression policies and more protocol coverage. |
| encoding/json: `go/src/encoding/json/encode.go`, `go/src/encoding/json/stream.go` | JSON serialization, incremental encoder/decoder and tokens; documents compatibility-sensitive parser choices. | `std/src/json.i` preserves an explicit Json sum, arbitrary-size integer values and object entry lists; `std/src/schema.i` has typed two-way JSON codecs. | A: incremental tokens/decoding, resource bounds, and explicit duplicate-key/number/unknown-field policies. Keep explicit codecs; no foreign reflection dependency. |
| encoding: `go/src/encoding/encoding.go` | Common binary/text serialization interfaces. | `std/src/schema.i` defines a JSON-specific codec value. | A: common explicit codec contracts where multiple consumers need them. Return new immutable values. Do not duplicate each format's parser in adapters. |
| encoding/binary: `go/src/encoding/binary/binary.go` | Fixed-width endian conversion and varints. | `docs/spec.md` §8 and `lib/i/bits.i` cover bit segments and endian operations. | A: reusable bounded binary cursors and varints. Build on bit syntax rather than mirror fixed-width type hierarchies. |
| encoding/base64 and encoding/hex: `go/src/encoding/base64/base64.go`, `go/src/encoding/hex/hex.go` | Whole and streaming codecs with diagnostic errors. | `std/src/bit_array.i` has hex, standard and URL base64. Its docs specify zero padding for incomplete bytes. | A: offset-bearing decode errors and incremental codecs. Make bit padding deliberate; do not silently change a stable codec. |
| encoding/base32: `go/src/encoding/base32/base32.go` | Base32 alphabets, padding, whole and streaming codecs. | No base32 export in `std/src/bit_array.i`. | B: base32 when required by common wire formats. Reuse codec machinery. |
| encoding/ascii85: `go/src/encoding/ascii85/ascii85.go` | ASCII85 for document formats, with whole and streaming encoding. | No ASCII85 export in `std/src/bit_array.i`. | B: add only for concrete document-format needs. The shared incremental codec contract should make it a small extension. |
| encoding/csv: `go/src/encoding/csv/reader.go`, `go/src/encoding/csv/writer.go` | Record reading/writing, position errors and options. | `std/src/csv.i` already has strict rows, options, incremental row streams, records and schema decoding. | A: connect to common byte/text I/O and output sinks. Preserve its stated scalar line/column rules instead of assuming identical Go offsets. |
| encoding/xml: `go/src/encoding/xml/xml.go` | XML tokens, namespaces and streaming decode. | No XML module in the `std/src` inventory. | B: XML token parsing and encoding for interoperability. Use immutable values and explicit resource limits. |
| encoding/asn1 and encoding/pem: `go/src/encoding/asn1/asn1.go`, `go/src/encoding/pem/pem.go` | DER data and PEM blocks, used in certificate/key interchange. | `lib/i/tls.i` accepts certificate, key and root bytes; `std/src/crypto.i` has SHA-2/HMAC/KDFs, not these formats. | B: typed PEM/DER helpers if certificate tooling is in scope. TLS bytes alone are not format coverage. |
| encoding/gob: `go/src/encoding/gob/doc.go` | Self-describing Go value streams. | `std/src/schema.i` is JSON-specific; no native binary object stream. | No requirement to copy gob. B: a versioned explicit binary schema only if i needs one. Do not expose private runtime representations as a wire format. |
| sync: `go/src/sync/mutex.go`, `go/src/sync/once.go`, `go/src/sync/waitgroup.go`, `go/src/sync/map.go` | Locks, once-only work, task counting and concurrent maps. The package recommends communication for higher-level coordination. | `lib/i/task.i` has scoped groups, channels, events, refs, shares and concurrent combinators; `std/src/dict.i` is immutable. | Adapt: bounded concurrency, queues, semaphores and scoped single-flight/cache services as ordinary libraries. Do not add public mutable maps or detached WaitGroup-style work. |
| log/slog: `go/src/log/slog/doc.go`, `go/src/log/slog/handler.go` | Structured records, handler filtering, grouped attributes, text and JSON handlers. | `std/src/log.i` has levels, entries, string fields, call-time Sink, filtering, lazy messages and stdio/file adapters. | A: typed attribute values, scoped annotations, structured JSON output and test capture. Preserve explicit timestamping and service replacement. Avoid a global mutable default logger. |
| testing: `go/src/testing/testing.go`, `go/src/testing/example.go` | Unit tests, examples checked against output, benchmarks, fuzzing, cleanup and temporary directories. | `std/src/test.is` defines test/suite/each/fails macros; `std/src/property.i` has generation, shrinking and replay; `std/test/` has broad unit tests and some provider tests. | A: per-function example linkage, shared fake services, cleanup fixtures and scoped scheduler tests. B: benchmark and persistent fuzz-corpus tooling. Do not infer complete coverage from module test filenames. |

The supplied Go tree also contains experimental JSON packages.
`go/src/encoding/json/v2/doc.go` separates semantic value conversion from
`go/src/encoding/json/jsontext/doc.go` token processing. Both files require
`goexperiment.jsonv2`. They support the proposed split between i codecs and
incremental syntax processing. Their presence is not evidence of a stable
Go 1 API promise for those packages. i has neither public token processing nor
an equivalent format-neutral codec layer in `std/src/json.i` and
`std/src/schema.i`; those remain proposed additions.

### 3.3 What “powerful from the start” should mean

Ship complete task paths in the distribution. The public API need not be one
large package. Proposed acceptance examples are:

- Read a bounded file or network body, scan UTF-8 lines, decode records, and
  stop early without leaking the source.
- Call HTTP with an explicit client, deadline and retry policy. Stream the
  reply through a decoder without first buffering the whole body.
- Run a server with scoped handlers, bounded concurrency, contextual logs,
  typed failures and cancellation that waits for cleanup.
- Build a CLI with typed options, environment access, useful diagnostics and
  an end-to-end example using fake host services.
- Use every pure codec and collection operation without an I/O host.

These are proposed release goals, not claims that they work now. Existing
starting points are `examples/io-fs/src/main.i`,
`examples/io-clock/src/main.i`, `std/test/http_network_test.i`,
`std/test/clock_test.i`, `std/src/cli.i`, `std/src/csv.i`, and
`std/src/stream.i`. The stream docs explicitly require the resource scope to
surround consumption. That requirement should remain visible in examples.

Effect remains the capability reference. `effect/packages/effect/src/Effect.ts`
documents retry, timeout, concurrent traversal and scoped operations.
`effect/packages/effect/src/Schedule.ts` models retry/repeat/pacing policy.
Implement these user capabilities with ordinary i functions, immutable policy
state and structured task scopes. `effect/packages/effect/src/Layer.ts`
also describes resource management and shared service construction: i still
needs those guarantees, but uses factories, explicit sharing and resource
scopes. Merely deleting the name Layer does not supply its useful behavior.

`effect/packages/platform/node/src/NodeFileSystem.ts` separates a host provider
from the platform-independent FileSystem service. The corresponding i boundary
is an ordinary service and provider function. The package name must not acquire
compiler privilege.

## 4. Proposed i conventions

All snippets in this section use syntax three. New names are illustrative
proposals, not exports that already exist. Needs omitted on function bodies
are inferred, as specified in `docs/spec.md` §6.8.

### 4.1 Names and argument order

| Rule | Reason | Example |
|---|---|---|
| Use snake_case values/modules and UpperCamelCase types. | Matches `docs/spec.md` §1.3 and existing code. | `read_exact`, `ParseError`, `std/bit_array`. |
| Prefer one short operation name within its module. Use `is_`/`has_` for Boolean questions. | Qualification carries context; names reveal the kind of answer. | `dict.insert(values, key, value)`, `string.is_empty(text)`. |
| Use `parse`/`decode` for validation; `print`/`format`/`encode` for representations. Use `from_`/`to_` for named representation boundaries. | A caller can find the inverse without guessing. Document every lossy conversion. | `time.from_unix_nanoseconds(n)`, `bit_array.to_string(bytes)`. |
| Put the subject first, configuration next, callback last. A fold callback takes accumulator first, element second. | Pipes insert position zero; callback order stays predictable. | `items |> list.fold(0, (total, x) { total + x })`. |
| A codec/parser value may be the subject while building it. The input is the subject while running it. | Both builder pipelines and data pipelines remain natural. | Proposed `schema.decode(value, codec)`; existing `schema.decode(codec, value)` would migrate before stability. |
| Use distinct words for time/size/index units. | An Int alone does not explain a unit. | `clock.sleep_ns(n)` at the host edge; `time.sleep(duration)` at the high-level API. |
| Do not add `try_` merely because a callback may raise. | Fail is already a need. Ordinary map propagates it. | `list.map(texts, parse_port)`; use `fail.catch` if the whole traversal should return Result. |
| Use shape groups for a default and an explicit configuration of one operation. | Avoid `_with` families when shape alone expresses the difference. | The `split` definition below. |

```i
(
	std/string
)

split =
	(text : String) -> List(String) { split(text, " ") }
	| (text : String, separator : String) -> List(String) {
		string.split(text, separator)
	}

words = "one two" |> split
parts = "one,two" |> split(",")
```

Shape groups share a need and, as stored function values, a union of results.
Do not overload pure and host operations under one name if this makes a pure
use need host services. Prefer one result family. This follows
`docs/spec.md` §6.12 and `docs/design/syntax-3.md` §2.6. Exported shapes and
labels are contracts, not optional named arguments. No keywords are added.

For labelled data, use nouns for fields and expose constructors only when
arbitrary construction is valid. A stable `Port` that requires validation
should export its type and constructor functions, not its raw constructor.
Use ordinary `Pair`/tuple syntax for anonymous positional data and named fields
for domain data. Rules come from `docs/spec.md` §§4.3, 6.4 and 2.4.

### 4.2 Propagate callback needs

The public contract of map is conceptually:

```i
// map : <A, B, N>(List(A), (A) <- N -> B) <- N -> List(B)
```

This is one traversal. `N` may include a clock, logger or `Fail(ParseError)`.
Do not add map_effect, a wrapped Effect value or a second task runtime.
`std/src/list.i` already accepts need-bearing callbacks; normalize its docs
and expose their inferred needs accurately.

Deferred values must say when callbacks run and which services are captured.
For example, `std/src/stream.i` uses services at pull time, while
`std/src/http.i` provider factories capture transport services. Both are valid.
The difference must be documented. Giving a new service with `<-` does not
replace a value already captured by a closure (`docs/spec.md` §9.2).

### 4.3 Failures, absence and stored outcomes

| Situation | Proposed public contract | Reason |
|---|---|---|
| Ordinary lookup with a normal missing case | `Option(A)` | Absence is useful data. Examples: dictionary lookup, first element, optional header. |
| Operation cannot fulfill its requested result | `(P) <- (Fail(ModuleError), ..N) -> A` | The caller composes ordinary calls and handles errors at a chosen boundary. This applies to pure validation too. |
| The caller wants to store, inspect, batch or transmit success/failure | `Result(A, E)` obtained by `fail.catch` | An outcome is data at that boundary. It is not a second execution mechanism. |
| Decoder accumulates multiple diagnostics, or parser reports `More`/`Complete`/`Invalid` | Explicit domain data | All alternatives are part of the requested answer. Do not force these protocols through first-error Fail. |
| Invalid internal assumption | Assert/stop with a documented invariant | Do not turn user input errors into assertions. Assert is separate from Fail. |

Sources for the language distinction are `docs/spec.md` §§9.3, 9.6. Existing
`std/src/json.i` uses Fail internally and catches at `parse`; `std/src/uri.i`
and `std/src/calendar.i` also expose Result validation. Under this proposal,
ordinary public validation would raise, with Result created at the caller's
chosen boundary. This is a deliberate pre-stability API change, not a claim
that E3 alone changes those functions.

```i
(
	i/fail
	std/int
	std/list
)

InvalidPort = (text = String)
PortError = InvalidPort

parse_port : (String) <- (Fail(PortError)) -> Int = (text) {
	int.parse(text) |>
		(Ok(n) ? n >= 1 && n <= 65535) { n }
		| (_) { fail.raise(InvalidPort(text = text)) }
}

// One ordinary call chain. Parsing stops at the first raised error.
ports = () { list.map(["80", "443"], parse_port) }

// Capture the whole traversal, or capture each element, intentionally.
all_ports = () { fail.catch(ports) }
each_port = () {
	list.map(["80", "bad"], (text) { fail.catch(() { parse_port(text) }) })
}
```

Do not put catches around every internal call. Catch at recovery, reporting,
parallel outcome collection or storage boundaries. To handle only one error
case, catch the union, match it, and raise the remaining cases. That is the
specified narrowing pattern in `docs/spec.md` §9.3.

### 4.4 Error ownership

Use a module-owned `ModuleError` sum as the normal entry point. Several related
operations may share it. Distinct recovery conditions should be distinct
labelled cases with useful fields. A module may expose smaller error sums when
that materially improves precision. Do not force unrelated domains into one
global Error. Do not allocate one error type for every function.

For example, a proposed file error can carry `operation`, `path` and a typed
reason. `NotFound`, `Denied` and `Unsupported` are useful conditions.
Keep an optional host detail for diagnosis; do not make applications parse
English messages. `lib/i/fs.i` already has operation/path context, but its code
is a String. `std/src/uri.i` demonstrates structured parts and offsets.

```i
NotFound = ()
Denied = ()
Unsupported = ()
FileProblem = NotFound | Denied | Unsupported
FileError = (operation = String, path = String, problem = FileProblem)
```

Share a reason type only when the contract is truly shared, for example the
common byte I/O contract. Keep ownership in one module. Higher layers may
preserve lower-layer errors in a union or wrap them with domain context.
Do not erase them into String merely to make every function return the same E.
Adding a case to a stable closed error sum is a breaking change: clients may
match it exhaustively (`docs/spec.md` §§4.3, 7.4).

### 4.5 Services and providers

A service is a nominal labelled value, usually with function fields. Name it
for its responsibility, such as `Clock`, `Settings` or `Client`. Do not append
Effect/Layer to the type. Keep the smallest useful capability together.
Two independently replaceable services need distinct nominal type constructors
because needs are keyed by constructor (`docs/spec.md` §9.2).

```i
(
	option = (Option, Some, None) = std/option
)

Unavailable = (detail = String)
SettingsError = Unavailable
Settings = (read = (String) <- (Fail(SettingsError)) -> Option(String))

read = (key : String) <- (settings : Settings) { settings.read(key) }

defaults = () -> Settings {
	Settings(
		read = (key) { key |> ("region") { Some("local") } | (_) { None } },
	)
}

example = () { read("region") <- (defaults()) }
```

There is no special service declaration or registry access in source.

Name provider functions by a real choice: `native_client`, `fetch_client`,
`stderr`, `from_config`, or `fixed`. Return an ordinary service, or positional
data of services. Provide with `app() <- (service)` or `app() <- services`.
Construct shared services once at the owner scope. Do not assume a factory
call is automatically memoized. A resource-backed provider's scope surrounds
every use and its cleanup.

Default selection belongs at the application boundary. Explicit provision
remains the simplest example. Package/target defaults may use the existing
provider plan data from `docs/spec.md` §2.6.1; `std/i.toml` already selects
native and fetch HTTP clients. This is the same mechanism available to other
packages. Document the chosen target defaults, their needs and unsupported
targets. Missing providers are compile errors, not optional runtime discovery.
Startup that may fail remains an ordinary Fail-bearing acquisition; provider
plan factories themselves obey the spec's no-uncaught-error restriction.

### 4.6 i/ versus std/

| Place | Rule and example | Reason |
|---|---|---|
| `i/…` | Runtime primitives, host capability contracts and their minimal adapters: `i/control`, `i/request`, `i/fs`, `i/net`, `i/clock`, `i/core`. | These operations need runtime or host cooperation. Keep the boundary small. |
| `std/…` | Collections, text, dates, codecs, policies, protocols and reusable algorithms: `std/list`, `std/uri`, `std/schema`. | Ordinary i should implement everything that can be expressed there. |
| `std/…` using i needs | High-level HTTP, logging, CLI, time convenience and buffered I/O may require host services through `i/…`. | Depending on a host need does not make an algorithm a host primitive. |
| Separate adapter module where useful | Keep native HTTP/fetch construction and sink output apart from pure HTTP messages or log formatting. | Pure use should not accidentally import transport authority or an incompatible target implementation. |

`docs/spec.md` §2.1 reserves supplied `i/` library paths, and the introduction
lists compiler-required modules. That distribution rule is not permission for
name-based implementation shortcuts. A later module move must respect those
specified contracts. `lib/i/random.i` is pure today but is specified runtime
support. Do not move it casually merely because the placement rule favors
ordinary algorithms in std. Also review whole-file convenience algorithms in
`lib/i/fs.i` for a future std home without duplicating their implementation.

### 4.7 Documentation, tests and examples

Use `////` for a module introduction and `///` for each exported item, as
defined in `docs/spec.md` §1.2. Avoid copying a second signature catalogue into
module comments. `std/src/log.i` shows how that catalogue can retain obsolete
effect syntax after executable signatures change.

Each public function should have:

- One sentence stating the result and the normal empty/missing case.
- Units, order, validation and meaningful cost guarantees. State index units.
- Needs and error cases. For deferred callbacks, state when they run and which
  services they use. For resources, state the owner and early-exit behavior.
- A short syntax-three example with expected data or observed behavior.
- A linked test case for its main contract and important boundary cases.

```i
/// The first item, or None when the list is empty.
///
/// Stability: experimental. Since: next release.
/// Example: first([7, 8]) == Some(7).
first = <A>(items : List(A)) -> Option(A) {
	items |> ([]) { None } | ([x, .._]) { Some(x) }
}
```

The example assumes the normal Option import. Stability text here is a proposed
documentation convention, not a new built-in attribute.

Use `std/test/<module>_test.i` and the ordinary `std/test` macros. Group several
small functions in one table-driven test when appropriate; do not require a
test file per function. Cover laws and observable behavior rather than private
tree shapes or closure identity. `std/test/sequence_test.i` and
`std/test/property_test.i` already demonstrate this distinction.

Every published function should map to a test or a documented reason why a
shared test covers it. Not every function needs a property generator. Prioritize
round trips, traversal order, stable sorting, early stop, errors, limits and
cleanup. For callback APIs include a service-bearing callback and a failure.
For resource APIs include cancellation and early exit. Use fake clock,
filesystem, client and log services where possible. `std/test/clock_test.i`
already substitutes a Clock with `<-`; `std/test/http_network_test.i` has a
non-terminating provider timeout case and a local network example.

For example, a test for the proposed port parser uses the same ordinary catch
as application code. A complete test module imports `std/test`, `i/fail`,
and the module that defines `parse_port` and `InvalidPort`:

```i
test("parse_port rejects invalid input", () {
	assert(
		fail.catch(() { parse_port("bad") }) ==
			Error(InvalidPort(text = "bad")),
	)
})
```

Provide one complete example per module family under `examples/`. Keep docs and
examples linked to the same contract tests. A future documentation runner can
extract examples, but the audit did not verify such a runner exists. The model
of checked examples comes from `go/src/testing/example.go`; rich unit-test and
fuzzing documentation appears in `go/src/testing/testing.go`.

## 5. Proposed stability policy

### 5.1 Scope and marking

Treat every public item as experimental until explicitly reviewed. The current
`docs/spec.md` §2.7 compatibility requirement still applies to published package
majors; the word experimental must not silently waive that language contract.

Use documentation metadata, initially ordinary comment lines:

```text
Stability: stable. Since: 1.0.0.
Stability: experimental. Since: 0.4.0. Tracking: issue or design path.
```

Module metadata may supply the default, with explicit per-item overrides.
Stable items must not expose experimental types or require experimental
services in their public closure. Record stable exports, constructors, shapes,
error cases and required host namespaces in a generated release manifest.
Comment parsing and that manifest are proposed additions, not existing tools.

The precedent for explicit maturity labels is
`effect/packages/effect/src/Effect.ts`, whose module and public items have
`@stability stable`, and
`effect/packages/platform/node/src/NodeFileSystem.ts`, which is marked
unstable. i should keep its own vocabulary and enforce its own package rules.

While the standard library is being restructured, do breaking work before its
first stable release and explain migrations. Once a major is published, even
experimental exports follow §2.7. If rapid incompatible trials are needed,
keep them unpublished or use a separate experimental package with new major
versions. A directory called experimental inside one published package cannot
escape the interface comparison.

### 5.2 What a stable item promises

Stable means existing correct source continues to type-check and has the
documented behavior within that package major. The promise includes needs,
errors, constructor visibility, callback order, lazy/eager behavior, byte/scalar
units, ownership and cancellation. It includes stated wire formats and target
availability. It does not promise private representations, unspecified order,
exact timings or identical performance unless documented.

The local Go tree has no standalone `doc/go1compat` file. The promise is stated
in `go/doc/go_spec.html`, appendix “Language versions”.
`go/doc/godebug.md` discusses bugs, security fixes and compatibility switches.
The full [Go 1 compatibility document](https://go.dev/doc/go1compat), read for
the missing policy text, describes source compatibility rather than binary
compatibility, allows API growth, and states exceptions for security,
unspecified behavior and specification-violating bugs. It also lists rare
breaks from API additions and excludes unreleased features from its guarantee.

Take the discipline, not every exception. i should retain old behavior where
the documented contract allows it. A bug fix that restores that contract may
be a patch. A security fix that must break a documented guarantee needs a clear
advisory, migration path and explicit release decision. Never describe an
intentional documented behavior change as a harmless patch just because the
function type is unchanged.

`go/src/encoding/json/encode.go` explicitly describes parser behaviors that
compatibility prevents changing. `go/src/encoding/encoding.go` warns that even
adding serialization methods can change wire behavior. These are reasons to
settle i's parsing and encoding policies before marking those APIs stable.

| Change to a stable item | Proposed release rule |
|---|---|
| Internal fix preserving documented semantics and interface | Patch, with regression evidence. |
| New independent export | Minor, after checking import-name conflicts and public dependencies. |
| Change parameter order/labels/shapes, result, required needs, error union or constructor visibility | Major under the current comparator. Do not waive its conservative result on intuition alone. |
| Add a constructor field or an error/sum member | Major. Labelled fields in i do not create Go's omitted-field default behavior. Exhaustive matches and constructor calls are contracts. |
| Change parser acceptance, ordering, encoding, replay, cleanup or cancellation guarantee | Behavioral review; major when documented callers can break, even if the interface is identical. |
| Add a shape branch to an existing function | Review selection and stored function behavior; treat a changed exported function type as major under current tooling. |
| Mark a stable item deprecated | Compatible warning plus a complete replacement and migration example; minor release is preferred. |

Deprecate with the existing `@deprecated("use …; migration …")` attribute
(`docs/spec.md` §2.2). Keep the item working throughout its stable major.
Removal is only in a new major. As a proposed minimum, publish the replacement
and warning in one minor release before removal in a later major. Specify any
shorter security exception explicitly. Implement compatibility wrappers through
the new canonical operation when semantics permit; do not maintain two engines.

### 5.3 Use the machine check that already exists

`crates/ilang-cli/src/commands/package.rs` implements the following in
`publish`, `interface_of`, `compare_with`, and `Diff::version_step`:

| Existing check | Exact scope or limitation |
|---|---|
| Clean working tree and valid release version | Requires a commit and a version newer than all parsed release tags. |
| Package check and tests | `check_and_test` runs them even for publish dry runs. No publish command was run in this audit. |
| Earlier baseline | Compares with the newest version tag, not an arbitrary hand-written snapshot. A missing package at that tag skips comparison. An unreadable old interface normally requires a major step; the legacy-syntax exception below skips it. |
| Public values and types | Includes exported value types, exported types/aliases, visible constructors and their fields. Module keys are relative to the package. |
| Host authority | Includes external imports and host service contracts, including relevant ones that are not themselves exported. |
| Canonical comparison | Tracks type identity, normalized type variables and semantic alias/scheme equivalence where supported. It is more than a text diff. |
| Version decision | Removal, change or unresolved representation change requires major; additions require minor; no interface change permits patch. Proof budget exhaustion is an error. |
| Publication | Creates a local tag after checks. It does not push it. Dry run does not create the tag. |

There is a migration exception in `earlier_interface`: when every old diagnostic
is attributed to source marked as pre-A3 syntax, it prints a manual-review
message and returns no baseline. Comparison is skipped. Release CI should
require a recorded manual review for that case, or seed a checked baseline
after migration. Do not claim the publisher proves compatibility across this
exception (`crates/ilang-cli/src/commands/package.rs`).

Use that implementation as the single interface comparison engine. Proposed
follow-up work should expose its read-only comparison as a CI command, so CI
can check against a selected release tag without creating tags. Do not build
a competing signature diff in a documentation script. Continue to use
`i publish --dry-run` as the final existing release gate, with the intended
version and package path. Its check/test execution belongs to an implementation
or release phase, not this audit.

Add these checks around the same engine:

1. Reject a stable public signature whose reachable types/services are
   experimental. Check all stable constructors and callback types too.
2. Compare maturity metadata. Reject silently dropping the stable label.
   Require a replacement and release note for a new deprecation.
3. Require a reviewed behavioral change note for each changed stable contract.
   Link tests and examples to the item. Signature equality alone is insufficient.
4. Compile representative clients from the previous release. Check target
   provider plans and documented default services, not only pure functions.
5. Preserve a release support table for native, nodejs, js, wasi and host.
   Namespace or capability changes are reviewed as interface changes.

These checks are proposals. The current `Interface` and `Diff` do not read
stability comments or prove behavior (`crates/ilang-cli/src/commands/package.rs`).
The spec also identifies packages by location and major, and uses minimal
version selection within a major (`docs/spec.md` §§2.6–2.8). Compatibility
must therefore hold for the whole major; “works only with the latest dependent
code” is not enough. Do not weaken this rule for version zero without a
separate language/package policy decision.

For the compiler-supplied `i/` modules, add a corresponding exported-contract
snapshot using the same comparison machinery and the compiler release policy.
The ordinary package publisher is not by itself proof that compiler-shipped
host contracts have stayed compatible. Keep the specified `i/` ABI and needs
in this review (`docs/spec.md` introduction, §9.7 and Appendix B).

## 6. Proposed order of work after this audit

1. Finish E3 documentation and canonical failure entry points. Decide the
   validation boundary and error naming rules before widening the API.
2. Define common byte I/O and resource contracts. Adapt existing fs, net, TLS
   and stdio services. Add buffering and bounded incremental codecs.
3. Apply naming, argument-order and shape-group conventions while still
   unstable. Split pure protocol/data modules from provider adapters where
   imports otherwise carry unwanted host authority.
4. Complete scoped timeout/retry/pacing and application examples. Verify the
   whole task paths from §3.3, including early exit and cancellation.
5. Add maturity metadata, behavior review and CI access to the existing
   interface comparison. Stabilize modules only after these contracts and
   representative clients agree.

This proposal does not add a second effects system, detached concurrency,
mutable public collections, keywords, or compiler-special standard modules.
It asks the library to use the mechanisms i already defines, with complete
contracts and a release policy strong enough to keep them useful.
