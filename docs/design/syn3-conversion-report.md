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
- Syntax three is unconditional. Stage 12 removes the production legacy
  parser and formatter and the `legacy` Cargo feature. Historical migration
  inputs use the frozen private readers under
  `crates/ilang-syntax/examples/support/legacy/`, shared with the conversion
  regression tests. The compiler cannot select those readers.
- `scripts/check-snippets` is the exact-diagnostic embedded-source gate used
  by the conversion lane, renamed from `old-syntax-check`. Its JSON retains
  every source expectation; only the obsolete `_syn2_old_forms` description
  is removed. Negative-fixture comments retain their existing marker spelling
  so source bytes and diagnostic positions remain unchanged. The conversion
  tools remain available because their regression inputs still use old text.

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
   (`f = (x : Start(Int))`) is not defined (E0202). Lane/fa's final tree makes
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
5. **`fail : Fail(E)` in a function head** is the function's error part, as
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
  §6.8: `(name : T)` is positional and labels require `label = p`.
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
was changed from `send : _` to the data type field `send = _`.

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


### Final recovery sweep and remaining package-interface failures

The post-main full checker run found nine failing targets. The library,
`types_joint`, `value_types` and `provider_graph` failures traced to a converter
rerun: previously converted nominal heads such as `Box(A) = (A)` had been
reread as old aliases. Restored those original snippets; main had no changes
to the three whole test files, and only the literal in the pattern unit test
was restored. `--since` now selects the same baseline for embedded programs
as whole files. Three isolated conversion-baseline tests and the eight
inventory unit tests pass. No expectation was regenerated to hide this error.

All checker targets then passed through the full run plus target retries,
with exactly `spec_corpus_reverse_order_and_recovery` skipped. The display-only
failures were reviewed positional binder hints; types, needs, unions, source
spans, alias identities and bounds remain checked. Twelve LSP hover/completion
expectations were ported by the same rule; signature-help binder labels and
offsets remain unchanged and still use the source parameter metadata.

The nine formerly parked CLI interface cases were rewritten for concrete
closed keys and ordinary generic value types, retaining all assertions and
adding the required equality under labelled-key reordering. This exposed a
lowering bug: wholly labelled function parameter data had been sent to the
old positional-hint representation. It now uses the existing `%data` adapter,
so actual keys survive and their order is immaterial (§2.3, §2.4). The existing
positional/mixed adapter and invalid-spread diagnostic are retained. A direct
lowering regression checks that both keys survive in one data parameter.

The `i` binary unit test retry is **17 passed, 6 failed**. The remaining names
in `commands::package::value_type_interfaces` are:

- `distinct_key_parameters_may_receive_the_same_string`
- `exported_record_schemes_compare_labels_and_value_types`
- `inferred_record_polymorphism_is_not_a_concrete_field_scheme`
- `key_parameter_and_field_type_parameter_are_compared_together`
- `key_parameter_can_collide_with_a_written_record_field`
- `record_union_interface_compares_denoted_sets`

Three diagnosed differences are classified `representation=1, changed=0`
(the proof is unknown); the unannotated open-record inference case is now
E0506 under the closed-data known-key rule. The other two remain semantic
comparison failures. Their assertions remain active and unchanged. This part
is unfinished; no weaker comparison or new blanket skip was introduced.
The final labelled-function-type fix has focused lowering/interface evidence;
the complete checker suite has not been repeated after it.


The fresh post-merge release build and std/lib/examples formatting pass.
The std opt-0 run stopped at `http_network_test`'s existing timeout callback:
`send_timeout`'s migrated `(Request) <- (Fail(HttpError)) -> Response` had lost
the old `! _` service need (`need-not-named` in the conversion notes), forcing
its callback to need nothing. Preserve that open need with `<N>` and
`(Request) <- (Fail(HttpError), ..N) -> Response`. The body and timeout test
are unchanged. Formatting passes. The corrected std retry waited for a memory slot without
producing compiler/test output; its own process group was stopped before the
14:00 deadline so lighter crate checks could run. The corrected std run is
unverified. The pre-main opt-0 run passed 2,169 tests.


Final lightweight checks: the macro crate passes all nine tests. The full
post-main Wasm run passed every target except the opaque-decoder fixture,
whose `A = Opaque` and `B = Opaque` had also been reread as aliases. Restored
the original two heads, leaving main's relative growth bound intact. Its
focused both-target retry passes; all other Wasm targets passed in the full
run. The JavaScript fetch fixture passes `node --check`, and both conversion
Python scripts parse. LSP/clippy/final inventory results are in the command
ledger and final report.

The formatted closed-data spread-order fixture keeps the exact `panic: base`
stop and both frames. Mapping the same calls from main's source gives
`panic("base")` 7:33 -> 9:35 and `base()` 10:6 -> 12:5; only those source
coordinates were changed. This was derived from the original and converted
source, not from a recorded runtime expectation. Spec chapters have not run.


### Evening continuation: main fd99f9e3

Merged main's stage-10 runtime, global cache, checker speedups, U1 numeric
modules and reduced test-build memory. Converted 15 imported files including
`big.i`, `bits.i`, `float_text.i`, and a new big-int test snippet. Three new
helpers in an otherwise already-converted numeric-lowering program still
used old clause syntax; translated those heads and branches directly while
keeping every body. The embedded retry reads all imported snippets without
errors. Stage 10 replaces multi_value_call's former rejected call with main's
runtime expected-output fixture, preserving its intent. The workspace
all-target check passes.


### Evening step 2: interface and completion recovery

The comparison implementation is identical to main's ba23adb0 family:
labelled data forces semantic comparison, closed shared groups have a
counterexample check, and open_alias uses closed-data difference. The
converted legacy cases now retain closed keys, with polymorphic field values
inferred from `(a = _)` instead of the removed open-record inference. All
five uncertain scheme differences remain breaking major-version changes;
tests pin their exact representation-only classification, as main's
open_alias test does, rather than asserting a semantic counterexample that
the read-only prover did not establish. Positive alpha-renaming and
generic-value comparisons remain unchanged. This preserves the migration's
versioning intent under syntax-3 §2.3's removal of dynamic keys and Record.

The denoted-set alias comparison lacked distribution through a closed data
field: `(a = Int | String)` equals `(a = Int) | (a = String)` (spec §4.1,
§4.4). The read-only prover now checks each field-union alternative under
its existing demand budget; no mutable solver or input acceptance changes.
The regression also keeps two-field correlation: the independent product
is different from the union of only the two correlated shapes.

An unfinished match branch `| () { ... }` lowers as a different-shape
function instead of a Case. Completion now recognizes a source branch head
inside the enclosing pipe, and uses the subject's known types for constructor
snippets. Token delimiters exclude strings/comments and branch bodies. The
new query regression checks Circle/Rect/Dot snippets and excludes the body;
the formerly failing LSP protocol test passes. The 23 CLI binary unit tests
pass, with the added correlation check verified separately.

The evening package-command run passed 24/27 initially. The three publication
failures contained only source parameter names in expected displayed types:
the diagnostics still correctly report changed arity, constructors and
dependency nominal identities, and still require the same major versions.
Updated those five signature strings only; the publication retry passes all
11 tests. All 27 cases now have passing evidence. Determinism passes three
tests and multicore passes all ten before proceeding with the remaining CLI
binaries and filtered spec chapters.

The value-types WAT equality regression initially differed only in $site_data:
conversion had formatted its executable call sites to different lines and
columns. Aligned executable source positions while retaining the complete
WAT equality assertion; all four native/WASI, tests-on/off comparisons pass.
No compiler behavior or expectation changed.

WASI passed 16/17 before finding the stop fixture's same-name constructor
converted as a transparent alias. Main's `Bad { Bad(Int) }` needs `Bad = (Int)`
by syntax-3 §2.8 rule 3. Restored that nominal declaration; the focused test
passes at every optimization level with unchanged messages and exit codes.

Chapter 11 initially passed 49/51 at both levels except stale frame coordinates
in two stop fixtures. Derived call anchors from main to converted source:
stop_tail_service: a.fail 12:12 -> 10:34, f(x) 16:19 -> 13:19, apply 20:2 ->
16:2; stop_tail_async_service: later.nan 26:2 -> 22:2, take 37:2 -> 32:2.
All five frames and exact stop messages/outputs remain asserted. The 11/
retry passes all 51 cases at O0/O2, including the matching 9.11/ paths.

Chapter 6's two failures were source conversion mistakes, with the original
expected outcomes retained. The stage-4 negative labelled call had been
normalized into a valid positional call; restored labels with `=` so it
still tests rejection by a positional function shape (spec §6.6). Unit's
empty-data annotation needs a transparent alias `Unit = { () }`, not the
new nominal declaration `Unit = ()` (syntax-3 §2.8 rules 3/4). The 6/ retry
passes 78/78 active cases at O0/O2; 30 removed open-record cases stay parked
with their written §2.3 reasons.

The 9/ filter passed 126/127 at O0/O2. keep_env lost main's explicit
`Extra(Nil, Never, Assert)` service annotation, creating a recursive inferred
environment. Kept that exact annotation in a typed local before providing it;
the focused keep_env retry now passes both levels with every freshness,
resumption, cancellation and portability assertion intact. A guarded edit
initially did not apply, and its premature retry repeated the original error;
that unsuccessful command is recorded in the ledger.

A source-position audit also mapped own_assert's same panic call 8:32 ->
7:40, and the async/sync host-error main definitions 15:1 -> 12:1 and 14:1 ->
11:1. These exact frame assertions pass in the 9/ and 7/ filters respectively.

Repeated conversion had promoted the two remaining E6 future-SYN3 fences
after their staging markers were already gone. The spec-fold helper now
only promotes such fences when it is folding a staged document. Checked
initial old/new staging on a temporary document and byte-for-byte idempotence
on the current spec. Restored the two future fence tags without changing
their source or registered diagnostics. Removed one confirmed dead inventory
identity for the formerly shown hover type with two row variables. Final
formatter and exact-diagnostic inventory gates will verify this sweep.

Reviewed all parked spec reasons and re-enabled two cases whose intent is
already closed-data compatible: written item evaluation order and rejection
of data at the host boundary. The evaluation-order panic anchor moves from
main's 7:7 to 9:7 after its export header is formatted. Their expected stop
and E0320 outcomes stay unchanged. The 6/ run now passes 80/80 at O0/O2.
The remaining 29 parked cases each have an individual reason: 28 rely on
removed dynamic labels, open Record, record operations or width subtyping;
one pins a former record-graph TypeView error which is now unreachable.
Std passes all 2,170 tests at both O0 and O2 after main's memory fix.

### Evening final gates

All requested active checks now have passing evidence after main fd99f9e3.
All remaining CLI binaries ran; all 28 spec filters ran, with passing
retries for every corrected case (458 unique active cases at O0/O2).
Std passes 2,170/2,170 at both O0 and O2. Numeric lowering's four tests
pass, including O0/O2/O3 agreement; the bigint benchmark's default invocation
compiles and reports one intentionally ignored benchmark, not a measurement.

Workspace rustfmt and its check pass. Workspace clippy with all targets and
-D warnings passes. The release source-format check of std/lib/examples
passes. The source formatter regression passes both tests. The exact syntax
inventory passes 4,723 extracted literals/blocks with five future-track
entries. Its first evening run exposed an imported template entry still
substituting the old SPIN constant and a Wasmtime preload argument parsed
as i. The entry now substitutes the actual converted constant; guest={}
is correctly classified as the Wasmtime command argument at its call site.
No expected diagnostic was regenerated. Reviewed rustfmt's four-file diff;
only Rust formatting changed. Main's marker still names fd99f9e3.

The detailed command/result ledger, including unsuccessful commands and
their retries, is in the coordinator cache report-conv.txt. The 29 retained
legacy spec cases each have their own written reason. Checker and CLI
pending lists are empty. Whole-workspace tests, unfiltered spec/corpus and
dev-verify remain coordinator-only. R2 opt-in measurements and the bigint
baseline benchmark were not enabled.

### Tonight integration: batch2 E1 and multicore

Merged batch2 1cb245b4. Kept its substantive fixture and row-scheme changes,
converted the incoming programs, retained both independent syntax tests,
and preserved the folded syntax-three spec with its E1 paragraphs. Two
old declarations inserted into already converted library modules needed
direct translation: control.escape's signature and the new prelude Fail.
The converter retry converts the remaining 31 embedded row-scheme snippets.
Batch2 golden observations are retained pending reviewed syntax/display and
coordinate translation. Corpus baseline snapshots are taken from batch2;
only the 1,216 review-k4 cases whose program text differs from that revision
may be re-recorded, with each phase exit and error-code set checked first.
The other nine cases must keep their observations unchanged.

Authority: spec §9.2, "The merge of two needs is their intersection (§4.4)."
"When the intersection for one service of a need is Never while neither
side is Never, the need cannot be satisfied: it is a compile error (E0515)."
"A provided value of type P satisfies the need entry S when P <: S (§4.5
rule 4)." Batch2's runtime/default-worker and fixture changes are preserved.

Merged num4 024ac887 and light-boundary 7c8e490a within their 15-minute
integration limits. Kept shaped and lazy callees together, retained both
sets of emitter fields and lowering passes, and adapted lazy closure calls
to the current shape argument and function-pointer field 9. Converted five
incoming fixtures, all numeric differential helpers, 316 E1 review shown
types and 42 exact requirement type fragments. Requirement outcomes and
error-code lists are unchanged. The queued converter was stopped by its
own process IDs and rerun using the already-built migrator; that source-only
run performs no build or compiler/corpus verification.

The E1 merge exposed two migration omissions. A bound ordinary Fail(E)
service now uses the same transparent alias as ordinary Fail in a need;
otherwise the transition lowerer removes its binder as legacy error syntax.
A lowering regression requires that the original handler binder remains an
environment parameter. Parameterized environment statements now bind their
value with the written service type before providing that local: dropping
that annotation could let inference choose a different service form. This
preserves spec §9.2's provider-subtyping requirement instead of weakening a
negative test. Re-converted 1,250 original batch2 sources/snippets; five
known invalid files retain their original registered diagnostic markers.

The checker display fixture's non-function polymorphic empty-list binding
uses inference for the same empty List(Never), since syntax-3 §2.16 puts
type parameters only on function values/types. The host-export fixtures
retain their written wider error types as binding annotations, preserving
the original non-host-passable error checks. Guarded corpus observation and
full checker verification are pending; no observation with a changed phase
exit or error-code set may be recorded.

E1 provider inference correction: syntax-three provision had selected the
needed service form from the rest, or invented fresh arguments, then
subtracted that form. It now subtracts the actual provided form, as the
legacy explicit-provider path already does. Spec §9.2 requires P <: S for
the provider P, and Rule B retains the directed obligation on the remaining
row. The focused regression passes: a concrete Read(Int) handler is not
falsely hidden, and Read(String) reports E0515 without an extra E0501.

The first guarded K4 pass was stopped at a checkpoint (about 100 cases):
92 matching observations were recorded and eight outcome changes refused.
No refused expectation was changed. Retained an anonymous closure's written
need through a typed binding and kept literal callback result annotations.
The new checker regressions pin all eight batch2 checker code sets. Seven
golden outputs were translated after reviewing types and source positions;
their diagnostic-code sets exactly equal batch2. Library/spec outputs still
await fresh verification. Exact post-fix full gates and corpus run are pending.

Keep handler callback types directly on function literals, rather than
erasing them behind an outer binding annotation: the former shares the
callback/provider parameters during body inference. The i/failure checker
reproducer now shows catch with only N and a caller with no Fail need.
The inline typed raise callback retains the light-boundary optimization.
Row-scheme replacement fragments were also translated: every permutation
or hole test now changes actual syntax-three text, and the value- and
error-row branches remain distinct.

The post-fix type suite now passes, including both library and spec snapshots.
Their observations were reviewed and diagnostic-code sets checked against
batch2 before translating. The eight refused K4 checker outcomes also pass
together; recursive-row cases retain their written function need through a
binding annotation. All refused corpus expectations remain untouched.

The first complete checker run found nine test failures: eight row-scheme
cases and the captured-row coalescing case. Corrections retain written wider
needs as binding types (syntax-3 §7.3: "A written `! N` that is wider than
what the body needs is kept as a type annotation on the binding
(`f : (P) <- … -> R = …`), so that the type does not change."). The inferred
provider fixture also keeps its original S(A) annotation. Bare legacy ! N
callback annotations retain Fail(Never), avoiding an unconstrained error
variable in this transitional lowerer. Four remaining expected strings only
change to syntax-three type display (unlabelled positions and `<-`). Row
algebra is supplied through a named alias spread rather than a union of
list items. The unused-local variant now actually removes
the call and asserts that its substitution changes the source.

Two snippet-inventory entries render actual function-header/template
fragments with their real surroundings; no diagnostic allowance was added.
All 37 row-scheme snippets parse, and the 4,761-snippet inventory has no stale
entries. The complete compiler-package run and focused correction rerun,
CLI/spec/std/corpus gates, clippy and final snippet gate remain pending;
this checkpoint is not a landable result.

The complete syntax, Wasm and LSP packages pass (98 protocol tests). The
checker package run had only the identified row/coalescing fixture failures.
The coalescing provider now preserves its original S(Dependency) annotation
through a local binding, under §7.3's environment-statement conversion and
spec §9.2's known provider form. The row algebra is named by a transparent
alias before it is spread: the grammar permits a named spread, not a
parenthesized algebra expression. Assertions and all three algebra operators
remain; this changes the representation, not the intended accepted/rejected
programs. Focused verification is recorded in the command ledger.

The last row-alias failure exposed a lowering bug: adjacent Bad = S(Never)
was mistaken for a constructor group, absorbing generic S(N) and leaving N
unbound. Constructor-group absorption now requires the member's arguments
to match the declaration's parameter names. Specialized aliases retain the
original constructor and scope, as syntax-3 §2.8 kind 4 requires: "A name is
a transparent alias, as today." A lowering unit test pins both definitions
and S's N parameter; that focused lowering regression passes. The final
checker row-algebra rerun and remaining gates are pending.

All nine initial checker fixture failures now pass focused verification,
including the row-algebra test after the alias correction. The complete
compiler packages are being repeated for that lowering change. Formatted
the edited library handler and verified its focused format check. The
stopped first corpus recording finalized 92 observations (90 was the live
checkpoint count); all 92 have source changes and batch2-equivalent phase
exits/error-code sets. The eight refused observations remain unrecorded.

The fresh compiler-package run and workspace all-target check pass. The
requested CLI command, multicore and syntax-three fixture binaries pass
(14, 11 and 1 cases). Typed macros exposed old Lisp declaration/function
fragments reintroduced by a prior main integration: 35 parser failures.
Restored the earlier reviewed conversion of those exact fragments, converted
the newer closed-data and determinism fragments, and gave SyntaxError calls
their written message/at labels. No assertion or expected rejection changed.
Authority: syntax-3 §6.8 uses `(-> [A] (, A) A)` for polymorphic function
signatures, `(-> (, value) _ body)` for functions and `|>`/`|` for pipes and
chains; §7.3 explicitly includes Lisp strings in Rust tests.

The ordinary inventory did not extract these Lisp fragments. A companion
Lisp check now parses every repository .is file and the complete modules,
signatures and functions assembled by macro_run/typed_macros. The existing
length-prefixed parser driver has a --lisp mode; normal inventory semantics
are unchanged. New gate and typed-macro rerun are pending behind the shared
load guard. This remains an unlandable checkpoint until all gates finish.

Typed macros now pass all 36 cases. Connected the companion Lisp inventory
to the ordinary inventory command, with actual generator substitutions for
the row-algebra and declared-macro templates. No diagnostics were allowed
or weakened. Combined inventory passes: 4,762 normal snippets, 34 .is files
and 89 CLI macro fragments; the eight inventory helper tests also pass.
Feature-enabled syntax/clippy, requested spec/std and full corpus gates
remain in progress.

Spec 4.4/ and 4.5/ pass at both optimization levels. The 9/ batch completed
134/139; all five failures were conversion omissions. Three light-boundary
fixtures now use the transparent alias for a bound ordinary Fail, and the
unused wider Fail need in directed_rows uses the same alias. The collect
examples in control/directed_rows/light_boundary keep callback and provider
parameter A directly on the literals, as the already-verified spec type
fixture does; Fail(Never) retains the original empty legacy error channel.
A manually provided Fail value retains its written Int form and raise label.

Focused reruns pass all six light-boundary cases and control.i and
directed_rows.i at O0 and O2. Thus all 139 cases are covered by the original
batch plus targeted source-only corrections; the unchanged long cases were
not repeated. No compiler code changed. Two stop-fixture headers moved only
by the inserted alias lines; positions were derived from the same f.raise,
fail.raise and control.resume call anchors (§11.7), and the exact stop checks
pass. The fixture format check also passes. Required 11/, release/std and
full corpus verification remain.

Merged main 46a6d43b6; its benchmark grouping/format correction was already
present in this lane, so the merge has no tree delta or new old-syntax text.
Spec 11/ passes all 51 cases at O0/O2, and the fresh release build passes.
The first std O0 run found the same erased generator callback/provider A
in std/test/list_test.i. Kept those annotations directly on the literals,
using the already-verified spec helper pattern. Entry compilation now
passes and the std O0 suite is executing; O2 and corpus remain pending.

The complete K4 check-only audit found 53 conversion gaps. Of these, 51 now
match batch2's check exits and error-code sets in one focused 51-case run:
24 two-row annotations retain both rows through a named transparent alias;
written recursive needs stay on the binding; callback/provider types remain
on literals/parameters where inference needs them. Five malformed programs
retain their original error intent in syntax three: unsupported block-comment
delimiters, a service statement without an expression, a typed binding in an
expression position, and a block ending in a binding. Their parser inventory
entries were individually explained and rekeyed, and their batch2 E0101 or
E0105 sets are unchanged. No corpus observation was changed by this audit.

Two copies of p1 still differ from batch2 (accepted instead of E0515).
An experiment preferring a provider's known binding form failed its new
regression twice; it was saved under target/conv-tonight and discarded,
including its uncommitted reproducer. Existing committed tests and production
code remain unchanged. The source-only p1 experiments were also discarded.
Std O0 and O2 now both pass all 2,170 tests. Workspace all-feature clippy and
the full requested compiler-package rerun pass. Complete K4 recording and
zero-failure validation remain required; this checkpoint is not landable.

A read-only audit of all required type fragments found twelve forwarder
fixtures (q25/q30/q35/q40 in three directories) whose written complete needs
were erased by conversion. Their bindings now retain those needs. The unused direct service read is
omitted: these functions only forward the need through use(), as §2.10.1
requires; retaining that redundant read alongside the annotation caused
false E0505 cycles. The
literal also retains the original empty legacy error channel with the
transition Fail(Never) marker. All twelve accept and match the unchanged
required type fragments. Their parser checks pass. No requirement was
changed, and their observations remain subject to the batch2 outcome guard.

Merged main 29515fbfa native-test executable-busy retry without changing any
embedded literal, compiler or runtime code. The requested latest-main std
O0/O2 suites each pass 2,170 tests; workspace all-feature clippy, both format
checks and the official normal/Lisp inventories pass. Native verification
for the incoming test-harness change is queued behind the corpus.

The first 100 complete K4 observations now pass the guarded recording and
an independent source-change/phase audit, including all eight initially
refused cases. Every source changed; phase exits, diagnostic-code sets and
runtime stdout equal batch2. The complete corpus is still running, and the
two p1 correctness blockers remain unresolved. This is not a landing claim.

The complete first 200 K4 observations have now passed the same independent
source-change, phase-exit, diagnostic-code-set and runtime-stdout audit.
Only these stable completed observations are committed; recording continues.
The full corpus and the two known p1 regressions still block landing.

The complete first 400 K4 observations have now passed the same independent
source-change, phase-exit, diagnostic-code-set and runtime-stdout audit.
Only these stable completed observations are committed; recording continues.
The full corpus and the two known p1 regressions still block landing.

A total of 600 completed K4 observations have passed the same independent
source-change, phase-exit, diagnostic-code-set and runtime-stdout audit.
Only eligible completed observations are committed; refused cases remain
untouched and recording continues.
The full corpus, two known p1 regressions and pending phase refusals still block landing.

The ongoing recorder refused multi_run: test0 printed its passing result but
hit the unchanged ten CPU-second bound; its other phases match. A guarded
retry with the same bound is pending. The first p1 copy has also been refused
as predicted; its original expectation is untouched.

Six source-unchanged programs need only type-display translations in their
current observations: stress_plain_1024/128/32/512, finite_never and
recursive_never. A proposed patch changes six check.stdout fields; all 1,698
type lines are exact notation translations and every other JSON field is
byte-preserved. Because the brief permits re-recording only source-changed
cases, an explicit exception was requested. The patch remains unapplied
pending the user's answer. No source was changed merely to permit recording.

A total of 800 completed K4 observations have passed the same independent
source-change, phase-exit, diagnostic-code-set and runtime-stdout audit.
Only eligible completed observations are committed; refused cases remain
untouched and recording continues.
The full corpus, two known p1 regressions and pending phase refusals still block landing.

Coordinator b15 item 1: the lost p1 obligation was caused by generalizing the
migration's typed provider temporary before its provide. The inferred pure
service form replaced the old explicit `S(_)` provider contract. Merely
choosing the checker's known type, or copying the annotation onto a provide
of the already-generalized variable, did not restore the obligation.

The lowerer now rejoins an immediately preceding typed provider temporary
with its environment statement when the temporary has exactly one use.
Both the written contract and original value reach inference together, as
in the legacy statement. Other locals retain their binding. Both complete
p1 copies now report exactly E0515; all 26 row-scheme tests and all 20
lowering tests pass, including single-use/reused-provider regressions.
The requirement remains reject E0515, and no expectation was re-recorded.

Coordinator b15 item 2: E0523 is the entry-provider plan error, here
`service std/list_test.Yield(A) must be closed`. The copied std fixture lost
its generator callback's direct parameter annotation during conversion,
leaving A open at the native test entry. Restored the original callback type
on the function literal and the emitter's x:A annotation, using the already
verified repository std helper. At O0 and O2 the complete test bundle builds
before CLI runtime filtering, then generator_callbacks_test passes. This is
not a full runtime run of the copied std suite. No bound or expectation was
changed, and there is no longer an E0523 compilation failure.

Coordinator b15 item 3: measured stdout differs only at failure source
positions: 12:21 became 13:4, and 24:2/29:2/34:2 became 25:2/30:2/35:2.
The converted fixture now retains the original call-token coordinates.
Both O0 and O2 stdout are byte-for-byte equal to main again: the same four
intentional failures and two passing tests, including the corpus startup
probe. No failure was suppressed and no expectation was changed. The
before/after execution diff is in target/conv-b15/failure-output-results.json.

Coordinator b15 item 4: each case ran alone three times per build, interleaved
main/converted, with CPU time from waited child processes. The reference
was built from exact main 29515fbfa in this worktree under a separate binary
name; dependency versions and registry identities equal main. Its old-surface
p1 check rejects E0515. No compiler executable was copied. All samples use
O2 and the same injected startup test, and all exit 0 with identical stdout.

| Case | Main CPU seconds (three runs) | Converted CPU seconds (three runs) |
|---|---|---|
| mutual200 | 10.788, 8.666, 8.767 | 10.584, 10.735, 8.694 |
| stress-keys-2000 | 11.289, 5.289, 5.355 | 11.447, 5.311, 5.497 |

Both builds exceed the saved ten-CPU-second bound when cold. Thus the gate
timeout alone is not evidence of a conversion regression. The converted
mutual200 second run has a checker outlier (8.21 CPU seconds versus roughly
6.2 in the other paired runs); its default-cache median must not be described
as steady-state cost. With cache disabled on both sides, three more isolated
mutual200 pairs use 9.318/9.468/9.439 CPU seconds on main and
9.690/9.639/9.634 converted: medians 9.439 versus 9.639, about 2.1% apart.
Median stage deltas are +0.05 expansion, +0.08 checking, +0.04 lowering and
+0.08 Binaryen CPU seconds; runtime execution is 0.03 seconds on both. This
localizes the small difference to compilation, with transition-front-end
work in expansion/lowering, rather than a runtime slowdown. The small
checker/Binaryen differences cannot be separated from machine variation
with these samples. No large repeatable conversion-specific slowdown was
found. No case bound or expected observation was changed. Raw per-stage,
CPU and wall samples are in target/conv-b15/perf-results.json and
perf-controlled-results.json; the probes run through the heavy wrapper.

Coordinator b15 item 5: all six intentionally old A1 programs are now valid.
No existing requirement matched any of them. Their source is unchanged.
Syntax-three §2.2 says: “In an expression, every `{` begins a block:
`{ s1 … sn e }`. The statements run in order and the value is `e`. A block
with only an expression is a group: `{ a - 48 } * 100`.” Section 2.4 says:
“A function is a parameter group, an optional `->` part, and a block:”.
The old brace-decision rejection is explicitly removed by §2.2. Each case
now has an accept requirement quoting these paragraphs, with its inferred
f type pinned; runtime copies retain their assertions and empty stdout.

| Case | Prior requirement | Syntax-three reading |
|---|---|---|
| `review-a1/_loose/cases/17_old_unbound_fn_group/original` | None | Block calls `(x) -> Int { x + 1 }` at 41; f returns 42. |
| `review-a1/_loose/cases/18_old_expected_fn_group/original` | None | Block groups `(x) -> Int { x + 1 }`; f returns that function. |
| `review-a1/_loose/runtime/17/original` | None | Block calls `(x) -> Int { x + 1 }` at 41; f returns 42. |
| `review-a1/_loose/runtime/18/original` | None | Block groups `(x) -> Int { x + 1 }`; f returns that function. |
| `review-a1/cases/18_old_expected_fn_group` | None | Block groups `(x) -> Int { x + 1 }`; f returns that function. |
| `review-a1/runtime/18` | None | Block groups `(x) -> Int { x + 1 }`; f returns that function. |

All six pass checking and O0/O2 native test compilation/execution. The three
runtime copies also pass their O0/O2 assertions. These observations were
read-only; expected.json files remain unchanged. Probe results are saved in
target/conv-b15/a1-observations.json.

Coordinator b15 supersedes the earlier per-hundred recording and display
exception discussion above. The coordinator accepts the 1,621 display cases
and owns all remaining gate recordings. This lane did not audit those display
cases. The 800 post-841 incremental recordings were removed deliberately;
all source fixes remain. No observation was re-recorded for this audit.

Coordinator b15 item 6: the 120 same-exit/different-code cases form the
following groups. Counts are cases, not diagnostics or phases. The initial
read-only comparison used exact main 29515fbfa and the converted release on
63 cases: every non-permutation case, plus p0/p36/p9 from the 60 permutations.
The source contracts were compared in all 60 permutations. Complete old/new
stderr is saved in target/conv-b15/code-observations.json; the guard's long
stream/tree lines were truncated and are not a complete diagnostic inventory.

| Cases | Count | Reason for the diagnostic change |
|---|---:|---|
| K4 `generic_conflict`; K4d x1/x4/x6/x8/x9, z1/z3/z4, h1 in both `_loose/p` and `proj`; K4d `perm/p0`–`p59` | 79 | Conversion erased written complete needs, as with the earlier forwarders. These are lost constraints, not harmless deduplication. Restore them on binding types under §7.3. In the permutations, the missing error belongs to `bad`, independently of the retained `loop` error; `both` and `uset` also retain their original complete needs. Handler binding types retain the obligation-hiding E0515, and h1 retains its written-service E0501 checks. |
| K4c `recursive_co_2`; K4s `mixed_install_6`, `mixed_install_12` | 3 | A synthetic provider local was generalized before its provide. Item 1's contract/value fusion restores the original inference point. Fresh checks now have main's multisets: one E0505 for the recursive case; respectively three/two E0515 and one E0501 for the mixed cases. No error code disappeared. |
| K4d b1 in both copies | 2 | The two distinct main errors remain: an empty same-key service intersection and incompatible provider arguments. The migrated joint provide also reports the reverse-ordered empty intersection at the combined scope. This is an additional report of the same contradiction, not a new rejection condition. §7.3 permits consecutive direct providers to merge into one provide; §4.5 rule 4 / §9.2 still require the incompatible service forms to reject. |
| A1 mixed_headers 03 in its two copies | 2 | The misplaced header still reports E0115. Its old clause block is now an ordinary computation block (§2.2), invalid on the right of `\|>` (§2.11); E0111 and an arrow-expression E0101 are additional recovery diagnostics. |
| A1 param_shadow 29 in its two copies | 2 | The old binding-head type parameters are forbidden by §2.16 / §3.2. E0101 now rejects `outer(A) =` before the old parser's end-of-block E0105. The parameter-shadowing body is not reached by either invalid program. |
| A1 ungrouped_product 48 and tuple_clauses_joined 53, each in two copies | 4 | Removed clause blocks are computation blocks, not chain literals (§2.2 / §2.5). The product still has E0111 and also an invalid expression arrow; the tuple case is E0111/E0102 at the invalid pipe/body instead of the old discard-as-value E0106. The bare old clause form never becomes accepted. |
| A1 old_guard_lines 56 in its two copies | 2 | `case`, `if`, and the old brace clauses no longer form a match. Under §2.5 / §3.3 the free `\|` must join function literals; recovery also reaches the arrow and `_` in expression positions. E0101 remains, with a further E0106. |
| A1 capture and its five confirmed/loose copies | 6 | Their intentionally invalid `src/types.i` still contains `export (Box(..))` and `Box { Box(Int) }`. E0101 remains at the old export; §3.2's removal of constructor lists adds a second E0101 at `Box {`. The converted caller is not the rejection's cause. |
| K4d typediff_41_16 and typediff_46_85 | 2 | Both retain the invalid `/* … */` delimiters. The old diagnostic dump between them is now line-commented, eliminating secondary errors in that dump while preserving the unsupported opening delimiter's E0101. Spec §1.2 defines only `//` comments; no block-comment feature was added, and no original error code was lost. |
| K4s mut_ord_tmp | 1 | The original illegal typed binding in an expression position is retained as `run(h1 : _ = …)`. §2.3 / §3.3 read a call's data items, where `:` cannot introduce a label or a binding. It fails at that item, rather than producing two legacy pattern-recovery E0101 reports. |
| Native3 alias/direct/external/int_assign/raw_ctor | 5 | The retained old `import(…)` / `export(…)` text now parses as ordinary calls with data arguments (§3.3), not header groups (§7.3). Resolver E0234 rejects those top-level macro-reserved calls; E0201 reports the unbound module/value that the call did not import. The main E0101 is therefore completely replaced by later-phase E0234/E0201; these programs remain rejected. |
| Native3 field/injection_control/positive; std1 stream/stream-positive | 5 | P0 (§3.5) explicitly makes bare `!` and `^` lexer errors, so retained old service/error syntax adds E0001 before recovery. Main's E0101/E0102 do **not** disappear completely in these cases: the streams have E0101×5/E0102×3 → E0101×11/E0102×5 plus E0001×10 each; field, injection_control and positive retain respectively 2, 7 and 1 E0101, adding 1, 3 and 1 E0001. These are removed-token errors, not generic substitutes for a dropped type error. |
| Rec loose/probes | 1 | Conversion dropped the illegal type spread `..R`, which incorrectly removed E0541. Restore that syntax so the closed-data rule is tested rather than silently deleting its counterexample. Under §2.3, “A data type has no spread”; syntax three diagnoses it in the parser (E0107), before resolution. Thus E0541 is replaced by the direct no-type-spread error. The original E0210 import rejection is independently pinned by `library::unknown_library_path_is_missing` for `i/record`, and both E0202 lookups by `front_end::closed_data_has_no_record_type_even_without_a_spread_error`; all 15 tests in these two binaries pass. The existing corpus requirement now names E0107 and documents that split, keeping all three original negative-test intents. |
| Runtime echo-types | 1 | Its old `import(task = …, net = …, i/fail, i/resource)` is read as a call: two positional items follow labels, producing E0110 under §2.3. Its old `case` clauses still have invalid expression arrows (§2.5 / §3.3), while the stub task module has two forbidden lowercase data-definition heads (§3.2). These separately reached syntax errors explain the additional diagnostics; no run phase is enabled. |
| S6 probe; std1 factory/tree/variance | 4 | Lowercase data-definition heads remain forbidden. §3.2 requires bare UpNames in those heads; the new parser rejects the head as E0101 before resolution. Consequently main's E0232, free lowercase E0201 and consequent arity E0224 disappear together. This is earlier enforcement of the same invalid head, not removal of those resolver checks from valid parsed programs. |
| T1 assert_group | 1 | The fitted pattern `(\"a\", _)` is impossible for the known first part `\"b\"`. Section 2.12 requires the warning when the known type is disjoint from the fitted pattern. W0505 is the documented behaviour change 1 above; `assertion.catch` still catches the same Assert, and the program remains accepted. |

The counts above sum to 120. The disappeared E0541 and the genuinely replaced
E0101 in the five native-call cases have distinct causes; they are not
classified together as display changes. The stream cases retain both of
their original parser codes, as the full diagnostics show.

After restoring the sources, all 120 check exits equal main, and all 79
restored-contract diagnostic multisets equal main exactly. The three
provider-generalization cases also match, so 82 of the 120 original code
changes are repaired; the remaining 38 are the individually explained
parser, duplicate-provider and warning groups above. Both main and converted
positive controls (temporary permutation copies without `bad` or `loop`)
check successfully with the same two unused-binding warnings. The real
fixtures retain both negative functions. Raw final checks and controls are
in target/conv-b15/final-code-observations.json and positive-controls.json.
No expected.json was re-recorded. The deliberate no-type-spread fixture is
registered with its exact E0107 in the snippet inventory and the converter's
unconvertible list, so conversion cannot silently erase it again.

Authority for all restored needs, §7.3: “A written `! N` that is wider than
what the body needs is kept as a type annotation on the binding
(`f : (P) <- … -> R = …`), so that the type does not change.” The literals also
retain their original empty error channel through the transition lowerer's
`Fail(Never)` marker. The restored type spread follows §2.3: “A data type has
no spread.” The old removed-library/type checks were split out, not dropped.

Final item-6 verification also passes workspace all-target/all-feature clippy,
normal snippet inventory (4,766 snippets, five unchanged future-track entries)
and Lisp inventory (34 files and 89 CLI macro fragments). The inventory first
exposed an unexpanded Rust format template in the new provider regression;
literal concatenation now produces the same i programs while exposing valid
literal snippets to the inventory. Its focused checker regression passes.
No tests were parked. The coordinator's final whole-corpus recording and
merged-gate run are outside this read-only anomaly audit.
