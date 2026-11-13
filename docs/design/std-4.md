# std: regex, CSV, and crypto

No new state mechanism, language surface, compiler change, or host function.
All three modules are ordinary immutable i. Random bytes use the existing
`i/process.Entropy` service. Branch: `lane/lib`, based on `34e4368d`.

## Decisions and controlling sources

The last Coordinator rulings in `.rd/cache/ledger.md` define this wave.
The later USER immutability decision controls all storage: parser cursors,
automaton programs, VM threads, hash schedules, and stream tails are immutable
values passed to the next call. There is no reference or hidden mutable cache.
`docs/spec.md` §§2, 4, 6, 8, 9 and `std-1.md`/`std-2.md` control exports,
scalar offsets, error values, and stream environments.

Interpretations where the task left a choice:

- Unicode scalar semantics use Rust regex's Unicode 16 `\d`, `\w`, `\s`,
  word boundaries, and simple case folding, rather than RE2's ASCII Perl
  classes. Source: Rust regex syntax and the task's Rust leftmost-first rule.
- Positions in regex errors/matches count zero-based scalars. CSV errors count
  one-based scalars, including when input comes in chunks. This follows
  `std/string`'s indexing convention; Go CSV's byte columns are not adopted.
- Counted repetitions have RE2's 1000-count boundary. Pattern syntax outside
  the requested subset is a compile error, with a reason and position.
- CSV skips blank physical lines and normalizes CRLF within quoted fields as
  Go CSV does. Spaces stay data; bare quotes and text after a closing quote
  are errors. Empty single-field output must be quoted to survive blank-line
  skipping. LF is the default output ending; CRLF is configurable.
- Headers must be unique. Header values decode as JSON strings through the
  existing schema combinators; there is no implicit number coercion or
  special schema/CSV hook. Source: `std/schema`'s two-way JSON design.
- RFC 6070 specifies HMAC-SHA-1, whereas this task specifies PBKDF2-HMAC-SHA-256.
  Its password/salt/count inputs are reused with independently calculated
  SHA-256 expected outputs. SHA-1 is not added.
- The existing `std/bit_array.base16_encode`/`base16_decode` supply hex.
  No duplicate public hex API is added.

## Implementation sources read

Reference checkout: `/home/agent/Programming/i/.rd/ref/INDEX.md`.
Algorithms were reimplemented in i; upstream implementation code is not copied.
SHA-2 round constants and IVs are the standard's mathematical constants.

| Source | Applied approach |
|---|---|
| `rust-regex/regex-automata/src/nfa/thompson/compiler.rs`, particularly `c_at_least` | Thompson instructions; ordered alternatives; nullable `x*` is `(x+)?` to preserve priority. |
| `rust-regex/regex-automata/src/nfa/thompson/pikevm.rs`, `search_imp`, `nexts`, epsilon closure | Per-position visited states, ordered threads, persistent capture slots, discard lower-priority paths after a candidate match. |
| `rust-regex/regex-automata/src/hybrid/dfa.rs` | Read lazy DFA tradeoffs. A PikeVM suffices for all requested captures and avoids a mutable cache. |
| `re2/doc/syntax.txt` | Group/repetition syntax, 1000-count restriction, explicit exclusion of lookaround and backreferences. |
| `rust-regex/regex-syntax/src/unicode_tables/{perl_decimal,perl_space,perl_word,case_folding_simple}.rs` | Unicode 16 property data and all simple fold pairs, packed into immutable tables. The data license is `std/UNICODE-LICENSE`. |
| `rust-regex/testdata/empty.toml` | Reference expectations for empty alternatives, nullable repeats, and non-overlapping iteration. |
| `go/src/encoding/csv/{reader,writer}.go` | Strict quote state machine, newline normalization, configurable rune delimiter, inferred/fixed/variable field counts. |
| [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180.txt) | Quoted fields, doubled quotes, embedded line breaks, record formatting. |
| `go/src/crypto/internal/fips140/{sha256,sha512}/*block.go`, `sha256/sha256.go` | Word schedule, compression, big-endian padding/serialization. |
| [RustCrypto SHA-256 compact compression](https://github.com/RustCrypto/hashes/blob/master/sha2/src/sha256/soft/compact.rs) | Retain only the preceding schedule window needed for recurrence; explicit modular words. |
| [RustCrypto HMAC](https://github.com/RustCrypto/MACs/blob/master/hmac/src/simple.rs) | Key normalization and inner/outer pads. Immutable prepared pads are reused within KDFs. |
| [RustCrypto HKDF](https://github.com/RustCrypto/KDFs/blob/master/hkdf/src/lib.rs), [RFC 5869](https://www.rfc-editor.org/rfc/rfc5869.txt) | Extract/expand separation, chained blocks, length bounds. |
| [RFC 4231](https://www.rfc-editor.org/rfc/rfc4231.txt), [RFC 6070](https://www.rfc-editor.org/rfc/rfc6070.txt) | HMAC vectors including long keys/messages; PBKDF2 inputs as qualified above. |

## regex API

`Regex` and `Captures` hide their constructors. `Match(start, end, text)` and
`RegexError(position, reason)` are exported data. End offsets are exclusive.

| Function | Result/contract |
|---|---|
| `compile(pattern : String)` | `Result(Regex, RegexError)`; literals, dot, classes/ranges/negation, Unicode Perl classes and negations, boundaries, greedy/lazy `* + ? {m} {m,n} {m,}`, alternatives, captures/named/noncapturing groups, anchors, `i m s` flags. Scoped/unset flags, scalar hex escapes, `\A` and `\z` also work. |
| `is_match(regex, input)` | `Bool`, including empty matches. |
| `find(regex, input)` | `Option(Match)`, leftmost first, then branch/greediness priority. |
| `find_all(regex, input)` | Non-overlapping `List(Match)`; empty matches advance a scalar, and an empty match immediately following a nonempty one is suppressed. |
| `captures(regex, input)` | `Option(Captures)` for the first match. |
| `group(captures, index)` | `Option(Match)`; group zero is the whole match; unmatched/out-of-range is `None`. |
| `named(captures, name)` | `Option(Match)` by name. |
| `replace(regex, input, with)` | First replacement. `$1`, `${name}`, `${1}`, `$0`, `$$`; absent groups expand to empty. An unbraced name consumes all following ASCII name characters. |
| `replace_all(regex, input, with)` | All non-overlapping replacements. |
| `split(regex, input)` | Pieces with separators removed, preserving empty pieces. |

The compiler lowers an AST into a persistent instruction dictionary. A VM
closure visits a state once at each input position, including epsilon states
before following their successors, so nullable loops terminate. Capture slots
are immutable dictionaries shared between threads until a save. No recursive
backtracking is used. A search takes O(m*n) state work for a fixed compiled
program of m states and n input scalars; dictionary/capture costs depend on m
and the fixed number of groups, not the input length. Input decodes once and
capture extraction uses UTF-8 byte slices at tracked scalar boundaries.

As with Rust regex's iterators, repeated leftmost-first searches can take
O(m*n²) for an adversarial greedy expression that scans ahead and subsequently
falls back to a short match. This applies to `find_all`, replacement-all, and
split, not a single `find`/`captures`/`is_match`. There is no lazy DFA cache.

## CSV API

`Options` is opaque; `CsvError(line, column, reason)` is public.

| Function | Result/contract |
|---|---|
| `defaults()` | Comma, infer field count, LF. |
| `options(delimiter : String, fields : Int, crlf : Bool)` | `Result(Options, CsvError)`. Delimiter must be one scalar, excluding NUL, quote, CR, LF, U+FFFD. Fields: 0 infer, positive fixed, negative variable. |
| `parse(input)` | `Result(List(List(String)), CsvError)`, default options. |
| `parse_with(input, config)` | The same with explicit options. |
| `parse_header(input, config)` | `Result(List(dict.Dict(String,String)), CsvError)` using the first row as column names. |
| `print(rows)` / `print_with(rows, config)` | `String`, quote only for delimiter/quote/line break or a single empty field. One ending after each row. |
| `rows(source: stream.Stream(String,E,N), config)` | `stream.Stream(Result(List(String),CsvError),E,N)`. Arbitrary chunk boundaries, including between CR and LF or doubled quotes. |
| `records(source, config)` | Streaming header mode, yielding named dictionaries. |
| `decode(codec: schema.Schema(A,E), row)` | `Result(A,E)` through a JSON object of string fields. |

Streaming keeps its cursor, partial field, and partial row in immutable call
arguments. One pull stops at a row boundary. Source failures remain in `E`;
CSV failures are emitted once as an error value, followed by end of stream.
Replaying a pull repeats its input operations, following `std/stream`.
The source owner must keep its resource scope around consumption.
Parsing/printing use O(input/output scalars) work. A pull retains the current
chunk plus row; collecting all rows naturally retains the complete result.

## Crypto API

Every digest/key/output is a `BitArray`. `CryptoError` has `InvalidLength`,
`InvalidIterations`, and `UnalignedInput`. Inputs to HMAC/KDFs must be bytes.
SHA padding preserves the exact bit length, including non-byte inputs.

| Function | Result/contract |
|---|---|
| `sha256(input)` / `sha512(input)` | 256/512-bit digest. |
| `hmac_sha256(key, input)` / `hmac_sha512(key, input)` | `Result(BitArray,CryptoError)`; keys of any byte length. |
| `hkdf_extract_sha256(salt, input)` / SHA-512 equivalent | HMAC extract. Empty salt equals a zero salt. |
| `hkdf_expand_sha256(prk, info, length)` / SHA-512 equivalent | At least hash-length PRK; 0..255*hash-length output bytes. |
| `hkdf_sha256(input, salt, info, length)` / SHA-512 equivalent | Extract then expand. |
| `pbkdf2_sha256(password, salt, iterations, length)` | Positive iterations, 0..(2^32−1)*32 output bytes. |
| `constant_time_equal(a, b)` | `Bool`; length is public, equal-length inputs visit every byte/remaining bit, with no content-dependent early return. |
| `random_bytes(length)` | `Result(BitArray,CryptoError) ^ process.ProcessError ! process.Entropy \| request.Await \| request.Owner`; negative length is a value error, service failures propagate. |

SHA-2 uses masked exact `Int` operations, immutable eight-word states, and a
reverse schedule list whose lookups are bounded by 16 positions. Each block
starts a new schedule. The extra cells retained per block are at most 80.
Padding copies the input once. HMAC normalizes oversized keys and prepares
immutable pads; HKDF/PBKDF2 reuse these within each derivation.

Constant-time equality is a source-level control-flow property. The current
Wasm/host runtime and optimizer do not provide a machine timing guarantee.
The tests check every length/position case, including partial bits; they do
not claim to establish a side-channel property by measuring elapsed time.

## Measurements

Reproduce with `target/release/i run --project std/bench/std4 --opt 2` after
building this lane's CLI. The benchmark prepares input before timing and uses
`i/clock.monotonic_ns`; assertions force the computed result to be observed.
Decimal MB/s = bytes × 1000 / nanoseconds. Single runs on the shared machine
are observations, not stable performance promises.

O2 run after the nullable-loop refinement:

| Operation | Bytes | Nanoseconds | MB/s |
|---|---:|---:|---:|
| SHA-256 million `a` | 1,000,000 | 7,726,543,126 | 0.129 |
| Regex `^a+b$` | 1,000,001 | 7,801,951,261 | 0.128 |
| Regex `(a*)*b`, no match | 5,000 | 92,212,746 | 0.054 |
| CSV parse 10,000 rows, quoted fields/Unicode | 280,000 | 457,057,669 | 0.613 |
| CSV print same rows | 280,000 | 29,847,738 | 9.381 |

The hash implementation is intentionally plain scalar i. No native word,
SIMD, hardware hash, or host crypto operation is introduced.

## Compiler observations and verification

A table represented as a 3,000-element unannotated list of numeric pairs
caused type-check time/memory growth. The first module check was interrupted
at 56 seconds and about 4.99 GB RSS. The plain library workaround stores
Unicode tables as packed hexadecimal strings and decodes them through the
existing byte codec. Property lookup binary-searches packed 24-bit endpoints;
fold pairs are scanned once when compiling an insensitive class. A flat
12,000-segment BitArray experiment was also abandoned during full test build;
that run did not isolate a compiler defect from ordinary whole-std build cost.
No compiler files were edited.

Minimal structural reproduction (write only inside a scratch project):

```python
from pathlib import Path
p = Path("target/std4/probe")
(p / "src").mkdir(parents=True, exist_ok=True)
(p / "i.toml").write_text('name = "std4_probe"\n')
(p / "src/main.i").write_text(
    '(main)\nvalues = [' + ','.join(f'({n},[{n+1}])' for n in range(3000))
    + ']\nmain = () { assert(values != [])\n Nil }\n')
```

Then `target/release/i check --project target/std4/probe`. The isolated probe
was stopped at 30.46 s with peak RSS 2,964,152 KiB, without completing the
check. This confirms the issue independently of the std test runner.

Validation results are recorded below after the final acceptance run.

Final acceptance run on 45c4972a: release CLI build passed; module counts
were regex 130, CSV 107, crypto 88; std O0 and O2 each reported 2170 passed,
0 failed. `scripts/dev-verify rust` failed in `ilang-package` test
`standard_library_modules`: its exact expected module list omits the three
new modules. The smallest reproduction is
`cargo test -p ilang-package --test load standard_library_modules`; actual
paths include `std/crypto`, `std/csv`, and `std/regex`, while the expected
list in `crates/ilang-package/tests/load.rs : 201` does not. Updating that
assertion conflicts with the task's acceptance rule forbidding any `crates/`
diff, so the assertion is unchanged. The full acceptance gate cannot pass
under both requirements until the coordinator updates that upstream test.

Independent remaining checks: `scripts/old-syntax-check` passed (4,634
extracted literals/blocks). `scripts/dev-verify i` failed its review corpus:
1,973 cases, two unclassified checker timeouts, and 20 listed comparison
failures. Examples: `review-k4/perf/wide_generic_1024: unclassified checker
timeout` and `review-k4s/std: unclassified checker timeout`; complete output
is `/tmp/std4-dev-verify-i.log`. The run had concurrent heavy jobs; the cause
of the differences was not established. WASI smoke and the preceding std
runs passed. The gate therefore remains unverified, independently of the
module-list assertion above.
