Run `scripts/old-syntax-check` in the development shell. It builds the small
`ilang-syntax` parser driver, using the shared heavy-job slot when run in a
redesign lane. It invokes only the new parser. It never scans whole Rust,
JavaScript or Markdown files for keyword words.

The extractor reads every tracked `.i` file and every fenced `i` block in
Markdown. Its Rust lexer decodes ordinary and raw string literals, skipping
comments and character literals. Its JavaScript lexer decodes strings and
templates, skipping comments. Source-shaped decoded literals are candidates;
`scripts/old-syntax-check.json` records reviewed ambiguous literals, templates
and assembly fragments. Non-i literals such as TOML manifests, ABI messages
and source-position search strings have explicit reasons. They are also sent
to the parser, but their parser result is not an i-source assertion. A changed
or missing registered literal makes its entry stale and fails the check.
Registered literals also force extraction of header-only replacements and
other fragments that do not have the shape of a complete binding.
The fixture application's source is read as `.i`, including its browser inputs.

An intentionally invalid source has `old-syntax-check: E0101,E0108` (its actual
ordered codes) in an adjacent comment. Use `//` in an i file, `/* */` immediately
before a Rust or JS literal, or an HTML comment immediately before an i fence.
The data entry contains the exact ordered codes **and messages** and explains
why the fixture is invalid. The check requires both the adjacent declaration
and the exact diagnostic result. Metadata comments in `.i` files are removed
before calculating their identity and parsing, so they do not hide diagnostics
inside an unterminated literal. The compiler and normal test suites still read
the original files, including the comments.

Later-track examples in accepted design notes have a single entry here, in
`scripts/old-syntax-check.json`, with `track`, `reason` and the current exact
diagnostics. They are always parsed. A successful parse or changed diagnostic
fails the check. Their owning track must remove the entry when its
syntax lands. There is no blanket exemption for any design note or crate.

Templates use explicit substitutions from the actual generator; assembly
fragments state their surrounding source. Normal runtime tests continue to
exercise the full generators. `--inventory` prints the extracted sources and
parser results for review; it does not update expectations or make a failing
check pass. Run `python3 -I scripts/old-syntax-check-test` for the extraction and
expectation regressions.

The inventory driver also reports an unqualified `@main` attribute as E0301;
the parser accepts general attribute syntax, and the checker rejects this name.

The normal-surface inventory also runs `python3 scripts/syn3-lisp-check`.
It can be run separately after the driver has been built.
It uses the same built driver with `--lisp` to parse every tracked `.is` file
and the complete Lisp modules, signatures and function fragments assembled
by the CLI's macro-run and typed-macro tests. Each fragment is put in its
actual declaration or macro-binding context. This catches old Lisp rows
that do not resemble normal-surface bindings; the runtime test binaries
still exercise their typing, expansion and execution.
