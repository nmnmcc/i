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
