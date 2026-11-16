# i for Zed

1. Install Zed and a Rust toolchain. The reference Zed currently compiles dev
   extensions with `wasm32-wasip2`; make that target available (with rustup,
   `rustup target add wasm32-wasip2`; with Nix, add it to the toolchain).
   Zed downloads wasi-sdk for the tree-sitter parser, or uses `WASI_SDK_PATH`.
2. In the repository development shell, run `scripts/dev-editors build` to build
   `target/release/i` (or run `scripts/dev-editors` to set up both editors).
3. Commit and push grammar changes before installing. Run
   `scripts/dev-editors zed` from the repository. This copies the extension into
   `target/editor-dev/zed` and writes the current commit of this checkout as the
   copy's `commit`, keeping `path = "editors/tree-sitter-i"`. The manifest still
   names the public repository, so Zed fetches that commit from it and needs the
   commit to be reachable there: push before installing.
4. In Zed open Extensions, choose **Install Dev Extension** (command palette:
   **extensions: install dev extension**) and select the printed directory.
5. Open the repository root and an `.i` file. Zed starts **i LSP**
   using `.zed/settings.json` and formats on save. Inspect Zed's language server
   logs if startup fails. `target/release/i lsp` must be executable.

The tracked manifest names the public repository and one of its commits, so a
direct install of `editors/zed` works for anyone with a clone of that repository.
The pinned commit is `ff9ea00ac` ("Editors highlight & as the chain separator"),
the last change to the grammar in `editors/tree-sitter-i`.
The preparation command instead makes a copy under `target/editor-dev/zed` that
works at any checkout path, including a linked worktree, without changing
tracked files: it rewrites the copy's `commit` to the current commit of this
checkout. The copy keeps the public repository, so the commit must be reachable
there for Zed to fetch it; push grammar changes before preparing the copy. Zed
clones and checks out the grammar commit rather than reading uncommitted files.
`commit` is Zed's supported alias for `rev`; both accept a Git revision the
remote can serve, not a directory or a `file://` URL used as a revision, and not
`HEAD` or a branch name. After a grammar change, commit it and update `commit`
to that grammar commit in a later commit (a commit cannot name itself), then
rerun `scripts/dev-editors zed` for local rebuilds. Grammar source lives in the
same repository; no published tree-sitter repository is needed.

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
