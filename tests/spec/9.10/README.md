# Pure random acceptance

The implementation is `lib/i/random.i`. It uses ordinary i types, integer
operations and error handling. There are no host declarations, services, task
imports or compiler primitives. Rust implementation files are unchanged; the
authorized Rust test expectations now recognize the implemented module.

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
top-level calls. The compiler's library-type golden records those six public
schemes and the 17 private implementation schemes.

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

## Coordinator follow-up

The coordinator authorized the test expectations and spec reservation updates.
The opening now lists seven implemented modules and reserves only task, clock,
fs, process, net and stdio as empty. The count in §2.1 is updated consistently.
The multicore.md random text is reconciled by the upstream native/multicore
merge 681ad65. Its merge leaves the complete R1 section of runtime.md unchanged.

Every changed Rust expectation has a source:

| Expectation | Change and reason |
|---|---|
| reserved_modules_are_placeholders | Check the six remaining reserved paths. Random now has the API required by runtime.md §4 and spec §9.10. |
| reserved_modules_import_cleanly | Import those six reserved paths; random imports and pure calls are covered by interface.i. |
| reserved_modules_have_no_names | Check missing names in those same six empty modules. Random's real public names belong to its API inventory. |
| library_names | Replace random's empty list with from_seed, word, int, float, split and at_path: exactly the §4/§9.10 table. |
| Binding-kind checks | Require all six random functions to be ordinary bindings: the assignment says plain i and no new primitive. |
| types/library.out | Append 23 inferred schemes; no existing scheme changes. The six public schemes are required by the API table above. The 17 private schemes describe the implementation below. |

The private golden additions and their reasons are:

- word_size, mask and xor_shift are pure Int operations for unsigned 64-bit
  arithmetic (runtime.md §4.1).
- mix, variant, popcount and mix_gamma are pure Int operations implementing M,
  V and G in the same numerical contract.
- space_for computes the least allowed power of W as an Int; word_group and
  bounded return an Int and Generator, threading every accepted/rejected draw
  (runtime.md §4.2).
- native_split returns two States and edge projects one State, as required by
  the published split and fork definitions.
- valid_path returns Bool; binary returns List(Int) and Int; zeros and follow
  return State; derive_path returns Generator. They implement validation,
  Elias gamma navigation and tagged component-wise history from §4.2.

Private golden entries record implementation schemes; they do not independently
prove the numerical requirements. The vectors and laws provide that evidence.

The golden is generated only with UPDATE=1 cargo test -p ilang-check --test
types --locked library. No old expectation was removed to conceal a failure.

## WAT acceptance and cost

The coordinator replaced byte equality with renumbering-only acceptance.
The compiler's enumeration is unchanged. The audit command is:

```sh
python3 tests/spec/9.10/wat_audit.py target/r1/wat-base target/r1/wat-final
```

The script invokes the unchanged scripts/wat-diff compare function on each
baseline export, using its reference normalization and no allowed exceptions.
The CLI rejects --renumber without an A1 may-fail migration report; no false
report was fabricated. The audit separately classifies nominal IDs using the
actual number_types order: constructor IDs at least 17 move by six, and program
type IDs at least 22 move by four. String data references and nullary global
names move consistently. A diagnostic removes only the individually verified
new entries/data to check for any other code change. This is a substitute
classification check, not a passing acceptance comparison.

All 383 previously successful exports were assembled and validated as Wasm;
the 219 baseline ERROR exports remain byte-identical. Thus there is no new
compilation failure. The normalizing wat-diff comparison reports 219 equal
ERROR exports, 328 differences, and 55 unsupported site forms. The latter are
pre-existing `(call $shadow_leave_tail (i32.const -1))` forms: the same
normalizer fails on the baseline alone in all 55 cases. Default wat-diff
comparison on those 55 also reports differences. No comparator was changed.

The size-independent raw audit finds six new constructor table entries in
every successful export. Existing string IDs and nullary global names are
renumbered along with nominal operands. One export, 9_environment.i.ilang.true,
also moves its existing State string definition earlier; its bytes are
unchanged, and wat-diff's established data-order normalization accounts for
that relocation. All additions remain failures of renumbering-only acceptance.

Across all 602 exports, the raw audit classifies 71 exports with nominal ID
references changed, six new table entries in each of the 383 successful
exports, 2,297 added string definitions containing 17,613 bytes in total, and
one relocation of an existing State data definition. After individually
checking those additions and the established reference/data-order
normalization, the diagnostic finds no other change in all 383 successful
exports. The unchanged ERROR files supply the other 219 comparisons. These
results do not turn the real additions into renumbering; acceptance still fails.

The exact per-export report is target/r1/wat-audit/results.json. The separate
baseline sentinel control is site-normalizer-check.json in that directory.
Validation of all successful exports is also reproducible without the longer
comparison pass:

```sh
python3 tests/spec/9.10/wat_audit.py target/r1/wat-base target/r1/wat-final --validate-only
```

The six extra installed names are Split, Path, State, Generator, InvalidBound
and InvalidPath. New constructor table entries and their startup work are not
renumbering and are reported as failures. Each of the two measured examples
installs six additional strings containing 46 payload bytes.

These static measurements assemble the original unoptimized WAT with
wasm-tools parse, run wasm-tools strip, and validate both resulting binaries.
They measure binary size, not execution latency or optimized CLI artifacts.

| Existing export | Before bytes | After bytes | Increase | Installed names before/after |
|---|---:|---:|---:|---:|
| examples_hello.ilang.false.wat | 47,011 | 47,159 | 148 | 2 / 8 |
| 11_float_arithmetic.i.ilang.false.wat | 46,513 | 46,662 | 149 | 1 / 7 |

Both WAT files grow by 716 bytes. A later compiler track should remove the
unneeded constructor table initialization and stabilize numbering against
unused library types. The measured size and extra initialization exist even
when the program never uses random. Stable IDs alone would not remove that
startup work. This track changes no compiler enumeration and does not mask
these additions to obtain a passing result.
