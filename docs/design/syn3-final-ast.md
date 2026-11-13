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
| 2c rows flip | GREEN on `lane/fa`, rows enabled; full evening gate passed on conversion merge `52f5e95e` |
| 2d engine on the new tree | GREEN: Parsed.ast and production macro passes use syn3; resolver/query lower on entry; full evening gate passed at 20:08 |
| 3 resolver | GREEN: expanded Nov14 gate completes on native source `3499484118`; all requested Rust/CLI/std/spec/generation/fmt/clippy checks pass |
| 4 deletion | COMPILES on lane/fa-delete: production old tree/lowerer/old row helpers deleted; workspace all-targets check passes; private package and full deletion gate still pending |

Historical checkpoint after 2b: readers of the old tree were everything in §2 except
`ilang-cli` `fmt --to`. The Lisp surface for files (`i fmt`, `i fmt --to`,
`syn3::lisp::{to_lisp, from_lisp}`) is on the new tree; the macro engine's
`Syntax` values are still in the rows before syntax three.

Morning additive checkpoint (lane/fa, 2026-11-13):

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

The evening gate is `cargo check --workspace --all-targets --locked`, all
`ilang-syntax` and `ilang-macro` tests, CLI `macro_run`, `typed_macros`,
`fmt`, `lisp`, and `macro_budget`, examples/macro-tests O0, std O0, and spec
filters `15.7/`, `6/`, `7/`. Step 2c lands as one commit only after this
passes with rows enabled. The evening follow-up authorizes steps 2d, 3 and
4 after each preceding gate is green.

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

The morning attempt used `lane/fa-2c`. The evening follow-up resumes the
preserved patch on `lane/fa`, after merging the conversion lane. No
production engine migration, resolver migration, or old-tree deletion is
part of the rows-flip commit.

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

Rows-flip attempt (resumed on `lane/fa`, conversion merge `820e6f60`):

- `ROWS3` is on; `NEW_ROWS` follows it. Written `.i` arguments use
  `SourceTree`, with normalization by position. Written `.is` arguments
  retain their original datums and scopes. Generated nodes retain their
  `ScopedSource` origins.
- Quotes retain §6.8 datums. Unquote expressions and capture patterns are
  read by the new reader, and macro results are read there and transition
  lowered. The production engine still traverses the old tree.
- Definition answers use `Definition::tree_with_datum` for written code.
  Lookup indexes absorbed sum members and retains their original export
  visibility. Built-ins accept the names of `builtin_definition_names(true)`.
  `Assert`'s answer has the unlabelled parameter of spec §9.6.
- Alias-pattern heads do not become formal type parameters (spec §4.7).
  The metering test still requires charging before copying/freshening; its
  written-definition cost is now `Datum`, as planned in §4 step 2c.
- `TokenIndex` includes interpolation tokens, so operators keep their
  written spans. A regression covers both ordinary and interpolated `+`.
- Quote capture patterns participate in the new tree's iterative drop,
  including the 20,000-deep small-stack regression.
- The `suite` macro and its review-fixture copy collect provides from the
  new rows, accept single-expression function bodies, and wrap each test
  with its services in the same nesting order.
- The nonfinal type-test choice stays legal Lisp code but has no normal
  spelling; its test checks refusal by conversion under spec §14.1,
  rather than retaining an old-row parser restriction.

Old `encode`, `encode_charge`, `to_ast`, `from_ast` and quote-conversion
helpers remain for legacy/test callers and the transition's old-row branch.
They are not the production macro argument/result path with the switch on.
Their deletion is deferred; no test of them was removed or weakened to
force the flip. Steps 2d, 3 and 4 remain untouched.

The first rows-on syntax run exposed five failures in two binaries: three
old-encoder tests were accidentally routed through the new source wrapper,
an alias-pattern head acquired a parameter identity, and a metering
expectation still required streamed old-row events. These were corrected
without dropping their assertions. The second syntax/macro run passed,
including operator-span and sum-member visibility regressions. The first
rows-on CLI run passed all 103 existing tests. A final gate is pending after
the quote-capture drop fix and the added production member-query regression;
the final evening gate below now covers both.

Evening 2c regressions also cover macro arguments that are multi-shape
functions, in both surfaces. The transition gives these a one-byte `%multi`
root. Source lookup recovers the written chain; Lisp lookup still returns
its original datum. The compiler-reserved `%multi` constructor bypasses
macro name lookup, because its synthetic span belongs to a whole chain,
not a name atom. This keeps the chain semantics of spec §6.12.

Evening gate progress: workspace check passed; the syntax/macro broad run
passed except a stale inactive-row assertion and the new multi-shape test's
choice of hidden item; both focused retries passed. CLI fmt 6, lisp 7,
macro_budget 9, macro_run 59 passed; typed_macros had 35 passes and one new
conversion fixture still in old rows. Its deterministic expansion/Wasm
assertions remain unchanged and the new-row retry passed. Spec `15.7/`
passed 28 active cases and `6/` passed 78, each at O0/O2, with their existing
pending lists unchanged. A mistaken worktree direnv invocation of `7/`
was refused before cargo ran; the corrected wrapper command is running.
The final broad rerun will cover the multi-shape regression and all fixes.

Final 2c gate (18:48, lane/fa, conversion lane through `dfe2d62d0` in merge
`52f5e95e`): **GREEN**. Workspace/all-targets check passed. All syntax and
macro tests passed (215); CLI fmt 6, lisp 7, macro_budget 9, macro_run 60,
syntax_runtime 10, typed_macros 36 passed (128). Examples passed 13 tests,
with one existing skip. Std O0 passed all 2,170 tests. Spec filters `15.7/`
28/28, `6/` 80/80, `7/` 104/104 passed at both O0/O2; the filter uses a
substring, so `7/` also runs `9.7/`. Existing pending counts are 1, 28, 1.
Hygiene resolution passed 20/20. Exact commands and raw results are the
`target/fa-integration/2c-final-*.log` files.

The earlier `7/` run had 102 passes and two stale host-error frame
coordinates. Conversion commit `db41ca849` maps the same main definitions
from 15:1 to 12:1 and 14:1 to 11:1; those exact assertions remain. Conversion
commit `dfe2d62d0` re-enables two closed-data cases; neither was skipped to
obtain the final gate. Conversion commit `c5918d467` restores future tags
on the two E6 fences that already failed lowering, so the formatter skip
inventory no longer lists those future fences.

The rows flip is complete as one commit. Steps 2d, 3 and 4 still require
separate green gates. A native-engine draft is prepared under `target/`,
outside the sources used by this completed gate.

Conversion final commit `9365b311f` is integrated after the green rows flip.
Its four Rust-file changes are formatting only; its other changes document
conversion verification and fix inventory extraction. Workspace/all-targets
check passed again (`conv-final-check.log`).

Step 2d implementation begins from green `9badcec8` (18:50). `Parsed.ast`
is a `syn3::Module`; the production driver uses the part-E readers and
walkers. The old resolver gets a private `LoweredParsed` at entry. Written
lowering diagnostics are retained at parse time, and generated readers
validate the same transition restrictions before retaining the native tree.
Written Lisp and standalone macro-result pipe classification keep their
existing boundary through the source map's virtual suffix. Native clones,
walkers, and quote lowering use `syn3::deep`.

The native type definition stores its written alias-pattern classification.
Spec §4.7 says, “This classification is made before expanding the right
side; it never depends on imports or on what a macro returns there.” A
baseline run of `Alias(A) = m.id(value)` reports E0221 at 3:7 for unused A;
the new regression retains that error and excludes reclassification as an
unknown alias-pattern head. The baseline command is recorded in
`target/fa-integration/2d-classification-baseline.log`.

2d checkpoint (19:35): workspace/all-targets check and syntax/macro tests
passed, then CLI fmt 6, lisp 7, macro_budget 9, macro_run 61,
syntax_runtime 10 and typed_macros 36 passed (129). Expanded-module and
origin tests passed. Hygiene 20 and quote-pattern 5 retries passed after
preserving prepared nested capture slots and locating the same f binding
among the now-separate native member items. Wasm source sites 2, CLI source
sites 1 and the LSP expansion-suffix regression 1 passed.

An additional old-engine/new-engine differential then found a real
transition-context mismatch: a returned `(Int) <- N -> Int` inherited
`N%E` when re-lowered inside a generic binding, whereas the old standalone
reader supplied no such error channel. The fix retains standalone fragment
boundaries, original formal-parameter lists and original sum membership as
names and spans only. This also prevents a macro-returned chain or union
from being regrouped as though it had been written at its final location.
Five differential inputs plus an item-result absorption case cover those
boundaries. Their fix and the final 2d gate are pending; 2d is uncommitted.
Newer conv commits after 9365b311f are still marked NOT LANDABLE by that
lane, so the last green conversion checkpoint remains the integration base.

Final 2d gate progress (19:53): workspace/all-targets check passed, all
syntax/macro tests passed (216), all six CLI binaries passed (130: fmt 6,
lisp 7, budget 9, macro_run 62, runtime 10, typed_macros 36), and examples
passed 13 with one existing skip. The five-case old/new differential and
the item-result absorption regression passed with the boundary metadata.
Std O0, spec 15.7/ + 6/ + 7/, and the final boundary rerun remain pending;
2d is not committed until the entire gate passes.

Final 2d gate (20:08): **GREEN**. Workspace/all-targets check passed;
syntax/macro 216, CLI fmt/lisp/budget/macro_run/runtime/typed 130, examples
13 with one existing skip, and all 2,170 std O0 tests passed. Spec filters
15.7/ 28, 6/ 80, 7/ 104 passed at both O0/O2 with the existing pending
counts 1/28/1. The final checker boundary rerun passed all 30 cases. The
five-case fragment differential and item-result sum-boundary regression
passed. Additional Wasm source-site 2, CLI source-site 1 and LSP suffix 1
checks passed before the final boundary-metadata refinement. Complete
commands, earlier explained failures and retries are in
`target/fa-integration/evening-verification.txt`; final logs use `2d-final-*`.
The initial queued std wrapper was cancelled by its own PID before starting,
so the spec and boundary checks could run first; std then ran last through
the wrapper. Step 3 is next; no deletion is authorized before its green gate.

### Night continuation, 21:43–02:00

The coordinator extends the deadline to 02:00 and authorizes steps 3 and 4
with the expanded gate: workspace check; syntax/macro/check/LSP tests
(skip only `spec_corpus_reverse_order_and_recovery`); CLI macro_run,
typed_macros, fmt, lisp, check, commands, macro_budget; macro examples;
std O0/O2; spec filters 6/, 7/, 9/, 15.7/; source/Rust fmt and clippy.
Targets are step 3 by 23:30 and deletion by 00:30. The committed green base
is still `6bc71b71e`. The former red faD merge was saved completely under
`target/fa-night/` before deliberately aborting it to integrate conv first.

Conv integration is through `424361875` (including tonight main's lazy-call
shape dispatcher). The specialized-alias member check is combined with
2d's recorded fragment metadata. Generated `Syn3ServiceFail` has ordinary
formal parameters, not an alias-pattern head. Correct labelled SyntaxError
fixtures are retained; two function-body quote patterns again capture the
single expression directly, as the §6.8 builder emits it. All escape and
rebuild assertions remain. The prior curly wrapper made the escape fixture
miss its target and take the fallback branch.

The broader CLI check gate reported E0505 in the captured-service-row
regression, which was outside the earlier 2d gate. Initial probes compared
unsupplied/supplied rows, alpha-renaming and a finite annotation. A tentative
local solver normalization made it pass, but that change was deliberately
removed after conv's `d67bcf7e1` identified the actual migration defect:
the original program wrote `! S(Dependency) = ...`; its conversion had
lost that provider annotation. The repair restores it through a typed local
before providing the value, and retains every type, missing-service and
O0/O2 runtime assertion. A legacy-to-native lowering comparison pins the
original written contract. Spec §4.7: “Annotations are used in inference,
and the inferred type must agree with them. A type variable in an annotation
may become a more exact type.” Spec §9.2: “A provided value of type `P`
satisfies the need entry `S` when `P <: S` (§4.5 rule 4).” No solver change
remains. The entire expanded gate is restarted on this restored snapshot.

The native resolver and fa2 query/shared-value integration are drafted
outside production under `target/fa-night/draft`. Missing stages 9/10 forms
are handled by grouping native chains into canonical hidden or inline
functions, native selection tables, ordered calls and general parameter
data. Native quote captures replace the obsolete old-row compatibility
reader. Written/standalone facts and validation are being moved into a
pure syntax-three prepass, so compiler parse and macro-result readers will
no longer build an old tree. The differential retains all comparisons and
adds group, alias, recursion, parameter-data and directed-row cases.

Conv's status reports a separate unresolved guarded-corpus observation:
`review-k4d/{_loose/p,proj}/p1` accepts instead of its batch2 E0515.
That audit is the coordinator's; its expectation has not been changed here.
No result for it is claimed by this lane's requested-gate checks.

22:47 checkpoint: the full Rust gate passes 683 tests (four existing
ignored, the one requested recovery-corpus skip), CLI passes all 144,
and macro examples pass 13 with the existing skip. Std O0 passes 2170,
including `std/schema_test.null_round`; this is the baseline before the
fa2 final-reader switch. Std O2 is queued since 22:36. The wrapper takes
both memory slots for O2, so no second heavy command is started.

The pending conversion parent advances to `7e1b73741` after incorporating
the full descendant delta: review fixtures/docs and the native-test
executable-busy retry helper. No production compiler, runtime, lib, std,
examples or spec input changed in that delta. Final all-target check and
clippy will compile the added helper. No global wrapper policy changed.

23:05 checkpoint: pending conv now includes `dfcf0ed83`, review observations
and documentation only since `7e1b73741`. Independently checked unchanged
phase exits, diagnostic-code sets and runtime stdout; recorded type displays
follow current main. No production inputs changed. Std O2 remains queued
since 22:36, holding m1 and waiting for m2 held by the conversion corpus.
No second heavy command was started and no shared lock policy was changed.

The old extra differential's first substantive difference is the missing
keyed function-group binding in `l0.is`; subsequent module local-ID offsets
cascade from it. The native draft includes that later stage-9/10 form,
canonical key order, selection tables and general parameter data, rather
than normalizing IDs or weakening comparisons. The incomplete query branch
is replaced by the final native metadata index with an old/native oracle.
All 35 draft Rust files parse under rustfmt; compilation and behavioral
verification remain pending until the conversion gate releases the slot.

23:25 checkpoint: temporary native preview passed workspace all-target
check, all 677 files and 519 program comparisons of the resolver
(628/628 repository `.i` coverage), prepared quotes and the 20,000-level
deep tree. It extends missing stage-9/10 grouping and table dispatch,
function parameter data, written branch result unions and Fail error
channels. A separate genuine discrepancy gave synthetic `%arg` parameters
labels; native resolution now follows the old resolver's unlabelled rule.
No HIR/diagnostic comparisons or expectations were relaxed. Query initially
passed 23/24; combining fa2's early pun relocation with main's late splitting
retained a spurious label-binder flag. Retaining main's binder/reference
handling gives 24/24 query tests and 68 checker unit tests (four existing
ignored). The corrected 35-file draft is saved outside production.

The exact conversion source snapshot was restored by recorded SHA256,
index tree and merge parent, then latest conv `31ecae801` was incorporated,
including the provider-contract fixture repair. The earlier O2 wrapper was
cancelled at 23:16 after 39 minutes queued; it had no `i` child and had not
started a test. Only this lane's wrapper, flock waiter and scheduler were
stopped by PID. No other process or shared policy was changed. The full
conversion gate is now rerunning, with O2 last so its memory-slot wait does
not hide other gate results. Step 3 remains uncommitted until its own full
gate, and deletion remains gated on it.

23:54 checkpoint: the restored full Rust gate found one target red:
`ilang-macro --test body_walk3`, whose repository differential first failed
at `review-k4-lane/q25`: the native walker exposed the written name of a
direct `Fail(E)` service pattern, while the current lowerer creates its
error channel and no local binder. The native walker now preserves that
scope in both plain and wrapped functions; ordinary service aliases keep
their binders. Three differential snippets pin both paths. All 19 walker
tests, including the unchanged whole-repository comparison, pass. The full
macro and LSP packages pass again; syntax/check package code and dependencies
are unchanged from their successful targets in the full run. The second
CLI pass is 144/144 and macro examples pass 13 with the existing skip.
Std O0 is running without failures. Workspace check passes on the repair;
remaining std/spec/fmt/clippy gates still precede the conversion commit.

00:24 GREEN conversion checkpoint through `31ecae801`: workspace all-target
check; the syntax/check targets of the full Rust run and repaired full
macro/LSP packages (683 tests collectively, four existing ignores and the
requested reverse/recovery-corpus skip); specified CLI 144; examples 13 with
one existing skip; std O0/O2 each 2170; spec 6/80, 7/104, 9/139, 15.7/28
at both levels; Rust/source fmt and workspace all-target clippy -D warnings.
Formatting changed eight Rust files and one macro-example layout; a needless
Rust borrow and equivalent nested-if lint were fixed without changing any
assertion. The macro example was rerun after formatting. `schema.null_round`
passes at both levels. Exact commands/results are under `target/fa-night`.

This checkpoint intentionally freezes the tested conversion parent. Newer
conversion work (provider-contract rejoining before generalization, p1
E0515 regressions and converted corpus repairs) arrived during this gate.
It is next, together with the native resolver switch; it is not claimed by
this checkpoint. The old solver experiment remains discarded. No deletion
has started, and the updated native provider-plan draft is not yet compiled.

00:27: green `baf2e3b11` was published before advancing. Pending integration
now includes faD `549e50510` and latest conv `d916037a5`. The complete conv
merge was saved and replayed alongside faD, including all reviewed source,
requirement and documentation changes. Its new provider-contract rejoining
is ported to native planning/resolution and macro scope traversal; ordinary
and reused locals remain bindings. Nested rewritten providers are counted
before their parent, matching the old pass. These new native changes still
require compilation and the full gate; they are not covered by the earlier
677-file preview. Deletion remains unapplied.

00:54 checkpoint: native workspace check and provider lowering regression pass.
Focused native walker (19), rows (26), and strict differential (677 files,
519 programs; all 628 repository `.i` files) pass. The full Rust gate again
passes that differential and row suite, but `types::types` has one golden
mismatch: `soundness.out` loses the outer-call E0515 at 14:5 and 16:5.
No expected output changed. The fixture's `get = () -> Int { 1 }` annotation
was introduced by conversion `ee039fd4d` before the newer provider-contract
rejoining pass. The original main and the initial retained-contract fixture
both had `get = () { 1 }`. That original body is restored; its unchanged
golden and the differential must be rerun before any green claim. The rest
of the serialized expanded gate is continuing. Fa2 deletion is still only
prepared under ignored `target/fa-night`; no production file was deleted.

01:03: restoring the original callback body did not change the missing two
outer-call diagnostics, so that hypothesis is rejected and the annotation
is restored. The actual distinction is the new adjacent-temporary pass:
it replaces the ordinary local's generalization with direct typed-Env
inference. The soundness fixture is about generalization and now provides
`{ _syn3_0 }` to keep that ordinary local in the test. §2.2: “A block with
only an expression is a group: `{ a - 48 } * 100`.” Every expected type and
diagnostic remains. Direct rejoined contracts remain independently tested
by rows/lowering, including both full p1 E0515 cases. This second repair
still requires the focused golden run. The other full-run failure is only
the formatter skip inventory: conv d916 restored the intentionally invalid
review-rec type spread (§2.3 “A data type has no spread”), so that one source
is added to the inventory; every formatting property assertion remains.

01:07 checkpoint: the repaired soundness golden passes with its expected
file byte-for-byte unchanged. Full formatting target passes after the one
explained skip-inventory update; native walker passes all 19 again; resolver
prepared quotes, 20,000-deep tree and strict whole-repository/program
differential all pass again on the final source snapshot. Together with the
full run this gives 690 passing Rust tests, four existing ignored and only
the requested recovery-corpus skip. The earlier full Rust exit is retained
as historical failure, not hidden; its only two failed tests have passing
focused reruns. The remaining serialized CLI/std/spec/fmt/clippy gate resumed
at 01:06. No deletion is applied and no native-switch commit is claimed yet.

01:13: all seven requested CLI binaries pass 144 tests; examples pass 13
with their existing skip. Std O0 now running with final readers enabled;
spec chapter filters, Rust/source fmt, clippy and std O2 remain. No switch
commit is claimed until those complete.

01:18: `std/schema_test.null_round` passes with FINAL_AST_READERS=true
on the final native snapshot; source and assertion are unchanged. It also
passed on the committed baseline at both levels. The old fa2 failure is
not reproduced; no guessed backend change or expectation relaxation is
made. Current std O0 still finishing and native O2 remains pending.

01:29: native std O0 passes 2170; spec 6 passes 80 and spec 7 passes
104 at both levels (existing pending list unchanged). Spec 9 queued from
01:23:54 until about 01:29:09 behind the shared load guard, then started
normally. No second heavy command, priority override or wrapper edit was
used. Chapter 15.7, formatting, clippy and std O2 still gate the commit.

01:34: latest conv a5992fda arrived while the earlier chapter-9 run was
active. It fixes the real closed-function provider regression in 9117cb588:
closed function-valued locals retain their generalization boundary; open
contracts and scalar providers still rejoin. The native provider planner is
ported with binding generic context, and the temporary soundness group is
removed: the original fixture and every golden byte remain. The upstream
exact four-E0515 regression and formatter-inventory explanation are retained.
The native walker gains closed/open/generic/scalar differential coverage.
Spec §4.7: “A binding is generalized when its value is a direct value (§6.10):
the type variables of its type that are not in the types of names from
outside it become general, so each use of the name may give them different
types.” Earlier passing gates are historical; this new source still needs
its affected reruns. The scheduler is paused while its active chapter-9
command finishes; no second heavy command runs. No green commit is claimed.

01:43 latest-source checkpoint: workspace check and clippy -D warnings
pass. All syntax/macro tests, including the legacy feature, pass 272.
Affected checker unit tests pass 68 (four existing ignored); strict resolver
differential and prepared/deep tests pass again; the 27-row suite is still
finishing. The original four-Reader-diagnostic regression passes. Remaining
checks are running serially. No green switch commit and no deletion claim.

01:55: affected checker suite passes 103 (68 units, 27 rows, five type
goldens, three strict resolver/deep/quote tests; four existing ignored).
All 144 CLI tests pass on the final source, including the metadata forwarding
for standalone expression fragments added during the oracle run. The latter
small forwarding change is compiled/exercised by CLI; the prior oracle
binaries predate it and should be repeated with the full final gate. Std
O2 is compiling since 01:52:44. LSP and format checks remain queued behind
it. Step 3 is not committed and deletion remains unapplied; only baf2e3b11
is claimed landable. The final-source complete checker integration, std O0,
macro examples and spec chapter matrix have not been repeated after a599.

Deadline freeze (01:58, before 02:00): candidate remains uncommitted on
lane/fa, with faD549 and latest conv a599 merge parents. Baf2e3b11 remains
the only landable hash. The scheduler is paused; the already-running std
O2 check may finish, but no further queued heavy checks will start after
the merge deadline. Compiler sources are frozen. Full final-source checker
integration, LSP, examples, std O0, chapters 6/7/9/15.7 and final formatting/
clippy repetition remain unverified; the exact matrix/commands are in
syn3-final-ast-fa-verification.md. Step 4 is prepared only, not applied.

Post-deadline read-only result (2026-11-14T02:01:53.534619+08:00): the already-running std O2
finishes 2170/2170, including unchanged schema.null_round. Wrapper kernel
exit status is zero. Only the paused own scheduler is stopped afterward;
no queued heavy check starts. No compiler source or commit changes after
the deadline. Full final-source gate remains incomplete, so step 3 stays
uncommitted and deletion remains unapplied.

03:31 continuation: requested WIP checkpoint cb02f0cb0 preserves the previous
staged tree; main68b720e9 merged at 5bc8229db; convdb743 and native alias-aware
provider resolution compile at c36823ff7. The syntax-time provider classifier
is removed, and macro expansion retains locals until resolution. Strict
resolver/deep/quote comparisons and all 29 row tests pass. A full expanded
gate (39 CLI integration binaries plus lib and filtered spec, release CLI,
short generation model) is running under target/fa-r3. Its first clippy
found one needless borrow after borrowing virtual statements; corrected
without changing behavior, with retry pending after the active Rust run.
The new coordinator instruction permits WIP/compiling milestone commits;
earlier no-commit-before-full-green notes are historical.


Nov14 05:35 preparation milestone: the unused guarded deletion tools are
retained under `scripts/syn3-delete-*.py`, with the exact five-case capture
helper and `syn3-final-ast-deletion.md` procedure. All Python syntax checks
and the driver dry run pass. No production tree or Rust test is changed by
this milestone. Preparation fixes the macro dev-dependency boundary, retains
private parse-lowered entry points and native-fact access, and preserves full
five-case engine output assertions after the old hook is removed. Native
step three has passed all runtime/spec checks so far; generation is queued
since05:10:59 under the shared load guard, followed by source formatting and
a final four-package Rust repeat. Neither step three nor deletion is claimed
fully green yet.


06:00 coordinator status (2026-11-14T06:00:19.817813+08:00): steps2c/2d remain green at the
committed prefixbaf2e3b1. Native step3 at compiling checkpoint`3499484118`
has passed workspace check, all package tests, all39 CLI integration binaries
and lib, macro examples, std2170 at both O0/O2, spec6/7/9/15.7 at both levels,
short generation, Rust/source fmt and strict clippy. A final four-package
Rust repeat on the restored comment-API source is active in its strict
resolver differential; deep and prepared-quote comparisons already pass.
No unresolved failure is reported. The green3 commit waits for this final
confirmation. Step4 remains prepared and unapplied.


Nov14 expanded step-three gate completes at 2026-11-14T06:23:53.425280+08:00. Native source `3499484118bc3bca6b127547c9cdbecf7c72f801`
passes the full coordinator matrix, including all CLI binaries, std2170 at
O0/O2 (schema.null_round unchanged), filtered spec6/7/9/15.7, short generation,
formatting and clippy. The final Rust rerun includes restored main comment
APIs and tests. The strict resolver comparison covers677 original files and
519 programs with complete HIR/diagnostic equality, including later grouping
and selection forms. Provider fusion is decided after alias/import expansion,
and the closed-function generalization boundary is retained. Step four is
still unapplied; the detailed command/result record is in
`syn3-final-ast-fa-verification.md`. The historical fa2 null-round failure
does not reproduce; no guessed backend fix or expectation change is made.


06:26: stage12 `58a35636` merges after green native prefix`6de9a39b6`.
The append-only row-test conflict retains every test from both parents,
including two incoming regressions and both alias-aware provider cases.
Alias-aware provider resolution and cached handler-row validation both remain.
Wrapped workspace all-targets check passes; this is a compiling integration
milestone, not the full deletion gate. Next capture the five matching
engine answers, then apply the guarded deletion. The unchanged null-round
assertion passed both2170-test std runs before this merge.


Nov14 07:08 deletion audit: the five original engine comparisons pass on
`5dd20acf9`; their complete matching answers are captured before removing
the old hook. The probe is removed and the clean-head guard passes. The
old tree/lowerer/encoder/readers move exclusively to the private migration
package and cfg(test) resolver oracle; native syntax moves to the crate
root. Production syntax and resolver libraries compile. All-target repairs
retain native/old assertions through one normal dependency instance rather
than mixing Rust unit-crate identities. Normalizer helpers exposed for this
read-only test boundary contain no old nodes. Backend/inventory readers
use native Macro/Bind/Chain forms and unwrap the same source spans.

The successive all-target checks find and explain: three native reader
entry references; private renderer visibility and shared quote/identifier
references; historical unit type identities; a remaining normalizer reader
visibility; checker old-tree test helpers; the optional unused inventory
reader; and private binder/backend span references. These are compile
boundaries, not diagnostic snapshot repairs. Their fixes are uncommitted
until the workspace compiles. Check9 is queued; its exact output is
`target/fa-r3/delete-check9.log`. Green3 remains`6de9a39b6` on lane/fa.


07:16 compiling deletion milestone: wrapped workspace all-targets check
passes on attempt11. All remaining generator/backend/inventory readers use
the native tree; old-node assertions use dev-only private helpers. The
migration package is absent from normal dependencies in all four consumers
(syntax/check/macro/CLI), and the historical resolver is cfg(test) only.
No diagnostic golden has changed. Initial all-target failures are explained
in the verification ledger; private standalone checks and the complete
deletion gate still follow. Green3 remains the landable prefix until they
pass. Formatting and the private lockfile are next.
