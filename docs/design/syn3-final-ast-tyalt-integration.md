# One-tree integration with main, stage 3 and annotation precedence

Merge target: lane/tyalt 046550bd71040daf219bb078387db7de2f5355cc.
Base: 68b720e9. Previous one-tree green: 5e7ef6e561ae01c7a121811729aa9c3005595cfc.
This integration compiles; the previous full matrix does not prove this merge.
The fresh full gate is pending.

## Changes in deleted or relocated compiler files

The inventory is the complete diff of syntax source paths from the common
base to the target. AST, AST equality/clone and walker files have no incoming
changes. No incoming implementation is silently discarded.

| Incoming change | Native destination / disposition |
|---|---|
| syn3/lower.rs: remove adjacency-based constructor absorption, single-member retry, sum assembly and absorbed-export filtering | facts/pre.rs::sums returns no absorption; the native resolver reads each data definition as its own constructor and each union as an alias. All source/member/export facts are retained. The private lowerer also receives the complete stage-3 diff for the strict differential. |
| syn3/lower.rs: remove typed-provider temporary fusion | Already moved, with the latest alias/import-aware contract classification, to resolve/three.rs and shared resolver code; the private resolver receives the same upstream change. No second syntactic fusion is reintroduced. |
| shape.rs: two annotation fixture strings gain spaces around : | Same assertions and inputs retained in examples/legacy/src/shape.rs with the incoming spacing. |
| syn3/parse.rs: Annot after type colons; binding-line diagnostic helper | Automatically followed the move to src/parse.rs. Native binding resolution emits the same warning over the same pattern-to-value source range. |
| syn3/print.rs: print_annotation, shared assignment/colon layout, recursive function annotation result grammar | All incoming printer code moves to src/print.rs; the root-owned string_literal and record_label helpers remain. |
| syn3/mod.rs: parser diagnostic and print_annotation exports | src/syn3.rs reexports the new root parser/printer functions. The private namespace continues to reexport the production native namespace. |
| legacy/parser.rs, legacy/format.rs, legacy/mod.rs; lib.rs and lisp/mod.rs legacy entry removals | Production remains deleted. The private migration reader is intentionally retained under examples, as design Step 4 requires. No production reader is restored. |
| lexer.rs: unconditional syntax-three tokenization | Already present from S12; incoming version retained, including <- and generic-statement boundaries. |
| lisp/definitions.rs and definition_table.rs: individual prelude constructor definitions and parameter identities | Native §6.8 definitions already provide every incoming name (Bool/True/False/Nil/Result/Ok/Error/Assert/AssertionFailure), with parameter identities by head order. Keep native datum storage and cost preflight; preserve all names. |

The resolver's additional stage-3 changes are ported: whole-sum exports
through local alias/union members; Result's arguments before alias expansion;
Bool's True/False union at the host boundary; outgoing boolean members; and
host member deduplication. HIR-only changes merge directly. Query union-member
navigation, completion, references and implementation lookup retain incoming
behavior over native syntax and HIR. The independent old resolver also receives
the complete incoming diff, retaining all exact HIR/diagnostic comparisons.

## Conflict decisions and diagnostic preservation

- Keep the fourteen syntax positions of syntax-three §6.7 while adopting
  stage 3's distinct Node wrapper and one-constructor nominal types (§2.8).
  Remove the six obsolete position declarations/exports, not any test.
- Keep the native suite expansion's provide form and recursive suite helper;
  wrap every syntax node construction in stage 3's Node constructor.
- Keep both sets of provider diagnostic regressions and all runtime/show_as
  assertions. Annotation formatting can shift source columns; any adjusted
  coordinate must be derived from the incoming formatted source and recorded.
- Two snippet-inventory entries classify the literal diagnostic-path names
  `import` and `export` as non-source. Move each classification to both native
  and private resolver destinations. Remove only the obsolete `import names`
  message classification at deleted lisp/to_ast.rs: that exact message remains
  in the explicitly excluded private historical reader. No expected diagnostic
  entry or error line is removed by this inventory repair.

Controlling paragraphs: syntax-3 §2.8 (one labelled type per data definition),
§5.2.1 D8 (whole-sum export), §6.7/§6.8 (positions/rows), design Step 4
(private migration copy), and current spec §3: “After every `:` that introduces
a type, the grammar reads `Annot`, not `Type`”; spec §1.7 gives colons the same
spacing/layout as equals. The compiler migration adds no further language rule.

## Verification

- Direct rustfmt on all94 changed Rust files: PASS; exact argv in
  target/fa-r3/tyalt-format-argv.json.
- First wrapped workspace all-target check:101, two printer integration
  references (position module and native QuotePattern.datum). Both corrected;
  the grammar-only formatting parse path is also preserved. Second check pending.
- Static snippet inventory:5033 snippets, three stale non-source classifications
  traced and moved/retired as explained above; actual parser gate pending.
- Every workspace check uses:
  `$H direnv exec /home/agent/Programming/i cargo check --workspace --all-targets --locked --message-format=short`.
  H is the shared heavy wrapper. No second heavy job is started while one waits.

10:43: third wrapped workspace all-target check PASS (38.50s after queue).
Second attempt failed only because the private Transition adapter still needs
HashMap after upstream removed it from the simpler lowerer; import restored.
No diagnostic expectation was edited to fix either compile failure.

10:45: format the two conflicted Lisp suite files with
`./target/release/i fmt std/src/test.is tests/review/cases/review-k4s/std/src/test.is`:
PASS (read/print structural check retained). This is the light formatter,
not a compiler/test invocation; the Lisp layout implementation is unchanged
by tyalt. Only line wrapping of the added Node calls changes, before the
fresh Rust/runtime gate. Check-snippets runs directly and owns its one
internally wrapped parser-driver build; no outer heavy wrapper is used.

10:57: check-snippets PASS, invoked directly (first ordinary build then
`--reuse-parser` after inventory repair):5033 extracted literals/blocks,
34 repository Lisp files,94 CLI macro fragments, five unchanged future-track
entries. Initial failure was21 unregistered fragments: seven non-program
Rust templates/substring/builder fragments, five valid expression/attribute/
pattern grammar fragments now supplied their full module context, and nine
intentional negative fixtures with every exact diagnostic pinned and an
adjacent code marker. The differential deliberately compares partial trees
of invalid EXTRA programs. No program string or assertion was removed.

The native-only print/position.rs FunctionResult path now uses the same
assigned/Annot helpers as the incoming full printer, including service
annotation spacing and union grouping. Sixteen renderer test strings receive
only spec §1.7's canonical colon spacing; bit-array format colons are unchanged.
This closes an additional new-reader path absent from tyalt's old-tree base.
Strict workspace clippy passes at11:02 on the current source; the fresh
Rust/runtime matrix follows.

Strict clippy command (exit0,26.03s after queue):
`$H direnv exec /home/agent/Programming/i cargo clippy --workspace --all-targets --locked -- -D warnings`.
The only warning belongs to the excluded private historical printer helper.
Final diagnostic golden audit against tyalt: zero .out/.expect.json differences.

## Follow-up tyalt head f61f812c9

Tyalt advanced during the Rust gate. Its only changes are the provider-test
coordinate fixture/assertions and annotation proof JSON; no production source
changes. Merged without conflicts before CLI, so the complete CLI gate will
compile and execute the new assertions. Current Rust targets are unaffected.
This test-only merge compiles in the complete CLI invocation (11:51,48.18s). The
branch reference will be checked again at the end.

All22 corrections across14 projects preserve every error code and all message
text except source coordinates. An initial audit assuming message text had no
coordinates caught the related `type at 32:25` → `32:26` reference; the complete
audit now checks primary and related source spans. This normalization is an
audit only: providers.rs still compares the exact full resulting strings,
every coordinate and the full error multiset. The reason for every move is
incoming canonical annotation braces/formatting, independently recorded by
tyalt's annotation-stage3-proof.json. No diagnostic is removed or masked.

| Project | Primary before | Primary after | Related source coordinates |
|---|---|---|---|
| review-k4d/_loose/fuzz/e0505/21_4 | `src/main.i:32:49` | `src/main.i:32:50` | 32:25 → 32:26 |
| review-k4d/fuzz/m21 | `src/main.i:24:7` | `src/main.i:24:7` | 22:25 → 22:26 |
| review-k4d/fuzz/m32 | `src/main.i:26:52` | `src/main.i:29:53` | 26:24 → 29:25 |
| review-k4d/fuzz/m32 | `src/main.i:34:9` | `src/main.i:37:9` | unchanged |
| review-k4d/fuzz/m32 | `src/main.i:35:8` | `src/main.i:38:8` | unchanged |
| review-k4d/fuzz/m32 | `src/main.i:35:17` | `src/main.i:38:17` | unchanged |
| review-k4d/fuzz/m32 | `src/main.i:35:8` | `src/main.i:38:8` | unchanged |
| review-k4d/fuzz/m32 | `src/main.i:34:9` | `src/main.i:37:9` | unchanged |
| review-k4d/fuzz/m32 | `src/main.i:34:18` | `src/main.i:37:18` | unchanged |
| review-k4d/fuzz/m32 | `src/main.i:35:17` | `src/main.i:38:17` | unchanged |
| review-k4d/fuzz/w2 | `src/main.i:23:7` | `src/main.i:23:7` | 21:28 → 21:29 |
| review-k4d/fuzz/w4 | `src/main.i:31:46` | `src/main.i:34:47` | 31:22 → 34:23 |
| review-k4d/fuzz/w4 | `src/main.i:59:7` | `src/main.i:62:7` | 57:22 → 60:23 |
| review-k4d/fuzz/w4 | `src/main.i:47:9` | `src/main.i:50:9` | 45:22 → 48:23 |
| review-k4d/fuzz/w4 | `src/main.i:21:7` | `src/main.i:21:7` | 19:31 → 19:32 |
| review-k4d/fuzz/w4 | `src/main.i:28:52` | `src/main.i:31:53` | 28:21 → 31:22 |
| review-k4d/fuzz/w4 | `src/main.i:43:9` | `src/main.i:46:9` | 41:25 → 44:26 |
| review-k4d/fuzz/w4 | `src/main.i:34:7` | `src/main.i:37:7` | 32:25 → 35:26 |
| review-k4d/fuzz/w4 | `src/main.i:39:9` | `src/main.i:42:9` | unchanged |
| review-k4d/fuzz/w4 | `src/main.i:61:6` | `src/main.i:64:6` | unchanged |
| review-k4d/fuzz/w4 | `src/main.i:61:11` | `src/main.i:64:11` | unchanged |
| review-k4d/fuzz/w4 | `src/main.i:55:20` | `src/main.i:58:20` | unchanged |

11:11: strict resolver HIR/diagnostic differential PASS (complete repository
comparison, unchanged assertions), along with deep and prepared-quote cases.
Remaining Rust integration tests continue; this is not the full gate yet.

11:46 full Rust run:760 pass, four existing ignored, requested recovery skip;
four failures confined to lisp_syntax's builtin-definition tests. The conflict
resolution mixed an alias Result(A,E) head with an Ok expectation, and incoming
Ok(A) inputs with the previous Result constructor-list body paths. Corrected
the test adaptations: retain Result's two-parameter alias/Ok-use identity test
(and explicitly assert builtin Ok identity), while the copied/duplicated Ok
cases follow their §6.8 body `(, A)` directly. The authority-forgery test keeps
the one-parameter head shape. No assertion intent is removed, and no compiler
implementation or diagnostic expectation changes. Design §2.8/§6.8 requires
separate one-constructor definitions, not the former sum constructor list.

The syntax retry is selected explicitly alongside all42 sequential CLI test
binaries plus CLI unit targets in one wrapped Cargo invocation; four harness
threads per binary, with only one binary active. This obeys the coordinator's
one-binary-at-a-time option while retaining one heavy command per worker.
The retry also recompiles the incoming f61 provider assertion update before
its merge checkpoint. Initial Rust exit101 remains recorded, not hidden.

11:51: all selected CLI targets and the repaired syntax target compile in
48.18s; CLI library tests pass2/2, main-unit and42 integration binaries follow.
This is the compiling checkpoint for the f61 test-only merge and the four
builtin-test shape repairs. Root Rust remains760 successful tests plus four
pending repaired assertions; no failed compiler diagnostic golden.

11:58 CLI compile_cache: two tests pass, cache_command_reports_collects_and_cleans
fails at its final status assertion. Incoming mechanical annotation commit
6b2cc2221 changed the English UI assertion from `total: 0 entries` to
`total : 0 entries`; the producer in commands/cache.rs remains
`println!("total: {entries} entries, ...")`, unchanged from the old green head.
The captured failing output is exactly `total: 0 entries, 0 B`. Restore that
single assertion's original text; this is status prose, not a type annotation
(spec §1.7 restricts the rule to a colon introducing a type). No production or
diagnostic behavior changes, and cache counts/GC/clean assertions remain.
The first whole CLI invocation retains the failure; run the complete cache
binary again after it finishes. The independent warm/cold/cache-off tests pass.

12:11 read-only formatting gate PASS on the current compiler/source. These
commands only parse/print files and do not compile or run i programs; they
run alongside the single wrapped CLI test job. Every build/test/clippy remains
wrapped and serial. The source formatter is the fresh debug i built by the
current CLI invocation; production code is unchanged since9e2764006.

```sh
direnv exec /home/agent/Programming/i cargo fmt --all -- --check
direnv exec /home/agent/Programming/i cargo fmt --manifest-path crates/ilang-syntax/examples/legacy/Cargo.toml -- --check
./target/debug/i fmt --check lib std bench examples
```

## 12:15 handoff

No fully green new-base hash is available at this checkpoint. The committed
candidate is c257c44a715f18b77a6882c9db94d7e8fbf28e73; production frontend code remains9e2764006. Workspace check,
strict clippy, exact HIR/diagnostic differential, checker/LSP/macro targets,
check-snippets and all formatting checks pass. Four original Rust failures
were mixed Result/Ok test shapes and are repaired/compiled; their complete
syntax binary retry is selected with CLI. CLI reported totals are[211, 1, 2]
(including child-harness reporting), with the sole known cache UI expectation
failure repaired exactly in the candidate. The rest of CLI, cache retry,
release/examples/std O0/O2, four spec chapters, short generation and private
package checks remain pending. Runtime plus shared queue/load consumed the
handoff window; none of the required checks is dropped.

Tyalt now1a6453d33 additionally contains the shared HIR backend environment-
ancestor continuation fix, a new chapter9 regression and raw-line/stop/CLI
fixture corrections. It must be merged and validated before final green.

## Latest merge1a6453d33 (12:16)

Merge applied while the already-compiled first CLI run continues. Its only
conflict was the appended snippet dictionary: preserve all local entries and
apply the three incoming key additions/removal (two substring/prose contexts,
no expected error entry removed). Current CLI output is from the pre-backend-
fix binary and is recorded as such. This merge is staged until compilation.

The shared HIR-only control.rs change retains transitive environment parents
in continuation join parameters, using the same parent walk already used for
frame capture. It has no syntax-tree dependency or deleted-file port. Add its
control unit regression to local validation and compile a fresh binary before
the remaining examples/std/spec/generation. Root frontend/diagnostic code is
unchanged. The new chapter9 continuation_environment_join fixture runs with
that gate. No implementation from a deleted AST path is dropped.

The incoming package_commands assertion changes `io: Io` to canonical
`io : Io` in a template-source search; the active old test binary reports the
old string at its assertion. Retain the exact search against the unchanged
new formatter and rerun that complete binary. Cache-prose correction matches
the already committed local repair.

Raw CR/CRLF fixtures are restored byte for byte; the annotation formatting
script now reads/writes bytes and explicitly excludes three diagnostic-sensitive
spacing/line-ending fixtures from rewriting, while checking equality. These
changes restore lexical test intent; no error line is deleted.

Five incoming stop-coordinate corrections come from canonical type-colon
spacing before the stop expression (all messages and the other frames retained):

| Fixture | Before | After |
|---|---|---|
|11/stop_tail_assert|spec/main:8:23|spec/main:8:24|
|11/stop_tail_library|spec/main:11:21|spec/main:11:22|
|2.5/order_go|spec/main:18:19|spec/main:18:20|
|2.5/stop_in_start|spec/main:11:58|spec/main:11:59|
|9/light_boundary_kept_raise|spec/main:14:31|spec/main:14:32|

These are source-format coordinates, not a relaxed comparison. The chapter9
stop case remains in the requested gate; the other incoming cases are outside
the specified chapters and their exact header edits are disclosed here.

12:25: the exact provider comparison found two required §6.8 presentation
changes in each of mixed_install_12 and mixed_install_6, rather than a lost
diagnostic. Their saved snapshots predate the rows flip. The expansion
changes from `(-> () _ {((-> () _ {(. (bad) {(-> 2 Nil) (-> other
(panic other))})}))})` to `(-> (,) _ {((-> (,) _ (|> (bad) (| (->
(, 2) _ Nil) (-> (, other) _ (panic other))))))})`. This is exactly
syntax-3 §6.8, Expressions and statements, Function/Chain/Pipe rows. The
section 7 migration table says: “Parameters `(a b)` become `(, a b)` and
`()` becomes `(,)`; the pipe `(| e f)` becomes `(|> e f)`”.
The following clause-block/match migration in the same paragraph accounts
for every remaining symbol. In each expansion the panic call begins at
zero-based byte52 before,68 after: the related virtual String-need coordinate
18:72 becomes18:88. These are appended expansion coordinates, not moved
user-source spans. The primary16:1 and16:61 spans stay identical.
All six/seven complete errors and all five/six provider errors remain pinned
exactly; the four added exact correction entries preserve all other text.
This is an explained, design-required representation change, not a new
checker behavior or a normalization of the comparison. Focused verification
of both projects and the full fourteen-project assertion follows.

The body-block difference in the same expansion is also specified by §6.8:
“A function body is any one expression (O20): `{ … }` is one of them, a block,
but a body need not be one.” The native function holds the pipe expression
as its body directly; this accounts for removal of the old extra `{ … }`.
No type constraint, warning filter, diagnostic mapping rule or user program
is changed by this fixture adaptation. The producer remains unmodified.

Additional light audits (PASS): both raw fixtures equal
`git show refs/remotes/origin/main:<path>` byte for byte, with the SHA256s in
the incoming stop-position proof; production old-tree/lower paths remain
absent; existing22 provider corrections remain byte-equivalent parsed JSON;
all four native-row corrections retain primary coordinates and full error
prefixes. Direct `rustfmt` without direnv was unavailable (command not found);
the retry through `direnv exec ... rustfmt --edition 2024 --check
crates/ilang-cli/tests/providers.rs` passes. `git diff --cached --check`
flags the intentionally restored CRLF fixture as trailing whitespace; raw
line endings must stay unchanged, so they are not stripped.

The final gate is serialized in target/fa-r3/tyalt-final-gate.sh. It waits
for the first CLI parent (1374984) to exit before entering any heavy wrapper.
It then runs current workspace check/clippy, every Wasm library unit test
(including the newly merged continuation regression), complete cache/package/
provider binaries, a fresh release compiler, examples, both std levels, all
four requested chapters, short generation, private all-targets/unit checks,
and the three formatting gates. Every command and exit has its own
`tyalt-final-*.command`/`.log` and a results.txt entry. Independent checks
continue if another test fails; compilation/release failures stop dependent
checks. No heavy command overlaps or is queued by this lane while another
heavy command is active. The snippets command remains direct.

12:36 compiling milestone: final workspace all-target check PASS (14.22s),
strict all-target clippy PASS, every Wasm unit test PASS including transitive
continuation environment capture. The complete first CLI invocation exits101
with exactly the cache-prose, canonical-template-search and native-row fixture
failures described above; no other target fails. Syntax library29 and the
complete repaired lisp_syntax46 pass, so the original Rust gate plus repaired
cases totals764 pass,4 existing ignored,1 requested recovery skip. The three
complete CLI binaries are now being rebuilt/rerun against the late backend
and the exact explained fixture repairs. This checkpoint is compiling, not
claimed fully green until the remaining runtime/generation gate finishes.

12:36 latest tyalt c220161d5 merged without conflicts: only the annotation
formatter script and the new stop-position proof. Python AST parsing of the
script passes. An independent audit checks that each of the five old/new
coordinates points to the identical declared statement in origin/main and
this tree, and both raw fixtures match their baseline bytes and proof SHA256.
All seven proof entries pass; no compiler, fixture source or diagnostic is
changed by this proof-only follow-up. The source remains the already compiling
and clippy-green e840c135b frontend/backend. Wasm unit result:27/27 pass.
The first combined CLI/syntax log reports416 passes,3 known failures and2
existing ignored (includes child harness output); full binary failures are
exactly the three listed above. No further target fails.

12:40: all27 package tests PASS; all3 cache tests PASS. The exact
fourteen-project provider diagnostic comparison PASS, including both
mixed-install projects with the four explained row/virtual-coordinate
changes. Every diagnostic count, code, full message and written-source
location remains compared exactly. The final transitive-provider runtime
case is still active; no retry failure has occurred.

12:41: full three-binary CLI retry PASS: cache3, package27, providers19,
including the exact fourteen-project comparison and transitive runtime.
No known failing Rust/CLI test remains. All first-run failures and their
repairs stay recorded. Fresh release build and runtime/spec/generation gate
continue; backend follow-up validation uses the fresh compiler.

12:43: fresh release compiler build PASS (1m32s). Macro examples PASS:
13 passed,0 failed,1 existing skip. Std O0 starts on this fresh binary,
followed by O2 and the requested spec/generation gate. Latest tyalt c220
remains an ancestor; no new incoming changes at this checkpoint.

12:48: std O0 PASS2170/2170, including schema_test.null_round. O2 starts.
Because the first all-CLI invocation compiled before the late backend merge,
all CLI unit targets and every integration binary except the unfiltered spec
suite will additionally be rebuilt/rerun on the merged backend after the
current runtime/spec/generation/private gate. The macro crate is also rerun
because it executes backend-generated guest code. Frontend checker/LSP/syntax
production code has not changed since its full passing gate. The fresh
CLI/macro script waits for parent1923201 to end before entering H, so the
lane never has two heavy commands running or queued.

12:53: std O2 PASS2170/2170, including schema_test.null_round. Both std
levels are green on the merged release compiler (O0 completed12:47:53;
O2 completed12:52:57). Requested chapters6/7/9/15.7 and short generation
follow, then private migration checks and final formatting. Fresh complete
CLI/macro replay remains serialized behind the gate; no failing test is
currently known. This is an interim runtime milestone, not final full green.
