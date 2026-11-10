# Design: time, calendar, command arguments, and logging

Status: accepted API and implementation contract. The pure implementation and
its verification are recorded in §12. Generic sink clients retain the §7 gate.
Source baseline: `b9173ce013405fd3a055ffa34edf3b8829672104`. Repository line
references below mean that commit, unless another revision is named. The spec
in this checkout already has the syntax migration; all examples use that syntax.
Signature inventories describe bindings with bodies, not bodyless declarations.
`Option`, `Some`, `None` come from `std/option`; `Order` from `std/order`.
`Result`, `Ok`, `Error`, `List`, and `Nil` are built-in names.

## 1. Problem, evidence, and boundaries

A standalone program needs to calculate times, name civil dates, decode its
arguments, and report events. Tests need to do the same without host clocks,
process state, or terminal output. These four modules supply small values and
functions; only the adapters at the edges request services.

Controlling sources, in order:

| Source | Decision it controls |
|---|---|
| `docs/spec.md:127–145`, `:221–230` | Ordinary package modules, acyclic imports, opaque constructors and fields. No new compiler-recognized std path. |
| `docs/spec.md:348–463`, `:854–909` | Types are sets; rank-one principal inference and value restriction; service arguments compare equally; errors and services are distinct. |
| `docs/spec.md:591–598`, `:1124–1155` | Strict calls, arbitrary-size exact integers, truncating division, structural value equality, tail calls. |
| `docs/design/syntax.md:99–253`, `:372–386` | Import/export groups, declared type variables, ordinary clause functions. |
| `docs/design/runtime.md:974–1060`, `:1186–1274` | Clock nanoseconds and origin; arguments exclude program name; explicit stderr output and flush; IO errors and Await/Owner requirements. These are approved future APIs, not available modules. |
| `docs/design/std-1.md:1–20`, §2, §3.1–3.4, §8–10 | Exact contracts, pure `Result` validation, invariant stored service rows, compiler probes and honest implementation gates. |
| Plan `/home/agent/.claude/plans/twinkling-swinging-perlis.md:30–58`, `:79`, `:510–549` | No keywords or library exceptions; log uses a sink service; pure std first, time.now after IO. |
| Ledger `/home/agent/Programming/i/.rd/cache/ledger.md:94–100`, `:128–147` | K1 repaired invariant factory generalization; Std-1 landed; native representation accepted; nominal field privacy landed. Keep inference intact; later IO/provider integration must follow the accepted target rulings. |

Code read, and its consequences:

| Code region | Checked fact |
|---|---|
| `lib/i/clock.i:1`, `lib/i/process.i:1`, `lib/i/stdio.i:1`; `crates/ilang-package/src/library.rs:7–24` | The three IO modules contain reservation comments and export no API. They are embedded by the compiler. Do not add imports of their future names in phase one. |
| `std/src/option.i:1–6`, `:28–31`; `std/src/result.i:1–5`; `lib/i/fail.i:10–33` | Absence uses Option, fallible pure operations use Result, and fail.from_result/catch bridge to error effects. Follow this style. |
| `std/src/int.i:60–110`, `:355–411` | Integer parsing has an explicit grammar. `/` and remainder helpers differ from floor division/modulo. Negative epoch conversion needs floor division. |
| `std/src/string.i:70–94`, `:299–308`; `lib/i/core.i:91–117` | Codepoint traversal and fragment concatenation exist. Repeated pop_codepoint reconstructs the remainder; scan one codepoint list or byte array instead. |
| `std/src/order.i:21–27`; `std/src/stream.i:43–73`, `:93–109` | Order is Less/Equal/Greater; a stored service row uses an ordinary negative marker to keep it invariant. Reuse that technique for sinks. |
| `std/src/dict.i:24–60`; `std/src/list.i:493–512` | Nominal storage and left-to-right Result folds are ordinary source; they suffice for descriptor metadata and validation; no typed heterogeneous runtime dictionary is needed. |
| `crates/ilang-check/src/types/infer.rs:231–247`, `:2127–2156`, `:2256–2285`, `:2340–2348` | Recursive groups start monomorphic; generalization and variance traverse ordinary types, including function effects. Do not propose polymorphic recursive parser trees. |
| `crates/ilang-check/src/types/infer.rs:1606–1624` | Field selection now checks hidden constructors. Calendar validation cannot be bypassed through constructor update or a public field. |
| `crates/ilang-check/src/types/solve.rs:688–714`, `:801–870`, `:950–999` | Environment unions, argument variance, and occurs checking are general solver operations. The logging probe in §7 exposes a failure here; this note does not diagnose its exact cause. |

These are reading results. Algorithms and costs below are design deductions;
§10 separately records executed checks. The four production modules do not yet
exist at this baseline. No production behavior or performance is claimed.

## 2. Alternatives and choice

Criteria: source agreement; an ordinary set-of-values interpretation; principal
inference without new features; testable effects; small, complete contracts;
representable closures and recursion; no hidden host dependency.

| Design | Time/calendar | CLI | Log | Assessment |
|---|---|---|---|---|
| A: host-shaped APIs | Host dates, zones and mutable timers | Reflection into a record, process globals | Global mutable logger | Familiar, but violates explicit services and ordinary source; requires host work even for tests. |
| B: untyped minimum | Integer seconds and date tuples | String dictionary | Print a string | Easy to build; mixes instants and lengths, loses CLI result types and substitutable sinks. |
| C: small nominal values and typed closures | Exact nanoseconds; validated Gregorian dates and offsets | Metadata plus a typed decoder, composed with map2 | Entry values plus a sink service | Choose C. All storage and calls are ordinary i; one current compiler limitation is made explicit. |

Keep A’s conversion/validation edge cases and B’s small surface. Do not port
foreign APIs wholesale. In particular there are no macros, reflection, mutable
registries, implicit type classes, dependent types, new operators, or new
language rules. A Date denotes values made by its private constructor; validity
is maintained by library functions, not a refinement the compiler must infer.
Parser(A) denotes values containing functions returning A. A sink denotes
functions with their stated error and environment types.

1. **Separate values from observations.** Time arithmetic and all calendar/CLI
   functions are pure; now/sleep/process reading/stderr are IO adapters. Reason:
   spec §9.1–9.3 and runtime §7. No clock is consulted by formatting or logging.
2. **Use exact signed nanoseconds and separate nominal types.** Reason: spec
   §11.2 and runtime §7.1. A duration may be negative; an instant is an epoch
   position. There is no implicit cast between them or Float conversion.
3. **Use a proleptic Gregorian calendar, UTC and explicit fixed offsets.** This
   is the requested civil model. No time zone database, local-zone lookup, DST,
   or historical jurisdiction rules: these require versioned external data and
   ambiguity policy. A later ordinary package can provide both. UTC here uses
   the Unix timeline; leap seconds have no distinct representation.
4. **Use Result for pure validation; expose effects as `? E ! N`.** Reason:
   std option/result style above and spec §9.3. A parser error is stored data;
   callers choose fail.from_result to raise it. No expected bad input asserts.
5. **Compose typed CLI decoders with fixed metadata.** Reason: spec §4.7;
   metadata is available without executing callbacks. No cast, existential
   payload, structural record synthesis, or higher-rank callback is needed.
6. **Log through an explicitly installed service, with its failures visible.**
   Reason: scope/plan decision and spec §9.2. There is no default Sink inserted
   by the compiler and no error swallowing. An application chooses stderr,
   filtering, capture, or silence. Service lookup is at the write call.
7. **Keep service parameters invariant inside stored callbacks.** Reason: spec
   §4.5 rule 4 and Std-1 §3.1. Hiding a constructor alone cannot stop an unsafe
   coercion of its type parameters. Do not weaken this rule to pass a probe.

Each module has three unrelated uses: time supports retry delays, media offsets,
and elapsed work; calendar supports accounting dates, archive timestamps, and
scheduled reports; cli supports a compiler, backup tool, and interactive-game
launcher; log supports server diagnostics, build events, and deterministic test
traces. They all use existing general features; there is no proposed feature
whose usefulness rests only on one library.

## 3. `std/time`

### 3.1 Ideas considered

| Source | What it offers | Take | Leave and why |
|---|---|---|---|
| [Go time](https://pkg.go.dev/time) | Time, Duration, monotonic readings | Different concepts for position and length; exact units | Combined wall/monotonic Time and fixed-width range; representation should stay simple and exact. |
| [Python time](https://docs.python.org/3/library/time.html) | Wall/monotonic clocks and nanosecond functions | Use monotonic observations for measurement | Ambient functions and float timestamps; observations need a replaceable service. |
| [Effect Duration](https://effect.website/docs/v3/data-types/duration) | Duration values, unit conversions | Unit constructors | Infinity and mixed representations; signed finite Int is enough. |
| [Gleam duration](https://gleam-time.hexdocs.pm/gleam/time/duration.html) | Separate duration arithmetic and units | Small value API | Approximate floating conversion; no rounding policy is needed in v1. |
| [Haskell time](https://hackage-content.haskell.org/package/time-1.15/docs/Data-Time.html) | Time positions distinct from differences | Pure arithmetic boundary | Type-class numeric surface; ordinary named functions suffice. |

### 3.2 Types and complete interface

Export Duration and Instant without constructors. These private definitions have
one canonical representation per value:

```i
Duration { Duration(Int) }
Instant { Instant(Int) }
TimeError { DivisionByZero }
```

Export TimeError(..) and exactly these functions. Units are integer counts.

```text
nanoseconds: (count: Int) -> Duration
milliseconds: (count: Int) -> Duration
seconds: (count: Int) -> Duration
minutes: (count: Int) -> Duration
hours: (count: Int) -> Duration
to_nanoseconds: (duration: Duration) -> Int
add: (first: Duration, second: Duration) -> Duration
subtract: (first: Duration, second: Duration) -> Duration
scale: (duration: Duration, by: Int) -> Duration
divide: (duration: Duration, by: Int) -> Result(Duration, TimeError)
compare: (first: Duration, second: Duration) -> Order
from_unix_nanoseconds: (count: Int) -> Instant
unix_nanoseconds: (instant: Instant) -> Int
shift: (instant: Instant, by: Duration) -> Instant
between: (start: Instant, end: Instant) -> Duration
compare_instants: (first: Instant, second: Instant) -> Order
```

Phase two adds exactly these IO wrappers (clock = i/clock, request = i/request):

```text
now: () -> Instant ? clock.ClockError ! clock.Clock | request.Await | request.Owner
sleep: (duration: Duration) -> Nil ? clock.ClockError ! clock.Clock | request.Await | request.Owner
measure(A, E, N): (body: () -> A ? E ! N) -> (A, Duration) ? E | clock.ClockError ! N | clock.Clock | request.Await | request.Owner
```

### 3.3 Rules and laws

- **T1 Units.** The private integer is nanoseconds. The five constructors
  multiply by 1, 10^6, 10^9, 60×10^9, 3600×10^9 respectively. Arbitrary Ints,
  including zero and negative numbers, are accepted. No wrap, clamp or Float.
- **T2 Arithmetic.** add/subtract/scale are integer arithmetic on the payload.
  divide by zero gives Error(DivisionByZero); otherwise it divides toward zero,
  including for a negative divisor. compare uses numeric order.
- **T3 Instants.** Zero is 1970-01-01T00:00:00Z. between(a,b) is b minus a;
  shift adds a duration. unix_nanoseconds is the inverse of
  from_unix_nanoseconds. No operation adds two Instants. Equality follows the
  canonical payload; no custom equality or hashing rule is needed.
- **T4 Observations.** now calls clock.unix_ns exactly once. sleep passes the
  signed payload unchanged to clock.sleep_ns: negative gives its invalid error,
  excessive host range gives range, and zero still awaits. Arithmetic has no
  host range limit. No busy wait, wall-clock deadline or clamping is added.
- **T5 Measurement.** measure reads clock.monotonic_ns, calls body once, then
  reads it again, returning (value, end minus start). The first error exits:
  failure of the first read means no body; body failure means no second read;
  second-read failure replaces the normal return with ClockError. This is not
  a finalizer. Cancellation/unwinding remains runtime behavior. Neither reading
  is a Unix instant. The two reads use the enclosing call’s Clock; a temporary
  service binding inside body does not replace that enclosing service.

Laws: to_nanoseconds(nanoseconds(n)) = n; shift(a,between(a,b)) = b;
between(a,shift(a,d)) = d; add(d,subtract(e,d)) = e. divide is truncating:
scale(divide(d,k),k) need not equal d. Calendar days/months/years are not duration
units: callers can explicitly use hours(24) for a fixed 86,400 seconds.

### 3.4 Tests and costs

| Test | Expected result and rule |
|---|---|
| to_nanoseconds(seconds(-2)) | -2000000000; T1 |
| divide(nanoseconds(-5),2), divide(nanoseconds(5),-2) | Both Ok(nanoseconds(-2)); T2 |
| divide(seconds(1),0) | Error(DivisionByZero); T2 |
| shift(from_unix_nanoseconds(-1),nanoseconds(1)) | Epoch zero; T3 |
| Hours count 10^100; shift by it and back | Exact inverse, no overflow; T1/T3 |
| Pass Instant to add | Compile error; T1/T3 nominal separation |
| Fake unix_ns gives -1 | now returns from_unix_nanoseconds(-1); T4 |
| sleep -1, zero, host-unrepresentable positive | invalid; one zero await; range; T4/runtime §7.1 |
| Fake monotonic readings 12 and 19; body gives "x" | ("x",nanoseconds(7)), trace read/body/read; T5 |
| Each of the three T5 steps fails in turn | No later step runs; exact original error; T5 |

Pure operations use O(1) integer operations and one wrapper at most. Bit cost
depends on integer size. measure adds two requests, sleep one. No scheduling
precision or timing benchmark is promised.

## 4. `std/calendar`

### 4.1 Ideas considered

| Source | What it offers | Take | Leave and why |
|---|---|---|---|
| [Go time](https://pkg.go.dev/time) | Civil fields, UTC/fixed locations, date normalization | Explicit offset conversions | Silent normalization of invalid dates and zone loading; validation should be explicit. |
| [Python datetime](https://docs.python.org/3/library/datetime.html) | Gregorian dates, timedeltas, timezone offsets | Validated civil fields | Naive/aware mixing and bounded positive years; use explicit offsets and all integer years. |
| [Effect DateTime](https://effect.website/docs/v4/api/effect/DateTime) | UTC/zoned values and offset zones | Separate civil interpretation from an instant | Named zones and host date dependencies; no data source or ambiguity policy in v1. |
| [Gleam calendar](https://gleam-time.hexdocs.pm/gleam/time/calendar.html) | Date, time-of-day, weekdays, validation | Small components and weekday functions | Ambient local offset; an offset must be supplied. |
| [Haskell time](https://hackage-content.haskell.org/package/time-1.15/docs/Data-Time.html) | Day, TimeOfDay, LocalTime, fixed TimeZone | Distinct civil and epoch concepts | Leap-second-aware alternatives; the underlying clock is Unix nanoseconds. |

### 4.2 Types and complete interface

Date, TimeOfDay, Offset hide constructors. Civil, CalendarError and Weekday
export constructors. `time` refers to std/time.

```i
Date { Date(Int, Int, Int) }
TimeOfDay { TimeOfDay(Int, Int, Int, Int) }
Offset { Offset(Int) }
Civil { Civil(date: Date, time: TimeOfDay) }
Weekday { Monday Tuesday Wednesday Thursday Friday Saturday Sunday }
CalendarError { BadMonth BadDay BadHour BadMinute BadSecond BadNanosecond BadOffset BadText }
```

```text
date: (year: Int, month: Int, day: Int) -> Result(Date, CalendarError)
time_of_day: (hour: Int, minute: Int, second: Int, nanosecond: Int) -> Result(TimeOfDay, CalendarError)
offset: (minutes: Int) -> Result(Offset, CalendarError)
utc: () -> Offset
date_parts: (date: Date) -> (Int, Int, Int)
time_parts: (time: TimeOfDay) -> (Int, Int, Int, Int)
offset_minutes: (offset: Offset) -> Int
is_leap_year: (year: Int) -> Bool
days_in_month: (year: Int, month: Int) -> Result(Int, CalendarError)
day_number: (date: Date) -> Int
from_day_number: (day: Int) -> Date
weekday: (date: Date) -> Weekday
add_days: (date: Date, days: Int) -> Date
days_between: (start: Date, end: Date) -> Int
at: (instant: time.Instant, offset: Offset) -> Civil
to_instant: (civil: Civil, offset: Offset) -> time.Instant
parse: (text: String) -> Result((Civil, Offset), CalendarError)
format: (civil: Civil, offset: Offset) -> String
```

### 4.3 Rules and conversion algorithm

- **C1 Dates.** Use the Gregorian leap rule for every integer year: divisible by
  4, except multiples of 100 unless also multiples of 400. Astronomical year 0
  is 1 BCE, -1 is 2 BCE. No 1582 discontinuity. Months are 1..12. Month lengths
  are 31,28/29,31,30,31,30,31,31,30,31,30,31. Validate month then day; return
  BadMonth or BadDay. days_in_month validates only the month.
- **C2 Time and offset.** Validate hour 0..23, minute 0..59, second 0..59,
  nanosecond 0..999999999, in that order, returning the named error. Neither
  24:00 nor leap second 60 is accepted. Offset is whole minutes east of UTC,
  from -1439 through 1439 inclusive; otherwise BadOffset. utc() is offset zero.
  Accessors return exactly the stored fields; Civil needs no extra validation.
- **C3 Day numbers.** Day zero is 1970-01-01, a Thursday. All Int day numbers
  are representable. Define floor division fd(a,b) for positive b by q=a/b,
  then subtract 1 when a%b<0. Define mod(a,b)=a-b×fd(a,b). Never use truncating
  division for eras, days or weekday on negative input.
- **C4 Civil conversion.** at adds offset_minutes×60×10^9 to the epoch count,
  splits by floor division into day_number and a nonnegative remainder below
  86400×10^9, then splits that remainder into time fields. to_instant performs
  the inverse and subtracts the offset. These are total for valid components.
  There is no host call and no guessed zone. add_days adds to day_number;
  days_between(a,b)=day_number(b)-day_number(a). Weekday is indexed Monday=0
  through Sunday=6 by mod(day_number+3,7).
- **C5 Text.** parse accepts exactly a year followed by
  `-MM-DDTHH:mm:ss`, optionally a dot and 1..9 decimal digits, then `Z` or
  `+HH:mm` or `-HH:mm`. All letters and digits are ASCII, T/Z uppercase;
  no spaces, trailing data, offset seconds or missing offset. A year in
  0..9999 has exactly four digits; larger positive years have `+` and their
  minimal decimal digits; negative years have `-` and their magnitude padded
  to at least four digits, with no extra leading zeros. Negative zero is
  invalid. Fraction digits are right-padded to nanoseconds. Reject `-00:00`
  because an unknown offset is not a fixed offset
  ([RFC 3339 §4.3](https://www.rfc-editor.org/rfc/rfc3339.html#section-4.3)). Accept `+00:00` as UTC.
  Offset hours 00..23 and minutes 00..59 are checked separately, so `+01:60`
  cannot normalize into a valid offset. Any bad syntax or invalid component
  gives Error(BadText); constructor functions retain their precise errors.
- **C6 Canonical printing.** format emits C5’s year, two-digit other civil
  fields, no fraction for zero nanoseconds and otherwise nine digits with
  trailing zeros removed. Offset zero prints Z; other offsets print their
  sign and two-digit hours/minutes. It is a small ISO-style format, not a
  claim to accept every ISO 8601 or RFC 3339 spelling. No locale templates.

Use the era decomposition from [Howard Hinnant’s date algorithms](https://howardhinnant.github.io/date_algorithms.html),
expressed below as integer mathematics (not new language syntax), with exact
Int arithmetic and floor division. For date (y,m,d), set y1=y-1 if m<=2 else y, era=fd(y1,400),
yoe=y1-era×400, mp=m-3 if m>2 else m+9, doy=fd(153×mp+2,5)+d-1,
doe=365×yoe+fd(yoe,4)-fd(yoe,100)+doy. The result is
`era×146097+doe-719468`. All divisors are positive constants.

For its inverse let z=day+719468, era=fd(z,146097), doe=z-era×146097,
yoe=fd(doe-fd(doe,1460)+fd(doe,36524)-fd(doe,146096),365),
y=yoe+era×400, doy=doe-(365×yoe+fd(yoe,4)-fd(yoe,100)),
mp=fd(5×doy+2,153), d=doy-fd(153×mp+2,5)+1, and
m=mp+3 if mp<10 else mp-9. Return (y+1 if m<=2 else y,m,d).
No iteration proportional to the year is allowed. Implement and check this
arithmetic locally; no Rust date library or host API is required.

Laws: from_day_number(day_number(d))=d; day_number(from_day_number(n))=n;
at(to_instant(c,o),o)=c; to_instant(at(t,o),o)=t;
parse(format(c,o))=Ok((c,o)); add_days(d,days_between(d,e))=e.
Different offsets can describe the same instant with different Civil values;
compare instants when that is the intended equality. Date/Civil equality is
ordinary component equality. No month/year addition is supplied: end-of-month
clipping versus rejection must be a separate explicit policy.

### 4.4 Tests and costs

| Test | Expected result and rule |
|---|---|
| Leap years 1900, 2000, 0, -100, -400 | False, True, True, False, True; C1 |
| date(1900,2,29); date(2000,2,29); date(2024,13,0) | BadDay; valid; BadMonth first; C1 |
| time_of_day(24,0,0,0); time_of_day(0,0,60,0) | BadHour; BadSecond; C2 |
| offset(-1440), offset(1439) | BadOffset; valid; C2 |
| day_number for 1970-01-01, 1969-12-31, 2000-03-01 | 0, -1, 11017; C3 |
| weekday of day -4, -1, 0 | Sunday, Wednesday, Thursday; C4 |
| at(epoch -1 ns, utc()) | 1969-12-31 23:59:59.999999999; C4 |
| at(epoch, offset(60)) | 1970-01-01 01:00:00; C4 |
| Adjacent 1582-10-04 and 1582-10-05 | One day apart; C1/C3 |
| `1970-01-01T00:00:00.1+00:00` | Valid; formats `1970-01-01T00:00:00.1Z`; C5/C6 |
| `...:60Z`, `...-00:00`, `...+01:60`, ten fraction digits | BadText; C2/C5 |
| `0000-02-29T00:00:00Z`, `-0001-01-01T00:00:00Z`, `+10000-01-01T00:00:00Z` | Round trips; C1/C5/C6 |
| Days around every century and 400-year boundary, ±10^100 | Both day-number inverse laws; C3 |
| Hidden Date constructor or constructor update from a client | Compile error; spec §2.4/C1 |

Conversion uses a constant number of integer operations. Parsing/printing is
linear in text length apart from big-integer digit arithmetic. Walking a
codepoint/byte list and concatenating fragments once avoids repeated suffix
copies. Calendar is entirely phase one and depends only on time’s pure part.

## 5. `std/cli`

### 5.1 Ideas considered

| Source | What it offers | Take | Leave and why |
|---|---|---|---|
| [Go flag](https://pkg.go.dev/flag) | Named options, parsing and usage | Declare option metadata once | Mutable destination pointers and global flag sets; typed immutable output is simpler here. |
| [Python argparse](https://docs.python.org/3/library/argparse.html) | Arguments, subcommands, automatic help | Help from the same description | Namespace reflection, implicit exits, abbreviations; these hide types or control flow. |
| [Effect CLI Flag](https://effect.website/docs/v4/api/effect/cli/Flag) | Typed flag descriptions and composition | Typed conversion and composition | A framework of commands, layers and prompts; parsing is a pure function. |
| [Gleam Glint](https://glint.hexdocs.pm/) | Flags, named arguments and help | Small builders and explicit metadata | Run-and-handle wrappers; caller owns output and exit policy. |
| [Haskell optparse-applicative](https://hackage-content.haskell.org/package/optparse-applicative-0.19.0.0/docs/Options-Applicative.html) | Predetermined parser structure and applicative composition | Fixed metadata plus map/map2 | Higher-kinded classes and dynamic bind; ordinary functions suffice. |

### 5.2 Types and complete interface

Export Parser without its constructor. Export Names, DefinitionError and
CliError with constructors. Import Option under module alias `opt`, since
`option` below is a function. A conversion callback has no effects: its expected
failure is a Result containing a message. That keeps parsing deterministic
provided the callbacks terminate and obey their own contracts.

```i
Names { Names(long: String, short: Option(String)) }
DefinitionError { InvalidName(String) DuplicateName(String) MisplacedRest }
CliError {
  BadDefinition(DefinitionError)
  UnknownOption(index: Int, token: String)
  MissingValue(index: Int, name: String)
  UnexpectedValue(index: Int, name: String)
  RepeatedOption(index: Int, name: String)
  MissingArgument(name: String)
  InvalidArgument(index: Int, name: String, text: String, reason: String)
  ExtraArgument(index: Int, token: String)
}
```

```text
pure(A): (value: A) -> Parser(A)
flag: (names: Names, help: String) -> Parser(Bool)
option(A): (names: Names, help: String, read: (String) -> Result(A, String)) -> Parser(A)
optional(A): (names: Names, help: String, read: (String) -> Result(A, String)) -> Parser(Option(A))
many(A): (names: Names, help: String, read: (String) -> Result(A, String)) -> Parser(List(A))
argument(A): (name: String, help: String, read: (String) -> Result(A, String)) -> Parser(A)
rest(A): (name: String, help: String, read: (String) -> Result(A, String)) -> Parser(List(A))
map(A, B): (parser: Parser(A), with: (A) -> B) -> Parser(B)
map2(A, B, C): (first: Parser(A), second: Parser(B), with: (A, B) -> C) -> Parser(C)
parse(A): (parser: Parser(A), args: List(String)) -> Result(A, CliError)
help(A): (parser: Parser(A), program: String) -> Result(String, DefinitionError)
text: (text: String) -> Result(String, String)
integer: (text: String) -> Result(Int, String)
```

Phase two adds the following convenience; `process` is i/process. It neither
prints nor exits. Process args already exclude the program name.

```text
read(A): (parser: Parser(A)) -> Result(A, CliError) ? process.ProcessError ! process.Process | request.Await | request.Owner
```

A complete typed description, using only nominal constructors and functions:

```i
(cli = std/cli, (None) = std/option)

Options { Options(jobs: Int, verbose: Bool) }

options = () {
  cli.map2(
    cli.option(cli.Names("jobs", None), "Worker count", cli.integer)
    cli.flag(cli.Names("verbose", None), "Show progress")
    Options
  )
}

example = () { cli.parse(options(), ["--verbose", "--jobs=4"]) }
// Ok(Options(4, True))
```

### 5.3 Exact grammar and rules

All errors for a recognized option use its canonical `--long` name, including
when a short alias was entered. Scanning errors identify that option token's
index; conversion errors retain A6's value-index rule. Unknown-option errors
preserve the original token instead of inventing a canonical name.

- **A1 Definitions.** Metadata order is the left-to-right leaf order of map2;
  map preserves it; pure contributes none. Long and positional names match
  ASCII `[a-z][a-z0-9-]*`; a short name is exactly one ASCII letter of either
  case. Every long/short name is unique in its own option namespace, including
  aliases of the same leaf. Positional display names are unique among
  positionals. At most one rest is allowed, and no argument or rest may follow
  it in positional order. Options after a rest in metadata are permitted.
  Validate the entire definition before looking at args or calling a decoder.
  Scan leaves left to right, checking long then short spelling, then duplicate
  names, then positional order. Return the first issue. InvalidName carries
  the offending raw string; DuplicateName carries `--long`, `-s`, or the raw
  positional name. MisplacedRest covers both a second rest and argument after
  rest. parse wraps this as BadDefinition; help returns it directly.
- **A2 Tokens.** args is already a list of arguments, not shell text. Preserve
  whitespace, Unicode, empty strings and NUL in token values. No expansion,
  splitting, response files or environment lookup. Indexes are zero-based
  positions in this list, never bytes. Before the first bare `--`, recognize
  exact `--long`, `--long=value`, and exact `-s`. No abbreviation, bundled
  `-abc`, short attached value `-n3`, short equals `-n=3`, or implicit `--no-x`.
  A lone `-` is positional. Any other token beginning with `-` is an unknown
  option; a negative positional therefore needs the `--` separator. Option
  recognition continues after positional tokens. `--` itself is discarded;
  all following tokens are positional, including another `--`.
- **A3 Option arity.** flag takes no value and defaults False; one occurrence
  yields True. `--flag=value` is UnexpectedValue. Value options take their
  `=value` suffix (including empty) or the next token unless that token is
  exactly `--` or absent, which gives MissingValue. A next token beginning with
  `-`, even another declared option, is the value: `--name --verbose` assigns
  the string `--verbose` to name. To pass the separator as a value write
  `--name=--`. This arity rule is independent of the read callback.
- **A4 Repetition.** option and optional allow at most one occurrence; flag
  also rejects repetition, even through its alias. many accepts zero or more
  occurrences and keeps input order. A second singleton occurrence gives
  RepeatedOption at that option token. Its canonical name is always `--long`,
  even if entered with a short alias. On a token with both repetition and bad
  arity, repetition wins. Unknown-option errors keep the entire original token.
- **A5 Allocation.** After token scanning, assign the remaining positionals
  to argument leaves in order, then all remaining ones to rest, if any. Every
  argument is required. option is required; optional absent gives None; many
  and rest absent give []; present optional gives Some. No implicit default:
  map optional with a caller-supplied default. Leftover positionals with no rest
  give ExtraArgument at the first leftover token.
- **A6 Error order and conversion.** First definition validation (A1), then
  token scanning errors (A2–A4, input order), then extra positionals (A5), then
  missing/value conversion errors in descriptor order. For each leaf, check
  its missing value and run its read callbacks in occurrence order. On Error(s)
  return InvalidArgument(index, canonical name, original value, s), and call no
  later callback. For a separated option value, index is the value token index;
  for `--name=value`, it is the option token index. Required missing options
  use MissingArgument("--name"); missing positionals use their raw name.
  pure returns its stored value; map invokes with once after successful inner
  decoding; map2 decodes first, then second, then calls with once. No callback
  runs at construction or during help. Pure callbacks may still stop or diverge
  under spec §11.7; the library cannot turn those into parse errors.
- **A7 Converters.** text(s)=Ok(s). integer accepts an optional ASCII + or -,
  then one or more ASCII decimal digits, with no whitespace, separators or
  trailing text, as std/int.parse does. Failure is Error("expected integer").
  Range checks belong in a user callback, so port numbers, counts and negative
  quantities need no additional library rule.
- **A8 Help.** help validates metadata, then emits `Usage: ` plus program plus
  `\n`, followed by one line per leaf in descriptor order. A line is two spaces,
  the display name, a tab, help text, and LF. A flag display is `--long` plus
  `, -s` when present; required option adds ` VALUE`; optional encloses that
  entire display in `[` and `]`; many appends `...` inside those brackets.
  argument displays `<name>`, rest displays `[<name>...]`. Flag displays are
  also enclosed in brackets. pure adds no line. Program/help text are copied
  verbatim; no terminal-width detection, wrapping, escaping or color is implied.
  Returned text always ends with LF. No flag, including help/version, is
  reserved or triggers output. A program may handle the exact args list
  `["--help"]` before parse, or declare its own help option and policy.

Law: map(p,id) and p have the same parse observations and help; map2 preserves
metadata and callback order under regrouping with the corresponding tuple
reassociation. These are observational laws, not `==` laws for closures.
Repeated parse with the same arguments gives the same result for deterministic
callbacks. No general alternative/backtracking combinator is promised.

### 5.4 Representation and implementer instructions

This is a representable choice, not a new public type facility:

```i
Token { Token(index: Int, text: String) }
Kind { Flag Required Optional Repeated Positional Remaining }
Field { Field(name: String, short: Option(String), help: String, kind: Kind) }
Parser(A) {
  Parser(List(Field), (List(List(Token))) -> Result((A, List(List(Token))), CliError))
}
```

The scanner builds one bucket of Token values per Field, in metadata order.
Flag buckets have zero/one occurrence; other buckets contain their value tokens.
The decoder consumes these buckets in the same order and returns unused buckets.
Each leaf consumes exactly one bucket; pure consumes none. map keeps the
remainder; map2 passes the first decoder’s remainder to the second. A leaf
without a bucket is an internal invariant failure, unreachable through exported
constructors, not a user-input error. The public parse helper must supply all
buckets, including empty ones, and check full consumption in internal tests.

This avoids an AST constructor with an existential child type: map2 closes over
Parser(A) and Parser(B) in a function returning C. No recursive function calls
itself at different type arguments. Parser is covariant in A and stores no
service row. Its hidden decoder cannot be projected by clients. The descriptor
is plain immutable metadata plus functions, not necessarily serializable.

A simple first implementation may use linear name lookup and bucket updates:
O(t×f+b) structural work for t tokens, f fields, b total token codepoints, plus
converters. Metadata composition by list append is O(f_left) per map2 and may
be O(f²) for a long left-associated builder. Decoding is O(t+f) plus callbacks;
closure nesting uses O(builder depth) stack. These honest small-CLI costs are
acceptable; no quadratic text copying is acceptable. A later index can improve
lookup without changing the API. Use reverse accumulators for repeated values.
Subcommands, mutually exclusive groups, completion and prompts are out of v1;
programs can dispatch a leading command token to separately typed parsers.

### 5.5 Test obligations

| Test | Expected result and rule |
|---|---|
| The Options example above | Ok(Options(4,True)); A2/A3/A6/A7 |
| Duplicate `jobs` leaves with different read functions; args=[] | BadDefinition(DuplicateName("--jobs")), neither reader called; A1 |
| Short name "ab"; argument after rest | InvalidName("ab"); MisplacedRest; A1 |
| `--jobs=-2`; `--jobs -2`; `--jobs=` | -2; -2; InvalidArgument with empty text; A2/A3/A7 |
| `--jobs --verbose` using integer | InvalidArgument(1,"--jobs","--verbose","expected integer"); A3/A6 |
| `--jobs --` | MissingValue(0,"--jobs"); A3 |
| `--verbose=false`; repeated flag with alias | UnexpectedValue; RepeatedOption; A3/A4 |
| `--verbose=false` after an earlier verbose | RepeatedOption first; A4 |
| many name with `--name=a --name= --name=é` | ["a","","é"], input order; A4/A7 |
| rest(text), args=["--","-2","--",""] | ["-2","--",""]; A2/A5 |
| Unknown `--jo`, bundled `-vj`, lone `-` | UnknownOption, UnknownOption, positional; A2 |
| Missing required option plus unknown token | UnknownOption first; A6 |
| Two failing read callbacks in reverse token order | Error of first descriptor, not first token; A6 |
| map2(pure(2),pure("x"),Pair) with a suitable nominal Pair | Typed result with both field types preserved; A6 |
| help for one optional `jobs` with short j and text "Workers" | `Usage: app\n  [--jobs, -j VALUE]\tWorkers\n`; A8 |
| Parser with read callback that stops; help called | Help succeeds without calling read; A8 |
| pure factory used separately at Int and String | Both compile; one weak binding follows ordinary value restriction, spec §4.7 |
| read with fake process.args returning ["--jobs=4"] | Same parse result as explicit args; runtime §7.4 |

## 6. `std/log`

### 6.1 Ideas considered

| Source | What it offers | Take | Leave and why |
|---|---|---|---|
| [Go slog](https://pkg.go.dev/log/slog) | Structured entries and handlers with enabled checks | Separate event data from a sink and filtering | Arbitrary dynamic attribute values, global defaults; explicit strings and services suffice. |
| [Python logging](https://docs.python.org/3/library/logging.html) | Levels, filters, handlers | Filtering and replaceable destinations | Global hierarchy and mutable configuration; environment binding already scopes replacement. |
| [Effect logging](https://effect.website/docs/v3/observability/logging) | Replaceable loggers and scoped annotations | Replacement at the call’s environment | Automatic fiber/span metadata; not a logging primitive here. |
| [Gleam logging](https://logging.hexdocs.pm/logging.html) | Log levels and small log operation | Small entry surface | Global level configuration and more severity levels; four levels cover v1. |
| [Haskell monad-logger](https://hackage-content.haskell.org/package/monad-logger-0.3.42/docs/Control-Monad-Logger.html) | Decoupled logging runners, including non-macro calls | Separate producer from output policy | Monad classes and source-location macros; callers supply context explicitly. |

### 6.2 Types and complete interface

Level, Entry export constructors. Sink hides its constructor and fields; make
is the public factory. `time` is std/time. Field pairs are ordered and may repeat.
Failure is the error severity; it avoids collision with built-in Error.

```i
Level { Debug Info Warning Failure }
Entry {
  Entry(level: Level, message: String, fields: List((String, String)), at: Option(time.Instant))
}
Sink(E, N) {
  Sink((Level) -> Bool, (Entry) -> Nil ? E ! N, (N) -> Nil)
}
```

```text
make(E, N): (accept: (Level) -> Bool, send: (Entry) -> Nil ? E ! N) -> Sink(E, N)
silent(N): () -> Sink(Never, N)
filter(E, N): (sink: Sink(E, N), accept: (Level) -> Bool) -> Sink(E, N)
minimum(E, N): (sink: Sink(E, N), level: Level) -> Sink(E, N)
enabled(E, N): (level: Level) -> Bool ! Sink(E, N)
write(E, N): (entry: Entry) -> Nil ? E ! N | Sink(E, N)
lazy(E, F, N): (level: Level, message: () -> String ? F ! N) -> Nil ? E | F ! N | Sink(E, N)
format: (entry: Entry) -> String
```

Phase two adds one factory, referring to i/stdio and i/request. Construction
is pure. This is not a host service declaration and introduces no namespace.

```text
stderr: () -> Sink(stdio.StdioError, stdio.Stdio | request.Await | request.Owner)
```

### 6.3 Rules and laws

- **L1 Data.** Level ordering is Debug < Info < Warning < Failure. Entries have
  explicit text, ordered string field pairs, and optional wall timestamp. All
  strings, including empty keys and control characters, are legal data. The
  timestamp is neither added nor replaced by write. No clock or task metadata
  is consulted. Applications can construct Entry with None without a clock.
- **L2 Delivery.** make stores callbacks without calling them. enabled calls
  the current Sink’s accept exactly once. write obtains the current service,
  calls its accept once and, only on True, calls send exactly once with the
  original Entry. False returns Nil. No retry, buffering, task spawn, flush,
  deduplication, or failure suppression occurs in the core. An error from send
  is the original E. Cancellation and cleanup follow ordinary call rules.
- **L3 Laziness.** lazy checks accept once; False calls neither message nor
  send. True calls message once, then sends Entry(level,text,[],None) once.
  If message raises F, send is not called. It does not call write (which would
  check accept again). Eager write arguments still evaluate before filtering.
- **L4 Filtering.** filter creates a sink whose accept is new_accept(level)
  && old_accept(level), in that order; send is the original callback. minimum
  uses the L1 numeric order and accepts at or above the threshold. silent’s
  accept is always False. All these factories are pure. Calling enabled then
  write performs two checks; enabled gives a query result, not a reservation.
- **L5 Services and errors.** E is covariant; N is invariant through the unused
  `(N)->Nil` marker, made as `(_: N) { Nil }` and never called. Composition uses
  one chosen N, not an invariant union of free row variables. Different Sink
  instances in an environment must have equal arguments, per spec §9.2;
  there is no automatic union of two sinks’ error/environment parameters.
  Choose the desired E and N when constructing a replacement. N must not
  recursively contain this same Sink(E,N); a callback that logs through itself
  is not a route to a finite type or a useful forwarding sink. Capture a
  concrete downstream sink/callback before installing a replacement instead.
- **L6 Text.** format returns one line without its final LF. Start with the
  lowercase level word (`debug`, `info`, `warning`, `failure`), one space and
  a quoted message. For Some(t), append ` @` followed by the signed decimal
  Unix nanosecond count; for None append nothing. Then for each field append
  one space, quoted key, `=`, quoted value, preserving order and duplicates.
  Quote by surrounding with double quotes; escape double quote as `\"`,
  backslash as `\\`, LF/CR/tab as `\n`/`\r`/`\t`. Other U+0000..001F and
  U+007F..009F, plus U+2028/U+2029, become `\u{h}` with minimal lowercase hex
  digits. All other scalar values are copied. This is the module’s own stable
  text format, not core.inspect, JSON, locale formatting or terminal styling.
- **L7 Stderr adapter.** stderr() accepts all levels and sends by
  stdio.print_error(format(entry)). That wrapper appends LF, writes all bytes,
  and flushes stderr (runtime §7.5). It needs Stdio, Await and Owner at delivery,
  captures no running owner, and does not request a clock. IO failures remain
  StdioError, including busy; no atomic multi-writer ordering is promised.
  A cancelled or failed write may have emitted a prefix; callers who need
  serialized logging can supply a queue sink in ordinary i after task exists.

Service laws: an installed fake receives precisely the accepted entries, in
program call order for sequential calls. filter(s,always_true) has s’s delivery
observations; minimum(minimum(s,a),b) has threshold max(a,b). They do not assert
representation equality of closures. Pure formatting is deterministic. No
library guarantee makes a user callback terminate, stay pure beyond its type,
or avoid recursively calling itself.

### 6.4 Tests and costs

| Test | Expected result and rule |
|---|---|
| Write Entry(Info,"hello",[],None) to capture sink | One identical entry, no clock; L1/L2 |
| Replace sink inside a block, call a previously made producer | Replacement sees entry; spec §9.2/L2 |
| minimum at Warning, write Debug then Failure | Only Failure delivered; L1/L4 |
| False accept and lazy message that raises | Nil, message not called; L3 |
| True accept and lazy message that raises "bad" | Original "bad", no send; L3 |
| Send raises custom error | Same error in fail.catch, no retry; L2/L5 |
| Format Entry(Info,"a\nb",[("k","v"),("k","")],None) | `info "a\nb" "k"="v" "k"=""` with escaped LF, no actual LF; L6 |
| Format timestamp -1, ESC, NUL, U+2028 | Signed `@-1`; escaped control values, no injected line; L6 |
| Replace Sink(String,N) with incompatible Sink(Int,N) | Compile error; L5/spec §9.2 |
| Coerce Sink(Never,Consumer(Int)) to Sink(Never,Consumer(Never)) | Compile error with invariant N; L5 |
| stderr with fake Stdio | One error-stream write then flush, none to stdout; L7 |
| Fake stderr write gives busy, or flush gives io | Exact StdioError; no second write; L7 |

Core delivery costs one predicate and at most one callback. Each filter adds
one predicate/closure layer. format uses O(total text length) traversal and
fragments, plus decimal timestamp conversion, and one final concatenate.
An unbounded chain of filters adds stack depth; no fusion is promised. Stderr
adds two IO operations per accepted event. Benchmark serialization/IO in the
coordinator’s quiet window; this note supplies no throughput figures.

## 7. Compiler conformance gate for generic sink calls

The public types in §6 are expressible. The cached new-syntax compiler
`/home/agent/Programming/i/.rd/cache/bin/i-1df2fe0` checks them as type aliases.
It also infers the intended write and lazy types when their bodies omit return
annotations. However, an explicit generic return annotation and an otherwise
ordinary generic client each produce E0505. This is not the earlier K1 empty
factory failure: the silent factory and its specialized clients pass.

Here is a self-contained reproduction, deliberately using a minimal Entry. The
same failure was reproduced with the complete four-field Entry of §6.

```i
(i/fail)

Entry { Entry(message: String) }
Sink(E, N) { Sink((Entry) -> Nil ? E ! N, (N) -> Nil) }

make(E, N) = (send: (Entry) -> Nil ? E ! N) -> Sink(E, N) {
  Sink(send, (_: N) { Nil })
}

write(E, N) = (entry: Entry, sink ! Sink(E, N)) {
  Sink(send, _) = sink
  send(entry)
}
// Shown: (entry: Entry) -> Nil ? E ! N | Sink(E, N)

example = () {
  ! Sink(String, Never) = make((entry) { fail.raise(entry.message) })
  fail.catch(() { write(Entry("seen")) })
}
// Current compiler: E0505 at example; expected Ok/Error value has type
// Result(Nil, String), with Error("seen") at run time.
```

A finite instantiation E=String, N=Never satisfies the intended call; no
recursive type is necessary. The inference that this is a compiler conformance
fault follows spec §4.7, not a new library typing rule. Root cause and a general
fix remain for the compiler owner. Solver environment union and occurs code are
cited in §1. `git diff 1df2fe0..b9173ce -- crates/ilang-check/src/types` shows
only the 19-line field-privacy addition in infer.rs:1606–1624; solve.rs and
display.rs are unchanged. That makes the cached probe relevant to this lane’s
checker, though it is still not a build of this exact baseline.

An explicit closed instantiation works with the inferred implementation:

```i
send: (Entry) -> Nil ? String ! Sink(String, Never) = write

working = () {
  ! Sink(String, Never) = make((entry) { fail.raise(entry.message) })
  fail.catch(() { send(Entry("seen")) })
}
// Error("seen")
```

The minimal reproduction and its closed workaround were also checked directly
from this note’s code blocks. The larger executable probe passed with
N=Consumer(Int), a separate
service used by the sink callback. It passed nested replacement, lazy
suppression, and silent widened to Sink(String,Never). Removing the N marker
is not an acceptable workaround: it would change which sinks can be coerced
under ordinary value subtyping, contrary to spec §4.5 and Std-1 §3.1.

**Gate.** The core values, factories, formatting and closed service clients can
be built now. Do not declare unrestricted generic log usage ready until both
the annotated body and the unannotated client reproduce successfully under a
compiler conformance fix. No library-specific inference rule is permitted.
The implementation can use inferred function bodies and explicitly typed
closed client calls in the meantime. Time, calendar and cli do not depend on
this fix. The eventual positive regression must keep the unsafe-row negative
case rejected:

```i
Consumer(A) { Consumer((A) -> Nil) }

bad = (s: Sink(Never, Consumer(Int))) -> Sink(Never, Consumer(Never)) { s }
// Must reject: stored environments preserve exact service arguments.
```

Also keep unequal Sink error arguments rejected in an environment. Fixing
inference must not turn the public errors/services into Any-like dynamic data.

## 8. Replacement spec text, section by section

The normative language spec explicitly leaves other libraries outside the
language (`docs/spec.md:14`). These modules need no language change. Follow
Std-1’s accepted no-spec-change decision (§10), rather than inserting std path
exceptions into §9. The exact replacement set for this track is empty:

| Section | Replacement text or action |
|---|---|
| Opening scope / §0 | Retain exactly: “Other library modules, the command-line tools, and the way test results are shown are not part of the language.” |
| §1 / §2 | No replacement. All names, imports, exports and package dependencies follow the existing rules. No reserved std paths. |
| §3 / §4 | No replacement. All public signatures use nominal types, ordinary functions and rank-one parameters. The §7 failure needs compiler conformance to §4.7, not weaker text. |
| §6 / §7 | No replacement. Lazy logging uses a zero-argument function. CLI builders are ordinary calls; constructor patterns follow label rules. |
| §9.1–§9.3 | No replacement. Sink is an ordinary user service and pure Result parsing uses existing error bridges. No automatic logging service at entry. |
| §9.7 and future §9.7.1/§9.7.4/§9.7.5 | This track adds no text. IO applies runtime.md §10’s accepted replacement text when its APIs land. In particular keep clock invalid/range errors, program-name exclusion, and stderr flush semantics. |
| §10 / Appendix B | No replacement by Std-2. No @host or @external appears in these modules; only IO wrappers cross the boundary. |
| §11–§13 | No replacement. Arithmetic, equality, text values, tail calls and core operations retain their rules. log.format is an ordinary library function, not §11.8’s value printer. |

Sections 3–6 of this note are the replacement **module documentation contract**:
copy their type/signature inventories, numbered behavior rules, and laws into
the four new module documentation headers. Their future IO paragraphs must not
appear as available exports before the corresponding IO phase. There are no
old std/time/calendar/cli/log sections to replace. Do not renumber the spec or
create tests/spec directories for library behavior; put it in std/test.

The ledger’s target ruling (ledger.md:127) still applies: a target without a
default implementation of a required host service fails compilation, even if
an unsupported error is caught. These modules neither emulate a host service
nor insert a target fallback. The later accepted JS/provider work recorded at
ledger.md:147 belongs to the IO/provider owners. If it changes wrapper service
names from runtime §7, coordinate the five phase-two signatures against the
landed IO API before adding them; do not invent a competing provider scheme.

## 9. Files, phases and hand-off

This design change owns only `docs/design/std-2.md`. The following is a hand-off
for later authorized implementation, not permission to modify other lanes.

| File / region | Work |
|---|---|
| New `std/src/time.i`, entire file | T1–T3 and the 16 pure exports; later append T4/T5 wrappers and imports. |
| New `std/src/calendar.i`, entire file | C1–C6, validated components, exact day conversion, text scan/print. Import only time’s pure operations. |
| New `std/src/cli.i`, entire file | A1–A8, descriptor validation, scanner, buckets, typed decoders, help; later add read. |
| New `std/src/log.i`, entire file | L1–L6 plus §7 gate; later append stderr and IO imports. No @host declaration. |
| New `std/test/{time,calendar,cli,log}_test.i`, entire files | All tables above; laws, error ordering and boundary cases. Keep IO-fake cases in the matching second phase. |
| `crates/ilang-check/src/types/solve.rs:688–714`, `:801–870`, `:950–999`; `infer.rs:2127–2156`; type-checker regression tests | Compiler owner investigates §7. These are starting regions, not a claim that all need edits. Exported generic wrappers and closed client calls need paired regression tests. |
| `lib/i/{clock,process,stdio}.i:1`, request module and registration `crates/ilang-package/src/library.rs:7–24` | IO/runtime owners replace reservations and add the accepted request APIs. Std-2 edits none of these. |
| `std/i.toml:1`; `docs/spec.md` | No manifest or language-spec change required. All new imports are std or accepted i library modules. |

1. Implement pure time first, then calendar. CLI is independent. Log needs
   time’s Instant type but no real clock. Use explicit annotations on public
   inputs and outputs except the documented current generic logging workaround;
   inspect exported inferred types, including every generic factory.
2. For CLI, build the typed bucket decoder before the scanner. Test heterogeneous
   map2 and decoder order. Then add metadata validation, scanner and help in
   that order. Check supplied closures are not called by builders/help.
3. For calendar, test the integer day algorithm against an independent reference
   before adding text. Test negative quotient/remainder boundaries separately.
   Keep Date/TimeOfDay/Offset constructors private; exported selectors return
   plain values. Do not derive unchecked dates from parsing.
4. Add log only with explicit evidence for its current usable subset, or wait
   for the compiler gate if the coordinator wants one complete public release.
   Keep the marker and error types. Never capture Await/Owner at sink creation.
5. After clock/request land, add now/sleep/measure. After process/request land,
   add cli.read. After stdio/request land, add log.stderr. Calendar has no IO
   phase. Real host error/range, cancellation and print/flush checks belong here.
6. For each implemented phase run the prebuilt compiler’s check --types, then
   `i test --project std --opt 0` and `--opt 2`, and the repository’s normal
   formatting/old-syntax checks. Full integration and performance are the
   coordinator’s responsibility. Any cargo command must use the shared heavy
   wrapper; this design work ran no cargo command.

## 10. Self-attack and actual verification

### 10.1 First pass: concrete defects found and repaired

**Soundness: mixing clock domains and service rows.** Candidate APIs reused
Instant for monotonic readings and stored a covariant sink environment. The
counterexamples were `calendar.at(monotonic(), calendar.utc())`, which gives a
civil date from an arbitrary origin, and the bad Consumer coercion in §7.
Repair: expose Unix Instant only, keep monotonic readings inside measure, and
make stored N invariant. The mixed Duration/Instant and unsafe-row programs
were rejected by the compiler; the latter gives E0501 (Int is not a subtype
of Never). Same-named Sink services with String and Int error arguments give
E0515, as required. Types stay ordinary sets of values.

**Cannot be built as proposed: logging inference.** Adding the explicit generic
write annotation yielded E0505; removing it inferred the expected type, but the
ordinary generic client still failed. Repair: document the finite closed
instantiation that runs, and make full generic usage a compiler gate (§7),
following Std-1’s precedent. No new “log effect” rule, captured Owner or erased
error is substituted. This is a remaining implementation prerequisite, not a
passing generic client.

**Cannot be built as an ordinary parser tree.** An AST proposal with map2
children Parser(A) and Parser(B) inside a Parser(C) node needs hidden type
parameters or a cast. The concrete test builds a result Pair(Int,String) from
two leaves. Repair: use metadata plus a typed decoder closure. A real i
prototype of pure/map/map2/one/decode passed that heterogeneous test, first-error
ordering and pure mapping. During this check, `Token(i,s)` was corrected to
`Token(i = index,s = text)`: labelled constructor patterns match by label
(spec.md:730–737). The conversion formula’s mathematical parentheses were
translated to expression braces, as the current syntax requires.

**Source conflicts: wall arithmetic and strict calls.**

```i
before_epoch = time.from_unix_nanoseconds(-1)
negative = time.divide(time.nanoseconds(-5), 2)
```

Truncating day division would put before_epoch in day 0; using floor division
for every duration division would make negative equal -3 ns instead of -2 ns.
Repair: C3 uses floor division only for calendar splitting; T2 retains normal
integer division. A rejected lazy log body does not run; a rejected eager
write’s argument does, by spec §6.2. No library exception can make that eager
argument lazy.

**Calendar input ambiguity.** `1970-01-01T00:00:00-00:00` is not a known zero
offset; `1970-01-01T00:00:00+01:60` must not normalize to +02:00. Repair: reject
unknown negative-zero offset and check offset fields separately (C5). Leap
seconds are rejected explicitly rather than silently mapped to another day.

**CLI precedence and logging injection.**

```i
cli.parse(options(), ["--jobs", "--verbose"])
```

A scanner that asks the integer reader whether the next token is a value has
callback-dependent token grammar. Repair: fixed arity (A3) and explicit A6
phase order. Logging a message containing LF or ESC must not forge another
line or terminal command; L6 escapes controls, including Unicode separators.
Log fields retain duplicates instead of silently discarding evidence in a map.

### 10.2 Second pass: no new serious issue after those repairs

Constructed these additional programs/inputs and followed their rules:

- Soundness: a log thunk made before a replacement sink must deliver to the
  replacement, and a sink needing Consumer(Int) must use the consumer at call
  time. Both ran successfully with closed signatures. Date/Offset accessors
  return Int data without a constructor capable of rewrapping unchecked input.
- Buildability: map2(pure(1),pure("x"),Pair), repeated use of a parser factory,
  and an error in the first bucket. The real closure prototype compiles and
  runs with no polymorphic recursion, record features or casts. The metadata
  scanner is still a future implementation; these checks do not prove it.
- Source conflicts: sleep(-1 ns), sleep(0), huge pure arithmetic, help on a
  parser whose converter would stop, and an entry requiring an uninstalled
  Sink. T4 preserves runtime invalid/await behavior; help runs no callback;
  the entry still requires installation. No compiler-supplied sink is added.
- Calendar: year 0 February 29, -400 February 29, century end, day -1,
  ±10^100 days, and offset ±1439 at an epoch boundary. The i day prototype
  passed its inverse tests; the text/offset rules give defined results.

No new serious design issue was found. The known generic logging gate remains
explicitly open; this pass does not claim to fix or waive it.

### 10.3 Third pass: second clean pass

Attacked the revised boundary with nested filter predicates, a failing lazy
message, a captured producer called in an inner service scope, and a silent
sink widened to the application’s error type. The closed probes pass; L2/L3
ensure one delivery attempt and no accidental second enabled check. No
background task or lifetime-bearing Owner is captured.

For CLI, tried `--name=--`, `--name --`, a repeated flag with an attached value,
interspersed options, an empty positional, two rest leaves separated by options,
a malformed definition with malformed args, and map2 regrouping. A1–A6 specify
one unambiguous outcome and preserve metadata/callback order. Help with newlines
in supplied documentation is explicitly verbatim; log’s control escaping is a
different, deliberate contract. No API claims to sanitize terminal help text.

For calendar, tried `-0000`, `+0001`, `10000` without its plus, offset seconds,
ten fraction digits, negative huge years and printing 1 ns. C5/C6 reject
noncanonical years, preserve full nanosecond precision and round-trip every
constructible Civil/Offset. Arithmetic does not acquire an artificial four-digit
year limit. A fixed offset remains data even when no present civil jurisdiction
uses it. No time-zone or local-clock lookup has entered the pure module.

No further serious issue was found in this second clean reading pass. It is a
contract review, not execution of the still-unwritten full modules.

### 10.4 What was run

All working files were under this lane, in temporary `.std2-probes` and
`.std2-negative` packages. They are not part of the commit. Every command ran
from the lane through `direnv exec /home/agent/Programming/i`. The prebuilt
compiler was `i-1df2fe0`; no compiler or dependency build was performed.

| Command/check | Actual result |
|---|---|
| `i-1df2fe0 check --project .std2-probes --types` | All 55 pure public function types were checked as aliases using the proposed nominal definitions; 54 also used an identity witness taking/returning that function type. lazy’s two-error union cannot be moved into a negative witness parameter under §4.4, so its alias and real inferred implementation were checked instead. |
| Same check, five future signatures | All five type shapes checked using **local stand-in nominal types** for Clock/ClockError, Await/Owner, Process/ProcessError and Stdio/StdioError. This does not check imports of unavailable i modules or their bodies/ABI. |
| `i-1df2fe0 test --project .std2-probes` | 11 tests pass: three CLI closure tests, three day-conversion tests, five closed logging tests. The day cycle checks 146,098 consecutive inverse conversions; huge tests cover ±10^100. |
| `i-1df2fe0 test --project .std2-probes --opt 2` | The final same 11 tests pass under optimization. |
| `i-1df2fe0 check --project .std2-negative`, separate source per case | Generic log client: E0505. Unsafe sink row: E0501. Incompatible installed Sink error arguments: E0515. Private Instant constructor access: E0204. Instant passed as Duration: E0501. |
| Explicit generic return annotation on write | E0505; inferred body plus a closed function binding passes and runs. |
| Python integer calendar model compared with Python datetime | 10,000 seeded dates across years 1..9999 agree in both directions; 13 further negative/huge day inverses pass. |
| `i-1df2fe0 fmt` on separate temporary files extracted from the note | All 11 fenced i snippets parse successfully in the accepted syntax, including the intentional type-error witnesses. |
| Source review and `git diff` of cached/current checker files | Confirmed the evidence table and that only field-privacy checks changed in types/ since the cached compiler. |

Signature checking establishes that the types are legal, not that all functions
have been implemented. Prototype tests exercise the stated difficult kernels;
they are not production module tests. In particular the complete CLI scanner,
calendar text parser, log formatter, real clock measurement, real process args,
and stderr delivery have not been run. No host cancellation, cross-target,
performance, full std suite or exact-baseline compiler build is claimed.

## 11. Risks and coordinator questions

The main risks are negative-date division, drift between the CLI metadata and
bucket decoder, inference of a service carrying its own callback environment,
and IO cancellation after partial stderr output. The tests and phase gates
above address each separately. Calendar text must not be advertised as full
RFC 3339; log text must not depend on core.inspect. Callback/closure depth and
linear CLI option lookup are documented costs, not measurements.

1. **How should the generic logging compiler failure be scheduled?** Recommend
   retaining the typed sink API and assigning a general inference regression/fix
   before declaring full log ready, as was done for Std-1/K1. Allow the pure
   data/factories/formatter and demonstrated closed clients to proceed. Require
   both positive cases and the invariant-row negative case in §7; do not remove
   N, erase E, capture a running Owner, or add a rule for std/log.
2. **Is the small calendar text profile the desired first format?** Recommend
   C5/C6: mandatory offset, nanoseconds, astronomical/expanded years and one
   canonical printer. It preserves the total conversion laws. Add a separately
   named strict RFC 3339 codec later if required; do not narrow Date’s range or
   claim the current parser accepts every standard spelling.
3. **Is the initial CLI surface sufficient without subcommands/completion?**
   Recommend yes: typed flags/options/positionals, map/map2 and generated help
   first. A caller can dispatch a leading token to separate parsers. A later
   command layer should compose ordinary data and functions without changing
   A1–A8 or needing macros, reflection or synthesized record types.

The sink service decision, separation of time values from clock observation,
and Gregorian UTC/fixed-offset scope are already given by the task; they need
no reconfirmation. Real IO remains unverified until the accepted runtime and
provider APIs land. Recheck the five IO signatures against that landed surface;
local stand-in type checks are not a substitute.

## 12. Pure implementation record

The implementation adds only the eight source/test files in §9 and updates this
note. The lane began clean; there was no interrupted implementation to keep or
discard. It merged redesign at `7b3cea3` before implementation. The four modules
contain all 55 pure functions; the five future IO functions remain phase two.
There is no Rust, manifest, language-spec, host-adapter, golden or derived-file
change. The module headers contain their public inventories, rules and laws.

### 12.1 Interpretation evidence

These checks establish the requirements independently of generated expectations:

| Choice or expected value | Controlling sentence and check |
|---|---|
| Exact signed units and distinct epoch positions | T1: “The private integer is nanoseconds”; T3: “between(a,b) is b minus a”. The tests spell out the unit multipliers, orientation, inverse laws and 10^100-hour case; hidden-constructor and nominal-mixing checker cases remain rejected. |
| Truncating duration division, floor calendar division | T2: “otherwise it divides toward zero, including for a negative divisor”; C3: “Never use truncating division for eras, days or weekday on negative input.” Signed duration cases and negative epoch/day boundaries check these separately. |
| Review calendar table | C1's Gregorian rule, C3's 1970 origin and C4's Monday-based weekday formula require the nine review table rows, including +10000. Tests use those fixed rows and a separate January-based year-counting oracle at 38,424 month endpoints, plus century and huge-day boundaries. This oracle does not use the production March-era inverse. |
| Epoch -1 ns and extreme offsets | C4: “at adds offset_minutes×60×10^9 to the epoch count, splits by floor division”. The fixed expected date/time fields cover -1 ns and epoch at both -1439 and +1439 minutes, alongside the inverse laws. |
| Strict calendar text, one parse error | C5: “Any bad syntax or invalid component gives Error(BadText); constructor functions retain their precise errors.” The parser validates components and catches its local validation errors; the tests independently spell out canonical text and malformed boundaries. |
| CLI namespace and diagnostic policy | A1: “Every long/short name is unique in its own option namespace”; positional names are unique among positionals. Long, short and positional namespaces are separate. The coordinator's ruling is now the first paragraph of §5.3. Short-alias missing/conversion cases assert canonical names and distinct option/value indexes. |
| CLI token ownership and error precedence | A3: “This arity rule is independent of the read callback”; A6: “First definition validation ... then token scanning errors ... then extra positionals ... then missing/value conversion errors in descriptor order.” Tests include every pure §5.5/review-table case, stopping later callbacks, heterogeneous composition and help bytes from A8. |
| Internal CLI bucket consumption | §5.4: “Each leaf consumes exactly one bucket; pure consumes none.” The executable test harness compiles the implementation as a temporary test module and checks empty supplied buckets, the exact unused suffix and full public parse consumption. |
| Call-time logging and lazy suppression | L2: “write obtains the current service ... original Entry”; L3: “False calls neither message nor send.” Closed clients record entry order, replacement of an already-created producer, eager argument evaluation, lazy suppression and unchanged custom errors with exactly one delivery attempt. |
| Sink row and generic gate | L5: “E is covariant; N is invariant through the unused `(N)->Nil` marker”; §7: “The implementation can use inferred function bodies and explicitly typed closed client calls in the meantime.” No marker, error or service parameter was removed. Both negative cases and all five waiting positive probes are executable in log_test.i. Successful generic clients must then pass runtime assertions at both optimization levels. |
| Log text bytes | L6: “Other U+0000..001F and U+007F..009F, plus U+2028/U+2029, become `\u{h}` with minimal lowercase hex digits.” Fixed expectations cover quote/backslash, LF/CR/tab, NUL, ESC, C1 boundaries, separators, Unicode, duplicate/empty fields and signed timestamps. |
| Future tests stay deferred | §9: “Keep IO-fake cases in the matching second phase.” T4/T5, cli.read and L7 require the unavailable IO services. Their absence does not change any pure signature. |

### 12.2 Inferred public interfaces

Measured with the lane-built `target/release/i check --project std --types`.
All 16 time, 18 calendar, 13 cli and 8 log functions agree with §3.2, §4.2,
§5.2 and §6.2 respectively, including parameter labels. No export is added or
omitted in the pure phase. Constructors and their visibility match those same
sections and spec §2.4.

The printer renames type variables: silent's A is the note's N; filter,
minimum and enabled use A/B for E/N; lazy uses E for the message's F and E2 for
the sink's E. These are alpha-renamings of the same universally quantified
types. The order of a union does not matter (spec §4.4). write/lazy omit result
annotations under the explicit §7 workaround, but their inferred public types
retain all required errors and services. There is no semantic API difference.

```text
std/time.nanoseconds : (count: Int) -> Duration
std/time.milliseconds : (count: Int) -> Duration
std/time.seconds : (count: Int) -> Duration
std/time.minutes : (count: Int) -> Duration
std/time.hours : (count: Int) -> Duration
std/time.to_nanoseconds : (duration: Duration) -> Int
std/time.add : (first: Duration, second: Duration) -> Duration
std/time.subtract : (first: Duration, second: Duration) -> Duration
std/time.scale : (duration: Duration, by: Int) -> Duration
std/time.divide : (duration: Duration, by: Int) -> Result(Duration, TimeError)
std/time.compare : (first: Duration, second: Duration) -> Order
std/time.from_unix_nanoseconds : (count: Int) -> Instant
std/time.unix_nanoseconds : (instant: Instant) -> Int
std/time.shift : (instant: Instant, by: Duration) -> Instant
std/time.between : (start: Instant, end: Instant) -> Duration
std/time.compare_instants : (first: Instant, second: Instant) -> Order

std/calendar.is_leap_year : (year: Int) -> Bool
std/calendar.days_in_month : (year: Int, month: Int) -> Result(Int, CalendarError)
std/calendar.date : (year: Int, month: Int, day: Int) -> Result(Date, CalendarError)
std/calendar.time_of_day : (hour: Int, minute: Int, second: Int, nanosecond: Int) -> Result(TimeOfDay, CalendarError)
std/calendar.offset : (minutes: Int) -> Result(Offset, CalendarError)
std/calendar.utc : () -> Offset
std/calendar.date_parts : (date: Date) -> (Int, Int, Int)
std/calendar.time_parts : (time: TimeOfDay) -> (Int, Int, Int, Int)
std/calendar.offset_minutes : (offset: Offset) -> Int
std/calendar.day_number : (date: Date) -> Int
std/calendar.from_day_number : (day: Int) -> Date
std/calendar.weekday : (date: Date) -> Weekday
std/calendar.add_days : (date: Date, days: Int) -> Date
std/calendar.days_between : (start: Date, end: Date) -> Int
std/calendar.at : (instant: time.Instant, offset: Offset) -> Civil
std/calendar.to_instant : (civil: Civil, offset: Offset) -> time.Instant
std/calendar.parse : (text: String) -> Result((Civil, Offset), CalendarError)
std/calendar.format : (civil: Civil, offset: Offset) -> String

std/cli.pure : (value: A) -> Parser(A)
std/cli.flag : (names: Names, help: String) -> Parser(Bool)
std/cli.option : (names: Names, help: String, read: (String) -> Result(A, String)) -> Parser(A)
std/cli.argument : (name: String, help: String, read: (String) -> Result(A, String)) -> Parser(A)
std/cli.optional : (names: Names, help: String, read: (String) -> Result(A, String)) -> Parser(Option(A))
std/cli.many : (names: Names, help: String, read: (String) -> Result(A, String)) -> Parser(List(A))
std/cli.rest : (name: String, help: String, read: (String) -> Result(A, String)) -> Parser(List(A))
std/cli.map : (parser: Parser(A), with: (A) -> B) -> Parser(B)
std/cli.map2 : (first: Parser(A), second: Parser(B), with: (A, B) -> C) -> Parser(C)
std/cli.text : (text: String) -> Result(String, String)
std/cli.integer : (text: String) -> Result(Int, String)
std/cli.parse : (parser: Parser(A), args: List(String)) -> Result(A, CliError)
std/cli.help : (parser: Parser(A), program: String) -> Result(String, DefinitionError)

std/log.make : (accept: (Level) -> Bool, send: (Entry) -> Nil ? E ! N) -> Sink(E, N)
std/log.silent : () -> Sink(Never, A)
std/log.filter : (sink: Sink(A, B), accept: (Level) -> Bool) -> Sink(A, B)
std/log.minimum : (sink: Sink(A, B), level: Level) -> Sink(A, B)
std/log.enabled : (level: Level) -> Bool ! Sink(A, B)
std/log.write : (entry: Entry) -> Nil ? E ! N | Sink(E, N)
std/log.lazy : (level: Level, message: () -> String ? E ! N) -> Nil ? E | E2 ! N | Sink(E2, N)
std/log.format : (entry: Entry) -> String
```

### 12.3 Compiler gate and verification

Five positive checker cases remain waiting for K4, each with E0505: the full
annotated logging body, the review's smaller annotated service body, an ordinary
generic write client, an ordinary generic lazy client, and the review's minimal
invariant-service client. The three client cases carry runtime tests that the
harness runs at optimization 0 and 2 once checking succeeds. They are excluded
from the ordinary passing suite under §7's gate, not advertised as supported.
Closed logging clients run in the regular suite. The unsafe Consumer row case
still gives E0501; incompatible installed Sink error arguments give E0515.

The smallest rejection needs no logging library:

```i
S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) -> Nil ! N | S(N) {
  S(f) = s
  f()
}
```

It is a finite type under spec §4.7/§4.8. The checker failure is the known
conformance defect, not permission to change Sink or weaken service equality.
No conflict between the controlling design contracts was found.

All commands below run from this lane through
`direnv exec /home/agent/Programming/i`. Cargo and dev-verify use the shared
`/home/agent/Programming/i/.rd/cache/heavy` wrapper.

| Command/check | Final result |
|---|---|
| `cargo build --release --locked -p ilang-cli` | Passed; exact lane compiler built with the pinned Binaryen submodule. |
| `./scripts/dev-verify i` | Passed: formatting of lib/std/bench/examples; std 1446 passed at both optimization levels; hello 2 and bench 12 passed at each level; WASI smoke passed. |
| `target/release/i test --project std --opt 0` and `--opt 2`, as invoked by dev-verify | Both passed, 1446 tests each. These are the runtime acceptance checks. |
| `target/release/i fmt --check std` | Passed. |
| `./scripts/old-syntax-check` | Passed: 1181 snippets, exact expected diagnostics, 19 unchanged future-track entries. |
| `target/release/i check --project std --types` | Passed; all 55 measured interfaces are listed above. |
| Executable checker harnesses in time_test.i and calendar_test.i | Required nominal separation E0501 and hidden-constructor/update E0204 cases rejected. Each harness first requires a passing baseline and restores its file in finally. |
| Executable internal CLI decoder harness in cli_test.i | One internal test passed at each optimization level; test file restored. |
| Executable logging harness in log_test.i | Negative cases rejected with E0501/E0515; the five positive cases remain E0505 gates, as described above. |

Development checks that failed or were superseded:

- Early time checks stopped on concurrently unfinished calendar/CLI syntax.
  The module writers corrected those syntax errors before the combined gates.
  Calendar also corrected local pattern/module shadowing during checking.
- The first calendar runtime run had three passing and two failing test groups.
  Its loops incorrectly treated list.range as exclusive and included month 13.
  list.i says the range has “both ends”; correcting those bounds preserves the
  intended month/year coverage. Both final calendar optimization runs passed.
- The first log run had four passing groups and one format failure because
  int.to_base16 emits uppercase. L6 explicitly requires lowercase; the
  implementation now lowercases those digits, leaving the expectation intact.
- The first CLI internal harness put @test in src/, giving E0305. It now
  compiles the production source as a temporary test module, preserving the
  required private bucket checks without changing the production API.
- One earlier full opt-2 run overlapped that temporary CLI test copy and ran
  1437 tests. That is a substitute measurement, not the acceptance result.
  The final dev-verify run used the restored suite and passed all 1446 twice.
- Cached-compiler module runs were substitutes during the exact compiler build.
  They do not replace the final lane-built results. Formatter-sensitive edit
  attempts that failed to match made no file change and were retried against
  the formatted text. No check or expected behavior was weakened.

The Binaryen submodule was initialized at its pinned revision and the lane's
target/release/i was built; these build artifacts remain available. Temporary
test mutations were restored byte for byte. No user checkout or other lane was
edited. No environment setting was changed. There are no derived files to
regenerate. Full generic logging waits for K4; IO adapters and their fake/host
tests wait for the separate IO phase. Neither requires an API change here.
The final redesign merge check was already up to date at `7b3cea3`.
