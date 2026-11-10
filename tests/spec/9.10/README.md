# Pure random acceptance

The implementation is `lib/i/random.i`. It uses ordinary i types, integer
operations and error handling. There are no host declarations, services, task
imports, compiler primitives or Rust changes.

## Interpretation and sources

The controlling API is runtime.md §4 and the assigned spec replacement in
§10, now docs/spec.md §9.10. The following source sentences determine the
implementation and test expectations:

- “Generator's constructor is private.” Export `Generator` without its
  constructor; constructor_hidden.i expects E0204 under spec §2.4. Export
  `RandomError` with both constructors, as the next paragraph requires.
- “Its values contain a normalized seed, SplitMix state, tagged derivation
  history, and nonnegative draw index; equality is ordinary structural
  equality.” The private value stores all four. The history is a persistent
  list, newest first; each entry retains its P/S tag and previous draw index.
  Reversing storage order changes neither identity nor composition.
- “from_seed normalizes an arbitrary Int by Euclidean remainder modulo W.”
  The mask 2^64-1 gives that remainder under core's infinite two's complement
  bit operations (spec §13). Negative and oversized seeds are covered.
- “Pinning a version matters: the two mixers' roles differ between this variant
  and Java's.” M is the Murmur finalizer; G uses Stafford Mix13. All shifts,
  multipliers, the golden increment, odd-bit operation, transition threshold
  24 and alternating repair mask come from runtime.md §4.1 and the pinned
  [Haskell source](https://hackage-content.haskell.org/package/splitmix-0.1.3.1/docs/src/System.Random.SplitMix.html),
  functions mkSMGen, nextWord64, splitSMGen, mix64, mix64variant13 and mixGamma.
- “Each fork uses two published splits and introduces no mixer or key
  derivation.” Each edge projects the specified fork. For bit 0 the second
  unused split is not evaluated; pure projection gives exactly the same child.
  S(1)=00, S(2)=01 and P(i)=1 followed by Elias gamma(i), from §4.2.
- “Consecutive path components are handled identically whether supplied by one
  at_path call or by several.” History is updated once per component, resetting
  the index, so composition retains full structural equality.
- “Invalid arguments raise the shown error without a draw.” The full input
  path is validated before navigation and InvalidPath contains that whole path.
  InvalidBound is checked before choosing or drawing a group. Reusing an
  immutable input alone cannot detect hidden work; validation order was also
  checked directly in the implementation.
- “Draw k words in least-significant-first order to form u” and “Reject
  u >= limit and draw another group.” Tests check both results and the complete
  returned generator after actual one-word and multiword rejection. The least
  k is at least one, including bound 1, and supports arbitrary Int bounds.
- “float draws one word u and returns floor(u/2^11) / 2^53 as a Float.” Tests
  check the high 53 bits and the complete next generator. The denominator is
  exactly 9007199254740992, not a sampled expectation.

`reference.py` computes vectors independently with Python integers, string bit
addresses and tuple states, using no output from i. The first three words for
seed 1337 and initial state for seed 42 are published doctests in the pinned
source. Every other word, derived stream and bounded vector is computed from
the source's operations and §4.2's adapter. Seed -0x9e3779b97f4a7c15 forces the
one-transition repair branch; seed 12 has 23 transitions, immediately below the
threshold. The 201-bit path input checks that indices are not truncated.

The reduced-word exhaustive partition check tests the rejection arithmetic:
each accepted residue has exactly limit/n preimages, as required by §4.2's
“avoids modulo bias under the word model.” The small-bound frequency test is a
diagnostic smoke check, not a normative finite-sample independence law. Its
expected center is 12000/3=4000 under that model; the loose ten-percent band is
a test tolerance, not a language guarantee. Distinct stream inequalities test
history identity, never a promise of disjoint output words.

## Inferred public interface

`i check --types` on unannotated aliases gives the following schemes. They
match every row of runtime.md §4, including labels and error types. Module
qualification in the display is the only spelling difference. There is no
service effect in any of them.

| Binding | Inferred scheme |
|---|---|
| from_seed | `(seed: Int) -> random.Generator` |
| word | `(g: random.Generator) -> (Int, random.Generator)` |
| int | `(g: random.Generator, below: Int) -> (Int, random.Generator) ? random.RandomError` |
| float | `(g: random.Generator) -> (Float, random.Generator)` |
| split | `(g: random.Generator) -> (random.Generator, random.Generator)` |
| at_path | `(g: random.Generator, path: List(Int)) -> random.Generator ? random.RandomError` |

interface.i keeps the aliases unannotated and exercises their labels and pure
top-level calls. The compiler's library-type test also prints the actual schemes
on its golden mismatch.

## Reproduction

Run from lane b. All commands require the repository dev shell. Cargo and
verification commands require the heavy wrapper specified in the lane brief.

Generate vectors, then format with the built compiler:

```sh
UPDATE=1 python3 tests/spec/9.10/reference.py
target/release/i fmt tests/spec/9.10/vectors.i
python3 tests/spec/9.10/reference.py
```

The ordinary spec runner executes these cases at both O0 and O2:

```sh
cargo test -p ilang-cli --test spec --locked
```

Cross-target assertion replay, including the same generated vectors:

```sh
python3 tests/spec/9.10/replay.py
```

This builds ignored target/r1/replay packages, runs their entry points on the
Rust host and Node SDK, and runs their WASI artifacts under stock Wasmtime and
Node WASI, all at O0 and O2. The Node SDK creates two fresh instances per build.

## Independent review

The reviewer derived criteria before reading the implementation and accepted
the pinned variant and traversal, with no correctness defect found. Its
independent Python/C models agreed on 98,304 words; its Python model also
matched reference.py on 3,900 words, 273 paths and 312 bounded outputs with
exact returned state/draw counts. It checked 10,003 prefix-free codes,
4,356 two-step addresses and 1,020 reduced-word rejection partitions.

PractRand 0.96 passed its self-test. Six 256 MiB batteries and three 1 GiB
controls reported no anomalies: ordinary roots, repaired weak seeds, 64
interleaved P streams, depth-256 S streams and repeated splitting. Four
65,536-sample bounded distribution diagnostics were also unexceptional.
These statistical measurements use an independent C model, a substitute for
bulk i statistical output. Real i equivalence is checked by the O0/O2 vectors
and cross-target replay. Finite tests do not prove statistical independence,
cover all weak gamma cases, or make SplitMix cryptographic.

The reproducible reviewer sources, exact commands, primary source downloads and
logs remain in the ignored target/r1-review directory. The report is
`target/r1-review/report.md`. Initial fetches returned 404 for a guessed
PractRand version and 502 for Oracle's 2017 follow-up. Official PractRand 0.96
and the pinned Haskell source were available. An initial reviewer C indentation
warning was corrected before its -Werror build; upstream PractRand warnings
were retained and its self-test passed. No system package was installed.

## Scope boundaries and acceptance blockers

The current spec opening still reserves i/random as empty, while the accepted
runtime note assigns its implementation and new §9.10 to R1. The opening's
module-list edits belong to IO under runtime.md §11; they remain an escalation.
Similarly, runtime.md's instruction to replace multicore.md §5's old PRF sketch
is outside R1's assigned paths. The implemented numerical contract follows
runtime.md §4, not that superseded sketch.

The full verification gate encounters stale assertions in
crates/ilang-check/tests/library.rs: reserved_modules_are_placeholders and
library_names still require i/random to be empty. The library-type golden
crates/ilang-check/tests/types/library.out also needs regeneration. These paths
are outside R1 ownership and were not edited or blessed. No check was weakened.

Existing WAT is not byte-equal after a real library introduces nominal types.
Lower::number_types in crates/ilang-wasm/src/lower.rs enumerates all program
types, installs every constructor name, and assigns numeric type/constructor
IDs before emission. Adding even unused Generator/RandomError types therefore
changes old constructor tables and IDs. Removing eager constant initialization
within R1 avoids unrelated new start-up work, but cannot fix that compiler
policy. Constructor reachability/numbering needs an outside-scope compiler
change, or the coordinator must revise the byte-equality acceptance rule.
