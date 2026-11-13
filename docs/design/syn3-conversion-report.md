# Syntax three: the conversion of the repository (stage 8, lane/conv)

Status report of the whole-repository conversion to syntax three
(`docs/design/syntax-3.md` §7.2 stage 8, §7.3). It is kept up to date while the
lane runs; the lists it names are the authority.

## How the tree is converted

- `scripts/syn3-convert-all` converts every tracked `.i` and `.is` file with
  `migrate-syntax3 convert`, then the i snippets in Rust and JavaScript strings
  and Markdown fences (`scripts/syn3-convert-embedded`), folds the staged
  syntax-three text of the spec (`scripts/syn3-spec-fold`), and runs the hand
  rewrites (`scripts/syn3-hand-rewrites`).
- A run on a newer tree reads only the `.i`/`.is` files changed since the last
  conversion commit, and only the strings and fences whose text was not in the
  tree of that commit (`SYN3_EMBEDDED_ALL=1` reads all). This settles the texts
  that both syntaxes read to different trees: `T = (A)` is an alias before and
  a labelled type after. Before this rule (fixed in 7405234b) the re-run
  9fd733b4 read hand-written `Box(A) = (A)` snippets again as the old syntax
  and turned them into names `Box(A) = { (A) }`; those were restored.
- The syntax-three parser is the default; the old parser stays behind the
  cargo feature `legacy` of `ilang-syntax`, for the migrators only.

## What is not converted, or converted with a loss

- `scripts/syn3-unconvertible.txt`: files and snippets that neither syntax
  reads (parse-error fixtures of the old syntax). Left as they are.
- `scripts/syn3-lossy.txt`: files the migrator converts with a loss it
  reports. `syn3-convert-all` fails on any other file with such a note. Kinds:
  - `open-record`, `string-label`: open records and label parameters
    (`lib/i/record.i`, the record fixtures); removed by stage 6.
  - `two-row-variables`, `need-not-named`, `row-difference`: need rows that
    the syntax-three `<-` list cannot write.
  - `type-params`: type parameters of a binding whose value is not a function
    are dropped (§2.16 has them only on function literals).
  - `constructor-name-clash`: a constructor named like its type that has
    other constructors; one labelled type per constructor needs a rename by
    hand.
  - `labelled-call`: a labelled call of a function whose parameters the text
    does not show; the stage 4 migrator (`labels`) makes it positional.
  - `empty-match`: `x |> {}` becomes `core.never(x)` (§2.13); the module must
    import `i/core`.
- `tests/spec/syn3-pending.txt`: retained legacy dynamic-label cases whose
  intent still needs a closed-data rewrite after stage 6; each has a written
  reason. Newly valid inspect and spread-order cases are unparked.
- `crates/ilang-check/tests/syn3-pending.txt`: checker tests that use a form a
  later stage removes, named with that stage; each returns early and says so.
- Spec fences that need forms of later stages (shape groups, stage 9;
  selection, stage 11; `..env`, E6) keep the staged `i future-SYN3` form with an
  inventory entry until that stage lands.

## Behaviour changes

1. **A function literal's `-> R` no longer closes its needs.** Syntax three
   writes needs only after `<-`, and the whole need is inferred (§2.10.1,
   "Type: the whole need, inferred"). So `f = () -> Int { s |> ("a" | "c") { 1 } }`
   with `s = "b"`, which was E0501 (`Assert` not allowed: the old `-> Int`
   with no `!` allowed no needs), now compiles: the one-branch group is fitted
   and "when it may fail it reports to `Assert`" (spec §6.12), so `f` needs
   `Assert`.
   The check that remains is the warning: no value of the known type `"b"`
   matches `"a" | "c"`, and "a branch that can never be used gives a compile
   warning" (spec §6.12; syntax-3 §2.12, D4). W0505 was given only for
   constructor patterns (at fitting); it is now also given for every fitted
   pattern whose failure is settled (one-branch matches and bindings, literal
   patterns included) when no value of the known type matches it
   (`Checker::warn_never_matches`, `types/infer.rs`). Test:
   `literal_types::fitted_choices_keep_the_deferred_binding_error`.
   In short: an error became a warning plus an `Assert` need.
2. **Still to come: sums are unions, not families of constructors** (§2.8).
   The lowering of this stage still reads `Event(A) = Start(A) | Stop(A)` with
   its members just before it as one type with two constructors (today's tree),
   so facts "across every constructor of the family" (literal-types §5.3) are
   unchanged, and a member of such a union written as a type elsewhere
   (`f = (x: Start(Int))`) is not defined (E0202). Lane/fa's final tree makes
   each member its own labelled type. Test that will change then:
   `types::pattern::shared_place_tests`.
3. **`()` as a value is the empty data** of type `()` (§7.1 row 7): `f = (()) { 1 }`
   is called as `f(())` (fixture `tests/spec/7/syn3_empty_data_argument.i`).
   The Lisp source surface writes this as `(,)` (§14.3); `()` has no source
   position. The transitional AST adapter also reads its internal `()`
   encoding as an empty tuple so its old printer can round-trip this tree.
4. **Lowercase type parameters** (`Codec(d, e) = …`): the old compiler read
   the head and rejected it (E0232); syntax three rejects it in the parser
   (E0101, §3.2), so the rest of such a file is no longer checked. The
   converter notes them (`lowercase-type-param`, a loss), and the five review
   cases that are error fixtures record E0101.
5. **`fail: Fail(E)` in a function head** is the function's error part, as
   `Fail(E)` in a need of a function type: the head has no service `Fail`
   (`Cx::params_of`, `ret`; patch from lane/cG2).

## Defects found and fixed during the conversion

- The lowering read `T(N) = (N)` followed by `Alias(N) = T(N)` as the old
  one-constructor sum `Alias(N) { T(N) }` (the form the converter writes for
  `TimeError { DivisionByZero }`), so `T` stopped being a type (E0202 on later
  uses). A one-member right side is now a one-constructor type only when its
  member is not written as a type elsewhere (`syn3/lower.rs`; test
  `syn3_lower::one_member_alias_keeps_a_type_written_as_a_type`).
- The legacy printer refused the empty tuple that `()` now lowers to.

## Recovery after the 11:08 reboot (2026-11-13)

- Reviewed every saved diff and committed the recovery changes as `9083a2d6`.
- Merged the language-server lane `lane/cG2` (`9df9e1e4`) without conflicts
  in `df3c5d95`. The saved head lowering includes its `Fail(E)` fix. The
  lowering regression also checks the inferred value and environment holes,
  and an error entry written only on a later branch.
- `cargo check --workspace --all-targets --locked` passed. The initial
  `syn3_lower` run with `--features legacy` passed all 15 tests. Further
  verification is in progress; command logs are in `target/conv-r2/`.
- Removed 107 stale old-syntax inventory records: 61 for files removed by
  earlier commits (the former `parse.rs`, `syn2.rs` and `golden/` parser suite),
  and 46 for literal/file texts no longer present after conversion. These
  identities matched no current source. Current snippets continue to be
  extracted and checked; diagnostic changes are reviewed separately.

### Reviewed checker golden changes

The initial full checker run found five golden mismatches. Each was reviewed
against the printed actual output before updating its expectation:

- `basics`: `unit` is `()` instead of `Record`, as §6.4's empty data requires.
- `known_types`: the migrated `annotated(1)` is a valid positional call and
  returns `Int`. Its old annotation-label error disappears; without an error
  the existing unused-binding warnings are no longer suppressed. All known
  type comparison warnings stay. §3: "a function type has no parameter names";
  §6.8: `(name: T)` is positional and labels require `label = p`.
- `labels`: migrated successful calls are positional and return their written
  result. The intentional calls with labels still check rejection in this
  transitional front end, duplicate arguments, missing arity and unknown
  constructor fields. The inferred hole of `f(1)` now receives `1`; its old
  label-resolution failure no longer forces `Never`. The comments now state
  the migrated cases' intent. Final data-shaped call checking is stage 7.
- `may_fail_group`: the same three provably impossible fitted literal
  patterns now receive the W0505 described in behaviour change 1. The
  top-level `Assert` error is retained unchanged.
- `type_params`: the lowercase-head parser error was split into the existing
  independent `front_end` case. The fixture now reaches all seven original
  checker conditions: the type-head collision, free type, duplicate variable,
  function variable collision, applied variable, missing type and lowercase
  value. No checker failure was removed to bless this output.

### Recovery verification milestones

- Syntax: the initial full run passed all but three targets. The focused
  retry passed `format` (2), `lisp_record` (6), `lisp_syntax` (46) and
  `syn3_lower` (16), including the new empty-data source regression.
- Checker: every target except `types` passed in the initial full run, with
  `spec_corpus_reverse_order_and_recovery` skipped as instructed. The focused
  `types` retry passed all 5 tests after the reviewed changes above.
- Macro: all 9 tests passed.
- Wasm: every target except `control_lazy` passed initially. Three converted
  providers incorrectly used `Out(print = …)` for `Out = (put = …)`; these
  now name `put`. For all six stop traces, the old call sites were identified
  in `019f9e19^` and the same calls located in the assembled converted source.
  Only their line/column coordinates changed. Messages and frame order remain
  pinned exactly (§11.7; syntax-3 §2.10.5 provides add no call frame). The
  focused `control_lazy` retry passed all 17 tests.
- LSP: all 8 unit tests and 98 protocol tests passed, including
  `lsp_hover_errors_and_environment` and the latency bound.

### Syntax inventory review

The first current-source inventory run found 37 mismatches, all reviewed:

- Eighteen spec examples now parse as syntax three, so their graduated
  future-track records were removed. Their source is still checked normally.
- Three spec example heads (`retry`, `collect`, assertion `catch`) incorrectly
  wrote a need where the grammar permits a service pattern. The heads now
  omit that part and infer their needs, as §6.8 and syntax-3 §2.10.1 require;
  the types of their callback parameters retain the same stated needs. Their
  future tags and old parser expectations were removed.
- The remaining affected future examples keep exact errors for shape groups
  (stage 9), mixed data (stage 6), and whole environments (stage E6), with a
  specific dependency reason for each.
- Four Rust host strings are Cargo/logging directives or a source-search
  fragment, not programs. One numeric test body is checked with the function
  head its generator supplies, instead of being parsed as a whole module.
- The independent lowercase-head test and the two negative string-pattern
  snippets keep exact syntax-three errors. Five existing negative review
  files record the specified lowercase-head E0101 instead of the old E0232
  checker path. All have adjacent diagnostic declarations. Three existing
  negative spec fixtures were missing only those declarations; their exact
  expectations did not change.

The retry passed: **4,713** extracted snippets/files/fences, exact expected
messages matched, **6** future-track entries. No blanket parser exemption or
expectation regeneration was used. Checker clippy also passed with
`--all-targets --locked -- -D warnings`.

### Release, standard library, and five negative review observations

- Release build passed; `i fmt --check std lib examples` passed unchanged.
- `i test --project std --opt 0` passed all **2,169** tests.
- A corpus run selected only `review-runtime/echo-types`, `review-s6/probe`,
  `review-std1/factory`, `review-std1/tree` and `review-std1/variance`. The first
  run reported only observation differences; all five still rejected with
  exit 1 and empty stdout. Every stderr diff was inspected before updating.
- `probe`, `factory`, `tree` and `variance` now stop at the lowercase-head
  E0101 parser errors described above, before their former E0232 and dependent
  checker errors. `variance` retains all four erroneous head sites.
- `echo-types` materializes its preserved `.invalid/src/main.i`, explicitly
  listed in `syn3-unconvertible.txt`. Its old `import(...)` is an ordinary call
  under the new grammar: the two positional imports after labelled items give
  E0110; its two old clause arrows give E0101. The converted task stub adds two
  lowercase-head E0101 errors. These are the exact negative errors already
  pinned by the syntax inventory. No source of this historical invalid input
  was changed to make the observation pass.
- The five observations keep their original **10 second** bounds, commands,
  exit codes and empty stdout. Only reviewed stderr and compiler metadata
  changed. The filtered retry passed all five with no failures. The unfiltered
  review corpus remains the coordinator's gate.

Of the inventory's six tracked-error records, three are the explicitly
staged spec examples and three are preserved historical invalid T3 inputs;
all six are always parsed and compared to their exact diagnostic lists.

### CLI migration guidance and conversion between surfaces

- `macro_run` passed 57 tests, `typed_macros` 35, and `commands` 14.
- `check` found one lost migration-guidance path: syntax-three parse errors
  bypassed the old reader's hints. CLI diagnostic rendering now gives a
  conditional note for a module with E01 parse errors, using the existing
  lexical hint helper. The hint writes a syntax-three chain, not the removed
  brace match. Diagnostic codes, locations and counts are unchanged. The
  valid ordinary-name case now also asserts that it has no migration hint.
  The full `check` retry passed all 10 tests (the documented stage-7
  `check_types` case remains parked).
- `fmt --to` still wrote/read the legacy tree. Ported the standalone
  `syn3/surface.rs`, `walk.rs`, `group.rs` helpers and CLI bridge from
  `d3ee3696` of the pending core lane; that revision's syntax-three AST is
  identical to this lane's. Conversions compare syntax-three trees with only
  expression groups removed, preserve comments at item/statement boundaries,
  and report comments with no such place. All 6 formatter tests passed.
- A macro definition has no normal spelling even without a quote (§14.1),
  so the bridge rejects that case before writing a file. The former
  unrepresentable-block fixture is now valid because every expression brace
  is a block (syntax-3 §2.2); it remains covered as a successful conversion,
  alongside the negative macro-definition case. All 6 Lisp tests passed,
  including boundary comments, unplaced comments and destination protection.
- Workspace all-target checking passed again, and the syntax inventory still
  matches all 4,713 texts with six tracked-error records.

The fresh release build and std/lib/examples formatting passed after this
bridge change. The five-case corpus comparison found exactly three new
conditional note lines, in `echo-types` (two modules) and `tree` (one).
Removing those note lines reproduced the prior stderr byte for byte; codes,
locations, counts, exit results, stdout and bounds stayed unchanged. The
reviewed observation update and filtered retry passed all five.

A final bridge review found that interpolation must also be lowered before
core-tree comparison (§1.6). The comparison now lowers its clone to left-folded
concatenation, including the empty text prefix for a sole expression. The
new end-to-end test converts a program to Lisp and back and executes an
assertion of its string result after each conversion. All **7** Lisp tests
passed. The release rebuild including this normalization also passed.


### Lisp macro inputs during the transitional lowering

The remaining CLI sweep found a panic in
`compile_time_tests::tests_use_each_builds_unmarked_quote_templates`.
`source_item_syntax` reparsed a syntax-three Lisp item with the legacy
reader, which rejects the empty data `(,)` in a function head. It now uses
the item kind already established by parsing. Source datums also pass through
the same span-preserving `unconvert_quote` adapter as Lisp lowering before
the transitional macro engine consumes them. This keeps both source surfaces
on the same compiler tree (§14.1), rather than sending syntax-three rows to
the old macro-result reader. No source fixture or runtime assertion changed.
The new unit regression and all three compile-time tests pass, including
both optimization levels and both source modules' unmarked quote templates.
The 57 macro-run and 35 typed-macro tests pass after the fix, retaining
source-position and hygiene checks.


### Main integration after 13:00

Merged the coordinator marker `eb2d7b03` (closed data, a single data
parameter, shape groups, selection and multicore). Kept the Fail-head lowering,
the outer-environment order of provided services, and the LSP CPU latency
bound from this lane. The two remaining feature-switch checks are unconditional
because syntax three is the default.

Re-ran the converter on the imported source: 26 files converted initially,
34 embedded programs converted, and eight multicore and eight HTTP Rust
format templates converted with their substitutions retained. Two constructor
documentation comments in the new task scheduler had been placed after the
converted union by the comment printer; they were moved onto their own new
types (`Woken`, `Away`). No comment was dropped. A mixed callback annotation
was changed from `send: _` to the data type field `send = _`.

Positional binder hints are no longer field keys in shown function types:
`(x) { x }` displays `(A) -> A` (§4.6), while actual labels of the data shape
remain labels. Closed data displays fields with `=` (syntax-3 §2.1).
The stage-7 CLI `check_types` test is now unparked and all ten check tests pass.
The three stage-7 package-command tests are also unparked for validation.
The retained cross-module scheme test now round-trips a generic value under a
fixed closed-data label; both scheme tests pass. Legacy dynamic-label tests
remain present with current rewrite reasons, rather than silently disappearing
in modify/delete resolutions. The old opaque record-graph query case remains
parked with its reason; main also brings its closed-data union replacement.

The checker golden review separated 37 files changed only by removing
positional binder names from types from 11 further changes. Those 11 follow:
closed-data access requires a known field-bearing type (§4.8 rule 13), unknown
fields now fail there and suppress the existing unused-binding warnings;
function arguments constrain one data shape (§4.6), so arity disagreement is
a data-subtyping error and an inferred function may accept a union of shapes;
selection proves `field(S1(...))` is `0` (syntax-3 §2.6); and main's worker
exports and corrected closed TLS annotations change library types. All five
checker type tests and both cross-module scheme tests pass after review.
No runtime assertion or bound was changed to obtain those results.

The full syntax run passed every target except the skip-list check: the
converter's fold had promoted two E6 examples. Their future fences were
restored, preserving their exact errors. Six stale expected-error records
now parse and were graduated; nine dead identities from removed/moved code
or replaced template strings were removed without changing retained entries.
The inventory retry passes all **4,728** texts with **five** exact tracked-error
entries (two E6 examples and three historical invalid inputs). Both provided
service and environment examples still have their explicit E6 reasons.

The new `syn3_fixtures` CLI binary passes after the imported formatter's
source moves were mapped to the same call sites. `multi_value_call`'s `f` call
moved from 21:24 to 22:16; the three `no_shape` calls and three
`select_reject` bindings moved with formatting. Error codes and messages are
unchanged. The `scale` type keeps its actual `by` key and writes `by = Int`.
The fixture library helper now checks a syntax-three AST/printer round trip
instead of converting a library that is already converted.
