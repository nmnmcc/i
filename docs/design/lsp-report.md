# Language server: report

Status: implemented on `lane/lsp`. This report lists what `i lsp` does, the
queries added to `ilang_check::query` for it, what it cannot do yet, and the
measured response times.

## Approach

The server follows the Gleam language server and rust-analyzer:

- **Identity, not text.** Every name of a module is gathered once from the
  resolved tree (HIR) as an *occurrence*: a span and the `Target` it names (a
  local `VarId`, a top-level `DefId`, a constructor, a type, a type variable, a
  field of one or more constructors, a module, a literal). Hover, definition,
  references, highlights and rename all start from the target at the position
  and compare targets, never names. This is Gleam's `reference.rs`
  (`FindVariableReferences`, `find_module_references`) and rust-analyzer's
  `ide-db::search` in a small form. The written import and export lists, which
  the HIR does not keep, are read from the parser's syntax tree.
- **One check per package, kept.** As before, a package is checked as a whole
  on a thread of its own. The check now also keeps a `query::Cache` (the parsed
  tree and the occurrences of each module, computed once, warmed on the check
  thread for the package's own modules), so a request reads instead of
  recomputing. This is the analysis snapshot of both reference servers.
- **Place by tree.** Completion decides what to offer from the node around the
  position (a type, the clauses of a match, the arguments of a call, the
  imports), as rust-analyzer's `CompletionContext` does; only for what the tree
  cannot have yet (just after `!` or `:`) does it look at the token before the
  word.
- **Fixes from the checker.** Code actions are attached to the checker's own
  problems (E0201 unknown name, W0504 unused binding, E0516 not covered), as
  Gleam's `code_action.rs` does, and say which fix they carry in `data`.

Because every feature reads the HIR and the query layer, the coming syntax
change only needs the HIR spans to stay right; nothing matches surface text,
except the edits themselves, which are written in the surface of the file
(`.i` or `.is`).

## Features

| # | Feature | Implemented | Limits |
|---|---|---|---|
| 1 | References | Top-level names, constructors, types, type parameters (of types and bindings), field labels (definitions, `v.f`, `C(..v, f: e)`, labelled call arguments of constructors, constructor pattern parts), modules (import names and `m.x`), locals; in every module and test of the package; with or without the declaration. Expanded code maps back to the written macro argument. | Uses of a package's names in *other* packages of the workspace are not searched (a package's check does not hold its dependents). Labels of function parameters are not references (the HIR keeps a parameter's label as a string). |
| 2 | Rename | `prepareRename` and `rename` for the same set, across files; a field renamed where a pattern part both binds a local and gives the label becomes `local = new`; a local renamed there keeps the label (`new = label`); a module name renames its import (adding `name =` when the import had none). Refused with a message: names of dependencies, of the library `i`, built-in types, names made by macro expansion; new names of the wrong kind (`name` vs `UpName`); new names that would be covered (a local or top-level name of that name where the name is used), that would capture another use, or that clash with a top-level name, type, or field. | A rename does not rename files. Renaming a function parameter changes its label, which callers using that label would also need. |
| 3 | Navigation | Go to definition through imported names, import paths (to the module), module names, macro calls (to the macro), into dependency sources and the library `i`. Go to type definition (custom types of the known type; of a constructor or field, its type). Go to implementation of a type: its constructors and the environment statements `! T = e` that give it as a service. | Re-exports do not exist in i (§2.4), so there is nothing to follow. |
| 4 | Completion | By place: members after `m.`, fields after `v.` (also after a dot just written, from a check with a placeholder), constructor patterns with snippets and `_` in a match, the labels of a call that are not given yet, module paths (with their exports as documentation) and the exported names of a module in `(…) = path`, services after `!`, types (with the type variables of the item) in types. Every item has its type as `detail`; documentation is given for module paths and by `completionItem/resolve` for names. Exported names of modules that are not imported come with an `additionalTextEdits` import. | The context after `:` in a binding being written relies on the previous token. Auto-import offers values and constructors, not types. |
| 5 | Signature help | Functions (top-level, local, external) and constructors: the label, the parameter offsets, the active parameter by place or by label, the documentation. Both surfaces. | A call whose `)` is not written yet is answered only when the parser's recovery keeps the call. |
| 6 | Hover | Inferred type with documentation; `Value:` of a literal constant; `Declared as` for an annotated name whose inferred type differs; `Errors:` and `Environment:` rows of a function type; for a macro call the macro, its module and its expansion (the expanded text, cut at 400 characters); type aliases (including unions) show their definition. | |
| 7 | Code actions | Import a missing name (`(x) = path`) or a module for `m.x`; remove an unused top-level binding (with its documentation comment); remove an unused import (or the whole group); add the missing clauses of a match (`C(..) -> panic("todo")`, or `_`); add a type annotation from the inferred type when every type in it can be written in the module. | The annotation action is `.i` only: types are shown in the syntax of §3. The missing clauses are found at the top level of the subject (one constructor per missing clause); a nested missing pattern gets a clause for its constructor with `..`. Unused imports have no warning of their own (the checker gives none); the action is offered on the import. |
| 8 | Editor shape | Inlay hints (types of unannotated bindings and parameters, not of literals, functions, or constructors), folding ranges (imports, exports, items, blocks, matches, clauses, calls, lists of several lines), selection ranges (the nested nodes of the HIR, then the item, the group, the file), document highlights, workspace symbols (fuzzy, over the checked packages, not dependencies), document links of import paths. | |
| 9 | Diagnostics | Related information, `data: {"fix": …}` for the three fixable codes, the `unnecessary` tag for W0504. Every package with an open document is checked and published; a change re-checks only the packages whose check has the file. Problems are cleared for a closed new document, a deleted file (`didChangeWatchedFiles`), and when a check fails. A waiting request is superseded by a newer one of the same method on the same document (`ContentModified`), and `$/cancelRequest` cancels it. | A check is of a whole package; within it, modules are not re-checked one by one. |
| 10 | Robustness | Every request at every character of correct and broken files of both surfaces gets a well-formed response (tested); requests answer from the recovered tree. | |
| 11 | Capabilities | Advertised exactly for the above; VS Code watches `*.is` too; the Zed settings of the repository turn on inlay hints. | Zed has no language for `.is` files yet. |

## Queries added to `ilang_check::query`

All take a module and a byte offset or span, and are tested in
`crates/ilang-check/tests/query.rs`.

- `Cache`, `Query::with_cache`, `Query::warm`: the trees and occurrences of the
  modules, kept with a check.
- `Target` (public), `target_at_offset`: the identity of the name at a place.
- `references(target, owner, modules)`, `own_modules`, `highlights`: every
  place that names a target, with the definition and the label binders marked.
- `prepare_rename`, `rename` → `Vec<Edit>` or `RenameError`; `is_up_name`.
- `type_definition_at`, `implementations_at`, `macro_call_at`, `expansion_of`,
  `document_links`, `folding_ranges`, `selection_ranges`, `workspace_symbols`.
- `completion_context` (`CompletionContext`), richer `Completion` items
  (documentation, insert text, snippet, replaced span, import edits),
  `import_path`, `documentation_of`, `signature_help` (`Signature`).
- `import_edit`, `missing_import_fixes`, `unused_imports`, `remove_import_edit`,
  `remove_binding_fix`, `missing_clauses` (`MissingClauses`), `annotation_fix`,
  `inlay_hints` (`InlayHint`).
- `Hover::notes`: the extra lines of a hover.

## What it cannot do yet, and why

- **References across packages of a workspace.** A check holds a package and
  its dependencies, not the packages that depend on it. Searching them needs a
  check of every dependent package, which the server does not keep.
- **Function parameter labels as names.** The HIR keeps a parameter's label as
  a string without a span, and a call's labelled argument is not resolved to a
  parameter, so these labels are not references and are not renamed.
- **Expected type at a position.** Inference does not keep the type of every
  expression (only of names), so completion uses the known types of names
  (`LocalTypes::named_types`) and of constructor patterns, not the type
  expected at the position.
- **Coverage as data.** The checker reports one missing witness in the message
  of E0516. The code action works out the missing constructors at the top
  level of the subject itself from the clauses in the HIR.
- **Incremental checking inside a package.** A changed file re-checks its
  package (and the packages whose check has the file), not single modules.

## Measured latencies

Measured by `lsp_latency_on_std` (debug build, `std/src/list.i`), after the
first check of the `std` package:

(filled in below from the test run)
