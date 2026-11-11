# Design: four pure standard modules

Status: API choices accepted by the coordinator, with the review corrections
below. Path and sequence may proceed independently. Stream implementation waits
for the compiler prerequisite in §3.4. This note changes no implementation.
Source baseline: `1e0d910e7308d5b69280eba4deef59d6ec156146`.
All repository source positions below refer to that baseline. The independent
review is `/home/agent/Programming/i/.rd/cache/gpt/report-review-std1.md`;
its two findings were checked again for this revision. Examples use the accepted
syntax. Signature inventories describe bindings with bodies, not external
declarations or new syntax. `Option`, `Some`, and `None` are explicitly imported
from `std/option`; `ContinueOrStop` is qualified through `std/list`.

## 1. Problem and controlling sources

We need lazy iteration, a persistent indexed deque, lexical POSIX paths, and
RFC 3986 URI references. Each must run as ordinary i source under `i run`.
Construction must not perform IO, introduce mutable state, or change inference.
Callbacks may use the language's existing errors and services.

The controlling order is the spec, accepted syntax, accepted scoped notes, the
plan, then the ledger. In particular:

| Source | Constraint used here |
|---|---|
| `docs/spec.md:124–142`, `:196–206` | Ordinary packages, acyclic module imports, hidden constructors, no re-export of imported names. |
| `docs/spec.md:312–414` | Sets of values, nominal custom types, variance, principal inference, value restriction, monomorphic recursive groups. |
| `docs/spec.md:511–518`, `:737–813` | Strict evaluation; a zero-argument function defers work; errors and services belong to calls. |
| `docs/spec.md:849–902`, `:968–975` | Resources are scoped computations; expected failures differ from assertions; equality follows representation. |
| `docs/design/syntax.md:99–200`, `:223–253`, `:372–386` | Import/export groups, declared UpName variables, ordinary clause functions. |
| `docs/design/literal-types.md:50–76`, `:416–438` | Written public types prevent accidental literal-only interfaces; callback state must use the accepted hidden-input known-type fix. |
| `docs/design/records.md:57–78`, `:480–486` | Nominal values are not structural records; no record patterns. This API needs neither records nor label parameters. |
| Plan `/home/agent/.claude/plans/twinkling-swinging-perlis.md:30–58`, `:74`, `:522–544` | No library exceptions; four plain-i modules; string offsets count codepoints; one writer per module; std test gate. |
| Ledger `/home/agent/Programming/i/.rd/cache/ledger.md:61–64`, `:89` | Syntax and literal/record notes accepted; baseline host work landed; new-syntax migration is still separate. |

Facts checked by reading code:

| Code region | Evidence and consequence |
|---|---|
| `std/src/list.i:9–18`, `:188–199`, `:271–278`, `:366–407`, `:447–455`, `:512–524` | Missing elements use `Option`; negative indexes miss; take/drop clamp negative counts; callbacks carry `^ E ! N`; folds run left to right. Keep these conventions. |
| `std/src/dict.i:24–60`, `:136–143`, `:237–244` | A public type hides private recursive nodes; updates return new values. Follow this encapsulation. |
| `std/src/string.i:4–6`, `:70–94`, `:110–139`, `:254–308`, `:428–455` | String indexes are codepoints; byte size is separate; concat takes fragments; slice/pop_codepoint convert codepoint lists. Do not repeatedly pop a string in a linear parser. |
| `std/src/option.i:1–6`, `:27–31`, `:54–59`; `std/src/result.i:1–5`, `:244–252` | Absence and failure are distinct; `Result` and error effects have explicit bridges. |
| `lib/i/fail.i:10–31` | Raise/catch already exist. Stream error adapters need no new primitive. |
| `std/test/list_test.i:1847–1853`, `:1973–1985`, `:2073–2139` | Existing tests cover bounds, error callbacks, labels, known nominal fields, services and control callbacks. Preserve all of these cases for new containers. |
| `std/test/dict_test.i:420–428`, `:857–1014` | Tests keep old versions and use a deterministic list model. Use that method for sequences. |
| `std/test/string_test.i:2060–2095`, `:2240–2247`; `std/test/option_test.i:255–280`; `std/test/result_test.i:319–342`, `:372–382` | Empty pieces, Unicode, known fields, deferred callbacks, long walks, and union errors have concrete precedents. |
| `crates/ilang-check/src/types/infer.rs:232–247`, `:292–325` | Recursive group names get monomorphic holders before generalization. A Haskell helper recursively called at `Node(A)` cannot simply be ported. |
| `crates/ilang-check/src/types/infer.rs:2237–2266`, `:2321–2329` | Custom variance is found by a fixed point; function result, errors and environment are traversed. A plain thunk wrapper would infer positive N variance. That is insufficient to preserve the stricter environment relation; §3 adds an ordinary negative occurrence to make N invariant. |
| `crates/ilang-check/src/types/display.rs:670–679`, `:1028–1052`; `crates/ilang-check/src/types/infer.rs:2104–2120` | Normalization can collapse equal Both sides; the ordinary constructor occurrence walk does not visit invariant arguments in both directions. Generalization consumes the simplification, so the factory failure in §3.4 affects accepted clients, not only display. This requires a general compiler conformance fix. |
| `crates/ilang-check/src/order.rs:135–168`, `:188–207` | Direct recursive functions are possible; eager self-dependent values are rejected. Build infinite streams through functions, not an eager cyclic list. |
| `std/src/bit_array.i:33–45`, `:70–77`; `crates/ilang-wasm/src/runtime/core.rs:110–125` | UTF-8 conversion can fail; whole byte fragments can be joined; conversion validates UTF-8. URI bytes need not be UTF-8. |

These are reading results, not evidence that the new modules compile. Cost
notes count element/byte/node operations; arbitrary-size integer arithmetic,
callbacks and element equality add their own costs. The cost
proofs and implementation sketches below are design inferences. No performance
measurements or new-module execution are claimed.

## 2. Alternatives, criteria, and decisions

Choose by: agreement with sources; representability under rank-one inference;
predictable effects and failure; persistent costs; few concepts; enough API for
independent writers and later IO/HTTP consumers.

| Design | Stream | Sequence | Path/URI | Assessment |
|---|---|---|---|---|
| A: direct ports | Stateful Python iterator or control generator | Nested Haskell finger tree | Go-shaped mutable URL and platform filepath | Familiar surface, but iterator ownership, polymorphic recursion and platform rules do not fit the task. |
| B: minimal lists and strings | Thunk returning a list | Two lists with occasional reversal | String splitting with decoded URI fields | Small, but misses logarithmic sequence indexing/concat, URI escape fidelity, and demand granularity. |
| C: explicit pure values | Replayable pull thunk with invariant service row | Strict measured finger tree with uniform private nodes | Lexical path functions; validated URI with raw parts | Fits existing features; more internal invariants and an honest strict cost bound. Choose this. |

Keep A's reference test suites and B's small surface and list-model tests. Do not
port whole foreign APIs. No type classes, generic measurement abstraction,
protocol dispatch, macros, or compiler registration are introduced.

Decisions, reasons, and sources:

1. **All four modules are optional package source.** Nothing in the compiler
   recognizes their paths or constructors. Existing module/export rules suffice
   (spec §2.1/§2.4, source table above).
2. **Factories are functions with written interfaces.** In particular `new()`
   avoids sharing one weakly typed empty value across unrelated uses. This follows
   the value restriction (spec §4.7) and `list.new` at `std/src/list.i:394–398`.
3. **Streams store calls, not running continuations.** Making/composing a stream
   is pure; pulling supplies services at that call. This is spec §9.1/§9.2, not a
   stream-specific effect rule. The pure case uses `Never` for both effect parts.
4. **Errors stop a traversal; they are not end-of-stream.** Value errors can be
   ordinary elements, or explicitly promoted/demoted with adapters. Keep the
   distinction of `std/src/result.i:1–5`, `:244–252`.
5. **Sequence is an eager, uniform-node finger tree.** Keep measured splitting and
   concatenation, avoiding polymorphic recursive helpers. End edits have O(log n)
   worst-case bounds in every persistent history. Do not promise Haskell's
   persistent amortized O(1) ends. The source language is strict; the
   [finger-tree paper's correction](https://www.staff.city.ac.uk/~ross/papers/FingerTree.html)
   identifies laziness as material to its analysis. The
   [Data.Sequence API](https://hackage-content.haskell.org/package/containers-0.8/docs/Data-Sequence.html)
   is the operation reference. The coordinator accepted this cost choice.
6. **Use `Option` for sequence misses and `Result` for malformed inputs.** No
   ordinary bounds/parse failure needs `Assert`. Reasons: spec §9.6 and existing
   option/result conventions in the source table.
7. **Paths use only `/`, with no host state.** Adopt a lexical API inspired by
   [Go path](https://pkg.go.dev/path) and the pure part of
   [filepath.Rel](https://pkg.go.dev/path/filepath#Rel). Make the base explicit;
   omit cwd, drives, filesystem access, symlinks and globbing.
8. **URI syntax and decoding are separate.** Use the `URI-reference` production
   of [RFC 3986, Appendix A](https://www.rfc-editor.org/rfc/rfc3986.html#appendix-A),
   including its host grammar, and §5.2 for resolution. The concrete storage,
   errors, byte codecs and form helpers below are this API's decisions.
   [Go net/url](https://pkg.go.dev/net/url#URL) is a useful component API reference;
   avoid its paired decoded-path/raw-path representation. Percent-encoded bytes
   may not be Unicode text, whereas i strings are Unicode (spec §4.2).
9. **Query forms are a separate convention.** The URI parser leaves query text
   untouched. A form helper treats `+` as space, like Go's
   [QueryUnescape](https://pkg.go.dev/net/url#QueryUnescape); generic decoding
   leaves it as `+`. Keep ordered duplicate pairs instead of a map. Neither
   [Python's permissive URL splitting](https://docs.python.org/3/library/urllib.parse.html#url-parsing-security)
   nor browser URL repair is a validation contract here.

No new language feature is needed. Every custom type denotes its constructor
values; a function field denotes functions with the stated input, result and
effects. Hidden invariants are library construction invariants, not refinements
the checker must infer. These building blocks have unrelated uses:

| Module | At least three uses |
|---|---|
| stream | Infinite numeric enumeration; incremental parsers; later paged network reads. |
| sequence | Breadth-first work queue; editor history pieces; indexed query results with concatenation. |
| path | Package paths; archive entry names; filesystem paths passed later to IO. |
| uri | Document links; service endpoint configuration; HTTP request targets and redirects. |
| json | Structured interchange; configuration; HTTP payloads. |
| http | Message framing; request construction; response inspection. |

## 3. `std/stream`

### 3.1 Types and complete exports

Export `Stream` without its constructor; export `Step(..)`. Export exactly the
functions below. `Step` is data; its `Next` tail describes later calls.

```i
Stream(A, E, N) {
  Stream(() -> Step(A, E, N) ^ E ! N, (N) -> Nil)
}
Step(A, E, N) {
  Done
  Next(A, Stream(A, E, N))
}
```

```text
new(A, N): () -> Stream(A, Never, N)
wrap(A, N): (item: A) -> Stream(A, Never, N)
from_list(A, N): (items: List(A)) -> Stream(A, Never, N)
make(A, E, N): (step: () -> Step(A, E, N) ? E ! N) -> Stream(A, E, N)
defer(A, E, N): (body: () -> Stream(A, E, N) ? E ! N) -> Stream(A, E, N)
unfold(S, A, E, N): (from: S, with: (S) -> Option((A, S)) ? E ! N) -> Stream(A, E, N)
pull(A, E, N): (stream: Stream(A, E, N)) -> Step(A, E, N) ? E ! N
map(A, B, E, F, N): (over: Stream(A, E, N), with: (A) -> B ? F ! N) -> Stream(B, E | F, N)
filter(A, E, F, N): (over: Stream(A, E, N), keeping: (A) -> Bool ? F ! N) -> Stream(A, E | F, N)
filter_map(A, B, E, F, N): (over: Stream(A, E, N), with: (A) -> Option(B) ? F ! N) -> Stream(B, E | F, N)
flat_map(A, B, E, F, G, N): (over: Stream(A, E, N), with: (A) -> Stream(B, F, N) ? G ! N) -> Stream(B, E | F | G, N)
append(A, E, F, N): (first: Stream(A, E, N), second: Stream(A, F, N)) -> Stream(A, E | F, N)
zip(A, B, E, F, N): (first: Stream(A, E, N), second: Stream(B, F, N)) -> Stream((A, B), E | F, N)
take(A, E, N): (from: Stream(A, E, N), up_to: Int) -> Stream(A, E, N)
drop(A, E, N): (from: Stream(A, E, N), up_to: Int) -> Stream(A, E, N)
take_while(A, E, F, N): (from: Stream(A, E, N), keeping: (A) -> Bool ? F ! N) -> Stream(A, E | F, N)
fold(A, Acc, E, F, N): (over: Stream(A, E, N), from: Acc, with: (Acc, A) -> Acc ? F ! N) -> Acc ? E | F ! N
fold_until(A, Acc, E, F, N): (over: Stream(A, E, N), from: Acc, with: (Acc, A) -> list.ContinueOrStop(Acc) ? F ! N) -> Acc ? E | F ! N
each(A, B, E, F, N): (over: Stream(A, E, N), with: (A) -> B ? F ! N) -> Nil ? E | F ! N
to_list(A, E, N): (stream: Stream(A, E, N)) -> List(A) ? E ! N
from_results(A, E, N): (items: List(Result(A, E))) -> Stream(A, E, N)
attempt(A, E, N): (stream: Stream(A, E, N)) -> Stream(Result(A, E), Never, N)
map_environment(A, E, N, M): (stream: Stream(A, E, N), run: { (body: () -> Step(A, E, N) ? E ! N) -> Step(A, E, N) ? E ! M }) -> Stream(A, E, M)
```

A/E are covariant; N is invariant. The second private Stream field is an unused
function accepting N and returning Nil. It is made as `(_: N) { Nil }` and never
called. This is an ordinary negative occurrence, so the existing variance rule
makes N invariant, including through Step's tail. It prevents a Stream needing
`Sink(Int)` from being coerced to one needing `Sink(Never)` through the ordinary
variance of Sink. Service type arguments require equality inside function
environments (spec §4.5 rule 4), unlike ordinary value subtyping. Merely hiding
the constructor would not prevent this coercion.

Consequently combinators use one shared N, and never write `Stream(A, E, N | M)`:
an invariant argument would make that multi-variable union illegal under spec
§4.4 (`infer.rs:600–655`). Choose the full common environment at construction.
The API keeps pure factories polymorphic in N even though they use no services,
so they can supply empty tails in any such environment once §3.4 is fixed;
a wholly pure client chooses
Never. An already typed stream cannot acquire a wider row by subtyping.
`map_environment` explicitly rebuilds its tails with a rank-one runner for each
step. The runner may install/remove services or have a written larger environment
of concrete service types. It is not polymorphic over the computation result:
its fixed Step(A,E,N) appears on both sides. This uses the existing principal
inference rules without a row-merging rule or a higher-rank handler. The current compiler does not yet preserve generic
invariant N in the empty factory; §3.4 is a prerequisite, not an API change.

All written unions of distinct error variables above are in positive output
places. Incompatible arguments of the same service still cannot occur in a
common N. Different nominal services can. Generic callers may need to write that
common environment in their constructors; this is a deliberate ergonomic cost.

### 3.2 Exact behavior

- **S1 Demand.** `make` stores its argument; `pull` calls it once. `new` pulls
  `Done`; `wrap(x)` pulls `Next(x, new())`; `from_list` creates one cell at a time.
  `defer` calls `body` only on pull, then pulls the returned stream in that call.
  `unfold` calls `with(state)` once per pull: `None` ends; `Some((x, next))`
  emits x and retains next without another callback.
- **S2 Replay.** There is no memoization or advancing cursor. Pulling the same
  value again calls its thunk again. A returned tail is the position after that
  element. Pure deterministic thunks reproduce their observations; effectful
  thunks may observe different services/results. Service lookup happens at pull,
  not construction. map_environment calls its runner once for each attempted
  input pull, maps each returned tail recursively, and keeps no result cache.
  The runner is responsible for actually calling its body if it wants that step;
  like any callback it may instead fail or return its own valid Step. The API
  does not inspect functions or compare streams by their elements; ordinary `==` keeps spec §11.4's meaning.
- **S3 Composition.** Map/filter callbacks run only while pulling, in source
  order. Filtering may pull many source cells to produce one cell. `flat_map`
  drains each inner stream before asking for another outer element; it is depth
  first and need not be fair for infinite inners. `append` exhausts first before
  pulling second. Function arguments themselves remain strict: callers use
  `defer` if *making* second needs effects. No combinator silently prefetches.
- **S4 Bounds.** `take` with a count <= 0 ends without a source pull. With k > 0
  it emits at most k values and never pulls the source tail after the kth value.
  `drop` clamps a negative count to zero, and performs its skipped pulls only on
  demand. On its first pull it skips k values then pulls the next cell, unless
  the source ends first. `take_while` consumes the first rejected element and
  ends; it does not pull that element's tail.
- **S5 Paired demand.** `zip` pulls left first. Left `Done` means right is not
  pulled. Left `Next` followed by right `Done` discards that one left element.
  Consequently zip is symmetric in pure output values, not in effect traces.
- **S6 Consumption.** Fold callbacks receive `(accumulator, element)`, once per
  element. `fold_until` on `list.Stop(acc)` returns acc without pulling the tail.
  `each` discards callback values. `to_list` preserves order. These operations
  may diverge on infinite streams, as may a filter that never finds a match.
- **S7 Failure.** A pull or callback error exits that traversal immediately;
  neither a tail nor an end marker is manufactured. `from_results` raises the
  first `Error(e)` when that position is pulled. A stream *of* results is instead
  `from_list(results)`. `attempt` emits `Ok` for every successful element, emits
  one `Error(e)` when a pull fails, then ends. It catches only errors, not Assert,
  stops, or cancellation. Retrying a failed position requires another explicit
  pull of that same value, and repeats its work under S2.
- **S8 Lifetime.** No stream owns a resource or promises `close`. Later IO code
  must keep resource scope around the complete consumption, including early
  termination. Core functions do not capture or manage continuations themselves.
  Supplied callbacks or elements may contain them; dropping a stream does not
  cancel them or run finalizers. Such continuations and captured external
  resource handles remain subject to their own library's scope/cancellation
  rules. This type is not a lifetime proof.

Implementation witnesses, using ordinary constructors and calls:

```i
make(A, E, N) = (step: () -> Step(A, E, N) ^ E ! N) -> Stream(A, E, N) {
  Stream(step, (_: N) { Nil })
}
pull(A, E, N) = (stream: Stream(A, E, N)) -> Step(A, E, N) ^ E ! N {
  Stream(step, _) = stream
  step()
}
map_environment(A, E, N, M) = (
  stream: Stream(A, E, N),
  run: { (body: () -> Step(A, E, N) ^ E ! N) -> Step(A, E, N) ^ E ! M }
) -> Stream(A, E, M) {
  make(() {
    run(() { pull(stream) }) |> {
      Done -> Done
      Next(x, tail) -> Next(x, map_environment(tail, run))
    }
  })
}
```

The private constructor has one variant, so taking it apart adds no Assert.
Multi-variant Step/Option matches need both branches; no one-clause partial
match is used to implement normal end-of-stream.

### 3.3 Laws, tests, and costs

Laws compare finite observations under pure deterministic callbacks, not thunk
identity. For every finite list xs: `to_list(from_list(xs)) == xs`; map identity
and composition preserve observations; append has empty identity and associates;
flat_map has singleton identity and associates where observations terminate.
Take/drop observations agree with `list.take`/`list.drop`. Errors and effects
also need separate demand tests, not just these extensional laws.

| Test | Expected observation | Rule |
|---|---|---|
| Collect defer(() { wrap(7) }) | `[7]`; body runs only at pull | S1 |
| Drop 2 from `[1, 2, 3]`, then collect | `[3]` | S4 |
| Map `[1, 2]` with doubling, then collect | `[2, 4]` | S3/S6 |
| Unfold from 0 with `Some((n, n + 1))`, take 3 | `[0, 1, 2]` | S1/S4 |
| Construct `make(() { fail.raise("bad") })`; take 0 and collect | `[]`, with no error | S1/S4 |
| Take 1 from append(wrap(1), failing stream), collect | `[1]`, no error | S3/S4 |
| Drop -2 from `[1, 2]`, collect | `[1, 2]` | S4 |
| Filter `[1, 2, 3]` with even test | `[2]` | S3 |
| Filter-map that gives Some only for 2 | `[20]` | S3 |
| Flat-map `[1, 2]` to `[x, x]` | `[1, 1, 2, 2]` | S3 |
| Take-while `< 3` from `[1, 2, 3, 1]` | `[1, 2]`; callback sees 3 once | S4 |
| Zip empty left with failing right | Empty; no error | S5 |
| Zip failing left with empty right | Error from left | S5 |
| Fold-until Stop at element 2 over `[1, 2]` plus a failing tail | Returned acc, no tail error | S6 |
| Fold `[1, 2, 3]` from 0 by addition | `6`; each returns `Nil` | S6 |
| Collect `from_results([Ok(1), Error("bad"), Ok(3)])` inside fail.catch | `Error("bad")` | S7 |
| Collect attempt of that stream | `[Ok(1), Error("bad")]` | S7 |

Use a local nominal service `Bias { Bias(value: Int) }` to test S2: construct
`make((b ! Bias) { Next(b.value, new()) })` before installing Bias; pull it under
Bias(10), then under Bias(20), expecting heads 10 and 20. A compile-fail client
pulling it without Bias must be rejected. Repeat the same pure position twice
and compare the heads, then advance its returned tail. A callback needing a
different nominal service is tested by constructing the source with a written
common environment containing both service types. Consumption needs that common
environment; construction still needs none. Test map_environment by providing
Bias(10) inside its runner and collecting the rebuilt stream with no Bias in the
caller: its head is 10. A runner that simply calls its body without providing
Bias cannot advertise a pure M=Never output. Also check all errors from a runner
remain in E, and that repeated adapter pulls rerun the runner.

Factories and simple combinator construction use O(1) work/space. One map pull
costs one input pull plus its callback. Filter/drop/flat_map can do unbounded
work in one output pull; document work in terms of consumed cells and callbacks.
Flat-map must use a tail-recursive private seek loop with explicit outer/current
inner state, so a million empty inners do not grow the call stack. Filter skips
use the same approach. Folds use tail calls; collection allocates O(k) output
and reverses once. Repeated pulls repeat costs; long chains of combinators can
still have depth proportional to the chain. No fusion guarantee.

### 3.4 Compiler prerequisite: retain generic invariant environments

The API above is unchanged, but stream implementation must wait for a separate
compiler track to fix generalization of invariant type arguments. This is a
compiler fault against spec §4.7 (`docs/spec.md:405–409`), which requires principal
types and meaning-preserving simplification. The private negative N field is
needed for sound service compatibility; neither removing it nor specializing
the public factory to Never is an acceptable workaround.

Use the independent review's factory probe as the compiler regression gate,
written here in the accepted syntax with Stream/Step as defined in §3.1:

```i
new(A, N) = () -> Stream(A, Never, N) {
  Stream(
    () -> Step(A, Never, N) ! N { Done },
    (_: N) { Nil }
  )
}
Bias { Bias(value: Int) }
needs_bias = () -> Stream(Int, Never, Bias) { new() }
```

Expected: the factory retains a generalized environment N, needs_bias is
accepted, and constructing its result needs no Bias service. Covariant A may
simplify to Never without losing the allowed element instances; invariant N
must not. The positive `make((b ! Bias) { Next(b.value, new()) })` test in §3.3
must also pass. As the negative gate, the `bad_coerce` Sink(Int) to Sink(Never)
client in §9 must still be rejected. Keep the existing equal-service-argument
rule (spec §4.5 rule 4 and §9.2); no stream-specific exception is permitted.

Checked independently with the cached `i-1e0d910` compiler, using a lane-local
copy of the review's baseline-syntax probe: new becomes
`() -> Stream(Never, Never, Never)` and needs_bias fails with E0501 because Bias
is not a subtype of Never. The review's concrete unsafe Sink coercion probe also
still fails with E0501. These runs reproduce the blocker; they do not verify a
fix or the migrated parser. Temporary probe copies were removed.

Source inspection confirms why the prerequisite is material:
`display.rs:670–679` collapses equal positive/negative Both forms;
`:1028–1052` treats invariant constructor arguments like a single polarity
unless walking an environment. `infer.rs:2108–2120` uses the simplified type for
generalization. The separate compiler track must correct this general behavior
and pass both gates before the stream writer starts; it must not teach the
compiler about Stream, Bias or any std path.

## 4. `std/sequence`

### 4.1 Types and complete exports

Export `Sequence` without constructors, and exactly these functions. A sequence
is finite and strict; it is also the queue type. `push_back` + `pop_front` is FIFO;
no second queue wrapper or reversed pop tuple is needed.

```text
new(A): () -> Sequence(A)
wrap(A): (item: A) -> Sequence(A)
from_list(A): (items: List(A)) -> Sequence(A)
to_list(A): (sequence: Sequence(A)) -> List(A)
length(A): (sequence: Sequence(A)) -> Int
is_empty(A): (sequence: Sequence(A)) -> Bool
first(A): (sequence: Sequence(A)) -> Option(A)
last(A): (sequence: Sequence(A)) -> Option(A)
push_front(A): (to: Sequence(A), item: A) -> Sequence(A)
push_back(A): (to: Sequence(A), item: A) -> Sequence(A)
pop_front(A): (from: Sequence(A)) -> Option((A, Sequence(A)))
pop_back(A): (from: Sequence(A)) -> Option((A, Sequence(A)))
at(A): (sequence: Sequence(A), index: Int) -> Option(A)
set(A): (in: Sequence(A), index: Int, value: A) -> Option(Sequence(A))
append(A): (first: Sequence(A), second: Sequence(A)) -> Sequence(A)
split(A): (sequence: Sequence(A), at: Int) -> (Sequence(A), Sequence(A))
take(A): (from: Sequence(A), up_to: Int) -> Sequence(A)
drop(A): (from: Sequence(A), up_to: Int) -> Sequence(A)
reverse(A): (sequence: Sequence(A)) -> Sequence(A)
map(A, B, E, N): (over: Sequence(A), with: (A) -> B ? E ! N) -> Sequence(B) ? E ! N
fold(A, Acc, E, N): (over: Sequence(A), from: Acc, with: (Acc, A) -> Acc ? E ! N) -> Acc ? E ! N
each(A, B, E, N): (over: Sequence(A), with: (A) -> B ? E ! N) -> Nil ? E ! N
equal(A): (first: Sequence(A), second: Sequence(A)) -> Bool
```

### 4.2 Exact behavior and representation

- **Q1 Values and persistence.** Every operation preserves element order except
  reverse. An edit returns a new sequence and leaves every old version usable.
  Pop tuples always put the removed item first; empty views return `None`.
- **Q2 Bounds.** Indices start at 0. Negative or >= length indexes give `None`
  for at/set; no negative-from-end convention. Split clamps to [0, length];
  take/drop are its two parts. Set replaces exactly one value, preserving length.
- **Q3 Walks.** Map, fold and each call their callbacks left to right and stop
  on the first error. Map is eager and changes no source value. No implicit
  streams or delayed service lookups live inside the resulting sequence.
- **Q4 Equality.** `equal` compares length and then elements using `==`, ignoring
  tree shape. Ordinary `==`, inspect and hash retain the language's structural
  behavior: different edit histories may produce different trees for the same
  list. In particular, equal-by-elements sequences need not be interchangeable
  dict keys. Use `to_list` as a key if that meaning is required. No equality hook.

One implementable private layout is:

```i
Sequence(A) { Sequence(Tree(A)) }
Node(A) {
  Leaf(A)
  Branch2(Int, Node(A), Node(A))
  Branch3(Int, Node(A), Node(A), Node(A))
}
Digit(A) {
  One(Node(A))
  Two(Node(A), Node(A))
  Three(Node(A), Node(A), Node(A))
  Four(Node(A), Node(A), Node(A), Node(A))
}
Tree(A) {
  EmptyTree
  Single(Node(A))
  Deep(Int, Digit(A), Tree(A), Digit(A))
}
```

The Ints cache leaf counts, not node counts. A Leaf has measure 1; each branch
and Deep caches the sum of its parts. Rank is a mathematical invariant, not a
type argument: rank-0 nodes are leaves; rank-(r+1) nodes have two or three
rank-r children. A rank-r Tree's digits/single contain rank-r nodes, and its
middle is rank-(r+1). An EmptyTree can be used at any rank. The public root is
rank 0. Digits are nonempty with at most four nodes. Thus every level shrinks by
at least a factor of two, and root digits give direct access to both ends.

Private helpers always recurse at the same i type. For example:

```text
push_node(A): (tree: Tree(A), node: Node(A)) -> Tree(A)
join_nodes(A): (left: Tree(A), between: List(Node(A)), right: Tree(A)) -> Tree(A)
split_tree(A): (tree: Tree(A), index: Int) -> Option((Tree(A), Node(A), Tree(A)))
```

`between` is bounded internal bridging data, never the elements of a whole
sequence. A full front digit `[a,b,c,d]` receiving x becomes `[x,a]`, with
Branch3(b,c,d) pushed into the middle. Mirror this at the back. An empty end
after pop borrows one branch from the middle, unpacking two or three children;
an empty middle collapses the opposite digit to a shallow tree. Concatenation
groups the left suffix, bridge and right prefix into 2/3-child nodes and joins
the middles. Split descends by cached measure and reconstructs both fringes,
carrying the residual leaf index into the chosen branch. Do not restart a search
from the root at each level or flatten to a list for any logarithmic operation.

For bottom-up construction, turn the list into rank-0 leaves. With 0/1 nodes
make empty/single; with 2–8 nodes use two bounded digits and an empty middle.
With at least 9 nodes, use two nodes at each end, group the remaining nodes into
twos/threes, and recursively build that middle at the next rank. Every integer
at least 2 is a sum of 2s and 3s; the middle group count drops geometrically.
This uses the same Tree(A)/Node(A) helper types at every level and O(n) total
list/node visits. For join bridges of length 2–12, prefer triples, replacing a
remainder of one with two pairs. The next bridge has at most four nodes.

Rank tests and malformed cases are private invariant checks. Use exhaustive
matches; an impossible private Leaf/Branch mismatch may panic with an internal
invariant message. It must never require public Assert or turn a valid client
input into a bounds panic. The type contains more representations than the
library constructs, just as a cached-size tree does; rank is not a new type rule.

### 4.3 Costs and tests

Let n/m be input lengths. The table counts node visits and allocations; Int
arithmetic has its ordinary bit cost for large lengths (spec §11.2). Reading a
cached measure does not traverse the tree.

| Operation | Contract |
|---|---|
| new, wrap, length, is_empty, first, last | O(1) node work. |
| push/pop at either end | O(log(n+1)) worst-case time and fresh nodes; normally only a bounded fringe changes. |
| at, set, split, take, drop | O(log(n+1)) time; at allocates O(1) result, edits copy O(log(n+1)) nodes. |
| append | O(log(n+m+1)) worst-case time/space, sharing untouched subtrees. No flatten/rebuild. |
| from_list | O(n) bottom-up construction; do not rely on an unproved amortized push bound. |
| to_list, reverse, map, fold, each, equal | O(n), or O(n+m) for equal; callbacks/equality add their own costs. |

Walk the tree once with an explicit traversal stack or bounded-depth structural
recursion. Repeated indexing/popping is not the O(n) implementation of a walk.
Map can preserve shape and counts; reverse mirrors digits, child order and
middle recursively. `equal` may use O(n+m) temporary lists in version one.
Structural depth is O(log n), including middle depth plus branch depth.

Laws hold through `to_list` or `equal`, never through representation `==`:
round trip to/from list; append identity and associativity; reversal involution;
split pieces append to the source; length of append is the sum. Every saved
source remains observationally equal to its old list after any descendant edit.

| Test | Expected value | Rule |
|---|---|---|
| first/last/pop either end of new | `None` | Q1 |
| Push back 1, 2, 3; repeatedly pop front | 1, 2, 3, then None | Q1/FIFO |
| Push front 1 onto `[2, 3]`; pop back | Item 3, remainder `[1, 2]` | Q1 |
| At `[10, 20]` at -1, 0, 2 | None, Some(10), None | Q2 |
| Set at 1 to 7; separately observe saved original | Some sequence `[10, 7]`; original `[10, 20]` | Q1/Q2 |
| Split `[1, 2, 3]` at -1, 1, 99 | `([], [1,2,3])`, `([1],[2,3])`, `([1,2,3],[])` after conversion | Q2 |
| Append `[1, 2]` and `[3]`, then reverse | `[3, 2, 1]` | Q1 |
| Map doubling; fold addition from 0; each | `[2,4,6]`; 6; Nil | Q3 |
| Callback raises at 2; callback at 3 would panic | First error, no panic | Q3 |
| Same elements built by append and repeated end insertion | equal gives True; no assertion about `==` | Q4 |

Private tests verify ranks, cached sizes and nonempty bounded digits after every
edit. Exercise lengths 0–20 exhaustively, then powers of 2 and 3 and their
neighbors through 100,000, splits at every boundary of small trees, unequal
height joins, middle borrowing and collapse, and alternating ends. A fixed-seed
operation model must retain and branch from old versions, not only the latest.
Instrumentation in a temporary development variant counts touched nodes for
at/set/split/append on growing sizes; it must show height work, including when
the two arguments alias. Deliver count scripts to the coordinator, not timings.

## 5. `std/path`

### 5.1 Complete exports

Paths are Strings, not validated filesystem handles. Export `RelativeError(..)`
and exactly these functions. Unicode and NUL are ordinary path characters in
this lexical module. An IO boundary may reject a path for its own reasons.

```i
RelativeError { DifferentRoots UnknownParent }
```

```text
clean: (path: String) -> String
join: (parts: List(String)) -> String
split: (path: String) -> (String, String)
base: (path: String) -> String
dir: (path: String) -> String
extension: (path: String) -> String
is_absolute: (path: String) -> Bool
relative: (from: String, to: String) -> Result(String, RelativeError)
```

### 5.2 Exact rules

- **P1 Clean.** Only slash is a separator; repeated slashes collapse, including
  leading `//`. Remove `.` segments. A `..` removes a preceding ordinary segment;
  remaining leading `..` survive in a relative path and disappear at an absolute
  root. Remove a trailing slash except at root. Empty result is `.` for relative
  input and `/` for rooted input. Case, Unicode normalization, backslashes, drive
  spellings, `~` and percent escapes have no special meaning.
- **P2 Join.** Skip empty pieces, put `/` between the rest, then clean. A later
  leading slash does not reset earlier pieces. All-empty input gives `""`,
  including `join([])`; this differs intentionally from `clean("") == "."`.
- **P3 Parts.** `split` cuts raw input after its last slash, retaining that slash
  in the directory part; it never cleans, so concatenating its results exactly
  reconstructs input. `base` strips trailing slashes and takes the final raw
  segment; empty input gives `.`, all-slash input gives `/`. It does not resolve
  dots. `dir` cleans the directory part from split. `extension` is the suffix
  starting at the last dot of the last *raw* segment, or empty; a trailing slash
  means the last raw segment is empty. A dotfile's leading dot counts.
- **P4 Root.** `is_absolute` tests only for a leading slash, without consulting
  a current directory or interpreting a drive name.
- **P5 Relative.** Clean both inputs. Different rootedness gives
  `Error(DifferentRoots)`. Remove their common segment prefix, treating `.` as
  no segments. If unmatched base segments include `..`, give
  `Error(UnknownParent)`; their actual names cannot be recovered lexically.
  Otherwise emit one `..` per unmatched base segment followed by the remaining
  target segments. Empty output gives `Ok(".")`. Both paths denote locations;
  there is no implicit "base is a file" convention.

These operations do not establish containment or resolve symlinks. For example
cleaning an apparent parent traversal cannot prove what a later filesystem call
will reach. URI paths have different rules and must not use this cleaner.

### 5.3 Laws, examples, and cost

Clean is idempotent. Split parts concatenate to the input. On a successful
relative result r, `clean(join([clean(from), r])) == clean(to)`. Do not claim
raw spelling preservation for cleaning or an inverse for join.

| Call | Expected | Rule |
|---|---|---|
| clean("") / clean("a//b/../") | `"."` / `"a"` | P1 |
| clean("/../../a") / clean("a/../../b") | `"/a"` / `"../b"` | P1 |
| clean("//server///x") | `"/server/x"` | P1 |
| clean("C:\\a\\b") / clean("%2e%2e/x") | Unchanged strings | P1 |
| join([]) / join(["", ""]) | `""` / `""` | P2 |
| join(["a", "/b", "..", "c"]) | `"a/c"` | P1/P2 |
| split("a//b/") | `("a//b/", "")` | P3 |
| base("a/../") / base("///") / base("") | `".."` / `"/"` / `"."` | P3 |
| dir("a/b/") / dir("a") | `"a/b"` / `"."` | P3 |
| extension(".profile") / extension("a.") / extension("a.txt/") | `".profile"` / `"."` / `""` | P3 |
| is_absolute("/a") / is_absolute("C:/a") | True / False | P4 |
| relative("/a/b", "/a/c") | `Ok("../c")` | P5 |
| relative("../a", "../b") / relative("..", "../a") | `Ok("../b")` / `Ok("a")` | P5 |
| relative("..", ".") / relative("a", "/a") | UnknownParent / DifferentRoots errors | P5 |

Costs: O(b) time/space in total input UTF-8 bytes (plus the list spine for join).
Convert each input to codepoints once, accumulate segments/fragments in reverse,
and join once. Count neither repeated string slicing nor growing-prefix `<>`
as O(1). Compare path segments over at most the total input size. For long tests,
use 100,000 segments, multibyte names, and long dot/parent runs. Check the laws
over all short strings from a small alphabet including `/`, `.`, `a`, `é`, `\\`.

## 6. `std/uri`

### 6.1 Types and complete exports

`Uri` is hidden. `Parts(..)`, `Authority(..)`, `Host(..)`, `UriPart(..)`,
`UriProblem(..)`, `UriError(..)`, and `CodecError(..)` are exported. Public parts
are editable candidate data; only parse/build produce a validated Uri.

```i
Uri { Uri(Parts) }
Parts {
  Parts(
    scheme: Option(String), authority: Option(Authority), path: String,
    query: Option(String), fragment: Option(String)
  )
}
Authority {
  Authority(userinfo: Option(String), host: Host, port: Option(String))
}
Host {
  RegName(String)
  IpLiteral(String)
}
UriPart {
  WholePart
  SchemePart
  AuthorityPart
  UserInfoPart
  HostPart
  PortPart
  PathPart
  QueryPart
  FragmentPart
}
UriProblem {
  BadCharacter
  BadPercent
  BadScheme
  BadAuthority
  BadHost
  BadPort
  BadPath
  RelativeBase
}
UriError { UriError(part: UriPart, offset: Int, problem: UriProblem) }
CodecError {
  BadEscape(offset: Int)
  NonAscii(offset: Int)
  InvalidUtf8
  UnalignedBits
}
```

```text
parse: (text: String) -> Result(Uri, UriError)
build: (parts: Parts) -> Result(Uri, UriError)
parts: (uri: Uri) -> Parts
to_string: (uri: Uri) -> String
has_scheme: (uri: Uri) -> Bool
resolve: (base: Uri, reference: Uri) -> Result(Uri, UriError)
encode_component: (text: String) -> String
encode_bytes: (bytes: BitArray) -> Result(String, CodecError)
decode_component: (text: String) -> Result(String, CodecError)
decode_bytes: (text: String) -> Result(BitArray, CodecError)
parse_query: (query: String) -> Result(List((String, String)), CodecError)
build_query: (pairs: List((String, String))) -> String
```

`Parts`/`Authority` use existing labelled constructor fields, not structural
record syntax. For example, modify a query with `Parts(..p, query: Some(q))`,
then call build. The constructor is known from its single-variant nominal type
(spec §6.7); no T3-only record pattern is involved.

### 6.2 Exact rules

- **U1 Grammar.** Parse accepts exactly ASCII URI references according to the
  RFC production named in decision 8, with no input repair or scheme whitelist.
  Empty reference, relative path, network-path reference and non-hierarchical
  scheme paths are included. Reject literal spaces/controls/non-ASCII, bad `%`
  triplets, invalid brackets/IP literals, and illegal component delimiters.
  Percent-encoded octets are syntax, not UTF-8 text to validate at parse time.
  This is generic syntax validation, not a DNS, port-range or scheme policy check.
- **U2 Storage.** Store all component text as encoded spelling, excluding outer
  separators: scheme without `:`, userinfo without `@`, port without `:`, query
  without `?`, fragment without `#`. Host IpLiteral stores its interior without
  brackets; RegName stores the other host spellings, including dotted decimal.
  Validate IP literal interiors using the RFC's IPv6/IPvFuture alternatives.
  No IPv6 zone extension. RegName syntax also permits numeric-looking names;
  do not reject such a name merely because it is not an IPv4 address.
- **U3 Presence.** `None` means no delimiter/component; `Some("")` means the
  delimiter was present with empty text. Path always exists as a String.
  Empty scheme is invalid; empty authority host and empty port are valid generic
  syntax. Port is digit text, with no Int conversion or 65535 limit. A `Uri`
  stores no redundant decoded or canonical form: `parse(s) == Ok(u)` implies
  `to_string(u) == s`, including case and hex spelling. has_scheme gives True
  exactly when the stored scheme is Some.
- **U4 Build.** Validate every raw field, then cross-field grammar constraints.
  Do not autoescape, repair, clean or normalize fields. An authority requires an
  empty or slash-leading path; without authority a path cannot start `//`;
  without scheme or authority its first segment cannot contain a raw colon.
  For valid p, `build(p) == Ok(u)` implies `parts(u) == p`. Build never silently
  reinterprets a path colon as a scheme or two slashes as an authority.
- **U5 Resolve.** Base must have a scheme, otherwise give
  `UriError(WholePart, 0, RelativeBase)`, even if reference has its own scheme.
  Use strict RFC §5.2 resolution, including §5.2.4 dot removal on the *encoded*
  path. Keep empty versus missing query distinctions. Never use `path.clean`,
  percent-decode dot/slash escapes, collapse repeated slashes, or retain a base
  fragment in place of a missing reference fragment. Then validate the computed
  Parts with the same validator as build. Return its Ok Uri or its Error value,
  using the offsets defined in U8. A base with a scheme does not guarantee Ok:
  dot removal can expose a leading `//` in a path whose authority is absent.
  Do not repair that candidate by escaping, reparsing or inventing an authority.
  Every successful result must still satisfy U4 and the raw round-trip law.
- **U6 Codec.** Encoders preserve only ASCII letters, digits, `-._~`; all other
  octets become upper-case `%HH`. Text encoding first takes UTF-8 bytes.
  encode_bytes rejects non-byte-aligned input with UnalignedBits, without padding.
  Decoders accept ASCII literal bytes plus `%HH`, replacing each triplet once.
  They accept either hex case, leave `+` unchanged, and reject raw non-ASCII.
  They do not enforce any component grammar on literal ASCII bytes.
  decode_component additionally validates UTF-8, giving InvalidUtf8 on failure.
  Neither decoder recursively decodes `%25` output.
- **U7 Forms.** parse_query takes raw query text without `?`, splits on `&`,
  skips empty pieces, splits each nonempty piece on its first `=`, and uses
  empty value when `=` is absent. Decode keys/values as UTF-8 with form `+` to
  space. Literal semicolon is data, not a separator or an error. Keep pair order
  and duplicates; stop on the first error, returning no partial result.
  build_query encodes UTF-8 keys/values with space as `+`, literal plus as `%2B`,
  otherwise the U6 alphabet, always writes `=`, joins with `&`, and does not sort.
  It may change flag spelling; raw Uri query round trips use U3 instead.
- **U8 Errors.** All public parse/build/resolve/codec failures are Result values.
  No parser API raises or asks for a service. UriError offsets count UTF-8 bytes:
  in parse, into the whole input; in build, into the field named by part. Resolve
  first reports its missing-scheme error at WholePart offset 0. Otherwise its
  validation errors use build's field-relative offsets in the computed Parts,
  not positions in either original input or the recomposed text. A leading `//`
  without authority gives `UriError(PathPart, 0, BadPath)`. EOF is the field/input
  length. Codec offsets are byte positions in that decoder's input; parse_query adjusts a key/value
  error back to the whole query. InvalidUtf8 and UnalignedBits have no offset.

Error selection is deterministic. Parse first scans the whole input left to
right for literal bytes outside the RFC URI alphabet and malformed percent
triplets; the first such failure is WholePart/BadCharacter or WholePart/BadPercent.
Then separate components by raw delimiters and validate scheme, authority, path,
query, fragment in that order. A colon before `/`, `?`, or `#` denotes a scheme
only when its prefix is a valid scheme; otherwise the no-scheme path check reports
BadPath at the colon. Authority splits at one raw `@` (a second is BadAuthority);
brackets delimit an IP literal; outside brackets, the one port colon is the port
delimiter, and any extra colon is BadHost. Validate userinfo, host, port in that
order. Missing closing bracket is BadHost at EOF of the authority. An illegal
port character is BadPort at that character. Query/fragment raw character-set
violations report BadCharacter at that byte. Cross-field build violations report
PathPart/BadPath at the first offending slash/colon; a non-slash path after an
authority reports offset 0. Lexical build scans run in scheme, userinfo, host,
port, path, query, fragment order (absent fields skipped), then per-field grammar
in that order, then cross-field checks. Component errors name that component;
an invalid IPv6/IPvFuture interior reports HostPart/BadHost at offset 0 of that
interior (translated to its first byte in parse). For all other grammar failures,
report the first byte violating the relevant production, or EOF when a required
part is missing: BadScheme for scheme, BadAuthority for userinfo/authority
structure, BadHost for reg-name/bracket syntax, BadPort for port, BadPath for
path. A byte after a closing host bracket must be a port colon or authority end;
otherwise it is BadHost at that byte. Empty scheme reports BadScheme at 0. This avoids inventing an IP
parser's internal diagnostic ordering. Resolve checks the base scheme before
transforming; after transformation it uses build's validation order and returns
the same error that build would give for those Parts. No error includes
credentials in a message.

### 6.3 Laws and tests

The laws are raw fidelity `parse(to_string(u)) == Ok(u)`, and preservation of
valid candidate parts through successful build followed by parts. The raw
round-trip law applies to every Uri made by parse, build or successful resolve.
For a base with a scheme, let p be the Parts computed by strict RFC resolution:
`resolve(base, reference) == build(p)`, including when build gives an Error.
Thus `resolve(base, reference) == Ok(u)` implies both `has_scheme(u) == True`
and `parse(to_string(u)) == Ok(u)`. There is no unconditional successful-resolution
law for scheme-bearing bases. No semantic URI equivalence is attached to `==`.
`decode_component(encode_component(s)) == Ok(s)` for all Strings. For whole bytes,
`encode_bytes(b) == Ok(encoded)` implies `decode_bytes(encoded) == Ok(b)`. `parse_query(build_query(pairs)) == Ok(pairs)`. The reverse form law is
false because spelling is lost. Use to_string after unwrapping successful parse
and resolution results in the following table.

| Test | Expected | Rule |
|---|---|---|
| Parse then print `""`, `"?"`, `"#"`, `"?#"`, `"//"`, `"//h:"` | Each original spelling | U1/U3 |
| Parse `"a:b"` / `"a/b:c"` | Scheme Some("a"), path "b" / scheme None, path "a/b:c" | U1/U2 |
| Parse-print `"HTTP://u:p@EXAMPLE:00080/a%2fb?x+#f"` | Exact original | U2/U3 |
| Parse `"/%FF"` then decode its path | Parse success; text decode InvalidUtf8 | U1/U6 |
| Parse `"http://[::1]:/"` / `"http://[v1.a:b]/"` | Both succeed, empty port preserved in first | U1/U2/U3 |
| Parse `"http://[::1"` | UriError(HostPart, 11, BadHost) | U8, authority EOF |
| Parse `"http://h:abc/"` | UriError(PortPart, 9, BadPort) | U8 |
| Parse `"a b"` / `"a%2"` | WholePart offset 1 BadCharacter / offset 1 BadPercent | U1/U8 |
| Parse `"1:x"` | PathPart offset 1 BadPath | U1/U8 |
| Build absent scheme/authority, path `"a:b"` | PathPart offset 1 BadPath | U4 |
| Build absent authority with path `"//h"` | PathPart offset 0 BadPath | U4 |
| Build authority RegName("h") with path `"x"` | PathPart offset 0 BadPath | U4 |
| Encode component `"é /+%"` | `"%C3%A9%20%2F%2B%25"` | U6 |
| Decode `"%252F+"` | `Ok("%2F+")` | U6 |
| Decode bytes `"%FF%00"` | `Ok(<<255, 0>>)` | U6 |
| Decode component `"%FF"` / encode bytes `<<1:u1>>` | InvalidUtf8 / UnalignedBits | U6 |
| parse_query("a+b=c%2Bd&k&k=&a=2") | Ok([("a b","c+d"),("k",""),("k",""),("a","2")]) | U7 |
| parse_query("&&x=1;2&&") | Ok([("x","1;2")]) | U7 |
| parse_query("x=1&y=%GG") | Error(BadEscape(6)) | U7/U8 |
| build_query([("b","2"),("a"," "),("b","+")]) | `"b=2&a=+&b=%2B"` | U7 |

Resolution fixtures use base `"http://a/b/c/d;p?q"`:

| Reference | Result | Rule |
|---|---|---|
| `"g"` | `"http://a/b/c/g"` | U5 |
| `"../g"` | `"http://a/b/g"` | U5 |
| `"?"` | `"http://a/b/c/d;p?"` | U3/U5 |
| `""` | `"http://a/b/c/d;p?q"` | U5 |
| `"#s"` | `"http://a/b/c/d;p?q#s"` | U5 |
| `"g//h"` | `"http://a/b/c/g//h"` | U5 |
| `"%2e%2e/g"` | `"http://a/b/c/%2e%2e/g"` | U5 |
| `"http:g"` | `"http:g"` | U5 strict, own scheme |

Resolution must also check candidates outside the authority-bearing fixtures:

| Base | Reference | Expected result | Rule |
|---|---|---|---|
| `"x:a"` | `"/.//h"` | Error(UriError(PathPart, 0, BadPath)); candidate path is `"//h"` with no authority | U4/U5/U8 |
| `"x:a"` | `"/a/..//h"` | Error(UriError(PathPart, 0, BadPath)) | U5/U8 |
| `"x:a"` | `"/.//a:b"` | Error(UriError(PathPart, 0, BadPath)); do not reparse it as a host/port | U4/U5/U8 |
| `"x:a"` | `"x:/.//h"` | Error(UriError(PathPart, 0, BadPath)), also when the reference has a scheme | U5/U8 |
| `"x://host/a"` | `"/.//h"` | Ok Uri printing `"x://host//h"`, retaining authority host | U4/U5 |
| `"a"` | `"/.//h"` | Error(UriError(WholePart, 0, RelativeBase)), before candidate validation | U5/U8 |

For every successful case, including all RFC fixtures and generated inputs,
check `parse(to_string(u)) == Ok(u)` on the resolved u. For the first four rows,
check that resolving returns the stated Error and never constructs an invalid
Uri or silently changes components.

Also run every RFC §5.4 normal and abnormal resolution example as a data table.
Test IPv6 full/compressed/mixed IPv4, duplicate compression markers, too many or
few groups, missing brackets, IPvFuture, forbidden zones, encoded delimiters,
multiple raw @ signs, missing versus empty userinfo/port, giant port strings,
empty/duplicate form keys, lowercase escapes, overlong/truncated UTF-8 and NUL.
Round-trip generated valid Parts and arbitrary short ASCII strings through
parse: success must print identically; failure must be a Result without stopping.

### 6.4 Implementation and cost

Scan original bytes once into a list or scan codepoints once with a byte counter.
Store slices/fragments for raw components, never recursively decode during
parsing. For scheme and host grammar use explicit ASCII predicates and small
finite state helpers. IPv6 validation can count colon groups and one optional
compression site, with an IPv4 tail counted as two groups; special-case empty
sides and forbid an empty single-colon group. Every branch must keep the full
RFC grammar; the group-count sketch is not a replacement for it.

Compose strings from fragment lists with one concat. For decoding, collect byte
fragments, concatenate once, then call the existing UTF-8 converter for the text
variant. Do not use a String to hold arbitrary decoded octets. Parse/build,
to_string, codecs, forms and resolution target O(b) work/space in total input and
output bytes, plus pair/list spines. Raw parts projection and has_scheme use O(1)
field work. Linear resolution needs a segment stack/output fragments for dot
removal; repeatedly copying a growing output string is quadratic. Validate the
finished resolution Parts before calling the hidden Uri constructor; that extra
linear pass does not change the bound. No host parser
or new string intrinsic is required. These bounds remain implementation gates.

## 7. Spec disposition, section by section

No replacement spec text is proposed. The coordinator declined the redundant
general clarifications after review. Keep existing section numbers and put these
module contracts in ordinary module documentation.

| Spec sections | Disposition and existing authority |
|---|---|
| §2.1 and §2.4 | Unchanged. Package/import rules and constructor visibility already make these ordinary package modules (`docs/spec.md:124–142`, `:203–206`). |
| §4.1–§4.8 and §6 | Unchanged, apart from the separately accepted syntax/type migrations. The stream prerequisite fixes the compiler against existing §4.7; it adds no inference or direct-value rule (`docs/spec.md:405–414`, `:590`). |
| §9.1–§9.6 | Unchanged. Stored computations are ordinary functions, with call-time environments, error propagation and scoped finalizers (`docs/spec.md:743`, `:779–784`, `:788–813`, `:828–860`). |
| §11.4 and §13 | Unchanged. Equality/hash already follow language rules independently of constructor visibility or a module's comparison function (`docs/spec.md:970–975`, `:1078`). |
| §12, Appendix B and all other sections | Unchanged. No built-in name, core operation, host service, package requirement or ABI entry is added. |

## 8. Files, writer boundaries, and implementation gates

Only this note is committed by this design task. Implementation is four
independent module writers after the new-syntax compiler is available. Path and
sequence may be implemented first and need not wait for the stream prerequisite.
URI may also proceed independently with the corrected U5/U8 contract. Stream
implementation starts only after the separate compiler track passes §3.4's
positive factory and negative service-compatibility gates. Each writer owns
one new `std/src/<name>.i` and one new `std/test/<name>_test.i`, from line 1 through
EOF. Module source regions, in order: imports; exact exports; public types;
factories/parsers; operations/builders; private helpers. Tests use `@test`,
assertions and pure local fake services like the existing tests. No module may
import a test module (spec §2.1). No writer edits string.i, list.i, dict.i, the
prelude, the compiler, or another writer's files. URI does not depend on path;
sequence does not depend on stream. Stream alone may import list for fold control.

No spec edit is assigned to Std-1. The coordinator assigns the compiler
conformance prerequisite to a separate compiler track; std writers do not patch
it or weaken the stream interface. No `std/i.toml` change is needed: source
discovery already walks source/test directories and accepts .i files
(`crates/ilang-package/src/modules.rs:22–31`, `:84–94`; spec §2.1). Any discovered compiler
defect is reported with a minimal ordinary-language witness, not patched in std.

### Path writer: ready independently

- [ ] Implement one-pass segment scanning and reversed-stack cleaning; build all
  outputs with fragment joins.
- [ ] Implement raw split/base/extension separately from clean and cleaned dir.
- [ ] Implement explicit-base relative, including unmatched parent errors.
- [ ] Add P1–P5 table/law tests, Unicode/backslash/NUL cases and long inputs.
  Expected IO validation stays outside this module.

### Sequence writer: ready independently

- [ ] Establish uniform rank/measure helpers and a private invariant checker.
- [ ] Implement symmetric end operations, bottom-up from_list and single-pass walks.
- [ ] Implement bounded bridging join and measured split; derive at/set/take/drop
  without list conversion. Inspect touched-node counts before claiming costs.
- [ ] Add Q1–Q4 tests, deterministic list models with branched versions, shared
  joins, empty/single/deep transitions, bounds, effects and nominal callbacks.
- [ ] Verify equal ignores shape; document raw equality and dict key behavior.
  Keep invariant checks private; run them inside the defining module if they
  need private constructors, using temporary development probes and removing
  probes before shipping. External std tests cover public observations.

### URI writer

- [ ] Implement raw representation and a single shared validator for parse/build.
  Parse must not be a permissive splitter; build must check cross-field grammar.
- [ ] Complete IP-literal grammar and deterministic U8 error translation.
- [ ] Implement raw printing and strict resolution over encoded paths. Validate
  every computed Parts before returning a Uri, using build's error order and
  field-relative offsets. Then add byte/text codecs and ordered form pairs.
- [ ] Add U1–U8 tests, the full resolution tables, generated raw round trips and
  invalid grammar/UTF-8 cases. Include authority-free resolution failures and
  raw round trips for every successful resolution. Form tests must not stand in
  for generic URI tests.
- [ ] Check long authority/port/query/path inputs without repeated string copies.
  HTTP policy, IDNA, URI templates and scheme-specific validation remain separate.

### Stream writer: after the compiler prerequisite

- [ ] Confirm that the separate compiler track passes §3.4's factory probe and
  still rejects unsafe service coercion on the nominated implementation compiler.
  Do not replace the generic factory, remove N invariance, or add a library rule.
- [ ] Add the exact opaque wrapper, Step and signatures; implement make/pull first.
- [ ] Add list/unfold factories and demand-preserving transformations. Keep outer,
  inner and callback error variables distinct until their output unions. Make
  service N invariant and share one N across composition; implement the explicit
  environment runner without a higher-rank signature.
- [ ] Implement skipping/empty-inner searches with tail-recursive state loops.
- [ ] Add S1–S8 tests, long filtering/collection, late services, distinct error
  types, nominal field inference, and new() used independently at two types.
- [ ] Check inferred exported interfaces; verify no construction needs services.
  Put compile-fail client probes in temporary packages, not failing std tests.

For each writer, use the coordinator's prebuilt migrated compiler:
`i check --project std --types`, then `i test --project std`, then the changed
module tests at `--opt 2`. Check all public labels and types against this note,
including negative clients and common environments with multiple services. CLI
options are defined at `crates/ilang-cli/src/main.rs:20–32`, `:68–80`; their help
was also checked with the cached baseline executable. Later integration repeats the
full std suite after T1's known-type changes. No new-module result is accepted
solely because the old std suite passes. Performance measurements belong to the
coordinator's quiet window; node-visit/fragment-growth scripts are sufficient
writer evidence. No cargo build is needed for these plain-i modules.

## 9. Self-attack and repairs

These are concrete new-syntax counterexample clients or private implementation
witnesses. They were evaluated against source rules by reading, not run against
nonexistent modules. Names use qualified modules unless imported explicitly.

### Pass 1: confirmed design defects, repaired above

**Inference/buildability: nested recursive port.** The tempting representation
and helper are:

```i
BadTree(A) { BadEmpty BadDeep(List(A), BadTree(BadNode(A))) }
BadNode(A) { BadNode(A, A) }
depth(A) = (tree: BadTree(A)) -> Int {
  tree |> {
    BadEmpty -> 0
    BadDeep(_, middle) -> 1 + depth(middle)
  }
}
```

The recursive call wants a different instantiation of depth inside its group.
Spec §4.7 and infer.rs's monomorphic holders forbid relying on that. Repair:
uniform Node/Tree and runtime rank invariants, with one A throughout. The public
type and logarithmic middle search do not require polymorphic recursion.

**Soundness/interface: losing latent effects.** This client must fail to type:

```i
(std/stream, i/fail)
bad = () -> List(Int) {
  s = stream.make(() { fail.raise("lost") })
  stream.to_list(s)
}
```

A proposed `Stream(A)` with an untyped/erased thunk or pure `pull` would accept
an unsound advertised interface. Repair: keep E/N in Stream, Step and every tail;
pull/consumers expose them, transformations union errors only in outputs and
share the invariant service environment. Expected failure is a compile error at bad's pure annotation. Likewise a missing service
cannot be erased by an intermediate combinator.

**Soundness/source conflict: service variance hidden in a nominal wrapper.**

```i
Sink(A) { Sink(put: (A) -> Nil) }
PlainStream(E, N) { PlainStream(() -> Nil ^ E ! N) }
need_int = (s ! Sink(Int)) { s.put(1) }
coerce = (source: PlainStream(Never, Sink(Int))) -> PlainStream(Never, Sink(Never)) {
  source
}
unsafe = () {
  ! Sink(Never) = Sink((x: Never) -> Nil { x |> {} })
  PlainStream(run) = coerce(PlainStream(need_int))
  run()
}
```

This proposed wrapper would route the integer 1 to a function whose parameter has no
values. In the final design the analogous coercion must be rejected:

```i
bad_coerce = (source: stream.Stream(Int, Never, Sink(Int))) -> stream.Stream(Int, Never, Sink(Never)) {
  source
}
```

A plain thunk wrapper makes N covariant under the existing variance walk
(`infer.rs:2237–2266`, `:2321–2329`). Ordinary value subtyping then permits a
wrapper with N=Sink(Int) to widen to N=Sink(Never), although function-environment
compatibility must forbid this. Repair: the unused negative N field makes
Stream/Step invariant in N. Checked the next obstruction too: writing N|M in
that invariant argument is rejected by `check_places` (`infer.rs:600–655`). Thus
use a shared N and the explicit map_environment runner, not an illegal union or
a claimed compiler fix. The API keeps new/list factories polymorphic in N so
empty tails can be built once §3.4's compiler prerequisite is fixed. A compile-fail coercion probe is a required implementation
gate; concrete common-service clients and runner-provided services are positive
gates. This may also merit a separate language review of covariant custom types
that hide environment parameters. The separate factory-generalization fault
found in independent review is the explicit prerequisite in §3.4; the initial
reading pass did not establish factory buildability.

**Behavior: overpull through take.**

```i
(std/stream, i/fail)
example = () {
  tail = stream.make(() { fail.raise("too far") })
  stream.to_list(stream.take(stream.append(stream.wrap(1), tail), 1))
}
```

If taking the last allowed item pulled its tail to discover whether it ends,
this would error. Repair: after the kth item return a fresh end stream without
another input pull (S4). Expected `[1]`. `take(0)` also cannot call a thunk.

**Lifetime: a user value can hold a suspended finalizer.**

```i
(std/stream, i/control, i/resource)
abandon = () {
  _ = control.boundary((b) {
    resource.ensure(
      () {
        control.suspend(b, (k) { stream.wrap(() { control.cancel(k) }) })
        stream.new()
      },
      () { panic("finalizer") }
    )
  })
  Nil
}
```

The handler returns a stream holding a cancellation closure; discarding that
stream does not call the closure. Spec §9.4 rule 6 says an unused continuation's
final functions are not run. Repair: S8 promises no automatic cancellation or
finalization, even though stream functions themselves never capture continuations.
The external scope or continuation owner must handle this. Do not infer lifetime
safety merely from the absence of control calls inside the stream implementation.

**Source/cost conflict: pretending delayed calls memoize.**

```i
(std/list, std/sequence)
branches = (q: sequence.Sequence(Int)) {
  list.map(list.range(1, 1000), (x) { sequence.push_back(q, x) })
}
```

On a full fringe every branch may repeat an O(log n) carry. A plain thunk does
not share its computed result; claiming amortized O(1) for this history is
unjustified. Repair: strict eager tree, O(log n) persistent worst-case end edits,
no imported Haskell laziness claim. The coordinator accepted this cost.

**Soundness of validation and fidelity: delimiter injection.**

```i
candidate = uri.Parts(None, None, "a:b", None, None)
```

An unchecked public Uri constructor, or print-then-parse builder, would change
the intended absent scheme into Some("a"). Repair: hidden Uri, public candidate
Parts, explicit cross-field checks. Build returns BadPath; parse("a:b") remains
valid and denotes the scheme. The same attack uses `"//h"` for authority.

**Wrong Unicode model and decoded-path cleanup.**

```i
encoded = uri.decode_bytes("%FF")
escaped_parent = uri.parse("%2e%2e/g")
```

Using String for all decoded bytes rejects valid generic syntax; decoding before
resolution turns data into a parent traversal. Repair: byte codec + fallible text
codec, raw encoded storage, RFC dot removal on raw path. Expected Ok(<<255>>) and
a relative path whose encoded parent segment survives resolution (U5 table).

**Equality conflict with spec.**

```i
a = sequence.from_list([1, 2, 3])
b = sequence.append(sequence.from_list([1]), sequence.from_list([2, 3]))
```

Representation equality is not a list model law. Repair: explicit equal, with
tests against to_list, and a warning about dict keys. No special comparator/hash.

### Pass 2: no further serious issue found after repairs

Rechecked soundness with `stream.zip(stream.new(), failing)` and the reversed
arguments: S5 now deliberately distinguishes effect order. Checked
`stream.attempt(stream.from_results([Error("x")]))`: S7 gives exactly one error
element and an end, without retrying. Checked two distinct services in a written common N and two errors in
map/flat_map: only errors union in positive output positions. The N marker stops
coercions through a contravariant service; map_environment rebuilds every tail
with M rather than returning a tail still typed with N. `State(Int)` and
`State(String)` requirements still fail together under spec §9.2. Checked new()
bound once at two types: ordinary value restriction still applies; independent
factory calls or an eta-expanded function are the supported polymorphic forms.

Rechecked buildability with middle push/borrow at rank r+1: all helpers still use
Tree(A)/Node(A); no changing type arguments or inferred recursive type variable.
Checked map from Sequence(Int) to Sequence(String): paired A/B remain fixed
through its node/tree helpers. Checked UTF-8 failure via bit_array.to_string and
fragment joining: both already have the required operations (source table).

Rechecked conflicts using `path.relative("..", ".")` (UnknownParent),
`path.base("a/../")` (`".."`, raw), URI empty query inheritance, a fragment on the
base only, repeated path slashes, an empty port and numeric reg-name. The
contracts give distinct and consistent answers. No source rule needs changing
to implement these cases. This is a reading/derivation pass, not a compiler run.

### Pass 3: second clean pass

Tried malformed public Parts with non-ASCII scheme, percent in port, raw # in
query, slashless path after authority and unmatched brackets. Shared validation
must reject each as a value. Tried `%252e%252e`, `%2F`, overlong UTF-8 and
one-bit encode_bytes: no double decode, structural delimiter change or silent
padding remains. Tried form pairs `[("", ""), ("a", ""), ("a", "+")]`:
build writes `"=&a=&a=%2B"`, which parses back exactly to the pairs.

For sequence, tried self-append, splitting at 0/length, borrowing from an empty
middle, and retained ancestors with shared subtrees: the node types, rank/count
invariants and stated worst-case bounds accommodate each. For streams, tried a
million rejected items, a million empty inner streams, a service installed only
at pull, and abandoning a stream after take: explicit tail-recursive searches
cover skipping, while caller-owned resource scopes and continuation owners are
required for cleanup. The abandon witness has no library finalization guarantee. A deep composition chain remains
a documented stack risk. No further serious design issue found in this second
clean reading pass.

### Independent review follow-up

The original clean reading passes did not establish full buildability or cover
authority-free resolution. The independent review found two real omissions.
Both were checked against sources and probes before this revision:

1. **Generic invariant factory:** independently reran the review's baseline-syntax
   factory and unsafe coercion probes with the cached compiler. The factory loses
   N and its Bias client fails; the unsafe service coercion remains rejected.
   Re-read display.rs and generalize as cited in §3.4. Repair: retain the API and
   service rules, and gate the stream writer on a general compiler conformance fix.
2. **Resolution can leave Uri's value set:** with base `"x:a"`, references
   `"/.//h"` and `"/.//a:b"` transform to absent-authority paths `"//h"` and
   `"//a:b"`. Re-read RFC §3.3/§5.2 and executed a separate dot-removal model.
   Repair: U5 validates the computed Parts; U8 reports PathPart/BadPath at offset
   0 of the candidate path. No invalid Uri is constructed, so the round-trip law
   stays true for every Uri the module can make.

Two further reading passes checked the revised boundary. The first checked the
positive Bias factory gate alongside the negative Sink coercion gate, and both
resolution counterexamples against the shared validator. The second checked
`"/a/..//h"`, an own-scheme reference `"x:/.//h"`, the valid authority-bearing
control `"x://host/a"` with `"/.//h"`, and missing-base-scheme error precedence.
It also checked that path/sequence have no dependency on the factory gate and
that no spec addition or weaker service rule remains. No further serious issue
was found in these two passes. The compiler prerequisite is still open; these
checks do not claim its fix or an execution of std/uri.

## 10. Risks, coordinator decisions, and verification limits

Main implementation risks are the finger tree's private rank/count invariants,
strict linear string construction, complete IP-literal validation, correct
demand after early stop, and the new known-type rules for stateful callbacks.
The relevant regression gates are explicit above. A callback with control effects
may suspend because it is an ordinary call; the stream library itself never
creates one-use continuations. It cannot make an arbitrary user-supplied thunk
replay-safe or guarantee resource validity after its external scope ends.

Coordinator decisions for this revision:

- Accept logarithmic persistent sequence end edits and constant-time end views.
- Keep strict ASCII RFC 3986 syntax initially, with scheme policy, IRI/IDNA and
  browser repair outside this module.
- Keep shared invariant stream environments and the rank-one environment adapter.
  Fix the compiler's generic factory inference in a separate track.
- Add no general spec clarifications: the current spec already defines these
  consequences. There is no outstanding request to edit it for Std-1.
- Validate resolution candidates and return defined build-style errors when they
  cannot be a Uri. Path and sequence may proceed before the stream compiler fix.

No API question from the original recommendation list remains open. Stream's
implementation prerequisite is a tracked compiler dependency, not a language
change or a reason to relax service compatibility.

Checked here: source baseline and branch, cited std/compiler regions, accepted
syntax and scoped type notes, plan/ledger decisions, reference API documents,
the initial manual counterexample passes, and the independent-review corrections
with two further clean reading passes. No new modules exist yet, so their compile/run behavior, exported inferred signatures, Wasm stack use,
node-count bounds and performance remain unverified. Executed separate Python
models of P1/P5 and U6/U7: clean idempotence on 130 paths; 8,734 successful
relative/join laws; 144 ordered form-pair round trips including empty keys, plus,
Unicode and NUL; and round-trip encoding all 256 byte values. These check the
stated laws independently, not an i implementation. A mechanical inventory
check found 23 stream, 23 sequence, 8 path and 12 URI function signatures with
all type variables declared, plus three private sequence helper signatures;
it also found no old keyword syntax in i blocks. For this revision, independently
reran the review's factory and unsafe coercion probes on `i-1e0d910`; both fail
as documented in §3.4. An independent RFC model confirmed four invalid transformed
paths (`"/.//h"`, `"/a/..//h"`, `"/.//a:b"`, `"x:/.//h"` against `"x:a"`)
and two standard dot-removal examples. These are baseline compiler failures and
model results, not passing std implementations. Do not report the design
witnesses or proposed test tables as passing implementation tests. The cached
baseline compiler predates the accepted syntax migration; full new-syntax
verification belongs to the four implementation writers and integration.
