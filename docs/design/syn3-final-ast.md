# One tree: syntax three as the final AST

Status: plan and inventory (lane/fa, 2026-11-13). Controlling sources:
`docs/design/syntax-3.md` §6, §6.7, §6.8 and §7.2 row 12.

Today there is one surface and two trees. The syntax-three parser builds
`crates/ilang-syntax/src/syn3/ast.rs` (the **new tree**), and
`syn3/lower.rs` lowers it to `crates/ilang-syntax/src/ast.rs` (the **old
tree**: clause blocks, `^ E ! N` rows, record forms, binding-head type
parameters, multi-constructor type definitions). Everything after the
parser reads the old tree. The goal is one tree: the new one.

This file lists every reader and writer of the old tree, what each needs,
and the order in which they move. Line counts are of 9fd733b4.

## 1. The two trees, form by form

What the lowering does today is what each consumer must do itself (or no
longer need) once it reads the new tree.

| New tree (`syn3::ast`) | Old tree (`ast`) made by `lower.rs` | Meaning that the lowering adds |
|---|---|---|
| `TypeDef { head, rhs: Data }` | `TypeDef { body: Custom([one constructor]) }` | The type and its constructor have one name. |
| `TypeDef { rhs: Opaque }` | `TypeBody::Custom([])` | `T {}`. |
| `TypeDef { rhs: Type(union) }` right after its `k` data definitions | one `TypeDef` with `k` constructors; the member definitions are absorbed, and an export of a member alone is dropped | The checker has no union of data types that stands for a sum (`items`, lower.rs:312). |
| `TypeDef { rhs: Type }`, other | `TypeBody::Alias`, `alias_pattern` when the right side is a value type and the head is not plain names | |
| `Binding` with `Chain` value whose functions have `generics`; `x: <A>(…) -> R` | `Binding.type_params` (union of the branches' lists, written order) | Type parameters live on the binding. A row variable written without `Fail` beside it also gets `N%E` (`binding`, lower.rs:515). |
| `Chain([f])`, no guard, one choice per item | `ExprKind::Fn` | |
| `Chain(fs)`, one shape | `ExprKind::Clauses` | Several shapes: error "needs stage 9". |
| `Pipe { rhs: Chain }` | `ExprKind::Case` | A match. |
| `Pipe`, other | `Binary { op: Pipe }` | |
| `Function.services` (`name: T`) | `Param::Env` | `..name: N`: error "needs stage E6". |
| `Function.ret` | `RetType { value, err: _, env: _ }` | Needs are inferred. |
| `Function.guard`, `PItem.choices` | `Clause.guard`, `Clause.patterns` | `PItem.key`: error "needs stage 9". |
| `Provide { target, services: Data }` | block `{ ! _ = s1 … target }` (`Stmt::Env` with type `_`) | Several services that are not names are bound to fresh locals first (§2.10.5). |
| `Data(items)` | `Tuple`, `Record { base, fields }` | Mixed positional and labelled data is an error today. |
| `Call { args: [Spread, labels…] }` on a constructor | `RecordUpdate` | |
| `Index` | `TupleIndex` | |
| `Interp(parts)` | chain of `<>` | |
| `PatternKind::And` | nested `Named` | Parts that are not names: error. |
| `Ctor { args: [DPItem] }`, `Data([DPItem])` | `Constructor { args, spread }`, `Tuple`; a key is a `Named` chain ending in the label | `Rest(Some)` in data: error. |
| `TypeKind::Data` | `Tuple`, `Record` | |
| `TypeKind::Fn(FnType { generics, param, services, ret })` | `Fn { params, ret, err, env }` | **Needs become rows here** (`fn_type`, lower.rs:1259): a `Fail(E)` member of the needs is `err: E`; the other members are the `env` union; `..N` is the row variable. |
| `Attribute.args: Option<Vec<DItem>>` | `args: Vec<(String, Span)>` when all are plain strings, else `call` | |
| `ExprKind::Quote`, `PatternKind::Quote` (rows of §6.8) | the same datum in the rows before syntax three (`lisp::unconvert_quote`) | The macro engine matches and builds the old rows. |

Things the old tree has and the new tree lacks, which consumers use:

- `PatternKind::Quote { template, captures }`: the template slot and the
  capture patterns filled by `quote_lower`.
- Iterative `Drop` (ast.rs:497): trees made by macros can be deeper than the
  host stack. The new tree drops, prints and lowers by recursion.
- Spans on `Segment`, on the index of `d.0`, on the imports and exports
  groups, and on an attribute's argument group.
- Structural equality that ignores spans is in the new tree by `Sp`; the
  old tree needs `ast_eq.rs` and `shape.rs` for it.

## 2. Consumers of the old tree

"Reads" names the old forms the consumer depends on. "Needs from the new
tree" is what must exist before it can move. Sizes are lines of the file and
(in brackets) the number of lines that name an old-tree type.

### 2.1 `ilang-syntax`: the tree's own helpers

| File | Lines | What it is | Users | Fate |
|---|---|---|---|---|
| `src/ast.rs` | 762 | The old tree and its iterative drop | all below | Deleted at the end. `BinOp`, `UnOp`, `LitValue` (re-exported by `syn3::ast` and used by HIR, the checker and the back end) move to `syn3/ast.rs`. `Ident` with a plain `Span` is used by HIR (`hir.rs`:14): HIR gets its own or uses `syn3::Ident`. |
| `src/ast_eq.rs`, `src/ast_clone.rs` | 530, 373 | `PartialEq` and `Clone` for `Expr`, `Pattern`, `TypeExpr` without recursion | derive users of the old tree | Deleted with `ast.rs`. |
| `src/walk.rs` | 337 | `Visitor`/`Walk` over the old tree (every span and string) | `ilang-macro` (`item_phase` 14, `typed`, `registry`, `project`, `body_engine`), `lisp/{expansion,from_ast,quote_lower}`, `shape.rs`, `syn3/convert.rs`, `ilang-cli` `expand`, tests `macro_run`, `body_walk` | A walker over the new tree is needed first; then deleted. |
| `src/print.rs` | 1001 | Old-surface printer of the old tree | `ast.rs` (`string_literal` for `LitValue::text`), `lisp/from_ast.rs`, `syn3/lisp.rs` (`string_literal`), tests `lisp_datum`, `string_forms`, `syn3_lower` | `string_literal` moves to the new printer; the rest is deleted (it prints syntax that no parser of the default build reads). |
| `src/shape.rs` | 164 | `shape`, `canon`, `equivalent` on the old tree | `ilang-cli` `fmt --to` (write check), `lisp/to_source.rs`, tests `common.rs`, `format`, `lisp_*`, `string_forms` | Replaced by `==` on the new tree (spans are not compared; a group is a `Block`, so the check needs the one-statement-block rule of `canonical`). |
| `src/legacy/{parser,format}.rs` | 2155, 1789 | Old parser and formatter, feature `legacy` | examples `migrate-syntax3`, `migrate-syntax3-pipe`, `migrate-tests`; tests `syn3`, `syn3_lower`; scripts `syn3-convert-all`, `syn3-convert-embedded`, `old-syntax-check`, `test-macro-migrate`, `migrate-syntax3-labels-text` | Stays while the migrator must read old text; then moves under `examples/` with a private copy of `ast.rs`, `walk.rs`, `print.rs` and `syn3/convert.rs`. |
| `src/syn3/convert.rs` | 2214 | Old tree to new tree (the migrator), and `syn3::print_current` | migrator; **`ilang-cli` `expand --to`** and **`ilang-check` `query.rs`:375** print an expanded old tree through it | The two compiler users go away when the expanded tree is the new tree; the file then belongs to the migrator only. |
| `src/syn3/lower.rs` | 1429 | New tree to old tree | `syn3::parse_lowered` (= `ilang_syntax::parse_module`), `syn3::parse_lisp_lowered` (= `lisp::parse_source`), migrator `--verify` | Deleted when the resolver reads the new tree (step 3). |

### 2.2 `ilang-syntax`: the Lisp surface (`src/lisp/`)

`Datum`, `Syntax`, `Scope`, the reader (`mod.rs`), the Lisp formatter
(`format.rs`), `definition_schedule.rs`, `trampoline.rs` and `comments.rs`
do not read the tree. The others:

| File | Lines | Direction | Rows | Users | Needs from the new tree |
|---|---|---|---|---|---|
| `encode.rs` | 1106 | old tree → `Syntax` (macro arguments and written items of `.i` files), spans of written atoms from a token index | old | `ilang-macro` `body_engine`, `item_phase`, `definition_world`, `typed`; `definitions.rs`; `source_syntax.rs`; tests `lisp_encode_deep`, `lisp_syntax`, `string_forms` | A tree → `Datum` builder that keeps spans (§3.2 below). |
| `encode_charge.rs` | 718 | the cost of `encode.rs`'s output, without building it | old | `SourceEncoder::visit_item_charge` (`definition_world`) | Same builder; the charge is then a walk of its output. |
| `to_ast.rs` | 1131 | `Datum` → old tree (macro results; old-row `.is` files for the migrator) | old | `scoped.rs` (every macro result), `lower.rs`:1083 (quote patterns), `ilang-cli` `fmt --to i`, `to_source.rs`, migrator, tests `lisp_datum`, `lisp_syntax`, `lisp_encode_deep`, `lisp_record` | `syn3::lisp::{expr,type,pattern}_from_datum` exist; items and stack safety are missing. |
| `from_ast.rs` | 517 | old tree → Lisp text, with comments | old | `ilang-cli` `fmt --to is`; tests | `syn3::lisp::to_lisp` exists; the comment transfer does not. |
| `to_source.rs`, `render.rs` | 404, 803 | `Datum` → normal text by position (`Syntax::show_as`, `fmt --to i`) | old rows, old normal syntax | `syntax.rs` (`show_as`), `ilang-host` `syntax.rs`, `fmt --to i`, tests `lisp_syntax` (25), `syntax_runtime` | The new reader and printer, by the positions of §6.7. |
| `source_syntax.rs` | 387 | written `.is` code → `Syntax`: finds the datum by span and removes groups by position | reads rows as written (new), but its position table (`Mode::{Params, Clause, ClauseHead, …}`) is of the old rows | `ilang-macro` `body_engine`, `item_phase`, `typed` | A position table of the §6.8 rows. Item kinds come from `ast::ItemKind`. |
| `quote_lower.rs` | 160 | expression quotes → calls of the compiler's syntax service; fills `PatternKind::Quote { template, captures }` | datum as given | `ilang-check` `lib.rs`:189, `ilang-macro` `prepare`, `project`, `ilang-cli` `expand`, tests | `Quote` nodes with template slot and captures in the new tree. |
| `scoped.rs` | 579 | macro result (`Syntax`) → text suffix → `Datum` → old tree, with origins by span | old (through `to_ast`) | `ilang-macro` (`expression_metered`, `ty_metered`, `items_metered`, `pattern`), `ilang-check` `resolve.rs` (origins) | A stack-safe `Datum` → new tree. |
| `definitions.rs` | 604 | built-in type definitions and `definition` answers as `Syntax` | old (constructor lists `(T {C…})`, `(-> … (! (^ R E) V))`) | `ilang-macro` `definition_world` | The item rows of §6.8. |
| `definition_table.rs` | 308 | `Definition` holds `ast::Item` | — | `definition_world` | New `Item`. |
| `expansion.rs` | 269 | marks, depth; `definitions` reads `ast::Module` for macro definitions | — | `ilang-macro` | New `Module`, new walker. |
| `syntax.rs` | 998 | `Syntax` values; `Position` (17 members, old list); `validate_expansion`; `match_quote` | `Position` is the old list | host, macro engine | `Position` of §6.7. |

Row state today (this is the inconsistency helper A reported): quotes in a
macro body are written in the §6.8 rows and lowered to the old rows; a
macro's arguments from a `.i` file are encoded in the old rows; arguments
from a `.is` file are passed as written, in the §6.8 rows; results are read
with the old rows. So a macro that matches a function called from a `.is`
file sees `(-> (, x) _ e)` against a pattern lowered to `(-> (x) _ e)`.

### 2.3 `ilang-macro`

| File | Lines [old-tree lines] | Reads | Writes |
|---|---|---|---|
| `body_walk.rs` | 545 [74] | Every expression, pattern, type, block, statement, clause and function, with a scope stack of binders (`Local`), to find macro calls outside-in. Old forms: `Fn`, `Clauses`, `Case`, `Clause`, `Param::Env`, `Stmt::Env`, `Record`, `RecordUpdate`, `Named`. | Replaces a call node by its expansion. |
| `body_engine.rs` | 498 [21] | Call nodes; `is_direct` (which values are made without running code: `Fn`, `Clauses`, constructor calls, pipes into constructors). | — |
| `call.rs` | 76 [8] | The shape of a macro call: `Call`, `Binary(Pipe)` with a call right side, `TypeKind::Call`. | — |
| `item_phase.rs` | 987 [26] | `ItemKind`, `Attribute { args, call }`, item spans; walks items for nested macro items. | Replaces items; adds `ExportItem`s. |
| `definition_world.rs` | 699 [12] | `ItemKind`, `TypeBody`, type heads (`pattern_binders`), `ImportItem`, binding and declaration `type_params`. | — |
| `registry.rs` | 279 [3] | `ItemKind`, binding patterns (macro definitions by name). | — |
| `prepare.rs` | 305 [5] | — | Builds a `_ = 0` item; generated wrapper modules are written as text. |
| `typed.rs` | 653 | Call nodes by span. | Builds `Fn` with `RetType`, a `_ = e` wrapper item; replaces expressions by span. |
| `project.rs` | 428 | Module list. | Runs `quote_lower`. |
| tests `body_walk.rs` | 400 [44] | Builds old trees by hand. | |

### 2.4 `ilang-check`

| File | Lines [old-tree lines] | Reads |
|---|---|---|
| `lib.rs` | 431 | `Parsed.ast: ast::Module`; `parse` chooses the reader by file name. |
| `resolve.rs` | 4644 [322] | The whole old tree → HIR. Entry points and old forms: `collect` (459), `collect_type` (709: `TypeBody`, `Constructor`, `Field`), `attributes` (798: `args`/`call`), `entry_form` (909), `exports` (941), `imports` (1037), `type_defs` (1328), `host_form` (1572), `resolve_alias_pattern` (1901), `type_pattern` (1960), `ty` (2014: `Tuple`, `Record`, `Fn { params, ret, err, env }`), `values` (2644), `push_declared_frame` (2733: binding-head type parameters), `binding` (2768), `is_direct` (2897), `expr` (3257: `Tuple`, `Record`, `RecordUpdate`, `Case`, `Clauses`, `Fn`, `TupleIndex`), `clause_function` (3492), `block` (3550: `Stmt::Env`, with `_` read as "key from the value's type"), `function` (3590: `Param::Env`, `RetType`), `clause` (3713), `bind` (3783), `pattern` (3819: `Named`, `Constructor { spread }`, `Quote { template, captures }`). |
| `query.rs` | 2283 [135] | The module's old tree for the language server: type parameters of items (1112), constructors of a type (1546-1603), a `_ = e` wrapper module (358), re-parsing of expanded text (327), and `print_current` (375). |
| `types/lits.rs` | 818 [3] | One binding with a string value (721); `LitValue`. |
| `typed_macro.rs`, `types/typed_macro.rs` | 116, 1094 | `ast::Expr` as the syntax of a typed macro call; `LitValue`. |
| `hir.rs`, `types/infer.rs`, `types/known.rs` | | Only `BinOp`, `UnOp`, `Ident`. |
| tests | | `hygiene_resolve` (23), `expansion_origins` (12), `unused_audit`, `expanded_modules`, `quote_patterns` build or inspect old trees. |

### 2.5 Other crates

| File | Reads |
|---|---|
| `ilang-wasm/src/lower.rs` | Only `BinOp`, `UnOp`. Tests `expansion_sites`, `control_lazy` build `Parsed` values. |
| `ilang-cli/src/commands/expand.rs` | Walks `Parsed.ast` to replace typed-macro calls; prints with `syn3::print_current`. |
| `ilang-cli/src/commands/fmt.rs` | `--to is`: `parse_module` (old tree) → `lisp::print_with_comments` → `lisp::parse_source` → `shape::canon`. `--to i`: `lisp::parse` → `lisp::parse_module` (old rows) → `lisp::normal_with_comments` → `parse_module` → `shape::canon`. Both write the rows and the normal syntax from before syntax three, so both fail on every converted file (§4, step 2a). |
| `ilang-cli` tests | `macro_run` (32), `expansion_sites`, `r2_fixtures`, `collections`. |
| `ilang-lsp/src/analysis.rs` | 312-319: the last statement of a one-function wrapper module. |
| `ilang-host/src/syntax.rs` | No tree; calls `Syntax::show_as` with a `Position` number (`lib/i/syntax.i` `position_id`). |
| `lib/i/syntax.i` | `Position` (17 members of the old list, line 95), `position_id` (491), `show_as` (513); `Node` is already the union of §2.8. |

## 3. What the new tree needs before consumers can move

1. **Deep trees.** An iterative `Drop` for `Expr`, `Pattern`, `Type` and
   `Block`, as in ast.rs:497. Readers of macro results (`Datum` → tree) must
   not recurse on the host stack either.
2. **Tree → `Datum` with spans.** `syn3/lisp.rs` writes text. A builder that
   returns `Datum`s whose atoms carry the spans of the written names makes
   one function serve three users: the Lisp printer (`show` of the datum),
   the macro argument encoder (`Syntax::from_datum`), and the charge.
3. **Quote nodes.** `PatternKind::Quote` gains the template slot and the
   captures; `quote_lower` works on the new tree.
4. **A walker** (`Visitor`) over the new tree.
5. **Missing spans**: bit-array segments, the index of `d.0`, header groups.

## 4. Order of work

Every step leaves the tree green. A step moves whole consumers.

**Step 1.** This inventory.

**Step 2. The Lisp surface and the macro engine.**

- 2a. `i fmt --to is|i` reads and writes the new tree and the §6.8 rows:
  `syn3::parse_module` → `syn3::lisp` → text, and back, compared with `==` on
  new trees. After it, `lisp::print_with_comments`, `normal_with_comments`
  and `shape::canon` have no user in the compiler.
- 2b. §3 items 1 and 2: iterative drop; the `Datum` builder, with
  `to_lisp` = `show(builder)`.
- 2c. **The rows flip** (one commit; it cannot be split, because a macro's
  arguments, its quotes and its results must be in the same rows):
  - arguments: the new node is encoded by the builder of 2b. While the
    engine still walks the lowered tree, the node is found in the new tree of
    the file by span and kind (as `source_syntax::lisp_source` finds a datum
    today); generated nodes keep using `ScopedSource::origin`.
  - quotes: `lower.rs` no longer calls `unconvert_quote`; capture patterns are
    read by `syn3::lisp::pattern_from_datum`.
  - results: `scoped.rs` reads the datum with `syn3::lisp::*_from_datum`, then
    `lower_*` until step 3.
  - `source_syntax.rs`: position table of the §6.8 rows.
  - `definitions.rs`: built-in definitions in the item rows of §6.8.
  - `syntax.rs`, `render.rs`, `to_source.rs`, `lib/i/syntax.i`,
    `ilang-host`: `Position` of §6.7; `show_as` reads the datum with the new
    reader and prints with the new printer.
  - `std/src/test.is` and the test macros that look at nodes by hand.
  - deleted: `lisp/encode.rs`, `encode_charge.rs`, `to_ast.rs`, `from_ast.rs`,
    `unconvert_quote`, `convert_quote` moves to the migrator.
- 2d. **The engine's tree**: `Parsed.ast` is a `syn3::Module`; `body_walk`,
  `body_engine`, `call`, `item_phase`, `definition_world`, `registry`,
  `prepare`, `typed`, `quote_lower`, `expansion`, `definition_table` read and
  write it; `resolve::resolve` and `query.rs` lower each module on entry.
  Lowering errors of written code are still reported at parse time (`parse`
  runs the lowering once and keeps only its diagnostics), so no error moves
  or disappears.

**Step 3. The resolver.** `resolve.rs` reads the new tree; `lower.rs`'s rows
of §1 become resolver code, in this order, each green: types (`collect_type`,
`type_defs`, `ty`, alias patterns; sums stay as the absorbed form until the
checker has unions of data types), patterns with keys, expressions (data,
calls, pipes, interpolation), chains (→ `Function`/`Case`), type parameters
on function values, provide as its own HIR node keyed by the type of the
provided value. `query.rs`, `lits.rs`, `ilang-lsp` `analysis.rs` and
`ilang-cli` `expand` follow in the same step.

**Needs → rows stays in one function.** Today it is `Cx::fn_type`
(lower.rs:1259) for types, with `Cx::ret` and `Cx::params_of` for function
heads. In step 3 it becomes one resolver function, `needs_row`, called from
`ty` and `function`: the effects lane changes how `Fail(E)` in a need is
represented by editing that function only.

**Step 4. Deletion.** `syn3/lower.rs`; `ast.rs`, `ast_eq.rs`, `ast_clone.rs`,
`walk.rs`, `print.rs`, `shape.rs`; the feature `legacy` (the migrator keeps a
private copy of the old tree, parser, walker and `convert.rs` under
`examples/` while it must read old text); `syn3/` becomes the crate's top
level.

## 5. State of this plan

Kept up to date at the end of each step; see the end of the file.
