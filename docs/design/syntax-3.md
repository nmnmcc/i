# Syntax three: one meaning per bracket

Status: controlling design note for SYN3, 2026-11-12, revised after the third
and fourth rounds of answers (the owner's decisions on this note, and the
details settled by the draft author and the coordinator), then amended for the
unification of services and effects and for first-class generics (stage 0 of
the effects plan: O23 to O32, §2.10, §2.16, stages E1 to E6). Workers implement
from this note. The base is `redesign` at 5c186400 (worktree `lane/editors`).
Nothing in it has been parsed or checked: no SYN3 parser exists. Statements
about today's compiler cite files at that commit; statements about the new
language are derived from the rules below. Every example in the new syntax
is in a `text` fence, so that `scripts/check-snippets`, which parses every
` ```i ` fence in the repository, does not read a proposal as source.

## 1. Status and sources

### 1.1 Sources and precedence

1. `.rd/cache/syntax-3-draft.md` (the draft, Chinese). Its sections 1 to 7
   are the owner's decisions. Its section 8 is the author's additions.
2. `.rd/cache/syntax-3-answers.md`: two rounds of answers by the draft author
   and later owner decisions relayed by the author; a third round with the
   owner's decisions on the first version of this note; a fourth round with
   the remaining details, settled by the draft author and accepted by the
   coordinator under the owner's instruction "不用什么都问我" (do not ask me
   about everything). Later text overrides earlier text.
3. `.rd/cache/gpt/report-eval-syn3.md` (the evaluation): grammar delta,
   attacks G1 to G23, blockers B1 to B5, overloading design (§4.1 to §4.4),
   decisions D1 to D11, staged plan.
4. The language today: `docs/spec.md`, `docs/design/records.md`,
   `syntax-2.md`, `meta.md`, `value-types.md`, and `crates/`.
5. `.rd/cache/effects-proposal.md` (Chinese, approved by the owner): the
   unification of services and effects and first-class generics (O23 to O28).
   `.rd/cache/gpt/report-effects-review.md` (the effects review): rules A, B,
   C of its Q1, the syntax of its Q6, the decisions of gaps 5 to 19 of its
   Q7, and its staged plan Q8 (merged into §7.2 as stages E1 to E6). The four
   owner questions of its Q7 (gaps 1 to 4) were answered with the review's
   defaults (O29 to O32). Where this source and the earlier ones disagree,
   this source wins.

Where this note says "owner", it means the user of the project, as quoted by
the draft author. Where it says "the author", it means the draft author.

### 1.2 Owner decisions

Each line gives the owner's words where the inputs quote them.

| # | Decision | Owner's words (source) |
|---|---|---|
| O1 | Every symbol has one meaning: `{ }` computation, `( )` data, `->` result type, `\|` cases joined, `=` binding, `:` a type follows. A function is a computation with parameters. | Draft §2 ("每个符号只有一个含义") |
| O2 | Parameters are always in brackets, with no omission rule; the parameter group is an ordinary pattern matched against the data made by the call. | Draft §3.2 |
| O3 | Type first, and every branch may carry its own type. | "先写类型、再写实现" (Type First, draft §3.2) |
| O4 | Several functions joined by `\|` are one function; the first written wins. Matching is sending a value into such a function. Clause blocks `{ p -> e }` are removed. | Draft §3.3 |
| O5 | Overloading is §3.3, not a separate feature. Overloading by arity is allowed and must be exact. | "这是 Erlang 中经典的功能", "精确地实现重载" (draft §3.4) |
| O6 | Precision is required at the call site, including a function passed as an argument whose expected parameter type is fixed by the other arguments; `list.map([1, 2], show)` is `List(String)`. Where the place cannot see types, fall back to the union of all branches. Intersections of function types are not kept as first-class types. | "在调用处精确" (draft §3.4; answers 1c) |
| O7 | Data is a finite map from keys (positions or labels, mixed) to values; shapes must match exactly; open records are deleted. | "全部封闭，删除开放记录" (draft §3.5) |
| O8 | A call's brackets make data: `f()` passes `()`, `f(x)` a one-tuple, `f(a, port = 443)` one position and one label. `x \|> f` is `f(x)`. `(x)` is not `x`. Data cannot be called; `t.0` and `r.name` are the only reads. | Draft §3.5; answers 3e |
| O9 | `( )` creates no scope; later items of a data literal do not see earlier labels. `(w = 3, h = 4, area = w * h)` is an error; `(w = h, h = w)` swaps. This withdraws draft §3.5's `area = w * h`. | "数据里后面的项还是不能调用前面的，因为 ( ) 并不是作用域，作用域只通过 { } 创建" |
| O10 | Labels in data types are written `label = Type`. `:` means only "a new name or pattern on the left, its type on the right". `(name: String)` is only a pattern (position 0), `(name = String)` only a type, `(name = n: String)` a pattern with label, binder and type. `add: (a: Int, b: Int) -> Int` is an error; documentation labels in function types are removed by the migration. | "用等于号" |
| O11 | In a parameter, `(name: T)` is a positional binder. Labels only by `key = pattern`; labels in function types are real. A positional key and a label key are two kinds of key; labels have no order. | "名字只是位置的别名反直觉，不要"; "我希望遵循函数参数几乎就是普通映射解构" (answers D1) |
| O12 | No shorthand when key and binder are equal, for now. | "同名时暂时不考虑简写" (answers D2) |
| O13 | Destructuring runs left to right; `=` chains; the name of a whole positional item comes after its structure. | "解构的时候所有的规则都反过来了，等号是从左往右赋值的，可以 a = b = c = d 一路赋值下去"; "结构在前，名字在后" (answers B4) |
| O14 | One form of type definition, `Name(params) = right side`. New things come only from the left of `=`. A right side that is data or `Opaque` makes a new labelled (nominal) type, which is also its constructor; any other right side makes only a name. Each labelled type has exactly one constructor. A sum type is a union; one constructor may belong to several sums. `T { C1 C2 }` and `T {}` are removed. | Draft §3.7; "一个构造器可以属于多个和类型" (answers 2d) |
| O15 | `\|>` is the weakest operator; its whole left expression is its left operand. Other levels are unchanged. | Draft §3.8 |
| O16 | ~~The environment is read by a statement `db ! Db` inside a computation, paired with `! Db = e`.~~ Withdrawn by O24 and O29: services are named in the function head after `<-`, and provided by `e <- (s)`. | Draft §3.8; effects proposal |
| O17 | The guard `? cond` is inside the parameter group, after all parameters: `(a: Int, b: Int ? a > b) -> Int { a - b }`. | "条件写进参数的括号里" |
| O18 | A function with several shapes is a first-class value without choosing a shape, and the run time supports it: `fs = [area]; f = list.first(fs); f(2.0); f(3.0, 4.0)` is legal and runs. | "让运行时支持这种函数值，这才是理所当然的逻辑" |
| O19 | Not every precise overload type has a spelling. The costs of draft §7 are accepted (no "any value with a `name` field" functions, no run-time field addition, the built record machinery is dropped). | Draft §3.4, §7 |
| O20 | In the Lisp surface, a function body is any one expression; `{ }` itself is a computation. Labels are in a fixed order, and the printed text of a value is a legal expression: `User(age = 36, name = "ada")`. A multi-shape function type is written `{ (Float) \| (Float, Float) } -> Float`. | "函数体是任意一个表达式，{ } 本身就表示一种计算。标签按固定顺序排，打印结果是合法的表达式。{ (Float) \| (Float, Float) } -> Float。" (third round) |
| O21 | A member of a sum need not be a constructor. `i/syntax` drops its `Int(Int)` and `Float(Float)` wrappers: the built-in types are members directly and are told apart by type tests. Members whose content is `String` (Name, Discard, Up, Symbol, Text) and Round, Square, Curly stay labelled types. `Node` is written with the outer parentheses. | "Node = (Name \| Int \| Float \| Text) 直接写成这样不就行了" (third round) |
| O23 | Type parameters are written on function values and function types, `<A, N>(…) { … }` (plan A); binding heads `name(A) = …` for functions are removed; type definitions `Name(A) = …` are unchanged (§2.16). | "方案 A 可以" (effects proposal) |
| O24 | `!` is replaced by `<-`, "what a function takes", the mirror of `->`; `!` and `^` leave the language (`!=` stays). | "感叹号太奇怪，不如用 <-，标识取，刚好和 -> 返回相对" |
| O25 | A call that only uses the current environment is not marked; `<-` is written only to take or provide something. | "<- 空的就不用写了，避免变得繁复" |
| O26 | No implicit capabilities. | "不要做隐式能力，这是 Rust 最让人头疼的问题" |
| O27 | `^ E` is abolished; errors are the ordinary service `Fail(E)`. Services are keyed by their type constructor: no order, no names, no repeats. | "^ E 废除，就是普通 Fail(E)"; "数据里加一种服务专用的键，比如用类型当键名"; "类型键没有顺序也不看名字，不会出现重复" |
| O28 | Services and effects are unified on the Effect-TS model (rules R1 to R6, §2.10.3); `<- (..env)` replaces `keep_env`; layers are ordinary functions. | "学 Effect-TS 的做法"; "可以啊" (effects proposal) |
| O29 | Provide is `e <- (s)` for any expression `e`: `e` is worked out with the current environment plus `s`. The block form `{ … } <- (s)` replaces the statement `! T = e`. | Owner's answer to review gap 1, relayed by the coordinator ("按默认执行"), then generalised by the owner |
| O30 | `catch` uses a light, escape-only boundary with no continuation state; bench gate: at most 1.25x today's `fail.catch` (§2.10.7). | Review gap 4, default confirmed ("按默认执行") |
| O31 | The platform provides a top-level `Fail` at entries: any `E` for `main` and tests, a host-passable `E` for host exports (§2.10.11). | Review gap 2, default confirmed ("按默认执行") |
| O32 | Per-target library defaults and package defaults are chosen explicitly in `main`; no package supplies services implicitly (§2.10.11). | Review gap 3, default confirmed ("按默认执行") |
| O22 | Details are decided by the coordinator, or by the draft author and the coordinator together; only what changes a principle the owner stated, or clearly changes the look of the language, goes to the owner. | "不用什么都问我" (fourth round) |

### 1.3 Rules accepted without explicit owner confirmation

These come from the author or the evaluation. The author accepted them; the
owner did not confirm them item by item. They are binding for this note but
must be shown to the owner as such.

**Not confirmed item by item** (the list required by the coordinator):

- **Opaque position and invariant parameters** (draft §8.1, answers 8.1):
  `Opaque` occurs only as the whole right side of a type definition; such a
  definition may have parameters, which are invariant.
- **Whole-sum export `Option(..)`** (answers 2d, D8): `A(..)` for a name
  exports the name and each member with `(..)`, only when every member is a
  labelled type of the same module.
- **Rest only with a known shape** (draft §8.3, answers 8.3): `..` spread and
  `..rest` need a known shape; rest is data, not a list.
- **D3**: `|` joins only function literals.
- **D5**: a labelled type whose data is `()` is a value (`None`); `None()` is
  an error.
- **D7**: a union equal to the expansion of a declared name is shown by that
  name, implemented before migration.
- **D8**: as above (whole-sum export).
- **D9**: the empty match is a core function `(Never) -> A`.
- **D10**: a stored overload has the union of all results; there is no
  per-shape result type.
- **D11**: host exports, entries and `@external` declarations are
  single-shape.
- **The solver checks arguments against the selected domains**, not against
  the whole group (answers B1). See §2.6.6 for what this does and does not
  guarantee.

Further author rules the owner did not object to but did not confirm:

- The run time takes the first matching branch in written order (answers 1a).
- Two rounds of argument checking: non-function arguments first, then
  function arguments with the expected types so found (answers 1b).
- Branch domains come from written patterns and annotations, not from bodies;
  an unannotated branch result is an inference variable, monomorphic in its
  recursion group (answers 1e).
- `Point = { (Int, Int) }` is a name because of the group (answers 2c).
- No default parameters; optional parameters are shape overloading
  (answers 3c).
- Alternatives are inside the parameter group: `(Sat | Sun) { … }`; `|` is
  tighter than `,` there (draft §8.6, answers 3d).
- Exports keep `User` (type only) and `User(..)` (draft §8.5).
- `Nil` is a prelude definition `Nil = ()`; `()` is a second, structural unit
  (draft §8.2, answers 8.2).
- Refutable constructor bindings stay legal (B3/D4): carried over from today's
  language; not discussed with the owner.
- The conjunction rule with the key exception (B4) is the author's statement
  of the owner's two examples.
- Migration makes function signatures and their labelled calls positional and
  keeps labels on constructor fields (D1).
### 1.3a Settled in the fourth round (draft author and coordinator, under O22)

These were the open questions Q1 to Q19 of the first version of this note.
They are not the owner's words; they are binding. Where the rules below state
one of them, it is marked **[R4]**.

- Q1: `name ! T = e` (a read followed by `=`) is a compile error. Obsolete:
  the read statement is removed (O24, §2.10.1).
- Q2: in data, a positional item may not follow a labelled item (§2.3).
- Q3 (owner, O20): labelled values print in a fixed label order, as a legal
  expression (§2.15).
- Q4: the fixed order is UTF-8 byte order of the labels, for storage, type
  display and the text form.
- Q5 (owner, O20): `{ (Float) | (Float, Float) } -> Float`.
- Q6: a binding with a written type, `f: T = chain`, has no selection table.
- Q7: no rest at the top of a parameter group.
- Q8: the empty-match function is `core.never : (Never) -> A`.
- Q9: known types are facts; the spec states it (§2.6.6).
- Q10: no selection in a match on a chain literal.
- Q11: a function-type member left ungrouped in a result union is not an
  error; the type error gets a better message (G2).
- Q12: a data pattern fitted to a union of shapes stays a compile error.
- Q13 (owner, O21): `Node = (Name | Int | Float | Text …)` with the outer
  parentheses, a new labelled type holding one item of union type; the `Int`
  and `Float` wrappers are removed; nothing is renamed (§2.8, §7.3).
- Built-in types are exported by `i/core` and may be named through it (§2.8).
- Q14: `x: T` in a one-branch group of a multi-group chain is an annotation.
- Q15: the need (errors included, as `Fail`) is shared by the branches of a
  chain (R6).
- Q16: host-facing parameter data (`@external`, host exports, host-service
  fields) has positions only.
- Q17: `.k` and `.f` read labelled values whose constructor is visible.
- Q18: variance per labelled type by spec §4.5 rule 6; `In(A) | Out(A)` is
  more permissive than today, and sound.
- Q19: no cap on the size of error rows; measured after stage 3.
- Lisp surface: parameters are written as data destructuring,
  `(-> (, a b) R e)`; positions Parameters, Parameter, Field and CallArgument
  merge into one "item of data" position each for values, types and patterns;
  empty parameters are `(,)`; the empty `()` has no legal position (§6.8).

Rules that this note itself adds, because no input settles them, are marked
**(this note)**. They are details under O22; none changes a principle or the
look of the language.

### 1.4 Contradictions between the inputs and how they are resolved

| Inputs | Contradiction | Resolution |
|---|---|---|
| Draft §3.5 against the owner's later D6 | `(w = 3, h = 4, area = w * h)` "later items see earlier names" | Later owner text: `( )` is no scope (O9). |
| Draft §3.5 against the owner's "用等于号" | Values use `=`, types `:` | Later owner text: types use `=` too (O10). The evaluation's `TItem = [ Label ":" ] Type` becomes `[ name "=" ] Type`. |
| Draft §3.6 examples | `((a, b) = args)` puts the name after; `(size = whole = (w, h))` puts a name first | B4: conjunction plus key exception; `size` is a key, `whole` a name (§2.7). |
| Draft §3.3 and §8.4 against the owner's O17 | Guard after the group: `(Some(v)) ? … { … }` | Later owner text: guard inside the group. |
| Answers 1a against answers B1 | Effective domain = pattern type minus earlier pattern types | B1: subtract only what is "taken" (spec §4.8 rule 12). |
| Answers 1f | The author recommended option 乙 (shape must be chosen) | The owner chose 甲 (run-time support), O18. |
| Draft §6 against the evaluation §6 | "Function types are untouched", "`e \|> branches` already compiles in place", "delete far more than change" | The evaluation's corrections, accepted by the author. |
| Draft §8.2 against answers 8.2 | `Nil` "defined by the standard library" | Prelude (`lib/prelude.i`), because `lib/i` cannot import `std`. |
| Evaluation grammar against the rest | `ParamItem` admits `..rest` at the top of a parameter group | No rest at the top of a group, because shape groups must be syntactic (§2.4, [R4] Q7). |
| Answers B1 claim against this note's analysis | "Checking against the selected domains is sound whether or not known types are reliable" | Not true in general; see §2.6.6. The choice stands; the guarantee is restated and the invariant "known types are facts" is added ([R4] Q9). |
| Third round against fourth round on Q13 | The third round records `Node = (…)` "pending confirmation against the plain union name `Node = Name \| Int \| …`" | The fourth round settles it: with the outer parentheses, a labelled type. |
| Today's Lisp surface against the new rows | Today a pipe `a \|> f` is written `(\| a f)` in `.is` files (`lisp/to_ast.rs`:489, `std/src/test.is`), while SYN3 uses `\|` for the branch join | The pipe head becomes `\|>`; `(\| …)` in an expression is a chain (§6.8). |
| `meta.md` §2.3 against `spec.md` | The meta table writes a match as `e.{ p -> v }`; the spec and the code write `e \|> { … }` | The spec controls. The new rows in §6.8 replace both. |
| O16 against the effects proposal | Reads `db ! Db` in blocks and `! Db = e` | Later owner text: `<-` in heads, `e <- (s)` (O24, O29). Mid-block reads disappear (review gap 6). |
| Effects proposal `catch` against rule B | The proposal writes the body as `() <- (Fail(E)) -> A`, a closed need that rejects every body needing another service | The body is `() <- (..N, Fail(E)) -> A` (review Q1; confirmed by the coordinator). |
| Effects proposal item 3 ("no repeats") against its own `test` example | `app() <- (..live(), fake_db)` has `Db` twice | Rule C: an explicit item wins over a spread; two explicit items or two spreads with one key are an error (§2.10.4). |
| Effects proposal item 6 ("errors keep today's speed") against the review's measurement | Library `catch` through `suspend`/`cancel` measured 1.6x to 2.8x slower | Light escape-only boundary and a 1.25x bench gate (O30). |
| Effects proposal item 10 (`keep` "can store only the pointer") against the review Q4 | Restoring the whole pointer also restores the captured `Fail` | `keep` leaves `Fail` at the call; the provide overlays the callee's keys outside `N`; `keep_env` stays a primitive until stage E6 (§2.10.8). |
| Effects proposal R5 example against the grammar | `map = <A, B, N>(xs, f: (A) <- N -> B) <- N -> List(B)` writes a bare row variable in a head | A head has only a bracketed service pattern; a need passes down without being written (R2). `<- N` is a type form; the example is given as `map`'s type (§2.16). |
| Effects proposal `raise` against `i/fail` | `raise` binds its service as `fail`, which shadows the module name `fail` | The binder is `handler`; the functions keep the names `fail.raise` and `fail.catch` (§2.10.6). |
| §2.14 (host-service fields "may not have an environment") against review gap 8 | Host errors were `^ E` | Exactly `<- (Fail(E))` with a host-passable `E` is allowed (§2.10.12). |
| O1 ("every symbol has one meaning") against O23 | `<` and `>` are comparisons and also bracket type parameters | Accepted by the owner with plan A: `<…>` occurs only directly before a parameter part, where no comparison can be; P14 settles the start of a line. |
| Coordinator's first relay against the owner's generalisation | Provide first allowed only on a call or a block | Later owner text: `e <- s` for any `e`, weakest binary level (O29, §3.3). |

## 2. Principles and rules

### 2.1 One meaning per symbol

| Symbol | Only meaning |
|---|---|
| `{ }` | A computation in an expression (statements then a result, worked out in place); a group in a type. `{ }` creates every scope. |
| `( )` | Data: a value, a type or a pattern of keys to parts. A call's brackets are data. A parameter group is a data pattern. |
| `->` | The result part of a function or function type. |
| `\|` | Cases joined: union in a type, alternatives in a parameter item, branch join after a function body. |
| `=` | Binding, left to right: a statement, a key in data (`label = …`), a conjunction in a pattern, a type definition. |
| `:` | The pattern or name on the left has the type on the right. Never a label. |
| `?` | A guard, only at the end of a parameter group. |
| `<-` | What a function takes from outside: the need `<- (Db, Log)` in a function type, the service pattern `<- (db: Db)` in a function head, and `e <- (s)`, which works out `e` with services provided (§2.10). The mirror of `->` (O24). |
| `<` … `>` | Type parameters, only directly before the parameter part of a function literal or function type: `<A, N>(…)` (§2.16). Elsewhere `<` and `>` are the comparison operators, as today. |
| `..` | The rest of some data: spread in values, rest in patterns, list rest. |

### 2.2 Computation `{ }`

- In an expression, every `{` begins a block: `{ s1 … sn e }`. The statements
  run in order and the value is `e`. A block with only an expression is a
  group: `{ a - 48 } * 100`.
- In a type, `{ T }` is a group. It also turns a data type into a name in a
  type definition (§2.8) and is the multi-shape parameter of a function type
  (§2.4).
- There is no other `{` form. Today's brace decision (spec §6.1, five tests)
  and its exceptions are removed.
- A block is the only thing that creates a scope. A function body is a block,
  so parameters scope over the body only.

### 2.3 Data `( )`

**Keys and shapes.** A data value is a finite map from keys to parts. A key is
a position (0, 1, …) or a label (a `name`). The **shape** of some data is its
number of positions and its set of labels. Positions are numbered from 0 in
the order in which positional items are written; labels have no order.

```text
pair = (1, "a")                  // shape: 2 positions
ada  = (name = "ada", age = 36)  // shape: labels {age, name}
conn = ("a.com", port = 443)     // shape: 1 position, label {port}
unit = ()                        // shape: empty
one  = (pair)                    // shape: 1 position; one is not pair
```

- **Closed.** Data of one shape is never a value of a type of another shape.
  `(name = "a", age = 1)` is not a `(name = String)`. There is no width
  subtyping, no `Record`, no field addition or removal at run time.
- **Items.** `label = e` gives a label, `e` gives the next position,
  `..e` spreads (below). A label may not be given twice.
- **Order [R4].** In data values, data types and data patterns, an
  explicit positional item may not follow an explicit labelled item:
  `(port = 443, "a.com")` is a compile error. Spreads are exempt. This keeps
  today's rule for calls (spec §6.6).
- **No scope (O9).** Items are worked out in the outer scope, left to right in
  written order. A label makes no name. `(w = h, h = w)` swaps the outer `w`
  and `h`.
- **One-tuple.** `(x)` is data with one position. Grouping in an expression
  is only `{ x }`.
- **Unit.** `()` is the empty data: the map with no keys. Data is the one
  kind of thing written with `( )`, and `()` is that thing wherever it is
  written. It is not `Nil`, which is the prelude's labelled type `Nil = ()`.
- **Access.** `d.k` (a decimal) reads position `k`; `d.f` reads label `f`.
  Both need a known type (spec §4.8) whose members are all data types, or
  labelled types whose data is visible (§2.8), each with that key. The result
  is the union of those parts. This is today's rule 13 for `.f` generalized to
  every key, and today's rule for `.k`.
- **Spread.** `..e` in a data value contributes all keys of `e`. `e` must have
  a known type with exactly one member that is a data type or a labelled type
  whose constructor is visible. Its positions continue the positional
  numbering at that point; its labels are added. An explicit labelled item
  written after a spread replaces a label that the spread contributed; any
  other repeated key is a compile error. `e` is worked out once, in written
  order.

```text
older  = (..ada, age = 37)    // (name = "ada", age = 37)
triple = (..pair, True)       // (1, "a", True)
v      = User(..u, age = 37)  // a constructor call whose data has a spread
```

- **Data types** are written with the same items: `(Int, String)`,
  `(name = String, age = Int)`, `(String, port = Int)`, `()`. A data type has
  no spread.
- **Equality** (spec §11.4): data values are the same when they have the
  same shape and their parts under each key are the same. Data and a labelled
  value are never the same.

### 2.4 Functions

A function is a parameter group, an optional `->` part, and a block:

```text
add     = (a: Int, b: Int) -> Int { a + b }
inc     = (n) { n + 1 }
now     = () { clock.read() }
connect = (host: String, port = p: Int) -> Conn { open(host, p) }
```

- **One data parameter.** A function takes exactly one argument, a data
  value, and its parameter group is a data pattern matched against it
  (O2, O8). `f()` passes `()`; `f(x)` passes `(x)`.
- **Parameter items.** `x`, `x: T`, any pattern: a positional item.
  `label = p`: a labelled item whose part is matched by `p`. `(name: T)` is a
  positional binder (O11). There is no rest `..` at the top of a parameter
  group [R4]: the shape of a function must be syntactic for shape
  groups (§2.6). Rest is allowed in nested data patterns with a known shape.
- **Shape of a branch.** The shape of a function literal is the shape of its
  parameter group: its number of positional items and its set of labels.
- **Function types** drop the names and the body. The parameter part is a
  data type: `(Int, Int) -> Int`, `(String) <- (Db, Fail(NotFound)) -> User`,
  `(String, port = Int) -> Conn`, `() -> Int`. Labels are part of the type
  (O11). `(a: Int) -> Int` is a compile error: write `(Int) -> Int`.
- **Multi-shape function types (O20).** The parameter part may also be a
  group whose members are data types: `{ (Float) | (Float, Float) } -> Float`.
  This is the type of a multi-shape value (§2.6.5).
  `((Float) | (Float, Float)) -> Float` is a different type: one position
  whose part is a union of data.
- **No defaults.** `(port = 80)` in a parameter group means "the label `port`
  must be the literal 80", not a default. Optional parameters are written as
  shape overloading (§4.5).
- **Services.** A function has no environment parameters. The services it
  uses itself are named by its head's service pattern,
  `(id: String) <- (db: Db) -> User { … }`, after the parameter group and
  before `->` (§2.10). The service pattern is not part of the shape.
- **Type parameters.** A function literal may begin with `<A, N>`, which
  binds type parameters for its parameter group, service pattern, result and
  body: `id = <A>(x: A) -> A { x }` (§2.16).

### 2.5 Branches joined by `|`, and matching

**Chains.** `F1 | F2 | … | Fn`, where every `Fi` is a function literal, is
one function value, a **chain** (O4). A single function literal is a chain of
one branch. `|` may follow only the `}` of a function body, and the thing
after it must be a function literal (D3).

```text
gcd =
    (a: Int, 0) -> Int { a }
    | (a: Int, b: Int) -> Int { gcd(b, a % b) }
```

**Shape groups.** The branches of a chain are partitioned by their shape.
Each **group** keeps the written order of its branches. A group is one
ordinary function of one shape.

**Run time.** A call with argument data `d` uses the group with the shape of
`d` (this is exact, because shapes are fixed by types). In it, the first
branch whose parameter group matches `d` and whose guard, if any, is `True`
is taken, and its block is the result. The guard is worked out only after its
group matched, and it may use the group's names. Alternatives in an item are
tried in order.

**Typing of a group.** A group with exactly one branch and no guard is typed
as a single function: its parameter group is fitted (spec §4.8 rule 4,
"fit"), `x: T` in it is an annotation unless the item has alternatives, it is
not checked for coverage, and a refutable pattern reports to `Assert`
(§2.12). Every other group (two or more branches, or a guard) is typed by
rule 11 (§5.2.8) with the keys of its parameter data as its places, and must
cover its parameter type (spec §7.4). `x: T` in it is a type test.

**The value of a chain.** A chain whose branches all have one shape has the
function type `{ B } <- N -> R`, where `B` is the group's parameter data
type and `R` the union of its branch results. A chain with several groups has
the multi-shape type of §2.6.5. The need `N` (errors included, as `Fail(E)`)
is shared by all branches of a chain ([R4] Q15, R6).

**Matching.** `e |> C`, where the right side is a chain literal (one or more
function literals), is a **match**. It works out `e` and calls `C` with the
one-tuple `(e)`, in place: it adds no call frame, a call in a branch body of
a match in tail position is in tail position (spec §11.6), the match is not a
call in a stop message (spec §11.7), and a branch pattern `C(…)` that matches
a name subject gives that name the branch-local view of spec §7.5. So matching
uses the group of shape "one position". Branches of other
shapes in a matched chain literal can never run and give the "never used"
warning. A match is typed by rule 11 with all branches of that group; there
is no selection in a match [R4]. A match whose chain has
one branch and no guard is typed as the binding of that branch's item to
`e` followed by the body, as today (spec §4.8 rule 11, second paragraph).

```text
label = opt |>
    (Some(v) ? string.is_empty(v)) { "empty" }
    | (Some(v)) { v }
    | (None) { "none" }

kind = day |> (Sat | Sun) { "rest" } | (_) { "work" }
```

**Tuple subjects.** `x |> f` passes `(x)`, so a pair subject is matched by a
one-position pattern whose part is a pair: `p |> ((0, y)) { y } | ((x, _)) { x }`.
To match two values directly, write a two-position chain and call it.

**Nesting.** A branch body is always braced, so an inner chain ends at the
outer `}` (G9).

### 2.6 Overloading

Overloading is §2.5 applied to a named chain (O5). Patterns can distinguish
shape (arity, labels), types, constructors and literals; so can overloading.

```text
show =
    (n: Int) -> String { int.to_string(n) }
    | (s: String) -> Int { string.length(s) }
    | (a: Int, b: Int) -> Bool { a == b }
    | (host: String, port = p: Int) -> Conn { open(host, p) }
```

Groups: `(1 position)` has branches 1 and 2 (rule 11, `n: Int` and
`s: String` are tests); `(2 positions)` has branch 3 (fitted, `a: Int` is an
annotation); `(1 position, {port})` has branch 4 (fitted).

#### 2.6.1 The selection table

A binding has a **selection table** when its pattern is one name with no
written type, and its value, after removing groups `{ }`, is

- a chain literal; or
- a name (`x`, `m.x`, local, top-level or imported) whose binding has a
  selection table (a **direct alias**, see §2.6.7).

The table lists, for each branch in written order: its shape, its written
parameter pattern (for selection), its domain type `Dom_i` and its result
type `R_i` (for the solver). A binding with a written type at the top of its
pattern, `f: T = chain`, has no table: its uses have the type `T`
[R4]. A chain bound inside a data value or a list, or passed
as an argument, has no table.

#### 2.6.2 Selection at a direct call

A **direct call** is `f(d)`, `x |> f` or `x |> f(…)` where `f` is a name with
a selection table. It is resolved in this order, during the walk that finds
known types, before and independent of solving:

1. **Shape.** The shape `S` of the argument data is read from the call's
   syntax: positional items and labels, plus the keys of each spread `..e`,
   which need a known shape. `x |> f(…)` inserts `x` as position 0. If a
   spread has no known shape, it is a compile error.
2. **Group.** The group `G` of the branches with shape `S`. No such group is a
   compile error that lists the shapes `f` has.
3. **Known type.** `K` is the known type of the argument data: data of shape
   `S` whose part under key `k` is the known type `K_k` of that item, or `_`.
4. **Selection.** For each branch `i` of `G`, in order, and each key `k`:
   let `W_i,k` be the written pattern type of branch `i` at `k` (spec §4.8
   "pattern type", in the rule-11 sense: a literal has its literal type; a
   catch-all has "every value"), and `T_i,k` the members **taken** at `k` for
   branch `i` (below). Branch `i` is **skipped** when for some key `k`, `K_k`
   and `W_i,k - T_i,k` are disjoint (spec §4.5 rule 3); for a catch-all, that
   is when every member of `K_k` is a member of `T_i,k`. A `_` part of `K`
   is never disjoint from anything. Every branch not skipped is **selected**,
   including guarded ones.
5. **No selection** is a compile error naming the known argument types.
6. **Typing.** The call is typed as a call of
   `F_sel = { Dom_i1 | … | Dom_ij } <- N -> R_i1 | … | R_ij` over the
   selected branches, where `Dom_i` is the parameter type of branch `i` in its
   group (for a fitted group, its pattern type; for a rule-11 group, the
   group's place types with the branch's pattern types at its non-catch-all
   places and the narrowed types of rule 12 at its catch-alls). The solver
   then sees an ordinary function type.

**Taken.** This is spec §4.8 rule 12 applied at every place, not only at
catch-alls. The candidates at key `k` are the type constructors, the `Int`
and `String` literal values and the type-test members that occur in
`W_i,k` (when branch `i` is not a catch-all at `k`) or at `k` in an earlier
unguarded branch of the group. A candidate is taken at `k` for branch `i`
when every value that branch `i`'s parameter pattern matches with a value of
that candidate at `k` is matched by an earlier unguarded branch of the same
group. Guarded
branches never take anything. `coverage.rs` already computes this for
catch-alls (`types/mod.rs`:13-14).

Results of the examples:

| Call | Selected | Type |
|---|---|---|
| `show(1)` | branch 1 (branch 2: `String` disjoint from `1`) | `String` |
| `show(1, 2)` | branch 3 | `Bool` |
| `show("a.com", port = 80)` | branch 4 | `Conn` |
| `show(x)` with `x: Int \| String` | branches 1, 2 | `String \| Int` |
| `f(1)` with `f = (x: Int) { "a" } \| (x: Int \| String) { 1 }` | branch 1 (branch 2: `Int` taken, `String` disjoint from `1`) | `String` |
| the same `f` with a guard on branch 1 | branches 1, 2 | `String \| Int` |
| `gcd(b, a % b)` | both (`0` taken at key 1 for branch 2 leaves `Int - 0`, not disjoint from `Int`) | `Int` |
| `gcd(5, 0)` | branch 1 (branch 2: `Int - 0` disjoint from `0`) | `Int` |
| `f(Some(5))` with `f = (Some(0)) { "zero" } \| (Some(n)) { n }` | both (`Some(5)` and `Some(0)` are not disjoint; `Some` not taken) | `String \| Int` |

#### 2.6.3 Fallback

- With no known type at all, nothing is skipped and the whole group is
  selected. `h = (x) { show(x) }`: the call has shape (1 position), `x` has
  no known type, so branches 1 and 2 are selected and
  `h : (Int | String) -> String | Int` (answers 1c). This is the owner's
  accepted fallback ("挑分支的位置看不到类型时，退回全部分支的并集").
- To be precise, give `x` a type or write `h` as a chain too.
- Run time is always first match in written order (§2.5); selection only
  computes a type.

#### 2.6.4 Selection by an expected type

When a name with a table is used other than as a callee and its place has an
expected type (spec §4.8 "Expected types") with exactly one member, a
function type with parameter part `P`:

- for each data member of `P` whose shape `S` the name has, run steps 2 to 4
  of §2.6.2 with `K` = that member, made whole as §4.8 says for expected
  types; the value then has the type `{ Dom… } -> R…` over the selected
  branches of those groups;
- if no member of `P` has a shape the name has, the value has its full type
  (§2.6.5) and the later constraint fails as a type error.

Arguments that are names with a table of more than one branch are checked in
the second round, with the function expressions whose parameter has no
written type (spec §4.8; `known.rs` `open_fn`). So in
`list.map([1, 2], show)`, `A` is fixed to `Int` by the first argument, the
expected type of `show` is `(Int) -> B`, branch 1 is selected, and the result
is `List(String)` (O6). `apply(show, 1)` works the same way (answers 1b).

#### 2.6.5 The type of a multi-shape value

A name with a table, used as a value with no expected type that selects (for
example `fs = [area]`), or a chain literal with several groups, has the type

```text
{ B_g1 | … | B_gm } <- N -> R_all
```

where `B_g` is the parameter data type of group `g` and `R_all` the union of
all branch results (evaluation §4.3, D10). It is a supertype of every branch.
A call of such a value with argument data of shape `S` constrains the data
against the member of shape `S`; the result is `R_all`. So with
`fs = [area]` and `Some(f) = list.first(fs)`, `f` has this type, and `f(2.0)`
and `f(3.0, 4.0)` both have the type `Float`. For `show`,
`Some(g) = list.first([show])` gives `g(1) : String | Int | Bool | Conn`.

Function types follow the lattice rules of spec §4.4 with one data parameter:
`{ P } -> R | { Q } -> S` is `{ P & Q } -> R | S`, and for different shapes `P & Q`
is `Never`, so `[area, show]` is legal and its elements cannot be called. A
single-shape function is not a subtype of a multi-shape type (its parameter
type is smaller), and a multi-shape type is a subtype of each of its shapes,
`{ (Int) | (Int, Int) } -> R <: (Int) -> R`. So a multi-shape value can reach
any call site, including inside a library compiled without knowledge of it;
the run time handles it there (§6.4).

`list.first` gives an `Option` (std/src/list.i:158), so the runnable form of
the owner's example is `Some(f) = list.first(fs)` followed by `f(2.0)` and
`f(3.0, 4.0)`, a refutable binding that reports to `Assert` (§2.12). The
owner's sketch leaves this out; it does not change the decision.

#### 2.6.6 What selection guarantees

Selection skips a branch only when the known type shows that no argument
value can make that branch the first match: if `v` makes branch `i` the first
match, then at every key `k`, `v_k` is in `W_i,k` and its member is not taken
(else an earlier unguarded branch matches `v`). So selection is sound when
**known types are facts**: every value an expression can have at run time is
in its known type. The spec intends this (known types come from written types,
schemes and matched patterns) but does not state it. [R4] The spec states it
as a rule of §4.8: **every value an expression can have at run time is in its
known type.** It is tested with the generated families of
`scripts/test-generation`; selection, narrowing and `.k` depend on it.

The solver then checks the argument against the selected domains
(answers B1). This makes the inferred argument type agree with the selection,
and it rejects a correct program when the known type is narrower than the
inferred type: with `f = (0) { "z" } | (n: Int) { n }`, `x = 0 = n` and
`n: Int`, `f(x)` selects branch 1 and then needs `Int <: 0`. It does **not**
make selection sound by itself if known types were not facts: with
`h = (0, 0) { "a" } | (a: Int, b: Int) { 1 }`, the domains of the two
branches overlap, so a wrong known type `(1, 0)` for an actual `(0, 0)` would
pass the check and run branch 1. The answers file says the check is sound
"whether or not known types are reliable"; that is not the case, and the
correct statement is the one above. The choice of the check is kept.

#### 2.6.7 Direct aliases stay precise

**Decision (this note):** `g = show; g(1)` is `String`.

How known definitions work in the checker today: a name carries, besides its
type, facts read from its **written binding**. `Scheme` has `key_slots`
(`types/infer.rs`:44-49). For a top-level definition it is computed from the
binding's written value (`key_slots`, infer.rs:111-130, filled at
infer.rs:690-700 and 718-728); for a local, from the binding when it is
checked (`label_key_slots`, infer.rs:3113-3116); for an external, from the
written type (infer.rs:354). A call reads them through
`label_slots_ref` (infer.rs:1985-2014), which goes from a `ValueRef::Def` to
the binding in the HIR. A name with key slots may only be a callee
(`LABEL_PARAMETER_VALUE`, infer.rs:1717-1723): the fact is not part of the
type. The known type of a name is a `Ty` (`kvars`, or the scheme for
imports), and a `Ty` cannot hold a branch list.

The selection table is the same kind of fact and is carried the same way: a
new `Scheme.branches` field, computed from the written binding, kept for
locals in a map beside `label_key_slots`, and written to the package
interface (`scheme_wire.rs`) because imported overloads must be precise at
direct calls anyway. Following a direct alias is one more case of reading the
written binding: when the value is a name, take that name's table. Cycles
cannot occur: `g = h  h = g` is already a compile error, since each is among
the other's needs (spec §2.5). So the cost is a lookup per alias, and the rule
is easy to state: precision is lost exactly where the value passes through a
type (an annotation, a parameter, data, a list, a function result), never by
renaming.

### 2.7 Destructuring

"As it is made, so it is taken apart" (draft §3.6). Patterns are data
patterns with the same items.

1. **Conjunction.** `Pattern = Part { "=" Part }`: all parts match the same
   value, and each binds its names (O13). `a = b = c = d` as a statement binds
   `a`, `b` and `c` to `d`.
2. **Key exception.** Directly inside a data pattern (a parameter group, a
   data pattern, a constructor pattern), an item whose first two tokens are a
   `name` and `=` is a **key item**: the name is a label, and the rest is the
   pattern for that label's part.
3. **Consequence.** Inside a data pattern, a leading name is never a binder of
   the whole item. To name a whole positional item, put the name last:
   `((a, b) = args)`. `(size = whole = (w, h))` is "label `size`, whose part
   matches both `whole` and `(w, h)`". A positional item that is a
   conjunction of names starts with a discard: `(_ = x = y)`.
4. **Statement level.** Outside data patterns, `t = (a, b) = e` binds `t` and
   `(a, b)`. The statement reads pattern parts while each is followed by `=`;
   the first part not followed by `=` begins the value (G12).

```text
(x, y) = pair
(name = n, age = a) = ada
(first, ..rest) = (1, "a", True)   // rest = ("a", True)
box = (size = (3, 4))
(size = whole = (w, h)) = box      // whole = (3, 4), w = 3, h = 4
connect = (host, port = p) { open(host, p) }
run = (mode = "fast") { quick() } | (mode = "slow") { careful() }
log_all = ((a, b) = args) { log(args) }
t = (a, b) = pair
```

- **All keys or `..`.** A data pattern lists every key of its shape, or ends
  with `..` (ignore the rest) or `..name` (bind the rest). Rest is data: the
  remaining positions renumbered from 0 in order, and the remaining labels.
  Rest and `..` need the shape of the part to be known (§1.3); in a
  constructor pattern the shape is known from the definition, so `C(x, ..)`
  and `C(..)` are always allowed.
- **Constructor patterns** take their data by the same items:
  `User(name = n, age = a)`, `User(age = a, ..)`, `Some(v)`, `Pair(x, y)`.
  A labelled field is matched only by its label; there is no punning
  (O12): `User(name, age)` is a shape error.
- **No shorthand** for `name = name` (O12). No spelling is reserved by this
  note; the author's `=:` family is rejected by the owner (answers D2).
- **Alternatives** `p1 | p2` are allowed only in the items of a parameter
  group (G20); `|` is tighter than `,` and looser than `=`. Every
  alternative binds the same names. A type test is the last alternative
  (G21).

### 2.8 Type definitions

One form: `Name [ (params) ] = right side` (O14). The kind is decided by the
**outermost written form** of the right side, before any expansion, in this
order:

1. **Alias pattern.** The right side, after removing groups, is a value type
   (`size`, `m.size`): `Codec(Size, _) = size` keeps today's meaning
   (spec §4.3, value-types.md). Its head is a list of type patterns.
2. **Opaque labelled type.** The right side is the word `Opaque`, not in a
   group: `Element = Opaque`, `Handle(A) = Opaque`. Its values come from the
   host; i code cannot make or take them apart. `Opaque` anywhere else, and a
   type named `Opaque`, are compile errors.
3. **Labelled type with data.** The whole right side is one data type, not in
   a group and not followed by `->`: `Some(A) = (A)`, `None = ()`,
   `User = (name = String, age = Int)`, `Pair(A, B) = (A, B)`. The left side
   is a new nominal type and its one constructor, with the same name.
4. **Name.** Anything else: `Meters = Int`, `Wrapper(A) = A`,
   `Option(A) = Some(A) | None`, `Handler = (Request) -> Response`,
   `Point = { (Int, Int) }`, `Wrap = (Int) | Nil`. A name is a transparent
   alias, as today.

For kinds 2 to 4 the head is a list of distinct unqualified `UpName`s with no
arguments (type parameters). In a name (kind 4), every parameter must be used
and every type variable must be a parameter (spec §4.3, unchanged). In a
labelled type (kinds 2 and 3), a parameter need not be used (a phantom), and a
parameter that is not used in the data is invariant; an `Opaque` parameter is
invariant (§1.3). Other parameters of labelled types get their variance from
the data by the simultaneous variance rule of spec §4.5.

- **Constructors.** A labelled type with data `D` has the constructor
  function `{ D } -> T(…)` (spec §4.6): `Some : (A) -> Some(A)`,
  `User : (name = String, age = Int) -> User`. When `D` is `()`, the
  constructor is a value: `None`, `Nil`, `True` (D5); `None()` is a compile
  error, in expressions and in patterns.
- **Sums are unions.** `Option(A) = Some(A) | None` is a name for the union.
  `Option(A) <: Maybe(A)` follows from set inclusion when both name the same
  members. One constructor may be in several sums (O14).
- **Recursion.** A name may not occur in its own expansion, directly or
  through other names (spec §4.3, unchanged). Because a labelled type's data
  is in its own definition, a name may reach itself through a labelled type's
  data: `Tree(A) = Leaf | Node(A)` with `Node(A) = (Tree(A), A, Tree(A))` is
  correct (it works in today's checker, evaluation §3, RAN a1). `T = List(T)`
  and `T = Box(T)` are cycles. A labelled type may use itself in its data.
- **Field reads and spreads** on a labelled value need its constructor to be
  visible (exported with `(..)`), as today's field selection and update do
  (spec §6.7).
- **One definition per name.** A labelled type is its own constructor, so
  `ParseError = (String)` needs no wrapper. Two definitions of one name in a
  module are an error (spec §2.2). A top-level type covers a built-in type of
  the same name in its module (spec §5), so `Int = (Int)` is a labelled type
  whose data contains itself, not a wrapper of the built-in `Int`.
- **Qualified built-in types.** `i/core` exports the built-in types. After
  `(core = i/core)`, `core.Int` names the built-in even when a local type
  named `Int` shadows the unqualified name.
- **Sums whose members are not constructors (O21).** A member of a sum need
  not be a labelled type made for it: built-in types (`Int`, `Float`,
  `String`, `List(A)`, …), data types and labelled types of other sums may be
  members directly, and values are told apart by type tests (spec §7.1) and
  constructor patterns. This works only when **every two members have
  different type constructors**: members with one type constructor are put
  together into one (spec §4.4, `List(Int) | List(String)` is
  `List(Int | String)`), all function types are one type constructor, all data
  of one shape is one, and a type test may not have a type variable as a
  member. A type test that would have to tell such members apart is already
  a compile error (spec §4.8 rule 11, `x: Box(Int)` against `y: Box(String)`).
  So the practice is: wrap in a labelled type exactly those members that
  would share a type constructor with another member, and leave the others
  bare. `i/syntax.Node` is the example:

```text
Node = (Name | Discard | Up | Symbol | Int | Float | Text | Round | Square | Curly | Bits | Quote | Unquote | Splice)
Name = (String)
Discard = (String)
Up = (String)
Symbol = (String)
Text = (String)
Round = (List(Syntax))
Square = (List(Syntax))
Curly = (List(Syntax))
Bits = (List(Syntax))
Quote = (Syntax)
Unquote = (Syntax)
Splice = (Syntax)
```

  `Int` and `Float` are the built-in types. The five `String` members, the
  four `List(Syntax)` members and the three `Syntax` members stay labelled
  types, because each group shares one type constructor (O21 names `String`
  members and Round, Square, Curly; Bits, Quote, Unquote and Splice follow
  from the same condition). `Node` itself is a labelled type whose data is one
  position of the union type (O21's outer parentheses): its values are
  `Node(5)`, `Node(Name("x"))`, and a match takes them apart with
  `(Node(n: Int))` and `(Node(Name(t)))`. `Node(..)` exports `Node` and its
  constructor; because its data names `Name` … `Splice`, those must be
  exported too (spec §2.4), each with `(..)` for macros to use them:
  `Node(..), Name(..), Discard(..), Up(..), Symbol(..), Text(..), Round(..),
  Square(..), Curly(..), Bits(..), Quote(..), Unquote(..), Splice(..)`. The
  whole-sum export `A(..)` (D8) is for names only, so it does not apply to
  `Node`. `node` and `make` in `lib/i/syntax.i` become:

```text
node = (syntax: Syntax) -> Node {
    raw_kind(syntax) |>
        (0) { Node(Name(raw_text(syntax))) }
        | (1) { Node(Discard(raw_text(syntax))) }
        | (2) { Node(Up(raw_text(syntax))) }
        | (3) { Node(Symbol(raw_text(syntax))) }
        | (4) { Node(raw_int(syntax)) }
        | (5) { Node(raw_float(syntax)) }
        | (6) { Node(Text(raw_text(syntax))) }
        | (7) { Node(Round(children(syntax, 0, raw_length(syntax), []))) }
        | (8) { Node(Square(children(syntax, 0, raw_length(syntax), []))) }
        | (9) { Node(Curly(children(syntax, 0, raw_length(syntax), []))) }
        | (10) { Node(Bits(children(syntax, 0, raw_length(syntax), []))) }
        | (11) { Node(Quote(raw_child(syntax, 0))) }
        | (12) { Node(Unquote(raw_child(syntax, 0))) }
        | (13) { Node(Splice(raw_child(syntax, 0))) }
        | (_) { panic("syntax: invalid node kind") }
}

make = (node: Node, like: Syntax) -> Syntax {
    result = node |>
        (Node(Name(value))) { raw_make_text(0, value, like) }
        | (Node(Discard(value))) { raw_make_text(1, value, like) }
        | (Node(Up(value))) { raw_make_text(2, value, like) }
        | (Node(Symbol(value))) { raw_make_text(3, value, like) }
        | (Node(value: Int)) { raw_make_int(value, like) }
        | (Node(value: Float)) { raw_make_float(value, like) }
        | (Node(Text(value))) { raw_make_text(6, value, like) }
        | (Node(Round(items))) { raw_make_list(7, build_list(items, raw_list_new()), like) }
        | (Node(Square(items))) { raw_make_list(8, build_list(items, raw_list_new()), like) }
        | (Node(Curly(items))) { raw_make_list(9, build_list(items, raw_list_new()), like) }
        | (Node(Bits(items))) { raw_make_list(10, build_list(items, raw_list_new()), like) }
        | (Node(Quote(item))) { raw_make_one(11, item, like) }
        | (Node(Unquote(item))) { raw_make_one(12, item, like) }
        | (Node(Splice(item))) { raw_make_one(13, item, like) }
    raw_syntax_ok(result) |>
        (True) { raw_syntax_value(result) }
        | (False) { fail.raise(SyntaxError(message = raw_syntax_error(result), at = like)) }
}
```

  In `make`, the group has 14 branches, so `value: Int` and `value: Float`
  are type tests; the place under `Node`'s position 0 has the type of the
  union and the branches cover it. In `node`, `raw_kind(syntax)` is an `Int`
  and the last branch is a catch-all. `Int` and `Float` in `i/syntax` now
  mean the built-in types, so no definition in the module covers them.
- **Exports** (§5.2.1). `T` exports the type only; `T(..)` exports a labelled
  type and its constructor. A name `A` is exported alone; `A(..)` exports the
  name and each member of its expansion with `(..)`, and is a compile error
  unless every member is a labelled type of this module (D8). An exported
  definition may not expose a private labelled type (spec §2.4, unchanged), so
  exporting `Option` alone requires `Some` and `None` to be exported at least
  as types.
- **The prelude.** `True = ()`, `False = ()`, `Bool = True | False`,
  `Nil = ()`, `Ok(A) = (A)`, `Error(E) = (E)`, `Result(A, E) = Ok(A) | Error(E)`,
  `AssertionFailure = (message = String)`,
  `Assert = (fail = (AssertionFailure) -> Never)`,
  `Fail(E) = (raise = (E) -> Never)` (§2.10.6).

### 2.9 Guards

- A guard is `? e` at the end of a parameter group, after the last item:
  `(a: Int, b: Int ? a > b)`, `(Some(v) ? string.is_empty(v))` (O17). It ends
  at the group's `)`.
- `?` is allowed only at the top level of a parameter group, never in a
  nested data pattern or in a binding statement's pattern.
- The guard has type `Bool`, may use the names of its group and of its
  branch's service pattern, and is worked out only when the group matched.
  Its need is part of the need of the chain (R6).
- A group with a guard is typed by rule 11 and must cover; a guarded branch
  is not counted for coverage and takes nothing (§2.6.2). So a lone guarded
  function is never covered, as `{ x ? c -> x }` is not today.

### 2.10 Services, errors and the environment

`->` is what a function gives; `<-` is what it takes from outside (O24). What
the outside gives is called a **service**: printing, logs, a database, errors
(`Fail`), assertions (`Assert`), generators (`Yield`), resources (`Scope`)
are all services. The language has services and the control operations of
`i/control` (§2.10.7), and nothing else for effects: no error channel, no
implicit capabilities (O26), no layer concept (§2.10.10).

#### 2.10.1 Writing

```text
find_user = (id: String) <- (db: Db, log: Log) -> User {
    log.write(id)
    db.find(id)
}
// find_user : (String) <- (Db, Fail(NotFound), Log) -> User
```

- **Head.** `(parameters) <- (service pattern) -> R`: the function names only
  the services it uses itself, directly. Every item is `name: NamedType`; the
  annotation is required (§3.3 `SvcPat`).
- **Type.** `(P) <- (S) -> R`: the whole need, inferred. In the example `Db`'s
  field `find` has the type `(String) <- (Fail(NotFound)) -> User`, so the need
  of `find_user` has `Fail(NotFound)` although the head does not name it.
- **Call.** `f(x)` is an ordinary call: the services come from the current
  environment, and nothing marks the call (O25). `f(x) <- (s)` provides or
  replaces services for that call (§2.10.5).
- **Nothing.** A function that takes no service, and a call that provides
  none, write no `<-` (O25).
- **Reads are only in heads.** There is no read statement in a block; today's
  `x ! T` reads in the middle of a block (the first version of this note) are
  removed. A function that needs a service only to pass it on writes nothing:
  the need passes down by R2.

#### 2.10.2 Keys

In needs, service patterns and provided data, every item is keyed by the type
constructor of its own type (O27). Order does not count and there are no
names or labels.

- `<- (Db, Log)` is the same type as `<- (Log, Db)`.
- In a service pattern, `db: Db` keys the item by `Db`: the written type is
  the key.
- In provided data, `<- (fake_db)` keys the item by the known type of
  `fake_db` at that point (spec §4.8 known types, [R4] Q9). That type must be
  one service type: not a union, not a type variable, not unknown. An
  unannotated parameter used as `<- (x)` is a compile error whose message asks
  for a written type.
- A labelled item after `<-` is a compile error. Needs are displayed in the
  UTF-8 byte order of the qualified names of their constructors, then the row
  variable: `<- (Db, Fail(NotFound), Log, ..N)`.
- Ordinary data keeps only positions and labels as keys (§2.3). Keying by
  type happens only after `<-`.

#### 2.10.3 Rules R1 to R6 (owner, O28)

- **R1.** A service is an ordinary value; a service type is an ordinary
  labelled type (§2.8). Two services of one type are made by defining two
  types.
- **R2.** The environment goes with the call and is dynamic, as in Effect-TS
  and today's spec §9.2. A need that a head does not write still passes down,
  and inference writes it into the type.
- **R3.** `f(x) <- (s)`: the environment of this call is the current
  environment plus `s`. For a key that `s` has, `s` wins; the other keys come
  from outside.
- **R4.** When a need has one constructor twice, the two are merged by
  direction (§2.10.4): `Fail(A)` and `Fail(B)` give `Fail(A | B)`; a place that
  can provide `Fail(A | B)` satisfies code that needs only `Fail(A)`. For a
  parameter that does not change (invariant), the two must be equal, else it
  is a compile error. This replaces spec §4.5 rule 4's "service arguments are
  compared as equal" and spec §9.2's "one constructor may not have different
  arguments in one environment".
- **R5.** Row variables. A type parameter can stand for a whole need:
  `map : <A, B, N>(List(A), (A) <- N -> B) <- N -> List(B)`, and a need can be
  written `<- (..N, Db)`. The environment is open in this sense; data
  everywhere else stays closed (O7).
- **R6.** The branches of a chain share one need ([R4] Q15, unchanged). Each
  branch may write its own service pattern; the need of the chain is the
  merge of all of them.

#### 2.10.4 Typing of needs (review rules A, B, C)

A **need** is a finite map from service type constructors to service types,
plus at most one row variable. A computation has a value type and a need,
written `A <- M` in this note (today's `A ^ E ! M` with `E` moved into `M` as
`Fail(E)`).

- **Merge (owner ruling).** A need is the intersection of its services:
  `<- (Db, Log, Fail(E))` is `Db & Log & Fail(E)`, and `<- (..N, Db)` is
  the open intersection `N & Db`. The merge of two needs is their
  intersection (spec §4.4).
- **Satisfying.** A provided value of type `P` satisfies the need entry `S`
  when `P <: S` as values. Because `Fail` is contravariant,
  `Fail(A | B) <: Fail(A)`.
- **Rule A (direction of needs).** In function subtyping
  `(P) <- N -> R <: (Q) <- M -> S` (replacing spec §4.5 rule 4): `Q <: P`,
  `R <: S`, and `N` **fits in** `M`: for every constructor `C` with an entry in
  `N`, `M` has an entry for `C` and `M_C <: N_C`, or `N`'s row variable and
  `M`'s row variable are the same and carry `C`. Needs are compared in the
  reverse direction of value subtyping. Spec.md:739's sentence ("Service
  arguments are compared as equal, not by direction …") is replaced by this
  rule.
- **Rule B (a row variable lacks the written keys).** In `(..N, C(…))`, `N`
  has no entry for `C`. The solver keeps the "handled" obligation on row
  variables (`solve.rs`:630-658) with the equality replaced by the directed
  constraint; a rigid row variable that reaches a provide of `C` without a
  stated `C` entry is a compile error. Without rule B, `catch` (§2.10.6)
  could be solved as `E = Int`, `N = (Fail(String))` for a body that raises
  `1` and `"s"`, and would return a `String` error typed `Int`.
- **Rule C (provided data).** In provided data an explicit item wins over an
  item from a spread with the same key; two explicit items, or two spreads,
  with one key are a compile error. A provided value that is not a data
  literal (`<- s`, `<- live()`) must not have two parts with one key.
- **Dropped entries.** A need entry `C(T…)` such that every value of every
  `C(…)` type is a value of it (`Fail(Never)`, since `Fail` is
  contravariant) is removed when needs are simplified. Nothing can use it.
- **Functions** (replacing spec §4.8 rule 4's environment parameters). If
  `Γ`, with the names of the parameter group and with `xi : Ti` for every
  service item, gives `body : A <- M`, the function has the type
  `(P) <- M' -> A` where `M'` is the merge of `M`, every `Ti`, and `Assert`
  when the parameter group may fail. With `..env: N` in the head, `M` must fit
  in `(..N, T1, …)` and `M'` is `(..N, T1, …)`.
- **Calls.** If `f : (P) <- N -> A` and the data `d : D`, `D <: P`, then
  `f(d) : A <- N`.
- **Provide.** For `e <- s` where `e : A <- N` and the provided data has keys
  `K` with types `T_k`: for every `k` in `K` with an entry in `N`, `T_k <: N_k`;
  the provide has the type `A <- (N without K)` merged with the need of working
  out `s`. A provided key that `N` does not have is a warning ("not needed").
  If `N` is or contains a rigid row variable, rule B applies.

#### 2.10.5 Providing: `e <- s` (owner, O29)

- `e <- s` works out `e` with the current environment plus `s` (R3), for any
  expression `e`. `s` is worked out first, in the outer environment; then `e`.
  `<-` is the weakest binary level, left associative (§3.3).
- `f(x) <- (s)` provides to one call: today's environment statement scoped to
  the call (`LetEnv` / `$env_apply`, `lower.rs`:2751-2765), one array copy per
  provide.
- `{ … } <- (s)` provides to a block. It **replaces** the statement `! T = e`
  of today's language: every statement and the result of the block run with
  `s` provided. Every provide compiles as today's `LetEnv` over its left
  operand: no thunk, no call frame, so stop messages and frames are as today.
- The function head's service pattern and a provide never meet (P1, G45).

```text
check_find = () {
    {
        u = find_user("1")
        assert(u.name == "ada")
    } <- (Db(find = (id) { User(name = "ada", age = 36) }), quiet_log)
}
```

#### 2.10.6 Errors: `Fail` (owner, O27; `^` removed)

The error part `^ E` of a function type is removed. An error is raised through
the service `Fail(E)`. The prelude defines the type; `i/fail` defines the two
functions as ordinary i code, keeping today's names so that the call sites
`fail.raise(e)` and `fail.catch(f)` keep their text (this note):

```text
// prelude
Fail(E) = (raise = (E) -> Never)

// lib/i/fail.i
raise = <E>(error: E) <- (handler: Fail(E)) -> Never { handler.raise(error) }

catch = <A, E, N>(body: () <- (..N, Fail(E)) -> A) -> Result(A, E) {
    control.boundary((b) {
        Ok(body() <- (Fail(raise = (e) { control.escape(b, Error(e)) })))
    })
}
// fail.raise : <E>(E) <- (Fail(E)) -> Never
// fail.catch : <A, E, N>(() <- (..N, Fail(E)) -> A) <- N -> Result(A, E)
```

- `catch`'s body type is `() <- (..N, Fail(E)) -> A`, with the row variable;
  the proposal's closed `() <- (Fail(E)) -> A` would reject every body that
  needs another service (§1.4). By rule B, `N` has no `Fail`, so `catch` takes
  every error of its body, as `fail.catch` does today.
- `fail.from_result : <A, E>(Result(A, E)) <- (Fail(E)) -> A` and
  `fail.map_error : <A, E, E2, N>(() <- (..N, Fail(E)) -> A, (E) <- N -> E2) <- (..N, Fail(E2)) -> A`
  are library code over `catch` and `raise`.
- `Assert` stays a separate service type, so `catch` does not take assertion
  failures, as today.
- **Library style.** An operation that may fail reports through `Fail`;
  `Result` is data used after `catch`. About 140 functions of `std/src` and
  `lib/i` that return `Result` today (`json.parse`, `float.divide`, …) change
  in stage E4.
- **A `Fail` that outlives its `catch`.** A `Fail` value returned, stored, or
  captured by a function that runs after its `catch` ended stops the program
  with "boundary not active" when used (§2.10.7). Today an error always reaches
  the nearest live `fail.catch`. A warning is given when a `Fail` bound in a
  service pattern is captured by a function value or put into data; `raise`
  only uses it.
- **Language server.** Hover lists the `Fail(…)` entries of a need separately
  as "possible errors"; an inlay hint after a call shows the services that
  step takes (§6.6).

#### 2.10.7 Control

`i/control` keeps `boundary`, `suspend`, `resume` and `cancel`, and adds
`escape`. The error parameter leaves `Boundary` and `Continuation`:

| Function | Type |
|---|---|
| `control.boundary` | `<R, N>((Boundary(R, N)) <- N -> R) <- N -> R` |
| `control.suspend` | `<A, R, N>(Boundary(R, N), (Continuation(A, R, N)) <- N -> R) -> A` |
| `control.resume` | `<A, R, N>(Continuation(A, R, N), A) <- N -> R` |
| `control.cancel` | `<A, R, N>(Continuation(A, R, N)) <- N -> Nil` |
| `control.escape` | `<R, N>(Boundary(R, N), R) -> Never` |
| `control.keep_env` | `<S, A, E, N>((S) <- (..N, Fail(E)) -> A) <- N -> (S) <- (Fail(E)) -> A` |

- **Variance** (replacing spec §4.5 rule 7's second sentence): in
  `Continuation(A, R, N)`, `A` goes in the other direction and `R` and `N` in
  the same direction. `Boundary(R, N)` does not change.
- **`escape(b, v)`** means `suspend(b, (k) { cancel(k); v })` done at once,
  but it throws directly and records no frames. If `b` is not active the
  program stops. Final functions (`resource.ensure`, §9.5) on the way run as
  for an error today. Compiled form: check that `b` is active, throw the
  existing `$error` tag with the pair `(b, v)`; `$boundary_core` compares `b`
  and gives `v` on a match, else throws again. Boundary identity is required:
  a `Fail` whose `raise` were a bare throw would be taken by an inner `catch`
  for another `E`.
- **Light boundary (owner, O30).** A call `control.boundary(f)` where `f` is a
  function literal whose parameter `b` is used only as the first argument of
  `control.escape` (directly or inside nested function literals) is compiled
  as an escape-only boundary: no continuation state, and the `Fail` value, its
  closure and the environment copy of `catch` may be one allocation. The
  decision is read from the literal's syntax, so it applies to every library,
  not by name to `catch`. **Bench gate:** the `scripts/bench` cases
  "`fail.catch` with no error" and "`fail.catch` + `fail.raise`", run on the
  library `catch` and `raise`, are each at most 1.25x the same cases on
  today's built-in `fail.catch` at stage E2's base. A stage that misses the
  gate is not accepted.
- **Removed built-ins.** `fail.raise` and `fail.catch` stop being built-in:
  `Op::Raise`, `Op::Catch`, `$fail_catch`, `$catch_run`, `$catch_reenter` and
  frame kind 4 are deleted (stage E3). The `$error` tag stays, for `escape`.
- Spec §9.4 rule 3 (the environment of a resumed computation) is unchanged;
  with errors as services, "errors after `resume` go to the handler of the
  place of `resume`" follows from it.
- `$env` passing is unchanged: a dense array with one slot per service type
  constructor, an extra parameter of every function (`lower.rs`:819-850).

#### 2.10.8 `..env` and `keep`

- `..env: N` binds the projected row value of spec §6.8 (owner:
  `<- (..env)` replaces `keep_env`). `g() <- env` provides it again;
  `<- (..env, fake)` overrides one key of it (rule C).
- A value whose type is a row variable is not data: it may be bound, captured
  and provided after `<-` (alone or as a spread in provided data); it may not
  be spread into ordinary data, compared, printed or matched.
- [R4] Q7 (no rest at the top of a parameter group) applies to the parameter
  group only: a service pattern is not part of a branch's shape.

```text
keep = <S, A, E, N>(f: (S) <- (..N, Fail(E)) -> A) <- (..env: N) -> (S) <- (Fail(E)) -> A {
    (x) { f(x) <- env }
}
```

- `Fail` stays at the call of the kept function, so an error raised inside it
  goes to the caller's `catch`, as with today's `control.keep_env`. This is
  sound only under rule B.
- **Implementation.** Capture uses spec §9.4: retain only the services in
  the inferred residual need. A stored environment must not retain unrelated
  services through its backing pointer. `f(x) <- env` overlays the retained
  services with the caller's `Fail`. Task substitutions are in spec §9.9.
- **`keep_env` stays a primitive** with the type of §2.10.7 until stage E6,
  where it is deleted only if `tests/spec/9/keep_env.i` and a
  `task.keep` error-propagation fixture pass on the library `keep`. The seven
  uses are in `lib/i/task.i` (1107, 1114, 3097, 3297, 3551, 3568, 3575).

#### 2.10.9 Resources: `Scope`

- `Scope` is a service of `i/resource`. `resource.acquire(open, release)`
  opens a resource and attaches `release` to the current `Scope`;
  `resource.scoped(body)` runs `body` with a new `Scope` and runs the attached
  releases in reverse order when `body` ends, also by an error, an escape or a
  cancel. `resource.ensure` is unchanged. Scope lifecycle and acquisition
  checks follow spec §9.5. The platform gives every entry a `Scope` (§2.10.11).
- `resource.acquire : <A, N>(() <- N -> A, (A) -> Nil) <- (..N, Scope) -> A`.
  A release that needs services is made with `keep` (§2.10.8). This is a std
  API break: today `acquire` takes three arguments with a `use` function.
- **Name (this note).** `task.Scope(E)` (`lib/i/task.i`:76) is renamed
  `task.Group(E)` in stage E5, so that `Scope` in a need always means the
  resource service.

#### 2.10.10 Layers are functions

There is no layer concept in the language. A layer is an ordinary function
that returns a group of services, and the group is ordinary data:

```text
base = () <- (io: Io) -> (Config, Log) {
    (Config(url = "…"), Log(write = (s) { io.print(s) }))
}
live = () -> (Config, Log, Db) {
    s = base()
    d = db_layer() <- s
    (..s, ..d)
}
main = () { app() <- live() }
test = () { app() <- (..live(), fake_db) }
```

- `live` needs `Io` through `base`; its type is
  `() <- (Io) -> (Config, Log, Db)`.
- In `test`, `Db` comes twice; the explicit `fake_db` wins over the spread
  (rule C).
- Dependency order is written by hand. There is no automatic dependency
  graph, ordering or sharing of layers.

#### 2.10.11 Entries and the platform (owner, O31 and O32)

- **Platform services.** At an entry (program entry, test, host export) the
  target platform provides its own services: the host services of the
  target's table (spec §9.7), `Assert`, `Scope`, `Root`, `Owner`, `Await`,
  `Runner`, `Scheduler`, and a top-level `Fail`. All other services are
  provided in `main` by i code.
- **Top-level `Fail`.** For `main` and tests it takes any `E`; an error that
  reaches it stops the program or the test, as an uncaught error does today.
  For a host export it takes only a host-passable `E` (spec §10.3), and the
  error is given to the host, as today.
- **The one check.** The need of an entry, after what the entry's own code
  provides, may only contain platform services of the target. Otherwise it is
  the compile error "`T` is not provided by the target `t`". (D11 / [R4] Q16
  are unchanged: entries are single-shape with positions only.)
- **Entry stub.** The compiler's entry stub provides the platform services,
  and wraps `main` in the shipped `task._entry` (`task.i`:3530-3536) when its
  need has any of `Scheduler`, `Runner`, `Await`, `Owner`, `Root`. An entry is
  asynchronous when its need has a request-mode host service or `Runner`;
  `wasm.synchronous` uses the same rule.
- **Per-target and package defaults.** A library does not choose a default
  service per target, and a package does not supply services to the program
  that uses it. The program chooses in `main`:
  `main = () { app() <- (http.native_client()) }`. A program built for
  targets that need different choices has one entry per target, named per
  target in the package file (stage E5 adds the key).
- **Provider plans removed.** Spec §2.6.1 (target provider plans: roles,
  adapters, seeds, guards, topological order) and the `providers` section of
  `i.toml` are deleted, with `crates/ilang-check/src/providers.rs`,
  `types/provider.rs`, `crates/ilang-package/src/providers.rs`, the provider
  section of `crates/ilang-wasm/src/lower.rs` and the guard run time of
  `emit/host.rs`. `lib/provider-plan.toml` becomes a per-target table of
  platform services (stage E5).

#### 2.10.12 Host services and `@external`

- The function type of a field of a host service, and of an `@external`
  declaration, may have no need, or exactly `<- (Fail(E))` with a
  host-passable `E`: `Storage = (read = (String) <- (Fail(String)) -> String)`.
  The glue reads `Fail` from `$env` and calls its `raise` with the host's
  error.
- Any other need there is a compile error.

### 2.11 `|>`

- `|>` is the weakest binary operator but one, below `||`, left associative
  (O15). Only the provide `<-` is weaker (§3.3): `x |> f <- (s)` is
  `{ x |> f } <- (s)`.
  `a == b |> f` is `{ a == b } |> f`. `a |> f |> g` is `{ a |> f } |> g`.
- Its right side is a name (`f`, `m.f`, `v.op`), a call (a postfix
  expression whose last part is a call), or a chain literal. Anything else is
  a compile error that suggests a group: `x |> g == 2` is now an error, where
  today it is `{ x |> g } == 2` (G5).
- `a |> f` is `f(a)`, passing `(a)`. `a |> f(b, port = 1)` is
  `f(a, b, port = 1)`: `a` is inserted as position 0. `a |> f(..t)` is
  `f(a, ..t)`. `a` is worked out first, then the rest in the order of the
  call (spec §6.2).
- `a |> C` with a chain literal `C` is a match (§2.5).
- `pair |> f` passes a one-tuple holding the pair; to pass its parts write
  `f(..pair)` (answers 3e).

### 2.12 Refutable bindings and the single-branch rule

- A **fitted** pattern is the pattern of a binding statement, of a top-level
  binding, or of a group with exactly one branch and no guard. All three
  follow one rule, because a parameter group is "almost an ordinary
  destructuring" (O11).
- Fitting a constructor pattern `C(…)` with pattern type `Q` to a type `B`
  requires `B <: Q | r - Q` with a new type variable `r` (D4), not `B <: Q`.
  The pattern may fail, so it reports to `Assert` (spec §9.6). When the known
  type of the value is disjoint from `Q`, the compiler warns that the pattern
  can never match. So with `r : Ok(Int) | Error(String)`, `Ok(v) = r` is
  correct and needs `Assert`, as today with the multi-constructor `Result`.
- Literal patterns keep their rule (fitted literals have the whole `Int` or
  `String`, spec §4.8).
- A data pattern fitted to a union of shapes stays a compile error: with
  `v: (Int, Int) | Nil`, `(a, b) = v` is an error, as today; write a match.
- A group with two or more branches or a guard must cover (spec §7.4) and
  never reports to `Assert` for failing to match.

### 2.13 The empty match

There is no chain with zero branches. `x |> {}` and `{}` are removed. The core
function `core.never : (Never) -> A` (D9; the name [R4]) is the match on
`Never`: `core.never(x)`, and as a value it replaces `{}`.

### 2.14 Host, entries and `@external`

- A host export, an entry (program entry, test) and the type of an
  `@external` declaration must have exactly one shape (D11). An entry has the
  shape `()`.
- [R4] Their parameter data, and the parameters of the
  function types of a host service's fields, have positions only, no labels.
  The ABI (spec §B.2) passes parameters in order, and these types are written
  by hand anyway after the migration removes their documentation labels.
- A host service is a labelled type with data whose every item is labelled
  with a function type: `@host("app:storage/v1", "async")
  Storage = (read = (String) <- (Fail(String)) -> String)`. Its field labels
  are the import names (spec §9.7, unchanged). A field's function type has no
  need or exactly `<- (Fail(E))` with a host-passable `E` (§2.10.12).
- An entry's need, after what it provides itself, is within the platform
  services of the target (§2.10.11).
- Types the host can take (spec §10.3): `Nil`, `Bool` (the prelude name for
  `True | False`), `Int`, `Float`, `String`, `BitArray`, and `Opaque` types.
  Data, labelled types with data, and functions do not cross.
- `@js("…") name: T` needs `T` to expand to one `Opaque` type.

### 2.15 Text form

| Value | Text form (spec §11.8) |
|---|---|
| Data | `(`, the positional parts in order, then `label = part` for each label in UTF-8 byte order of the labels, with `, ` between, `)`. `()` when empty. |
| Labelled value | The constructor name; when its data is not `()`, then the text form of its data: `Some(1)`, `User(age = 36, name = "ada")`, `None`. The printed text is a legal expression (O20); labels in UTF-8 byte order [R4]. |
| Function | `//fn(`, one name per key of its shape: positions `a`, `b`, … as today, then each label as `label = ` followed by the next name of the same sequence, `) { ... }`. A multi-shape value writes each group's part joined by ` \| `: `//fn(a) { ... } \| (a, b) { ... }`, in the written order of the groups' first branches. |

Float text uses the source form specified by spec §11.8.

### 2.16 Type parameters on function values (owner, O23)

Type parameters are written on function values and function types, not on
binding heads (plan A of the effects proposal):

```text
id    = <A>(x: A) -> A { x }
map   = <A, B, N>(xs: List(A), f: (A) <- N -> B) -> List(B) { … }
// map : <A, B, N>(List(A), (A) <- N -> B) <- N -> List(B)
first = <A>(xs: List(A)) -> Option(A) { … }
```

- **Form.** `<A, N, …>` directly before the parameter group of a function
  literal (§3.3 `Function`) or the parameter part of a function type (§3.1
  `FnType`). The names are distinct unqualified `UpName`s that do not name an
  accessible type. A type parameter used in a need position (`<- N`,
  `<- (..N, …)`) is a row variable (R5); every other use makes it an ordinary
  type variable; using one name both ways is a compile error.
- **Scope.** On a function literal: its parameter group, service pattern,
  `->` part and body, with nested literals and annotations in it. On a
  function type: the rest of that type. An `UpName` in a type that is neither
  a type in scope nor a type parameter in scope is a compile error (unchanged
  from today's "declared type variable").
- **Binding heads.** `name(A) = …` for functions is removed: today's
  `count(A, E, N) = (…) -> … { … }` becomes `count = <A, N>(…) -> … { … }`
  (the `E` goes, §2.10.6). `name: <A>(A) -> A = …` and the declaration
  `name: <A>(A) -> A` carry the parameters in the written type. Type
  definitions `Name(A) = …` are unchanged.
- **Generalisation** happens at function literals, named or not, as today's
  local `let` generalisation; the value restriction is unchanged.
- **Flexible in definitions.** In a function literal's own `<A>`, `A` keeps
  today's flexible meaning (spec §4.7: a declared type variable may be
  narrowed by inference). So `id = <A>(x: A) -> A { 1 }` still checks, with
  the type shown as today.
- **Polymorphic types are rigid.** A written function type with
  `TypeParams` in a parameter, a field, a type definition or an annotation is
  a polymorphic type: `Layer = <N>() <- N -> (Config, Log)`,
  `Show = (write = <A>(A) -> Nil)`. A value checked against it is checked with
  fresh rigid constants for its parameters and a level-escape check
  (skolemisation, Peyton Jones et al. 2007); each use instantiates it afresh
  through the existing `instantiate` (`infer.rs`:1618). Inference never
  guesses a polymorphic type: a parameter gets one only from a written type,
  as for polymorphic fields in OCaml and polymorphic function types in
  Scala 3. A rigid row variable is one opaque member of a need.
- **Chains.** Each branch of a chain writes its own `<…>`, which scopes over
  that branch only: `f = <A>(x: A) -> A { x } | <A>(x: A, y: A) -> A { y }`.
  Selection instantiates each candidate branch once (stage 11).
- **Bounds.** A bound on a type variable is written as an intersection at the
  use, `x: A & Show`. Capabilities are passed as ordinary parameters or as
  services; there are no implicit capabilities (O26).
- **Parsing.** `<` never begins an expression today (there is no prefix `<`),
  so `<A>(` at the start of a primary is unambiguous; at the start of a line
  P14 applies.

## 3. Grammar

The grammar is a delta against spec §3 (types), §2.2/§2.5 (items), §4.3
(type definitions), §6.1 (expressions) and §7.1 (patterns). Productions not
shown are unchanged. EBNF as in spec §0.

### 3.1 Types (replaces spec §3's grammar)

```text
Type      = Union .
Union     = Inter { "|" Inter } .
Inter     = Diff { "&" Diff } .
Diff      = Prim { "-" Prim } .
Prim      = FnType | DataType | NamedType | TypeCall | ValueType | Literal | "_" | "{" Type "}" .
FnType    = [ TypeParams ] FnParam [ "<-" Needs ] "->" Union .
TypeParams = "<" UpName { "," UpName } [ "," ] ">" .
FnParam   = DataType | "{" Type "}" .
Needs     = UpName | "(" [ NItem { "," NItem } [ "," ] ] ")" .
NItem     = NamedType | ".." UpName .
DataType  = "(" [ TItem { "," TItem } [ "," ] ] ")" .
TItem     = [ name "=" ] Type .
NamedType = [ name "." ] UpName [ "(" Type { "," Type } [ "," ] ")" ] .
ValueType = [ name "." ] name .
TypeCall  = [ name "." ] name "(" [ TypeArg { "," TypeArg } [ "," ] ] ")" .
TypeArg   = Type .
Literal   = int_lit | "-" IntPart | string_lit .
IntPart   = int_lit | "{" IntPart "}" .
```

Removed: `TupleType`, both copies of `RecordType`, `FPart`, `RPart`, `Label`
(string and `UpName` labels), `ParamType`, `Record`, the error part `"^" Union`
and the environment part `"!" Union` of a function type.

- **Needs.** `<- N` with a bare `UpName` is the same type as `<- (..N)`: the
  `UpName` must be a type parameter in scope (a row variable). In the bracket
  form every `NItem` is a named service type (its constructor is its key,
  §2.10.2) or one `..UpName` row variable, at most once and in any place. Two
  items with one constructor are merged by §2.10.4 rule R4 when the type is
  read, and the merged form is what is displayed. `<- ()` is written as no
  `<-` part at all; the printer never writes `<- ()`.
- **Order.** `<-` comes before `->`: `(String) <- (Db, Fail(NotFound), Log) -> User`.
  A missing `<-` part means "needs nothing", as a missing `! N` meant
  `! Never` today.
- **Type parameters.** `TypeParams` bind fresh type variables whose scope is
  the rest of that `FnType`. `<>` is one token (string concatenation), so an
  empty parameter list cannot be written and is not needed. Inside a type `<`
  has no other meaning, so `<A>` there needs no look-ahead. A written type
  with `TypeParams` in a parameter, a field or an annotation is a
  **polymorphic type** (§2.16).

### 3.2 Items and type definitions (replaces spec §4.3's grammar; §2.2 and §2.5 keep their productions)

```text
TypeDef     = UpName [ TypeHead ] "=" TypeRhs .
TypeHead    = "(" TypePattern { "," TypePattern } [ "," ] ")" .
TypeRhs     = "Opaque" | Type .
TypePattern = UpName | "_"
            | [ name "." ] UpName "(" TypePattern { "," TypePattern } [ "," ] ")"
            | name "." UpName
            | "(" [ TPItem { "," TPItem } [ "," ] ] ")" .
TPItem      = [ name "=" ] TypePattern .
Binding     = Pattern "=" Expr .
Declaration = name ":" Type .
```

`TypeHead` is read as type patterns and then checked by the kind (§2.8): for
kinds 2 to 4 every part must be a bare, distinct, unqualified `UpName`.
Removed: `TypeBody` with `{ Constructor }`, `Constructor`, `Field`, and the
binding head with type parameters `name TypeParams [ ":" Type ] "=" Expr`
(today `count(A, E, N) = …`). Type parameters of a function are written on the
function literal (§2.16, §3.3) or on its written type: `id = <A>(x: A) -> A { x }`,
`id: <A>(A) -> A = (x) { x }`, and the declaration `id: <A>(A) -> A`. Type
definitions keep their head: `Option(A) = Some(A) | None` is unchanged.

### 3.3 Expressions (replaces spec §6.1's grammar)

```text
Expr      = Provide .
Provide   = Pipe { "<-" Postfix } .
Pipe      = Binary { "|>" Binary } .
Binary    = Unary { BinOp Unary } .
BinOp     = "||" | "&&" | "==" | "!=" | "<" | "<=" | ">" | ">=" | "<." | "<=." | ">." | ">=."
          | "<>" | "+" | "-" | "+." | "-." | "*" | "/" | "%" | "*." | "/." .
Unary     = [ "-" | "-." ] Postfix .
Postfix   = Primary { Call | "." name | "." decimal } .
Call      = Data .
Primary   = int_lit | float_lit | string_lit | Name | Data | List | BitArray | Block | Branches .
Name      = [ name "." ] ( name | UpName ) .
Data      = "(" [ DItem { "," DItem } [ "," ] ] ")" .
DItem     = name "=" Expr | ".." Expr | Expr .
List      = "[" [ Expr { "," Expr } [ "," ".." Expr ] [ "," ] ] "]" .
Block     = "{" { Statement } Expr "}" .
Statement = Binding | Expr .
Branches  = Function { "|" Function } .
Function  = [ TypeParams ] Params [ "<-" SvcPat ] [ "->" Union ] Block .
Params    = "(" [ PItem { "," PItem } [ "," ] ] [ "?" Expr ] ")" .
SvcPat    = "(" [ SvcItem { "," SvcItem } [ "," ] ] ")" .
SvcItem   = name ":" NamedType | ".." name ":" UpName .
```

`TypeParams` is the production of §3.1.

Removed: `Arg` with `name ":"`, `Tuple`, `Record`, `VField`, `ValueLabel`,
`Clauses`, `Clause`, `Param` (environment parameters), `CtorUpdate`, the
environment statement `"!" NamedType "=" Expr`, the environment read
`name "!" NamedType`, and the `^` and `!` parts of `Function`.

- **Function head.** `<-` after `Params` is the **service pattern** (§2.10.3):
  each `SvcItem` binds a name to the service whose key is the written named
  type; the annotation is required. `..name: N` binds the projected row value
  of spec §6.8 (§2.10.8); it is
  the last item, at most once, and `N` must be a type parameter of the
  function. No other pattern is allowed in a service item, and a service
  pattern has no labels, no guard and no alternatives. `<- ()` is a compile
  error (write nothing). A service pattern is not part of the shape of the
  branch (§2.4) and is never used for selection (§2.6).
- **Provide (owner, O29).** `e <- s` works out `e` with the current
  environment plus `s` (§2.10.5), for **any** expression `e`. `<-` is the
  weakest binary level, below `|>`, and left associative: `a + f(x) <- (s)`
  provides to `a + f(x)`; `x |> g <- (s)` provides to `x |> g`;
  `e <- live() <- (x)` is `{ e <- live() } <- (x)`. Group with `{ }` to
  provide to a part: `a + { f(x) <- (s) }`. `f(x) <- (s)` and `{ … } <- (s)`
  are instances of this one rule, with no rules of their own.
- **Provided value.** The right operand is one `Postfix`: a data literal whose
  items are `Expr` or `..Expr` (a labelled item is a compile error), or any
  other postfix expression whose type is a data type with positions only or a
  row type (`env`, `live()`, `s`, `s.d`). `e <- (s) + 1` is a compile error
  (`+` after the operand; group the provide: `{ e <- (s) } + 1`).
- A binding's value is a whole `Expr`, so `u = find_user(id) <- (db)` binds
  the provided result. In a data item, a list element, a call argument or a
  guard the `Expr` may be a provide: `f(g() <- (s), 1)`.

### 3.4 Patterns (replaces spec §7.1's grammar)

```text
Pattern   = Part { "=" Part } .
Part      = Atom | StringPat .
StringPat = string_lit "<>" Binder [ "<>" string_lit ] | Binder "<>" string_lit .
Binder    = name | discard .
Atom      = name | discard | ( name | discard ) ":" Type
          | [ "-" ] int_lit | [ "-." ] float_lit | string_lit
          | Name [ DataPat ]
          | DataPat
          | "[" [ ListPats ] "]"
          | BitArrayPattern .
DataPat   = "(" [ DPItem { "," DPItem } [ "," ] ] ")" .
DPItem    = name "=" Pattern | Pattern | Rest .
PItem     = name "=" Choices | Choices .
Choices   = Pattern { "|" Pattern } .
ListPats  = Rest | Pattern { "," Pattern } [ "," [ Rest ] ] .
Rest      = ".." [ name | discard ] .
```

- A `Name` in a pattern must be a constructor (an `UpName`, or `m.C`). A
  lowercase name followed by `(` is not a pattern.
- `Rest` is the last item of a `DataPat`, at most once. `PItem` has no rest.
- `PatArgs` is removed: constructor patterns use `DataPat`.

### 3.5 Disambiguation rules

Every rule decides by tokens and brackets, never by names or types, and none
needs a comma.

- **P0 Lexer.** `<-` is one token, read by longest match after the `<<` arm
  (`crates/ilang-syntax/src/lexer.rs`:326-331): `<<-1` stays `<<` then `-1`,
  as in today's bit arrays. `x<-1` is `x`, `<-`, `1`, a compile error (a
  provide on a name); a comparison with a negative number is written
  `x < -1`. `a<-.5` is `a`, `<-`, `.5`. No `.i` file in the corpus has `x<-1`
  (review Q6, all 28 `<-` are `<<-`). `!` alone and `^` are no longer tokens of
  i and are lexer errors; `!=` is unchanged. `<>`, `<`, `<=`, `<.`, `<=.` are
  unchanged.
- **P1 Function or data.** A `(` that begins a primary (not a call) is a
  function's `Params` when its matching `)` is followed by `->`, or by `{` on
  the same line, or by `<-` and a bracket group `( … )` whose matching `)` is
  followed on the same line by `->` or `{` (then the group is the `SvcPat`);
  otherwise it is `Data`, and a `<-` after it is a provide to that data
  (`(a, b) <- (s)` works out the data with `s` provided). A primary that begins with
  `<` is a function literal with `TypeParams` (§2.16); its `>` must be
  followed by `(`, which is then `Params`. The decision is made before the
  contents are read (as spec §6.1 today). A bare row variable after `<-`
  occurs only in types, where `->` always follows.
- **P2 Call.** A `Data` on the same line right after a postfix expression is a
  call (spec §1.7, unchanged). `f(x) { … }` is a call and then a block
  statement (G7).
- **P3 Items.** In `Data`, `DataType`, `DataPat` and `Params`, an item whose
  first two tokens are a `name` and `=` is a labelled (key) item. `==` is one
  token, so `(a == b)` is positional. `..` begins a spread or rest.
- **P4 Branch join.** After the `}` of a function body, a `|` (also at the
  start of the next line, spec §1.7) continues the chain; the next token must
  begin a function literal (`(`, or `<` of `TypeParams`), else a compile
  error says that `|` joins only function literals. A `|` after any other
  expression is a compile error. A postfix call or field after a chain
  applies to the whole chain. Each branch writes its own `TypeParams`; they
  scope over that branch only (§2.16).
- **P5 The `->` part.** Spec §1.7's last paragraph is unchanged except for
  the list of tokens: in the `->` part of a function, a `{` right after `->`,
  `|`, `&` or `-` is a type group; the first other `{` not in brackets is the
  body. (`^` and `!` are removed from the list; a service pattern is in
  brackets and comes before `->`, so it never meets this rule.) A branch join
  only follows a body's `}`, so it never meets a union `|` (G1).
- **P6 Statements.** In order: (a) read pattern parts while each is followed
  by `=`: if at least one was, the statement is a binding, the parts read form
  its pattern, and the value begins at the first part not followed by `=`
  (that text is read again as an expression); (b) otherwise the statement is
  an expression. Today's rules for a binding head with type parameters
  (`name(A) =`), for the environment statement `! T = e` and for the
  environment read `name ! T` are removed with those forms. A statement that
  begins with `name (` `UpName` … `) =` is now read by (a) as a pattern part
  `name(…)`, which is not a pattern (a lowercase name followed by `(`, §3.4),
  so it is an expression statement followed by a stray `=`: the parser gives a
  compile error whose message shows the `<A>(…)` form (§2.16).
- **P7 Key exception.** Rule 2 of §2.7: inside `DataPat` and `Params`, a
  leading `name =` is a key, never a conjunction; at statement level it is a
  conjunction.
- **P8 Alternatives and types.** In a `PItem`, `|` separates alternatives,
  except after a `:` type, where `|` continues the union; a type is read as
  far right as it goes with no going back (spec §7.1, unchanged). A type in a
  `PItem` ends at `,`, `?`, or `)`.
- **P9 Guard.** `?` in `Params` begins the guard, after the last item; the
  guard ends at the matching `)`. `?` in any other pattern is an error.
- **P10 Types.** In a `DataType`, `name =` is a label; a `name` not followed by
  `=` or `(` is a value type. `( … )` followed by `->` or `<-` is a function
  type; after `<-` comes `Needs` and then `->`. A `<` in a type begins
  `TypeParams`, which must be followed by a `FnParam`.
  `{ T }` followed by `->` or `<-` is a function type whose parameter is `T`; every
  member of `T`, after expanding names, must be a data type. `:` inside a type
  is a compile error whose message says that labels are written
  `label = Type` and that a function type has no parameter names.
- **P11 `|>`.** Lowest precedence, left associative; the right operand must
  be a name, a call, or a chain literal (§2.11). A provide after it applies
  to the whole pipe (`<-` is weaker, §3.3).
- **P12 Type definitions.** An item that begins with an `UpName` is a type
  definition (spec §2.2, unchanged); its kind follows §2.8 from the written
  right side. Blocks have no type definitions (spec §4.3, unchanged).
- **P13 Lines.** Spec §1.7 is unchanged except for the text about clauses: a
  line that begins with `|` continues the line before, and a line that begins
  with `(` begins a new statement. So every continued branch begins its line
  with `|`. Directly inside `( )` (data, data types, data patterns, parameter
  groups, service patterns, needs) a break is a `,`. Directly inside `{ }` a
  break ends a statement.
- **P14 `<` at the start of a statement.** Spec §1.7 says that a line that
  begins with a binary operator continues the line before. Exception: a line
  whose first tokens are `<` `UpName` { `,` `UpName` } [ `,` ] `>` `(` begins a
  new statement, a function literal with type parameters. This is the same
  kind of look-ahead as `at_type_params` (`parser.rs`:657-685). So
  ```text
  log()
  <A>(x: A) { x }
  ```
  is two statements, not `log() < A > (x: A) { x }`. To continue a comparison
  across lines in that one form, put the operator at the end of the line
  before. The same look-ahead decides `<` at the start of a primary (P1),
  after `|` of a chain (P4), and after `=` or `(` or `,`; in those places `<`
  cannot be binary anyway.
- **P15 `<-` and lines.** `<-` is read like a binary operator for line
  breaks: a line that begins with `<-` continues the line before, and a line
  that ends with `<-` continues on the next, also when the next begins with
  `(`. So
  ```text
  result = app()
      <- (live(), fake_clock)
  ```
  is one provide. The service pattern of a function head follows the same
  rule. No statement begins with `<-`, so there is no analogue of today's
  `name ! T = e` trap (G19).

### 3.6 Attacks

G1 to G23 are the evaluation's, re-derived under the final decisions
(`label = Type`, guard inside the group, no scope in data, P1 to P13). G24
onward are new.

| # | Form | Resolution under the final rules |
|---|---|---|
| G1 | `(p) -> A { e } \| (q) { f }` against a union `\|` | P5: the body's `{` ends the `->` part; `\|` after `}` is a join. No ambiguity. |
| G2 | A forgotten body: `(a) -> Int \| (b) -> Int { … }` | One function whose result is `Int \| (b) -> Int`. Deterministic and not an error [R4]; the type error that follows mentions the missing body when a result union has a function type member written without a group. |
| G3 | A leading `\|` on a line | Continues the line before (P13). A line starting with `(` is a new statement. |
| G4 | `x \|> (p) { e } \| (q) { f }` | `Branches` is a primary, so the whole chain is the right operand (P4, P11). |
| G5 | `\|>` weakest | `x \|> g == 2` is `x \|> { g == 2 }`, which P11 rejects with a hint; today it is `{ x \|> g } == 2`. `x == 1 \|> core.not` becomes legal. Stage 5 rewrites the 28+ and 3+ lines in std (§7.2). |
| G6 | `(x) { y }` | Always a function (P1). |
| G7 | `f(x) { … }` | A call, then a block statement (P2). |
| G8 | Guard inside the group | P9. A function inside the guard is inside its own brackets; a `?` line continues (spec §1.7). |
| G9 | Nested chains | Every body is braced; an inner chain ends at the outer `}`. |
| G10 | One-tuple `(x)` against the argument group | No parse ambiguity. A pair subject is written `((0, y)) { … }` (58 clause lines in lib+std, evaluation). |
| G11 | `(w = 3, h = 4, area = w * h)` as a statement | P6(d): read as a pattern, `area = w * h` is not a pattern, so no part is followed by `=`: an expression statement. As an expression its items see only outer names (O9), so `w` and `h` must be outer names. |
| G12 | `(x, y) = whole = e` | P6(d): parts `(x, y)` and `whole` are followed by `=`; `e` is the value. Today's `binding_head` stops after the first non-name atom (parser.rs:891-895); the new parser chains. |
| G13 | Pattern `(name = n)` against value `(name = n)` | Decided by position: patterns are binding left sides, parameter groups and their insides. |
| G14 | `(port = p: Int)` | Key `port`, pattern `p: Int`; the type ends at `,`, `?` or `)` (P8). |
| G15 | `(name: String)` in a type against a parameter | Gone: `:` in a type is an error (P10, O10). `(name: String)` is only a pattern (position 0), `(name = String)` only a type. |
| G16 | `T = (A)` against `T = { (A) }` | §2.8: nominal against name. `Pair(A, B) = (A, B)` is an alias today and nominal after (2 in std, 130 in tests); the migrator writes `{ (A, B) }` (§7.3). |
| G17 | `Wrap = (Int)` against `Wrap = (Int) \| Nil` | The first is a labelled type; the second a name for a union of a data type and `Nil`. Deterministic by the outermost form. |
| G18 | `User(name = "ada")` as a statement | P6(d): a pattern not followed by `=`, so an expression (a call). |
| G19 | `db ! Db` | The read statement is removed (§2.10); `!` alone is a lexer error (P0). Today `db ! Db = make()` is silently two statements; the migrator rewrites every read into a head pattern (§7.3). |
| G20 | `(Sat \| Sun, x)` | `\|` is tighter than `,` in a `PItem`. |
| G21 | `(x: Int \| y)` | P8: `Int \| y` is a type (`y` a value type). A type test is the last alternative: `(True \| _: Int)`. |
| G22 | `f \| g` with names | Rejected by P4 (D3). |
| G23 | Bare `None` | A value (D5); `None()` is an error in expressions and patterns. |
| G24 | `T = (name = String)` against a binding | At the top level an item starting with an `UpName` is a type definition (P12): a labelled type. In a block there are no type definitions: it is a binding with the constructor pattern `T` and the value `(name = String)`, where `String` is an expression, so a compile error ("`String` is a type, not a value"). |
| G25 | A data type against a data value where both could occur | No position admits both. Types are after `:` in a pattern, in the `->` part, the right side and head of a type definition, a declaration, a type macro's arguments; everything else in an expression is a value. The position is known before the brackets are read. |
| G26 | `Codec(Size, _) = size` against `Codec(D, E) = (…)` against `Codec(D, E) = Make(D, E)` | P12 and §2.8: the head is read as type patterns; the right side decides alias pattern (value type), labelled type (one data type) or name. For the last two, a head part that is not a bare `UpName` is an error. |
| G27 | `(x = 1) { … }` against `(x = 1)` | P1: a parameter group with key `x` that must be `1`, against data with label `x`. |
| G28 | `(x)` against `(x = Int)` in a type | P10: one token after `name`. `(x)` is one position of the value type `x`. |
| G29 | `(name = n: String)` against `(name = String)` in a pattern | The first is key, binder and type. The second is key `name` with the constructor pattern `String`: an error whose message suggests `name = n: String`. |
| G30 | `x: (n = Int) = (n = 1)` | P6(d): the type stops at `=` outside brackets; part `x: (n = Int)` is followed by `=`; `(n = 1)` is not, so it is the value. `a = (n = 1) = b` instead binds `a` and refutably matches `b` against the data pattern `(n = 1)`. |
| G31 | `(..rest) { … }` | Rest is not allowed at the top of a parameter group: a compile error (§2.4). |
| G32 | `((a) { a }, 1)` | P1 on the outer `(`: no `{` or `->` after its `)`, so data; the inner item is a function. |
| G33 | `(f = (a) { a } \| (b) { b })` | The labelled item's value is a chain (P3, P4). |
| G34 | `{ (x) -> Int { x } }` | Now always a block whose value is a function. Today it is a clause block (spec §6.1). The legacy parser reads the old meaning during migration (§7.1). |
| G35 | `(a) { 1 } \| (b) { 2 }(x)` | The call applies to the whole chain (P4). |
| G36 | `(port = 80) { … }` read as a default | It is a literal key pattern; there are no defaults (§2.4). A documentation point, not an ambiguity. |
| G37 | `{ a = f  (b, c) = g }` on one line | Still one statement, because a same-line `(` is a call (spec §1.7, unchanged). With a lowercase `f`, `f(b, c)` is not a pattern part, so the value begins at `f` and the `=` after the call is a compile error; with a constructor, `mk = Pair  (b, c) = g` is the binding `mk = Pair(b, c) = g`, as today. |
| G38 | `a = b = c = d` | P6(d): `a`, `b`, `c` are followed by `=`, so all three bind `d` (O13). |
| G39 | `Opaque` in a type | Only the whole ungrouped right side of a type definition (§2.8); elsewhere an error. |
| G40 | `-1` as a branch pattern | `\| (-1) { … }`: the `-` is inside the group; spec §1.7's clause-line problem with `-` is gone. |
| G41 | `x<-1` | P0: `x`, `<-`, `1`; a provide of `1` to `x`, a compile error (`1` is not service data) whose message suggests `x < -1`. |
| G42 | `<<-1` in a bit array | P0: `<<` is matched first; unchanged. |
| G43 | `log()` then a line `<A>(x: A) { x }` | P14: a new statement. |
| G44 | `a < B > (x)` inside a line | After a left operand `<` is binary (P1 applies only where a primary begins), so this is today's chain of comparisons; no change. |
| G45 | `(x) <- (db: Db) -> R { … }` against `(x) <- (s)` | P1: the group after `<-` is a `SvcPat` only when its `)` is followed on the same line by `->` or `{`; otherwise the first group is data and `<-` provides to it. `f(x) <- (s)` is always a provide (a call is not a primary). |
| G46 | `f(x) <- (s)` with `(s)` on the next line | P15: a line ending with `<-` continues. With `<-` at the end of the first line or the start of the second, it is one provide. A `(` on the next line after a complete `f(x)` is a new statement (P13), as today. |
| G47 | `f(x) <- (s) <- (t)` | `{ f(x) <- (s) } <- (t)`: left associative; for a key in both, `s` wins inside. |
| G48 | `{ … } <- (s)` at the start of a statement | An expression statement: the block worked out with `s` provided (§2.10.5). |
| G52 | `a + f(x) <- (s)` | `{ a + f(x) } <- (s)`: `<-` is the weakest level. |
| G53 | `x \|> g <- (s)` | `{ x \|> g } <- (s)`. |
| G54 | `e <- (s) + 1` | Compile error: the right operand is one postfix expression; `+` cannot follow a provide's operand. Write `{ e <- (s) } + 1`. |
| G49 | `<- (db: Db, ..env: N)` against `<- (..env, fake)` | The first is a service pattern (after `Params`), the second provided data (after a call or block); position decides, as G25. |
| G50 | `(A) <- N -> B` against `(A) <- (N) -> B` in a type | The first is the row variable `N`; the second is a row whose one item is a named type `N`, a compile error when `N` is a type parameter ("a type variable is not a service key; write `<- N` or `<- (..N)`"). |
| G51 | `id = <A>(x: A) { x }` against today's `id(A) = (x: A) { x }` | The second is now an expression statement followed by `=`, a compile error that shows the first (P6). |

No ambiguity was found that needs a comma.

## 4. Examples, before and after

"Before" is today's language at 5c186400; "after" is SYN3.

### 4.1 `list.count` (std/src/list.i:114)

```text
// before
count(A, E, N) = (list: List(A), where: (A) -> Bool ^ E ! N) -> Int ^ E ! N {
    go = (rest, n) {
        rest |> {
            [] -> n
            [x, ..xs] -> where(x) |> {
                True  -> go(xs, n + 1)
                False -> go(xs, n)
            }
        }
    }
    go(list, 0)
}

// after
count = <A, N>(list: List(A), where: (A) <- N -> Bool) -> Int {
    go =
        ([], n) { n }
        | ([x, ..xs], n) {
            where(x) |>
                (True) { go(xs, n + 1) }
                | (False) { go(xs, n) }
        }
    go(list, 0)
}
```

The type is `count : <A, N>(List(A), (A) <- N -> Bool) <- N -> Int`: the
error part `E` is gone (errors of `where` are a `Fail` entry in `N`), the type
parameters move to the function literal, and the need `N` of the result is
inferred from the call `where(x)` (R2), so the head writes no `<-`.
`list` and `where` are now positional binders; callers that wrote
`count(list: xs, where: f)` become `count(xs, f)`. `go` has one group of
shape (2 positions) with two branches (rule 11, covered: `[]` and
`[x, ..xs]` cover `List`, `n` is a catch-all). The inner match is a match on
`(where(x))` with the one-position group `True`, `False`, which covers
`Bool = True | False`.

### 4.2 A sum type with a recursive member

```text
// before
Tree(A) {
    Leaf
    Node(left: Tree(A), value: A, right: Tree(A))
}
size(A) = (t: Tree(A)) -> Int {
    t |> {
        Leaf -> 0
        Node(left: l, right: r, ..) -> size(l) + 1 + size(r)
    }
}

// after
Leaf = ()
Node(A) = (left = Tree(A), value = A, right = Tree(A))
Tree(A) = Leaf | Node(A)

size = <A>(t: Tree(A)) -> Int {
    t |>
        (Leaf) { 0 }
        | (Node(left = l, right = r, ..)) { size(l) + 1 + size(r) }
}
```

`Tree(A)` is a name; it reaches itself only through `Node`'s data. Exports:
`(Tree(..), size)` exports `Tree`, `Leaf(..)` and `Node(..)` (D8).

### 4.3 A service function with an environment

```text
// before
NotFound { NotFound(String) }
Db { Db(find: (id: String) -> User ^ NotFound) }

find_user = (id: String, db ! Db) -> User ^ NotFound ! Db {
    db.find(id)
}

fake = () -> User ^ NotFound {
    ! Db = Db(find: (id) { fail.raise(NotFound(id)) })
    find_user("1")
}

// after
NotFound = (String)
Db = (find = (String) <- (Fail(NotFound)) -> User)

find_user = (id: String) <- (db: Db) -> User {
    db.find(id)
}
// find_user : (String) <- (Db, Fail(NotFound)) -> User

fake = () -> User {
    find_user("1") <- (Db(find = (id) { fail.raise(NotFound(id)) }))
}
// fake : () <- (Fail(NotFound)) -> User
```

`db.find(id)` reads the label `find` of a value whose known type is `Db`
(the service pattern's annotation).
The parameter `id` of the fake gets its expected type `String` from `Db`'s
data type.

### 4.4 A labelled constructor

```text
// before
User { User(name: String, age: Int) }
u = User("ada", 36)
v = User(..u, age: 37)
adult = (u: User) { u |> { User(name, age) -> age > 17 } }

// after
User = (name = String, age = Int)
u = User(name = "ada", age = 36)
v = User(..u, age = 37)
adult = (u: User) { u |> (User(age = age, ..)) { age > 17 } }
```

`User("ada", 36)` is a shape error after. The one-branch match is the binding
`User(age = age, ..) = u` followed by the body; it is not refutable because
`u: User`.

### 4.5 Overloading by arity and by label

```text
// before: two names, or a label used only for documentation
area_circle = (r: Float) -> Float { 3.14159 *. r *. r }
area_rect = (w: Float, h: Float) -> Float { w *. h }
connect = (host: String, port: Int) -> Conn { open(host, port) }
c = connect("a.com", port: 80)

// after
area =
    (r: Float) -> Float { 3.14159 *. r *. r }
    | (w: Float, h: Float) -> Float { w *. h }

connect =
    (host: String) -> Conn { connect(host, port = 80) }
    | (host: String, port = p: Int) -> Conn { open(host, p) }

c1 = area(2.0)                 // Float, branch 1
c2 = connect("a.com")          // Conn, group (1 position)
c3 = connect("a.com", port = 8080)
fs = [area]                    // List({ (Float) | (Float, Float) } -> Float)
```

Each group of `area` and `connect` has one branch, so `r: Float` and
`host: String` are annotations. `connect(host, port = 80)` inside the first
branch selects the group (1 position, {port}) from the written patterns
(answers 1e).

### 4.6 A match with a guard

```text
// before
describe = (opt: Option(String)) -> String {
    opt |> {
        Some(v) ? string.is_empty(v) -> "empty"
        Some(v) -> v
        None -> "none"
    }
}

// after
describe = (opt: Option(String)) -> String {
    opt |>
        (Some(v) ? string.is_empty(v)) { "empty" }
        | (Some(v)) { v }
        | (None) { "none" }
}
```

### 4.7 A tuple subject

```text
// before
pick = (p: (Int, Int)) -> Int {
    p |> {
        (0, y) -> y
        (x, _) -> x
    }
}

// after: the subject is a one-tuple holding the pair
pick = (p: (Int, Int)) -> Int {
    p |> ((0, y)) { y } | ((x, _)) { x }
}

// after, written as a two-position chain
pick2 = (0, y: Int) -> Int { y } | (x: Int, _) -> Int { x }
pick3 = (p: (Int, Int)) -> Int { pick2(..p) }
```

### 4.8 Other changed forms

```text
// records become data
// before                          after
ada = (name: "ada", age: 36)       ada = (name = "ada", age = 36)
older = (..ada, age: 37)           older = (..ada, age = 37)
Point = (x: Int, y: Int)           Point = { (x = Int, y = Int) }   // a name, as today
Pair(A, B) = (A, B)                Pair(A, B) = { (A, B) }          // keeps today's alias meaning

// function types lose documentation labels
fail.raise: (error: E) -> Never ^ E        fail.raise: <E>(E) <- (Fail(E)) -> Never

// type parameters move to the function literal
id(A) = (x: A) -> A { x }                  id = <A>(x: A) -> A { x }

// errors and environment become one need after <-
(String) -> User ^ NotFound ! Db           (String) <- (Db, Fail(NotFound)) -> User

// environment parameters and reads become the service pattern
(a, log ! Log) -> T { body }               (a) <- (log: Log) -> T { body }

// environment statements become a provide
{ ! Db = fake_db  run() }                  { run() } <- (fake_db)

// opaque types
Element {}                                 Element = Opaque

// clause blocks as values
list.map(opts, { Some(u) -> u.name  None -> "" })
list.map(opts, (Some(u)) { u.name } | (None) { "" })

// alias binders in parameters flip
swap = (t = (a, b)) { (b, a) }             swap = ((a, b) = t) { (b, a) }

// refutable constructor bindings stay
Ok(v) = parse(text)                        Ok(v) = parse(text)

// pipe level
ok = x |> f == y                           ok = { x |> f } == y

// the empty match
absurd = (x: Never) { x |> {} }            absurd = (x: Never) { core.never(x) }
```

## 5. What is removed, and replacement spec text

### 5.1 Removed or changed, section by section

| Spec section | Change |
|---|---|
| Preamble (lines 17-21) | Keep the first exception, restated for groups (§5.2.7). Remove the second (brace decision) and third (`n = ( … ) -> T` in a first statement). Remove "a clause block is a function too"; a chain is. |
| §1.4 | "matching is a call of a clause block" becomes "matching is a call of a chain of functions". |
| §1.6 | "record labels" becomes "labels". |
| §1.7 | Remove "a clause block, or the constructors of a type" and "the `( )` of a record or a record type"; remove the clause half of the paragraph on lines starting with `(`, `-`, `-.` (statements keep it); add P13. |
| §2.2 | "two constructors may not have the same name" becomes "each labelled type defines one constructor with its name". |
| §2.3 | "Importing an `UpName` puts in the type and the constructor with that name": unchanged. "Header groups are not record or tuple values" becomes "are not data". |
| §2.4 | Replaced (§5.2.1). |
| §2.6.1 | Deleted: target provider plans (roles, adapters, seeds, guards, topological order) and the `providers` section of `i.toml` (§2.10.11). §2.6.2 and §10.2 lose their references to it. |
| §2.4.1 | "Environment parameters do not count as value parameters" is removed. |
| §3 | Grammar replaced (§3.1); bullets replaced (§5.2.2). `Record`, record labels, `(..R, f: T)`, `(..R, -f)`, label parameters are removed. |
| §4.1 | "Records are values of one type constructor" becomes "data of one shape is one type constructor"; "Tuples with different numbers of parts are different" becomes "data of different shapes is different". |
| §4.2 | `Record` removed from the table; the `Bool`, `Nil`, `Result` definitions become §2.8's prelude lines. |
| §4.3 | Replaced (§5.2.3). |
| §4.4 | Bullets replaced (§5.2.4); the record exceptions, the arity failure and the label rule are removed. |
| §4.5 | Rules 4, 5 and 9 replaced (§5.2.4). |
| §4.6 | Replaced (§5.2.5). |
| §4.7 | Remove the record and label-parameter sentences ("Record fields and spread bases count…", "Label parameters alone are rigid labels"). |
| §4.8 | Rules 4, 5, 6, 7, 9, 11, 13 changed and rule 15 removed (§5.2.8); the known-type bullets on records, clause blocks and calls by label change (§5.2.8). |
| §4.9 | Referenced but absent; every reference is removed. |
| §5 | "A name in a pattern of a clause may be used in its guard and its expression" becomes "the names of a parameter group may be used in its guard and its body". |
| §6.1 | Grammar replaced (§3.3); bullets replaced by P1 to P13; the five tests, the examples of blocks and clause blocks, and the two bullets on functions in first clauses are removed. |
| §6.2 | Record and update sentences become: data items are done in written order, spreads where they are written; in a call, the function, then the items of the argument data in written order. |
| §6.4 | Replaced by Data (§5.2.6). |
| §6.6 | Replaced (§5.2.6). |
| §6.7 | Replaced (§5.2.6). |
| §6.8 | "`! N` must have the types of the environment parameters" becomes "the need after `<-` contains the types of its head's service pattern". |
| §6.9 | Table: `\|>` moves to a new level 0 below `\|\|`, and the provide `<-` is a new level below it, the weakest, left associative, whose right operand is one postfix expression (§3.3). The `\|>` bullets are replaced by §2.11. The bullet on a match at level 6 is removed. |
| §6.10 | Remove the environment statement; a binding's value may be a provide (§2.10.5); "`name = pattern` is itself a pattern" becomes the conjunction rule; direct values: "a function expression or a chain; … data whose items are direct values …"; the record example goes. |
| §6.11 | Replaced (§5.2.7). Labels of parameters and environment parameters are removed. |
| §6.12 | Replaced by "Branches and matching" (§5.2.7). `{}` removed. |
| §6.13 | Removed. |
| §7 | Replaced (§3.4, §5.2.9). "There are no record patterns" and "Empty tuple patterns are compile errors" are removed. |
| §9.1-§9.12 | Examples rewritten. Tables of function types lose documentation labels and use `<-` needs. §9.1: a computation has a value and a need (`A <- M`); the error part goes. §9.2 replaced by §2.10.1 to §2.10.5 and §2.10.8 (service patterns, keys, R1 to R6, rules A to C, provide); "a record is not a value of a custom type, so a record type is never a service type" becomes "a service type is a labelled type"; "one constructor may not have different arguments in one environment" is replaced by R4. §9.3 replaced by §2.10.6 (`Fail`, library `fail.raise` and `fail.catch`). §9.4: the table and rule 3's `keep_env` sentences as §2.10.7 and §2.10.8, plus `control.escape`. §9.5: `resource.acquire` and `resource.scoped` on the `Scope` service (§2.10.9). §9.7: host services as in §2.14 and §2.10.12; the platform table of §2.10.11. §9.9: `task.Scope(E)` becomes `task.Group(E)`. |
| §10.1 | Types positional (§2.14). §10.1.1: `Element {}` becomes `Element = Opaque`; "closed nominal type with no constructors" becomes "`Opaque` type". |
| §10.3 | "custom types with no constructors" becomes "`Opaque` types"; the records and label-parameter sentences become "data, labelled types with data and functions do not cross". |
| §11.1 | "tuples … records" becomes "data". |
| §11.4 | Tuple and record rules become §2.3's data rule. |
| §11.6 | "the expression of a clause of a clause block that is called as a function" becomes "the body of a branch of a chain that is called"; "a clause of a match" becomes "a branch of a match". |
| §11.8 | Rows as §2.15. |
| §10.2 | Entries: the platform services and the one check of §2.10.11; the top-level `Fail`. |
| §12 | `Record` and `i/record` removed; `Bool`, `Result` are names, `True`, `False`, `Nil`, `Ok`, `Error`, `AssertionFailure`, `Assert`, `Fail` labelled types; `assert : (Bool) <- (Assert) -> Nil`, `panic : (String) -> Never`. |
| §13 | Labels removed from the types (`index_of : (String, String, Int) -> Result(Int, Nil)`, `byte_slice : (String, Int, Int) -> …`, `portable : (A) -> Bool`); add `never : (Never) -> A`. |
| §14 | Removed. |
| App. B.2 | "A custom type with no constructors" becomes "An `Opaque` type". |

### 5.2 Replacement text

The texts below replace the named sections. Examples in them use `i future-SYN3`
fences when they are put into `docs/spec.md` and are registered in
`scripts/check-snippets.json` with `"track": "SYN3"` until stage 8.

#### 5.2.1 §2.4 Exports

```text
ExportItem = name | UpName [ "(" ".." ")" ] .
```

- A top-level name that is not in the exports may only be used in its module.
- For a labelled type `T`, `T` exports the type but not its constructor:
  other modules may use the type, but may not make its values, take them
  apart, read their keys, or spread them. `T(..)` exports the type and its
  constructor.
- For a name (an alias) `A`, `A` exports the name. `A(..)` exports the name and
  every member of its expansion with `(..)`; every member must be a labelled
  type defined in this module, or it is a compile error.
- An alias pattern binds names with no constructors; `(..)` on it is the same
  as exporting it alone.
- Every exported name must be defined in this module. If the type of an
  exported definition, or the expansion of an exported name, uses a type
  defined in this module, that type must be exported too.
- The value-type rules of today's last paragraph are unchanged.

#### 5.2.2 §3 Type syntax: bullets

- An `UpName` with no module name before it is a declared type variable or a
  type (unchanged). `_` is a hole.
- Literal types, value types, type calls, `A | B`, `A & B`, `A - B`:
  unchanged. The error part `^ E` and the environment part `! N` of a
  function type are removed; a function type is
  `[<Params>] P [<- Needs] -> R` (§3.1, §2.10).
- `( … )` is a **data type**. Its items are `T` (the next position) and
  `name = T` (a label). Positions come before labels. A label may not occur
  twice. `()` is the data type of the empty data. A `:` in a type is a compile
  error.
- `( … ) -> R` is a function type whose parameter is that data type.
  `{ T } -> R` is a function type whose parameter is `T`; every member of `T`
  must be a data type, and members of one shape are not allowed twice.
- A function type goes as far to the right as it can; `{ }` groups.
- In a written function type, a missing `<-` part means "needs nothing".
- `<A, N>` before the parameter part binds type parameters for the rest of
  the type (§2.16).
- `Opaque` is not a type. It is only the right side of a type definition
  (§4.3).

#### 5.2.3 §4.3 Type definitions

Grammar as §3.2. Then:

- An item `T(P1, …, Pn) = v` is an **alias pattern** exactly when its written
  right side is a value type, after removing groups (unchanged from today,
  with today's bullets on alias patterns).
- Otherwise the head is a list of type parameters: distinct, unqualified
  `UpName`s that do not already name an accessible type.
- `T = Opaque` defines a new **opaque labelled type**. i code cannot make or
  take apart its values; they come from the host (§10).
- `T = ( … )`, where the whole right side is one data type not in a group,
  defines a new **labelled type** with that data, and its **constructor** `T`.
  The constructor is the function `{ D } -> T(P1, …)` where `D` is the data
  type, or, when `D` is `()`, the value `T`. Two labelled definitions are
  different types even with the same data.
- Any other right side defines a **name**, a transparent alias. A name may not
  occur in its own expansion, directly or through other names or alias
  patterns. The data of a labelled type is not part of the expansion of a
  name, so a name may reach itself through a labelled type's data.
- In a name, every parameter must be used, and every type variable must be a
  parameter. In a labelled type, a parameter need not be used; a parameter
  not used in the data, and every parameter of an opaque type, does not
  change (§4.5 rule 6). The others change as the data says.
- `Opaque` may not be the name of a type, and may not be used in a type.
- A sum is a name for a union, or a labelled type whose data is one item of a
  union type. Its members may be any types, labelled or not, also built-in
  types; values are told apart by type tests and constructor patterns, so
  every two members must have different type constructors (§4.4, §7.1).
- Built-in types are exported by `i/core`; `core.Int` names the built-in
  even where a module definition shadows `Int`.
- Blocks have no type definitions.

```text
Some(A) = (A)
None = ()
Option(A) = Some(A) | None
User = (name = String, age = Int)
Pair(A, B) = (A, B)        // a labelled type: Pair(Int, Int) is not (Int, Int)
Point = { (Int, Int) }     // a name for (Int, Int)
Meters = Int
Element = Opaque
Codec(Size, _) = size      // an alias pattern
```

#### 5.2.4 §4.4 and §4.5: changed bullets and rules

§4.4, second bullet, replaced:

- Members of a union with the same type constructor are put together into
  one, as today. Data of one shape is one type constructor; its parts go in
  the same direction as the data. All function types are one type
  constructor: `(P) <- N -> R | (Q) <- M -> S` is
  `(P & Q) <- N ∧ M -> R | S`, where `N ∧ M` is the merge of the two needs
  (§2.10.4, R4); when `P` and `Q` have no shape in
  common, `P & Q` is `Never`, which is correct and gives a function that
  cannot be called. The same holds for `&` the other way round.

§4.4: remove "Record unions are an exception…", "Record intersections…",
"A spread is also a record alternative…", "except that a union may have
several record and spread alternatives", and every sentence on record fields
and spreads in the last bullet.

§4.5 rule 4 replaced:

4. `{P} <- N -> R <: {Q} <- M -> S` when `Q <: P`, `R <: S`, and `N` fits
   in `M` (rule A of §2.10.4: for every constructor of `N`, `M` has it and
   `M_C <: N_C`). The sentence "Service arguments are compared as equal, not
   by direction …" is removed. Here `{P}` is the parameter type: a data type
   or a union of data types.

§4.5 rule 5 replaced:

5. Data of the same shape: `D1 <: D2` when the part under every key of `D1` is
   a subtype of the part under the same key of `D2`. Data of different shapes
   are disjoint.

§4.5 rule 7, second sentence, replaced: in `Continuation(A, R, N)`, `A` goes
in the other direction and `R` and `N` in the same direction.
`Boundary(R, N)` does not change.

§4.5 rule 9: removed.

#### 5.2.5 §4.6 Function types

The function type `{ D } <- N -> A` uses the complete-data notation of
spec §4.6: `D` is the whole argument data type. It has a need
`N` written after `<-` (what it takes from outside, §9.2) and a value `A`
after `->`. Errors are the need entry `Fail(E)`, so `Effect<A, E, R>` of
Effect-TS is `() <- (..R, Fail(E)) -> A`. A missing `<-` part means "needs
nothing". The shape of `D` is part of the type: its positions and its
labels; the need is not. There is no currying and there are no default
parameters. `{ D1 | … | Dn } <- N -> A` is the type of a function with
several shapes (§6.12); calling it with data of the shape of `Di` needs that
data to be in `Di`, and gives `A`. `<A, N>` before the parameter part makes
the type polymorphic (§2.16).

#### 5.2.6 §6.4 Data, §6.6 Calls, §6.7 Constructors and keys

**§6.4 Data.** `( … )` in an expression makes data (§4.1). Its items are
`e` (the next position, counting from 0), `name = e` (a label) and `..e` (a
spread). The type of the data is the data type with the type of each item at
its key. Positions come before labels; a label may not be given twice. A
spread `..e` needs a known type of `e` with exactly one member, which is a data
type or a labelled type whose constructor may be used here; its positions come
next, and its labels are added. A labelled item after a spread may replace a
label of that spread; any other repeated key is a compile error. `( )` is not
a scope: the items are worked out in the scope around the data, in the order
written. `()` is the empty data. `(e)` is data with one position, not `e`.
`d.k` (a decimal) and `d.f` read a key (§4.8 rule 13).

**§6.6 Calls.** `f(…)` calls the function `f` with the data in the brackets.
Every function is called in this way, whatever expression gives it. The data
must have a shape that `f` takes. There are no labels outside data and no
default parameters; every key is given exactly once, by the data.

**§6.7 Constructors and keys.** `C(…)` makes a value of the labelled type
`C`: it is a call of the constructor function with the data in the brackets,
which must be in the data type of `C`. `C` alone is the value of a labelled
type whose data is `()`; `C()` is a compile error. `C(..v, label = e)` is an
ordinary call whose data has a spread. Reading a key of a labelled value, and
spreading it, requires the same visibility as taking it apart (§2.4). `v.f`
and `v.k` read a key of the data, or of the data of a labelled value: every
member of the known type of `v` must have that key (§4.8 rule 13). With no
known type it is a compile error.

#### 5.2.7 §6.11 Parameters and §6.12 Branches and matching

**§6.11 Parameters.** A function's parameter group is a data pattern (§7).
When the function is called, the argument data is matched with it as in a
binding (§6.10), but its names are not generalized (§4.7). An item `p` is a
positional item; an item `label = p` matches the part under `label`. The
parameter group has no rest. A group may end with a guard `? e` (§6.12). The
names of a parameter group may not be bound twice, and its labels are
distinct. A function has no environment parameters; the services it uses
itself are named by its service pattern `<- (x: T, …)` after the parameter
group (§9.2).

**§6.12 Branches and matching.** A chain `F1 | … | Fn` of function literals
(n ≥ 1) is one function value. Its branches are grouped by shape (§4.6). When
it is called with data, the group with the shape of the data is used, and in
it the first branch whose parameter group matches the data, and whose guard,
if it has one, is `True`, is taken; the value of its body is the value of the
call. A guard is any expression of type `Bool`, worked out only after its
group matched; it may use the names of the group. Alternatives in an item are
tried in order.

- A group with one branch and no guard is a function of one shape: its
  parameter group is fitted (§4.8), `x: T` in it is a type annotation unless
  its item has alternatives, it is not checked for coverage, and when it may
  fail it reports to `Assert`.
- Any other group is typed by §4.8 rule 11, and its branches must cover every
  value of its parameter type (§7.4).
- `e |> C`, where `C` is a chain literal, is a **match**, and `e` is its
  subject. It works out `e` and calls `C` with `(e)`. It adds no call frame:
  the branches run in the place of the match, a call in a branch body of a
  match in tail position is in tail position (§11.6), and the match is not one
  of the calls in a stop message (§11.7).
- A branch that can never be used, also a branch whose shape is not one
  position in a match, gives a compile warning.
- A chain is a direct value (§6.10), as every function is. Its guards and
  branch bodies count as function bodies (§2.5, §5, §6.10). Each time a chain
  literal is worked out, it makes a new value.
- A call of a name bound to a chain selects branches by the known types of the
  arguments (§4.8 rule 5a). This chooses types, never behavior.

The preamble's first exception becomes: "`x: T` in a parameter item is a type
annotation when its branch is the only branch of its shape group, has no
guard, and the item has no alternatives, and a test when not (§7.1). So with
`v: Int | String`, `v |> (x: Int) { x }` is a compile error, but
`v |> (x: Int) { x } | (_) { 0 }` is correct."

#### 5.2.8 §4.8 Typing rules: changed rules

4. **Functions.** Let the parameter group be the data pattern `q`. `q` is
   fitted to a data type `P` (below). If `Γ` with the names of `q` gives
   `body : A <- M` (with the names of the service pattern bound to their
   annotated types), the function has the type `{ P } <- M' -> A`, where `M'` is
   the merge of `M`, the service pattern's types, and `Assert` when `q` may
   fail for `P` (§7.3) (§2.10.4). Making the function needs nothing.
5. **Calls.** If `f : {P} <- N -> A` and the argument data `d : D` with
   `D <: P`, then `f(d) : A <- N`.
   **5b. Provide.** `e <- s` as §2.10.4 "Provide".
   **5a. Selection.** When `f` is a name with a selection table, its type at
   the call is `F_sel` of §2.6.2, computed from the known types
   before inference constrains the call.
6. **Data and lists.** If every item `ei : Ti` at key `ki`, the data has the
   data type with `Ti` at `ki`; a spread contributes the parts of the type of
   its base at their keys. Lists: unchanged.
7. **Constructors** are functions from their data type to their type, or
   values when the data is `()`.
9. **Blocks.** Unchanged, with needs in place of errors and environment: the
   need of a block is the merge of the needs of its statements and result.
   The environment statement and the environment read are removed.
11. **Groups.** This rule is for a group of a chain with more than one branch
    or with a guard. Let it be called with data of type `B`: in a match, the
    one-tuple `(A)` of the subject's type; in a call, the argument data type.
    The places are those of today's rule 11, with "the parts of a data pattern
    at each key" for "each part of a tuple" and "each key of a constructor's
    data" for "each field of a constructor"; the top place is the argument
    data, which has one shape. Each branch body has its own type `Ri`; the
    group's result is the union of the `Ri`. Every guard has type `Bool`, and
    the need of every guard and body is part of the need `N` of the chain.
    The rest of today's rule 11 (shared type variables per type constructor,
    type tests, catch-alls, `B <: P | r - P`, literal-only blocks, the coverage
    error in a match) is unchanged.
12. **Narrowing.** Unchanged. §2.6.2 uses "taken" at every place.
13. **Keys.** For `v.k` and `v.f`, `v` must have a known type, and every member
    must be data with that key, or a labelled type whose data has that key and
    whose constructor may be used here. The result is the union of those parts
    with the type arguments of the bound forms, as today for `.f`.
15. Removed.

Alternatives use spec §4.8 in every shape group; they are not restricted to
fitted groups. The pattern type of a conjunction `p1 = … = pk` is the
intersection of the pattern types of its parts that are not catch-alls; a name part gets that
type. A catch-all is a name, a discard, or a conjunction of catch-alls. A data
pattern has the data type of its items' pattern types at their keys (with new
type variables, as today for tuples); a rest gets the data type of the
remaining keys. The fitting of a constructor pattern needs `B <: Q | r - Q`
(§2.12).

Known types: "A record literal has the record…" becomes "A data literal has
the data of the known types of its items, with `_` for an item that has none;
a spread contributes the known parts of its base". "A clause block that is
not the right side of a match has a known function type…" becomes "A chain
literal has, for each shape group, a known function type made as for a
function expression; a chain with several groups has the known multi-shape
type of §2.6.5." Remove "as in TypeScript, a function that uses its own result
in `.f`, `.k`, or a call by label needs a written `->` part" except for `.f`
and `.k`. "When a function expression has an expected type with one member, a
function type with the same number of parameters" becomes "whose parameter
type has a member of the function's shape".

#### 5.2.9 §7 Patterns: changed bullets

Grammar as §3.4. Then:

- A `Name` must be a constructor. Its data is matched by a data pattern
  `C(…)`, with every key of its data type or `..` at the end. `C` alone
  matches a labelled value whose data is `()`.
- A data pattern `( … )` matches data of its shape whose parts match its
  items. A positional item matches the next position; `label = p` matches the
  part under `label`; `..` leaves out the other keys; `..r` binds them as data.
  The unique-known-shape requirement for `..` is in spec §7.1.
- `p1 = … = pk` matches what every part matches. Directly inside a data
  pattern, an item that begins with `name =` is a key item.
- `x: T` is unchanged: a type annotation where it is fitted and not in an
  alternative, a test otherwise.
- `p1 | p2` occurs only as an item of a parameter group. Every alternative
  binds the same names; a name gets the union of its types. `|` is looser
  than `=`.
- The table of §7.2 replaces the rows for `C(p1, …)`, `(p1, …, pn)` and
  `x = p` by rows for `C(…)`, a data pattern, and a conjunction.
- §7.4: "All records count as one type constructor" is removed; data of one
  shape is one type constructor. §7.5: "When a pattern `x = C(…)` matches"
  becomes "When a conjunction with a name part `x` and a part `C(…)` matches".

## 6. Type checker and run time

### 6.1 `FnTy` with one data parameter

```text
// today (types/mod.rs:94-100)
pub(crate) struct FnTy { pub params: Vec<(Option<String>, Ty)>, pub ret: Ty, pub err: Ty, pub env: Ty }
// after
pub(crate) struct FnTy { pub param: Ty, pub ret: Ty, pub env: Ty }
```

- **One need row.** `err` is removed (stage E3): errors are the `Fail(E)`
  entry of `env`. `Fail`'s parameter keeps `Mode::Row`, so error unions keep
  today's cheap set treatment; `env` keeps `Mode::Env`.
- **Directed rows (stage E1).** The `equal` of service arguments
  (`solve.rs`:1135-1141, and in `handle` and `add_bound`) becomes the
  variance-directed constraint of R4 and rule A. `add_bound`'s check of a new
  upper bound against only the first bound with the same head
  (`solve.rs`:1228-1235) was justified by equality and is replaced by one
  canonical entry per (row variable, constructor) with fresh argument
  variables; each new bound is constrained against it, which keeps it linear.
  The handled obligation on row variables (`solve.rs`:630-658) implements
  rule B.
- **Polymorphic types (stage 7).** An internal `Ty::Poly(vars, body)` for a
  written type with `TypeParams` in a parameter, field or annotation;
  checking against it skolemises with a level-escape check, each use
  instantiates (§2.16). Declared type parameters of a function literal stay
  flexible.

- `param` is a data type (`Ty::Con(Head::Shape(s), …)`), a union of data
  types of different shapes, `Never`, or a type variable.
- `.params` is used at 181 places in `crates/ilang-check/src/types`
  (evaluation §4.3, 28 inside `records/`). Each becomes a use of `param`.
- The arity failures (`FailKind::Arity`, solve.rs:1147-1150, 1443, 2094) become
  ordinary shape mismatches of the parameter data:
  `(Fn f, Fn g) => constrain(g.param, f.param); …`.
- Labels move from `FnTy` into the data shape, where they are part of the
  type (O11). The label merge rule of today's §4.6 disappears.
- Display: `(Int, port = Int) -> Conn`; a union parameter as
  `{ (Float) | (Float, Float) } -> Float`; `Never` as `{ Never } -> R`.

### 6.2 Shape heads; removal of open records and of `Ty::Graph`

- `Head::Tuple(u32)` (29 uses) and `Head::Record` become
  `Head::Shape(ShapeId)`. A `ShapeId` is interned per program: the number of
  positions and the sorted list of labels. Parts are stored in canonical
  order: positions, then labels in UTF-8 byte order. All parts are covariant
  (spec §4.5 rule 5).
- Records are deleted: `types/records.rs` and `types/records/*.rs` (5,176
  lines), `Record`, spreads in types, forgetting, label parameters and their
  key slots, the record branches of `constrain_in` (solve.rs, 66 lines naming
  the record API), the record parts of `scheme_wire.rs` (105), `infer.rs` (41),
  `display.rs` (25), `value_types.rs` (23), `known.rs` (20), `alias.rs` (10),
  `mod.rs` (10) (evaluation §6 counts).
- **`Ty::Graph`.** It is a variant of the core type (`types/mod.rs`:73), and
  unions and intersections route records through its pool (solve.rs:1866-1951).
  Every producer of a graph is a record operation (`self.record(…)`,
  solve.rs:257, 1015-1191, 1965; infer.rs:1491, 1808-1811, 2464). Closed
  labelled data is an ordinary `Ty::Con(Head::Shape(..))`, which unions merge
  by head like tuples today. So after the records are removed, nothing makes a
  `Ty::Graph`: the variant, its pool in the solver, `records::Node`, the BDD
  demand budget and the graph codec in `scheme_wire.rs` are deleted, and every
  `Ty::Graph` arm is removed. The service modes `Mode::Env` and `Mode::Row`
  are not record machinery and stay (solve.rs:21-26, answers 5a).
- `members`, `disjoint`, `one_member` (known.rs:22-120) lose their graph arms.
  `match_k` matches data by shape and part.

### 6.3 The selection algorithm in the checker

- **Resolve** (`resolve.rs`): `clause_function` (resolve.rs:3325) is replaced
  by a chain lowering: group the branches by shape; each group becomes one
  HIR function whose parameters are the keys of its shape and whose body is a
  `Case` over those parameters with one row per branch, in written order
  (guards kept). A fitted group (one branch, no guard) has no `Case`; its
  parameter group is the parameters' patterns, as a function today. A chain
  with one group is that function. A chain with several groups is a new HIR
  node `Multi(groups)`.
- **Selection tables** (§2.6.1) are computed from the HIR of the written
  binding (as `key_slots` is) and stored in `Scheme.branches`. For each branch:
  shape, written pattern (a `Pattern` per key), and after inference the types
  `Dom_i`, `R_i` as parts of the generalized scheme, so one instantiation of
  the scheme gives consistent `Dom_i` and `R_i` for all branches.
- **Rule 11 per group.** `pattern.rs` keeps one result type per branch
  (`R_i <: R`), so that `F_sel` can use only the selected `R_i`.
- **At a call** (`infer.rs`, call rule): when the callee is a name with a
  table, compute the shape from the syntax, the group, the known types of the
  items (`infer_k`), then skip branches by §2.6.2 with `coverage.rs`'s
  "taken" sets per place and `known::disjoint`. Build `F_sel` from the
  instantiated scheme and constrain the call against it.
- **Arguments.** Extend `open_fn` (known.rs:124-135) so that a name with a
  table of more than one branch is checked in the second round; then select by
  its expected type (§2.6.4).
- **Interfaces.** `scheme_wire.rs` gets a branch-table artifact. The interface
  of a multi-shape exported binding is its table, since the precise type has
  no surface syntax (evaluation §4.2).
- **Display (D7).** After simplification, a union equal to the expansion of a
  declared name of the members' module, or of an imported name in scope, is
  shown by that name: `List(Option(Int))`, not `List(Some(Int) | None)`.
  When two names match, the one declared in the module of the members wins,
  then the first in module order. Stage 1.
- **B2.** Today's solver is exponential on unannotated recursive matches over
  parametric unions of one-constructor types (1.1 s at 7 members, 78 s at 9,
  evaluation B2). Every sum becomes such a union, so this is fixed first
  (stage 1). The cause is not diagnosed.
- **`Bool`.** `infer.rs`:254 finds the type `Bool` by name for `==` and
  guards; it becomes the expansion of the prelude name `True | False`.
  `resolve.rs`:2423-2424 maps the prelude types `Nil` and `Bool` to host types;
  `Bool` becomes "a type equal to `True | False`".

### 6.4 Run time

**Calls.** A function of shape `S` with `n` keys is a `$fn{n}`; its
parameters are the parts in canonical order (positions, then labels in UTF-8
order). The argument data of a call is never built: the caller works out the
items in written order into locals and passes them in canonical order. A
parameter group cannot name the whole argument data, so nothing needs it.
`f(..t)` passes the parts of `t`. `(a, b)` and `(a, port = p)` are both
`$fn2`; types guarantee that a single-shape closure is only called with its
own shape.

**Closures.** Field 2 (`i64`) of `$clo{n}` holds the arity today and is used
only for display (emit/mod.rs:286-291, inspect.rs:266). It becomes the
**shape id**: for a positional shape the id is `n`, so every closure built
today is unchanged; a shape with labels has an id at or above 2^32 into a
static shape table (labels as strings). `inspect` reads the table for labels.

**Multi-shape values** (evaluation §4.4, decided by O18). A new final subtype
of `$obj`, `$multi`, with tag 7 (so `inspect` and identity equality treat it
as a function), field 2 set to −1, and a field holding a `$values` array of
pairs (shape id, group closure). Each group closure is an ordinary `$clo{n}`
with its own captures.

**Indirect call sites.** Today every closure call is
`call_ref $fn{n} (ref.cast (ref $clo{n}) f) env args (struct.get $clo{n} 8 (ref.cast (ref $clo{n}) f))`
(emit/mod.rs:566-570, 1150-1154). After:

```text
(block $slow
  (local.set $c (br_on_cast_fail $slow eqref (ref $clo{n}) f))
  ({return_}call_ref $fn{n} (local.get $c) env args (struct.get $clo{n} 8 (local.get $c)))
  (br $done))
({return_}call $dispatch{n} f (i64.const shape) env args)
```

- `$dispatch{n}` casts to `$multi`, scans for the static shape id, and ends in
  `return_call_ref` of the found group closure. A miss is unreachable by
  typing.
- The fast path has one type test, where today there are two casts of the same
  atom. Not measured.
- All 11 sites need the slow path: 4 in emit/mod.rs and 7 in emit/control.rs,
  including `$call0`, `$call1`, `$catch_run`, `$ensure_run` and the
  `return_call_ref` at control.rs:494, because a body passed to
  `control.boundary` or `resource.ensure` can be a multi-shape value with a
  `()` shape. `$catch_run` is deleted in stage E3 (§2.10.7); if stage 10
  comes later, it has 10 sites.
- **Tail calls** (spec §11.6): a tail site uses `return_call $dispatch{n}`;
  the dispatcher ends in `return_call_ref`, so no frame is left.
- **Suspension.** A non-tail closure call is a `Stmt::Suspendable` whose
  handler wraps the call (ir.rs:50-63, emit/mod.rs:546). The fast and the slow
  path are the same statement and one frame record covers both. The dispatcher
  must not stay on the stack; ending in `return_call_ref` ensures it.
  Re-entry functions that call closures (control.rs:796-805) get the same slow
  path.
- **Host exports** are single-shape (D11), so the export wrapper's call
  (emit/mod.rs:1150-1154) needs no slow path.
- **Direct calls** of a named chain go to the group function of the call's
  shape: no dispatch.

**Services.** `$env` is unchanged (§2.10.7). A provide `e <- s` is today's
`LetEnv` over `e` (one array copy); a service pattern item reads its slot of
`$env` at entry, as an environment parameter does today
(`lower.rs`:2302-2307); `..env: N` keeps the `$env` pointer; `escape` throws
the `$error` tag with `(b, v)`.

**Matches.** `e |> chain` lowers to the one-position group's `Case` in place,
as a `Clauses` right side does today (parser.rs:1015-1033). A function literal
after `|>`, which is an ordinary call today, becomes a match.

**Data values.** A materialized data value is a tuple (tag 4) whose field 1
(`i32`) is the index of its label set in a static table (0 for no labels, so
today's tuples are unchanged) and whose `$values` hold the parts in canonical
order. `()` is a tuple with no parts. `.k` and `.f` are static indexes, since
the shape is known. Equality and hashing compare the label-set index, the
number of parts and the parts. The record
run time (`$record_get`, tag 14, runtime/values/record.rs) is deleted.

**Labelled values** keep today's custom-type representation (tag 5,
constructor id, fields): the fields are the data's parts in canonical order.
The special representations chosen by name for the prelude (lower.rs:905-940:
`Bool`, `Nil`, `Result`, `AssertionFailure`, `Assert`) move from the old type
names to the new labelled types (`True`, `False`, `Nil`, `Ok`, `Error`,
`AssertionFailure`, `Assert`).

**Numeric lowering.** Every function used as a value keeps its ordinary
reference entry, and scalar workers are reached only from known calls
(numeric-lowering.md:392-401). The entries of a multi-shape value are the
reference entries of its groups; a direct call with a static shape may still
go to that group's worker. No conflict found; the worker selection code was
not read.

### 6.5 Formatter and printer

- `crates/ilang-syntax/src/format.rs` (1,778 lines) and `print.rs` (1,001):
  print a chain on one line when it fits; otherwise the binding's `=` ends the
  line and each branch begins a line, the second and later with `| `. A guard
  stays inside the group. Data items `label = e`.
- Groups are inserted where the new precedence needs them: an operand of a
  binary operator that is a `|>` expression, a `|>` right side that is not a
  name, call or chain.
- A name whose right side is a data type is printed with its group,
  `Point = { (Int, Int) }`; a labelled type without.
- Line breaks before a statement that begins with `(`, `-` or `-.` stay as
  today; the clause variant of that rule is gone.

### 6.6 LSP and editors

- **LSP** (`crates/ilang-lsp`, 2,902 lines): hover on a call of a named chain
  shows the selected branches and `F_sel`; hover on the name shows the shape
  groups. Go-to-definition from a call goes to the selected branches.
  Completion offers the labels of a known shape after `(` and after `.`.
  Hover on a function lists the `Fail(…)` entries of its need separately as
  "possible errors" and the other entries as services; an inlay hint after a
  call shows, in grey, the services that step takes (review gap 18). Hover on
  `<-` of a provide shows the keys it provides and which of them the left
  side needs. Signature help shows `<A, …>` and the need.
  Signature help lists one signature per shape group. Semantic tokens: labels
  in data and types, the guard `?`, the join `|`.
- **tree-sitter** (`editors/tree-sitter-i`): remove `clause_block`, `clause`,
  `record`, `record_type`, `custom_type`, `environment_parameter`; add
  `branches`, `function` with `parameters` and `guard`, `data`, `data_type`,
  `data_pattern`, `labelled_item`, `spread`, `rest`, `type_parameters`,
  `service_pattern`, `needs`, `provide`, `type_definition` with an `opaque`
  alternative; remove `environment_read`, `environment_statement`,
  `error_type`, `environment_type`. `provide` is the lowest precedence, then
  `|>`. The 10 corpus files are rewritten; `05-records.txt`
  becomes `05-data.txt`.
- **VS Code and Zed** grammars (`editors/vscode`, `editors/zed`): scopes for
  `|` after `}`, `?` in a group, `label =` in brackets; remove the clause
  arrow scope.

### 6.7 `i/syntax` positions

`Position` (lib/i/syntax.i:55-73) becomes, in this order [R4]:

```text
ImportItem  ExportItem  Item  Attribute  Type  TypeItem  FunctionResult
Expression  ValueItem  Statement  Pattern  PatternItem  Segment  Format
```

- `Constructor` is removed (no constructor lists).
- `Clause` and `ClauseHead` are removed.
- `Parameters`, `Parameter`, `Field` and `CallArgument` (the first version of
  this note had `Parameters` and `Parameter`; today's list has `Field`,
  `Parameter` and `CallArgument`) merge into one "item of data" position per
  kind of data:
  - `ValueItem`: an item of a data value or of a call's data: `e`,
    `(= label e)`, `.. e`.
  - `TypeItem`: an item of a data type: `T`, `(= label T)`.
  - `PatternItem`: an item of a data pattern or of a constructor pattern:
    `p`, `(= key p…)`, `..`, `.. r`; and, only in the parameter data of a
    function, `(| p1 p2 …)`.
- The parameter part of a function is in position `Pattern`: a data pattern
  `(, …)`, or, only there, the guard form `(? (, …) g)`.
- A branch of a chain is in position `Expression`.
- The items of a service pattern and of a need are in `PatternItem` and
  `TypeItem` (`(: x T)`, `.. (: env N)`, `T`, `.. N`); the result form
  `(<- R …)` is in position `FunctionResult`; the Square of type parameters
  is part of the function form. No position is added.

`position_id` (lib/i/syntax.i:437) and `show_as` follow the new order.

### 6.8 Lisp surface (meta.md §2.3): new rows

Rows not listed are unchanged. A function body is any one expression (O20):
`{ … }` is one of them, a block, but a body need not be one. The printer of
the normal surface puts a body that is not a block into `{ }`.

**Files and items**

| Core form | Normal surface | Lisp surface |
|---|---|---|
| Labelled type | `U = (T1, x = T2)`, `U(A) = (A)`, `U = ()` | `(= U (, T1 (= x T2)))`, `(= (U A) (, A))`, `(= U (,))` |
| Opaque type | `U = Opaque`, `U(A) = Opaque` | `(= U Opaque)`, `(= (U A) Opaque)` |
| Name (alias) | `U = T`, `U(A) = T`, `U = { (T1, T2) }` | `(= U T)`, `(= (U A) T)`, `(= U { (, T1 T2) })` |
| Custom type `U { … }` | removed | removed |

The `{ }` around the right side of a name whose right side is a data type is
part of the item form, not a type group: it is kept in `Syntax` and is not
removed for macros (meta.md §3.1), so a macro can tell a name from a labelled
type.

**Types**

| Core form | Normal surface | Lisp surface |
|---|---|---|
| Data type | `(T1, x = T2)`, `(T)`, `()` | `(, T1 (= x T2))`, `(, T)`, `(,)` |
| Function type | `(T1, x = T2) -> R`, `() -> R` | `(-> (, T1 (= x T2)) R)`, `(-> (,) R)` |
| Function type with a need | `(T) <- (Db, Log) -> R`, `(T) <- N -> R`, `(T) <- (..N, Db) -> R` | `(-> (, T) (<- R (, Db Log)))`, `(-> (, T) (<- R N))`, `(-> (, T) (<- R (, .. N Db)))` |
| Polymorphic function type | `<A>(A) -> A`, `<A, N>(A) <- N -> A` | `(-> [A] (, A) A)`, `(-> [A N] (, A) (<- A N))` |
| Error part, environment part `(! (^ R E) V)` | removed | removed |
| Multi-shape function type | `{ (T) \| (T, T) } -> R` | `(-> (\| (, T) (, T T)) R)` |
| Tuple type, record type | removed | removed |

- The parameter part of a function type is a type whose members are data
  types: one `(, …)`, a `(| …)` of them, or a name that expands to them. The
  normal printer writes every parameter part that is not one data type in a
  group, `{ Args } -> R`.

**Expressions and statements**

| Core form | Normal surface | Lisp surface |
|---|---|---|
| Call | `f(e1, x = e2)`, `f()`, `f(..e)` | `(f e1 (= x e2))`, `(f)`, `(f .. e)` |
| Data | `(e1, x = e2)`, `(e)`, `()`, `(..e, x = v)` | `(, e1 (= x e2))`, `(, e)`, `(,)`, `(, .. e (= x v))` |
| Function | `(p1, x = p2) { e }`, `(p) -> R { e }`, `() { e }` | `(-> (, p1 (= x p2)) _ e)`, `(-> (, p) R e)`, `(-> (,) _ e)` |
| Function with a service pattern | `(id: String) <- (db: Db) -> User { e }`, `(x) <- (log: Log) { e }`, `(f) <- (..env: N) { e }` | `(-> (, (: id String)) (<- User (, (: db Db))) e)`, `(-> (, x) (<- _ (, (: log Log))) e)`, `(-> (, f) (<- _ (, .. (: env N))) e)` |
| Function with type parameters | `<A, N>(x: A) -> A { e }` | `(-> [A N] (, (: x A)) A e)` |
| Provide | `e <- (s1, ..env)`, `app() <- live()` | `(<- e (, s1 .. env))`, `(<- (app) (live))` |
| Function with a guard | `(p1, p2 ? g) { e }` | `(-> (? (, p1 p2) g) _ e)` |
| Chain | `F1 \| F2` | `(\| F1 F2)` |
| Pipe, match | `e \|> f`, `e \|> F1 \| F2` | `(\|> e f)`, `(\|> e (\| F1 F2))` |
| Environment read `(! x N)`, environment statement `(! N e)` | removed | removed |
| Clause block, clause, record, record update, environment parameter | removed | removed |

- `<-` in the result position of a function or function type (the third part
  of `(-> …)` after the optional Square) is `(<- R needs)`: the result type
  (`_` for none) and the need or service pattern. In an expression position,
  `(<- e s)` is the provide, with `e` first as in the normal surface.
- Type parameters are a leading Square right after `->`: `(-> [A N] …)`. A
  parameter part is never a Square, and `<>` (string concatenation) and
  `(< …)` (comparison) are other heads, so this is unambiguous.
- `(| …)` in the position of an expression is a chain: two or more parts, each
  a three-part `(-> …)`. Today `(| a f)` is the pipe (`lisp/to_ast.rs`:489,
  `encode.rs`:969); the pipe's head becomes `|>`, the spelling of its token in
  the normal surface, as for every other operator.
- `(,)` is the empty data wherever data is written, and so the empty
  parameter data. The empty Round `()` has no legal position; `(f)` is
  a call with one part, not an empty Round.

**Patterns**

| Core form | Normal surface | Lisp surface |
|---|---|---|
| Data pattern | `(p1, x = p2)`, `(p, ..)`, `(p, ..r)`, `()` | `(, p1 (= x p2))`, `(, p ..)`, `(, p .. r)`, `(,)` |
| Constructor | `C`, `C(p1, x = p2)`, `C(p, ..)` | `C`, `(C p1 (= x p2))`, `(C p ..)` |
| Conjunction | `p1 = p2 = p3` | `(= p1 p2 p3)` |
| Alternatives in a parameter item | `p1 \| p2` | `(\| p1 p2)` |
| Two patterns for one value `x = p` | replaced by the conjunction | |

- In position `PatternItem`, `(= n …)` whose first part is a name is a key
  item: `n` is the label and the other parts are the pattern (one part) or a
  conjunction (more). So the normal `(size = whole = (w, h))` is
  `(, (= size whole (, w h)))`, and `((a, b) = args)` is
  `(, (= (, a b) args))`. This is P7 in the Lisp surface.
- The statement `t = (a, b) = e` is `(= (= t (, a b)) e)`.

**Examples, checked against these rows**

```text
// normal
connect = (host: String, port = p: Int) -> Conn { open(host, p) }
pick = (0, y) { y } | (x, _) { x }
label = (opt) { opt |> (Some(v) ? string.is_empty(v)) { "empty" } | (Some(v)) { v } | (None) { "none" } }

// Lisp
(= connect (-> (, (: host String) (= port (: p Int))) Conn (open host p)))
(= pick (| (-> (, 0 y) _ y) (-> (, x _) _ x)))
(= label (-> (, opt) _
  (|> opt (| (-> (? (, (Some v)) ((. string is_empty) v)) _ "empty")
             (-> (, (Some v)) _ v)
             (-> (, None) _ "none")))))
```

The bodies `(open host p)`, `y`, `"empty"` are single expressions, not
blocks; the normal printer writes them as `{ open(host, p) }`, `{ y }`,
`{ "empty" }`.

#### Cost of the data-form parameters

The fourth round chose `(-> (, a b) R e)` over the first version's
`(-> (a b) R e)`. Compared plainly:

- **Ambiguity: none found.** The parameter part is an ordinary data pattern
  node, so a macro reads it with the same code as any `(, …)`; the first form
  needed a separate "list of parameters" reading of a Round that otherwise
  reads as a call (`(a b)` is the call `a(b)` everywhere else). Function and
  function type are still told apart by the number of parts and the position,
  as today. The guard form `(? (, …) g)` and the multi-shape `(| (, …) …)` have
  heads that no data pattern has. One-parameter data patterns are distinct:
  `(-> (, (, a b)) _ e)` takes one pair, `(-> (, a b) _ e)` takes two values.
- **One asymmetry.** A call keeps its items flat after the head, `(f a b)`,
  while a function's parameters are a `(, …)` node. So a macro that turns a
  parameter list into the matching call writes `(f ~..items)` from the items
  of `(, ~..items)`. The first form had the same asymmetry in another place
  (parameters as a bare list, data as `(, …)`).
- **Size.** One token more per function and per function type.
- **Migration of `.is` files and macros.** There are three `.is` files:
  `std/src/test.is` (14 lines), its copy
  `tests/review/cases/review-k4s/std/src/test.is`, and
  `tests/spec/6/interpolation_macro/src/macros.is` (3 lines). `test.is` has
  12 function literals with parameters, 4 quoted zero-parameter templates or
  patterns `` `(-> () _ {…}) `` and 13 clauses; `macros.is` has 1 function.
  Rust tests embed about 110 more Lisp functions (`(-> (` in
  `crates/ilang-cli/tests/macro_run.rs` 55, `crates/ilang-syntax/tests/lisp_syntax.rs`
  16, `lisp_datum.rs` 13, `crates/ilang-check/tests/quote_patterns.rs` 7,
  `macro_names.rs` 6, `crates/ilang-cli/tests/compile_time_tests.rs` 4, and
  fewer elsewhere). All of these change in stage 8 anyway: the clauses become
  branches, the pipe head `|` becomes `|>`, records and labels change, and
  `Node` patterns gain `Node(…)`. The `,` is one more mechanical edit in the
  same pass, done by re-rendering the converted tree (`lisp/from_ast.rs`,
  `render.rs`), so its own cost is small: the converter rows and the review of
  quote patterns, which must now say `(, ~..ps)` and `(,)` where they said
  `(~..ps)` and `()`. A quote pattern written in the old form no longer matches
  converted code; the migrator rewrites quotes in `.is` files and Rust strings
  with the same rows.

No cost was found that the first form does not also have; the form stays.

For example, two definitions of `std/src/test.is` after the conversion
(`text` uses the new `Node`):

```text
(= text (-> (, x) _
  (|> ((. syntax node) x)
      (| (-> (, ((. syntax Node) ((. syntax Text) s))) _ s)
         (-> (, _) _ (panic "test name must be a string literal"))))))
(= append (-> (, xs ys) _
  (|> xs (| (-> (, []) _ ys) (-> (, [x .. rest]) _ [x .. (append rest ys)])))))
```

Today they are `(-> (x) _ {(| … {(-> ((. syntax Text) s) s) …})})` and
`(-> (xs ys) _ {(| xs {(-> [] ys) …})})`. A clause pattern `p` becomes the one
item of a branch's parameter data, `(, p)`, as in the normal surface (§7.3).

### 6.9 Macros

Typed macros (meta.md §7) that name record graphs (meta.md:730, 818-825) lose
them; a macro sees data types as `(, …)` Syntax. The schema and codec
libraries planned in records.md §5.8-5.9 must be built another way (draft §7;
not studied here).

`meta.md` §2.3 takes the rows of §6.8, §3.1 takes the `Node` and `Position`
definitions of §2.8 and §6.7 (its `TypeView(TypeHandle)` member, which
`lib/i/syntax.i` does not define, becomes a labelled member
`TypeView = (TypeHandle)` if it is kept), and the datum-error paragraph of
§2.3 says that `()` is an error in every position and `(,)` is the empty
data. A macro that reads a node writes `(Node(Name(t)))` or `(Node(n: Int))`
where it wrote `Name(t)` or `Int(n)`, and builds `Node(Text(s))` or
`Node(5)` for `make`.

**Macro signatures** (review gap 9). The macro function type of meta.md:382,
today `(Syntax, …) -> Syntax ^ SyntaxError ! Definitions | Items | Diagnostics | Assert`,
becomes `(Syntax, …) <- (Assert, Definitions, Diagnostics, Fail(SyntaxError), Items) -> Syntax`;
the generated wrapper text (`crates/ilang-macro/src/prepare.rs`:243) is
regenerated in stage E3.

**`suite`** (review gap 10). The `suite` macro of `std/src/test.is`:11 copies
`(! service value)` statements into test bodies today; it is rewritten to
wrap each body as `(<- body (, value …))`, the provide of §2.10.5.

## 7. Migration

### 7.1 Spellings that keep their form and change meaning

| # | Spelling | Today | After | Loud or silent |
|---|---|---|---|---|
| 1 | `(list: List(A), where: …)` parameters | Labelled parameters | Positional binders | Silent for positional callers; labelled callers become errors |
| 2 | `{ … }` in an expression | Clause block or block by five tests | Always a block | `{ (x) -> Int { x } }` silently becomes a block giving a function (G34); `{ p -> e }` becomes an error |
| 3 | `\|>` level | Above `<>`, below `+` | Lowest | `x \|> g == 2` becomes an error; `a == b \|> f` silently regroups |
| 4 | Inner `name = pattern` in a parameter or data pattern | Alias binder (and label) | Key | Silent in a parameter group: `(t = (a, b))` becomes "label `t`" |
| 5 | `T = (A, B)` | Transparent alias | Labelled type | Silent until a value of `(A, B)` is used as `T` |
| 6 | `e \|> (x) { … }` | Ordinary call of a function literal | Match in place | Silent: no frame, tail position and stop messages change |
| 7 | `()` | Empty record, type `Record` | Empty data, type `()` | Silent for values; types change |
| 8 | Inspect text of records and labelled constructor values | `(label: value)`, `User("ada", 36)` | `(label = value)`, `User(age = 36, name = "ada")` | Silent: printed output changes |
| 9 | `x<-1` | `x < -1` | A provide of `1` to `x` | Loud: a compile error (none in the corpus) |
| 10 | An error raised inside a function kept by `keep` and called after its `catch` ended | Goes to the caller's `fail.catch` | Same with the signature of §2.10.8; a `Fail` captured elsewhere stops with "boundary not active" | Loud at run time, with a warning at compile time |
| 11 | `fail.raise`, `fail.catch` | Built-in operations | Library code over `Fail` and `control.escape` | Silent; frames in stop messages may change |

Spellings that become errors (loud): `label: value` in calls and records,
`(name: T)` in types, labels in function types, `T { … }`, `T {}`, `x |> {}`,
`{}`, clause guards `p ? g -> e`, environment parameters `x ! T` in groups,
positional construction of labelled fields, punning in constructor patterns,
`^ E` and `! N` in types, the statements `! T = e` and `x ! T`, binding heads
`name(A) = …` of functions, a need of an entry that the target does not
provide, the `providers` section of `i.toml`.

### 7.2 Staged plan

The repository is green after every stage. The legacy parser is kept beside
the new one from stage 8 to stage 12 (SYN2 did the same:
`crates/ilang-syntax/examples/migrate-syntax2.rs` uses `parser_legacy`).
`I` below is `target/release/i` built from the stage's tree.

| # | Stage and scope | Files | Acceptance | Mechanical | On today's language |
|---|---|---|---|---|---|
| 0 | This note, amended for services and effects and for type parameters (§2.10, §2.16, O23 to O32); spec replacement text staged in `i future-SYN3` fences, with fences for R1 to R6 and rules A to C | `docs/design/syntax-3.md`, `docs/spec.md`, `scripts/check-snippets.json` | `scripts/check-snippets` exits 0 | No | Yes |
| 1 | Solver: unannotated recursive matches over parametric unions of one-constructor types (B2); display folds unions to declared names (D7) | `crates/ilang-check/src/types/solve.rs`, `display.rs`, `shown.rs`, `shown_tree.rs`; new `scripts/gen-syn3-union.py`; new fixtures `tests/spec/4.4/syn3_union_perf/` (12 members, generated) and `tests/spec/4.7/syn3_display_name.i` | `timeout 2 I check --project tests/spec/4.4/syn3_union_perf` exits 0; `cargo test -p ilang-cli --test spec --locked` passes, with the display fixture expecting `first : (xs: List(Option(Int))) -> Int` | No | Yes |
| E1 | (after 1) Directed need rows: R4 merge by direction, rule A for function subtyping, rule B (handled obligation, directed), one canonical entry per (row variable, constructor) | `crates/ilang-check/src/types/solve.rs`, `infer.rs`, `display.rs`; fixtures under `tests/spec/9/` | Fixtures: `both = (f ! Fail(Int \| String)) { need_int() }` checks (today's syntax); a provider `Yield(Never)` for a need `Yield(Int)` is rejected; `collect` over a body emitting `Int` and `String` shows `List(Int \| String)` (review gap 13); the Q1-B split of `catch` is rejected; a covariant merge to `Never` is an error; `cargo test -p ilang-cli --test spec --locked` | No | Yes |
| E2 | `control.escape`; light (escape-only) boundary; library `Fail`, `raise`, `catch` beside today's `i/fail` | `crates/ilang-wasm/src/emit/control.rs`, `emit/mod.rs`, `ir.rs`, `lower.rs`, `lib/i/control.i`, `lib/i/fail.i`, prelude | A fixture runs `catch` + `raise` through nested catches and `resource.ensure`; `escape` on an inactive boundary stops; the bench gate of §2.10.7 (≤ 1.25x today's `fail.catch`, with and without an error) | No | Yes |
| E3 | (after E1, E2) Remove the error channel: `-> R ^ E ! N` becomes `-> R ! Fail(E) \| N` everywhere; entries get the platform `Fail` (O31); host-service, export and `@external` glue (§2.10.12); macro signatures (§6.9); delete `FnTy.err`, `Op::Raise`, `Op::Catch`, `$fail_catch`, `$catch_run`, `$catch_reenter` | migrator subcommand `errors`; `types/*`, `ilang-wasm`, `ilang-macro`, `lib/**`, `std/**`, `tests/**`, `examples/**` | `! grep -rn 'Op::Catch' crates`; `scripts/dev-verify` passes | Migration yes; checker and codegen no | Yes |
| E4 | (after E3) Std API: about 140 `-> Result(` signatures report through `Fail`; callers and tests; `resource.acquire` on `Scope` (§2.10.9) | `std/src`, `std/test`, `lib/i`, examples | `scripts/dev-verify`; `json.parse("{")` under `fail.catch` gives `Error(…)` | Partly (per-function judgement) | Yes |
| E5 | (after E3) Entry stub and per-target platform table; delete the provider plan (§2.10.11, spec §2.6.1); per-target entries in the package file; `task.Scope(E)` renamed `task.Group(E)` | `crates/ilang-check/src/providers.rs`, `types/provider.rs`, `crates/ilang-package/src/providers.rs` (deleted), `ilang-wasm/src/lower.rs` provider section, `emit/host.rs`:575-640, `lib/provider-plan.toml`, the three fixture packages under `tests/spec/9.11/`, provider tests | `test ! -e crates/ilang-check/src/providers.rs`; `i run` of `examples/echo`, `http-get`, `http-server` works; `i check --target js` of a program needing `Net` gives the one "not provided by the target" error | Deletion yes; stub no | Yes |
| 2 | Fitting of constructor patterns `B <: Q \| r - Q`, with the disjointness warning (D4) | `types/pattern.rs`, `infer.rs`, `coverage.rs`; fixtures `tests/spec/4.8/syn3_refutable_member.i` (expects a stop `assertion failed: pattern Some(q) did not match None`) and `tests/spec/4.8/syn3_refutable_disjoint.i` (expects the warning) | `cargo test -p ilang-cli --test spec --locked` | No | Yes |
| 3 | Types in today's syntax: every multi-constructor type becomes one-constructor types plus an alias, prelude included (`True { True }`, `False { False }`, `Bool = True \| False`, …); `T {}` stays; then a definition with two or more constructors is a compile error | migrator `crates/ilang-syntax/examples/migrate-syntax3.rs` (subcommand `types`); `lib/**`, `std/**`, `tests/**`, `examples/**`, docs; `crates/ilang-wasm/src/lower.rs`:905-940 and `crates/ilang-check/src/resolve.rs`:2423 (prelude representations), `types/infer.rs`:254 | `cargo run -p ilang-syntax --example migrate-syntax3 -- --check types` reports 0 multi-constructor definitions; `scripts/dev-verify` passes | Yes, after 1 and 2; wrappers of a built-in of the same name and other name clashes are a manual list (§7.3); `i/syntax.Node` is rewritten by hand to `Node { Node(Name \| Discard \| … \| Int \| Float \| …) }` with one-constructor member types, which is today's spelling of §2.8's `Node` | Yes |
| 4 | Labels in today's syntax: labelled calls of non-constructor functions become positional (evaluation order kept); constructor calls give every labelled field by label; constructor patterns write every label (no punning); labels in function types removed (also `@external`, host-service fields, doc tables); then a labelled argument to a non-constructor function is a compile error | migrator (subcommand `labels`, resolver-aware: runs on the checked HIR's spans); `types/infer.rs`; fixture `tests/spec/6.6/syn3_label_call_rejected.i` (`t9(second: 1, first: 5)` expects a compile error) | `cargo test -p ilang-cli --test spec --locked`; `scripts/dev-verify` | Yes (needs the resolver) | Yes |
| 5 | `\|>` level in today's syntax: group every `a \|> f OP b` (OP at levels 1 to 5) as `{ a \|> f } OP b` and every `a OP b \|> f` as `a OP { b \|> f }`; then move `\|>` to the lowest level | migrator (subcommand `pipe`); `crates/ilang-syntax/src/parser.rs`:1005-1056; fixture `tests/spec/6.9/syn3_pipe_weakest.i` | `scripts/dev-verify` passes before and after the level change, with no source edit between | Yes | Yes |
| 6 | Remove open records in today's syntax: `Record`, type spreads, forgetting, label parameters, string and `UpName` labels, `i/record`, width subtyping; records become closed labelled data; tuples and records share `Head::Shape`; `()` has the type `()`; `Ty::Graph` deleted; record run time deleted; text form `(label = value)` | `crates/ilang-check/src/types/records.rs`, `records/` (deleted), `solve.rs`, `infer.rs`, `known.rs`, `display.rs`, `scheme_wire.rs`, `value_types.rs`, `alias.rs`, `mod.rs`; `crates/ilang-wasm/src/runtime/values/record.rs` (deleted), `values.rs`, `inspect.rs`, `lower.rs`; `lib/i/record.i` (deleted); the 23 test files that use open records rewritten or deleted; `tests/t3-record-*.py` (deleted) | `test ! -e crates/ilang-check/src/types/records.rs && ! grep -rn "Ty::Graph" crates`; `scripts/dev-verify` passes; fixture `tests/spec/11/syn3_data_equality.i` (`(a: 1) == (a: 1)`, `(1, 2) != (a: 1, b: 2)`, inspect `(a = 1)`) | No | Yes |
| 7 | `FnTy` with one data parameter and one need row (no `err`) in today's syntax; arity failure becomes a shape mismatch; unions of functions of different shapes give a `Never` parameter; internal `Ty::Poly` with rigid checking and the level-escape check (§2.16) | `types/mod.rs`, `solve.rs`, `known.rs`, `infer.rs`, `display.rs`, `shown.rs`, `scheme_wire.rs`, `pattern.rs`, `coverage.rs`; fixture `tests/spec/4.4/syn3_fn_shapes.i` (`xs = [(a) { a }, (a, b) { a }]` checks with no E0502 and its shown parameter part is `{ Never }`) | `cargo test -p ilang-cli --test spec --locked`; a Rust unit test: `(x) { 1 }` against `<A>(A) -> A` fails and `(x) { x }` passes | No | Yes |
| 8 | New parser, AST, printer, formatter, Lisp rows of §6.8 (data-form parameters, pipe head `\|>`, chain `\|`), `i/syntax` positions, tree-sitter, editor grammars, LSP syntax; the migrator converts every `.i`, `.is`, embedded Rust string and doc fence in one commit; spec §§ replaced by §5.2; chains with one shape group only (a chain with two groups is a temporary compile error); guards; `<-` (lexer P0, needs, service patterns, `e <- s` at the weakest level, P14, P15); `<A, …>` on function literals and types; no `!`, `^`, `name(A) =`; the Lisp rows of §6.8 for them | `crates/ilang-syntax/src/{lexer,parser,parser/*,ast,ast_eq,ast_clone,print,format,shape,walk}.rs`, `lisp/*`, `parser_legacy.rs` (kept); `crates/ilang-check/src/resolve.rs` (chain lowering), `hir.rs`; `lib/i/syntax.i`; `editors/**`; `crates/ilang-lsp`; migrator subcommand `convert` | `cargo run -p ilang-syntax --example migrate-syntax3 -- --verify` (for every file: `ast_eq(new_parse(print(convert(old_parse(f)))), convert(old_parse(f)))`, and no file outside the manual list fails to convert); `scripts/check-snippets`; a fixture file with `find_user`, `fail.raise`, `fail.catch`, `map`, `live` and a block provide (§2.10) parses and checks; `scripts/dev-verify` | Conversion yes; parser no | No |
| E6 | (after 8) `..env: N` service patterns and library `keep`; delete `control.keep_env` only if its fixture passes on the library version (§2.10.8) | `infer.rs`, `lower.rs`, `lib/i/control.i`, `lib/i/task.i` | `tests/spec/9/keep_env.i` and a `task.keep` error-propagation fixture pass on `keep` | No | No |
| 9 | Shape groups in the checker: chains with several groups; direct calls choose the group by shape; the multi-shape value type; a multi-shape value may only be called directly (temporary error) | `types/infer.rs`, `pattern.rs`, `coverage.rs`, `known.rs`, `scheme_wire.rs`; fixtures `tests/spec/6.12/syn3_connect.i` (answers 3c `connect`, runs) and `tests/spec/4.8/syn3_area_value.i` (`fs = [area]` shows `fs : List({ (Float) \| (Float, Float) } -> Float)`) | `cargo test -p ilang-cli --test spec --locked` | No | No |
| 10 | Run time: `$multi`, shape ids in closures, the slow path at the 11 call sites, display; lift stage 9's temporary error | `crates/ilang-wasm/src/emit/mod.rs`, `emit/control.rs`, `ir.rs`, `lower.rs`, `runtime/values/inspect.rs`; fixtures `tests/spec/11/syn3_multi_value.i` (`fs = [area]`, `Some(f) = list.first(fs)`, prints `f(2.0)` and `f(3.0, 4.0)`), `tests/spec/11/syn3_multi_tail.i` (10^6 tail calls through a multi-shape value, constant stack), `tests/spec/9/syn3_multi_suspend.i` (a call through it across `control.suspend`) | `cargo test -p ilang-cli --test spec --locked` (both optimization levels); `scripts/bench` closure-call case within noise of stage 9 | No | No |
| 11 | Selection by known types and by expected types; direct aliases | `types/infer.rs`, `known.rs`, `coverage.rs` (taken at every place) | fixture `tests/spec/4.8/syn3_select.i`: `s: String = show(1)` checks; `gcd` checks; `x: String \| Int = f(Some(5))` checks and `y: String = f(Some(5))` is rejected; `g = show; t: String = g(1)` checks; `h = (x) { show(x) }` shows `h : (Int \| String) -> String \| Int`; `list.map([1, 2], show)` shows `List(String)`; a chain with one generic branch (`<A>(x: A) { x } | (x: Int, y: Int) { y }`) selects and instantiates per branch; then `cargo test -p ilang-cli --test spec --locked` | No | No |
| 12 | Delete the legacy parser, clause-block AST, brace decision, label and record leftovers, leftovers of `!`, `^` and binding-head type parameters, spec exceptions, stage 9's temporary error | `crates/ilang-syntax/src/parser_legacy.rs` and users | `test ! -e crates/ilang-syntax/src/parser_legacy.rs`; `scripts/dev-verify` | Yes | No |

Order constraints of the effects stages: E1 follows stage 1 (the B2 fix);
E3 needs E1 and E2; E4 and E5 need E3 (E5 because of the platform `Fail`);
stage 8 follows stage 0 and the owner's answers O29 to O32 (given); E6 needs
stage 8. E1 to E5 work on today's syntax, because `-> R ^ E ! N` rewrites
mechanically to `-> R ! Fail(E) | N`; stage 8 then prints that as
`<- (Fail(E), ..N) -> R`. Rough sizes (review Q2, Q8): checker 2,500 to 2,800
lines, syntax 700 to 900, std migration 1,500 to 2,500 changed lines,
provider-plan deletion about 2,650 source and 1,550 test lines.

The evaluation's stages map as: its 0-6 are 0-6 here; its 8 (`FnTy`) is split
into 7 (representation, on today's language) and 9 (groups); its 7 is 8; its
9 is 10; its 10 is 11; its 11 is 12. Stages 0 to 7 are valid on today's
language and useful even if the rest is not done (the evaluation names 1, 2,
3 and 5).

### 7.3 What the migrator does for each form

The migrator reads with the legacy parser and, from stage 4, with the
resolver's facts; it writes with the new printer. A form not listed is
printed unchanged.

| Form today | Written after | Notes |
|---|---|---|
| `T { C1(A) C2 }` | `C1(A) = (A)`, `C2 = ()`, `T(A) = C1(A) \| C2` | Each constructor keeps only the type parameters its fields use (stage 3). Field labels become data labels: `C(x: T)` becomes `C = (x = T)`. A one-constructor type whose constructor has the type's name becomes one labelled type. |
| A constructor `C(B)` whose one field is the built-in type named `C` | suggested: drop the wrapper | O21. The migrator does not rewrite it; it lists it with the suggested text: the sum gets `B` as a member directly; `C(e)` becomes `e`; the pattern `C(p)` becomes `p: B` when `p` is a name or a discard, else `(p = _: B)`, and is a type test because the group has several branches. The suggestion is valid only when no other member of the sum then shares `B`'s type constructor (§2.8); otherwise the wrapper stays and is renamed by hand. When the sum has another wrapper whose content is a different built-in, as `i/syntax.Node`'s `Int(Int)` and `Float(Float)` (lib/i/syntax.i:43-44), the sum becomes a labelled type with one union item, `Node = (…)` (§2.8), so that its constructor still marks the values. |
| A constructor whose new type name clashes otherwise | manual list | A constructor named like another type of its module or an imported type. |
| `T {}` | `T = Opaque` | |
| `T = (A, B)`, `T = (x: A)` | `T = { (A, B) }`, `T = { (x = A) }` | Keeps today's alias meaning (G16). |
| `T(..)` export of a multi-constructor type | `T(..)` | D8 gives the same set. `T` alone becomes `T, C1, C2` (types only). |
| Clause block `e \|> { p1 -> e1  p2 -> e2 }` | `e \|> (p1) { e1 } \| (p2) { e2 }` | A tuple pattern `(a, b)` becomes the item `((a, b))`. |
| Clause with choices `p1 \| p2 -> e` | `(p1 \| p2) { e }` | |
| Clause with a guard `p ? g -> e` | `(p ? g) { e }` | |
| Clause pattern `n1 = … = nk = p` (names first) | `(p = n1 = … = nk)` | P7; when `p` is also a name, `(_ = n1 = … = p)`. |
| A clause block as a value | a chain literal | `{ Some(u) -> u.name  None -> "" }` becomes `(Some(u)) { u.name } \| (None) { "" }`. |
| `x \|> {}`, `{}` | `core.never(x)`, `core.never` | |
| Parameter `(t = (a, b))` | `((a, b) = t)` | Same for nested data patterns. |
| Parameter `(x = label)`, `(x = label: T)` | `(_ = x = label)`, `(label: T = x)` | Today the label is `label` and both names are bound; signatures become positional (D1). |
| Environment parameters `(a, log ! Log) -> T { body }` | `(a) <- (log: Log) -> T { body }` | The service pattern in parameter order; the `! N` of the `->` part goes into the type only (§2.10.1). |
| Environment reads `log ! Log` in a block (the first version of this note) | a service pattern item of the enclosing function | Never written by users; listed for completeness (review gap 6). |
| Function type `(P) -> R ^ E ! N` | `(P) <- (Fail(E), ..N) -> R` | The members of `N` are written one by one; a row variable becomes `..N`; `^ Never`, `! Never` disappear; a need that is empty writes no `<-`. Stage E3 does the semantic part on today's syntax (`-> R ! Fail(E) \| N`); stage 8 prints it. |
| Result part `-> R ^ E ! N` of a function literal | `-> R` with the needs left to inference, plus a service pattern for the services the body reads | A written `! N` that is wider than what the body needs is kept as a type annotation on the binding (`f: (P) <- … -> R = …`), so that the type does not change. |
| Environment statement `! T = e` followed by the rest `r` of a block | `{ r } <- (e)` | Consecutive statements whose values are direct values (spec §6.10) merge into one provide data `(e1, e2)`; otherwise they nest, `{ { r } <- (e2) } <- (e1)`, because `e2` may need `e1`. When `r` is the whole rest of a function body, the provide becomes the body's last expression. |
| Binding head `f(A, E, N) = (…) -> … { … }` | `f = <A, N>(…) -> … { … }` | A parameter used only in `^` parts is dropped: its `Fail(E)` is carried by the row variable (`count(A, E, N)` becomes `<A, N>`, §4.1). A declaration `f(A): (A) -> A` becomes `f: <A>(A) -> A`. Type definitions are not touched. |
| `fail.raise(e)`, `fail.catch(f)` | unchanged text | Now library code (§2.10.6). |
| `control.keep_env(f)` | unchanged until stage E6, then `keep(f)` | §2.10.8. |
| `resource.acquire(open, release, use)` | `resource.scoped(() { use(resource.acquire(open, release)) })` | Stage E4 (§2.10.9). |
| `providers` section of `i.toml` | deleted; overrides written in `main` as `app() <- (…)` | Stage E5. |
| `x<-1` | `x < -1` | None in the corpus; the migrator checks. |
| Labelled call of a function `f(b: g(), a: h())` | `f(h2, b1)` with locals | Positional in parameter order. When the order changes and two or more moved arguments are not direct values (spec §6.10), bind each such argument, in written order, to a fresh local in a group block: `{ _syn3_0 = g()  f(h(), _syn3_0) }`. Fresh names start with `_syn3_` and are checked against every name in scope. Stage 4. |
| Constructor call with positional arguments to labelled fields | `C(name = e1, age = e2)` | Stage 4. |
| Labelled constructor argument `C(name: e)` | `C(name = e)` | |
| Constructor pattern punning `User(name, age)`, `User(n = name, ..)` | `User(name = name, age = age)`, `User(name = n, ..)` | No shorthand (O12). |
| Record `(name: "ada")`, `(..e, f: v)`, `C(..v, f: e)` | `(name = "ada")`, `(..e, f = v)`, `C(..v, f = e)` | |
| Record type `(name: String)` | `(name = String)` | Inside a function type's parameter, `((a: Int)) -> T` becomes `((a = Int)) -> T`. |
| Function type labels `(key: String) -> R` | `(String) -> R` | Also in `@external` declarations, host-service fields and spec tables. |
| `a \|> f == b`, `a == b \|> f` | `{ a \|> f } == b`, `a == { b \|> f }` | Stage 5. |
| `e \|> (x) { … }` (function literal) | unchanged text | Now a match (§7.1 row 6). The migrator lists these for review of stop-message expectations. |
| Open records, `Record`, `i/record`, label parameters | not migrated | 23 test files (evaluation), rewritten by hand or deleted in stage 6. |
| Inspect expectations in tests | regenerated | `(label: value)` becomes `(label = value)`; labelled constructor values gain their labels. |
| `.is` files and Lisp strings in Rust tests | Lisp rows of §6.8 | Parameters `(a b)` become `(, a b)` and `()` becomes `(,)`; the pipe `(\| e f)` becomes `(\|> e f)`; a clause block `{ (-> p e) … }` becomes the chain `(\| (-> (, p) _ e) …)` and a match `(\| s { … })` becomes `(\|> s (\| …))`; `(: x e)` labels become `(= x e)`; `(U { … })` becomes several `(= C (, …))` items; `(! x N)` in the parameters becomes the service pattern `(<- R (, (: x N)))`; `(! N e)` statements become `(<- … (, e))`; `(! (^ R E) V)` result wrappers become `(<- R (, (Fail E) …))`; `i/syntax` node values gain `Node(…)` and lose the `Int` and `Float` wrappers. Quote patterns and templates are rewritten with the same rows. |
| Rust-embedded snippets and doc fences | converted | The `old-syntax-check` inventory lists them. |

## 8. Open questions

The questions Q1 to Q19 of the first version are settled (§1.2 O20, O21;
§1.3a) and their answers are in the rules above. One question raised by this
revision is still open; it is a detail for the coordinator (O22):

1. **Exports of the members of a labelled type whose data is a union.**
   `Node(..)` exports `Node` and its constructor, and spec §2.4 forces each
   labelled member (`Name` … `Splice`) to be exported too, so `i/syntax`
   lists thirteen items (§2.8). Recommended: keep this; D8's whole-sum
   `A(..)` stays a rule for names only. Otherwise `T(..)` on such a type also
   exports every labelled member of its data's union defined in the module,
   which saves the list but makes `(..)` mean two things for labelled types.

The four owner questions of the effects review (block provide, platform
`Fail`, per-target defaults, the cost of `catch`) are answered (O29 to O32).

Things this note could not determine:

- Whether the light boundary meets the 1.25x gate; it was not prototyped
  (review Q5 projects 1.6x with an error and 2.8x without for a plain
  boundary).
- The effect of `Fail` rows on B2 (review Q2: expected neutral or better,
  not measured).
- How `Root` and the other platform services reach the stub on every target
  beyond `request._entry` and `task._entry`, and spec §10.1.2's
  certification of a platform-provided `Target` (review Q3, not checked).

- The cause of B2 and whether a small solver change removes it.
- How `Bool` as `True | False` reaches the ABI `i32` beyond the name-based
  lookups found (lower.rs:905-940, resolve.rs:2423-2424); not traced in
  `ilang-host`.
- Whether macros can replace the field-by-field record construction planned
  for schema and JSON codecs (records.md §5.8-5.9).
- Fast-path time and code size of the new call sites (not measured).
- The numeric worker selection code (not read).
