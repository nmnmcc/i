# STR step 01

No language or library surface for state was added.

Implemented cooked/raw/multi-line interpolation before the core tree, literal-only diagnostics, string prefix/suffix/combined patterns, coverage and lowering, normal/Lisp round trips, semantic tokens, tree-sitter and editor highlighting. Spec cases use `main`, leaving test attributes untouched. Existing CLI usage strings, HTTP URL assembly and the hello example now use interpolation; JSON escaping uses raw strings and the filesystem example uses a multi-line literal.

## Prior art and interpretations

- Read `.rd/ref/INDEX.md`, Swift `lib/Parse/Lexer.cpp` (`delimiterMatches`, `advanceIfMultilineDelimiter`, `skipToEndOfInterpolatedExpression`, `getMultilineTrailingIndent`, `validateMultilineIndents`, `lexStringLiteral`, `getEncodedStringSegmentImpl`) and Rust `compiler/rustc_lexer/src/lib.rs` (`raw_string_unvalidated`). Follow their delimiter counting, nested expression lexing and closing-indent approach; no upstream code was copied.
- Ledger USER decisions control existing pipe/match syntax and immutability. Retained `|>` into ordinary clause blocks and used no state feature.
- `docs/design/meta.md` interpolation item controls the core: omit empty text and fold `<>` from the left, keeping `"" <> x` for a sole expression. The existing operator checker enforces String, including when only an expression is present. Macros see the resulting expressions and names.
- The task explicitly requires the formatter to preserve interpolation, whereas meta.md described canonical printing as `<>`. Kept the source formatter's interpolation spelling and canonical AST/Lisp printers' concatenation spelling; clarified that distinction in meta.md.
- Existing spec §1.6 permits physical newlines in ordinary strings; retained that rule. Triple quotes add indentation stripping rather than withdrawing the existing form.
- Indentation is an exact space/tab prefix, following Swift validation. Blank lines are exempt from indentation errors; spaces beyond the stripped prefix remain in the value. The first and final physical line breaks are excluded before escape decoding, so escaped line feeds are retained. Backslash-newline joins only physical lines, after indentation removal.
- A closing delimiter has only indentation before it; following expression punctuation is permitted, as in the task's acceptance `""" )` (without the space). This follows Swift and the acceptance source.
- Raw quote-only strings such as `#"""#` use the single-quote opener when a matching raw close occurs on the opening line, following Swift `advanceIfMultilineDelimiter`; the value of that example is one quote.
- Raw escape activation requires exactly the opening hash count, per the task; closing quotes consume that count, following Rust's raw lexer. Raw multi-line joins use the activated backslash/hash prefix.
- Prefix and suffix must not overlap: `"ab" <> r <> "ba"` does not match `"aba"`. This follows concatenation semantics, Swift-style literal values and the task's single-binder rule. Empty prefix/suffix covers String. Known/inferred binder types remain String (§4.8).
- Tree-sitter uses a serialized delimiter stack and ordinary expression grammar inside interpolation. LSP semantic tokens use the compiler lexer, including exact hash counts and nested expressions. TextMate recognizes arbitrary raw delimiters; the LSP supplies expression highlighting inside raw strings.

## Verification

- `cargo check -p ilang-syntax -p ilang-check -p ilang-wasm -p ilang-lsp --all-targets`: passed (through the shared heavy wrapper and development shell).
- `cargo test -p ilang-syntax --test string_forms --test lisp_encode_deep`: passed, 14 tests, including nested interpolation, canonical/formatter/Lisp round trips, source encoding and exact macro syntax charges.
- Focused `ilang-cli --test spec` run with the new single-file cases: 28/28 passed at O0 and O2. A further package case exercises macros around and inside interpolation; the workspace spec run includes it.
- `cargo test -p ilang-syntax --test string_forms --test format`: passed after fixing duplicated comments between prefix/binder/suffix parts; 6 tests, including comment preservation and repository formatter properties.
- `i run --project tests/spec/6/interpolation_macro --opt 0`: passed; macro calls both around and inside interpolation retain scope and values.
- `i run --project examples/hello` and `i run --project examples/io-fs`: passed (`Hello from i!`, `fs ok`).
- `UPDATE=1 cargo test -p ilang-syntax --test format`: passed, golden layout and repository-wide round-trip/idempotence properties; the skip list records the new intentionally invalid spec cases.
- `cargo test -p ilang-lsp --lib --test protocol`: passed, 8 unit tests and 17 protocol tests, including raw interpolation hover and nonoverlapping UTF-16 semantic tokens for nested expressions/comments.
- `tree-sitter test --grammar-path editors/tree-sitter-i`: passed, 11 corpus cases. Set `XDG_CACHE_HOME=$PWD/target/str-tree-cache` to isolate the parser cache from other concurrently running lanes.
- `npm --prefix editors/vscode test`: passed, 9 tests.
- `scripts/old-syntax-check`: passed, 4,602 extracted sources at the focused run. Intentional negative cases have adjacent markers and exact diagnostics in its registry.
- The task's complete acceptance script is the final gate: release build, O0/O2 output, formatter preservation, rejected Int interpolation, workspace tests, std O0 and old-syntax checks. Its command logs are `/tmp/str-accept-build.log`, `/tmp/str-accept-workspace.log` and `/tmp/str-accept-syntax.log`.

A manual formatter check found duplicate `// prefix` and `// suffix` comments in a string pattern. Structured printing of the literal/binder parts fixes it, and the formatter test checks that each comment occurs once.

Earlier failures corrected during implementation: the first syntax check reported `missing field suffix`; the IR check reported `cannot find function atom` and mismatched match-arm types; the initial string test used the datum-only Lisp parse API; TextMate's standalone regex test rejected dynamic end-pattern backreferences (`invalid backref number/name`), so it now resolves those as TextMate does. Tree-sitter initially needed the development shell's C compiler and regenerated corpus nodes. A broad test run built before the raw quote-only literal addition then read that new fixture and failed `unexpected parse failure: .../raw_string_literal.i`; the final gate recompiles everything.

## Remaining work

None in the STR step's requested scope. No changes to test attributes, no additional worktrees, and no pushes.
