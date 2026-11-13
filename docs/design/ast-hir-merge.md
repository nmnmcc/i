# Merging the AST and the HIR: an evaluation

The owner asked for an attempt at merging the syntax tree and the HIR and for an honest answer.
Measured at commit `d893698a`. Constraint: no change to which programs are accepted or what they do.

All line counts below were produced with `wc -l`, `grep -c` or `sed -n … | grep -cv '^\s*$'`
on 2026-11-13. Numbers marked "estimate" are not measured, and the method is given.

## 0. Summary

There are three trees today, not two:

| tree | definition | lines |
|---|---|---:|
| old AST | `crates/ilang-syntax/src/ast.rs` | 762 (of which 266 are the iterative `Drop`, lines 497-762) |
| syntax-three tree | `crates/ilang-syntax/src/syn3/ast.rs` | 427 |
| HIR node types | `crates/ilang-check/src/hir.rs` lines 264-635 (types, expressions, patterns) | 372 |

and three converters between them:

| converter | file | lines |
|---|---|---:|
| syntax three -> old AST | `syn3/lower.rs` | 1398 |
| old AST -> syntax three | `syn3/convert.rs` | 2107 |
| old AST -> HIR | `ilang-check/src/resolve.rs` (tree-walking part only) | 917 non-blank, of which 182 are pure structure copying |

The copying between the AST and the HIR is small: 182 non-blank lines out of 4527 non-blank
lines in `resolve.rs` (4.0%). The rest of `resolve.rs` is name tables, scopes, imports, type
definitions, host and export rules, and diagnostics, and it stays whichever tree layout is used.
The duplication that costs something is between the two syntax trees: 3505 lines of
converters (`lower.rs` + `convert.rs`), two parsers, two printers and two Lisp bridges. The
syntax-three plan already removes it.

Recommendation: keep two trees (option a). Make the syntax-three tree the only syntax tree,
and lower it to the HIR in one step inside the resolver. Do not merge the syntax tree and
the HIR. Borrow two ideas from merged designs: keep written-name spans in the HIR, as OCaml's
`Texp_ident` does, and later perhaps give nodes ids instead of using spans as keys. A small,
mechanical first stage exists, but it is preparation and not a merge (section 5).

## 1. What each tree carries that the other does not

### 1.1 Syntax trees (old AST and syntax three), not in the HIR

- **Unresolved names.** `ExprKind::Name { module: Option<Ident>, name: Ident }`,
  `PatternKind::Constructor { module, name, .. }` / syn3 `Ctor`, `TypeKind::Named { module, name, args }`.
  `m.x` is a `Field` in the tree because the parser cannot tell a module from a value
  (`ast.rs:248-253`). In the HIR, `m.x` becomes `ExprKind::Name(ValueRef)` and the module ident and its
  span are dropped (`resolve.rs:3419-3437`).
- **Macro and quote nodes.** `ExprKind::Quote(Box<Datum>)`, `PatternKind::Quote { datum, template, captures }`,
  `ItemKind::MacroItem`, `TypeKind::Call` ("kept for macros"), and syn3 `Quoted`, `ItemKind::Macro`,
  `Attribute.args: Option<Vec<DItem>>`. The HIR has no quote expression: `quote_lower.rs` (160 lines)
  rewrites quote expressions before resolution, and the resolver rejects any that are left
  (`resolve.rs:3311-3318`). A quote pattern becomes a *resolved* `PatternKind::Quote { template, syntax: TypeId, functions: [DefId; 5], captures }`
  (`hir.rs:585-590`, `resolve.rs:3827-3880`).
- **Surface sugar (syntax three only).** `Interp`, `Data(Vec<DItem>)`, `Chain(Vec<Function>)`,
  `Provide`, `Pipe`, `PatternKind::And`, `DPItem`, `PItem` keys and choices, `Function.generics/guard/services`,
  `FnType { generics, param, services, ret }`, `TypeRhs::{Opaque, Data, Type}`. `syn3/lower.rs` removes all of
  them. The printer needs them, because the tree is built so that `parse(print(t)) == t`. This is
  why `Sp` takes no part in equality (`syn3/ast.rs:13-21`).
- **Old-AST desugarable forms.** `ExprKind::Clauses` (a clause block that is not on the right of `|>`),
  `Case` with one clause, and `RecordUpdate { constructor: Box<Expr> }`. The resolver
  desugars them (`clause_function`, `resolve.rs:3492-3541`; single-clause case to a block, `resolve.rs:3368-3402`).
- **Items as written.** `Module { doc, imports, exports, items, imports_span, exports_span }`,
  `Item { doc, attributes, kind, span }`, `Attribute`, `ImportItem`, `ExportItem`, `TypeDef { alias_pattern, params, head, body }`,
  `Binding.type_params`. The HIR has none of these as a tree. It has tables:
  `Program.modules/types`, `Module.defs/bindings/import_names/imported_types`, and `Def { exported, deprecated }`,
  `TypeDef { host, constructors_exported }`, `DefKind::External`, `TopBinding.main`, built from the attributes.
- **Documentation comments.** Only `doc: Vec<String>` on `Module` and `Item`. Neither syntax tree
  holds ordinary comments or trivia. The formatters lex the source again and merge comments by span
  (`format.rs:47-51` uses `lex(source)`; `syn3/print.rs:34-52` uses `print_with_comments(module, source, &lexed.comments)`).
  The HIR has no doc comments. The LSP parses the source again to get them (`query.rs:317-331`, `parse_ast`).
- **Stack-safety machinery.** `ast_clone.rs` (373), `ast_eq.rs` (530), and the iterative `Drop`
  (266 lines in `ast.rs`), all for trees that macros produce. The HIR uses derived `Clone` and
  `Debug`, with no `PartialEq` (`hir.rs:437`). `crates/ilang-check/tests/hir_deep.rs` covers deep resolved trees.

### 1.2 The HIR, not in the syntax trees

- **Resolved references and ids.** `ValueRef::{Local(VarId), Def(DefId), Ctor(CtorRef)}`, `Binder { var: VarId }`,
  `TypeKind::Var(TyVarId)`, `TypeKind::Named { ty: TypeRef }`, `TypeKind::Of { value, site: OfId, projection }`,
  `RecordUpdate { ctor: CtorRef, ctor_span }`, `RecordLabel.parameter: Option<TyVarId>`, and `Param::Env { var }`.
  The AST has no node ids. Where later phases need identity, they key on spans:
  `Program.expansion_scopes: HashMap<(ModuleId, Span), _>`, `typed_originals: HashMap<(ModuleId, Span), Expr>`
  (`hir.rs:60-63`), and `CheckedInputs { expressions: HashMap<Span, Expr>, binders: HashMap<Span, VarId> }` (`resolve.rs:4574-4577`).
- **Error nodes.** `ExprKind::Error`, `PatternKind::Error(Vec<Pattern>)`, `TypeKind::Error`. Neither syntax tree has
  error nodes. The parsers recover by skipping (`syn3/parse.rs:292`, `parser.rs:378`). A module with
  a syntax error is not expanded (`ilang-macro/src/project.rs:44-47`), and the LSP falls back to
  checking without expansion (`ilang-lsp/src/analysis.rs:208-213`).
- **Computed facts.** `Binding.direct`, `Binding.ty_vars`, `Param::Pattern.label`, `Function.span`,
  `Clause` choices that share `VarId`s, and the whole-program data (`Order`, `of_sites`,
  `alias_patterns`, `host_fns`, `external_fns`, `typed_macros`, `expansion_names`).

### 1.3 Shape overlap (measured by reading the enums)

| enum | old AST | syn3 | HIR | same in old AST and HIR, apart from names and ids |
|---|---:|---:|---:|---|
| `ExprKind` | 19 | 18 | 18 | Int, Float, String, Tuple, Record, List, BitArray, Block, Call, Field, TupleIndex, Unary, Binary (13) |
| `PatternKind` | 13 | 13 | 14 | Int, Float, String, Tuple, List, BitArray, Typed, Prefix, Var (9, with binders changed to `VarId`) |
| `TypeKind` | 11 | 10 | 12 | Lit, Hole, Tuple, Record, Fn, Union, Inter, Diff (8) |

The old AST and the HIR are close; that is the rustc situation. The syn3 tree is further from
the HIR. Of its 18 expression kinds, only Int, Float, String, List, BitArray, Block, Field, Unary
and Binary carry over unchanged. Data, Call (`DItem`), Chain, Pipe, Interp and Provide all change
shape on the way down. A merge would be with the syn3 tree, because the old AST is being deleted.

## 2. Who consumes each tree

### 2.1 Syntax-tree consumers (`wc -l` of the file; count of `ast::` lines where an import alias hides it)

- **Parsers and printers (old):** `parser.rs` 2158 + `parser/trampoline.rs` 233, `print.rs` 1001,
  `format.rs` 1791 (the `i fmt` and LSP formatter: `ilang-lsp/src/format.rs` 12 lines calls
  `ilang_syntax::format::format`), `walk.rs` 337, `shape.rs` 164, `ast_clone.rs` 373, and `ast_eq.rs` 530.
  Total for the old-syntax files: 7349.
- **Syntax three:** `syn3/` is 8596 in total: `parse.rs` 1668, `print.rs` 1197, `lisp.rs` 1734,
  `convert.rs` 2107, `lower.rs` 1398, `ast.rs` 427, and `mod.rs` 65.
- **Lisp surface and macro bridge:** `ilang-syntax/src/lisp/` is 9394 in total. The parts that touch the AST
  are `encode.rs` 1114 and `encode_charge.rs` 718 (AST to Datum, for macro inputs), `to_ast.rs` 1131
  (Datum to AST, for macro outputs and `.is` files), `from_ast.rs` 517, `to_source.rs` 404,
  `source_syntax.rs` 387, `scoped.rs` 579 (origins of expanded spans), `expansion.rs` 269,
  `definitions.rs` 604, `definition_table.rs` 308, and `quote_lower.rs` 160.
- **Macro engine:** `ilang-macro/src` is 5548 in total. It holds AST nodes directly in `body_walk.rs` (545, 33 `ast::` lines),
  `body_engine.rs` (498), `item_phase.rs` (987, 11), `typed.rs` (653, 19), `call.rs` (76), `prepare.rs` (305),
  `registry.rs` (279), and `definition_world.rs` (699). `project.rs:101` clones each module's AST to expand it.
- **Checker:** `resolve.rs` (4644, 107 `ast::` lines) is the only large one. `lib.rs` (`Parsed.ast`),
  `typed_macro.rs`, `types/lits.rs:721-724`, `types/known.rs:750-760` (`UnOp` only), and `types/typed_macro.rs` (`LitValue` only) use a little.
- **LSP and queries:** `query.rs` (2264, 20 `ast::` lines) and `query/nav.rs` (694, 18) parse the
  *unexpanded* source again (`query.rs:304-331`) for doc comments, written type parameters
  (`query.rs:1092-1103`), macro calls (`nav.rs:334-370`), and binder spans (`nav.rs:410-415`).
  `query/actions.rs:692` uses it too. `ilang-lsp/src/analysis.rs:313-319` edits an AST for completion.
- **CLI:** `commands/expand.rs` (173) prints expanded trees.

### 2.2 HIR consumers

The table counts lines that match `ExprKind::|PatternKind::|TypeKind::|Stmt::`, as a proxy for
code that inspects tree nodes; files that only use ids such as `TypeId` or `DefId` are left out.

| file | lines | node-match lines |
|---|---:|---:|
| `ilang-wasm/src/lower.rs` | 4037 | 191 |
| `ilang-check/src/types/infer.rs` | 4118 | 114 |
| `ilang-check/src/query.rs` (mixed with AST) | 2264 | 106 |
| `ilang-check/src/types/pattern.rs` | 1399 | 60 |
| `ilang-check/src/types/known.rs` | 1153 | 49 |
| `ilang-check/src/types/value_types.rs` | 2029 | 33 |
| `ilang-check/src/order.rs` | 563 | 32 |
| `ilang-check/src/unused.rs` | 208 | 27 |
| `ilang-check/src/types/alias.rs` | 352 | 13 |
| `ilang-cli/src/commands/package.rs` | 2432 | 11 |
| `ilang-check/src/types/coverage.rs` | 1675 | 10 |
| `ilang-check/src/types/typed_macro.rs` | 1094 | 10 |
| `ilang-check/src/providers.rs` | 1283 | 9 |
| `ilang-check/src/types/mod.rs` | 708 | 9 |
| `ilang-macro/src/typed.rs` | 653 | 9 |
| `ilang-check/src/hir.rs` own walkers (`walk_type`, `binding_types`, `visit_expr_mut`, `expression_binders`) | lines 680-1153 | — |

Full expression walkers, using one `RecordUpdate` arm as the proxy:
`wasm/lower.rs` 3, `hir.rs` 3, `infer.rs` 2, `query.rs` 2, `query/nav.rs` 2, `order.rs` 1, `unused.rs` 1.
Error-node arms (`ExprKind::Error`): `wasm/lower.rs` 3, `infer.rs` 2, `order.rs` 1, `unused.rs` 1, `query.rs` 1, `query/nav.rs` 1.

The LSP uses HIR spans for occurrences (`query.rs:1010`, `compute_walk`) and has to work
around the dropped module ident: "`m` in `m.x`: the HIR has one name for `m.x`" (`query.rs:953-962`).

## 3. Three options

### Baseline numbers (measured)

- `resolve.rs`: 4644 lines and 4527 non-blank.
- Tree-walking functions in `resolve.rs` (`ty`, `record_label` at 2014-2209; `expr` through `pattern` at 3257-3986): 917 non-blank.
- Pure structure copy inside them: **182 non-blank lines**. These are arms that rebuild a node with the same shape
  and no lookup, scope or diagnostic. They were classified by hand, range by range:
  3310, 3319-3321, 3344-3365, 3403-3418, 3439-3447, 3479-3490, 3543-3548, 3572-3579, 3583-3586,
  3665-3669, 3692-3697, 3775-3780, 3826, 3882, 3898-3913, 3944-3954, 3960, 3976-3982, 3985-3986,
  2042-2043, 2119-2120, 2136-2159, 2172-2184, and 2205-2208. The error margin is about ±20 lines,
  because some arms mix a copy with a `record_label` or `bind` call.
- The remaining ~735 tree-walking lines handle scopes (`Frame`, `pending`, `fn_depth`), binder rules, choice
  consistency, labels, environment parameters, quote services, typed-macro checked inputs, and desugaring.
- HIR node type definitions: 372 lines (`hir.rs` 264-635).
- `syn3/lower.rs`: 1398 lines. Its `expr` (893-1037) is about half copy and half desugaring (Interp,
  Data, Chain, Provide, Pipe-match, core `Never`). `chain` (108 lines), `fn_type` (103), `items` (92), `binding` (76),
  `and` (65), `provide` (40) and `clause` (38) are desugaring and transition errors
  ("needs stage N of syntax three"), and they must survive while those stages are missing,
  because they decide which programs are accepted.

### (a) Keep two trees: the syn3 tree and the HIR

The syntax-three plan makes the syn3 tree the only syntax tree. The resolver then reads it
directly, and the desugarings of `lower.rs` move into the resolver's walk, which already
desugars `Clauses` and single-clause `Case`.

- Deleted by the syntax-three plan, not by this option: old-syntax files 7349, `convert.rs` 2107,
  and the old-AST halves of `lisp/`. This is the upper bound; the exact share of `lisp/` is not measured,
  because `syn3/lisp.rs` (1734) takes over some of it.
- `lower.rs` (1398): an estimated 700-900 lines move into `resolve.rs` as arms that produce HIR
  directly, and its ~180 lines of glue (`id`, `opt_id`, `binder`, `expr`/`pat`/`ty` constructors, and the
  copy arms) disappear. Method: function sizes from the `grep -n fn` listing of `lower.rs`; the
  copy arms of `expr`, `pattern` and `ty` are counted as removed and the desugaring functions as moved.
- AST-to-HIR copying stays at about 182 lines.
- No consumer of the HIR changes.

### (b) One tree with phase parameters (Trees that Grow, or Gleam-style generics)

`Expr<P: Phase>` with associated types for the name payload, binder payload and type reference, plus
extension slots for phase-only variants, uninhabited in the other phase.

- Deleted: the HIR node definitions, 372 lines (measured). The 182 copy lines do not go away.
  The renamer still has to build a new tree, because the type of every name changes. In GHC,
  `rnExpr :: HsExpr GhcPs -> RnM (HsExpr GhcRn, FreeNames)` (`ghc/compiler/GHC/Rename/Expr.hs:300`)
  rebuilds the tree just as `resolve.rs` does. A generic `map_names` fold could replace the copy
  arms, but it would itself be about 150-200 lines (estimate: one arm per variant, the size of `hir.rs`'s `visit_expr_mut`, 115 lines, plus patterns and types).
- Added (estimate): the generic tree is the union of syn3's 18 and the HIR's 18 expression kinds, about 26
  variants with 6-8 phase-only ones; the same holds for patterns and types. Expect about 600-700 lines
  for the definitions plus a `Phase` trait with 15-25 associated types. GHC needs 380 `type family X…`
  declarations (`ghc/compiler/Language/Haskell/Syntax/Extension.hs`, measured with `grep -c`) and 257
  `type instance` lines in `GHC/Hs/Expr.hs` alone. Net change in definitions: about −372 + 200 to 300.
- Cost to consumers (measured counts, estimated effort): every HIR consumer in section 2.2 changes its
  signatures to `Expr<Resolved>`. Every exhaustive match through a reference (`match &e.kind`, which
  is how the consumers are written) also needs an arm for each parsed-only variant, even when the
  payload is uninhabited, because stable Rust does not let you leave out arms for empty types
  behind a reference. That is about 15 full walkers multiplied by 6-8 dead arms in expressions, plus
  the same for patterns and types: roughly 200-300 lines of `Interp(n, ..) => match *n {}`.
- The syntax side also pays. `Sp`-equality (`parse(print(t)) == t`) and the stack-safe `Clone`, `Eq` and `Drop`
  must become generic, or must also be paid for on the HIR.
- The structures differ in the type system as well as in node payloads. The syn3 `Function { generics, params: Vec<PItem>, guard, services, ret }`
  and the HIR `Function { params: Vec<Param>, ret: Option<RetType{value, err, env}>, span }` do not
  line up through a name parameter, so the shared tree would hold both.
- Net: a negative saving with large churn in files that other lanes are rewriting (`infer.rs`, `solve.rs`, `resolve.rs`, `lower.rs`).

What Gleam actually does (`gleam/compiler-core/src/ast.rs`): items are generic over the type
annotation (`Function<T, Expr>` at line 889, `Definition<T, Expr, PackageName>` at 1292,
`Arg<T>` at 210, `Constant<T>` in `ast/constant.rs:12`), but expressions are **two separate enums**,
`UntypedExpr` (`ast/untyped.rs:11`, 341 lines) and `TypedExpr` (`ast/typed.rs:15`, 1940 lines). Its
formatter (`gleam/format/src/lib.rs`, 249 mentions of `UntypedExpr`, 0 of `TypedExpr`) runs on the
untyped tree, and the language server (`gleam/language-server/src/{engine,reference,completer}.rs`) on the typed one.
So Gleam is option (a) for expressions. The generic part, items, has no counterpart in i, because the
HIR's top level is tables and not a tree of items.

### (c) The syntax-three parser builds HIR-shaped nodes; names are resolved in place later

`Name { written: (Option<Ident>, Ident), res: Option<ValueRef> }`, and so on, filled in by a mutable
pass.

- Deleted: the HIR node definitions (372) and the 182 copy lines. The resolver's other ~735
  tree-walking lines stay as an in-place visitor.
- It does not work as stated. The HIR shape is desugared (no `Chain`, `Pipe`, `Interp`, `Data`,
  `Provide`, `And`, or `Clauses`), but:
  1. **Macros see syntax.** `lisp/encode.rs` and `encode_charge.rs` (1832 lines) turn the written tree into
     the Datum that macros receive, and `to_ast.rs` reads their output back. If the parser
     desugared, a macro would receive `{ "a" <> x }` where the user wrote `"a\(x)"`, a function where
     the user wrote a chain, and so on. That changes what macro programs compute, which breaks the
     rule.
  2. **The formatter and the printers need the written shape** (`syn3/print.rs`, the `parse(print(t)) == t`
     property, and `syn3::lisp::to_lisp` for the Lisp surface).
  So the syntax tree would have to stay anyway, giving three trees again (syn3, HIR-shaped
  unresolved, HIR-shaped resolved), or the HIR-shaped tree would have to keep every surface form, which is (b)
  without type safety.
- Other costs: resolution reads `&[Parsed]` immutably and is run again for typed-macro output in a
  cloned `Program` (`resolve.rs:4580-4644`, `typed_output`). Expansion clones each module's tree
  (`ilang-macro/src/project.rs:101`). The LSP needs the *unexpanded* tree next to the resolved, expanded one
  (`query.rs:304-331`). In-place resolution therefore still needs copies, and `Option<Res>` fields
  move "is this resolved?" from the type checker to runtime `expect`s in about 20 consumer files.
- One real benefit: typed macros already build *mixed* trees, with fresh syntax around holes that
  were resolved earlier, joined through span-keyed maps (`CheckedInputs`, `resolve.rs:3259-3288`, and
  `ilang-macro/src/typed.rs:343-367`). A tree with resolution slots could hold such a hole directly.
  That benefit is real but narrow; section 5, stage 3, gets it more cheaply with node ids.

### Comparison (measured unless marked)

| | (a) two trees | (b) phase-generic | (c) parse to HIR shape |
|---|---|---|---|
| HIR node definitions removed | 0 | 372 | 372 |
| AST-to-HIR copy removed | 0 | ~0 (a fold replaces it; estimate) | 182 |
| New lines | `lower.rs` desugarings move into resolve (estimate 700-900 moved, not added) | 600-700 definitions + 200-300 dead arms (estimate) | resolution slots + mutable visitor + still a surface tree (estimate: more than removed) |
| Consumer churn | none | all of section 2.2 | all of section 2.2, plus runtime `expect`s |
| Macro-visible behaviour | unchanged | unchanged | changes unless a surface tree is kept |
| Conflicts with current lanes | low, once they land | high (`infer.rs`, `solve.rs`, `resolve.rs`, `lower.rs`) | high |

## 4. Risks

- **Macros (quote and unquote).** Macros run before resolution, on written syntax: `ilang-macro`,
  5548 lines, plus the Datum bridge in `lisp/`. Quote patterns are resolved late, against the
  `\0quote-syntax` module (`resolve.rs:3834-3871`). A merged tree must represent "not yet resolved"
  for the whole macro phase and "resolved" afterwards, and typed macros mix the two
  (`typed_output`, `CheckedInputs`). Option (b) types this but cannot express the mixture without an
  extra variant. Option (c) expresses the mixture but loses the static guarantee. Option (a) keeps today's
  span-keyed bridge.
- **LSP.** It needs three views: the unexpanded written tree (doc comments, written type parameters, macro
  calls), the expanded resolved tree (occurrences, types), and origins for expanded spans
  (`hir.rs:131-157`, `Program::origin`, over `lisp::scoped::ScopedSource`). A merge removes none of these
  needs. What does hurt today is the HIR dropping written-name spans (`query.rs:953`). That is fixable inside the HIR,
  as OCaml does: `Texp_ident of Path.t * Longident.t loc * Types.value_description` keeps
  the written long identifier with its location next to the resolved path (`ocaml/typing/typedtree.mli:184`).
- **Formatting.** Comments are not in any tree; both formatters lex again and merge by span. The formatter
  needs the exact written shape (sugar, groups, `Sp`-insensitive equality). A HIR-shaped or
  generic tree that drops or adds forms would break the `parse(print(t)) == t` tests. Gleam keeps
  comments in a side table (`ModuleExtra`, `gleam/compiler-core/src/parse/extra.rs:11-18`) and formats only the
  untyped tree; rustfmt likewise works on `rustc_ast`, not the HIR.
- **Lisp surface (`.is`).** `.is` files reach the checker through `lisp::parse_source`, which gives the old AST
  (`lisp/mod.rs:268-276`), or through `syn3::lisp::from_lisp` and `lower_lisp` (`syn3/mod.rs:34-41`). Quote
  patterns in syntax three pass through `unconvert_quote` and the old Lisp pattern parser (`lower.rs:1051-1058`).
  Every option has to move this off the old AST. Only (a) does it without also changing the HIR.
- **Error recovery.** The syntax trees have no error nodes; the HIR has three kinds. Merging would
  either add error nodes to the parsed phase, which every syntax consumer including macros would then have to handle,
  or make them resolved-only, which is (b) again. Today a syntax error stops expansion
  (`project.rs:44-47`), so nothing downstream sees a partial syntax tree. A merge would not change that, and changing it
  would change behaviour.
- **Stack depth.** The AST is stack-safe for `Clone`, `Eq` and `Drop` (1169 lines: `ast_clone`, `ast_eq` and the iterative drop) because macros
  build deep trees. The HIR derives `Clone`. A merged tree must choose one policy for both: either the HIR pays
  for stack safety, or the macro side loses it.
- **Concurrent lanes.** Other lanes are rewriting the `ilang-syntax` parsers, `types/infer.rs`, `solve.rs`, `display.rs`,
  `resolve.rs` and `wasm/lower.rs`. Options (b) and (c) touch all of them; (a) touches only
  `resolve.rs`, and only after the syntax-three lanes land.

## 5. Recommendation and staged plan

**Recommendation: (a). Do not merge the syntax tree and the HIR.** The numbers do not support it.
The merge could remove at most 372 lines of type definitions and 182 lines of copying (554 in
total, 3% of the 17,419 lines across `ast.rs`, `syn3/*.rs`, `hir.rs`, `resolve.rs` and `query.rs`). Against that, it costs a phase
trait or runtime resolution slots, dead match arms in about 15 walkers, and a forced choice
between macro-visible behaviour and keeping a surface tree anyway. Every reference compiler that has macros or a formatter keeps
the written tree separate from the resolved one:

- rustc: `rustc_ast` is expanded by macros and lowered to `rustc_hir`. `rustc_ast_lowering/src/lib.rs:1-31` says the
  lowering is "mostly a simple procedure, much like a fold" and treats ids and spans as the
  hard part (12017 lines in `rustc_ast_lowering/src/*.rs`).
- GHC: the types are shared through Trees that Grow (`GhcPass 'Parsed/'Renamed/'Typechecked`, `GHC/Hs/Extension/Pass.hs:38`;
  `IdGhcP` at `GHC/Hs/Extension.hs:159-162`), but the renamer still rebuilds the tree (`GHC/Rename/Expr.hs:300`),
  and phase-only forms need extension constructors (`XXExprGhcRn`/`XXExprGhcTc`, `GHC/Hs/Expr.hs:652-689`).
  The saving is in shared declarations, at the cost of 380 type families.
- OCaml: `Parsetree` (1193 lines) and `Typedtree` (971 lines) are separate, and the typed tree keeps the written
  identifier (`typedtree.mli:184`).
- Lean 4: macros work on `Syntax`, which has `missing` for parse errors (`Init/Prelude.lean:5146`); elaboration produces `Expr`
  (`Lean/Expr.lean:300`); the LSP maps back through info trees that store the `Syntax` (`Lean/Elab/InfoTree/Types.lean:74-89`, `ElabInfo.stx`).
- Roc (now in Zig): `src/parse/AST.zig` (3355 lines) and the canonical IR `src/canonicalize/CIR.zig`/`Expression.zig`
  are separate, and canonicalization (`Can.zig`, 21529 lines) also removes sugar (`roc/Glossary.md`, "can" section).
- Gleam: separate `UntypedExpr` and `TypedExpr`; only the item wrappers are generic (section 3b).

The duplication worth removing is the **two syntax trees**, which the syntax-three plan already
targets: 3505 lines of converters, plus the old parser, printer, formatter, Lisp bridge and stack-safety files.

### Staged plan

**Stage 0: small and mechanical; a few hours; no behaviour change.**
Move the leaf vocabulary that all three trees share out of `ast.rs`, so that deleting
`ast.rs` later does not touch the HIR or its consumers.

- New file `crates/ilang-syntax/src/leaf.rs` (or `common.rs`), registered as `pub mod leaf;` in `lib.rs`.
- Move from `crates/ilang-syntax/src/ast.rs`:
  - `UnOp` (lines 162-166);
  - `BinOp` and its `impl` with `level` and `text` (168-239);
  - `LitValue` and its `impl` (481-495), which calls `crate::print::string_literal`. Keep the call; it is a path, not a dependency cycle.
  This is about 98 lines.
- In `ast.rs`, add `pub use crate::leaf::{BinOp, LitValue, UnOp};`. `syn3/ast.rs:11` already does
  `pub use crate::ast::{BinOp, LitValue, UnOp}`, so it keeps working. Repoint it to `crate::leaf` only if no lane is editing it.
- Do **not** edit `hir.rs`, `types/infer.rs`, `types/lits.rs`, `types/known.rs`, `types/typed_macro.rs` or `ilang-wasm/src/lower.rs`.
  They keep compiling through the re-export (`ilang_syntax::ast::{BinOp, UnOp, Ident, LitValue}`), and other lanes are editing several of them.
- Leave `Ident` alone. The HIR uses `ast::Ident { name, span: Span }` (`hir.rs:14`) and syn3 uses `Ident { name, span: Sp }`.
  Choosing between them is a design decision (see stage 2), not a mechanical move.
- Check: `cargo build` and the test suite with no expected changes, since all moved items are plain type definitions with derives.
  Risk: a merge conflict in `ast.rs` if a parser lane edits those exact lines. They are leaf enums that the syntax-three work does not change.

Stage 0 is not done yet: it is preparation, not a merge, and it edits `ast.rs` while the
syntax lanes are rewriting the parsers. Do it when those lanes land, or skip it, since stage 1
deletes `ast.rs` and can move the three enums then.

This stage is honest preparation for (a). It does not merge anything, and there is no small mechanical
stage that does merge the AST and the HIR: every real merge step changes either the HIR's type, which touches the ~20
consumer files above, or the shape that macros see.

**Stage 1: after the syntax-three lanes land the macro engine and the Lisp surface on the syn3 tree (stages 8-12 of `docs/design/syntax-3.md`).**
Point `resolve.rs`'s walk (`expr`, `pattern`, `ty`, `block`, `function`, `clause`, and the item phases) at
`syn3::ast` instead of `ast`, and move the desugarings of `syn3/lower.rs` into it. `lower.rs`'s transition
errors ("needs stage N") must move unchanged while those stages are missing. Then delete `lower.rs`,
`convert.rs` and the old-syntax files. This should be done after, or by, the lane that rewrites `resolve.rs`,
not in parallel with it.

**Stage 2: optional, borrowing the useful part of a merge.**
Keep written spans in the HIR where the LSP needs them. Two examples: `ExprKind::Name` gains the span of the written
module qualifier, which removes the workaround at `query.rs:953-962`, and constructor patterns keep the span of the
constructor name. This is additive, about 10-30 lines in `hir.rs` and `resolve.rs` (estimate), and changes no checking.

**Stage 3: optional.**
Replace span-keyed side tables (`expansion_scopes`, `typed_originals`, `CheckedInputs`) with node ids that
the parser or expansion assigns, as rustc does ("IDs are assigned to AST nodes just before
lowering", `rustc_ast_lowering/src/lib.rs:8-10`). This is the one real problem a merged tree would have solved:
typed-macro holes. It should be weighed separately, after the typed-macro and resolver lanes settle.
