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

The current baseline is `e975c5e93`, merged with all e4 code and deletions
preserved. The cleanup reused 1,497 exact contextual entries from its previous
map; 70 stale entries were withheld and reviewed against the new representation.
The regenerated map below replaces the earlier map, rather than forcing stale
text onto new code. Conflict decisions are recorded in
[the merge review](doc-comments-e4-merge.md).

| Scope | Before e4 cleanup | After |
| --- | ---: | ---: |
| Rust comment section marks | 1,381 | 0 |
| Rust comment `spec.md` paths | 5 | 0 |
| Rust comment lines with either reference | 1,238 | 0 |
| `i/` and `lib/` comment section marks | 78 | 0 |
| `i/` and `lib/` comment reference lines | 72 | 0 |
| Additional checker-fixture section marks / lines | 6 / 5 | 0 / 0 |
| Stale-comment detector lines | 584 | 0 |
| Dormant Python recipe code lines / files | 231 / 5 | 0 / 0 |
| TODO/FIXME/XXX comments | 0 | 0 |

All 154 Rust string literals containing section marks remain exact: 134
single-line diagnostic message occurrences, nine test/assertion strings, and
11 multiline embedded programs. They contain 189 section marks in total.
There are no retained language-rule references or TODOs in audited comments.
The migrator usage example still names its input format twice; both occurrences
are protected example text and remain byte-identical.

The application below reproduces the reviewed result from `e975c5e93`. Each map
entry contains the exact old text, its SHA-256 hash and two surrounding lines of
context. All entries are checked before any file is written. Missing files,
ambiguous matches, bad hashes, changed tokens, changed fixture directives and
changed Rustdoc examples are errors. Already-applied maps are stale too.

```sh
base=e975c5e93
python3 scripts/test-doc-comments.py
scripts/doc-comments apply --all
scripts/doc-comments check --proof "$base" --i-bin target/release/i
```

The release binary must already exist in this worktree. The i proof invokes it
through `/home/agent/Programming/i/.rd/cache/heavy`; do not run that proof alongside
another heavy command in this lane. It writes comment-free copies into a temporary
directory, converts both revisions with `i fmt --to is`, compares every result,
and removes the temporary directory. It never copies the binary.

For a tree where the cleanup is already applied, check replay without writing:

```sh
scripts/doc-comments apply --all --check-replay e975c5e93
```

On a later tree, retain its pre-application revision. Run `apply --all`; inspect
and adapt each refused entry against the actual code before retrying. Do not
force a match from a deleted file or an obsolete representation. Once the
reviewed edits are applied, regenerate the map from that revision:

```sh
scripts/doc-comments record --base "$base" \
  --output scripts/doc-comments-rewrites/final-tree.json
scripts/doc-comments apply --all --check-replay "$base"
scripts/doc-comments check --proof "$base" --i-bin target/release/i
```

The proof lexes every existing tracked Rust file, plus `.i` and `.is` files under
`i/` and `lib/`, plus the three checker fixtures listed in `EXTRA_I`. Tokens
include exact string and character literals; Rust whitespace is ignored, while
i line breaks are retained. A separate comparison
keeps all seven fenced or indented Rustdoc example blocks byte-identical, including
their fence attributes. The migrator's fenced command-usage example retains two
mentions of its input format; it is protected example text rather than stale
implementation prose. No language-rule references or TODO/FIXME/XXX comments
remain in the audited sources. Library documentation's obsolete signature copies
and dormant Python recipes were removed; registered tests and executable test
bodies are unchanged. The map preserves the exact removed recipe text.

These are the checking commands for the e4 merge. Heavy commands run
sequentially. `check-snippets` acquires the wrapper for its own build and is run
directly in the development environment. The submodule was initialized before
the first build; it remains at the revision required by e4.

```sh
H=/home/agent/Programming/i/.rd/cache/heavy
"$H" direnv exec /home/agent/Programming/i cargo fmt --all
scripts/doc-comments scan --base e975c5e93
scripts/doc-comments scan
scripts/doc-comments record --base e975c5e93 \
  --output scripts/doc-comments-rewrites/final-tree.json
scripts/doc-comments apply --all --check-replay e975c5e93
python3 -I scripts/test-doc-comments.py
"$H" direnv exec /home/agent/Programming/i cargo build --locked --release -p ilang-cli --bin i
"$H" direnv exec /home/agent/Programming/i env RUST_TEST_THREADS=2 cargo test --locked -p ilang-binaryen -p ilang-check -p ilang-cli -p ilang-host -p ilang-lsp -p ilang-macro -p ilang-package -p ilang-runner -p ilang-syntax -p ilang-wasm --lib --bins
"$H" direnv exec /home/agent/Programming/i env RUST_TEST_THREADS=2 cargo test --locked -p ilang-syntax-migration --lib
"$H" direnv exec /home/agent/Programming/i env RUST_TEST_THREADS=2 cargo test -p ilang-syntax --test format --test lisp_syntax --test lisp_syntax3 --test lisp_definition_rows --test syn3 --test syn3_lower --locked
"$H" direnv exec /home/agent/Programming/i env RUST_TEST_THREADS=2 cargo test -p ilang-check --test library --test query --test coalescing --test row_schemes --test generic_services --test rest_services --locked
"$H" direnv exec /home/agent/Programming/i env RUST_TEST_THREADS=2 cargo test -p ilang-cli --test doc_comments --test check --test provider_diagnostics --test rest_services --test wasi --locked
"$H" direnv exec /home/agent/Programming/i cargo fmt --all --check
"$H" direnv exec /home/agent/Programming/i cargo clippy --workspace --all-targets --locked -- -D warnings
"$H" ./target/release/i fmt --check lib std bench examples
"$H" ./target/release/i check --project std
scripts/doc-comments check --proof e975c5e93 --i-bin target/release/i
direnv exec /home/agent/Programming/i scripts/check-snippets
python3 -I scripts/check-snippets-test
```

The shared worker rules reserve the full workspace test run, unfiltered CLI
conformance suite and review corpus for the coordinator. This lane runs library and binary tests for all ten changed workspace crates,
library tests for the excluded migration helper, and the focused integration
binaries above.
No corpus expectation changes relative to e4: every string literal, embedded
fixture, executable token and fixture directive is preserved. The coordinator
runs the combined e4/cmx2/fmtgap gate. The initial cleanup's standard-library O0
and O2 tests and documentation example/build checks are recorded in the earlier
lane report; they are not substituted for the new merge checks.

An initial expanded unit command also selected the excluded migration helper's
binary targets with `-p ilang-syntax-migration --lib --bins`. Its
`migrate-syntax3-pipe` binary fails to compile because
`support/legacy/parser.rs` constructs `Stmt::Env` without `group`/`spread` and
`Param::Env` without `rest`. Its executable tokens are identical to e975c5e93
and it is outside the workspace targets. The comment-only lane preserves it. The adjusted
commands above test all ten workspace packages' library/binary targets and the
migration helper's library. The excluded helper's binary tests could not run.
