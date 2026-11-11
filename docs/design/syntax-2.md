# Syntax 2: feasibility of the six proposed surface changes

Status: accepted for SYN2 by the user on 2026-11-11. The coordinator rulings in the SYN2 scope settle §10. The conditional wording below records the feasibility study; the installed language text is docs/spec.md. Old forms remain accepted during the coordinated lane migration.

Source baseline: `d655684bdb809c5a38bf86b61fc5ef26f5bc5392`, branch
`lane/note-syn2`. All unqualified i code citations below are at that commit.
Reference paths starting with a project name are under `.rd/ref/` in the main
checkout. Experiments are under this lane's `target/syn2/`; only this note is
committed. Prospective examples use the proposed surface, in `text` fences so
that the current old-syntax checker does not mistake a proposal for accepted
source. Old spellings appear only where identifying a replacement.

## 1. Result and limits

All six mechanisms can be parsed with the existing parser architecture.
They cannot all be described as a semantics-preserving surface migration:
removing the empty tuple removes an existing value, type and pattern. The
proposal also needs the boundary decisions in §10 before its spec is exact.
No new type inference algorithm, library privilege, keyword, or runtime
matching mechanism is needed for the other changes.

| Change | Verdict | Evidence and amendment needed |
|---|---|---|
| 1. Postfix match | Buildable as stated, with the existing number lexer cost | The prototype produces the existing `Case` node. Names, strings, calls, tuples, groups and line-leading dots work. A decimal integer immediately followed by `.{` fails: its dot belongs to the float token. All binary subjects, including levels 7 and 8, need grouping when the whole operation is the subject. |
| 2. Bare pipe | Buildable with an explicit brace-classification rule | `{ x \| f }` must be a block. The old classifier says clause block. Require a completed alternatives head, followed by `?` or `->`, before choosing clauses at a bare `\|`. Preserve longest-type parsing. Decide whether a literal clause block remains a pipe target. |
| 3. Error marker `^` | Buildable as stated | Move only the error-slot token; keep `?` for guards. The formerly ambiguous function-type guard parses correctly. Nearest-arrow attachment and error-before-environment order are unchanged. |
| 4. Parenthesised records | Buildable with the retained context rules stated explicitly | Look past the matching `)` before reading its contents. Existing call, pattern, header and guard contexts take precedence. Keep brace type groups; “braces hold only code” cannot literally cover type syntax. No comma distinguishes a function from a record. |
| 5. Positional nonempty tuples | Buildable with an explicit empty-pattern decision | Mixed data is rejected; mixed call arguments and constructor fields still parse. Reject the old empty tuple pattern if there are no record patterns. This removes a value and is not just spelling. Confirm the mixed-call/constructor exception. |
| 6. Empty record and `Record` | Buildable with ordinary name resolution | `()` is a record value, while empty parameter lists still work. `()` in a type is rejected with a `Record` hint. `Record` names the set of all structural records, not the singleton empty record. Preserve shadowing. Correct the claimed exceptions for bare `..`. |

The combined prototype parsed all 3,548 inputs extracted by the repository's
snippet extractor. Of 3,196 accepted by the baseline parser, 3,186 have exactly
the same canonical AST after mechanical migration. Seven inputs now have empty
record values instead of empty tuple values. Three reject an empty tuple type
or pattern. There are no other tree differences or new rejections. All 18
fenced `i` snippets in `docs/spec.md` retain their canonical trees.

This is a parser result, not a type-check, execution, formatter or WAT result.
The prototype does not implement T3 checking or lowering. The other 352 inputs
already fail baseline parsing; they include intentional syntax errors,
future-track examples and extracted host-language fragments. It would be
incorrect to call those successful migrations. §7 gives counts and §8 states
what a production migration must still do.

## 2. Controlling sources and alternatives

Read in the required order: spec §§1–7 and the affected effect/runtime clauses;
`syntax.md`, especially its replacement §§1.7, 3, 6 and 7; records §§7–8,
12–16 and the accepted amendments; meta §§1–3; the plan's “已定的设计” and
A1/A2 track; then the ledger. The spec at this base already uses the first
migration's syntax (`spec.md:97`, `:147`, `:824`). The old migration status
paragraph in `syntax.md:3` is historical, not evidence of an old parser.

The lane's `records.md` is older than the accepted T3 contract. I read
`git show 0d0ce19:docs/design/records.md`, the revision accepted by the ledger,
into `target/syn2/records-accepted.md`. Citations written “records@0d0ce19”
refer to that revision, including §§20–21. Its §8.1 is at :492, §8.2 at :530,
and §21 at :1932. The syntax change does not revive its superseded eager
normalisation or common-field union rules. Exact record sets, open records,
Without, label parameters, U3/U4, invariant sharing and the 65,536-state budget
remain that note's rules (records@0d0ce19:38, :530, :1978, :2135).

I compared three designs before choosing the prototype's approach:

| Design | Benefit | Cost against this task |
|---|---|---|
| A. Replace tokens and delimiters, leaving every parser decision alone | Small initial patch | Wrong for `{ x \| f }`; blindly grouping match subjects breaks a typed function at the start of a block; cannot migrate empty tuple semantics. Rejected. |
| B. Contextual recursive descent with matching-delimiter lookahead; preserve AST meanings | Uses the existing parser and reference methods; diagnostics can identify the actual context; no solver involvement | Requires a revised brace probe, separate datum/type/pattern rules, and an explicit migration for removed tuples. Chosen provisionally. |
| C. Parse a general parenthesised datum first, defer all interpretation to checking | One broad concrete tree could preserve incomplete editor input | Type-dependent interpretation would violate the required syntax-only decision and threaten principal inference. A syntax-only second pass is possible but duplicates the current AST boundary and every consumer. Not justified for six surface changes. |

Criteria were deterministic parsing without symbol tables, no comma used to
settle ambiguity, unchanged set meanings and principal inference, preservation
of source evaluation order, predictable errors, bounded lookahead, and an
AST-based migration with an auditable exception list. B keeps A's reuse of
`Case`, `Binary(Pipe)` and function effect slots. From C it keeps source spans
and explicit context, not deferred type-dependent disambiguation.

These are syntax mechanisms used across unrelated domains: matching protocol
replies, numeric branches and tree traversals; pipes in collection transforms,
codec builders and service composition; records in configuration, decoded data
and compiler metadata; error annotations in parsers, IO and validation. Their
sets and inference are the existing ones. Deleting the empty tuple is the one
change to the value universe; it needs explicit approval as such.

## 3. Prior art read before the prototype

The reference index and manifest were read first. The following are source
findings, not claims based on language familiarity. No reference was built,
modified, or copied into i. Additional source files absent from the library
were downloaded into this lane's `target/syn2/reference/`, not into another
lane or the shared read-only library. Those supplementary files are moving
upstream snapshots read on 2026-11-11, not pinned historical evidence.

### Postfix matching and parenthesis decisions

- **Scala 3.** `scala3/compiler/src/dotty/tools/dotc/parsing/Parsers.scala`,
  `selectorOrMatch` (:1321), `matchClause` (:2864), `simpleExprRest` (:3060):
  dot selection can produce a match and return to the same suffix loop. The
  suffix path also diagnoses ambiguity after negative numeric literals. Take
  the suffix-loop method and the attention to negative literals. Leave its
  `match` keyword, indentation grammar and Scala-specific patch suggestions.
- **Rust.** `rust/compiler/rustc_parse/src/parser/expr.rs`,
  `parse_dot_suffix` (:1182–1196), gates `postfix_match` and calls the ordinary
  match-block parser with `MatchKind::Postfix`. This is implemented experimental
  syntax, not a stable-language precedent. The checked `rust-rfcs/text` tree
  has no merged postfix-match RFC text; the [unstable-book entry](https://doc.rust-lang.org/nightly/unstable-book/language-features/postfix-match.html)
  is the primary online status reference. Take reuse of the existing match
  representation. i uses `.{` because it has no keywords, and retains its own
  one-clause binding and tail rules.
- **Dart 3.** `dart-sdk/pkg/_fe_analyzer_shared/lib/src/parser/parser_impl.dart`,
  `parseParenthesizedExpressionFunctionLiteralOrRecordLiteral` (:8251), looks
  at `endGroup.next` for an arrow/body before parsing contents.
  `parseParenthesizedExpressionOrRecordLiteral` (:8315) recognizes named
  fields and zero fields, and diagnoses a lone positional field without a
  comma; `parseRecordType` (:1750) has matching type diagnostics. Start with
  this known suffix-lookahead solution. i already has `close[]` and already
  treats `(x)` as a one-item tuple, so Dart's grouping/one-field comma problem
  does not arise. Do not copy its comma disambiguation, mixed record shapes,
  nominal `Record` class machinery or field-name exclusions.
- **Swift.** `swift/lib/Parse/ParseType.cpp`, `parseTypeTupleBody` (:1286), reads
  labels and then uses an arrow/effect suffix to decide the function context
  (:1393). Its error paths handle a second label and misplaced parameter
  modifiers. Take explicit context-specific validation; leave Swift's labelled
  tuple identity, parameter modifiers and keyword effects.
- **C#.** Supplementary [Roslyn LanguageParser.cs](https://github.com/dotnet/roslyn/blob/main/src/Compilers/CSharp/Portable/Parser/LanguageParser.cs),
  `ParseTupleType` (:7923), `ParseTupleElement` (:7957),
  `ParseTupleExpressionTail` (:12885), distinguishes a named tuple part from
  a grouping/cast and reports `ERR_TupleTooFewElements`. Its minimum is two
  and its expression parser needs cast lookahead. Take early arity and mixed
  context diagnostics; neither restriction is i's rule. One-item tuples are
  already unambiguous in i.

### Pipes, alternatives and unions

- **Gleam.** `gleam/compiler-core/src/parse/token.rs:77` has a distinct `Pipe`
  token for the two-character operator. `parse.rs`, `parse_case_clause`
  (:1731), loops on `Vbar` for alternatives, then reads the guard, arrow and
  expression. Take this separation of head and body. i can share a token only
  after fixing its brace probe; Gleam's distinct token avoids that cost.
- **Elixir.** `elixir/lib/elixir/src/elixir_parser.yrl:75,84` gives bare bar and
  arrow operators separate precedence; :206–209 explicitly warns about pipe
  precedence with calls without parentheses. Take the dedicated diagnostic
  approach. i's required call parentheses avoid that particular ambiguity,
  but a bare bar still needs the context rules in §4.2.
- **OCaml.** `ocaml/stdlib/stdlib.ml:48` declares the two-character pipe as
  `%revapply`. It is ordinary reverse application, not shell stream wiring.
  i retains its existing first-argument insertion and evaluation order rather
  than importing OCaml's operator implementation or all of its precedence.
- **F#.** Supplementary [pars.fsy](https://github.com/dotnet/fsharp/blob/main/src/Compiler/pars.fsy)
  separates `BAR` (:91, :267) from `INFIX_BAR_OP` (:53, :360, :4770).
  This is evidence of a grammar separation, not proof of the historical
  reason for choosing the glyph. Do not claim that any of these projects
  proves bare bar impossible.
- **Bash.** Supplementary [parse.y](https://github.com/gitGNU/gnu_bash/blob/master/parse.y),
  `pattern` (:1060) and `pipeline` (:1256), uses bare bar for both alternatives
  and pipelines, in distinct grammar productions. Take context separation.
  Its command boundaries and `case` syntax differ from i, and it has no i-like
  type union syntax. Process creation and streams are not i pipe semantics.
- **Nushell.** Supplementary [lite_parser.rs](https://github.com/nushell/nushell/blob/main/crates/nu-parser/src/lite_parser.rs)
  handles `Pipe` (:451), continuation after a pipe (:455) and a specific
  `ShellOrOr` diagnostic (:295, :361). Its [type-signature documentation](https://www.nushell.sh/lang-guide/chapters/types/type_signatures.html)
  writes unions with `oneof`; `parse_calls.rs`, `parse_oneof` (:759), orders
  alternative shape parsers and postpones string fallback. Take the explicit
  pipe/logical-or diagnostic, not type-driven expression parsing.

I found no verified reference in these reads that combines all three of i's
proposed uses with i's keyword-free clause blocks and longest-type rule.
That is a limit of this search, not a claim that no such language exists.
The concrete trouble here is established by i's parser experiment, not by an
invented history of another language. Distinct pipe tokens in the ML-family
references remove one ambiguity; they do not dictate i's choice.

### Named record top and error slots

- **Dart.** `dart-sdk/sdk/lib/core/record.dart:7–16,58–69` describes `Record` as
  the supertype of record shapes and records field-name conflicts with Object
  members and positional getters. Take the named set of record values.
  i has no Object methods on records, and its labels remain ordinary strings;
  no Dart exclusions are needed.
- **TypeScript.** `typescript-5.9.3/src/compiler/checker.ts:2101` creates the
  intrinsic nonprimitive `object`; :6827–6829 prints it as an ObjectKeyword.
  Take the distinction between a top name and an empty value. Leave its
  broader nonprimitive set and keyword status: i's `Record` excludes tuples,
  functions and nominal constructor values.
- **Zig.** `zig/lib/std/zig/Parse.zig`, `parseErrorUnionExpr` (:2336), consumes
  a suffix expression, an optional bang, and a type expression. Take a
  dedicated effect/error slot rather than overload the guard marker. i cannot
  copy bang because bang already identifies its independent environment axis.
- **Swift typed throws.** `swift/lib/Parse/ParseExpr.cpp:132–157` parses effect
  specifiers around function arrows; `ParseStmt.cpp`, `parseStmtDo`
  (:2177–2201), parses the thrown type inside a matching parenthesis pair.
  Take explicit effect boundaries and misplaced-marker errors; do not add
  a keyword or change i's error/service separation.
- **Koka.** `koka/src/Syntax/Parse.hs`, `teffect`, `textend`, `tlabel`
  (:2616–2640), delimit effect rows and reject a variable in the label portion
  with an actionable message. **Effekt**, `effekt/effekt/shared/src/main/scala/effekt/Parser.scala`,
  `effects` and `maybeEffects` (:1588–1604), delimit effects after slash.
  Take syntactic slot separation and errors tied to the offending slot.
  Leave their row spellings: this task does not combine i's two effect axes.

Thus the proposed implementation is the Scala/Rust suffix loop plus the
Dart/Swift matching-parenthesis decision, with Gleam's head/body split adapted
to a shared bar token. The departures follow i's existing tuples, keyword-free
blocks, set types and independent error/environment slots. There was no
cross-language performance measurement; none is implied by these readings.

## 4. Exact proposed rules and collisions

The rules in this section are conditional recommendations for confirmation.
They do not alter the ledger's approval status.

### 4.1 Match suffix

**M1.** Add `"." Clauses` to the postfix loop. After that dot the braces must
contain clauses, including zero clauses; they are not a generic code block.
Construct the existing `ExprKind::Case { subject, clauses }`. A single
unguarded clause keeps the binding/Assert rule, including choices. The result
can take another call, field, tuple projection, or match suffix.

Reason and code: `parser.rs:977–1027` already turns the old pipe/clause pair
into `Case`; `:1044–1072` is the existing suffix loop. The resolver's one-clause
path is `resolve.rs:2515`, its other-case path :2547; standalone clause blocks
use `clause_function` at :2514. `lower.rs:1580` lowers `Case`. Moving construction
of that same AST node preserves these paths; it does not add a call frame.

**M2.** Postfix binds above unary, above every binary level. Consequently:

| Source | Meaning |
|---|---|
| `n.{ 0 -> 1 _ -> 2 } * 10` | Multiply the match result. |
| `1 + n.{ 0 -> 1 _ -> 2 }` | Add to the match of `n`. |
| `{ 1 + n }.{ 0 -> 1 _ -> 2 }` | Match the sum. |
| `-n.{ 0 -> 1 _ -> 2 }` | Negate the match result. |
| `{ -n }.{ 0 -> 1 _ -> 2 }` | Match negative `n`. |
| `{ a == b }.{ True -> 1 False -> 0 }` | Match the comparison. |

The cost is not limited to levels 1–5: addition, multiplication and unary
subjects also need groups. Their operator trees used to lie to the left of
level-6 matching. The migration must read the AST, not replace text globally.

**M3.** Retain the number lexer unless the user explicitly chooses a new
exception. `1.{ ... }` starts with Float `1.` and fails as a match. `1 .{ ... }`
and `{ 1 }.{ ... }` work. `1.0.{ ... }`, a string, a call, or a closing
parenthesis can take the suffix directly. `1..{ ... }` would use Float `1.` as
the subject; it is not a spelling for matching Int `1`. Format integer subjects
as `{ 1 }.{ ... }`. Source: `lexer.rs:371–417`, especially :387–397. No numeric
literal semantics change is needed.

**M4.** A line-leading dot continues the previous expression, exactly as a
field dot does. It cannot begin a new match without a subject. Comments are
ignored for break classification. This follows `token.rs:87–110` and spec
§1.7, not a new indentation rule. Space between dot and brace is immaterial.

### 4.2 Bar in three contexts

**P1.** `|` in an expression is the existing left-associative level-6 pipe:
`a | f(b)` evaluates `a` first, then the rest as in `f(a, b)`. It is not Boolean
or, bitwise or, a shell channel or a new library operation. `||` keeps its
short-circuit Bool rule. The prototype retains the lexer token for obsolete
`|>` only to reject it; the final grammar has no such operator.

**P2.** In a clause head before a guard, bar separates patterns. After `?`,
the guard is an expression, so bar is a pipe even though it is still left of
`->`. In a type, bar is union and the longest type wins. Types occur in aliases,
constructor fields, generic arguments, service annotations and patterns as
well as after a function arrow. The relayed explanation “left of arrow means
alternatives” and “types only after colon/arrow” is therefore not exact.

Examples: `n.{ p ? p | ready -> 1 _ -> 0 }` has a pipe in a guard.
`n.{ x: Int | y -> x _ -> 0 }` has the value type `y` in a union; it does not
bind an alternative `y`. The same holds for literal types. This preserves
spec §7.1's no-backtracking type rule and T2; capitalization is not a parser
oracle (`parser.rs:1494–1508`, `:1653–1687`, `:1760–1878`).

**P3.** Change the first-statement brace probe. A bare bar after a successfully
read pattern is not enough to decide a clause block. Continue reading pattern
alternatives with the existing longest-pattern rules; choose clauses only if
that head reaches `?` or `->`. Otherwise try the ordinary block expression.
Keep the existing generic-binding and function-binding priorities, and the
existing malformed-clause recovery rule. Do not reinterpret a type union as
a pipe when a type parse fails. Direct `.{` calls the clause parser without
this probe.

This fixes `{ x | f }` and `{ (a: b) | f }`. The old early return is
`parser.rs:1404–1415`. The prototype changes that branch and removes the obsolete
brace-record reservation at :1425–1428. It does not replace the parser.

**P4.** Line-leading bar always continues the current construct; it does not
start a new clause. Thus a clause body ending in `x` followed by `| f` continues
that body. To start the next clause, start with its first pattern on a new
line, then any alternatives. A line `| 1 -> y` after a body is an error, not
an alternative attached retroactively to a previous head. This follows the
unchanged break sets (`token.rs:87–110`). No comma is required or permitted as
a clause separator.

**P5.** Diagnose the bar/or confusion in checking when there is evidence.
Both `a | b` and `a || b` are syntactically valid. If the inferred pipe target
is Bool and is not callable, point at `|` and suggest `||` for Boolean or.
Do not reject every `a | b`: `b` may be an ordinary function returning Bool.
A non-Bool, noncallable target keeps the ordinary call error, with pipe context.
The current parser's `PipeTarget` check concerns syntactic target shapes
(`parser.rs:1002–1015`); it cannot make the Bool decision. Allocate any new
checker diagnostic through the coordinator.

**P6, unresolved.** The existing spec permits a literal clause block as a pipe
target. The prototype excludes that direct form and points users toward the
suffix, interpreting “match becomes postfix” as a replacement. The smallest
alternative is to retain it as an alias, constructing the same `Case` node as
before. It must not silently become an ordinary closure call with a new frame.
Recommendation: keep it as an alias unless the user intends to remove it;
canonical printing can use `.{}`. The production grammar/diagnostic must record
that answer. All migration counts here use the suffix, so neither answer
invalidates the corpus result.

### 4.3 Error marker

**E1.** `FnType` and function return annotations use `^ E` in the slot that
used to use the question marker. `?` belongs only to clause guards. `! N`
retains its present meaning and line restriction. `^` can begin a continuation
line; it cannot begin a statement.

**E2.** Error and environment slots belong to the nearest arrow, in that order;
missing slots mean `Never`. A duplicate caret or caret after the nearest
arrow's environment is an error. Group the inner function type to give an
outer arrow its own effect slot. Source: `parser.rs:1689–1711`; the prototype
changes its error-token tests only. The lexer currently has no caret token
(`token.rs:13–70`, `lexer.rs:245–278`); it does not displace an existing
operator. A future XOR operator would have to choose another syntax or a
separately specified expression context; do not reserve it twice by accident.

This now parses without a type group around the function type:

```text
pick = (v, ready) {
    v.{ f: (Int) -> Int ? ready -> f(1)
        _ -> 0 }
}
```

That is a syntax result. Its eventual typing still obeys the existing
head-based function test and coverage rules. A marker rename does not make
function signatures dynamically distinguishable.

### 4.4 Parentheses by context

**D1.** First decide the surrounding context. At file start, an eligible group
is a header under §2.2; following an expression on the same line it is a call;
in a pattern it is a tuple pattern; in an attribute/constructor/function
header it is that construct's argument/field/parameter list. Only otherwise
apply datum-versus-function lookahead. `at_header_group` uses `close[]` and the
token after the close (`parser.rs:460`); `postfix` uses `at_joined(LParen)`
(:1048); neither should be replaced by record detection.

**D2.** At the start of an expression, a matching `)` followed by a same-line
body `{`, or by `->`, selects a function before any field parsing. The guard
exception stays: an unbracketed arrow ends the guard, rather than selecting a
function. Otherwise, zero items select a record; first `..` or a label-colon
selects a record; any other first item selects a nonempty positional tuple.
The rest of the items must fit the chosen form. A comma or newline separates
items but never chooses the form. Existing `paren` already does suffix
lookahead and the guard exception (`parser.rs:1216–1233`).

**D3.** In a type, a matching `)` followed by `->` selects a function type.
Otherwise `..` or label-colon selects a record type; other nonempty contents
select a positional tuple type. Empty contents are an error suggesting
`Record`. Function type parameter lists may still be empty and may have labels
(`parser.rs:1782–1816`). A parenthesised record used as a function parameter
type needs its own nesting: `((a: Int)) -> T` takes one record argument;
`(a: Int) -> T` takes one Int argument labelled `a`.

**D4.** Record values have name or noninterpolated string labels. Record types
also have declared uppercase label parameters. Preserve duplicate-label,
spread-order, forgetting and label-scope rules from records@0d0ce19 §§5,7,8.2.
No positional part is allowed in either record form. No labels are allowed
in tuple data or tuple types. A colon within a tuple pattern still annotates
that pattern; it is not a record field.

| Meeting point | Required reading / result (prototype exercised each) |
|---|---|
| `(a: b)` in an expression | Record whose field value is `b`. |
| `(a: b) { a }` | Function whose parameter has value type `b`; then checking decides whether `b` is in scope. |
| `(a: Int)` in a pattern | One-item tuple pattern containing a type test/annotation. |
| `f(a: b)` and `f (a: b)` | Labelled call, not a record argument. Use `f((a: b))` for that. |
| `f` then `(a: b)` on the next line in a block | Two expressions; the second is a record. A break inside an enclosing parenthesis instead separates items. |
| `(name: path)` at file start | Invalid header, not a record expression statement. Top-level expression statements are not permitted. Imports use `name = package/module`, not colon. |
| `(name: std/path)` at file start | Still invalid; the slash does not validate the colon. |
| `(a, b) = value` at file start | Existing tuple-pattern binding. |
| Block's first expression `(a: b)` | Record if no function suffix follows. No brace-record scan needed. |
| `(n) { (name: n) }` | Function returning a record; function bodies remain blocks. |
| `()` as call argument, tuple/list part, RHS, guard or clause body | Empty record, except where a following function suffix or a pattern context decides otherwise. A record guard subsequently fails Bool checking. |
| `() { ... }`, `() -> Nil { ... }`, `() -> Nil` in a type | Empty parameter lists. |
| `List((a: Int))`, `List((a: Int) -> T)` | Record argument type versus function argument type. |
| `x: (a: Int) -> 1` in a clause | The type parser takes the arrow; no clause arrow remains. Error. Use `x: { (a: Int) } -> 1`. This is the old tuple/function-type boundary applied to records. |
| `(f: (x) -> Int { x })` | One-field record, with no extra comma. |
| `(1, name: "ada")`, `(name: "ada", 1)` | Errors, in either order; type counterparts also fail. |
| `retry(3, body: f)`, `User(String, age: Int)` in a constructor declaration | Existing mixed argument/field lists, pending confirmation. |

A naked typed function at the start of an ordinary brace group still hits the
existing clause/function ambiguity: `{ (x) -> Int { x } }` is rejected after
being read as clauses. Bind it to a name first when a group is required.
Function bodies remain forced blocks, so returning a function from a function
body works. The proposal does not remove this unrelated ambiguity
(spec §6.1; `parser.rs:1350–1441`).

**D5.** Keep `{ T }` for type grouping, including nearest-arrow attachment and
record type tests. “Braces hold only code” must mean “records do not use
braces”; literal removal of all brace type groups is not specified by these
six changes and leaves essential grouping without a replacement. Do not turn
`(T)` into grouping: it is still a tuple type. This is a question for the user,
not permission to change grouping on taste.

### 4.5 Empty values, patterns and names

**N1.** A tuple value, tuple type and tuple pattern have at least one part.
`(x)` is still a tuple, not grouping. `()` in expression position constructs
the empty structural record. It has the open record type `Record`, the set
of all structural records. The value is empty; the type does not assert that
its values have zero fields. `(..R)` is `R & Record`, under the existing place
rules. It does not become unconstrained `R`.

**N2.** Reject `()` as a pattern. Do not make it a record pattern, an
irrefutable unit pattern or a test of all records. Those would contradict
“records have no pattern” or change matching silently. The diagnostic can
suggest a binder/discard if the value is unused; it must not promise that
`_: Record` tests emptiness. Existing record tests remain head-based and must
satisfy spec §4.8 rule 11 (records@0d0ce19:482–488).

**N3.** Add `Record` as a normal built-in type name with no type arguments,
following `Int`'s lookup rules. Do not lex it as a keyword. A module-defined,
imported or declared type variable named `Record` covers it by the ordinary
scope rules. `lookup_type` checks module scope before builtins
(`resolve.rs:1468–1473`); `builtins` populates the fallback table (:810–835);
the built-in inventory is `hir.rs:221–260`. No source word `Record` occurs in
any of the 2,508 tracked `.i` files, including std, lib, examples and tests.
The same word search over all 3,548 extracted snippet sources has no hits.
The spec's substring hits are the grammar identifier `RecordUpdate`, not a
declared name. Existing Rust enums called Record are not source-name conflicts.

User programs outside this repository may define `Record`. They must keep
working. A migration inserting the built-in name into a scope that covers it
needs a fresh, ordinary alias/import supplied from a scope that can name the
built-in. A formatter must never print a capture-prone type and claim it reads
back identically. Do not assume `core.Record` already works: qualified lookup
currently finds exported custom types, not built-in table entries
(`resolve.rs:1441–1465`). An ordinary exported alias can provide a qualified
route, without giving `i/record` a parser rule. That library/API addition is
not necessary to parse the proposal, but its exact name should be settled
before a public migration tool promises capture-free rewriting. `Record` is
the only new source name proposed here.

**N4.** Correct the bare-spread claim. `..` requires a following expression or
type in a record spread. Bare `..` remains in constructor patterns, list-rest
patterns such as `[..]` and `[x, ..]`, and constructor export groups such as
`(User(..))`. Removing the latter two would change features outside this task
(spec §§2.4,7.1; `parser.rs:519–544`, `:1584–1651`).

## 5. Replacement spec text, section by section

This is a patch contract for confirmation, not a second normative spec. Text
not identified here stays. In every example and signature, migrate syntax by
its AST role; do not globally replace every question mark, brace or `()`.
The literal pipe-target choice P6 must be filled in before adoption.

### §1.5 and §1.7

In §1.5's symbol table remove the two-character pipe and add `^`. Keep the
longest-token rule and float grammar unchanged (M3). `^` is not a statement
starter. The §1.7 can-end/can-begin lists otherwise stay.

Replace the continuation sentence with:

> A line that ends in an operator, `,`, `=`, `:`, `->`, `(`, `[`, `{`, or `<<`
> goes on in the next line. A line that begins with `.`, `|`, `?`, `^`, `->`,
> or a binary operator other than `-` and `-.` goes on from the line before.

Replace the final paragraph's question marker with caret. Keep its rule that
an opening brace immediately following an arrow or type operator belongs to
the type, and other unbracketed braces begin the body. Keep the ban on breaks
inside brace type groups. Parenthesised records automatically use the existing
newline-as-comma rule; no separate brace-record line rule is added.

Migrate §1.7's match examples with M2. Keep the warning about clause patterns
starting with a parenthesis or minus on the same line as a prior body.

### §2.2 and §2.3

The header grammar and classifier stay. Add:

> Header groups are not record or tuple values. A colon does not introduce an
> import alias. An arbitrary parenthesised record is not a top-level item.

Replace the sentence calling imports and exports “each a tuple of items” with
“each a parenthesised group of items.” They are not values, and their existing
syntax must not imply empty records or labelled data. Retain nonempty header
requirements, attribute placement and tuple-binding priority.

### §3

Keep union/intersection/difference precedence, names, literal types, value
types, type macro calls and all inference restrictions. Replace/add these
productions; `Label` and `ValueLabel` retain the accepted record restrictions:

```text
Prim       = FnType | TupleType | RecordType | NamedType | TypeCall
           | ValueType | Literal | "_" | "{" Type "}" .
FnType     = "(" [ ParamType { "," ParamType } [ "," ] ] ")"
             "->" Union [ "^" Union ] [ "!" Union ] .
ParamType  = [ name ":" ] Type .
TupleType  = "(" Type { "," Type } [ "," ] ")" .
RecordType = "(" FPart { "," FPart } [ "," ] ")"
           | "(" ".." Type { "," RPart } [ "," ] ")" .
FPart      = Label ":" Type .
RPart      = FPart | "-" Label .
Label      = name | string_lit | UpName .
```

Add D3/D4 as the disambiguation and label constraints. `Record` is a
`NamedType`, not a new grammar production. Remove the sentence “Without `->`,
it is a tuple type”; replace it by D3. Replace the question marker in the
nearest-arrow, absent-error and value-type-header-scope bullets with caret.
The same restrictions still apply to the new error slot.

### §4.2, §4.6–4.8 and T3's §4.9

Add `Record`, with arity zero, to the built-in table and specify its set by N1.
Remove zero-arity tuple cases from the set of values/types. Keep tuple
covariance at positive arities and all accepted record rules. Use caret in
written function types and `Record` wherever the record-top notation is
needed. Change record type delimiters; preserve type groups, record unions,
Without, label parameters, row invariance and the BDD contract verbatim in
meaning. The inference of `()` now uses the empty record rule. This is the
explicit semantic exception, not a claimed equality with the removed tuple.

### §6.1

Retain the existing binary precedence grammar with bar in place of the old
pipe token. Replace/add:

```text
Postfix  = Primary { Call | "." name | "." decimal | "." Clauses } .
Primary  = int_lit | float_lit | string_lit | Name | Tuple | Record | List
         | BitArray | Block | Clauses | Function | CtorUpdate .
Tuple    = "(" Expr { "," Expr } [ "," ] ")" .
Record   = "(" ")"
         | "(" VField { "," VField } [ "," ] ")"
         | "(" ".." Expr { "," VField } [ "," ] ")" .
VField   = ValueLabel ":" Expr .
ValueLabel = name | string_lit .
Function = "(" [ Param { "," Param } [ "," ] ] ")"
           [ "->" Union [ "^" Union ] [ "!" Union ] ] Block .
```

`CtorUpdate` is the accepted T3 rename of the existing nominal constructor
update production; it is not parenthesised structural record construction.
Call/argument, list, block, clause and parameter productions otherwise stay.

Replace the first parenthesis-disambiguation bullet with D1/D2. In brace
decision test 2, replace “followed by bar, question or arrow” with P3's
completed-head test. Keep its binding-of-function exception, using caret in
the arrow tail. Delete test 4's brace-record reservation and the subsequent
“kept for records” bullet. Renumber the remaining diagnostic and fallback
tests; their meanings stay. A function body is always a block. Do not apply
records@0d0ce19's planned `Block | Record` body grammar or its brace scan.
Add D4's mixed-data error, without any comma-based exception.

### §§6.2, 6.4, 6.6, 6.8–6.10 and T3's §6.13

- §6.2: write the pipe evaluation example with bar. Add structural record
  fields in written left-to-right order, spread base first, retaining T3's
  evaluation rule. Sorting labels must not reorder evaluation.
- §6.4: replace “n may be 0, 1, or more” by “n is 1 or more.” Add “Tuples
  have positional parts only; a parenthesised datum with labels is a record.”
  Keep one-item tuples and brace grouping.
- §6.6: retain all call rules, including positional arguments before labelled
  arguments, **if Q5 is confirmed**. Add “A labelled argument is not a record
  argument; pass `(label: value)` inside the call to pass a record.”
- §6.8: change only the return error marker. Function bodies remain blocks.
- §6.9: replace the level-6 entry and its pipe examples by bar. Add match
  suffixes to the list stronger than unary. Delete the paragraph requiring
  matches used as operands to be grouped. Replace it with M2/M3. Replace the
  match definition with M1 and settle the pipe target sentence by P6. The
  level-1–5 grouping rule remains true for a **pipe** subject, while a postfix
  match subject follows M2 for every binary level. Keep W0503 and all scalar
  operator rules.
- §6.10: use T3's direct-value and unused-value rules for records with the new
  spelling. No new generalization rule is introduced.
- §6.13: install T3's record expression rules with D2/D4/N1. The empty record
  is `()`. Remove all record-versus-block comma rules. A function returning a
  record writes it as an expression inside the function body.

### §6.12 and §11.6

Replace the definition paragraph with:

> `e.{ clauses }` is a match, and `e` is its subject. It evaluates `e` and
> calls the clause block with that value. It adds no call frame: its clauses
> run in the place of the match. A call in a clause expression of a match in
> tail position is in tail position, and the match is not a call in a stop
> message.

Keep every rule about clause blocks, one-clause binding, choices, guards,
coverage, labels, direct values and empty clause blocks. Change only the
match spellings in their examples. If P6 retains the pipe alias, state its
same-frame semantics explicitly. §11.6's match references use this definition.

### §7

Change the tuple-pattern production to require at least one pattern. Keep
constructor field/rest and list/rest patterns, string-prefix patterns, choices
and literal patterns. Add N2's empty-pattern error. There is still no record
pattern. `(x: Int)` still tests/annotates the single tuple part.

In §7.1's longest-type paragraph delete the sentence saying that a question
marker after a function type is part of that type and requires grouping before
a guard. It is now a guard. Keep the arrow-consuming rule for tuple types,
extend its explanation to record types, and keep the union-consuming rule.
Delete the explanation that the malformed first clause gives the
“kept for records” error. Give an arrow/type-boundary diagnostic instead.
Migrate examples; remove the already stale “before value types are implemented”
wording rather than reinstating that obsolete reservation. §7.2's tuple row
gets `n >= 1`; no record destructuring row is added.

### §§9, 11.8, 12 and remaining signatures

In §9's function signatures and prose about the error slot, replace the
question marker by caret; guards are unchanged. Apply the same role-aware
replacement to library signature tables, host declarations and appendices.
In §11.8 remove the empty-tuple text-form case and give records the text form
`(label: value, ...)`, or `()` when empty. T3's sorting and label quoting rules
stay (records@0d0ce19:644–654). Add the normal type name `Record` to §12 and
any built-in-name inventory; do not add a keyword. Update T3's `i/record`
signatures without changing their checking or runtime rules.

### Companion notes: exhaustive supersession checklist

This list identifies the prose rules that become false, beyond mechanical
changes to examples/signatures. It is not authority to rewrite historical
reports or expected-error fixtures indiscriminately.

| Source region | Sentences/rules to remove or replace |
|---|---|
| `syntax.md:9,13` summary; §6.12 at :376 | Old match spelling and brace record reservation. Use M1 and D2. |
| `syntax.md` opening replacement (:36–42) and §6.1 (:288–340) | Its record reservation, early clause decision at bare bar, and function arrow-tail error marker. Existing first-function clause ambiguity stays. |
| `syntax.md` §1.7 (:71–97) | Old pipe continuation token and question marker in the type-tail brace rule. Parenthesis/newline rules stay. |
| `syntax.md` §§2.3,3 (:148,206) | Headers called tuples; parenthesised nonfunction forms described only as tuples; old effect slots. T2's supersession of reserved value types remains in force. |
| `syntax.md` §6.9 (:342–350), §7.1 (:392) and A1/A2 notes (:431–503) | Level-6 match/operand grouping and the old migration's match/guard conversion instructions as instructions for future work. Mark those migration notes historical; do not rerun them as this migration. |
| records@0d0ce19 §8.1 (:492–528) | All four record/brace decisions; the “comma decides” proof; the function-valued first-field misreading; the comma escape; forced printer comma; brace newline separator; same-scan function body rule and its rationale. Replace with D1/D2/D4. Label, access and nominal-update constraints stay. |
| records@0d0ce19 §8.2 (:530–555) | Brace record grammar/classifier, empty record-top spelling, distinction of brace group versus brace record, and direct record function body examples. Replace by D3/N1; preserve U4 and place rules. |
| records@0d0ce19 §8.3 (:557), §9.5 (:644), §12 (:747), §13 (:888), §§14–16 | Printer delimiters, inspect text, duplicate replacement spec, proposed function-body scan and comma-based round-trip tests. Change these before implementation. No BDD/checker equations change. |
| `meta.md` §2.3 (:129–199), records@0d0ce19 §13 (:896) | Empty tuple rows, error-slot question heads, normal-surface pipe/match forms and the claim that a comma is necessary for record round-trip. Resolve the explicit `.is` decisions below. |

Do not mechanically replace braces in mathematical sets or in `{ T }` type
groups. The plan's old fixed-syntax bullets and macro-use pipe examples need
an explicit coordinator supersession entry after approval; this note cannot
quietly overrule them.

## 6. The `.is` surface

No `.is` parser exists in this lane's syntax crate (the file inventory is
`crates/ilang-syntax/src/{ast,format,lexer,lib,parser,print,shape,span,token,walk}.rs`).
These are design checks against `meta.md`, not executed `.is` tests.

Recommended additions to its position-based tables:

| Core form | Normal surface | Lisp surface |
|---|---|---|
| Pipe | `x \| f` | `(\| x f)` in expression position |
| Match | `x.{ p -> e }` | `(. x { (-> p e) })` |
| Error result | `() -> R ^ E ! N` | `(-> () (! (^ R E) N))` in type position |
| Guard | `p ? g -> e` | `(-> (? p g) e)`, unchanged |
| Nonempty tuple | `(x)` or `(x, y)` | `(, x)` or `(, x y)` |
| Record top type | `Record` | `Record` in type position |

A dot's last datum being a clause block distinguishes match from existing
`(. e name)` and `(. e decimal)` projection rows. Pipe, union and alternatives
use bar in three **positions**, exactly as meta §2.3 already requires for
arrows; caret frees question for guards there too. `^` is a normal symbol,
not unquote. Backtick, `~`, and `~..` keep their quote/unquote/splice roles
(meta.md:74–103). No collision with caret is present.

The simplest record mapping is to retain records@0d0ce19 §13's proposed
explicit `.is` datums, `{ (: name e) }` and ` Record `, while changing only their
normal-surface printing. They are already structural data in a deliberately
different surface. Then bare `.is` `()` remains a parameter list, not the
normal surface's empty value; `(,)` is removed in expression, type and pattern
positions. The comma round-trip justification disappears: the normal surface
prints `(f: (x) -> Int { x })` without a comma. Do not independently invent a
second parenthesised `.is` record grammar. Ask M1/coordinator to confirm this
mapping, particularly if “braces hold only code” was meant for both surfaces.

Update meta's result rows and the paragraph listing all allowed function
result forms to caret, including the explicit-underscore-result example.
Remove `(,)` from all tuple rows. Preserve no-argument **calls** `(f)` and
constructor patterns `(C)`: those are not tuples. Update macro pipe use in
§4.3. Preserve the two-surface same-code rule and test both conversions. Quote
patterns must recognize the same core nodes; a symbol-only rewrite in quoted
source is not a hygienic migration.

## 7. Experiments and counts

### Method and reproducibility

I copied `crates/ilang-syntax` into `target/syn2/baseline` and
`target/syn2/prototype`, with standalone manifests. Both use the real lexer,
parser, AST and span normalization. The harness adds JSON-lines input/output
and AST traversal hooks for counts and source edits. In the prototype,
records have explicit expression/type AST variants; they are not disguised
as tuples. Its formatter and printer modules are not built; an unused
literal-display dependency is stubbed. Thus this is not evidence of formatter
round-trip or of a complete compiler build. Dependencies for the standalone
harness were resolved separately; the checked-in workspace was not changed.

Build commands, run in this lane:

```text
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo build --manifest-path target/syn2/baseline/Cargo.toml --bin syn2-probe --offline
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo build --manifest-path target/syn2/prototype/Cargo.toml --bin syn2-probe --offline
```

The initial dependency fetch used the same commands without `--offline`.
The baseline parser SHA-256 is
`6bc61de516f764c04e248238dc163e636fc9fd727323d66f13d6932bf6113b47`;
the prototype parser is
`4679973f535248de7d3739bca4ab745f39c6f9ed5cd6ec39af51c1e5ded31e2a`.
The production source files were never edited.

The extraction used `scripts/old-syntax-check`'s `snippets()`
(:105–125), including every tracked `.i`, fenced Markdown and candidate Rust/JS
literal. I did not apply its fixture substitutions/expectation policy or run
its full acceptance command. Hence the 352 baseline failures must not be
reported as 352 repository regressions. The raw inventory and JSON results
are in `target/syn2/{snippets.json,baseline.jsonl,prototype.jsonl,final-corpus.json}`.

Migration uses AST spans to identify `Case`, ordinary pipe, and function error
slots. It changes only those operator tokens. A match subject is grouped if
it is binary, unary or numeric, so its precedence and integer token survive.
The subject is evaluated once. The result is compared using `shape::canon`
(`shape.rs:29–54`), which removes spans and transparent expression groups,
while preserving names, literals, effects, parameters and clause structure.
It is not a comparison of printed Debug strings with regex-deleted content.
It deliberately leaves empty tuple uses in place to expose the semantic gap.
Comments and source excerpts inside documentation are a later migration pass.

| Extracted kind | Baseline accepted, identical after migration | Baseline rejected | Empty-tuple tree changes | Empty-tuple new parse errors |
|---|---:|---:|---:|---:|
| Whole `.i` files | 2,378 | 120 | 7 | 3 |
| Markdown `i` fences | 146 | 19 | 0 | 0 |
| Embedded candidates | 662 | 213 | 0 | 0 |
| Total | 3,186 | 352 | 7 | 3 |

The review corpus has 1,855 project manifests and 1,926 `.i` files. Its whole
files contribute 1,843 identical results, 82 baseline parse failures and one
empty-value tree change. Project count is not file count or test-pass count.
The spec contributes 18 accepted and identical fences. Future record syntax
in accepted design notes cannot be parsed by the baseline at this commit;
that is why §4 also has direct new-syntax probes rather than pretending there
is a complete old-record AST to migrate.

### Counts by directory

The following are AST counts in **baseline-parseable whole `.i` files**.
“Edited” counts files needing executable match/pipe/error-token edits, not
all prose, expectations or empty-tuple decisions. V/T/P count empty tuple
values/types/patterns. A zero does not count empty parameter lists as tuples.

| Directory | Parsed files | Edited files | Match | Ordinary pipe | Error slot | V / T / P |
|---|---:|---:|---:|---:|---:|---|
| `std/src` | 21 | 21 | 730 | 0 | 216 | 0 / 0 / 0 |
| `std/test` | 22 | 18 | 104 | 204 | 17 | 1 / 0 / 0 |
| `lib` | 14 | 5 | 16 | 0 | 16 | 0 / 0 / 0 |
| `examples` | 2 | 2 | 3 | 0 | 0 | 0 / 0 / 0 |
| `bench` | 2 | 2 | 10 | 0 | 0 | 0 / 0 / 0 |
| `crates` fixtures | 231 | 70 | 203 | 49 | 40 | 4 / 1 / 1 |
| `tests/spec` | 248 | 128 | 355 | 12 | 19 | 3 / 0 / 1 |
| `tests/review` | 1,844 | 820 | 3,257 | 209 | 665 | 1 / 0 / 0 |
| other `tests` | 4 | 2 | 1 | 0 | 2 | 0 / 0 / 0 |

Total: 2,388 parseable files; 1,068 need executable token edits; 4,679 matches,
474 ordinary pipes, 975 error slots; nine empty tuple values, one type, two
patterns. There are no implemented structural record nodes at this base to
count. Parenthesised labelled data is currently a parse error; call labels
and tuple-pattern annotations are not old record syntax.

For comparison, these are **raw text occurrences**, including comments,
strings and invalid fixtures. They are not semantic counts:

| Directory | All `.i` files | Old pipe token text | Question-mark text | Empty-parenthesis text |
|---|---:|---:|---:|---:|
| `std/src` | 21 | 739 | 259 | 107 |
| `std/test` | 22 | 315 | 55 | 1,834 |
| `lib` | 14 | 16 | 18 | 14 |
| `examples` | 2 | 3 | 0 | 2 |
| `bench` | 2 | 10 | 0 | 13 |
| `crates` | 261 | 274 | 82 | 335 |
| `tests/spec` | 255 | 372 | 54 | 692 |
| `tests/review` | 1,926 | 3,516 | 939 | 123,539 |
| other `tests` | 4 | 1 | 2 | 20 |
| `editors` | 1 | 3 | 2 | 3 |

The relayed “737, of which 732 matches” is reproducible as a **line count**:
737 lines contain the old pipe token, and 732 lines contain that token followed
by a clause brace. The other five lines are documentation examples of ordinary
pipes. `rg -o '\|>' std/src --glob '*.i'` counts occurrences instead and gives
739, because two comment lines have two pipes each. AST traversal finds 730
executable matches and zero ordinary pipes. The nine documentation occurrences
are two matches (`std/src/dict.i:327`, `std/src/list.i:519`) and seven pipes
(`std/src/dict.i:82,134,353,433`, `std/src/set.i:82`).
The source file list for executable edits is kept locally in
`target/syn2/files-to-migrate.txt`; this is a lower bound for a full migration.

For old brace records, this baseline has no valid expression/type record AST
at all, so a semantic occurrence count would be fabricated. A separate raw
scan of the extracted snippets finds the following **candidates**, including
comments, strings and intentional errors. The patterns are an opening brace
followed by bare `..` and a close, followed by `..` and a non-close payload, or
followed by a name/string label and colon. The third also finds typed clauses;
the second also finds printed-value ellipses. These are worklist sizes, not
record counts or instructions to replace braces automatically.

| Extracted directory/kind | Empty brace-record text | Spread-prefix candidates | Label-colon brace candidates |
|---|---:|---:|---:|
| `std/src`, `lib`, `examples`, `bench`, `editors` | 0 | 0 | 0 |
| `std/test` | 0 | 9 | 1 |
| `crates` whole files | 0 | 0 | 37 |
| `tests` except review | 0 | 9 | 33 |
| `tests/review` | 3 | 50 | 207 |
| Markdown fences | 14 | 9 | 34 |
| Embedded candidates | 0 | 0 | 26 |

The no-mixing rule removes no previously valid labelled tuple data: the old
value/type tuple productions already have only expressions/types, not labels
(`spec.md:549,838`; `parser.rs:1216–1230,1782–1816`). The new error is needed
because records now share their delimiters. Reclassifying future record
examples needs the accepted record grammar in the migration reader; simple
text counts cannot distinguish a record type, record value and typed clause.

The ten affected files with real empty tuples are:

- `crates/ilang-check/tests/projects/attr_entry/src/main.i` (two values).
- `crates/ilang-check/tests/types/basics.i` (one value).
- `crates/ilang-syntax/tests/golden/fn_vs_tuple.i` (one value).
- `crates/ilang-syntax/tests/golden/patterns.i` (one pattern).
- `crates/ilang-syntax/tests/golden/types.i` (one type).
- `std/test/string_test.i` and its review-corpus copy under
  `tests/review/cases/review-k4s/std/test/string_test.i` (one value each).
- `tests/spec/11/text_form.i:83` and `tests/spec/6/data.i:42` (one value each).
- `tests/spec/7/patterns.i:153–154` (one value and one nested pattern).

These need reviewed expectation/meaning changes, not blind substitution of
`Record` for every `()` and not a regex replacement by `Nil`.

### Focused parser tests

`target/syn2/matrix.json` records 85 named probes, their source, expected syntax
outcome, actual diagnostics and canonical AST. All 85 meet the stated syntax
expectation. They cover every row of §4's tables, effects in both orders,
duplicate effects, function-return nesting, lowercase/literal types, all
numeric/string/call suffix cases, headers, multiline records, both directions
of mixing, constructor/list rest, and old-token rejection. The existing typed
function-at-block-start error is an expected rejection, not newly accepted
syntax. These probes do not run the checker.

The following value/type tests must be added to the actual compiler gates;
the expected values are deductions from the cited rules, not runs of the
prototype:

| Test | Expected result and rule |
|---|---|
| `{ 0 }.{ 0 -> 1 _ -> 2 } * 10` | `10`, M1/M2. |
| `-{ 1 }.{ 0 -> 1 _ -> 2 }` | `-2`, M2. |
| `3 \| (n) { n + 1 }` | `4`, P1. |
| `False \|\| True` | `True`; Bool short circuit unchanged, P1. |
| `False \| True` | Callability error with Boolean-or hint, P5. |
| `(name: "ada").name` | `"ada"`, D4 and existing record field rule. |
| `(..(a: 1), a: "x").a` | `"x"`; Without plus replacement, not intersection, D4/N1. |
| `() == ()` | `True`; existing structural record equality, N1. |
| Function taking `(a: Int)` called with `()` | Type error: record top does not guarantee field `a`, N1. |
| A record field containing a function, with no trailing comma | Parses as a record; calling its field gives that function's result, D2. |
| `() -> Nil` as annotation and `() { Nil }` as value | Both accepted; zero function parameters are not empty tuples, D2/D3. |
| Empty tuple pattern | Syntax error, N2. |
| Mixed positional and labelled datum in either order | Syntax error, D4. |
| Typed record clause against a union of differently shaped records | Do not add field-shape runtime discrimination; enforce existing head-test coverage obligation, records §7. |

Add effectful field/spread tests that log order, and tail-recursive match tests
that check bounded frames and stop-message positions. The purpose is to prove
reuse of `Case`, not just parse acceptance.

## 8. Implementation hand-off, files and acceptance

### T3: avoid doing syntax twice

At the observed `lane/a` head `31444065`, the diff against this lane contains
only the record kernel under `crates/ilang-check/src/types/records/` and its
module wiring. `bdd.rs:7–56` defines handles, operations and the demand budget;
`graph.rs:14–18,53–62` has record-domain identity and record/union/intersection
nodes. There is no syntax-crate diff. This is a read of a branch snapshot,
not a claim about another worker's uncommitted files or later progress.

Until confirmation, the ledger explicitly says no syntax implementation
change. T3 can continue the representation/checking/lowering work independent
of delimiters. Keep record AST/HIR payloads independent of concrete brackets,
keep `Id::RECORD` and Without unchanged, and avoid entangling a lexer decision
with the BDD. After confirmation, implement D2/D3 directly instead of the
§8.1 brace scan, do not permit a bare record as a function body, and omit the
comma-insertion logic. Implement record printers, inspect output and goldens
once in the approved surface. Parser adoption, built-in name wiring and the
record formatter must land together with T3's syntax consumers.

### File and region map

| File/region at baseline | Required work |
|---|---|
| `crates/ilang-syntax/src/token.rs:13–70,87–147`; `lexer.rs:245–278,371–417` | Caret token; remove valid old pipe; preserve break sets and number tokenization; dedicated removed-token guidance. |
| `parser.rs:119–135,977–1072` | Bare pipe and postfix `Case`; settle P6; remove obsolete level-6 match-operand diagnostic. |
| `parser.rs:1216–1264,1350–1441,1494–1651,1689–1878` | Datum/function split; revised brace probe; nonempty tuple patterns; caret error slot; record/function/tuple type split and empty-type hint. |
| `parser.rs:381–481,1074–1135` | Retain header and call contexts; add regression tests for records beside them, not a new classifier shared indiscriminately across contexts. |
| `ast.rs:202–304,408–443`; `walk.rs:64–81,159–209,250–282`; `shape.rs:29–54` | T3 record nodes/visitors, preserving label/base order and distinct tuple nodes. `Case` and pipe nodes stay. |
| `format.rs:458–480,1164,1205,1290–1335,1379–1445,1605–1690` | Record delimiters and field lines; match subject precedence/number spacing; bar continuation and comments; caret printing; type-test grouping. |
| `print.rs:29–38,530–572,650–670,796–900` | Same rules without source comments; remove obsolete match grouping helpers where no longer needed; print only parseable records. |
| `ilang-syntax/tests/{lex,parse,golden,format,spec}.rs` and fixture directories | New boundary matrix; reparse/canonical-tree and idempotence properties; reviewed new diagnostic goldens. Current properties are in `tests/format.rs:1–9` and `tests/spec.rs:29–54`. |
| `crates/ilang-check/src/hir.rs:221–260`, `resolve.rs:810,1418,1795,2514`; `types/display.rs:1357` and T3's shown types | Normal `Record` name, no empty tuple surface, record display, preserved single-clause lowering, and scoped pipe/or diagnostic. Use T3's existing structural type representation. |
| `crates/ilang-wasm/src/lower.rs:1562–1580,2289,2342`; T3 runtime record inspector | Verify tuple arity removal at the source boundary; retain existing Case lowering; print record delimiters. Existing nominal `Record` names are unrelated. |
| `scripts/old-syntax-check:105–125,143–185`, its JSON, tests and docs; `scripts/a2-meaning`; `scripts/wat-diff:67,382–422` | Second-migration extraction/reporting and explicit exceptions; regenerate reviewed content identities and diagnostics; retain strict WAT modes. |
| `docs/spec.md`, `docs/design/{syntax,records,meta}.md` | Apply §5 together. Do not leave two accepted contradictory grammars. |
| `editors/vscode`, package templates, Rust/JS embedded sources, all changed `.i`, comments and text/type expectations | Inventory by source role; update highlighting for caret, bar and match dot; preserve CR/CRLF and layout-sensitive tests. |

The plan locates the first migration's one-off A2 programs in the shared cache,
not `scripts/`, and at :411–429 says only two partitions were replayable. The checked-in
`scripts/a2-meaning:1–80` is an audit of changed one-clause semantics that calls
an old formatter-report interface; it is not a general second migrator. The current formatter command takes only project, check and paths
(`crates/ilang-cli/src/commands/fmt.rs:15–43`), with no old-source conversion
mode to reuse.
Use the extractor and AST-edit method as a new migration pass rather than
claiming the old commands can simply be rerun.

### Acceptance and order

1. Confirm §10's boundary answers; write the approved spec/syntax/records/meta
   amendments before changing production syntax. Freeze a base and file/snippet
   inventory, including invalid and future-track fixtures.
2. Keep T3's semantic representation work independent. Add caret and postfix
   parser support plus a second-migration reader; add bare bar and the brace
   probe in the same parser change. Add record datums/types/name with their
   T3 checking/lowering consumers. Do not ship a normal parser where `()`
   sometimes means tuple and sometimes record based on an inferred type.
3. Implement both printers and update their AST equality/idempotence tests.
   Use an explicit source-version migration mode for the old empty tuple;
   accepting both old/new interpretations of the same `()` token is impossible
   without such a mode. Temporary distinct-token compatibility for the old
   pipe/error markers is easier, but must not leak into the final grammar.
4. Run AST-span migration over baseline-accepted code and review separate edits
   for the ten empty-tuple files. Migrate documentation comments and embedded
   snippets with extractor substitutions, not whole-file regexes. Handle old
   brace records only in a migration reader that implements their accepted
   grammar; this base's parser has no such node. Invalid fixtures need explicit
   expected-diagnostic decisions, not best-effort recovered AST rewrites.
5. Run parser/formatter/spec tests, full `old-syntax-check`, review corpus with
   its held/known-failure policy, compiler/runtime tests and development gates.
   Preserve each fixture's purpose and expected error location. Regenerate
   content-hash registrations only after checking what changed.
6. Compare code generation from a fixed baseline. For the syntax-only subset,
   require identical canonical ASTs and byte-identical WAT when source-site
   metadata is preserved. If new source spans change `$site_data`, use the
   existing **separate** `wat-diff` site-masked mode with the same site count;
   label this normalized equality, not byte equality. Its `--byte-equal` mode
   explicitly forbids masking (`scripts/wat-diff:419–422`). Do not authorize
   nominal renumbering or broad allowed-difference lists for this task.
7. Empty-tuple removal is an explicit exception to both AST and WAT equality.
   Record allocation and tuple allocation are different lowering paths
   (`lower.rs:1562`; records@0d0ce19 §9.4). Give each migrated fixture an
   approved new value/type/diagnostic expectation. A global “all WAT equal”
   claim for all six changes would be false. If strict equality for every
   existing program is mandatory, the smallest proposal change is to retain
   the empty tuple under another spelling or postpone its removal; ask the
   user, do not silently invent that spelling.
8. Remove compatibility paths, update new-project templates and editor tests,
   and land syntax plus corpus migration together as A1/A2 did. Run the
   old-syntax gate again without exceptions for ordinary source.

The 1,855-project corpus includes historical sources and host/generator tools;
parsing its files is not enough. Preserve `expected.json`, known-failure
status and origin metadata. The corpus importer explicitly treats host tools
as data (`scripts/qa/corpus.py:63–85`). Do not execute or mass-rewrite those
historical tools merely because they contain old symbols.

## 9. Self-attack and corrections

I attacked soundness, implementability and source agreement separately.

**First pass found real issues.**

1. `{ x | f }` was a clause block under the unchanged probe. Fixed the
   prototype's probe and P3. No comma workaround was introduced.
2. Blindly grouping every old match subject broke the existing fixture
   `crates/ilang-syntax/tests/golden/case_head.i`, where the subject begins
   with a typed anonymous function. The resulting brace group was read as
   clauses. Fixed migration to group binary, unary and numeric subjects only;
   postfix-safe subjects stay ungrouped. That file now has the same AST.
3. Treating `()` as a mere token substitution hid ten changed files. Separated
   removed tuple values, types and patterns, and removed any promise of global
   WAT equality. Proposed N2 rather than adding a record pattern implicitly.
4. The slogan about braces would remove required type groups. Kept them as an
   explicit question. The general bare-`..` claim also missed list patterns
   and export groups; N4 names those unchanged exceptions.
5. The local record note was stale. Read the ledger-accepted revision and
   retained its exact unions/BDD/inference rules. In particular, record unions
   remain legal in every place; this study must not restore the old E0518
   record-alternative count.
6. `Record` printing can be captured by a user type with that name. N3 preserves
   scope and asks for a qualified ordinary alias before promising a total
   public migrator. A special lexer keyword is not an acceptable fix.
7. A pipe into a literal clause block was underspecified. P6 records both
   precise readings; silently changing it into a frame-adding call is excluded.

**Soundness counterexamples retained for the implementation.**

- A function requiring field `a: Int` must reject an empty record. Do not read
  the name `Record` as an exact zero-field shape, and do not erase a required
  field during parenthesis disambiguation.
- Replacing `a: Int` by `a: String` with a spread must give the String field,
  not the impossible intersection. Parenthesis parsing cannot alter Without.
- A union of `(a: Int)` and `(b: String)` cannot be distinguished at runtime
  merely by a typed record clause. The existing head-test obligation remains;
  the syntax probe's acceptance of a type is not proof of a valid program.
- `Record { Record }` must still declare an ordinary nominal type. Parser
  spelling must not bypass resolution and let it masquerade as structural top.
- A one-clause match on a failing constructor pattern still reports Assert;
  adding a guard or another clause still invokes coverage. Reusing `Case`
  keeps these differences; rewriting to a new generic call node might not.
- Converting an old empty tuple value blindly to `Nil` changes equality against
  existing `Nil` values. Converting its exact type to `Record` broadens the set.
  Neither is a semantics-preserving universal migration.

After those corrections, two further passes found nothing serious:

- **Pass A, run:** all 85 saved boundary probes reproduced their exact
  diagnostics and canonical trees. Repeated all 3,548 corpus inputs; there
  were zero unexplained new errors or changed trees beyond the explicitly
  identified empty tuples. Rechecked the replacement rules against the spec,
  accepted record amendment, meta tables and ledger; no further source
  conflict was found. This is a finite check, not a proof of grammar completeness.
- **Pass B, run:** generated 630 programs from 15 subject forms, seven clause
  body forms and six enclosing contexts. Every baseline program parsed; every
  mechanically migrated program parsed to the same canonical tree. Subjects
  included nested matches, pipes, unary/binary expressions, and typed/untyped
  function literals. Bodies included functions, tuples and nested pipelines.
  Separately generated 113 record expression/type probes, including
  function-valued fields, value/literal types, nested generics and effects;
  all parsed. Revisited the six soundness counterexamples above against the
  proposed AST paths and unchanged type rules; no new semantic rule was
  needed. Their compiler-level verdicts remain implementation gates.

Results are in `target/syn2/attack-passes.json`. These passes support the
reported parsing feasibility; they do not claim the unimplemented checker,
formatter or runtime tests passed.

## 10. Questions for the coordinator/user

1. **Approve these six changes as a new contract?** Recommendation: confirm the
   surface directions with the explicit boundaries here. Until then T3 follows
   the current ledger and does not build a competing record parser.
2. **Is empty-tuple removal an intended semantic break, including patterns?**
   Recommendation: yes if that is the user's intent; reject empty tuple patterns
   and review the ten files separately. If all-program WAT byte equality is a
   hard condition, postpone that removal. No syntax-only tool can satisfy both.
3. **May `{ T }` remain a type group?** Recommendation: yes; qualify the brace
   slogan to exclude grouping in types. The six changes supply no replacement
   for it, and `(T)` is still a tuple type.
4. **Keep the existing float lexer at integer-dot-brace?** Recommendation: yes;
   print `{ 1 }.{ ... }`, with `1 .{ ... }` also valid. If `1.{ ... }` must work,
   approve the narrow lexical exception “decimal followed by dot then opening
   brace does not consume that dot as a float,” and test it separately. Do not
   silently reinterpret `1..{ ... }` as an Int subject.
5. **Keep mixed call arguments, constructor fields and function parameter-type
   lists?** Recommendation: yes. Restrict the no-mixing rule to tuple/record
   data and their types. The user has not explicitly confirmed that exception.
6. **May `x | { clauses }` remain an alias?** Recommendation: yes, as an ordinary
   pipe target that retains existing direct-match semantics and prints as
   `x.{ clauses }`. If only the postfix spelling is wanted, reject it with a
   targeted hint. This changes no library rule, but must be explicit.
7. **Should `Record` be shadowable, and how should generated types name it under
   shadowing?** Recommendation: ordinary shadowing, plus an ordinary exported
   type alias usable through a fresh module alias for migrations/printing.
   Settle that alias's name and check it before promising a total converter;
   do not pretend built-ins already resolve as `core.Record`.
8. **Apply glyph changes to `.is`, keeping its explicit record datums?**
   Recommendation: caret for error results, bar for expression pipe, dot with a
   clause datum for match; retain its proposed brace record datums. Remove empty
   tuple datums. M1 should confirm these table entries before implementing them.
9. **Retain list-rest and export uses of bare `..`?** Recommendation: yes; amend
   the “always followed” claim rather than remove those unrelated features.

## 11. What is not verified

No production compiler or T3 code was changed. I did not run Wasm execution,
WAT comparison, code-size or parser-throughput benchmarks, full development
gates, formatter tests on the prototype, `.is` round trips, or type checks of
new records. Record checking/lowering is still another track. I did not prove
that the extended brace probe has no pathological performance case; retain
cached brace decisions and matching delimiters, and add generated stress tests
before landing. The reference study is source reading, not a benchmark against
those implementations. The local experiment files are uncommitted evidence,
not a shipped migration tool. The spec changes and questions above remain
reviewable independently of those temporary files.
