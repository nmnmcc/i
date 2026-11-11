# Editing i in this repository

VS Code and Zed cannot silently install an unpublished extension from workspace
settings. The checked-in settings select the local compiler and formatting once
the extension is installed. `.vscode/extensions.json` intentionally has no
Marketplace recommendation: it cannot recommend a filesystem extension.

In the project development shell, run:

```sh
scripts/dev-editors
```

This builds `target/release/i`, installs the VS Code client's npm dependencies,
links `editors/vscode` into `~/.vscode/extensions/ilang.ilang-0.1.0`, and prepares
`target/editor-dev/zed` with this worktree's grammar URL and committed revision.
It refuses to overwrite an existing extension. Set `VSCODE_EXTENSIONS_DIR` for a
custom VS Code extensions directory. Reload the VS Code window after linking.
The link follows edits to this repository's client and TextMate grammar.

**The manual installation step:** in Zed run **extensions: install dev extension**
and select the printed `target/editor-dev/zed` directory. See
[Zed setup](zed/README.md) for prerequisites and rebuild instructions.

Alternatively, open this repository in VS Code and run the **Extension Development
Host** launch configuration (F5). Its prelaunch tasks build the compiler and
install client dependencies; the new window loads `editors/vscode` directly.
No link or Marketplace installation is needed for this development window.

Both workspace settings run `target/release/i lsp`. The VS Code client resolves
`./target/release/i` against the first workspace folder. The Zed adapter resolves
`target/release/i` against the current worktree root explicitly; it does not rely
on Zed's extension working directory or `${workspaceFolder}` interpolation.
Open the repository root as the workspace. Both enable LSP formatting on save;
the i server advertises `textDocument/formatting`.

Individual setup commands are `scripts/dev-editors build`, `vscode`, and `zed`.
A custom Cargo target directory is overridden for the compiler build so that the
workspace paths remain correct. No compiler or editor installation is attempted
when opening the repository.

The tree-sitter grammar covers the branch's dual syntax (parenthesised and brace
records, `^` and legacy `?` errors), while pipes/matches remain `|>`. There are no
reserved keywords. [Highlight classes](tree-sitter-i/README.md) are shared with
the TextMate grammar.

References: [Zed extension development](https://zed.dev/docs/extensions/developing-extensions),
[Zed languages](https://zed.dev/docs/extensions/languages),
[VS Code extension debugging](https://code.visualstudio.com/api/get-started/your-first-extension),
[VS Code extension installation](https://code.visualstudio.com/docs/configure/extensions/extension-marketplace).
