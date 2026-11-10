# Design: metaprogramming with two surfaces

Status: implementation design, 2026-11-11. This retains the rules from
lane/note-meta at 44021cc, aligned with redesign and the user's SYN2 decisions.
The collection contract (§4.5) and conservative typed-result rule (§7.3) are
the implementation choices; their behavior is verified by implementation tests.

Sources, in order: spec.md; syntax.md as amended by the user's confirmed SYN2
decisions; literal-types.md, value-types.md, types-joint.md, records.md at the
ledger-accepted 0d0ce19 (the copy in this merge is older), js.md and runtime.md;
the plan's decided design and M tracks; the ledger through its meta restart.
The user's later removal of the test attribute also controls. The SYN2 scope
rulings retain brace type groups and the general pipe into a clause function.
All prospective normal-surface examples below use SYN2. Reference paths in
§13 are relative to `/home/agent/Programming/i/.rd/ref/`. Code citations in
§12 refer to merged redesign 2d1cfc560, not to a running implementation of macros.

## Summary

- i is one language with one **core tree** and two ways to write it. The **normal surface** is the syntax of the spec, in files that end in `.i`. The **Lisp surface** writes the core tree with brackets, in files that end in `.is`.
- A file uses one surface. A module written in one surface is used from the other with no difference.
- A **macro** is an i function that runs in the compiler. It takes code as data and gives code as data. Macros are defined only in `.is` files. They are used from both surfaces: as a call `m(…)`, after `|`, or before an item as `@m(…)`.
- There are two kinds of macro. A **syntax macro** runs before type checking and sees only code. A **typed macro** has a declared type, runs after type checking, and may ask for the types of its arguments.
- A macro may ask for the written definition of a type, also a type of another module.
- A macro has no host services, so it cannot read files or use the network, and it gives the same result for the same input on every machine.

Decisions taken with the user:

- Macros are defined only in the Lisp surface, and used from both surfaces.
- One file has one surface, chosen by the end of its name. The surfaces are not mixed in a file.
- The Lisp surface ends in `.is`.
- `@macro` in a `.i` file is a compile error. A `.i` file may have normal functions that take and give `Syntax`.
- The Lisp surface has no keywords. Where the normal surface has a symbol for a form, that symbol is the head of the form. Lists, bit arrays, blocks, and clause blocks keep their brackets.
- Quote is `` ` ``, unquote is `~`, and unquote of a list is `~..`.
- Code is given to a macro as a general tree of brackets (`Node`), not as one constructor for each core form.
- Names made by a macro are kept apart from the names of the caller with marks (§5).
- A macro may ask for written definitions (§6).
- Typed macros work as the "blackbox" macros of Scala do: the type of a call comes from the declared type only, never from what the macro gives (§7).

The user has also confirmed the decisions below, numbered 1 to 15, which were made while the draft was written and reviewed.

About the surfaces:

1. A function in the Lisp surface always has three parts, `(-> (params) Result body)`. `_` as the result means that no `->` part is written. A clause has two parts (§2.3).
2. The imports and the exports are tuples `(, …)` in the Lisp surface, as they are tuples in the normal surface.
3. A quote is a form of the core tree that only the Lisp surface can write. A `.is` file with a quote or with `@macro` cannot be changed into a `.i` file (§1).

About using macros:

4. A macro may be put before an item, `@m(args) Item`. Before a type definition, the macro only adds items and the type definition stays as it is. Before a binding or a declaration, the result takes the place of the item (§4.3).
5. In the position of a type, the arguments of a macro call are types, integer literals, and strings (§4.3).
6. `a | m` is the macro call `m(a)`, and `a | m(b)` is the macro call `m(a, b)`, as for every call (§4.3).
7. No macro call gives a statement or a pattern (§4.3, §10). An item run may contribute a collected binding (§4.5).
8. A macro may not be used in the module that defines it (§4.1).

About `Syntax`:

9. `Syntax` has no constructors. A macro reads it with `node` and makes it with a quote or with `make` (§3). A quote may be used as a pattern.
10. A name made with `make` can only reach exported names of other modules (§5).
11. The queries are services in the environment, `Definitions` and `Types`, with constructors, so a test may give its own (§6, §7.4). A type is given to a macro as the `Syntax` of a written type.
12. `i/syntax` may be used by macros and by tests. A program entry or a host export that can reach it is a compile error (§8).

About typed macros:

13. A typed macro is a declaration and a binding with the same name, both with `@macro` (§7.1).
14. A type variable of the signature that is only in the result can only be met by `Never`. So a typed macro cannot choose by the type that the position of the call needs (§7.3, §7.5, open question 5).
15. A typed macro cannot delay an argument that has errors or needs an environment (§7.5).

## 1. Two surfaces

A file ends in `.i` or `.is`; a module path omits that extension. Both files
for one module are E0624. Imports, exports, dependencies and visibility have
the same meaning in both surfaces. A `.is` file is a complete module.

The core tree carries source locations and documentation comments, and literal
values rather than their spelling. Two trees are **the same code** up to
expression/type groups only. Group removal uses the parsed node kind, never
“one child”: a one-field record and a one-clause function are not groups.
Macros receive that group-free tree. Written alias/pattern classification is
retained even when a child expansion has the shape of a different RHS (§2.3).

A `.is` file converts to `.i` only if the normal surface can express its core
tree. This applies the A1 correction in cache/gpt/report-a1d.md:

- Quotes and macro definitions have no normal-surface spelling.
- A block starting an expression whose first statement starts with an unbound
  function with a written result and only pattern parameters is not expressible.
  Function bodies are always blocks and are not this exception.
- The first unguarded single-pattern clause with an alias chain ending in a
  tuple pattern, whose expression starts with that function, is also not
  expressible when the required group would be misread as clauses.
- A non-final pattern choice ending, after any alias chain, in a type annotation
  is not expressible: the bar is consumed by the type union.

The temporary A1 E0113 lone-clause exception expired with A3; do not reinstate
it. SYN2 does not remove the remaining function/clause ambiguity. Printers
insert line breaks before statements/clauses beginning with parentheses,
braces or minus when needed, and groups around guards containing an exposed
arrow. They print postfix matches, including a grouped numeric/binary subject:
`{ 1 }.{ 1 -> True _ -> False }`. A failed conversion writes nothing;
`show_as` returns `SyntaxError`. Conversion never invents a binding, since
that can capture a name and change evaluation or generalization.

M1 parses quote forms for the M2 expansion phase. Empty attribute calls and grouped result holes remain distinct written forms; an attribute macro may wrap an item macro call.

Literal types and T2 are landed. Lowercase value types and alias patterns
therefore have rows now. Record rows follow T3's accepted algebra and SYN2's
surface. STR's suffix pattern and interpolation remain gated by STR; if its
final interpolation semantics exceed String concatenation, it must add a core
row before either surface or macros may use that form.

## 2. The Lisp surface

### 2.1 Tokens

The tokens are the tokens of spec §1 (names, discards, `UpName`s, literals, symbols, `//` comments), and two more symbols, `` ` `` and `~`. Documentation comments (`///`, `////`) work as in the normal surface.

- The rule of lines (spec §1.7) is not used. Every line end is a space.
- `,` is not used between parts. It is only the head of a tuple.
- Tokens are cut by the longest match, as in spec §1.5. So a space is needed where two tokens would be read as one: between the head `<` or `>` and a `<<` after it, and between an integer and a `..` after it (`[1 ..]`, because `1.` begins a float).
- The rule of spec §1.5 for digits after `.` is not used. In `(. e k)`, `k` is a `decimal`.

### 2.2 Datums

```
Datum   = Atom | Round | Square | Curly | Bits | Quote | Unquote | Splice .
Atom    = name | discard | UpName | int_lit | float_lit | string_lit | Symbol .
Round   = "(" { Datum } ")" .
Square  = "[" { Datum } "]" .
Curly   = "{" { Datum } "}" .
Bits    = "<<" { Datum } ">>" .
Quote   = "`" Datum .
Unquote = "~" Datum .
Splice  = "~" ".." Datum .
Symbol  = (* a symbol of spec §1.5 that is not a bracket *) .
```

- A `.is` file is a list of datums.
- `~` with `..` after it is always a `Splice`, never an `Unquote` of the atom `..`.
- An `Unquote` or a `Splice` may only be inside a `Quote`. A `Quote` may be inside a `Quote` only when an `Unquote` or a `Splice` of the outer quote is around it, as in `` `(f ~..(map xs (-> (x) _ `(g ~x)))) ``.
- The **head** of a `Round` is its first part.

The parser changes datums into the core tree with the tables of §2.3. The tables are read by **position**. The position of a datum says which rows may be used, and each row says which positions its parts are in. The positions are: import item, export item, item, attribute, constructor, field, type, function result, parameter, expression, statement, call argument, clause, clause head, pattern, bit array segment, and format. A datum that fits no row of its position is a compile error.

### 2.3 How each core form is written

In the tables, `e` is an expression, `arg` a call argument (`e`, or `(: x e)` with a label, as in the row Call), `p` a pattern, `T` a type, `R`, `E`, and `V` the types in the places of a result, of the errors, and of the environment of a function (each may be any union of spec §3, as `Log | Http`, a hole, or a group), `N` a named type (`U`, `(. a U)`, `(U T…)`, or `((. a U) T…)`), used only in the environment statement and the environment parameter, `x` and `a` a name, `C` and `U` an `UpName`, and `…` zero or more of the thing before it.

**Files and items**

| Core form | Normal surface | Lisp surface |
|---|---|---|
| Imports | `(a/b/c, x = a/b, (C, y) = a/b, x = (C, y) = a/b)` | `(, (/ a b c) (= x (/ a b)) (= (, C y) (/ a b)) (= x (, C y) (/ a b)))` |
| Exports | `(x, U, U(..))` | `(, x U (U ..))` |
| Built-in attribute | `@deprecated("old") Item`, `@host("ns", "async") Item` | `(@ (deprecated "old") Item)`, `(@ (host "ns" "async") Item)` |
| Macro before an item | `@m Item`, `@a.m Item`, `@m(arg…) Item`, `@a.m(arg…) Item` | `(@ m Item)`, `(@ (. a m) Item)`, `(@ (m arg…) Item)`, `(@ ((. a m) arg…) Item)` |
| Custom type | `U { C1(T, x: T) C2 }` | `(U { (C1 T (: x T)) C2 })` |
| Custom type with parameters | `U(A, B) { … }` | `((U A B) { … })` |
| Type alias | `U = T`, `U(A) = T` | `(= U T)`, `(= (U A) T)` |
| Binding | `p = e` | `(= p e)` |
| Binding with type variables | `x(A, B) = e` | `(= (x A B) e)` |
| Binding with type variables and a type | `x(A, B): T = e` | `(= (: (x A B) T) e)` |
| Declaration | `x: T`, `x(A): T` | `(: x T)`, `(: (x A) T)` |
| Macro call as an item | `m(arg…)`, `a.m(arg…)` | `(m arg…)`, `((. a m) arg…)` |
| Items from an expansion | none | `{ Item… }` |

- At the top level of a file, a `(, …)` is the imports when it has a `(/ …)` in it, and the exports when not. It may not be empty. As in syntax.md §2.2, a file has at most one imports `(, …)` and at most one exports `(, …)`, they are the first datums of the file, and the imports come before the exports. Any other `(, …)` at the top level is a compile error.
- `(x A …)` is the left part of a `(: …)` only in a declaration `(: (x A …) T)`, and directly under `=` in a binding item or a binding statement `(= (: (x A …) T) e)`. In every other place, as in a pattern, it is a compile error.
- A module path `(/ …)` has two or more names. An import item `(= …)` has two parts, or three when it has both a name and a list of names. Every other `(= …)` has exactly two parts.
- An item `(U { … })` or `((U A …) { … })` is a custom type. An
  `(= U T)` is an alias. For `(= (U P …) rhs)`, apply T2 P1 to the written
  RHS before child expansion: a bare value type after groups makes an alias
  pattern; every other RHS makes a generic alias. The outer U in a pattern
  is a reference. Bare UpName parts bind fresh aliases; `_` skips; nested
  named/tuple parts match shapes. Imported-name collisions are errors, not
  tests. A qualified outer head remains an ordinary constructor-pattern
  binding, as spec §4.3 requires. Add `AliasPart` to the positions of §2.2.
  This replaces the old import-dependent classification and all speculative
  “may define this head” checks. See value-types.md P1–P5 and M2/M3.
- Every generated alias-pattern item is classified when it first becomes an
  item. Once classified it cannot change kind by expanding a child.

**Types**

| Core form | Normal surface | Lisp surface |
|---|---|---|
| Union, intersection, difference | `A \| B \| C`, `A & B`, `A - B - C` | `(\| A B C)`, `(& A B)`, `(- A B C)` |
| Function type | `(T1, x: T2) -> R` | `(-> (T1 (: x T2)) R)` |
| With errors and environment | `(T) -> R ^ E ! V` | `(-> (T) (! (^ R E) V))` |
| With errors only, environment only | `(T) -> R ^ E`, `(T) -> R ! V` | `(-> (T) (^ R E))`, `(-> (T) (! R V))` |
| Tuple type | `(T1, T2)`, `(T)` | `(, T1 T2)`, `(, T)` |
| Value type | `x`, `a.x` | `x`, `(. a x)` |
| Record type | `(x: T)`, `(..R, -x, y: U)`, `Record` | `{ (: x T) }`, `{ .. R (- x) (: y U) }`, `Record` |
| Named type | `U`, `a.U`, `U(T1, T2)`, `a.U(T)` | `U`, `(. a U)`, `(U T1 T2)`, `((. a U) T)` |
| Hole | `_` | `_` |
| Group | `{ T }` | `{ T }` |
| Literal type | `"a"`, `1`, `-1` | `"a"`, `1`, `(- 1)` |
| Macro call | `m(T, 3)`, `a.m(T)` | `(m T 3)`, `((. a m) T)` |

- `|`, `&`, and `-` take two or more parts in a type. A part may be a `|`, `&`, or `-` form; that is the type with a group there, `A | { B | C }`.
- A string/integer atom is a literal type; `(- k)` is a negative literal
  type. Float types are invalid. Type macro arguments are exactly types, so
  `m(-1)` is valid syntax by types-joint.md §5 and literal-types.md §5.9.
  Bare lowercase names are value types; a joined call is always a type macro,
  never runtime type application. AliasPart is a separate position: no calls
  or records there, even though a bound alias may denote a record.
- A function result is `R`, `(^ R E)`, `(! R V)`, or `(! (^ R E) V)`. No other use of `^` and `!` is correct there. So `(x) -> Int ! Log | Http { … }` is `(-> (x) (! Int (| Log Http)) …)`.
- A named type with arguments has one or more arguments.

**Expressions and statements**

| Core form | Normal surface | Lisp surface |
|---|---|---|
| Literal | `1`, `1.5`, `"a"` | `1`, `1.5`, `"a"` |
| Name, member of a module | `x`, `C`, `a.x`, `a.C` | `x`, `C`, `(. a x)`, `(. a C)` |
| Call | `f(e1, x: e2)`, `f()` | `(f e1 (: x e2))`, `(f)` |
| Binary operator | `e1 + e2` | `(+ e1 e2)` |
| Unary operator | `-e`, `-.e` | `(- e)`, `(-. e)` |
| Field, tuple part | `e.x`, `e.0` | `(. e x)`, `(. e 0)` |
| Tuple | `(e1, e2)`, `(e)` | `(, e1 e2)`, `(, e)` |
| Structural record | `()`, `(x: e)`, `(..e, x: v)` | `()`, `{ (: x e) }`, `{ .. e (: x v) }` |
| Match | `e.{ p -> v }` | `(. e { (-> p v) })` |
| List | `[e1, e2]`, `[e1, ..e2]` | `[e1 e2]`, `[e1 .. e2]` |
| Bit array | `<<e1, e2: f, e3: f(e4)>>` | `<<e1 (: e2 f) (: e3 (f e4))>>` |
| Block, group | `{ s1  s2  e }`, `{ e }` | `{ s1 s2 e }`, `{ e }` |
| Binding statement | `p = e`, `x(A) = e`, `x(A): T = e` | `(= p e)`, `(= (x A) e)`, `(= (: (x A) T) e)` |
| Environment statement | `! N = e` | `(! N e)` |
| Clause block | `{ p1 -> e1  p2 -> e2 }`, `{}` | `{ (-> p1 e1) (-> p2 e2) }`, `{}` |
| Clause with choices | `p1 \| p2 \| p3 -> e` | `(-> (\| p1 p2 p3) e)` |
| Clause with a guard | `p ? g -> e`, `p1 \| p2 ? g -> e` | `(-> (? p g) e)`, `(-> (? (\| p1 p2) g) e)` |
| Function with no `->` part | `(p1, p2) { e }` | `(-> (p1 p2) _ e)` |
| Function with a `->` part | `(p) -> R ^ E ! V { e }` | `(-> (p) (! (^ R E) V) e)` |
| Environment parameter | `x ! N` | `(! x N)` |
| Record update | `C(..e, x: e2)`, `a.C(..e)` | `(C .. e (: x e2))`, `((. a C) .. e)` |
| Quote | none | `` `d `` |

- A `Round` with one or more parts whose head is not a symbol is a call, but when its second part is `..` it is a record update. The head is the function. A binary operator takes exactly two parts; `-` and `-.` may take one.
- **Functions and clauses.** In the position of an expression, a `(-> …)` has three parts: the parameters, the result, and the body. The body is one expression. The result `_` means that the function has no `->` part, so its value, errors, and environment are all inferred (spec §6.8). Any other result is a `->` part, in which a missing `^` or `!` means `Never`; the normal `(x) -> _ { e }` is `(-> (x) (^ _ Never) e)`. A bare `_` alone denotes an omitted result; a grouped hole denotes a written result, equivalent to `(^ _ Never)`. A clause is a `(-> …)` with two parts, and it is only in a clause block. A function type also has two parts, and it is only in the position of a type. A macro that looks at a `(-> a b)` must know its position to know which of the two it is.
- **Records and blocks.** In expression/type position a nonempty Curly starting with a labelled field or `..` is a structural record, before any group test. Empty record is the empty Round `()` only in expression position; its type is `Record`, and there is no empty-record pattern. Record field labels are text, including String labels and T3 label parameters in types. Spread/forgetting keep T3 source order and exact Without algebra. Other curly datums use the following rule. A `{ }` in the position of an expression is a clause block when it is empty, or when its first part is a `(-> …)` with two parts. Then every part must be a clause. When not, it is a block, and it has one or more parts.
- A clause block with one clause, `{ (-> p e) }`, and the function `(-> (p) _ e)` are two distinct trees with the same function-value meaning. Rewriting a direct postfix match into a call is not an identity: preserve its tail-frame and known-constructor rules (spec §§6.12, 7.5, 11.6).
- A clause head is `p`, `(| p1 p2 …)` with two or more choices, or `(? head g)` with a head of the first two kinds. `(| …)` is a compile error in every other position of a pattern. A choice that **ends in a type test** may only be the last choice, because the normal surface reads `x: T | p` as a union type, and a pattern has no group to stop it. A pattern ends in a type test when it is `(: x T)`, or `(= x p)` where `p` ends in a type test: `(| (= a (: b Int)) Unit)` would be `a = b: Int | Unit`, which is the one pattern `a = b: { Int | Unit }`. A clause block whose first clause head is a type test, as `{ n: Int -> … }`, is read as a clause block by syntax.md §6.1.
- `(. a x)` is a member of a module or a field, as `a.x` is in the normal surface (spec §2.3). `(. a C)` is a constructor that is a member of a module, so `opt.Some(1)` is `((. opt Some) 1)`.
- **String interpolation** (stream STR) has no row. It takes only `String` values, so it is sugar for `<>`: the parser of the normal surface changes it into the `<>` of its parts, from the left, with empty string parts left out, before the core tree. `"user-\(n)"` is `(<> "user-" n)`, and `"a\(x)b"` is `(<> (<> "a" x) "b")`. A string whose only part is one expression keeps one empty string part, so that the type of the expression is still checked as `String`: `"\(x)"` is `(<> "" x)`. So a macro sees the names in it, and the printer of the normal surface writes the `<>` form back, as it writes `16` for `0x10`. The Lisp surface has no interpolation.

**Patterns**

| Core form | Normal surface | Lisp surface |
|---|---|---|
| Name, discard | `x`, `_`, `__rest` | `x`, `_`, `__rest` |
| Type test or annotation | `x: T`, `_: T` | `(: x T)`, `(: _ T)` |
| Literal | `1`, `-1`, `-.1.5`, `"a"` | `1`, `(- 1)`, `(-. 1.5)`, `"a"` |
| String prefix | `"a" <> x` | `(<> "a" x)` |
| String suffix (stream STR) | `x <> "a"` | `(<> x "a")` |
| Constructor | `C`, `a.C`, `C()`, `C(p1, p2)`, `a.C(p)`, `C(p, ..)`, `C(..)` | `C`, `(. a C)`, `(C)`, `(C p1 p2)`, `((. a C) p)`, `(C p ..)`, `(C ..)` |
| Tuple | `(p1, p2)`, `(p)` | `(, p1 p2)`, `(, p)` |
| List | `[p1, p2]`, `[p, ..]`, `[p, ..x]`, `[..]`, `[..x]` | `[p1 p2]`, `[p ..]`, `[p .. x]`, `[..]`, `[.. x]` |
| Bit array | `<<p1, p2: f, p3: f(e)>>` | `<<p1 (: p2 f) (: p3 (f e))>>` |
| Two patterns for one value | `x = p` | `(= x p)` |
| Quote pattern (§3.3) | none | `` `d `` |

- In `(: x T)`, in `(<> "a" x)` and `(<> x "a")`, and in `[p .. x]` and `[.. x]`, `x` is a name or a discard, as in spec §7.1. The `(: (x A …) T)` of a binding with type variables and a type is not a pattern. In `(= x p)`, `x` is a name. In `(- k)` and `(-. k)`, `k` is a literal of the kind of the operator.
- The statement `a = (x, y) = e` is `(= (= a (, x y)) e)`.

**Datums that are compile errors.** These fit no row: `()` outside an expression or a list of parameters; `(,)` in every data/type/pattern position; a named type or a type head with no arguments in brackets, `(U)` or `((U) { … })`; a constructor of a type definition with no fields in brackets, `(C1)`; a binding head with no type variables, `(= (x) e)` or `(= (: (x) T) e)`; a `(: (x A …) T)` that is not a declaration or directly under `=` in a binding; a `(, …)` at the top level that is not the one imports group or the one exports group at the start of the file; an empty list of import names, `(= (,) (/ a b))`; a list that begins with `..` in the position of an expression, `[.. e]`; a part of a record update with no label, `(C .. e 1)`; an attribute before a `Curly` of items, `(@ A { … })`; a pattern of a bit array segment that is or ends in a type test, with or without a format, as `<<(= x (: y Int))>>` or `<<(: (= x (: y Int)) u8)>>`, because in a segment of the normal surface a `:` always begins the format (spec §8.1), so `<<x = y: Int>>` is the pattern `x = y` with the format `Int`.

### 2.4 An example

```i
fact = (n) {
  n.{
    0 -> 1
    _ -> n * fact(n - 1)
  }
}
```

```lisp
(= fact (-> (n) _
  (. n {
    (-> 0 1)
    (-> _ (* n (fact (- n 1)))) })))
```

## 3. Code as data

### 3.1 The types

The library module `i/syntax` exports these types:

```i
Syntax {}

Node {
  Name(String)
  Discard(String)
  Up(String)
  Symbol(String)
  Int(Int)
  Float(Float)
  Text(String)
  Round(List(Syntax))
  Square(List(Syntax))
  Curly(List(Syntax))
  Bits(List(Syntax))
  Quote(Syntax)
  Unquote(Syntax)
  Splice(Syntax)
  TypeView(TypeHandle)
}

TypeHandle {}

Position {
  ImportItem  ExportItem  Item  Attribute  Constructor  Field  Type  FunctionResult
  Parameter  Expression  Statement  CallArgument  Clause  ClauseHead  Pattern
  Segment  Format
}

SyntaxError { SyntaxError(message: String, at: Syntax) }
```

- A `Syntax` value is one datum of §2.2 with where it came from: its span in a source file, and its **scope**, which says where its names are looked up (§5). `Syntax` has no constructors. Only the compiler, a quote, and `make` give `Syntax` values. Type query handles (§7.4) are an explicit compiler-only extension, not source datums.
- `Position` has one constructor for each position of §2.2, in the same order (`Segment` is the bit array segment). So `show_as` can print any part that a macro takes out of a block, a clause block, a function, or a definition.
- `node(s)` gives the `Node` of a `Syntax`. `Name`, `Discard`, `Up`, and `Symbol` have the text of the token. `Int`, `Float`, and `Text` have the value of the literal.
- An atom is always one token of its kind. The text of a `Name`, a `Discard`, an `Up`, or a `Symbol` is one name, one discard, one `UpName`, or one symbol of spec §1 that is not a bracket. The value of an `Int` or a `Float` is one that a literal can write: it is not negative, and a `Float` is not `-0.0`. A negative number is the `Round` `(- 1)`, as in a pattern (§2.3). `make` gives a `SyntaxError` for any other atom, as `Int(-1)`, `Name("Foo")`, `Name("a b")`, `Name("")`, or `Discard("x")`. So every source-form `Syntax` value can be written as text, and an atom always fits the rows of §2.3 that its kind of token fits.
- **The `Syntax` of a piece of code is the datum that the Lisp surface writes for it** (§2.3). This is true for code from a `.i` file too. So a macro always sees the Lisp form: the node of the argument `n > 0` is a `Round` of three parts whose nodes are `Symbol(">")`, `Name("n")`, and `Int(0)`.
- A macro never sees a group. The compiler takes every group out of the code it gives to a macro (the arguments of a call, the item under a macro before an item, and the answers of `Definitions` and `Types`), and puts its one part in its place. A group is a `Curly` with one part in the position of an expression, when that part is not a clause, and a `Curly` in the position of a type or of a function result. So code that is the same code (§1) gives the same `Syntax`, and the groups that the printer of the normal surface adds, as in `x * { y + z }`, do not change how a quote pattern matches. A group that a template makes, as `` `{ ~x } ``, stays in the result, where it has no effect.
- `Syntax` has no documentation comments in it.
- `==` on two `Syntax` values is `True` only when they are the same value. To compare code, use `same_code`.

### 3.2 Functions of `i/syntax`

| Function | Type | What it gives |
|---|---|---|
| `node` | `(Syntax) -> Node` | The node of the value |
| `make` | `(node: Node, like: Syntax) -> Syntax ^ SyntaxError` | A new `Syntax` with this node, and the span and the scope of `like`. A `SyntaxError` when the node is an atom that is not one token of its kind (§3.1). §5 says how a name made in this way is looked up. |
| `same_code` | `(Syntax, Syntax) -> Bool` | `True` when the two are the same datum. Spans and scopes are not looked at. Code that the compiler gives has no groups (§3.1), so for such code this is the same code of §1. |
| `show` | `(Syntax) -> String` | The datum in the Lisp surface, on one line |
| `show_as` | `(Syntax, Position) -> String ^ SyntaxError` | The code in the normal surface, read by §2.3 for this position. It is on one line, but for the line breaks that the printer must put in (§1); a `Curly` of items gives one item on each line. A `SyntaxError` when it fits no row, or when the normal surface cannot write it (§1). |
| `definition` | | §6 |
| `type_of`, `signature`, `subtype`, `members` | | §7.4 |

The bodies of `node`, `make`, `same_code`, `show`, and `show_as` are given by the compiler. `definition` and the functions of `Types` are written in i over the services `Definitions` and `Types` (§6, §7.4), so a test that gives its own service gets its own answers. §8 says where these functions may be used.

### 3.3 Quote

A quote is written only in a `.is` file. The code in a quote is a **template**. A `Quote`, `Unquote`, or `Splice` node in the result of an expansion is a compile error (§4.3), so every quote is in a source file and has the scope of that file.

As an expression, `` `d `` gives the `Syntax` of the datum `d`. Every atom and every bracket in `d` gets its own span in the file, and the scope of the module the quote is written in. Inside `d`:

- `~e` puts in the value of the expression `e`, which must have the type `Syntax`.
- `~..e` puts in every member of the value of `e`, which must have the type `List(Syntax)`. It may only be a part of a bracket.
- `` `~e `` is the same as `e`.

As a pattern, `` `d `` matches a `Syntax` value:

- An atom matches a node of the same kind with the same text or value. A bracket matches the same bracket with the same number of parts, each matched in turn.
- `~p` matches any one `Syntax` with the pattern `p`.
- `~..p` matches the rest of the parts, zero or more, as a `List(Syntax)`, with the pattern `p`. It may only be the last part of a bracket.
- Spans and scopes are not looked at. So a quote pattern compares names by their text only: `` `(assert ~c) `` also matches a call of a local name `assert` of the caller. A macro cannot ask what a name is bound to.

A quote pattern may fail, as a constructor pattern may.

In a template, `` `{ ~..parts} `` is: `{}`, the clause block with no clauses, when `parts` is empty; a group when it has one part that is not a clause; and a clause block with one clause, which is a function value, when its one part is a clause, as `` `(-> 0 1) ``.

## 4. Syntax macros

### 4.1 Defining

A **syntax macro** is a top-level binding in a `.is` file with the attribute `@macro`, and with no declaration of the same name (§7.1):

```lisp
// my/control.is
(, unless)

(@ macro
  (= unless (-> (cond body) _
    `(| ~cond { (-> True Nil) (-> False ~body) }))))
```

- The pattern of the binding is one name, with no type variables. The value is a function expression `(-> …)`. Each parameter is a name or an environment parameter.
- The value is checked against the type `(Syntax, …) -> Syntax ^ SyntaxError ! Definitions | Items | Assert`, with one `Syntax` for each parameter. If it does not have a subtype of this type, it is a compile error.
- `@macro` in a `.i` file is a compile error. An item with `@macro` in the result of an expansion is a compile error.
- A macro is exported and imported by its name, as a value is. It is in the space of value names.
- The name of a macro may only be used in these ways: in the imports and the exports, and as the macro of a macro call (§4.3). Any other use is a compile error.
- A macro may not be used in the module that defines it. Code in a template is not a use: it is data. So a template may have a call of the same macro or of another macro of its module, and that call is expanded where the template is used.

### 4.2 An example with a quote pattern

```lisp
// my/check.is
(,
  (= (, make show_as Text Expression) (/ i syntax))
  (= (, inspect concat) (/ i core)))
(, that)

(= failed (-> (code left right (! a Assert)) _
  ((. a fail) (AssertionFailure (concat [code ": " left " is not " right])))))

(@ macro
  (= that (-> (claim) _
    (| claim {
      (-> `(== ~left ~right)
          `{ (= l ~left)
             (= r ~right)
             (| (== l r) {
               (-> True Nil)
               (-> False (failed ~(make (Text (show_as claim Expression)) (: like claim))
                                 (inspect l) (inspect r))) }) })
      (-> _ `(assert ~claim)) }))))
```

Used in a `.i` file:

```i
(check = my/check)

total_is_right = (cart) { check.that(total(cart) == 42) }
```

`failed` is not exported, and the caller may have names `l` and `r`. §5 says why both are correct.

### 4.3 Using

A **macro call** is written in one of these ways, in both surfaces. `m` is a name, or `a.m` with a module name `a`, that is a macro at that position. A local name covers a macro of the same name, and a local name covers a module name, as for every name (spec §2.3).

| Position | Written | Arguments | The result must be |
|---|---|---|---|
| Expression | `m(args)`, `e \| m`, `e \| m(args)` | Expressions, with or without labels | An expression |
| Item at the top level | `m(args)`, `a.m(args)` | Expressions, with or without labels | An item, or a `Curly` of zero or more items |
| Before an item | `@m Item`, `@m(args) Item` | Expressions, with or without labels, and then the item as the last argument | See below |
| Type | `m(args)` | Types, integer literals, and strings, with no labels | A type |

- The parser does not know which names are macros. The arguments are always read by the grammar of the position, as in the table, so a file can be read and formatted with no other file. From this, an argument in the position of an expression cannot be a type such as `Int | Nil` or `(Int) -> String`, a discard, or a list pattern with `..`.
- In the normal surface, syntax.md already reads a macro call as an item, a macro before an item with any arguments or named with its module (`@a.m`), a macro call in a type, a call whose first argument is an `UpName` at the start of a statement or an item, as `decode(User, text)`, and a declaration `name [TypeParams] : Type` with no `=`, whatever its attributes, also one with no `@external` and one with a macro before it, as `@rpc fetch: (Id) -> User ! Http` (§1). It keeps each of them for macros, and the macro streams give them their meaning.
- `e | m` is the macro call `m(e)`, and `e | m(args)` is the macro call `m(e, args)`, as `|` works for every call (spec §6.9). This is only in the position of an expression.
- The number of arguments must be the number of parameters of the macro. An argument with a label goes to the parameter with that label, by the rule of calls (spec §6.6), and the macro gets it with no label.
- The arguments are not worked out. The macro gets their `Syntax`, and its result says if and when each of them is worked out.
- Type macro arguments use the full accepted type grammar, including negative literals, value types and records; alias-pattern parts are not type macro positions.
- A name in a bit array format, as `size` in `<<x: size(8)>>`, is never a macro call.
- No macro call gives a statement: the result of a call in the position of an expression is an expression, also when the call is a statement of a block. So an expansion cannot add a name to the block around it, and which names are local at a call does not depend on other expansions.

**A macro before an item.**

- Before a **type definition**, the macro cannot change the item. The type definition stays in the module as it is written, and the result of the macro, an item or a `Curly` of zero or more items, is added after it. So a type definition that is written is always the definition of its type.
- Before a **binding or a declaration**, the result, an item or a `Curly` of zero or more items, takes the place of the item. To keep the item, the macro puts it in its result.
- The item is given to the macro with the attributes that are written after the macro. The item always goes to the last parameter, also when another argument has a label.
- A built-in attribute before a macro, as in `@deprecated("old") @m f = …`, is a compile error; write it after the macro, so that it is a part of the item. An attribute whose name is one of the built-in attributes (`external`, `js`, `host`, `deprecated`; see §11 for the `.is` phase marker `macro`) is that attribute; a macro with such a name is used with its module name, as `@a.test`, which syntax.md §2.2 reads as a macro.
- An attribute macro before an item macro call receives that call as its item argument. Its resulting items are expanded in the normal order, including any item calls it returns. Built-in attributes retain their ordinary applicability rules and cannot attach directly to an unexpanded item call.
- With more than one macro before an item, the first is expanded first, and it gets the item with the other macros still on it.
- The rule of spec §2.2 that an attribute is not given two times is for built-in attributes only.
- The documentation comment of the item stays on the type definition, or goes to the first item of the result that defines the same name.

**Limits on what a result may have.**

- Imports cannot be in a result. Export requests and collected bindings use §4.5; a raw import/export header is not an item.
- A `Quote`, `Unquote`, or `Splice` node cannot be in a result.
- The attributes `@external`, `@js`, and `@host` may be in a result only when the whole `(@ …)` datum, with the name atom, the arguments of the attribute, and the item it is on, came from a source file of the caller and reached the result through the arguments of each run, unchanged. Each such datum may be in the result at most one time. Having no marks is not enough: a name made with `make` (§5) is never such an atom, also when it has no marks after the run. So a macro of a package that the program depends on cannot add a host import that the caller did not write: it cannot reuse the caller's `external` atom with other arguments, put the caller's `@js(…)` or `@host(…)` on an item of its own, or give one declaration two times.
- A `Curly` of items may only be the whole result. A `Curly` in the position of an item anywhere inside a result, directly inside the `Curly` of the result or under a `(@ …)`, is a compile error.

### 4.4 Expanding

The compiler expands a macro call by calling the macro with the `Syntax` of the arguments. One call of the macro is one **run**.

- Expanding goes from the outside to the inside. An argument is given to the macro as it is written, with the macro calls in it not expanded. After a call is expanded, the macro calls in its result are expanded in the same way.
- The result is read by the tables of §2.3 for the position of the call. If it fits no row, it is a compile error at the call, and the message names the macro.

A module is expanded in two steps:

1. **Items.** The compiler expands the macro calls in the position of an item, the macros before items, and the macro calls in the position of a type that are inside type definitions. It goes on until none is left. A run may be put off in this step (§6). After this step, the top-level names of the module are all known and every type definition is complete. Then the exports are checked: every exported name must be defined.
2. **Bodies.** The compiler expands the macro calls in the position of an expression, and the other macro calls in the position of a type. Their order has no effect on the result, because they cannot add a top-level name or a local name outside their own result.

Typed macro calls are expanded later (§7.3). Contributions and export requests commit with step 1; they cannot change the import graph.

**Limits.** The spec gives limits for one run: on the work it may do, on the memory it may use, and on how deep its calls may go. It also gives a limit for how deep expansions may be inside the results of other expansions. The limits are counted in units of the language, such as calls and the size of values, and the depth in frame units (§12), so they do not depend on the machine or on how the compiler optimizes. The ledger fixes 100,000,000 work units, 2^28 allocation words, 10,000 frame units and expansion depth 256, with no package override. M2 must publish the abstract charge table before implementation acceptance (§11). Calls alone cannot measure a bigint multiplication or a large string copy. Going over a limit is a compile error that names the macro and the limit.

**Errors.**

- A macro that gives a `SyntaxError` makes a compile error with that message at the span of `at`. When that span is in a template, the error is reported at the call, with the template as a note.
- A macro that stops (spec §11.7), also by a failure reported to `Assert`, makes a compile error at the call.

### 4.5 Collected bindings and export requests

This is a proposed general addition. It follows ExUnit's definition plus
registration, Rust's later harness collection, and Racket's module-end lifts
(§13). It does not give a macro access to a tool or an entry's capabilities.

Three designs were compared before choosing it:

| Design | Reason to take or leave it |
|---|---|
| A. Discover ordinary exported bindings by a name convention | Small, like Gleam's generated project. Misspelled names disappear; exporting every helper exposes an accidental API; heterogeneous function effects cannot be represented by one homogeneous list without losing entry-specific requirements. Keep ordinary binding definitions, reject discovery by spelling. |
| B. Mutable compile-time registry, as ExUnit module attributes | Convenient grouping and registration. Retry, cache and expansion order become observable; global state is incompatible with fresh macro runs. Keep its definition/metadata split, reject shared mutation. |
| C. Transactional contributions associated with an item expansion | Chosen. Contributions contain resolved identities and plain metadata; tools select roots after inference. Retry/cache has one commit point. More driver work, but no parser keyword, no library dispatch or runtime registration. |

Criteria: stable discovery; imported libraries work unchanged; no inference
from metadata; no effects during discovery; independent entry environments;
source attribution; deterministic retries; no new foreign authority.

The `Items` service, available only during an item expansion, exports ordinary
i wrappers with these signatures (these are API types, not external declarations):

```i
Items {
  Items(
    add: (collection: Syntax, target: Syntax, data: List((String, String)), at: Syntax) -> Nil ^ SyntaxError,
    expose: (name: Syntax) -> Nil ^ SyntaxError,
  )
}
```

`Items` is supplied to syntax macros with `Definitions` and `Assert`. A macro
may infer a smaller environment; an expression/type expansion requiring
`Items` is rejected before execution. `add` and `expose` are plain i field
calls. The compiler implements the service, as it does definition queries.

C1. `collection` names an **exported ordinary binding**, by hygienic name
resolution; its value is irrelevant and is not evaluated. It is an identity
anchor, not a magic String and not a type. Package identity, module identity
and binding identity form the key. Any package can define one. Tools refer to
it by its qualified exported name. `data` is a finite list of unique String
keys and String values. Duplicate keys are errors. Metadata carries no code,
no effect and no type equation. Examples of uses are benchmark declarations,
command dispatch entries and documentation examples.

C2. `target` names one zero-ordinary-parameter function binding defined in the
same expanded module. It may be hygienically private. A direct function or
clause function is required, so collecting an alias does not create a second
initializer or disguise a cross-module target. Its result/errors/environment
are inferred normally; no generalization or widening comes from collection.
An entry consumer chooses the required result contract and the ordinary
provider plan. There is no heterogeneous runtime list or existential cast.
A type denotes precisely its existing set of values; collection changes no
set, only which already checked bindings a build selects.

C3. Requests are attached to the returned item tree. No request is visible
while its run executes. On error, deferral, invalid output or cancellation,
discard the whole run's requests and diagnostics. On success, transform the
requests' syntax scopes by the same mark flip as the output, then resolve
against the finished step-1 module. The target must exist, be unique, and meet
C2 after checking. A discarded macro argument registers nothing. There is no
query that enumerates a partially built collection.

C4. Repeated `(collection identity, target identity)` is an error, even with
identical data. Distinct targets may have identical metadata; a tool decides
whether that is useful. The compiler adds the target's binding origin and the
`at` origin chain (source module, byte range, line/column, outer-to-inner
expansion calls). `at` must derive from this invocation's arguments or its
current call location, never a manufactured source path. Generated sites
cannot replace the call's source. Stable ordering is module path, original
call offset, nested expansion path, then request ordinal; it never uses fresh
mark numbers, hash iteration order or cache completion time.

C5. Tools read a collection after successful expansion/checking. Its manifest
contains metadata, source positions, inferred signature and an opaque target
handle. Merely importing a module does not run collected functions. Only a
consumer-selected handle becomes a root. Selection precedes provider planning
and the foreign-link pass of spec §10.1.2. The plan validates every selected
entry independently; async classification follows its transitive selected
providers (runtime D5), never the collection's name. The normal main and host
exports remain exclusively configured by source/manifest rules. Foreign items
still obey §4.3. Collection does not grant a host import or generate `@host`.

C6. `expose(name)` appends an export request to the ordinary export set after
step 1. The name must resolve to a same-module unmarked caller-scope name,
subject to §5's visibility/type-leak checks. Repeating an identical export is
idempotent; missing or marked names are errors. Constructors use the ordinary
explicit export forms. No request may add imports, change package metadata,
change configured entries, publish a foreign module's private names, or export
one module under another identity. A collection target needs no source export.

C7. Cache the result and its entire request journal together, before the mark
flip. Replay freshens **every** run mark, node identity, answer-instance binder
and source chain, including those stored in journal entries. Revalidate all
query dependencies (including negative/fallback answers in §6). Reusing a
partially expanded subtree is not a cache hit. `note(String)` is a buffered
compiler diagnostic, replayed once with the successful invocation; it never
becomes program output. It is a field of the general compiler diagnostic
service, available in both syntax and typed macro runs.

C8. No macro may generate another macro definition. Items may generate normal
types, aliases, bindings, permitted declarations, more item calls, export
requests and collection contributions. Expansion to an empty item sequence is
valid; a nested item sequence is not. Removing an attribute-marked declaration
is allowed; synthesizing, altering or duplicating foreign authority is not.

## 5. Names in expanded code

Every `Syntax` value, an atom or a bracket, has a scope. A scope is a module and a set of **marks**. The scope of an atom is used to look up its name. The scope of a bracket is only used by `make`.

- Code read from a file of the module `M` has the scope `M` with no marks.
- Code in a template written in the module `D` has the scope `D` with no marks.
- `make(node, like)` gives the scope of `like`. A name made in this way is a **made name**.
- For each run, the compiler makes a new mark `k`. It puts `k` on every `Syntax` of the arguments, and on every `Syntax` that `Definitions` or `Types` give in the run (§6, §7.4). It runs the macro, and then changes `k` on every `Syntax` of the result: one that has `k` loses it, and one that does not have `k` gets it. So the code that came from the arguments or from the compiler is as it was, and the code that the macro made has `k`. A run that is done again after it was put off (§6) is a new run with a new mark.
- A type variable in an answer of `Types` has a scope of its own kind, which names that type variable of the checker (§7.4). Rules 1 to 3 below find that variable directly, and no binding, of a template or of the caller, can bind it.
- A type parameter in an answer of `definition` (§6), where the head of the definition binds it and at each use of it in the answer, also has a scope of its own kind, which names that parameter in this answer instance. Copying a whole definition carries its binder and uses together; separate answers have distinct parameter identities, and duplicating a head freshens its bound identities as one unit. Only that binding in that head can bind it (rule 1). So a use of it that a macro moves out of the head has no binding and is a compile error, and no other binding, of a template, of the caller, or of another answer, can bind it.
- An atom in an answer of `definition` for a built-in type (§6), or in any answer of `Types` (§7.4), that names a built-in type or a built-in constructor, as `Ok` and `Error` in the answer for `Result`, or `Result`, `Int`, and `String` in the type `(Result Int String)` that `type_of` gives, has a scope of its own kind too, which names that built-in name. Rules 2 and 3 find that built-in name directly, so a top-level name, an imported name, or a name that an expansion defined, with the same text, does not cover it.
- A made name whose scope is of one of these three kinds is a compile error, unless it has the text of the type variable, the parameter, or the built-in name that the scope names.

A name is looked up in this way:

1. A name is bound by the nearest binding around it (a parameter, a pattern, a binding statement, a declared type variable, a type parameter of a type definition or a type alias), or by a name bound by an earlier segment of the same bit array pattern (spec §7.1), that has the same text **and the same scope**: the same module and the same set of marks.
2. If there is none, and the name has no marks: it is a top-level name or an imported name of its module, by the normal rules, and then a built-in name. When it is a made name and its module is not the module where the call is, a top-level name found in this way must be exported. An example is a name made like an atom of an answer of `Definitions` or `Types`, which has the scope of another module and no marks after the run.
3. If there is none, and the name has marks: it is a top-level name that an expansion defined with the same text and the same scope in the module where the call is (below). If there is none, it is a top-level name, an imported name, or a module name of **the module of its scope**, and then a built-in name. A top-level name found in this way does not have to be exported, but for a made name it must be exported. For a made name that counts as coming from an answer of `Definitions` or `Types` (below), the first part of this rule, the names that an expansion defined, is left out.

Labels of fields, of parameters, and of arguments are not names in this sense. They are compared by their text only.

From this:

- A name that a template binds cannot take a name of the caller, and a name that the caller binds cannot take a name of a template. In §4.2, `l` and `r` have a mark, so they are not the caller's `l` and `r`.
- A free name of a template means what it means in the module of the template. In §4.2, `failed`, `assert`, and `inspect` are the names of `my/check`, whatever the caller has. Two templates of different modules that are used in one run do not take the names of one another, because their scopes have different modules.
- This has a limit. All templates of one module that are used in one run get the same scope: that module and the mark of the run. So when a macro puts the result of one template inside another, a name that the outer template binds takes a free name of the inner one with the same text. If `wrap` gives `` `(-> (limit) _ ~body) `` and `check` gives `` `(< 1 limit) ``, a macro that gives `(wrap (check))` binds the `limit` of `check` to the parameter of `wrap`, not to the top-level `limit` of the module. This is a limitation of these i rules; §13 compares, but does not equate, them with Racket. The author of the macro uses names that do not clash.
- To make a name that the caller can see, a macro uses a name from its arguments, or `make(Name("it"), like: arg)`. A macro with no arguments cannot make such a name.
- A name made like an argument has the caller's scope, so as a binder it takes the caller's names with the same text in the code from arguments that it is around. So a macro makes it like an argument only for a name that the caller is to use, as a top-level name. To bind a local name that the caller does not use, with a text that the macro gets as data, as the text of a field, the macro makes it like an atom of a template: ``make(Name(text), like: `x)``. That name has the scope of the macro's module and the mark of the run, as a binder of a template has. A constructor pattern takes a labeled field only through a part whose last `=` name is the label (spec §7.1), and that name is always bound (spec §6.11), so this is the only way to read a field by its label without taking a name of the caller. For `User { User(name: String) }` and a macro `field_or(v, T, fallback)` that gives `` `(| ~v { (-> (User ~n) (if_empty ~n ~fallback)) (-> _ ~fallback) }) ``, `n` must be ``make(Name("name"), like: `x)``. With `make(Name("name"), like: v)`, the call `field_or(u, User, name)` inside `(name) { … }` would bind the caller's `name` in `~fallback` to the field, and nothing would report it.
- Only a template can reach a name of its module that is not exported. `make` cannot, by rule 2 or by rule 3, so a module cannot reach the private names of another module by using a `Syntax` that came from it: not from a template, and not from an answer of `Definitions` or `Types`.
- Services are found by their type (spec §9.2), and errors go to the nearest handler, not by name. So marks do not keep them apart: **code from a template can take the services and the error handlers of the caller's code, and the caller's code can take those of the template.** A result that puts code from an argument after an environment statement, as `` `{ (! Assert quiet_assert) ~body } ``, or inside a function that handles errors, as `fail.catch`, changes which service or which handler that code uses. This is the same as for a normal function that takes a function and calls it, and the checks of §7.3 do not stop it. `i expand` (§9) shows it.
- Because labels are text, a function in a template with the parameter `i`, used with a parameter `i` from the caller in the same list, has two parameters with one label, which is a compile error (spec §6.11). The message names the macro. The author of the macro uses another name.

**Top-level names.** When an expansion defines a top-level name (a binding, a type, a constructor) in the module `U`:

- A defined name with no marks and the scope of `U` is a normal top-level name of `U`. It may be used and exported by its text. So the names that the caller is to use come from the arguments of the macro.
- A defined name with no marks and the scope of another module is a compile error. Only an answer of `Definitions` or `Types` can give such a name, as a field name from `definition` (§6). Rules 1 and 2 would not find it from the caller's code with the same text, and a use with its own scope would go to the other module, so such a name has no clear meaning. To define a name from the text of a field, a macro makes it with `make(Name(text), like: arg)`, where `arg` came from the caller.
- An atom of a result that came from an answer of `Definitions` or `Types` may not be a binder either: a parameter, a name in a pattern, the name of a binding statement, or a declared type variable. It is a compile error, also when the answer is about a type of the module of the call. The one exception is a type parameter in the head of the definition it came in, which only that head binds (above). The compiler knows which atoms came from an answer, as it knows which came from the caller (§4.3). A name made with `make(…, like: s)`, where `s` came from an answer or is itself such a made name, counts as coming from that answer for this rule, so `make(node(f), like: f)` for a field atom `f` may not be a binder either. Without this rule, a field atom from `definition` would bind the caller's names with the same text when the type is defined in the module of the call, and would not when the type is imported, so one call would have two meanings. To bind a name with the text of a field, a macro makes it like an atom of a template, ``make(Name(text), like: `x)``, so that it gets the scope of the macro's module and, after the run, the mark of the run (above).
- A defined name with marks may only be used by names with the same text and the same scope: by code of the same run, and by atoms of answers of `Definitions` and `Types` that name it (§6, §7.4). It cannot be exported, also not as a constructor through `U(..)`. A made name that counts as coming from an answer (above) never reaches such a name: rule 3 leaves out, for it, the names that expansions defined, and goes to the module of its scope, where the name must be exported. So a macro that gets the type `Cell`, with the marks of an earlier run, from `type_of` cannot use `make(Name("reset_all"), like: t)` to reach a marked `reset_all` that the earlier run defined.
- A defined name with marks may not have the text of a top-level name, an imported name, or a module name of the module of its scope. If it has, it is a compile error. So rule 3 never has two answers.
- The type of an exported item may not use a type that its own module does not export, also when that module is another module. Spec §2.4 has this rule only for types of the same module; this is a new rule added to §2.4, because a template can bring in a type of another module.
- Every rule of the spec that compares names compares the text and the scope (module and marks), not the text alone. These are: a name is defined one time only (spec §2.2); an imported name or a module name may not be the name of a top-level name (spec §2.3); a type parameter or a declared type variable may not have the name of a type that may be used in the module (spec §4.3, §4.7); a list of parameters may not bind one name two times (spec §6.11); and a pattern may not bind one name two times, and the choices of a pattern must bind the same names (spec §7.1). The test of §2.3 on the arguments of an item of the form `(= (U A …) T)`, and the rule on an expansion that defines a type named by such an argument (§2.3), compare the text and the scope too; for an item of a source file, which has no marks, this is the same as the test of syntax.md §4.3 on the text. So a type variable `Key` that a template binds does not clash with a type `Key` of the caller, a marked `helper` does not clash with the caller's `helper`, a marked helper `json` does not clash with the caller's module name `json`, and `(, acc acc)`, with one `acc` from a template and one from the caller, binds two names. Labels are not names, so two parameters with one label are still a compile error (above).

**What the compiler does with this.** After expanding, each name with marks is changed to a name that cannot be written in a source file, and each name found in another module by rule 3 becomes a direct reference to that top-level name. In the text form of a value (spec §11.8), a constructor with marks is shown by its text only. `i expand` (§9) shows these names in a form for reading: a name with marks as `l#1`, with the marks numbered in the order they come in the output, and a direct reference as `my/check.failed`. That text is not source text.

## 6. Asking for definitions

```i
Definitions { Definitions(definition: (of: Syntax) -> Syntax ^ SyntaxError) }

definition = (of: Syntax, d ! Definitions) { d.definition(of) }
// type: (of: Syntax) -> Syntax ^ SyntaxError ! Definitions
```

`Definitions` is a service type exported by `i/syntax`. The compiler gives the service when it runs a macro. A test may give its own with an environment statement.

- `of` is a type name: an `UpName`, or `(. a U)`. Any other datum is a `SyntaxError`. It is looked up with the marks it would have in the result of this run, in two steps:
  1. First by rule 1 of §5, among the type variables around it (type parameters and declared type variables): around the place where it is written when it came from an argument or from an answer of `definition`, and around the call when not. If one binds it, it is a `SyntaxError`, also when a top-level type has the same text.
  2. Then among the type names at the top level, by rules 2 and 3 of §5. So a name from an argument is looked up where the caller wrote it.
- The value is the `Syntax` of the item that defines the type: a custom type or a type alias, with its attributes. An alias is given as it is written, not the type it stands for. A type with parameters is given with its parameters, not filled in. The `Syntax` has the scopes that the definition has in its module after expanding, but for its type parameters, which have a scope of their own kind (§5). As an argument does, it has the mark of the run only while the run lasts (§5), so its names mean the same when a macro puts them in its result. A name made like an atom of the answer reaches only exported names of that module (§5, rule 2).
- For a value-dependent alias, return its written alias or alias-pattern item,
  not its inferred fields. All aliases bound by one pattern return that item.
  This needs no type inference and agrees with T2 M2. A caller wanting the
  completed set uses `Types` after the completion barrier, not `definition`.
- What may be seen is what the module of the scope of `of` may see. A type of another module that is not exported is not defined. A name that rule 3 finds among the names that an expansion defined in the module of the call may always be seen: it is the name that the same template, in the same run, means. A type of another module whose constructors are not exported is a `SyntaxError`.
- Built-in types have no written definition, and asking for one is a `SyntaxError`. But `Bool`, `Nil`, `Result`, `Assert`, and `AssertionFailure` give the definitions that the spec gives for them. In such an answer, each atom that names a built-in type or constructor has the scope of its own kind that §5 gives, so it means the built-in name also in a module that defines its own `Error` or `Ok`.
- A name that step 2 does not find is a compile error at the span of `of`. The macro cannot catch it. A name bound by a type variable never gets to step 2, so it is always a `SyntaxError`.
- Only type definitions can be asked for. The definition of a binding cannot, because then the result of a macro would depend on the bodies of functions of other modules.

**When a definition is ready.** A type definition of another module is always ready, because that module is expanded first (§8). A type definition of the module that is being expanded is **ready** when it is in the module and no macro call is left in it. A type definition under a macro before it is in the module from the start (§4.3).

A name with marks is first looked up, by rule 3 of §5, among the names that expansions define in the module that is being expanded, also when the module of its scope is another module. So in step 1 of §4.4, a run that asks for a name with marks waits as a name of the module that is being expanded does (but for a name with its own mark, below), and is only answered from the module of its scope, or from the built-in names, when no expansion that is left can define it.

In step 1 of §4.4, when a run asks for a name of the module that is being expanded, or for a name with marks:

- If the name is defined by a ready definition, `definition` gives it.
- A name with the mark of the run that asks is never waited for. Only that run could define it, and its result is not there yet, so it is looked up as if no expansion defined it: in the module of its scope, and then among the built-in names. When the run ends, each such answer is checked against its result: when the result defines that name, so that rule 3 of §5 now finds the defined name, as a run that asks for ``(definition `Result)`` and gives its own `Result`, it is a compile error that names the macro and the name.
- If the name is not ready, and another run of step 1 is still to be done, the run that asked is **put off**: it is dropped, and it is done again from the start, as a new run, after another run has ended. This is also done when the name is a built-in name or is not defined, and, for a name with marks, when the module of its scope defines it, because an expansion may still define it in the module that is being expanded.
- When every run that is left has been put off, the compiler does them again one at a time, in the order of their calls in the module as it is at that time. Each question of such a run is answered from the module as it is at that time, as if no run that is left defined the name: from a ready definition, or else from the module of the scope and then the built-in names. A run that asks for a name that is not found in this way is put off again, and the next one is done. When every run that is left asks for such a name, each of them is a compile error, and the message gives the name that each one waits for. At the end of step 1, each answer that was given in this way is asked again of the complete module. When one is different, as when a later run defined the name, it is a compile error that gives the name and the two runs. So two `@json.derive` items in one module that ask for `String` and `Int` are both answered. A run is only an error when the runs that are left wait for one another in a circle, or when an answer that was given early is changed by a later run.
- A name is defined one time only (spec §2.2), a type definition that is written is never changed (§4.3), a name that is not ready is never answered while a run that is not put off may still define it, and every answer that is given early is checked against the complete module. So the result does not depend on the order in which the compiler does the runs.

In step 2, and in typed macros, every definition is ready.

## 7. Typed macros

### 7.1 Defining

A **typed macro** is a declaration and a binding with the same name in a `.is` file, both with `@macro`:

```lisp
// my/text.is
(,
  (= (, type_of subtype Yes No NotKnown SyntaxError) (/ i syntax))
  (= (, raise) (/ i fail)))
(, to_text)

(= int_text (-> ((: n Int)) String …))

(@ macro (: (to_text A) (-> ((: value A)) String)))

(@ macro
  (= to_text (-> (value) _ {
    (= t (type_of value))
    (| (subtype t `Int) {
      (-> Yes `(int_text ~value))
      (-> _ (| (subtype t `String) {
        (-> Yes value)
        (-> NotKnown (raise (SyntaxError "to_text: the type is not known here" value)))
        (-> No (raise (SyntaxError "to_text: this type has no text form" value))) })) }) })))
```

- The declaration is the **signature**. It is a full function type, with its type variables declared after the name. It may have errors and an environment. It may not have `_`. The rules for written types (spec §4.4) and for types in exported items (spec §2.4) are used for it.
- The binding is the **implementation**. It follows the rules of §4.1, with these changes: it has one parameter for each parameter of the signature, in the same order, and it is checked against `(Syntax, …) -> Syntax ^ SyntaxError ! Types | Definitions | Assert`.
- This is the only case where two items have one name. The declaration has neither `@external` nor `@js`; both surfaces read it as a declaration by its form (syntax.md §2.2 and §2.5 read `name [TypeParams] : Type` with no `=` as a declaration whatever its attributes, and keep the one with neither `@external` nor `@js` for macros).
- The documentation comment of the macro is on the declaration.

### 7.2 Checking a call

A typed macro is used only in the position of an expression, written as in §4.3. For type checking, the call is a call of a function with the type of the signature:

- The arguments are checked as the arguments of that function. Labels, expected types (spec §4.8), and `|` work as for a function.
- The call has the result type, the errors, and the environment of the signature, with its type variables filled in by inference as for any call. The errors and the environment of the arguments are part of the call, as for any call, also when the macro does not use an argument.
- So the types of the code around the call depend only on the signature, never on the result of the macro. Whether a binding is a direct value, and how it is generalized (spec §4.7, §6.10), are decided before any typed macro is expanded.

### 7.3 Expanding

**T1. Barrier.** Infer the binding SCC once, with typed calls represented by
their declared signatures. Complete its T2 reads and retain its ordinary
constraints and obligations. If it has an error, expand none of its typed
calls. Save this graph as an immutable snapshot. Expand outer calls before
inner calls; independent calls all see this same snapshot. This uses the
existing spec §4.7 group boundary, not source order among independent groups.

**T2. Observation.** A caller variable is *closed* if it is local to this SCC,
not generalized, and unreachable in either direction through bounds and
obligations from any published scheme, declared variable, generalized variable
or non-generalized top-level value. Every other variable is *fixed*. For query
presentation only, count a closed variable's polarities across all parts of
the instantiated signature: each argument type, result, error row and service
row starts in an output place. If every occurrence is positive, substitute
the union of its lower bounds (Never if none); if every occurrence is negative
and upper bounds exist, substitute their intersection. Otherwise keep one
opaque variable identity. Do not fabricate a universal top for an empty
intersection. Invariant service/record/nominal arguments retain identity.
Apply one substitution consistently to `signature` and `type_of`; never
replace the same variable differently at separate occurrences. Unrelated
expression queries use the same rule with their root in an output place.
This is not the shortened display type and not a solver mutation.

**T3. Validation (correction to the earlier draft).** Freeze *all* variables of
the snapshot, including closed ones, and its T2 fixation state. The old draft
allowed result code to add bounds to closed variables; that let `default()`
observed as Never manufacture a String and made validation order-sensitive.
Only fresh variables allocated for the result may acquire bounds. Validate
against the original instantiated signature and the closed components exposed
by T2. A result-only variable therefore cannot guess a value from a contextual
upper bound. This is the conservative implementation of the ledger's GM5.

Use a disposable solver transaction for the result. Preserve complete argument
nodes as checked holes with their original inferred types, expected types,
resolved binding identities and obligations. Rebuilt or partial argument code
is checked anew, but its original names retain their resolved meaning. Neither
capture of an argument's already resolved free name nor escape of a binder's
use outside that binder is allowed. No caller scheme or obligation changes
when the transaction commits. Commit only checked output code and facts for
new nodes. Later callers still see the signature, even if output is narrower
or arguments disappear. Error and environment rows also remain those of the
signature plus the original argument evaluation, as in §7.2.

**T4. Rigid proof rule.** Reuse ordinary closed subtyping, including exact T3
U3/U4 proofs. An obligation involving a frozen variable is proved from the
snapshot; it is never solved by adding a bound to that variable. For `a <: T`,
first accept reflexivity or a recorded transitive frozen-variable edge. Then
try the intersection of non-variable upper bounds reachable from a through
frozen upper edges. For `T <: a`, use the dual union of reachable non-variable
lower bounds (Never when none). Keep shared variable identities inside these
bounds. No upper bounds means “not proved”, not a top value. Direct edges to a
fresh variable add bounds only to that fresh variable; ordinary decomposition
then proves any remaining obligations. In particular `a <: r - U` also needs
the ordinary difference/disjointness proof: it is not proved just by `a <: r`.

The proof worklist memoizes completed goals, never an in-progress goal as
success. A repeated unguarded rigid goal is “not proved”; it is not evidence
for a recursive type. Reject the expansion when a required proof is not
established. Branch trials use isolated transactions; no failed trial leaks
bounds. T3 proof-budget exhaustion remains E0546, not No. This deliberately
conservative decision procedure may refuse a valid macro implementation; it
does not change principal inference of ordinary source or guess among caller
solutions. M4 acceptance requires differential identity/forwarding tests and
proof-cache replay tests; it is not a claim that today's solver implements it.

**T5. Dependencies.** New runtime value references must be in the SCC or its
strict ancestor dependency groups, or completed imported modules. A new
reference to an unrelated same-module group is rejected regardless of the
order the checker happened to visit it. Pure type definitions available at
module start are allowed. For value-dependent aliases all same-module sources
must be strict ancestors and imported sources must be completed. New direct
value-type reads are E0531, even local ones. Add no T2 demands or fixation
equations (value-types.md M3). Expand expression/type syntax macros in a typed
result under these restrictions, and prohibit item/export/collection effects.

**T6. Code and diagnostics.** Expand each generic binding once, not for each
instantiation. A complete forwarded argument keeps its type; a result's fresh
code has fresh node identities. Dropped arguments are still diagnosed from
the first check. Do not repeat those warnings. Report invalid output at the
call with its template origin as a note. Recompute runtime dependency order
from the validated output; never re-infer the caller from its expansion.

### 7.4 Asking for types

```i
Truth { Yes  No  NotKnown }

Types {
  Types(
    type_of: (Syntax) -> Syntax ^ SyntaxError,
    signature: () -> Syntax,
    subtype: (Syntax, Syntax) -> Truth ^ SyntaxError,
    members: (Syntax) -> List(Syntax) ^ SyntaxError,
  )
}
```

`Types` is a service type exported by `i/syntax`, with one function of `i/syntax` for each field, as `definition` is for `Definitions`. The compiler gives the service when it runs a typed macro.

A type is given to a macro as scoped Syntax, using the written-type rows when possible and an opaque TypeView otherwise (§2.3 plus the extension below). In a type that the compiler gives:

- The scopes are set so that every name means the same type in every position where the macro may put it. A name of a built-in type or a built-in constructor has the scope of its own kind of §5, so a top-level or imported name of the module of the call with the same text, as its own `Result`, does not cover it. A fixed variable has the scope of its own kind (below). A type of a module, with no marks there, has the scope of that module with no marks, so rule 2 of §5 finds it there. A type that an expansion defined with marks has the scope it was defined with, its module and its marks, so rule 3 of §5 finds it. These `Syntax` values have the mark of the run only while the run lasts (§5).
- The type given is the snapshot type, with the observation-only substitution of §7.3 T2. A closed variable that is in the needed type is replaced as it is there, with its places counted in the parts of §7.3, so that `type_of` and `signature()` give one answer for one variable; any other closed variable is replaced by its places in the type that is given, with the top of that type an output place. No other variable is removed or replaced: the display form of spec §4.7, which shows a variable that is not generalized as the union of the types that come to it, is not used. Only fixed variables, and closed variables that are not replaced, are left, and both are given as below. A fixed variable is always given as a variable.
- A fixed variable is given as an `Up` node, as `A`, with a scope of its own kind that names that variable (§5). That scope keeps it apart from every type name and every type variable that a template or the caller declares, also one with the same text. One variable has the same node in every answer of one run. It may be written in the result with no declaration: the rule of syntax.md §4.7 that a type variable must be declared is not used for it. The ordinary spec §4.4 place checks still apply; a frozen handle cannot evade them. It may not be in a type test of a clause (spec §7.1).
- T3 types keep exact shared graph structure, variable identities and retained
  obligations (records@0d0ce19 §§20–21). They are never widened or expanded into
  DNF merely for a query. Add an opaque `TypeView(TypeHandle)` Node case holding a checked
  type handle when the full graph cannot be written as surface type syntax.
  It is valid only in type position, cannot be made with `make`, and is scoped
  to the module check/replayed snapshot. `subtype` accepts it; `show_as` may
  give SyntaxError and `show` prints a diagnostic token, not reparseable code.
  This is an explicit exception to the text round-trip promise for compiler
  answers, not for source Syntax. A record graph is one `members` item;
  finite scalar sets still split into literal items. Cache serialization must
  use the graph bundle, not printed type text. M4 must test this gate.

| Function | What it gives |
|---|---|
| `type_of(s)` | The type that the checker gave to `s` at its position in the call. `s` must be a `Syntax` that the compiler gave to this run: an argument, an expression inside an argument, or a name that a pattern inside an argument binds. For any other `Syntax`, also one made by `make` or by a quote, and for a datum that is not an expression or such a name, it is a `SyntaxError`. |
| `signature()` | The needed type of the call (§7.3), as a function type. Closed variables are filled in, and fixed variables are given as in the list above. |
| `subtype(a, b)` | `Yes` when `a` is a subtype of `b` (spec §4.5) by §7.3 T4, with no snapshot variable given a bound. The types are taken in the form that `type_of` gives. `No` when it is not, and neither type has a type variable in it. `NotKnown` in every other case. A `Syntax` that is not a correct type, or has `_` in it, is a `SyntaxError`. |
| `members(t)` | The members of the type (spec §4.4), in the order of their `show` text. `Never` has no members. `A & Int` and `A - U`, with a fixed variable `A`, are one member each. `Bool` is one member. With literal types, a member of `Int` or `String` with a finite set of values gives one item for each value, a literal type, and a member with all values but a finite set is one item, as `String - "a"`; here only, an item is not a member of spec §4.4. So `members` of `"GET" \| "POST"` is `"GET"` and `"POST"`, and of `2 \| 10 \| Nil` it is `10`, `2`, `Nil`. The same `SyntaxError` as for `subtype`. |

- `type_of` gives the type at the position of the argument in the call. When the macro puts the argument in another position of its result, for example inside a clause, the argument keeps that type and its type variables (§7.3), and only the constraints of the new position are checked. With literal types, the type may be a literal type, as `1` for `to_text(1)`, a `Syntax` whose node is `Int(1)`, and `subtype` and `members` take it.
- A type is a set of values, and one type may be written in more than one way: `Int | String` and `String | Int` are the same type. So a macro asks `subtype` and `members`, and does not match the `Syntax` of a type by its form.
- `definition` (§6) gives the constructors and fields of a named type, and returns the written alias item for a value-dependent alias (T2 M2).

### 7.5 What typed macros cannot do

- The result type of a call cannot depend on the result of the macro. A macro that reads a table of a database cannot give its rows a type that is not in the signature.
- A result-only signature variable has no lower evidence: `signature()` gives
  `Never` if it closes, or the opaque frozen variable otherwise. It does not
  reveal the caller's desired output. A guessed `""` for `default(A): () -> A`
  is rejected. Pass the type explicitly to a syntax macro when needed.
- A typed macro cannot put an argument that has errors or needs an environment inside a function to be called later, because the signature has no way to say that the errors of the argument go to that function.
- A typed macro cannot choose by the type inside a binding with type variables.
- A typed macro cannot make items. Syntax macros do that.

## 8. Running macros

**Order of modules.** A program has no circles of imports (spec §2.1). A module is expanded and type checked after every module it imports. A macro is imported, so its module is done before the module that uses it. This is a new rule: today all modules of a program are compiled together.

**The macro program.** To run the macros of a module `D`, the compiler compiles `D` and all modules that `D` imports as one program (spec Appendix B.1), with the macros of `D` as its entry points. This is a new kind of entry point that only the compiler uses:

- `Syntax` values go between the compiler and the macro program in a form that the compiler defines. It is not seen in i.
- The entry point gets `Definitions`, `Items` for an item run, `Types` for a typed macro, the buffered diagnostic service, and an `Assert` whose `fail` stops the run. These are services of the compiler, not host services (spec §9.7). It gets no other service, so a macro cannot read a file, use the network, or read the time.
- It is a compile error when an `@external` function that is not of the library package `i` is in the full needs (spec §2.5, here also through other modules) of a macro, or of a top-level value of the macro program that is not a direct value. The functions of the library are given by the compiler, as the runtime gives them to a program.
- A macro program contains no executable `@js` binding, value getter, Js/JsRequests provider or JS initializer. Compute executable needs through the macro entries and all ordinary top-level initializers, including through other modules. If they need one, reject before instantiation, even when no macro entry needs Target. Unused runtime functions containing such declarations may be excluded from the macro program, but may not leave an initializer behind. A quoted declaration is syntax data and does not execute JS; the caller-origin checks of §4.3 still apply when it becomes an expansion. No file-reading/d.ts import tool, callback bridge or JS evaluator is part of the macro sandbox.
- The macro program is not for a target. It always runs in the compiler, whatever the program is compiled for.

**The same result everywhere.**

- The limits of §4.4 do not depend on the machine. A supported compiler target must establish that its logical limits are enforced before stack/heap exhaustion. Until that gate passes, exhaustion is a compiler resource failure, never a catchable macro answer or a cacheable SyntaxError. For this, the units are also counted inside the functions of the runtime, and the count of depth follows frames that are left by an error, a continuation, or a tail call (§12).
- `hash` (spec §13) gives fixed numbers in a macro program.
- Each run starts from the same state: the compiler uses a new instance, or does something that a macro cannot tell from a new instance. The compiler may keep the result of a run and use it again when the macro program and everything the run was given are the same, also every answer of `Definitions` and `Types`. What is kept is the result before the mark of the run is changed (§5). A result that is used again is changed with the mark of the new run, so two calls with the same arguments, as two calls of `gen.helpers()` with no arguments, still get marked names that do not clash, whether the result is kept or not.

**What is in the compiled program.** A **compile-time binding** is a top-level binding that has `@macro`, or whose full needs have a quote or a function of `i/syntax` in them.

- Compile-time bindings are not in a program selected by the normal entry configuration or host exports. It is a compile error when such a program needs one.
- Every other binding of a module with macros is in the program as for any module (spec §2.5). So in §4.2, `failed` is in a program that uses `check.that`.
- `i test` runs tests with the functions of `i/syntax`, so a test may call the normal functions that a macro is made of, give its own `Definitions` or `Types`, and compare results with `same_code`. In a test, a quote gives the scope of its module with no marks.

**What the sandbox is for.** A macro of a package that the program depends on runs when the program is compiled, with the limits above. That keeps the machine that compiles safe. It does not make the expanded code safe: the code that a macro gives runs in the program with the services of the program, it may use any name that can be seen at the call, and it may give its own services to the caller's code (§5). The limits on attributes in §4.3 keep it from adding foreign authority: such an attribute, with its arguments and the item it is on, must be one that the caller wrote and that reached the result unchanged, at most one time, so neither a template nor `make` can give one.

## 9. Errors and tools

- An error in expanded code has the span of the code. Code from an argument has the span of the caller's source. Code from a template is reported at the macro call, as "in the expansion of `m`", with the span of the template as a note and the expanded code shown.
- Warnings are given only for code with a span in the source of the module that is compiled, so not for code from a template.
- When a program stops (spec §11.7) in code from a template, the message gives the position of the macro call and the position in the template.
- `i fmt` formats a file in its own surface, and does not expand macros. `i fmt --to=is` and `i fmt --to=i` write the file of the other surface and remove the old one (§1). The new file is the same code. Documentation comments are kept. A normal comment is kept when it is before an item or a statement, and the tool reports each comment that it cannot keep. The spelling of literals and empty lines are not kept. The layout of the Lisp surface is defined with the formatter.
- `i expand` is a new command. It writes a module after all expanding, in the surface that is asked for. The text is for reading (§5), and a module with quotes can only be shown in the Lisp surface. `i expand --trace` also writes each run: the macro, its arguments, and its result.
- `i check` and the language server must run macros, so they compile and run macro programs. They keep the results of runs (§8). While a macro module has errors, its macro calls are reported as not expanded and the rest of the module is checked without them.
- The language server shows the expansion of a macro call, goes from a name in expanded code to its definition by its span, and marks macro calls with a semantic token, so that an editor may show them in another colour.
- In a typed macro call, the language server gives types and completion in the arguments without running the macro, because they are checked by the signature.

## 10. Not in this design

- New tokens, new brackets, or macros that change the parser. A macro only gives a new meaning to forms that can already be read.
- Macro calls that give a statement or a pattern. A clause block as an argument does the work of a pattern: `each(list, { x -> … })`.
- Macros that change a type definition, and macros that define macros.
- Macros that decide the type of their call (the "whitebox" macros of Scala), and computing inside types.
- Choosing an implementation by type inside a binding with type variables. That needs a feature of the type system, not a macro.
- Reading files when compiling.
- A way for a macro to ask what a name is bound to.
- Mixing the two surfaces in one file, and writing a template in the normal surface inside a `.is` file. The second may be added later with no change to the core tree.

