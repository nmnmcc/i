# i for Zed

1. Install Zed and a Rust toolchain. The reference Zed currently compiles dev
   extensions with `wasm32-wasip2`; make that target available (with rustup,
   `rustup target add wasm32-wasip2`; with Nix, add it to the toolchain).
   Zed downloads wasi-sdk for the tree-sitter parser, or uses `WASI_SDK_PATH`.
2. In the repository development shell, run `scripts/dev-editors build` to build
   `target/release/i` (or run `scripts/dev-editors` to set up both editors).
3. Commit grammar changes before installing. Run `scripts/dev-editors zed` from
   the repository. This copies the extension into `target/editor-dev/zed` and
   writes an absolute `file:///…` Git URL for this checkout, an exact commit SHA,
   and `path = "editors/tree-sitter-i"` in that copy's manifest.
4. In Zed open Extensions, choose **Install Dev Extension** (command palette:
   **extensions: install dev extension**) and select the printed directory.
5. Open the repository root and an `.i` file. Zed starts **i Language Server**
   using `.zed/settings.json` and formats on save. Inspect Zed's language server
   logs if startup fails. `target/release/i lsp` must be executable.

The tracked manifest uses the canonical `/home/agent/Programming/i` checkout and
Git revision `HEAD`; the preparation command makes it work at any checkout path,
including a linked worktree, without changing tracked files. Zed's builder fetches
and checks out the grammar revision, rather than reading uncommitted source from
that path. `commit` is Zed's supported alias for `rev`; both accept a Git revision,
not a directory or `file://` URL. Grammar source lives in the same repository;
no published tree-sitter repository is needed.

After extension or grammar changes, commit, rerun `scripts/dev-editors zed`, then
use **Rebuild** for the dev extension on Zed's Extensions page. Reinstall the dev
extension if it is not already registered. The copied queries and Rust source
are refreshed by the preparation command.

For another project, omit `lsp.ilang.binary.path` to find `i` through the
worktree's PATH, or set it to an absolute path. Relative settings are explicitly
joined to `worktree.root_path()` by this adapter. The server always receives
`lsp` as its argument. The extension uses `zed_extension_api = 0.1.0`, matching
`gleam-lang/zed-gleam` at the reference revision.

The repository check uses `cargo check --target wasm32-wasip1` when that target is
installed and native `cargo check` otherwise, as required by the editor lane.
That compile check is distinct from Zed's current dev installer target above.

Prior art: Zed `docs/src/extensions/{developing-extensions,languages}.md`,
`crates/extension/src/extension_builder.rs::compile_grammar/checkout_repo`,
`extension_manifest.rs::GrammarManifestEntry`, and the Gleam extension's
`src/gleam.rs::language_server_command` and `languages/gleam/` queries. Shallow
reference revisions are recorded in `.rd/ref/INDEX.md` by the editor lane.
