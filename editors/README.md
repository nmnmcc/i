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

## Language server

`i lsp` speaks the Language Server Protocol over standard input and output. It
checks each package with an open document as a whole, with the text of the open
documents in place of the files, and answers from that check. Every feature
works on both surfaces, `.i` and the Lisp surface `.is`, and on documents with
syntax errors (from the parser's recovered tree).

| Feature | Request | Notes |
|---|---|---|
| Diagnostics | `publishDiagnostics` | With related information, `data` for the quick fixes, and the `unnecessary` tag on unused bindings. Several packages at once; a change re-checks only the packages that have the file. Problems of closed new documents and deleted files are cleared. |
| Hover | `textDocument/hover` | The inferred type and the documentation comment; the value of a literal constant, the declared type of an annotated name, the errors and the environment of a function, and for a macro call the macro and its expansion. |
| Go to definition | `textDocument/definition` | Through imported names, import paths, module names, macro calls (to the macro), into dependencies and the library `i` (written to a private directory). |
| Go to type definition | `textDocument/typeDefinition` | The custom types of the known type of a value, of a constructor or a field. |
| Go to implementation | `textDocument/implementation` | Of a type: its constructors, and the environment statements `! T = e` that give a service. |
| Find references | `textDocument/references` | Top-level names, constructors, types, type parameters, field labels (also in calls and constructor patterns), modules, and local names, in all modules and tests of the package. |
| Document highlights | `textDocument/documentHighlight` | The same, in the document; the definition is a write. |
| Rename | `textDocument/prepareRename`, `textDocument/rename` | The same names, in every file of the package. Refused for names of dependencies, of the library `i`, built-in types, and names made by macros; a new name of the wrong kind, or one that would cover or be covered by another name, is refused with a message. Renaming a module name renames its import. |
| Completion | `textDocument/completion`, `completionItem/resolve` | By place: module members after `m.`, fields after `v.`, constructor patterns in a match (snippets), labels in a call, module paths and their exported names in the imports, services after `!`, types in a type. Each item has its type as detail; documentation is resolved. Names of modules that are not imported come with the edit that imports them. |
| Signature help | `textDocument/signatureHelp` | Functions and constructors, with labels and the active parameter (by place or label). |
| Code actions | `textDocument/codeAction` | Import a missing name or module; remove an unused binding or import; add the missing clauses of a match; add a type annotation from the inferred type (`.i` only). Converting between the surfaces is `i fmt --to`, not a code action. |
| Inlay hints | `textDocument/inlayHint` | The inferred types of unannotated bindings and parameters, not of literals. |
| Folding, selection | `textDocument/foldingRange`, `textDocument/selectionRange` | Imports, items, blocks, matches; the nested nodes of the resolved tree. |
| Symbols | `textDocument/documentSymbol`, `workspace/symbol` | Top-level definitions; the workspace search covers the checked packages, not their dependencies. |
| Document links | `textDocument/documentLink` | Each import path links to its module. |
| Semantic tokens, formatting | `textDocument/semanticTokens/full`, `textDocument/formatting` | As `i fmt`. |

A request that waits for a check is answered when the check is done; a newer
request of the same kind for the same document supersedes it (`ContentModified`),
and `$/cancelRequest` cancels it.

The VS Code client watches `i.toml`, `*.i` and `*.is` files and needs no other
setting; inlay hints follow `editor.inlayHints.enabled`. The Zed settings of this
repository turn on inlay hints for i, which Zed leaves off by default. Zed has no
language for `.is` files yet, so its server runs for `.i` documents only.
