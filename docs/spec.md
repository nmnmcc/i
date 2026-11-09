# The i Language

This document is the definition of the i language. A compiler or a program that does not keep to it is wrong. The present compiler does not yet do everything in this document.

This document is complete in itself. It defines:

- the source language (§1 to §8);
- effects, and the library modules `i/fail`, `i/control`, `i/resource`, and `i/assertion` (§9);
- the library module `i/core`, which has the operations that cannot be written in i (§13);
- packages, versions, and the files `i.toml` and `i.lock` (§2.1, §2.6 to §2.8);
- what programs do when they run, and the text form of values (§11);
- the binary interface between a compiled module and its host (§10 and Appendix B).

Every compiler must give the five library modules above. Other library modules, the command-line tools, and the way test results are shown are not part of the language.

i is small, and every part of it is what it looks like. A thing that looks like a function is a normal function. A binding is a binding wherever it is written. Where two forms look the same, they have the same rules.

## 0. How to read this document

The grammar is in EBNF: `|` is a choice, `[ ]` may be there or not, `{ }` may come zero or more times, and text in quotes is written as it is.

These words have a fixed sense in this document:

- **Compile error**: the compiler does not take the program and makes no output.
- **Stop**: the program ends at once while it is running. The host gets a message (§11.7). In a test, a stop makes that test fail, and the other tests go on.
- **Error**: a value that a function gives in place of its normal value (§9.3). An error is not a compile error.
- **Host**: the system that runs the compiled Wasm module, for example a web browser or Wasmtime.
- **Type constructor**: a type name with a fixed number of parameters. `List` has one parameter; `Int` has none.
- **Program source**: all modules but the library modules (§2.1). Built-in names (§12) are not in the program source.
- **Not fixed**: this document does not say which result is given. A compiler may give any result that keeps to the stated rules, but the same compiled module gives the same result every time.

## 1. Words and symbols

### 1.1 Source files

A source file is UTF-8 text, and its name ends in `.i`. `\r\n` and `\r` are the same as `\n`.

### 1.2 Comments

A comment starts with `//` and goes to the end of the line. A comment that starts with `///` is the documentation of the item after it. A comment that starts with `////` is the documentation of the module. Comments have no effect on the meaning of a program.

### 1.3 Names

```
name    = [ "_" ] lower { lower | digit | "_" } .
discard = "_" | "__" { lower | digit | "_" } .
UpName  = upper { upper | lower | digit } .
lower   = "a" … "z" .   upper = "A" … "Z" .   digit = "0" … "9" .
hex     = digit | "a" … "f" | "A" … "F" .   oct = "0" … "7" .   bin = "0" | "1" .
char    = (* any Unicode scalar value but '"' and "\" *) .
```

- A `name` is a value, a parameter, a label, a part of a module path, or a type variable. A name may start with one `_`, as in `_count`. It is a normal name in every way, but a compiler does not warn when it is not used.
- A `discard` is `_` alone, or a word that starts with two `_`, as in `__old_value`. It is a place in a pattern that binds no name; the word after `__` only says what is left out.
- An `UpName` is a type or a constructor.

### 1.4 Keywords

```
import  export  case  if
```

A keyword may not be used as a name.

### 1.5 Symbols

```
=   :   ,   .   ..   ->   ?   !   |   |>   @
(   )   [   ]   {   }   <<   >>
+   -   *   /   %   <   <=   >   >=   ==   !=   &&   ||   <>   &
+.  -.  *.  /.      <.  <=.  >.  >=.
```

Source text is cut into tokens from left to right. At every point, the longest token that matches is taken: `<=.` is one token, and `<<` is one token. Spaces (U+0020 and tab), line ends, and comments come between tokens, and some line ends are breaks (§1.7). After `.`, digits are read as a `decimal`, never as a float: `t.0.1` is `(t.0).1`. A token may not be followed directly by a letter, a digit, or `_` that would continue it as a word or a number: `12px`, `0xG`, `1.0E5`, and `fooBar` are compile errors.

### 1.6 Literals

```
int_lit    = decimal | "0x" hex { hex | "_" } | "0o" oct { oct | "_" } | "0b" bin { bin | "_" } .
decimal    = digit { digit | "_" } .
float_lit  = decimal "." [ decimal ] [ "e" [ "-" ] decimal ] .
string_lit = '"' { char | escape } '"' .
escape     = "\" ( "n" | "r" | "t" | "f" | '"' | "\" ) | "\u{" hex { hex } "}" .
```

- A literal is never negative. A negative number is made with the operator `-` (Int) or `-.` (Float) (§6.9). In a pattern, a negative literal is written in the same way (§7.1).
- `_` only puts digits into groups. It may not be first, last, or next to another `_`.
- The value of a float literal must be a finite IEEE 754 binary64 number. If not, it is a compile error.
- `\u{…}` has 1 to 6 hex digits. Its value must be a Unicode scalar value: 0 to 10FFFF, but not D800 to DFFF. If not, it is a compile error.
- An integer literal may be of any size.
- A string may go over more than one line. Line ends in it are changed as in §1.1.

### 1.7 Lines

A line end is a **break** when both sides of it allow one:

- the last token before it **can end** an expression: a name, an `UpName`, a discard, a literal, `)`, `]`, `}`, or `>>`; and
- the first token after it **can begin** a statement: a name, an `UpName`, a discard, a literal, `(`, `[`, `{`, `<<`, `-`, `-.`, `..`, `!`, `@`, or a keyword other than `if`.

Comments are not tokens, so the tokens on the two sides of a comment are what count. Every line end that is not a break is only a space.

A break ends something. What it ends depends on the brackets it is directly inside:

- Directly inside `{ }` (a block, the clauses of a `case`, or the constructors of a type), and at the top level of a file, a break ends a statement, a clause, a constructor, or an item. Attributes are part of the item after them, so a break after an attribute does not end the item.
- Directly inside a `{ }` that groups a type (§3), a break is a compile error.
- Directly inside `( )`, `[ ]`, or `<< >>`, a break is the same as a `,`. A `,` at the end of a line may be left out.

With no break, as on one line, a statement or a clause is the longest expression that may be read from its start, and a `,` is needed between parts. So statements and clauses may share a line: `case b { True -> 1  False -> 0 }`.

From this rule:

- When `(` is on the same line as the expression before it, it is a call (§6.6). When `(` is the first thing on a line, it begins a new statement or part.
- The `{` of a function body (§6.8) is on the same line as the `)` or the type before it.
- A line that ends in an operator, `,`, `=`, `:`, `->`, `(`, `[`, `{`, or `<<` goes on in the next line. A line that begins with `|>`, `.`, `|`, `?`, `->`, `if`, or a binary operator other than `-` and `-.` goes on from the line before.
- `a` and then `- b` on the next line are two statements. To write `a - b` on two lines, put `-` at the end of the first line. In the same way, `! N` of a function type may not begin a line.

Two rules do not depend on lines:

- In the `->` part of a function, after the `)` of its parameters, a `{` right after `->`, `?`, `!`, `|`, `&`, or `-` is part of the type (§3). The first other `{` that is not inside brackets begins the body.
- In the expression after `case`, a `{` that comes after a complete expression, and is not inside brackets, ends the expression. This comes before the rule of §6.1, so `case (a, b) { … }` is a `case` on a tuple.

## 2. Modules

### 2.1 Packages and modules

A **package** is a directory with an `i.toml` file in it (§2.6). Every `.i` file under its `src/` or `test/` directory is a **module** of the package.

The **path** of a module is the `name` of its package (§2.6), then `/`, then its path under `src/` or `test/`, without `.i`, with `/` between the parts. In the package named `app`, the path of `src/users/admin.i` is `app/users/admin`. Every part of a path must be a `name`. Two modules of one package with the same path are a compile error.

Modules in `test/` are only compiled when the tests of their package are run. They may import modules in `src/`, but not the other way.

In a module, the first part of a module path says which package the module is in. It must be one of these:

- `i`, for the five **library modules** (§9, §13), which every compiler gives. How it gives them is not fixed: the operations that cannot be written in i may be built into the compiler, and they are not host imports (§10.1);
- the `name` of the module's own package;
- a dependency name of the module's own package (§2.6); in a test module, also a dev-dependency name.

A module may only import modules that are named in this way. Then the rest of the path is the path of the module in that package, as that package names it. So with the dependency `json`, `json/decode` is the module `src/decode.i` of that package.

Each package gives its own names to the packages it uses, so there is no space of package names that all packages share. All modules of the packages that a program uses, directly or through other packages, are compiled together. A type or a value is named by its package (§2.7), the path of its module in that package, and its name. So two definitions with the same name in different modules or packages are different, and two packages that give one dependency different names still use the same types.

If modules import one another in a circle, it is a compile error.

### 2.2 Form of a file

```
File      = [ Import ] [ Export ] { Item } .
Item      = { Attribute } ( TypeDef | Binding | Declaration ) .
Attribute = "@" name [ "(" string_lit { "," string_lit } [ "," ] ")" ] .
```

A file has at most one `import` and at most one `export`. They come first, `import` before `export`. An item with `@external` is a declaration (§10.1). Any other item that starts with an `UpName` is a type definition (§4.3). Every other item is a binding (§2.5).

In one module, every top-level name is defined one time only. Two bindings may not bind the same name, two type definitions may not define the same type, and two constructors may not have the same name. A constructor and a top-level binding may not have the same name either, because they are in one space of names (§2.3).

These are the attributes. Any other attribute, an attribute in another place, or one attribute given two times on one item, is a compile error.

- `@external("module", "function")`: on a declaration (§10.1). The module may not start with `ilang:`, which is kept for the runtime (§B.4).
- `@host("namespace")` and `@host("namespace", "async")`: on a type definition (§9.7). The namespace may not start with `ilang:`.
- `@main`: on a binding whose pattern binds one name and whose value is a function expression (§10.2).
- `@test`: on such a binding in a module under `test/` (§10.2).
- `@deprecated("note")`: on any item. Using the marked item from another module gives a compile warning.

### 2.3 import

```
Import     = "import" "(" [ ImportItem { "," ImportItem } [ "," ] ] ")" .
ImportItem = [ name "=" ] [ "(" ImportName { "," ImportName } [ "," ] ")" "=" ] ModulePath .
ImportName = name | UpName .
ModulePath = name "/" name { "/" name } .
```

`import` and `export` are each followed by a tuple of items. As in every tuple, the items have `,` or a break (§1.7) between them:

```i
import (
  i/fail
  decode = json/decode
  opt = (Option, Some) = std/option
)

export (main, User(..))
```

Every item gives a name to a module:

- With no `=`, the name is the last part of the path.
- `alias = path` gives the name `alias`.
- `(A, b) = path` also puts the exported names `A` and `b` into this module. They may then be used with no module name before them.
- Both may be used together: `opt = (Option, Some) = std/option`.

Importing an `UpName` puts in the type and the constructor with that name, if they are there. Only exported names may be imported. Two imports may not put in the same name. An imported name may not be the same as a top-level name of this module.

Module names, value names, and constructor names are in one space of names. So a module name may not be the same as a top-level name or another imported name of the module, and a local name may cover a module name. In `a.b`, when `a` is a value, `.b` is a field; when `a` is a module, `a.b` is a member of the module. Type names are in a different space, so a type and a value may have the same name.

### 2.4 export

```
Export     = "export" "(" [ ExportItem { "," ExportItem } [ "," ] ] ")" .
ExportItem = name | UpName [ "(" ".." ")" ] .
```

- A top-level name that is not in the `export` list may only be used in its module.
- `T` exports the type `T` but not its constructors. Other modules may use the type, but they may not make or take apart its values. `T(..)` exports the type and all its constructors.
- Every exported name must be defined in this module.
- If the type of an exported definition uses a type defined in this module, that type must be exported too. If not, it is a compile error.

### 2.5 Top-level bindings

```
Binding     = Pattern "=" Expr .
Declaration = name ":" Type .
```

A top-level binding has the same rules as a binding in a block (§6.10). Only these things are different, and they come from its place:

- Its pattern may not begin with an `UpName`, because a top-level item that begins with an `UpName` is a type definition (§2.2).

- Its names may be used anywhere in the module, before or after it. So top-level functions may use one another.
- Its value is worked out once, when the program starts, before any entry point runs (§B.1). The order is below.
- It may not give errors or need an environment, because nothing is there to take them. So its pattern may not fail, because a pattern that may fail needs `Assert` (§4.8 rule 10).
- If working out a value stops the program, no entry point runs.

**Order.** Working out a direct value (§6.10) runs no code, so it only reads the names outside the function bodies in it. Working out any other value may call every function it can get to, so it may read every name in it, also inside function bodies. From this:

- The **full needs** of a binding are all the top-level names of this module in its value, also inside function bodies, and the full needs of their bindings.
- The **needs** of a binding with a direct value are the top-level names of this module outside the function bodies in its value, and the needs of their bindings. The needs of any other binding are its full needs.

A binding may not be among its own needs. If it is, it is a compile error: `x = [x]` and `x = f()`, where the body of `f` uses `x`, are compile errors, but `f = (n) { f(n - 1) }` is not. The values are worked out one at a time: each after its needs, and otherwise in the order of the source, with a module after the modules it imports.

A declaration may only be used with `@external` (§10.1).

### 2.6 The package file

The file `i.toml` is in TOML. These keys have a meaning; other keys are not used:

```toml
name = "app"                                   # required: the first part of the paths of this package's modules

[dependencies]                                 # packages that this package uses
json = "github.com/alice/json@1.4.0"           # a location, "@", and the lowest version needed
text = "codeberg.org/bob/tools//text@0.3.2"    # a package in a directory of a repository
util = { path = "../util" }                    # a directory on this computer, relative to this file

[dev-dependencies]                             # packages that only the tests of this package use
check = "github.com/carol/check@2.1.0"

[wasm]
exports = ["app/math.add"]                     # host exports (§10.2): module path, ".", function name
```

- `name` must be a `name` (§1.3), and not `i`. It is only used inside the package. A package may have any `name`, and the packages that use it give it their own dependency names.
- The key of a dependency is its **dependency name**. It must be a `name`, not `i`, not the `name` of this package, and not the same as another dependency name or dev-dependency name of this package.
- A **location** is the address of a git repository, without `https://`, for example `github.com/alice/json`. The repository is got with git from `https://` and the location. A location may end with `//` and a directory path, for a package that is in that directory of the repository. Any git host may be used; there is no central list of packages.
- A version is `major.minor.patch`, three decimal numbers, as in Semantic Versioning.
- `{ path = "…" }` is a package in a directory on this computer. Only the package that is compiled, and the packages that it gets by path, may have path dependencies. A package that is got by location may not have them.
- The packages that a package uses, directly or through other packages, may not be in a circle.
- The `dev-dependencies` of a package are only used when its own tests are compiled. The `dev-dependencies` of other packages are not used.

### 2.7 Versions

A **version** of a package with a location is a git tag `vmajor.minor.patch` in its repository, for example `v1.4.0`. The package is the files of the repository at that tag, or the files of the directory after `//`, with its `i.toml` at the top.

A **package** in a program is a location and a major version, or a directory for a path dependency. So `github.com/alice/json@1.4.0` and `github.com/alice/json@2.0.1` are two different packages, and their types are different. A program may use both, with two dependency names. Inside one major version, a newer version must be able to take the place of an older one.

A program uses one version of each package. It is chosen by **minimal version selection**:

1. Start with the dependencies of the package that is compiled, and, when its tests are compiled, its dev-dependencies. Each one is a package and a lowest version.
2. For each package and version that is reached, read its `i.toml` at that version. Its dependencies are reached too, each with its lowest version.
3. For each package, the version used is the highest version that is reached.

So the versions used change only when an `i.toml` changes. A newer version that is put on a repository is not used until a package asks for it.

### 2.8 The lock file

The file `i.lock` is next to the `i.toml` of the package that is compiled. It has one line for each version of a package with a location that the program uses, when it is compiled with or without its tests, in the order of locations and then versions as UTF-8 bytes:

```
github.com/alice/json 1.4.0 9c3e1f0a5b7d2c4e8f6a1b3d5c7e9f0a2b4c6d8e sha256-6d2f…
```

The parts are the location, the version, the git commit of the tag, and the **hash** of the package, with one space between them. The hash is `sha256-` and the lowercase hex SHA-256 of these bytes: for each file of the package, in the order of their paths as UTF-8 bytes: the path from the package directory, with `/` between the parts; a zero byte; the size of the file as a decimal number; a zero byte; the bytes of the file. The files of the package are the files in the git tree of the commit under the package directory, with their bytes as git keeps them, before any change that a checkout makes to line ends. A symbolic link is a file whose bytes are its target. Submodules are not part of the package.

- When the files that are got for a package do not have the commit and the hash in `i.lock`, it is an error, and nothing is compiled. So a tag that is moved, or files that are changed on the way, are found.
- When `i.lock` has no line for a version that is used, the line is added. Lines for versions that are no longer used, with or without the tests, are taken out. So when a version is raised, the line for the old version is taken out and a line for the new one is added.
- A tool may get the files from the repository or from any copy of it, because the hash makes certain they are the same files.

## 3. Type syntax

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

- A `name` is a type variable. `_` is a hole that type inference fills in.
- `A | B` is a union, `A & B` is an intersection, and `A - B` is a difference (§4.4). `-` is stronger than `&`, and `&` is stronger than `|`.
- `( … )` with `->` after it is a function type. Without `->`, it is a tuple type.
- A parameter of a function type may have a label: `(in: String, each: String) -> String` (§4.6).
- A function type goes as far to the right as it can: `(A) -> B | C` gives `B | C`. `{ }` puts a type in a group: `{ (A) -> B } | C`, `(a) -> { (b) -> c } ! E`.
- `?` and `!` are part of the nearest `->`. `?` comes before `!`. A `?` or `!` that the nearest `->` cannot take, because it already has one or because `!` came first, is a compile error: in `(A) -> (B) -> C ! N ? E`, the inner function type is put in `{ }` to give `? E` to the outer `->`.
- In a written function type, a missing `? E` means `? Never`, and a missing `! N` means `! Never`. This is the same in every place where a type is written.

## 4. Types

### 4.1 Types are sets of values

Every type is a set of values.

- Values of different type constructors are never the same.
- Values of different forms of one type constructor, such as `List(Int)` and `List(String)`, may not be told apart at run time (§7.1).
- Tuples with different numbers of parts are different. The values of a function type are functions.

### 4.2 Built-in types

These types may be used in every module:

| Type | Values |
|---|---|
| `Int` | Whole numbers of any size |
| `Float` | Finite IEEE 754 binary64 numbers |
| `String` | Lists of Unicode scalar values, kept as UTF-8 |
| `List(a)` | Lists that may not be changed |
| `BitArray` | Lists of bits; the number of bits may be any number |
| `UtfCodepoint` | One Unicode scalar value |
| `Never` | No values |

These built-in types are normal custom types (§4.3) with these definitions:

```i
Bool { True False }
Nil { Nil }
Result(a, e) { Ok(a) Error(e) }
```

The types `Assert` and `AssertionFailure` are in §9.6. The types `Boundary` and `Continuation` are in §9.4.

### 4.3 Custom types

```
TypeDef     = UpName [ "(" name { "," name } [ "," ] ")" ] TypeBody .
TypeBody    = "{" { Constructor } "}" | "=" Type .
Constructor = UpName [ "(" Field { "," Field } [ "," ] ")" ] .
Field       = [ name ":" ] Type .
```

- `T { C1 C2 … }` is a custom type. Every constructor has zero or more fields. A field may have a label.
- `T {}` is a type with no constructors. i code may not make its values. They only come from the host (§10).
- `T = Type` is a type alias. `T` is the same as the type on the right. An alias may not use itself, directly or through other aliases.
- Every type parameter must be used on the right, and every type variable on the right must be a parameter.
- Custom types are **nominal**: two different definitions make two different types, even if they have the same form.

### 4.4 Union, intersection, and difference

- The values of `A | B` are the values of `A` and the values of `B`. Order has no effect, `A | A` is `A`, and `A | Never` is `A`.
- Members of a union with the same type constructor are put together into one. `T(A1, …) | T(B1, …)` is `T(C1, …)`, where for each parameter of `T` (§4.5 rules 6 and 7), `Ci` is `Ai | Bi` when the parameter goes in the same direction as the type, and `Ai & Bi` when it goes in the other direction. When the parameter does not change, `Ai` and `Bi` must be equal, and `Ci` is that type; if they cannot be equal, it is a compile error. Tuples of the same size count as one type constructor, and so do all function types, whose parameters go in the other direction and whose value, errors, and environment go in the same direction. Function types with different numbers of parameters cannot be put together; that is a compile error. A label is kept only where both have it. So `List(Int) | List(String)` is `List(Int | String)`.
- The **members** of a type are the types that make it up as a union, after these rules are used. So no two members of a type have the same type constructor. A type variable may be a member.
- The values of `A & B` are the values that are in both `A` and `B`. `{ A | B } & C` is `A & C | B & C`. When `T` and `U` are not type variables, `T & U` is `Never` when they have different type constructors; when they have the same one, it is that type constructor with each pair of arguments put together the other way round: `&` where the parameter goes in the same direction, `|` where it goes in the other direction, and equal where it does not change. `a & T`, with a type variable `a`, stays as it is.
- Intersections come from inference. `(x) { x + 1  x }` has the type `(x: a & Int) -> a`: the value that comes in is given back, and it must be an `Int`.
- `A - B` takes out of `A` every member whose type constructor is the type constructor of a member of `B`. These rules work it out:
  - `{ A | B } - C` is `A - C | B - C`, and `A - { B | C }` is `A - B - C`.
  - When `T` and `U` are not type variables: `T - U` is `Never` when they have the same type constructor, and `T` when they do not. Tuples of the same size count as one type constructor, and so do all function types.
  - When `T - U` is `Never`, `T <: U` must be true. This is a constraint for inference, like any other. When it cannot be true, it is a compile error: for example `List(Int | String) - List(String)`.
  - The right side of `-` may not have a type variable as a member. `a - U`, with a type variable `a`, stays as it is until `a` is known.
- So `{ Int | String | Nil } - String` is `Int | Nil`, `A - A` is `Never`, and `{ Yield(a) | n } - Yield(a)` is `n - Yield(a)`.
- Unions, intersections, and differences may be used in every place where a type may be used: values, errors, and environments. In a written type, a union with two or more type variables as members may only be in an output place, and an intersection with a type variable only in an input place (§4.5 rule 6), because inference cannot choose between them in the other places.

### 4.5 Subtypes

`A <: B` says that every value of `A` is a value of `B`. These are the rules:

1. `Never <: A`, and `A <: A`.
2. `A <: A | B`. `A | B <: C` when `A <: C` and `B <: C`. `A & B <: A`. `C <: A & B` when `C <: A` and `C <: B`.
3. `A - B <: A`. `X <: A - B` when `X <: A` and `X` and `B` are **disjoint**: every member of `X` has a different type constructor from every member of `B`. A type variable as a member is never disjoint from anything.
4. `(P1, …) -> R ? E ! N <: (Q1, …) -> S ? F ! M` when the numbers of parameters are the same, `Qi <: Pi` for every parameter, `R <: S`, `E <: F`, and `N` is **in** `M`: every member of `N` is a type variable that is a subtype of `M`, or has a member of `M` with the same type constructor and equal type arguments. Labels have no effect. Service arguments are compared as equal, not by direction, because a service is a value that goes in and comes out: with directions, a function that needs `Yield(Int)` could be given a `Yield(Never)`.
5. `(A1, …) <: (B1, …)` when the numbers of parts are the same and `Ai <: Bi` for every part.
6. A type parameter of a custom type changes in the same direction as the type, in the other direction, or not at all. This is decided from the definition: a parameter that is only in output places goes in the same direction; one that is only in input places goes in the other direction; one that is in both does not change. Field types are output places, the parameters of a function type are input places, and a place inside an input place is the other way round.
7. `List(a)` goes in the same direction as `a`. In `Continuation(a, r, e, n)`, `a` goes in the other direction and `r`, `e`, `n` in the same direction. `Boundary(r, e, n)` does not change.

Two types are **equal** when each is a subtype of the other. A value of a subtype may be used in every place where a value of the type is needed.

### 4.6 Function types

The function type `(T1, …, Tn) -> A ? E ! N` has three parts after the arrow. They are in the same order as `Effect<A, E, R>` in Effect-TS:

- `A`: the type of the **value** that the function gives back.
- `E`: the **errors** that the function may give in place of a value (§9.3).
- `N`: the **environment**: the service types that the function needs (§9.2).

`E` and `N` are normally unions, and `Never` means none. The number of parameters is part of the type. There is no currying and there are no default parameters. Environment parameters (§9.2) are not among `T1 … Tn`; their types are in `N`.

A parameter of a function type may have a **label**. Labels are not part of the meaning of a type: two function types that are the same but for labels are equal. A label is only used to put the arguments of a call in order (§6.6). The type of a function expression has the labels of its parameters (§6.11), and a written type has the labels written in it. When a type is made from function types that have different labels for one parameter, that parameter has no label.

### 4.7 Type inference

Types are worked out by inference with subtypes (algebraic subtyping).

- Type annotations may be left out everywhere for inference, but not on `@external` declarations. Some uses of a name need a known type, which may need an annotation (§4.8).
- For every binding, the compiler must find the principal type: the most general type, of which every other possible type is an instance. MLsub (Dolan and Mycroft, POPL 2017) and MLstruct (Parreaux and Chau, OOPSLA 2022) give ways to do this.
- A type is made simpler before it is shown: equal parts are put together, and type variables that have no effect on the meaning are taken out. This never changes which programs are correct or what they do. A type scheme that is used as a known type (§4.8) is in this simpler form.
- Annotations are used in inference, and the inferred type must agree with them. A type variable in an annotation may become a more exact type.
- The **annotations of a binding** are the types written in its pattern and its value, also those in the parameters, `->` parts, and clause patterns in its value, but not those inside an inner binding. An inner binding is a binding statement in a block inside the value; a parameter or a clause pattern is not one. A type variable written in an annotation of a binding is one variable of that binding, and it is generalized with it. But when the same name is also written in an annotation of a binding that has this one inside it, it is the variable of that outer binding. So two local bindings next to each other that both write `a` have two different variables.
- **A binding is generalized when its value is a direct value** (§6.10): the type variables of its type that are not in the types of names from outside it become general, so each use of the name may give them different types. A binding whose value is not a direct value is not generalized, because working out its value runs code, and that code may make a value for one type only (for example with `control.suspend`). This is the value restriction of ML, which is known to be safe. Functions are direct values, so every function binding is generalized.
- A type variable that is not generalized is one type for all uses of the binding.
- Parameters (§6.11) and clause patterns (§6.12) are matched like bindings, but they are not generalized. Their values are not known where they are written, so each of their names has one type in the whole body or clause. So `(f) { (f(1), f("a")) }` needs `f` to take both `Int` and `String`.
- Inside its own value (§6.10), a name of a binding is not general: all its uses there have one type. It becomes general when the binding is generalized.
- A **group** is a set of top-level bindings that use one another, directly or through other bindings of the group. The bindings of a group are worked out together and then generalized together, after the groups that they use. Inside the group, their names are not general.
- A type that has itself inside it, through a type variable, is a compile error.

### 4.8 Typing rules

`Γ ⊢ e : A ? E ! N` says: with the names in `Γ`, the expression `e` has the type `A`, may give the errors `E`, and needs the environment `N`. `Γ` gives every name a type scheme `∀a1 … an. T`. A program is correct when every binding has a type by these rules. Every rule may use any `E` and `N`; the parts of one rule use the same `E` and `N`, unless the rule says otherwise.

1. **Subtypes.** If `e : A ? E ! N`, `A <: A'`, `E <: E'`, and `N <: N'`, then `e : A' ? E' ! N'`.
2. **Literals** have the types of §6.3.
3. **Names.** If `Γ(x) = ∀a1 … an. T`, then `x : T` with each `ai` changed to any type.
4. **Functions.** Let the parameters be the patterns `q1 … qn`, and the environment parameters `x1 ! T1 … xk ! Tk`. Each `qi` is fitted to a type `Pi` (below). If `Γ` with the names of the `qi` and `xi : Ti` gives `body : A ? F ! M`, then the function has the type `(P1, …, Pn) -> A ? F ! M | T1 | … | Tk | S`, with the labels of its parameters. `S` is `Assert` when some `qi` may fail for `Pi` (§7.3), and `Never` when not. Making the function needs nothing, so this rule may use any `E` and `N` for the function expression itself.
5. **Calls.** If `f : (P1, …, Pn) -> A ? E ! N` and every `ai : Pi`, then `f(a1, …, an) : A`. Arguments with labels are first put in the places of §6.6.
6. **Tuples and lists.** If every `ei : Ti`, then `(e1, …, en) : (T1, …, Tn)`. If every `ei : T` and `r : List(T)`, then `[e1, …, en] : List(T)` and `[e1, …, en, ..r] : List(T)`.
7. **Constructors** are functions from their field types to their type, with the field labels (§6.3). Fields and updates follow §6.7.
8. **Operators** are not functions. Each has the types of §6.9. `a |> f(b)` has the type of `f(a, b)`.
9. **Blocks.** `{ s1 … sn e }` has the type of `e`, with the names bound by `s1 … sn` (§6.10). For an environment statement `! T = e1` and the rest `r` of the block after it: if `e1 : T ? E ! N1` and `r : A ? E ! M`, then together they have `A ? E ! N1 | M - T`. So `e1` uses the environment from outside the statement.
10. **Bindings.** For `p = e` with `e : A`: `p` is fitted to `A` (below). If `p` may fail for `A` (§7.3), then `Assert <: N`.
11. **case.** For `case e { … }` with `e : A`, the clause patterns (each choice of `|` counts) are looked at by **place**. The top place is the value of `e`. Below a pattern at a place, its parts are at places of their own: each field of a constructor `C`, each part of a tuple of n parts, and the parts and the rest of a list. Places below different constructors are different places. At each place, let `B` be the type of the values there (`A` at the top place, and else the part type of the pattern type above it), and let `P` be the union of the pattern types (below) of the patterns at that place that are not catch-alls. Patterns at one place with the same type constructor use the same new type variables, so `Ok(v)` and `Error(e)` at one place give one `Result(b1, b2)`. A type test `x: T` at a place also needs `B <: T | r - T`, with a new type variable `r`: it takes every value whose type constructor is that of a member of `T` (§7.1), so those values must be in `T`. So `case v { x: Box(Int) -> …  y: Box(String) -> … }` with `v: Box(Int | String)` is a compile error. If some clause without a guard has a catch-all at that place or at a place above it, then `B <: P | r - P`, with a new type variable `r`: a value that no pattern takes may still come there. If not, then `B <: P`. A catch-all at a place gets the type of rule 12. Every guard has the type `Bool`, and every clause expression has the type `R`. Then the `case` has the type `R`. Coverage (§7.4) is checked with the types that inference finds.
12. **Narrowing.** Let a catch-all be at some place in a clause pattern, and let `B` be the type of the values at that place. A type constructor `T` is **taken** at that place when every value that the clause pattern matches, with a value of `T` at that place, is matched by an earlier clause without a guard. The catch-all gets the type `B - H`, where `H` is the union of the taken type constructors, each with new type variables as its arguments. So in §9.3, `other` gets `HttpError`, because the clause before it takes every `Error` with a `ParseError` in it.
13. **Fields and tuple parts.** For `v.f`, every constructor that may have made `v` (§6.7) must have a field with the label `f`. The type of `v.f` is the union of the types of these fields, with the type arguments put in. For `v.k`, every member of the known type of `v` (below) must be a tuple with more than k parts, and the type of `v.k` is the union of their parts k. If not, it is a compile error. In both cases, the type of `v` must be a subtype of its known type.
14. **Generalization.** A binding whose value is a direct value is generalized (§4.7) after its value is typed. Other bindings, parameters, and clause patterns are not.

A **catch-all** is a `name`, a discard, or `x = q` where `q` is a catch-all.

To **fit** a pattern `q` to a type `B`, in a binding or a parameter: if `q` is a catch-all, its names get `B`. If not, `B <: Q`, where `Q` is the pattern type of `q`, and the parts of `q` are fitted to the parts of `Q` given below. Fitting never adds `Assert`. In a `case`, rules 11 and 12 take the place of fitting.

The **pattern type** of a pattern that is not a catch-all, and its parts, are:

- A literal: the type of the literal.
- `C(p1, …)`: `T(b1, …, bm)`, the type of `C` with a new type variable for each type parameter. The part for `pi` is the type of the field that `pi` is matched to (§7.1), with these variables.
- `(p1, …, pn)`: `(b1, …, bn)`, with new type variables. The part for `pi` is `bi`.
- `[p1, …, pn]` and `[p1, …, pn, ..r]`: `List(b)`, with a new type variable. The part for each `pi` is `b`, and `r` gets `List(b)`.
- `"lit" <> r`: `String`, and `r` gets `String`.
- A bit array pattern: `BitArray`. The parts get the types of their formats (§8.2).
- `x = q`: the pattern type of `q`, and `x` gets that type.
- `x: T`: `T`, with a new type variable in place of each `_` in it, and `x` gets that type.

The **known type** `K(e)` of an expression is found from the parts of the expression, left to right, and from the expected type of its place (below). Where no case below gives it, `e` has no known type.

- A literal has its type, and a bit array has `BitArray`. A tuple has the tuple of the known types of its parts, with `_` for a part that has none. A list has `List` of the union of the known types of its parts when they all have one, and `List(_)` when not. So `[]` has `List(Never)`.
- An operator expression has the result type of §6.9; `a |> f(…)` is the call `f(a, …)`.
- A `case` has the union of the known types of its clause expressions when they all have one. A record update `C(..v, f1: e1, …)` has the known type of the call of `C` with the given fields and the other fields of `v` as its arguments.
- The names of a parameter get known types as names in patterns (below), but the value of a parameter has no known type. So `x: T` gives `x` the type `T`, and a parameter with no type has no known type. An environment parameter `x ! T` has `T`.
- A built-in name, an imported name, and a member `m.x` of an imported module have their type scheme. So does the name of a generalized binding (§4.7), local or top-level, where it is used outside its own value and outside its group. A constructor has its function type, with the field labels (§6.3).
- A function expression has the type made from: the labels of its parameters; for each parameter, the type written at its top (as in `x: T`), or else the type it gets from an expected type (below), or else `_`; and for the result, the written `->` part, or else the known type of its body, or else `_`.
- A name used inside the value of its own binding, and a top-level name of the same group, get their known types only from what is written, because their values are not yet known there: from `x: T` at the top of the binding's pattern when it is there, or else from what is written in the value, with `_` for the rest. So, as in TypeScript, a function that uses its own result in `.f`, `.k`, or a call by label needs a written `->` part. Every other name bound by a pattern gets its known type as below.
- A known type may have `_` in it, for a part that is not known. A part that is `_` has no known type, but the rest of the type is still known. So the labels of a function are known even when its types are not.
- A call `f(a1, …)`, also when written with `|>`, has a known type when `f` has a known function type, and every type variable of its result gets a type when its parameter types are matched with the known types of the arguments (below). Then it has the result type with those types put in. A constructor with arguments is a call.
- To **match** a type `P` with a known type `K`: when `P` is a type variable, `K` is one of its types. When `P` is a named type, a tuple type, or a function type, and `K` has one member with the same type constructor, the parts are matched: the type arguments, the tuple parts, or the parameter and result types. When `P` is a union, each member of `P` that is not a type variable is matched with the member of `K` that has its type constructor, if there is one, and each type variable in `P` is matched with `K` less the type constructors of the other members of `P` (§4.4). A `_` in `K` matches nothing. Before matching, a difference in `K` is replaced by its members (§4.4), and an intersection `a & T` by `T`. When a type variable gets more than one type, it gets their union. So matching `(x: a | Nil)` with `User | Nil` gives `a` the type `User`.
- **Expected types.** An expression may have an **expected type** from its place, as in TypeScript:
  - An argument of a call (also with `|>`) whose callee has a known function type has the type of its parameter there. Its type variables are put in when matching the parameter types with the known types of the other arguments fixes them. Function expressions that have a parameter with no written type are left out of this matching.
  - The value of a binding whose pattern has `x: T` at its top has `T`. The body of a function with a written `->` part has that type.
  - The last expression of a block has the expected type of the block, and every clause expression of a `case` has the expected type of the `case`. A part of a tuple or a list has the matching part of the expected type, when that type has one member, a tuple or a list.
  - When a function expression has an expected type with one member, a function type with the same number of parameters, every parameter with no type written at its top is fitted to the matching parameter type, as for names in patterns below. This only gives known types. So `list.map(users, (u) { u.name })` is correct when `users` has a known type.
- `v.f` and `v.k` have the field or part type of rule 13, when it has no type variable that is not put in.
- A block has the known type of its last expression.
- **Names in patterns**: when a pattern `q` is fitted to a value with a known type `B`: a catch-all gets `B`, less the members that rule 12 takes out. If `q` is not a catch-all, take the one member of `B` with the type constructor of `Q`; if there is not exactly one, the names of `q` get no known type. The parts of `q` then have the known part types of that member, with its type arguments put in. The name `x` of `x = q` gets that member. The name `x` of `x: T` gets `T`, whatever the value. So in `f: (x: Int) -> Int = (y) { … }`, `f` has the label `x`, from its annotation.

So `(u) { u.name }` needs a type, `(u: User) { u.name }`, but with `opt: Option(User)`, `case opt { Some(u) -> u.name  None -> "" }` is correct.

## 5. Names and scope

- Built-in names (§12) may be used in every module. Imported names and top-level names may cover them.
- A top-level name may be used anywhere in its module (§2.5).
- A local name may be used from the next statement to the end of its block. When the value of its binding is a direct value, it may also be used inside the function bodies in that value (§6.10). The names of a parameter may be used in the function body. A name in a pattern of a `case` clause may be used in its guard and its expression.
- A local name may cover an outer name with the same name, and an earlier binding in the same block.
- A `discard` makes no name.

## 6. Expressions

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
          | Block | Function | Case | RecordUpdate .
Name      = [ name "." ] ( name | UpName ) .
Tuple     = "(" [ Expr { "," Expr } [ "," ] ] ")" .
List      = "[" [ Expr { "," Expr } [ "," ".." Expr ] [ "," ] ] "]" .
Block     = "{" { Statement } Expr "}" .
Statement = Pattern "=" Expr | "!" NamedType "=" Expr | Expr .
Function  = "(" [ Param { "," Param } [ "," ] ] ")" [ "->" Union [ "?" Union ] [ "!" Union ] ] Block .
Param     = Pattern | name "!" NamedType .
RecordUpdate = Name "(" ".." Expr { "," name ":" Expr } [ "," ] ")" .
Case      = "case" Expr "{" { Clause } "}" .
Clause    = Pattern { "|" Pattern } [ "if" Expr ] "->" Expr .
```

- `( … )` that is not a call is a function when `{` or `->` comes after it, and a tuple when not (§1.7). So its parts are patterns in a function and expressions in a tuple. In a guard, a `->` that is not inside brackets always ends the clause head. So in `n if (a) -> 1`, the guard is the tuple `(a)`, which is a compile error because a guard must be a `Bool`, and not a function.
- A statement that starts with a pattern and then `=` is a binding. When more than one such pattern may be read, the longest is taken: in `a = (x, y) = e`, the pattern is `a = (x, y)`. A statement that starts with `!` is an environment statement. Every other statement is an expression.

### 6.2 Order of evaluation

i is strict. Expressions are worked out from left to right:

- In a call, first the function, then the arguments from left to right as they are written (also when they have labels), then the call.
- The parts of tuples, lists, constructor arguments, and bit arrays are done from left to right.
- In a binary operation, first the left side, then the right side. `&&` and `||` do not do the right side when the left side is enough.
- `a |> f(b)` does `a` first, and then the rest in the order of `f(a, b)`.

### 6.3 Literals and names

The type of an integer literal is `Int`, of a float literal is `Float`, and of a string literal is `String`. The value of a name is the value it is bound to. A member of a module is `module.name`, or an imported name (§2.3). A constructor with no fields is a value. A constructor with fields is a function whose parameters are its fields, with their labels.

### 6.4 Tuples

The type of `(e1, …, en)` is `(T1, …, Tn)`. n may be 0, 1, or more: `(x)` is a tuple of one part. `t.k`, where k is a decimal number, is part k, counting from 0. A k that is too big is a compile error. `{ }` puts an expression in a group (§6.10).

### 6.5 Lists

The type of `[e1, …, en]` is `List(T)`, where every part has the type `T`. `T` may be a union: `[1, "a"]` has the type `List(Int | String)`. `[e1, …, en, ..rest]` puts the parts in front of the list `rest`.

### 6.6 Calls

`f(a1, …, an)` calls the function `f`. Every function is called in this way, whatever expression gives it.

- An argument may be given by position or by label. Arguments by position come before arguments by label, and they go to the parameters in order, from the first. Arguments by label may be in any order.
- A call with an argument by label needs a known function type for `f` (§4.8). Each such argument is put in the place of the parameter with that label in this type. If there is no such parameter, it is a compile error.
- Every parameter must get exactly one argument. Environment parameters (§9.2) are not parameters of the type, so they get no argument.

### 6.7 Constructors and fields

`C(a1, …, an)` makes a value. It is a call (§6.6); the labels are the field labels.

`v.f` gets the field with the label `f` from the value `v`. The constructors that **may have made** `v` are the one constructor that `v` is known to be made by (§7.5), when there is one. If not, every member of the known type of `v` (§4.8) must be a custom type, and they are all the constructors of these types. Each of them must have a field with the label `f`, but it may be in any position and of any type (§4.8 rule 13). So, as in TypeScript, `v.f` is correct on a union when every member has the field, and its type is the union of the field types.

`C(..v, f1: e1, …)` makes a new value. Its fields are the given fields, and the other fields come from `v`. `v` must be known to be made by `C`: its type has only `C`, or §7.5 says so. The type of the new value comes from `C` and the types of all its fields.

### 6.8 Functions

`(p1, …, pn) -> A ? E ! N { body }` is a function value. It may use local names from outside. The `->` part may be left out; then the value type, errors, and environment are inferred. When the `->` part is written, it is the type of the function after its parameters, so a missing `?` or `!` in it means `Never` (§3), and `! N` must have the types of the environment parameters, and `Assert` when a parameter may fail (§4.8 rule 4). The body is a block, and the value of the block is the value that the function gives back.

### 6.9 Operators

Operators are part of the syntax. They are not functions, and they may not be used as values. They are in this order, from weakest to strongest. Operators at the same level go from left to right.

| Level | Operators | Operand type | Result type |
|---|---|---|---|
| 1 | `\|\|` | Bool | Bool |
| 2 | `&&` | Bool | Bool |
| 3 | `==` `!=` | Any two types; see below | Bool |
| 4 | `<` `<=` `>` `>=` | Int | Bool |
| 4 | `<.` `<=.` `>.` `>=.` | Float | Bool |
| 5 | `<>` | String | String |
| 6 | `\|>` | See below | |
| 7 | `+` `-` | Int | Int |
| 7 | `+.` `-.` | Float | Float |
| 8 | `*` `/` `%` | Int | Int |
| 8 | `*.` `/.` | Float | Float |

The unary operators `-` (Int) and `-.` (Float) are stronger than all binary operators. Calls, `.f`, and `.k` are stronger still. There is no operator for "not"; the function `core.not` (§13) does this.

- Int and Float have different operators. One is never changed into the other without a function such as `core.to_float` (§13).
- `a |> f` is `f(a)`, and `a |> f(b, …)` is `f(a, b, …)`. The right side of `|>` must be a name, a call, or a function.
- When both sides of `==` or `!=` have known types (§4.8), and these are disjoint (§4.5 rule 3), the result would always be the same, so it is a compile error, as in TypeScript. A member of a known type that is `_` or a type variable is never disjoint from anything. So `1 == "a"` and `(s: String) { s == 1 }` are compile errors, but `x == Nil` with `x: Int | Nil`, `[1] == []`, `a == b` with a type variable, and `(s) { s == 1 }` are not. What `==` does is in §11.4.

### 6.10 Blocks and bindings

The statements of a block are done in order. The value of a block is the value of its last expression. The last statement may not be a binding or an environment statement. A block with only one expression puts it in a group: `{ a + b } * c`.

`p = e` is a binding. It works out `e` and matches it with the pattern `p`. The names in `p` may be used from the next statement.

- The type of `e` must be a subtype of the pattern type of `p` (§4.8 rule 10). So `x: Int = e` says that `e` has the type `Int`, and `(a, b) = v` with `v: (Int, Int) | Nil` is a compile error; `case` is used for this.
- If `p` can not fail for the type of `e` (§7.3), the binding needs no environment.
- If not, as in `Ok(v) = r`, the binding works like a `case` on `e` with two clauses: `p`, which goes on with the rest of the block, and a catch-all, which calls `a.fail(AssertionFailure(m))`. Here `a` is the `Assert` service of the environment, and `m` is the text `pattern `, then the source text of `p`, then ` did not match `, then the text form (§11.8) of the value. So `Assert` is in the environment of the binding (§9.6).

`name = pattern` is itself a pattern (§7.1). So `a = (x, y) = e` binds `e` to `a` and to `(x, y)`.

A **direct value** is an expression that is made without running any code: a function expression; a literal, also with `-` or `-.` before it; a name; or a tuple, a list, a constructor call, or a group `{ e }` whose parts are direct values. When the value of a binding is a direct value, the names of the binding may also be used inside the function bodies in that value. This is the same for every binding, local or top-level. So a local function may call itself, and one tuple binding may make functions that call one another:

```i
fact = (n) {
  case n {
    0 -> 1
    _ -> n * fact(n - 1)
  }
}

(even, odd) = ((n) { n == 0 || odd(n - 1) }, (n) { n != 0 && even(n - 1) })
```

`! T = e` is an environment statement (§9.2). It works out `e` and puts its value in the environment, as the service of type `T`, for the rest of the block. It binds no name.

### 6.11 Parameters

A parameter is a pattern (§7), the same as the left side of a binding, but its names are not generalized (§4.7). When the function is called, each argument is bound to its parameter as in a binding (§6.10): the names of the pattern may be used in the body, and a pattern that may fail reports to `Assert`.

- The **label** of a parameter is the last name in the chain of `=` at the top of its pattern. So `x` and `x: T` have the label `x`. `x = label` and `x = label: T` have the label `label`, and they bind both `x` and `label`. `x = (a, b)` has the label `x`. A pattern with no name at its top, such as `(a, b)` or `_`, has no label.
- `x ! T` is an environment parameter (§9.2). It is named `x` in the body and has the type `T`. It is not a parameter of the function type.
- The parameters of one function may not bind one name two times, and may not have the same label.

### 6.12 case

`case e { clauses }` works out `e`. Then it takes the first clause whose pattern matches and whose guard, if there is one, is `True`, and works out the expression of that clause. A guard is any expression of type `Bool`. It is worked out only after its pattern has matched, and it may use the names of the pattern.

- The clauses must cover every possible value of the type of `e` (§7.4). If not, it is a compile error.
- The type of the `case` is a type of every clause expression, for example their union.
- A clause that can never be used gives a compile warning.

## 7. Patterns

### 7.1 Grammar

```
Pattern    = name "=" Pattern | PrefixPat | Atom .
PrefixPat  = string_lit "<>" ( name | discard ) .
Atom       = name | discard | ( name | discard ) ":" Type
           | [ "-" ] int_lit | [ "-." ] float_lit | string_lit
           | Name [ "(" [ PatArgs ] ")" ]
           | "(" [ Pattern { "," Pattern } [ "," ] ] ")"
           | "[" [ ListPats ] "]"
           | BitArrayPattern .
PatArgs    = ".." | Pattern { "," Pattern } [ "," [ ".." ] ] .
ListPats   = Rest | Pattern { "," Pattern } [ "," [ Rest ] ] .
Rest       = ".." [ name | discard ] .
```

- A `Name` in a pattern must be a constructor. Its parts are matched to its fields in the same way as parameters get their labels (§6.11):
  - The fields without labels are matched by position: the first parts go to them, in order.
  - The fields with labels are matched by label. Every other part must have a label, the last name in the chain of `=` at its top, that is the label of a field. These parts may be in any order. So with `User { User(name: String, age: Int) }`, `User(name, age)`, `User(age, name)`, and `User(n = name, ..)` are correct, but `User(n, a)` is not.
  - A field may be matched only one time. `..` at the end leaves out the fields that are not matched, and `C(..)` does not look at the fields. With no `..`, every field must be matched.
  - A call of `C` may still give the fields with labels by position (§6.6), but a pattern takes them only by label. A call puts values in, where the order is clear; a pattern takes them out and names them, and the names are the labels. TypeScript does the same: `new User("bob", 3)` and `const { name, age } = user`.
- The type after `:` goes as far to the right as it can, as every type does (§3). So `x: Int | String` tests for a union, and `f: (Int) -> Int = e` needs no brackets. In a `case` clause, `p: (Int, Int) -> e` is read as a function type, so a tuple type there is put in `{ }`, or a tuple pattern is used. A `|` after a type is part of the type, so a type test is written after the other choices: `True | _: Int`.

`x: T` is one thing in every place where it may be written (in a bit array segment, `:` begins a format instead, §8.1). It matches the values of the type `T`, and `x` gets the type `T`. In a binding or a parameter, every value that comes there is in `T` (§4.8 rules 4 and 10), so `x: Int = e` and `(x: Int) { … }` are type annotations. In a `case`, `x: T` tests the value. Only type constructors are tested at run time, so in a `case`, `T` may not have a type variable as a member.

### 7.2 What patterns match

| Pattern | Matches | Names |
|---|---|---|
| `x` | Any value | `x` |
| A discard | Any value | None |
| A literal, or `-` or `-.` and a literal | An equal value (§11.4) | None |
| `C(p1, …)` | A value made by `C` whose fields match their parts (§7.1) | Those of the parts |
| `(p1, …, pn)` | A tuple of n parts that match | Those of the parts |
| `[p1, …, pn]` | A list of exactly n parts that match | Those of the parts |
| `[p1, …, pn, ..r]` | A list of n or more parts; `r` is the rest | Those of the parts, and `r` |
| `"lit" <> r` | A string that starts with `"lit"`; `r` is the rest | `r` |
| `x = p` | What `p` matches | `x` (the full value), and those of `p` |
| `x: T` | A value whose type constructor is that of a member of `T` | `x` |
| `<< … >>` | See §8 | |

- A pattern may not bind one name two times.
- In choices `p1 | p2`, every choice must bind the same names. A name gets the union of its types in the choices.
- `=` is stronger than `|`.

### 7.3 Patterns that may fail

A pattern **can not fail** for a type `T` when it matches every value of `T`. Other patterns **may fail**. A catch-all can not fail for any type, and `x: U` can not fail for a type `T` with `T <: U`, so `(x: a) -> a { x }` needs no `Assert`. A value of a type variable may be any value, so a pattern that tests more than its type, such as a literal or a constructor, may fail for it.

### 7.4 Covering every value

The clauses of a `case` on a value of type `A` **cover** `A` when, for every value of `A`, some clause without a guard has a pattern that matches it. A clause with a guard is not counted. A clause is **never used** when every value that its pattern matches is matched by an earlier clause without a guard.

So literal patterns of Int, Float, String, and BitArray never cover their type, and a union is covered when every member is covered.

### 7.5 Known constructors

When a pattern `x = C(…)` matches, `x` is known to be made by `C` where `x` may be used. When a `case` clause pattern `C(…)` matches the value of a name `v`, `v` is known to be made by `C` inside that clause. This is used by §6.7.

## 8. Bit arrays

### 8.1 Grammar

```
BitArray        = "<<" [ Segment { "," Segment } [ "," ] ] ">>" .
BitArrayPattern = "<<" [ PatSegment { "," PatSegment } [ "," ] ] ">>" .
Segment         = Expr [ ":" Format ] .
PatSegment      = Pattern [ ":" Format ] .
Format          = name [ "(" Expr ")" ] .
```

The format names are fixed words of the bit array syntax. They are not names of values. In a segment, `:` always starts the format, and the format gives the type, so the pattern of a segment may not be a type test `x: T`.

### 8.2 Formats

| Format | Value type | Number of bits |
|---|---|---|
| `uN`, `iN` (N is 1 or more, and N is at most 8 or not a multiple of 8) | Int | N |
| `uNbe`, `uNle`, `iNbe`, `iNle` (N is a multiple of 8 and at least 16) | Int | N |
| `u(n)`, `i(n)` | Int | n |
| `f16be` `f16le` `f32be` `f32le` `f64be` `f64le` | Float | 16, 32, 64 |
| `bits` | BitArray | The length of the value |
| `bits(n)` | BitArray | n |
| `bytes` | BitArray | The length of the value, which must be a multiple of 8 |
| `bytes(n)` | BitArray | 8 × n |
| `utf8` `utf16be` `utf16le` `utf32be` `utf32le` | String | The length of the encoding |
| `utf8_codepoint` `utf16be_codepoint` `utf16le_codepoint` `utf32be_codepoint` `utf32le_codepoint` | UtfCodepoint | The length of the encoding |

- A segment with no format is `u8`.
- `u` is unsigned and `i` is signed (two's complement). `be` is big-endian and `le` is little-endian. An integer format with no ending puts the highest bit first, so `u(16)` is the same as `u16be`.
- The expression in brackets has the type Int and may be any expression.
- Any other format name is a compile error.

### 8.3 Making bit arrays

`<<s1, …, sn>>` puts the bits of the segments one after the other:

- For an integer, the lowest N bits of the value are used. Negative numbers use two's complement.
- A float is rounded to the nearest value of the format. If the result is not finite, the program stops.
- `bits(n)` takes the first n bits of the value, and `bytes(n)` the first 8 × n bits. If the value is too short, the program stops. For `bytes`, if the length of the value is not a multiple of 8, the program stops.
- If a length is negative, the program stops.
- Strings and codepoints are encoded in the given format.

### 8.4 Matching bit arrays

A bit array pattern matches the segments in order from the start. It matches only when all the bits are used.

- An integer segment reads N bits, signed or unsigned, and matches them with its pattern.
- A float segment reads its bits. It matches only when they are a finite number.
- `bits` and `bytes` with no length take all the bits that are left, so they may only be the last segment. For `bytes`, the number of bits left must be a multiple of 8.
- A string format in a pattern needs a string literal, which gives the bits to match. A `_codepoint` format matches one correctly encoded codepoint.
- A length may use names bound by earlier segments of the same pattern.
- If there are not enough bits, the pattern does not match. The program does not stop.

## 9. Effects

i has no special effect definitions. Effects are made from normal things: functions with no parameters (§9.1), services in the environment (§9.2), errors (§9.3), and one control operation (§9.4).

### 9.1 Computations

A function with no parameters, `() -> A ? E ! N`, is a **computation** that has not been done yet. It is the same idea as an `Effect<A, E, R>` value in Effect-TS. It may be stored, given to other functions, and put together with other computations. Calling it does it. Retrying, time limits, and running at the same time are normal functions:

```i
retry = (times: Int, body: () -> a ? e ! n) -> a ? e ! n {
  case fail.catch(body) {
    Ok(value) -> value
    Error(error) -> case times > 1 {
      True -> retry(times - 1, body)
      False -> fail.raise(error)
    }
  }
}

user = () { retry(3, () { load_user("1") }) }
```

### 9.2 Environment and services

A **service** is a value of a named type, normally a record of functions. The **environment** `N` in a function type is the union of the service types that the function needs. Services are taken and given with two forms of syntax:

- An **environment parameter** `name ! T` in the parameter list of a function (§6.11) gets the service of type `T` from the environment when the function is called. The caller does not give it as an argument. `T` is in the environment of the function.
- An **environment statement** `! T = e` in a block (§6.10) puts the value of `e`, which must have the type `T`, in the environment for the rest of the block. `T` is taken out of the environment that the rest of the block needs.

```i
Clock { Clock(now: () -> Int) }

elapsed = (start: Int, clock ! Clock) { clock.now() - start }
// type: (start: Int) -> Int ! Clock

fixed = () {
  ! Clock = Clock(now: () { 42 })
  elapsed(40)
}
// type: () -> Int
```

- `T` is written in the source, and must be one named type `T(…)`. It may have type variables as arguments, as in `State(s)`, but it may not be a type variable alone, a union, or `Never`.
- The environment goes with every call: a called function gets the environment of the place of the call. An environment statement changes it for the calls in the rest of its block, also calls of functions that were made earlier. This is like the environment of Effect-TS: what a computation needs is given by the code that runs it. So the environment of a function is a set of parameters that are given without being written: its type `! N` says which services the caller must have, and a function made in one place and called in another gets its services from the place where it is called.
- An environment may not have two forms of one type constructor with different arguments: `State(Int) | State(String)` is a compile error, also for a service type whose parameter goes in one direction (§4.5 rule 4). Two environment parameters of one function may not have the same type constructor. A later environment statement for the same type constructor takes the place of an earlier one.
- A nominal type is the name of a service. To have two services of the same form, make two types.
- A service is a normal value. It may be stored and given to functions, and code that has it may call its functions directly.
- Type checking makes certain that every environment parameter gets its service. Getting it never fails at run time.

### 9.3 Errors

The `E` part of a function type is the set of errors the function may give. An error may be a value of any type. Errors come together by union (§4.4): when a function calls functions with the errors `A` and `B`, its errors are `A | B`. The module `i/fail` has these functions:

| Function | Type | What it does |
|---|---|---|
| `fail.raise` | `(error: e) -> Never ? e` | Ends the computation with `error` |
| `fail.catch` | `(body: () -> a ? e ! n) -> Result(a, e) ! n` | Changes every error into `Error(…)` |
| `fail.from_result` | `(result: Result(a, e)) -> a ? e` | Gives the error `x` for `Error(x)` |
| `fail.map_error` | `(body: () -> a ? e ! n, with: (e) -> e2 ! n) -> a ? e2 ! n` | Changes the errors |

```i
load = (id) { parse_user(http_get(id)) }
// inferred: (id: String) -> User ? HttpError | ParseError ! Http

safe = (id) {
  case fail.catch(() { load(id) }) {
    Ok(user) -> user
    Error(_: ParseError) -> default_user
    Error(other) -> fail.raise(other)        // other: HttpError
  }
}
// inferred: (id: String) -> User ? HttpError ! Http
```

- To handle only some errors, take all of them with `fail.catch`, then use `case` with type tests. A later catch-all gets the rest of the union (§4.8 rule 12), which may be given again with `fail.raise`, as in `safe` above.
- An error goes out through the calls until `fail.catch` or `fail.map_error` takes it. The computation that is jumped over is unwound first (§9.5).
- An error that comes to the program entry or a test (§10.2) stops the program. An error that comes to a host export is given to the host.

### 9.4 Control

The module `i/control` has one operation to pause a computation and go on with it later:

| Function | Type |
|---|---|
| `control.boundary` | `(body: (b: Boundary(r, e, n)) -> r ? e ! n) -> r ? e ! n` |
| `control.suspend` | `(b: Boundary(r, e, n), f: (k: Continuation(a, r, e, n)) -> r ? e ! n) -> a` |
| `control.resume` | `(k: Continuation(a, r, e, n), value: a) -> r ? e ! n` |
| `control.cancel` | `(k: Continuation(a, r, e, n)) -> Nil ! n` |

`Boundary` and `Continuation` are built-in types. They are not custom types: their values are made only by `control.boundary` and `control.suspend`, and the host cannot take them (§10.3).

1. `boundary(body)` makes a boundary `b` and does `body(b)`. The value of `body` is the value of `boundary`. `b` is **active** while the computation between it and the running code is on the stack: from the start of `body` until `body` ends, but not while that part is paused in a continuation. After `resume`, it is active again.
2. `suspend(b, f)` pauses the computation from this point back to the boundary `b`. That paused part is a **continuation** `k`. Then `f(k)` is done in the place of the boundary, and its value is the value of `boundary`. If `b` is not active, the program stops.
3. `resume(k, v)` goes on with the paused computation, with `v` as the value of `suspend`. It goes on on top of the place where `resume` is called, and the value that the computation has when it gets to the boundary is the value of `resume`. The paused part keeps the environment statements that were inside it; for the rest, it gets the environment of the place where `resume` is called.
4. `cancel(k)` unwinds the paused computation (§9.5) and gives `Nil`.
5. **One use only**: `resume` and `cancel` together may be used one time for a continuation. A second use stops the program.
6. If a continuation is never resumed or cancelled, its final functions are not run.

Generators, async code, and schedulers are libraries made from services and this operation. For example, a generator:

```i
Yield(a) { Yield(emit: (value: a) -> Nil) }

collect = (body: () -> Nil ! Yield(a) | n) -> List(a) ! n {
  control.boundary((b) {
    ! Yield(a) = Yield((x) { control.suspend(b, (k) { [x, ..control.resume(k, Nil)] }) })
    body()
    []
  })
}
```

### 9.5 Unwinding and final functions

A computation that is dropped is first **unwound**: its final functions are run, from the innermost to the outermost. This takes place when an error goes through it (§9.3), and when its continuation is cancelled (§9.4).

The module `i/resource` puts final functions in place:

| Function | Type | What it does |
|---|---|---|
| `resource.ensure` | `(body: () -> a ? e ! n, with: () -> Nil ! n) -> a ? e ! n` | Runs `with` after `body`, whether `body` ends normally, ends with an error, or is unwound |
| `resource.acquire` | `(open: () -> r ? e ! n, release: (r) -> Nil ! n, use: (r) -> a ? e ! n) -> a ? e ! n` | After `open` is done, runs `release` when `use` ends or is unwound |

A final function may not give errors. When the program stops (§11.7), final functions are not run.

### 9.6 Assert

`Assert` and `AssertionFailure` are built-in custom types with these definitions:

```i
AssertionFailure { AssertionFailure(message: String) }
Assert { Assert(fail: (failure: AssertionFailure) -> Never) }
```

An `Assert` service is where failures are reported. Its `fail` never gives back a value. The built-in function `assert` is a normal function, which may be written in i:

```i
assert = (c: Bool, a ! Assert) {
  case c {
    True -> Nil
    False -> a.fail(AssertionFailure("assert"))
  }
}
// type: (c: Bool) -> Nil ! Assert
```

A binding whose pattern may fail also reports to `Assert` (§6.10).

At an entry point (§10.2), the runtime gives an `Assert` whose `fail` stops the program; in a test, the test fails. Any code may give its own `Assert` with an environment statement. The module `i/assertion` has one function that does this:

```i
catch = (body) {
  control.boundary((b) {
    ! Assert = Assert((failure) {
      control.suspend(b, (k) {
        control.cancel(k)
        Error(failure)
      })
    })
    Ok(body())
  })
}
// type: (body: () -> a ? e ! n) -> Result(a, AssertionFailure) ? e ! n - Assert
```

`Assert` is for mistakes in the program. A failure that may be expected uses an error (§9.3). `fail.catch` and `fail.map_error` do not take `Assert` failures.

### 9.7 Host services

A type definition with the attribute `@host("namespace")` is a **host service**:

```i
@host("app:storage/v1", "async")
Storage { Storage(read: (key: String) -> String ? String) }
```

- It has exactly one constructor. Every field has a label and a function type. The parameters, values, and errors of these functions must be types that the host can take (§10.3), and they may not have an environment. Two host services may not have the same namespace and a field with the same label.
- With `@host("namespace", "async")`, the host does the functions **later**: the program waits at the call, and goes on from the call when the host gives a value or an error. In i code, such a call is a normal call.
- A host service in the environment of an entry point (§10.2) is given by the host. Every field is the Wasm import with the import module `namespace` and the field label as its name, encoded as in Appendix B.
- i code may also give a host service with an environment statement, for example to put a fake in its place in a test.

## 10. The host boundary

### 10.1 External functions

```
@external("module", "function")
name: (T1, …, Tn) -> R
```

This declares a function that the host gives. It is the Wasm import `function` in the import module `module`, encoded as in Appendix B. The full type must be written. Parameters and the value must be types that the host can take (§10.3), with no errors and no environment. Every call of an external function is done when it is reached, like all calls. An external function should have no side effects that i code may see; work with side effects on the host uses host services (§9.7).

### 10.2 Entry points

These functions are **entry points**:

- **The program entry**: the function marked `@main` in the package that is compiled. It has no parameters. Its value is not used. More than one `@main` in a package is a compile error. `@main` in other packages is not used.
- **Tests**: the functions marked `@test` in the modules under `test/` of the package that is compiled. Each has no parameters and gives `Nil`. A test **passes** when it gives `Nil`. It **fails** when it stops (§11.7). Tests run one at a time, each in a new instance of the module that has been started (§B.1), so one test cannot change another.
- **Host exports**: the functions named in `[wasm].exports` (§2.6) of the package that is compiled. The `[wasm]` of other packages is not used. They must be in the `export` list of their module. Their parameters, value, and errors must be types that the host can take (§10.3).

"No parameters" here does not count environment parameters. An entry point may have environment parameters and an environment, but only of host services and `Assert`; the host and the runtime give them. Any other service must be given before it gets to the entry point. If not, it is a compile error.

An error that comes to the program entry or a test stops the program. An error that comes to a host export is given to the host (Appendix B).

An entry point that may call a function of an async host service (§9.7) is **async**. The host has to wait for it (Appendix B).

### 10.3 Types the host can take

`Nil`, `Bool`, `Int`, `Float`, `String`, `BitArray`, and custom types with no constructors (§4.3). Values of custom types with no constructors go to the host as opaque references. No other type may go to the host. An error type that the host can take is a union of `Nil`, `Bool`, `Int`, `Float`, `String`, and `BitArray`, with no two the same.

## 11. Run-time behavior

### 11.1 Values

No value may be changed. The values are integers, floats, strings, codepoints, bit arrays, tuples, lists, values of custom types, functions, boundaries, continuations, and host references.

### 11.2 Int

An Int may be of any size. It never overflows.

- `a / b` rounds toward zero. `a % b` has the sign of `a`, and `a == (a / b) * b + a % b`.
- When `b` is 0, `a / b` and `a % b` are 0.

### 11.3 Float

A Float is a finite IEEE 754 binary64 number. Results are rounded to the nearest value, with ties to even.

- If the result of an operation is not finite, the program stops.
- When `b` is 0.0, `a /. b` is 0.0.

### 11.4 Equality

`a == b` is `True` when `a` and `b` are the same value, and `a != b` is the opposite. This is the only equality in i. Values are the same in these cases:

- Ints are the same when their numbers are equal. Floats are the same when they are equal by IEEE 754, so `0.0 == -0.0`.
- Strings are the same when their scalar values are equal. Codepoints are the same when their numbers are equal. Bit arrays are the same when their bits are equal.
- Tuples, lists, and values of custom types are the same when they have the same constructor and their parts are the same, in order. Values with different type constructors, and tuples of different sizes, are never the same.
- A function, a boundary, a continuation, and a host reference have no parts that may be looked at, so each is the same only as itself: the value made by one evaluation of a function expression, by one call of `control.boundary` or `control.suspend`, or one reference from the host (§B.4). A top-level function is worked out once (§2.5), so all its uses are the same value. In the same way, a constructor, a built-in function, and a library function are each one value. So two function expressions that are written the same way give values that are not the same.

### 11.5 Comparing

`< <= > >=` compare Ints by number. `<. <=. >. >=.` compare Floats by IEEE 754.

### 11.6 Tail calls

A call in tail position uses no more stack. Tail positions are: the last expression of a function body; the expression of a clause of a `case` in tail position; and the last expression of a block in tail position. An environment statement does not change which calls are in tail position.

### 11.7 Stops

The program stops in these cases:

| Case | Reason |
|---|---|
| `panic(m)` is called | `panic: ` and `m` |
| The `Assert` of an entry point gets a failure (§9.6) | `assertion failed: ` and the message |
| An error comes to the program entry or a test (§9.3) | `error: ` and the text form of the error |
| `control.suspend` with a boundary that is not active (§9.4) | `control: boundary is not active` |
| A continuation is used a second time (§9.4) | `control: continuation was already used` |
| A float operation has a result that is not finite (§11.3) | `float: result is not finite` |
| Making a bit array fails (§8.3) | `bit array: value is too short`, `bit array: length is negative`, `bit array: bytes are not whole`, or `bit array: float is not finite` |
| A value from the host is not correct (Appendix B) | `host: value is not correct` |
| There is no more memory or stack | none |

Before the program stops, the host gets a **stop message**, in one call of `panic` (Appendix B): the reason, then one line for each source position, with a line feed between lines. The first position is that of the innermost expression in the program source that was being worked out: a call, an operator, a `<<`, or a binding. Then come the positions of the calls in the program source that were running, the innermost first. Calls that were replaced by tail calls are gone. For an error at an entry point, there is only the position of the entry point's binding.

A **source position** is written `module path:line:column`, for example `app/users:12:5`, with the module path as its own package names it. For a module of a package with a location, the location, `@`, the major version, and a space come first: `github.com/alice/json@1 json/decode:12:5`. Lines and columns count from 1; a column counts Unicode scalar values. The position of an expression is the position of its first token.

A function with no errors and no environment (`? Never ! Never`) may still stop for these reasons, and it may never end.

### 11.8 Text form of values

The **text form** of a value is given by `core.inspect` (§13):

| Value | Text form |
|---|---|
| Int | Decimal digits, with `-` before a negative number: `-42` |
| Float | See below |
| String | `"`, then the characters, then `"`. `"` and `\` are written as `\"` and `\\`. Line feed, carriage return, tab, and form feed are `\n`, `\r`, `\t`, `\f`. Other characters from U+0000 to U+001F, U+007F, and U+0080 to U+009F are `\u{XXXX}`, with four upper-case hex digits. All other characters are written as they are. |
| List | `[`, the text forms of the parts with `, ` between them, `]` |
| Tuple | `(`, the text forms of the parts with `, ` between them, `)` |
| Value of a custom type | The constructor name. If it has fields: `(`, the text forms of the fields in order with `, ` between them, `)`. Labels are not written. So `True`, `Nil`, `Ok(1)`. |
| BitArray | `<<`, the whole bytes as decimal numbers with `, ` between them, `>>`. If the number of bits is not a multiple of 8, the last k bits come last, written as `value:uk`: `<<82, 2:u2>>` |
| UtfCodepoint | `//utfcodepoint(`, the character, `)` |
| Function | `//fn(`, one name for each parameter of its type with `, ` between them, `) { ... }`. Environment parameters are not counted. Parameter i (counting from 0) is named by the letter `a` + (i mod 26), with the decimal number i div 26 after it when i is 26 or more: `//fn(a, b) { ... }` |
| Boundary, continuation | `//boundary`, `//continuation` |
| Host reference | `//wasm(opaque)` |

A **Float** is written in this way. Let d1 … dn be the shortest list of decimal digits, with dn not 0, such that 0.d1…dn × 10^k gives back the same float when read. When more than one list of that length does this, take the one nearest to the exact value of the float; when two are equally near, take the one whose last digit is even. `-` comes first for a negative float, including -0.0. Zero is `0.0`. Let p = k − n.

- The **long form** is used when p is at least L and at most U, and not when p ≥ 0 and the size of the float is 2^53 or more. L is −4 when n = 1, and −2 − n when not. U is 2 when n = 1 or k ≥ 11, and 1 when not.
- In the long form, the digits are written as a decimal number, with `.` and at least one digit after it: `123.0`, `12.67`, `0.0123`.
- In the short form, d1 is written, then `.`, then d2 … dn, or `0` when n = 1, then `e`, then k − 1 as a decimal number with `-` when it is negative: `3.0e26`, `1.0e-7`, `1.0e3`.

## 12. Built-in names

These names may be used in every module. Other names may cover them.

| Name | What it is | See |
|---|---|---|
| `Int` `Float` `String` `List` `BitArray` `UtfCodepoint` `Never` | Types | §4.2 |
| `Bool` `Nil` `Result`, and `True` `False` `Nil` `Ok` `Error` | Custom types and their constructors | §4.2 |
| `Assert` `AssertionFailure` | Custom types and their constructors | §9.6 |
| `Boundary` `Continuation` | Built-in types made by `i/control` | §9.4 |
| `panic` | `(message: String) -> Never` | §11.7 |
| `assert` | `(c: Bool) -> Nil ! Assert` | §9.6 |

These are normal functions and types. The modules `i/fail` (§9.3), `i/control` (§9.4), `i/resource` (§9.5), `i/assertion` (§9.6), and `i/core` (§13) have the other functions.

## 13. The module `i/core`

These are normal functions: they are called, given, and compared like any other function. Only their bodies cannot be written in i, because they do things that the rest of the language cannot do, such as looking at how any value is made (`inspect`, `hash`). They have no errors and need no environment.

| Function | Type | What it gives |
|---|---|---|
| `not` | `(Bool) -> Bool` | `True` for `False`, and `False` for `True` |
| `to_float` | `(Int) -> Float` | The nearest Float, with ties to even. If it is not finite, the program stops. |
| `truncate` | `(Float) -> Int` | The Float without its fraction, rounded toward zero |
| `floor` | `(Float) -> Float` | The greatest whole number that is not greater |
| `ceiling` | `(Float) -> Float` | The smallest whole number that is not smaller |
| `sqrt` | `(Float) -> Float` | The square root. For a negative number the result is not finite, so the program stops (§11.3). |
| `float_to_bits` | `(Float) -> Int` | The 64 bits of the IEEE 754 binary64 value, as a number from 0 to 2^64 − 1 |
| `float_from_bits` | `(Int) -> Float` | The float with the lowest 64 bits of the number. If it is not finite, the program stops. |
| `bit_and`, `bit_or`, `bit_xor` | `(Int, Int) -> Int` | The bitwise operation on the numbers as two's complement with no end |
| `bit_not` | `(Int) -> Int` | `-x - 1` |
| `shift_left` | `(Int, Int) -> Int` | `x` × 2^n. A negative n is `shift_right` by −n. |
| `shift_right` | `(Int, Int) -> Int` | `x` ÷ 2^n, rounded down. A negative n is `shift_left` by −n. |
| `concat` | `(List(String)) -> String` | The strings one after the other |
| `byte_size` | `(String) -> Int` | The number of UTF-8 bytes |
| `byte_slice` | `(String, start: Int, length: Int) -> Result(String, Nil)` | The bytes from `start` (counting from 0), `length` of them. `Error(Nil)` when they are not all in the string, or when the start or the end is inside a character. |
| `codepoints` | `(String) -> List(UtfCodepoint)` | The characters in order |
| `from_codepoints` | `(List(UtfCodepoint)) -> String` | The string of these characters |
| `codepoint_to_int` | `(UtfCodepoint) -> Int` | The number of the character |
| `int_to_codepoint` | `(Int) -> Result(UtfCodepoint, Nil)` | The character with this number, or `Error(Nil)` when it is not a Unicode scalar value |
| `bit_size` | `(BitArray) -> Int` | The number of bits |
| `bit_array_to_string` | `(BitArray) -> Result(String, Nil)` | The string, when the bits are whole bytes of correct UTF-8; `Error(Nil)` when not |
| `inspect` | `(a) -> String` | The text form of the value (§11.8) |
| `hash` | `(a) -> Int` | A number from 0 to 2^32 − 1. Values that are the same (§11.4) give the same number. Which number is not fixed. |

## Appendix A. Choices

The rules below were chosen without a full discussion. Each one follows Gleam or TypeScript where that agrees with the ideas of i, and says why where it does not. Rules that were settled in discussion are not listed.

1. Evaluation goes from left to right, the function before its arguments, as in Gleam and TypeScript (§6.2).
2. The operator levels are those of Gleam (§6.9).
3. Labels do not change what a type means, like parameter names in TypeScript. A call by label needs a known function type (§4.6, §6.6).
4. Making a bit array that is not correct stops the program, as in Gleam. Matching one only fails to match (§8.3, §8.4).
5. A string prefix pattern does not name the prefix, though Gleam lets it, because the prefix is always the literal. `whole = "lit" <> rest` names the whole string (§7.1).
6. Imported names and top-level names are in one space and may not be the same, as in Gleam and TypeScript (§2.3).
7. An exported definition may not use a type that is not exported, as in Gleam (§2.4).
8. Division by zero gives 0, as in Gleam. A float result that is not finite stops the program, as Gleam does on Erlang (§11.2, §11.3).
9. Narrowing in a `case` follows what the earlier clauses match, like narrowing after a check in TypeScript (§4.8 rule 12).
10. A binding is generalized only when its value is a direct value: the value restriction of ML. TypeScript makes only generic functions general, and functions are direct values (§4.7).
11. A continuation that is never resumed or cancelled does not run its final functions. Cancelling is not automatic, because that would need the runtime to find continuations that are dropped (§9.4).
12. Final functions run when an error goes through them, like `finally` in TypeScript, and when a continuation is cancelled (§9.5).
13. A guard may be any `Bool` expression, like a condition in TypeScript. Gleam allows only some expressions there (§6.12).
14. The text form of a value is written as i source, as `string.inspect` in Gleam writes Gleam source. So tuples are `(a, b)` and the last bits of a bit array are `value:uk` (§11.8).
15. A stop message is the reason and the stack of source positions, like a stack trace in TypeScript (§11.7).
16. An error that comes to a host export goes to the host, as an exception does in TypeScript (§10.2).
17. There is no `echo`, no `todo`, and no `!` for "not", though Gleam has them. `!` always means the environment, a debug print is a host service, and `panic("todo")` does the work of `todo` (§6.9, §13).
18. The program entry and tests are marked with `@main` and `@test`. Gleam finds them by their names, which would give names a special meaning (§10.2).
19. The binary interface (Appendix B) and the module `i/core` (§13) are new. They are made for WasmGC, and Gleam and TypeScript have nothing to follow here.
20. Members of a union with the same type constructor are put together, as MLsub needs for principal types. So `List(Int) | List(String)` is `List(Int | String)`, and a union never has two members with one type constructor (§4.4).
21. Types have intersections `&`, because principal types need them where values come in, as in `(x: a & Int) -> a`. MLsub and TypeScript both have them (§3, §4.4).
22. A `case` is typed by places, so a value that no pattern takes may only come where a catch-all can take it. Coverage is checked with the inferred types, and narrowing is a difference by the taken type constructors (§4.8 rules 11 and 12).
23. Errors, unwinding, and `suspend` use Wasm exception handling, which Chrome, Firefox, Safari, and Wasmtime all have. Frames are saved only while a `suspend` goes out through them, so a normal call allocates nothing (Appendix B.1).

## Appendix B. The binary interface

### B.1 The module

A compiled program is one Wasm module. It needs the Wasm features garbage collection, typed function references, tail calls, and exception handling with `exnref`. Exceptions are used only inside the module: its tags are not exported, every export catches them, and an import must not throw one into the module. It exports:

- `memory`: the linear memory used to pass buffers.
- `ilang_alloc(size: i32) -> i32`: gives the address of a new buffer of `size` bytes in `memory`. For size 0 it may give 0.
- `ilang_free(address: i32, size: i32)`: frees a buffer made by `ilang_alloc`.
- `ilang_start()`: starts the program by working out the top-level values (§2.5). It has no parameters and no results. The host calls it one time, after it makes the instance and before it calls any other export but `ilang_alloc` and `ilang_free`. During `ilang_start`, imports may be called as at any other time.

The module has no Wasm start function, so that imports called while top-level values are worked out may call `ilang_alloc`. The module has one custom section named `ilang.abi` with the metadata of §B.6.

### B.2 Values

A value of a type that the host can take (§10.3) is passed as zero or more Wasm values:

| Type | Wasm values |
|---|---|
| `Nil` | none |
| `Bool` | `i32`: 0 is `False`, 1 is `True` |
| `Float` | `f64` |
| `Int` | `i32` sign, `i32` address, `i32` count. The sign is −1, 0, or 1. The size of the number is `count` unsigned 32-bit parts at the address, lowest part first, each part little-endian. The last part is not 0. Zero is sign 0 and count 0. |
| `String` | `i32` address, `i32` length: the UTF-8 bytes |
| `BitArray` | `i32` address, `i32` number of bits. There are ⌈bits / 8⌉ bytes. In each byte the highest bit comes first. Bits after the end in the last byte are 0. |
| A custom type with no constructors | `externref` |

The size of the buffer of an `Int` is 4 × count bytes, of a `String` its length, and of a `BitArray` ⌈bits / 8⌉ bytes.

A function `(P1, …, Pn) -> A ? E` is passed like this. Environment parameters are not passed.

- **Parameters**: the values of `P1`, then `P2`, and so on.
- **Results**, when `E` is `Never`: the values of `A`.
- **Results**, when `E` has members: first an `i32` **tag**, then the values of `A`, then the values of each member of `E` in the **canonical order**: `Nil`, `Bool`, `Int`, `Float`, `String`, `BitArray`. Tag 0 means a normal value; tag i means an error of the i-th member. The values that are not used are 0, or null for an `externref`.

When the host gives a value that is not correct (a `String` that is not UTF-8, an `Int` with a wrong sign or a last part of 0, a `Float` that is not finite, a `Bool` that is not 0 or 1, or a tag that is too big), the program stops (§11.7).

### B.3 Buffers

- A buffer given as a **parameter** belongs to the caller. It may only be used during the call. The callee makes a copy if it needs one.
- A buffer given as a **result** is made with `ilang_alloc` and belongs to the receiver, which frees it with `ilang_free`.

So for an export, the host makes the parameter buffers with `ilang_alloc`, frees them after the call, and frees the result buffers. For an import, the module makes and frees the parameter buffers, and the host makes the result buffers by calling `ilang_alloc` before it returns. During an import, the host may only call `ilang_alloc` and `ilang_free`.

### B.4 Imports

- **External functions** (§10.1): the import module and name of the `@external` attribute, encoded as in §B.2.
- **Functions of host services** (§9.7): the import module is the namespace and the name is the field label, encoded as in §B.2.
- **Async functions of host services**: the import has the parameters of §B.2 and no results. The host starts the work and returns at once. The result comes later through `ilang_resume` (§B.5).
- **Runtime imports**, with the import module `ilang:host/v1`:
  - `panic(address: i32, length: i32)` gives the stop message (§11.7) as UTF-8. Then the module traps.
  - `reference_id(reference: externref) -> i64` gives a number for a host reference: the same number for the same reference, different numbers for different references, and 0 for null. It is used by `==` (§11.4).

### B.5 Exports

- **Host exports** (§10.2) have the name written in `[wasm].exports`, for example `app/math.add`, and are encoded as in §B.2.
- **The program entry and tests** have the name `module path.function name`, no parameters, and no results.
- A function that is both a host export and the program entry or a test is a compile error, because their export names would be the same.
- **Async entry points** (§10.2) have the parameters above and one result, an `i32` **status**: 0 means done, 1 means waiting. When the status is 1, the program is waiting for exactly one host function that it has started. When that work is done, the host calls the export `ilang_resume:namespace:label` of that function. Its parameters are the results of the function as in §B.2, and its result is a status again. When the status is 0, the host calls the export `ilang_result:name` of the entry point, with no parameters, to get its results as in §B.2.
- While an async entry point is waiting, the host may only call `ilang_resume:…`, `ilang_alloc`, and `ilang_free`.
- When the program stops, the export traps after the `panic` import (§B.4). After a trap, the host may not use that instance of the module again; it makes a new instance and calls `ilang_start` on it.

### B.6 Metadata

The custom section `ilang.abi` is UTF-8 JSON in this form:

```json
{
  "version": 2,
  "entry": { "name": "app/main.main", "async": false },
  "tests": [ { "name": "app/math_test.add", "async": false } ],
  "exports": {
    "app/math.add": { "params": ["int", "int"], "result": "int", "errors": [], "async": false }
  },
  "services": {
    "app:storage/v1": {
      "read": { "params": ["string"], "result": "string", "errors": ["string"], "async": true }
    }
  },
  "externals": {
    "app:math/v1": { "sqrt": { "params": ["float"], "result": "float" } }
  }
}
```

- Type names are `nil`, `bool`, `int`, `float`, `string`, `bit_array`, and `externref`. `errors` is in the canonical order (§B.2).
- `entry` is `null` when the program has no `@main`. `tests` is empty unless tests were compiled.
- `services` lists every host service function that the entry points need, and `externals` every external function.
- The host must give every import in `services`, `externals`, and `ilang:host/v1` before it makes an instance of the module. If not, making the instance fails.
