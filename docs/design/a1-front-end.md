# Design: the A1 front end, and how A2 and A3 migrate

Status: design note for stream A1 (A1-D) of the redesign plan. Base: `dcb19df`. Input: the accepted `docs/design/syntax.md` (G0). Its language text is fixed. Its "Notes for A1 and A2" and "Notes for A3" are input that this note corrects where it must (§10). This note also gives the mechanics of A2 and A3.

Words used here:

- **Old mode** and **Both mode**: the two modes of the parser. Old mode reads only the syntax at `dcb19df`. Both mode reads the old and the new syntax.
- **The corpus**: every `.i` file in `git ls-files`, every ```` ```i ```` block of the `*.md` files, and the i source in Rust strings that the A2 list names.
- **Base**: `dcb19df`, the start of A1.
- **Plan**: the approved plan, `twinkling-swinging-perlis.md`.

## 1. The problem

A1 must make the front end read the syntax of `syntax.md` while the whole corpus is still in the old syntax, and A2 must then rewrite every line of i code. Four things make this hard:

1. Old code and new code can share a file and a token sequence. A `{` that begins an expression is a block in the old syntax, but in the new syntax its first statement decides whether it is a clause block (§6.1). One old form, a block whose first statement is an unbound function with a `->` part, reads as a clause block by the new tests. Some such programs are correct under both readings, with different meanings.
2. The plan forbids changes to `infer.rs` and `lower.rs`, and gives A1 only small ranges of `resolve.rs`. But the new forms need a new desugaring, a new kind of type variable, and several checks in places owned by T1 and T2.
3. The plan wants proof by machine that nothing changes meaning: WAT byte-equal on unformatted sources, and a `wat-diff` with a short allowed list on formatted sources.
4. The migration must reach every line, including files where layout is what the test checks, Markdown, Rust strings, comments, and the `// at:` positions that tests rely on.

## 2. Decisions

Each decision has its reason. "Coordinator" means the decision is outside what the plan already allows and needs explicit sign-off (§14).

- **D1. The lexer does not change until A3.** `import`, `export`, `case`, and `if` stay keyword tokens; `if` stays out of `can_begin`; the parser keeps `no_brace` for the subject of an old `case`. Reason: old code needs the two old line rules (finding 1), and the can-begin and can-end sets of the new §1.7 are already the lexer's sets except for `if`. `?`, `|`, `->`, and `|>` already never make a break.

- **D2. Two parser modes behind one lexer.** `parse_module` (Both mode, same signature, used by the checker, the LSP, query, and `format()`) and a new `parse_module_old` (Old mode, used only by the migration and the tests). Reason: finding 10. An old block such as `r = { (x) -> Int { x + n }(1) }` has no keyword that marks it as old, so the migration must read old files with the old grammar.

- **D3. Claim D: Both mode is Old mode plus error-only extensions, with one named exception.** For every source S:
  - (a) `parse_module_old(S)` gives the tree and diagnostics that `parse_module(S)` gave at base, with the new fields empty.
  - (b) If `parse_module_old(S)` has no diagnostic, then `parse_module(S)` gives the same tree, and either no diagnostic or only E0113 diagnostics (D4).

  Every new branch in `parser.rs` is guarded by the mode and sits where the old code returned an error; each carries a comment `// both: old gave <Code> here`. Reason: this is what makes the migration, the fallback of `format()` (D10), and the first equivalence check safe. It is checked by tests (§6.1), not proved.

- **D4. E0113 DualAmbiguous: a temporary parse error, removed in A3 (coordinator; needs user approval because `syntax.md` does not have it).** In Both mode, a `{` that begins an expression is reported as E0113 when its first statement starts with `(`, and the old function test of `paren()` holds there: the `)` is followed by `->`, the `->` part (`arrow_tail`) reads as a type, and a `{` follows on the same line. The `{` is then read as an old block, so the tree is the old tree.
  - Message: "until the old syntax is removed, a `{` whose first statement starts with `( … ) -> T {` is not allowed: the old syntax reads it as a block that starts with a function, and the new syntax as a clause block. For a function, bind it to a name first: `g = (x) -> T { … }`. For a clause whose expression starts with a function, put that expression in a group: `(x) -> { (y) { y } }`."
  - Programs it rejects that the old syntax accepts: exactly the old blocks of that form. The new syntax rejects them too (they read as a clause block that then fails), except where the clause reading also succeeds, so E0113 changes only the message for them, and never lets their meaning change silently.
  - Programs it rejects that `syntax.md` accepts: clause blocks whose first clause pattern is a group or tuple pattern `( … )` and whose clause expression, read as a type, is followed by `{` on the same line. In practice this is a clause expression that starts with a function, as in `{ (x) -> (y) { x + y } }` or `{ (x) -> () { log(x) } }`. The rewrite in the message gives a form that both modes accept.
  - The corpus has none: at base, a search for a `{` that begins an expression and is followed by `( … ) ->` finds only function bodies (such as `known_types.i:55`, which is a body and is never decided) and type groups. The stage-1 test `both_agrees_with_old` (§6.1) asserts zero E0113 on the corpus.
  - It is not in `spec.md`. It is listed in the `syntax.md` A3 notes (§10). Blind spec writers are told not to write a first clause with a `( … )` pattern whose expression starts with a function, and the judge treats such a mismatch as expected.

- **D5. A match reuses the `case` node.** The parser builds `ExprKind::Case { subject, clauses, syntax: Syntax::New }` for `e |> { clauses }`, and `Syntax::Old` for `case e { … }`. The only new expression node is `ExprKind::Clauses` for a clause block that is not the right side of `|>`. Reason: every existing `Case` arm (resolve, format, print, `expr_annotations`) keeps working; the `syntax` field tells the printers which words to use, and tells resolve which typing rule applies (a new one-clause match follows binding rules; an old one-clause `case` follows rule 11). This corrects the note that says the parser keeps `|>` with a `Clauses` right side (§10).

- **D6. Resolve lowers the new forms onto existing hir; `infer.rs` and `lower.rs` do not change.** A one-clause, no-guard, no-choice match becomes a hir `Block` of a non-direct `Bind` and the body; the made-up `Bind` has the span of its pattern `p`. Reason: `lower.rs` takes the stop position from `b.span.start` (`lower.rs:1622`, `:1632`), so with span `p.span` it is the start of `p`, as §6.12 says, with no change to `lower.rs`. The cost is in `query.rs`: `Scope::block` uses `cover(b.span, b.value.span)` so that the subject `e` is still found (§3.12). This corrects the note that asks for a `lower.rs` change (finding 7, §10).

- **D7. One clause with choices: E0235, an error until T1 (default), or the old `case` node (alternative). The user decides before stage 2.** Both go against §6.12 until T1. Their costs to programmers:
  - E0235 (default): `{ Ok(v) | Error(v) -> v }` is a compile error, "a clause block with one clause and choices is not supported yet; write one clause for each choice". The migration writes one clause per choice, which duplicates the clause expression. It never gives a meaning that the spec does not have. In the corpus it changes 12 clauses in 7 test files and no library code: `crates/ilang-check/tests/projects/ok_multi/src/main.i:37`, `crates/ilang-check/tests/types/case.i:66`, `choices.i:5`, `known.i:49` and `:68`, `known_choices.i:7-14` (six clauses), and `crates/ilang-syntax/tests/golden/spec_inline.i:11`.
  - Old `case` node (alternative, from the migration-first proposal): no duplicated code and no WAT difference, but choices that do not cover give E0516 instead of reporting to `Assert`, and the subject name gets a known constructor (§7.5) that the spec does not give.

- **D8. Declared type variables live in a separate kind of frame that always makes new variables.** `push_declared_frame` hides outer variables of the same name; `push_ty_frame` (old lowercase sharing) does not change. A binding or declaration that declares `UpName` variables may not use a new lowercase variable: that is E0236 TYPE_VAR_MIXED, never a silent variable (§3.10). Reason: §4.7, findings 8 and 9, and the silent fallback of `infer.conv` for a variable that no binding owns.

- **D9. Kept forms: one family of messages, split by what the parser can build.** Records (`{ name: e }`) are a parse error, E0114 RecordReserved, because their grammar is not given and no tree can be built. Every other kept form is read in full into a tree node and reported by the checker: values of types (E0229), macros in types, macro items, macro attributes and declarations with no `@external` (E0234, E0301, E0308), and alias patterns (E0233). Reason: every ```` ```i ```` block of `spec.md` must parse with no diagnostic (`tests/spec.rs`), and the §2.2 example has `@m`. Every message says "is kept for …" in the same words (§3.8, §3.11).

- **D10. `format()` writes the new syntax from stage 3, as the plan says.** It reads Both mode; when Both mode reports errors and Old mode reports none, it formats the Old tree. Claim D makes this safe: when both modes accept a file, the trees are equal, so the outputs are equal. `i fmt --from-old` forces Old mode. Reason: the plan's stage 3 and finding 10; a plain `i fmt` on an old file that gives E0113 then needs no hint, because it takes the fallback.

- **D11. One migration plan, three users.** `migrate::plan(&Module, &Env) -> Plan` is a pure function on the old tree. The formatter prints from it, the splice printer (`--keep-layout`) edits bytes from it, and `migrate::apply(&Plan, Module) -> Module` builds the tree the output must parse to. The formatter applies the plan at the Doc level and makes no AST nodes, so comments stay in source order and `closer()` only sees real spans.

- **D12. Every migrated file is checked when it is written.** On the `--from-old` path, the Old fallback of `format()`, and the splice tool: `canon(parse_module(out)) == canon(apply(plan, parse_module_old(src)))`, or nothing is written and the first differing node is reported. Plain `format()` of a file that parses in Both mode does not run this check, so the LSP does not slow down.

- **D13. Known constructors after a one-clause match: rule R7a, mechanical.** For a one-clause, no-guard, no-choice old `case` on a name `s`, whose pattern does not bind `s` and whose clause expression uses `s`, the migration writes the pattern as `s = p`. This keeps the §7.5 known constructor of `s` (§7.5: "when a pattern `x = C(…)` matches, `x` is known to be made by `C`") and every use of `s`. Each use is a report line and an allowed-list entry. Reason: the argument that no such case needs the known constructor is not a proof. A heuristic scan of the corpus finds no candidates today; the rule is there so that A2 never depends on finding exceptions by hand.

- **D14. A test-only cargo feature `one-clause-as-case` on `ilang-check` (coordinator).** It lowers a one-clause match as the old `case` node. It is used only by check E2 (§3.16), never in a normal, release, or dev-verify build, and A3 removes it. Reason: it lets the formatted corpus be compared with base strictly, so the allowed list of the plan's check 2 (E3) cannot hide a regression inside a listed function behind a one-clause difference.

- **D15. `print.rs` prints each node in the syntax it was written in, and adds no groups except the type group before `?`.** Reason: finding 13. A tree from the parser already has every expression group as a one-statement `Block`. A type group leaves no node, so `print.rs` puts `{ }` around a function type that ends a clause pattern when `?` or another `|` choice follows. No golden `.out` of an old input changes because of `print.rs`.

## 3. Exact rules

### 3.1 Lexer

No change in A1 or A2. `crates/ilang-syntax/tests/lex.rs:19-24` stays. A3 changes both (finding 16).

So until A3 these rules of the new text do not hold yet, and these old rules still hold:
- §1.4 (any `name` may be used as a name);
- `if` keeps its old place in the two lists of §1.7: it cannot begin a statement, and a line that begins with `if` goes on;
- in the subject of an old `case`, a `{` after a complete expression ends the subject.

### 3.2 Parser modes and entry points

- `pub fn parse_module(src) -> (Module, Vec<Diagnostic>)`: Both mode.
- `pub fn parse_module_old(src) -> (Module, Vec<Diagnostic>)`: Old mode. Exactly the base grammar: every `{` that begins an expression is a block; no header groups, `?` guards, type parameters, `Clauses`, `MacroItem`, attribute calls, `ValueOf`, `TypeCall`, or type heads of types; `@external` forces a declaration.
- `#[doc(hidden)] pub fn parse_stats(src) -> ParseStats { decisions, braces }` for the test of §3.5.
- `Parser` gets `mode: Mode { Old, Both }`, `decided: HashMap<usize, Brace>` and `decisions: usize`.

### 3.3 Syntax tree changes (`crates/ilang-syntax/src/ast.rs`)

- `pub enum Syntax { Old, New }`.
- `ExprKind::Case { subject, clauses, syntax: Syntax }`. The span of a new match is the whole `e |> { … }`.
- `ExprKind::Clauses(Vec<Clause>)`, span from `{` to `}`. `{}` is `Clauses(vec![])`.
- `Binding { pattern, value, span, type_params: Vec<Ident> }` (top-level bindings and `Stmt::Bind`).
- `ItemKind::Declaration { name, type_params: Vec<Ident>, ty }`.
- `ItemKind::MacroItem { module: Option<Ident>, name: Ident, args: Vec<Arg> }`.
- `Attribute { module: Option<Ident>, name, args: Vec<(String, Span)>, call: Option<(Vec<Arg>, Span)>, span }`. `args` is filled as today when every argument is a string literal with no label and there is at least one. `call` is kept in every other case with `( )`, including `@main()`.
- `TypeDef { name, params: Vec<Ident>, head: Vec<TypeExpr>, body }`. `params` holds a plain head: all lowercase names (old, until A3) or all `UpName`s with no module and no arguments (new). `head` is filled, and `params` is empty, when the head is anything else.
- `TypeKind::ValueOf { module: Option<Ident>, name: Ident }` for a lowercase `a.x`.
- `TypeKind::Call { module: Option<Ident>, name: Ident, args: Vec<TypeArg> }` and `enum TypeArg { Type(TypeExpr), Int(..), String(..) }`.
- `Module` gets `imports_span`, `exports_span: Option<Span>` and `imports_style`, `exports_style: HeaderStyle { Keyword, Group }`. Each span covers the keyword, if any, and the group.

Code that must change when these fields and variants are added (stage 1, so that stage 1 builds):
- `Case` patterns without `..`: `resolve.rs:289`, `:2007`; `format.rs:1104`; `print.rs:326`; `parser.rs:953`.
- `Declaration` patterns without `..`: `parser.rs:413`, `:431`; `resolve.rs:503`; `format.rs:914`; `print.rs:196`.
- `Binding` literals: `parser.rs:438`, `:584`.
- `ast::ExprKind` matches that must get a `Clauses` arm: `resolve.rs` `expr_annotations` (:258), `expr()` (:~1990-2010), `print.rs` (:35, :~326), `format.rs` (:~1104 and the expression printer).
- `ast::TypeKind` matches that must get `ValueOf` and `Call` arms: `resolve.rs` `type_vars` (:207), `fn ty` (:1330), `format.rs` (:~1510), `print.rs` (:~596), and the type printers in `parser.rs` if any.
- `ast::ItemKind` matches that must get a `MacroItem` arm: `resolve.rs` `collect()` (:457, `Slot::None`), `query.rs:999` (`symbols`, no symbol), `format.rs:899`, `:909`, `print.rs:194`.
- `Attribute` literal: `parser.rs:452`.

In stage 1, resolve's new arms report E0235 with the text "not yet supported" for `Clauses`, a new `Case`, and `type_params`, so the crate builds; stage 2 replaces them.

### 3.4 File header and items (Both mode)

Header, before the first item and only when no attribute comes first:
- The keyword path is unchanged (`imports_style = Keyword`), including an empty `import ()`.
- A `(` whose matching `)` (from the `close` table) is not followed by `=` is a header group. It is the imports when a `/` token is directly inside it (depth 1), read with the import item code; otherwise the exports, read with the export item code. `imports_style = Group`.
- Errors, all E0115 HeaderGroup:
  - an empty group: "a file with nothing to import or export leaves the group out";
  - an imports group after an exports group: "the imports come before the exports";
  - a second group of one kind: "a file has at most one imports group and one exports group".

Items, in this order:
1. Attributes: `@` [name `.`] name, then a `(` on the same line starts a `Call`, whose arguments are read as expressions. Then the rules of §3.3 fill `args` or `call`.
2. A group in `( )` whose `)` is not followed by `=`: E0115. With attributes before it: "a group in `( )` with an attribute before it is not a binding, because no `=` follows it; the imports and the exports have no attribute before them". Without: "a group in `( )` that is not followed by `=` is the imports or the exports, which come first in a file". An old `import` or `export` keyword here keeps E0103 and its old message until A3.
3. `UpName`: a type definition (§3.7).
4. `name ( UpName {, UpName} [,] )` followed by `=` or `:` (a scan with the `close` table): a binding or a declaration with type parameters. The name becomes `PatternKind::Var`, or `Typed` when `:` follows; `=` after it makes a binding, no `=` a declaration.
5. [name `.`] name `(` … `)` with no `=` or `:` after the `)`: `MacroItem`. The call is read in full.
6. `binding_head()`: a binding.
7. `name :` Type with no `=`: a declaration, whatever its attributes. (In Old mode, `@external` still forces a declaration.)
8. Anything else: E0101, as today.

`recover()` does not change: it already resumes at a break before `(`, a name, an `UpName`, and `@`.

### 3.5 Deciding a `{` (Both mode, in `primary()` only)

A function body (`function()` calls `block()`), the constructors of a type, and a type group are never decided.

The decision is a pure function of the tokens and the mode, so it is cached by the token index of the `{` and never changes after a speculative parse rolls back `pos` or truncates `diags`. Each `{` is decided once, and `decisions` counts the decisions. The probe saves `pos` and `diags.len()`, runs under `nested()` (which clears `no_brace` and `guard`), and restores both; it stores only the kind, never a diagnostic. A nested `{` met during the probe (for example in a bit-array size) is decided by the same function and cached.

The tests, in order; the first that holds decides:
0. The next token is `}`: Clauses.
1. Test 1: `name ( UpName {, UpName} [,] )` then `=` or `:`: Block.
1a. D4: the statement starts with `(`, the `)` is followed by `->`, `arrow_tail` reads, and a `{` follows on the same line: AmbiguousBlock. `primary()` reports E0113 at the `(` and reads an old block.
2. Read the longest pattern `p` with `pattern_in`. A type in it is read as far right as it goes, with no going back. If `|`, `?`, or `->` follows `p`: Clauses, except when `p` is a chain `n1 = … = nk = ( … )` and `->` follows, `arrow_tail` reads, and a `{` follows on the same line: then Block.
3. `p` followed by `=`, or the statement starts with `!`: Block.
4. `name :` where the pattern `name: Type` is not followed by `=`, `->`, `?`, or `|`: Clauses, with E0114 RecordReserved at the `{`: "`{ name: e }` is kept for records; a tuple type in a clause pattern is put in `{ }`". The rest is read as clauses, for recovery.
5. A `->` at depth 0 (brackets skipped with `close`) before the next break or the closing `}`, and the statement either fails to read as an expression or its longest expression is followed by `->`: Clauses. The error is the one found by reading the start as a pattern followed by `->`.
6. Otherwise: Block.

### 3.6 Clauses, guards, and pipes

- A clause is `Pattern { "|" Pattern } [ "?" Expr ] "->" Expr`. The guard is read with the `guard` flag set, so `->` ends it.
- An old `case` accepts only `if`; a clause block accepts only `?`. The other word is E0101 with the message "a guard in a clause block is written with `?`" or the old message.
- `binary()`: after the right side of `|>` is read, a right side that is exactly `ExprKind::Clauses` turns the `Binary` into `Case { subject: lhs, clauses, syntax: New }`, with the span of the whole expression.
- Any other right side keeps the E0111 check. When the right side is a `Binary`, `Field`, `TupleIndex`, or `Call` whose leftmost operand is a `Clauses`, the E0111 message is "a match is an operation at level 6, so what comes after the clause block is part of the right side; put the match in a group: `{ e |> { … } } * 10`". E0111 messages for other right sides keep their text.

### 3.7 Types and type heads (Both mode)

- A lowercase `name` alone: `TypeKind::Var` (old type variable, until A3).
- `name . name` with a lowercase second name, whatever the first name is: `TypeKind::ValueOf`. This needs no scope, which settles finding 4.
- A lowercase `name (`, or `name . name (` with a lowercase second name: `TypeKind::Call`; each argument is a type, an int literal, or a string literal.
- `name . UpName`: `Named`, as today.
- A type definition head is read as `( Type {, Type} [,] )`. All plain lowercase names: `params` (old). All `UpName`s with no module and no arguments: `params` (new). Anything else, including a mix: `head`, and resolve reports it (§3.11). In Old mode the head is read with `Self::name`, as today.

### 3.8 New parse error codes (`crates/ilang-syntax/src/lib.rs`)

- E0113 DualAmbiguous (A3 removes it), message in D4.
- E0114 RecordReserved, message in §3.5 test 4.
- E0115 HeaderGroup, messages in §3.4.

New messages under existing codes: E0111 (§3.6), E0101 for the guard word (§3.6).

### 3.9 Resolve: matches and clause blocks (`resolve.rs`, the new arm at about `:2007`)

`E::Case { syntax: Old }` is resolved as today.

`E::Case { syntax: New, subject: e, clauses }`:
- (a) One clause, no guard, one pattern `p`, body `b`:
  - with the feature `one-clause-as-case` on: as an old case (only for E2);
  - otherwise: resolve `e` in the current scope, with no new frame; resolve `p` (no type frame; `p`'s type variables come from the enclosing frames); push a normal `Frame` with `p`'s binders; resolve `b`; pop. The result is `hir Block { stmts: [Stmt::Bind(Binding { pattern: p, value: e, direct: false, ty_vars: vec![], span: p.span }), Stmt::Expr(b)], span: <the match> }`.
  - It does not call `binding()` or `is_direct`, so `e` is never in `p`'s scope and `p` is not generalized: in `g = () { (x) { f(x) * 2 } |> { f -> f(1) } }`, the `f` in the subject is the top-level `f`.
  - `infer.rs` gives `e` the expected type of `p` (`binding_expected`), so `x: T` at the top of `p` is an annotation, as §4.8 says. A `Block` gives no "not used" warning (`has_no_effect` is false), as §6.10 says of a match. The body is the last statement, so it is in tail position (§11.6).
- (b) One clause, no guard, more than one pattern: E0235 CHOICES_ONE_CLAUSE and hir `Error` (D7 default).
- (c) Otherwise: hir `Case { subject, clauses }`, exactly as an old `case`. So known types, narrowing, §7.5 known constructors, tail positions, and the order of evaluation are those of today's `case`.

`E::Clauses(cs)` (a clause block that is not the right side of `|>`):
- (a) One clause, no guard, one pattern `p`: build `ast::Function { params: [Param::Pattern(p)], ret: None, body: Block [Stmt::Expr(b)] }` with the span of the block, and call the existing function code. The label comes from `label_of`.
- (b) One clause with choices: E0235.
- (c) Otherwise: hir `Fn` with one parameter `Param::Pattern { pattern: Var("%arg"), label: None }` and the body `Block [Expr(Case { subject: Name("%arg"), clauses })]`.
  - `%arg` cannot be written in i, so no source name equals it (`__arg` would be a discard by §1.3).
  - The pattern span and the name span are zero-width at the `{`. The `Fn` span is the block span. The `Case` span runs from the start of the first clause to the end of the last; for `{}` it is zero-width just after the `{`. So no span-keyed table entry of the made-up nodes is shared with a real node.
- A top-level `f = { … }` is a hir `Fn`, so `lower.rs` still makes `f` a known function.

`is_direct` (outside the range): `E::Clauses(_)` is direct. `ungroup` already sees through a group of one. A `Case` stays not direct.

`expr_annotations` (in the range): a `Clauses` arm that recurses as the `Case` arm does; the `Case` arm adds `..`.

### 3.10 Resolve: type variables

- Frames carry a flag: `ty_frames: Vec<TyFrame { names: HashMap<String, TyVarId>, declared: bool }>`.
- `push_declared_frame(&[Ident]) -> Vec<TyVarId>`:
  - repeated names: E0226 TYPE_PARAM_TWICE, "the type variable `A` is declared more than once here";
  - a name that is a type that may be written with no module name (a quiet `lookup_type(None, name)` with no diagnostic): E0231 TYPE_VAR_IS_TYPE, "a type variable may not have the name of a type";
  - always makes new variables, so an inner declaration hides an outer one.
- `push_ty_frame` (old lowercase sharing) does not change.
- `binding()`:
  - with `type_params`: `ty_vars = push_declared_frame(type_params)`. `annotation_vars` is not used, and no lowercase frame is pushed. Pop after the value.
  - without: as today.
- `values()`, the `Declaration` branch (finding 8): with `type_params`, `push_declared_frame`; without, `type_vars(ty)` and `push_ty_frame` as today. The result is `DefKind::External.ty_vars`.
- `ty_var` (lowercase names), in this order:
  1. in a type definition: as today;
  2. found in any frame, innermost first: that variable;
  3. not found, and the last frame is a declared frame: E0236 TYPE_VAR_MIXED, "this binding declares its type variables, so `a` must be declared too, as an `UpName`: `f(A, …) = …`", and `TypeKind::Error`;
  4. not found otherwise: as today (a new variable put in the last frame).

  So in a binding with declared variables, a lowercase name lands nowhere: it is either an outer binding's variable or an error. Inner bindings with no `type_params` push their own lowercase frame as today, so old code inside new code keeps working.
- `fn ty`, the `T::Named { module: None, name, args }` branch, before `lookup_type`:
  1. look up `name` in `ty_frames`, innermost first, then in `ty_def.params` (marking it used);
  2. found with arguments: E0230 TYPE_VAR_ARGS, "a type variable takes no arguments";
  3. found: `TypeKind::Var(id)`;
  4. not found: `lookup_type` as today. For an unknown lone `UpName`, the E0202 message adds "to use a type variable, declare it after the name of the binding, as in `f(A) = …`".

  Old programs have only lowercase keys in frames and in old type parameters, so an `UpName` is never found there, and old results do not change. E1 checks this.
- `T::ValueOf`: E0229 TYPE_OF_VALUE_RESERVED, "`p.name` in a type is kept for the type of a value". `T::Call`: E0234 MACRO_RESERVED, "a call in a type is kept for macros". Both give `TypeKind::Error`.

### 3.11 Resolve: kept forms, attributes, and heads

- `MacroItem`: E0234, "a call at the top level is kept for macros".
- A declaration with no `@external`: E0308, message now "a declaration with no `@external` is kept for macros".
- An attribute with a module, or whose name is not built in: E0301, message now "the attribute `@m` is kept for macros; the built-in attributes are `@external`, `@host`, `@main`, `@test`, and `@deprecated`".
- A built-in attribute with `call` present: E0302, "the arguments of `@x` are string literals given by position" or, for `()` with nothing in it, "`@main` takes no arguments and is written with no `( )`".
- `@external` on a binding: E0303, which exists today.
- Type definition heads, in `type_defs` (after imports are known), in this order (finding 3):
  1. `head` is not empty (a part is not a plain name, or the parts mix lowercase names and `UpName`s): a custom type gives E0232 TYPE_PARAM_FORM, "a type parameter is an `UpName` with no module name and no arguments"; an alias gives E0233 ALIAS_PATTERN_RESERVED, "the left side of an alias that is not a list of type parameters is kept for alias patterns". The body is not resolved.
  2. Every part is an `UpName`, and one is a type that may be written with no module name: a custom type gives E0231, "a type parameter may not have the name of a type"; an alias gives E0233.
  `collect_type` makes the same `Vec` of parameters for old and new heads, so the E0221 check stays in line.

New checker codes in `crates/ilang-check/src/lib.rs`: E0229 TYPE_OF_VALUE_RESERVED, E0230 TYPE_VAR_ARGS, E0231 TYPE_VAR_IS_TYPE, E0232 TYPE_PARAM_FORM, E0233 ALIAS_PATTERN_RESERVED, E0234 MACRO_RESERVED, E0235 CHOICES_ONE_CLAUSE (temporary), E0236 TYPE_VAR_MIXED (A3 removes it with the lowercase forms).

### 3.12 `query.rs` (outside the range)

Every place that reads a `Bind` span, checked at base:
- `:263` `local_type`: top-level bindings only; a made-up `Bind` is never top-level. Safe.
- `:1163` `Walk::binder`: binder spans; `%arg` gets a zero-width span. It is skipped (below).
- `:1247` `Walk::block`: reads pattern and value, not the span. Safe.
- `:1413` `Scope::block`: becomes `Stmt::Bind(b) => cover(b.span, b.value.span)`. For a written binding this is `b.span`. For the made-up binding it covers `e` and `p`, so completions inside `e` still offer local names, and the names of `p` are offered in the body (`:1435`, after the span).
- `:1419`, `:1435`: unchanged.
- Binders whose name starts with `%` are left out of completions, hover, and definitions (`Walk::binder` and `pattern_binders`).
- `:999`: a `MacroItem` arm (no symbol).
- `builtin_head` changes in A2.

### 3.13 `print.rs`

Each node prints in its own syntax:
- the header by its style (keywords or groups);
- `Case Old`: `case S {`, guards `if`; `Case New`: `S |> {`, guards `?`;
- `Clauses`: `{` clauses `}`, one per line, or `{}`;
- `type_params` as `name(A, B)` before `:` or `=`;
- `MacroItem`, attribute `module` and `call`, `ValueOf`, `TypeCall`, `head` in the new forms;
- `level()` stays 10 for every `Case`.

The only group it adds: `{ }` around a function type that ends a clause pattern when `?` or another `|` choice follows. It never adds an expression group: a tree from the parser has each group as a one-statement `Block`, and `common::shape` stays strict in `golden.rs` and `spec.rs`. A tree from Both mode never holds an expression `{` whose first statement is an unbound function with a `->` part (D4), so the round trip is exact.

### 3.14 `format.rs` and the migration plan

`format(src)`:
1. Parse in Both mode. With no errors: compute the plan on that tree and print.
2. With errors: parse in Old mode. With no errors there: print from the Old tree with the plan, and run the write check (D12). Otherwise return the Both-mode errors.

`format_from_old(src)` always reads Old mode and runs the write check. `migrate::splice(src, &plan)` is the splice printer. `Formatter::module` finds the header from `imports_span` and `exports_span`, not from keyword tokens.

The plan (`crates/ilang-syntax/src/migrate.rs`, removed in A3) holds, keyed by spans of the old tree: renames, inserted declarations, groups, g-bindings, splits, dropped empty groups, choice splits, R7a rewrites, line ends, and the report. `Env` is the set of type names that may be written with no module name: `migrate::UNQUALIFIED_TYPES` (built-in and prelude types; a test in `ilang-check` asserts it equals `BuiltinType::ALL` plus the types of `lib/prelude.i`), the module's own type definitions, and the `UpName`s of its imports.

Rules:
- **R1 Header.** Drop the `import` and `export` words. Drop an empty `import ()` or `export ()` with its line. Each group and the first item start their own line (R8).
- **R2 Match.** `case S { … }` becomes `S |> { … }`.
  - R2a: `S` gets a group, `{ S }`, when its top operator is at level 1 to 5.
  - R2b: the whole match gets a group when it is an operand of a level 7 or 8 operator, of unary `-` or `-.`, a callee, or the target of `.f` or `.k`. It needs none as an operand at level 1 to 6, an argument, a list part, or a spread.
- **R3 Guard.** `if` becomes `?`. When the pattern of the last choice ends in a type whose rightmost part (through the right side of `|`, `&`, and `-`) is a function type not already in `{ }`, that function type is put in `{ }`.
- **R4 Type variables.** Owners are found exactly as resolve finds them today (`annotation_vars`, `push_ty_frame`, from the outside in). New name: take out each `_`, make the letter after it a capital, and make the first letter a capital. Variables of one binding are named in the order of first use. A name clashes with a name in `Env`, with another new name of the same owner, or with a new name of an outer owner whose variable is used inside this binding; it then takes the first free name of `A1`, `A2`, … and is reported.
  - Declare on the owner: `n(A…) =`, `n(A…): T =`, `n(A…): T` for a declaration (variables in the order of first use in its type), `Name(A…)` for a type head.
  - An owner whose pattern is not one name: inside another binding, declare on the nearest enclosing binding of one name (reported: `tyvar-moved`). At the top level, split the binding into one binding per name when its value is a tuple with one part per name (reported: `split`). Otherwise the file is refused and goes to the hand list.
- **R5 Function at the start of a `{`.** When the first statement of a `{` that begins an expression in the output (old blocks and groups, groups R2 adds, old subjects that become a first statement) starts with a function that has a `->` part and only patterns as parameters, insert `g = F` before it and put `g` in its place. `g` is the first of `g`, `g2`, … not used in that `{` (reported: `g-binding`).
- **R6 First clause that gives a function.** When the first clause pattern is `n1 = … = (…)` and the clause expression starts with a function, group the expression; when that function has a `->` part, also bind it inside the group as in R5.
- **R7 One clause, no guard.**
  - R7a (no choices): see D13. Rewrite `p` as `s = p` (reported: `one-clause-known`). Every other one-clause, no-guard, no-choice case is reported as `one-clause`.
  - R7b (choices, under D7 default): write one clause per choice, each with the same expression (reported: `choices-split`).
- **R8 Line ends.** Before a clause, statement, or item whose output starts with `(`, `-`, or `-.` and that shares a line with the one before, insert a line end with the indentation of that line. In splice mode, a file that has a CR refuses this rule and goes to the hand list, so the number of CRs never changes.
- **R9 Splice spacing.** An inserted `{ ` or ` }` goes right at the node's edges; no other space changes.
- **R10 Files with parse errors.** `--keep-layout` does only the parts that touch nodes the Old parse read (R1, R2, R3, R4 where the owner was read). The rest is by hand from the A2 list.
- **R11 Comments.** Never changed by the migration (A2 changes them by hand).
- **R12 Report.** Every R4 fallback, split, and move; every R5, R6, R7a, R7b; every one-clause case; and every refused file. Each line has a category, the enclosing top-level binding (`module.name`), the source position, and the position of `p` for one-clause lines.

### 3.15 The write check and the splice printer

- `canon(m)`: the `common::shape` text of `m` after removing every one-statement expression `Block` (a group). The only difference between the two sides of the check is the choice of groups.
- Write check (D12): `canon(parse_module(out)) == canon(apply(&plan, parse_module_old(src)))`, and `parse_module(out)` has no diagnostic (for R10 files: the same codes as before).
- Splice asserts: every byte outside an edit is unchanged; the number of CRs is unchanged; for a `format/*.i` input, the output still differs from its `.out`.
- The splice printer returns an offset map (old offset → new offset). The A2 position view uses it to check each `// at:` line that `UPDATE=1` writes.

### 3.16 The test-only feature

- `crates/ilang-check/Cargo.toml`: `[features] one-clause-as-case = []`. One `cfg!` test in the desugaring arm (§3.9 (a)).
- It never reaches a normal build:
  - `ilang-check` exports `pub const ONE_CLAUSE_AS_CASE: bool = cfg!(feature = "one-clause-as-case")`, and `crates/ilang-cli/tests/check.rs` asserts it is false, so a plain `cargo test` (and dev-verify) fails if feature unification turns it on;
  - `const _: () = assert!(!(ONE_CLAUSE_AS_CASE && !cfg!(debug_assertions)));` makes a release build with it fail to compile;
  - E2 runs in its own target directory (`$C/target-e2`) in the assigned rev lane, with `cargo build -p ilang-cli --features ilang-check/one-clause-as-case --target-dir $C/target-e2`.
- A3 removes it.

### 3.17 `scripts/wat-diff` and the allowed list

`scripts/wat-diff A B [--allowed FILE] [--renumber]`, Python 3 standard library only, run with `python3 -I`.

- Files: the union of the files under A and B. A file on one side only fails, unless it is a blind case under `tests/spec` on side B. A file whose text starts with `ERROR` on either side fails.
- **Strict mode** (no `--allowed`; the plan's rule): mask only the string of the `(data $site_data "…")` line; assert that the site count, the `(i32.const N)` of the `$sites_install` call in `$ilang_start`, is equal; every other line must be equal.
- **Allowed mode**: the same, except that the lines inside a `(func` block whose `;; path.name` comment (or its `:fnN` closures) is listed are skipped. The number and order of functions must be equal. Library functions are listed once and match in every dump.
- **Site count.** A one-clause binding gets a stop site only when infer marks its pattern as may-fail (`bind_checked`, `lower.rs:2472`). An old one-clause case compiled only when its clause covered its subject's type, and an old covering case has no site, so the expected count difference is zero, and strict equality is the default. The bound is checked by machine: the test helper `ONE_CLAUSE_REPORT=<file>` in `crates/ilang-check/tests/` reads the migration report, looks up `may_fail` at the position of each `p`, and writes `may-fail` on the entry. Only with entries marked `may-fail` does `--renumber` apply: site codes in the known site forms (`$shadow_site`, the site argument of `$async_begin`, `$stop_error`) are decoded to `module:line:col` through `$site_data`, `$sN` names become their string content, data lines are sorted, and the count difference must be at most the number of such entries. The tool fails on a site form it cannot decode.
- **Allowed file** `$C/wat/allowed-a1.txt`, one line per entry: `<category> <function path.name> <source file:line> <report id> [may-fail] <reason>`. It is made from the R12 report over the rev pool, and every entry must match a report line (checked by the equivalence view). Categories:
  - `one-clause`: a match lowered as a binding;
  - `one-clause-known`: R7a;
  - `choices-split`: R7b;
  - `g-binding`: R5 and R6 (a `g = F` local may change the WAT);
  - `split` and `tyvar-moved`: R4;
  - `hand`: anything fixed by hand in A2, with a reason.
- **Groups have no category.** A group is a hir `Block` of one `Expr`; `lower.rs` lowers it through `block()`, which lowers the last expression with the same tail flag, and `infer.rs` infers it with the same expected type. So groups must leave the WAT byte-equal; if E2 finds a difference only a group explains, that is a finding, not an allowed entry.
- Expected size at base: the scan in this design finds one-clause cases only in tests; in the WAT dumps (std, bench, examples/hello, `tests/spec`) these are `tests/spec/6/blocks.i:32` and the seven in `tests/spec/7/patterns.i:96-139` (`7/not_covered.i` does not compile and has no WAT). No `choices-split` entry is in a dumped file.

## 4. Replacement spec text, section by section (stage 1)

The text is `syntax.md` "Replacement text". The rule of `syntax.md` line 28 is applied as written (finding 18): only new text goes in (text in quotes, in code blocks, or after "becomes" or "becomes:", or in a sentence that says it is added). Quoted old text only finds the place. The other sentences of an (edits) part are notes for A1. No note, no A1-period rule (D4, D7, E0236, the old `if` line rule), and no text about the old syntax goes into `spec.md`.

| Section | Mark | What goes in |
|---|---|---|
| Opening | edits | the paragraph with the three exceptions (finding 2: the text already names them) |
| §0 | edits | the notation paragraph after the EBNF paragraph |
| §1.3 | full | the grammar and three bullets |
| §1.4 | full | two paragraphs (no keywords; kept forms) |
| §1.7 | full | the whole section |
| §2.2 | edits | the grammar; the `Call` paragraph with the `@m` example; the header paragraph; the paragraph on declarations and `MacroItem`; the attributes paragraph. Not the note for the macro streams |
| §2.3 | edits | heading "2.3 Imports"; the grammar; the tuple paragraph with its example |
| §2.4 | edits | heading "2.4 Exports"; the grammar; the first bullet |
| §2.5 | edits | the grammar; the four paragraphs; the sentence on `@external` is removed |
| §3 | edits | `Prim`, `TypeCall`, `TypeArg`; the four bullets; the `{ }` example |
| §4.3 | full | the grammar and the bullets |
| §4.7 | edits | the seven bullets in place of the bullet on annotations |
| §4.8 | edits | rule 11 and its paragraph; the two fitting paragraphs; the six known-type changes (finding 17: the wording "the bullet on matches, before **Names in patterns**, below" is already fixed at base) |
| §5 | edits | the changed sentence and the added bullet |
| §6.1 | full | the whole section |
| §6.9 | edits | the changed sentence and the three bullets |
| §6.10 | edits | the six sentence changes and the new example |
| §6.12 | full | heading "6.12 Clause blocks and matching" and the section |
| §7.1 | edits | the four sentence changes |
| §7.4 | full | the section |
| §7.5 | full | the section |
| §9.3 | edits | one sentence |
| §10.2 | edits | one sentence |
| §11.4 | edits | one sentence |
| §11.6 | full | the section |

Examples in unchanged sections stay in the old syntax until A2; Both mode reads them, so every ```` ```i ```` block still passes `crates/ilang-syntax/tests/spec.rs`. The §9.6 `// type:` comment changes in A2 phase 0 (finding 5).

## 5. Files to change, with their regions

Inside A1's ranges:
- `crates/ilang-syntax/src/ast.rs`: §3.3.
- `crates/ilang-syntax/src/parser.rs`: entry points (:18), `module()` (:286), `item()` (:397), `attribute()` (:452), `type_def()` (:474), `stmt()` (:572), `binary()` (:636), `primary()` (:766), `case()` (:921), `type_prim()` (:1228), plus the new `decide()` and `clauses()`.
- `crates/ilang-syntax/src/lib.rs`: codes, `parse_module_old`, `parse_stats`.
- `crates/ilang-syntax/src/print.rs`: §3.13.
- `crates/ilang-syntax/src/format.rs`: §3.14 (stage 3).
- `crates/ilang-syntax/src/migrate.rs` (new, stage 3).
- `crates/ilang-syntax/tests/*`: §6.
- `resolve.rs` ranges: `type_vars`/`expr_annotations` (:206-325), `ty_var` (:1382-1407), `push_ty_frame` and the new `push_declared_frame` (:1742), the desugaring arm (:2007).
- `docs/spec.md`: §4.
- `scripts/wat-diff` (new).

Outside A1's ranges (one change to the split of files for the coordinator; T1 and T2 are told to rebase on it):
- `resolve.rs`: the `T::Named` branch and the `ValueOf`/`Call` arms of `fn ty` (:1329, T2's); the head checks in `type_defs` (:1098) and the alias code (T2's); `is_direct` (:1791); `values()` Declaration branch (:1688) and `binding()` (:1760); `collect()` `MacroItem` and `Declaration` arms (:455-510); `attributes()` (:600-735) messages and new forms; the `cfg!` branch of D14.
- `crates/ilang-check/src/lib.rs`: the new codes.
- `crates/ilang-check/Cargo.toml`: the feature.
- `crates/ilang-check/src/query.rs` (T2's and A3's): `Scope::block` (:1413), the `%` filter (:1163 and `pattern_binders`), the `MacroItem` arm (:999); `builtin_head` in A2.
- `crates/ilang-cli/src/commands/fmt.rs`: `--from-old`, `--keep-layout`, `--report`.
- `crates/ilang-cli/tests/check.rs`: the feature guard.
- `crates/ilang-check/tests/*`: the new checker tests and the `ONE_CLAUSE_REPORT` helper.
- In A2: `crates/ilang-check/src/types/display.rs` (Namer), the messages in `types/pattern.rs:734` and `resolve.rs:2600`, `crates/ilang-cli/tests/spec.rs` (`UPDATE=1`).

Not changed: the lexer, `infer.rs`, `lower.rs`, `coverage.rs`, `order.rs`, hir.

## 6. Tests

### 6.1 Syntax (stage 1)

- `tests/both.rs` (new), for claim D:
  - (i) `both_agrees_with_old`: for every corpus source, if Old mode has no diagnostic, Both mode gives the same `shape` and only E0113 diagnostics; the number of E0113 on the corpus is zero.
  - (ii) Seeded mutants of each corpus file (a break added or removed, `(` moved to a new line, `->` put in, `{` swapped with `(`), about 50 000 parses; the same assertion for each mutant that Old mode accepts. The A1-RF ambiguity hunter adds what it finds here.
  - (iii) Each new form parses only in Both mode.
  - (iv) E0113 cases: `{ (x) -> Int { x } }`, `r = { (x) -> Int { x + n }(1) }`, `{ (x) -> (y) { y } }`.
  - (v) The cache gives the same tree as a run with no cache.
- `tests/golden_old/` (new): the 48 `.out` files of base. `golden.rs` renders every golden input with `parse_module_old` and compares with them, which pins Old mode byte for byte (claim D (a)).
- Golden `.out` files that change at stage 1, checked against the real parser at stage 1; any other changed `.out` fails the stage:
  - `err_attribute_line`: errors become `E0115 @2:1 a group in ( ) with an attribute before it is not a binding, because no = follows it; the imports and the exports have no attribute before them` (finding 14); the tree is `main = () { Nil }`.
  - `err_block_end`: the third error goes; line 3 prints `h = {}`.
  - `err_toplevel`: no parse errors; the tree prints `f(x)`, `Some(x) = y`, and `@external("m", "f")` with `g = 1` (the checker reports E0234 and E0303).
  - `err_trailing`: `E0101 @1:3 expected a type, found ,` (was "a name"); the error at 4:7 goes, and `@main()` with `x = 1` prints (the checker reports E0302).
- New golden inputs (new syntax): `header_groups.i` (a mix of keywords and groups, a header followed by `(a, b) = …`), `clauses.i` (every block and clause block of §6.1, `{}`, `?` guards, a function type before `?`, the first-clause group), `type_params.i` (`id(A) = …`, `id(A): (A) -> A = …`, `inspect(A): …`, `Option(A) { … }`, `send(Stop)` as an expression), `attr_next_line.i` (`@m`, then `(even, odd) = …` on the next line), `kept_forms.i` (`p.name`, `m(Int)` in a type, `f(x)` at the top level, `Schema(User, _) = user`).
- New error goldens: `err_attr_same_line.i` (`@m (even, odd) = …`), `err_record.i`, `err_dual_ambiguous.i`, `err_header_empty.i`, `err_header_order.i`, `err_pipe_group.i` (`n |> { 0 -> 1  _ -> 2 } * 10`).
- `tests/parse.rs` (new): `parse_module_old` of `r = { (x) -> Int { x + n }(1) }` gives a block, and `parse_module` gives E0113 and the same tree (finding 10); 40 nested `{` and 200 nested `{` give `decisions == braces`; every example of the opening of the spec and of §6.1 decides as the text says.
- `spec.rs`: every block parses in Both mode with no diagnostic and round-trips through `print.rs`.

### 6.2 Checker (stage 2), `crates/ilang-check/tests/`

- `types/type_params.i`: inner `g(A)` hides outer `A`; `pair(A, A)` (E0226); a variable named `Int` (E0231); `A(Int)` (E0230); an unknown `UpName` with the hint; a declaration used at two types; a declared binding generalized and used at two types; `Box(Int) { B(Int) }` (E0231); `Box(a, B)` (E0232); `f(A) = (x: A, y: b) { x }` (E0236); an outer old binding with `a` whose inner binding declares `B` and uses `a` (correct).
- `types/clauses.i`: a one-clause match is fitted, `x: T` is an annotation, no coverage error; `{ n -> n }` has the label `n`; a multi-clause lone block has none; `loop = { 0 -> Done  n -> loop(n - 1) }` at the top level and locally; `g = () { (x) { f(x) * 2 } |> { f -> f(1) } }` calls the top-level `f`; one clause with choices (E0235); `x |> {}` with `x: Never`.
- `types/kept.i`: E0229, E0233, E0234, E0301, E0302, E0303, E0308 messages.
- `hir.rs`: the shapes of the desugaring, with the made-up `Bind` span equal to `p.span`.
- `query.rs`: completions inside the subject `e` of a one-clause match offer local names, including inside a function in `e`; the names of `p` are offered in the body; `%arg` is never offered or shown.
- `ilang-wasm/tests/codegen.rs`: top-level `f = { 0 -> 1  _ -> 2 }` is a known function; `get = (o) { o |> { Some(x) -> x } }` called with `None` stops at the position of `Some(x)`.
- A test that `migrate::UNQUALIFIED_TYPES` equals the built-in and prelude types, and one that, for every corpus file, the R4 owners equal the frames resolve builds.

### 6.3 Formatter (stage 3), `crates/ilang-syntax/tests/format.rs`

`check(src)` asserts:
1. `out = format(src)` succeeds;
2. `parse_module(out)` has no diagnostic;
3. `canon(parse_module(out)) == canon(apply(&plan, read(src)))`, where `read` is Both mode with the Old fallback: the same tree after type-variable migration;
4. the comments are the same;
5. `format(out) == out`, and the plan of `out` is empty;
6. the existing checks on tabs and spaces.

`format_properties` asserts that the list of files that fail to parse equals `tests/format_skip.txt` exactly, so no file drops out silently. The format `.out` files are made again with `UPDATE=1` at stage 3. `tests/migrate.rs` checks splice on every tracked `.i`, every ```` ```i ```` block, and one input per rule (R1-R9, an `A1` fallback, a split, CRLF, a bare CR, empty headers).

### 6.4 Blind spec tests (stage 4)

- Writers see only `docs/spec.md` at the stage-1 commit and the `tests/spec` header format, never code. They write `tests/spec/<section>/*.i` in the new syntax, one or more per item: the three exceptions of the opening; §1.7 (the `(`, `-`, `-.` rule); §2.2 header groups, attributes, the `@m` example; §2.3, §2.4; §2.5 and §4.7 declarations and hiding; §3 kept forms as compile errors; §4.3 head errors; §4.8 fitting, `Assert`, the expected type from `x: T`; §6.1 tests 1-6 with the given examples; §6.9 groups; §6.12 (stop at `p` with `// at:`, labels, `{}`, `loop`); §7.1 (`?` after a function type); §7.4; §7.5 (no known constructor from one clause); §11.6 (a deep match in tail position).
- They do not use the four words as names, the lowercase-type error, a lowercase or mixed type head, a first clause of D4's form, or one clause with choices (until the user decides D7).
- A writer who does not know an error code writes `compile-error ?`; the judge fills it in after checking the message.
- The judge sorts each mismatch: implementation bug, test bug, or spec gap (to the coordinator, then the user). A mismatch on a temporary difference (D4, D7, §3.1) is expected and recorded.

## 7. Stages and gates

1. **S1**: `scripts/wat-diff`; `ast.rs`, both parser modes, the decision cache, the codes; `print.rs` new nodes; the spec text; `both.rs`, `parse.rs`, `golden_old/`, the new goldens; resolve's temporary arms (§3.3). Gate: E1; the golden rule of §6.1; `both_agrees_with_old`; `i test` and `wasi-smoke`.
2. **S2** (after the user's D7 decision): resolve §3.9-§3.11, `query.rs` §3.12, the feature, checker tests. Gate: E1; checker `.out` files unchanged except new tests and the listed message changes (E0301, E0308).
3. **S3**: `migrate.rs`, `format()`, `format_from_old`, splice, `i fmt` flags, the property, format `.out` with `UPDATE=1`, `allowed-a1.txt`. Gate: E1, E2, E3; `i test` (std, examples/hello, bench, O0 and O2) and `wasi-smoke` on the formatted copy. From here, only `i fmt --check lib std bench examples` in dev-verify may fail.
4. **S4**: blind cases merged and judged; E1, E2, E3 again; the corrections to the `syntax.md` notes (§10).

The checks, all against the export of base:
- **E1** (plan check 1): WAT of unformatted sources, normal build, byte-equal with `cmp`. Files only on the A1 side come only from blind cases.
- **E2**: in the rev lane, `i fmt --from-old` on every `.i` outside the A2 layout list; build with the feature (§3.16); `wat-diff --allowed` with every category except `one-clause`.
- **E3** (plan check 2): the same formatted copy, normal build (feature off), `wat-diff --allowed` with every category. Then the formatting is undone.

`i test` and `wasi-smoke` stay green at every stage, because the sources stay old and old heads and lowercase variables are still read (finding 9).

`i new` and `format()`: from stage 3, `i new` (`package.rs:147`) writes the new syntax from the old templates. Checked at base: `package_commands.rs:207-221` asserts only `contains("(Io) = i/io")` and `contains("@main")`, which still hold; `:721` runs `i fmt --check` on the output, which passes because `format()` is idempotent; `ilang-lsp/tests/protocol.rs:334` compares with `format()` itself; `ilang-cli/tests/fmt.rs` uses text with no old forms. So no test beyond the one dev-verify check fails, and the templates stay for A2 partition 3.

## 8. A2 mechanics

- **Phase 0** (compiles):
  - `UPDATE=1` in `crates/ilang-cli/tests/spec.rs` rewrites only the `// at:` lines and the first line of `expect: stop "…"`; it keeps each file's line end (CRLF in `binding_text_crlf.i`), never touches lines after the header (the bare CR in `binding_text_cr.i` stays), and fails when a value has spaces at its start or end.
  - The Namer in `display.rs`: type variables `A`, `B`, …, error variables `E`, `E2`, …, environment variables `N`, `N2`, …, skipping names that are types in the module.
  - In the same commit, by hand: Rust test strings with shown types (finding 6): `check.rs` (`app/main.id : (x: a) -> a`), `ilang-check/tests/query.rs`, `ilang-lsp/tests/protocol.rs` hover, others found by a search for `-> a`, `(a)`, `? e ! n` and by running the tests; `builtin_head`; the checker `.out` files made again.
- **Partition 1**: `i fmt --from-old --report` on every `.i` outside the layout list.
- **Partition 2**: `i fmt --keep-layout --report` on the 14 `format/*.i`, the 48 `golden/*.i`, `binding_text_cr.i`, `binding_text_crlf.i`, and the parse-error samples of `tests/spec`; then the hand edits of the `syntax.md` list (err_module_path, err_case_brace, err_clause_tuple_type, err_pattern, err_segment_type, err_block_end line 3, err_toplevel lines 1, 3, 4, err_trailing lines 1 and 4, err_import_late, imports_empty) (finding 12). Each `format/*.i` still differs from its `.out`; the CR count does not change.
- **Partitions 3-5**: plain Rust string literals are extracted, unescaped, migrated with `--keep-layout`, escaped again, and put back. `format!` templates (`package.rs` `i new`, `codegen.rs` `many_points`, `wasi.rs`) are changed by hand, with a test that renders each template and parses it. Needles and hard-coded positions change with their source.
- **Partition 6**: each ```` ```i ```` block of the `*.md` files with `--keep-layout`; prose, doc comments, line comments, and the §9.6 `// type:` line (finding 5) by hand from searches; the compiler messages of `pattern.rs:734` and `resolve.rs:2600` by a search of message strings for the four words (finding 15).
- **AG**: merges the partitions; adds the check that each `format/*.i` differs from its `.out`; makes `.out`, `// at:`, and checker files again; records partitions 1 and 2 in `$C/a2/commands.sh`; the position view maps each old `// at:` position through the splice offset map and compares with the `UPDATE=1` result; full dev-verify green.

## 9. A3

Remove: the four keyword tokens, the `if` exception in `can_begin`, `no_brace`; Old mode, `parse_module_old`, `format_from_old`, `--from-old`, `--keep-layout`, `migrate.rs`, `golden_old/`, `both.rs`; the `Syntax::Old` paths in the parser, `print.rs`, and `format.rs`, and then `Case.syntax` and the header styles; E0113; E0236 with old lowercase variables and heads, `push_ty_frame`, `annotation_vars`, `type_vars`, `pattern_annotations`, `expr_annotations`; the feature `one-clause-as-case`; the keyword completions at `query.rs:971`; `is_name` in `ilang-package/src/lib.rs:180` and "or a keyword" in `package.rs`; the tmLanguage keywords, `sample.i` and its snapshot, `grammar.test.js`. Change `tests/lex.rs:19-24` (finding 16), `package_modules/src/if.i`, `manifest.rs`, `package_commands.rs`. Add the error for a lowercase name in a type, with a new code, in resolve's type-name lookup.

## 10. Corrections to the `syntax.md` notes (committed at S4)

- A match is `Case { syntax: New }`, built by the parser (D5).
- The made-up `Bind` has the span of `p`; `lower.rs` does not change; `query.rs` `Scope::block` uses `cover()` (D6).
- One clause with choices: the user's D7 decision.
- Known constructors: R7a, not renaming uses to `c` (D13).
- `print.rs` adds no expression groups (D15); the format mode is D10.
- Kept forms: records are a parse error, the rest checker errors (D9).
- `err_trailing` line 1 expects "a type", not "an `UpName`"; `@main()` moves to the checker.
- The A3 list gains: E0113; E0236; the old `if` line rule and the old `case` `{` rule (already there); `sample.i` moves to A3.
- The list of edits outside A1's ranges is §5 of this note; `lower.rs` and `infer.rs` leave it.

## 11. Findings of `a0-round8`, one by one

1. The two old §1.7 rules: §3.1, D1, the A3 list.
2. The opening's exceptions: in the text as written; the blind matrix tests them.
3. Two errors for one head: §3.11, in a fixed order, parser for the form and resolve for the name of a type.
4. `p.x` with a value `p`: §3.7, `ValueOf` whatever `p` is.
5. §9.6 `// type:`: A2 partition 6.
6. Rust tests with shown types: A2 phase 0.
7. Stop position: D6 (span `p.span`, no `lower.rs` change).
8. Declaration frames: §3.10, `values()`.
9. Old heads: §3.7, kept until A3.
10. Old blocks with a function: D2, D4, D10.
11. Edits outside the ranges: §5.
12. Layout-sensitive files: §3.15, A2 partition 2.
13. `print.rs`: D15, §3.13.
14. `err_attribute_line`: §6.1, two new goldens.
15. Compiler messages: A2 partition 6.
16. `lex.rs`: A3.
17. Edit wording in a bullet: already fixed at base.
18. What goes into the spec: §4.

## 12. Risks

- E0113 and E0235 are temporary differences from the accepted text. If the user refuses E0113, the only other safe choice is an Old-mode-only migration with no Both-mode reading of such blocks, which leaves the silent ambiguity of `{ (x) -> (y) { y } }` until A3.
- Claim D is tested, not proved. A missed overlap shows only if the corpus, the mutants, or the ambiguity hunter reach it. A miss would let `format()` print a changed tree only on the Old fallback path, where the write check catches it.
- The `{` decision rests on speculative parses of patterns, which can hold expressions with `{`. A rollback bug would be fixed in place by the cache. `both.rs` (v) and the nested-brace test cover this.
- Allowed mode skips the bodies of listed functions. E2 narrows this to the categories other than `one-clause`, but a regression inside a function with a `g-binding` or R7a entry stays hidden in both E2 and E3.
- The R4 owner rule copies resolve's rule in the syntax crate. The cross-crate test covers only the corpus.
- R7a is chosen from syntax alone. A one-clause case whose meaning depended on narrowing by a type test may still change; the checker `.out` files and `i test` must show it.
- The site-count argument assumes a covering old case gives a pattern that infer does not mark may-fail. The machine report checks it; if it fails, `--renumber` is needed, which depends on the site forms of `emit` at base.
- From stage 3, `format()` on old files depends on the plan. The LSP formats old files into the new syntax on the A1 branch. A1 is not merged alone, so no user sees it.
- Error codes E0113-E0115 and E0229-E0236 may clash with codes that T1, T2, B, or C choose. The coordinator reserves them.
- The feature could leak through feature unification in a workspace build; the guard test catches it in `cargo test`, and the release assertion in release builds.

## 13. Review items of this design, and where they are settled

- E0113 rules, corpus, A3 list, approval, message, blind writers: D4, §6.4, §10, §14.
- Allowed-list categories for g-bindings and groups: §3.17.
- Site count: §3.17.
- The feature: D14, §3.16, E2 and E3 in §7.
- E0235 and the choice for the user, with the corpus files: D7, §14.
- Known constructors: D13, R7a.
- `i new`, `package_commands.rs`, the LSP: §7.
- Edits outside the ranges: §5.
- `query.rs` Bind spans: §3.12.
- Exhaustive matches: §3.3.
- Frame order and mixing: §3.10, E0236, its test in §6.2.
- The stage-1 goldens and their messages: §6.1.
- Plain `i fmt` on an E0113 file: D10.
- Kept forms split and wording: D9, §3.11.

## 14. Open questions

For the user:
1. Approve E0113 DualAmbiguous (D4) as a temporary rule, until A3, that `syntax.md` does not have.
2. Choose for one clause with choices (D7): E0235 until T1 with the migration splitting such clauses (default), or the old `case` node with E0516 and a known constructor until T1. Needed before stage 2.

For the coordinator:
3. Add the edits of §5 "outside A1's ranges" to the split of files, and tell T1 and T2 to rebase on them.
4. Accept the test-only feature `one-clause-as-case` in `ilang-check` (D14).
5. Accept the allowed-list categories beyond the plan's one (`one-clause-known`, `choices-split`, `g-binding`, `split`, `tyvar-moved`, `hand`).
6. Reserve the error codes E0113-E0115 and E0229-E0236.
7. Confirm that the LSP formatting old files into the new syntax on the unmerged A1 branch is acceptable.
