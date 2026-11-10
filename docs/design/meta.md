# Design: metaprogramming with two surfaces

Status: draft, after three rounds of review by independent readers. Every decision in it is confirmed; the open questions are in §11. Nothing here is in `docs/spec.md` or in the compiler yet. This design is written on top of the syntax with no keywords (`docs/design/syntax.md`, streams A0 to A2), and every example uses that syntax. It can only be built after stream A1. §12 says in one line what each stream named here is.

This text cites two drafts that are not in this tree. Every reference to `docs/design/syntax.md` by section or test number (as §1.4, §2.2 `MacroItem`, §4.3, and tests 1 to 6 of §6.1) is to the draft on the branch `lane/note-a0` at commit a88558c, not to the older syntax.md next to this file. Every reference to `docs/design/literal-types.md` (§5.9) is to the draft on the branch `lane/note-s2` at commit 6130ab2. Both are unmerged drafts, so each such reference must be checked again when its draft lands.

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

Two core trees are **the same code** when they are different only in groups: `{ e }` and `e`, and `{ T }` and `T` in a type. The result `_` of a function expression is never in a group: `{ _ }` there is a compile error (§2.3, and the change to syntax.md §6.8 in §12), so a group cannot change whether a function has a `->` part. A macro never sees a group (§3.1), so code that is the same code expands in the same way.

Each surface has a parser and a printer. The printer of the normal surface adds the groups that its rules of binding need, and the line breaks that its rule of lines needs (syntax.md §1.7); the Lisp surface never needs either. With no line break, syntax.md §1.7 reads a statement, a clause, or an item as the longest expression it can, so one that begins with `(`, `{`, `-`, or `-.` would join the one before it: `{ (= y g) (= (, a b) (f y)) a }` on one line is `{ y = g  (a, b) = f(y)  a }`, which reads `g(a, b)`. So the printer puts a line break before every statement, clause, or item that begins with one of these tokens, and between an attribute and an item that begins with `(`. It also puts a guard in a group when the guard has a `->` that is not inside brackets, because syntax.md §6.1 ends the clause head at that `->`. And it puts a clause expression in a group when the clause is the first of its clause block, its pattern is `n1 = … = nk = ( … )`, and its expression begins with a function, because syntax.md §6.1 test 2 would read the clause as the binding of that function: `(|> v { (-> (= p (, a b)) (-> () _ (log p))) })` is written `v |> { p = (a, b) -> { () { log(p) } } }`.

The printer needs no other line break. Syntax.md §6.1 makes its test for a clause whose pattern is wrong only for a first statement that its earlier tests do not decide, so never for a binding, a binding with type variables, or an environment statement, and a `->` in a later statement on the same line does not count. So a first statement `id(A): (A) -> A = (x) { x }`, and a line `{ y = g  h = (x) -> Int { x }  h(y) }`, are read as blocks. This design depends on that rule of syntax.md; if syntax.md changes it, the printer must put a line break after a first statement that is a binding. So:

- Every `.i` file may be changed into a `.is` file that is the same code, but for the forms below that wait for a change to syntax.md.
- A `.is` file may be changed into a `.i` file that is the same code, but not when it has one of the forms below, because the normal surface cannot write them. Then `i fmt --to=i` reports an error and writes nothing, and `show_as` gives a `SyntaxError`. §2.3 says which other datums the Lisp parser does not take, so that every other file can be changed.
  - A quote (§3.3), or an item with `@macro`.
  - A `{` that begins an expression, whose first statement is, or begins with, a function with a `->` part whose parameters are all patterns, and that is not bound to a name: a block or a group that is not the body of a function, a guard that the printer must put in a group, or the expression of a first clause that the printer must put in a group (above). Examples are a block whose first statement is a call of such a function, `{ ((-> (b) Int b) 1) }`, the guard in `(-> (? x ((-> (a) Bool (> a 0)) x)) 1)`, and the clause expression in `(|> v { (-> (= p (, a b)) (-> (x) Int x)) })`. For that last one, syntax.md §6.1 binds the function to a name inside the group, which is not the same code. Syntax.md §6.1 reads each such `{` as a clause block, and a group does not help, because a group is also a `{ }`. This is the rule of syntax.md, not a change that is waiting: the author binds the function to a name first. Three cases are not this case. The body of a function is always a block, and syntax.md §6.1 does not make its tests for it, so `(-> (a) _ (-> (b) Int (+ a b)))` is `(a) { (b) -> Int { a + b } }`, which can be written. A binding of such a function to names, `(= g (-> (x) Int x))` as the first statement, is read by syntax.md §6.1 as the binding `g = (x) -> Int { x }`. And a function with an environment parameter is not read as a clause, because its parameters are not a pattern, so tests 1 to 5 of syntax.md §6.1 do not hold and by test 6 the `{` is a block. So `{ ((-> (n (! l Log)) Int n) 1) }` is `{ (n, l ! Log) -> Int { n }(1) }`; the guard in `(-> (? x ((-> (a (! l Log)) Bool (> a 0)) x)) 1)` is correct in the group that the printer puts; and in `(|> v { (-> (= p (, a b)) (-> (x (! l Log)) Int x)) })`, the exception of test 2 does not hold, because `(x, l ! Log)` cannot be read as a type, so `v |> { p = (a, b) -> (x, l ! Log) -> Int { x } }` is a clause whose expression is the function, and the group that the printer puts there is a block by test 6. All three can be written.

Syntax.md already reads most forms that this design needs, and keeps them for macros (syntax.md §1.4): a statement or an item `name ( … )` is a binding with type variables only when the `( )` has only `UpName`s and `=` or `:` comes after it (syntax.md §2.5, §6.1), so `(decode User text)`, `(log Info)`, and the item `(derive User)` can be written; an item `name [TypeParams] : Type` with no `=` is a declaration whatever its attributes (§2.2, §2.5), so `(@ rpc (: fetch (-> (Id) (! User Http))))` is `@rpc fetch: (Id) -> User ! Http`; a `{` followed by `name: Type` and then `=`, `->`, `?`, or `|` begins a clause block or a block (§6.1), so `{ (-> (: n Int) "int") (-> _ "other") }` can be written; an attribute takes call arguments and may be named with its module, and a call with no `=` or `:` after it is an item (§2.2); and a type may have a call `m(…)` or `a.m(…)` (§3).

Changes to syntax.md §2.2 and §6.8 are still needed for these promises (§12). Until they are made:

- A `.i` file with `{ _ }` as the `->` part of a function, as `(x) -> { _ } { e }`, cannot be changed into a `.is` file, because the Lisp surface has no form for it (§2.3). `i fmt --to=is` reports an error for it.
- A `.i` file with a macro with empty brackets before an item, `@m() f = 1` or `@a.m() f = 1`, or with an attribute before a macro call as an item, as `@main gen(User)`, cannot be changed into a `.is` file, because the Lisp surface has no form for it (§2.3, §4.3). Syntax.md §2.2 reads both; the change makes them compile errors. `i fmt --to=is` reports an error for them.

Three forms of later streams need rows of §2.3 that come with those streams:

- Literal types (stream T1) have the row "Literal type" of §2.3. Until T1 lands, there are no literal types, and the row is not used. After T1, a clause `x: 1 -> "one"` tests a literal type, so the printer writes `n |> { x: 1 -> "one"  _ -> "other" }` for a correct clause block. Syntax.md §6.1 test 4 gives `{ x: 1 -> 2 }` as an example of the records error, "because `x: 1` is not a pattern", which is no longer true after T1. So T1 must change that example and its reason in syntax.md §6.1 (and in the spec text made from it), for example to `{ x: f(1) -> 2 }`, which stays a records error.
- The suffix pattern `r <> "lit"` (stream STR) has the row "String suffix" of §2.3, `(<> x "lit")`. Until STR lands, the row is not used. If STR changes the form of the pattern, it changes the row before it lands.
- String interpolation, `"user-\(n)"` (stream STR), is sugar that the parser of the normal surface removes before the core tree, as §2.3 says. If STR gives it a meaning that `<>` cannot write, STR must give it a row of §2.3 and a form in `Node` first; until then a `.i` file with it cannot be changed into a `.is` file.

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
| Built-in attribute | `@main Item`, `@host("ns", "async") Item` | `(@ main Item)`, `(@ (host "ns" "async") Item)` |
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
- An item `(U { … })` or `((U A …) { … })` is a custom type. An item `(= U T)` or `(= (U A …) T)` is a type alias. A top-level binding may not have a pattern that starts with an `UpName`, as in syntax.md §2.2. An item that is a `Round` whose head is a name or a `(. a m)` is a macro call. A type alias whose left side is a type pattern (stream T2, as `Schema(User, _) = user`) is written as the same datum, `(= (Schema User _) user)`. Both surfaces tell it apart from an alias with parameters by the test of syntax.md §4.3 on the arguments, which A1 builds into the parser and resolving; this design takes that test and adds no other. When every argument is an `UpName` atom with no module name and no arguments, and none of them is a type that may be written with no module name in the module, the item is a type alias with parameters, which defines its head. When not, as when an argument is `_`, a nested type, a qualified name, or a type that the module may use (`Schema(User) = user` with a type `User`), the item is the form that syntax.md keeps for alias patterns: a compile error until T2, and a type pattern after it. The test compares the text and the scope of each argument (§5): an argument is a type that the module may use when rule 2 or 3 of §5 finds a type for it. Code read from a file has no marks, so for every item of a source file this is the test of syntax.md on the text, and the `.i` and `.is` forms of one file are read in the same way by A1 and by the macro streams. Marks only count in an item that an expansion gives: a `User` with marks, in `(= (Codec User) …)` from a template of `D`, is looked up as a name of `D`, and not as the caller's type `User`. The types that are looked at in the module that is being expanded are the built-in types, the imported types, and the types written in its source file, also the head of every type alias written there, whatever its form. The types that an expansion defines are left out, and an expansion that defines a type whose text and scope are those of an argument of an item of this form, in the source file or given by another expansion, is a compile error. So the reading never depends on the order of expansions, and the test is done when the item is there: for an item in the source file, before any run of step 1 of §4.4, and for an item that an expansion gives, when that run ends, before the macro calls inside the item are expanded. So the names that an alias defines are known as soon as the item is, and a run that asks for them is answered as §6 says. Two aliases with one head are two definitions of one name, which is a compile error (spec §2.2). From this test, a type pattern whose arguments are all new `UpName`s, as `Codec(User) = user_codec` with `Codec` imported and `User` new, is read as an alias that defines `Codec`, which is a compile error because `Codec` is imported (spec §2.3). If T2 wants such a type pattern, T2 must change syntax.md §4.3 and its notes for A1, and this text with it (open question 2). Which `UpName`s in the argument places a type pattern defines, as `User` here, or `Json` imported in `Schema(User, Json) = user`, or `List` in `Schema(List(User), _) = users`, is decided by T2 (open question 2). When the type comes from another module, the head is qualified, `(= ((. s Schema) User _) user)`, written `s.Schema(User, _) = user`. That datum fits no row of a type alias today, and it is a correct binding with the constructor pattern `(. s Schema)`, where `User` is a constructor with no fields. Such a head can never be an alias with parameters, because an alias defines its head and a qualified name is not defined in this module. So the T2 datum also covers a qualified head, `(= ((. a U) X …) T)`: it is always a type pattern, and such an item is no longer a binding. In the normal surface, it starts with a lowercase name, so syntax.md §2.2 reads it as a binding too, and T2 must change that rule as well. The exact rule comes with T2 (open question 2).
- **Items from an expansion.** A `Curly` of zero or more items is only taken as the result of a macro in the position of an item (§4.3). It is never taken in a source file, and it has no form in the normal surface: `show_as` and `i expand` write its items one after the other, as the items of a file.
- Each `(@ …)` has one attribute. More attributes are put one inside the other, the first on the outside: `@main @deprecated("x") f = e` is `(@ main (@ (deprecated "x") (= f e)))`.
- `@m Item` gives the macro only the item. `@m()` and `@a.m()` are compile errors in both surfaces, as `@main()` is in syntax.md §2.2 (a change to syntax.md, §12); `(@ (m) Item)` and `(@ ((. a m)) Item)` are compile errors.
- An attribute, built-in or a macro, may not be put before a macro call as an item: `@main gen(User)` and `@m gen(User)` are compile errors in both surfaces (§4.3; in the normal surface a change to syntax.md §2.2, §12), and so are `(@ A (m …))` and `(@ A { … })` in the Lisp surface.

**Types**

| Core form | Normal surface | Lisp surface |
|---|---|---|
| Union, intersection, difference | `A \| B \| C`, `A & B`, `A - B - C` | `(\| A B C)`, `(& A B)`, `(- A B C)` |
| Function type | `(T1, x: T2) -> R` | `(-> (T1 (: x T2)) R)` |
| With errors and environment | `(T) -> R ? E ! V` | `(-> (T) (! (? R E) V))` |
| With errors only, environment only | `(T) -> R ? E`, `(T) -> R ! V` | `(-> (T) (? R E))`, `(-> (T) (! R V))` |
| Tuple type | `(T1, T2)`, `()` | `(, T1 T2)`, `(,)` |
| Named type | `U`, `a.U`, `U(T1, T2)`, `a.U(T)` | `U`, `(. a U)`, `(U T1 T2)`, `((. a U) T)` |
| Hole | `_` | `_` |
| Group | `{ T }` | `{ T }` |
| Literal type (stream T1) | `"a"`, `1`, `-1` | `"a"`, `1`, `(- 1)` |
| Macro call | `m(T, 3)`, `a.m(T)` | `(m T 3)`, `((. a m) T)` |

- `|`, `&`, and `-` take two or more parts in a type. A part may be a `|`, `&`, or `-` form; that is the type with a group there, `A | { B | C }`.
- A string or integer atom in the position of a type is a literal type. A `(- k)` with exactly one part, where `k` is an integer atom, is a negative literal type; it does not clash with the difference, which has two or more parts. So `Int - -1` is `(- Int (- 1))`, and `send = (m: "GET" | "POST") { … }` is `(= send (-> ((: m (| "GET" "POST"))) _ …))`. A float atom in a type is a compile error, as in the normal surface. This row and this bullet come with T1 (`docs/design/literal-types.md` §5.9). With them, T1 changes the example `{ x: 1 -> 2 }` of syntax.md §6.1 test 4 (§1).
- A function result is `R`, `(? R E)`, `(! R V)`, or `(! (? R E) V)`. No other use of `?` and `!` is correct there. So `(x) -> Int ! Log | Http { … }` is `(-> (x) (! Int (| Log Http)) …)`.
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
| Tuple | `(e1, e2)`, `()` | `(, e1 e2)`, `(,)` |
| List | `[e1, e2]`, `[e1, ..e2]` | `[e1 e2]`, `[e1 .. e2]` |
| Bit array | `<<e1, e2: f, e3: f(e4)>>` | `<<e1 (: e2 f) (: e3 (f e4))>>` |
| Block, group | `{ s1  s2  e }`, `{ e }` | `{ s1 s2 e }`, `{ e }` |
| Binding statement | `p = e`, `x(A) = e`, `x(A): T = e` | `(= p e)`, `(= (x A) e)`, `(= (: (x A) T) e)` |
| Environment statement | `! N = e` | `(! N e)` |
| Clause block | `{ p1 -> e1  p2 -> e2 }`, `{}` | `{ (-> p1 e1) (-> p2 e2) }`, `{}` |
| Clause with choices | `p1 \| p2 \| p3 -> e` | `(-> (\| p1 p2 p3) e)` |
| Clause with a guard | `p ? g -> e`, `p1 \| p2 ? g -> e` | `(-> (? p g) e)`, `(-> (? (\| p1 p2) g) e)` |
| Function with no `->` part | `(p1, p2) { e }` | `(-> (p1 p2) _ e)` |
| Function with a `->` part | `(p) -> R ? E ! V { e }` | `(-> (p) (! (? R E) V) e)` |
| Environment parameter | `x ! N` | `(! x N)` |
| Record update | `C(..e, x: e2)`, `a.C(..e)` | `(C .. e (: x e2))`, `((. a C) .. e)` |
| Quote | none | `` `d `` |

- A `Round` with one or more parts whose head is not a symbol is a call, but when its second part is `..` it is a record update. The head is the function. A binary operator takes exactly two parts; `-` and `-.` may take one.
- **Functions and clauses.** In the position of an expression, a `(-> …)` has three parts: the parameters, the result, and the body. The body is one expression. The result `_` means that the function has no `->` part, so its value, errors, and environment are all inferred (spec §6.8). Any other result is a `->` part, in which a missing `?` or `!` means `Never`; the normal `(x) -> _ { e }` is `(-> (x) (? _ Never) e)`. The result `_` is only the bare atom: a `{ _ }` there is a compile error, in both surfaces. In the normal surface, `(x) -> { _ } { e }` is correct under syntax.md today, so this is a change to syntax.md §6.8 (§12). A clause is a `(-> …)` with two parts, and it is only in a clause block. A function type also has two parts, and it is only in the position of a type. A macro that looks at a `(-> a b)` must know its position to know which of the two it is.
- **Blocks.** A `{ }` in the position of an expression is a clause block when it is empty, or when its first part is a `(-> …)` with two parts. Then every part must be a clause. When not, it is a block, and it has one or more parts.
- A clause block with one clause, `{ (-> p e) }`, and the function `(-> (p) _ e)` are two core trees with one meaning (`docs/design/syntax.md` §6.12).
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
| Tuple | `(p1, p2)`, `()` | `(, p1 p2)`, `(,)` |
| List | `[p1, p2]`, `[p, ..]`, `[p, ..x]`, `[..]`, `[..x]` | `[p1 p2]`, `[p ..]`, `[p .. x]`, `[..]`, `[.. x]` |
| Bit array | `<<p1, p2: f, p3: f(e)>>` | `<<p1 (: p2 f) (: p3 (f e))>>` |
| Two patterns for one value | `x = p` | `(= x p)` |
| Quote pattern (§3.3) | none | `` `d `` |

- In `(: x T)`, in `(<> "a" x)` and `(<> x "a")`, and in `[p .. x]` and `[.. x]`, `x` is a name or a discard, as in spec §7.1. The `(: (x A …) T)` of a binding with type variables and a type is not a pattern. In `(= x p)`, `x` is a name. In `(- k)` and `(-. k)`, `k` is a literal of the kind of the operator.
- The statement `a = (x, y) = e` is `(= (= a (, x y)) e)`.

**Datums that are compile errors.** These fit no row: `()` in every position but a list of parameters; a named type or a type head with no arguments in brackets, `(U)` or `((U) { … })`; a constructor of a type definition with no fields in brackets, `(C1)`; a binding head with no type variables, `(= (x) e)` or `(= (: (x) T) e)`; a `(: (x A …) T)` that is not a declaration or directly under `=` in a binding; a `(, …)` at the top level that is not the one imports group or the one exports group at the start of the file; an empty list of import names, `(= (,) (/ a b))`; a list that begins with `..` in the position of an expression, `[.. e]`; a part of a record update with no label, `(C .. e 1)`; an attribute with empty brackets, `(@ (m) Item)` or `(@ ((. a m)) Item)`; an attribute before a macro call as an item or before a `Curly` of items, `(@ A (m …))` or `(@ A { … })`; a pattern of a bit array segment that is or ends in a type test, with or without a format, as `<<(= x (: y Int))>>` or `<<(: (= x (: y Int)) u8)>>`, because in a segment of the normal surface a `:` always begins the format (spec §8.1), so `<<x = y: Int>>` is the pattern `x = y` with the format `Int`.

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

Position {
  ImportItem  ExportItem  Item  Attribute  Constructor  Field  Type  FunctionResult
  Parameter  Expression  Statement  CallArgument  Clause  ClauseHead  Pattern
  Segment  Format
}

SyntaxError { SyntaxError(message: String, at: Syntax) }
```

- A `Syntax` value is one datum of §2.2 with where it came from: its span in a source file, and its **scope**, which says where its names are looked up (§5). `Syntax` has no constructors. Only the compiler, a quote, and `make` give `Syntax` values.
- `Position` has one constructor for each position of §2.2, in the same order (`Segment` is the bit array segment). So `show_as` can print any part that a macro takes out of a block, a clause block, a function, or a definition.
- `node(s)` gives the `Node` of a `Syntax`. `Name`, `Discard`, `Up`, and `Symbol` have the text of the token. `Int`, `Float`, and `Text` have the value of the literal.
- An atom is always one token of its kind. The text of a `Name`, a `Discard`, an `Up`, or a `Symbol` is one name, one discard, one `UpName`, or one symbol of spec §1 that is not a bracket. The value of an `Int` or a `Float` is one that a literal can write: it is not negative, and a `Float` is not `-0.0`. A negative number is the `Round` `(- 1)`, as in a pattern (§2.3). `make` gives a `SyntaxError` for any other atom, as `Int(-1)`, `Name("Foo")`, `Name("a b")`, `Name("")`, or `Discard("x")`. So every `Syntax` value can be written as text, and an atom always fits the rows of §2.3 that its kind of token fits.
- **The `Syntax` of a piece of code is the datum that the Lisp surface writes for it** (§2.3). This is true for code from a `.i` file too. So a macro always sees the Lisp form: the node of the argument `n > 0` is a `Round` of three parts whose nodes are `Symbol(">")`, `Name("n")`, and `Int(0)`.
- A macro never sees a group. The compiler takes every group out of the code it gives to a macro (the arguments of a call, the item under a macro before an item, and the answers of `Definitions` and `Types`), and puts its one part in its place. A group is a `Curly` with one part in the position of an expression, when that part is not a clause, and a `Curly` in the position of a type or of a function result. So code that is the same code (§1) gives the same `Syntax`, and the groups that the printer of the normal surface adds, as in `x * { y + z }`, do not change how a quote pattern matches. A group that a template makes, as `` `{ ~x } ``, stays in the result, where it has no effect.
- `Syntax` has no documentation comments in it.
- `==` on two `Syntax` values is `True` only when they are the same value. To compare code, use `same_code`.

### 3.2 Functions of `i/syntax`

| Function | Type | What it gives |
|---|---|---|
| `node` | `(Syntax) -> Node` | The node of the value |
| `make` | `(node: Node, like: Syntax) -> Syntax ? SyntaxError` | A new `Syntax` with this node, and the span and the scope of `like`. A `SyntaxError` when the node is an atom that is not one token of its kind (§3.1). §5 says how a name made in this way is looked up. |
| `same_code` | `(Syntax, Syntax) -> Bool` | `True` when the two are the same datum. Spans and scopes are not looked at. Code that the compiler gives has no groups (§3.1), so for such code this is the same code of §1. |
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

In a template, `` `{ ~..parts} `` is: `{}`, the clause block with no clauses, when `parts` is empty; a group when it has one part that is not a clause; and a clause block with one clause, which is a function value, when its one part is a clause, as `` `(-> 0 1) ``.

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
- In the normal surface, syntax.md already reads a macro call as an item, a macro before an item with any arguments or named with its module (`@a.m`), a macro call in a type, a call whose first argument is an `UpName` at the start of a statement or an item, as `decode(User, text)`, and a declaration `name [TypeParams] : Type` with no `=`, whatever its attributes, also one with no `@external` and one with a macro before it, as `@rpc fetch: (Id) -> User ! Http` (§1). It keeps each of them for macros, and the macro streams give them their meaning.
- `e |> m` is the macro call `m(e)`, and `e |> m(args)` is the macro call `m(e, args)`, as `|>` works for every call (spec §6.9). This is only in the position of an expression.
- The number of arguments must be the number of parameters of the macro. An argument with a label goes to the parameter with that label, by the rule of calls (spec §6.6), and the macro gets it with no label.
- The arguments are not worked out. The macro gets their `Syntax`, and its result says if and when each of them is worked out.
- After T1, an integer literal or a string in the position of a type is a literal type (§2.3), and its `Syntax` is the same atom as before, so a macro sees no difference. A negative literal type, as `m(-1)`, written `(m (- 1))`, is then a type too; it is not in decision 5, and taking it changes that decision, which the user decides (`docs/design/literal-types.md` §5.9 asks for it).
- A name in a bit array format, as `size` in `<<x: size(8)>>`, is never a macro call.
- No macro call gives a statement: the result of a call in the position of an expression is an expression, also when the call is a statement of a block. So an expansion cannot add a name to the block around it, and which names are local at a call does not depend on other expansions.

**A macro before an item.**

- Before a **type definition**, the macro cannot change the item. The type definition stays in the module as it is written, and the result of the macro, an item or a `Curly` of zero or more items, is added after it. So a type definition that is written is always the definition of its type.
- Before a **binding or a declaration**, the result, an item or a `Curly` of zero or more items, takes the place of the item. To keep the item, the macro puts it in its result.
- The item is given to the macro with the attributes that are written after the macro. The item always goes to the last parameter, also when another argument has a label.
- A built-in attribute before a macro, as in `@test @m f = …`, is a compile error; write it after the macro, so that it is a part of the item. An attribute whose name is one of the built-in attributes (`external`, `js`, `host`, `main`, `test`, `deprecated`, `macro`) is that attribute; a macro with such a name is used with its module name, as `@a.test`, which syntax.md §2.2 reads as a macro.
- An attribute, built-in or a macro, before a macro call as an item, as `@deprecated("old") gen.derive(User)` or `@m gen(User)`, is a compile error. Only a type definition, a binding, or a declaration may have a macro or another attribute before it. A macro call as an item that is to give items with attributes puts them on the items of its result itself, and takes what it needs for that as arguments.
- With more than one macro before an item, the first is expanded first, and it gets the item with the other macros still on it.
- The rule of spec §2.2 that an attribute is not given two times is for built-in attributes only.
- The documentation comment of the item stays on the type definition, or goes to the first item of the result that defines the same name.

**Limits on what a result may have.**

- Imports and exports cannot be in a result.
- A `Quote`, `Unquote`, or `Splice` node cannot be in a result.
- The attributes `@external`, `@js`, `@host`, `@main`, and `@test` may be in a result only when the whole `(@ …)` datum, with the name atom, the arguments of the attribute, and the item it is on, came from a source file of the caller and reached the result through the arguments of each run, unchanged. Each such datum may be in the result at most one time. Having no marks is not enough: a name made with `make` (§5) is never such an atom, also when it has no marks after the run. So a macro of a package that the program depends on cannot add a host import or an entry point that the caller did not write: it cannot reuse the caller's `external` atom with other arguments, put the caller's `@main` or `@host(…)` on an item of its own, or give one declaration two times.
- A `Curly` of items may only be the whole result. A `Curly` in the position of an item anywhere inside a result, directly inside the `Curly` of the result or under a `(@ …)`, is a compile error.

### 4.4 Expanding

The compiler expands a macro call by calling the macro with the `Syntax` of the arguments. One call of the macro is one **run**.

- Expanding goes from the outside to the inside. An argument is given to the macro as it is written, with the macro calls in it not expanded. After a call is expanded, the macro calls in its result are expanded in the same way.
- The result is read by the tables of §2.3 for the position of the call. If it fits no row, it is a compile error at the call, and the message names the macro.

A module is expanded in two steps:

1. **Items.** The compiler expands the macro calls in the position of an item, the macros before items, and the macro calls in the position of a type that are inside type definitions. It goes on until none is left. A run may be put off in this step (§6). After this step, the top-level names of the module are all known and every type definition is complete. Then the exports are checked: every exported name must be defined.
2. **Bodies.** The compiler expands the macro calls in the position of an expression, and the other macro calls in the position of a type. Their order has no effect on the result, because they cannot add a top-level name or a local name outside their own result.

Typed macro calls are expanded later (§7.3).

**Limits.** The spec gives limits for one run: on the work it may do, on the memory it may use, and on how deep its calls may go. It also gives a limit for how deep expansions may be inside the results of other expansions. The limits are counted in units of the language, such as calls and the size of values, and the depth in frame units (§12), so they do not depend on the machine or on how the compiler optimizes. Which units count the work and the memory, and whether a package may raise a limit, is open question 3. Going over a limit is a compile error that names the macro and the limit.

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
- A type parameter in an answer of `definition` (§6), where the head of the definition binds it and at each use of it in the answer, also has a scope of its own kind, which names that parameter of that definition. Only that binding in that head can bind it (rule 1). So a use of it that a macro moves out of the head has no binding and is a compile error, and no other binding, of a template, of the caller, or of another answer, can bind it.
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
- This has a limit. All templates of one module that are used in one run get the same scope: that module and the mark of the run. So when a macro puts the result of one template inside another, a name that the outer template binds takes a free name of the inner one with the same text. If `wrap` gives `` `(-> (limit) _ ~body) `` and `check` gives `` `(< 1 limit) ``, a macro that gives `(wrap (check))` binds the `limit` of `check` to the parameter of `wrap`, not to the top-level `limit` of the module. Racket has the same limit inside one transformer. The author of the macro uses names that do not clash.
- To make a name that the caller can see, a macro uses a name from its arguments, or `make(Name("it"), like: arg)`. A macro with no arguments cannot make such a name.
- A name made like an argument has the caller's scope, so as a binder it takes the caller's names with the same text in the code from arguments that it is around. So a macro makes it like an argument only for a name that the caller is to use, as a top-level name. To bind a local name that the caller does not use, with a text that the macro gets as data, as the text of a field, the macro makes it like an atom of a template: ``make(Name(text), like: `x)``. That name has the scope of the macro's module and the mark of the run, as a binder of a template has. A constructor pattern takes a labeled field only through a part whose last `=` name is the label (spec §7.1), and that name is always bound (spec §6.11), so this is the only way to read a field by its label without taking a name of the caller. For `User { User(name: String) }` and a macro `field_or(v, T, fallback)` that gives `` `(|> ~v { (-> (User ~n) (if_empty ~n ~fallback)) (-> _ ~fallback) }) ``, `n` must be ``make(Name("name"), like: `x)``. With `make(Name("name"), like: v)`, the call `field_or(u, User, name)` inside `(name) { … }` would bind the caller's `name` in `~fallback` to the field, and nothing would report it.
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
Definitions { Definitions(definition: (of: Syntax) -> Syntax ? SyntaxError) }

definition = (of: Syntax, d ! Definitions) { d.definition(of) }
// type: (of: Syntax) -> Syntax ? SyntaxError ! Definitions
```

`Definitions` is a service type exported by `i/syntax`. The compiler gives the service when it runs a macro. A test may give its own with an environment statement.

- `of` is a type name: an `UpName`, or `(. a U)`. Any other datum is a `SyntaxError`. It is looked up with the marks it would have in the result of this run, in two steps:
  1. First by rule 1 of §5, among the type variables around it (type parameters and declared type variables): around the place where it is written when it came from an argument or from an answer of `definition`, and around the call when not. If one binds it, it is a `SyntaxError`, also when a top-level type has the same text.
  2. Then among the type names at the top level, by rules 2 and 3 of §5. So a name from an argument is looked up where the caller wrote it.
- The value is the `Syntax` of the item that defines the type: a custom type or a type alias, with its attributes. An alias is given as it is written, not the type it stands for. A type with parameters is given with its parameters, not filled in. The `Syntax` has the scopes that the definition has in its module after expanding, but for its type parameters, which have a scope of their own kind (§5). As an argument does, it has the mark of the run only while the run lasts (§5), so its names mean the same when a macro puts them in its result. A name made like an atom of the answer reaches only exported names of that module (§5, rule 2).
- A type whose definition depends on the type of a value (stream T2, as `User` in `Schema(User, _) = user`) has no written fields or constructors. Asking for it is a `SyntaxError`, so a macro cannot read such a type (open question 2). Its type is only known after type checking, which a syntax macro may not wait for.
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
    (|> (subtype t `Int) {
      (-> Yes `(int_text ~value))
      (-> _ (|> (subtype t `String) {
        (-> Yes value)
        (-> NotKnown (raise (SyntaxError "to_text: the type is not known here" value)))
        (-> No (raise (SyntaxError "to_text: this type has no text form" value))) })) }) })))
```

- The declaration is the **signature**. It is a full function type, with its type variables declared after the name. It may have errors and an environment. It may not have `_`. The rules for written types (spec §4.4) and for types in exported items (spec §2.4) are used for it.
- The binding is the **implementation**. It follows the rules of §4.1, with these changes: it has one parameter for each parameter of the signature, in the same order, and it is checked against `(Syntax, …) -> Syntax ? SyntaxError ! Types | Definitions | Assert`.
- This is the only case where two items have one name. The declaration has neither `@external` nor `@js`; both surfaces read it as a declaration by its form (syntax.md §2.2 and §2.5 read `name [TypeParams] : Type` with no `=` as a declaration whatever its attributes, and keep the one with neither `@external` nor `@js` for macros).
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
- The result is put in the position of the call. Its syntax macro calls are expanded (§4.4, step 2). Then it is type checked there, with the local names and their types as they are at the call. A whole argument that is in the result, as the `Syntax` value that the compiler gave for it, is not inferred again: it keeps the types and the type variables that it had in §7.2, also its expected type, and only the constraints of the place where the result puts it are checked. Any other code from an argument, as a part of it or a bracket that the macro built again from its parts, is checked again with new type variables, as code from a template is, so a constraint of code that the macro left out does not stay on it. Its names still mean what they meant in §7.2 (below).
- Then the typed macro calls in the result are expanded in the same way. The limits of §4.4 are used.

**What the result must be.** In this section, the **lower bounds** of a type variable are the types that come to it (spec §4.7), and its **upper bounds** are the types that it must be a subtype of. Two kinds of type variable are left when the binding group of the call has been checked:

- A type variable is **closed** when it is local to the binding group of the call, is not generalized, and cannot be reached by following bounds, in either direction, from the type of any top-level binding or from a declared or generalized type variable. No other part of the program can give it more types, or see the types it gets.
- Every other type variable is **fixed**: a declared or generalized type variable, a variable of a top-level binding of an earlier group, a variable that can be reached from the type of a top-level binding that is not generalized, also one of the group of the call, and a variable that can be reached by following bounds from another fixed variable. This is decided once, before the result is checked; a type variable that checking the result makes is never fixed. Later groups may still give such a variable more types (spec §4.7), so the macro may not take it to be the types it has now. In `f = compose((x) { to_text(x) }, id)`, the type variable of `x` is in the type of `f`, which is not generalized, so it is fixed, and a later `f("hello")` gives it `String`.

Let the **needed type** of the call be the function type of the signature, with each of its type variables filled in with the type that the checker gave it at this call, and each closed variable in that type replaced by a type that is chosen by its places (spec §4.5 rule 6). The places are not counted in the function type as a whole, where the type of an argument is an input place. They are counted in its **parts**: the type of each argument by itself, and the result type, the errors, and the environment, each by itself, and the top of each part is an output place. So in the part `(y) -> r`, the type of an argument `f`, the variable of `y` is in an input place, and `r` in an output place. `signature()` is built from these parts. A closed variable is replaced only when every place it has, in all the parts together, is of one direction:

- when every place is an output place, by the union of its lower bounds, or `Never` when it has none;
- when every place is an input place and it has an upper bound, by the intersection of its upper bounds;
- in every other case, it is not replaced: when it is in an input place and has no upper bound, when it is in a place that is both (as a type argument that does not change, or a service), or when it is in an input place and in an output place, as the variable of `y` in `(y) -> y`. It is then given to the macro as a fixed variable is (§7.4).

A fixed variable stays in the needed type as itself. The needed type is what `signature()` gives, and the types that `Types` gives are replaced in the same way (§7.4). The replacement is only for what the macro is told: a type with a replaced variable is a type the value has at the call, and a variable that is not replaced is given as a variable, so that the macro does not take it to be one type. The result itself is checked with the type variables themselves (below). This is sound: the check of the result is the normal check for every type variable but a fixed one, so a closed variable that the result gives new types to is checked as in any code, and the code that sends new values into it is checked with the code that takes them out; and a fixed variable is checked by the rigid rules below, which hold for every type that it may stand for later. In `call_with_one(F): (f: F) -> Int` and `g = () { inc = id((y) { y + 1 })  call_with_one(inc) }`, the variable of `y` is closed (the value of `inc` is a call, so `inc` is not generalized), with the upper bound `Int` and no lower bound. In the part `(y) -> r` for `f`, it is only in an input place, so it is replaced by `Int`, and `r`, only in an output place with the lower bound `Int`, by `Int`: `type_of(f)` is `(Int) -> Int`, and not `(Never) -> Int`, and `signature()` is `(f: (Int) -> Int) -> Int`. In `curry_text(B, A): (x: B) -> (A) -> String` and `w = () { curry_text(0) }`, the variable for `B` is closed and is only at the top of the part for `x`, an output place, with the lower bound `Int`, so it is replaced by `Int`. `A` is a generalized variable of `w`, so the needed type is `(x: Int) -> (A) -> String` with `A` fixed, and the result `(-> (y) _ (int_text y))` is not correct. In the same way, in a top-level `t = to_text(id(1))`, the variable for `A` of `to_text` is closed, because the type of `t` is `String`, and it is at the top of the part for `value` with the lower bound `Int`, so `type_of(value)` is `Int`. The needed type can always be written as a type, with each fixed variable written as in §7.4. These examples are written as before T1. After T1, an integer literal has its literal type (§7.4), so the lower bound of the variable for `B` in `curry_text(0)` is `0`, the needed type is `(x: 0) -> (A) -> String`, and in `t = to_text(id(1))`, `type_of(value)` is `1`. In `call_with_one`, the bounds come from `y + 1`, so they stay `Int`.

The whole check of the result, also the check of its type against the result type of the call, uses the type variables themselves, as the local names, the arguments, and the result type of the call have them at the call, and never the replaced types. Every closed variable, replaced or not, takes bounds as in normal checking, because no other part of the program sees it, and a fixed variable is rigid (below). So in the example, a result `(~f 1)` adds `Int` to the lower bounds of the variable of `y`, which its upper bound `Int` allows, and the result is correct. In the same way, with `inc = id((y) { y })`, the variable of `y` is in an input and an output place of `(y) -> y` and has no upper bound, so it is given to the macro as a variable, and the result `(~f 1)` is correct, because it only adds `Int` to its lower bounds.

- The type of the result must be a subtype of the result type of the signature, with its type variables filled in as at the call.
- The errors of the result must be among the errors of the signature and the errors of the arguments. The environment of the result must be in the environment of the signature and of the arguments, by the rule for environments (spec §4.5 rule 4).
- A fixed variable `a` is **rigid**: checking the result never adds a bound to it. Every other type variable, a closed one or one that checking the result makes, takes bounds as in normal checking. The **fixed variables above** `a` are `a` and every fixed variable that is reached from `a` by following upper bounds that are fixed variables, one after the other; the **fixed variables below** `a` are found in the same way through lower bounds. The **upper type** of `a` is the intersection of the upper bounds of the fixed variables above `a` that are not type variables; `a` has an upper type only when there is at least one such bound. The **lower type** of `a` is the union of the lower bounds of the fixed variables below `a` that are not type variables, and `Never` when there is none. Each is found once, before the test, and a circle of fixed variables, as a recursive binding group gives, only means that the same variables are reached again. A constraint `a <: T` holds only when:
  1. `T`, or a part of the union `T`, is one of the fixed variables above `a`; or
  2. `a` has an upper type, and the upper type of `a` is a subtype of `T`, where only variables that are not fixed may be given bounds. So there is one subtype test, with nothing to choose and nothing to undo; or
  3. `T` is a union with a part `r`, where `r` is a type variable that checking the result made, and then `a` is added to the lower bounds of `r` as in normal checking; or `T` is a union with a part `r - U`, in the same way, when the type constructor of each member of `U` fixes its type: the member is a nominal type, built-in or custom, with no type arguments, as `Int`, `String`, or a custom type with no parameters. A record type (stream T3) is never such a member, also with no type arguments, because its values are not fixed by a tag.

  A constraint `T <: a` holds only when:
  1. `T`, or a part of the intersection `T`, is one of the fixed variables below `a`; or
  2. `T` is a subtype of the lower type of `a`, where only variables that are not fixed may be given bounds; or
  3. `T` is an intersection with a part `r` or `r - U`, where `r` is a type variable that checking the result made, and then `a` is added to the upper bounds of `r`, or `a | U` for a part `r - U`, as in normal checking. So `{ r - Int } & String <: a` holds by this case, and gives `r` the upper bound `a | Int`.

  These cases decide a constraint where one whole side is a single fixed variable. The rule of spec §4.4 about where a union or an intersection with a type variable may be written is used for fixed variables in a result too (§7.4), and a result may not write a union or an intersection of two or more fixed variables at the top of a type in a pattern either, as `(: v (| A B))`. These limits do not keep a constraint with two or more fixed variables in one union or intersection from coming up: spec §4.4 allows such a union in every output place, so `(= (: v (List (| A B))) [1])` gives `Int <: A | B`, and a signature `both(A, E1, E2): (f: () -> A ? E1, g: () -> A ? E2) -> A ? E1 | E2` gives `Failure <: ε1 | ε2` when `E1` and `E2` are filled in with the fixed variables `ε1` and `ε2` and the result raises a `Failure`. So two more cases are used, with no choice:
  - A constraint `T <: U`, where `U` is a union with two or more fixed variables among its parts and `T` is not a fixed variable, holds only when `T` is a subtype of `U` with each fixed variable part replaced by its lower type, where only variables that are not fixed may be given bounds. When `T` is a fixed variable, the cases for `a <: T` above are used.
  - A constraint `T <: U`, where `T` is an intersection with two or more fixed variables among its parts and `U` is not a fixed variable, holds when case 1 of `a <: T` holds for one of those parts, or when every such fixed variable has an upper type and `T`, with each fixed variable part replaced by its upper type, is a subtype of `U`, where only variables that are not fixed may be given bounds.

  Each holds for every type that the fixed variables may stand for later, because a fixed variable is always above its lower type and below its upper type.

  In case 3, nothing is added to the bounds of `a`. A bound that `r` has or gets later is checked against `a` by these same rules, so it never adds a bound to `a` either. Because every variable reached from a fixed one is fixed, case 2 never gives a bound to a variable that is a bound of `a`, so a constraint on `a` cannot come back to `a` through such a variable. Case 1 lets a result pass a value through: with `same(A): (v: A) -> A` and the result `v`, in `f = (x) { same(x) }`, the variable of `x` is fixed, its upper bound is the fixed variable of the result type, and the constraint holds by case 1.

  Case 2 lets a new variable take a bound, so a call whose parameter type has a new variable inside a constructor, as `List(r)`, is correct when the upper type of `a` fits it. With records (stream T3), `(length (. ~v items))` on a fixed `a` with the upper type `{ items: List(Int) }` needs `a <: { items: r }`, which holds by case 2 and gives `r` the lower bound `List(Int)`. Case 3 is still needed for a fixed variable whose upper type does not fit `T`, or that has no upper type: its values go to a new variable, as the values that no type test takes go to the catch-all of a clause block. In case 3, `a` is not shown to be disjoint from `U` (spec §4.5 rule 3), so `r - U` is taken only when a value that passes the type test of `U` is always in `U`.
- So a clause block on an argument with type tests on types such as `Int` and `String`, and a last clause `_`, is correct: the catch-all part goes to a new variable by (3). A type test on a type with type arguments, as `(: b (Box Int))`, is correct only when the upper type of `a` shows it by (2), because the test looks only at the type constructor, so a `Box(String)` would pass it (spec §4.8 rule 11). A use of the argument as a type that the bounds do not show, such as `(+ ~value 1)` when the upper type of `a` is not a subtype of `Int`, is not correct either.
- In the result, a top-level name of the module of the call that has no marks must be one of these, so that the names that may be used do not depend on the order in which the checker takes groups that do not use one another:
  - a binding or a constructor that the binding already used before the expansion, or one of a group that the group of the call depends on (spec §4.7);
  - a type, or a constructor of a type, whose definition does not depend on the type of a value (stream T2, open question 2). Such a type is known before any group is checked, so it may always be used;
  - a type whose definition depends on the type of a value, as `Cfg` in `Cfg = config`, only when the binding already used it, or when the group of the call depends on the group of that value (open question 2).

  A name of a binding that is inferred together with the binding of the call has the type it has at the call, not a general type.
- Each name in code from an argument means in the result what it meant in the check of §7.2: the same binding, or the same top-level, imported, or built-in name. That code was checked, and its syntax macros expanded, with those meanings. So a binder of the result may not take such a name: with `with_x(v, body)` giving `{ (= ~(make (Name "x") (: like v)) ~v) ~body }`, the call `with_x(2, x + 1)` inside `(x: Int) { … }` is a compile error, and not a quiet change of `x`. Only a binder made with `make` or taken from an argument can do this, because a binder from a template has the mark of the run. Syntax macros do not need this rule, because they are expanded before names are resolved.
- A name that is bound inside the code of an argument, by a parameter, a pattern, or a binding statement there, may only be used in the result inside that same binder, as it was in the argument. With `body_of(F): (f: F) -> Int`, a result that takes `(+ y 1)` out of the argument `(y) { y + 1 }` uses `y` with no binding around it, and that is a compile error at the call, not a local with no binding in the program.
- If one of these is not true, it is a compile error at the call. The message says that the macro is wrong, not the caller.

After all expanding, the order of top-level values (spec §2.5) and all later steps use the expanded program.

**Messages.** The errors and the warnings for the code of an argument are given from the check of §7.2, before expanding. So an argument that the macro drops is still checked, and `i check` gives the same errors as the language server, which checks by the signature (§9). When the constraints of the place where the result puts an argument are checked (above), an error that comes from them is given at the argument, with the call as a note; the argument itself is not checked again, so no error or warning of §7.2 is given two times. An error in other code from an argument, which is checked again (above), is given at its span, with the call as a note, and its warnings are not given again. An error in code from a template is reported at the call, with the template as a note. A warning in code from a template is not given.

A binding group that has type errors in the check of §7.2 is not expanded. Its typed macro calls are reported as not expanded, and `type_of` is never asked for in it.

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

- The scopes are set so that every name means the same type in every position where the macro may put it. A name of a built-in type or a built-in constructor has the scope of its own kind of §5, so a top-level or imported name of the module of the call with the same text, as its own `Result`, does not cover it. A fixed variable has the scope of its own kind (below). A type of a module, with no marks there, has the scope of that module with no marks, so rule 2 of §5 finds it there. A type that an expansion defined with marks has the scope it was defined with, its module and its marks, so rule 3 of §5 finds it. These `Syntax` values have the mark of the run only while the run lasts (§5).
- The type given is the type that the checker gave, with equal parts of a union or an intersection put together, and with each closed variable replaced as §7.3 says, by the direction of its places. A closed variable that is in the needed type is replaced as it is there, with its places counted in the parts of §7.3, so that `type_of` and `signature()` give one answer for one variable; any other closed variable is replaced by its places in the type that is given, with the top of that type an output place. No other variable is removed or replaced: the display form of spec §4.7, which shows a variable that is not generalized as the union of the types that come to it, is not used. Only fixed variables, and closed variables that are not replaced, are left, and both are given as below. A fixed variable is always given as a variable.
- A fixed variable is given as an `Up` node, as `A`, with a scope of its own kind that names that variable (§5). That scope keeps it apart from every type name and every type variable that a template or the caller declares, also one with the same text. One variable has the same node in every answer of one run. It may be written in the result with no declaration: the rule of syntax.md §4.7 that a type variable must be declared is not used for it. The rule of spec §4.4 about where a union or an intersection with a type variable may be written is used for it, with the extra limit of §7.3 for the top of a type in a pattern. It may not be in a type test of a clause (spec §7.1).
- A type with no type variable in it is given in full, with its aliases filled in.

| Function | What it gives |
|---|---|
| `type_of(s)` | The type that the checker gave to `s` at its position in the call. `s` must be a `Syntax` that the compiler gave to this run: an argument, an expression inside an argument, or a name that a pattern inside an argument binds. For any other `Syntax`, also one made by `make` or by a quote, and for a datum that is not an expression or such a name, it is a `SyntaxError`. |
| `signature()` | The needed type of the call (§7.3), as a function type. Closed variables are filled in, and fixed variables are given as in the list above. |
| `subtype(a, b)` | `Yes` when `a` is a subtype of `b` (spec §4.5) by the rules for fixed variables in §7.3, with no variable given a bound. The types are taken in the form that `type_of` gives. `No` when it is not, and neither type has a type variable in it. `NotKnown` in every other case. A `Syntax` that is not a correct type, or has `_` in it, is a `SyntaxError`. |
| `members(t)` | The members of the type (spec §4.4), in the order of their `show` text. `Never` has no members. `A & Int` and `A - U`, with a fixed variable `A`, are one member each. `Bool` is one member. After T1, a member of `Int` or `String` with a finite set of values gives one item for each value, a literal type, and a member with all values but a finite set is one item, as `String - "a"`; here only, an item is not a member of spec §4.4. So `members` of `"GET" \| "POST"` is `"GET"` and `"POST"`, and of `2 \| 10 \| Nil` it is `10`, `2`, `Nil`. The same `SyntaxError` as for `subtype`. |

- `type_of` gives the type at the position of the argument in the call. When the macro puts the argument in another position of its result, for example inside a clause, the argument keeps that type and its type variables (§7.3), and only the constraints of the new position are checked. After T1, the type may be a literal type, as `1` for `to_text(1)`, a `Syntax` whose node is `Int(1)`, and `subtype` and `members` take it.
- A type is a set of values, and one type may be written in more than one way: `Int | String` and `String | Int` are the same type. So a macro asks `subtype` and `members`, and does not match the `Syntax` of a type by its form.
- `definition` (§6) gives the constructors and fields of a named type, but not of a type whose definition depends on the type of a value (stream T2, open question 2).

### 7.5 What typed macros cannot do

- The result type of a call cannot depend on the result of the macro. A macro that reads a table of a database cannot give its rows a type that is not in the signature.
- A type variable of the signature that is only in its result, as in `default(A): () -> A`, has no type that comes to it, so the needed type that the macro is told has `Never` or a fixed variable in its place (decision 14). The macro is not told what the position of the call needs, so it can only guess. The result is still checked as §7.3 says: a guess is correct when the variable is closed and the guess fits its upper bounds, as `""` for `s: String = default()` inside a block, and is not correct when the variable is fixed, as for the same call in a top-level binding. So a typed macro cannot choose by the type that the position of the call needs, as in `user: User = decode(text)`. A syntax macro that takes the name of the type, `decode(User, text)`, does this work (open question 5). Syntax.md §6.1 reads such a call as an expression, also at the start of a statement.
- A typed macro cannot put an argument that has errors or needs an environment inside a function to be called later, because the signature has no way to say that the errors of the argument go to that function.
- A typed macro cannot choose by the type inside a binding with type variables.
- A typed macro cannot make items. Syntax macros do that.

## 8. Running macros

**Order of modules.** A program has no circles of imports (spec §2.1). A module is expanded and type checked after every module it imports. A macro is imported, so its module is done before the module that uses it. This is a new rule: today all modules of a program are compiled together.

**The macro program.** To run the macros of a module `D`, the compiler compiles `D` and all modules that `D` imports as one program (spec Appendix B.1), with the macros of `D` as its entry points. This is a new kind of entry point that only the compiler uses:

- `Syntax` values go between the compiler and the macro program in a form that the compiler defines. It is not seen in i.
- The entry point gets `Definitions`, `Types` for a typed macro, and an `Assert` whose `fail` stops the run. These are services of the compiler, not host services (spec §9.7). It gets no other service, so a macro cannot read a file, use the network, or read the time.
- It is a compile error when an `@external` function that is not of the library package `i` is in the full needs (spec §2.5, here also through other modules) of a macro, or of a top-level value of the macro program that is not a direct value. The functions of the library are given by the compiler, as the runtime gives them to a program.
- A macro program contains no executable `@js` binding, value getter, Js/JsRequests provider or JS initializer. Compute executable needs through the macro entries and all ordinary top-level initializers, including through other modules. If they need one, reject before instantiation, even when no macro entry needs Target. Unused runtime functions containing such declarations may be excluded from the macro program, but may not leave an initializer behind. A quoted declaration is syntax data and does not execute JS; the caller-origin checks of §4.3 still apply when it becomes an expansion. No file-reading/d.ts import tool, callback bridge or JS evaluator is part of the macro sandbox.
- The macro program is not for a target. It always runs in the compiler, whatever the program is compiled for.

**The same result everywhere.**

- The limits of §4.4 do not depend on the machine. A run does not stop for want of memory or stack before it is over a limit. For this, the units are also counted inside the functions of the runtime, and the count of depth follows frames that are left by an error, a continuation, or a tail call (§12).
- `hash` (spec §13) gives fixed numbers in a macro program.
- Each run starts from the same state: the compiler uses a new instance, or does something that a macro cannot tell from a new instance. The compiler may keep the result of a run and use it again when the macro program and everything the run was given are the same, also every answer of `Definitions` and `Types`. What is kept is the result before the mark of the run is changed (§5). A result that is used again is changed with the mark of the new run, so two calls with the same arguments, as two calls of `gen.helpers()` with no arguments, still get marked names that do not clash, whether the result is kept or not.

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
2. **Types of values and records (streams T2 and T3).** `docs/design/syntax.md` keeps a lowercase name in a type for "the type of a value" (T2), and `{ name: e }` for records (T3). This design makes choices in the same places, and T2 and T3 must agree with them or change them. These are the questions:
   - (a) Is `m(…)` or `a.m(…)` in a type a macro call, as here and as syntax.md keeps it, or does T2 take a lowercase name with arguments in a type for itself (§2.3, §4.3)?
   - (b) Is a `{ }` with one part in a type always a group, or may T3 use it for a record type with one field (§2.3, §3.1)?
   - (c) This design tells the item `Schema(User, _) = user`, the datum `(= (Schema User _) user)`, from an alias with parameters by the test of syntax.md §4.3 on the arguments, comparing text and scope (§2.3). By that test, a type pattern whose arguments are all new `UpName`s, as `Codec(User) = user_codec` with `Codec` imported, is read as an alias. Should T2 (i) keep this test, so that such a type pattern cannot be written; (ii) change syntax.md §4.3, and this text, to another test, as a lookup of the head; or (iii) give type patterns a datum and a form of their own?
   - (d) Is a qualified head, `s.Schema(User, _) = user`, a type pattern and not a binding, in both surfaces (§2.3): (i) yes, as this text says; or (ii) no, it stays a binding?
   - (e) Which `UpName`s in the argument places of a type pattern does it define: (i) all of them, so that `Schema(User, Json) = user` with `Json` imported is a compile error; or (ii) only one that does not name a type the module may use, so that `Json` there matches the imported type?
   - (f) Inside a nested type in an argument place, as `List` in `Schema(List(User), _) = users`: (i) the name of the nested type, `List`, is defined too; (ii) `List` must name a type that is already there, and only its arguments may be defined; or (iii) a nested type there is a compile error?
   - (g) Does `definition` give a `SyntaxError` for a type that depends on a value, as here (§6, §7.4), or something that T2 defines?
   - (h) In §7.3, the result of a typed macro may always use a type that does not depend on a value. It may use a type that depends on a value, as `Cfg` in `Cfg = config`, only when the binding already used it or the group of the call depends on the group of that value. Is this rule right, or does T2 give another one?
3. **The units and the numbers of the limits.** §4.4 has three limits for one run: on work, on memory, and on depth. The depth is counted in frame units (§12) with every answer. With every answer, a function of the library whose cost depends on the size of its input, as `concat` or `*` on big integers, counts units in proportion to that size (§12). There are three questions:
   - 3a. Which units count the work and the memory: (a) calls for the work and the size of the values made for the memory; or (b) one count of the steps of an abstract machine that the spec defines for the work, with the memory still counted by the size of the values made?
   - 3b. How many frame units does one frame count: (c) one, and one more for each parameter and each local name of the i function, which the spec can count; or (d) a number that the spec gives for each form of function in another way? The count may not depend on the Wasm locals that the compiler makes, because then it would depend on how the compiler optimizes (§4.4).
   - 3c. Are the numbers (e) fixed in the spec for every program, or (f) defaults that a package may raise up to a cap that the spec gives? With (f), the limits of a run are still the same on every machine (§8), because they come from the package and not from the machine.
4. **The order of work.** Not open any more; the number is kept so that the other numbers stay. The order is in §12.
5. **Choosing by the needed type.** By decision 14 and §7.5, a typed macro cannot choose by the type that its call needs. Should a typed macro be allowed to choose by the needed type when the type variable of its result is **closed** (§7.3), by taking that variable, when no type comes to it, to be the intersection of its upper bounds at the time of the expansion (`User` in `f = (text) { user: User = decode(text)  … }`, inside a block): (a) no, as decision 14 says; or (b) yes, for closed variables only? Fixed variables (§7.3) are not covered by either answer: later groups may still give them types, so taking them as their present types is not sound (the example of `f = compose(…)` in §7.3). So under (b) the same line as a top-level item, `user: User = decode(text)`, still cannot choose, because its variable is fixed (§7.5). Answer (b) needs a careful look at inference (spec §4.7). The text of decision 14 also has to be read again with this question: it says that such a variable "can only be met by `Never`", while §7.5 says that a guess that fits the upper bounds of a closed variable is correct. One of the two must change, and the decision is the user's.
6. **Changes to the core forms.** The tables of §2.3 are what every quote and quote pattern is written against, so a change of a core form breaks macros in packages. Should (a) the tables only change in the way the spec does, with no version, so that an old macro just breaks; or (b) a package say in its manifest which version of the tables its macros were written for, so that the compiler can refuse it with a clear message?
7. **Writing a macro.** A macro has no way to print while it runs; `i expand --trace` is the only help. Should `i/syntax` have a function, as `note(String)`, that adds a line to the compiler output, with the same result on every machine?

## 12. What changes

This is not a small addition. Today the compiler reads all modules, resolves them, orders them, and checks them as one program, and makes one Wasm module. Macros need the compiler to work module by module in the order of imports, and to compile and run macro programs in the middle of that. Typed macros need the checker to keep a type for each expression, to give types as written types, to check a piece of code in the middle of a binding, and to check without adding bounds. None of these is in `ilang-check` today.

The order of work is: after stream A1, the Lisp surface and the changing between surfaces (M1); then the driver and syntax macros with marks (M2); then `definition` (M3); then typed macros (M4). M2 has three parts done one after the other: the driver (M2a), syntax macros (M2b), and the tools (M2c).

The other streams of work named in this text are:

- **A0 to A3**: the syntax with no keywords. A0 is `docs/design/syntax.md`, A1 changes the parser and the formatter, A2 changes every line of i code in the repository to the new syntax, and A3 removes the old syntax (also from `query.rs` and `commands/package.rs`).
- **G0**: the point where the user approves `docs/design/syntax.md`, before A1 starts.
- **P2**: done. It made `MODULES` in `crates/ilang-package/src/library.rs` a slice, with a row and an empty file for each reserved library module (`i/task`, `i/clock`, `i/random`, `i/fs`, `i/process`, `i/net`, `i/stdio`).
- **B**: continuations that resume one frame at a time (`emit/control.rs`, `control.rs`, and the parts of `ir.rs` for frames).
- **C**: the host boundary, ABI version 3 (`ilang-host`, `emit/host.rs`, the entry kinds of spec Appendix B.4 to B.6).
- **R1 and R2**: R1 is the pure part of the library module `i/random`. R2 is the library module `i/task`, with the default scheduler at the entry point (`emit/host.rs` after C, and the entry code in `infer.rs` and `lower.rs`).
- **IO**: the library modules for clocks, files, the network, processes, and standard input and output (`ilang-host` services).
- **T1 to T3**: types. T1 gives literal types and value sets, T2 the type of a value in a type (`Schema(User, _) = user`), and T3 records. They own the tables of the checker in `types/` and `infer.rs`.
- **STR**: the new forms of string literals and string patterns (spec §1.6 and §7.1), after T3.

What changes, part by part:

- **`docs/design/syntax.md`.** The draft of syntax.md that this text cites (`lane/note-a0`, a88558c) has made most of the changes that this design needs, and keeps each form for macros as a compile error until the macro streams: the look past `name ( UpName, … )` to `=` or `:` (its §2.5 and §6.1); a `{` followed by `name: Type` and then `=`, `->`, `?`, or `|` as a block or a clause block (§6.1); the attribute with call arguments and a module name, and the `MacroItem` (§2.2); the `TypeCall` in a type (§3); a declaration `name [TypeParams] : Type` with no `=`, whatever its attributes, and the one with no `@external` kept for macros (§2.2, §2.5); and the test for a clause whose pattern is wrong, made only for a first statement that the earlier tests do not decide, so never for a binding or an environment statement (§6.1), which the printer of §1 depends on. This design also takes the test of syntax.md §4.3 that tells an alias with parameters from the form kept for alias patterns (§2.3), and needs no change to it. Changes to two sections are still needed, so that every file that §1 does not leave out can be changed into the other surface. None changes the rule of syntax.md §6.1 that a `{` whose first statement begins with a function with a `->` part whose parameters are all patterns, and that is not bound to a name, is a clause block (§1). They are:
  - §2.2: an attribute with a macro name and empty brackets, `@m()` or `@a.m()`, is a compile error, as `@main()` is; and an attribute before a `MacroItem` is a compile error (§2.3, §4.3).
  - §6.8 (and §3): a `->` part of a function that is `{ _ }`, a hole in a group, is a compile error, so that `{ _ }` and `_` are never two different things there (§1, §2.3).

  These are best made in syntax.md before it is approved (G0), so that A1 builds them in the parser. If they are not, M1 writes them into spec §2.2 and §6.8, together with the parser (below).
- **Spec.** §0 and §2.1 (file names, the library module `i/syntax`, the order of modules), §1.1, §1.5 (`` ` ``, `~`), §2.2 (attributes: `@macro`, a macro before an item, two items with one name; the item forms above; names compared by text and scope, §5), §2.3, §6.11, and §7.1 (names compared by text and scope, §5), §2.4 (exports checked after expanding; the new rule of §5 on types of other modules in exported items), §2.5 (a declaration with `@macro`; compile-time bindings), §3 (a macro call in a type), §4.3 and §4.7 (names compared by text and scope), §6.8 (`{ _ }` as the `->` part), §6.9 (`|>` with a macro), §10.2 and §10.3 (the macro entry point; the compiler services are not host services), §11.7 (stops in expanded code), §11.8 (the text form of a constructor with marks), §13 (`hash` in a macro program), Appendix B.6 (a new kind of entry), a new section for macros, and a new appendix for the Lisp surface. M1 writes §1.1, §1.5, §6.8, the sentences of §0 and §2.1 on `.is` files, on module paths, and on the error for a module with two files, the rules of §2.2 above when syntax.md does not have them, and the appendix, because M1 lands `.is` modules and that error; M2 to M4 write the rest. For §0 and §2.1, M1 is an exception to the rule below that the M stream is merged last: M1 must not wait for IO, because M1 is merged before T3 (the order for `ast.rs` below), and IO waits for C and the runtime work. So the order for §0 and §2.1 is M1 → IO → M2, and IO writes its sentences on top of those of M1. The plan of the redesign, which today has IO → M2 for these sections, must say this. The work list of M1, in the plan of the redesign that is kept outside this repository, must name these sections. Other streams change many of these sections too, and the order of merging must be set for each: §0 and §2.1 (IO), §2.2, §2.3, §2.4, and §6.11 (A1), §7.1 (A1, T1, STR), §2.5 and §3 (A1, T1, T2, T3), §4.3 and §4.7 (A1, T2), §6.9 (A1, T1), §10.3 (C, R2, IO, T3), §11.8 (T3), and Appendix B.5 and B.6 (C, R2, IO). In each of them, the M stream is merged last, but for the sentences of M1 in §0 and §2.1 (above), and the M tracks in the order M1 → M2 → M3 → M4.
- **`ilang-syntax`.** A parser and a printer for the Lisp surface, and datums (M1). M1 reads every row of §2.3 into the core tree but the rows whose AST nodes only M2 adds: a macro before an item, a macro call as an item, a macro call in a type, items from an expansion, and the quote and the quote pattern. M1 reads each of these datums in full and reports it as a compile error whose message says that the form is kept for macros, as the parser of the normal surface does for the same forms (syntax.md §1.4), and a quote datum as not yet supported. So until M2, a file with one of these forms cannot be compiled or changed into the other surface, in either surface; the work list of M1 in the plan of the redesign must limit its test of every row to the other rows, and test that these give the error. Quote and quote patterns, in the AST and in the reader of the Lisp surface, are part of M2b, as the plan says, because a new `ExprKind` or `PatternKind` needs changes to `resolve.rs`, `format.rs`, and `print.rs`, which M2 makes in their order of merging. The normal surface: A1 builds the parser of syntax.md, which already reads the look past `name ( … )`, the clause block that begins with `name: Type ->`, the attribute form, the `MacroItem`, and the `TypeCall`, each kept for macros as a compile error. The changes to syntax.md §2.2 and §6.8 above are also changes to the normal parser (in `parser.rs`, and maybe `format.rs` and `print.rs`). When G0 takes them into syntax.md, A1 builds them, and M1 does not touch the parser of the normal surface. When G0 does not, they are part of M1: M1 then changes those files of `crates/ilang-syntax/src/*` besides `ilang-syntax/src/lisp/`, the work list of M1 in the plan of the redesign must name them, and every file of `crates/ilang-syntax/src/*` that M1 changes is merged in the order below. Either way M1 lands only after A1, so that `i fmt --to=i` never writes a form that the parser of the day rejects. M2 turns the kept forms into macro calls, in the AST and in resolving. The order of merging for `ast.rs` and `lib.rs` of `crates/ilang-syntax`, and for any other file of `crates/ilang-syntax/src/*` that M1 changes, is A3 → M1 → T3 → STR → M2: M1 changes no AST of types, and M2, which changes the AST of `TypeCall` and of items, is merged after T2 and T3, which change the types.
- **A new driver** over the parser, `ilang-check`, `ilang-wasm`, and `ilang-host`: the order of modules, the two steps of §4.4, marks and the lookup of §5, putting off, the limits, and kept results. The driver is above `ilang-check`, so `ilang-check` cannot call it. Today every program is checked by a direct call of `ilang_check::check` or `query::check_for_queries`, which would not expand macros, so `i package` and `i build --wat` would report `@check.that` as an unknown name while `i build` works. Every such caller goes through the driver: `ilang-cli` (`main.rs`, `lib.rs`, `commands/package.rs` in `interface_of` and in `report`, `tests/wat_dump.rs`) and `ilang-lsp` (`analysis.rs`). `report` builds the permissions report of `i package` from the `@host` and `@external` items, and only step 1 of §4.4 can add, remove, or change items, so it needs the module after step 1, not a full expansion; it must not call `ilang_check::resolve_program` on the module before expanding. `check_for_queries` moves out of `ilang-check`, or takes the expansion as a callback. So `ilang-lsp` gains the driver, and with it `ilang-wasm`, `ilang-host`, and Wasmtime. Tests of `ilang-check` and `ilang-wasm` that check a program with no macros may still call the checker directly.
- **`ilang-check`.** The work named above, and resolving names with scopes. Choosing the parser by the end of the file name in `ilang_check::parse`, which needs no new field in `ModuleSource` (M1). Each expression needs an identity that is different in each expansion (a node id, or a span with a source and an expansion id), because two expansions of one template in one module give nodes with the same span; every table keyed by `(ModuleId, Span)` moves to it. These tables are in files of T1 and T3 (`types/mod.rs`, `infer.rs`), and T3 adds more of them, so this change is made only after T3 has landed, and M2 is in the order of merging of `infer.rs` and `types/mod.rs`. The rule that the environment of an entry point has only host services and `Assert` (`Checker::entry_points`, `entry_point`, `entry_services`, error E0522) must not be used for the macro entry point, which has `Definitions`, `Types`, and `Assert`; today entry points come only from the root package, and macros of other packages are entry points too. R2 changes this code, so M2 is merged after R2 in `infer.rs`. `Query::target_at` and `local_type` in `query.rs` find a node by its byte span in the HIR of a module; after expanding, that HIR has nodes whose spans are in the file of a template, so they must look only at nodes whose source is the file that is asked about. `Query::ast` parses the text of a module again with `ilang_syntax::parse_module`, the parser of the normal surface, whatever the name of the file, and `doc`, `walk_imports_exports`, the completion of imports, and `symbols` use it; so for a `.is` module it reads Lisp text as `.i`. M1 makes it choose the parser by the end of the file name, or reuse the AST that the check already has, because M1 lands `.is` modules in `i check` and the language server. After M2, `symbols` and `doc` use the items after expanding, so that the items that a macro before an item or a macro call as an item adds are listed. The needs and the full needs of spec §2.5 across modules, in `order.rs`: today `order::needs` only follows names inside one module, and both rules of §8, compile-time bindings and the rule on `@external` in the full needs of a macro, follow needs through other modules. `order.rs` is merged in the order A3 → T2 → M2. A module whose macro calls are not expanded is type checked without them, also when another module has errors (§9); today any error of resolution stops type checking for the whole program.
- **`ilang-wasm`.** The macro entry point, as a new kind of entry (spec Appendix B.6), and the form of `Syntax`. A `Syntax` value is a Wasm GC value of the runtime, so a quote builds it in Wasm, and `node`, `make`, and `same_code` are functions of the runtime. Three things in the lowering of today do not allow this, and M2 changes them in `lower.rs`, after T1 and T3 (the order of `lower.rs` in the plan of the redesign). First, the WAT of the runtime knows only the constructor ids 0 to 15 (`runtime/mod.rs`), but `node` and `make` build `Node` and `SyntaxError` values, whose ids are only known when a program is lowered; so `node` and `make` are emitted by the code generator for each program, with the ids of that program, and are not part of the fixed runtime text. Second, a type with no constructors lowers as `TypeRepr::Host`, and its type test looks at the tag of a host value; `Syntax` gets a `TypeRepr` and a tag of its own, so that a clause `(: s Syntax)` over `Syntax | Int` matches. Third, every `@external` of the package `i` goes through `external_op`, and one that it does not know lowers to `Op::Panic`; an `@external` of the library that the runtime does not give, as `show` and `show_as`, lowers instead to an import of the new kind of host function (below). Only `show`, `show_as`, the compiler services `Definitions` and `Types`, and the arguments and the result of a macro cross the boundary, and they cross it as an encoding in bytes that the program and the host both know. Only the part of the encoding for the tree of a `Syntax` value, which has no constructors, may be fixed runtime text. `Position`, `SyntaxError`, `Node`, and `Truth` values have the constructor ids of the program, so the code that encodes and decodes them at the boundary is emitted for each program with that program's ids, as `node` and `make` are. They cannot be normal host imports: `HostImports` in `ilang-host` takes and gives only `Value`, and `Node`, `Position`, and `SyntaxError` have no `AbiType`. So `ilang-host` gets a new kind of host function that passes this encoding (below). `Definitions`, `Types`, and `Assert` are services of the compiler, not `@host` services, so the rule of spec §10.3 on host types (E0320) is not used for them. Quotes; leaving out compile-time bindings. Code in function calls and in allocation that counts the units of the limits (§4.4); the counters are in generated code, because the host can only count Wasm instructions and bytes of stack. Much of the work and the memory of a run is in the functions of the runtime (`ilang-wasm/src/runtime/`), as `concat`, `bit_array_concat`, the `*` of big integers, and the functions on strings, and a call of one of them is one call. So each runtime function whose cost depends on the size of its input counts work and size units in proportion to that size, and checks the limits before it allocates, so that `concat([s, s])` with a very large `s` is over the limit, and not out of memory. The counters are put in only when a macro program is lowered, chosen by a field of `ilang_wasm::Options`, and the runtime is then emitted in a second, counting form chosen by the same field. A program for a target, and a program of `i test`, has no counters and the runtime of today. `runtime::wat()` today gives the whole runtime to every module, so it is given two more choices: the `Syntax` type and the functions `node`, `make`, and `same_code` are emitted only when the program uses `i/syntax`, and the form of the table of sites with two positions (below) only when the program has code from a template. This is not enough to keep the WAT of other programs as it is. `library::add` puts every row of `MODULES` into every graph, and `Lower::number_types` gives each type of `program.types` the id `FIRST_ID + i`, and each constructor a name, with the library as package 0. So a new library module with types, as `i/syntax` with `Syntax`, `Node`, `Position`, and `SyntaxError` and their constructors, would change the id of every type and constructor of every program, and the data of constructor names. So `number_types` numbers only the types of modules in the needs of the program, and the WAT of every program without macros, which uses no `i/syntax` and has no code from a template, does not change. Adding a type to any library file already changes these ids, also a file of a module that has a row in `MODULES` today: R1, R2, and IO fill in `i/random`, `i/task`, and the modules of IO, and R1 and IO are not in the order of `lower.rs` in the plan of the redesign. So this change to `lower.rs` is a small step of its own, with the WAT of every program unchanged, made before wave 2 and before the first stream that adds a type to a library file (R1, R2, IO, T3, or M2). The plan of the redesign must name it in the order of `lower.rs`, ahead of R1, R2, and IO. The limit on how deep calls go must stop a run before the Wasm stack is used up. The compiler cannot know the size of a native frame, which depends on Wasmtime, the CPU, and binaryen. So the depth limit counts frame units, not calls: each function counts a fixed number of units that the spec gives (open question 3). The count goes up when a function is entered and down when it returns, and it must follow the real depth also where frames do not return in the normal way. Errors unwind with `throw $error` caught by `try_table`, continuations are taken by throwing `$capture` and resumed by `return_call_ref`, and tail calls use `return_call`. The code of today already follows each of these for its shadow stack (`runtime/stops.rs`), which keeps the sites of the stop message: every `try_table` catch site saves `$shadow_depth` when it is entered and calls `$shadow_restore` when it catches (`emit/mod.rs`, `emit/control.rs`, and `$async_run` in `emit/host.rs`); a tail call takes the entry of its frame off with `$shadow_leave_tail` before it is made; and each frame that a resumed continuation puts back enters again with `$shadow_enter` (`emit/control.rs`). So the units of the depth are kept in the shadow stack, as a running sum stored in each entry next to its site, so that setting back the number of entries also sets back the sum. M2 changes only the push and the pop of `runtime/stops.rs` (`$shadow_enter`, `$shadow_enter_call`, `$shadow_leave`, `$shadow_leave_tail`), which add or take off the units and check the limit, and the code in `emit/mod.rs` that gives a function's units when it enters. M2 adds no code at the catch sites or in `emit/host.rs`, and B, which rewrites the code of continuations, keeps the calls of `$shadow_enter` and `$shadow_restore` as they are. If the count were kept in a global of its own, every catch site, also the one in `emit/host.rs`, would need new code; this design does not do that. B, T3, and M2 all change `emit/mod.rs`, so it has an order of merging of its own (below). The stack given to a run is sized from a bound for one unit that is measured for each target the compiler is built for, with a wide margin, and a test checks that bound on each target. If that cannot be made sure, deep frames are put on the heap after B. Stop sites (§9): today `Lower::site` finds the line and column in the source of the module that is lowered, and the table of sites in `emit/mod.rs` has one position for each site. A site takes its source from the identity of the expression, so that code from a template has the position in the template's file, and the table holds two positions, the call and the template, in a program that has code from a template (above). `runtime/stops.rs` prints both. These changes to the runtime, with the push and the pop in `runtime/stops.rs`, are part of M2. They are merged after B for `emit/control.rs`, after B and T3 for `emit/mod.rs`, and after T3, which owns the runtime (`runtime/`), and STR when it adds functions to the runtime.
- **`ilang-host`.** Running a macro program with only the services of the compiler, and reporting when it goes over a limit. A run has a `Config` of its own: today `Program::from_bytes` uses one fixed `max_wasm_stack` and thread stack for every program, and reads `ILANG_GC` and `ILANG_GC_HEAP` from the environment. For a macro run, the collector and the heap are fixed, the environment is not read, and the driver chooses the stack (above), so that a run does not stop for want of memory or stack on one machine only (§8). The new kind of host function that passes the encoding of `Syntax` (above) is here too, given by the driver and by the test runner of the CLI. These changes are merged in the order C → R2 → IO → M2.
- **`ilang-package`.** `.is` files as modules, in `modules.rs`: `walk` takes files that end in `.is` as well as `.i`. The error when a module has two files is the error that `collect` gives today for two files with one module path, `MODULE_TWICE` (E0624): `collect` takes the end off the file name, so `src/a.i` and `src/a.is` already give it once `walk` takes `.is`, and the sentence of spec §2.1 that M1 writes says no more than its message. The names of the files of the library, which today all end in `.i`. This part is M1, because M1 must compile `.is` files.
- **`ilang-package`, notes.** `Diagnostic` has one source and one span; it gets notes, each with a source, a span, and a message, for "the template as a note" (§4.4, §7.3, §9). `render_all` shows them, and the language server sends them as related information. This part is M2.
- **`ilang-cli`, `ilang-lsp`, editors.** `i fmt` for `.is` and `--to`; `i expand`; `i check` and the language server running macros; choosing the parser by the end of the file name, in place of taking only `.i` files; a grammar for `.is`; the package template; tests of the spec for the new sections. The spec tests (`crates/ilang-cli/tests/spec.rs`) and the WAT dump (`crates/ilang-cli/tests/wat_dump.rs`) take only `.i` files and copy each one to `test/{stem}.i`; they must take `.is` files too and keep the end of the name. This is part of M1, merged after A2's change to `tests/spec.rs`. For `i test`, `node`, `make`, and `same_code` are in the runtime, and `show` and `show_as` are host functions of the new kind (`ilang-host`, above) that the test runner of the CLI gives, because they need the printer of `ilang-syntax`, which `ilang-host` does not depend on (M2b). `definition` and the functions of `Types` are i code over the services (§3.2), so they are not imports.
- **Library.** The module `i/syntax`.

Some of this touches files that other streams own: the boundary, the entry kinds, and `emit/host.rs` (streams C and R2); the counters in `emit/control.rs` and `emit/mod.rs` (streams B and T3); the runtime (stream T3, and STR when it adds functions to it); `order.rs` (streams A3 and T2); `ilang-host` (streams C and IO); the entry points in `infer.rs` (stream R2); the per-expression tables (streams T1 to T3); and `query.rs` and `commands/package.rs` (streams A3 and T2). Each of these parts of M2 is merged after those streams. The rules for files that many streams change get more lines:

- `query.rs`: A3 → M1 → T2 → M2. M1 changes `Query::ast` (above), and the work list of M1 and this order in the plan of the redesign must say so.
- `commands/package.rs`: A3 → M2.
- `crates/ilang-package/src/library.rs`: A3 → M1 → T3 → M2, the same order as for `ast.rs`. It holds `MODULES`, the slice of the library modules, and makes their names as `<i>/{file}.i`. Since P2, `MODULES` already has a row for every reserved module, so R1, R2, and IO only fill in their files and do not change it. M1 makes the name of each file by its surface, T3 adds a row for `i/record`, and M2 adds a row for `i/syntax`.
- `crates/ilang-package/src/lib.rs`: A3 (`is_name`) → M2.
- `crates/ilang-package/src/modules.rs`: M1 (`walk` takes `.is` files; `collect` gives `MODULE_TWICE` for a module with two files), after A3 if A3 changes it.
- `order.rs`: A3 → T2 → M2.
- `crates/ilang-wasm/src/runtime/`, with `runtime/stops.rs`: T3 → STR → M2, and `emit/control.rs`: B → M2.
- `crates/ilang-wasm/src/emit/mod.rs`: B → T3 → M2, with C before M2 when C changes the code there that emits imports. The plan of the redesign must add this line.
- `crates/ilang-wasm/src/lower.rs`: the step for `number_types` before R1, R2, and IO (above); and M2 after T1 and T3, in the order that the plan gives for `lower.rs` (the `TypeRepr` of `Syntax`, the code at the boundary for each program, and the imports of `@external` functions of the library, above).

## References

- McCarthy, *Recursive Functions of Symbolic Expressions and Their Computation by Machine, Part I* (1960): M-expressions and S-expressions.
- Bawden, *Quasiquotation in Lisp* (1999).
- Kohlbecker, Friedman, Felleisen, Duba, *Hygienic Macro Expansion* (1986); Clinger, Rees, *Macros That Work* (1991); Dybvig, Hieb, Bruggeman, *Syntactic Abstraction in Scheme* (1992), for marks; Flatt, *Binding as Sets of Scopes* (2016).
- Flatt, *Composable and Compilable Macros: You Want It When?* (2002), for running macros in a separate step.
- Rafkind, Flatt, *Honu: Syntactic Extension for Algebraic Notation through Enforestation* (2012); Flatt and others, *Rhombus: A New Spin on Macros without All the Parentheses* (2023), for macros in a syntax that is not Lisp.
- Burmako, *Scala Macros: Let Our Powers Combine! On How Rich Syntax and Static Types Work with Metaprogramming* (2013), for macros in Scala. The names "blackbox" and "whitebox" seem to come later, from the documentation of Scala 2.11 (2014); this reference is likely wrong for them and must be checked.
- Languages: Reason and OCaml (two syntaxes for one tree); Elixir (a macro call has the form of a call); Swift macros (macros in a sandbox that see only syntax, and macros that add items next to a declaration); Rust procedural macros and the `watt` project (macros as Wasm); Template Haskell `reify` and Nim `getImpl` (asking for definitions).

The years, the titles, and what each work or language is said to do are from memory, here and wherever the body names another language or a paper (the decisions, §10, §11). Check all of them before this text goes into the spec.
