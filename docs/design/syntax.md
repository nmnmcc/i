# Design: syntax with no keywords

Status: draft for review (stream A0 of the redesign plan). When it is accepted, stream A1 puts the text below into `docs/spec.md`, together with the parser that accepts it, and stream A2 changes every example in the spec, the library, and the tests. Until then the spec keeps the old syntax, because every `i` block in it must parse.

## Summary

- i has no keywords. Every word is a `name` or an `UpName`.
- The imports and the exports are groups in `( )` at the start of a file, with no word before them. A group with a module path in it is the imports. A group with only names is the exports.
- `case` is gone. A **clause block** `{ p1 -> e1  p2 -> e2 }` is a function of one parameter, and `e |> { … }` matches `e`.
- A guard is written with `?`: `n ? n > 0 -> "positive"`.
- A type variable is an `UpName`, and it is declared after the name of its binding: `map(A, B) = …`. Type parameters of type definitions are `UpName`s too.
- A lowercase name in a type is kept for "the type of a value" (stream T2). Until then it is a compile error.
- `{ name: e, … }` is kept for records (stream T3).

Decisions taken with the user:

- A clause block with one clause and no guard is a binding: when its pattern does not match, it reports to `Assert`. A clause block with more clauses must cover every value, or it is a compile error. This is the rule of parameters and bindings today. It is not the rule of a `case` with one clause today, which must cover its value and reads `x: T` as a test; the notes for A2 say how the tests that use this change.
- A guard uses `?`.
- The imports and exports are told apart by what is in them, not by where they are.

## Replacement text for `docs/spec.md`

Each part below takes the place of the section with the same number. Sections not named here keep their text, but their examples and their type variables change to the new syntax in stream A2 (for example `List(a)` becomes `List(A)` and `{ Yield(a) | n }` becomes `{ Yield(A) | N }`).

### 1.3 Names

```
name    = [ "_" ] lower { lower | digit | "_" } .
discard = "_" | "__" { lower | digit | "_" } .
UpName  = upper { upper | lower | digit } .
lower   = "a" … "z" .   upper = "A" … "Z" .   digit = "0" … "9" .
hex     = digit | "a" … "f" | "A" … "F" .   oct = "0" … "7" .   bin = "0" | "1" .
char    = (* any Unicode scalar value but '"' and "\" *) .
```

- A `name` is a value, a parameter, a label, or a part of a module path. A name may start with one `_`, as in `_count`. It is a normal name in every way, but a compiler does not warn when it is not used.
- A `discard` is `_` alone, or a word that starts with two `_`, as in `__old_value`. It is a place in a pattern that binds no name; the word after `__` only says what is left out.
- An `UpName` is a type, a type variable (§4.7), or a constructor.

### 1.4 Keywords

i has no keywords. Every word is a `name` or an `UpName`, and any `name` may be used as a name. The forms that other languages make with keywords are made with brackets and symbols: imports and exports are groups at the start of a file (§2.2), matching is a call of a clause block (§6.12), and a guard is written with `?`.

### 1.7 Lines

A line end is a **break** when both sides of it allow one:

- the last token before it **can end** an expression: a name, an `UpName`, a discard, a literal, `)`, `]`, `}`, or `>>`; and
- the first token after it **can begin** a statement: a name, an `UpName`, a discard, a literal, `(`, `[`, `{`, `<<`, `-`, `-.`, `..`, `!`, or `@`.

Comments are not tokens, so the tokens on the two sides of a comment are what count. Every line end that is not a break is only a space.

A break ends something. What it ends depends on the brackets it is directly inside:

- Directly inside `{ }` (a block, a clause block, or the constructors of a type), and at the top level of a file, a break ends a statement, a clause, a constructor, or an item. Attributes are part of the item after them, so a break after an attribute does not end the item.
- Directly inside a `{ }` that groups a type (§3), a break is a compile error.
- Directly inside `( )`, `[ ]`, or `<< >>`, a break is the same as a `,`. A `,` at the end of a line may be left out.

With no break, as on one line, a statement or a clause is the longest expression that may be read from its start, and a `,` is needed between parts. So statements and clauses may share a line: `b |> { True -> 1  False -> 0 }`.

But a clause or a statement whose pattern starts with `(`, `-`, or `-.` begins a new line, because the expression before it on the same line goes on with it. In `p |> { (0, y) -> y  (x, _) -> x }`, `y(x, _)` is read as a call, and in `n |> { 0 -> a  -1 -> b }`, `a -1` is read as `a - 1`. In the same way, `{ a = f  (b, c) = g … }` reads `a = f(b, c)`. In each case, what is left is a compile error.

From this rule:

- When `(` is on the same line as the expression before it, it is a call (§6.6). When `(` is the first thing on a line, it begins a new statement or part.
- The `{` of a function body (§6.8) is on the same line as the `)` or the type before it.
- A line that ends in an operator, `,`, `=`, `:`, `->`, `(`, `[`, `{`, or `<<` goes on in the next line. A line that begins with `|>`, `.`, `|`, `?`, `->`, or a binary operator other than `-` and `-.` goes on from the line before.
- `a` and then `- b` on the next line are two statements. To write `a - b` on two lines, put `-` at the end of the first line. In the same way, `! N` of a function type may not begin a line.

One rule does not depend on lines: in the `->` part of a function, after the `)` of its parameters, a `{` right after `->`, `?`, `!`, `|`, `&`, or `-` is part of the type (§3). The first other `{` that is not inside brackets begins the body.

### 2.2 Form of a file

```
File      = [ Imports ] [ Exports ] { Item } .
Imports   = "(" ImportItem { "," ImportItem } [ "," ] ")" .
Exports   = "(" ExportItem { "," ExportItem } [ "," ] ")" .
Item      = { Attribute } ( TypeDef | Binding | Declaration ) .
Attribute = "@" name [ "(" string_lit { "," string_lit } [ "," ] ")" ] .
```

At the start of a file, a group in `( )` that is not followed by `=` is not an item:

- When an item in it has a module path (§2.3), which always has a `/`, the group is the **imports**.
- When not, it is the **exports**: every item in it is a `name` or an `UpName`, with or without `(..)`.

A file has at most one imports group and at most one exports group. They come first, the imports before the exports. Neither may be empty: a file with nothing to import or export leaves the group out. A group in `( )` that is followed by `=` is a binding with a tuple pattern, as in `(even, odd) = …`. Any other group in `( )` at the top level is a compile error.

An item with `@external` is a declaration (§10.1). Any other item that starts with an `UpName` is a type definition (§4.3). Every other item is a binding (§2.5).

In one module, every top-level name is defined one time only. Two bindings may not bind the same name, two type definitions may not define the same type, and two constructors may not have the same name. A constructor and a top-level binding may not have the same name either, because they are in one space of names (§2.3).

The attributes are unchanged.

### 2.3 Imports

```
ImportItem = [ name "=" ] [ "(" ImportName { "," ImportName } [ "," ] ")" "=" ] ModulePath .
ImportName = name | UpName .
ModulePath = name "/" name { "/" name } .
```

The imports and the exports are each a tuple of items. As in every tuple, the items have `,` or a break (§1.7) between them:

```i
(
  i/fail
  decode = json/decode
  opt = (Option, Some) = std/option
)

(main, User(..))
```

The rest of the section is unchanged.

### 2.4 Exports

```
ExportItem = name | UpName [ "(" ".." ")" ] .
```

- A top-level name that is not in the exports may only be used in its module.

The other rules are unchanged, with "the `export` list" read as "the exports".

### 2.5 Top-level bindings

```
Binding     = Pattern "=" Expr | name TypeParams [ ":" Type ] "=" Expr .
Declaration = name [ TypeParams ] ":" Type .
TypeParams  = "(" UpName { "," UpName } [ "," ] ")" .
```

A binding whose pattern is one name, with or without a type, may declare type variables after the name (§4.7): `map(A, B) = …`, `id(A): (A) -> A = (x) { x }`. A declaration may declare them in the same way: `inspect(A): (A) -> String`. The rest of the section is unchanged.

### 3. Type syntax

```
Type      = Union .
Union     = Inter { "|" Inter } .
Inter     = Diff { "&" Diff } .
Diff      = Prim { "-" Prim } .
Prim      = FnType | TupleType | NamedType | name | "_" | "{" Type "}" .
FnType    = "(" [ ParamType { "," ParamType } [ "," ] ] ")" "->" Union [ "?" Union ] [ "!" Union ] .
ParamType = [ name ":" ] Type .
TupleType = "(" [ Type { "," Type } [ "," ] ] ")" .
NamedType = [ name "." ] UpName [ "(" Type { "," Type } [ "," ] ")" ] .
```

- An `UpName` with no module name before it is a type variable when a type variable with that name is declared where the type is written (§4.7), and a type when not. `_` is a hole that type inference fills in.
- A lowercase `name` in a type is kept for a later use, the type of a value, and is a compile error. It is in the grammar, so a type with it is read in full, and the error says what the name is kept for.

The other rules are unchanged, with their examples written with type variables in capitals: `{ (A) -> B } | C`, `(A) -> { (B) -> C } ! E`.

### 4.3 Custom types

```
TypeDef     = UpName [ TypeParams ] TypeBody .
TypeBody    = "{" { Constructor } "}" | "=" Type .
Constructor = UpName [ "(" Field { "," Field } [ "," ] ")" ] .
Field       = [ name ":" ] Type .
```

- `T { C1 C2 … }` is a custom type. Every constructor has zero or more fields. A field may have a label.
- `T {}` is a type with no constructors. i code may not make its values. They only come from the host (§10).
- `T = Type` is a type alias. `T` is the same as the type on the right. An alias may not use itself, directly or through other aliases.
- The type parameters are the type variables of the definition: `Option(A) { Some(A) None }`. Every type parameter must be used on the right, and every type variable on the right must be a parameter.
- A type parameter may not have the name of a type that may be used in the module.
- Custom types are **nominal**: two different definitions make two different types, even if they have the same form.

### 4.7 Type inference

The bullet "The annotations of a binding …" is replaced by these bullets. The other bullets are unchanged.

- A type variable is **declared** in one of three places: after the name of a binding (§2.5, §6.10), `map(A, B) = …`; after the name of a declaration, `inspect(A): (A) -> String`; or after the name of a type definition (§4.3).
- A type variable declared by a binding may be used in the type on its pattern, as in `id(A): (A) -> A = …`, and in every type written in its value, also in the parameters, `->` parts, and clause patterns there, and in inner bindings. It is one type variable of that binding, and it is generalized with it. An inner binding may declare a type variable with the same name; inside it, the name means its own variable.
- An `UpName` in a type that is neither a type nor a declared type variable is a compile error. The message says how to declare it.
- A declared type variable may not have the name of a type that may be used in the module, because then a type would mean two things.
- A type variable may only be declared on a binding whose pattern is one name, `x`, or one name with a type, `x: T`. A binding with another pattern, such as `(even, odd) = …`, uses the type variables of the bindings it is inside. A top-level binding with another pattern is inside no binding, so it may not use type variables; its values are written as bindings of one name each when they need them.

### 4.8 Typing rules

Rule 11 becomes:

> 11. **Clauses.** This rule is for a clause block with more than one clause, or with a guard (§6.12). Let it be called with a value of type `A`: in a match `e |> { … }` with `e : A`, or as a function whose parameter has the type `A`. The clause patterns (each choice of `|` counts) are looked at by **place**. The top place is the value of the parameter. Below a pattern at a place, its parts are at places of their own: each field of a constructor `C`, each part of a tuple of n parts, and the parts and the rest of a list. Places below different constructors are different places. At each place, let `B` be the type of the values there (`A` at the top place, and else the part type of the pattern type above it), and let `P` be the union of the pattern types (below) of the patterns at that place that are not catch-alls. Patterns at one place with the same type constructor use the same new type variables, so `Ok(v)` and `Error(e)` at one place give one `Result(b1, b2)`. A type test `x: T` at a place also needs `B <: T | r - T`, with a new type variable `r`: it takes every value whose type constructor is that of a member of `T` (§7.1), so those values must be in `T`. So `v |> { x: Box(Int) -> …  y: Box(String) -> … }` with `v: Box(Int | String)` is a compile error. A constructor pattern with `..` counts as having a catch-all at the places of the fields that it leaves out. If some clause without a guard has a catch-all at that place or at a place above it, then `B <: P | r - P`, with a new type variable `r`: a value that no pattern takes may still come there. If not, then `B <: P`. A catch-all at a place gets the type of rule 12. Every guard and every clause expression has the errors `E` and the environment `N`, every guard has the type `Bool`, and every clause expression has the type `R`. Then the match `e |> { … }` has the type `R`, and the clause block has the type `(A) -> R ? E ! N`, whose parameter has no label. Making the clause block needs nothing, as for a function in rule 4. Coverage (§7.4) is checked with the types that inference finds.
>
> A clause block with one clause and no guard is not typed by this rule. It is the function `(p) { e }` of rule 4: its pattern is fitted to the type of the parameter (below), it is not checked for coverage, and `S` is `Assert` when the pattern may fail. A match with such a clause block is a call of it (rule 5).

The fitting paragraph after the rules becomes:

> To **fit** a pattern `q` to a type `B`, in a binding, a parameter, or a clause block with one clause and no guard: if `q` is a catch-all, its names get `B`. If not, `B <: Q`, where `Q` is the pattern type of `q`, and the parts of `q` are fitted to the parts of `Q` given below. Fitting never adds `Assert`. In a clause block with more than one clause or with a guard, rules 11 and 12 take the place of fitting.
>
> Only a clause block with one clause may have choices `q1 | … | qn` where a pattern is fitted. If one choice is a catch-all, the names get `B`. If not, `B <: Q1 | … | Qn`, where `Qi` is the pattern type of `qi`, and choices with the same type constructor use the same new type variables. The parts of each `qi` are fitted to the parts of its `Qi`, and a name gets the union of its types in the choices (§7.2). The choices may fail for `B` when together they do not match every value of `B` (§7.3); then `S` in rule 4 is `Assert`.

The known-type bullets change in the same way:

- "A `case` has the union of the known types of its clause expressions …" becomes "A match `e |> { … }` has the union of the known types of its clause expressions when they all have one."
- A bullet is added after the bullet for a function expression: "A clause block that is not the right side of a match has a known function type with one parameter, made as for a function expression: the label of its parameter (§6.12); for the parameter, the type written at the top of its pattern when the block has one clause, no guard, and no choices, or else the type it gets from an expected type, or else `_`; and for the result, the union of the known types of its clause expressions when they all have one, or else `_`. In the bullets on expected types, such a clause block counts as a function expression of one parameter."
- "every clause expression of a `case` has the expected type of the `case`" becomes "every clause expression of a match has the expected type of the match".
- The example after the rules becomes: "So `(u) { u.name }` needs a type, `(u: User) { u.name }`, but with `opt: Option(User)`, `opt |> { Some(u) -> u.name  None -> "" }` is correct."

### 5. Names and scope

The fourth bullet becomes: "The names of a parameter may be used in the function body. A name in a pattern of a clause may be used in its guard and its expression." A new bullet is added:

- A declared type variable (§4.7) may be used in the types written in the value of its binding.

### 6.1 Grammar

```
Expr      = Binary .
Binary    = Unary { BinOp Unary } .
BinOp     = "||" | "&&" | "==" | "!=" | "<" | "<=" | ">" | ">=" | "<." | "<=." | ">." | ">=."
          | "<>" | "|>" | "+" | "-" | "+." | "-." | "*" | "/" | "%" | "*." | "/." .
Unary     = [ "-" | "-." ] Postfix .
Postfix   = Primary { Call | "." name | "." decimal } .
Call      = "(" [ Arg { "," Arg } [ "," ] ] ")" .
Arg       = [ name ":" ] Expr .
Primary   = int_lit | float_lit | string_lit | Name | Tuple | List | BitArray
          | Block | Clauses | Function | RecordUpdate .
Name      = [ name "." ] ( name | UpName ) .
Tuple     = "(" [ Expr { "," Expr } [ "," ] ] ")" .
List      = "[" [ Expr { "," Expr } [ "," ".." Expr ] [ "," ] ] "]" .
Block     = "{" { Statement } Expr "}" .
Statement = Pattern "=" Expr | name TypeParams [ ":" Type ] "=" Expr | "!" NamedType "=" Expr | Expr .
Clauses   = "{" { Clause } "}" .
Clause    = Pattern { "|" Pattern } [ "?" Expr ] "->" Expr .
Function  = "(" [ Param { "," Param } [ "," ] ] ")" [ "->" Union [ "?" Union ] [ "!" Union ] ] Block .
Param     = Pattern | name "!" NamedType .
RecordUpdate = Name "(" ".." Expr { "," name ":" Expr } [ "," ] ")" .
```

- `( … )` that is not a call is a function when `{` or `->` comes after it, and a tuple when not (§1.7). So its parts are patterns in a function and expressions in a tuple. In a guard, a `->` that is not inside brackets always ends the clause head. So in `n ? (a) -> 1`, the guard is the tuple `(a)`, which is a compile error because a guard must be a `Bool`, and not a function.
- A `{` that begins an expression begins a **clause block** when it is `{}`, or when its first statement is not a binding (next bullet) and the first thing in it is a pattern followed by `|`, `?`, or `->`. When not, it begins a block. So `{ x }` is a block and `{ x -> x }` is a clause block.
- Whether the first statement is a binding is decided before any test for `->`. A pattern may have `=` in it (§7.1), so `go = (n) -> Int { n + 1 }` could also be read as a clause with the pattern `go = (n)` and the expression `Int`. It is read as a binding of a function when the part after `->` is a type (§3) followed by a `{` on the same line, because a clause expression cannot go on with `{`. When not, it is a clause. So `{ go = (n) -> Int { n + 1 }  go(1) }` is a block, and `{ t = (0, _) -> t  _ -> 0 }` is a clause block.
- A block whose first statement is a function with a `->` part that is not bound to a name, such as `{ (x) -> Int { x } … }`, is read as a clause block, with the pattern `(x)` and the expression `Int`. Such a function is bound to a name first: `g = (x) -> Int { x }`. A group does not help, because `{ (x) -> Int { x } }` is read in the same way.
- A `{` followed by `name :` begins a clause block or a block when the pattern `name: Type` is followed by `=`, `->`, `?`, or `|`, as in `{ x: Int -> x }` and `{ x: Int = f()  x }`. When not, as in `{ name: e, … }`, it is kept for records, and is a compile error.
- A statement that starts with a pattern and then `=` is a binding. When more than one such pattern may be read, the longest is taken: in `a = (x, y) = e`, the pattern is `a = (x, y)`. A statement that starts with `name ( UpName { , UpName } )` and then `=` or `:` is a binding with type parameters; the parser looks past the `)` to find the `=` or `:`. So `id(A) = …` and `id(A): (A) -> A = …` are bindings, and `send(Stop)` is an expression. A statement that starts with `!` is an environment statement. Every other statement is an expression.

### 6.9 Operators

"The right side of `|>` must be a name, a call, or a function." becomes "The right side of `|>` must be a name, a call, a function, or a clause block."

Three notes are added after the table:

- `|>` is weaker than `+` and stronger than `<>`, comparisons, `&&`, and `||`. So a subject made with those operators is put in a group: `{ a == b } |> { True -> …  False -> … }`.
- `e |> { … }`, where the right side is a clause block, is a match (§6.12).
- A match is an operation at level 6, so an operator of level 7 or 8, `.f`, `.k`, or a call after the clause block is part of the right side: `n |> { 0 -> 1  _ -> 2 } * 10` is `n |> ({ … } * 10)`, and `opt |> { … }.name` is `opt |> ({ … }.name)`. In the same way, `1 + n |> { … }` is `(1 + n) |> { … }`, and `-n |> { … }` is `(-n) |> { … }`. So a match that is an operand of such an operator or of `-` or `-.`, or that has `.f`, `.k`, or a call after it, is put in a group: `{ n |> { 0 -> 1  _ -> 2 } } * 10`, `1 + { n |> { … } }`, `-{ n |> { … } }`, `{ opt |> { … } }.name`.

### 6.10 Blocks and bindings

These sentences change:

- In the list of expressions that give no "not used" warning, "a `case`" becomes "a match".
- After "`p = e` is a binding.", add: "A binding whose pattern is one name, with or without a type, may declare type variables after the name, as in `twice(A) = (f: (A) -> A, x: A) { f(f(x)) }` and `id(A): (A) -> A = (x) { x }` (§4.7)."
- "`(a, b) = v` with `v: (Int, Int) | Nil` is a compile error; `case` is used for this" becomes "…; a match is used for this".
- "the binding works like a `case` on `e` with two clauses: `p`, which goes on with the rest of the block, and a catch-all, which calls `a.fail(AssertionFailure(m))`" becomes "when it runs, the binding works like the match `e |> { p -> …  _ -> a.fail(AssertionFailure(m)) }`, where the first clause goes on with the rest of the block". The binding is still typed by §4.8 rule 10, not by rule 11.
- In the list of direct values, "a function expression;" becomes "a function expression or a clause block;".

The example becomes:

```i
fact = (n) {
  n |> {
    0 -> 1
    _ -> n * fact(n - 1)
  }
}

(even, odd) = ((n) { n == 0 || odd(n - 1) }, (n) { n != 0 && even(n - 1) })
```


### 6.12 Clause blocks and matching

A clause block `{ clauses }` is a function of one parameter. When it is called with a value, it takes the first clause whose pattern matches the value and whose guard, if there is one, is `True`, and gives the value of the expression of that clause. A guard is any expression of type `Bool`. It is worked out only after its pattern has matched, and it may use the names of the pattern. When the pattern has choices `p1 | p2`, the choices are tried in order: the clause is taken with the first choice that matches and whose guard is `True`.

`e |> { clauses }` is a **match**. It works out `e` and calls the clause block with it, as every `a |> f` is `f(a)` (§6.9). A match is not a new kind of call: it is how a clause block is used most often. A match adds no call frame: the clauses run in the place of the match, so a call in a clause expression of a match in tail position is in tail position (§11.6), and the match is not one of the calls in a stop message (§11.7).

- A clause block with one clause and no guard is the same as a function with one parameter: `{ p -> e }` is `(p) { e }`. Its pattern is fitted to the type of the parameter (§4.8 rule 4), it is not checked for coverage, and `x: T` in it is a type annotation. When `p` may fail, it reports to `Assert`, as a parameter does (§6.11). So `opt |> { Some(x) -> x }` is the same as the binding `Some(x) = opt` followed by `x`.
- When that one clause has choices, as in `{ Ok(v) | Error(v) -> v }`, the same rule holds, and the choices are fitted together (§4.8). There is no function `(p1 | p2) { e }` to write it as, because a parameter has no choices.
- Any other clause block, with more than one clause or with a guard, is typed by §4.8 rule 11, and its clauses must cover every value of the type of its parameter (§7.4). If not, it is a compile error.
- `{}` is a clause block with no clauses. Its parameter has the type `Never`, so `x |> {}` is correct only when `x` has no values.
- The type of a match is a type of every clause expression, for example their union. A clause block has the function type from the type of its parameter to that type.
- The parameter of a clause block with one clause and no guard has the label of its pattern, as in `(p) { e }` (§6.11): `{ n -> n }` has the label `n`, and `{ Some(u) -> u }` has none. The parameter of any other clause block has no label.
- A clause that can never be used gives a compile warning.
- A clause block is a direct value (§6.10), as every function is.

### 7.1 Grammar (changed sentences)

- "In a `case` clause, `p: (Int, Int) -> e` is read as a function type …" becomes: "In a clause, `p: (Int, Int) -> e` is read as a function type, so a tuple type there is put in `{ }`, or a tuple pattern is used. In the same way, a `?` after a function type is part of the type, so a function type before a guard is put in `{ }`: `f: { (Int) -> Int } ? ok -> …`."
- "In a `case`, `x: T` tests the value." becomes: "In a clause block with more than one clause or with a guard, `x: T` tests the value. In a clause block with one clause and no guard, it is a parameter, so it is a type annotation."
- "so in a `case`, `T` may not have a type variable as a member" becomes "so in a test, `T` may not have a type variable as a member".

### 7.3, 7.4, 7.5

"`case`" becomes "clause block" in each, and "the clauses of a `case` on a value of type `A`" becomes "the clauses of a clause block whose parameter has the type `A`". §7.4 is about clause blocks with more than one clause or with a guard; a clause block with one clause and no guard is not checked for coverage (§6.12). In §7.5, "When a `case` clause pattern `C(…)` matches the value of a name `v`" becomes "When a clause pattern `C(…)` of the match `v |> { … }` matches the value of the name `v`".

### 9.3 Errors

"then use `case` with type tests" becomes "then use a match with type tests".

### 10.2 Entry points

"They must be in the `export` list of their module." becomes "They must be in the exports of their module."

### 11.6 Tail calls

"Tail positions are: …" becomes: "Tail positions are: the last expression of a function body; the expression of a clause of a clause block that is called as a function, and not in a match, because that expression is the last thing the function works out; the expression of a clause of a match in tail position; the last expression of a block in tail position; and the right side of `&&` or `||` in tail position." So in `loop = { 0 -> Done  n -> loop(n - 1) }`, the call `loop(n - 1)` uses no more stack. A match adds no call frame (§6.12).

## Notes for A1 and A2

- **How the front end maps the new syntax.** The new syntax maps onto the existing tree:
  - `e |> { clauses }` becomes the existing `case` node;
  - a clause block that is not the right side of `|>` becomes `(x) { x |> { … } }` with a fresh `x`, whose parameter has no label (a `Param::Pattern` with the label `None`), so the fresh name is never seen; the one exception is a block with one clause, no guard, and no choices, `{ p -> e }`, which becomes `(p) { e }` and has the label of `p`;
  - a `case` node with one clause and no guard is marked, so that the checker fits its pattern (with its choices) as a binding, adds `Assert` when it may fail, and does not check coverage, and the lowering adds the catch-all that calls `a.fail`.

  The first rule keeps the known types, narrowing, §7.5 known constructors on the name `e`, tail positions, and the order of evaluation exactly as they are today.
- **Showing type schemes.** Shown type schemes use capital letters: `id : (x: A) -> A`. Error variables are `E`, `E2`, … and environment variables are `N`, `N2`, …, as the lowercase forms do today.
- **Renaming type variables in the formatter's migration.**
  - Turn a lowercase variable into an `UpName`: take out each `_` and make the letter after it a capital, and make the first letter a capital. So `a` becomes `A`, `key` becomes `Key`, `e2` becomes `E2`, `key_type` becomes `KeyType`, and `_a` becomes `A`.
  - Declare it on the binding that owned it under the old §4.7 rule. When that binding has a type on its pattern, the declaration goes after the name and before the `:`: `id: (a) -> a = (x) { x }` becomes `id(A): (A) -> A = (x) { x }`. The labels stay as they are, because the annotation is not moved.
  - When the new name is a type that may be used in the module, such as `result` becoming `Result`, or is the same as another new name of the same binding, as `ab` and `a_b` both becoming `AB`, choose another name and report it.
  - When the binding that owned it has a pattern that is not one name, as in `(wrap, unwrap) = (…, …)`: inside another binding, declare the variable on the nearest binding of one name around it, and report it, because the local binding is then less general. At the top level, split the binding into one binding of one name for each part, when its value is a tuple with one part for each name, and declare the variables on each. When it cannot be split, report it, and leave it for the user to change.
- **Matches in operator expressions.** The formatter's migration puts in a group (§6.9) every old `case` that is an operand of an operator of level 7 or 8 or of `-` or `-.`, or that has `.f`, `.k`, or a call after it. So `1 + case n { … }` becomes `1 + { n |> { … } }`, and `case x { _ -> 1 }.y` becomes `{ x |> { _ -> 1 } }.y`. It also puts each clause or statement whose pattern starts with `(`, `-`, or `-.` on a new line (§1.7).
- **Bare functions in prose.** An example function in prose that uses type variables and is not bound, such as `(x: a) -> a { x }` in §7.3, has no binding to declare them on. A2 writes it as the value of a binding that declares them: `id(A) = (x: A) -> A { x }`.
- **Tests whose meaning changes.** A `case` with one clause and no guard used to have to cover its value, and read `x: T` as a test. After the change, the same clause block is fitted, reports to `Assert`, and reads `x: T` as an annotation. So the checker tests that expect the old errors are rewritten, and their `.out` files change:
  - Tests that expect a "not covered" error from one clause, such as `case r { Ok(x) -> x }` in `crates/ilang-check/tests/types/coverage.i`, get a second clause that still does not cover the rest, or are moved to tests of the `Assert` behavior.
  - Tests that expect "type variable in a type test" from one clause, such as `bad = (v) { case v { x: a -> x } }` in `crates/ilang-check/tests/types/case.i`, get a second clause, so that `x: A` is still a test.
  - Tests of one clause with choices, such as those in `crates/ilang-check/tests/types/known_choices.i`, keep their meaning only where fitting the choices gives the same types as rule 11; their `.out` files are checked again.
- **What A2 changes.** Every `case`, `if` guard, `import (` and `export (` in the spec, the library, `std`, the tests, the README, the `i new` templates, and the editor grammar changes in stream A2. So does every lowercase type variable in prose, such as `List(a)` in §4.2 and `Boundary(r, e, n)` in §9.4.
