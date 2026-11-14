# Applying the comment cleanup

This cleanup implements the owner's request to remove incidental rule citations
and obsolete implementation history. It changes existing source comments only.
The new Python tool and its enforcing test are support for checking and replaying
that change; no existing compiler or i token changes.

The lexers originate in `lane/cmx` (`dbace0402`). Of that lane's explicit
rewrites, 266 still matched this tree after citation removal. Those were reviewed
alongside the remaining comments. The old diagnostic-message map was deliberately
excluded: diagnostics are observable output and this cleanup preserves every
string literal, including embedded i and WAT programs.

The baseline audit found 1,204 Rust comment lines with 1,343 section marks
and five `spec.md` paths; all are gone. In `std/` and `lib/`, 68 comment lines
contained 74 section marks; all are gone. The stale-comment detector found
329 prose lines before and zero after. The two unchanged format-name mentions
inside the migrator's protected usage example are counted separately. No
TODO/FIXME/XXX comments existed or were retained. Five dormant Python recipes
contained 231 commented code lines; the recipes and their run instructions are
removed, with their invariant descriptions retained.

All 136 Rust string literals containing section marks remain exact: 116
single-line diagnostic message occurrences, nine test/assertion strings, and
11 multiline embedded programs. They contain 169 section marks in total.
There are no retained language-rule references in audited comments: the
constraints are stated directly where needed.

The application below reproduces the reviewed result from `5307a664e`. Each map
entry contains the exact old text, its SHA-256 hash and two surrounding lines of
context. All entries are checked before any file is written. Missing files,
ambiguous matches, bad hashes, changed tokens, changed fixture directives and
changed Rustdoc examples are errors. Already-applied maps are stale too.

```sh
base=5307a664e
python3 scripts/test-doc-comments.py
scripts/doc-comments apply --all
scripts/doc-comments check --proof "$base" --i-bin target/release/i
```

The release binary must already exist in this worktree. The i proof invokes it
through `/home/agent/Programming/i/.rd/cache/heavy`; do not run that proof alongside
another heavy command in this lane. It writes comment-free copies into a temporary
directory, converts both revisions with `i fmt --to is`, compares every result,
and removes the temporary directory. It never copies the binary.

For a tree where the cleanup is already applied, verify replay without writing:

```sh
scripts/doc-comments apply --all --verify-replay 5307a664e
```

On a later tree, retain its pre-application revision. Run `apply --all`; inspect
and adapt each refused entry against the actual code before retrying. Do not
force a match from a deleted file or an obsolete representation. Once the
reviewed edits are applied, regenerate the map from that revision:

```sh
scripts/doc-comments record --base "$base" \
  --output scripts/doc-comments-rewrites/final-tree.json
scripts/doc-comments apply --all --verify-replay "$base"
scripts/doc-comments check --proof "$base" --i-bin target/release/i
```

The proof lexes every existing tracked Rust file, plus `.i` and `.is` files under
`std/` and `lib/`. Tokens include exact string and character literals; Rust
whitespace is ignored, while i line breaks are retained. A separate comparison
keeps all seven fenced or indented Rustdoc example blocks byte-identical, including
their fence attributes. The migrator's fenced command-usage example retains two
mentions of its input format; it is protected example text rather than stale
implementation prose. No language-rule references or TODO/FIXME/XXX comments
remain in the audited sources. Library documentation's obsolete signature copies
and dormant Python recipes were removed; registered tests and executable test
bodies are unchanged. The map preserves the exact removed recipe text.

This is the full verification command sequence. Run heavy commands sequentially.
`check-snippets` and the documentation generator/build scripts acquire the heavy
wrapper for their own Rust builds and standard-library checks, so run those
scripts directly in the development environment. `I_SNIPPET_PARSER` avoids another
build of the parser driver after `check-snippets` has built it.

```sh
H=/home/agent/Programming/i/.rd/cache/heavy
git submodule update --init crates/ilang-binaryen/binaryen
"$H" direnv exec /home/agent/Programming/i cargo build --locked --release -p ilang-cli --bin i
"$H" direnv exec /home/agent/Programming/i cargo check --workspace --all-targets --locked
"$H" direnv exec /home/agent/Programming/i cargo clippy --workspace --all-targets --locked -- -D warnings
"$H" direnv exec /home/agent/Programming/i cargo fmt --all --check
"$H" direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test doc_comments --locked
"$H" ./target/release/i fmt --check std lib examples
"$H" ./target/release/i test --project std --opt 0
"$H" ./target/release/i test --project std --opt 2
direnv exec /home/agent/Programming/i scripts/check-snippets
python3 -I scripts/check-snippets-test
"$H" direnv exec /home/agent/Programming/i cargo test -p ilang-syntax --test spec --test format --test lisp_syntax --test lisp_syntax3 --test lisp_definition_rows --test syn3 --test syn3_lower --locked
"$H" direnv exec /home/agent/Programming/i cargo test -p ilang-check --test library --test query --locked
(cd docs && I_BIN="$OLDPWD/target/release/i" I_SNIPPET_PARSER="$OLDPWD/target/debug/examples/parse-snippets" direnv exec /home/agent/Programming/i npm run check:examples)
(cd docs && direnv exec /home/agent/Programming/i npm run build)
```

The shared worker rules reserve `cargo test --workspace`, the unfiltered CLI
conformance suite and the review corpus for the coordinator. This lane runs the
focused tests above. The coordinator must run the workspace and corpus gates;
no corpus expectation is intentionally changed. The only two recorded review
outputs mentioning library paths name `std/src/test.is`, which is unchanged.
