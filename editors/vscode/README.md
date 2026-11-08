# i for Visual Studio Code

Editor support for **the i programming language** (iLang).

## Features

- Syntax highlighting for `.i` source files: comments and doc comments (`//`,
  `///`, `////`), strings with escapes (including `\u{...}`), integer and float
  literals (`0x`, `0o`, `0b`, `_` separators, exponents), keywords, types and
  constructors, module qualifiers, labelled arguments, attributes such as
  `@external(wasm, "module", "fn")`, bit arrays and operators.
- Bracket matching, auto-closing pairs, comment toggling and indentation.
- Language server integration: diagnostics, hover, go-to-definition and
  whatever else `i lsp` provides. The server is notified when `.i` files or
  `i.toml` manifests change.

## Requirements

The `i` compiler must be installed. The extension starts the language server
by running `i lsp` over stdio.

## Settings

| Setting             | Default | Description                                        |
| ------------------- | ------- | -------------------------------------------------- |
| `ilang.path`        | `"i"`   | Path to the `i` executable used to run `i lsp`.    |
| `ilang.trace.server`| `"off"` | Trace LSP traffic (`off`, `messages`, `verbose`).  |

Use the **i: Restart Language Server** command after rebuilding the compiler.
The server also restarts automatically when `ilang.path` changes.

## Development

```sh
cd editors/vscode
npm install
npx vsce package   # produces ilang-<version>.vsix
```

To try it without packaging, open `editors/vscode` in VS Code and press
<kbd>F5</kbd> to launch an Extension Development Host.
