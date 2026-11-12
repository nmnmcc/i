# tree-sitter-i

In the repository development shell:

```sh
cd editors/tree-sitter-i
npm ci
npx tree-sitter generate
npx tree-sitter test
npx tree-sitter parse ../../examples/hello/src/main.i
```

Commit generated `src/parser.c`, `src/grammar.json`, `src/node-types.json` and
`src/tree_sitter/` after grammar edits. The external scanner implements the
break rule in spec §1.7: it recognizes newlines only where an ending expression
can be followed by a statement starter, looking across comments. The grammar
uses the break as a separator in data and parameter lists. An operator at the
end of a line or a continuing token at the beginning does not introduce a break.

The grammar is syntax three (docs/design/syntax-3.md §3): data `( )` with
`name = value` items, branches `(p) { e } | (q) { e }`, matches `e |> …`,
service patterns and needs with `<-`, provide `e <- (s)`, and `<A, …>` type
parameters on function literals and types. A binding's pattern is atoms joined
by `=` (`a = (x, y) = e`); the last `=` begins the value. Known gap: a type
that begins with `{ … }` and is followed by an operator (`{ 1 | 2 } - 2`). Semantic
constraints (such as repeated labels and type/name resolution) belong to `i lsp`.
No identifiers are classified as reserved keywords. The approach follows
`tree-sitter-gleam/grammar.js` for postfix expressions, operator precedence,
comments, and corpus testing; scanner interfaces follow the tree-sitter guide.

Highlight mapping to the VS Code TextMate grammar:

| Construct | Tree-sitter / Zed | TextMate |
| --- | --- | --- |
| names | variable | variable.other.i |
| types and constructors | type | entity.name.type.i |
| built-in types | type.builtin | support.type.builtin.i |
| True, False, Nil | constant.builtin | constant.language.i |
| discard | variable.special | variable.language.discard.i |
| function definition/call | function | entity.name.function.*.i |
| fields | property | variable.other.property.i |
| labels and typed parameters | variable.parameter | variable.parameter.i |
| attributes | attribute | storage.type.annotation.i |
| integer / float literals | number | constant.numeric.*.i |
| strings / escapes | string / string.escape | string.quoted.double.i / constant.character.escape.*.i |
| comments / documentation | comment / comment.doc | comment.line.*.i |
| bit formats | type | storage.type.format.i |
| operators, including ^ and legacy ? | operator | keyword.operator.*.i |
| brackets / delimiters | punctuation.bracket / punctuation.delimiter | punctuation.*.i |

The Zed highlight query is an exact copy of `queries/highlights.scm`.
