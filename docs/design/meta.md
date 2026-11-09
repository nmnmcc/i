# Design: metaprogramming with two surfaces

Status: draft, after one round of review by four independent readers. Every decision in it is confirmed; the open questions are in §11. Nothing here is in `docs/spec.md` or in the compiler yet. This design is written on top of the syntax with no keywords (`docs/design/syntax.md`, streams A0 to A2), and every example uses that syntax. It can only be built after stream A1. §12 says in one line what each stream named here is.

## Summary

- i is one language with one **core tree** and two ways to write it. The **normal surface** is the syntax of the spec, in files that end in `.i`. The **Lisp surface** writes the core tree with brackets, in files that end in `.is`.
- A file uses one surface. A module written in one surface is used from the other with no difference.
- A **macro** is an i function that runs in the compiler. It takes code as data and gives code as data. Macros are defined only in `.is` files. They are used from both surfaces: as a call `m(…)`, after `|>`, or before an item as `@m(…)`.
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
6. `a |> m` is the macro call `m(a)`, and `a |> m(b)` is the macro call `m(a, b)`, as for every call (§4.3).
7. No macro call gives a statement or a pattern (§4.3, §10).
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

- A source file ends in `.i` (normal surface) or `.is` (Lisp surface).
- A module path has no end of a file name in it. `my/control` is the module in `my/control.i` or in `my/control.is`. When both files are there, it is a compile error.
- Importing, exporting, the spaces of names, and every other rule of modules are the same for both surfaces. A module does not know which surface another module is written in.
- A `.is` file is a complete module. It may have type definitions, bindings, and declarations, with or without macros. It may be under `test/` and have `@test` functions.

The **core tree** is what the parser gives for a file: the imports, the exports, and the items, and everything in them. Documentation comments are part of the item or the module they are on. The core tree has the value of a literal, not its spelling: `0x10` and `16` are the same tree. A group `{ e }` around one expression is a block with one expression, and it is in the core tree.

Two core trees are **the same code** when they are different only in groups: `{ e }` and `e`, and `{ T }` and `T` in a type. The result `_` of a function expression is never in a group: `{ _ }` there is a compile error (§2.3), so a group cannot change whether a function has a `->` part.

Each surface has a parser and a printer. The printer of the normal surface adds the groups that its rules of binding need, and the line breaks that its rule of lines needs (syntax.md §1.7); the Lisp surface never needs either. With no line break, syntax.md §1.7 reads a statement, a clause, or an item as the longest expression it can, so one that begins with `(`, `{`, `-`, or `-.` would join the one before it: `{ (= y g) (= (, a b) (f y)) a }` on one line is `{ y = g  (a, b) = f(y)  a }`, which reads `g(a, b)`. So the printer puts a line break before every statement, clause, or item that begins with one of these tokens, and between an attribute and an item that begins with `(`. It also puts a guard in a group when the guard has a `->` that is not inside brackets, because syntax.md §6.1 ends the clause head at that `->`. So:

- Every `.i` file may be changed into a `.is` file that is the same code.
- A `.is` file may be changed into a `.i` file that is the same code, but not when it has a quote (§3.3) or an item with `@macro`, because the normal surface cannot write them. Then `i fmt --to=i` reports an error and writes nothing. §2.3 says which other datums the Lisp parser does not take, so that every other file can be changed.

This promise needs five changes to `docs/design/syntax.md`, listed in §12. Until they are made, these `.is` files cannot be changed into `.i` files either, and `i fmt --to=i` and `show_as` report an error for them:

- A statement or an item that is a macro call whose first argument is an `UpName`, as `(decode User text)`, `(log Info)`, or the item `(derive User)`. Syntax.md §6.1 and §2.2 read `name (` and an `UpName` as a binding with type variables, and a group cannot be put around a statement or an item.
- A declaration with an attribute other than `@external`, as `(@ rpc (: fetch (-> (Id) (! User Http))))`, and a macro call as an item. Syntax.md §2.2 reads the first as a binding, and has no item of the second form.
- An attribute with an argument that is not a string, as `(@ (m x) Item)` or `(@ (m (: path "/users")) Item)`. Syntax.md §2.2 takes only strings as the arguments of an attribute.
- A clause block whose first clause head is a type test, as `{ (-> (: n Int) "int") (-> _ "other") }`. Syntax.md §6.1 keeps `{ n: Int …` for records, and a pattern has no group.
- A block whose first statement is a function with a `->` part, as `(-> (a) _ (-> (b) Int (+ a b)))`, which would be `(a) { (b) -> Int { a + b } }`. Syntax.md §6.1 reads `{ (b) -> …` as a clause block. A group is also a `{ }`, so no bracket helps.

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

In the tables, `e` is an expression, `arg` a call argument (`e`, or `(: x e)` with a label, as in the row Call), `p` a pattern, `T` a type, `N` a named type (`U`, `(. a U)`, `(U T…)`, or `((. a U) T…)`), `x` and `a` a name, `C` and `U` an `UpName`, and `…` zero or more of the thing before it.

**Files and items**

| Core form | Normal surface | Lisp surface |
|---|---|---|
| Imports | `(a/b/c, x = a/b, (C, y) = a/b, x = (C, y) = a/b)` | `(, (/ a b c) (= x (/ a b)) (= (, C y) (/ a b)) (= x (, C y) (/ a b)))` |
| Exports | `(x, U, U(..))` | `(, x U (U ..))` |
| Built-in attribute | `@main Item`, `@host("ns", "async") Item` | `(@ main Item)`, `(@ (host "ns" "async") Item)` |
| Macro before an item | `@m Item`, `@a.m Item`, `@m(arg…) Item`, `@a.m(arg…) Item` | `(@ m Item)`, `(@ (. a m) Item)`, `(@ (m arg…) Item)`, `(@ ((. a m) arg…) Item)` |
| Custom type | `U { C1(T, x: T) C2 }` | `(U { (C1 T (: x T)) C2 })` |
| Custom type with parameters | `U(A, B) { … }` | `((U A B) { … })` |
| Type alias | `U = T`, `U(A) = T` | `(= U T)`, `(= (U A) T)` |
| Binding | `p = e` | `(= p e)` |
| Binding with type variables | `x(A, B) = e` | `(= (x A B) e)` |
| Declaration | `x: T`, `x(A): T` | `(: x T)`, `(: (x A) T)` |
| Macro call as an item | `m(arg…)`, `a.m(arg…)` | `(m arg…)`, `((. a m) arg…)` |
| Items from an expansion | none | `{ Item… }` |

- At the top level of a file, a `(, …)` is the imports when it has a `(/ …)` in it, and the exports when not. It may not be empty.
- A module path `(/ …)` has two or more names. An import item `(= …)` has two parts, or three when it has both a name and a list of names. Every other `(= …)` has exactly two parts.
- An item `(U { … })` or `((U A …) { … })` is a custom type. An item `(= U T)` or `(= (U A …) T)` is a type alias. A top-level binding may not have a pattern that starts with an `UpName`, as in syntax.md §2.2. An item that is a `Round` whose head is a name or a `(. a m)` is a macro call. A type alias whose left side is a type pattern (stream T2, as `Schema(User, _) = user`) is written as the same datum, `(= (Schema User _) user)`, and the `UpName`s in the pattern are the names it defines; the exact rule comes with T2 (open question 2).
- **Items from an expansion.** A `Curly` of zero or more items is only taken as the result of a macro in the position of an item (§4.3). It is never taken in a source file, and it has no form in the normal surface: `show_as` and `i expand` write its items one after the other, as the items of a file.
- Each `(@ …)` has one attribute. More attributes are put one inside the other, the first on the outside: `@main @deprecated("x") f = e` is `(@ main (@ (deprecated "x") (= f e)))`.
- `@m Item` gives the macro only the item. `@m()` is not written; `(@ (m) Item)` is a compile error.

**Types**

| Core form | Normal surface | Lisp surface |
|---|---|---|
| Union, intersection, difference | `A \| B \| C`, `A & B`, `A - B - C` | `(\| A B C)`, `(& A B)`, `(- A B C)` |
| Function type | `(T1, x: T2) -> R` | `(-> (T1 (: x T2)) R)` |
| With errors and environment | `(T) -> R ? E ! N` | `(-> (T) (! (? R E) N))` |
| With errors only, environment only | `(T) -> R ? E`, `(T) -> R ! N` | `(-> (T) (? R E))`, `(-> (T) (! R N))` |
| Tuple type | `(T1, T2)`, `()` | `(, T1 T2)`, `(,)` |
| Named type | `U`, `a.U`, `U(T1, T2)`, `a.U(T)` | `U`, `(. a U)`, `(U T1 T2)`, `((. a U) T)` |
| Hole | `_` | `_` |
| Group | `{ T }` | `{ T }` |
| Macro call | `m(T, 3)`, `a.m(T)` | `(m T 3)`, `((. a m) T)` |

- `|`, `&`, and `-` take two or more parts in a type. A part may be a `|`, `&`, or `-` form; that is the type with a group there, `A | { B | C }`.
- A function result is `R`, `(? R E)`, `(! R N)`, or `(! (? R E) N)`. No other use of `?` and `!` is correct there.
- A named type with arguments has one or more arguments.

**Expressions and statements**

| Core form | Normal surface | Lisp surface |
|---|---|---|
| Literal | `1`, `1.5`, `"a"` | `1`, `1.5`, `"a"` |
| Name, member of a module | `x`, `C`, `a.x` | `x`, `C`, `(. a x)` |
| Call | `f(e1, x: e2)`, `f()` | `(f e1 (: x e2))`, `(f)` |
| Binary operator | `e1 + e2` | `(+ e1 e2)` |
| Unary operator | `-e`, `-.e` | `(- e)`, `(-. e)` |
| Field, tuple part | `e.x`, `e.0` | `(. e x)`, `(. e 0)` |
| Tuple | `(e1, e2)`, `()` | `(, e1 e2)`, `(,)` |
| List | `[e1, e2]`, `[e1, ..e2]` | `[e1 e2]`, `[e1 .. e2]` |
| Bit array | `<<e1, e2: f, e3: f(e4)>>` | `<<e1 (: e2 f) (: e3 (f e4))>>` |
| Block, group | `{ s1  s2  e }`, `{ e }` | `{ s1 s2 e }`, `{ e }` |
| Binding statement | `p = e`, `x(A) = e` | `(= p e)`, `(= (x A) e)` |
| Environment statement | `! N = e` | `(! N e)` |
| Clause block | `{ p1 -> e1  p2 -> e2 }`, `{}` | `{ (-> p1 e1) (-> p2 e2) }`, `{}` |
| Clause with choices | `p1 \| p2 \| p3 -> e` | `(-> (\| p1 p2 p3) e)` |
| Clause with a guard | `p ? g -> e`, `p1 \| p2 ? g -> e` | `(-> (? p g) e)`, `(-> (? (\| p1 p2) g) e)` |
| Function with no `->` part | `(p1, p2) { e }` | `(-> (p1 p2) _ e)` |
| Function with a `->` part | `(p) -> R ? E ! N { e }` | `(-> (p) (! (? R E) N) e)` |
| Environment parameter | `x ! N` | `(! x N)` |
| Record update | `C(..e, x: e2)`, `a.C(..e)` | `(C .. e (: x e2))`, `((. a C) .. e)` |
| Quote | none | `` `d `` |

- A `Round` with one or more parts whose head is not a symbol is a call, but when its second part is `..` it is a record update. The head is the function. A binary operator takes exactly two parts; `-` and `-.` may take one.
- **Functions and clauses.** In the position of an expression, a `(-> …)` has three parts: the parameters, the result, and the body. The body is one expression. The result `_` means that the function has no `->` part, so its value, errors, and environment are all inferred (spec §6.8). Any other result is a `->` part, in which a missing `?` or `!` means `Never`; the normal `(x) -> _ { e }` is `(-> (x) (? _ Never) e)`. The result `_` is only the bare atom: a `{ _ }` there is a compile error, in both surfaces. A clause is a `(-> …)` with two parts, and it is only in a clause block. A function type also has two parts, and it is only in the position of a type. A macro that looks at a `(-> a b)` must know its position to know which of the two it is.
- **Blocks.** A `{ }` in the position of an expression is a clause block when it is empty, or when its first part is a `(-> …)` with two parts. Then every part must be a clause. When not, it is a block, and it has one or more parts.
- A clause block with one clause, `{ (-> p e) }`, and the function `(-> (p) _ e)` are two core trees with one meaning (`docs/design/syntax.md` §6.12).
- A clause head is `p`, `(| p1 p2 …)` with two or more choices, or `(? head g)` with a head of the first two kinds. `(| …)` is a compile error in every other position of a pattern. A choice that **ends in a type test** may only be the last choice, because the normal surface reads `x: T | p` as a union type, and a pattern has no group to stop it. A pattern ends in a type test when it is `(: x T)`, or `(= x p)` where `p` ends in a type test: `(| (= a (: b Int)) Unit)` would be `a = b: Int | Unit`, which is the one pattern `a = b: { Int | Unit }`. A clause block whose first clause head is a type test cannot yet be written in the normal surface (§1).
- `(. a x)` is a member of a module or a field, as `a.x` is in the normal surface (spec §2.3).

**Patterns**

| Core form | Normal surface | Lisp surface |
|---|---|---|
| Name, discard | `x`, `_`, `__rest` | `x`, `_`, `__rest` |
| Type test or annotation | `x: T`, `_: T` | `(: x T)`, `(: _ T)` |
| Literal | `1`, `-1`, `-.1.5`, `"a"` | `1`, `(- 1)`, `(-. 1.5)`, `"a"` |
| String prefix | `"a" <> x` | `(<> "a" x)` |
| Constructor | `C`, `a.C`, `C()`, `C(p1, p2)`, `a.C(p)`, `C(p, ..)`, `C(..)` | `C`, `(. a C)`, `(C)`, `(C p1 p2)`, `((. a C) p)`, `(C p ..)`, `(C ..)` |
| Tuple | `(p1, p2)`, `()` | `(, p1 p2)`, `(,)` |
| List | `[p1, p2]`, `[p, ..]`, `[p, ..x]`, `[..]`, `[..x]` | `[p1 p2]`, `[p ..]`, `[p .. x]`, `[..]`, `[.. x]` |
| Bit array | `<<p1, p2: f, p3: f(e)>>` | `<<p1 (: p2 f) (: p3 (f e))>>` |
| Two patterns for one value | `x = p` | `(= x p)` |
| Quote pattern (§3.3) | none | `` `d `` |

- In `(: x T)`, in `(<> "a" x)`, and in `[p .. x]` and `[.. x]`, `x` is a name or a discard, as in spec §7.1. In `(= x p)`, `x` is a name. In `(- k)` and `(-. k)`, `k` is a literal of the kind of the operator.
- The statement `a = (x, y) = e` is `(= (= a (, x y)) e)`.

**Datums that are compile errors.** These fit no row: `()` in every position but a list of parameters; a named type or a type head with no arguments in brackets, `(U)` or `((U) { … })`; a constructor of a type definition with no fields in brackets, `(C1)`; a binding head with no type variables, `(= (x) e)`; an empty list of import names, `(= (,) (/ a b))`; a list that begins with `..` in the position of an expression, `[.. e]`; a part of a record update with no label, `(C .. e 1)`.

### 2.4 An example

```i
fact = (n) {
  n |> {
    0 -> 1
    _ -> n * fact(n - 1)
  }
}
```

```lisp
(= fact (-> (n) _
  (|> n {
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
}

Position { Expression  Type  Pattern  Item }

SyntaxError { SyntaxError(message: String, at: Syntax) }
```

- A `Syntax` value is one datum of §2.2 with where it came from: its span in a source file, and its **scope**, which says where its names are looked up (§5). `Syntax` has no constructors. Only the compiler, a quote, and `make` give `Syntax` values.
- `node(s)` gives the `Node` of a `Syntax`. `Name`, `Discard`, `Up`, and `Symbol` have the text of the token. `Int`, `Float`, and `Text` have the value of the literal.
- **The `Syntax` of a piece of code is the datum that the Lisp surface writes for it** (§2.3). This is true for code from a `.i` file too. So a macro always sees the Lisp form: the node of the argument `n > 0` is a `Round` of three parts whose nodes are `Symbol(">")`, `Name("n")`, and `Int(0)`. A group that the caller wrote, `{ e }`, is a `Curly` with one part.
- `Syntax` has no documentation comments in it.
- `==` on two `Syntax` values is `True` only when they are the same value. To compare code, use `same_code`.

### 3.2 Functions of `i/syntax`

| Function | Type | What it gives |
|---|---|---|
| `node` | `(Syntax) -> Node` | The node of the value |
| `make` | `(node: Node, like: Syntax) -> Syntax` | A new `Syntax` with this node, and the span and the scope of `like`. §5 says how a name made in this way is looked up. |
| `same_code` | `(Syntax, Syntax) -> Bool` | `True` when the two are the same datum. Spans and scopes are not looked at. |
| `show` | `(Syntax) -> String` | The datum in the Lisp surface, on one line |
| `show_as` | `(Syntax, Position) -> String ? SyntaxError` | The code in the normal surface, read by §2.3 for this position. It is on one line, but for the line breaks that the printer must put in (§1); a `Curly` of items gives one item on each line. A `SyntaxError` when it fits no row, or when the normal surface cannot write it (§1). |
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

Two things to know about `{ }` in a template: `` `{ ~..parts} `` with an empty list is `{}`, the clause block with no clauses; and with one part it is a group.

## 4. Syntax macros

### 4.1 Defining

A **syntax macro** is a top-level binding in a `.is` file with the attribute `@macro`, and with no declaration of the same name (§7.1):

```lisp
// my/control.is
(, unless)

(@ macro
  (= unless (-> (cond body) _
    `(|> ~cond { (-> True Nil) (-> False ~body) }))))
```

- The pattern of the binding is one name, with no type variables. The value is a function expression `(-> …)`. Each parameter is a name or an environment parameter.
- The value is checked against the type `(Syntax, …) -> Syntax ? SyntaxError ! Definitions | Assert`, with one `Syntax` for each parameter. If it does not have a subtype of this type, it is a compile error.
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
    (|> claim {
      (-> `(== ~left ~right)
          `{ (= l ~left)
             (= r ~right)
             (|> (== l r) {
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
| Expression | `m(args)`, `e \|> m`, `e \|> m(args)` | Expressions, with or without labels | An expression |
| Item at the top level | `m(args)`, `a.m(args)` | Expressions, with or without labels | An item, or a `Curly` of zero or more items |
| Before an item | `@m Item`, `@m(args) Item` | Expressions, with or without labels, and then the item as the last argument | See below |
| Type | `m(args)` | Types, integer literals, and strings, with no labels | A type |

- The parser does not know which names are macros. The arguments are always read by the grammar of the position, as in the table, so a file can be read and formatted with no other file. From this, an argument in the position of an expression cannot be a type such as `Int | Nil` or `(Int) -> String`, a discard, or a list pattern with `..`.
- In the normal surface, a macro call as an item, a declaration with a macro before it, a macro before an item with an argument that is not a string, and a call whose first argument is an `UpName` at the start of a statement or an item, as `decode(User, text)`, need the changes to syntax.md listed in §12. Until then they can only be written in a `.is` file (§1).
- `e |> m` is the macro call `m(e)`, and `e |> m(args)` is the macro call `m(e, args)`, as `|>` works for every call (spec §6.9). This is only in the position of an expression.
- The number of arguments must be the number of parameters of the macro. An argument with a label goes to the parameter with that label, by the rule of calls (spec §6.6), and the macro gets it with no label.
- The arguments are not worked out. The macro gets their `Syntax`, and its result says if and when each of them is worked out.
- A name in a bit array format, as `size` in `<<x: size(8)>>`, is never a macro call.
- No macro call gives a statement: the result of a call in the position of an expression is an expression, also when the call is a statement of a block. So an expansion cannot add a name to the block around it, and which names are local at a call does not depend on other expansions.

**A macro before an item.**

- Before a **type definition**, the macro cannot change the item. The type definition stays in the module as it is written, and the result of the macro, an item or a `Curly` of zero or more items, is added after it. So a type definition that is written is always the definition of its type.
- Before a **binding or a declaration**, the result, an item or a `Curly` of zero or more items, takes the place of the item. To keep the item, the macro puts it in its result.
- The item is given to the macro with the attributes that are written after the macro. The item always goes to the last parameter, also when another argument has a label.
- A built-in attribute before a macro, as in `@test @m f = …`, is a compile error; write it after the macro, so that it is a part of the item. An attribute whose name is one of the built-in attributes (`external`, `host`, `main`, `test`, `deprecated`, `macro`) is that attribute; a macro with such a name is used with its module name.
- With more than one macro before an item, the first is expanded first, and it gets the item with the other macros still on it.
- The rule of spec §2.2 that an attribute is not given two times is for built-in attributes only.
- The documentation comment of the item stays on the type definition, or goes to the first item of the result that defines the same name.

**Limits on what a result may have.**

- Imports and exports cannot be in a result.
- A `Quote`, `Unquote`, or `Splice` node cannot be in a result.
- The attributes `@external`, `@host`, `@main`, and `@test` may be in a result only when the whole `(@ …)` datum, with the name atom, the arguments of the attribute, and the item it is on, came from a source file of the caller and reached the result through the arguments of each run, unchanged. Each such datum may be in the result at most one time. Having no marks is not enough: a name made with `make` (§5) is never such an atom, also when it has no marks after the run. So a macro of a package that the program depends on cannot add a host import or an entry point that the caller did not write: it cannot reuse the caller's `external` atom with other arguments, put the caller's `@main` or `@host(…)` on an item of its own, or give one declaration two times.
- A `Curly` directly inside the `Curly` of a result of items is a compile error.

### 4.4 Expanding

The compiler expands a macro call by calling the macro with the `Syntax` of the arguments. One call of the macro is one **run**.

- Expanding goes from the outside to the inside. An argument is given to the macro as it is written, with the macro calls in it not expanded. After a call is expanded, the macro calls in its result are expanded in the same way.
- The result is read by the tables of §2.3 for the position of the call. If it fits no row, it is a compile error at the call, and the message names the macro.

A module is expanded in two steps:

1. **Items.** The compiler expands the macro calls in the position of an item, the macros before items, and the macro calls in the position of a type that are inside type definitions. It goes on until none is left. A run may be put off in this step (§6). After this step, the top-level names of the module are all known and every type definition is complete. Then the exports are checked: every exported name must be defined.
2. **Bodies.** The compiler expands the macro calls in the position of an expression, and the other macro calls in the position of a type. Their order has no effect on the result, because they cannot add a top-level name or a local name outside their own result.

Typed macro calls are expanded later (§7.3).

**Limits.** The spec gives fixed limits for one run: the work it may do, the memory it may use, and how deep its calls may go. It also gives a limit for how deep expansions may be inside the results of other expansions. The limits are counted in units of the language, such as calls and the size of values, so they do not depend on the machine or on how the compiler optimizes (open question 3). Going over a limit is a compile error that names the macro and the limit.

**Errors.**

- A macro that gives a `SyntaxError` makes a compile error with that message at the span of `at`. When that span is in a template, the error is reported at the call, with the template as a note.
- A macro that stops (spec §11.7), also by a failure reported to `Assert`, makes a compile error at the call.

## 5. Names in expanded code

Every `Syntax` value, an atom or a bracket, has a scope. A scope is a module and a set of **marks**. The scope of an atom is used to look up its name. The scope of a bracket is only used by `make`.

- Code read from a file of the module `M` has the scope `M` with no marks.
- Code in a template written in the module `D` has the scope `D` with no marks.
- `make(node, like)` gives the scope of `like`. A name made in this way is a **made name**.
- For each run, the compiler makes a new mark `k`. It puts `k` on every `Syntax` of the arguments, and on every `Syntax` that `Definitions` or `Types` give in the run (§6, §7.4). It runs the macro, and then changes `k` on every `Syntax` of the result: one that has `k` loses it, and one that does not have `k` gets it. So the code that came from the arguments or from the compiler is as it was, and the code that the macro made has `k`. A run that is done again after it was put off (§6) is a new run with a new mark.
- A type variable in an answer of `Types` has a scope of its own kind, which names that type variable of the checker (§7.4). Rules 1 to 3 below find that variable directly, and no binding, of a template or of the caller, can bind it.

A name is looked up in this way:

1. A name is bound by the nearest binding around it (a parameter, a pattern, a binding statement, a declared type variable) that has the same text **and the same scope**: the same module and the same set of marks.
2. If there is none, and the name has no marks: it is a top-level name or an imported name of its module, by the normal rules, and then a built-in name.
3. If there is none, and the name has marks: it is a top-level name that an expansion defined with the same text and the same scope in the module where the call is (below). If there is none, it is a top-level name, an imported name, or a module name of **the module of its scope**, and then a built-in name. A top-level name found in this way does not have to be exported, but for a made name it must be exported.

Labels of fields, of parameters, and of arguments are not names in this sense. They are compared by their text only.

From this:

- A name that a template binds cannot take a name of the caller, and a name that the caller binds cannot take a name of a template. In §4.2, `l` and `r` have a mark, so they are not the caller's `l` and `r`.
- A free name of a template means what it means in the module of the template. In §4.2, `failed`, `assert`, and `inspect` are the names of `my/check`, whatever the caller has. Two templates of different modules that are used in one run do not take the names of one another, because their scopes have different modules.
- To make a name that the caller can see, a macro uses a name from its arguments, or `make(Name("it"), like: arg)`. A macro with no arguments cannot make such a name.
- Only a template can reach a name of its module that is not exported. `make` cannot, so a module cannot reach the private names of another module by using a `Syntax` that came from it.
- Services are found by their type (spec §9.2), and errors go to the nearest handler, not by name. So marks do not keep them apart: **services and errors are not hygienic.** A result that puts code from an argument after an environment statement, as `` `{ (! Assert quiet_assert) ~body } ``, or inside a function that handles errors, as `fail.catch`, changes which service or which handler that code uses. This is the same as for a normal function that takes a function and calls it, and the checks of §7.3 do not stop it. `i expand` (§9) shows it.
- Because labels are text, a function in a template with the parameter `i`, used with a parameter `i` from the caller in the same list, has two parameters with one label, which is a compile error (spec §6.11). The message names the macro. The author of the macro uses another name.

**Top-level names.** When an expansion defines a top-level name (a binding, a type, a constructor) in the module `U`:

- A defined name with no marks and the scope of `U` is a normal top-level name of `U`. It may be used and exported by its text. So the names that the caller is to use come from the arguments of the macro.
- A defined name with no marks and the scope of another module is a compile error. Only an answer of `Definitions` or `Types` can give such a name, as a field name from `definition` (§6). Rules 1 and 2 would not find it from the caller's code with the same text, and a use with its own scope would go to the other module, so such a name has no clear meaning. To define a name from the text of a field, a macro makes it with `make(Name(text), like: arg)`, where `arg` came from the caller.
- A defined name with marks may only be used by names with the same text and the same scope, so only by code of the same run. It cannot be exported, also not as a constructor through `U(..)`.
- A defined name with marks may not have the text of a top-level name, an imported name, or a module name of the module of its scope. If it has, it is a compile error. So rule 3 never has two answers.
- The type of an exported item may not use a type that its own module does not export, also when that module is another module. Spec §2.4 has this rule only for types of the same module; this is a new rule added to §2.4, because a template can bring in a type of another module.
- The rules that a name is defined one time only (spec §2.2) and that a type parameter or a declared type variable may not have the name of a type that may be used in the module (spec §4.3, §4.7) compare the text and the scope (module and marks), not the text alone. So a type variable `Key` that a template binds does not clash with a type `Key` of the caller, and a marked `helper` does not clash with the caller's `helper`.

**What the compiler does with this.** After expanding, each name with marks is changed to a name that cannot be written in a source file, and each name found in another module by rule 3 becomes a direct reference to that top-level name. In the text form of a value (spec §11.8), a constructor with marks is shown by its text only. `i expand` (§9) shows these names in a form for reading: a name with marks as `l#1`, with the marks numbered in the order they come in the output, and a direct reference as `my/check.failed`. That text is not source text.

## 6. Asking for definitions

```i
Definitions { Definitions(definition: (of: Syntax) -> Syntax ? SyntaxError) }

definition = (of: Syntax, d ! Definitions) { d.definition(of) }
// type: (of: Syntax) -> Syntax ? SyntaxError ! Definitions
```

`Definitions` is a service type exported by `i/syntax`. The compiler gives the service when it runs a macro. A test may give its own with an environment statement.

- `of` is a type name: an `UpName`, or `(. a U)`. Any other datum is a `SyntaxError`. It is looked up with the marks it would have in the result of this run, in two steps:
  1. First by rule 1 of §5, among the type variables around it (type parameters and declared type variables): around the place where it is written when it came from an argument or from an answer of `definition`, and around the call when not. If one binds it, it is a `SyntaxError`, also when a top-level type has the same text.
  2. Then among the type names at the top level, by rules 2 and 3 of §5. So a name from an argument is looked up where the caller wrote it.
- The value is the `Syntax` of the item that defines the type: a custom type or a type alias, with its attributes. An alias is given as it is written, not the type it stands for. A type with parameters is given with its parameters, not filled in. The `Syntax` has the scopes that the definition has in its module after expanding. As an argument does, it has the mark of the run only while the run lasts (§5), so its names mean the same when a macro puts them in its result.
- A type whose definition depends on the type of a value (stream T2, as `User` in `Schema(User, _) = user`) has no written fields or constructors. Asking for it is a `SyntaxError`, so a macro cannot read such a type. Its type is only known after type checking, which a syntax macro may not wait for.
- What may be seen is what the module of the scope of `of` may see. A type of another module that is not exported is not defined. A type of another module whose constructors are not exported is a `SyntaxError`.
- Built-in types have no written definition, and asking for one is a `SyntaxError`. But `Bool`, `Nil`, `Result`, `Assert`, and `AssertionFailure` give the definitions that the spec gives for them.
- A name that step 2 does not find is a compile error at the span of `of`. The macro cannot catch it. A name bound by a type variable never gets to step 2, so it is always a `SyntaxError`.
- Only type definitions can be asked for. The definition of a binding cannot, because then the result of a macro would depend on the bodies of functions of other modules.

**When a definition is ready.** A type definition of another module is always ready, because that module is expanded first (§8). A type definition of the module that is being expanded is **ready** when it is in the module and no macro call is left in it. A type definition under a macro before it is in the module from the start (§4.3).

In step 1 of §4.4, when a run asks for a name of the module that is being expanded:

- If the name is defined by a ready definition, `definition` gives it.
- If not, and another run of step 1 is still to be done, the run that asked is **put off**: it is dropped, and it is done again from the start, as a new run, after another run has ended. This is also done when the name is a built-in name or is not defined, because an expansion may still define it.
- When every run that is left has been put off and none can go on, each of them is a compile error: the message gives the name that each one waits for.
- A name is defined one time only (spec §2.2), a type definition that is written is never changed (§4.3), and a name that is not ready is never answered while it may still be defined. So the result does not depend on the order in which the compiler does the runs.

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
    (|> (subtype t `Int) {
      (-> Yes `(int_text ~value))
      (-> _ (|> (subtype t `String) {
        (-> Yes value)
        (-> NotKnown (raise (SyntaxError "to_text: the type is not known here" value)))
        (-> No (raise (SyntaxError "to_text: this type has no text form" value))) })) }) })))
```

- The declaration is the **signature**. It is a full function type, with its type variables declared after the name. It may have errors and an environment. It may not have `_`. The rules for written types (spec §4.4) and for types in exported items (spec §2.4) are used for it.
- The binding is the **implementation**. It follows the rules of §4.1, with these changes: it has one parameter for each parameter of the signature, in the same order, and it is checked against `(Syntax, …) -> Syntax ? SyntaxError ! Types | Definitions | Assert`.
- This is the only case where two items have one name. The declaration has no `@external`; it is read as a declaration by its form (§12).
- The documentation comment of the macro is on the declaration.

### 7.2 Checking a call

A typed macro is used only in the position of an expression, written as in §4.3. For type checking, the call is a call of a function with the type of the signature:

- The arguments are checked as the arguments of that function. Labels, expected types (spec §4.8), and `|>` work as for a function.
- The call has the result type, the errors, and the environment of the signature, with its type variables filled in by inference as for any call. The errors and the environment of the arguments are part of the call, as for any call, also when the macro does not use an argument.
- So the types of the code around the call depend only on the signature, never on the result of the macro. Whether a binding is a direct value, and how it is generalized (spec §4.7, §6.10), are decided before any typed macro is expanded.

### 7.3 Expanding

Typed macros are expanded after the binding they are in has been type checked, with all bindings that are inferred together with it.

- By then every syntax macro call in the binding has been expanded. So the arguments may have names with marks in them.
- Expanding goes from the outside to the inside: a typed macro gets its arguments with the typed macro calls in them not expanded.
- The result is put in the position of the call. Its syntax macro calls are expanded (§4.4, step 2). Then it is type checked there, with the local names and their types as they are at the call. An argument that is in the result keeps the expected type that it had in §7.2.
- Then the typed macro calls in the result are expanded in the same way. The limits of §4.4 are used.

**What the result must be.** In this section, the **lower bounds** of a type variable are the types that come to it (spec §4.7), and its **upper bounds** are the types that it must be a subtype of. Two kinds of type variable are left when the binding group of the call has been checked:

- A type variable is **closed** when it is local to the binding group of the call, is not generalized, and cannot be reached from the type of any top-level binding by following bounds, in either direction. No other part of the program can give it more types. Before the result is checked, it is taken to be the union of its lower bounds, or `Never` when it has none.
- Every other type variable is **fixed**: a declared or generalized type variable, a variable of a top-level binding of an earlier group, and a variable that can be reached from the type of a top-level binding that is not generalized, also one of the group of the call. Later groups may still give such a variable more types (spec §4.7), so the macro may not take it to be the types it has now. In `f = compose((x) { to_text(x) }, id)`, the type variable of `x` is in the type of `f`, which is not generalized, so it is fixed, and a later `f("hello")` gives it `String`.

Let the **needed type** of the call be the function type of the signature, with each of its type variables filled in with the type that the checker gave it at this call, and each closed variable in that type replaced by the union of its lower bounds, or `Never`. A fixed variable stays in the needed type as itself. This is sound: a closed variable gets no more types, so only values of the union of its lower bounds reach the call, and the checker has shown that each of them is a subtype of every upper bound of the variable; and a fixed variable is checked by the rigid rules below, which hold for every type that it may stand for later. In `make(B, A): (x: B) -> (A) -> String` and `w = () { make(0) }`, `A` is a generalized variable of `w`, so the needed type is `(x: Int) -> (A) -> String` with `A` fixed, and the result `(-> (y) _ (int_text y))` is not correct. The needed type can always be written as a type, with each fixed variable written as in §7.4.

- The type of the result must be a subtype of the result type of the needed type.
- The errors of the result must be among the errors of the signature and the errors of the arguments. The environment of the result must be in the environment of the signature and of the arguments, by the rule for environments (spec §4.5 rule 4).
- A fixed variable `a` is **rigid**: checking the result never adds a bound to it. Type variables that checking the result makes take bounds as in normal checking. A constraint `a <: T` holds only when:
  1. `a` is `T`, or a part of the union `T`; or
  2. an upper bound of `a`, followed through the upper bounds of other fixed variables, is a subtype of `T` with no type variable given a bound; or
  3. `T` is a union with a part `r` or `r - U`, where `r` is a type variable that checking the result made, and then `a` is added to the lower bounds of `r` as in normal checking.

  A constraint `T <: a` holds only when:
  1. `T` is `a`, or an intersection with a part `a`; or
  2. `T` is a subtype of a lower bound of `a`, followed through the lower bounds of other fixed variables, with no type variable given a bound; or
  3. `T` is an intersection with a part `r` or `r - U`, where `r` is a type variable that checking the result made, and then `a` is added to the upper bounds of `r`, or `a | U` for a part `r - U`, as in normal checking. So `{ r - Int } & String <: a` holds by this case, and gives `r` the upper bound `a | Int`.
- So a clause block on an argument with type tests and a last clause `_` is correct: the catch-all part goes to a new variable by (3). A use of the argument as a type that the bounds do not show, such as `(+ ~value 1)` when no upper bound of `a` is a subtype of `Int`, is not.
- In the result, a top-level name of the module of the call that has no marks must be one of these, so that the names that may be used do not depend on the order in which the checker takes groups that do not use one another:
  - a binding or a constructor that the binding already used before the expansion, or one of a group that the group of the call depends on (spec §4.7);
  - a type, or a constructor of a type, whose definition does not depend on the type of a value (stream T2). Such a type is known before any group is checked, so it may always be used;
  - a type whose definition depends on the type of a value, as `Cfg` in `Cfg = config`, only when the binding already used it, or when the group of the call depends on the group of that value.

  A name of a binding that is inferred together with the binding of the call has the type it has at the call, not a general type.
- If one of these is not true, it is a compile error at the call. The message says that the macro is wrong, not the caller.

After all expanding, the order of top-level values (spec §2.5) and all later steps use the expanded program.

**Messages.** The errors and the warnings for the code of an argument are given one time, from the check of the expanded program. An error in code from a template is reported at the call, with the template as a note. A warning in code from a template is not given.

A binding with type variables is expanded one time, not one time for each use. Inside it, the type of an argument may be a type variable, and the macro cannot know more (§7.4, `NotKnown`).

### 7.4 Asking for types

```i
Truth { Yes  No  NotKnown }

Types {
  Types(
    type_of: (Syntax) -> Syntax ? SyntaxError,
    signature: () -> Syntax,
    subtype: (Syntax, Syntax) -> Truth ? SyntaxError,
    members: (Syntax) -> List(Syntax) ? SyntaxError,
  )
}
```

`Types` is a service type exported by `i/syntax`, with one function of `i/syntax` for each field, as `definition` is for `Definitions`. The compiler gives the service when it runs a typed macro.

A type is given to a macro, and taken from a macro, as the `Syntax` of a written type (§2.3). In a type that the compiler gives:

- The scopes are set so that every name means the same type in every position where the macro may put it. These `Syntax` values have the mark of the run only while the run lasts (§5).
- The type given is the type after the simplification of spec §4.7, with each closed variable replaced by the union of its lower bounds, or `Never` (§7.3). Only fixed variables are left.
- A fixed variable is given as an `Up` node, as `A`, with a scope of its own kind that names that variable (§5). That scope keeps it apart from every type name and every type variable that a template or the caller declares, also one with the same text. One variable has the same node in every answer of one run. It may be written in the result with no declaration: the rule of syntax.md §4.7 that a type variable must be declared is not used for it, and neither is the rule of spec §4.4 about where a union or an intersection with a type variable may be written. It may not be in a type test of a clause (spec §7.1).
- A type with no type variable in it is given in full, with its aliases filled in.

| Function | What it gives |
|---|---|
| `type_of(s)` | The type that the checker gave to `s` at its position in the call. `s` must be a `Syntax` that the compiler gave to this run: an argument, an expression inside an argument, or a name that a pattern inside an argument binds. For any other `Syntax`, also one made by `make` or by a quote, and for a datum that is not an expression or such a name, it is a `SyntaxError`. |
| `signature()` | The needed type of the call (§7.3), as a function type. Closed variables are filled in, and fixed variables are given as in the list above. |
| `subtype(a, b)` | `Yes` when `a` is a subtype of `b` (spec §4.5) by the rules for fixed variables in §7.3, with no variable given a bound. The types are taken in the form that `type_of` gives. `No` when it is not, and neither type has a type variable in it. `NotKnown` in every other case. A `Syntax` that is not a correct type, or has `_` in it, is a `SyntaxError`. |
| `members(t)` | The members of the type (spec §4.4), in the order of their `show` text. `Never` has no members. `A & Int` and `A - U`, with a fixed variable `A`, are one member each. `Bool` is one member. The same `SyntaxError` as for `subtype`. |

- `type_of` gives the type at the position of the argument in the call. When the macro puts the argument in another position of its result, for example inside a clause, the checker checks it again there (§7.3), and its type there may be different.
- A type is a set of values, and one type may be written in more than one way: `Int | String` and `String | Int` are the same type. So a macro asks `subtype` and `members`, and does not match the `Syntax` of a type by its form.
- `definition` (§6) gives the constructors and fields of a named type, but not of a type whose definition depends on the type of a value (stream T2).

### 7.5 What typed macros cannot do

- The result type of a call cannot depend on the result of the macro. A macro that reads a table of a database cannot give its rows a type that is not in the signature.
- A type variable of the signature that is only in its result, as in `default(A): () -> A`, has no type that comes to it, so its needed type is `Never` or a fixed variable, and in both cases only a result that never gives a value is correct. So a typed macro cannot choose by the type that the position of the call needs, as in `user: User = decode(text)`. A syntax macro that takes the name of the type, `decode(User, text)`, does this work (open question 5). In the normal surface, such a call at the start of a statement needs the change to syntax.md §6.1 in §12; until then, put it after `=` or in a `.is` file.
- A typed macro cannot put an argument that has errors or needs an environment inside a function to be called later, because the signature has no way to say that the errors of the argument go to that function.
- A typed macro cannot choose by the type inside a binding with type variables.
- A typed macro cannot make items. Syntax macros do that.

## 8. Running macros

**Order of modules.** A program has no circles of imports (spec §2.1). A module is expanded and type checked after every module it imports. A macro is imported, so its module is done before the module that uses it. This is a new rule: today all modules of a program are compiled together.

**The macro program.** To run the macros of a module `D`, the compiler compiles `D` and all modules that `D` imports as one program (spec Appendix B.1), with the macros of `D` as its entry points. This is a new kind of entry point that only the compiler uses:

- `Syntax` values go between the compiler and the macro program in a form that the compiler defines. It is not seen in i.
- The entry point gets `Definitions`, `Types` for a typed macro, and an `Assert` whose `fail` stops the run. These are services of the compiler, not host services (spec §9.7). It gets no other service, so a macro cannot read a file, use the network, or read the time.
- It is a compile error when an `@external` function that is not of the library package `i` is in the full needs (spec §2.5, here also through other modules) of a macro, or of a top-level value of the macro program that is not a direct value. The functions of the library are given by the compiler, as the runtime gives them to a program.
- The macro program is not for a target. It always runs in the compiler, whatever the program is compiled for.

**The same result everywhere.**

- The limits of §4.4 do not depend on the machine. A run does not stop for want of memory or stack before it is over a limit.
- `hash` (spec §13) gives fixed numbers in a macro program.
- Each run starts from the same state: the compiler uses a new instance, or does something that a macro cannot tell from a new instance. The compiler may keep the result of a run and use it again when the macro program and everything the run was given are the same, also every answer of `Definitions` and `Types`.

**What is in the compiled program.** A **compile-time binding** is a top-level binding that has `@macro`, or whose full needs have a quote or a function of `i/syntax` in them.

- Compile-time bindings are not in a program that is compiled for `@main` or for host exports. It is a compile error when such a program needs one.
- Every other binding of a module with macros is in the program as for any module (spec §2.5). So in §4.2, `failed` is in a program that uses `check.that`.
- `i test` runs tests with the functions of `i/syntax`, so a test may call the normal functions that a macro is made of, give its own `Definitions` or `Types`, and compare results with `same_code`. In a test, a quote gives the scope of its module with no marks.

**What the sandbox is for.** A macro of a package that the program depends on runs when the program is compiled, with the limits above. That keeps the machine that compiles safe. It does not make the expanded code safe: the code that a macro gives runs in the program with the services of the program, it may use any name that can be seen at the call, and it may give its own services to the caller's code (§5). The limits on attributes in §4.3 keep it from adding host imports and entry points: such an attribute, with its arguments and the item it is on, must be one that the caller wrote and that reached the result unchanged, at most one time, so neither a template nor `make` can give one.

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

## 11. Open questions

1. **A sign on macro calls.** A call does not show that it is a macro, so a reader cannot see that an argument may be worked out zero or many times. Rust (`m!`) and Julia (`@m`) put a sign on such calls; Elixir and Lisp do not. This draft has no sign and leaves it to the editor (§9), but no sign is not a confirmed decision: the question is open. Should a macro call have a sign in the source? If yes, the forms of §4.3 and decision 6 change with it.
2. **Other plans for the syntax.** `docs/design/syntax.md` keeps a lowercase name in a type for "the type of a value" (stream T2), and `{ name: e }` for records (stream T3). This draft uses `m(…)` and `a.m(…)` in a type for a macro call, and reads a `{ }` with one part as a group. Both must be checked together with T2 and T3, and so must the datum for a type alias with a type pattern (§2.3) and what `definition` gives for a type that depends on a value (§6).
3. **The units and the numbers of the limits.** §4.4 counts the limits in units of the language. Which units: (a) calls, the size of the values made, and the depth of calls, each with its own limit; or (b) one count of the steps of an abstract machine that the spec defines? And are the numbers (a) fixed in the spec for every program, or (b) fixed defaults that a package may raise up to a cap that the spec gives?
4. **The order of work.** Not open any more; the number is kept so that the other numbers stay. The order is in §12.
5. **Choosing by the needed type.** §7.5 does not let a typed macro choose by the type that its call needs. It could be let in for a type variable that is not generalized, by fixing the variable to the type that is needed at the time of the expansion. This needs a careful look at inference (spec §4.7).
6. **Changes to the core forms.** The tables of §2.3 are what every quote and quote pattern is written against, so a change of a core form breaks macros in packages. Should (a) the tables only change in the way the spec does, with no version, so that an old macro just breaks; or (b) a package say in its manifest which version of the tables its macros were written for, so that the compiler can refuse it with a clear message?
7. **Writing a macro.** A macro has no way to print while it runs; `i expand --trace` is the only help. Should `i/syntax` have a function, as `note(String)`, that adds a line to the compiler output, with the same result on every machine?

## 12. What changes

This is not a small addition. Today the compiler reads all modules, resolves them, orders them, and checks them as one program, and makes one Wasm module. Macros need the compiler to work module by module in the order of imports, and to compile and run macro programs in the middle of that. Typed macros need the checker to keep a type for each expression, to give types as written types, to check a piece of code in the middle of a binding, and to check without adding bounds. None of these is in `ilang-check` today.

The order of work is: after stream A1, the Lisp surface and the changing between surfaces (M1); then the driver and syntax macros with marks (M2); then `definition` (M3); then typed macros (M4). M2 has three parts done one after the other: the driver (M2a), syntax macros (M2b), and the tools (M2c).

The other streams of work named in this text are:

- **A0 to A2**: the syntax with no keywords. A0 is `docs/design/syntax.md`, A1 changes the parser and the formatter, and A2 changes every line of i code in the repository to the new syntax.
- **B**: continuations that resume one frame at a time (`emit/control.rs` and the runtime).
- **C**: the host boundary, ABI version 3 (`ilang-host`, `emit/host.rs`, the entry kinds of spec Appendix B.4 to B.6).
- **R2**: the library module `i/task`, with the default scheduler at the entry point (`emit/host.rs` after C, and the entry code in `infer.rs` and `lower.rs`).
- **IO**: the library modules for clocks, files, the network, processes, and standard input and output (`ilang-host` services).
- **T1 to T3**: types. T1 gives literal types and value sets, T2 the type of a value in a type (`Schema(User, _) = user`), and T3 records. They own the tables of the checker in `types/` and `infer.rs`.

What changes, part by part:

- **`docs/design/syntax.md`.** This design needs five changes there, so that every `.is` file with no quote and no `@macro` can be written in the normal surface (§1):
  - §6.1 and §2.2: a statement or an item is a binding with type variables only when `name (` is followed by a list of `UpName`s only and then `) =` (or `) :` for a declaration). It is not decided by the first two tokens.
  - §2.2: a declaration is an item of the form `name [TypeParams] ":" Type` with no `=`, whatever its attributes. A top-level `m(args)` or `a.m(args)` with no `=` after it is a macro call as an item.
  - §6.1: a `{` followed by `name :` is a clause block, and not kept for records, when `->`, `|`, or `?` comes after the type.
  - §6.1: a `{` whose first thing is a list in `( )`, then `->`, then a type (§3, with the rule of §1.7 for a `{` in the `->` part), and then a `{` on the same line that is not inside brackets, is a block whose first statement is that function, not a clause block. A clause whose expression would be read in this way, as `(b) -> (C) { c }`, is written with its expression in a group: `(b) -> { (C) { c } }`. Today such a clause block is correct, so A2 and the formatter must add the group.
  - §2.2: an attribute takes call arguments, not only strings: `Attribute = "@" [ name "." ] name [ "(" [ Arg { "," Arg } [ "," ] ] ")" ]`, with `Arg` as in §6.1. The built-in attributes still take only strings, and that is checked after the item is read. An item that begins with `(` after an attribute begins on a new line (§1.7), because a `(` on the same line is the arguments of the attribute.
- **Spec.** §0 and §2.1 (file names, the library module `i/syntax`, the order of modules), §1.1, §1.5 (`` ` ``, `~`), §2.2 (attributes: `@macro`, a macro before an item, two items with one name; the item forms above; names compared by text and scope, §5), §2.4 (exports checked after expanding; the new rule of §5 on types of other modules in exported items), §2.5 (a declaration with `@macro`; compile-time bindings), §3 (a macro call in a type), §4.3 and §4.7 (names compared by text and scope), §6.9 (`|>` with a macro), §10.2 and §10.3 (the macro entry point; the compiler services are not host services), §11.7 (stops in expanded code), §11.8 (the text form of a constructor with marks), §13 (`hash` in a macro program), Appendix B.6 (a new kind of entry), a new section for macros, and a new appendix for the Lisp surface. M1 writes §1.1, §1.5, and the appendix; M2 to M4 write the rest. Other streams change many of these sections too, and the order of merging must be set for each: §0 and §2.1 (IO), §2.2 and §2.4 (A1), §2.5 (A1, T2), §3 (A1, T1, T2), §4.3 and §4.7 (A1, T2), §6.9 (A1, T1), §10.3 and §11.8 (T3), and Appendix B.6 (C). In each of them, the M stream is merged last.
- **`ilang-syntax`.** A parser and a printer for the Lisp surface; datums; quote; the attribute form; a macro call as an item and in a type.
- **A new driver** over the parser, `ilang-check`, `ilang-wasm`, and `ilang-host`: the order of modules, the two steps of §4.4, marks and the lookup of §5, putting off, the limits, and kept results.
- **`ilang-check`.** The work named above, and resolving names with scopes. Choosing the parser by the end of the file name in `parse` (or a field for the surface in `ModuleSource`). Each expression needs an identity that is different in each expansion (a node id, or a span with a source and an expansion id), because two expansions of one template in one module give nodes with the same span; every table keyed by `(ModuleId, Span)` moves to it. These tables are in files of T1 and T3 (`types/mod.rs`, `infer.rs`), and T3 adds more of them, so this change is made only after T3 has landed (or T1 makes it), and M2 is in the order of merging of `infer.rs` and `types/mod.rs`. The rule that the environment of an entry point has only host services and `Assert` (`Checker::entry_points`, `entry_point`, `entry_services`, error E0522) must not be used for the macro entry point, which has `Definitions`, `Types`, and `Assert`; today entry points come only from the root package, and macros of other packages are entry points too. R2 changes this code, so M2 is merged after R2 in `infer.rs`. A module whose macro calls are not expanded is type checked without them, also when another module has errors (§9); today any error of resolution stops type checking for the whole program.
- **`ilang-wasm`.** The macro entry point, as a new kind of entry (spec Appendix B.6), and the form of `Syntax` at the boundary: a private encoding that the host never sees as a `Value`. `Definitions`, `Types`, and `Assert` are services of the compiler, not `@host` services, so the rule of spec §10.3 on host types (E0320) is not used for them. Quotes; leaving out compile-time bindings. Code in function calls and in allocation that counts the units of the limits (§4.4); the counters are in generated code, because the host can only count Wasm instructions and bytes of stack. The limit on how deep calls go must stop a run before the Wasm stack is used up: the stack given to a run is larger than the limit times the largest frame, or deep frames are on the heap. Stop sites (§9): today `Lower::site` finds the line and column in the source of the module that is lowered, and the table of sites in `emit/mod.rs` has one position for each site. A site takes its source from the identity of the expression, so that code from a template has the position in the template's file, and the table holds two positions, the call and the template. `runtime/stops.rs` prints both. These changes to the runtime are part of M2 and are merged after B.
- **`ilang-host`.** Running a macro program with only the services of the compiler, and reporting when it goes over a limit.
- **`ilang-package`.** `.is` files as modules; the error when a module has two files. The names of the files of the library, which today all end in `.i`. `Diagnostic` has one source and one span; it gets notes, each with a source, a span, and a message, for "the template as a note" (§4.4, §7.3, §9). `render_all` shows them, and the language server sends them as related information. This is part of M2.
- **`ilang-cli`, `ilang-lsp`, editors.** `i fmt` for `.is` and `--to`; `i expand`; `i check` and the language server running macros; choosing the parser by the end of the file name, in place of taking only `.i` files; a grammar for `.is`; the package template; tests of the spec for the new sections. The spec tests (`crates/ilang-cli/tests/spec.rs`) and the WAT dump (`crates/ilang-cli/tests/wat_dump.rs`) take only `.i` files and copy each one to `test/{stem}.i`; they must take `.is` files too and keep the end of the name. This is part of M1, merged after A2's change to `tests/spec.rs`. For `i test`, the functions `node`, `make`, `same_code`, `show`, and `show_as` of `i/syntax` are imports that the test runner of the CLI gives, because they need `ilang-syntax`, which `ilang-host` does not depend on (M2b). `definition` and the functions of `Types` are i code over the services (§3.2), so they are not imports.
- **Library.** The module `i/syntax`.

Some of this touches files that other streams own: the boundary, the entry kinds, and `emit/host.rs` (streams C and R2); the counters in `emit/control.rs`, the runtime, and `runtime/stops.rs` (stream B); `ilang-host` (streams C and IO); the entry points in `infer.rs` (stream R2); and the per-expression tables (streams T1 to T3). Each of these parts of M2 is merged after those streams.

## References

- McCarthy, *Recursive Functions of Symbolic Expressions and Their Computation by Machine, Part I* (1960): M-expressions and S-expressions.
- Bawden, *Quasiquotation in Lisp* (1999).
- Kohlbecker, Friedman, Felleisen, Duba, *Hygienic Macro Expansion* (1986); Clinger, Rees, *Macros That Work* (1991); Dybvig, Hieb, Bruggeman, *Syntactic Abstraction in Scheme* (1992), for marks; Flatt, *Binding as Sets of Scopes* (2016).
- Flatt, *Composable and Compilable Macros: You Want It When?* (2002), for running macros in a separate step.
- Rafkind, Flatt, *Honu: Syntactic Extension for Algebraic Notation through Enforestation* (2012); Flatt and others, *Rhombus: A New Spin on Macros without All the Parentheses* (2023), for macros in a syntax that is not Lisp.
- Burmako, *Scala Macros: Let Our Powers Combine! On How Rich Syntax and Static Types Work with Metaprogramming* (2013), for macros in Scala. The names "blackbox" and "whitebox" seem to come later, from the documentation of Scala 2.11 (2014); this reference is likely wrong for them and must be checked.
- Languages: Reason and OCaml (two syntaxes for one tree); Elixir (a macro call has the form of a call); Swift macros (macros in a sandbox that see only syntax, and macros that add items next to a declaration); Rust procedural macros and the `watt` project (macros as Wasm); Template Haskell `reify` and Nim `getImpl` (asking for definitions).

The years, the titles, and what each work or language is said to do are from memory, here and wherever the body names another language or a paper (the decisions, §10, §11). Check all of them before this text goes into the spec.
