# i for Visual Studio Code

Editor support for **the i programming language** (iLang), as defined in
`docs/spec.md`.

## Features

- Syntax highlighting for `.i` files (`syntaxes/i.tmLanguage.json`):
  - comments, documentation comments `///`, and module documentation `////`;
  - strings with escapes, including `\u{…}`, over one or more lines;
  - integers in every base (`0x`, `0o`, `0b`, `_` between digits) and
    floats (`1.0`, `2.`, `6.25e-3`); `t.0.1` is two tuple parts;
  - the keywords `import`, `export`, `case`, and `if`, with the module paths
    and aliases of `import (…)`;
  - the attributes `@main`, `@test`, `@external`, `@host`, and
    `@deprecated`;
  - types and constructors (`UpName`s), built-in types, `True`, `False`, and
    `Nil`;
  - the operators, also the Float ones (`+.`, `-.`, `*.`, `/.`, `<.`,
    `<=.`, `>.`, `>=.`), `?`, `!`, `->`, `&`, `|`, `|>`, `..`, and bit arrays
    `<< … >>` with their formats (`u16be`, `utf8`, …);
  - function definitions, calls, labels, fields, and discards (`_`, `__x`).
- Bracket matching, auto-closing pairs, comment toggling, and indentation.
- The language server `i lsp`:
  - errors and warnings, with their codes (`E0201`, `W0501`, …), as you
    type: the package of the file is checked again, with the unsaved text of
    the open files;
  - the type of a name on hover, with its documentation comment;
  - go to definition, also into other modules, dependencies, and the
    library modules of `i`;
  - the outline of a file (document symbols);
  - completion of the names in scope, of the members of a module after
    `m.`, and of the fields of a value after `v.`;
  - formatting, in builds of `i` that have the formatter.

## Requirements

The `i` compiler. The extension starts the language server by running
`i lsp` over stdio.

## Settings

| Setting              | Default | Description                                       |
| -------------------- | ------- | ------------------------------------------------- |
| `ilang.path`         | `"i"`   | The `i` executable that runs `i lsp`.             |
| `ilang.trace.server` | `"off"` | Trace LSP traffic (`off`, `messages`, `verbose`). |

`ilang.path` may be an absolute path, a name on the `PATH`, or a path
relative to the first workspace folder, such as `./target/debug/i`. The
server starts again when it changes. Use the **i: Restart Language Server**
command after rebuilding the compiler.

## Development

```sh
cd editors/vscode
npm install
npm test           # the grammar, token scopes, and the extension
npx vsce package   # produces ilang-<version>.vsix
```

`npm test` tokenizes `test/sample.i` with `vscode-textmate`, as VS Code
does, and compares the scopes with `test/sample.i.snap`. After a change to
the grammar, check the differences and write the snapshot again with
`UPDATE=1 npm test`.

To try the extension without packaging, open `editors/vscode` in VS Code
and press <kbd>F5</kbd> to launch an Extension Development Host. Set
`ilang.path` to a build of `i`, for example `cargo build -p ilang-cli` and
`<repo>/target/debug/i`.
