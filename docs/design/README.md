# Design documents

- A fenced block in a design document is `text` by default. Code fences are
  shapes and do not have to compile.
- Mark a fence `i` only when the snippet is meant to be parsed by
  `scripts/check-snippets` and is kept current with the compiler. A snippet
  that needs context it does not carry stays `text`.
- `scripts/check-snippets` error entries belong in `scripts/check-snippets.json`
  only for deliberate diagnostics; do not add a directory whitelist.
