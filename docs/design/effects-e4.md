# E4 standard-library operations

Reviewed against syntax-3 §7.2 row E4 and §2.10.6:

> **Library style.** An operation that may fail reports through `Fail`;
> `Result` is data used after `catch`. About 140 functions of `std/src` and
> `lib/i` that return `Result` today (`json.parse`, `float.divide`, …) change
> in stage E4.

The table lists 75 exported operations with written or inferred `Result`
return types in `std/src`; each now raises its existing error. The private `_name_result`
helpers retain the original implementation and same-module Result composition.
Other std and example callers use `fail.catch` to keep their previous Result
handling, including every success/error assertion. Function-valued uses are
adapted with a function literal; piped calls keep their first argument.
Arguments containing calls are evaluated before entering the adapter catch,
so their errors still reach the caller (notably URI escape validation).
The power-pipe and chained-slice tests keep named function calls in their
pipelines, catching around the pipeline while retaining the original values.
Parameter names in this table document arguments, rather than type labels.
The table shows the Fail need instantiated at the original Result error type.
Printed schemes may additionally contain open error-row variables; the host
error translators bind `Fail(String)` to keep their export contract closed. Wrapper literal heads omit it, following
§2.10.1: "A function that needs a service only to pass it on writes nothing:
the need passes down by R2." Their call to `fail.from_result` infers Fail;
the other needs of original bodies also continue to be inferred.

Per-function choices for Result data:

- All of `std/result` and `option.to_result` construct or consume Result data.
- `list.try_map`, `try_fold`, and `try_each` sequence Result-valued callbacks;
  these explicitly choose a Result-based interface, alongside effectful map/fold/each.
- `cli.text` and `cli.integer` are Result-valued conversion callbacks stored in
  `Parser`; keep that protocol. The parsing/help operations themselves raise.
- `schema.Schema.read` and the private parser/codec helpers keep their stored
  Result protocol; public decode/parse operations raise.
- CSV streams of Result rows and stream `from_results`/`to_results` keep Result
  elements as data; parsing a complete document raises.
- `http.body_text` and `cli.read` have inferred Result returns; both are
  included below because they forward a failing operation.

The initial port owns std, examples, this table, and the migration script.
The follow-up additionally covers the JavaScript/browser fetch callers and
the native HTTP integration-test fixture.
The `resource.acquire`/`Scope` library change in §2.10.9 requires `lib/i`, which
this lane is explicitly instructed not to edit. The only std caller, `log.file`,
now opens its handle and uses `resource.ensure` directly: §2.10.9 says
"`resource.ensure` is unchanged." This keeps its release after each log entry
and remains valid after the library owner replaces three-argument acquire.

`random_bytes_service_error` retains its original single catch and its exact
`process.ProcessError` assertion. Adding another catch would turn the caught
service failure into `Ok(Error(...))`; §9.3 of the spec says catch takes every
error, and §2.10.6 puts both library and service failures through Fail.
`e4_json_parse_fail` additionally pins direct success and the requested
`json.parse("{")` caught error, including its reason and offset.

| Function | Old type | New type | Callers adapted (module-local calls retain private helpers) |
|---|---|---|---|
| `bit_array.to_string` | `(bits: BitArray) -> Result(String, Nil)` | `(bits: BitArray) <- (Fail(Nil)) -> String` | `examples/http-server/src/main.i`, `std/src/http.i`, `std/src/json.i`, `std/src/uri.i`, `std/test/bit_array_test.i` |
| `bit_array.slice` | `(from: BitArray, at: Int, take: Int) -> Result(BitArray, Nil)` | `(from: BitArray, at: Int, take: Int) <- (Fail(Nil)) -> BitArray` | `std/test/bit_array_test.i` |
| `bit_array.base16_decode` | `(input: String) -> Result(BitArray, Nil)` | `(input: String) <- (Fail(Nil)) -> BitArray` | `std/src/regex.i`, `std/test/bit_array_test.i`, `std/test/crypto_test.i` |
| `bit_array.base64_decode` | `(encoded: String) -> Result(BitArray, Nil)` | `(encoded: String) <- (Fail(Nil)) -> BitArray` | `std/test/bit_array_test.i` |
| `bit_array.base64_url_decode` | `(encoded: String) -> Result(BitArray, Nil)` | `(encoded: String) <- (Fail(Nil)) -> BitArray` | `std/test/bit_array_test.i` |
| `calendar.days_in_month` | `(year: Int, month: Int) -> Result(Int, CalendarError)` | `(year: Int, month: Int) <- (Fail(CalendarError)) -> Int` | `std/test/calendar_test.i` |
| `calendar.date` | `(year: Int, month: Int, day: Int) -> Result(Date, CalendarError)` | `(year: Int, month: Int, day: Int) <- (Fail(CalendarError)) -> Date` | `std/test/calendar_test.i` |
| `calendar.time_of_day` | `(hour: Int, minute: Int, second: Int, nanosecond: Int) -> Result(TimeOfDay, CalendarError)` | `(hour: Int, minute: Int, second: Int, nanosecond: Int) <- (Fail(CalendarError)) -> TimeOfDay` | `std/test/calendar_test.i` |
| `calendar.offset` | `(minutes: Int) -> Result(Offset, CalendarError)` | `(minutes: Int) <- (Fail(CalendarError)) -> Offset` | `std/test/calendar_test.i` |
| `calendar.parse` | `(text: String) -> Result((Civil, Offset), CalendarError)` | `(text: String) <- (Fail(CalendarError)) -> (Civil, Offset)` | `std/test/calendar_test.i` |
| `cli.parse` | `<A>(parser: Parser(A), args: List(String)) -> Result(A, CliError)` | `<A>(parser: Parser(A), args: List(String)) <- (Fail(CliError)) -> A` | `std/test/cli_test.i` |
| `cli.help` | `<A>(parser: Parser(A), program: String) -> Result(String, DefinitionError)` | `<A>(parser: Parser(A), program: String) <- (Fail(DefinitionError)) -> String` | `std/test/cli_test.i` |
| `cli.read` | `<A>(parser: Parser(A)) -> Result(A, CliError)` | `<A>(parser: Parser(A)) <- (Fail(CliError)) -> A` | No external std/example callers |
| `crypto.hmac_sha256` | `(key: BitArray, input: BitArray) -> Result(BitArray, CryptoError)` | `(key: BitArray, input: BitArray) <- (Fail(CryptoError)) -> BitArray` | `std/test/crypto_test.i` |
| `crypto.hmac_sha512` | `(key: BitArray, input: BitArray) -> Result(BitArray, CryptoError)` | `(key: BitArray, input: BitArray) <- (Fail(CryptoError)) -> BitArray` | `std/test/crypto_test.i` |
| `crypto.hkdf_extract_sha256` | `(salt: BitArray, input: BitArray) -> Result(BitArray, CryptoError)` | `(salt: BitArray, input: BitArray) <- (Fail(CryptoError)) -> BitArray` | `std/test/crypto_test.i` |
| `crypto.hkdf_extract_sha512` | `(salt: BitArray, input: BitArray) -> Result(BitArray, CryptoError)` | `(salt: BitArray, input: BitArray) <- (Fail(CryptoError)) -> BitArray` | `std/test/crypto_test.i` |
| `crypto.hkdf_expand_sha256` | `(prk: BitArray, info: BitArray, length: Int) -> Result(BitArray, CryptoError)` | `(prk: BitArray, info: BitArray, length: Int) <- (Fail(CryptoError)) -> BitArray` | `std/test/crypto_test.i` |
| `crypto.hkdf_expand_sha512` | `(prk: BitArray, info: BitArray, length: Int) -> Result(BitArray, CryptoError)` | `(prk: BitArray, info: BitArray, length: Int) <- (Fail(CryptoError)) -> BitArray` | No external std/example callers |
| `crypto.hkdf_sha256` | `( input: BitArray, salt: BitArray, info: BitArray, length: Int ) -> Result(BitArray, CryptoError)` | `( input: BitArray, salt: BitArray, info: BitArray, length: Int ) <- (Fail(CryptoError)) -> BitArray` | `std/test/crypto_test.i` |
| `crypto.hkdf_sha512` | `( input: BitArray, salt: BitArray, info: BitArray, length: Int ) -> Result(BitArray, CryptoError)` | `( input: BitArray, salt: BitArray, info: BitArray, length: Int ) <- (Fail(CryptoError)) -> BitArray` | `std/test/crypto_test.i` |
| `crypto.pbkdf2_sha256` | `( password: BitArray, salt: BitArray, iterations: Int, length: Int ) -> Result(BitArray, CryptoError)` | `( password: BitArray, salt: BitArray, iterations: Int, length: Int ) <- (Fail(CryptoError)) -> BitArray` | `std/test/crypto_test.i` |
| `crypto.random_bytes` | `(length: Int) -> Result(BitArray, CryptoError)` | `(length: Int) <- (Fail(CryptoError)) -> BitArray` | `std/test/crypto_test.i` |
| `csv.options` | `(delimiter: String, fields: Int, crlf: Bool) -> Result(Options, CsvError)` | `(delimiter: String, fields: Int, crlf: Bool) <- (Fail(CsvError)) -> Options` | `std/test/csv_test.i` |
| `csv.parse` | `(input: String) -> Result(List(List(String)), CsvError)` | `(input: String) <- (Fail(CsvError)) -> List(List(String))` | `std/bench/std4/src/main.i`, `std/test/csv_test.i` |
| `csv.parse_with` | `(input: String, config: Options) -> Result(List(List(String)), CsvError)` | `(input: String, config: Options) <- (Fail(CsvError)) -> List(List(String))` | `std/test/csv_test.i` |
| `csv.parse_header` | `(input: String, config: Options) -> Result(List(dict.Dict(String, String)), CsvError)` | `(input: String, config: Options) <- (Fail(CsvError)) -> List(dict.Dict(String, String))` | `std/test/csv_test.i` |
| `csv.decode` | `<A, E>( codec: schema.Schema(A, E), row: dict.Dict(String, String) ) -> Result(A, E)` | `<A, E>( codec: schema.Schema(A, E), row: dict.Dict(String, String) ) <- (Fail(E)) -> A` | `std/test/csv_test.i` |
| `float.parse` | `(text: String) -> Result(Float, Nil)` | `(text: String) <- (Fail(Nil)) -> Float` | `std/src/json.i`, `std/test/float_test.i` |
| `float.power` | `(base: Float, of: Int) -> Result(Float, Nil)` | `(base: Float, of: Int) <- (Fail(Nil)) -> Float` | `std/test/float_test.i` |
| `float.square_root` | `(x: Float) -> Result(Float, Nil)` | `(x: Float) <- (Fail(Nil)) -> Float` | `std/test/float_test.i` |
| `float.divide` | `(a: Float, by: Float) -> Result(Float, Nil)` | `(a: Float, by: Float) <- (Fail(Nil)) -> Float` | `std/test/float_test.i` |
| `float.modulo` | `(a: Float, by: Float) -> Result(Float, Nil)` | `(a: Float, by: Float) <- (Fail(Nil)) -> Float` | `std/test/float_test.i` |
| `float.exponential` | `(x: Float) -> Result(Float, Nil)` | `(x: Float) <- (Fail(Nil)) -> Float` | `std/test/float_test.i` |
| `float.logarithm` | `(x: Float) -> Result(Float, Nil)` | `(x: Float) <- (Fail(Nil)) -> Float` | `std/test/float_test.i` |
| `float.real_power` | `(base: Float, of: Float) -> Result(Float, Nil)` | `(base: Float, of: Float) <- (Fail(Nil)) -> Float` | `std/test/float_test.i` |
| `float.asin` | `(x: Float) -> Result(Float, Nil)` | `(x: Float) <- (Fail(Nil)) -> Float` | `std/test/float_test.i` |
| `float.acos` | `(x: Float) -> Result(Float, Nil)` | `(x: Float) <- (Fail(Nil)) -> Float` | `std/test/float_test.i` |
| `float.degrees` | `(radians: Float) -> Result(Float, Nil)` | `(radians: Float) <- (Fail(Nil)) -> Float` | `std/test/float_test.i` |
| `float.hypot` | `(x: Float, y: Float) -> Result(Float, Nil)` | `(x: Float, y: Float) <- (Fail(Nil)) -> Float` | `std/test/float_test.i` |
| `float.tan` | `(x: Float) -> Result(Float, Nil)` | `(x: Float) <- (Fail(Nil)) -> Float` | `std/test/float_test.i` |
| `http.parse_query` | `(query: String) -> Result(List((String, String)), uri.CodecError)` | `(query: String) <- (Fail(uri.CodecError)) -> List((String, String))` | `std/test/http_test.i` |
| `http.body_text` | `(res: Response) -> Result(String, Nil)` | `(res: Response) <- (Fail(Nil)) -> String` | `examples/http-get/src/main.i`, `tests/browser/fixtures/http-fetch/src/main.i`, `bindings/javascript/test/fetch.test.mjs` |
| `http.body_json` | `(res: Response) -> Result(json.Json, BodyError)` | `(res: Response) <- (Fail(BodyError)) -> json.Json` | No external std/example callers |
| `http.body_decode` | `<A>( res: Response, codec: schema.Schema(A, schema.SchemaError) ) -> Result(A, BodyError)` | `<A>( res: Response, codec: schema.Schema(A, schema.SchemaError) ) <- (Fail(BodyError)) -> A` | No external std/example callers |
| `int.parse` | `(text: String) -> Result(Int, Nil)` | `(text: String) <- (Fail(Nil)) -> Int` | `std/src/cli.i`, `std/src/http.i`, `std/src/json.i`, `std/src/regex.i`, `std/test/int_test.i` |
| `int.parse_base` | `(text: String, base: Int) -> Result(Int, Nil)` | `(text: String, base: Int) <- (Fail(Nil)) -> Int` | `std/test/int_test.i` |
| `int.to_base` | `(x: Int, base: Int) -> Result(String, Nil)` | `(x: Int, base: Int) <- (Fail(Nil)) -> String` | `std/test/int_test.i` |
| `int.power` | `(base: Int, of: Int) -> Result(Int, Nil)` | `(base: Int, of: Int) <- (Fail(Nil)) -> Int` | `std/test/calendar_test.i`, `std/test/int_test.i` |
| `int.square_root` | `(x: Int) -> Result(Float, Nil)` | `(x: Int) <- (Fail(Nil)) -> Float` | `std/test/int_test.i` |
| `int.digits` | `(x: Int, base: Int) -> Result(List(Int), Nil)` | `(x: Int, base: Int) <- (Fail(Nil)) -> List(Int)` | `std/test/int_test.i` |
| `int.undigits` | `(numbers: List(Int), base: Int) -> Result(Int, Nil)` | `(numbers: List(Int), base: Int) <- (Fail(Nil)) -> Int` | `std/test/int_test.i` |
| `int.divide` | `(a: Int, by: Int) -> Result(Int, Nil)` | `(a: Int, by: Int) <- (Fail(Nil)) -> Int` | `std/test/int_test.i` |
| `int.remainder` | `(a: Int, by: Int) -> Result(Int, Nil)` | `(a: Int, by: Int) <- (Fail(Nil)) -> Int` | `std/test/int_test.i` |
| `int.modulo` | `(a: Int, by: Int) -> Result(Int, Nil)` | `(a: Int, by: Int) <- (Fail(Nil)) -> Int` | `std/test/int_test.i` |
| `int.floor_divide` | `(a: Int, by: Int) -> Result(Int, Nil)` | `(a: Int, by: Int) <- (Fail(Nil)) -> Int` | `std/test/int_test.i` |
| `json.parse` | `(text: String) -> Result(Json, JsonError)` | `(text: String) <- (Fail(JsonError)) -> Json` | `std/src/http.i`, `std/src/schema.i`, `std/test/json_test.i` |
| `list.strict_zip` | `<A, B>(list: List(A), with: List(B)) -> Result(List((A, B)), Nil)` | `<A, B>(list: List(A), with: List(B)) <- (Fail(Nil)) -> List((A, B))` | `std/test/list_test.i` |
| `path.relative` | `(from: String, to: String) -> Result(String, RelativeError)` | `(from: String, to: String) <- (Fail(RelativeError)) -> String` | `std/test/path_test.i` |
| `property.int` | `(min: Int, max: Int) -> Result(Gen(Int), BuildError)` | `(min: Int, max: Int) <- (Fail(BuildError)) -> Gen(Int)` | `std/test/float_test.i`, `std/test/property_test.i` |
| `property.element` | `<A>(choices: List(A)) -> Result(Gen(A), BuildError)` | `<A>(choices: List(A)) <- (Fail(BuildError)) -> Gen(A)` | `std/test/property_test.i` |
| `property.list` | `<A>(of: Gen(A), min: Int, max: Int) -> Result(Gen(List(A)), BuildError)` | `<A>(of: Gen(A), min: Int, max: Int) <- (Fail(BuildError)) -> Gen(List(A))` | `std/test/property_test.i` |
| `property.string` | `(max_length: Int) -> Result(Gen(String), BuildError)` | `(max_length: Int) <- (Fail(BuildError)) -> Gen(String)` | `std/test/http_test.i`, `std/test/json_test.i`, `std/test/property_test.i` |
| `regex.compile` | `(pattern: String) -> Result(Regex, RegexError)` | `(pattern: String) <- (Fail(RegexError)) -> Regex` | `std/bench/std4/src/main.i`, `std/test/regex_test.i` |
| `schema.decode` | `<A, E>(schema: Schema(A, E), value: json.Json) -> Result(A, E)` | `<A, E>(schema: Schema(A, E), value: json.Json) <- (Fail(E)) -> A` | `std/src/csv.i`, `std/src/http.i`, `std/test/schema_test.i` |
| `schema.parse` | `<A>(schema: Schema(A, SchemaError), text: String) -> Result(A, ParseError)` | `<A>(schema: Schema(A, SchemaError), text: String) <- (Fail(ParseError)) -> A` | `examples/http-server/src/main.i`, `std/test/schema_test.i` |
| `string.utf_codepoint` | `(value: Int) -> Result(UtfCodepoint, Nil)` | `(value: Int) <- (Fail(Nil)) -> UtfCodepoint` | `std/test/string_test.i`, `std/test/uri_test.i` |
| `time.divide` | `(duration: Duration, by: Int) -> Result(Duration, TimeError)` | `(duration: Duration, by: Int) <- (Fail(TimeError)) -> Duration` | `std/test/time_test.i` |
| `uri.parse` | `(text: String) -> Result(Uri, UriError)` | `(text: String) <- (Fail(UriError)) -> Uri` | `std/src/http.i`, `std/test/uri_test.i` |
| `uri.build` | `(parts: Parts) -> Result(Uri, UriError)` | `(parts: Parts) <- (Fail(UriError)) -> Uri` | `std/test/uri_test.i` |
| `uri.resolve` | `(base: Uri, reference: Uri) -> Result(Uri, UriError)` | `(base: Uri, reference: Uri) <- (Fail(UriError)) -> Uri` | `std/src/http.i`, `std/test/uri_test.i` |
| `uri.encode_bytes` | `(bytes: BitArray) -> Result(String, CodecError)` | `(bytes: BitArray) <- (Fail(CodecError)) -> String` | `std/test/uri_test.i` |
| `uri.decode_component` | `(text: String) -> Result(String, CodecError)` | `(text: String) <- (Fail(CodecError)) -> String` | `std/test/uri_test.i` |
| `uri.decode_bytes` | `(text: String) -> Result(BitArray, CodecError)` | `(text: String) <- (Fail(CodecError)) -> BitArray` | `std/test/uri_test.i` |
| `uri.parse_query` | `(query: String) -> Result(List((String, String)), CodecError)` | `(query: String) <- (Fail(CodecError)) -> List((String, String))` | `std/src/http.i`, `std/test/uri_test.i` |

Public documentation follows the new API. The original descriptions moved from
private helpers to public wrappers; examples that show Result values now use
`fail.catch`. Existing Result-data combinator examples adapt their callbacks.

The follow-up also adapts the browser HTTP fixture and the JavaScript fetch
test's embedded i source: both catch `http.body_text` before their existing
Ok/Error matches and retain their original UTF-8 error behavior.

Initial validation at `dc4e5b21` was blocked by the eff3 base's Fail handling.
The follow-up merges eff3 through `d5d8e9042` and rebuilds the compiler. Its
compiler sources are identical to `b73532527`; the final merge changes only
review fixtures and conversion documentation.

Follow-up changes and their basis:

- Browser and JavaScript fetch fixtures choose `http.fetch_client()`; native
  integration programs choose `http.native_client()`. Design §2.10.11 says:
  "All other services are provided in `main` by i code."
- Host error translators name `handler: Fail(String)` and call `handler.raise`
  on the same inspected error text. This closes the host-passable error type
  without changing the translated value (spec §9.3 and §10.3).
- The HTTP server assigns its caught parse result the type
  `Result(User, schema.ParseError)` before matching. Spec §4.8 says: "The value
  of a binding whose pattern has `x: T` at its top has `T`." The existing
  `.path` projection therefore retains its known schema error type.
- `_body_text_result` explicitly retains `Result(String, Nil)`, its original
  inferred data contract; the migration script now produces that annotation.

Verification checkpoints (2026-11-13):

- Release builds pass through the final merged eff3 `d5d8e9042` base.
- `i check --project std --types` passes on that base with two inherited HTTP
  unreachable-clause warnings.
- Final std O0 and O2 on the latest merged base each pass **2,171/2,171**.
  The earlier runs on eff3 `6b0377b2a` also pass at both optimization levels.
- `i fmt --check bench std lib examples` and the browser fixture pass; the
  Rust HTTP test fixture and JavaScript source pass their format/syntax checks.
- Native integration tests pass: **HTTP 11/11**, **CLI/log 2/2**. The first HTTP
  run exposed the example's missing known schema type; the complete rerun is
  green with every original response/error assertion unchanged.
- The browser HTTP fixture builds for JS at O0. Its first build exposed an
  open error variable at the host export; its String Fail handler closes it.
- JavaScript fetch: **9/10 subtests pass**, including GET/POST/headers/status,
  timeout, both cancellation paths, refused connection and DNS errors.
  Streaming fails with an illegal cast in `field`, `f157`, `f156`, `s28`,
  `keep_reenter`. Two attempts reproduce the same failure; adding a helper
  return annotation produced identical failing Wasm and was discarded. This
  part is stopped per the brief, and the streaming assertions remain intact.
- There is no ilang-cli `json` test target in this checkout. JSON is covered by
  the std JSON suite and the HTTP integration's schema/JSON cases.
- The requested filtered spec gates pass at both i O0 and O2: `9/` is
  **140/140**, and `11/` is **48/48**. Both runs use a two-CPU affinity limit;
  chapter 9 completes in the Rust debug test profile and chapter 11 in the
  Rust release test profile. No spec expectation is changed.
- Strict workspace Clippy, including every target, passes with `-D warnings`.
- `scripts/build-abi-fixture.sh` completes, including the R2 conformance fixture
  generator and Wasm validation. The regenerated binding plans have unchanged
  contracts; only generated identity metadata changes.
- The regenerated JavaScript R2 tests pass **3/3**, including 200 seeded replays
  at O0/O2 and every runnable R2 law body at both levels.
- Regenerated JavaScript compiled-fixture tests pass **7/8**. The ordinary
  `hypot(3, 4)` import call still traps `unreachable`, with its expected `5`
  unchanged. An explicit existing Float return contract does not fix the trap.
  After two attempts this additional runtime issue is stopped per the brief.
  Wrong host replies, exact stop positions, and reply validation all pass.
- Browser engine tests were not run. No test or assertion has been removed or
  weakened. The two additional JavaScript runtime failures above remain open.

Fixture decisions:

- Pure R2 task probes constrain their otherwise open error type through
  `NoFail(N) = N - Fail(Never)`. The helper calls the original body unchanged;
  it does not catch errors or remove other service needs or assertions.
  Spec §4.4: "`A - B` takes out of `A` every member whose type constructor is
  the type constructor of a member of `B`." Spec §9.2: "An entry that every
  value of its type constructor satisfies, as `Fail(Never)`, is removed."
- The compiled fixture writes the existing Float result contract of `distance`
  explicitly. Stop-position assertions now name the actual call tokens in the
  converted source: `clock.later` at 45:2 and `hypot` at 41:44. Spec §13 says:
  "Lines and columns count from 1; a column counts Unicode scalar values. The
  position of an expression is the position of its first token." The tests
  still require the complete exact stop message, including its position.
