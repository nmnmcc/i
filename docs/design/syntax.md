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

- A clause block with one clause and no guard is a binding: when its pattern does not match, it reports to `Assert`. A clause block with more clauses must cover every value, or it is a compile error. This is the rule of parameters and bindings today, and the rule of `case` today.
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
Binding     = Pattern "=" Expr | name TypeParams "=" Expr .
Declaration = name [ TypeParams ] ":" Type .
TypeParams  = "(" UpName { "," UpName } [ "," ] ")" .
```

A binding whose pattern is one name may declare type variables after it (§4.7): `map(A, B) = …`. A declaration may declare them in the same way: `inspect(A): (A) -> String`. The rest of the section is unchanged.

### 3. Type syntax

```
Type      = Union .
Union     = Inter { "|" Inter } .
Inter     = Diff { "&" Diff } .
Diff      = Prim { "-" Prim } .
Prim      = FnType | TupleType | NamedType | "_" | "{" Type "}" .
FnType    = "(" [ ParamType { "," ParamType } [ "," ] ] ")" "->" Union [ "?" Union ] [ "!" Union ] .
ParamType = [ name ":" ] Type .
TupleType = "(" [ Type { "," Type } [ "," ] ] ")" .
NamedType = [ name "." ] UpName [ "(" Type { "," Type } [ "," ] ")" ] .
```

- An `UpName` with no module name before it is a type variable when a type variable with that name is declared where the type is written (§4.7), and a type when not. `_` is a hole that type inference fills in.
- A lowercase name is not a type. It is kept for the type of a value, and is a compile error for now.

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
- A type variable declared by a binding may be used in every type written in its value, also in the parameters, `->` parts, and clause patterns there, and in inner bindings. It is one type variable of that binding, and it is generalized with it. An inner binding may declare a type variable with the same name; inside it, the name means its own variable.
- An `UpName` in a type that is neither a type nor a declared type variable is a compile error. The message says how to declare it.
- A declared type variable may not have the name of a type that may be used in the module, because then a type would mean two things.
- A type variable may only be declared on a binding whose pattern is one name. A binding with another pattern, such as `(even, odd) = …`, uses the type variables of the bindings it is inside.

### 4.8 Typing rules

Rule 11 changes its name and its first sentence:

> 11. **Clauses.** For `e |> { … }` with `e : A`, and for a clause block called with a value of type `A` (§6.12), the clause patterns (each choice of `|` counts) are looked at by **place**. …

The rest of rule 11 is unchanged, with "the `case`" read as "the clause block". The known-type bullets change in the same way:

- "A `case` has the union of the known types of its clause expressions …" becomes "A match `e |> { … }` has the union of the known types of its clause expressions when they all have one."
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
Statement = Pattern "=" Expr | name TypeParams "=" Expr | "!" NamedType "=" Expr | Expr .
Clauses   = "{" { Clause } "}" .
Clause    = Pattern { "|" Pattern } [ "?" Expr ] "->" Expr .
Function  = "(" [ Param { "," Param } [ "," ] ] ")" [ "->" Union [ "?" Union ] [ "!" Union ] ] Block .
Param     = Pattern | name "!" NamedType .
RecordUpdate = Name "(" ".." Expr { "," name ":" Expr } [ "," ] ")" .
```

- `( … )` that is not a call is a function when `{` or `->` comes after it, and a tuple when not (§1.7). So its parts are patterns in a function and expressions in a tuple. In a guard, a `->` that is not inside brackets always ends the clause head. So in `n ? (a) -> 1`, the guard is the tuple `(a)`, which is a compile error because a guard must be a `Bool`, and not a function.
- A `{` that begins an expression begins a **clause block** when it is `{}`, or when the first thing in it is a pattern followed by `|`, `?`, or `->`. When not, it begins a block. So `{ x }` is a block and `{ x -> x }` is a clause block. A block whose first statement is a function with a `->` part, such as `{ (x) -> Int { x } … }`, is read as a clause block, so such a function is put in a group or bound to a name first.
- A `{` followed by a label `name :` and then an expression that is not a binding is kept for records. It is a compile error for now.
- A statement that starts with a pattern and then `=` is a binding. When more than one such pattern may be read, the longest is taken: in `a = (x, y) = e`, the pattern is `a = (x, y)`. A statement that starts with `name (` and an `UpName` is a binding with type parameters. A statement that starts with `!` is an environment statement. Every other statement is an expression.

### 6.9 Operators

Two notes are added after the table:

- `|>` is weaker than `+` and stronger than `<>`, comparisons, `&&`, and `||`. So a subject made with those operators is put in a group: `{ a == b } |> { True -> …  False -> … }`.
- `e |> { … }`, where the right side is a clause block, is a match (§6.12).

### 6.10 Blocks and bindings

Two sentences change:

- In the list of expressions that give no "not used" warning, "a `case`" becomes "a match".
- After "`p = e` is a binding.", add: "A binding whose pattern is one name may declare type variables after it, as in `twice(A) = (f: (A) -> A, x: A) { f(f(x)) }` (§4.7)."

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

`e |> { clauses }` is a **match**. It works out `e` and calls the clause block with it, as every `a |> f` is `f(a)` (§6.9). A match is not a new kind of call: it is how a clause block is used most often, and the typing rules give it the type of the clause expressions (§4.8 rule 11).

- A clause block with one clause and no guard is the same as a function with one parameter: `{ p -> e }` is `(p) { e }`. When `p` may fail, it reports to `Assert`, as a parameter does (§6.11). So `opt |> { Some(x) -> x }` is the same as the binding `Some(x) = opt` followed by `x`.
- The clauses of any other clause block must cover every value of the type of its parameter (§7.4). If not, it is a compile error.
- `{}` is a clause block with no clauses. Its parameter has the type `Never`, so `x |> {}` is correct only when `x` has no values.
- The type of a match is a type of every clause expression, for example their union. A clause block has the function type from the type of its parameter to that type.
- A clause that can never be used gives a compile warning.
- A clause block is a direct value (§6.10), as every function is.

### 7.1 Grammar (changed sentences)

- "In a `case` clause, `p: (Int, Int) -> e` is read as a function type …" becomes: "In a clause, `p: (Int, Int) -> e` is read as a function type, so a tuple type there is put in `{ }`, or a tuple pattern is used. In the same way, a `?` after a function type is part of the type, so a function type before a guard is put in `{ }`: `f: { (Int) -> Int } ? ok -> …`."
- "In a `case`, `x: T` tests the value." becomes: "In a clause block with more than one clause, `x: T` tests the value. In a clause block with one clause and no guard, it is a parameter, so it is a type annotation."
- "so in a `case`, `T` may not have a type variable as a member" becomes "so in a test, `T` may not have a type variable as a member".

### 7.3, 7.4, 7.5

"`case`" becomes "clause block" in each, and "the clauses of a `case` on a value of type `A`" becomes "the clauses of a clause block whose parameter has the type `A`". In §7.5, "When a `case` clause pattern `C(…)` matches the value of a name `v`" becomes "When a clause pattern `C(…)` of the match `v |> { … }` matches the value of the name `v`".

## Notes for A1 and A2

- **How the front end maps the new syntax.** The new syntax maps onto the existing tree:
  - `e |> { clauses }` becomes the existing `case` node;
  - a clause block that is not the right side of `|>` becomes `(x) { x |> { … } }` with a fresh `x`;
  - one clause with no guard follows the binding rule above.
  
  The first rule keeps the known types, narrowing, and §7.5 known constructors on the name `e` exactly as they are today.
- **Showing type schemes.** Shown type schemes use capital letters: `id : (x: A) -> A`. Error variables are `E`, `E2`, … and environment variables are `N`, `N2`, …, as the lowercase forms do today.
- **Renaming type variables in the formatter's migration.**
  - Turn a lowercase variable into an `UpName` (`a` becomes `A`, `key` becomes `Key`, `e2` becomes `E2`), and declare it on the binding that owned it under the old §4.7 rule.
  - When the new name is a type that may be used in the module, such as `result` becoming `Result`, choose another name and report it.
- **What A2 changes.** Every `case`, `if` guard, `import (` and `export (` in the spec, the library, `std`, the tests, the README, the `i new` templates, and the editor grammar changes in stream A2. So does every lowercase type variable in prose, such as `List(a)` in §4.2 and `Boundary(r, e, n)` in §9.4.
