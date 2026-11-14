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

- **D3. Claim D: Both mode is Old mode plus error-only extensions, with two named exceptions.** For every source S:
  - (a) `parse_module_old(S)` gives the tree and diagnostics that `parse_module(S)` gave at base, with the new fields empty.
  - (b) If `parse_module_old(S)` has no diagnostic, then either `parse_module(S)` gives the same tree with no diagnostic or only E0113 diagnostics (D4), or S has a joined type call (J1, below).
  - **J1, joined type call.** The old item loop needs no break between items, and the old `type_prim` ends a type at a lowercase name. So in Old mode a type that ends an item with a lowercase name can be followed on the same line by a `(` that starts the next item, a binding with a tuple pattern: `Id(a) = a  (p, q) = (1, 2)`, or `@external("m", "f") f : a  (p) = 1`. Both mode reads `a (` on one line as a `TypeCall` (§3.7). Such a tree always holds a `TypeCall` with a lowercase name, which is either a parse error (the next item then starts at `=`, E0101) or a checker error (E0234, which no program escapes). So J1 never changes the meaning of a program without a diagnostic. `format()` refuses such a file and names `i fmt --from-old` (D10), and `both.rs` asserts that every mismatch it finds is E0113 or J1 (§6.1).
  - The forms that start with `name (` (§3.4 rule 4, §3.5 test 1, and the same check in `stmt()` for every statement of a block, §3.5) are not an exception: they need the `(` on the same line as the name, and Old mode then reads the pattern `name`, which is not followed by `=`, so the old statement or item is an error.

  Every new branch in `parser.rs` is guarded by the mode and sits where the old code returned an error, except the `TypeCall` of J1; each carries a comment `// both: old gave <Code> here` (or `// both: J1`). Reason: this is what makes the migration and the first equivalence check safe. It is checked by tests (§6.1), not proved.

- **D4. E0113 DualAmbiguous: a temporary parse error, removed in A3 (coordinator; needs user approval because `syntax.md` does not have it).** In Both mode, a `{` that begins an expression is reported as E0113 when its first statement starts with `(`, the whole `( … )` reads as a pattern with `pattern_in` (as spec test 2 reads it), and the old function test of `paren()` holds there: the `)` is followed by `->`, the `->` part (`arrow_tail`) reads as a type, and a `{` follows on the same line. The `{` is then read as an old block, so the tree is the old tree.
  - Test 1a is not made for a `{` that is the right side of `|>` (the token before it is `|>`) when its matching `}` is followed by nothing that `postfix()` goes on with: not by a `(` on the same line (`at_joined`), and not by `.`. Then the right side is a bare block, which `binary()` rejects with E0111 at base (`parser.rs:646-652` accepts only a name, a `Call`, or a `Fn`, and `is_name` is false for a block), so Old mode never accepts it and claim D does not need the test there. When the `}` is followed by `(` or `.`, Old mode can read a postfix chain that ends in a call, such as `{ … }(1)`, `{ … }.0(1)`, or `{ … }.g(1)`: the right side is then a `Call` whose callee is the block, or a `Field` or `TupleIndex` of it, and `binary()` accepts it. So test 1a is made there. For example, in `pair = ((x, y) { x + y }, 0)` and `main = () { 5 |> { (a) -> Int { a }  pair }.0(1) }`, Old mode reads a block, and Both mode gives E0113 and reads the same block; it never reads a clause block there. So `hs |> { (name, _) -> (x) { log(name, x) }  _ -> (x) { x } }`, an example of §6.1, is a match in Both mode, as the spec says.
  - Message: "until the old syntax is removed, a `{` whose first statement starts with `( … ) -> T {` is not allowed: the old syntax reads it as a block that starts with a function, and the new syntax as a clause block. For a function, bind it to a name first, and write its type on the name when the place gave it one: `g : (Int) -> T = (x) -> T { … }`. For a clause whose expression starts with a function, put that expression in a group: `(x) -> { (y) { y } }`; when that function has a `->` part, also bind it to a name with its type inside the group: `(x) -> { g : (Int) -> Int = (y) -> Int { y }  g }`." The type is written on the name because a binding with no written type gives its value no expected type (§4.8), so a parameter with no written type would lose the known type that the clause or the block gave it.
  - Programs it rejects that the old syntax accepts: exactly the old blocks of that form. The new syntax rejects them too (they read as a clause block that then fails), except where the clause reading also succeeds, so E0113 changes only the message for them, and never lets their meaning change silently.
  - Programs it rejects that `syntax.md` accepts: lone clause blocks (and clause blocks after `|>` that are followed by `(` or `.`, as in `e |> { … }(a)`; with `.` the new syntax gives E0111 or a type error anyway, because a function has no fields) whose first clause pattern is a group or tuple pattern `( … )` and whose clause expression, read as a type, is followed by `{` on the same line. In practice this is a clause expression that starts with a function, as in `{ (x) -> (y) { x + y } }` or `{ (x) -> () { log(x) } }`. The rewrite in the message gives a form that both modes accept. The migration never writes such a clause block (R2 makes every old `case` a match after `|>` whose `}` is followed by neither `(` nor `.`, and old code has no lone clause block), so R6 does not need this case.
  - Programs it does not touch: a block whose first statement is a function with a parameter that is not a pattern, such as the §6.1 example `{ (n, log ! Log) -> Int { log.write(n)  n } }`. Its `( … )` does not read as a pattern, so test 1a does not hold, and both syntaxes read a block with the same meaning. R5 skips such functions for the same reason.
  - The corpus has none: at base, a search for a `{` that begins an expression and is followed by `( … ) ->` finds only function bodies (such as `known_types.i:55`, which is a body and is never decided) and type groups. The stage-1 test `both_agrees_with_old` (§6.1) asserts zero E0113 on the corpus, and from stage 3 the write check (D12) asserts zero E0113 on every migrated file, so migrated first clauses are checked too.
  - Neither `i fmt` nor the LSP settles it: `format()` has no Old fallback (D10), so a file with E0113 is not formatted, and the error names `i fmt --from-old` for a file in the old syntax.
  - It is not in `spec.md`. It is listed in the `syntax.md` A3 notes (§10). Blind spec writers are told not to write, in a lone clause block, a first clause with a `( … )` pattern whose expression starts with a function, and the judge treats such a mismatch as expected. A function with a non-pattern parameter at the start of a block, and a clause block after `|>` that is not followed by `(` or `.`, are not in this rule, so the writers test the §6.1 examples as written.

- **D5. A match reuses the `case` node.** The parser builds `ExprKind : : Case { subject, clauses, syntax : Syntax : : New }` for `e |> { clauses }`, and `Syntax::Old` for `case e { … }`. The only new expression node is `ExprKind::Clauses` for a clause block that is not the right side of `|>`. Reason: every existing `Case` arm (resolve, format, print, `expr_annotations`) keeps working; the `syntax` field tells the printers which words to use, and tells resolve which typing rule applies (a new one-clause match follows binding rules; an old one-clause `case` follows rule 11). This corrects the note that says the parser keeps `|>` with a `Clauses` right side (§10).

- **D6. Resolve lowers the new forms onto existing hir; `infer.rs` and `lower.rs` do not change.** A one-clause, no-guard, no-choice match becomes a hir `Block` of a non-direct `Bind` and the body; the made-up `Bind` has the span of its pattern `p`. Reason: `lower.rs` takes the stop position from `b.span.start` (`lower.rs:1622`, `:1632`), so with span `p.span` it is the start of `p`, as §6.12 says, with no change to `lower.rs`. The cost is in `query.rs`: `Scope::block` uses `cover(b.span, b.value.span)` so that the subject `e` is still found (§3.12). This corrects the note that asks for a `lower.rs` change (finding 7, §10).

- **D7. One clause with choices: E0235, an error until T1 (default), or the old `case` node (alternative). The user decides before stage 2.** Both go against §6.12 until T1. T1 is the stream that implements fitting choices, so the coordinator adds "one clause with choices as a binding; remove E0235 (or the alternative) and the `choices-split` note" to T1's stages (§14, item 8). A1 does not leave it unowned: at stage 4 it adds the pending spec case of §6.4, which T1's gate must turn green. Their costs to programmers:
  - E0235 (default): `{ Ok(v) | Error(v) -> v }` is a compile error, "a clause block with one clause and choices is not supported yet; write one clause for each choice". The migration writes one clause per choice, which duplicates the clause expression. It never gives a meaning that the spec does not have. In the corpus it changes 11 clauses in 5 checker test files and no library code: `crates/ilang-check/tests/projects/ok_multi/src/main.i:37`, `crates/ilang-check/tests/types/case.i:66`, `choices.i:5`, `known.i:49` and `:68`, `known_choices.i:7-14` (six clauses). The parser-only golden `crates/ilang-syntax/tests/golden/spec_inline.i:11` is not split: E0235 is a checker error and goldens are only parsed and printed, so partition 2 does not apply R7b to the layout list (§8), and the golden view sees no extra clause.
  - Old `case` node (alternative, from the migration-first proposal): no duplicated code and no WAT difference, but choices that do not cover give E0516 instead of reporting to `Assert`, and the subject name gets a known constructor (§7.5) that the spec does not give.

- **D8. Declared type variables live in a separate kind of frame that always makes new variables.** `push_declared_frame` hides outer variables of the same name; `push_ty_frame` (old lowercase sharing) does not change. A binding or declaration that declares `UpName` variables may not use a new lowercase variable: that is E0236 TYPE_VAR_MIXED, never a silent variable (§3.10). Reason: §4.7, findings 8 and 9, and the silent fallback of `infer.conv` for a variable that no binding owns.

- **D9. Kept forms: one family of messages, split by what the parser can build.** Records (`( name : e )`) are a parse error, E0114 RecordReserved, because their grammar is not given and no tree can be built. Every other kept form is read in full into a tree node and reported by the checker: values of types (E0229), macros in types, macro items, macro attributes and declarations with no `@external` (E0234, E0301, E0308), and alias patterns (E0233). Reason: every ```` ```i ```` block of `spec.md` must parse with no diagnostic (`tests/spec.rs`), and the §2.2 example has `@m`. Every message says "is kept for …" in the same words (§3.8, §3.11).

- **D10. `format()` writes the new syntax from stage 3, as the plan says, and reads only Both mode.** In stages 1 and 2 it prints each node in the syntax it was written in (§3.14). From stage 3 it migrates. It has no Old fallback: by claim D (b), when Old mode accepts a file, Both mode gives the same tree, or E0113, or a J1 tree, so a fallback would only ever run on E0113 or J1, and it would settle that ambiguity silently in favour of the old reading. So when Both mode reports errors, `format()` returns them. When Both mode reports no error but its tree holds a `TypeCall` whose name is lowercase and has no module (J1 is only possible then), `format()` also reads Old mode, and if Old mode has no diagnostic and a different tree it refuses the file with "this file reads differently in the old syntax; run `i fmt --from-old`". Every parse error that `format()` returns, and every parse error of `i check`, gets the hint "if this file is in the old syntax, run `i fmt --from-old`" when `parse_module_old` reads the same source with no diagnostic (one helper, `ilang_syntax::old_syntax_hint`, run only on the error path). `i fmt --from-old` (`format_from_old`) reads Old mode and runs the write check (D12). Reason: the plan's stage 3, finding 10, and D4 (an E0113 or J1 program never changes meaning without a diagnostic).

- **D11. One migration plan, three users.** `migrate::plan(&Module, &Env) -> Plan` is a pure function on the old tree. The formatter prints from it, the splice printer (`--keep-layout`) edits bytes from it, and `migrate::apply(&Plan, Module) -> Module` builds the tree the output must parse to. The formatter applies the plan at the Doc level and makes no AST nodes, so comments stay in source order and `closer()` only sees real spans.

- **D12. Every migrated file is checked when it is written.** On the `--from-old` path and the splice tool: `canon(parse_module(out)) == canon(apply(plan, parse_module_old(src)))`, or nothing is written and the first differing node is reported. Plain `format()` of a file that parses in Both mode does not run this check, so the LSP does not slow down.

- **D13. Known constructors after a one-clause match: rule R7a, mechanical detection, fix by hand.** A one-clause, no-guard, no-choice old `case` on a name `s` gives `s` a known constructor (§7.5) only when its pattern `p`, with its `name =` chain and its type annotation taken off, is a constructor pattern `C(…)` or `C`. When that holds, `p` does not bind `s`, and the clause expression uses `s`, the migration refuses the file (category `hand`, reason `one-clause-known`). It does not rewrite `p` as `s = p`: that rebinds `s` as a pattern name, which is never generalized (§3.9 (a)), so a generalized `s` used at two types would stop checking. A catch-all or literal `p` gives no known constructor, so it is a plain `one-clause` entry. Reason: the argument that no such case needs the known constructor is not a proof, and the rewrite that keeps it is not always correct. A heuristic scan of the corpus finds no candidates today, so the hand list costs nothing now, and A2 never depends on finding such cases by eye.

- **D14. A test-only cargo feature `one-clause-as-case` on `ilang-check` (coordinator).** It lowers a one-clause match as the old `case` node. It is used only by check E2 (§3.16), never in a normal, release, or dev-verify build, and A3 removes it. Reason: it lets the formatted corpus be compared with base strictly, so the allowed list of the plan's check 2 (E3) cannot hide a regression inside a listed function behind a one-clause difference.

- **D15. `print.rs` prints each node in the syntax it was written in, and adds the groups that the rules of binding need where the tree has none, as the `syntax.md` note and `meta.md` §1 ask.** Reason: finding 13, and `meta.md` ("the printer of the normal surface adds the groups that its rules of binding need"): a core tree may come from the Lisp surface or a macro, which never has groups, and a match is a `Case` node (D5), so a tree such as `(* (|> n { … }) 10)` has a `Case` as the left operand of `*` with no `Block` around it. `print.rs` checks the tree, not the source: it adds a group only where the node in that place is not already a group. Each rule is limited to exactly the place where the parser would read the text without the group in another way, and only new nodes (`Case { syntax : New }`, `Clauses`) get the clause rules, so a tree from the parser with no diagnostic never meets a rule: no group is added to it, the strict `shape` round trip of `golden.rs` and `spec.rs` holds, and no golden `.out` of an old input changes because of `print.rs`. A type group leaves no node, so `print.rs` also puts `{ }` around a function type that ends a clause pattern when `?` or another `|` choice follows. Some core trees have no text in the normal surface that is the same code (a block whose first statement starts with a function with a `->` part, and two clause forms). `print.rs` never invents a binding for them: `print::unprintable` finds them, and the callers that need the same code report an error, as `meta.md` §1 already does for a quote. This is a correction to `meta.md` §1 (§10). The rules are in §3.13.

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
- `Parser` gets `mode : Mode { Old, Both }`, `decided : HashMap<usize, Brace>` and `decisions: usize`.

### 3.3 Syntax tree changes (`crates/ilang-syntax/src/ast.rs`)

- `pub enum Syntax { Old, New }`.
- `ExprKind : : Case { subject, clauses, syntax : Syntax }`. The span of a new match is the whole `e |> { … }`.
- `ExprKind::Clauses(Vec<Clause>)`, span from `{` to `}`. `{}` is `Clauses(vec![])`.
- `Binding { pattern, value, span, type_params: Vec<Ident> }` (top-level bindings and `Stmt::Bind`).
- `ItemKind : : Declaration { name, type_params : Vec<Ident>, ty }`.
- `ItemKind : : MacroItem { module : Option<Ident>, name : Ident, args : Vec<Arg> }`.
- `Attribute { module: Option<Ident>, name, args: Vec<(String, Span)>, call: Option<(Vec<Arg>, Span)>, span }`. `args` is filled as today when every argument is a string literal with no label and there is at least one. `call` is kept in every other case with `( )`, including `@custom()`.
- `TypeDef { name, params: Vec<Ident>, head: Vec<TypeExpr>, body }`. `params` holds a plain head: all lowercase names (old, until A3) or all `UpName`s with no module and no arguments (new). `head` is filled, and `params` is empty, when the head is anything else.
- `TypeKind : : ValueOf { module : Option<Ident>, name : Ident }` for a lowercase `a.x`.
- `TypeKind : : Call { module : Option<Ident>, name : Ident, args : Vec<TypeArg> }` and `enum TypeArg { Type(TypeExpr), Int(..), String(..) }`.
- `Module` gets `imports_span`, `exports_span: Option<Span>` and `imports_style`, `exports_style : HeaderStyle { Keyword, Group }`. Each span covers the keyword, if any, and the group.

Code that must change when these fields and variants are added (stage 1, so that stage 1 builds):
- `Case` patterns without `..`: `resolve.rs:289`, `:2007`; `format.rs:1104`; `print.rs:326`; `parser.rs:953`.
- `Declaration` patterns without `..`: `parser.rs:413`, `:431`; `resolve.rs:503`; `format.rs:914`; `print.rs:196`.
- `Binding` literals: `parser.rs:438`, `:584`.
- `TypeDef` literals (the new `head` field): `parser.rs:483`, `:523`.
- `ast::ExprKind` matches that must get a `Clauses` arm: `resolve.rs` `expr_annotations` (:258), `expr()` (:~1990-2010), `print.rs` (:35, :~326), `format.rs` (:~1104 and the expression printer).
- `ast::TypeKind` matches that must get `ValueOf` and `Call` arms: `resolve.rs` `type_vars` (:207), `fn ty` (:1330), `format.rs` (:~1510), `print.rs` (:~596), and the type printers in `parser.rs` if any.
- `ast::ItemKind` matches that must get a `MacroItem` arm: `resolve.rs` `collect()` (:457, `Slot::None`), `query.rs:999` (`symbols`, no symbol), `format.rs:899`, `:909`, `print.rs:194`.
- `Attribute` literal: `parser.rs:466` (in `attribute()`, which starts at `:452`).
- In `ilang-check`, for §3.10 at stage 2: the `Resolver` field `ty_frames` (`resolve.rs:144`), whose type becomes `Vec<TyFrame>`, and every use of it: `:40` (`Vec::new()`, unchanged), `:1399`, `:1403` (`ty_var`), `:1693` (`values()`), `:1746-1753` (`push_ty_frame`), `:1779` (`binding()`).

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
2. A group in `( )` whose `)` is not followed by `=`: E0115. With attributes before it: "a group in `( )` with an attribute before it is not a binding, because no `=` follows it; the imports and the exports have no attribute before them". Without: "a group in `( )` that is not followed by `=` is the imports or the exports, which come first in a file". Before it returns the error, rule 2 moves `pos` past the matching `)` (from the `close` table), so `recover()` resumes after the group and does not stop again at its `(`; the group gives one error, not one with each message (`dedup()` merges only equal messages). An old `import` or `export` keyword here keeps E0103 and its old message until A3.
3. `UpName`: a type definition (§3.7).
4. `name ( UpName {, UpName} [,] )`, with the `(` on the same line as the name (`at_joined`), followed by `=` or `:` (a scan with the `close` table): a binding or a declaration with type parameters. The name becomes `PatternKind::Var`, or `Typed` when `:` follows; `=` after it makes a binding, no `=` a declaration.
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
1. Test 1: `name ( UpName {, UpName} [,] )`, with the `(` on the same line as the name, then `=` or `:`: Block. (`stmt()` then reads that statement as below.)
1a. D4: the statement starts with `(`, the `( … )` reads as a pattern with `pattern_in`, the `)` is followed by `->`, `arrow_tail` reads, and a `{` follows on the same line: AmbiguousBlock. `primary()` reports E0113 at the `(` and reads an old block. (`{ (n, log ! Log) -> Int { … } }` fails the pattern part, so it goes on to the later tests and is a Block by test 6.) Test 1a is skipped when the token before the `{` is `|>` and the matching `}` (from the `close` table) is followed by nothing that `postfix()` goes on with: not by a `(` on the same line (`at_joined`), and not by `.` (D4).
2. Read the longest pattern `p`, as `binding_head` does: read the chain `n1 = … = nk =` one pattern atom at a time, and then the last pattern with `pattern_in`. When the last pattern does not read, back off to the longest chain that does read, so `p` is `n1 = … = nk-1` and `nk` is followed by `=`. So in `{ go = (n, log ! Log) -> Int { … }  go(1) }`, `p` is `go`. When no part reads, there is no `p`, and test 2 does not hold. A type in `p` is read as far right as it goes, with no going back. If `|`, `?`, or `->` follows `p`, or the keyword token `if` does (until A3, so that the clause reader gives the guard-word message of §3.6; Old mode never reads a block whose statement is a pattern followed by `if`, so claim D holds): Clauses, except when `p` is a chain `n1 = … = nk = ( … )` and `->` follows, `arrow_tail` reads, and a `{` follows on the same line: then Block.
3. The statement starts with a pattern followed by `=` (spec test 3): the chain of test 2 has at least one part (`k ≥ 1`, so the first pattern atom is followed by `=`), or `p` is followed by `=`; or the statement starts with `!`: Block. So `{ k = 2  g = (x) -> Int { x * k }  g(1) }` is a block: `p` is `k = 2`, which test 2 does not decide, and `k` is followed by `=`. Since test 2 comes first, a clause `x = Some(y) -> e` is still a clause.
4. `name :` where the pattern `name : Type` is not followed by `=`, `->`, `?`, or `|`: Clauses, with E0114 RecordReserved at the `{`: "`( name : e )` is kept for records; a tuple type in a clause pattern is put in `{ }`". The rest is read as clauses, for recovery.
5. A `->` at depth 0 (brackets skipped with `close`) before the next break or the closing `}`, and the statement either fails to read as an expression or its longest expression is followed by `->`: Clauses. The error is the one found by reading the start as a pattern followed by `->`.
6. Otherwise: Block.

Statements of a block (Both mode, `stmt()`, for every statement, the first and every later one): before `binding_head()`, `stmt()` makes the check of test 1 at the start of the statement: `name ( UpName {, UpName} [,] )` with the `(` on the same line as the name (`at_joined`), and, after the `)` (found with the `close` table), `=` or `:`. When it holds, `stmt()` reads the name as `PatternKind::Var`, the `UpName`s as `type_params`, then `: Type` if present (the pattern becomes `Typed`), then `=` and the value, and builds `Stmt::Bind(Binding { type_params, … })`. A `:` form with no `=` after the type is E0101 "expected `=`", because a block has no declarations. When the check does not hold, `stmt()` goes on as at base. In Old mode `stmt()` makes no such check: `binding_head()` reads `wrap` and finds no `=`, the expression `wrap(A)` is then followed by `=`, and the statement is an error, so claim D holds. Example: in `{ k = 1  wrap(A) = (x : A) { Some(x) }  wrap(k) }`, the second statement is `Binding { pattern: wrap, type_params: [A], value: … }` in Both mode.

### 3.6 Clauses, guards, and pipes

- A clause is `Pattern { "|" Pattern } [ "?" Expr ] "->" Expr`. The guard is read with the `guard` flag set, so `->` ends it.
- An old `case` accepts only `if`; a clause block accepts only `?`. The other word is E0101 with the message "a guard in a clause block is written with `?`" or the old message.
- `binary()`: after the right side of `|>` is read, a right side that is exactly `ExprKind::Clauses` turns the `Binary` into `Case { subject : lhs, clauses, syntax : New }`, with the span of the whole expression.
- Any other right side keeps the E0111 check. When the right side is a `Binary`, `Field`, or `TupleIndex` whose leftmost operand is a `Clauses`, the E0111 message is "a match is an operation at level 6, so what comes after the clause block is part of the right side; put the match in a group: `{ e |> { … } } * 10`". For other right sides, Both mode gives the E0111 message of the new §6.9 sentence, "the right side of `|>` must be a name, a call, a function, or a clause block"; Old mode keeps the base text, so `golden_old/err_pipe_target.out` does not change and `golden/err_pipe_target.out` does (§6.1). A `Call` right side is accepted as today (`parser.rs:646-652`), even when its callee is a clause block or a `Field` or `TupleIndex` of one: `n |> { 0 -> 1  _ -> 2 }()` is `{ 0 -> 1  _ -> 2 }(n)` by §6.9. For such a right side the `{` was decided with test 1a (§3.5), so a first statement of D4's form gives E0113 and the old block, as in Old mode.
- The tuple-type note of §7.1: while a clause of a clause block (`|>` or lone) is read, the parser remembers whether its pattern holds a type annotation whose type is a tuple type `( … )` not in `{ }`. When that clause then gives any parse error (for example E0101 "expected `->`, found `}`" in `v |> { 0 -> 1  p : (Int, Int) -> e }`), the message gets "; a tuple type in a clause pattern is put in `{ }`". E0114 (§3.5 test 4) already says it. An old `case` keeps the base messages, so no golden `.out` of an old input changes; `err_clause_tuple_type` changes in A2 with its hand edit.

### 3.7 Types and type heads (Both mode)

- A lowercase `name` alone: `TypeKind::Var` (old type variable, until A3).
- `name . name` with a lowercase second name, whatever the first name is: `TypeKind::ValueOf`. This needs no scope, which settles finding 4.
- A lowercase `name (`, or `name . name (` with a lowercase second name, where the `(` is on the same line as the name (`at_joined`, as `type_def` and `attribute` check it at base): `TypeKind::Call`; each argument is a type, an int literal, or a string literal. This is the one place where Both mode reads a token sequence that Old mode accepts in another way (J1, D3); the checker always rejects the result (E0234). A `(` after a break is not part of the type: inside `( )` the break is a `,` (§1.7), so `f = (\n  x: a\n  (y, z)\n) { x }` keeps two parameters.
- `name . UpName`: `Named`, as today.
- A type definition head is read as `( Type {, Type} [,] )`. All plain lowercase names: `params` (old). All `UpName`s with no module and no arguments: `params` (new). Anything else, including a mix: `head`, and resolve reports it (§3.11). In Old mode the head is read with `Self::name`, as today.

### 3.8 New parse error codes (`crates/ilang-syntax/src/lib.rs`)

- E0113 DualAmbiguous (A3 removes it), message in D4.
- E0114 RecordReserved, message in §3.5 test 4.
- E0115 HeaderGroup, messages in §3.4.

New messages under existing codes: E0111 (§3.6), E0101 for the guard word (§3.6), the tuple-type note on any error in a clause of a clause block (§3.6).

### 3.9 Resolve: matches and clause blocks (`resolve.rs`, the new arm at about `:2007`)

`E : : Case { syntax : Old }` is resolved as today.

`E : : Case { syntax : New, subject : e, clauses }`:
- (a) One clause, no guard, one pattern `p`, body `b`:
  - with the feature `one-clause-as-case` on: as an old case (only for E2);
  - otherwise: resolve `e` in the current scope, with no new frame; resolve `p` (no type frame; `p`'s type variables come from the enclosing frames); push a normal `Frame` with `p`'s binders; resolve `b`; pop. The result is `hir Block { stmts : [Stmt : : Bind(Binding { pattern : p, value : e, direct : false, ty_vars : vec![], span : p.span }), Stmt : : Expr(b)], span : <the match> }`.
  - It does not call `binding()` or `is_direct`, so `e` is never in `p`'s scope and `p` is not generalized: in `g = () { (x) { f(x) * 2 } |> { f -> f(1) } }`, the `f` in the subject is the top-level `f`.
  - `infer.rs` gives `e` the expected type of `p` (`binding_expected`), so `x : T` at the top of `p` is an annotation, as §4.8 says. A `Block` gives no "not used" warning (`has_no_effect` is false), as §6.10 says of a match. The body is the last statement, so it is in tail position (§11.6).
- (b) One clause, no guard, more than one pattern: E0235 CHOICES_ONE_CLAUSE and hir `Error` (D7 default).
- (c) Otherwise: hir `Case { subject, clauses }`, exactly as an old `case`. So known types, narrowing, §7.5 known constructors, tail positions, and the order of evaluation are those of today's `case`.

`E::Clauses(cs)` (a clause block that is not the right side of `|>`):
- (a) One clause, no guard, one pattern `p`: build `ast : : Function { params : [Param : : Pattern(p)], ret : None, body : Block [Stmt : : Expr(b)] }` with the span of the block, and call the existing function code. The label comes from `label_of`.
- (b) One clause with choices: E0235.
- (c) Otherwise: build `ast : : Function { params : [Param : : Pattern(Var("%arg"))], ret : None, body : Block [Stmt : : Expr(Case { subject : Name("%arg"), clauses, syntax : Old })] }` and call the existing function code, as in (a). That code sets the label of each parameter with `label_of` (`resolve.rs:2147`), which gives `Some("%arg")` for a `Var`, and infer copies labels into the function type (`infer.rs:1943-1954`). So right after the call, the arm sets the label of the made-up hir `Param::Pattern` to `None`, as the `syntax.md` note asks: the type of `f = { 0 -> 1  _ -> 2 }` is `(A) -> Int`, with no label (§6.12, rule 11), in hover, in messages, in `.out` files, and in the interface printer, and a union with `(x) { x }` follows the §4.6 rule for one side with no label. The existing code also raises `fn_depth`, so a local `loop = { 0 -> Done  n -> loop(n - 1) }` finds its own name: `binding()` pushed `loop` with `fn_only : Some(fn_depth)`, and `find` (`resolve.rs:1837`) sees it only from a deeper function. The result is a hir `Fn` whose body is a hir `Case`, exactly as an old `case`.
  - `%arg` cannot be written in i, so no source name equals it (`__arg` would be a discard by §1.3).
  - The pattern span and the name span are zero-width at the `{`. The `Fn` span is the block span. The `Case` span runs from the start of the first clause to the end of the last; for `{}` it is zero-width just after the `{`. So no span-keyed table entry of the made-up nodes is shared with a real node.
- A top-level `f = { … }` is a hir `Fn`, so `lower.rs` still makes `f` a known function.

The general parameter in the example corrects the earlier `(Int) -> Int` text here and in §6.2. `syntax.md` §6.12 defines this clause block as a function of one fresh parameter whose body is the match, and spec §4.8 rule 11 gives `B <: P | r - P` when an unguarded catch-all is present. The `_` therefore accepts values outside `Int`, including a `String`, just as the old `(x) { case x { 0 -> 1  _ -> 2 } }` does. Until the A2 Namer change, this compiler prints the variable as `a`. A1 must keep that generality; requiring `Int` needs a clause block with no catch-all or the literal-type rules of T1.

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
- `fn ty`, the `T : : Named { module : None, name, args }` branch, before `lookup_type`:
  1. look up `name` in `ty_frames`, innermost first, then in `ty_def.params` (marking it used);
  2. found with arguments: E0230 TYPE_VAR_ARGS, "a type variable takes no arguments";
  3. found: `TypeKind::Var(id)`;
  4. not found: `lookup_type` as today. For an unknown lone `UpName` (no module, no arguments), the E0202 message adds a hint that depends on the place: inside a type definition, "to use a type parameter, declare it in the head, as in `Box(A) { … }`"; anywhere else, "to use a type variable, declare it after the name of the binding, as in `f(A) = …`". This changes existing checker `.out` files (at base: `projects/scope_rules/expected.out : 7`, `projects/type_rules/expected.out : 14`, and any others the S2 run finds); the S2 gate lists them (§7).

  Old programs have only lowercase keys in frames and in old type parameters, so an `UpName` is never found there, and old results do not change. E1 checks this.
- `T::ValueOf`: E0229 TYPE_OF_VALUE_RESERVED, "`p.name` in a type is kept for the type of a value". `T::Call`: E0234 MACRO_RESERVED, "a call in a type is kept for macros". Both give `TypeKind::Error`.

### 3.11 Resolve: kept forms, attributes, and heads

- `MacroItem`: E0234, "a call at the top level is kept for macros".
- A declaration with no `@external`: E0308, message now "a declaration with no `@external` is kept for macros".
- An attribute with a module, or whose name is not built in: E0301, message now "the attribute `@m` is kept for macros; the built-in attributes are `@external`, `@host`, and `@deprecated`".
- A built-in attribute with `call` present gives E0302: its arguments must be positional string literals. The removed test attribute has no built-in arity or placement rules; an unresolved use gives ordinary E0301.
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
- Names that start with `%` are left out of completions, hover, and definitions: binders in `Walk::binder` and `pattern_binders`, and uses in `Walk::expr`, which does not push a `Name` whose name starts with `%` into `occ`. As a second guard, `target_at` skips zero-width spans. So with the cursor at the `{` of `g = { 0 -> 1  _ -> 2 }`, or just after the `,` in `list.map(xs,{ 0 -> 1  _ -> 2 })`, hover and definitions find the real node there, never `%arg`.
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

Groups and lines (D15). `print.rs` adds each of these only where the tree does not already have it ("a group" in the tree is a one-statement `Block`), and only in the place named, which is exactly the place where the parser would read the text without it in another way:
- a type group `{ }` around a function type that ends a clause pattern when `?` or another `|` choice follows (a type group leaves no node, so this one is always checked);
- a group around a `Case { syntax : New }` that is an operand of an operator at level 7 or 8, of unary `-` or `-.`, a callee, or the target of `.f` or `.k`; and a group around its subject when the subject's top operator is at level 1 to 5 (§6.9). So in the output a match is never followed by `(` or `.`, and test 1a (§3.5) never applies to the clause block of a `Case { syntax : New }`;
- a line end before each clause or statement whose pattern starts with `(`, `-`, or `-.` (§1.7);
- a group around the first clause expression E of a clause block only when that clause has one pattern and no guard, and E starts with a function F with no `->` part (`( … ) {`), in two places only: (1) the block is a `Clauses` node (a lone clause block, also as a callee after `|>`; the parser turns a clause block right after `|>` into a `Case`, so every `Clauses` node is in the place of test 1a) and its first pattern starts with `(`; (2) the block is a `Clauses` or a `Case { syntax : New }` and its first pattern is a chain `n1 = … = ( … )` (the exception of test 2). Rule (1) is removed in A3 with E0113. In every other place a first clause expression that starts with a function gets nothing: `hs |> { (name, _) -> (x) { log(name, x) }  _ -> (x) { x } }` and `v |> { (a, b) -> (x) -> Int { x + a } }` print as they are. `Case { syntax : Old }` gets none of these clause rules, so `golden/case_head.i:8` does not change.

The one-pattern, no-guard condition follows §6.1 test 2: `|` or `?` after the first pattern prevents its arrow from being read as a function arrow. Such clauses must round-trip without an added group.

`print::unprintable(&Module) -> Option<Span>` finds the trees that have no text in the normal surface that is the same code, and gives the first one:
- a `{` that begins an expression (a `Block`, a group included) whose first statement starts with a function with a `->` part and only patterns as parameters (a group is itself such a `{`, so no group helps; and in the new syntax §6.1 reads it as a clause block, so this one stays after A3);
- the first clause expression of places (1) or (2) above when it starts with a function with a `->` part and only patterns as parameters (the group would start with that function, which is the case above). Place (1) leaves this list in A3. The parameter condition corrects the broader wording: an environment parameter does not read as a pattern, so grouping that function is allowed by D4 and §6.1, and a parsed tree with it must round-trip strictly.
- a choice other than the last whose pattern is a type annotation, also after a `name =` chain. The longest-type rule of §6.1 takes the following `|` as a type union, even when the annotated type is in `{ }`. Parentheses would make a tuple pattern, which is different code. This corrects the incomplete list: the core tree with the choices `f : { (Int) -> Int }` and `f : String` has no same-code text. An annotation inside a constructor or tuple ends inside its brackets and is not in this case. Parsed trees with no diagnostic still round-trip strictly.

`print_module` prints such a tree as it is, with no binding added (a `g = F` with no written type would take the expected type away from F, §4.8, so it is not the same code and can make a correct program fail to check). The callers that need the same code call `unprintable` first and report an error with the span: the tests here, and later `i fmt --to=i` of `meta.md` §1, which already reports an error for a quote. A tree from the parser with no diagnostic is never in this list: the first case and place (1) are E0113 (D4), and in place (2) the parser decides Block (test 2), so it never builds such a clause block. So for every tree from the parser with no diagnostic `print.rs` adds nothing, `unprintable` is `None`, `common::shape` stays strict in `golden.rs` and `spec.rs`, and the round trip is exact. For a tree with no groups (a core tree from the Lisp surface or from a macro, `meta.md` §1) that `unprintable` accepts, the round trip holds up to `canon` (§3.15). A test in `tests/parse.rs` builds such trees by hand and checks that `print.rs` adds the group or the line where it must, and nothing where it need not, and that the text parses back to the same `canon`: a `Case` under `*`, a `Case` as a callee, a subject `a + b`, a pattern that starts with `-` after another clause on one line, a `Clauses` with the first clause `(x) -> (y) { y }` (group), a `Case { syntax : New }` with the same first clause (no group, strict `shape`), a `Case { syntax : New }` with the first clause `n = (x) -> (y) { y }` (group), and a `Case { syntax : Old }` with it (no group). It also checks that `unprintable` gives the span for `{ (x) -> Int { x }(1)  2 }`, for `{ (a) -> (x) -> Int { x } }` as a `Clauses`, and for `n = (a) -> (x) -> Int { x }` as the first clause of a `Case { syntax : New }`, and `None` for every tree that `golden.rs` and `spec.rs` parse.

### 3.14 `format.rs` and the migration plan

`format(src)` in stages 1 and 2 (no migration yet):
- It parses in Both mode and prints each node in the syntax it was written in, as `print.rs` does (§3.13): the header by its style; `Case Old` as `case S {` with `if`; `Case New` as `S |> {` with `?`; `Clauses` as `{` clauses `}` or `{}`; `type_params` as `name(A, B)`; `MacroItem`, attribute `module` and `call`, `ValueOf`, `TypeCall`, and `head` in their new forms; `{ }` around a function type that ends a clause pattern before `?` or `|`. The break rules of the old printer apply to the new nodes as to their old twins (a `Clauses` breaks as a `case` body does).
- So in stages 1 and 2, `format()` keeps the syntax of every file: `format_properties` (which already runs `check()` on every ```` ```i ```` block of `docs/spec.md` and every `.i` file that parses) holds for the new spec blocks and the new goldens, and `i fmt --check` stays green. The stage-1 gate includes `cargo test -p ilang-syntax` (§7).

`format(src)` from stage 3:
1. Parse in Both mode. With errors: return them (D10), with the `--from-old` hint when Old mode reads the file with no diagnostic.
2. If the tree holds a lowercase `TypeCall` with no module and Old mode reads the file with no diagnostic and a different tree (J1): refuse the file (D10).
3. Compute the plan on that tree. If the plan refuses the file (a `hand` reason of R4, R5, R6, R7a, or R8 in splice mode), `format()` returns an error, `Refused { reason, span }`, with the message "this file needs a hand edit before it can be written in the new syntax: <reason>", and prints nothing. `i fmt` reports it like a parse error, and the LSP returns no edit.
4. Otherwise print from the plan.

`format_from_old(src)` always reads Old mode and runs the write check. `migrate::splice(src, &plan)` is the splice printer. `Formatter::module` finds the header from `imports_span` and `exports_span`, not from keyword tokens.

The plan (`crates/ilang-syntax/src/migrate.rs`, removed in A3) holds, keyed by spans of the old tree: renames, inserted declarations, groups, g-bindings, splits, dropped empty groups, choice splits, R7a rewrites, line ends, and the report. `Env` is the set of type names that may be written with no module name: `migrate::UNQUALIFIED_TYPES` (built-in and prelude types; a test in `ilang-check` asserts it equals `BuiltinType::ALL` plus the types of `lib/prelude.i`), the module's own type definitions, and the `UpName`s of its imports.

Rules:
- **R1 Header.** Drop the `import` and `export` words. Drop an empty `import ()` or `export ()` with its line. Each group and the first item start their own line (R8).
- **R2 Match.** `case S { … }` becomes `S |> { … }`.
  - R2a: `S` gets a group, `{ S }`, when its top operator is at level 1 to 5.
  - R2b: the whole match gets a group when it is an operand of a level 7 or 8 operator, of unary `-` or `-.`, a callee, or the target of `.f` or `.k`.
  - R2c: the whole old match also gets a group when it is the left operand of any other binary operator, `|>` included. This rule does not wrap a new-syntax match: its subject may receive an expected type from its fitted pattern (§6.12), which an extracted function could lose. The binding rules do not need this group, but it keeps the number of stop sites (§3.17): `lower.rs` puts the site of a pipe call, a call, and an operator that may stop at `e.span.start`, and `site()` merges sites at the same module, line, and column (`site_map`, `lower.rs:1350-1356`). Old `r = case h() { _ -> 1 } |> g` has the pipe site at `case` and the `h()` site at `h`: two sites. Without R2c the output `h() |> { _ -> 1 } |> g` starts both at `h`, and they merge into one. With R2c, `{ h() |> { _ -> 1 } } |> g` puts the pipe site at the `{` and the call site at `h`, so the count and the order stay. The same holds for `case f(x) { … } == y`, which becomes `{ f(x) |> { … } } == y`.
  - A match needs no group as the right operand of an operator at level 1 to 6, an argument, a list part, a spread, a statement, or the value of a binding. In these places no expression that starts with the match gets a site at its start: a binding's site is at its pattern, and a call's site is at its callee.
- **R3 Guard.** `if` becomes `?`. When the pattern of the last choice ends in a type whose rightmost part (through the right side of `|`, `&`, and `-`) is a function type not already in `{ }`, that function type is put in `{ }`.
- **R4 Type variables.** Owners are found exactly as resolve finds them today (`annotation_vars`, `push_ty_frame`, from the outside in). New name: take out each `_`, make the letter after it a capital, and make the first letter a capital. Variables of one binding are named in the order of first use. A new name `N` of an owner `O` clashes with:
  - a name in `Env`;
  - another new name of `O`;
  - a new name of an outer owner whose variable is used inside `O`;
  - an `UpName` type variable that the tree already declares (Both mode reads files that mix the two kinds, §3.10): one declared by a binding, declaration, or type head that encloses `O` and is in scope at `O` (in `f(A) = (x : A) { g = (y : a, z : A) { y } … }`, g's `a` may not become `A`, or z's `A` would then name g's variable); and one declared by any binding inside the value of `O` at a place where a variable of `O` is used (in `f = (x : b) { g(B) = (y : B, z : b) { z } … }`, f's `b` may not become `B`, or g's declared frame would capture z's `b`).

  A name that clashes takes the first free name of `A1`, `A2`, … (free by the same four rules) and is reported.
  - **A variable with no owner.** A lowercase variable in a type definition that is not in its head (E0222 at base, `resolve.rs:1393`, as in `Free { Free(a) }` and `AliVar = List(b)` of `projects/type_rules`, and the aliases with no head of the parser-only golden `types.i`, such as `T5 = (a) -> { (b) -> c } ! E`) has no owner. The plan renames it by the same rule and declares it nowhere; its name may not clash with a name in `Env`, a parameter of that head, or any `UpName` written in that type definition, and takes `A1`, … otherwise. So `apply(plan, …)` is defined for such a file, and its tree after the plan is the old tree with the names changed. Each one is reported (`unowned`). In a parser-only file nothing else happens. In a compiled file the program is an error before and after, but the code changes: E0222 becomes E0202 with the type-definition hint of §3.10 (`to use a type parameter, declare it in the head`). A head is never added, since that would make an error program correct and lose the expected error. Each such file gets a `hand` entry with reason `tyvar-unowned` (no hand edit is needed; the entry records the changed code), so the AG check of lost errors (§8) accepts E0222 → E0202 there and nowhere else. So after the plan every type variable use names the same owner as before; the checker test of §6.2 asserts this with resolve on both trees, since the write check and the format property compare only syntax.
  - Declare on the owner: `n(A…) =`, `n(A…): T =`, `n(A…): T` for a declaration (variables in the order of first use in its type), `Name(A…)` for a type head.
  - An owner whose pattern is not one name (a tuple, a chain `a = b = …`, a constructor pattern), at the top level or inside another binding: split the binding into one binding of one name per part (reported: `split`) only when all of these hold:
    - its value is a tuple with one part per name;
    - the whole value is direct at base, by `migrate::is_direct`, a copy of `resolve.rs` `is_direct` (`:1791`) on the syntax tree in which every `m.x` counts as not direct (resolve needs scope to know that `m` is a module; counting it as not direct only sends more files to the hand list, never a wrong split);
    - no part uses a name of another part;
    - no type variable is used in more than one part.

    Why the value must be direct: `binding()` (`resolve.rs:1760`) puts the names of the pattern in scope inside a local value only when the whole value is direct, and a binding is generalized only when its value is direct. A tuple is direct only when every part is. When the whole is direct, every part is direct, so after the split each part sees its own name as before (and, by the third condition, uses no other name of the pattern), and each part is generalized as the whole was. When the whole is not direct, a part that is a function would become a direct binding of its own: its own name would then refer to itself, not to an outer binding, and it would be generalized where base did not generalize it. For example, in `f = (x) { x + 1 }` and `main = () { (f, n) = ((x : a) { f(x) }, list.length([1]))  f(n) }`, the inner `f(x)` calls the top-level `f` at base, and `main()` gives 2; split, `f(A) = (x : A) { f(x) }` would call itself for ever. So such a binding is refused. Otherwise the file is refused and goes to the hand list (reported: `hand`, reason `tyvar-owner`). The variables are never moved to an enclosing binding: that binding is not generalized over them inside its own value, so a program that uses the inner names at two types would stop checking. The R4 capture test (§6.2) also compares, for each name use, the binder it resolves to before and after the plan, so a split that changes what a name refers to fails it.
- **R5 Function at the start of a `{`.** When the first statement of a `{` that begins an expression in the output (old blocks and groups, groups R2 adds, old subjects that become a first statement) starts with a function F that has a `->` part and whose `( … )` reads as a pattern (exactly the condition of test 1a, which also holds for the same form in the new syntax by test 2), the rewrite depends on the place of F (§4.8 lists the places that give an expected type: an argument, also with `|>`; the value of a binding with `x : T` at its top; the body of a function with a `->` part; the last expression of a block; a clause expression; a part of a tuple or a list):
  - F is a callee, the subject of a match (an old `case` subject), an operand of a binary operator other than `|>`, the target of `.f` or `.k`, or a whole statement that is not the last of its block. None of these places gives F an expected type, so binding F to a name with no written type changes no type: insert `g = F` before the statement and put `g` in the place of F. `g` is the first of `g`, `g2`, … not used in that `{` (reported: `g-binding`).
  - F is in any other place (the whole last expression of the block, so it has the expected type of the block; or the left side of a `|>` whose right side is not a clause block, so it is an argument): the file is refused (reported: `hand`, reason `fn-expected-type`). The hand edit writes the type on the name, `g : T = F`, as the D4 message says.
  At base the corpus has no old block of this form (zero E0113, §6.1), so R5 can only fire on a group or subject that R2 makes, where F is the start of a match subject.
- **R6 First clause that gives a function.** Only where §6.1 needs it, which is place (2) of §3.13: the first clause pattern of a clause block in the output is a chain `n1 = … = ( … )` (test 2 then decides Block), and the clause expression starts with a function F. Place (1) of §3.13, a lone clause block or one followed by `(` or `.`, never comes from the migration: R2 makes every old `case` a match after `|>`, R2b puts it in a group when it is a callee or the target of `.f` or `.k`, and old code has no lone clause block. So R6 never fires on `case p { (a, b) -> … }`, which becomes `p |> { (a, b) -> … }` and is read as a match.
  - F has no `->` part: group the expression: `n = (a, b) -> { (x) { x + a } }` (reported: `group`, with no category in the allowed list, §3.17).
  - F has a `->` part, and F is only the start of the clause expression, in one of the places of R5 that give no expected type: `n = (a, b) -> { g = F  g(1) }` (reported: `g-binding`).
  - F has a `->` part and is the whole clause expression, which has the expected type of the match: the file is refused (`hand`, reason `fn-expected-type`), since `g = F` would take that type away from F.
  So the rule runs the same way on files that are already in the new syntax (§3.14 step 3): plain `i fmt` never rewrites a correct first clause such as `p |> { (a, b) -> (u) -> String { u.name } }`. The write check (D12) asserts no E0113 and the same tree, and the formatter test inputs of `tests/migrate/` hold the cases of R5 and R6, including the expected-type case `h = (p : (Int, Int)) -> (User) -> String { case p { (a, b) -> (u) -> String { u.name } } }`, whose output must be `p |> { (a, b) -> (u) -> String { u.name } }` with no binding; the checker test of §6.2 checks that this output still checks.
- **R7 One clause, no guard.**
  - R7a (no choices): see D13. When `s` would get a known constructor, the file is refused (reported: `hand`, reason `one-clause-known`). Every other one-clause, no-guard, no-choice case is reported as `one-clause`.
  - R7b (choices, under D7 default): write one clause per choice, each with the same expression (reported: `choices-split`).
  - R7c (tests whose meaning changes, the `syntax.md` note of that name): a one-clause, no-guard case in a test that expects an error would lose that error, because the same clause block is fitted, reports to `Assert`, and reads `x : T` as an annotation. So such cases are found by machine and fixed by hand before the migration, in the old syntax (A2 phase M, §8). The finder is `scripts/a2-meaning`: it runs `i fmt --from-old --check --report` (no write) on every test source and joins the `one-clause` and `choices-split` lines of the report with the expectations of the test: a `tests/spec` file whose header expects `compile-error`, and a checker test (`crates/ilang-check/tests/**/*.i` with its `.out` or `expected.out`) with a diagnostic whose position is inside the reported case. Each hit goes on the `one-clause-test` list, `$C/a2/meaning.txt`, with the file, the position, and the expected code. At base this takes in at least `tests/spec/7/not_covered.i` (E0516, `case c { Red -> "red" }`) and `bad = (v) { case v { x: a -> x } }` in `crates/ilang-check/tests/types/case.i:85` (E0511, `case.out : 17`). The `case r { Ok(x) -> x }` cases of `crates/ilang-check/tests/types/coverage.i` that `syntax.md` names (`:126-130`, `:146-150`) are tests that one clause does cover (`Result(Int, Never)`, and the known value `Ok(1)`); they expect no error, so they are not hits, and they keep their meaning as fitted bindings. The finder, not this list, decides. The plan itself does not change: `format()` has no test expectations.
- **R8 Line ends.** Before a clause, statement, or item whose output starts with `(`, `-`, `-.`, or `{` and that shares a line with the one before, insert a line end with the indentation of that line. `{` is in the list because R2a, R2b, and R2c make statements that start with a group: in splice mode, old `p = (1, 2)  case a == b { … }` would become `p = (1, 2)  { a == b } |> { … }`, and `(1, 2) { a == b }` reads as a function and its body (§6.1, §1.7). With the line end, the `)` ends the line and the `{` begins a statement. The printer of `format()` puts each statement on its own line anyway. In splice mode, a file that has a CR refuses this rule and goes to the hand list, so the number of CRs never changes.
- **R9 Splice spacing.** An inserted `{ ` or ` }` goes right at the node's edges; no other space changes.
- **R10 Files with parse errors.** `--keep-layout` does only the parts that touch nodes the Old parse read (R1, R2, R3, R4 where the owner was read). The rest is by hand from the A2 list.
- **R11 Comments.** Never changed by the migration (A2 changes them by hand).
- **R12 Report.** Every R4 fallback, split, and variable with no owner (`unowned`); every R2c group and R6 group; every R5, R6, R7b; every one-clause case; every binding or parameter pattern whose text the plan changes (`pattern-text`, with the old and the new text of the pattern, §3.17); and every refused file with its reason (`tyvar-owner`, `one-clause-known`, `fn-expected-type`, R10 parts). The R7c list is made from this report (`scripts/a2-meaning`). Each line has a category, the enclosing top-level binding (`module.name`), the source span, and, for one-clause lines, the position of `p` in the old file and in the output. The output position is found by walking `parse_module(out)` and `apply(plan, old)` side by side: the write check has just proved them canon-equal, so their nodes match one to one once groups are skipped.

### 3.15 The write check and the splice printer

- `canon(m)`: the shape text of `m` after removing every one-statement expression `Block` (a group). The only difference between the two sides of the check is the choice of groups. The write check runs in library code (`format_from_old`, `migrate::splice`, and `i fmt --from-old` and `--keep-layout` in `ilang-cli`), which cannot reach `crates/ilang-syntax/tests/common.rs`. So stage 1 moves the shape printer into the library as `ilang_syntax::shape::shape(&Module) -> String` (new `src/shape.rs`, `#[doc(hidden)] pub`, the `Debug` text with spans removed, as `common::shape` is at base), adds `shape::canon(&Module) -> String` next to it at stage 3, and makes `tests/common.rs` call `ilang_syntax::shape::shape`, so the goldens and `spec.rs` keep the same text. `migrate.rs` removes `canon` in A3; `shape` stays for the tests.
- Write check (D12): `canon(parse_module(out)) == canon(apply(&plan, parse_module_old(src)))`, and `parse_module(out)` has no diagnostic (for R10 files: the same codes as before).
- Splice asserts: every byte outside an edit is unchanged; the number of CRs is unchanged; for a `format/*.i` input, the output still differs from its `.out`.
- The splice printer returns an offset map (old offset → new offset). The A2 position view uses it to check each `// at:` line that `UPDATE=1` writes.

### 3.16 The test-only feature

- `crates/ilang-check/Cargo.toml`: `[features] one-clause-as-case = []`. One `cfg!` test in the desugaring arm (§3.9 (a)).
- It never reaches a normal build:
  - `ilang-check` exports `pub const ONE_CLAUSE_AS_CASE: bool = cfg!(feature = "one-clause-as-case")`, and `crates/ilang-cli/tests/check.rs` asserts it is false, so a plain `cargo test` (and dev-verify) fails if feature unification turns it on;
  - `const _ : () = assert!(!(ONE_CLAUSE_AS_CASE && !cfg!(debug_assertions)));` makes a release build with it fail to compile;
  - E2 runs in its own target directory (`$C/target-e2`) in the assigned rev lane, with `cargo build -p ilang-cli --features ilang-check/one-clause-as-case --target-dir $C/target-e2`.
- S4 safety: the old-rule feature may remove the expected type needed by a new match subject. When inference reports E0506, keep the ordinary diagnostic and return no typing result for audit codegen. Lowering needs the missing field/call information. The normal build keeps its existing behavior, and the blind expected-subject regression covers both settings. This small check is in `ilang-check/src/lib.rs`, part of the feature fix required by S4 ruling 2; inference and lowering stay unchanged.
- A3 removes it.

### 3.17 `scripts/wat-diff` and the allowed list

`scripts/wat-diff A B [--allowed FILE] [--renumber] [--a1-unformatted DIR] [--byte-equal]`, Python 3 standard library only, run with `python3 -I`.

- Files: compare every shared export against base. A file only on side A fails; a file only on side B must come from an added blind case under `tests/spec`, and the tool lists each one. E1 uses `--byte-equal`: shared files must be byte-identical, and added blind exports are listed without comparison to base. E2 and E3 use `--a1-unformatted DIR`: each added export is compared with the same unformatted source's A1 export, using the same compiler settings (the feature on for both E2 exports, off for both E3 exports) and the same allowed categories as for shared exports. The added export sets must match the unformatted reference exactly. A missing reference, an unknown addition, or a missing export fails. When either compared export starts with `ERROR`, compare the text exactly, with no masking: identical errors pass, and a changed error or a transition between an error and WAT fails. List every base ERROR export for E2 and E3. This corrects the earlier rejection of all new blind ERROR exports, by the coordinator's S4 ruling: a new blind case has no base export, so base cannot be its reference. In particular, the test-only package may correctly have no WASI program entry with tests disabled. The reference extension validates its exact error instead of rejecting it merely for being new.
- **Strict mode** (no `--allowed`; the plan's rule): mask only the string of the `(data $site_data "…")` line; assert that the site count, the `(i32.const N)` of the `$sites_install` call in `$ilang_start`, is equal; every other line must be equal.
- **The funcs of the WAT at base**, which allowed mode must know (checked in `emit/mod.rs` and `emit/control.rs`):
  - `$f<id>` of kind `FuncKind::Code` (`emit/mod.rs:249`): the code of a function, with a `;; <name>` comment line just before its `(func` (`emit/mod.rs:297`). A top-level function has the name `path.name`; a closure has `<module path>:fn<N>` (`lower.rs:1646`), with `N` its func id, which is global to the program.
  - `$f<id>` of kind `FuncKind::Ctor` or `FuncKind::Runtime` (`emit/mod.rs:255-276`): a constructor with fields, or a library function, used as a value (as in `check(show(Ok), …)` in `tests/spec/11/text_form.i:105`). It is printed by `header()` alone, with no comment line. `ctor_func` and `op_func` (`lower.rs:1233-1259`) make at most one for each constructor and each op, and number it with the lazy `new_func()` among the other `$f` funcs. Its body names only its own locals, `$record` with the constructor and type ids, or the op, so its body text is unique in the file.
  - `$s<N>` and `$j<N>` (`emit/control.rs:430-505`): the rest of a function that may suspend, cut at point `N` (a segment) or at join `N`. `control::split` copies the code after each point and join of such a function into these funcs. Their comment is `;; <name> after point N` or `;; <name> join N`, where `<name>` is the comment name of the function they come from. `N` is numbered across the whole program.
  - `$r<N>`: the re-entry func of point `N`, written right after `$s<N>` with no comment of its own (`emit/control.rs:512`).
  - `$start`: the code of the top-level values, with the comment `;; ilang_start` (`lower.rs:807`, `emit/mod.rs:87`), and its own `$s`, `$r`, and `$j` funcs, whose comments start with `ilang_start`.
  - `$ilang_start`, the exported start, with no comment: it installs the sites (`$sites_install`, with the site count) and the constructors, and then calls `$start`. It holds no code of any binding.
  - `$f<id>` wrappers with the comment `import <module> <name>` or `async entry`, and their segments and re-entry funcs. These have no source owner and are always compared. This corrects the owner model after checking the actual base exports: it omitted the wrappers made by `make_import_func` and `async_body` in `lower.rs`. Their stable names use the hex-encoded comment and an occurrence count for that comment and func kind in file order, never the program-wide func number. Repeated `async entry` comments are allowed. Unknown code comments still fail the check.
  - Module-level lines outside every func that name funcs by their global number: `(global $c<N> (ref $clo<n>) … (ref.func $f<N>))` for each func used as a constant (`emit/mod.rs:210-217`, used as `global.get $c<N>`), and the one `(elem declare func $f0 … $fK $r0 … $rP …)` line (`emit/mod.rs:220-223`).
  - Strings: one `(data $s<K> "…")` line per string, numbered by first use, read by `array.new_data $bytes $s<K> (i32.const 0) (i32.const <length>)` (`emit/mod.rs:463-473`). Data names are not func names, so the renaming below never touches them. One kind of string holds source text: `pattern <source> did not match ` (`lower.rs:2494-2500`), the text of a binding or parameter pattern that may fail, which is also the start of its stop message.
- **Allowed mode**: the same as strict mode, after two steps on each side.
  1. **Owners.** Each func gets an owner, the top-level function or `ilang_start` whose code it holds: a `$f` func with a `path.name` comment owns itself; `$s<N>` and `$j<N>` belong to the `<name>` of their comment; `$r<N>` belongs to the owner of `$s<N>`; `$start` belongs to `ilang_start`; a closure `$f` (comment `<module path>:fn<N>`) belongs to the owner of the first func, in the order of the file, whose code has its `$fN` as an operand of `ref.func` or of a call. A closure that no func reaches, or a func the tool cannot place, fails the check. `$ilang_start` and the funcs of the runtime have no owner and are always compared.
  2. **Renaming.** Every func identifier is renamed to a name that does not depend on numbering across the program: `$f` of a top-level function to `$<path.name>`, a closure to `$<owner>/fn#k`, and `$s<N>`, `$r<N>`, `$j<N>` to `$<owner>/s#k`, `$<owner>/r#k`, `$<owner>/j#k`, where `k` counts the funcs of that kind and owner in the order of the file. A `$f` func with no comment line (a `Ctor` or `Runtime` func) is renamed to `$value#<d>`, where `<d>` is a digest of its body text with its own name taken out; it has no owner and is always compared. A body digest that is on one side twice fails the check. Each constant global `$c<N>` is renamed after its func: `$c<N>` with `$f<N>` renamed to `$X` becomes `$c : X`. The rename applies to the definition, to every `call`, `return_call`, `ref.func`, and `global.get $c` operand, to the numbers in the comment lines, and to the module-level lines: the `(global $c… …)` lines are then sorted, and those whose func belongs to a listed owner are left out; the `(elem declare func …)` line is compared as a set of renamed names, leaving out the names of the funcs of listed owners. So a different number of points, joins, or closures in one function, which moves the numbers of every later `$f`, `$c`, and `$r`, does not change the text of any other func or of any module-level line.

  Then the funcs of a listed owner are skipped, whatever their number; every other func is compared line by line after renaming; the set of owners and, for each owner that is not listed, its funcs, in order, must be equal on the two sides. An entry names its owner by `path.name`, or `ilang_start` for a top-level value that is not a function. An `ilang_start` entry skips `$start` and all its segments, which hold every top-level value of the program, so the tool prints a warning line for it, and each such entry needs a reason in the allowed file. Expected at base: no `ilang_start` entry, since the one-clause cases of the dumped files are all inside functions (`values_in_every_place` in `tests/spec/6/blocks.i`, and the test functions of `7/patterns.i`).
  - Example: `t = (p, g) { case p { (a, b) -> g(a) + b }  g(0) }` may suspend (it calls the closure `g`), so it has `$s`, `$r`, and `$j` funcs. The `one-clause` entry for `t` changes its joins (base lowers a case, E3 a `Bind`), and so the numbers `N` of every later `$s`, `$r`, and `$j` func, and the `return_call $jN` and `ref.func $rN` lines in other functions. After renaming, the funcs of `t` are skipped as one group, and the other funcs read the same on both sides.
  - **Whole-dump entries.** A `split` of a top-level binding (R4) whose parts are functions changes which bindings are known functions (`lower.rs:716-735` makes a top-level binding a known function only when its pattern is one name and its value is a `Fn`). Callers that are not listed then change from a closure call to `call $fK`, which renaming does not absorb. So a top-level `split` entry (and a `hand` entry marked `whole`) makes the tool skip the whole dump of every program that holds that module: it only checks that the dump exists on both sides and does not start with `ERROR`, and prints a warning line. Such programs are then covered only by `i test` on the formatted copy. A whole-dump entry on a module that is in every dump (`lib/`) is not allowed: the check fails, and the case goes to the coordinator. A split inside another binding stays a normal entry for its enclosing top-level function. Expected at base: none; the R12 report gives the real number.
  - A test of the tool (`scripts/wat-diff-test`, run in dev-verify from S1) runs it on hand-made WAT pairs: a skipped owner with one more join, so that later `$jN` numbers shift; a closure of a skipped owner, which adds a name to the `(elem declare func …)` line and shifts the numbers of a later `Ctor` func, its `$c` global, and its `global.get` in a func that is not listed; a `Ctor` and a `Runtime` func with no comment, unchanged (must pass with no entry); a `pattern-text` entry, and the same string change with no entry (must fail); an `ilang_start` entry; a change in a func that is not listed, which must fail; and an owner on one side only, which must fail.
- **Pattern text.** R4 renames type variables, and R3 puts `{ }` in a clause pattern, so the text of a pattern can change, and with it a `pattern <source> did not match ` string (and the stop message a user sees, which shows the source as written; that change is intended). For each `pattern-text` entry of the allowed file, allowed mode first rewrites side A: the `(data $s<K> "pattern <old> did not match ")` line gets the new text, and every `array.new_data $bytes $s<K> (i32.const 0) (i32.const <length>)` gets the new byte length. A string that is not found, or a new text that equals another string of side A, fails the check. No other data line may differ. A pattern whose text changes but that may not fail has no such string, and its entry does nothing. Strict mode has no such step: E1 runs on unformatted sources, whose text does not change. The `expect: stop "…"` lines of `tests/spec` that show such a pattern are made again by `UPDATE=1` in A2 (§8).
- **Site count.** Two things could change the number of sites. First, a one-clause binding gets a stop site only when infer marks its pattern as may-fail (`bind_checked`, `lower.rs:2472`); an old one-clause case compiled only when its clause covered its subject's type, and an old covering case has no site. Second, R2 moves the start of a match from `case` to its subject, and sites at the same line and column merge (`site_map`); R2b and R2c put a group around every match that is the start of a larger expression that may get a site there (an operand on the left of any binary operator, a callee, a target of `.f` or `.k`), so the larger expression starts at the `{`, where `case` was, and nothing merges. Other rewrites do not move a site onto another: R1, R3, R8 move lines but keep each site on its own position, and R5 and R6 add a binding with no site and move a call from the function to the name `g`. So the expected count difference is zero, and strict equality is the default; a difference is a finding. The bound is checked by machine: the test helper `ONE_CLAUSE_REPORT=<file>` in `crates/ilang-check/tests/` reads the migration report and checks the formatted copy with the feature off (as E3 builds it; with the feature on, the match is a case and `binding_may_fail` never records it). It looks up `may_fail` (keyed by `(module, pattern span)` of the compiled source, `pattern.rs:363`) at the output position of each `p` that the report gives (R12), and writes `may-fail` on the entry. An entry whose output position finds no `may_fail` key and no binding pattern there is an error, so a wrong map cannot pass silently. Only with entries marked `may-fail` does `--renumber` apply: site codes in the known site forms (`$shadow_site`, the site argument of `$async_begin`, `$stop_error`) are decoded to `module:line:col` through `$site_data`, `$sN` names become their string content, data lines are sorted, and the count difference must be at most the number of such entries. The tool fails on a site form it cannot decode.
- **Allowed file** `$C/wat/allowed-a1.txt`, one line per entry: `<category> <function path.name> <source file:line> <report id> [may-fail] [whole] <reason>`. It is made from the R12 report over the rev pool, and every entry must match a report line (checked by the equivalence view). Categories:
  - `one-clause`: a match lowered as a binding;
  - `choices-split`: R7b;
  - `g-binding`: R5 and R6 (a `g = F` local may change the WAT);
  - `split`: R4 (a top-level split of functions is a whole-dump entry, above);
  - `pattern-text`: a binding or parameter pattern whose text the plan changes (R12), with its old and new text; it changes only the string of that pattern (above), and the funcs of its owner are still compared;
  - `hand`: anything fixed by hand in A2, with a reason (including the refused files of R4, R5, R6, and R7a, and the `tyvar-unowned` files of R4).
- **Groups have no category.** A group is a hir `Block` of one `Expr`; `lower.rs` lowers it through `block()`, which lowers the last expression with the same tail flag, and `infer.rs` infers it with the same expected type. So groups must leave the WAT byte-equal; if E2 finds a difference only a group explains, that is a finding, not an allowed entry.
- Expected size at base: the scan in this design finds one-clause cases only in tests; in the WAT dumps (std, bench, examples/hello, `tests/spec`) these are `tests/spec/6/blocks.i:32` and the seven in `tests/spec/7/patterns.i:96-139`. `7/not_covered.i` expects a compile error, so its one-clause case is on the `one-clause-test` list (R7c): E2 and E3 leave it unformatted, and it has no WAT on either side. No `choices-split` entry is in a dumped file.

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

Examples in unchanged sections stay in the old syntax until A2; Both mode reads them, so every ```` ```i ```` block still passes `crates/ilang-syntax/tests/spec.rs`. The §9.6 `// type:` comment changes in A2 phase 0, by hand, in the commit that changes the Namer, because it must match what the Namer shows (`crates/ilang-check/tests/types/spec.i:68`) (finding 5).

## 5. Files to change, with their regions

Inside A1's ranges:
- `crates/ilang-syntax/src/ast.rs`: §3.3.
- `crates/ilang-syntax/src/parser.rs`: entry points (:18), `module()` (:286), `item()` (:397), `attribute()` (:452), `type_def()` (:474), `stmt()` (:572), `binary()` (:636), `primary()` (:766), `case()` (:921), `type_prim()` (:1228), plus the new `decide()` and `clauses()`.
- `crates/ilang-syntax/src/lib.rs`: codes, `parse_module_old`, `parse_stats`, `old_syntax_hint`.
- `crates/ilang-syntax/src/shape.rs` (new, stage 1; `canon` at stage 3) and `crates/ilang-syntax/tests/common.rs` (calls it).
- `crates/ilang-syntax/src/print.rs`: §3.13.
- `crates/ilang-syntax/src/format.rs`: §3.14 (the syntax-keeping printer at stage 1, the migration at stage 3).
- `crates/ilang-syntax/src/migrate.rs` (new, stage 3).
- `crates/ilang-syntax/tests/*`: §6.
- `resolve.rs` ranges: `type_vars`/`expr_annotations` (:206-325), `ty_var` (:1382-1407), `push_ty_frame` and the new `push_declared_frame` (:1742), the desugaring arm (:2007).
- `docs/spec.md`: §4.
- `scripts/wat-diff` and its test `scripts/wat-diff-test` (new, S1); `scripts/a2-meaning` (new, S3, R7c).

Outside A1's ranges (one change to the split of files for the coordinator; T1 and T2 are told to rebase on it):
- `resolve.rs`: the `T::Named` branch and the `ValueOf`/`Call` arms of `fn ty` (:1329, T2's); the head checks in `type_defs` (:1098) and the alias code (T2's); `is_direct` (:1791); `values()` Declaration branch (:1688) and `binding()` (:1760); `collect()` `MacroItem` and `Declaration` arms (:455-510); `attributes()` (:600-735) messages and new forms; the `cfg!` branch of D14; the E0215 message for an exported name that is not defined (`:784`, below).
- `resolve.rs:784` (stage 2): when a lowercase exported name is not defined and is the last part of a module path that the package may import (a module `<p>/<name>` of a package `p` in its graph, the library `i` included), the message `` `{name}` is exported but not defined in this module `` gets "; a module path has a `/`, as in `(i/{name})`", with the first such path. This is the checker side of `golden/err_module_path.i`, which moves to the checker (the `syntax.md` note on that file).
- `crates/ilang-check/src/lib.rs`: the new codes and feature declaration; the feature-only E0506 typing guard required by S4 ruling 2 (§3.16).
- `crates/ilang-check/Cargo.toml`: the feature.
- `crates/ilang-check/src/query.rs` (T2's and A3's): `Scope::block` (:1413), the `%` filter (:1163 and `pattern_binders`), the `MacroItem` arm (:999); `builtin_head` in A2.
- `crates/ilang-cli/src/commands/fmt.rs`: `--from-old`, `--keep-layout`, `--report`, `--no-choices-split`, and the `Refused` error of §3.14.
- The parse-error path of `i check` in `ilang-cli`: the `--from-old` hint of D10.
- `crates/ilang-lsp`: no edit for a `Refused` file.
- `crates/ilang-cli/tests/check.rs`: the feature guard.
- `crates/ilang-check/tests/*`: the new checker tests and the `ONE_CLAUSE_REPORT` helper.
- In A2: `crates/ilang-check/src/types/display.rs` (Namer), `crates/ilang-cli/src/commands/package.rs` (`TypeShow`, the interface printer), the messages in `types/pattern.rs:734` and `resolve.rs:2600`, `crates/ilang-cli/tests/spec.rs` (`UPDATE=1`).

Not changed: the lexer, `infer.rs`, `lower.rs`, `coverage.rs`, `order.rs`, hir.

## 6. Tests

### 6.1 Syntax (stage 1)

- `tests/both.rs` (new), for claim D:
  - (i) `both_agrees_with_old`: for every corpus source, if Old mode has no diagnostic, Both mode gives the same `shape` and no diagnostic or only E0113 diagnostics, or the source is J1: Both mode's tree holds a lowercase `TypeCall` with no module at a place where Old mode's type ends, and Both mode has a diagnostic or `format()` refuses the file (D10). The number of E0113 and of J1 on the corpus is zero.
  - (ii) Seeded mutants of each corpus file (a break added or removed, `(` moved to a new line or up to the line before, `->` put in, `{` swapped with `(`), about 50 000 parses; the same assertion for each mutant that Old mode accepts. The A1-RF ambiguity hunter adds what it finds here.
  - (iii) Each new form parses only in Both mode.
  - (iv) E0113 cases: `{ (x) -> Int { x } }`, `r = { (x) -> Int { x + n }(1) }`, `{ (x) -> (y) { y } }`, `{ (a, b) -> (x) { x + a } }`, `v |> { (a, b) -> (x) { x + a } }(1)`, and, with a postfix chain that Old mode accepts as a call, `main = () { 5 |> { (a) -> Int { a }  pair }.0(1) }` and `h = (x, r) { x |> { (a) -> (b) { a } }.g(1) }` (E0113 and the same tree as Old mode, which reads a block). Not E0113, and a block in both modes: `{ (n, log ! Log) -> Int { log.write(n)  n } }`, `{ go = (n, log ! Log) -> Int { log.write(n)  n }  go(1) }`, `{ k = 2  g = (x) -> Int { x * k }  g(1) }`, and `{ x = y  f = (a) -> Int { a }  f(x) }`. Not E0113, and a match in Both mode (an E0111 error in Old mode): `v |> { (a, b) -> (x) { x + a } }` and `hs |> { (name, _) -> (x) { log(name, x) }  _ -> (x) { x } }`.
  - (vii) J1 cases: `Id(a) = a  (p, q) = (1, 2)` (Old: an alias and a binding; Both: E0101 with the `--from-old` hint) and `@external("m", "f") f : a  (p) = 1` (Old: a declaration and a binding; Both: no parse error, and `format()` refuses it). Not J1: `f(A) = x` at the top level, and in a block as the first statement and as a later one (`{ k = 1  wrap(A) = (x : A) { Some(x) }  wrap(k) }`), which Old mode rejects and Both mode reads as a binding with `type_params: [A]`; and `f\n(A) = x`, which both modes read as two statements.
  - (vi) After A2 the tracked files are in the new syntax, which Old mode rejects, so (i) and (ii) also run on the frozen old inputs of `golden_old/` and `tests/migrate/`, which keep them meaningful until A3.
  - (v) The cache gives the same tree as a run with no cache.
- `tests/golden_old/` (new): frozen copies of the 48 golden inputs of base and their 48 `.out` files. `golden.rs` renders each `golden_old/*.i` with `parse_module_old` and compares with its `golden_old/*.out`, which pins Old mode byte for byte (claim D (a)). It never reads `golden/`, so A2 can rewrite `golden/*.i` and new inputs such as `header_groups.i` have no Old-mode render. `golden/` is rendered in Both mode only.
- Golden `.out` files that change at stage 1, checked against the real parser at stage 1; any other changed `.out` fails the stage:
  - `err_attribute_line`: the errors become exactly one error (rule 2 of §3.4 skips the group, so `recover()` does not report it again), `E0115 @2:1 a group in ( ) with an attribute before it is not a binding, because no = follows it; the imports and the exports have no attribute before them` (finding 14); the tree is `main = () { Nil }`.
  - `err_block_end`: the third error goes; line 3 prints `h = {}`.
  - `err_toplevel`: no parse errors; the tree prints `f(x)`, `Some(x) = y`, and `@external("m", "f")` with `g = 1` (the checker reports E0234 and E0303).
  - `err_trailing`: `E0101 @1:3 expected a type, found ,` (was "a name"); the error at 4:7 goes, and `@custom()` with `x = 1` prints (the checker reports E0301).
  - `err_pipe_target`: the five E0111 lines read "the right side of `|>` must be a name, a call, a function, or a clause block" (§3.6); `golden_old/err_pipe_target.out` keeps the base text.
- New golden inputs (new syntax): `header_groups.i` (a mix of keywords and groups, a header followed by `(a, b) = …`), `clauses.i` (every block and clause block of §6.1, `{}`, `?` guards, a function type before `?`, the first-clause group), `type_params.i` (`id(A) = …`, `id(A): (A) -> A = …`, `inspect(A): …`, `Option(A) { … }`, `send(Stop)` as an expression), `attr_next_line.i` (`@m`, then `(even, odd) = …` on the next line), `kept_forms.i` (`p.name`, `m(Int)` in a type, `f(x)` at the top level, `Schema(User, _) = user`).
- New error goldens: `err_attr_same_line.i` (`@m (even, odd) = …`), `err_record.i`, `err_dual_ambiguous.i` (its message has the bound-function fix of D4), `err_header_empty.i`, `err_header_order.i`, `err_pipe_group.i` (`n |> { 0 -> 1  _ -> 2 } * 10`), `err_guard_word.i` (`v |> { x if c -> 1 }`, which test 2 decides as Clauses because `if` follows `x` (§3.5), so the clause reader gives E0101 "a guard in a clause block is written with `?`"; `v |> { 0 -> 1  x if c -> 2 }` with the same message; and `case v { x ? c -> 1 }` with the old message), `err_clause_tuple_note.i` (`v |> { 0 -> 1  p : (Int, Int) -> e }` and the lone `{ 0 -> 1  p : (Int, Int) -> e }`, each with the tuple-type note of §3.6).
- `tests/parse.rs` (new): `parse_module_old` of `r = { (x) -> Int { x + n }(1) }` gives a block, and `parse_module` gives E0113 and the same tree (finding 10); 40 nested `{` and 200 nested `{` give `decisions == braces`; every example of the opening of the spec and of §6.1 decides as the text says (including the block `{ k = 2  g = (x) -> Int { x * k }  g(1) }` and the match `hs |> { (name, _) -> (x) { log(name, x) }  _ -> (x) { x } }`); a `name (UpName) =` whose `(` is on the next line is not test 1.
- `spec.rs`: every block parses in Both mode with no diagnostic and round-trips through `print.rs`.
- `format.rs`: at stage 1 `format_properties` keeps its base code (it skips every source that does not parse) and runs on the new spec blocks and the new goldens with the syntax-keeping `format()` of §3.14. The `format_skip.txt` list and its equality check are added at stage 3 (§6.3).

### 6.2 Checker (stage 2), `crates/ilang-check/tests/`

- `types/type_params.i`: inner `g(A)` hides outer `A`; `pair(A, A)` (E0226); a variable named `Int` (E0231); `A(Int)` (E0230); an unknown `UpName` with the hint; a declaration used at two types; a declared binding generalized and used at two types; `Box(Int) { B(Int) }` (E0231); `Box(a, B)` (E0232); `f(A) = (x : A, y : b) { x }` (E0236); an outer old binding with `a` whose inner binding declares `B` and uses `a` (correct); the scope of declared variables (§5): `A` used in an annotation inside the body, `f(A) = (x : A) { y : A = x  y }`, and an unknown `A` in a sibling binding; the E0202 hints, one in a binding and one in a type definition.
- `types/clauses.i`: a local `go = (n, log ! Log) -> Int { … }` at the start of a block; a one-clause match is fitted, `x : T` is an annotation, no coverage error; `{ n -> n }` has the label `n`; a multi-clause lone block has none: `f = { 0 -> 1  _ -> 2 }` shows `(A) -> Int` in its scheme, in hover, and in a type error message, and `[{ 0 -> 1  _ -> 2 }, { 1 -> 0  _ -> 3 }]` has no label either; the known type of a lone clause block reaches its clauses: `list.map(opts, { Some(u) -> u.name  None -> "" })` with `opts : List(Option(User))` checks (§4.8); a statement `{ 0 -> 1  _ -> 2 }` gives the "not used" warning, and `(even, odd) = ({ 0 -> True  n -> odd(n - 1) }, { 0 -> False  n -> even(n - 1) })` is a direct value (§6.10); `==` on a clause block uses function identity (§11.4): the same value is equal to itself, and separately made values are different; `loop = { 0 -> Done  n -> loop(n - 1) }` at the top level and locally; `g = () { (x) { f(x) * 2 } |> { f -> f(1) } }` calls the top-level `f`; one clause with choices (E0235); `x |> {}` with `x : Never`.
- `types/kept.i`: E0229, E0233, E0234, E0301, E0302, E0303, E0308 messages.
- `projects/export_module_path` (new, in the new syntax): a file whose header is `(fail)`, with `i/fail` a library module, gives E0215 with the module-path hint of §5 ("…; a module path has a `/`, as in `(i/fail)`"); a file whose header is `(nothere)`, which is no module, gives E0215 with no hint.
- The migration and expected types (stage 3): the output of `i fmt --from-old` on `User { User(name : String) }` and `h = (p : (Int, Int)) -> (User) -> String { case p { (a, b) -> (u) -> String { u.name } } }` checks with no diagnostic, and has no `g` binding; the same input with the clause pattern `n = (a, b)` is refused with `fn-expected-type`; and `i fmt` on the new-syntax text `p |> { (a, b) -> (u) -> String { u.name } }` leaves it as it is.
- `hir.rs`: the shapes of the desugaring, with the made-up `Bind` span equal to `p.span`.
- `query.rs`: completions inside the subject `e` of a one-clause match offer local names, including inside a function in `e`; the names of `p` are offered in the body; `%arg` is never offered or shown, including hover and definitions with the cursor at the `{` of a lone clause block or right after a `,` before it.
- `ilang-wasm/tests/codegen.rs`: top-level `f = { 0 -> 1  _ -> 2 }` is a known function; `get = (o) { o |> { Some(x) -> x } }` called with `None` stops at the position of `Some(x)`.
- `ilang-cli/tests/check.rs`: §10.2 with a header exports group: a package whose `main` is in `(main)` runs, and a private program entry runs too; only host exports must be in the exports of their module; `i check` on an old file with a J1 form gives the `--from-old` hint.
- An interface test in `package_commands.rs`: an exported `g = { 0 -> 1  _ -> 2 }` is printed with no label, never `%arg`.
- A test that `migrate::UNQUALIFIED_TYPES` equals the built-in and prelude types, and one that, for every corpus file, the R4 owners equal the frames resolve builds.
- `R4` capture test (stage 3, in `ilang-check/tests/`, since it needs resolve): for every corpus file and every input of `ilang-syntax/tests/migrate/`, resolve the tree before and after `apply(plan, …)` and map each type variable use (by its position in the old file, through the plan) to the span of its owner; the two maps must be equal. In the same walk it maps each name use to the binder it resolves to (top-level definition, or the span of a local binder, through the plan), and those maps must be equal too, so a split that changes what a name refers to fails (the `(f, n) = ((x : a) { f(x) }, list.length([1]))` input of R4 is refused, and a direct variant of it is split with the same binders). Its inputs include the two mixed cases of R4: `f = (x : b) { g(B) = (y : B, z : b) { z }  g(1, x) }` and `f(A) = (x : A) { g = (y : a, z : A) { y }  g(1, x) }`, which must get `A1`-style names, not `B` and `A`.

### 6.3 Formatter (stage 3), `crates/ilang-syntax/tests/format.rs`

`check(src)` asserts:
1. `out = format(src)` succeeds;
2. `parse_module(out)` has no diagnostic;
3. `canon(parse_module(out)) == canon(apply(&plan, parse_module(src)))`: the same tree after type-variable migration (the corpus has no E0113, so Both mode reads every corpus file that parses);
4. the comments are the same;
5. `format(out) == out`, and the plan of `out` is empty;
6. the existing checks on tabs and spaces.

`format_properties` runs `check(src)` on every source that parses and that `format()` does not refuse. It asserts that the list of skipped sources equals `tests/format_skip.txt` exactly, so no file drops out silently. The list has one line per source: `parse <path>` for a source that does not parse, and `refused <path> <reason>` for one that `format()` refuses (§3.14: the `tyvar-owner`, `one-clause-known`, and `fn-expected-type` inputs of `tests/migrate/`, any J1 input, and every corpus file that R4 or R7a sends to the hand list at stage 3). A refused corpus file leaves the list in A2, when its hand edit is made. The format `.out` files are made again with `UPDATE=1` at stage 3. `tests/migrate.rs` checks `format_from_old` and splice on its own frozen old inputs in `tests/migrate/` (one per rule: R1-R9, an `A1` fallback, the two mixed-kind clash cases of R4, a split, a refused `tyvar-owner` and `one-clause-known` file, CRLF, a bare CR, empty headers); for the refused inputs it asserts the `Refused` error and its reason and on `golden_old/*.i`. These stay old after A2. The run over every tracked `.i` and every ```` ```i ```` block is done by the A2 partition commands and by E2, not by this test, because after A2 those files are new.

### 6.4 Blind spec tests (stage 4)

- Writers see only `docs/spec.md` at the stage-1 commit and the `tests/spec` header format, never code. They write `tests/spec/<section>/*.i` in the new syntax, one or more per item: the three exceptions of the opening; §1.7 (the `(`, `-`, `-.` rule); §2.2 header groups, attributes, the `@m` example; §2.3, §2.4; §2.5 and §4.7 declarations and hiding; §3 kept forms as compile errors; §4.3 head errors; §4.8 fitting, `Assert`, the expected type from `x : T`, the known type of a lone clause block given as an argument (`list.map(opts, { Some(u) -> u.name  None -> "" })`); §5 the scope of declared type variables; §6.1 tests 1-6 with the given examples; §6.9 groups; §6.10 (a clause block as a statement is not used; a clause block is a direct value); §6.12 (stop at `p` with `// at:`, labels, including no label on a block of several clauses, `{}`, `loop`); §7.1 (`?` after a function type); §7.4; §7.5 (no known constructor from one clause); §10.2 (an entry point in the exports group); §11.4 (equality of a clause block); §11.6 (a deep match in tail position).
- They do not use the four words as names, the lowercase-type error, a lowercase or mixed type head, or, in a lone clause block, a first clause of D4's form (a `( … )` pattern whose clause expression starts with a function). A block that starts with a function whose parameters are not a pattern is fine, and so is any first clause of a clause block after `|>` whose `}` is not followed by `(` or `.`, so every example of §6.1 may be tested as written.
- One clause with choices: the judge adds one case written to the spec, `tests/spec/6/one_clause_choices.i` (`r |> { Ok(v) | Error(v) -> v }` gives `v`), and keeps it out of the merged run in `$C/pending/` with the reason "E0235 until T1" (D7). The T1 handoff names it as a case T1's gate must pass.
- A writer who does not know an error code writes `compile-error ?`; the judge fills it in after checking the message.
- The judge sorts each mismatch: implementation bug, test bug, or spec gap (to the coordinator, then the user). A mismatch on a temporary difference (D4, D7, §3.1) is expected and recorded.

## 7. Stages and gates

1. **S1**: `scripts/wat-diff`; `ast.rs`, both parser modes, the decision cache, the codes; `print.rs` new nodes; the syntax-keeping `format()` of §3.14; the spec text; `both.rs`, `parse.rs`, `golden_old/`, the new goldens; resolve's temporary arms (§3.3). Gate: E1; the golden rule of §6.1; `both_agrees_with_old`; full dev-verify green, including `cargo test --workspace` (so `format_properties` on the new spec blocks) and `i fmt --check`; `i test` and `wasi-smoke`.
2. **S2** (after the user's D7 decision): resolve §3.9-§3.11, `query.rs` §3.12, the feature, checker tests. Gate: E1; full dev-verify green; checker `.out` files unchanged except new tests and the listed message changes (E0301, E0302, E0308, and the E0202 hint of §3.10 step 4, at base in `projects/scope_rules/expected.out : 7` and `projects/type_rules/expected.out : 14`); any other changed `.out` fails the stage.
3. **S3**: `migrate.rs`, `format()`, `format_from_old`, splice, `i fmt` flags, the property, format `.out` with `UPDATE=1`, `allowed-a1.txt`. Gate: E1, E2, E3; `i test` (std, examples/hello, bench, O0 and O2) and `wasi-smoke` on the formatted copy. From here, only `i fmt --check lib std bench examples` in dev-verify may fail.
4. **S4**: blind cases merged and judged; E1, E2, E3 again; the corrections to the `syntax.md` notes (§10).

The checks use base for shared exports and the unformatted A1 source for added blind exports (§3.17). A blind case has no base counterpart; this is the reason for the S4 reference correction.
- **E1** (plan check 1): WAT of unformatted sources, normal build, `wat-diff --byte-equal`. Every shared file is byte-equal to base. List every file only on A1; each must come from a blind case.
- **E2**: in the rev lane, `i fmt --from-old` on every `.i` outside the A2 layout list, skip list, exclusion list (§8), and `one-clause-test` list of R7c. Blind cases are on the formatting exclusion list because they are already new; they remain present in both exports. Export both unformatted and formatted A1 with the feature on (§3.16), then run `wat-diff BASE FORMATTED --a1-unformatted UNFORMATTED --allowed FILE` with every category except `one-clause`. An ordinary diagnostic can prevent an export on both feature sides; a panic is always a bug. No blind case is excluded from checking or exporting.
- **E3** (plan check 2): the same formatted copy, normal build (feature off), `wat-diff BASE FORMATTED --a1-unformatted E1 --allowed FILE` with every category. Then the formatting is undone.

`i test` and `wasi-smoke` stay green at every stage, because the sources stay old and old heads and lowercase variables are still read (finding 9).

`i new` and `format()`: from stage 3, `i new` (`package.rs:147`) writes the new syntax from the old templates. Checked at base: `package_commands.rs:207-221` asserts only `contains("(Io) = i/io")` and `contains("main")`, which still hold; `:721` runs `i fmt --check` on the output, which passes because `format()` is idempotent; `ilang-lsp/tests/protocol.rs:334` compares with `format()` itself; `ilang-cli/tests/fmt.rs` uses text with no old forms. So no test beyond the one dev-verify check fails, and the templates stay for A2 partition 3.

## 8. A2 mechanics

- **Phase 0** (compiles):
  - `UPDATE=1` in `crates/ilang-cli/tests/spec.rs` rewrites only the `// at:` lines and the first line of `expect: stop "…"`; it keeps each file's line end (CRLF in `binding_text_crlf.i`), never touches lines after the header (the bare CR in `binding_text_cr.i` stays), and fails when a value has spaces at its start or end.
  - The Namer in `display.rs`: type variables `A`, `B`, …, error variables `E`, `E2`, …, environment variables `N`, `N2`, …, given in order of first appearance and skipping names that are types in the module. It no longer keeps written names: the "Written names first" pass of `assign` (`display.rs:1158`) is removed, as `syntax.md` says shown names "do not keep the names declared on the binding". So `get(K, V) = (d : Dict(K, V), key : K) …` shows as `(d : Dict(A, B), key : A) -> Option(B)`, an R4 fallback `A1` never shows, and old lowercase names never show next to new ones.
  - The interface printer in `crates/ilang-cli/src/commands/package.rs` (`TypeShow::param`, :1281, and its `TypeKind::Var` fallback), used by `i publish` and the interface diff: type-definition parameters and variables are named `A`, `B`, … in the same way, so it never prints `Box(a)`, which A3 makes an error. Its expected strings in `package_commands.rs` change in the same commit.
  - The §9.6 `// type:` comment of `docs/spec.md`, by hand, to match the new Namer (`crates/ilang-check/tests/types/spec.i:68`).
  - In the same commit, by hand: Rust test strings with shown types (finding 6): `check.rs` (`app/main.id : (x : a) -> a`), `ilang-check/tests/query.rs`, `ilang-lsp/tests/protocol.rs` hover, others found by a search for `-> a`, `(a)`, `? e ! n` and by running the tests; `builtin_head`; the checker `.out` files made again.
- **Phase M, tests whose meaning changes** (after phase 0, before the partitions; the `syntax.md` note "Tests whose meaning changes"): `scripts/a2-meaning` makes the `one-clause-test` list (R7c), and each entry is fixed by hand in the old syntax, so that the migration then sees a case of two or more clauses and keeps its meaning:
  - a case that must still not cover gets a second clause for a missing constructor with a guard that never holds, which the coverage check does not count: `case c { Red -> "red" }` in `tests/spec/7/not_covered.i` becomes `case c { Red -> "red"  Green if False -> "green" }`, which partition 1 writes as `c |> { Red -> "red"  Green ? False -> "green" }`, the text that `syntax.md` gives, so §7.4 keeps its one sample of E0516;
  - a case that must still test a type gets a second clause, so that `x: a` is still a test: `bad = (v) { case v { x: a -> x } }` in `types/case.i` becomes `bad = (v) { case v { x: a -> x  _ -> v } }`, which R4 writes as `bad(A) = (v) { v |> { x : A -> x  _ -> v } }`, still E0511;
  - a test whose point is the old one-clause rule itself is moved to a test of the `Assert` behavior, with a reason;
  - a one-clause case with choices in such a test follows D7 (R7b).

  Then `scripts/a2-meaning` runs again and must report an empty list. The changed expected messages (for example the constructors that are not covered) are checked by hand in this commit, and each entry is a `hand` line of the allowed list. The AG check of lost errors (below) makes sure no entry was missed.
- **The exclusion list** (`$C/a2/exclude.txt`, made by a command and recorded): sources that partitions 1-6 and E2 never rewrite.
  - Every `.i` file that A1 adds: `git diff --name-only --diff-filter=A dcb19df <A1 tip> -- '*.i'`. This takes in the frozen old inputs `crates/ilang-syntax/tests/golden_old/*.i` and `crates/ilang-syntax/tests/migrate/*.i` (they stay old until A3; `golden.rs`, `both.rs` (vi), and `tests/migrate.rs` need them old), and the inputs that are already new: the S1 goldens (`header_groups.i`, `clauses.i`, `type_params.i`, `attr_next_line.i`, `kept_forms.i`, and the new `err_*.i`), the S2 checker tests (`types/type_params.i`, `types/clauses.i`, `types/kept.i`, and the new project tests), and the S4 blind cases under `tests/spec/`.
  - Every ```` ```i ```` block that is already new: Both mode reads it with no diagnostic and Old mode does not. This takes in the new blocks of `docs/design/meta.md`, `docs/design/syntax.md`, this note, `docs/spec.md` (the replacement text of S1), and the design notes merged after base. Each skipped block is written in the report as `already-new`. A block that both modes reject goes by R10.
  - Any file listed by hand, each with a reason.
  A file that the command lists and the A2 list also names is an error, so the two lists never overlap.
- **Partition L, the library** (in the compiled lane `a`, after phase 0 and before AG; not an isolated worktree): `lib/prelude.i` and `lib/i/*.i` are compiled into `i` with `include_str!` (`crates/ilang-package/src/library.rs:8-10`), so by the plan only a build lane may change them. Lane `a` runs `i fmt --from-old --report` on them, changes their doc comments by hand (the `/// type:` comment of `lib/prelude.i` to the new Namer's text), rebuilds `i`, and runs full dev-verify. Its binary `$C/bin/i-<L sha>` is the one AG uses. Partitions 1-6 never touch `lib/`.
- **Partition 1**: `i fmt --from-old --report` on every `.i` outside `lib/`, the layout list, the skip list, and the exclusion list. The skip list is `editors/vscode/test/sample.i`, whose tokens `grammar.test.js` compares with `sample.i.snap` under the keyword tmLanguage; A3 rewrites it in the new syntax, with the grammar, the snapshot, and the test (§9). This partition rewrites `tests/browser/fixtures/compiled/src/app.i`, the source of the checked-in `tests/browser/fixtures/compiled.wasm`; the rebuild of that file is an AG step (below), because it needs the `i` of partition L.
- **Partition 2**: `i fmt --keep-layout --report` on the 14 `format/*.i` and the 48 `golden/*.i` with `--no-choices-split`, and without it on `binding_text_cr.i`, `binding_text_crlf.i`, and the parse-error samples of `tests/spec`; then the hand edits of the `syntax.md` list (err_module_path: `golden/err_module_path.i` and its `.out` are removed, since `(fail)` parses with no error and the case now lives in the checker test `projects/export_module_path` of §6.2; `golden_old/err_module_path.i` keeps the old input; err_case_brace, err_clause_tuple_type, err_pattern, err_segment_type, err_block_end line 3, err_toplevel lines 1, 3, 4, err_trailing lines 1 and 4, err_import_late, imports_empty) (finding 12). Each `format/*.i` still differs from its `.out`; the CR count does not change. `--no-choices-split` leaves out R7b: the `format/*.i` and `golden/*.i` files are only parsed and printed, never checked, so E0235 never applies, and `spec_inline.i:11` becomes `v |> { True | _ : Int -> 1 }` with the same clauses, which the golden view accepts. The other files of this partition are compiled, so R7b stays on for them.
- **Partitions 3-5**: the list of Rust files is made by a command and recorded, not taken from `syntax.md` (whose list misses files): `git grep -l -E 'import \(|export \(|case |\) -> [a-z]\b|: [a-z]\)' -- '*.rs'`, read by hand to drop the hits that hold no i code. At base it holds at least the `syntax.md` files and `crates/ilang-check/tests/library.rs`. In each file, plain Rust string literals are extracted, unescaped, migrated with `--keep-layout`, escaped again, and put back. `format!` templates are changed by hand, with a test that renders each template and parses it in Both mode with no diagnostic; the templates are found by a search for `format!(` whose string holds one of the patterns above, and at base they are `package.rs` (`i new`), `codegen.rs` (`many_points`), `wasi.rs`, and `library.rs:264`, `:279` (`format!("import (\n{imports})\n\nx = 1\n")`) and `:309` (`format!("import ({path})\n\nx = 1\n")`). Needles and hard-coded positions change with their source: in `library.rs`, `src/main.i:1:9` of `:309` becomes the column of the path in `({path})`, and `src/main.i:5:10` of the plain literal at `:294` (`"import (\n  i/task\n)\n…"`) is worked out again from the new text.
- **Partition 6**: each ```` ```i ```` block of the `*.md` files that is not on the exclusion list, with `--keep-layout`; the compiler messages of `pattern.rs : 734` and `resolve.rs:2600` by a search of message strings for the four words (finding 15). The rest is by hand from searches for `case `, ` if `, `import (`, `export (`, and lowercase type variables (`: a`, `(a)`, `-> a`, `? e`, `! n`), with these rules from the `syntax.md` note "What A2 changes":
  - Prose of `docs/spec.md`: a type variable in i code changes, such as `Result(a, e)` in the §4.2 table and the types in the §9.4 table (`Continuation(a, r, e, n)` becomes `Continuation(A, R, E, N)`). The notation of §0 in the rules of §4, such as `(x: a & Int) -> a` in §4.4, is not i code and stays lowercase. The §9.6 `// type:` line is already done in phase 0.
  - A bare function in prose that uses type variables and is not bound becomes the value of a binding that declares them: `(x : a) -> a { x }` in §7.3 (`docs/spec.md:670`) becomes `id(A) = (x : A) -> A { x }`.
  - Doc comments (`///`, `////`) and line comments of `.i` files, with the groups of §6.9: `case x < 3 { True -> … }` at `std/src/list.i:509` becomes `{ x < 3 } |> { True -> … }`.
  - i code in Rust doc and line comments (`//!`, `///`, `//`), found by the same searches over `*.rs`: for example `//! import ((Io) = i/io)` at `crates/ilang-host/src/services.rs:8` becomes `//! ((Io) = i/io)`.
  - Markdown outside ```` ```i ```` blocks: inline code spans that are i code, by the same rules.
- **AG**: merges phase M, the partitions, and partition L; checks that no file of the exclusion list changed (other than the hand-listed ones); adds the check that each `format/*.i` differs from its `.out`; makes `.out`, `// at:`, and checker files again; then checks that no error was lost: for every checker `.out` and `expected.out` and every `tests/spec` header, each diagnostic code that base expects and AG no longer expects must match a `hand` entry with a reason, or AG fails (the made-again files cannot then hide a lost error); rebuilds `tests/browser/fixtures/compiled.wasm` from the migrated `app.i` with `scripts/build-abi-fixture.sh` and the `i` of partition L, checks in the new file, and runs the browser fixture tests on it; records partitions 1 and 2 in `$C/a2/commands.sh`; checks that `sample.i` and `sample.i.snap` are unchanged; the vscode tests pass; the position view maps each old `// at:` position through the splice offset map and compares with the `UPDATE=1` result; full dev-verify green.

## 9. A3

Remove: the four keyword tokens, the `if` exception in `can_begin`, `no_brace`; Old mode, `parse_module_old`, `format_from_old`, `--from-old`, `--keep-layout`, `migrate.rs`, `golden_old/`, `both.rs`; the `Syntax::Old` paths in the parser, `print.rs`, and `format.rs`, and then `Case.syntax` and the header styles; E0113, and with it test 1a, place (1) of the `print.rs` group rule and of `unprintable` (§3.13), and the `if` token of test 2 (§3.5); E0236 with old lowercase variables and heads, `push_ty_frame`, `annotation_vars`, `type_vars`, `pattern_annotations`, `expr_annotations`; the feature `one-clause-as-case`; the old `golden_old/` and `tests/migrate/` inputs; the keyword completions at `query.rs:971` and the words "and the keywords" in the doc comment at `query.rs:673`; `is_name` in `ilang-package/src/lib.rs:180` and "or a keyword" in `package.rs`.

Change:
- `tests/lex.rs:19-24` (finding 16), `package_modules/src/if.i` and its `expected.out`, `manifest.rs`, `package_commands.rs` (other bad names, as the `syntax.md` A3 note says).
- The tests that expect `case` among the completions: `crates/ilang-check/tests/query.rs:314` (the list `["panic", …, "case"]` and its comment "and keywords") and `crates/ilang-lsp/tests/protocol.rs:272` drop `case`, and each asserts that `case` is no longer offered.
- The editor grammar, which is updated, not removed: `editors/vscode/syntaxes/i.tmLanguage.json` loses the keyword scopes of `case`, `if`, `import`, and `export`, and gets scopes for the new forms: a header group (the `(` … `)` at the start of a file that is not followed by `=`, with the `/` of a module path), the `?` of a guard, the `|>` before a clause block, and the declared `UpName` parameters of a binding head `name(A, B)`. `editors/vscode/test/sample.i` is rewritten by hand in the new syntax with the same coverage of forms, `sample.i.snap` is made again from it, and `grammar.test.js:109-111` (the `has("import" | "export" | "case" | "if", "keyword.control…")` lines) is replaced by assertions on the new scopes. `grammar.test.js` stays, so the grammar is still tested after A3, and A3's check that no tracked file is in the old syntax then passes on `sample.i`.

Add the error for a lowercase name in a type, with a new code, in resolve's type-name lookup. A3 does not remove E0235 (D7): that is T1's, because only T1 can make one clause with choices a binding.

Source at old tags (the `syntax.md` A3 note of that name). A published version is a git tag, which the migration cannot change, and after A3 nothing reads the old syntax. A3 adds:
- `ilang_syntax::old_syntax_marks(src) -> bool`, a scan of the tokens with no parse, which A3 keeps: true when a top-level line starts with the name `import` or `export` followed by `(` on the same line, or when a name `case` is followed on the same line, before any break, by a `{` whose first statement has a `->` at depth 0. These are the old header and the old match; every old file with imports, exports, or a match has one.
- `i build` and `i check`: when a source of a dependency (a package in the graph that is not the root) has diagnostics and `old_syntax_marks` is true for it, the diagnostics are reported as for any other file, and then one note per package: "this version of `<package>` (`<tag>`) is written in the syntax from before <the A3 version of i>; it cannot be used with this compiler; use a newer version of the package, or an older version of `i`". The dependency is not changed or moved. The place is the code in `ilang-cli` that prints the diagnostics of a build, which knows the package of each source.
- `i publish`: in `earlier_interface` (`crates/ilang-cli/src/commands/package.rs`), when the source of the newest earlier tag has errors and every file with an error has `old_syntax_marks`, `i publish` prints "the interface of `<tag>` is in the syntax from before <the A3 version of i> and cannot be compared; check the interface by hand", and goes on as when that tag has no package ("nothing to compare with"), so `compare_with` is not run and no new major version is asked for (§2.7). When some file with an error has no marks, the error is reported as today.
- Two tests in `crates/ilang-cli/tests/package_commands.rs`: `i publish` against an earlier tag in the old syntax expects the message and a published patch version; `i build` with a dependency at such a tag expects the parse errors and the note. Their old-syntax sources are written in the test as plain strings, since A3 has no old reader.

### A3 corrections from the coordinator

The spec wins over the old-form error requirement in the plan: §1.4 says
"any `name` may be used as a name", and §2.2 reads top-level calls in full as
forms kept for macros. An old spelling that is also valid new syntax keeps
that grammar. An unambiguous old form gets a parse error with a migration
hint; an overlapping call gets a hint on its ordinary compile error. The
four former words have no parser exception. Lowercase names in types are
read as value-type forms and get E0237 until T2, as §3 requires.

The repository tool is `scripts/old-syntax-check`. It parses every extracted
snippet. Deliberately invalid fixtures document their expected diagnostic
codes beside the source; the tool's data records and checks the exact ordered
codes and messages. Accepted examples of later syntax stay in the design
notes. Their data entries name the owning track and the current diagnostic;
the check fails when an entry starts to parse or its diagnostic changes, so
that its owner must remove it when that syntax lands. This corrects
the plan's unconditional "all must parse" requirement, which cannot apply to
negative parser fixtures or accepted later-track examples.

## 10. Corrections to the `syntax.md` notes (committed at S4)

- A match is `Case { syntax : New }`, built by the parser (D5).
- The made-up `Bind` has the span of `p`; `lower.rs` does not change; `query.rs` `Scope::block` uses `cover()` (D6).
- One clause with choices: the user's D7 decision.
- Known constructors: R7a refuses the file to the hand list when a known constructor is at stake; no renaming of uses to `c`, and no `s = p` rewrite (D13).
- Shown names: the Namer and the interface printer of `package.rs` change in A2 phase 0 and no longer keep written names (§8).
- `format()` reads Both mode only, with no Old fallback (D10).
- Kept forms: records are a parse error, the rest checker errors (D9).
- `err_trailing` line 1 expects "a type", not "an `UpName`"; `@custom()` moves to the checker.
- The A3 list gains: E0113; E0236; the old `if` line rule and the old `case` `{` rule (already there); `sample.i` moves to A3.
- The list of edits outside A1's ranges is §5 of this note; `lower.rs` and `infer.rs` leave it.
- R4: a type variable whose owner pattern is not one name is never declared "on the nearest binding of one name around it". The binding is split into one binding per name when that keeps generalization, and otherwise the file goes to the hand list (`tyvar-owner`). The clash rule also counts `UpName` variables that the file already declares.
- Lone clause blocks: the `Function` has the span of the clause block; the made-up `case` node has the span from the first clause to the last (zero-width after the `{` for `{}`); the `%arg` binder and name have zero-width spans at the `{`. The label of the parameter is `None`, set by resolve after the function code runs (§3.9 (c)), as the note says.
- The §4.3 head-form errors are reported by resolve (E0232 for a custom type, E0233 for an alias), not by the parser: the parser reads any head into `head` (§3.7, §3.11).
- E0113 applies only to a lone clause block and to a clause block after `|>` whose `}` is followed by `(` or `.`; any other clause block after `|>` is decided by the spec tests alone (D4).
- Claim D has the J1 exception (D3); `format()` and `i check` give the `--from-old` hint on parse errors when Old mode reads the file (D10).
- `format()` refuses a file that the plan sends to the hand list (§3.14).
- The library files are migrated in a compiled lane, not in partition 1 (§8); the A2 exclusion list (§8).
- E0113 (D4) is also made for a clause block after `|>` whose `}` is followed by `.`, since Old mode then reads a postfix chain that can end in a call.
- Statements of a block: `stmt()` reads `name(UpName, …) =` for every statement, not only the first (§3.5).
- R4 splits a binding only when its whole value is direct; "generalization does not change" holds only then (R4).
- R2 adds a group (R2c) around a match that is the left operand of any binary operator, to keep the number of stop sites; `wat-diff` works by owners and renames funcs, because a function that may suspend has `$s`, `$r`, and `$j` funcs, and top-level values are in `$start` (§3.17).
- `print.rs` adds the groups of the binding rules where the tree has none, as the note says, but only in the exact place where the parser would read the text in another way: the first-clause group only for a `Clauses` node whose first pattern starts with `(` (until A3) and for a first pattern that is a chain `n1 = … = ( … )`; never for a match after `|>` with a `( … )` pattern. A tree from the parser with no diagnostic never meets a rule (D15, §3.13).
- R6, and the `g = F` binding of the note, apply only where F has no expected type. The note's chain case `p = (a, b) -> (u) -> String { u.name }` loses the expected type with a bare `g = F`, so when F has a `->` part and is the whole clause expression, the file goes to the hand list (`fn-expected-type`), and the hand edit writes the type on the name. R5 binds F only when F is a callee, a match subject, an operand of a binary operator other than `|>`, the target of `.f` or `.k`, or a statement that is not the last; the D4 message suggests `g : T = F` (R5, R6, D4).
- `print.rs` never adds a `g` binding; a core tree with no same-code text is found by `print::unprintable` (§3.13).
- A type variable with no owner (E0222 at base) is renamed and declared nowhere; the file gets a `hand` entry `tyvar-unowned` for E0222 → E0202 (R4).
- R8 also puts a line end before a statement that starts with `{` (R8).
- In Both mode the E0111 message for a right side of `|>` that is not allowed names the clause block, as the new §6.9 sentence does (§3.6).
- `err_module_path`: the checker change is the E0215 hint in `resolve.rs:784` at S2, with the test `projects/export_module_path`; the golden is removed in A2 partition 2 (§5, §6.2, §8).
- `wat-diff`: constructor and library funcs with no comment, the `$c` globals, the `(elem declare func …)` line, and the `pattern <source> did not match ` strings (category `pattern-text`) (§3.17).
- The A3 notes gain: the `case` completions in `crates/ilang-check/tests/query.rs:314` and `crates/ilang-lsp/tests/protocol.rs:272`; the editor grammar is updated, with new scopes, `sample.i` rewritten, its snapshot made again, and `grammar.test.js` kept (§9).

The `docs/design/meta.md` §1 correction is handed to its owner, because meta.md is outside A1. Its claim that every other core tree has a same-code normal surface must include all `print::unprintable` cases of §3.13: a block that begins an expression and starts with an unbound function with a `->` part and only patterns as parameters; a first clause with one pattern and no guard, whose pattern is a chain `n1 = … = ( … )` and whose expression starts with such a function; a non-final choice whose pattern is a type annotation after any `name =` chain; and, until A3, the corresponding lone-clause-block case whose first pattern starts with `(` (E0113). The reason is §6.1's first-statement decision and §7.1's longest-type rule. A same-code printer must report an error rather than change the program. The final handoff quotes the exact old and proposed replacement sentences.
- Tests whose meaning changes: the second clause is added by hand in the old syntax before the migration (A2 phase M), from a list made by machine (R7c); AG fails on any expected error that goes away without a `hand` entry. `coverage.i`'s `case r { Ok(x) -> x }` cases cover and keep their meaning.
- The rebuild of `compiled.wasm` is an AG step (§8).
- The Rust files with i source are found by a search, which adds `crates/ilang-check/tests/library.rs` (three `format!` templates and a plain literal with positions) to the `syntax.md` list; i code in Rust doc comments (`crates/ilang-host/src/services.rs:8`) is changed by hand (§8).
- Source at old tags: `old_syntax_marks`, the note in `i build` and `i check`, and the `i publish` path, with the two tests (§9).

## 11. Findings of `a0-round8`, one by one

1. The two old §1.7 rules: §3.1, D1, the A3 list.
2. The opening's exceptions: in the text as written; the blind matrix tests them.
3. Two errors for one head: §3.11, in a fixed order, both in resolve: E0232 or E0233 for the form, then E0231 or E0233 for the name of a type. The parser only reads the head (§3.7).
4. `p.x` with a value `p`: §3.7, `ValueOf` whatever `p` is.
5. §9.6 `// type:`: A2 phase 0, with the Namer.
6. Rust tests with shown types: A2 phase 0.
7. Stop position: D6 (span `p.span`, no `lower.rs` change).
8. Declaration frames: §3.10, `values()`.
9. Old heads: §3.7, kept until A3.
10. Old blocks with a function: D2, D4 (with the pattern condition of test 1a), D10 (no Old fallback), R5, R6.
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
- Claim D is tested, not proved. A missed overlap shows only if the corpus, the mutants, or the ambiguity hunter reach it. The migration reads Old mode (`--from-old`), and the write check compares that tree with what Both mode reads from the output, so a miss cannot change a migrated file. A miss could still make plain `i fmt` on an unmigrated old file print a different program; only the unmerged A1 branch has such files, and A2 uses `--from-old`.
- The `{` decision rests on speculative parses of patterns, which can hold expressions with `{`. A rollback bug would be fixed in place by the cache. `both.rs` (v) and the nested-brace test cover this.
- Allowed mode skips all funcs of a listed owner (its `$f`, `$s`, `$r`, `$j`, and its closures). E2 narrows this to the categories other than `one-clause`, but a regression inside a function with a `g-binding`, `split`, or `hand` entry stays hidden in both E2 and E3.
- The R4 owner rule copies resolve's rule in the syntax crate. The cross-crate test covers only the corpus.
- R7a is chosen from syntax alone. A one-clause case whose meaning depended on narrowing by a type test may still change; the checker `.out` files and `i test` must show it.
- R4 and R7a send some files to the hand list. Their number is known only when the S3 report runs; the scan in this design found none for R7a.
- The site-count argument assumes a covering old case gives a pattern that infer does not mark may-fail. The machine report checks it; if it fails, `--renumber` is needed, which depends on the site forms of `emit` at base.
- From stage 3, `format()` on old files depends on the plan. The LSP formats old files into the new syntax on the A1 branch. A1 is not merged alone, so no user sees it.
- J1 makes Both mode read a few correct old files differently (a type that ends an item with a lowercase name, followed on the same line by a tuple-pattern binding). Such a file always gets a diagnostic, and `format()` refuses it, but `i check` on the A1 branch rejects a program that base accepts. The corpus has none (§6.1).
- Error codes E0113-E0115 and E0229-E0236 may clash with codes that T1, T2, B, or C choose. The coordinator reserves them.
- The feature could leak through feature unification in a workspace build; the guard test catches it in `cargo test`, and the release assertion in release builds.

## 13. Review items of this design, and where they are settled

- E0113 rules, corpus, A3 list, approval, message, blind writers: D4, §6.4, §10, §14.
- Allowed-list categories for g-bindings and groups: §3.17.
- Closures and `$ilang_start` in allowed mode: §3.17.
- Generalization kept by R4 (no moves) and R7a (no rebinding): R4, D13.
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
- Plain `i fmt` on an E0113 file: D10 (no fallback; the error names `--from-old`).
- Who removes E0235: T1 (D7, §14 item 8).
- Kept forms split and wording: D9, §3.11.
- Spec test 3 for a statement that starts with `name =`: §3.5 test 3.
- Clashes of R4 with declared `UpName` variables: R4, the capture test of §6.2.
- Correct old files that Both mode reads differently (J1): D3, D10, §3.7, §6.1 (vii).
- No `%arg` label: §3.9 (c), §6.2.
- `format()` on a file the plan refuses, and `format_skip.txt`: §3.14, §6.3.
- `canon` in library code: §3.15 (`shape.rs`).
- Top-level splits and `$fN` closures in `wat-diff`: §3.17.
- The `|>` example of §6.1 and E0113: D4, §3.5 test 1a, §6.4.
- The E0113 fix for a function with a `->` part: D4 message.
- E0302 wording for each attribute, and the E0202 hint: §3.11, §3.10, §7 (S2 gate).
- The A2 exclusion list and the library lane: §8.
- Tests for §4.8, §5, §6.10, §10.2, §11.4, the guard word, and the tuple-type note: §6.1, §6.2, §6.4.
- R7b on parser-only goldens: D7, §8 partition 2.
- Test 1a after `|>` with a postfix chain (`}.0(1)`, `}.g(1)`): D4, §3.5, §6.1 (iv).
- R4 splits of a value that is not direct: R4, the capture test of §6.2.
- The WAT model of allowed mode (`$s`, `$r`, `$j`, `$start`): §3.17.
- Sites that merge after R2: R2c, §3.17.
- Code that must change for `TypeDef.head` and `ty_frames`: §3.3.
- Tests whose meaning changes: R7c, §8 phase M and AG.
- Groups for core trees with no groups: D15, §3.13.
- `compiled.wasm`: §8 partition 1 and AG.
- Source at old tags: §9.
- Rust files with i source, `library.rs`: §8 partitions 3-5.
- Prose rules and Rust doc comments: §8 partition 6.
- `stmt()` for later statements: §3.5, §6.1 (vii).
- `g = F` takes the expected type away; R6 limited to chain patterns: R5, R6, D4, §6.2.
- `print.rs` groups only where the parser needs them, strict round trip, no invented bindings, `meta.md` §1: D15, §3.13, §10.
- `Ctor` and `Runtime` funcs, `$c` globals, and the `elem declare` line in `wat-diff`: §3.17.
- Pattern text in WAT strings and stop messages: §3.17 (`pattern-text`), R12.
- The guard word in `v |> { x if c -> 1 }`: §3.5 test 2, §6.1.
- One E0115 for `err_attribute_line`: §3.4 rule 2, §6.1.
- Type variables with no owner (E0222): R4.
- The module-path hint of E0215: §5, §6.2, §8 partition 2.
- Statements that start with `{` in splice mode: R8.
- The E0111 message: §3.6, §6.1.
- A3 editor grammar and completion tests: §9.

## 14. Open questions

For the user:
1. Approve E0113 DualAmbiguous (D4) as a temporary rule, until A3, that `syntax.md` does not have.
2. Choose for one clause with choices (D7): E0235 until T1 with the migration splitting such clauses (default), or the old `case` node with E0516 and a known constructor until T1. Needed before stage 2.

For the coordinator:
3. Add the edits of §5 "outside A1's ranges" to the split of files, and tell T1 and T2 to rebase on them.
4. Accept the test-only feature `one-clause-as-case` in `ilang-check` (D14).
5. Accept the allowed-list categories beyond the plan's one (`choices-split`, `g-binding`, `split`, `pattern-text`, `hand`).
6. Reserve the error codes E0113-E0115 and E0229-E0236.
7. Confirm that the LSP formatting old files into the new syntax on the unmerged A1 branch is acceptable.
8. Add to T1's stages: one clause with choices as a binding (§6.12, §4.8 fitting choices), removing E0235 or the D7 alternative, with the pending case `tests/spec/6/one_clause_choices.i` as its gate. Tell T1 this is a task, not only a rebase.

## 15. S4 judgments and corrections

The frozen writer's cases and source sentences are in
`tests/spec/6/a1-blind-judgments.md`. The writer saw only syntax.md and the header
format before freezing 38 cases. The coordinator's S4 ruling corrects two test
bugs against the spec, leaving 37 active cases and one pending T1 case (D7).
The original frozen files remain in audit artifacts. Diagnostic codes were filled
only after checking the messages against the writer's source sentences.

Two statements in §6.2 above are corrected from the earlier note:

- Syntax.md §11.4 says "each time a clause block is worked out, it makes a new
  value, which is the same only as itself." Function equality is identity, not
  a compile error. The blind recursion and identity case checks this rule.
- Syntax.md §10.2 edits the host-export bullet of spec §10.2. "They must be in
  the exports of their module" applies to host exports. The program-entry
  bullet instead says "the selected entry function in the package that is
  compiled" and does not require an export. The private-main case now expects
  success, with the same program, as the coordinator's S4 ruling requires.

The other erroneous blind case calls `assert` inside `outer(A) = (x : A) -> A`.
Spec §6.8 says a missing `!` in a written `->` part means `Never`, and §9.6 gives
`assert` the environment `Assert`. E0501 was correct. The case now adds `! Assert`
to that return part, keeping its success expectation and isolating the intended
scope rule. This is the second source-based correction accepted in the S4 ruling.

Two S3 refinements also follow the fixed language text. The first-clause function
group applies only with one pattern and no guard (§6.1 test 2 requires the arrow
immediately after the pattern). R2c groups old cases only: a new one-clause match
can give its subject an expected type (§6.12), so extracting its function subject
would violate D12. Mixed reserved type heads whose lowercase parts would become
valid parameters after renaming are refused as `tyvar-owner`, preserving their
existing error rather than inventing a definition (§4.3 and R10).
