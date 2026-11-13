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
| `TypeKind::Fn(FnType { generics, param, services, ret })` | `Fn { params, ret, err, env }` | **Needs become rows here** (`Cx::needs_row`, called by `fn_type`): a `Fail(E)` member of the needs is `err: E`; the other members are the `env` union; `..N` is the row variable. |
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

1. **Deep trees.** A tree made by a macro can be deeper than the host stack.
   The old tree is safe by hand: iterative `Drop` (ast.rs:497), `ast_eq.rs`,
   `ast_clone.rs`, an explicit work list in `encode.rs`, `to_ast.rs`,
   `body_walk.rs` and `render.rs`. The new tree drops, compares, prints,
   reads datums and lowers by recursion, and so does every new pass
   (`syn3/walk.rs`, `group.rs`, `surface.rs`). Two ways: write every pass
   with a work list again (about 3,000 lines of today's code are that), or
   run the passes over a tree on a stack that grows (one helper around the
   entry points: reading a macro result, walking, resolving, dropping).
   Decision for step 2c: the second, because it keeps one plain recursive
   definition per pass; the tests `lisp_deep`, `lisp_encode_deep`,
   `parser_deep` and the 20,000-deep cases of `source_syntax.rs` and
   `scoped.rs` are the acceptance.
2. **Tree → `Datum` with spans.** Done in step 2b: `syn3::lisp::item_datum`,
   `expr_datum`, `type_datum`, `pattern_datum`. An atom that stands for a
   written name has that name's span; a compound has its node's span; a
   symbol that the tree does not keep (`,`, `->`, `=`, an operator) has the
   empty span at the start of its node. `to_lisp` is `show` of these datums.
   The macro argument encoder is `Syntax::from_datum` of them; the charge of
   an item is a walk of the datum.
3. **Quote nodes.** `PatternKind::Quote` gains the template slot and the
   captures; `quote_lower` works on the new tree.
4. **A walker.** Done in step 2a: `syn3::walk::Visitor` (enter and leave
   for expressions, patterns, types and blocks, in source order).
5. **Missing spans**: bit-array segments, the index of `d.0`, header groups
   (`surface.rs` finds the header bracket from the tokens), operator tokens.

## 4. Order of work

Every step leaves the tree green. A step moves whole consumers.

**Step 1.** This inventory.

**Step 2. The Lisp surface and the macro engine.**

- 2a. Done. `i fmt --to is|i` reads and writes the new tree and the §6.8
  rows (`syn3/surface.rs`: `normal_to_lisp`, `lisp_to_normal`): the file is
  read, written in the other surface, read back and compared with `==` on
  trees without groups (`ungrouped`), and the comments are moved by pairing
  the items and statements of the two trees. `regroup` moved from the
  converter to `syn3/group.rs`, because printing a Lisp-read tree needs it.
  Deleted: `lisp::print_with_comments`, `lisp::normal_with_comments`,
  `lisp/comments.rs`. `lisp::print_module` (`from_ast.rs`), `normal_source`
  and `shape::canon` have only tests as users now.
- 2b. Done. The `Datum` builder of §3 item 2, and `Cx::needs_row`.
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

**Needs → rows stays in one function.** It is `Cx::needs_row` in
`syn3/lower.rs`, the only place where the needs of a function type become
today's two rows (`Fail(E)` → the error channel, the other entries → the
environment, `..N` → the row variable). A function head does not go through
it: a service pattern `name: T` is an environment parameter and a written
`-> R` leaves both rows to inference. In step 3 the function moves to the
resolver unchanged, called from `ty`; the effects lane changes how `Fail(E)`
in a need is represented by editing that function only.

**Step 4. Deletion.** `syn3/lower.rs`; `ast.rs`, `ast_eq.rs`, `ast_clone.rs`,
`walk.rs`, `print.rs`, `shape.rs`; the feature `legacy` (the migrator keeps a
private copy of the old tree, parser, walker and `convert.rs` under
`examples/` while it must read old text); `syn3/` becomes the crate's top
level.

## 5. Questions that step 2c must answer first

These are not mechanical, and each changes what a macro sees.

1. **Definitions of sums.** `definition(T)` answers with the item of `T`.
   For `T = C1 | C2` over `C1 = (…)`, `C2 = (…)` the old rows answer one
   item with a constructor list. In the §6.8 rows the answer is
   `(= T (| C1 C2))`, and `C1` is a type of its own with the answer
   `(= C1 (, …))`. The built-in definitions (`definitions.rs`: `Bool`, `Nil`,
   `Result`, `AssertionFailure`, `Assert`) need the same split, and
   `expanded_definition`, `freshen_parameters` and `freshen_duplicate_heads`
   read the head of an item by its old position.
2. **A typed macro's function type.** `types/typed_macro.rs`:656-662 writes
   a function type as `(-> (params) (! (^ R E) V))`. §6.8 has
   `(-> (, T…) (<- R (, Fail(E) N…)))`; which entries stand for today's two
   rows is the same mapping as `needs_row`, reversed.
3. **Spans of symbols.** `encode.rs` gives an operator the span of its
   token. The builder gives the empty span at the start of the node. A
   diagnostic that points at an operator of a macro argument moves to the
   start of the operand; if that must not move, the builder needs the token
   index that `SourceEncoder` has.
4. **`Position` numbers.** `lib/i/syntax.i` `position_id` and
   `ilang-host/src/syntax.rs` share the numbers 0 to 16 of the old list. The
   list of §6.7 has 14 members; a compiled macro program and the host must
   change together (they are built together, so there is no stored form).

## 6. State of this plan

| Step | State |
|---|---|
| 1 inventory | done |
| 2a `i fmt --to` on the new tree | done |
| 2b datum builder; `needs_row` | done |
| 2c rows flip | additive preparation merged; `NEW_ROWS = false`, `ROWS3 = false`; additive prefix GREEN; workspace check, syntax/macro tests and four CLI binaries passed |
| 2d engine on the new tree | not started; faE readers are beside the old engine, with no production switch-over |
| 3 resolver | not started; faD is deliberately not merged in this prefix |
| 4 deletion | not started |

Readers of the old tree after 2b: everything in §2 except
`ilang-cli` `fmt --to`. The Lisp surface for files (`i fmt`, `i fmt --to`,
`syn3::lisp::{to_lisp, from_lisp}`) is on the new tree; the macro engine's
`Syntax` values are still in the rows before syntax three.

Integration checkpoint (lane/fa, 2026-11-13):

- faB `1bd592dd`: built-in definition answers and typed-macro function
  types in the new rows are available beside the old writers. The
  `NEW_ROWS` switch is off.
- faC `1c0932f0`: new-reader `show_as_rows3`, public normalization helpers,
  and the fourteen positions. The library and host position IDs changed
  together, as syntax-3.md §6.7 requires; `ROWS3` is off.
- faE `1a433ed3` (including the reboot's pending fixture fixes): new-tree
  macro engine modules and quote helpers are additive. The production
  engine still reads the old tree. `cargo check -p ilang-macro
  --all-targets --locked` passed in faE before committing those fixes.
- faF `89c0ecf4` (including the reboot's pending test fixes): written Lisp
  fixtures use §6.8; tests of old readers/encoders keep their old rows.
  `cargo check -p ilang-syntax -p ilang-check --all-targets --locked`
  passed in faF before committing. The conversion inventory is in
  `syn3-final-ast-tests.md`; its post-flip entries are still a checklist.

The integrator's next gate is `cargo check --workspace --all-targets
--locked`, all tests of `ilang-syntax` and `ilang-macro`, and the CLI
`macro_run`, `typed_macros`, `fmt`, and `lisp` binaries. Step 2c may only
land as one commit after that same gate passes with the rows enabled.
Steps 2d, 3 and 4 are outside this integration's authorized scope.

The first additive test run exposed a stale `format_skip.txt`: six spec
fences were already skipped by `parse_module`, but absent from its recorded
list. No parsing or formatter behavior changed. A diagnostic-only rerun
identified each entry before the snapshot was reconciled:

| Spec fence | Existing diagnostic that prevents this formatter property from running |
|---|---|
| 1130 | multi-shape `area` needs stage 9 |
| 1296 | `retry` writes needs in a function head, where the reader requires `name: Type` services |
| 1348 | whole-value/whole-environment provide needs E6; mixed data needs stage 6 |
| 1437 | whole-environment service pattern and provide need E6 |
| 1447 | `collect` writes a bare row variable in a function head; the reader only permits it in a type |
| 1496 | `catch` writes a bare row variable in a function head, with the same diagnostic |

The spec-fence test independently rejected fences 1296, 1447 and 1496.
Their examples now omit those function-head needs: spec §6.8 says, “The
need of the function is inferred (§4.8 rule 4), also when the `->` part is
written.” Their parameter types and bodies are unchanged, and the inferred
needs keep the examples' meaning. No parser rule or diagnostic changes.
The other three fences remain unchanged: the formatter snapshot records
existing lowering limitations, without adding any new skip in test code.
Stages 9/E6 are outside this prefix.

Verification of the additive prefix: workspace/all-targets check passed.
All syntax and macro test binaries passed across the broad diagnostic run
and the targeted `format`/`spec` rerun after the documented fixture fixes.
The four requested CLI binaries passed: `fmt` 6, `lisp` 5,
`macro_run` 57 and `typed_macros` 35 tests (103 total). The additive prefix
is green with both row switches off. Logs are under `target/fa-integration/` in this worktree.

Step 2c will be attempted separately on `lane/fa-2c`, based on this green
prefix. It is not part of `lane/fa` unless the same gate passes with the
new rows enabled. No production engine migration, resolver migration, or
old-tree deletion is included.

Evening resumption (18:01, extended deadline 20:30): the complete surviving
2c patch was preserved in `target/fa-integration/surviving-2c.patch` before
integrating `lane/conv`. Its shape-based checker and function types are
combined with the additive row writers, deep helper, quote captures,
normalization and position printers. The new walker comparison removes
only compiler-synthesized `%multi` calls and hidden `%g` binders and compares
all written calls with their lexical environments in source order. Multi
values use the transition's one-byte root for the direct-value comparison.
The old engine now recognizes the transition `%multi` construction as a
direct value: spec §6.12 says “A chain is a direct value (§6.10), as every
function is.” The formatter skip entries now name spec fences 1349 and
1438: whole-data/environment provide still needs E6, while `area` now parses
through the shape-group lowering. No parser or diagnostic expectation was
changed to hide that limitation.
