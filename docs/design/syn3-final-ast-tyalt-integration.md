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
