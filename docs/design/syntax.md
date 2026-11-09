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
- The forms of macros in `docs/design/meta.md` are kept: `@m` and `@m(args)` before an item, a top-level item that is a call, and a call `m(…)` in a type. Until the macro streams, each is a compile error with a message.

Decisions taken with the user:

- A clause block with one clause and no guard is a binding: when its pattern does not match, it reports to `Assert`. A clause block with more clauses must cover every value, or it is a compile error. This is the rule of parameters and bindings today. It is not the rule of a `case` with one clause today, which must cover its value and reads `x: T` as a test; the notes for A2 say how the tests that use this change.
- A guard uses `?`.
- The imports and exports are told apart by what is in them, not by where they are.

## Replacement text for `docs/spec.md`

Each part below has one of two marks after its heading. The mark is not part of the heading.

- **(full text)**: the part takes the place of the whole section with the same number. Its heading is the new heading of the section.
- **(edits)**: the part names sentences, bullets, paragraphs, or the grammar of the section, and says how each changes or what is added. The rest of the section keeps its text. Only text in quotes, text in a code block, and the bullets or paragraphs given after "becomes:" or "is added:" go into the spec. The other sentences of such a part are notes for A1.

Sections not named here keep their text. In stream A2, their examples change to the new syntax, and so do the type variables in i code in them: in `i` blocks, in written types and type definitions, and in the types of library names. For example, `Result(a, e) { Ok(a) Error(e) }` in §4.2 becomes `Result(A, E) { Ok(A) Error(E) }`, and the types in the table of §9.4 change in the same way. The letters of the notation of §0 in the rules of §4, such as `a - U` in §4.4 and `∀a1 … an. T` in §4.8, are not i code and do not change.

In §4, only the `i` blocks and the table of §4.2 are i code, so only they change, as in `List(A)` and `Result(A, E) { Ok(A) Error(E) }`. Every other type in §4 keeps its lowercase type variables as notation of §0, also when it is written in i syntax. So these do not change: `(x: a & Int) -> a` and `{ Yield(a) | n } - Yield(a)` in §4.4, `List(a)` and `Continuation(a, r, e, n)` in §4.5 rule 7, and `(x: a | Nil)` in §4.8. The type `Continuation(A, R, E, N)` in the table of §9.4 is i code, and it changes; by §0, the two ways of writing it say the same thing.

### 0. How to read this document (edits)

A paragraph is added after the paragraph on EBNF:

> Inside a type in the rules and in the prose, a capital letter, alone or with a digit, such as `A`, `T`, `E`, `N`, or `P1`, stands for any type, and a lowercase letter, alone or with a digit, such as `a`, `r`, or `b1`, stands for a type variable. Outside a type, the letters keep the sense that the text gives them: in §4.8, for example, `C` is a constructor, `e` an expression, and `r` an expression or a name. This is notation of this document, not i syntax: in i code, a type variable is an `UpName` that is declared (§4.7), as in `id(A): (A) -> A`.

### 1.3 Names (full text)

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

### 1.4 Keywords (full text)

i has no keywords. Every word is a `name` or an `UpName`, and any `name` may be used as a name. The forms that other languages make with keywords are made with brackets and symbols: imports and exports are groups at the start of a file (§2.2), matching is a call of a clause block (§6.12), and a guard is written with `?`.

Some forms are kept for macros, which are not yet part of i: an attribute that is not built in, such as `@m` or `@m(args)` before an item (§2.2); a top-level item that is a call, `m(args)` or `a.m(args)` (§2.2); and, in a type, a lowercase name or `a.m` followed by `(` (§3). The parser reads each of them in full, and each is a compile error whose message says that the form is kept for macros.

### 1.7 Lines (full text)

A line end is a **break** when both sides of it allow one:

- the last token before it **can end** an expression: a name, an `UpName`, a discard, a literal, `)`, `]`, `}`, or `>>`; and
- the first token after it **can begin** a statement: a name, an `UpName`, a discard, a literal, `(`, `[`, `{`, `<<`, `-`, `-.`, `..`, `!`, or `@`.

Comments are not tokens, so the tokens on the two sides of a comment are what count. Every line end that is not a break is only a space.

A break ends something. What it ends depends on the brackets it is directly inside:

- Directly inside `{ }` (a block, a clause block, or the constructors of a type), and at the top level of a file, a break ends a statement, a clause, a constructor, or an item. Attributes are part of the item after them, so a break after an attribute does not end the item.
- Directly inside a `{ }` that groups a type (§3), a break is a compile error.
- Directly inside `( )`, `[ ]`, or `<< >>`, a break is the same as a `,`. A `,` at the end of a line may be left out.

With no break, as on one line, a statement or a clause is the longest expression that may be read from its start, and a `,` is needed between parts. So statements and clauses may share a line: `b |> { True -> 1  False -> 0 }`.

But a clause or a statement whose pattern starts with `(`, `-`, or `-.` begins a new line, because the expression before it on the same line goes on with it. In `p |> { (0, y) -> y  (x, _) -> x }`, `y(x, _)` is read as a call, and in `n |> { 0 -> a  -1 -> b }`, `a -1` is read as `a - 1`; in each case, what is left is a compile error. In the same way, `{ a = f  (b, c) = g … }` is the one binding `a = f(b, c) = g` (§6.1), which is a compile error because `f` is not a constructor. With a constructor, the one binding may be correct and mean something else: `{ mk = Pair  (b, c) = g … }` is the binding `mk = Pair(b, c) = g`, which takes `g` apart, and it does not bind `mk` to `Pair`.

From this rule:

- When `(` is on the same line as the expression before it, it is a call (§6.6). When `(` is the first thing on a line, it begins a new statement or part.
- The `{` of a function body (§6.8) is on the same line as the `)` or the type before it.
- A line that ends in an operator, `,`, `=`, `:`, `->`, `(`, `[`, `{`, or `<<` goes on in the next line. A line that begins with `|>`, `.`, `|`, `?`, `->`, or a binary operator other than `-` and `-.` goes on from the line before.
- `a` and then `- b` on the next line are two statements. To write `a - b` on two lines, put `-` at the end of the first line. In the same way, `! N` of a function type may not begin a line.

One rule does not depend on lines: in the `->` part of a function, after the `)` of its parameters, a `{` right after `->`, `?`, `!`, `|`, `&`, or `-` is part of the type (§3). The first other `{` that is not inside brackets begins the body.

### 2.2 Form of a file (edits)

The grammar becomes:

```
File      = [ Imports ] [ Exports ] { Item } .
Imports   = "(" ImportItem { "," ImportItem } [ "," ] ")" .
Exports   = "(" ExportItem { "," ExportItem } [ "," ] ")" .
Item      = { Attribute } ( TypeDef | Binding | Declaration | MacroItem ) .
Attribute = "@" [ name "." ] name [ Call ] .
MacroItem = [ name "." ] name Call .
```

A paragraph is added after the grammar:

> `Call` is the call of §6.1, so the arguments of an attribute and of a `MacroItem` are read as expressions.

The first paragraph, "A file has at most one `import` and at most one `export`. …", becomes:

> At the start of a file, a group in `( )` that is not followed by `=` is not an item:
>
> - When an item in it has a module path (§2.3), which always has a `/`, the group is the **imports**.
> - When not, it is the **exports**: every item in it is a `name` or an `UpName`, with or without `(..)`.
>
> A file has at most one imports group and at most one exports group. They come first, the imports before the exports. Neither may be empty: a file with nothing to import or export leaves the group out. A group in `( )` that is followed by `=` is a binding with a tuple pattern, as in `(even, odd) = …`. Any other group in `( )` at the top level is a compile error.
>
> An item with `@external` is a declaration (§10.1). Any other item that starts with an `UpName` is a type definition (§4.3). An item that is a call, `m(args)` or `a.m(args)`, with no `=` or `:` after its `)`, is a `MacroItem`. Every other item is a binding (§2.5).
>
> A `MacroItem` is kept for macros. It is a compile error, and the message says that a call at the top level is kept for macros.

The paragraph "In one module, every top-level name is defined one time only. …" keeps its text. The paragraph "These are the attributes. …" becomes:

> These are the attributes. An attribute whose name is one of them, with no module name before it, is that attribute. Its arguments must be string literals, given by position, with no labels, and an attribute that takes no arguments is written with no `( )`, as in `@main`. Any other argument, such as `@external("env" <> "", "f")`, `@external(module: "env", "f")`, or `@main()`, is a compile error. Any other attribute, such as `@m`, `@m(args)`, or `@a.m(args)`, is a macro before an item. It is kept for macros, and it is a compile error whose message says so. An attribute in another place, or one attribute given two times on one item, is a compile error.

The list of the attributes keeps its text.

### 2.3 Imports (edits)

The heading "2.3 import" becomes "2.3 Imports". The grammar becomes:

```
ImportItem = [ name "=" ] [ "(" ImportName { "," ImportName } [ "," ] ")" "=" ] ModulePath .
ImportName = name | UpName .
ModulePath = name "/" name { "/" name } .
```

The paragraph "`import` and `export` are each followed by a tuple of items. …" and its example become:

> The imports and the exports are each a tuple of items. As in every tuple, the items have `,` or a break (§1.7) between them:
>
> ```i
> (
>   i/fail
>   decode = json/decode
>   opt = (Option, Some) = std/option
> )
>
> (main, User(..))
> ```

The rest of the section keeps its text.

### 2.4 Exports (edits)

The heading "2.4 export" becomes "2.4 Exports". The grammar becomes:

```
ExportItem = name | UpName [ "(" ".." ")" ] .
```

The first bullet becomes: "A top-level name that is not in the exports may only be used in its module." The other bullets keep their text.

### 2.5 Top-level bindings (edits)

The grammar becomes:

```
Binding     = Pattern "=" Expr | name TypeParams [ ":" Type ] "=" Expr .
Declaration = name [ TypeParams ] ":" Type .
TypeParams  = "(" UpName { "," UpName } [ "," ] ")" .
```

A paragraph is added after the grammar:

> A binding whose pattern is one name, with or without a type, may declare type variables after the name (§4.7): `map(A, B) = …`, `id(A): (A) -> A = (x) { x }`. A declaration may declare them in the same way: `inspect(A): (A) -> String`.
>
> An item that starts with `name ( UpName { , UpName } )` and then `=` or `:` is always a binding or a declaration with type parameters. It is never a binding with a constructor pattern, though `f(A)` could also be read as one (§7.1). So `wrap(A) = (x: A) { Some(x) }` declares `A`. The same is true for a statement of a block (§6.1).
>
> An item that is a call, `m(args)` or `a.m(args)`, and has no `=` or `:` after its `)`, is not a binding. It is kept for macros (§2.2).

The rest of the section keeps its text.

### 3. Type syntax (edits)

In the grammar, the rule for `Prim` changes, and two rules are added after `NamedType`. The other rules keep their text.

```
Prim      = FnType | TupleType | NamedType | TypeCall | name | "_" | "{" Type "}" .
TypeCall  = [ name "." ] name "(" [ TypeArg { "," TypeArg } [ "," ] ] ")" .
TypeArg   = Type | int_lit | string_lit .
```

The bullet "A `name` is a type variable. `_` is a hole that type inference fills in." becomes these bullets:

> - An `UpName` with no module name before it is a type variable when a type variable with that name is declared where the type is written (§4.7, §5), and a type when not. `_` is a hole that type inference fills in.
> - A type variable takes no arguments: `A(Int)`, where `A` is a declared type variable, is a compile error.
> - A lowercase `name` in a type with no `(` after it is kept for a later use, the type of a value, and is a compile error. It is in the grammar, so a type with it is read in full, and the error says what the name is kept for.
> - A lowercase `name`, or `a.m` with a module name `a`, followed by `(` is a `TypeCall`. It is kept for macros, and is a compile error. It is in the grammar, so a type with it is read in full, and the error says that it is kept for macros.

In the bullet on `{ }`, the example `(a) -> { (b) -> c } ! E` becomes `(A) -> { (B) -> C } ! E`. The other bullets keep their text.

### 4.3 Custom types (full text)

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
- The type parameters of one definition must have different names. A type parameter may not have the name of a type that may be used in the module.
- Custom types are **nominal**: two different definitions make two different types, even if they have the same form.

### 4.7 Type inference (edits)

The bullet "The **annotations of a binding** are …" is replaced by these bullets. The other bullets keep their text.

> - A type variable is **declared** in one of three places: after the name of a binding (§2.5, §6.10), `map(A, B) = …`; after the name of a declaration, `inspect(A): (A) -> String`; or after the name of a type definition (§4.3).
> - The type variables declared in one place must have different names. So `pair(A, A) = …` is a compile error.
> - A type variable declared by a binding may be used in the type on its pattern, as in `id(A): (A) -> A = …`, and in every type written in its value, also in the parameters, `->` parts, and clause patterns there, and in inner bindings. It is one type variable of that binding, and it is generalized with it. An inner binding may declare a type variable with the same name; inside it, the name means its own variable.
> - A type variable declared by a declaration may be used in the type of the declaration. It is general: each use of the declared name may give it a different type.
> - An `UpName` in a type that is neither a type nor a declared type variable is a compile error. The message says how to declare it.
> - A declared type variable may not have the name of a type that may be used in the module, because then a type would mean two things.
> - A type variable may only be declared on a binding whose pattern is one name, `x`, or one name with a type, `x: T`. A binding with another pattern, such as `(even, odd) = …`, uses the type variables of the bindings it is inside. A top-level binding with another pattern is inside no binding, so it may not use type variables; its values are written as bindings of one name each when they need them.

### 4.8 Typing rules (edits)

Rule 11 becomes:

> 11. **Clauses.** This rule is for a clause block with no clauses, with more than one clause, or with a guard (§6.12). Let it be called with a value of type `A`: in a match `e |> { … }` with `e : A`, or as a function whose parameter has the type `A`. The clause patterns (each choice of `|` counts) are looked at by **place**. The top place is the value of the parameter. Below a pattern at a place, its parts are at places of their own: each field of a constructor `C`, each part of a tuple of n parts, and the parts and the rest of a list. Places below different constructors are different places. At each place, let `B` be the type of the values there (`A` at the top place, and else the part type of the pattern type above it), and let `P` be the union of the pattern types (below) of the patterns at that place that are not catch-alls. Patterns at one place with the same type constructor use the same new type variables, so `Ok(v)` and `Error(e)` at one place give one `Result(b1, b2)`. A type test `x: T` at a place also needs `B <: T | r - T`, with a new type variable `r`: it takes every value whose type constructor is that of a member of `T` (§7.1), so those values must be in `T`. So `v |> { x: Box(Int) -> …  y: Box(String) -> … }` with `v: Box(Int | String)` is a compile error. A constructor pattern with `..` counts as having a catch-all at the places of the fields that it leaves out. If some clause without a guard has a catch-all at that place or at a place above it, then `B <: P | r - P`, with a new type variable `r`: a value that no pattern takes may still come there. If not, then `B <: P`. With no clauses, `P` is `Never` at the top place, so `A <: Never`. A catch-all at a place gets the type of rule 12. Every guard and every clause expression has the errors `E` and the environment `N`, every guard has the type `Bool`, and every clause expression has the type `R`. Then the match `e |> { … }` has the type `R`, and the clause block has the type `(A) -> R ? E ! N`, whose parameter has no label. Making the clause block needs nothing, as for a function in rule 4. Coverage (§7.4) is checked with the types that inference finds.
>
> A clause block with one clause and no guard is not typed by this rule. It is the function `(p) { e }` of rule 4: its pattern is fitted to the type of the parameter (below), it is not checked for coverage, and `S` is `Assert` when the pattern may fail. A match with such a clause block is a call of it (rule 5).

The paragraph on fitting after the rules becomes:

> To **fit** a pattern `q` to a type `B`, in a binding, a parameter, or a clause block with one clause and no guard: if `q` is a catch-all, its names get `B`. If not, `B <: Q`, where `Q` is the pattern type of `q`, and the parts of `q` are fitted to the parts of `Q` given below. Fitting never adds `Assert`. In a clause block with no clauses, with more than one clause, or with a guard, rules 11 and 12 take the place of fitting.
>
> Only a clause block with one clause may have choices `q1 | … | qn` where a pattern is fitted. If one choice is a catch-all, the names get `B`. If not, `B <: Q1 | … | Qn`, where `Qi` is the pattern type of `qi`, and choices with the same type constructor use the same new type variables. The parts of each `qi` are fitted to the parts of its `Qi`, and a name gets the union of its types in the choices (§7.2). In choices, `x: T` is a test, not a type annotation (§7.1), so `T` may not have a type variable as a member. The choices may fail for `B` when together they do not match every value of `B` (§7.3); then `S` in rule 4 is `Assert`.

The known-type bullets change in this way:

- "A `case` has the union of the known types of its clause expressions when they all have one." becomes "A match `e |> { … }` has the union of the known types of its clause expressions when they all have one."
- A bullet is added after the bullet for a function expression: "A clause block that is not the right side of a match has a known function type with one parameter, made as for a function expression: the label of its parameter (§6.12); for the parameter, `Never` when the block has no clauses, the type written at the top of its pattern when the block has one clause, no guard, and no choices, or else the type it gets from an expected type, or else `_`; and for the result, the union of the known types of its clause expressions when they all have one, or else `_`. When the parameter has a known type `B`, from a written type or from an expected type, every clause pattern is a pattern fitted to a value with the known type `B`, as in a match (next bullet), and the narrowing by earlier clauses of §4.8 rule 12 is used in the same way. So with `opts: List(Option(User))`, `list.map(opts, { Some(u) -> u.name  None -> "" })` is correct, as `list.map(opts, (o) { o |> { Some(u) -> u.name  None -> "" } })` is. In the bullets on expected types, such a clause block counts as a function expression of one parameter."
- A bullet is added before the bullet "**Names in patterns**": "In a match `e |> { … }`, every clause pattern is a pattern fitted to a value with the known type of `e` (below), also when the clause block has one clause and no guard. So with `opt: Option(User)`, `opt |> { Some(u) -> u.name }` is correct, as `Some(u) = opt` followed by `u.name` is."
- "every clause expression of a `case` has the expected type of the `case`" becomes "every clause expression of a match has the expected type of the match".
- The sentence after the bullets becomes: "So `(u) { u.name }` needs a type, `(u: User) { u.name }`, but with `opt: Option(User)`, `opt |> { Some(u) -> u.name  None -> "" }` is correct."

### 5. Names and scope (edits)

In the third bullet, the sentence "A name in a pattern of a `case` clause may be used in its guard and its expression." becomes "A name in a pattern of a clause may be used in its guard and its expression." The other sentences of that bullet keep their text.

A bullet is added after the third bullet:

> - A type variable declared on a binding (§4.7) may be used in the type on its pattern and in every type written in its value. A type variable declared on a declaration may be used in the type of the declaration. A type parameter of a type definition may be used in that definition (§4.3).

### 6.1 Grammar (full text)

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
- A `{` that begins an expression begins a **clause block** when it is `{}`, or when its first statement is a clause (next bullet). When not, it begins a block. So `{ x }` is a block and `{ x -> x }` is a clause block.
- To find whether the first statement is a clause, these tests are made in order. The first that holds decides.
  1. When the statement starts with `name ( UpName { , UpName } )` and then `=` or `:`, it is a binding with type parameters (last bullet), and not a clause, though `f(A)` could also be read as a constructor pattern (§7.1). So `id(A) = (x: A) -> A { x }` is a binding, and not a clause with the pattern `id(A) = (x: A)`.
  2. The longest pattern `p` at the start of the statement is read. When `p` is followed by `|`, `?`, or `->`, the statement is a clause, with one exception. The exception is a binding of a function: `p` is `n1 = … = nk = ( … )`, one or more names, each followed by `=`, and then a tuple pattern; after the `->` that follows `p`, the `->` part of a function, `Union [ "?" Union ] [ "!" Union ]` (§6.8), may be read in full; and a `{` comes after it on the same line. Then the statement is the binding of the pattern `n1 = … = nk` to the function that starts at `( … )`.
  3. When the statement starts with a pattern and then `=`, it is a binding (last bullet), and not a clause. When it starts with `!`, it is an environment statement, and not a clause. So a binding of a function needs no exception when the name has a type, as in `f: (Int) -> Int = (n) -> Int { n }`, because its longest pattern, `f: (Int) -> Int`, is followed by `=`. Nor does a function with an environment parameter, as in `go = (n, log ! Log) -> Int { … }`, because `log ! Log` is not a pattern, so the longest pattern is `go`.
  4. When the statement is not a correct expression, or its longest expression is followed by `->`, and a `->` that is not inside brackets comes before the next break or the `}`, the statement is a clause whose pattern is wrong. Then the `{` begins a clause block, and the error is the first error found when the start of the statement, up to that `->`, is read as a pattern followed by `->`. So `x |> { -y -> 1 }` and `b |> { <<x: Int>> -> 1 }` give an error in the pattern, and not a block with a stray `->`. This test is only made for a statement that the tests before it do not decide, so a `->` in a later statement on the same line, as in `{ k = 2  g = (x) -> Int { x * k }  g(1) }`, does not count.
  5. When none of these holds, the statement is an expression, and not a clause.

  So these are blocks:
  - `{ go = (n) -> Int { n + 1 }  go(1) }`
  - `{ go = (n) -> Int ? String ! Log { step(n) }  go(1) }`
  - `{ k: (Int) -> Int = f  k(1) }`
  - `{ go(A) = (x: A) -> A { x }  go(1) }`
  - `{ go = (n, log ! Log) -> Int { log.write(n)  n }  go(1) }`
  - `{ k = 2  g = (x) -> Int { x * k }  g(1) }`

  and these are clause blocks:
  - `{ t = (0, _) -> t  _ -> 0 }`
  - `{ r = Ok(v) -> v  _ -> 0 }`
  - `{ s = "ab" <> rest -> s  _ -> "" }`
- So in a first clause whose pattern is `name = ( … )`, a clause expression that is a function, such as `() { … }` or `(x) -> Int { x }`, would be read as the `->` part of a function and its body. There, such a function is put in a group: `{ p = (a, b) -> { () { log(p) } } }` is a clause block whose one clause gives a function with no parameters. The test above does not take `{ () { log(p) } }` as a type, because a type in `{ }` ends at its `}`, and a function in it has a `{` before that. Later clauses, and first clauses with other patterns, need no group: `hs |> { (name, _) -> (x) { log(name, x) }  _ -> (x) { x } }`.
- A block whose first statement is a function with a `->` part that is not bound to a name, such as `{ (x) -> Int { x } … }`, is read as a clause block, with the pattern `(x)` and the expression `Int`. Such a function is bound to a name first: `g = (x) -> Int { x }`. A group does not help, because `{ (x) -> Int { x } }` is read in the same way.
- A `{` followed by `name :` begins a clause block or a block when the pattern `name: Type` is followed by `=`, `->`, `?`, or `|`, as in `{ x: Int -> x }` and `{ x: Int = f()  x }`. When not, as in `{ name: e, … }`, it is kept for records, and is a compile error.
- In a block, a statement that starts with a pattern and then `=` is a binding. When more than one such pattern may be read, the longest is taken: in `a = (x, y) = e`, the pattern is `a = (x, y)`. A statement that starts with `name ( UpName { , UpName } )` and then `=` or `:` is a binding with type parameters; the parser looks past the `)` to find the `=` or `:`. Such a statement is never read as a binding with a constructor pattern, though `f(A)` could also be read as one (§7.1), and this is the same at the top level (§2.5). So `id(A) = …` and `id(A): (A) -> A = …` are bindings, `wrap(A) = (x: A) { Some(x) }` declares `A`, and `send(Stop)` is an expression. A statement that starts with `!` is an environment statement. Every other statement is an expression.

### 6.9 Operators (edits)

"The right side of `|>` must be a name, a call, or a function." becomes "The right side of `|>` must be a name, a call, a function, or a clause block."

Three bullets are added after the bullet "`a |> f` is `f(a)`, …":

> - `|>` is weaker than `+` and stronger than the operators of levels 1 to 5: `||`, `&&`, `==`, `!=`, comparisons, and `<>`. So the left side of `|>` that is made with those operators is put in a group: `{ a == b } |> { True -> …  False -> … }`.
> - `e |> { … }`, where the right side is a clause block, is a match (§6.12).
> - A match is an operation at level 6, so an operator of level 7 or 8, `.f`, `.k`, or a call after the clause block is part of the right side: in `n |> { 0 -> 1  _ -> 2 } * 10`, the right side is `{ 0 -> 1  _ -> 2 } * 10`, and in `opt |> { … }.name`, it is `{ … }.name`. In the same way, the left side of `1 + n |> { … }` is `1 + n`, as in `{ 1 + n } |> { … }`, and the left side of `-n |> { … }` is `-n`. So a match that is an operand of such an operator or of `-` or `-.`, or that has `.f`, `.k`, or a call after it, is put in a group: `{ n |> { 0 -> 1  _ -> 2 } } * 10`, `1 + { n |> { … } }`, `-{ n |> { … } }`, `{ opt |> { … } }.name`.

### 6.10 Blocks and bindings (edits)

These sentences change:

- In the list of expressions that give the "not used" warning, "functions" becomes "functions, clause blocks".
- In the list of expressions that give no "not used" warning, "a `case`" becomes "a match".
- After "`p = e` is a binding.", this is added: "A binding whose pattern is one name, with or without a type, may declare type variables after the name, as in `twice(A) = (f: (A) -> A, x: A) { f(f(x)) }` and `id(A): (A) -> A = (x) { x }` (§4.7)."
- "`(a, b) = v` with `v: (Int, Int) | Nil` is a compile error; `case` is used for this" becomes "`(a, b) = v` with `v: (Int, Int) | Nil` is a compile error; a match is used for this".
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

### 6.12 Clause blocks and matching (full text)

A clause block `{ clauses }` is a function of one parameter. When it is called with a value, it takes the first clause whose pattern matches the value and whose guard, if there is one, is `True`, and gives the value of the expression of that clause. A guard is any expression of type `Bool`. It is worked out only after its pattern has matched, and it may use the names of the pattern. When the pattern has choices `p1 | p2`, the choices are tried in order: the clause is taken with the first choice that matches and whose guard is `True`.

`e |> { clauses }` is a **match**, and `e` is its **subject**. It works out `e` and calls the clause block with it, as every `a |> f` is `f(a)` (§6.9). A match is not a new kind of call: it is how a clause block is used most often. A match adds no call frame: the clauses run in the place of the match, so a call in a clause expression of a match in tail position is in tail position (§11.6), and the match is not one of the calls in a stop message (§11.7).

- A clause block with one clause and no guard is the same as a function with one parameter: `{ p -> e }` is `(p) { e }`. Its pattern is fitted to the type of the parameter (§4.8 rule 4), and it is not checked for coverage. When `p` has no choices, `x: T` in it is a type annotation. When `p` may fail, it reports to `Assert`, as a parameter does (§6.11). So `opt |> { Some(x) -> x }` is the same as the binding `Some(x) = opt` followed by `x`, and `x` gets its known type in the same way (§4.8).
- When that one clause has choices, as in `{ Ok(v) | Error(v) -> v }`, the same rule holds, and the choices are fitted together (§4.8). In choices, `x: T` tests the value (§7.1). There is no function `(p1 | p2) { e }` to write it as, because a parameter has no choices.
- Any other clause block, with no clauses, with more than one clause, or with a guard, is typed by §4.8 rule 11, and its clauses must cover every value of the type of its parameter (§7.4). If not, it is a compile error.
- `{}` is a clause block with no clauses. Its parameter has the type `Never` (§4.8 rule 11), so `x |> {}` is correct only when `x` has no values.
- The type of a match is a type of every clause expression, for example their union. A clause block has the function type from the type of its parameter to that type.
- The parameter of a clause block with one clause, no guard, and no choices has the label of its pattern, as in `(p) { e }` (§6.11): `{ n -> n }` has the label `n`, and `{ Some(u) -> u }` has none. The parameter of any other clause block has no label, also when its one clause has choices, as in `{ v = Ok(_) | v = Error(_) -> v }`.
- A clause that can never be used gives a compile warning.
- A clause block is a direct value (§6.10), as every function is. Its guards and clause expressions are its function body: where §2.5, §4.7, §5, and §6.10 speak of the function bodies in a value, they include them. So `loop = { 0 -> Done  n -> loop(n - 1) }` is correct: `loop` is not among its own needs, and as a local binding it may be used in its clause expressions.
- Where this document speaks of a function expression, a clause block counts as one, unless the text says otherwise. So a clause block whose value is not used gives a compile warning (§6.10), and each time a clause block is worked out, it makes a new value, which is the same only as itself (§11.4).

### 7.1 Grammar (edits)

These sentences change:

- "In a `case` clause, `p: (Int, Int) -> e` is read as a function type, so a tuple type there is put in `{ }`, or a tuple pattern is used." becomes: "In a clause, `p: (Int, Int) -> e` is read as a function type, so a tuple type there is put in `{ }`, or a tuple pattern is used. In the same way, a `?` after a function type is part of the type, so a function type before a guard is put in `{ }`: `f: { (Int) -> Int } ? ok -> …`. In the first clause of a clause block, such a pattern has no `->` after it, so the `{` before it is kept for records (§6.1), and that is the compile error. Its message also says that a tuple type in a clause pattern is put in `{ }`."
- "In a `case`, `x: T` tests the value." becomes: "In a clause block with more than one clause or with a guard, and in a pattern with choices, `x: T` tests the value. In a clause block with one clause, no guard, and no choices, the pattern is a parameter (§6.12), so `x: T` there is a type annotation."
- "so in a `case`, `T` may not have a type variable as a member" becomes "so in a test, `T` may not have a type variable as a member".

### 7.4 Covering every value (full text)

The clauses of a clause block whose parameter has the type `A` **cover** `A` when, for every value of `A`, some clause without a guard has a pattern that matches it. A clause with a guard is not counted. A clause is **never used** when every value that its pattern matches is matched by an earlier clause without a guard.

So literal patterns of Int, Float, String, and BitArray never cover their type, a union is covered when every member is covered, and `{}` covers only `Never`.

Only a clause block with no clauses, with more than one clause, or with a guard must cover the type of its parameter (§6.12). A clause block with one clause and no guard is not checked for coverage: when its pattern may fail, it reports to `Assert`.

### 7.5 Known constructors (full text)

When a pattern `x = C(…)` matches, `x` is known to be made by `C` where `x` may be used. When a clause pattern `C(…)` of the match `v |> { … }` matches the value of the name `v`, `v` is known to be made by `C` inside that clause. This is used by §6.7.

### 9.3 Errors (edits)

"then use `case` with type tests" becomes "then use a match with type tests".

### 10.2 Entry points (edits)

"They must be in the `export` list of their module." becomes "They must be in the exports of their module."

### 11.4 Equality (edits)

In the bullet on functions, "the value made by one evaluation of a function expression" becomes "the value made by one evaluation of a function expression or a clause block". The rest of the section keeps its text.

### 11.6 Tail calls (full text)

A call in tail position uses no more stack. Tail positions are: the last expression of a function body; the expression of a clause of a clause block that is called as a function, and not in a match, because that expression is the last thing the function works out; the expression of a clause of a match in tail position; the last expression of a block in tail position; and the right side of `&&` or `||` in tail position. An environment statement does not change which calls are in tail position.

So in `loop = { 0 -> Done  n -> loop(n - 1) }`, the call `loop(n - 1)` uses no more stack. A match adds no call frame (§6.12).

## Notes for A1 and A2

- **How the front end maps the new syntax.** The parser makes a new node for a clause block in the syntax tree, `ast::ExprKind::Clauses`, and a match is a `|>` whose right side is such a node. The parser does not rewrite it. Resolve, not the parser, lowers it onto the existing resolved tree:
  - A block with one clause, no guard, and no choices becomes the existing nodes of a binding or a parameter, so that the checker and the lowering do not change for it. The match `e |> { p -> body }` becomes the block `{ p = e  body }`, and a lone block `{ p -> body }` becomes the function `(p) { body }`, with the label of `p` from `label_of`. These nodes are already fitted, already report to `Assert`, and already have the catch-all that calls `a.fail`. This is the one WAT difference that the plan allows for A1.
  - Any other match `e |> { clauses }`, with no clauses, with more than one clause, or with a guard, becomes the existing `case` node.
  - Any other clause block that is not the right side of `|>` becomes a `Function` of a fresh `x` whose body is the `case` node for `x |> { … }`. Resolve sets the label of that parameter to `None` itself and does not use `label_of`, which would give the label `x`, so the fresh name is never seen, in calls by label or in type schemes.
  - A block with one clause, no guard, and choices, as in `r |> { Ok(v) | Error(v) -> v }`, has no existing node: a binding and a parameter take one pattern, and a `case` node is typed by rule 11 and checked for coverage. So it becomes a `case` node with a mark, and the checker fits its choices as in §4.8, adds `Assert` when they may fail, and does not check coverage, and the lowering adds the catch-all that calls `a.fail`. This needs changes in `types/infer.rs`, `types/coverage.rs`, the lowering, and a field of `hir::ExprKind::Case`, which the plan does not give to A1. The coordinator decides whether A1 makes them, which also changes the WAT difference that the plan allows, or whether this form waits for a later stream and is a compile error until then.

  The second rule keeps the known types, narrowing, §7.5 known constructors on the name `e`, tail positions, and the order of evaluation exactly as they are today. The first rule gives what a binding gives today: known types by fitting, and the tail position of `body`.
- **Showing type schemes.** Shown type schemes use capital letters: `id : (x: A) -> A`. Error variables are `E`, `E2`, … and environment variables are `N`, `N2`, …, as the lowercase forms do today. The names are given in order, as today, and do not keep the names declared on the binding. A name that is a type that may be used in the module is skipped, and the next name is taken, so in a module with a type `E`, the first error variable is shown as `E2`, and with a type `A`, the first type variable is shown as `B`.
- **Renaming type variables in the formatter's migration.**
  - Turn a lowercase variable into an `UpName`: take out each `_` and make the letter after it a capital, and make the first letter a capital. So `a` becomes `A`, `key` becomes `Key`, `e2` becomes `E2`, `key_type` becomes `KeyType`, and `_a` becomes `A`.
  - Declare it on the binding that owned it under the old §4.7 rule. When that binding has a type on its pattern, the declaration goes after the name and before the `:`: `id: (a) -> a = (x) { x }` becomes `id(A): (A) -> A = (x) { x }`. The labels stay as they are, because the annotation is not moved.
  - When the new name is a type that may be used in the module, such as `result` becoming `Result`, or is the same as another new name of the same binding, as `ab` and `a_b` both becoming `AB`, choose another name and report it.
  - A type variable in a declaration with `@external` had no binding that owned it. Declare it after the name of the declaration, with the variables in the order of their first use in its type: `catch: (body: () -> a ? e ! n) -> Result(a, e) ! n` becomes `catch(A, E, N): (body: () -> A ? E ! N) -> Result(A, E) ! N`. This is needed for `inspect` and `hash` in `lib/i/core.i`, `catch` in `lib/i/fail.i`, and `boundary`, `resume`, and `cancel` in `lib/i/control.i`, and A2 searches for other such declarations.
  - When the binding that owned it has a pattern that is not one name, as in `(wrap, unwrap) = (…, …)`: inside another binding, declare the variable on the nearest binding of one name around it, and report it, because the local binding is then less general. At the top level, split the binding into one binding of one name for each part, when its value is a tuple with one part for each name, and declare the variables on each. When it cannot be split, report it, and leave it for the user to change.
- **Matches in operator expressions.** The formatter's migration puts in a group (§6.9) every old `case` that is an operand of an operator of level 7 or 8 or of `-` or `-.`, or that has `.f`, `.k`, or a call after it. So `1 + case n { … }` becomes `1 + { n |> { … } }`, and `case x { _ -> 1 }.y` becomes `{ x |> { _ -> 1 } }.y`. It also puts the subject `S` of an old `case S { … }` in a group when the top operator of `S` is at level 1 to 5 (`||`, `&&`, `==`, `!=`, a comparison, or `<>`), because `|>` is stronger than these (§6.9): `case S { … }` becomes `{ S } |> { … }`. Without the group, `case times > 1 { … }` in the example `retry` of §9.1 would become `times > (1 |> { … })`, and `case a <> b { … }` would become `a <> (b |> { … })`, which may be correct code with another meaning. This is needed in §9.1, in `std/src/float.i`, and in the tests with `case got == want { … }`. It also puts each clause or statement whose pattern starts with `(`, `-`, or `-.` on a new line (§1.7).
- **Guards after function types.** When a clause has a guard and its pattern ends in a type whose last part is a function type, the migration puts that function type in `{ }` before it writes `?` (§7.1). So `f: (Int) -> Int if ok -> 1` becomes `f: { (Int) -> Int } ? ok -> 1`, and `x: Int | (Int) -> Int if g -> …` becomes `x: Int | { (Int) -> Int } ? g -> …`. Without this, the `?` would be read as the error part of the function type, and with `! N` in the type, a `?` after it would be a compile error.
- **First clauses that give a function.** When the first clause of an old `case` has a pattern `name = ( … )` and its expression is a function, the migration puts the function in a group (§6.1): `p = (a, b) -> () { log(p) }` becomes `p = (a, b) -> { () { log(p) } }`.
- **Bare functions in prose.** An example function in prose that uses type variables and is not bound, such as `(x: a) -> a { x }` in §7.3, has no binding to declare them on. A2 writes it as the value of a binding that declares them: `id(A) = (x: A) -> A { x }`.
- **Tests whose meaning changes.** A `case` with one clause and no guard used to have to cover its value, and read `x: T` as a test. After the change, the same clause block is fitted, reports to `Assert`, and reads `x: T` as an annotation when it has no choices. So the checker tests that expect the old errors are rewritten, and their `.out` files change:
  - Tests that expect a "not covered" error from one clause, such as `case r { Ok(x) -> x }` in `crates/ilang-check/tests/types/coverage.i`, get a second clause that still does not cover the rest, or are moved to tests of the `Assert` behavior.
  - Tests that expect "type variable in a type test" from one clause, such as `bad = (v) { case v { x: a -> x } }` in `crates/ilang-check/tests/types/case.i`, get a second clause, so that `x: A` is still a test.
  - Tests of one clause with choices, such as those in `crates/ilang-check/tests/types/known_choices.i`, keep their meaning only where fitting the choices gives the same types as rule 11; their `.out` files are checked again.
- **Syntax error tests with no new form.** Some golden tests of syntax errors are correct code, or a different error, in the new syntax, so A2 removes or replaces them:
  - `crates/ilang-syntax/tests/golden/err_module_path.i`, `import (fail)`, becomes `(fail)`, which is an exports group, and the error becomes "`fail` is exported but not defined". The test moves to the checker. When an exported name that is not defined is the last part of a module path that the package may import, the message says that a module path has a `/`, as in `(i/fail)`.
  - `crates/ilang-syntax/tests/golden/err_case_brace.i`, `f = case x` with `{` on the next line, becomes `f = x |>` and then `{ … }` on the next line, which is correct, because a line that ends in an operator goes on (§1.7). The test is removed.
  - `crates/ilang-syntax/tests/golden/err_clause_tuple_type.i`, `p |> { p: (Int, Int) -> e }`, now gives the error that `{ name: e }` is kept for records, with the note on tuple types (§7.1). Its `.out` file changes.
  - `crates/ilang-syntax/tests/golden/err_pattern.i`, `x |> { -y -> 1 }`, and `crates/ilang-syntax/tests/golden/err_segment_type.i`, `b |> { <<x: Int>> -> 1 }`, keep their errors in the pattern, because a first statement that is not a correct expression and has a `->` after it is a clause whose pattern is wrong (§6.1). Their `.out` files are checked again.
  - `crates/ilang-syntax/tests/golden/err_block_end.i`, line 3, `h = {}`, expects "a block ends with an expression", but `{}` is now a correct clause block (§6.12). The line is removed, and a test of `{}` is added to the checker tests.
  - `crates/ilang-syntax/tests/golden/err_trailing.i`, line 4, `@main()`, expects "expected a part, found `)`", but `()` is now a correct `Call` with no arguments. A built-in attribute that takes no arguments is written with no `( )` (§2.2), so the error stays, with a new message, and the `.out` file changes.
  - `crates/ilang-syntax/tests/golden/err_toplevel.i`, line 1, `f(x)`, expects "expected an item", but it is now a `MacroItem`, which is kept for macros (§2.2). The line is replaced by a form that is still not an item, such as `1`, and the `.out` file is checked again.
- **Empty imports and exports.** The old syntax allows `import ()` and `export ()`, but an empty group is a compile error in the new syntax (§2.2). The migration removes an empty `import ()` or `export ()`. `crates/ilang-syntax/tests/format/imports_empty.i` is changed to test the error for an empty group. The test in `crates/ilang-lsp/tests/protocol.rs` that changes `export (double)` into `export ()` removes the exports group instead, so that it still tests a module with no exports.
- **Words that are no longer keywords.** `import`, `export`, `case`, and `if` are now normal names, so the tests and checks that reject them as keywords change:
  - `crates/ilang-check/tests/projects/package_modules/src/if.i`, which expects E0623, is removed or given another bad name, and `expected.out` changes.
  - `is_name` in `crates/ilang-package/src/lib.rs` no longer rejects these words, and the message in `crates/ilang-cli/src/commands/package.rs` no longer says "or a keyword". The tests that expect these words to fail, in `crates/ilang-package/tests/manifest.rs` (`name = "case"`) and in `crates/ilang-cli/tests/package_commands.rs` (`import` for `i new`, `case` for `i add`), use other bad names.
  - The keyword completions in `crates/ilang-check/src/query.rs` (the loop over `case`, `if`, `import`, `export`) and the doc comment that mentions keywords are removed. The tests that expect `case` among the completions, in `query.rs` and in `crates/ilang-lsp/tests/protocol.rs`, change.
- **What A2 changes.** Every `case`, `if` guard, `import (` and `export (` changes in stream A2, and so does every type variable in i code (see the start of the replacement text), such as `Result(a, e)` in §4.2 and the types in the table of §9.4, but not the notation of §0 in the rules of §4. The places are:
  - the spec, the library, `std`, the `.i` tests and their `.out` files, the README, the `i new` templates, and the editor grammar;
  - i source in Rust strings and in Markdown files, which the `.i` formatter does not reach. At `dd3771d` these are `crates/ilang-check/tests/{codegen,hir,query,types}.rs`, `crates/ilang-cli/tests/{commands,wasi,package_commands}.rs`, `crates/ilang-wasm/tests/codegen.rs`, `crates/ilang-lsp/tests/protocol.rs`, `crates/ilang-cli/src/commands/package.rs`, `bindings/javascript/README.md`, `docs/performance.md`, and `editors/vscode/README.md` (for example `import (i/core, c = (not) = i/core)` in `hir.rs`). A2 runs the migration on each such string, and checks again for others with a search for `case `, `import (` and `export (`. Some of these strings are `format!` templates, such as the `i new` templates in `crates/ilang-cli/src/commands/package.rs`, where `{{` and `}}` are i braces and `{name}` is a hole. They are not i source as written. A2 changes them by hand, and adds a test that renders each template and parses the result.
  - The tests in `crates/ilang-lsp/tests/protocol.rs` that point at places in the text: the line and column of each place are worked out again from the new text.
