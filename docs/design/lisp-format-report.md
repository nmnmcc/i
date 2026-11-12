# Lisp surface formatter report

No language or library surface for state was added.

## Implemented layout

- Both surfaces share `format.rs`'s Wadler document algebra, fitting function,
  renderer, 80-column width and four-column tabs. Lisp builds documents from
  source datums rather than regenerating a flat AST representation.
- A round datum that fits stays together. A broken round datum keeps the head
  and its distinguished first part together: `=` binding name, `->` parameter
  list or clause head, `|` subject, `@` attribute. Other calls keep the callee.
  Remaining parts break one per line, one tab in. A distinguished part that
  cannot fit may itself break.
- Blocks with multiple statements and clause lists with multiple clauses force
  one statement/clause per line. Single-part blocks can stay flat. Lists,
  tuples and bit arrays stay flat when they fit and otherwise break one part
  per line. Broken forms put closing delimiters on separate lines.
- Quote, unquote and splice prefixes attach directly to their datum, except
  where a source comment already intervenes. Comments between `~` and `..`
  are preserved too. Strings and literal spellings are retained, never split.
- Comments belong to source gaps between datums. They retain order and position
  relative to tokens, including inside calls and templates and before closing
  delimiters. Documentation text is retained exactly; ordinary comments lose
  trailing whitespace. Trailing comments stay on the datum's line when they fit
  and otherwise go on the following line in the same gap.
- At most one existing blank line between top-level items/comment groups is
  kept. Output ends in one newline.
- Indentation is capped at 19 tabs, leaving one indentation unit for content.
  A line can exceed the width only for a single indivisible atom/string/comment
  with its surrounding delimiters or reader prefix. Long strings are not split.
- `syntax.show` and the single-line datum renderer are unchanged.

## Prior art read

- Racket `racket/collects/racket/pretty.rkt`: `pp-one-up`, `pp-general`,
  `pp-list`, `pp-down`, and `style`: distinguish a form's leading parts from
  its body and try a flat form before choosing vertical layout.
- Clojure `src/clj/clojure/pprint/dispatch.clj`: `pprint-defn`,
  `pprint-binding-form`, `pprint-let`: logical blocks for distinguished prefixes
  and bodies.
- [cljfmt `core.cljc`](https://github.com/weavejester/cljfmt/blob/master/cljfmt/src/cljfmt/core.cljc):
  `surrounding-whitespace?`, `line-break?`, `count-newlines`,
  `remove-consecutive-blank-lines`, `list-indent`: preserve comment boundaries
  and collapse excess blank lines while formatting source syntax.
  No cljfmt/zprint checkout was present in the reference directory; the primary
  cljfmt source was read online.
- Existing `crates/ilang-syntax/src/format.rs`: `Doc`, `group`, `fits`,
  `render`, `nest` implement the Wadler approach requested by this task.
  They are reused, with an optional indentation limit in that same renderer.

## CLI, editor and verification integration

The CLI already collected both extensions and the LSP already selected
`lisp::format` for `.is`; replacing the flat implementation makes both use
this layout. CLI help now names both extensions. `--to is` passes conversion
output through this printer after the canonical-tree conversion check.

`dev-verify i` invokes `i fmt --check lib std bench examples`. Its directory
collection includes `.is` by default, confirmed by the CLI integration tests;
no script change is necessary. All three tracked `.is` files were reformatted.

The corpus property test visits every repository `.is`, and converts every
parseable `.i` under `std`, `lib`, `examples` and `tests/spec` with the same
comment-preserving encoder used by `--to is`. It checks canonical core-tree
equality, comment preservation, idempotence and line width. Parse-error
fixtures have no tree to convert and are excluded. Focused checks also cover
comment/token order, documentation, blank lines, long Unicode strings and
comments, literal spellings, reader prefixes, CLI selection/check/conversion,
and LSP formatting/idempotence/syntax-error behavior.

## Interpretations

- The task's block/clause rule takes priority over the general flat-datum rule
  for blocks containing multiple statements or clauses: they always break.
- The width rule takes priority when a distinguished prefix itself is too
  large: its own document can break. Deep layouts stop increasing indentation
  at 19 tabs to leave room for brackets; this follows the task's width property.
- The unsplittable-line exception includes comments because preserving comment
  text and order forbids wrapping them into different comments (task rule 1).
- The spec contained no existing `i fmt` sentence in this checkout; the requested
  description was added at the end of §1.7. Meta §2.2 and §9 were updated.
- The old macro-quote formatter test expected AST regeneration to add implicit
  braces. It now expects source-preserving formatting, with the same core tree
  (task rule 2), while the canonical AST printer's separate assertion remains.

## Commands and results

- Heavy-wrapped `cargo check -p ilang-syntax -p ilang-cli -p ilang-lsp --all-targets`: passed.
- Heavy-wrapped `cargo test -p ilang-syntax --test lisp_format`: passed, four tests.
- Heavy-wrapped `cargo test -p ilang-syntax -p ilang-cli --test fmt -p ilang-lsp --test protocol`:
  passed, six CLI and 18 LSP tests.
- `cargo fmt --all`: passed.
- `target/debug/i fmt` on all three tracked `.is` files, then
  `target/debug/i fmt --check std lib examples`: passed.
- Initial property runs failed with `splittable line of width 88` and then
  `splittable line of width 81`; separate closing-bracket lines and the shared
  renderer's indentation limit fixed those cases. Initial test fixtures had
  an unmatched bracket/unquote outside quotation and were corrected.
- One intermediate test compile failed with `no variant, associated function,
  or constant named Backtick`; the correct lexer token is `Quote`.
- The first whole syntax-crate run failed at
  `macro_quotes_are_core_nodes_and_can_be_formatted`; the obsolete
  AST-regeneration expectation was corrected as described above.
- Full acceptance verification: running after the formatting/report commit;
  final results will be recorded here before completion.

## std/src/test.is before and after

Before: 14 lines, maximum 1019 characters.
The requested first 30 lines comprise the whole original file:

```is
(, (= syntax (/ i syntax)) (= int (/ std int)))
(, tests test skip only fails suite each)
(= tests Nil)
(= text (-> (x) _ {(| ((. syntax node) x) {(-> ((. syntax Text) s) s) (-> _ (panic "test name must be a string literal"))})}))
(= _entry (-> (name body mode (! items (. syntax Items))) _ {(= target `generated_test) ((. items add) `tests target [(, "name" (text name)) (, "mode" mode)] name) `(= ~target (-> () _ {(~body)}))}))
(@ macro (= test (-> (name body (! items (. syntax Items))) _ {(_entry name body "test")})))
(@ macro (= skip (-> (name body (! items (. syntax Items))) _ {(_entry name body "skip")})))
(@ macro (= only (-> (name body (! items (. syntax Items))) _ {(_entry name body "only")})))
(@ macro (= fails (-> (name body (! items (. syntax Items))) _ {(_entry name body "fails")})))
(= append (-> (xs ys) _ {(| xs {(-> [] ys) (-> [x .. rest] [x .. (append rest ys)])})}))
(= suite_items (-> (prefix statements env) _ {(| statements {(-> [] []) (-> [statement .. rest] (| statement {(-> `(! ~service ~value) (suite_items prefix rest (append env [statement]))) (-> `(suite ~name (-> () _ {~..children})) (append (suite_items (<> (<> prefix (text name)) " / ") children env) (suite_items prefix rest env))) (-> `((. ~module suite) ~name (-> () _ {~..children})) (append (suite_items (<> (<> prefix (text name)) " / ") children env) (suite_items prefix rest env))) (-> `(~callee ~name ~cases ~body) {(= full ((. syntax make) ((. syntax Text) (<> prefix (text name))) name)) (= wrapped `(-> (case_value) _ {~..env (~body case_value)})) [`(~callee ~full ~cases ~wrapped) .. (suite_items prefix rest env)]}) (-> `(~callee ~name ~body) {(= full ((. syntax make) ((. syntax Text) (<> prefix (text name))) name)) (= wrapped `(-> () _ {~..env (~body)})) [`(~callee ~full ~wrapped) .. (suite_items prefix rest env)]}) (-> _ (panic "suite body expects tests, suites, and environment statements"))}))})}))
(@ macro (= suite (-> (name body) _ {(| body {(-> `(-> () _ {~..statements}) `{~..(suite_items (<> (text name) " / ") statements [])}) (-> _ (panic "suite body must be a zero-argument function"))})})))
(= case_items (-> (prefix cases body index (! items (. syntax Items))) _ {(| cases {(-> [] []) (-> [case .. rest] (| case {(-> `(, ~name ~value) {(= full ((. syntax make) ((. syntax Text) (<> (<> prefix " / ") (text name))) name)) (= target ((. syntax make) ((. syntax Name) (<> "generated_test_" ((. int to_string) index))) `generated_test)) ((. items add) `tests target [(, "name" (text full)) (, "mode" "test")] name) [`(= ~target (-> () _ {(~body ~value)})) .. (case_items prefix rest body (+ index 1))]}) (-> _ (panic "each case must be a (name, value) tuple"))}))})}))
(@ macro (= each (-> (name cases body (! items (. syntax Items))) _ {(| cases {(-> `[~..values] `{~..(case_items (text name) values body 0)}) (-> _ (panic "each expects a list of cases"))})})))
```

After: 298 lines, maximum 72 characters.
First 30 lines:

```is
(, (= syntax (/ i syntax)) (= int (/ std int)))
(, tests test skip only fails suite each)
(= tests Nil)
(= text
	(-> (x)
		_
		{
			(| ((. syntax node) x)
				{
					(-> ((. syntax Text) s) s)
					(-> _ (panic "test name must be a string literal"))
				}
			)
		}
	)
)
(= _entry
	(-> (name body mode (! items (. syntax Items)))
		_
		{
			(= target `generated_test)
			((. items add)
				`tests
				target
				[(, "name" (text name)) (, "mode" mode)]
				name
			)
			`(= ~target (-> () _ {(~body)}))
		}
	)
```

## Unfinished

Acceptance verification is in progress; no implementation behavior is deferred.
