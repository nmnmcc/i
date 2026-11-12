# Design: syntax with no keywords

Status: accepted syntax, amended by the retained SYN2 decisions of 2026-11-11. The A1/A2/A3 migration notes below are historical. The normative language text is `docs/spec.md`.

## Summary

- i has no keywords. Every word is a `name`, an `UpName`, or a discard.
- The imports and the exports are groups in `( )` at the start of a file, with no word before them. A group with a module path in it is the imports. A group with only names is the exports.
- `case` is gone. A **clause block** `{ p1 -> e1  p2 -> e2 }` is a function of one parameter, and `e |> { … }` matches `e`.
- A guard is written with `?`: `n ? n > 0 -> "positive"`.
- A type variable is an `UpName`, and it is declared after the name of its binding: `map(A, B) = …`. Type parameters of type definitions are `UpName`s too.
- A lowercase name in a type reads the type of a value; see `docs/spec.md` §3 and §4.7 (T2). Calls in types remain reserved for macros.
- Records use `(name: e)` and `()`; record types use `(name: T)` and the ordinary name `Record`. Tuples are positional and nonempty. Record semantics are implemented by T3.
- Alias patterns, as in `Schema(User, _) = user`, use the written-RHS classification and scope rules in `docs/spec.md` §4.3 (T2). The earlier replacement text below records the syntax migration; T2 supersedes its temporary reservation.
- The forms of macros in `docs/design/meta.md` are kept: `@m` and `@m(args)` before an item, a top-level item that is a call, a declaration with neither `@external` nor `@js`, and a call `m(…)` in a type. Until the macro streams, each is a compile error with a message.

Decisions taken with the user:

- A clause block with one clause and no guard is a binding: when its pattern does not match, it reports to `Assert`. Any other clause block (no clauses, more than one clause, or a guard) must cover every value, or it is a compile error. This is the rule of parameters and bindings today. It is not the rule of a `case` with one clause today, which must cover its value and reads `x: T` as a test; the notes for A2 say how the tests that use this change.
- A guard uses `?`.
- The imports and exports are told apart by what is in them, not by where they are.

## Replacement text for `docs/spec.md`

Each part below has one of two marks after its heading. The mark is not part of the heading.

- **(full text)**: the part takes the place of the whole section with the same number. Its heading is the new heading of the section.
- **(edits)**: the part names sentences, bullets, paragraphs, or the grammar of the section, and says how each changes or what is added. The rest of the section keeps its text. Only new text goes into the spec: the text in quotes, in a code block, or in bullets or paragraphs that comes after "becomes" or "becomes:", or after a sentence that says it is added. Quoted old text, before "becomes" or in a sentence that says it is removed, only finds the place in the section, and does not go into the spec. The other sentences of such a part are notes for A1.

Sections not named here keep their text. In stream A2, their examples change to the new syntax, and so do the type variables in i code in them: in `i` blocks, in written types and type definitions, and in the types of library names. For example, `Result(a, e) { Ok(a) Error(e) }` in §4.2 becomes `Result(A, E) { Ok(A) Error(E) }`, and the types in the table of §9.4 change in the same way. The letters of the notation of §0 in the rules of §4, such as `a - U` in §4.4 and `∀a1 … an. T` in §4.8, are not i code and do not change.

In §4, only the `i` blocks and the table of §4.2 are i code, so only they change, as in `List(A)` and `Result(A, E) { Ok(A) Error(E) }`. Every other type in §4 keeps its lowercase type variables as notation of §0, also when it is written in i syntax. So these do not change: `(x: a & Int) -> a` and `{ Yield(a) | n } - Yield(a)` in §4.4, `List(a)` and `Continuation(a, r, e, n)` in §4.5 rule 7, and `(x: a | Nil)` in §4.8. The type `Continuation(A, R, E, N)` in the table of §9.4 is i code, and it changes; by §0, the two ways of writing it say the same thing.

### Opening of the spec (edits)

The paragraph "i is small, and every part of it is what it looks like. …", before §0, becomes:

> i is small, and every part of it is what it looks like. A thing that looks like a function is a normal function, and a clause block is a function too (§6.12). A binding is a binding wherever it is written, and a clause block with one clause and no guard binds its pattern as a parameter does. Where two forms look the same, they have the same rules, with these exceptions, where the place of a form decides its rules:
>
> - `x: T` in a clause pattern is a type annotation when the clause block has one clause, no guard, and no choices, and a test when not (§7.1). So with `v: Int | String`, `v |> { x: Int -> x }` is a compile error, but `v |> { x: Int -> x  _ -> 0 }` is correct.
> - A `{` that begins an expression is a clause block or a block by its first statement, but the body of a function is always a block (§6.1). So in `make = (n) { (x) -> Int { x + n } }`, the body is a block that gives a function, but `g = { (x) -> Int { x + n } }` is a clause block with the pattern `(x)`.
> - In the first statement of a `{`, `n = ( … ) -> T` is the binding of a function when a `{` comes after `T` on the same line, and a clause pattern followed by `->` when not (§6.1).

### 0. How to read this document (edits)

A paragraph is added after the paragraph on EBNF:

> Inside a type in the rules and in the prose, a capital letter, alone or with a digit, such as `A`, `T`, `E`, `N`, or `P1`, stands for any type, and a lowercase letter, alone or with a digit, such as `a`, `r`, or `b1`, stands for a type variable. A letter followed by an index, `i`, `j`, `k`, `m`, or `n`, as in `Ti`, `Qi`, `ai`, `an`, or `bm`, is that letter with that index, and stands for the same kind of thing as the letter alone. Outside a type, the letters keep the sense that the text gives them: in §4.8, for example, `C` is a constructor, `e` an expression, and `r` an expression or a name. This is notation of this document, not i syntax: in i code, a type variable is an `UpName` that is declared (§4.7), as in `id(A): (A) -> A`.

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

i has no keywords. Every word is a `name`, an `UpName`, or a discard (§1.3), and any `name` may be used as a name. The forms that other languages make with keywords are made with brackets and symbols: imports and exports are groups at the start of a file (§2.2), matching is a call of a clause block (§6.12), and a guard is written with `?`.

Some forms are kept for macros, which are not yet part of i: an attribute that is not built in, such as `@m` or `@m(args)` before an item (§2.2); a top-level item that is a call, `m(args)` or `a.m(args)` (§2.2); a declaration with neither `@external` nor `@js` (§2.2); and, in a type, a lowercase name or `a.m` followed by `(` (§3). The parser reads each of them in full, and each is a compile error whose message says that the form is kept for macros.

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
- A line that ends in an operator, `,`, `=`, `:`, `->`, `(`, `[`, `{`, or `<<` goes on in the next line. A line that begins with `.`, `|`, `?`, `^`, `->`, or a binary operator other than `-` and `-.` goes on from the line before.
- `a` and then `- b` on the next line are two statements. To write `a - b` on two lines, put `-` at the end of the first line. In the same way, `! N` of a function type may not begin a line.

One rule does not depend on lines: in the `->` part of a function, after the `)` of its parameters, a `{` right after `->`, `^`, `!`, `|`, `&`, or `-` is part of the type (§3). The first other `{` that is not inside brackets begins the body.

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

> `Call` is the call of §6.1, so the arguments of an attribute and of a `MacroItem` are read as expressions. A `(` on the same line right after the name of an attribute is always the start of the arguments of that attribute (§1.7). So when an attribute has no arguments and the item after it starts with `(`, as the binding `(even, odd) = …` does, the item begins on the next line:
>
> ```i
> @m
> (even, odd) = ((n) { n == 0 || odd(n - 1) }, (n) { n != 0 && even(n - 1) })
> ```
>
> On one line, `@m (even, odd) = …` is the attribute `@m(even, odd)` followed by `= …`, which is a compile error.

This is a note for the macro streams, and not text of the spec: the printer that changes an `.is` file into an `.i` file (`docs/design/meta.md` §2.3) always puts a line end between an attribute with no arguments and an item that starts with `(`.

The first paragraph, "A file has at most one `import` and at most one `export`. …", becomes:

> At the start of a file, a group in `( )` that is not followed by `=` is not an item:
>
> - When an item in it has a module path (§2.3), which always has a `/`, the group is the **imports**.
> - When not, it is the **exports**: every item in it is a `name`, or an `UpName` with or without `(..)`.
>
> A file has at most one imports group and at most one exports group. They come first, the imports before the exports. Neither may be empty: a file with nothing to import or export leaves the group out. Attributes may not come before the imports or the exports: a group in `( )` with an attribute before it is an item. A group in `( )` that is followed by `=` is a binding with a tuple pattern, as in `(even, odd) = …`. Any other group in `( )` at the top level, with or without attributes before it, is a compile error.
>
> An item that is a `name`, with or without type parameters (§2.5), then `:` and a type, with no `=` after the type, is a declaration, whatever its attributes. Any other item that starts with an `UpName` is a type definition (§4.3). An item that is a call, `m(args)` or `a.m(args)`, with no `=` or `:` after its `)`, is a `MacroItem`. Every other item is a binding (§2.5).
>
> A `MacroItem` is kept for macros. It is a compile error, and the message says that a call at the top level is kept for macros.
>
> A declaration requires `@external` or `@js` (§10.1), or expansion by a macro. A declaration with neither attribute is kept for macros, where it goes with a binding of the same name. Until that expansion is defined, it is a compile error, and the message says that the declaration is kept for macros.

The paragraph "In one module, every top-level name is defined one time only. …" keeps its text. The paragraph "These are the attributes. …" becomes:

> These are the attributes. An attribute whose name is one of them, with no module name before it, is that attribute. Its arguments must be string literals, given by position, with no labels,. Any other argument, such as `@external("env" <> "", "f")`, `@external(module: "env", "f")`, is a compile error. Any other attribute, such as `@m`, `@m(args)`, or `@a.m(args)`, is a macro before an item. It is kept for macros, and it is a compile error whose message says so. An attribute in another place, or one attribute given two times on one item, is a compile error.

The list of the attributes keeps its text, with this addition from js.md §9:

> - `@js("expression")`: on a declaration of a closed opaque type (§10.1.1). A declaration cannot have both `@js` and `@external`.

### 2.3 Imports (edits)

The heading "2.3 import" becomes "2.3 Imports". The grammar becomes:

```
ImportItem = [ name "=" ] [ "(" ImportName { "," ImportName } [ "," ] ")" "=" ] ModulePath .
ImportName = name | UpName .
ModulePath = name "/" name { "/" name } .
```

The paragraph "`import` and `export` are each followed by a tuple of items. …" and its example become:

> The imports and the exports are each a parenthesised group of items. In each group, the items have `,` or a break (§1.7) between them:
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

These paragraphs are added after the grammar:

> A binding whose pattern is one name, with or without a type, may declare type variables after the name (§4.7): `map(A, B) = …`, `id(A): (A) -> A = (x) { x }`. A declaration may declare them in the same way: `inspect(A): (A) -> String`.
>
> An item that starts with `name ( UpName { , UpName } )` and then `=` or `:` is always a binding or a declaration with type parameters. It is never a binding with a constructor pattern, though `f(A)` could also be read as one (§7.1). So `wrap(A) = (x: A) { Some(x) }` declares `A`. The same is true for a statement of a block (§6.1).
>
> An item that is a call, `m(args)` or `a.m(args)`, and has no `=` or `:` after its `)`, is not a binding. It is kept for macros (§2.2).
>
> An item `name [ TypeParams ] : Type` with no `=` after the type is a declaration, whatever its attributes. A declaration requires `@external` or `@js`, or expansion by a macro (§2.2). A `@js` declaration has the closed opaque type and foreign link semantics of §10.1. It is initialized once during start only if retained by the target link pass; ordinary bindings retain this section's initialization rules.

The sentence "A declaration may only be used with `@external` (§10.1)." is removed, because the last paragraph above takes its place. The rest of the section keeps its text.

### 3. Type syntax (edits)

In the grammar, the rule for `Prim` changes, and two rules are added after `NamedType`. The other rules keep their text.

```
Prim      = FnType | TupleType | RecordType | NamedType | TypeCall | [ name "." ] name | "_" | "{" Type "}" .
TypeCall  = [ name "." ] name "(" [ TypeArg { "," TypeArg } [ "," ] ] ")" .
TypeArg   = Type | int_lit | string_lit .
```

The bullet "A `name` is a type variable. `_` is a hole that type inference fills in." becomes these bullets:

> - An `UpName` with no module name before it is a type variable when a type variable with that name is declared where the type is written (§4.7, §5), and a type when not. `_` is a hole that type inference fills in.
> - A type variable takes no arguments: `A(Int)`, where `A` is a declared type variable, is a compile error.
> - A lowercase `name` in a type, or `a.x` with a lowercase `x`, whatever `a` is, with no `(` after it, is kept for a later use, the type of a value, and is a compile error. It is in the grammar, so a type with it is read in full, and the error says what the name is kept for. So `(p: User, q: p.name) { q }` is a compile error, where `p` is a parameter, as it is where `p` is a module.
> - A lowercase `name`, or `a.m` with a lowercase `m`, whatever `a` is, followed by `(` is a `TypeCall`. It is kept for macros, and is a compile error. It is in the grammar, so a type with it is read in full, and the error says that it is kept for macros.

In the bullet on `{ }`, the example `(a) -> { (b) -> c } ! E` becomes `(A) -> { (B) -> C } ! E`. The other bullets keep their text.

### 4.3 Custom types (full text)

```
TypeDef     = UpName [ TypeHead ] TypeBody .
TypeHead    = "(" Type { "," Type } [ "," ] ")" .
TypeBody    = "{" { Constructor } "}" | "=" Type .
Constructor = UpName [ "(" Field { "," Field } [ "," ] ")" ] .
Field       = [ name ":" ] Type .
```

- `T { C1 C2 … }` is a custom type. Every constructor has zero or more fields. A field may have a label.
- `T {}` is a type with no constructors. i code may not make its values. They only come from the host (§10).
- `T = Type` is a type alias. `T` is the same as the type on the right. An alias may not use itself, directly or through other aliases.
- The head `( … )` is a **list of type parameters** when every part of it is an `UpName` with no module name and no arguments, and none of them is a type that may be written with no module name in the module: a type of the module, an imported `UpName`, or a built-in type. Its parts are then the **type parameters**, as in `TypeParams` of §2.5.
- An alias whose head is not a list of type parameters, as in `Schema(User, _) = user`, or `Schema(User) = user` with a type `User`, is kept for a later use, alias patterns, and is a compile error whose message says so. The rule that tells an alias pattern from a generic alias is not yet given.
- A custom type `T( … ) { … }` whose head is not a list of type parameters is a compile error. When a part of the head is not an `UpName` with no module name and no arguments, as in `Box(a)`, `Box(m.T)`, or `Box(List(A))`, the message says that a type parameter is an `UpName`. When every part is such an `UpName`, but one of them is a type that may be written with no module name in the module, as in `Box(Int) { B(Int) }` or `Pair(User) { Pair(User, User) }` with a type `User`, the message says that a type parameter may not have the name of a type.
- The type parameters are the type variables of the definition: `Option(A) { Some(A) None }`. Every type parameter must be used on the right, and every type variable on the right must be a parameter.
- The type parameters of one definition must have different names.
- Custom types are **nominal**: two different definitions make two different types, even if they have the same form.

### 4.7 Type inference (edits)

The bullet "The **annotations of a binding** are …" becomes these bullets, and the other bullets keep their text:

> - A type variable is **declared** in one of three places: after the name of a binding (§2.5, §6.10), `map(A, B) = …`; after the name of a declaration, `inspect(A): (A) -> String`; or after the name of a type definition (§4.3).
> - The type variables declared in one place must have different names. So `pair(A, A) = …` is a compile error.
> - A type variable declared by a binding may be used in the type on its pattern, as in `id(A): (A) -> A = …`, and in every type written in its value, also in the parameters, `->` parts, and clause patterns there, and in inner bindings. It is one type variable of that binding, and it is generalized with it. An inner binding may declare a type variable with the same name; inside it, the name means its own variable.
> - A type variable declared by a declaration may be used in the type of the declaration. It is general: each use of the declared name may give it a different type.
> - An `UpName` in a type that is neither a type nor a declared type variable is a compile error. The message says how to declare it.
> - A declared type variable may not have the name of a type that may be written with no module name in the module (a type of the module, an imported `UpName`, or a built-in type), because then a type would mean two things.
> - A type variable may only be declared on a binding whose pattern is one name, `x`, or one name with a type, `x: T`. A binding with another pattern, such as `(even, odd) = …`, uses the type variables of the bindings it is inside. A top-level binding with another pattern is inside no binding, so it may not use type variables; its values are written as bindings of one name each when they need them.

### 4.8 Typing rules (edits)

Rule 11 becomes:

> 11. **Clauses.** This rule is for a clause block with no clauses, with more than one clause, or with a guard (§6.12). Let it be called with a value of type `A`: in a match `e |> { … }` with `e : A`, or as a function whose parameter has the type `A`. The clause patterns (each choice of `|` counts) are looked at by **place**. The top place is the value of the parameter. Below a pattern at a place, its parts are at places of their own: each field of a constructor `C`, each part of a tuple of n parts, and the parts and the rest of a list. Places below different constructors are different places. At each place, let `B` be the type of the values there (`A` at the top place, and else the part type of the pattern type above it), and let `P` be the union of the pattern types (below) of the patterns at that place that are not catch-alls. Patterns at one place with the same type constructor use the same new type variables, so `Ok(v)` and `Error(e)` at one place give one `Result(b1, b2)`. A type test `x: T` at a place also needs `B <: T | r - T`, with a new type variable `r`: it takes every value whose type constructor is that of a member of `T` (§7.1), so those values must be in `T`. So `v |> { x: Box(Int) -> …  y: Box(String) -> … }` with `v: Box(Int | String)` is a compile error. A constructor pattern with `..` counts as having a catch-all at the places of the fields that it leaves out. If some clause without a guard has a catch-all at that place or at a place above it, then `B <: P | r - P`, with a new type variable `r`: a value that no pattern takes may still come there. If not, then `B <: P`. With no clauses, `P` is `Never` at the top place, so `A <: Never`. A catch-all at a place gets the type of rule 12. Every guard and every clause expression has the errors `E` and the environment `N`, every guard has the type `Bool`, and every clause expression has the type `R`. Then the match `e |> { … }` has the type `R`, and the clause block has the type `(A) -> R ^ E ! N`, whose parameter has no label. Making the clause block needs nothing, as for a function in rule 4. Coverage (§7.4) is checked with the types that inference finds.
>
> A clause block with one clause and no guard is not typed by this rule. It is the function `(p) { e }` of rule 4: its pattern is fitted to the type of the parameter (below), it is not checked for coverage, and `S` is `Assert` when the pattern may fail. A match `s |> { p -> e }` with such a clause block is typed as the binding `p = s` followed by `e` (rule 10), with the one difference that `p` is not generalized, as a parameter is not. So the subject `s` gets the expected type that a binding gives its value (below).

The paragraph on fitting after the rules becomes:

> To **fit** a pattern `q` to a type `B`, in a binding, a parameter, or a clause block with one clause and no guard: if `q` is a catch-all, its names get `B`. If not, `B <: Q`, where `Q` is the pattern type of `q`, and the parts of `q` are fitted to the parts of `Q` given below. Fitting never adds `Assert`. In a clause block with no clauses, with more than one clause, or with a guard, rules 11 and 12 take the place of fitting.
>
> Only a clause block with one clause may have choices `q1 | … | qn` where a pattern is fitted. If one choice is a catch-all, the names get `B`. If not, `B <: Q1 | … | Qn`, where `Qi` is the pattern type of `qi`, and choices with the same type constructor use the same new type variables. The parts of each `qi` are fitted to the parts of its `Qi`, and a name gets the union of its types in the choices (§7.2). In choices, `x: T` is a test, not a type annotation (§7.1), so `T` may not have a type variable as a member. The choices may fail for `B` when together they do not match every value of `B` (§7.3); then `S` in rule 4 is `Assert`.

The known-type bullets change in this way:

- "A `case` has the union of the known types of its clause expressions when they all have one." becomes "A match `e |> { … }` has the union of the known types of its clause expressions when they all have one."
- A bullet is added after the bullet for a function expression: "A clause block that is not the right side of a match has a known function type with one parameter, made as for a function expression: the label of its parameter (§6.12); for the parameter, `Never` when the block has no clauses, the type written at the top of its pattern when the block has one clause, no guard, and no choices, or else the type it gets from an expected type, or else `_`; and for the result, the union of the known types of its clause expressions when they all have one, or else `_`. When the parameter has a known type `B`, from a written type or from an expected type, every clause pattern is a pattern fitted to a value with the known type `B`, as in a match (the bullet on matches, before **Names in patterns**, below), and the narrowing by earlier clauses of §4.8 rule 12 is used in the same way. So with `opts: List(Option(User))`, `list.map(opts, { Some(u) -> u.name  None -> "" })` is correct, as `list.map(opts, (o) { o |> { Some(u) -> u.name  None -> "" } })` is. In the bullets on expected types, such a clause block counts as a function expression of one parameter."
- A bullet is added before the bullet "**Names in patterns**": "In a match `e |> { … }`, every clause pattern is a pattern fitted to a value with the known type of `e` (below), also when the clause block has one clause and no guard. So with `opt: Option(User)`, `opt |> { Some(u) -> u.name }` is correct, as `Some(u) = opt` followed by `u.name` is."
- "every clause expression of a `case` has the expected type of the `case`" becomes "every clause expression of a match has the expected type of the match".
- A bullet is added to the bullets on expected types: "In a match `s |> { p -> e }` whose clause block has one clause, no guard, and no choices, when `p` has `x: T` at its top, `s` has the expected type `T`, as the value of a binding `x: T = s` does. So `(u) { u.name } |> { f: (User) -> String -> f(bob) }` is correct, as `f: (User) -> String = (u) { u.name }` followed by `f(bob)` is."
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
Primary   = int_lit | float_lit | string_lit | Name | Tuple | Record | List | BitArray
          | Block | Clauses | Function | CtorUpdate .
Name      = [ name "." ] ( name | UpName ) .
Tuple     = "(" Expr { "," Expr } [ "," ] ")" .
Record    = "(" ")" | "(" VField { "," VField } [ "," ] ")"
          | "(" ".." Expr { "," VField } [ "," ] ")" .
VField    = ValueLabel ":" Expr .
ValueLabel = name | string_lit .
List      = "[" [ Expr { "," Expr } [ "," ".." Expr ] [ "," ] ] "]" .
Block     = "{" { Statement } Expr "}" .
Statement = Pattern "=" Expr | name TypeParams [ ":" Type ] "=" Expr | "!" NamedType "=" Expr | Expr .
Clauses   = "{" { Clause } "}" .
Clause    = Pattern { "|" Pattern } [ "?" Expr ] "->" Expr .
Function  = "(" [ Param { "," Param } [ "," ] ] ")" [ "->" Union [ "^" Union ] [ "!" Union ] ] Block .
Param     = Pattern | name "!" NamedType .
CtorUpdate = Name "(" ".." Expr { "," name ":" Expr } [ "," ] ")" .
```

- Header groups, calls, attributes, constructor fields and patterns keep their own context. Otherwise a matching `)` followed by a same-line `{`, or by `->`, selects a function before its contents are parsed. Without that suffix, zero items, a first `..`, or a label followed by `:` selects a record; other nonempty contents select a positional tuple. Tuple and record parts never mix. Commas and breaks separate parts; they never decide the form. Function bodies are always blocks. In a guard, a `->` that is not inside brackets always ends the clause head. So in `n ? (a) -> 1`, the guard is the tuple `(a)`, which is a compile error because a guard must be a `Bool`, and not a function.
- A `{` that begins an expression begins a **clause block** when it is `{}`, or when its first statement is a clause (next bullet). When not, it begins a block. So `{ x }` is a block and `{ x -> x }` is a clause block. The body of a function (§6.8) does not begin an expression: it is always a block, and the tests below are not made for it.
- To find whether the first statement is a clause, these tests are made in order. The first that holds decides.
  1. When the statement starts with `name ( UpName { , UpName } )` and then `=` or `:`, it is a binding with type parameters (last bullet), and not a clause, though `f(A)` could also be read as a constructor pattern (§7.1). So `id(A) = (x: A) -> A { x }` is a binding with type parameters, and not a binding with the constructor pattern `id(A)` (§7.1), which test 3 would find, and not a clause.
  2. The longest pattern `p` at the start of the statement is read. A type in it is read as §7.1 says, as far to the right as it goes, with no going back: the pattern is not cut short to end before a `->` or a `|` that the type takes. When `p` is followed by `|`, `?`, or `->`, the statement is a clause, with one exception. The exception is a binding of a function: `p` is `n1 = … = nk = ( … )`, one or more names, each followed by `=`, and then a tuple pattern; after the `->` that follows `p`, the `->` part of a function, `Union [ "^" Union ] [ "!" Union ]` (§6.8), may be read in full; and a `{` comes after it on the same line. Then the statement is the binding of the pattern `n1 = … = nk` to the function that starts at `( … )`.
  3. When the statement starts with a pattern and then `=`, it is a binding (last bullet), and not a clause. When it starts with `!`, it is an environment statement, and not a clause. So a binding of a function needs no exception when the name has a type, as in `f: (Int) -> Int = (n) -> Int { n }`, because its longest pattern, `f: (Int) -> Int`, is followed by `=`. Nor does a function with an environment parameter, as in `go = (n, log ! Log) -> Int { … }`, because `log ! Log` is not a pattern, so the longest pattern is `go`.
  4. When a `->` that is not inside brackets comes before the next break or the `}`, and the statement either is not a correct expression or has a longest expression that is followed by `->`, the statement is a clause whose pattern is wrong. Then the `{` begins a clause block, and the error is the first error found when the start of the statement, up to that `->`, is read as a pattern followed by `->`. In this reading, the type of a pattern `x: T` is still read as far to the right as it goes, so the `->` that ends the pattern is the first one after it. So `x |> { -y -> 1 }` and `b |> { <<x: Int>> -> 1 }` give an error in the pattern, and not a block with a stray `->`. This test is only made for a statement that the tests before it do not decide, so a `->` in a later statement on the same line, as in `{ k = 2  g = (x) -> Int { x * k }  g(1) }`, does not count.
  5. When none of these holds, the statement is an expression, and not a clause.

  So these are blocks:
  - `{ go = (n) -> Int { n + 1 }  go(1) }`
  - `{ go = (n) -> Int ^ String ! Log { step(n) }  go(1) }`
  - `{ k: (Int) -> Int = f  k(1) }`
  - `{ go(A) = (x: A) -> A { x }  go(1) }`
  - `{ go = (n, log ! Log) -> Int { log.write(n)  n }  go(1) }`
  - `{ k = 2  g = (x) -> Int { x * k }  g(1) }`

  and these are clause blocks:
  - `{ t = (0, _) -> t  _ -> 0 }`
  - `{ r = Ok(v) -> v  _ -> 0 }`
  - `{ s = "ab" <> rest -> s  _ -> "" }`
- So in a first clause whose pattern is `n1 = … = nk = ( … )`, as in test 2, a clause expression that is a function, such as `() { … }` or `(x) -> Int { x }`, would be read as the `->` part of a function and its body. There, a function with no `->` part is put in a group: `{ p = (a, b) -> { () { log(p) } } }` is a clause block whose one clause gives a function with no parameters. The test above does not take `{ () { log(p) } }` as a type, because a type in `{ }` ends at its `}`, and a function in it has a `{` before that. A group alone does not help a function with a `->` part whose parameters are all patterns (next bullet), so it is bound to a name inside the group: `{ p = (a, b) -> { g = (x) -> Int { x + a }  g } }`. By the exception of test 2, the first statement of that group is a binding, so the group is a block. Later clauses, and first clauses with other patterns, need no group: `hs |> { (name, _) -> (x) { log(name, x) }  _ -> (x) { x } }`.
- A `{` that begins an expression and whose first statement starts with a function that has a `->` part, is not bound to a name, and has only patterns as parameters, such as `{ (x) -> Int { x } … }`, is read as a clause block by test 2, with the pattern `(x)` and the expression `Int`. Such a function is bound to a name first: `g = (x) -> Int { x }`. A group does not help, because `{ (x) -> Int { x } }` is read in the same way. A function with an environment parameter is not read in this way: in `{ (n, log ! Log) -> Int { log.write(n)  n } }`, `(n, log ! Log)` is not a pattern, so tests 1 to 4 do not hold, and by test 5 the `{` is a block whose one statement is the function. A function body is not such a `{`: in `make = (n) { (x) -> Int { x + n } }`, the body is a block whose one statement is a function.
- In a block, a statement that starts with a pattern and then `=` is a binding. When more than one such pattern may be read, the longest is taken: in `a = (x, y) = e`, the pattern is `a = (x, y)`. A statement that starts with `name ( UpName { , UpName } )` and then `=` or `:` is a binding with type parameters; the parser looks past the `)` to find the `=` or `:`. Such a statement is never read as a binding with a constructor pattern, though `f(A)` could also be read as one (§7.1), and this is the same at the top level (§2.5). So `id(A) = …` and `id(A): (A) -> A = …` are bindings, `wrap(A) = (x: A) { Some(x) }` declares `A`, and `send(Stop)` is an expression. A statement that starts with `!` is an environment statement. Every other statement is an expression.

The literal-type correction to §6.1 test 4 above follows [literal-types.md §6.1](literal-types.md): “with the `Literal` Prim of §3, `x: 1` is a pattern, so `{ x: 1 -> 2 }` is a clause block with one clause, the function `(x: 1) { 2 }`, and not the records error.”

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
- After "The names in `p` may be used from the next statement.", at the end of the paragraph that starts "`p = e` is a binding.", this is added: "A binding whose pattern is one name, with or without a type, may declare type variables after the name, as in `twice(A) = (f: (A) -> A, x: A) { f(f(x)) }` and `id(A): (A) -> A = (x) { x }` (§4.7)."
- "`(a, b) = v` with `v: (Int, Int) | Nil` is a compile error; `case` is used for this" becomes "`(a, b) = v` with `v: (Int, Int) | Nil` is a compile error; a match is used for this".
- "the binding works like a `case` on `e` with two clauses: `p`, which goes on with the rest of the block, and a catch-all, which calls `a.fail(AssertionFailure(m))`" becomes "when it runs, the binding works like the match `e |> { p -> …  _ -> a.fail(AssertionFailure(m)) }`, where the first clause goes on with the rest of the block. The binding is still typed by §4.8 rule 10, not by rule 11."
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

`e |> { clauses }` is a **match**, and `e` is its **subject**. It works out `e` and calls the clause block with it, as a call of the clause block (§6.9). A match is not a new kind of call: it is how a clause block is used most often. A match adds no call frame: the clauses run in the place of the match, so a call in a clause expression of a match in tail position is in tail position (§11.6), and the match is not one of the calls in a stop message (§11.7).

- A clause block with one clause and no guard is the same as a function with one parameter: `{ p -> e }` is `(p) { e }`. Its pattern is fitted to the type of the parameter (§4.8 rule 4), and it is not checked for coverage. When `p` has no choices, `x: T` in it is a type annotation. When `p` may fail, it reports to `Assert`, as a parameter does (§6.11), and the first position of the stop message (§11.7) is the position of `p`, as for a parameter. So `opt |> { Some(x) -> x }` is the same as the binding `Some(x) = opt` followed by `x`, and it gives only what that binding gives: `x` gets its known type in the same way (§4.8), and `opt` is not known to be made by `Some` (§7.5).
- When that one clause has choices, as in `{ Ok(v) | Error(v) -> v }`, the same rule holds, and the choices are fitted together (§4.8). In choices, `x: T` tests the value (§7.1). There is no function `(p1 | p2) { e }` to write it as, because a parameter has no choices.
- Any other clause block, with no clauses, with more than one clause, or with a guard, is typed by §4.8 rule 11, and its clauses must cover every value of the type of its parameter (§7.4). If not, it is a compile error.
- `{}` is a clause block with no clauses. Its parameter has the type `Never` (§4.8 rule 11), so `x |> {}` is correct only when `x` has no values.
- The type of a match is a type of every clause expression, for example their union. A clause block has the function type from the type of its parameter to that type.
- The parameter of a clause block with one clause, no guard, and no choices has the label of its pattern, as in `(p) { e }` (§6.11): `{ n -> n }` has the label `n`, and `{ Some(u) -> u }` has none. The parameter of any other clause block has no label, also when its one clause has choices, as in `{ v = Ok(_) | v = Error(_) -> v }`.
- A clause that can never be used gives a compile warning.
- A clause block is a direct value (§6.10), as every function is. Where §2.5, §5, and §6.10 speak of the function bodies in a value, the guards and clause expressions of a clause block count as function bodies. So `loop = { 0 -> Done  n -> loop(n - 1) }` is correct: `loop` is not among its own needs, and as a local binding it may be used in its clause expressions.
- Where this document speaks of a function expression, a clause block counts as one, unless the text says otherwise. So a clause block whose value is not used gives a compile warning (§6.10), and each time a clause block is worked out, it makes a new value, which is the same only as itself (§11.4).

### 7.1 Grammar (edits)

These sentences change:

- "In a `case` clause, `p: (Int, Int) -> e` is read as a function type, so a tuple type there is put in `{ }`, or a tuple pattern is used." becomes: "In a clause, `p: (Int, Int) -> e` is read as a function type, so a tuple type there is put in `{ }`, or a tuple pattern is used. A `?` after a function type begins a guard; error slots use `^`. A missing clause arrow after a function type is an arrow/type-boundary error; group a tuple or record type in `{ T }` when needed."
- At the end of the bullet that starts "The type after `:` goes as far to the right as it can", after "so a type test is written after the other choices: `True | _: Int`.", this is added: "The type is read with no going back: once `( … )` is followed by `->`, it is a function type, and once a `|` follows a type, the next part is a type. When what follows cannot be read as a type, the pattern is wrong; it is not read again as a shorter pattern. So `v |> { p: (Int, Int) -> 0  _ -> 1 }` and `v |> { x: Int | 0 -> a  _ -> b }` are compile errors, in the first clause and in any other."
- "In a `case`, `x: T` tests the value." becomes: "In a clause block with more than one clause or with a guard, and in a pattern with choices, `x: T` tests the value. In a clause block with one clause, no guard, and no choices, the pattern is a parameter (§6.12), so `x: T` there is a type annotation."
- "so in a `case`, `T` may not have a type variable as a member" becomes "so in a test, `T` may not have a type variable as a member".

### 7.4 Covering every value (full text)

The clauses of a clause block whose parameter has the type `A` **cover** `A` when, for every value of `A`, some clause without a guard has a pattern that matches it. A clause with a guard is not counted. A clause is **never used** when every value that its pattern matches is matched by an earlier clause without a guard.

So literal patterns of Int, Float, String, and BitArray never cover their type, a union is covered when every member is covered, and `{}` covers only `Never`.

Only a clause block with no clauses, with more than one clause, or with a guard must cover the type of its parameter (§6.12). A clause block with one clause and no guard is not checked for coverage: when its pattern may fail, it reports to `Assert`.

### 7.5 Known constructors (full text)

When a pattern `x = C(…)` matches, `x` is known to be made by `C` where `x` may be used. When a clause pattern `C(…)` of the match `v |> { … }` matches the value of the name `v`, and the match has no clauses, more than one clause, or a guard (§4.8 rule 11), `v` is known to be made by `C` inside that clause. A match with one clause and no guard gives only what the binding of its pattern to `v` gives (§6.12). This is used by §6.7.

### 9.3 Errors (edits)

"then use `case` with type tests" becomes "then use a match with type tests".

### 10.2 Entry points (edits)

"They must be in the `export` list of their module." becomes "They must be in the exports of their module."

### 11.4 Equality (edits)

In the bullet on functions, "the value made by one evaluation of a function expression" becomes "the value made by one evaluation of a function expression or a clause block". The rest of the section keeps its text.

### 11.6 Tail calls (full text)

A call in tail position uses no more stack. Tail positions are: the last expression of the body of a function expression (§6.8); the expression of a clause of a clause block that is called as a function, and not in a match, because that expression is the last thing the function works out; the expression of a clause of a match in tail position; the last expression of a block in tail position; and the right side of `&&` or `||` in tail position. An environment statement does not change which calls are in tail position. The clause expressions of a clause block are in tail position only by the two items on clauses above.

So in `loop = { 0 -> Done  n -> loop(n - 1) }`, the call `loop(n - 1)` uses no more stack. A match adds no call frame (§6.12).

## Notes for A1 and A2

- **How the front end maps the new syntax.** The parser makes a new node for a clause block in the syntax tree, `ast::ExprKind::Clauses`, and a match is the existing `ast::ExprKind::Case` with `syntax: Syntax::New`. Old cases have `Syntax::Old`. This keeps existing Case visitors and lets resolve choose the typing rule (A1 D5). Resolve, not the parser, lowers it onto the existing resolved tree:
  - A block with one clause, no guard, and no choices becomes the existing nodes of a binding or a parameter, so that the checker and the lowering do not change for it. The match `e |> { p -> body }` becomes the block `{ p = e  body }`, so `e` gets the expected type of the binding, as §4.8 says, and a lone block `{ p -> body }` becomes the function `(p) { body }`, with the label of `p` from `label_of`. These nodes are already fitted, already report to `Assert`, and already have the catch-all that calls `a.fail`. The permitted WAT differences are the reviewed owner categories of A1 §3.17: `choices-split`, `g-binding`, `split`, `pattern-text`, and `hand`.
  - Spans of the block `{ p = e  body }`: the made-up `Bind` has `p.span`, so the existing lowering already stops at `p` (§6.12). `lower.rs` stays unchanged. `query.rs` searches `cover(b.span, b.value.span)` so it still finds the earlier subject `e` (A1 D6 and §3.12). This replaces the proposed lowering edit with a span and query change.
  - In the block `{ p = e  body }`, the binding is never a direct value, even when `e` is one: resolve resolves `e` outside the scope of the names of `p`, and sets `direct` of the binding to `false`, so `p` is not generalized, as a parameter is not (§4.8 rule 14). Without this, in `g = () { (x) { f(x) * 2 } |> { f -> f(1) } }`, the `f` inside the subject would be the new `f`, and `g()` would call itself without end, where by §6.12 it is the top-level `f`.
  - Any other match `e |> { clauses }`, with no clauses, with more than one clause, or with a guard, becomes the existing `case` node.
  - Any other clause block that is not the right side of `|>` becomes a `Function` of a fresh `x` whose body is the `case` node for `x |> { … }`. Resolve sets the label of that parameter to `None` itself and does not use `label_of`, which would give the label `x`, so the fresh name is never seen, in calls by label or in type schemes. The `Function` has the clause block span; the `case` spans only its clauses, or has a zero-width span after `{` when empty. The fresh binder and use have zero-width spans at `{`. `query.rs` leaves both out of completions, hover, and definitions (A1 §3.9 and §3.12).
  - Whether a binding is a direct value is decided on the syntax tree, before this lowering: `resolve::is_direct` matches `ast::ExprKind`, and `hir::Binding.direct` comes from it. `is_direct` returns true for `ast::ExprKind::Clauses`, and for a group of one around it, as for a function expression. Without this, `loop = { 0 -> Done  n -> loop(n - 1) }` of §6.12 is not direct: at the top level, `loop` is among its own needs and is not generalized, and as a local binding, no frame for function bodies is pushed, so `loop` does not resolve in its clause expressions.
  - A block with one clause, no guard, and choices waits for T1. A1 reports E0235, by the accepted D7 default; it does not change `infer.rs` or `lower.rs`. The migration splits an old one-clause case with choices into separate clauses, preserving its coverage and known-constructor rules (A1 R7b). The pending blind case retains the intended success expectation for T1.

  The rule that makes a `case` node of any other match keeps the known types, narrowing, §7.5 known constructors on the name `e`, tail positions, and the order of evaluation exactly as they are today. The rule for one clause with no guard and no choices gives what a binding gives today: known types by fitting, and the tail position of `body`. It gives no known constructor on the name `e`, as §6.12 and §7.5 say.
- **Deciding each `{` once.** The test of §6.1 that decides whether a `{` begins a clause block reads the first statement, and may read it again. That statement may hold another `{` that needs the same test, so with no care the work grows by a factor at each level of nesting, and the LSP parses on every edit. The parser keeps the decision for each `{` by its token index (the lexer's `close` table gives the key), so each `{` is decided once.
- **Declared type variables.** Today the parser makes every lowercase name in a type a `TypeKind::Var` and every `UpName` a `TypeKind::Named`, resolve sends every `T::Named` to `lookup_type`, and `annotation_vars` collects the variables of a binding from its annotations. A declared variable is now an `UpName`, so with no change, `Option(A) { Some(A) None }` reports that `A` is not used (E0221, `TYPE_PARAM_UNUSED`), and `id(A) = (x: A) -> A { x }` reports that the type `A` is not known. A1 changes these places:
  - The syntax tree: `ast::Binding`, which holds the top-level bindings and `Stmt::Bind`, gets a field `ty_vars: Vec<Ident>`, empty when the binding declares none, and `ItemKind::Declaration` gets the same field. `ast::TypeDef` keeps `params: Vec<Ident>` for a head of type parameters (§4.3); the parser reads the head as types, fills `params` when every part is an `UpName` with no module name and no arguments, and retains the head otherwise. Resolve reports the error of §4.3 when it is not a parameter list: for an alias, that the head is kept for alias patterns, and for a custom type, that a type parameter is an `UpName` (A1 §3.7 and §3.11). When a parameter is a type that may be written with no module name, resolve reports the error of §4.3: for an alias, that the head is kept for alias patterns, and for a custom type, that a type parameter may not have the name of a type. Until A3, a head whose parts are all plain lowercase names is read as old type parameters, as today, with `TypeKind::Var` for them in the body, and is not an error. So `Result(a, e) {` in `lib/prelude.i`, `Option(a) {` in `std/src/option.i`, and `Dict(k, v) {` in `std/src/dict.i` still compile on the A1 branch, and `i test` and `wasi-smoke` pass there. A head that mixes lowercase names and `UpName`s is the error that a type parameter is an `UpName`. For a head of only lowercase names, that error holds only from A3 on (Notes for A3).
  - The frame of a declaration: in `values()` (about `resolve.rs:1688`), the branch for `ItemKind::Declaration` builds its frame with `type_vars(ty)`, which takes only lowercase `TypeKind::Var` names. So with no change, `inspect(A): (A) -> String` pushes an empty frame, `A` goes to `lookup_type` and is an unknown type, and after A2, `catch(A, E, N): …` in `lib/i/fail.i` does not compile. A1 changes this branch: when the declaration has `type_params`, it checks that they have different names (§4.7) and pushes the frame from them, so that their variables are the `ty_vars` of `DefKind::External`, and the declaration is general (§4.7). It uses `type_vars` only for a declaration with no `type_params`, for the old lowercase variables, until A3.
  - The parser does not know which names are declared, so it keeps making `TypeKind::Named` for an `UpName`. In resolve, the `T::Named` branch of `fn ty` (about `resolve.rs:1332`), for a name with no module name, first looks for it in the declared frames (`ty_frames`, innermost first) and in the parameters of `ty_def`, and marks a parameter as used, as `ty_var` does today. When it is found, the type is that variable, and arguments after it are the error of §3. Only when it is not found does the branch call `lookup_type`. The plan gives this branch to T2, not to A1, so the coordinator adds it to the range of A1 in the split of files, and T2 is rebased on it.
  - `push_ty_frame` makes a new variable for every name declared on the binding, also when an outer frame has a variable with that name, so that an inner declaration hides the outer one (§4.7). Without this, in `f(A) = (x: A) { g(A) = (y: A) { y }  g(1) }`, the `A` of `g` would be the `A` of `f`, and `g(1)` would make it `Int`. The names declared on one binding are checked to be different (§4.7) before the frame is pushed.
  - For a binding with type parameters, `annotation_vars` is not used. It stays only for the old lowercase variables (`T::Var`) while the parser accepts the old forms, and A3 removes it, with `type_vars`, `pattern_annotations`, and `expr_annotations`.
- **Showing type schemes.** Shown type schemes use capital letters: `id : (x: A) -> A`. Error variables are `E`, `E2`, … and environment variables are `N`, `N2`, …, as the lowercase forms do today. The names are given in order, as today, and do not keep the names declared on the binding. A name that is a type that may be written with no module name in the module (a type of the module, an imported `UpName`, or a built-in type) is skipped, and the next name is taken, so in a module with a type `E`, the first error variable is shown as `E2`, and with a type `A`, the first type variable is shown as `B`. The hover text of built-in types does not go through the namer: `builtin_head` in `crates/ilang-check/src/query.rs` writes `List(a)`, `Boundary(r, e, n)`, and `Continuation(a, r, e, n)` (from `PARAMS`). It is changed to write `List(A)`, `Boundary(R, E, N)`, and `Continuation(A, R, E, N)`. This is i code in a Rust string, so A2 changes it with the namer, though the plan gives the rest of `query.rs` to T2 and A3.
  - The namer in `crates/ilang-check/src/types/display.rs` and the interface printer in `crates/ilang-cli/src/commands/package.rs` are changed in phase 0 of A2, as the plan says. Rust tests that expect a shown type with lowercase variables fail then. Their strings are shown types, not i source, so the migration does not reach them, and phase 0 rewrites them by hand in the same commit. They include `app/main.id : (x: a) -> a` in `crates/ilang-cli/tests/check.rs`, `Some: (a) -> Option(a)` and forms such as `f: (a) -> b ? e ! n` in `crates/ilang-check/tests/query.rs`, and the hover tests in `crates/ilang-lsp/tests/protocol.rs`. Phase 0 finds the others with a search of the Rust tests for strings such as `-> a`, `(a)`, and `? e ! n`, and by running the tests. The checker `.out` files that hold shown types are made again.
- **Renaming type variables in the formatter's migration.**
  - Turn a lowercase variable into an `UpName`: take out each `_` and make the letter after it a capital, and make the first letter a capital. So `a` becomes `A`, `key` becomes `Key`, `e2` becomes `E2`, `key_type` becomes `KeyType`, and `_a` becomes `A`.
  - Declare it on the binding that owned it under the old §4.7 rule. When that binding has a type on its pattern, the declaration goes after the name and before the `:`: `id: (a) -> a = (x) { x }` becomes `id(A): (A) -> A = (x) { x }`. The labels stay as they are, because the annotation is not moved.
  - The bindings are renamed from the outside in, and the variables of one binding in the order of their first use. A new name **clashes** when it is a type that may be written with no module name in the module, such as `result` becoming `Result`; when it is the new name of another variable of the same binding, as `ab` and `a_b` both becoming `AB`; or when it is a variable already declared in the file that would be captured, or the new name of a variable of a binding around this one that is used inside this one, as in `f = (x: ab) { g = (y: a_b) -> ab { x }  g(1) }`, where `g` would take `AB` from `f`. A name that clashes is replaced by the first of `A1`, `A2`, … that does not clash, and the rename is reported.
  - A type variable in a declaration with `@external` had no binding that owned it. Declare it after the name of the declaration, with the variables in the order of their first use in its type: `catch: (body: () -> a ? e ! n) -> Result(a, e) ! n` becomes `catch(A, E, N): (body: () -> A ^ E ! N) -> Result(A, E) ! N`. This is needed for `inspect` and `hash` in `lib/i/core.i`, `catch` in `lib/i/fail.i`, and `boundary`, `resume`, and `cancel` in `lib/i/control.i`, and A2 searches for other such declarations.
  - When the owner pattern is not one name, the migration never declares its variable on an outer binding: that would reduce generality. It splits an all-direct tuple value into separate bindings only when the names are distinct and free of cross-dependencies; otherwise it refuses the file as `tyvar-owner`. A variable with no owner is renamed and declared nowhere, with a `hand` entry `tyvar-unowned` for E0222 → E0202. Mixed reserved heads whose rename would create a valid parameter list are refused too (A1 R4 and R10).
- **Matches in operator expressions.** The formatter's migration puts in a group (§6.9) every old `case` that is an operand of an operator of level 7 or 8 or of `-` or `-.`, or that has `.f`, `.k`, or a call after it. So `1 + case n { … }` becomes `1 + { n |> { … } }`, and `case x { _ -> 1 }.y` becomes `{ x |> { _ -> 1 } }.y`. It also puts the subject `S` of an old `case S { … }` in a group when the top operator of `S` is at level 1 to 5 (`||`, `&&`, `==`, `!=`, a comparison, or `<>`), because `|>` is stronger than these (§6.9): `case S { … }` becomes `{ S } |> { … }`. Without the group, `case times > 1 { … }` in the example `retry` of §9.1 would become `times > (1 |> { … })`, and `case a <> b { … }` would become `a <> (b |> { … })`, which may be correct code with another meaning. This is needed in §9.1, in `std/src/float.i`, and in the tests with `case got == want { … }`. For an old case on the left of any binary operator, it also groups the whole match to preserve stop-site count (A1 R2c). This rule applies only to old cases: a new one-clause match may give its subject an expected type. It puts each clause or statement whose output starts with `(`, `-`, `-.`, or `{` on a new line (§1.7 and A1 R8); splice mode refuses the insertion when the file has CRs.
- **Guards after function types.** When a clause has a guard and its pattern ends in a type whose last part is a function type, the migration puts that function type in `{ }` before it writes `?` (§7.1). So `f: (Int) -> Int if ok -> 1` becomes `f: { (Int) -> Int } ? ok -> 1`, and `x: Int | (Int) -> Int if g -> …` becomes `x: Int | { (Int) -> Int } ? g -> …`. Without this, the `?` would be read as the error part of the function type, and with `! N` in the type, a `?` after it would be a compile error.
- **First clauses that give a function.** For one first pattern and no guard, a chain `n1 = … = nk = ( … )` followed by a function expression may be read as a function binding (§6.1 test 2). The migration groups that expression when its function has no `->` part. With a `->` part, a group alone is a clause block, and an untyped `g = F` would lose the expected type of the clause expression. The file is therefore refused as `fn-expected-type`; the hand edit writes the expected type on `g`. Extraction is safe only where the function has no expected type, as A1 R5 and R6 specify. Choices or a guard prevent the immediate-arrow ambiguity, so this rule does not group them.
- **Functions with a `->` part at the start of a `{`.** When the first statement of a `{` that begins an expression in the migrated code starts with a function that has a `->` part and only patterns as parameters, the `{` would be read as a clause block (§6.1). This holds for old blocks and groups, for the groups that the migration adds, and for the subject of an old `case` that becomes the first statement of such a `{`. The migration binds the function to a name first, as a new statement before it, and puts the name in its place: `r = { (x) -> Int { x + n }(1) }` becomes `r = { g = (x) -> Int { x + n }  g(1) }`, and `{ case (x) -> Int { x }(1) { _ -> 0 } }` becomes `{ g = (x) -> Int { x }  g(1) |> { _ -> 0 } }`. The name is the first unused `g`, `g2`, `g3`, … in that scope. R5 extracts only where the function has no expected type (A1 §3.14); otherwise it refuses as `fn-expected-type`. The body of a function is not such a `{` and does not change.
  - An old block such as `r = { (x) -> Int { x + n }(1) }` is correct old code, but the parser that accepts both syntaxes reads it by the tests of §6.1, as a clause block with the pattern `(x)` and the expression `Int`, and then fails on `{ x + n }(1)`. There is no keyword in it to show that it is old. So the migration does not read old files with that parser. It reads them with a mode of A1's parser that takes only the old forms, in which a `{` that begins an expression is always a block, as in the old syntax, and no new form is read. The formatter then writes the new syntax with the rewrites of these notes. `i fmt` on a file in the new syntax uses the parser for both, as usual, and needs no such rewrite, because there such a `{` is already a clause block. `format()` and ordinary `i fmt` read Both mode only and give the `--from-old` hint on an Old-only parse. `i fmt --from-old` and the splice path read Old mode. `--keep-layout` selects the splice path. `--report` writes the plan, and `--no-choices-split` refuses R7b (A1 D10 and §3.14).
- **Bare functions in prose.** An example function in prose that uses type variables and is not bound, such as `(x: a) -> a { x }` in §7.3, has no binding to declare them on. A2 writes it as the value of a binding that declares them: `id(A) = (x: A) -> A { x }`.
- **Tests whose meaning changes.** A `case` with one clause and no guard used to have to cover its value, and read `x: T` as a test. After the change, the same clause block is fitted, reports to `Assert`, and reads `x: T` as an annotation when it has no choices. So the checker tests that expect the old errors are rewritten, and their `.out` files change:
  - Tests that expect a "not covered" error from one clause, when they are actually not covered, get a second clause that still does not cover the rest, or are moved to tests of the `Assert` behavior. The machine finder `scripts/a2-meaning` joins the migration report with expected diagnostics, producing the hand list before A2 migration (A1 R7c). The samples in `tests/spec` are searched in the same way. `tests/spec/7/not_covered.i`, the one sample of E0516, has `case c { Red -> "red" }`; it becomes `c |> { Red -> "red"  Green ^ False -> "green" }`, so that it still does not cover `Color` and §7.4 keeps its sample.
  - Tests that expect "type variable in a type test" from one clause, such as `bad = (v) { case v { x: a -> x } }` in `crates/ilang-check/tests/types/case.i`, get a second clause, so that `x: A` is still a test.
  - Tests of one clause with choices, such as those in `crates/ilang-check/tests/types/known_choices.i`, keep their meaning only where fitting the choices gives the same types as rule 11; their `.out` files are checked again. When the form waits for a later stream, these tests, and `case.i:66`, `choices.i:5`, and `known.i:49` and `known.i:68` in the same folder, are written with one clause for each choice, as the migration does, and keep their meaning.
  - Code that used a known constructor from a one-clause `case` on a name, as in `case s { Circle(..) -> s.r }`, no longer has it (§7.5). The migration refuses the file as `one-clause-known` for a reviewed hand edit. It never renames uses of the subject or invents `s = p` (A1 D13 and R7a). The covering `case r { Ok(x) -> x }` cases in `coverage.i` keep their meaning; they are not missing-coverage tests (A1 R7c).
- **Syntax error tests with no new form.** Some golden tests of syntax errors are correct code, or a different error, in the new syntax, so A2 removes or replaces them:
  - `crates/ilang-syntax/tests/golden/err_module_path.i`, `import (fail)`, becomes `(fail)`, which is an exports group, and the error becomes "`fail` is exported but not defined". The test moves to the checker. When an exported name that is not defined is the last part of a module path that the package may import, the message says that a module path has a `/`, as in `(i/fail)`.
  - `crates/ilang-syntax/tests/golden/err_case_brace.i`, `f = case x` with `{` on the next line, becomes `f = x |>` and then `{ … }` on the next line, which is correct, because a line that ends in an operator goes on (§1.7). The test is removed.
  - `crates/ilang-syntax/tests/golden/err_clause_tuple_type.i`, `p |> { p: (Int, Int) -> e }`, now gives the error that `( name: e )` is kept for records, with the note on tuple types (§7.1). Its `.out` file changes in A2; old-syntax errors stay unchanged in A1.
  - `crates/ilang-syntax/tests/golden/err_pattern.i`, `x |> { -y -> 1 }`, and `crates/ilang-syntax/tests/golden/err_segment_type.i`, `b |> { <<x: Int>> -> 1 }`, keep their errors in the pattern, because a first statement that is not a correct expression and has a `->` after it is a clause whose pattern is wrong (§6.1). Their `.out` files are checked again.
  - `crates/ilang-syntax/tests/golden/err_block_end.i`, line 3, `h = {}`, expects "a block ends with an expression", but `{}` is now a correct clause block (§6.12). The line is removed, and a test of `{}` is added to the checker tests.
  - The former `@test()` fixture is now an ordinary unknown-attribute rejection (E0301). Attribute-call syntax itself remains general for macros.
  - `crates/ilang-syntax/tests/golden/err_toplevel.i`, line 1, `f(x)`, expects "expected an item", but it is now a `MacroItem`, which is kept for macros (§2.2). The line is replaced by a form that is still not an item, such as `1`, and the `.out` file is checked again. Lines 3 and 4, `@external("m", "f")` and then `g = 1`, expect "expected `:`", but an item is now a declaration by its form, not by `@external` (§2.2), so they are a binding with an attribute in the wrong place, which the parser accepts. They are removed, and a test of `@external` on a binding is added to the checker tests.
  - `crates/ilang-syntax/tests/golden/err_import_late.i`, `import (i/fail)` after an item, becomes `(i/fail)` after an item, which is a group in `( )` at the top level that is not an item (§2.2). It is still an error, and the message still says that the imports come first, but the `.out` file changes.
  - `crates/ilang-syntax/tests/golden/err_trailing.i`, line 1, `T(,) { C }`, expects "expected a name", but the parser now reads the head as types (§4.3), so the error expects a type, and the `.out` file changes.
  - `crates/ilang-syntax/tests/golden/err_attribute_line.i` is `@deprecated`, then `("x")` on the next line, then `main = () { Nil }`, and expects E0101 at 2:1, because the arguments of an attribute may not come after a line break. It has no keyword, so the migration does not change it, but by the new §2.2, `("x")` is now the item after the attribute: a group in `( )` with an attribute before it, which is not followed by `=`, so it is a compile error of another kind. The file is kept for that error, and its `.out` file changes. The rule that it was written for is tested again by two new tests: `@m` and then `(even, odd) = …` on the next line, the example of §2.2, which the parser reads as an attribute and a binding, and the same on one line, `@m (even, odd) = …`, which is the golden error of an attribute `@m(even, odd)` followed by `=`.
- **Empty imports and exports.** The old syntax allows `import ()` and `export ()`, but an empty group is a compile error in the new syntax (§2.2). The migration removes an empty `import ()` or `export ()`. `crates/ilang-syntax/tests/format/imports_empty.i` loses its empty group; `crates/ilang-syntax/tests/golden/err_header_empty.i` tests the error for an empty group. The test in `crates/ilang-lsp/tests/protocol.rs` that changes `export (double)` into `export ()` removes the exports group instead, so that it still tests a module with no exports.
- **The canonical printer.** `crates/ilang-syntax/src/print.rs` is not the formatter. It prints a syntax tree in the old syntax, with `import (`, `export (`, `case `, and ` if `. The golden `.out` files are its output, and `tests/golden.rs` and `tests/spec.rs` check that what it prints parses back to the same tree. A clause block that is not on the right side of `|>` has no old form, so from A1 on, print.rs prints each node of the new forms in the new syntax, with the groups that the new syntax needs so that the text parses back to the same tree: a match that is an operand of an operator of level 7 or 8 or of `-` or `-.`, or that has `.f`, `.k`, or a call after it, is put in a group, and so is a subject whose top operator is at level 1 to 5 (§6.9). For example, the tree of `{ n |> { 0 -> 1  _ -> 2 } } * 10` is not printed as `n |> { 0 -> 1  _ -> 2 } * 10`, which parses as `n |> { … }`. It also puts each clause or statement whose pattern starts with `(`, `-`, or `-.` on a new line (§1.7), and groups a first clause expression only where §6.1 requires it (A1 §3.13). The printer never adds a binding. `print::unprintable` reports core trees that have no same-code surface text, including typed non-final choices; it never prints different code to make them parse. Until A3, it prints each node of the old forms in the old syntax, which A1's parser still reads. Every parsed tree with no diagnostic round-trips strictly. The migration alone compares `canon(parse(out))` with `canon(apply(plan, old))`; canon removes only expression groups. A core tree with no same-code text is an error (A1 D12, D15 and §3.13). A3 removes the old forms from print.rs with those of the parser.
- **Edits outside the ranges of A1.** The plan gives A1 `crates/ilang-syntax/src/*` and its tests, the parts of `resolve.rs` on type variables (about `:206-325`, `:1382-1407`, and `:1742`), and the new branch that lowers the new forms (about `:2007`). These notes also need these edits outside those ranges. The coordinator adds each to the split of files, as for the `T::Named` branch, and the owner of that place is rebased on it:
  - the `T::Named` branch of `fn ty` (about `resolve.rs:1332`), and the checks of §4.3 on the head of a type definition and an alias, in the code of type definitions and aliases of `resolve.rs`, which the plan gives to T2;
  - `is_direct` (about `resolve.rs:1791`), which returns true for a clause block;
  - the branch for `ItemKind::Declaration` in `values()` (about `resolve.rs:1688`) and `binding()` (about `resolve.rs:1760`), which build the frames of type variables;
  - in `crates/ilang-check/src/query.rs`, which the plan gives to T2 and A3: A1 leaves the fresh binder and use out of completions, hover, and definitions; A2 changes `builtin_head` with shown types;
  - `Scope::block` in `query.rs` searches `cover(b.span, b.value.span)`; `lower.rs` and `infer.rs` are excluded from A1 (D6);
  - in A2, the namer in `crates/ilang-check/src/types/display.rs`, which the plan gives to A2 phase 0 and otherwise to T1, and the compiler messages of "What A2 changes", among them one in `crates/ilang-check/src/types/pattern.rs`, which the plan gives to T1.
- **What A2 changes.** Every `case`, `if` guard, `import (` and `export (` changes in stream A2, and so does every type variable in i code (see the start of the replacement text). In §4, only definition heads and right sides change; types in the mathematical rules and notation of §0 stay lowercase. `Continuation(A, R, E, N)` in the §9.4 table is i code and changes (A1 §10). The migration reaches every `.i` file in the repository (`git ls-files '*.i'`), not only the places named here, in one of two ways, as the plan says:
  - `i fmt` rewrites every `.i` file that parses and is not sensitive to layout.
  - `format()` in `crates/ilang-syntax/src/format.rs` formats nothing in a file with a syntax error, and formatting a file that tests layout would change what it tests: a `format/*.i` input that is formatted is equal to its `.out` file and tests nothing, and formatting `binding_text_crlf.i` may change its `\r\n` line ends and the stop text that it checks. So these files are rewritten token by token with a one-time tool that keeps their layout: the 14 inputs `crates/ilang-syntax/tests/format/*.i`, the 48 `crates/ilang-syntax/tests/golden/*.i` (with the `err_*` ones, such as `err_pattern.i`, `err_segment_type.i`, `err_clause_tuple_type.i`, and `err_case_brace.i`, unless the note on syntax error tests above removes them), `tests/spec/1.1/binding_text_cr.i` and `binding_text_crlf.i`, and the samples in `tests/spec` with a syntax error. The tool makes the same changes as the formatter's migration: the new words and forms, the subject groups and operand groups of §6.9, the binding of a function to a name and the groups of §6.1, the new line before a clause or statement whose pattern starts with `(`, `-`, or `-.` (§1.7), and the new names of type variables. It does not change the number of CRs, and each `format/*.i` input stays different from its `.out` file.

  The places are:
  - the spec, the library, `std`, the `.i` tests and their `.out` files, the README, and the `i new` templates. The editor grammar and `editors/vscode/test/sample.i` move to A3 (A1 §9);
  - the `.i` files outside these: `examples/hello/src/main.i`, `examples/hello/test/main_test.i`, `bench/test/bench_test.i` (where `[y, ..ys2] -> case x > y {` needs the group of §6.9), and `tests/browser/fixtures/compiled/src/app.i`. The last is built into the checked-in `tests/browser/fixtures/compiled.wasm` by `scripts/build-abi-fixture.sh`, so after its source changes, A2 builds `compiled.wasm` again with that script and checks in the new file;
  - i source in Rust strings and in Markdown files, which the `.i` formatter does not reach. At `dd3771d` these are `crates/ilang-check/tests/{codegen,hir,query,types}.rs`, `crates/ilang-cli/tests/{commands,wasi,package_commands}.rs`, `crates/ilang-wasm/tests/codegen.rs`, `crates/ilang-lsp/tests/protocol.rs`, `crates/ilang-cli/src/commands/package.rs`, `bindings/javascript/README.md`, `docs/performance.md`, and `editors/vscode/README.md` (for example `import (i/core, c = (not) = i/core)` in `hir.rs`). A2 runs the migration on each such string, and checks again for others with a search for `case `, `import (` and `export (`. Some of these strings are `format!` templates, such as the `i new` templates in `crates/ilang-cli/src/commands/package.rs`, where `{{` and `}}` are i braces and `{name}` is a hole. They are not i source as written. A2 changes them by hand, and adds a test that renders each template and parses the result.
  - The Rust-source search also includes `crates/ilang-check/tests/library.rs`, its three `format!` templates and its position-bearing literal. Rust doc comments, including `crates/ilang-host/src/services.rs:8`, are changed by hand (A1 §8).
  - The tests in `crates/ilang-lsp/tests/protocol.rs` that point at places in the text: the line and column of each place are worked out again from the new text.
  - The `// at: spec/<file>:<line>:<column>` header comments of the samples in `tests/spec`. The migration does not change comments, but it moves the code they point at: `case S {` becomes `{ S } |> {`, which adds two columns, and the rule of one clause per line (§1.7) moves lines. So every such header is worked out again from the new text. For example, the header `:13:45` of `tests/spec/11/positions.i` becomes `:13:47`, and the header `:11:36` of `tests/spec/2.5/stop_in_start.i` becomes `:11:38`.
  - i code in doc comments (`///`, `////`) and in line comments of `.i` files, which the `.i` formatter does not reach and the LSP shows in hover. A2 searches the comments for `case `, ` if `, `import (`, `export (`, and lowercase type variables, and changes them by hand, with the groups of §6.9. For example, `case x < 3 { True -> … }` at `std/src/list.i:509` becomes `{ x < 3 } |> { True -> … }`, not `x < 3 |> { … }`, which is `x < (3 |> { … })`. Other places are `std/src/dict.i:327`, `Result(a, e)` at `std/src/result.i:1`, `List(a)` at `std/src/list.i:1`, and `std/src/set.i:4`.
  - The `// type:` comments in the `i` blocks of the spec, which show a type as the checker shows it. They are not i code, and the migration does not change comments, so A2 writes them in the new form of "Showing type schemes" by hand. At 229e3a4 there is one, in §9.6: `// type: (body: () -> a ? e ! n) -> Result(a, AssertionFailure) ? e ! n - Assert` becomes `// type: (body: () -> A ^ E ! N) -> Result(A, AssertionFailure) ^ E ! N - Assert`, the same as its copy in `crates/ilang-check/tests/types/spec.i`.
  - Compiler messages that name `case`, `if`, `import`, or `export`. They are not i code, so the migration and the searches above do not reach them. They change to the new words: "in a `case`, `x: T` tests the type constructor, …" in `crates/ilang-check/src/types/pattern.rs` becomes "in a test, `x: T` tests the type constructor, …", and "… not a function in the `export` list of a module …" in `crates/ilang-check/src/resolve.rs` becomes "… not a function in the exports of a module …". A2 searches the message strings of the compiler crates for these words, and the `.out` files are made again.

## Notes for A3

- **The change from the old syntax.** Until A3, the parser accepts both the old forms and the new ones. While the old forms are accepted, `import`, `export`, `case`, and `if` are still keywords, and a lowercase name in a type is still an old type variable. So these rules of the text above hold only from A3 on:
  - §1.4, that any `name` may be used as a name;
  - the bullet of §3 that makes a lowercase `name` in a type a compile error;
  - the error of §4.3 that a type parameter is an `UpName`, for a head of only lowercase names, which is read until A3 as old type parameters (Notes for A1 and A2);
  - two parts of §1.7. The old forms need two rules of the old §1.7 that the new text does not have, and A1's parser keeps every line rule of the old §1.7 for the old forms. So until A3, `if` keeps its old place in the two lists of §1.7: it cannot begin a statement, and a line that begins with `if` goes on from the line before, as in `case x {`, `1`, `| 2`, `if y`, `-> 3` in `crates/ilang-syntax/tests/golden/lines.i` and in `crates/ilang-syntax/tests/format/comments_middle.i`. And in the expression after `case`, a `{` that comes after a complete expression, and is not inside brackets, still ends the expression, so `case (a, b) { … }` is still a `case` on a tuple, and not a `case` on a function.

  A3 also removes E0113 (DualAmbiguous) and E0236 (mixed old and declared variables), plus their tests. E0113 is temporary for a lone clause block whose first pattern starts with `(` and first expression starts with a function, and for that clause block after `|` when followed by `(` or `.`; other matches obey the spec tests alone (A1 D4).

  A3 removes the keywords, the exception for `if` in `can_begin`, the old rule for `{` after the subject of a `case`, the old type parameters, the mode of the parser that reads only the old syntax, and the old forms from the parser, the formatter, and the printer, and adds the error for a lowercase name in a type, with a new error code, in the resolution of type names. The tests of A1 and A2 do not use these four words as names, and do not test the error for a lowercase name in a type.
- **Source at old tags.** A published version of a package is a git tag (§2.7), and the migration cannot change the source at a tag that is already published. A3 keeps no reader of the old syntax, so after A3, source at such a tag does not parse. Two commands meet it:
  - `i build` and `i check` with a dependency at a tag in the old syntax: the parse errors are reported as for any other file, and a note after them says that this version of the package is written in the syntax from before A3, that it cannot be used with this compiler, and that a newer version of the package, or a version of `i` from before A3, is needed. A dependency is not changed or moved to another version without the user.
  - `i publish`, which reads the source of the newest earlier tag to compare interfaces (`earlier_interface` and `compare_with` in `crates/ilang-cli/src/commands/package.rs`): when that source does not parse and every error is one of the errors that A3 gives for an old form, `i publish` says that the interface of that tag is in the old syntax and cannot be compared, and goes on as when the tag has no package ("nothing to compare with"). It does not ask for a new major version, because by §2.7 that would make a different package. The author is told to check the interface by hand.
  - A3 adds a test of `i publish` against an earlier tag in the old syntax, which expects this message and a published patch version, and a test of `i build` with a dependency at such a tag, which expects the note.
- **Words that are no longer keywords.** From A3 on, `import`, `export`, `case`, and `if` are normal names, so A3 changes the tests and checks that reject them as keywords:
  - `crates/ilang-check/tests/projects/package_modules/src/if.i`, which expects E0623, is removed or given another bad name, and `expected.out` changes.
  - `is_name` in `crates/ilang-package/src/lib.rs` no longer rejects these words, and the message in `crates/ilang-cli/src/commands/package.rs` no longer says "or a keyword". The tests that expect these words to fail, in `crates/ilang-package/tests/manifest.rs` (`name = "case"`) and in `crates/ilang-cli/tests/package_commands.rs` (`import` for `i new`, `case` for `i add`), use other bad names.
  - `crates/ilang-syntax/tests/lex.rs` checks that a line that begins with `if f` goes on from the line before, and that `case`, `import`, and `export` at the start of a line begin a statement as keywords. A3 takes `"if f"` out of the cases that go on, and tests the four words as names that begin a statement.
  - The keyword completions in `crates/ilang-check/src/query.rs` (the loop over `case`, `if`, `import`, `export`) and the doc comment that mentions keywords are removed. The tests that expect `case` among the completions, in `query.rs` and in `crates/ilang-lsp/tests/protocol.rs`, change.


## Reasons for the A1 note corrections

These are implementation-note corrections, not changes to the replacement
language text above. The accepted A1 note §10 gives the decisions and exact
mechanics. D5 reuses Case to keep existing visitors. D6 gives the synthetic Bind
its pattern span to keep the prescribed stop position without changing lowering.
D7 defers choices fitting to T1 because its inference and lowering are outside
A1. D10 avoids an Old fallback that could reinterpret new source. D12 checks an
independently applied plan; D15 requires strict parsed-tree round trips and
errors for unprintable core trees. R4 refuses moves that change generalization
or capture; R5 and R6 refuse extraction that loses an expected type; R7a refuses
known-constructor rewrites; R7c preserves negative-test meaning before A2.
R8 preserves line boundaries and CR count. R2c preserves stop-site count only
when migrating old cases. A2 changes shown names and both printers together;
A3 owns the editor grammar, removal of the old syntax, and old-tag handling.

The §2.2 attribute/group error skips the whole group and reports E0115 once.
An old `err_module_path` stays an old parser error until A2; its new checker
counterpart gives E0215 and the module-path hint only for an available module.
The old `err_trailing` head error expects a type. General attributes are parsed
as attribute calls; the removed test attribute is rejected by ordinary resolution. Layout-sensitive sources use the shared splice plan, and
syntax-error files are refused when their diagnostic codes would change.

WAT comparison follows A1 §3.17: exact ERROR text on shared exports, site-data
masking, reviewed owner categories, stable names for ownerless constructor and
library functions and import/async wrappers, and exact pattern-text pairs. New
blind cases have no base export: E1 lists their added exports; E2 and E3 compare
them with the same unformatted A1 source under the same compiler settings and
allowed categories, including exact ERROR text (the S4 coordinator ruling). An
ownerless function is always compared. The test-only `one-clause-as-case` flag
is forbidden in release builds and off in normal CLI tests. The report drives
both the A2 meaning finder and the E3 may-fail-position check; no blanket skip or
site renumbering is implied by a permitted category.

The compiled library is migrated in a compiled lane, never by a source-only
partition; AG rebuilds `tests/browser/fixtures/compiled.wasm` with its script.
The A2 exclusion list and its generated files are specified in A1 §8. A1 §9
also requires an updated editor grammar, scopes, sample, and regenerated
snapshot while keeping `grammar.test.js`; its query-completion removal includes
both checker and LSP tests.

The companion correction proposed for `docs/design/meta.md` §1 is still an
outside-scope handoff: list core trees with no same-code normal surface and make
`i fmt --to=i` report them as errors, as it does for quotes. The exact cases are
A1 §3.13 and §10. This track does not edit meta.md.

The A2 empty-header test placement follows a1-front-end.md §8: migration removes
empty groups. Formatter inputs must format without diagnostics, so the negative
case belongs in the parser golden `err_header_empty.i`, which A1 already added.
This corrects the implementation note above; §2.2 still rejects an empty header.
