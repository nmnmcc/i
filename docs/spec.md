# The i Language

This document is the definition of the i programming language. A compiler or a program that does not keep to it is wrong. The present compiler does not yet do everything in this document.

This document is complete in itself. It defines:

- the source language (§1 to §8);
- effects, and the library modules `i/fail`, `i/control`, `i/resource`, `i/assertion`, `i/io`, `i/key`, `i/task`, `i/random`, `i/request`, `i/clock`, `i/fs`, `i/process`, `i/net`, `i/tls`, and `i/stdio` (§9);
- the library module `i/core`, which has the operations that are not written in i (§13);
- the library module `i/record`, which reads and sets fields of records (§14);
- packages, versions, and the files `i.toml` and `i.lock` (§2.1, §2.6 to §2.8);
- what programs do when they run, and the text form of values (§11);
- the binary interface between a compiled module and its host (§10 and Appendix B).

Every compiler must give the library modules above. The target modules `i/target`, `i/js`, and `i/wasi` have the contracts of §9.7, §10.1 and §B.7.1. A program may not define a module with any of these library paths. Other library modules, command-line behavior not specified in §2.6, and the way test results are shown are not part of the language.

i is small, and every part of it is what it looks like. A thing that looks like a function is a normal function, and a clause block is a function too (§6.12). A binding is a binding wherever it is written, and a clause block with one clause and no guard binds its pattern as a parameter does. Where two forms look the same, they have the same rules, with these exceptions, where the place of a form decides its rules:

- `x: T` in a clause pattern is a type annotation when the clause block has one clause, no guard, and no choices, and a test when not (§7.1). So with `v: Int | String`, `v |> { x: Int -> x }` is a compile error, but `v |> { x: Int -> x  _ -> 0 }` is correct.
- A `{` that begins an expression is a clause block or a block by its first statement, but the body of a function is always a block (§6.1). So in `make = (n) { (x) -> Int { x + n } }`, the body is a block that gives a function, but `g = { (x) -> Int { x + n } }` is a clause block with the pattern `(x)`.
- In the first statement of a `{`, `n = ( … ) -> T` is the binding of a function when a `{` comes after `T` on the same line, and a clause pattern followed by `->` when not (§6.1).

## 0. How to read this document

The grammar is in EBNF: `|` is a choice, `[ ]` may be there or not, `{ }` may come zero or more times, and text in quotes is written as it is.

Inside a type in the rules and in the prose, a capital letter, alone or with a digit, such as `A`, `T`, `E`, `N`, or `P1`, stands for any type, and a lowercase letter, alone or with a digit, such as `a`, `r`, or `b1`, stands for a type variable. A letter followed by an index, `i`, `j`, `k`, `m`, or `n`, as in `Ti`, `Qi`, `ai`, `an`, or `bm`, is that letter with that index, and stands for the same kind of thing as the letter alone. Outside a type, the letters keep the sense that the text gives them: in §4.8, for example, `C` is a constructor, `e` an expression, and `r` an expression or a name. This is notation of this document, not i syntax: in i code, a type variable is an `UpName` that is declared (§4.7), as in `id(A): (A) -> A`.

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

- A `name` is a value, a parameter, a label, or a part of a module path. A name may start with one `_`, as in `_count`. It is a normal name in every way, but a compiler does not warn when it is not used.
- A `discard` is `_` alone, or a word that starts with two `_`, as in `__old_value`. It is a place in a pattern that binds no name; the word after `__` only says what is left out.
- An `UpName` is a type, a type variable (§4.7), or a constructor.

### 1.4 Keywords

i has no keywords. Every word is a `name`, an `UpName`, or a discard (§1.3), and any `name` may be used as a name. The forms that other languages make with keywords are made with brackets and symbols: imports and exports are groups at the start of a file (§2.2), matching is a call of a clause block (§6.12), and a guard is written with `?`.

Some forms are kept for macros, which are not yet part of i: an attribute that is not built in, such as `@m` or `@m(args)` before an item (§2.2); a top-level item that is a call, `m(args)` or `a.m(args)` (§2.2); a declaration with neither `@external` nor `@js` (§2.2); and, in a type, a lowercase name or `a.m` followed by `(` (§3). The parser reads each of them in full, and each is a compile error whose message says that the form is kept for macros.

### 1.5 Symbols

```
=   :   ,   .   ..   ->   ?   !   |   ^    @
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
string_lit = cooked_string | multiline_string | raw_string .
cooked_string = '"' { char | escape | interpolation } '"' .
interpolation = "\(" Expr ")" .
multiline_string = '"""' line_end { char | escape | interpolation } line_end indent '"""' .
raw_string = hashes ( cooked_string | multiline_string ) hashes .
hashes = "#" { "#" } .
escape     = "\" ( "n" | "r" | "t" | "f" | '"' | "\" ) | "\u{" hex { hex } "}" .
```

- A literal is never negative. A negative number is made with the operator `-` (Int) or `-.` (Float) (§6.9). In a pattern, a negative literal is written in the same way (§7.1).
- `_` only puts digits into groups. It may not be first, last, or next to another `_`.
- The value of a float literal must be a finite IEEE 754 binary64 number. If not, it is a compile error.
- `\u{…}` has 1 to 6 hex digits. Its value must be a Unicode scalar value: 0 to 10FFFF, but not D800 to DFFF. If not, it is a compile error.
- An integer literal may be of any size.
- A string may go over more than one line. Line ends in it are changed as in §1.1.
- In an expression, `"a\(e)b"` interpolates an expression `e`, including calls, blocks and nested string literals. Every interpolated expression must have type `String`; there is no implicit conversion. Before the core tree, interpolation becomes `<>` of its parts from the left, leaving out empty text parts. `"a\(x)b"` is `"a" <> x <> "b"`; a sole expression keeps an empty part: `"\(x)"` is `"" <> x`. Interpolation is forbidden in record labels, attribute arguments, import paths, literal types and patterns. The normal formatter retains interpolation as written; the Lisp surface has no interpolation and uses `<>`.
- A multi-line string begins with `"""` immediately followed by a line break. Its closing `"""` is on a line preceded only by spaces or tabs. That exact indentation prefix is removed from every nonblank content line; a line with less or different indentation is a compile error. Blank lines with less indentation are accepted and made empty; indentation beyond the closing prefix remains part of a blank line. The opening line break and the line break before the closing line are not part of the value. Escapes and interpolation work as in ordinary strings. A backslash immediately before a physical line break joins that line to the next after indentation is removed.
- A raw string surrounds its quotes with one or more matching `#`, as in `#"text"#`, `##"text"##` or `#"""` … `"""#`. Backslashes are literal unless followed by exactly the opening number of hashes. With one hash, `\#n` is a line feed and `\#(e)` interpolates, while `\n` and `\(e)` remain literal text. Raw multi-line strings use the same indentation and delimiter-line rules; `\#` at line end joins lines in a one-hash raw string. Closing quotes consume the matching number of hashes; too few do not close the string.

### 1.7 Lines

A line end is a **break** when both sides of it allow one:

- the last token before it **can end** an expression: a name, an `UpName`, a discard, a literal, `)`, `]`, `}`, or `>>`; and
- the first token after it **can begin** a statement: a name, an `UpName`, a discard, a literal, `(`, `[`, `{`, `<<`, `-`, `-.`, `..`, `!`, or `@`.

Comments are not tokens, so the tokens on the two sides of a comment are what count. Every line end that is not a break is only a space.

A break ends something. What it ends depends on the brackets it is directly inside:

- Directly inside `{ }` (a block, a clause block, or the constructors of a type), and at the top level of a file, a break ends a statement, a clause, a constructor, or an item. Attributes are part of the item after them, so a break after an attribute does not end the item.
- Directly inside the `( )` of a record (§6.13) or a record type (§3), a break is a `,`.
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

## 2. Modules

### 2.1 Packages and modules

A **package** is a directory with an `i.toml` file in it (§2.6). Every `.i` file under its `src/` or `test/` directory is a **module** of the package.

The **path** of a module is the `name` of its package (§2.6), then `/`, then its path under `src/` or `test/`, without `.i`, with `/` between the parts. In the package named `app`, the path of `src/users/admin.i` is `app/users/admin`. Every part of a path must be a `name`. Two modules of one package with the same path are a compile error.

Modules in `test/` are checked when their package is checked (§2.6), and compiled as test entries when its tests are run. They may import modules in `src/`, but not the other way.

A test module may be imported only by test modules of the same package. Its exports alone do not make its bindings used (§2.4.1).

In a module, the first part of a module path says which package the module is in. It must be one of these:

- `i`, for the **library modules** (§9, §10, §13, §14, §B.7.1) and kept paths named at the start of this document. How it gives them is not fixed: the operations that cannot be written in i may be built into the compiler, and they are not host imports (§10.1);
- the `name` of the module's own package;
- a dependency name of the module's own package (§2.6); in a test module, also a dev-dependency name.

A module may only import modules that are named in this way. Then the rest of the path is the path of the module in that package, as that package names it. So with the dependency `json`, `json/decode` is the module `src/decode.i` of that package.

Each package gives its own names to the packages it uses, so there is no space of package names that all packages share. All modules of the packages that a program uses, directly or through other packages, are compiled together. A type or a value is named by its package (§2.7), the path of its module in that package, and its name. So two definitions with the same name in different modules or packages are different, and two packages that give one dependency different names still use the same types.

If modules import one another in a circle, it is a compile error.

### 2.2 Form of a file

```
File      = [ Imports ] [ Exports ] { Item } .
Imports   = "(" ImportItem { "," ImportItem } [ "," ] ")" .
Exports   = "(" ExportItem { "," ExportItem } [ "," ] ")" .
Item      = { Attribute } ( TypeDef | Binding | Declaration | MacroItem ) .
Attribute = "@" [ name "." ] name [ Call ] .
MacroItem = [ name "." ] name Call .
```

`Call` is the call of §6.1, so the arguments of an attribute and of a `MacroItem` are read as expressions. A `(` on the same line right after the name of an attribute is always the start of the arguments of that attribute (§1.7). So when an attribute has no arguments and the item after it starts with `(`, as the binding `(even, odd) = …` does, the item begins on the next line:

```i
@m
(even, odd) = ((n) { n == 0 || odd(n - 1) }, (n) { n != 0 && even(n - 1) })
```

On one line, `@m (even, odd) = …` is the attribute `@m(even, odd)` followed by `= …`, which is a compile error.

At the start of a file, a group in `( )` that is not followed by `=` is not an item:

- When an item in it has a module path (§2.3), which always has a `/`, the group is the **imports**.
- When not, it is the **exports**: every item in it is a `name`, or an `UpName` with or without `(..)`.

A file has at most one imports group and at most one exports group. They come first, the imports before the exports. Neither may be empty: a file with nothing to import or export leaves the group out. Attributes may not come before the imports or the exports: a group in `( )` with an attribute before it is an item. A group in `( )` that is followed by `=` is a binding with a tuple pattern, as in `(even, odd) = …`. Any other group in `( )` at the top level, with or without attributes before it, is a compile error.

An item that is a `name`, with or without type parameters (§2.5), then `:` and a type, with no `=` after the type, is a declaration, whatever its attributes. Any other item that starts with an `UpName` is a type definition (§4.3). An item that is a call, `m(args)` or `a.m(args)`, with no `=` or `:` after its `)`, is a `MacroItem`. Every other item is a binding (§2.5).

A `MacroItem` is kept for macros. It is a compile error, and the message says that a call at the top level is kept for macros.

A declaration requires `@external` or `@js` (§10.1), or expansion by a macro. A declaration with neither attribute is kept for macros, where it goes with a binding of the same name. Until that expansion is defined, it is a compile error, and the message says that the declaration is kept for macros.

In one module, every top-level name is defined one time only. Two bindings may not bind the same name, two type definitions may not define the same type, and two constructors may not have the same name. A constructor and a top-level binding may not have the same name either, because they are in one space of names (§2.3).

These are the attributes. An attribute whose name is one of them, with no module name before it, is that attribute. Its arguments must be string literals, given by position, with no labels,. Any other argument, such as `@external("env" <> "", "f")`, `@external(module: "env", "f")`, is a compile error. Any other attribute, such as `@m`, `@m(args)`, or `@a.m(args)`, is a macro before an item. It is kept for macros, and it is a compile error whose message says so. An attribute in another place, or one attribute given two times on one item, is a compile error.

- `@js("expression")`: on a declaration of a closed opaque type (§10.1.1). A declaration cannot have both `@js` and `@external`.
- `@external("module", "function")`: on a declaration (§10.1). The module may not start with `ilang:`, which is kept for the runtime (§B.4).
- `@host("namespace")` and `@host("namespace", "async")`: on a type definition (§9.7). The namespace may not start with `ilang:`.
- `@deprecated("note")`: on any item. Using the marked item from another module gives a compile warning.

### 2.3 Imports

```
ImportItem = [ name "=" ] [ "(" ImportName { "," ImportName } [ "," ] ")" "=" ] ModulePath .
ImportName = name | UpName .
ModulePath = name "/" name { "/" name } .
```

Header groups are not record or tuple values. A colon does not introduce an import alias. An arbitrary parenthesised record is not a top-level item.

The imports and the exports are each a parenthesised group of items. In each group, the items have `,` or a break (§1.7) between them:

```i
(
  i/fail
  decode = json/decode
  opt = (Option, Some) = std/option
)

(main, User(..))
```

Every item gives a name to a module:

- With no `=`, the name is the last part of the path.
- `alias = path` gives the name `alias`.
- `(A, b) = path` also puts the exported names `A` and `b` into this module. They may then be used with no module name before them.
- Both may be used together: `opt = (Option, Some) = std/option`.

Importing an `UpName` puts in the type and the constructor with that name, if they are there. Only exported names may be imported. Two imports may not put in the same name. An imported name may not be the same as a top-level name of this module.

Module names, value names, and constructor names are in one space of names. So a module name may not be the same as a top-level name or another imported name of the module, and a local name may cover a module name. In `a.b`, when `a` is a value, `.b` is a field; when `a` is a module, `a.b` is a member of the module. Type names are in a different space, so a type and a value may have the same name.

### 2.4 Exports

```
ExportItem = name | UpName [ "(" ".." ")" ] .
```

- A top-level name that is not in the exports may only be used in its module.
- `T` exports the type `T` but not its constructors. Other modules may use the type, but they may not make or take apart its values, including by field selection or constructor update (§6.7). `T(..)` exports the type and all its constructors.
- Every exported name must be defined in this module.
- If the type of an exported definition uses a type defined in this module, that type must be exported too. If not, it is a compile error.

A type bound by an alias pattern is an alias with no constructors of its own; exporting it with `(..)` has the same effect as exporting it alone. An exported alias or annotation that depends on a value is checked after its type is complete and must not expose a private nominal type. Its source value need not be exported. Reading from another module never fixes that source’s weak type; a new permanent read requires a wholly closed completed source, or the consumer uses a closed alias exported by the source module (§4.7).

### 2.4.1 Unused top-level bindings

A top-level value name bound by a binding (§2.5) gives warning W0504 when it is not used. Names starting with `_` do not give this warning (§1.3). The warning does not change initialization or make `i check` fail.

A name is used when it is referenced in its module, exported from a module outside `test/`, an entry of its package (§10.2), a test, or named in `[wasm].exports`. References inside any binding, including unused bindings, function bodies, patterns, and written types, count. Merely importing a name does not use it. In test modules, an export is used when referenced by a module of the same package. Each name in a binding is checked separately; using one name does not make its other names used. A name starting with `_` suppresses its own warning.

This warning does not apply to type definitions, aliases, service definitions, constructors, or declarations (`@external` and `@js`). References in their written types count as uses. For an unused function in a test module, with no value parameters and result type `Nil`, the warning adds: "did you forget to register this function with `test`?" Environment parameters do not count as value parameters (§10.2). The hint is not given outside test modules.

### 2.5 Top-level bindings

```
Binding     = Pattern "=" Expr | name TypeParams [ ":" Type ] "=" Expr .
Declaration = name [ TypeParams ] ":" Type .
TypeParams  = "(" UpName { "," UpName } [ "," ] ")" .
```

A binding whose pattern is one name, with or without a type, may declare type variables after the name (§4.7): `map(A, B) = …`, `id(A): (A) -> A = (x) { x }`. A declaration may declare them in the same way: `inspect(A): (A) -> String`.

An item that starts with `name ( UpName { , UpName } )` and then `=` or `:` is always a binding or a declaration with type parameters. It is never a binding with a constructor pattern, though `f(A)` could also be read as one (§7.1). So `wrap(A) = (x: A) { Some(x) }` declares `A`. The same is true for a statement of a block (§6.1).

An item that is a call, `m(args)` or `a.m(args)`, and has no `=` or `:` after its `)`, is not a binding. It is kept for macros (§2.2).

An item `name [ TypeParams ] : Type` with no `=` after the type is a declaration, whatever its attributes. A declaration requires `@external` or `@js`, or expansion by a macro (§2.2). A `@js` declaration has the closed opaque type and foreign link semantics of §10.1. It is initialized once during start only if retained by the target link pass; ordinary bindings retain this section's initialization rules.

A top-level binding has the same rules as a binding in a block (§6.10). Only these things are different, and they come from its place:

- Its pattern may not begin with an `UpName`, because a top-level item that begins with an `UpName` is a type definition (§2.2).

- Its names may be used anywhere in the module, before or after it. So top-level functions may use one another.
- Its value is worked out once, when the program starts, before any entry point runs (§B.1). The order is below.
- It may not give errors or need an environment, because nothing is there to take them. So its pattern may not fail, because a pattern that may fail needs `Assert` (§4.8 rule 10).
- If working out a value stops the program, no entry point runs.

**Order.** Working out a direct value (§6.10) runs no code, so it only reads the names outside the function bodies in it. Working out any other value may call every function it can get to, so it may read every name in it, also inside function bodies. From this:

- The **full needs** of a binding are all the top-level names of this module in its value, also inside function bodies, and the full needs of their bindings.
- The **needs** of a binding with a direct value are the top-level names of this module outside the function bodies in its value, and the needs of their bindings. The needs of any other binding are its full needs.

A binding may not be among its own needs. If it is, it is a compile error: `x = [x]` and `x = f()`, where the body of `f` uses `x`, are compile errors, but `f = (n) { f(n - 1) }` is not. The values are worked out one at a time, with a module after the modules it imports. In a module, the next value worked out is always the first binding, in the order of the source, whose needs are all worked out, as in Go. So in `a = (1, z)  y = p("y")  z = p("z")`, the order is `y`, `z`, `a`.

**Type dependencies.** A type may name a value (§3). This does not work out that value or add it to runtime needs or full needs. It only orders type inference: a binding whose annotations use the type of a value of its module, directly or through aliases, is typed after that value's group (§4.7). A type dependency within one group, including on itself, or a cycle between groups is a compile error. The message gives the chain of dependencies.

### 2.6 The package file

The file `i.toml` is in TOML. These keys and the provider keys below have a meaning; other root keys are not used. Unknown keys inside a provider plan are errors (§2.6.1):

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

The optional `[entries]` table maps entry names (ordinary `name`s) to
strings `"module.path.function"`. The module path is relative to this package;
its components name directories below `src` or `test`, followed by the function. Targets must be top-level functions of this package;
they need not be exported. Each has no value parameters (environment
parameters do not count). All declarations are checked even when unselected.
Names are unique as TOML keys; several names may select the same function.
`main` in `src/main.i` is the default entry, if present; `[entries]` does
not replace it. A package without that function has no default entry.
Entry declarations and main functions in dependencies have no effect.

`i run` and `i build` select the default entry when no entry is named.
`i run NAME` and `i build NAME` select a key of `[entries]`; an unknown key is an error listing the declared names. Program arguments to `i run` follow `--`.
Entry selection changes only the program entry, not module visibility or
host exports. The selected function uses the existing entry checking,
provider, error, and calling rules of §10.2. An unused-binding warning must
exclude both the default entry and every declared entry.

### 2.6.1 Target provider plans

A target provider plan maps resolved service type identities to ordinary i factory functions or host registrations. Infer source types first. For each entry, supply every service left in its inferred environment and all dependencies of the chosen factories. Supply whole services, never selected fields. Missing required services and dependency cycles are compile errors naming the target, service and requiring entry. Catching operation errors, including `unsupported`, does not remove a requirement. A target with no default for a standard capability and no explicit provider is therefore a compile error even when all its operation errors are caught.

Every factory has an ordinary checked type, returns its service and has no uncaught typed errors. Construction may stop, but cannot add an error to the source entry's ABI error row. All source factories are type-checked; only the chosen factories' needs are provisioned. A reference to an unchosen factory in executable source still follows §10.1.2. Construct each graph node once per entry invocation, after top-level initialization. Captured services do not change when another service is later substituted (§9.2).

This is the full version-1 schema; string cells are its nullary shorthand. A service key or factory value is a fully qualified module path, a dot and its type or binding name, resolved through the package graph.
The package plan is data, not executable macro code. The root package selects target cells over the shipped plan; dependency packages cannot override its selections. A package may declare default cells for its own nominal service constructors; these retain that package's authority, and root cells override them. A dependency row for another package's service is an error. There is no wildcard or implicit service discovery. The shipped capability registry is plan data: a registered row without a cell for this target is unsupplied, including user-registered capabilities. An absent root cell inherits the shipped cell when one exists. A plan is resolved with
its declaring package's authority: it may name that package's private bindings;
references into dependencies must be exported. Inherited cells keep their
original declaring authority. Copying a dependency's private path into a
root cell is an access error. This gives every package the same plan-only access,
without making a private factory callable from ordinary outside source. It
preserves private root-runner construction. The rule does not change source visibility (§2.4).
Unknown keys, duplicate resolved rows/cells, unknown
targets, ill-kinded type arguments and unresolved names are errors. The five
columns are js, nodejs, native, wasi and host; ilang is a CLI alias only.

```text
providers."<resolved service type constructor>":
  role = "none" | "scheduler" | "runner"                 # default none
  adapter = "<accessible ordinary function>"               # iff role != none
  discriminant = {js=C, nodejs=C, native=C, wasi=C, host=C} # optional
  <target> = "<factory>"                                 # compact form
           | {factory="<factory>", arguments=[...], seed_required=false}
           | {host=true}
provider_adapters."<accessible ordinary function>":
  supplies = ["<closed service type>", ...]
  requires = ["<closed service type>", ...]
wasm.exports = ["<exported entry>", ...]
wasm.synchronous = ["<member of wasm.exports>", ...]
```

Exactly one variant occupies a target cell. `arguments` defaults to []; its
only elements are `"seed"` and `"entry-root-key"`, each at most once, in the
factory's argument order. A string is `{factory=..., arguments=[]}`.
`seed_required` defaults to false and is legal only with a seed argument.
A host cell is legal only for a declared host service; §9.7's open
custom-host registration is its implicit cell. A registered ordinary row
never falls through to host registration. A discriminant row may omit cells;
its default cell constructs `discriminant[current_target]`, provisioning
constructor fields as services. Each constructor must belong to that row's
type, and each field type must be a closed valid service type. Constructor
aliases resolve before validation; maps must cover all five targets but need
not be injective. An explicit cell replaces generation and disables §10.1.2's
certification for that row. Generated payloads are dependency edges too.

A row key is a nominal service constructor identity, not its spelling. A
parameterized row's factory scheme is instantiated at each required set of
type arguments, by ordinary type checking; no holes or free variables may
remain in a chosen instance. Distinct `Cell(Int)`/`Cell(String)` nodes stay
distinct. Each factory invocation binds only its own checked environment in
its lexical scope. It does not union incompatible instantiations into one
environment; §9.2's same-constructor restrictions still apply to each
actual function environment. This changes no source inference.

A factory for service S has type `(generated arguments) -> S ^ Never ! R`,
with R its ordinary inferred environment. The selected arguments have these
exact types and meanings. `Key(Nil)` and the request/task services below refer to the accepted R2 contracts in [runtime.md §§5–6](design/runtime.md); their definitions are installed by that track. This plan does not give them host-reference representations:

| Argument | Type | Value and dependency |
|---|---|---|
| seed | `List(S)` | [] outside the adapter; inside it, [s] when supplies contains S, otherwise []; a nonempty list depends on the adapter entering |
| entry-root-key | `Key(Nil)` | The initial registered root key of this entry's primitive context, available before factories; no service dependency |

`seed_required=true` requires exactly [s] at this construction site, or the
plan is rejected; it is not a runtime missing-service fallback. A seed is an
ordinary value; the factory decides how to use it. It may return it, wrap it,
or ignore it. Even with a seed, **the selected factory is called** and its
result is the chosen S. A nullary override receives no seed and still replaces
the chosen value. This prevents an adapter from silently undoing an Await,
Root or user-service override. Supplying a seed is not a second provider row.

The root-key argument serializes the narrow initial-root bootstrap:
open the primitive entry registry, register its initial key when this argument
is present in the graph, then let an ordinary factory construct the nominal
Root value. A graph needing no key registers no initial root. It is not a public unscoped root allocator. Any ordinary factory with this argument type can receive the
key; no rule recognizes Root or its module. All uses in one entry receive the
same initial key. The registry exists for synchronous fake-only entries too;
only entries classified async by §10.2 have async admission. The primitive opening is fixed,
dependency-free and constructs no replaceable service value. On every exit
it retires the entry's registered roots/groups, even after a stop.

The **role belongs to the row**. A root-package target cell replaces only the
provider cell; the inherited role, adapter and discriminant are unchanged.
Redeclaring inherited metadata identically is allowed; changing it is an
error. New application rows may declare any role with the same rules. A row
with a role must name a declared adapter, and a row with role none must not.
An override must still type-check as a provider of S; supplying a pure fake
does not erase its role. Request mode likewise remains the declared host mode.
All row/adapter declarations are schema-checked in a target plan; only chosen
factory dependencies need supplies. Source-only check does no plan validation.

An adapter declaration names an accessible ordinary generalized function with
this checked type (schema notation, R and S are the declared service unions):

```text
adapter(A,E,N): (body: () -> A ^ E ! N) -> A ^ E ! R | N - S
```

It is generalized over arbitrary A, E and N; no extra result/error restriction
or requirement is permitted. The declared closed supplies and requires lists
each form a valid ordinary environment. They may overlap **across** lists:
the required outer Runner and supplied inner Runner, for example, are different
lexical values. A candidate's inferred scheme must satisfy this entire type,
not only the current entry instantiation. There is no rank-two field or new
function kind: the plan names an ordinary top-level generalized function.
The adapter supplies S by calling body under ordinary environment bindings;
those bindings are captured as seeds by the generated continuation. It must
not assume its seeds are the final services used by the source entry.

The continuation has entry lifetime. Generated guards allow exactly one
invocation, wholly within the adapter invocation's dynamic extent. Re-entry
or a second invocation stops with `provider: body already called`; invocation
after adapter exit stops with `provider: body is not active`; returning normally
without a completed invocation stops with `provider: body was not completed`.
A stop may abandon the invocation; it runs no i finals and the primitive
context still retires. Suspension is not exit. Adapters may use ordinary
structured control and resource.ensure around body; they cannot detach it.
The guard is required for every adapter, not trusted library code. Ordinary
service values can escape normally; their later operations obey their own
lifetime rules. This is not a new linear type system.

Resolve and validate the graph as follows, before linking or optimization:

1. Resolve body providers transitively from the entry's full inferred needs.
   Each service uses its one target cell. Include all factory requirements,
   generated-constructor payloads and declared request modes, whether used at
   runtime or not. Choose roles from these rows, not their factories' names.
   Expansion must be finite: on any dependency path within one scope,
   revisiting a service constructor is rejected even at different type
   arguments (`provider: recursive dependency`). This conservative rule also
   rejects a finitely decreasing chain such as Box(Box(Config)) -> Box(Config)
   -> Config. Independent sibling instances remain legal. It prevents an
   expanding Cell(A) -> Cell(List(A)) plan from hanging the compiler, without
   adding type-level termination analysis. Completed nodes may be shared;
   sharing must not hide a repeated constructor on any graph path.
2. If a role is reached, include its adapter and resolve that adapter's required
   services in an **outer** scope using the same target cells. Include all roles
   and requirements reached there too. Repeat until closed. No adapter means
   no outer scope. More than one resolved adapter identity is an error naming
   the entry and both originating service rows and their adapter names. Repeated
   references to the same adapter are shared. Adapter references must resolve
   to a top-level function binding, allowing only direct lexical binding
   aliases; arbitrary computed function values cannot be plan adapter names.
   There is no nested composition.
3. Make one node per (outer/body scope, fully instantiated service). An edge
   to each required service stays in that scope. An adapter-enter node depends
   on all outer requires. Seed nodes depend on adapter-enter; a body factory
   with a seed argument depends on the appropriate seed if one is supplied.
   Outer factories always receive []; an unsatisfied required seed is an error.
   The primitive context/key is a leaf, never a service-provider dependency.
   Two scopes do not share provider instances, even if their factories match.
4. Check all supplied/required types, missing cells and cycles over this **one**
   graph, including adapter and seed edges. A factory cannot obtain an inner
   seed while constructing an outer prerequisite. Such a requirement is a
   plan error, not permission for a temporary default service. Source factories
   and adapters have already undergone ordinary type checking.
5. Execute a stable topological order. First materialize primitive arguments
   and host-registration leaves in canonical order; these run no i factory.
   Then, among ready service nodes, order lexicographically by canonical
   package/module/type path and normalized type arguments, then outer before
   body. Expand aliases and sort/deduplicate union members recursively when
   serializing closed type arguments; never sort by allocated TypeIds. Prefer
   all ready provider nodes to adapter-enter. Enter the adapter
   once its prerequisites are ready; finish the same order in its guarded
   continuation, then invoke the source entry with its chosen body services.
   No provider may execute before primitive admission is opened when async.

This ordering may construct a body service independent of seeds before adapter
entry; it is then an ordinary lexical capture in the continuation. It constructs
seed-dependent providers after entry. This is still one topological order, not
Root/Owner/Await followed by another sort. No second provider is substituted
to break a cycle. Classification includes both scopes and the adapter's
requirements; linkage includes the adapter and all chosen factory bodies.
A generated wrapper reference is not a *source* reference in §10.1.2's certification.

Shipped task rows use the same schema: scheduler/runner roles select one
ordinary adapter with the structured run type and supplied
Scheduler/Runner/Await/Owner/Root, requiring outer Runner. Seed-aware ordinary
factories return the supplied values inside that adapter; outside it they
construct the accepted default Runner through its private ordinary constructor
and the direct context services. The plan names that constructor with its
own package authority, never via a public root-runner API. The outer
Runner factory has no required seed and no dependency cycle. Root's direct
factory takes entry-root-key; Owner's direct factory needs Root. Seed-aware
factories still declare their full ordinary environments on both paths; those
requirements are provisioned even when a seed is present. The seeded default
run remains FIFO, path [], seed 0, with no entropy draw. No special factory
elision, role erasure or hidden context provider is permitted.

#### 2.6.2 Synchronous exports and commands

`wasm.synchronous` is a list of host exports and must be a subset of `wasm.exports`. A listed export whose complete chosen provider graph classifies async (§10.2) is a compile error showing a qualifying dependency path. Otherwise the caller uses the async entry protocol when the metadata says async, even if the first call returns status 0. No source type promises synchronous host calling, and no implicit blocking adapter changes that contract. WASI retains §B.7's blocking ABI.

The target columns are `js`, `nodejs`, `native`, `wasi` and `host`. `--target ilang` is an alias of `--target host`, with no removal scheduled. Legacy host artifacts keep Appendix B's format. The native provider column does not require an AOT executable: execution location and provider selection are separate.

| Command | Provider column / Target value | Validation and execution |
|---|---|---|
| `i run` | native / Native | Validate the selected entry plan; execute in process |
| `i test` | native / Native | Validate test entry plans; execute in process with the normal fresh test instances |
| `i build` | selected target; default native | Validate that target's entry plans; emit its artifacts: for native, one executable `build/<name>` (below); for the other targets, the module `build/<name>.wasm` |
| `i check` | none | Parse, resolve and infer all source, including test bodies; no target plan selection |
| `i check --target t` | t, with ilang normalized to host | Check source and the plans for that target's main, configured exports and eligible tests; emit and execute nothing |

WASI has no compiled test entries under §B.7: its check still checks test source,
but constructs no test wrapper/plan. Targeted checking validates supplies,
adapter types/conflicts, cycles, synchronous contracts and §10.1.2's foreign
availability. Untargeted checking promises none of these: success does not
promise that any target can build or run the program. Neither check form
promises actual host grants, engine features, successful initializers, or
runtime results. Malformed manifest syntax is still a package-load error.
The target-independent source checker retains entry arity and ABI checks, but does not reject an entry merely for requiring an ordinary service. These command rules concern provider selection; native packaging remains a runner plus an embedded compiled program, requires main, and rejects only required imports that runner cannot supply. A package with only host exports uses host (or its ilang alias).

**Native packages.** `i build` with the native target writes one executable file, `build/<name>`, where `<name>` is the package name or the entry `NAME` of `i build NAME` (`--out` chooses another path). It is the native runner — Wasmtime's runtime without a compiler, the standard host services and the reactor of `i run` — followed by a payload: a manifest, the program's `ilang.*` metadata and its code compiled ahead of time, sealed by a SHA-256 digest. The manifest records the contract the code was compiled for (Wasmtime version, target triple, CPU feature baseline, collector and engine settings), the digest of the module and its trusted binding plan; the runner loads nothing else, and refuses a missing, truncated or damaged payload and a payload whose contract is not its own. The executable runs the program entry with its arguments, environment and standard streams, and exits as `i run` does. On Linux the executable is statically linked and needs no shared library; on macOS it depends only on system libraries. Its code is compiled for the runner's target triple (on Linux the musl triple of the building machine's architecture); cross-target builds and signing are not specified. Building fails, with the namespace and label, for each required import the runner cannot supply, including a declaration it does not grant (§9.7); a service replaced by ordinary i code needs no import.

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
Prim      = FnType | TupleType | RecordType | NamedType | TypeCall | ValueType | Literal | "_" | "{" Type "}" .
FnType    = "(" [ ParamType { "," ParamType } [ "," ] ] ")" "->" Union [ "^" Union ] [ "!" Union ] .
ParamType = [ name ":" ] Type .
TupleType = "(" Type { "," Type } [ "," ] ")" .
RecordType = "(" FPart { "," FPart } [ "," ] ")"
           | "(" ".." Type { "," RPart } [ "," ] ")" .
FPart      = Label ":" Type .
RPart      = FPart | "-" Label .
Label      = name | string_lit | UpName .
NamedType = [ name "." ] UpName [ "(" Type { "," Type } [ "," ] ")" ] .
ValueType = [ name "." ] name .
TypeCall  = [ name "." ] name "(" [ TypeArg { "," TypeArg } [ "," ] ] ")" .
TypeArg   = Type .
Literal   = int_lit | "-" IntPart | string_lit .
IntPart   = int_lit | "{" IntPart "}" .
RecordType = "(" ".." Type { "," RPart } [ "," ] ")"
           | "(" FPart { "," FPart } [ "," ] ")" .
RPart      = Label ":" Type | "-" Label .
FPart      = Label ":" Type .
Label      = name | string_lit | UpName .
```

- An `UpName` with no module name before it is a type variable when a type variable with that name is declared where the type is written (§4.7, §5), and a type when not. `_` is a hole that type inference fills in.
- A `Literal` is a literal type (§4.2): `"a"`, `0`, `-1`. A `-` followed by an integer literal, also inside groups, where a `Prim` begins, is part of the literal: `-{1}` is `-1`. Every other `-` is a difference. So `Int - -1` is `Int` without `-1`, `Int - 1` is `Int` without `1`, and `-1 | 1` is a union of two literal types. A float literal is not a type, because Float has no literal types.
- A type variable takes no arguments: `A(Int)`, where `A` is a declared type variable, is a compile error.
- A lowercase `x` in a type, without `(` after it, means the type of the value named `x` at that place. The name follows value scope (§5). `m.x` means an exported value of module `m`, never a field of a value. If `m` names a value, it is a compile error. `{ x }` groups the same type. A lowercase name or member followed by `(` remains a macro call, not a value type or a type application.
- A type may not name a value bound by that same pattern; a parameter in the types of that function's own header, including its `->`, `^` and `!` parts; or its own binding's reserved name inside that binding's value. A distinct inner binder that has entered scope may cover that reserved name. This is a compile error even if an outer value has the same name. Those parameters may be named in types in the body. For example, `(x, y: x)` and `(x) -> x { x }` are errors, but `(x: Int) { y: x = x  y }` is allowed.
- A value type is not its known type (§4.8). It follows the scheme, instantiation and completion rules of §4.7. It never makes a monomorphic value polymorphic.
- A value type, directly or through aliases, may not occur in a custom constructor field, an external or host declaration, or a service/environment type. Those types must be complete before inference.
- A lowercase `name`, or `a.m` with a lowercase `m`, whatever `a` is, followed by `(` is a `TypeCall`. It is kept for macros, and is a compile error. It is in the grammar, so a type with it is read in full, and the error says that it is kept for macros.
- `A | B` is a union, `A & B` is an intersection, and `A - B` is a difference (§4.4). `-` is stronger than `&`, and `&` is stronger than `|`.
- `( … )` with `->` after it is a function type. Without `->`, a first `..` or label followed by `:` selects a record type; other nonempty contents select a positional tuple type. Empty `()` is a compile error suggesting `Record`. A record parameter type needs nesting: `((a: Int)) -> T`; `(a: Int) -> T` has one Int parameter labelled `a`.
- A parameter of a function type may have a label: `(in: String, each: String) -> String` (§4.6).
- A function type goes as far to the right as it can: `(A) -> B | C` gives `B | C`. `{ }` puts a type in a group: `{ (A) -> B } | C`, `(A) -> { (B) -> C } ! E`.
- In a clause pattern, `?` begins the guard even after a function type.
- `^` and `!` are part of the nearest `->`. `^` comes before `!`. A `^` or `!` that the nearest `->` cannot take, because it already has one or because `!` came first, is a compile error: in `(A) -> (B) -> C ! N ^ E`, the inner function type is put in `{ }` to give `^ E` to the outer `->`.
- In a written function type, a missing `^ E` means `^ Never`, and a missing `! N` means `! Never`. This is the same in every place where a type is written.

- `Record` is the type of all records. A `{ }` in a type groups exactly one type. Breaks in record types separate fields; breaks directly in type groups are errors.
- A record label is a `name`, a string with no interpolation, or a declared label parameter (§4.9.4). A fixed label starting with a capital letter is written as a string. Labels are compared after decoding escapes: `a`, `"a"`, and `"\u{61}"` are the same label. A label may not occur twice, including as a field and a forgotten label.
- `(..R, f: T)` sets the field type; `(..R, -f)` forgets the field; `(..R)` is `R & Record` (§4.9.2).

## 4. Types

### 4.1 Types are sets of values

Every type is a set of values.

- Values of different type constructors are never the same.
- Values of different forms of one type constructor, such as `List(Int)` and `List(String)`, may not be told apart at run time (§7.1).
- Records (§4.9) are values of one type constructor.
- Tuples with different numbers of parts are different. The values of a function type are functions.

A **member** of a type (§4.4) is a type constructor, its type arguments, and a **value set**: the values of that type constructor that are in the member. Only `Int` and `String` may have a value set that is not all of their values (§4.2). For every other type constructor, the value set is all of its values.

### 4.2 Built-in types

These types may be used in every module:

| Type | Values |
|---|---|
| `Int` | Whole numbers of any size |
| `Float` | Finite IEEE 754 binary64 numbers |
| `String` | Lists of Unicode scalar values, kept as UTF-8 |
| `List(A)` | Lists that may not be changed |
| `BitArray` | Lists of bits; the number of bits may be any number |
| `UtfCodepoint` | One Unicode scalar value |
| `Never` | No values |
| `Record` | All structural records; no type arguments |

These built-in types are normal custom types (§4.3) with these definitions:

```i
Bool { True False }
Nil { Nil }
Result(A, E) { Ok(A) Error(E) }
```

The types `Assert` and `AssertionFailure` are in §9.6. The types `Boundary` and `Continuation` are in §9.4. The types `Key` and `Keyed` are in §9.8. Request(A) and Ticket are built-in types, with operations in i/request (§9.11). Request(A) contains request handles whose answer belongs to A. Ticket is an answer-erased handle. Neither has public constructors.

#### Literal types

Every value of `Int` and of `String` is also a type, the **literal type** with only that value. It is written as the literal: `1`, `-1`, `"a"`. `1 <: Int`, and `"a" <: String`. Two Int literal types are the same when their numbers are equal, so `0x10` is `16` and `-0` is `0`. Two String literal types are the same when their scalar values are equal (§11.4).

The value set (§4.1) of an `Int` or `String` member is one of these:

- all the values: `Int`, `String`;
- a finite set of values: `"a" | "b"`;
- all the values but a finite set: `String - "a" - "b"`.

Union, intersection and difference of these sets give a set of the same kind, so the rules of §4.4 and §4.5 always have an answer, and inference still finds principal types (§4.7). `Float`, `BitArray` and `UtfCodepoint` have no literal types.

Values are never changed, so a literal type is never made wider: `x = "a"` gives `x` the type `"a"`, and `[200, 404]` has the type `List(200 | 404)`. Such a value may be used where a `String` or an `Int` is needed, because `"a" <: String`. Operators give the whole type: `1 + 1` has the type `Int` (§6.9).

### 4.3 Custom types

```
TypeDef     = UpName [ TypeHead ] TypeBody .
TypeHead    = "(" Type { "," Type } [ "," ] ")" .
TypeBody    = "{" { Constructor } "}" | "=" Type .
Constructor = UpName [ "(" Field { "," Field } [ "," ] ")" ] .
Field       = [ name ":" ] Type .
TypePattern = UpName | "_"
            | [ name "." ] UpName "(" TypePattern { "," TypePattern } [ "," ] ")"
            | name "." UpName
            | "(" [ TypePattern { "," TypePattern } [ "," ] ] ")" .
```

- `T { C1 C2 … }` defines a nominal custom type. Each constructor has zero or more fields, which may have labels. Two custom type definitions are different types even if their fields are the same.
- `T {}` has no constructors; i code cannot make its values. They come from the host (§10), including `@js` declarations with the full closed nominal annotation of §10.1.1.
- `T = Type` is a transparent alias. Aliases may not use themselves, directly or through other aliases or alias patterns.
- An item `T(P1, …, Pn) = v` is an **alias pattern** exactly when its written right side is a value type (§3), after removing groups. Otherwise its head is a list of type parameters. This classification is made before expanding the right side; it never depends on imports or on what a macro returns there.
- In a list of type parameters, each part is a different, unqualified UpName with no arguments, and must not already name an accessible type. Every parameter must be used in the body, and every type variable in the body must be a declared parameter.
- In an alias pattern, the unqualified head names an existing type, with the written arity; the item does not define that head. It may be a custom or built-in type, or a transparent alias expanding to one named type or tuple. In a head alias each parameter is used at most once, only as a named argument or tuple part. Constant subtrees of that alias require an equal, fully determined source part.
- A bare unqualified UpName in a pattern always defines a new zero-parameter structural alias. Its part of the source type must be fully determined (§4.7). It may not duplicate a definition or conflict with an imported or built-in type. It never tests an existing type with that name.
- `_` in an alias pattern skips a part and binds nothing; it is not an inference hole. For a new read after source completion, the whole source type must first be closed (§4.7); a skip cannot hide an open argument from that check. Elsewhere in a type, `_` remains a hole. Shared type variables remain shared even if one occurrence is skipped.
- A named pattern with arguments, or a qualified named pattern, requires exactly one member with that type constructor and matches its arguments. A tuple pattern requires one tuple of the same size. No union member is discarded to make a match. A mismatch is a compile error. Function, record, literal, union, intersection, difference, value-type and macro-call patterns are compile errors.
- Every pattern and every value-dependent alias is checked, also when unused. Its demands are processed at the completion of their source (§4.7), and its results are assembled after its alias dependencies are complete. A new read after source completion follows the closed-source rule in §4.7. A definition read may not choose an instance of a generalized variable to make its pattern fit.
- Blocks have no type definitions. An item beginning with `(` or `{` is a value binding; `m.T(…) = v` is also a value binding under §2.2. Import `T` to use it as an alias-pattern head.

```i
Codec(D, E) { Codec(decode: (String) -> D, encode: (E) -> String) }
make(D, E) = (d: (String) -> D, e: (E) -> String) {
  Codec(decode: d, encode: e)
}
size = make((s) { string.length(s) }, (n: Int) { int.to_string(n) })
Codec(Size, _) = size
// Size = Int; the distinct encode part is not fixed.

Pair(A, B) = (A, B)
Pair(Tree, _) = parsed
// If parsed has the closed type (Ast, List(Warning)), Tree = Ast.
```

### 4.4 Union, intersection, and difference

- The values of `A | B` are the values of `A` and the values of `B`. Order has no effect, `A | A` is `A`, and `A | Never` is `A`.
- Members of a union with the same type constructor are put together into one. For `Int` and `String`, the value set of the member is the union of their value sets: `"a" | "b"` is one member, and `"a" | String` is `String`. `T(A1, …) | T(B1, …)` is `T(C1, …)`, where for each parameter of `T` (§4.5 rules 6 and 7), `Ci` is `Ai | Bi` when the parameter goes in the same direction as the type, and `Ai & Bi` when it goes in the other direction. When the parameter does not change, `Ai` and `Bi` must be equal, and `Ci` is that type; if they cannot be equal, it is a compile error. Tuples of the same size count as one type constructor, and so do all function types, whose parameters go in the other direction and whose value, errors, and environment go in the same direction. Function types with different numbers of parameters cannot be put together; that is a compile error. A label is kept only where both have it. So `List(Int) | List(String)` is `List(Int | String)`. All records have one runtime type constructor. Record unions are an exception to same-constructor merging: they retain their alternatives and denote set union. Record intersections combine all required fields (§4.9). A spread is also a record alternative; runtime overlap and head removal count every alternative as a record.
- The **members** of a type are the types that make it up as a union, after these rules are used. So no two members of a type have the same type constructor, except that a union may have several record and spread alternatives (§4.9.3). A type variable may be a member.
- The values of `A & B` are the values that are in both `A` and `B`. `{ A | B } & C` is `A & C | B & C`. When `T` and `U` are not type variables, `T & U` is `Never` when they have different type constructors; when they have the same one, it is that type constructor with each pair of arguments put together the other way round: `&` where the parameter goes in the same direction, `|` where it goes in the other direction, and equal where it does not change. For `Int` and `String`, the value set is the values in both; when there are none, `T & U` is `Never`. So `"a" & String` is `"a"`, and `"a" & "b"` is `Never`. `a & T`, with a type variable `a`, stays as it is.
- Intersections come from inference. `(x) { x + 1  x }` has the type `(x: a & Int) -> a`: the value that comes in is given back, and it must be an `Int`.
- `A - B` takes out of `A` every member whose type constructor is the type constructor of a member of `B`. For `Int` and `String`, it takes out only the values of the value set of that member of `B` (below). These rules work it out:
  - `{ A | B } - C` is `A - C | B - C`, and `A - { B | C }` is `A - B - C`.
  - When `T` and `U` are not type variables: `T - U` is `T` when they have different type constructors. When they are both `Int` or both `String`, `T - U` has the values of the value set of `T` that are not in the value set of `U`, and is `Never` when none are left: `String - "a"` is every string but `"a"`, `{"a" | "b"} - "a"` is `"b"`, and `"a" - String` is `Never`. For every other type constructor that is the same in both, `T - U` is `Never`, and `T <: U` must be true (below). Tuples of the same size count as one type constructor, and so do all function types.
  - When `T - U` is `Never`, `T <: U` must be true. This is a constraint for inference, like any other. When it cannot be true, it is a compile error: for example `List(Int | String) - List(String)`.
  - The right side of `-` may not have a type variable as a member. `a - U`, with a type variable `a`, stays as it is until `a` is known.
- So `{ Int | String | Nil } - String` is `Int | Nil`, `A - A` is `Never`, and `{ Yield(a) | n } - Yield(a)` is `n - Yield(a)`.
- `{ Int | String | Nil } - "a" - 1` is `Int - 1 | String - "a" | Nil`.
- Unions, intersections, and differences may be used in every place where a type may be used: values, errors, and environments. In a written type, a union with two or more type variables as members may only be in an output place, and an intersection with a type variable only in an input place (§4.5 rule 6), because inference cannot choose between them in the other places. The parameters of a function are input places; its `->` part, the type at the top of a binding pattern, and the fields of a type are output places. Record fields and spread bases preserve the direction of their surrounding place. A spread with a field or a forgotten label is not an intersection with a type-variable member, so it may be in any place; `(..R)` alone is `R & Record`, so it may only be in an input place when `R` is a type variable. A record union is not restricted by its number of alternatives or its place. Closed record unions are ordinary types in every position; their uses obey §4.9.3. The existing restrictions on ordinary variable members still apply to expanded annotations and shown types, including after substitution and completion.

### 4.5 Subtypes

`A <: B` says that every value of `A` is a value of `B`. These are the rules:

1. `Never <: A`, and `A <: A`.
2. `A <: A | B`. `A | B <: C` when `A <: C` and `B <: C`. `A & B <: A`. `C <: A & B` when `C <: A` and `C <: B`.
3. `A - B <: A`. `X <: A - B` when `X <: A` and `X` and `B` are **disjoint**: every member of `X` and every member of `B` have different type constructors, or are both `Int` or both `String` with value sets that have no value in common. So `"a"` and `"b"` are disjoint, and `"a"` and `String - "b"` are not. Only the members themselves are compared, not their type arguments: `(1, 2)` and `(3, 4)` are not disjoint. A type variable as a member is never disjoint from anything.
4. `(P1, …) -> R ^ E ! N <: (Q1, …) -> S ^ F ! M` when the numbers of parameters are the same, `Qi <: Pi` for every parameter, `R <: S`, `E <: F`, and `N` is **in** `M`: every member of `N` is a type variable that is a subtype of `M`, or has a member of `M` with the same type constructor and equal type arguments and the same value set. Labels have no effect. Service arguments are compared as equal, not by direction, because a service is a value that goes in and comes out: with directions, a function that needs `Yield(Int)` could be given a `Yield(Never)`.
5. `(A1, …) <: (B1, …)` when the numbers of parts are the same and `Ai <: Bi` for every part.
6. A type parameter of a custom type changes in the same direction as the type, in the other direction, or not at all. This is decided from the definition: a parameter that is only in output places goes in the same direction; one that is only in input places goes in the other direction; one that is in both does not change. Field types are output places, the parameters of a function type are input places, and a place inside an input place is the other way round.
7. `List(a)` goes in the same direction as `a`. In `Continuation(a, r, e, n)`, `a` goes in the other direction and `r`, `e`, `n` in the same direction. `Boundary(r, e, n)` does not change.

8. A member of `Int` or `String` is a subtype of a member with the same type constructor when every value of its value set is in the value set of the other. So `"a" <: "a" | "b" <: String`, and `String - "a" <: String`, but `String` is not a subtype of `"a" | "b"`, and `String - "a"` is not a subtype of `"b"`.

9. Records obey the width, intersection, exact union, and forgetting rules of §4.9.3.

Two types are **equal** when each is a subtype of the other. A value of a subtype may be used in every place where a value of the type is needed.

`Key(A)` (§9.8) does not change. `Request(A)` (§9.11) goes in the same direction as A. The variance of task types follows their ordinary custom definitions (§9.9).

### 4.6 Function types

The function type `(T1, …, Tn) -> A ^ E ! N` has three parts after the arrow. They are in the same order as `Effect<A, E, R>` in Effect-TS:

- `A`: the type of the **value** that the function gives back.
- `E`: the **errors** that the function may give in place of a value (§9.3).
- `N`: the **environment**: the service types that the function needs (§9.2).

`E` and `N` are normally unions, and `Never` means none. The number of parameters is part of the type. There is no currying and there are no default parameters. Environment parameters (§9.2) are not among `T1 … Tn`; their types are in `N`.

A parameter of a function type may have a **label**. Labels are not part of the meaning of a type: two function types that are the same but for labels are equal. A label is only used to put the arguments of a call in order (§6.6). The type of a function expression has the labels of its parameters (§6.11), and a written type has the labels written in it. When a type is made from function types that have different labels for one parameter, that parameter has no label.

### 4.7 Type inference

Types are worked out by inference with subtypes (algebraic subtyping).

- Type annotations may be left out everywhere for inference, but not on `@external` or `@js` declarations. Some uses of a name need a known type, which may need an annotation (§4.8).
- Target provider selection and foreign linking (§2.6, §10.1.2) do not change inferred source types or remove inferred requirements.
- For every binding, the compiler must find the principal type: the most general type, of which every other possible type is an instance. MLsub (Dolan and Mycroft, POPL 2017) and MLstruct (Parreaux and Chau, OOPSLA 2022) give ways to do this.
- A type is made simpler before it is shown: equal parts are put together, and type variables that have no effect on the meaning are taken out. This never changes which programs are correct or what they do. A type scheme that is used as a known type (§4.8) is in this simpler form. A type variable that is not generalized is shown as the union of the types that come to it.
- Annotations are used in inference, and the inferred type must agree with them. A type variable in an annotation may become a more exact type.
- A type variable is **declared** after a binding name (§2.5, §6.10), `map(A, B) = …`; after a declaration name, `inspect(A): (A) -> String`; or in the list of type parameters of a type definition (§4.3). The parts of an alias pattern are not declarations of type variables: its bare UpNames define structural aliases.
- The type variables declared in one place must have different names. So `pair(A, A) = …` is a compile error.
- A type variable declared by a binding may be used in the type on its pattern, as in `id(A): (A) -> A = …`, and in every type written in its value, also in the parameters, `->` parts, and clause patterns there, and in inner bindings. It is one type variable of that binding, and it is generalized with it. An inner binding may declare a type variable with the same name; inside it, the name means its own variable.
- A type variable declared by a declaration may be used in the type of the declaration. It is general: each use of the declared name may give it a different type.
- An `UpName` in a type that is neither a type nor a declared type variable is a compile error. The message says how to declare it.
- A declared type variable may not have the name of a type that may be written with no module name in the module (a type of the module, an imported `UpName`, or a built-in type), because then a type would mean two things.
- A type variable may only be declared on a binding whose pattern is one name, `x`, or one name with a type, `x: T`. A binding with another pattern, such as `(even, odd) = …`, uses the type variables of the bindings it is inside. A top-level binding with another pattern is inside no binding, so it may not use type variables; its values are written as bindings of one name each when they need them.
- **A binding is generalized when its value is a direct value** (§6.10): the type variables of its type that are not in the types of names from outside it become general, so each use of the name may give them different types. A binding whose value is not a direct value is not generalized, because working out its value runs code, and that code may make a value for one type only (for example with `control.suspend`). This is the value restriction of ML, which is known to be safe. Functions are direct values, so every function binding is generalized.
- A type variable that is not generalized is one type for all uses of the binding.
- Parameters (§6.11) and clause patterns (§6.12) are matched like bindings, but they are not generalized. Their values are not known where they are written, so each of their names has one type in the whole body or clause. So `(f) { (f(1), f("a")) }` needs `f` to take both `Int` and `String`.
- Inside its own value (§6.10), a name of a binding is not general: all its uses there have one type. It becomes general when the binding is generalized.
- A **group** is a set of top-level bindings that use one another, directly or through other bindings of the group. The bindings of a group are worked out together and then generalized together, after the groups that they use. Inside the group, their names are not general.
- A type that has itself inside it, through a type variable, is a compile error. Record fields and spread bases count as places inside a type constructor.
- Simplification and display preserve type variables affecting invariant arguments, including inside records. Reading a type never generalizes a weak variable. Label parameters alone are rigid labels (§4.9.4).
- A long union of literal types may be shortened in messages and hovers, as `"a" | "b" | … (40 more)`, but never in a type scheme or in the interface of a package. An inferred type may have literal types in it, also in the interface: an exported constant or an unannotated exported function whose value is a literal has that literal type, so giving one more literal as a value changes the interface. Exported functions should have written types, as the standard library has.
- Simplification and type display must preserve type variables that affect invariant arguments, even when their input and output forms print alike.

**Reading types of values.**

- In an annotation, a completed generalized value is read as a fresh instance of its scheme (§4.8 rule 3), separately for each occurrence. A parameter, clause name or local monomorphic value shares its one type. Generalized local schemes also keep their non-general variables shared. A completed top-level value uses its fixed non-general parts; reading an unfixed part is a compile error. Reading through a local does not copy such a part or allow a structural alias to escape from it. A value type never generalizes its source or its annotated binder.
- In an alias body or alias pattern, every used value-derived part must be closed: it has no generalized variable, unresolved type variable, label parameter, unknown top type or hole. The alias's own declared formal parameters remain parameters. A skip demands no contents, but the shapes needed to reach the used parts must also be determined.
- A top-level value's group has one **completion point**, after its values are typed and linked and its generalization is decided. All value-type demands written in its own module are collected before inference, including annotation reads and unused type definitions. An annotation read takes part in fixation like an alias read; its quantified parts still use fresh scheme instances. Ordinary group checks finish first. All demands at this completion point read one unchanged snapshot and are checked in one batch before any permanent alias is published. At this point their weak parts are fixed as follows. A demand in another module never fixes them.
- For a demanded non-general variable, collect all types that come to it, including through variable relationships. Put them together by union, replacing the non-general variables needed inside them by their fixed types. Pure cycles of variable relationships count once. A structural recursive type, an unfilled part, a generalized variable needed inside this replacement, or a part that cannot be made closed is a compile error. An explicit closed `Never` is allowed; the absence of any information does not invent `Never`. Literal types are never widened.
- Fixing adds both subtype constraints between the variable and the resulting closed type, keeping every previous constraint. Every needed variable is fixed together, and the usual occurs and place rules apply. If these equations cannot hold, it is a compile error. Pattern shape checks use the original argument variables, so skipping a distinct argument leaves it shared and unfixed. The pattern cannot supply a type to fill an unknown.
- A group that constrains an earlier non-general variable that was not fixed cannot fix its own non-general parts. A demanded part carrying such an earlier variable also cannot be fixed. This conservative rule makes the result independent of unrelated groups' inference order.
- From then on, later uses must agree with each fixed part. A use that would widen it incompatibly is a type error with a note at the place that fixed it. Write a wider annotation at the source when needed. For example, reading `r = cell(1)` may fix its invariant argument to `1`; write `r: Cell(Int) = cell(1)` to allow other Int values.
- In another module, or when a new definition read is performed by a later group, an alias pattern or other permanent value-derived definition must start from a wholly closed completed source type. A fixed component behind an open enclosing type is not sufficient, even when the open arguments are skipped. This read adds no constraints to the source. If the source remains open, the error says to use the closed alias made by its source module and to export it when needed. Annotation instantiation and current-group local monotype sharing keep their rules above. Adding foreign fixation demands cannot change the source module's completion result. Ordinary expression constraints and local monomorphic sharing remain shared as elsewhere in this section; this rule does not fix the display of every uncompleted weak part.
- A permanent alias denotes the completed closed part. It does not change when later bindings are checked. A same-module top-level pattern is handled at its source's completion, regardless of where the item is written. Aliases with several sources assemble already closed per-source results, not saved partial source types. Definitions are completed before their dependents. Importing or using the closed alias later does not read its source value again.
- The whole annotation after value types and aliases are read must obey §4.4's rules of places. A value-type spelling does not hide variable members from that check.

Simplification and type display must preserve type variables that affect invariant arguments, even when their input and output forms print alike. Reading a type never changes a weak variable into a generalized one.

### 4.8 Typing rules

`Γ ⊢ e : A ^ E ! N` says: with the names in `Γ`, the expression `e` has the type `A`, may give the errors `E`, and needs the environment `N`. `Γ` gives every name a type scheme `∀a1 … an. T`. A program is correct when every binding has a type by these rules. Every rule may use any `E` and `N`; the parts of one rule use the same `E` and `N`, unless the rule says otherwise.

1. **Subtypes.** If `e : A ^ E ! N`, `A <: A'`, `E <: E'`, and `N <: N'`, then `e : A' ^ E' ! N'`.
2. **Literals** have the types of §6.3.
3. **Names.** If `Γ(x) = ∀a1 … an. T`, then `x : T` with each `ai` changed to any type.
4. **Functions.** Let the parameters be the patterns `q1 … qn`, and the environment parameters `x1 ! T1 … xk ! Tk`. Each `qi` is fitted to a type `Pi` (below). If `Γ` with the names of the `qi` and `xi : Ti` gives `body : A ^ F ! M`, then the function has the type `(P1, …, Pn) -> A ^ F ! M | T1 | … | Tk | S`, with the labels of its parameters. `S` is `Assert` when some `qi` may fail for `Pi` (§7.3), and `Never` when not. Making the function needs nothing, so this rule may use any `E` and `N` for the function expression itself.
5. **Calls.** If `f : (P1, …, Pn) -> A ^ E ! N` and every `ai : Pi`, then `f(a1, …, an) : A`. Arguments with labels are first put in the places of §6.6.
6. **Tuples and lists.** If every `ei : Ti`, then `(e1, …, en) : (T1, …, Tn)`. If every `ei : T` and `r : List(T)`, then `[e1, …, en] : List(T)` and `[e1, …, en, ..r] : List(T)`.
7. **Constructors** are functions from their field types to their type, with the field labels (§6.3). Fields and updates follow §6.7.
8. **Operators** are not functions. Each has the types of §6.9. `a |> f(b)` has the type of `f(a, b)`.
9. **Blocks.** `{ s1 … sn e }` has the type of `e`, with the names bound by `s1 … sn` (§6.10). For an environment statement `! T = e1` and the rest `r` of the block after it: if `e1 : T ^ E ! N1` and `r : A ^ E ! M`, then together they have `A ^ E ! N1 | M - T`. So `e1` uses the environment from outside the statement.
10. **Bindings.** For `p = e` with `e : A`: `p` is fitted to `A` (below). If `p` may fail for `A` (§7.3), then `Assert <: N`.
11. **Clauses.** This rule is for a clause block with no clauses, with more than one clause, or with a guard (§6.12). Let it be called with a value of type `A`: in a match `e |> { … }` with `e : A`, or as a function whose parameter has the type `A`. The clause patterns (each choice of `|` counts) are looked at by **place**. The top place is the value of the parameter. Below a pattern at a place, its parts are at places of their own: each field of a constructor `C`, each part of a tuple of n parts, and the parts and the rest of a list. Places below different constructors are different places. At each place, let `B` be the type of the values there (`A` at the top place, and else the part type of the pattern type above it), and let `P` be the union of the pattern types (below) of the patterns at that place that are not catch-alls. Patterns at one place with the same type constructor use the same new type variables, so `Ok(v)` and `Error(e)` at one place give one `Result(b1, b2)`. A type test `x: T` at a place also needs `B <: T | r - T`, with a new type variable `r`: it takes every value whose type constructor is that of a member of `T`, and that, for a member of `Int` or `String`, is in the value set of that member (§7.1), so those values must be in `T`. So `v |> { x: Box(Int) -> …  y: Box(String) -> … }` with `v: Box(Int | String)` is a compile error. A constructor pattern with `..` counts as having a catch-all at the places of the fields that it leaves out. If some clause without a guard has a catch-all at that place or at a place above it, then `B <: P | r - P`, with a new type variable `r`: a value that no pattern takes may still come there. If not, then `B <: P`. So a clause block with only literal clauses and no catch-all takes only those values: `{ 0 -> "zero"  1 -> "one" }` has the type `(0 | 1) -> "one" | "zero"`. In a match `e |> { … }` whose subject already has a type with values that are in no literal of `P`, so that `B <: P` is not true only for that reason, as `(n: Int) { n |> { 0 -> "zero"  1 -> "one" } }`, the error is the one of §7.4, with a value that no clause matches, and no other error is given for it. A clause block that is not the right side of a match gets its values later, through the type of its parameter: from a call, or from the type it is given to, as in `list.map(xs, { 0 -> "zero"  1 -> "one" })` with `xs: List(Int)`, or `f: (Int) -> String = { 0 -> "zero"  1 -> "one" }`. Then there are two errors, also when its parameter has a known type from an expected type: a type error where the values come, and the error of §7.4 at the clause block, which is also not covered. With no clauses, `P` is `Never` at the top place, so `A <: Never`. A catch-all at a place gets the type of rule 12. Every guard and every clause expression has the errors `E` and the environment `N`, every guard has the type `Bool`, and every clause expression has the type `R`. Then the match `e |> { … }` has the type `R`, and the clause block has the type `(A) -> R ^ E ! N`, whose parameter has no label. Making the clause block needs nothing, as for a function in rule 4. Coverage (§7.4) is checked with the types that inference finds.

    A clause block with one clause and no guard is not typed by this rule. It is the function `(p) { e }` of rule 4: its pattern is fitted to the type of the parameter (below), it is not checked for coverage, and `S` is `Assert` when the pattern may fail. A match `s |> { p -> e }` with such a clause block is typed as the binding `p = s` followed by `e` (rule 10), with the one difference that `p` is not generalized, as a parameter is not. So the subject `s` gets the expected type that a binding gives its value (below).

12. **Narrowing.** Let a catch-all be at some place in a clause pattern, and let `B` be the type of the values at that place. A type constructor `T` is **taken** at that place when every value that the clause pattern matches, with a value of `T` at that place, is matched by an earlier clause without a guard. In the same way, a value `v` of an earlier `Int` or `String` literal pattern at that place, and a member of the type of an earlier type test at that place, is **taken** when every value that the clause pattern matches with it at that place is matched by an earlier clause without a guard. The catch-all gets the type `B - H`, where `H` is the union of the taken members: each taken type constructor with new type variables as its arguments, the literal type of each taken value, and each taken member of a type test. So in `s |> { "a" -> 0  rest -> f(rest) }` with `s: String`, `rest` gets `String - "a"`. So in §9.3, `other` gets `HttpError`, because the clause before it takes every `Error` with a `ParseError` in it.
13. **Fields and tuple parts.** For `v.f`, every constructor that may have made `v` (§6.7) must have a field with the label `f`. The inferred result is the union of those field types with the type arguments from the corresponding bound forms, or the constructor-pattern arguments already supplied by §7.5. The inferred type of `v` must fit the bound. With no possible constructors, the result is `Never`. For `v.k`, `v` must have a known type and every member must be a tuple with more than k parts. Its inferred type must fit the bound of that known type, and the result is the union of parts k from those bound forms. If not, it is a compile error. The bound takes all Int and String values at covariant places, removes exclusions of only some of those values there, and takes fresh holes at noncovariant places. Use this bound also when a known function type constrains a call. Known field and tuple results are obtained separately from the original known facts, not by narrowing the inferred result to them. A known type may have fewer values than the inferred type: after `x = "a" = s` with `s: String`, `x` has the type `String` and the known type `"a"`, and `(x, 0).0` is correct.
14. **Generalization.** A binding whose value is a direct value is generalized (§4.7) after its value is typed. Other bindings, parameters, and clause patterns are not.
15. **Records.** If every `ei : Ti`, then `(f1: e1, …, fn: en) : (f1: T1, …, fn: Tn)`. If `r : R`, `R <: Record`, and every `ei : Ti`, then `(..r, f1: e1, …) : (..R, f1: T1, …)`. `() : Record`.

A **catch-all** is a `name`, a discard, or `x = q` where `q` is a catch-all.

To **fit** a pattern `q` to a type `B`, in a binding, a parameter, or a clause block with one clause and no guard: if `q` is a catch-all, its names get `B`. If not, `B <: Q`, where `Q` is the pattern type of `q`, and the parts of `q` are fitted to the parts of `Q` given below. Fitting never adds `Assert`. In a clause block with no clauses, with more than one clause, or with a guard, rules 11 and 12 take the place of fitting.

Only a clause block with one clause may have choices `q1 | … | qn` where a pattern is fitted. If one choice is a catch-all, the names get `B`. If not, `B <: Q1 | … | Qn`, where `Qi` is the pattern type of `qi`, and choices with the same type constructor use the same new type variables. The parts of each `qi` are fitted to the parts of its `Qi`, and a name gets the union of its types in the choices (§7.2). In choices, `x: T` is a test, not a type annotation (§7.1), so `T` may not have a type variable as a member. The choices may fail for `B` when together they do not match every value of `B` (§7.3); then `S` in rule 4 is `Assert`.

The **pattern type** of a pattern that is not a catch-all, and its parts, are:

- A literal of `Int` or `String`, also `-` and an integer literal: in a clause block with more than one clause or with a guard, its literal type (§4.2); where it is fitted (a binding, a parameter, or a clause block with one clause and no guard), `Int` or `String`, so that every value of that type may come there and the pattern may fail (§7.3). At a place whose type is shared with another place, or inside one, also `Int` or `String`: an item of a list pattern, because all the items of a list have one type, and a field of a constructor whose written type has a type parameter that occurs more than once in all the fields of all the constructors of its type (counting two in one field, and through aliases), as both fields of `Pair(A) { Pair(A, A) }`, the fields of `Start` and `Stop` in `Event(A) { Start(A)  Stop(A) }`, and the parts of the field of `W(A) { W((A, A)) }`. A literal type at one such place would be a type for all of them. So when that type is not written and does not come from the subject, a clause block that matches those places only with literals is not covered (§7.4): `(xs) { xs |> { [] -> 0  ["a", ..] -> 1  ["b", ..] -> 2 } }` gives `xs` the type `List(String)`, and the error names a value such as `[""]`. With `(xs: List("a" | "b"))` written, the same block is covered. A float literal: `Float`.
- `C(p1, …)`: `T(b1, …, bm)`, the type of `C` with a new type variable for each type parameter. The part for `pi` is the type of the field that `pi` is matched to (§7.1), with these variables.
- `(p1, …, pn)`: `(b1, …, bn)`, with new type variables. The part for `pi` is `bi`.
- `[p1, …, pn]` and `[p1, …, pn, ..r]`: `List(b)`, with a new type variable. The part for each `pi` is `b`, and `r` gets `List(b)`.
- `"lit" <> r`, `r <> "lit"`, and `"a" <> r <> "b"`: `String`, and `r` gets `String`.
- A bit array pattern: `BitArray`. The parts get the types of their formats (§8.2). A literal value in a segment also has the type of its format, not its literal type.
- `x = q`: the pattern type of `q`, and `x` gets that type.
- `x: T`: `T`, with new variables for holes and value types read as §4.7 says; `x` gets that type.

A value type in an annotation uses the inferred type of its source, not a narrower known type at the use. A completed scheme gives its known structural form, with `_` for general variables. For a local source still being inferred, use its stored structural monotype with unresolved variables erased to `_`. A written type on the source name is part of that monotype. The known type of a matched value does not supply a type for a name inside its pattern. Do not infer a smaller shape from temporary lower bounds. Later literal tests or clause narrowing do not refine this shape. Thus `((a: Int, b: String)) { q: a = a  q == 1 }` gives `q` the known type `Int`. For `x = "a" = s` with `s: String`, x's inferred type is String even though its known type is "a"; an annotation naming x uses String.

Record fields and spread bases are covariant in the hidden-input and whole-literal operations below; invariant arguments inside them still count in both directions. A field written in a literal or after a spread gets its matching expected record field, when guaranteed. The base expression of a spread gets no expected type.

The **known type** `K(e)` of an expression is found from the parts of the expression, left to right, and from the expected type of its place (below). Where no case below gives it, `e` has no known type.

- A record literal has the record of the known types of its fields, with `_` for a field that has none. A spread has the base’s conservative record summary, forgetting equal or possibly equal labels, and the known types of its new fields. An unknown base supplies all records. Only guaranteed fields shared by every possible record alternative survive; non-record and unknown alternatives of an enclosing union remain. Thus `(..e)` always has a record known type, and rule 13b reads the guaranteed field known type separately from inference.
- A literal, also `-` and an integer literal, also inside groups, has its type (§6.3), and a bit array has `BitArray`. A tuple has the tuple of the known types of its parts, with `_` for a part that has none. A list has `List` of the union of the known types of its parts when they all have one, and `List(_)` when not. So `[]` has `List(Never)`.
- An operator expression has the result type of §6.9; `a |> f(…)` is the call `f(a, …)`.
- A match `e |> { … }` has the union of the known types of its clause expressions when they all have one. A record update `C(..v, f1: e1, …)` has the known type of the call of `C` with the given fields and the other fields of `v` as its arguments.
- The names of a parameter get known types as names in patterns (below), but the value of a parameter has no known type. So `x: T` gives `x` the type `T`, and a parameter with no type has no known type. An environment parameter `x ! T` has `T`.
- A built-in name, an imported name, and a member `m.x` of an imported module have their type scheme. So does the name of a generalized binding (§4.7), local or top-level, where it is used outside its own value and outside its group. A type variable in such a scheme that is not generalized is replaced by the matching part of the known type of the binding's value, or by `_`. A constructor has its function type, with the field labels (§6.3).
- A function expression has the type made from: the labels of its parameters; for each parameter, the type written at its top (as in `x: T`), or else the type it gets from an expected type (below), or else `_`; and for the result, the written `->` part, or else the known type of its body, or else `_`.
- A clause block that is not the right side of a match has a known function type with one parameter, made as for a function expression: the label of its parameter (§6.12); for the parameter, `Never` when the block has no clauses, the type written at the top of its pattern when the block has one clause, no guard, and no choices, or else the type it gets from an expected type, or else `_`; and for the result, the union of the known types of its clause expressions when they all have one, or else `_`. When the parameter has a known type `B`, from a written type or from an expected type, every clause pattern is a pattern fitted to a value with the known type `B`, as in a match (the bullet on matches, before **Names in patterns**, below), and the narrowing by earlier clauses of §4.8 rule 12 is used in the same way. So with `opts: List(Option(User))`, `list.map(opts, { Some(u) -> u.name  None -> "" })` is correct, as `list.map(opts, (o) { o |> { Some(u) -> u.name  None -> "" } })` is. In the bullets on expected types, such a clause block counts as a function expression of one parameter.
- A name used inside the value of its own binding, and a top-level name of the same group, get their known types only from what is written, because their values are not yet known there: from `x: T` at the top of the binding's pattern when it is there, or else from what is written in the value, with `_` for the rest. So, as in TypeScript, a function that uses its own result in `.f`, `.k`, or a call by label needs a written `->` part. Every other name bound by a pattern gets its known type as below.
- A known type may have `_` in it, for a part that is not known. A part that is `_` has no known type, but the rest of the type is still known. So the labels of a function are known even when its types are not.
- A call `f(a1, …)`, also when written with `|`, has a known type when `f` has a known function type, and every type variable of its result gets a type when its parameter types are matched with the known types of the arguments (below). Then it has the result type with those types put in. A constructor with arguments is a call.
- To **match** a type `P` with a known type `K`: when `P` is a type variable, `K` is one of its types. When `P` is a named type, a tuple type, or a function type, and `K` has one member with the same type constructor, the parts are matched: the type arguments, the tuple parts, or the parameter and result types. When `P` is a union, each member of `P` that is not a type variable is matched with the member of `K` that has its type constructor, if there is one, and each type variable in `P` is matched with `K` less the other members of `P` (§4.4). A type variable in `P` that is matched with a `_` in `K`, or with a type variable that is generalized in `K`, gets `_`, because that part may be any value. When `P` is an intersection, each of its members is matched with `K`. Before matching, a difference in `K` is replaced by its members (§4.4), and an intersection `a & T` by `T`. When a type variable gets more than one type, it gets their union. So matching `(x: a | Nil)` with `User | Nil` gives `a` the type `User`.
- **Expected types.** An expression may have an **expected type** from its place, as in TypeScript:
  - An argument of a call (also with `|`) whose callee has a known function type has the type of its parameter there. Its type variables are put in when matching the parameter types with the known types of the arguments before it fixes them. Function expressions that have a parameter with no written type are left out of this matching, and come last: their expected types use all the other arguments. A function given as an argument may give values to the called function that the other arguments do not show: as its result or errors, as the argument of a function it is given (a parameter of a parameter), or to a service of its environment. A service in the called function's own environment gives it values that come from the caller's environment statement. And a function that the called function gives back may get values from a later caller. Walk the type of the called function, also its own environment: its parameters are given to it, its result is given by it, and every type variable in its own environment is given to it (an environment statement of the caller gives it those values), and the direction turns at each parameter of a function type and at each contravariant argument of a constructor; an argument of a constructor that is invariant or bivariant, and an environment, count as both directions. A type variable at a place given to the called function, other than a place in a parameter reached from the top of that parameter only through parts of tuples and covariant arguments of constructors (the argument shows the values there), is matched as usual, and then its type is made whole, both in the expected type of every argument and in the known type of the call (the bullet on a call `f(a1, …)` above): every `Int` and `String` member, also in an argument of a tuple or a constructor, or in the result of a function type, has all its values, `Never` there is `_`, and every other part that is not covariant (a parameter of a function type, an argument of a constructor that is not covariant, an environment) is `_`. So in `list.fold(xs, 0, (n, x) { … })`, `n` has the known type `Int`, not `0`; with `(0, 0)` as the start, `(Int, Int)`; with `[]` as the start, `List(_)`. With `loop(S): (init: S, body: (S, (S) -> Nil) -> Nil) -> Nil`, in `loop(0, (n, again) { … })`, `n` has the known type `Int`. With `make(S): (init: S) -> ((S) -> S) -> S`, `make(0)` has the known type `((Int) -> Int) -> Int`. With `cur(S): (seed: S) -> S ! State(S)`, `cur(0)` has the known type `Int`, so `cur(0) == 5` is correct.
  - The value of a binding whose pattern has `x: T` at its top has `T`. The body of a function with a written `->` part has that type.
  - The last expression of a block has the expected type of the block, and every clause expression of a match has the expected type of the match. A part of a tuple or a list has the matching part of the expected type, when that type has one member, a tuple or a list.
  - When a function expression has an expected type with one member, a function type with the same number of parameters, every parameter with no type written at its top is fitted to the matching parameter type, as for names in patterns below. This only gives known types. So `list.map(users, (u) { u.name })` is correct when `users` has a known type.
  - In a match `s |> { p -> e }` whose clause block has one clause, no guard, and no choices, when `p` has `x: T` at its top, `s` has the expected type `T`, as the value of a binding `x: T = s` does. So `(u) { u.name } |> { f: (User) -> String -> f(bob) }` is correct, as `f: (User) -> String = (u) { u.name }` followed by `f(bob)` is.
- `v.f` and `v.k` have the field or part type of rule 13, when it has no type variable that is not put in.
- A block has the known type of its last expression.
- In a match `e |> { … }`, every clause pattern is a pattern fitted to a value with the known type of `e` (below), also when the clause block has one clause and no guard. So with `opt: Option(User)`, `opt |> { Some(u) -> u.name }` is correct, as `Some(u) = opt` followed by `u.name` is.
- **Names in patterns**: when a pattern `q` is fitted to a value with a known type `B`: a catch-all gets `B`, less the members that rule 12 takes out. If `q` is not a catch-all, take the one member of `B` with the type constructor of `Q`; for a literal pattern with the value `v`, only that value, if the member has it: `B & v`, with `v` the literal type of the value (not `B & Q`, because where a pattern is fitted `Q` is the whole `Int` or `String`); if there is not exactly one, the names of `q` get no known type. The parts of `q` then have the known part types of that member, with its type arguments put in; the binder `r` of a string prefix, suffix, or combined pattern has the known type `String`, all its values, because the known type of the whole value says nothing about its remaining part. The name `x` of `x = q` gets that member. The name `x` of `x: T` gets `T`, whatever the value. So in `f: (x: Int) -> Int = (y) { … }`, `f` has the label `x`, from its annotation.

So `(u) { u.name }` needs a type, `(u: User) { u.name }`, but with `opt: Option(User)`, `opt |> { Some(u) -> u.name  None -> "" }` is correct.

`Record` follows ordinary built-in lookup and may be shadowed, like `Int`. The empty record value is `()`; its open type is `Record`, not an exact zero-field type. Records have name or noninterpolated string labels; record types also admit declared uppercase label parameters. Tuples, tuple types and tuple patterns have at least one positional part. Empty tuple patterns are compile errors suggesting `Nil` or a binder; there are no record patterns. `{ T }` still groups a type. Bare `..` remains in list and constructor patterns and constructor export groups.

## 5. Names and scope

- Built-in names (§12) may be used in every module. Imported names and top-level names may cover them.
- A top-level name may be used anywhere in its module (§2.5).
- A local name may be used from the next statement to the end of its block. When the value of its binding is a direct value, it may also be used inside the function bodies in that value (§6.10). The names of a parameter may be used in the function body. A name in a pattern of a clause may be used in its guard and its expression.
- A type variable declared on a binding (§4.7) may be used in the type on its pattern and in every type written in its value. A type variable declared on a declaration may be used in the type of the declaration. A type parameter of a type definition may be used in that definition (§4.3). An alias pattern has no type parameters; its binders are types of the module.
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

### 6.2 Order of evaluation

i is strict. Expressions are worked out from left to right. In a record `(f1: e1, …)`, fields are done in the order written. In `(..e, f1: e1, …)`, `e` is done first:

- In a call, first the function, then the arguments from left to right as they are written (also when they have labels), then the call.
- The parts of tuples, lists, constructor arguments, and bit arrays are done from left to right. Structural record fields are done in written order, the spread base first; sorting labels never changes this order. In a record update `C(..v, f: e, …)`, `v` is done first, then the fields as they are written.
- In a binary operation, first the left side, then the right side. `&&` and `||` do not do the right side when the left side is enough.
- `a |> f(b)` does `a` first, and then the rest in the order of `f(a, b)`.

### 6.3 Literals and names

The type of an integer literal is its literal type (§4.2): `1` has the type `1`. When the operand of `-` is an integer literal, also inside groups, the result has the literal type of the negative number: `-1` and `-{1}` have the type `-1`. Every other `-e` has the type `Int`, as `-x` and `-{-1}`. The type of a string literal is its literal type: `"a"` has the type `"a"`. The type of a float literal is `Float`. The value of a name is the value it is bound to. A member of a module is `module.name`, or an imported name (§2.3). A constructor with no fields is a value. A constructor with fields is a function whose parameters are its fields, with their labels.

### 6.4 Tuples

The type of `(e1, …, en)` is `(T1, …, Tn)`. n is 1 or more: `(x)` is a tuple of one part. `t.k`, where k is a decimal number, is part k, counting from 0. A k that is too big is a compile error. `{ }` puts an expression in a group (§6.10).

Tuples have positional parts only; a parenthesised datum with labels is a record.

### 6.5 Lists

The type of `[e1, …, en]` is `List(T)`, where every part has the type `T`. `T` may be a union: `[1, "a"]` has the type `List(1 | "a")`, and may be used where a `List(Int | String)` is needed. `[e1, …, en, ..rest]` puts the parts in front of the list `rest`.

### 6.6 Calls

`f(a1, …, an)` calls the function `f`. Every function is called in this way, whatever expression gives it.

- An argument may be given by position or by label. Arguments by position come before arguments by label, and they go to the parameters in order, from the first. Arguments by label may be in any order.
- A call with an argument by label needs a known function type for `f` (§4.8). Each such argument is put in the place of the parameter with that label in this type. If there is no such parameter, it is a compile error.
- Every parameter must get exactly one argument. Environment parameters (§9.2) are not parameters of the type, so they get no argument.

A labelled argument is not a record argument; use `f((label: value))` to pass a record. Calls and constructor fields may still mix positions and labels.

### 6.7 Constructors and fields

`C(a1, …, an)` makes a value. It is a call (§6.6); the labels are the field labels.

Selecting a field of a nominal value requires the same constructor visibility as taking it apart by a constructor pattern (§2.4). Exporting `T` without its constructors permits neither field selection nor constructor update outside its module. Structural record fields retain their own rules.

`v.f` gets the field with the label `f` from the value `v`. The constructors that **may have made** `v` are the one constructor that `v` is known to be made by (§7.5), when there is one. If not, and every member of the known type of `v` (§4.8) is a custom type, they are all the constructors of these types. When `v` has no known type, or a nonempty known type with only record, unknown, or variable members, `v.f` reads a field of a record (§6.13). In every other case it is a compile error. See §4.8 rule 13. Each of them must have a field with the label `f`, but it may be in any position and of any type (§4.8 rule 13). So, as in TypeScript, `v.f` is correct on a union when every member has the field, and its type is the union of the field types.

`C(..v, f1: e1, …)` makes a new value. Its fields are the given fields, and the other fields come from `v`. `v` must be known to be made by `C`: its type has only `C`, or §7.5 says so. The type of the new value comes from `C` and the types of all its fields.

### 6.8 Functions

`(p1, …, pn) -> A ^ E ! N { body }` is a function value. It may use local names from outside. The `->` part may be left out; then the value type, errors, and environment are inferred. When the `->` part is written, it is the type of the function after its parameters, so a missing `^` or `!` in it means `Never` (§3), and `! N` must have the types of the environment parameters, and `Assert` when a parameter may fail (§4.8 rule 4). The body is a block, and the value of the block is the value that the function gives back.

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

The operators take values of literal types, because these are subtypes of `Int` and `String`. Their results have the whole type: `1 + 2` has the type `Int`, not `3`, and `"a" <> "b"` has the type `String`. The one exception is `-` on an integer literal, also inside groups, which has a literal type (§6.3): `-1` and `-{1}` have the type `-1`, but `-x` has the type `Int`.

The unary operators `-` (Int) and `-.` (Float) are stronger than all binary operators. Calls, `.f`, and `.k` are stronger still. There is no operator for "not"; the function `core.not` (§13) does this.

- Int and Float have different operators. One is never changed into the other without a function such as `core.to_float` (§13).
- `a |> f` is `f(a)`, and `a |> f(b, …)` is `f(a, b, …)`. The right side of `|>` must be a name, a call, a function, or a clause block.
- `|>` is weaker than `+` and stronger than the operators of levels 1 to 5: `||`, `&&`, `==`, `!=`, comparisons, and `<>`. So the left side of `|>` that is made with those operators is put in a group: `{ a == b } |> { True -> …  False -> … }`.
- `e |> { … }`, where the right side is a clause block, is a match (§6.12).
- A match is an operation at level 6, so an operator of level 7 or 8, `.f`, `.k`, or a call after the clause block is part of the right side: in `n |> { 0 -> 1  _ -> 2 } * 10`, the right side is `{ 0 -> 1  _ -> 2 } * 10`, and in `opt |> { … }.name`, it is `{ … }.name`. In the same way, the left side of `1 + n |> { … }` is `1 + n`, as in `{ 1 + n } |> { … }`, and the left side of `-n |> { … }` is `-n`. So a match that is an operand of such an operator or of `-` or `-.`, or that has `.f`, `.k`, or a call after it, is put in a group: `{ n |> { 0 -> 1  _ -> 2 } } * 10`, `1 + { n |> { … } }`, `-{ n |> { … } }`, `{ opt |> { … } }.name`.
- When both sides of `==` or `!=` have known types (§4.8), and these are disjoint (§4.5 rule 3), the compiler gives warning W0503, showing both types: the two sides can never be equal. The comparison is well defined and has type `Bool`: `==` is always `False` and `!=` is always `True`. A member of a known type that is `_` or a type variable is never disjoint from anything. Literal types count. So `1 == "a"`, `1 == 2`, `1 != 2`, `"a" == "b"`, `x == "b"` after `x = "a"`, and `(s: String) { s == 1 }` give the warning, but `(s: String) { s == "a" }`, `(m: "a" | "b") { m == "b" }`, `x == Nil` with `x: Int | Nil`, `[1] == []`, `a == b` with a type variable, and `(s) { s == 1 }` do not. Only the top members of the two types are compared (§4.5 rule 3), so `(1, 2) == (3, 4)` and `Some(1) == Some(2)` give no warning, though they are always `False`. Literal types are not widened. The warning adds no narrowing fact. A clause guard `? x == 2` with `x: 1` is always false; coverage treats it as any guard (§7.4). What `==` does is in §11.4.

### 6.10 Blocks and bindings

The statements of a block are done in order. The value of a block is the value of its last expression. The last statement may not be a binding or an environment statement. A block with only one expression puts it in a group: `{ a + b } * c`.

An expression statement that is not the last, and whose working out can do nothing but make its value, gives a compile warning: its value is not used. These are literals, names, functions, clause blocks, and tuples, lists, constructor calls, `.f`, `.k`, `-`, and Int, Bool, String, and comparison operators made only of these. A call, `|>`, a match, a block, a bit array, and a Float operator, which may stop (§11.3), do not give the warning. So `assert 1 + 1 == 3`, which is the two statements `assert` and `1 + 1 == 3` (§1.7), gives two warnings.

`p = e` is a binding. It works out `e` and matches it with the pattern `p`. The names in `p` may be used from the next statement. A binding whose pattern is one name, with or without a type, may declare type variables after the name, as in `twice(A) = (f: (A) -> A, x: A) { f(f(x)) }` and `id(A): (A) -> A = (x) { x }` (§4.7).

- The type of `e` must be a subtype of the pattern type of `p` (§4.8 rule 10). So `x: Int = e` says that `e` has the type `Int`, and `(a, b) = v` with `v: (Int, Int) | Nil` is a compile error; a match is used for this.
- If `p` can not fail for the type of `e` (§7.3), the binding needs no environment.
- If not, as in `Ok(v) = r`, when it runs, the binding works like the match `e |> { p -> …  _ -> a.fail(AssertionFailure(m)) }`, where the first clause goes on with the rest of the block. The binding is still typed by §4.8 rule 10, not by rule 11. Here `a` is the `Assert` service of the environment, and `m` is the text `pattern `, then the source text of `p`, then ` did not match `, then the text form (§11.8) of the value. So `Assert` is in the environment of the binding (§9.6).

`name = pattern` is itself a pattern (§7.1). So `a = (x, y) = e` binds `e` to `a` and to `(x, y)`.

A **direct value** is an expression that is made without running any code: a function expression or a clause block; a literal, also with `-` or `-.` before it, also when the literal is inside groups (`-{1}`, `- |> {1.5}`); a name; or a tuple, a list, a record with no `..`, a constructor call, or a group `{ e }` whose parts are direct values. When the value of a binding is a direct value, the names of the binding may also be used inside the function bodies in that value. This is the same for every binding, local or top-level. So a local function may call itself, and one tuple binding may make functions that call one another:

```i future-SYN2
fact = (n) {
  n |> {
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

### 6.12 Clause blocks and matching

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

### 6.13 Records

`(f1: e1, …, fn: en)` makes a record (§4.9). An explicit label may not be given twice; `a` and `"a"` are the same label. `(..e, f1: e1, …)` makes the record `e` with the given fields added or put in place of its fields, including fields already in `e`; `e` must be a record. `()` is the record with no fields; `{}` is the clause block with no clauses (§6.12).

`v.f` reads the field `f` (§4.8 rule 13). A field whose label is not a `name` is read with `record.get`. Fields are set and removed with `record.put` and `record.remove` (§14):

```i
ada = (name: "ada", age: 36)
older = (..ada, age: ada.age + 1)
r = () |> record.put("name", "ada") |> record.put("age", 36)
```

## 7. Patterns

### 7.1 Grammar

```
Pattern    = name "=" Pattern | StringPat | Atom .
StringPat  = string_lit "<>" Binder [ "<>" string_lit ] | Binder "<>" string_lit .
Binder     = name | discard .
Atom       = name | discard | ( name | discard ) ":" Type
           | [ "-" ] int_lit | [ "-." ] float_lit | string_lit
           | Name [ "(" [ PatArgs ] ")" ]
           | "(" Pattern { "," Pattern } [ "," ] ")"
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
- The type after `:` goes as far to the right as it can, as every type does (§3). So `x: Int | String` tests for a union, and `f: (Int) -> Int = e` needs no brackets. In a clause, `p: (Int, Int) -> e` is read as a function type, so a tuple type there is put in `{ }`, or a tuple pattern is used. A `?` after a function type begins the guard; error slots use `^`. A missing clause arrow after a function type is an arrow/type-boundary error; group a tuple or record type in `{ T }` when needed. A `|` after a type is part of the type, so a type test is written after the other choices: `True | _: Int`. The type is read with no going back: once `( … )` is followed by `->`, it is a function type, and once a `|` follows a type, the next part is a type. When what follows cannot be read as a type, the pattern is wrong; it is not read again as a shorter pattern. So `v |> { p: (Int, Int) -> 0  _ -> 1 }` is a compile error, in the first clause and in any other. In `v |> { x: Int | y -> a  _ -> b }`, `Int | y` is a type; `y` is a value type (§3). But `v |> { x: Int | 0 -> a  _ -> b }` is correct, because `0` is a literal type (§4.2) and `Int | 0` is `Int`; so is `_: Int | "a"`.

`x: T` is one thing in every place where it may be written (in a bit array segment, `:` begins a format instead, §8.1). It matches the values of the type `T`, and `x` gets the type `T`. In a binding or a parameter, every value that comes there is in `T` (§4.8 rules 4 and 10), so `x: Int = e` and `(x: Int) { … }` are type annotations. In a clause block with more than one clause or with a guard, and in a pattern with choices, `x: T` tests the value. In a clause block with one clause, no guard, and no choices, the pattern is a parameter (§6.12), so `x: T` there is a type annotation. At run time, a member of `T` is tested by its type constructor; a member of `Int` or `String` whose value set is not all the values is also tested by equality (§11.4) with the values in it. So `x: "a" | "b"` takes the strings `"a"` and `"b"`, and `x: String - "a"` takes every other string, but no Int. So in a test, `T` may not have a type variable as a member. Custom types with no constructors cannot be told apart at run time, so for a type test they all count as one type constructor, as all function types do.

A type test whose type is a record or spread tests only the record head: every possible record there must satisfy the written field requirements (§4.9.3). There are no record patterns.

### 7.2 What patterns match

| Pattern | Matches | Names |
|---|---|---|
| `x` | Any value | `x` |
| A discard | Any value | None |
| A literal, or `-` or `-.` and a literal | An equal value (§11.4) | None |
| `C(p1, …)` | A value made by `C` whose fields match their parts (§7.1) | Those of the parts |
| `(p1, …, pn)` | A tuple of n >= 1 positional parts that match | Those of the parts |
| `[p1, …, pn]` | A list of exactly n parts that match | Those of the parts |
| `[p1, …, pn, ..r]` | A list of n or more parts; `r` is the rest | Those of the parts, and `r` |
| `"lit" <> r` | A string that starts with `"lit"`; `r` is the rest | `r` |
| `r <> "lit"` | A string that ends with `"lit"`; `r` is the start | `r` |
| `"a" <> r <> "b"` | A string with that prefix and suffix, without overlap; `r` is the middle | `r` |
| `x = p` | What `p` matches | `x` (the full value), and those of `p` |
| `x: T` | A value whose type constructor is that of a member of `T`; for a member of `Int` or `String`, the value must also be in the value set of the member | `x` |
| `<< … >>` | See §8 | |

- A string pattern has exactly one binder (or discard), whose inferred and known type is `String` (§4.8). Its literals have no interpolation. `a <> "x" <> b` is a compile error. Prefix and suffix must not overlap: `"ab" <> r <> "ba"` does not match `"aba"`.
- A pattern may not bind one name two times.
- In choices `p1 | p2`, every choice must bind the same names. A name gets the union of its types in the choices.
- `=` is stronger than `|`.

### 7.3 Patterns that may fail

A pattern **can not fail** for a type `T` when it matches every value of `T`. Other patterns **may fail**. A catch-all can not fail for any type, and `x: U` can not fail for a type `T` with `T <: U`, so `id(A) = (x: A) -> A { x }` needs no `Assert`. A value of a type variable may be any value, so a pattern that tests more than its type, such as a literal or a constructor, may fail for it.

A literal pattern can not fail for its literal type, so `"a" = x` with `x: "a"` needs no `Assert`. Where a pattern is fitted (§4.8), a literal has the whole `Int` or `String`, so `"a" = s` with `s: String` is correct and may fail: it reports to `Assert`. Whether a pattern may fail is decided with every value that may come, also when the value is a direct value of the same group (§6.10), whose values are known only when the whole group is checked: in `pair = ("b", () { ("a", _) = pair  1 })`, the binding may fail, and the function reports to `Assert`. The same holds for a match or a clause block with one clause and no guard.

### 7.4 Covering every value

The clauses of a clause block whose parameter has the type `A` **cover** `A` when, for every value of `A`, some clause without a guard has a pattern that matches it. A clause with a guard is not counted. A clause is **never used** when every value that its pattern matches is matched by an earlier clause without a guard.

A literal pattern matches its own value, and string prefix/suffix patterns match the strings with their fixed ends. So literal patterns cover an `Int` or `String` member whose value set is finite, when every value in it is matched: with `m: "GET" | "POST"`, `m |> { "GET" -> 1  "POST" -> 2 }` is covered. Literal patterns alone never cover a member with all its values, or with all but a finite set; a catch-all, a type test such as `x: String - "a"`, or, for `String`, the patterns `"" <> r` and `r <> ""`, which match every string, must take the other values. A literal in an item of a list pattern, or in a field of a constructor whose type parameter occurs more than once in the fields of its type (`Pair(A) { Pair(A, A) }`, `Event(A) { Start(A)  Stop(A) }`), has the whole `Int` or `String` (§4.8), so such literals cover a finite set only when that type is written or comes from the subject: `(xs: List("a" | "b")) { xs |> { [] -> 0  ["a", ..] -> 1  ["b", ..] -> 2 } }` is covered, and the same block with `(xs)` is not. Literal patterns of Float and BitArray never cover their type. A union is covered when every member is covered, and `{}` covers only `Never`.

When the values that are not covered are only values of `Int` or `String` that no literal pattern takes, the type of the clause block already says so (§4.8 rule 11: `(n: Int) { n |> { 0 -> "zero"  1 -> "one" } }` needs `Int <: 0 | 1`). In a match whose subject already has such a type, that is reported as this error alone, with a value that no clause matches, such as `2`. A clause block that is not the right side of a match gets these values later, through its parameter, so there it is a type error where the values come and also this error (§4.8 rule 11). When a value that is not covered is at a list item place, or at such a field, where the clauses have only literals, the message also says to write that type, such as `List("a" | "b")` or `Pair("a" | "b")`.

Only a clause block with no clauses, with more than one clause, or with a guard must cover the type of its parameter (§6.12). A clause block with one clause and no guard is not checked for coverage: when its pattern may fail, it reports to `Assert`.

All records count as one type constructor for coverage.

### 7.5 Known constructors

When a pattern `x = C(…)` matches, `x` is known to be made by `C` where `x` may be used. When a clause pattern `C(…)` of the match `v |> { … }` matches the value of the name `v`, and the match has no clauses, more than one clause, or a guard (§4.8 rule 11), `v` is known to be made by `C` inside that clause. A match with one clause and no guard gives only what the binding of its pattern to `v` gives (§6.12). This is used by §6.7.

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

A function with no parameters, `() -> A ^ E ! N`, is a **computation** that has not been done yet. It is the same idea as an `Effect<A, E, R>` value in Effect-TS. It may be stored, given to other functions, and put together with other computations. Calling it does it. Retrying, time limits, and running at the same time are normal functions:

```i future-SYN2
retry(A, E, N) = (times: Int, body: () -> A ^ E ! N) -> A ^ E ! N {
  fail.catch(body) |> {
    Ok(value) -> value
    Error(error) -> { times > 1 } |> {
      True -> retry(times - 1, body)
      False -> fail.raise(error)
    }
  }
}

user = () { retry(3, () { load_user("1") }) }
```

### 9.2 Environment and services

A **service** is a value of a named type, normally a custom type whose constructor holds functions. A record (§4.9) is not a value of a custom type, so a record type, also through an alias, is never a service type. The **environment** `N` in a function type is the union of the service types that the function needs. Services are taken and given with two forms of syntax:

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
- `control.keep_env(f)` (§9.4) gives a function that, wherever it is called, calls `f` with the services that were bound at the place of the `keep_env` call when it ran. It keeps every service bound there, also those that `f` does not need. An environment statement inside `f` gives its service its own value there as usual.
- Type checking makes certain that every environment parameter gets its service. Getting it never fails at run time.

Await, Root and Owner (§9.11), Scheduler and Runner (§9.9) are normal service values.
Replacing a host service in i uses an environment statement as for any
service. Replacing a random seed with task.top keeps the run route of the
current Runner. Entry defaults do not change call-site service lookup.

A provider can capture a service value. Replacing the service later changes calls that take it from their caller, not the captured value. This is the same rule for shipped and user providers:

```i
Clock { Clock(now: () -> Int) }
Reading { Reading(read: () -> Int) }
make = (clock ! Clock) { Reading(read: () { clock.now() }) }
now = (clock ! Clock) { clock.now() }
example = () {
  ! Clock = Clock(now: () { 1 })
  saved = make()
  ! Clock = Clock(now: () { 2 })
  (saved.read(), now())
}
// example() gives (1, 2).
```

### 9.3 Errors

The `E` part of a function type is the set of errors the function may give. An error may be a value of any type. Errors come together by union (§4.4): when a function calls functions with the errors `A` and `B`, its errors are `A | B`. The module `i/fail` has these functions:

| Function | Type | What it does |
|---|---|---|
| `fail.raise` | `(error: E) -> Never ^ E` | Ends the computation with `error` |
| `fail.catch` | `(body: () -> A ^ E ! N) -> Result(A, E) ! N` | Changes every error into `Error(…)` |
| `fail.from_result` | `(result: Result(A, E)) -> A ^ E` | Gives the error `x` for `Error(x)` |
| `fail.map_error` | `(body: () -> A ^ E ! N, with: (E) -> E2 ! N) -> A ^ E2 ! N` | Changes the errors |

```i future-SYN2
load = (id) { parse_user(http_get(id)) }
// inferred: (id: String) -> User ^ HttpError | ParseError ! Http

safe = (id) {
  fail.catch(() { load(id) }) |> {
    Ok(user) -> user
    Error(_: ParseError) -> default_user
    Error(other) -> fail.raise(other)        // other: HttpError
  }
}
// inferred: (id: String) -> User ^ HttpError ! Http
```

- To handle only some errors, take all of them with `fail.catch`, then use a match with type tests. A later catch-all gets the rest of the union (§4.8 rule 12), which may be given again with `fail.raise`, as in `safe` above.
- An error goes out through the calls until `fail.catch` or `fail.map_error` takes it. The computation that is jumped over is unwound first (§9.5).
- An error that comes to the program entry or a test (§10.2) stops the program. An error that comes to a host export is given to the host.

### 9.4 Control

The module `i/control` has one operation to pause a computation and go on with it later:

| Function | Type |
|---|---|
| `control.boundary` | `(body: (b: Boundary(R, E, N)) -> R ^ E ! N) -> R ^ E ! N` |
| `control.suspend` | `(b: Boundary(R, E, N), f: (k: Continuation(A, R, E, N)) -> R ^ E ! N) -> A` |
| `control.resume` | `(k: Continuation(A, R, E, N), value: A) -> R ^ E ! N` |
| `control.cancel` | `(k: Continuation(A, R, E, N)) -> Nil ! N` |
| `control.keep_env` | `(f: (x: S) -> A ^ E ! N) -> { (x: S) -> A ^ E } ! N` |

`Boundary` and `Continuation` are built-in types. They are not custom types: their values are made only by `control.boundary` and `control.suspend`, and the host cannot take them (§10.3).

1. `boundary(body)` makes a boundary `b` and does `body(b)`. The value of `body` is the value of `boundary`. `b` is **active** while the computation between it and the running code is on the stack: from the start of `body` until `body` ends, but not while that part is paused in a continuation. After `resume`, it is active again.
2. `suspend(b, f)` pauses the computation from this point back to the boundary `b`. That paused part is a **continuation** `k`. Then `f(k)` is done in the place of the boundary, with the environment of that place, and its value is the value of `boundary`. If `b` is not active, the program stops.
3. `resume(k, v)` goes on with the paused computation, with `v` as the value of `suspend`. It goes on on top of the place where `resume` is called, and the value that the computation has when it gets to the boundary is the value of `resume`. The paused part keeps the environment statements that were inside it; for the rest, it gets the environment of the place where `resume` is called.
   The services that a function made by `control.keep_env` keeps count, for the code inside that function, like environment statements inside the paused part: when a paused part that holds such a function's frame is resumed, that code still gets the kept services.
4. `cancel(k)` unwinds the paused computation (§9.5) and gives `Nil`. While it unwinds, the boundary stays not active, and the final functions get the environment as in rule 3, from the place where `cancel` is called.
   The same holds for final functions inside a function made by `control.keep_env`: they get the kept services, and the place of `cancel` gives only the services that are not kept.
5. **One use only**: `resume` and `cancel` together may be used one time for a continuation. A second use stops the program.
6. If a continuation is never resumed or cancelled, its final functions are not run.
7. A boundary and a continuation stay on the worker that made them (§11.9). They are not portable.

A structured task cancel resumes its scheduling continuation with a cancel
answer; i code then cancels an inner continuation (§9.9). It does not change
control.cancel's rule that the boundary of the cancelled continuation is
not active during unwinding.

Generators, async code, and schedulers are libraries made from services and this operation. For example, a generator:

```i
Yield(A) { Yield(emit: (value: A) -> Nil) }

collect(A, N) = (body: () -> Nil ! Yield(A) | N) -> List(A) ! N {
  control.boundary((b) {
    ! Yield(A) = Yield((x) { control.suspend(b, (k) { [x, ..control.resume(k, Nil)] }) })
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
| `resource.ensure` | `(body: () -> A ^ E ! N, with: () -> Nil ! N) -> A ^ E ! N` | Runs `with` after `body`, whether `body` ends normally, ends with an error, or is unwound |
| `resource.acquire` | `(open: () -> R ^ E ! N, release: (R) -> Nil ! N, use: (R) -> A ^ E ! N) -> A ^ E ! N` | After `open` is done, runs `release` when `use` ends or is unwound |

A final function may not give errors. When the program stops (§11.7), final functions are not run.

A structured scope waits for cancelled children and their final functions
before it returns (§9.9). Those final functions may suspend and resume and
use services, but may not raise errors. An unstarted child has no installed
final functions. A host failure or a destroyed instance cannot run i final
functions; the host instead releases its own work and resources (Appendix B).

### 9.6 Assert

`Assert` and `AssertionFailure` are built-in custom types with these definitions:

```i
AssertionFailure { AssertionFailure(message: String) }
Assert { Assert(fail: (failure: AssertionFailure) -> Never) }
```

An `Assert` service is where failures are reported. Its `fail` never gives back a value. The built-in function `assert` is a normal function, which may be written in i:

```i future-SYN2
assert = (c: Bool, a ! Assert) {
  c |> {
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
// type: (body: () -> A ^ E ! N) -> Result(A, AssertionFailure) ^ E ! N - Assert
```

`Assert` is for mistakes in the program. A failure that may be expected uses an error (§9.3). `fail.catch` and `fail.map_error` do not take `Assert` failures.

### 9.7 Host services

A type definition with the attribute `@host("namespace")` is a **host service**:

```i future-SYN2
@host("app:storage/v1", "async")
Storage { Storage(read: (key: String) -> String ^ String) }
```

- It has exactly one constructor. Every field has a label and a function type. The parameters, values, and errors of these functions must be types that the host can take (§10.3), and they may not have an environment. Two host services may not have the same namespace and a field with the same label.
- With `@host("namespace", "async")`, the host does the functions **later**: the program waits at the call, and goes on from the call when the host gives a value or an error. In i code, such a call is a normal call.
- A host service supplied by a host-registration cell in the entry provider plan (§2.6) is given by the host. Every field is the Wasm import with the import module `namespace` and the field label as its name, encoded as in Appendix B. An ordinary chosen factory may supply the same service without that import. Unregistered custom host services retain open host registrations; a registered capability with no default or explicit provider does not fall through to one. WASI keeps required custom imports (§B.7). Generated JS/NodeJS artifacts must supply all required imports or fail the build; arbitrary embedding uses Host. Native builds reject only required imports their runner cannot supply.
- i code may also give a host service with an environment statement, for example to put a fake in its place in a test.
- The library module `i/io` declares and exports the **standard service**: `@host("i:io/v1") Io { Io(print: (text: String) -> Nil) }`. `print` writes the text and a line feed to the standard output. A program that prints imports it, for example `((Io) = i/io)`, and takes it from the environment of an entry point like any host service. `i run` and `i test` provide it. The WASI target chooses an ordinary provider using the checked Wasi adapter (§B.7), and generated JS/NodeJS builds choose their ordinary provider. It gives `Nil`, including after an output error. A Host embedder must explicitly supply it, like any other required standard service. A host service declared in another module with the namespace `i:io/v1` is a normal host service, and the rule above about namespaces and labels is true for it as for any other: with a field `print`, it is a compile error.

**Target services and defaults.**

The ordinary module `i/target` exports this nominal service and its constructors. The modules `i/js` and `i/wasi` export the roots named in it (§10.1, §B.7.1). These are ordinary module types, not built-in names:

```i
(js = i/js, wasi = i/wasi)
(Target(..))
Target {
  Js(js: js.Js)
  NodeJS(js: js.Js)
  Wasi(wasi: wasi.Wasi)
  Native
  Host
}
```

The shipped plan constructs the actual target through §2.6's discriminant map. Native and Host have no root payload. A Js or Wasi payload can be bound by an ordinary environment statement; a tag cannot discharge any other service requirement. A program may construct a replacement Target. Only §10.1.2's certified entry binding is known to contain the build default.

The initial synchronous JS milestone supplies Io, Target and its root. Further defaults apply only when their implementations and tests ship:

| Target | Defaults after IO integration |
|---|---|
| js | Io, Clock, Entropy |
| nodejs | Io, Clock, Entropy, Arguments, Environment, WorkingDirectory, Fs, Net, Process, Stdio |
| wasi | Io, Clock, Entropy, Arguments, Environment, Fs over preopened directories, Stdio; Net, Tls, Process and WorkingDirectory have no default |
| native | The native runner's ordinary standard registrations, split by capability below |
| host | An explicit embedder/provider plan; no arbitrary default services |

The JS loader checks the required feature profile before starting: WasmGC, tail calls, exception handling, console and clocks/timers, and secure entropy at the IO milestone. There is no insecure random fallback. Generated targets omit absent capabilities; they do not install automatic `unsupported` stubs. An explicit Host SDK embedder may choose such stubs. Resource services use the same granted factories in generated Node builds and the SDK; an arbitrary Js value cannot manufacture authority over their private resource numbers.

The process module separates its capabilities as follows, keeping its existing operation arguments, results, ProcessError values and cancellation rules. Each public wrapper requires only the service in its row, plus Await and Owner when it waits. Clock, Fs, Net, Stdio and Io remain separate:

| Service | Namespace | Raw fields |
|---|---|---|
| Entropy | `i:entropy/v1` | entropy |
| Arguments | `i:arguments/v1` | args |
| Environment | `i:environment/v1` | env |
| WorkingDirectory | `i:working-directory/v1` | cwd |
| Process | `i:process/v1` | spawn, wait, terminate, close |

The first four have public constructors for ordinary fakes. Process keeps private raw fields and a private nominal Child wrapper over Int. The same private Int representation is used for other IO resources. Neither these wrappers nor their constructed errors add a boundary type (§10.3). Request ownership and raw signatures are the accepted [runtime.md §§5/7](design/runtime.md) contract, installed by R2/IO; this split adds no new field or error.

`i/net` includes DNS: `connect` accepts a host name or numeric address, and `resolve(name)` returns addresses. The native host uses the system resolver and tries addresses in order, without IPv4/IPv6 racing. TLS belongs to `i/tls`; UDP is not in v1.

### 9.8 Keys

`Key(A)` and `Keyed` are built-in types. They are not custom types: their values are made only by `key.new` and `key.wrap`. A key is the same only as itself (§11.4). Two `Keyed` values are equal when their keys are the same and their values are equal. `Key(A)` does not change (§4.5 rule 7). A `Keyed` holds its value (§11.9).

| Function | Type | What it does |
|---|---|---|
| `new` | `() -> Key(A)` | A new key, the same only as itself |
| `wrap` | `(k: Key(A), value: A) -> Keyed` | The value, marked with the key |
| `unwrap` | `(k: Key(A), x: Keyed) -> Lookup(A)` | `Found(v)` when `x` was made with `k` from `v`, and `Missing` when not |

`Lookup(A) { Found(A) Missing }` is a custom type of `i/key`, not a built-in name.

### 9.9 Tasks



i/task is a library implemented using i/control, i/key and i/request. A run
owns its tasks and scheduler state. Its implicit root permits local values.
Scopes constrain the lifetime of spawned work: none outlives its scope.
Values pass between tasks only through go captures/kept services, channels,
child results/errors and shares. Portable paths check core.portable even
with one worker: sends at the call, send events at construction, children
at completion, scope body errors at failure, shares at creation. Failure
stops with panic("task: value is not portable"). go never rejects for
non-portability; on multiple workers it silently classifies the complete task
record at go and publishes portable records. Other records stay local.
Local scopes/channels hold the maker's boundary only as a non-portability
witness. A Task/join event from a local scope holds that scope too. Every
stateful handle checks run identity; events have fresh keys and sync checks
only their stateful leaves, including losing alternatives. Channels belong
to the first scope in their maker's chain, or the run; channels and shares are freed at scope exit.
Every run has the result of some serial order of task steps. Timing may
choose a sender, a race/select winner, which failures precede cancellation,
or whether a handle is used before its owner ends. These order-dependent
results may change between runs or worker counts. Tests must either avoid
that dependence or list the allowed outcomes. No task API observes workers.

Export types Scheduler, Runner, Scope(E), Task(A,E), Channel(A), Event(A,E,N),
Shared(A), Ref(A), and ScheduleError; export only ScheduleError's constructors. The
other constructors and scheduler protocol are private ordinary custom types,
not host opaque types. Scope, Task, Channel, Shared, and Ref contain invariant keys;
their parameters are therefore invariant. Event's answer/error/environment
parameters are covariant: its private representation keeps typed decoding
functions, rather than Key(A) in an output field. A public Event can hold user
mapping functions and the local handles of its leaves. Such contents affect
portability by the normal walk. The library creates no event that captures its
Scheduler or mutable loop state. A join event keeps its task's scope handle.

Schemes below declare their capital variables on their bindings. N is the body
service requirement; supplying Scheduler/Runner/Await/Owner/Root discharges those members.
Assert and user host services remain in N where the body needs them.

| Binding | Type |
|---|---|
| `top` | `(seed: random.Generator) -> Runner ! Runner` |
| `run(A,E,N)` | `(body: () -> A ^ E ! N) -> A ^ E ! Runner \| N - Scheduler - Runner - request.Await - request.Owner - request.Root` |
| `test(A,E,N)` | `(seed: random.Generator, choices: List(List(Int)), body: () -> A ^ E ! N) -> A ^ E \| ScheduleError ! Runner \| N - Scheduler - Runner - request.Await - request.Owner - request.Root` |
| `scope(A,E,N)` | `(body: (s: Scope(E)) -> A ^ E ! N) -> A ^ E ! N \| Scheduler` |
| `local_scope(A,E,N)` | Same scheme as scope |
| `go(A,E,N)` | `(s: Scope(E), body: () -> A ^ E ! N) -> Task(A,E) ! Scheduler \| N - Scheduler - Runner - request.Await` |
| `join(A,E)` | `(t: Task(A,E)) -> A ^ E ! Scheduler` |
| `yield` | `() -> Nil ! Scheduler` |
| `path` | `() -> List(Int) ! Scheduler` |
| `random` | `() -> random.Generator ! Scheduler` |
| `keep(S,A,E,N)` | `(f: (S) -> A ^ E ! N) -> { (S) -> A ^ E ! Scheduler } ! Scheduler \| N - Scheduler - Runner - request.Await` |
| `channel(A)` | `() -> Channel(A) ! Scheduler` |
| `local_channel(A)` | `() -> Channel(A) ! Scheduler` |
| `send(A)` | `(c: Channel(A), value: A) -> Nil ! Scheduler` |
| `receive(A)` | `(c: Channel(A)) -> A ! Scheduler` |
| `send_event(A)` | `(c: Channel(A), value: A) -> Event(Nil,Never,Never)` |
| `receive_event(A)` | `(c: Channel(A)) -> Event(A,Never,Never)` |
| `join_event(A,E)` | `(t: Task(A,E)) -> Event(A,E,Never)` |
| `always(A)` | `(value: A) -> Event(A,Never,Never)` |
| `never` | `() -> Event(Never,Never,Never)` |
| `choose(A,E,N)` | `(events: List(Event(A,E,N))) -> Event(A,E,N)` |
| `map(A,B,E,F,N,M)` | `(e: Event(A,E,N), f: (A) -> B ^ F ! M) -> Event(B,E \| F,N \| M)` |
| `sync(A,E,N)` | `(e: Event(A,E,N)) -> A ^ E ! N \| Scheduler` |
| `share(A,E)` | `(s: Scope(E), value: A) -> Shared(A) ! Scheduler` |
| `get(A)` | `(s: Shared(A)) -> A ! Scheduler` |
| `ref(A)` | `(value: A) -> Ref(A) ! Scheduler` |
| `update(A,B)` | `(ref: Ref(A), change: (A) -> (A, B)) -> B ! Scheduler` |
| `both(A,B,E,N)` | `(left: () -> A ^ E ! N, right: () -> B ^ E ! N) -> (A,B) ^ E ! Scheduler \| N - Scheduler - Runner - request.Await` |
| `race(A,E,N)` | `(left: () -> A ^ E ! N, right: () -> A ^ E ! N) -> A ^ E ! Scheduler \| N - Scheduler - Runner - request.Await` |
| `map_concurrently(A,B,E,N)` | `(xs: List(A), f: (A) -> B ^ E ! N) -> List(B) ^ E ! Scheduler \| N - Scheduler - Runner - request.Await` |


In tables, `\|` is only Markdown escaping; the i operator is `|`. The schemes
for combinators use one common E and N; subtyping admits either child's error
and needs. No union of invariant Task(Int,E) and Task(String,E) is required:
`both` keeps two separate handles. No local variants of these three combinators
are exported in v1. Local callers write a local_scope explicitly.

The differences in the go/keep/combinator environments matter: their wrappers
supply Scheduler/Runner/Await themselves, so requiring all of N at their call
sites would overstate their principal needs. Owner is kept, and is not subtracted
there. Root is kept there too. run/test supply a fresh Root and default Owner,
so they subtract both. test still needs
the calling Runner for its nested path/wait route. Attribute expansion for host
fields happens before ordinary constraint generation; it adds fixed ordinary
service types, not a type-dependent constraint. Keys are invariant and obey the
existing value restriction; Request and Event add no qualified types or higher
rank quantification. Thus the design reduces inference to the existing rules,
with the new primitive variance rows. The prototype gate checks that argument
against the implementation rather than assuming it from the notation.

No public spawn detaches a task. `go` never checks its closure's portability.
It increments the calling task's spawn/run counter, beginning at 1; the child
path is the parent's path with that index appended. A nested run takes an index
from that same counter. `path` and `random` are data reads without a suspend.
A top seed that was already split or drawn keeps that entire derivation state.

A scope waits for all children even when its body returns without joining them.
Returning a Task handle does not extend the right to join it. Handles check run
identity before other checks, then chain membership or liveness. `share` checks
membership as `go` does (same outside-scope stop), including local scopes; values
shared through either scope kind must be portable. get checks run then share
liveness. Handle equality means the same object of the same run, never equal
numeric ids in two runs. Text uses constructors without hidden ids: a private
identity-bearing key is printed as `//key`, and implementation ids/paths
are kept under Keyed fields whose text is //keyed, not exposed as plain fields. `path()` alone exposes the semantic task path.

#### Scopes, errors, and cancellation

R1. A task's chain is its active scope bodies, inner first, followed by its
starting scope's chain. A scope's chain is itself plus its opener's chain at
opening. `go` and join require the relevant scope in this chain. Run-key mismatch
stops with `task: handle of another run`; failed membership stops with
`task: go outside its scope` or `task: join outside its scope`. These messages
are passed to panic, so the stop prefix is `panic: `.

R2. A scope body executes in the opener task under an inner scope boundary;
it does not start another scheduler. A child executes from the run loop under
its own task boundary. On body/child failure, mark the scope cancelling, cancel
its body and every child, and propagate down their active scope trees. Wait for
all unwind operations, including scopes created by finals. Choose the least
path among actual failures; a prefix sorts before its extensions and the scope
body has the opener's path. A cancellation is not a failure. Types accumulate
ordinary error unions; stops cannot be caught. A go accepted while cancelling
creates an unstarted, immediately dropped child and its ordinary Task(A,E)
handle. A shielded final receives that handle; its body never runs. If that
scope's cancellation can unwind the caller, the command delivers cancellation
instead of returning the handle. This priority also applies at a valid join.
A cancelling scope never starts new user work just to manufacture a join value.

R3. On a normal scope exit, join every live child before returning its body
value. Before either successful or failing return, free the scope's channels,
shares, completed-task table entries, and registration records. The owning
Scope/Task handle can survive but does not retain its result/state after exit.
Reads needing dead state stop by the defined checks. Channel ownership is the
first scope in the maker's chain, or the run. A channel wait still present when
that owner ends stops with `task: channel is gone`, including a waiter in an
outer scope; it is not silently dropped. share get after exit stops with
`task: shared value is gone`. Scope ends are scheduling/commit steps.

R4. A paused or runnable started task receives Cancel at the next switch after
cancellation reaches it. A running task sees it at its next scheduler suspend.
An unstarted task is dropped without calling its body. The Scheduler command
which receives Cancel suspends to the innermost cancellation delimiter being
cancelled; that handler calls control.cancel only on that inner continuation.
The outer task boundary stays active. Finals use the task's saved services;
all awaits and sends they perform go to that still-active task boundary. An
unwind flag excludes that task from a second cancel, including cancellation
of scopes it creates while unwinding. Such new scopes still obey their own
internal failure/normal-exit rules. The outer cancel waits for them to end.
A final that waits forever can keep cancellation waiting forever.

R4a. Keep outcome and cleanup completion separate. A task outcome is pending,
success, failure or cancelled; drop sets cancelled immediately, and a started
task sets cancelled when its home scheduler accepts cancellation, before running
its unwind. A mere cancellation request not yet accepted is not a cancelled
outcome: that task can still finish first. Normal success/failure is recorded
after its final functions end. Scope exit still waits for cleanup completion,
independently of the outcome. Keep dropped/cancelled records until scope exit.
A cancelled outcome retains its causing scope and cancellation epoch, so a join
can apply that cause even before its caller receives a cancel message. Do not
use only a worker-local “cancel delivered” flag to decide whether to stop.
join's type remains `(Task(A,E)) -> A ^ E ! Scheduler`. After the run/chain checks, process any applicable scope cancellation
before observing a target's cancellation outcome. A joiner still cancellable by
that cancellation is unwound; it never races between this path and a join panic.
This includes a final executing a fresh scope body that the fresh scope can
cancel: unwind that inner body, not the already-shielded outer final. A cleanup
scope and its children are exempt from the older cancellation as R4 requires.

A joiner shielded from the target scope's cancellation cannot obtain A or E
from a dropped/cancelled child. It stops immediately with
`panic("task: join of a cancelled task")` when that outcome is known; it does
not wait for the cancelled target's final functions. If it joined while the
outcome was still pending, accepting cancellation wakes that join immediately.
This also avoids cycles between shielded finals joining one another. The stop
ends the run and, as with any stop, may abandon remaining i finals. In a valid
structured exit without that stop, scopes still wait for all cleanup to finish.
An already recorded success or failure gives ordinary A or E. No Cancelled
error value, aggregate error, or changed join signature is introduced. A join
can wait for genuinely live work, but never for an already cancelled outcome.

A join_event has the same rule when selected/committed. A known cancelled
outcome makes that alternative ready, even if cleanup is not complete; it is not a prevalidation fault of a losing
alternative. A cancellable syncing task processes cancellation first. A shielded
syncing task stops only if that join alternative wins; losing map functions
still do not execute. This preserves CML selection and prevents an invalid
stop from racing cancellation of an ordinary joiner.

R5. A race commits the first successful child completion observed by its
scope. Commit atomically records the winner and initiates the ordinary scope
cancel of the loser and body wait. Return only after their finals end. Failure
before a success commits invokes R2; a failure already observed wins over a
later success. Once success has committed, cancellation suppresses later
unobserved errors. Thus race is first completion unless that completion was a
failure; errors use the same scope rules as other combinators. It is not “first
successful result despite failures”. Both/map_concurrently give argument/input
order and R2's errors. All three use portable scopes and check child results
and errors before recording a completion, including race losers that finish
before cancellation reaches them.

R6. The runtime entry wrapper and every explicit run own a local root task,
which they run to completion with structured children. An entry error is
reported only after the run has unwound; a host export receives its ordinary
error. A stop immediately traps and runs no finals, as §9.5 requires. Host
trap/failure/drop also invalidates the instance under ABI H8–H11; it cannot
promise to resume i cleanup. “Cancellation runs finals” refers to i structured
cancellation, not destruction of a trapped instance by its host.

i values and bindings are immutable; `task.ref` is the only state construct, owned by its scope, changed only by `task.update` with a pure function.

A `Ref(A)` is an immutable handle to scheduler-owned state. `ref(value)`
creates one in the first scope of the maker's chain, or in the run at its root.
`update(ref, change)` atomically replaces its value with the first part of
`change(old)` and returns the second part. The callback is pure, with no typed
errors or services. Stored values must be portable. Check the run identity and
ref liveness before calling the callback. Owner exit frees its state; later
updates stop with `task: shared value is gone`. Refs use the same scheduler
arbitration as channels, and do not introduce host state or mutable i values.

#### Events, FIFO behavior, and fairness

Channels are unbuffered rendezvous channels in v1. No close-channel operation is
needed: owner-scope exit frees their state. An event is a reusable immutable
description. choose([]) behaves as never(); sync(always(x)) gives x. New event constructions
have fresh identity keys, independent of A/E/N, so equality means the same
event handle rather than a comparison of event meanings. map/choose
perform no Scheduler operation. send_event checks portable payloads immediately;
receive_event and join_event do not test liveness until sync. A send_event can
also be built without Scheduler; its handle records whether it is local.

One sync creates one registration identity with ordered alternatives. Validate
run identities, join chains and channel liveness of all leaves before registering
any. An invalid losing alternative is still an invalid sync. Register without
running map callbacks. Among ready alternatives use source order; among partner
registrations use FIFO arrival order. Distinct leaves from the same registration
must never rendezvous with one another. A registration may commit once. Commit
atomically marks both partner registrations, removes all their losing leaves,
transfers one value, then queues the tasks. Apply only the winning branch's map
functions, in composition order, in the syncing task after commit. A mapper can
raise, await, spawn, or stop; it cannot roll back the message. No guard or nack
operation is exported. Cancelling a sync unregisters all leaves before unwinding.

go appends the child to the FIFO queue and returns to the parent in the same
turn, after cancellation checks and a host poll only if a task needs host
progress; it does not yield the parent. yield ends the current turn and queues the caller at the tail. A blocking command removes
its task until ready; a nonblocking sync/join still makes a scheduling point.
A completion queues waiters FIFO. Cancellation bookkeeping runs before another
normal task turn; a task already unwinding is queued normally. Scope ends,
async awaits, and each loop switch are poll points too. path/random are not.

At every switch, first run cancellation bookkeeping and scan terminal ticket
waiters. Build the progress-interest set from tasks actually suspended in
Await.wait on Waiting tickets, including interests forwarded by nested runs.
Root-level Runner.poll calls request.poll once only when this set is nonempty.
Otherwise skip it, including before the first task turn. Pure go/yield/join/
scope operations and merely starting an unawaited raw request do not by
themselves return status 2. A task/run which never blocks, and makes no explicit
root wait/poll call, therefore never yields to the host. A pure Scheduler-only
entry can return async status 0 immediately. A runnable pure task can of course
share its run with a sibling needing host progress: that sibling's wait causes
the run's hand-off. This is service of an existing wait, not an implicit yield
added to the pure task.

While there is progress interest and a task is runnable, poll once before its
next turn, using status 2. Scan settled tickets, then run a task even if further
host answers might be ready. This keeps an infinite yielding task from starving
an IO-waiting sibling, without making pure task switches pay an event-loop yield.
Unawaited raw work alone does not force progress hand-offs during a runnable
loop; its caller must eventually wait/poll explicitly or let the run become
idle. Idle accounting still counts it as specified next.

A nested loop's private Runner.poll passes its current interest tickets to the
outer Scheduler with its nested-run id, then yields only its calling outer task.
The outer loop keeps that interest registration while the nested run lives and
forwards it in turn if nested again. Thus an inner IO waiter alongside an inner
CPU-yielding task can receive host answers even when the outer caller is runnable.
Terminal tickets are filtered; each poll replaces that run's prior interests,
and an error-free final removes the registration on every nested-run exit.
Cancellation removes a nested registration before completing the calling task's
unwind; borrowed tickets themselves are not cancelled by unregistering interest.
No new ABI operation is needed: these are private ordinary Scheduler commands.
The root Runner's poll takes the aggregated tickets; it never scans all instance
requests to invent a wait. Explicit low-level request.poll still intentionally
hands off, and stops request: no active entry from a synchronous entry.

When no task is runnable, scan terminal tickets first. The logical waitable set
is the union of outstanding requests in every group of the run's stored program
Root and Waiting tickets its tasks explicitly await, including borrowed tickets
from another root on the same worker. Group-observer tickets contribute their
underlying outstanding members, not the observer cell itself (§9.11). Deduplicate
identities, including an owned request also explicitly awaited. This is a set of work, not a List(Ticket) that
can be exported across workers: observe owned groups through their counts and
revision notifications (§9.11); borrowed Ticket values stay local. If this union
is nonempty, Runner.wait waits for a member to settle and then rescans. An
unawaited raw request in any custom group prevents deadlock until it settles.
Its detached Ready answer is retained only by its handle. If the set is empty
and no task or completion delivery can run, stop with panic("task: deadlock"),
even if an unrelated root has work. A nested run therefore includes an explicitly
borrowed outer request, but not unrelated outer work.

A dispatcher may own several ordinary Roots with separate Owners. It chooses
which root's work it drives; no Root or request has a magic module-based class.
The scheduler driver keeps a separate control Root/Owner for its wake requests,
scoped to the run's driver lifetime and closed on every driver exit. It does not
bind that Owner for user tasks or expose its wake tickets. The stored program
Root plus explicit borrowed waits above alone justify program progress; the
control root does not. This same separation serves a batch dispatcher, a timeout
manager and a plugin notification loop. Both kinds still use the same Root/Owner
registration, count, cancel and teardown rules. The distinction is which roots
the ordinary driver selects, not a host import name or a compiler exception.

Quiescence snapshots count runnable/claimed-but-unstarted/running tasks and
pending completion-to-task or group-notification processing as work. Requests
stay counted until their answer/cancel transition publishes that work atomically;
there is no gap where both counts are zero. With no such work, zero members of
the program waitable set means deadlock, whatever the control count. MC takes
that snapshot and records the stop under the shared scheduler lock; a wake-only
control delivery does not count as program work. At the ABI level every actual
outstanding request, including a wake, still satisfies status 1's requirement.
Never return status 1 with zero actual outstanding requests or busy-poll with no
runnable task. A one-worker run needs no wake request. CPU code without suspension
can monopolize M:1; a go-only loop may postpone children. No bounded wall-time
guarantee is made for final functions.

#### Runner, nested runs, and testing

The runtime supplies an initial portable Runner: prefix, full seed generator,
policy, the enclosing request-context Root, and constant functions implementing
root wait/poll. run uses that Root while opening its fresh run Root, so its
public environment still needs Runner rather than an extra Root argument. `top(seed)` replaces
only the seed in the current Runner and preserves its path allocator and wait
route. This is why its scheme needs Runner: rebinding a seed inside a task must
not reset its nested-run prefix or make its nested wait bypass the outer loop.
Entry provisioning uses the private ordinary root-runner constructor, not top.
The ledger supplies this initial service, so a fixed-seed test can write
`! task.Runner = task.top(random.from_seed(7))` without a bootstrap problem. In each task its Runner
routes wait/poll to the outer Scheduler and takes paths from that task's shared
spawn/run counter. That per-task Runner is non-portable; go's detached substitution
above prevents accidental retention in children. Explicit run inside the entry's
default root task is nested. Each run opens a fresh request.Root and its default Owner around its entire
loop, in a retained frame. Tasks keep Root and their go-site Owner. Custom
within groups remain registered to that root. The run closes the entire root
after all tasks/finals end, not just its default group.
Only the runtime-created entry run
is eligible for future multicore execution; v1 has one worker throughout.

A nested loop owns fresh run identity and FIFO state on the caller task's stack.
Nested children do not execute an outer task's channel commands: all handle
uses check their run key. Its wait hands tickets to the outer task's Scheduler;
its poll yields that calling task without directly yielding to the host. Only that task is paused; other outer tasks
continue. When nested work ends it cancels remaining requests/children before
return. A nested deadlock is detected from that nested run's own tasks and
waitable requests. It does not return status 1 from a task stack. Chains do not
cross run identities.

`task.test(seed, choices, body)` supplies a Runner for a fresh nested run. Before
a turn, take the next script path, if any; it must be runnable, otherwise raise
`ScheduleError { NotRunnable(path: List(Int)) }` after structured cleanup. When
the script ends choose FIFO. Do not root-poll or use real clocks. Completed fake
requests are allowed; reaching a required wait on an unsettled host request stops
with `panic("task: test needs fake IO")`. Fixed inputs and script give the same
values, task paths, selected alternatives and trace on every run. Tests for real
host overlap use ABI completion gates instead. The testing policy changes only
which legal runnable step is chosen; it cannot commit an unready event.

### 9.10 Random generators

The module `i/random` gives pure, repeatable random computations. It needs no
service and does not import `i/task`. It exports the type `Generator` with its
constructor hidden, the error type below with its constructors, and these
functions. The signatures describe bindings with bodies.

```i
RandomError { InvalidBound(Int) InvalidPath(List(Int)) }
```

| Name | Type |
|---|---|
| `from_seed` | `(seed: Int) -> Generator` |
| `word` | `(g: Generator) -> (Int, Generator)` |
| `int` | `(g: Generator, below: Int) -> (Int, Generator) ^ RandomError` |
| `float` | `(g: Generator) -> (Float, Generator)` |
| `split` | `(g: Generator) -> (Generator, Generator)` |
| `at_path` | `(g: Generator, path: List(Int)) -> Generator ^ RandomError` |

A Generator contains a normalized seed, SplitMix state, tagged derivation
history, and a nonnegative draw index. Equality is ordinary structural equality.
Reusing the same immutable input repeats the same result; callers pass the next
generator to get more values. This is not a cryptographic generator. Different
streams need not have disjoint outputs, and their finite states can collide.

The algorithm is SplitMix, by Steele, Lea and Flood
([OOPSLA 2014](https://gee.cs.oswego.edu/dl/papers/oopsla14.pdf)), using the
`mkSMGen`, `nextWord64`, and `splitSMGen` variant of
[splitmix 0.1.3.1](https://hackage-content.haskell.org/package/splitmix-0.1.3.1/docs/src/System.Random.SplitMix.html).
Its exact operations are as follows. Let W = 2^64. Shifts are unsigned, and
additions and multiplications reduce modulo W. M(x) applies xor-shift 33,
multiply by 0xff51afd7ed558ccd, xor-shift 33, multiply by 0xc4ceb9fe1a85ec53,
then xor-shift 33. V(x) applies xor-shift 30, multiply by 0xbf58476d1ce4e5b9,
xor-shift 27, multiply by 0x94d049bb133111eb, then xor-shift 31. G(x) sets
the low bit of V(x) to give z; if popcount(z xor (z >> 1)) < 24, it gives
z xor 0xaaaaaaaaaaaaaaaa, otherwise z.

`from_seed` reduces its arbitrary Int seed by Euclidean remainder modulo W to
n in 0..W-1. Seeds differing by W give the same generator. Its initial state is
(M(n), G(n + 0x9e3779b97f4a7c15)), with empty history and draw index zero.
A state (s, gamma) draws M(s + gamma), then becomes (s + gamma, gamma).
`word` gives this word in 0..W-1 and increments the draw index. Native splitting
gives continuation (s + 2*gamma, gamma) and child
(M(s + gamma), G(s + 2*gamma)).

The public derivations navigate this native split tree. Define fork(h) by
native-splitting h into (rest, a), then native-splitting rest into (_, b), and
returning the two children (a, b). Each bit 0 or 1 selects that fork child,
using the entire current state. Give the split step S(1) code 00 and S(2) code
01. For a positive index i, give the path step P(i) code 1 followed by Elias
gamma(i): put k-1 zeros before the k-bit binary representation of i. Thus
P(1) = 11, P(2) = 1010, and P(3) = 1011. These codes are prefix-free.

`split` returns the generators reached by S(1) and S(2). `at_path` follows
P(i) for each successive list element. Each derivation appends the tagged step
and the previous draw index to the history and resets its draw index to zero.
Derivation after a draw therefore differs from derivation of its input.
Consecutive path components behave the same whether given in one `at_path`
call or several. `at_path(g, [])` is g. Path and split steps are distinct,
including for arbitrarily large Int indices; scheduling never selects bits.

`int(g, below)` requires below > 0, including arbitrary-size bounds. Otherwise
it raises `InvalidBound(below)` without a draw. Use the least k with W^k >= below,
at least one. Draw k words in least-significant-first order to form u. Let
limit = W^k - (W^k % below). Reject u >= limit and draw another group. Give
u % below and the generator after every accepted and rejected draw. This
avoids modulo bias under the uniform word model. `float` draws one word u and
gives floor(u / 2^11) / 2^53 as a Float, with that word's next generator. Its
result is in [0, 1); at most 53 bits enter the conversion.

`at_path(g, path)` requires every index to be strictly positive. Otherwise it
raises `InvalidPath(path)` without a draw or derivation, carrying the whole
supplied path.

### 9.11 Requests

Request(A) and Ticket are compiler-defined names available in every module,
like Boundary and Continuation. They have no public constructors. i/request
exports the following ordinary status/service types, their constructors, and
functions; it does not redeclare or re-export the primitive types.

```i
Await { Await(wait: (ticket: Ticket) -> Nil) }
Root { Root(key: Key(Nil)) }
Owner { Owner(key: Key(Nil), root: Root) }
RequestState(A) { Waiting Ready(A) Cancelled Taken }
```

| Function | Type | Rule |
|---|---|---|
| `open_root` | `() -> Root ! Root` | Open an independent request run within the current entry context |
| `close_root` | `(root: Root) -> Nil` | Close every live group of this root and the root itself; idempotent |
| `open_owner` | `() -> Owner ! Root` | Register a group at the current root; changing Owner does not change that root |
| `close_owner` | `(owner: Owner) -> Nil` | Cancel its outstanding work and remove the live group; idempotent |
| `count` | `(owner: Owner) -> Int` | Number of outstanding requests in this group |
| `count_root` | `(root: Root) -> Int` | Number of outstanding requests in all groups of this root |
| `settled_owner` | `(owner: Owner) -> Bool` | True exactly when count(owner) is zero |
| `settled_root` | `(root: Root) -> Bool` | True exactly when count_root(root) is zero |
| `cancel_all` | `(owner: Owner) -> Nil` | Cancel the group's outstanding work at this step; leave the group open |
| `cancel_all_root` | `(root: Root) -> Nil` | Cancel outstanding work in all its groups at this step; leave them open |
| `wait_none` | `(owner: Owner) -> Nil ! Await` | Await until this group's count is zero |
| `wait_none_root` | `(root: Root) -> Nil ! Await` | Await until this root's count is zero |
| `within(A,E,N)` | `(body: () -> A ^ E ! N) -> A ^ E ! Root \| N - Owner` | Fresh Owner environment; ensure close_owner on exit |
| `ready(A)` | `(value: A) -> Request(A)` | A fresh completed request; available in pure fakes |
| `ticket(A)` | `(r: Request(A)) -> Ticket` | The same underlying request identity; no answer access |
| `state(A)` | `(r: Request(A)) -> RequestState(A)` | Reads do not consume the answer |
| `take(A)` | `(r: Request(A)) -> A` | Ready becomes Taken atomically; other states stop |
| `settled` | `(r: Ticket) -> Bool` | True for Ready, Cancelled or Taken |
| `cancel` | `(r: Ticket) -> Nil` | Idempotent; cancels only Waiting, discards a Ready answer |
| `await(A)` | `(r: Request(A)) -> A ! Await` | Borrow the producer, wait, then take the Ready value |
| `await_owned(A)` | `(r: Request(A)) -> A ! Await` | Own a non-resource-producing operation; ensure ticket cancellation on exit |
| `wait_any` | `(rs: List(Ticket)) -> Nil` | Root wait; return when any listed ticket settles |
| `poll` | `() -> Nil` | Root hand-off with status 2, even with no requests |

`cancel` makes every alias observe Cancelled, even for a previously Ready cell.
A Taken request stays Taken on cancel. It invokes ABI cancel only for a still-outstanding host id. Cancelling a Ready
request never invokes that import. `await` or take on Cancelled stops with
`panic("request: cancelled")`; on Taken it stops with `panic("request: taken")`;
take on Waiting stops with `panic("request: not ready")`; if Await.wait returns with Waiting it stops with
`panic("request: wait returned early")`. `wait_any([])` stops with
`panic("request: empty wait")`. A nonempty list may contain aliases; deduplicate
by identity. Terminal tickets make it return immediately. It must keep waiting
through answers of unlisted requests, which are stored for later use. All-listed
cancelled tickets count as settled. No root continuation is stored on a cell.

Cancellation ownership and run affiliation are separate. Root identifies a
request run and its enclosing entry context. The runtime creates the initial
Root only for an entry whose inferred needs require request/task support.
open_root needs an existing live Root to retain that entry context; it does
not require the new root to share the old root's request count. A dispatcher
can thus open an independent root without inventing a second entry. The
initial bootstrap is part of the narrow entry provision, not a public unscoped
root allocator. Entry request contexts and each task.run open a fresh Root and
bind it for their whole lifetime. open_owner uses that Root, not the currently
bound Owner, to register its group. request.within opens/binds a fresh Owner and
ensures close_owner on exit; it leaves Root unchanged. Thus custom groups of a
run have the same root, and a nested task.run has a different root. Both Root
and Owner are ordinary portable key-bearing service values, not host requests.

Every raw host start records its Owner group and that group's immutable root.
close_owner cancels only its own group. close_root closes all groups registered
at that root; no subgroup or old Owner binding escapes it. All group operations
include every worker, including groups made inside request.within and retained
by go. Counts and readiness are atomic snapshots; a later start can change them.
A closed group/root has count zero and is settled. cancel_all cancels the Waiting
members at its linearization step, not subsequent starts or detached Ready values;
close additionally refuses future starts. Opening a group under a closed Root
stops with request: root is gone; a host start under a closed Owner stops with
request: owner is gone, before making the ABI import. Only live registrations
and outstanding cells are retained; absence supplies the gone check.

There is no public outstanding/outstanding_root enumeration, including no
worker-local variant. A Root/Owner operation never returns a Request or Ticket.
Group cancellation is accepted atomically and forbids later success publication
for its selected members. Before returning it applies the host cancellation
actions, including lease revocation. Every state, settled, take, cancel, wait
and delivery operation observes that terminal state. A host id is retired once.

wait_none/wait_none_root use Await, so a task waits without pausing runnable
siblings. They return immediately on zero, including for a closed group.
Otherwise a private local Request(Nil) observation cell subscribes to the group
and settles on its next zero observation. It has no host id, producer operation
or Owner membership: a wait is not work that can complete itself. Its Ticket
never leaves the observer's worker. Check-and-subscribe shares the registration
lock so completion cannot be lost between checking and sleeping. A concurrent
start before the zero observation is included; one after it does not undo the
return. An ordinary ensure unregisters/cancels this local observer on waiter
exit without cancelling the group's producers. These private cells use the
same typed state/await machinery as requests; no foreign Ticket is manufactured.
For progress accounting, an observation cell depends on the actual outstanding
members of its target group, not on itself; add those members as explicitly
borrowed work and deduplicate identities (§9.9). In particular it cannot keep
an otherwise deadlocked empty group alive. Remote members are driven on their
home workers; group changes reach the local observer through its wake route.
The private Runner idle operation subscribes to group revisions for any change,
combined with explicitly borrowed local tickets (§9.9), without enumerating
foreign tickets. Its observation/check/subscription must be atomic too.

Root is kept at go sites as a normal service. Changing Owner alone cannot move
a group/request to another run. open_owner under a nested run's Root makes a
nested group even if user code temporarily rebinds an outer Owner. Explicitly
using a previously made outer Owner for a raw start still charges that outer
group/root; this is a capability use, not a transfer of the group. Borrowed
request tickets may be awaited by another run on the same worker. Opening or
rebinding an independent Root is the public low-level operation for writing a
separate request dispatcher, like calling root wait directly; it does not
silently reaffiliate an existing i/task run's stored root or its old groups.
The default task scheduler counts the root it owns, not whichever Owner is
bound at an idle point. An independent request dispatcher must drive its root.

A Ready cell has left its group and belongs to its handle. Closing a group/root
does not discard such an answer; raw callers must own it just as they own files
returned by open. Completed cells retain decoded i values only while handles
are reachable, not through the outstanding table. Low-level cancellation cannot
roll back a host side effect or run i final functions. Closing a root after its
structured tasks/finals end cancels any still-outstanding unowned raw work.

Root wait/poll require an active async entry context when they must hand off;
otherwise they stop with `panic("request: no active entry")`. A later synchronous export must not hand off through a stale boundary from a completed async call. An immediate wait
on completed fake requests works without that context. They are public primitives
for writing dispatchers, batch IO, or request races. Direct use inside a task
pauses that entire run until it returns; normal await uses Await instead. This
is explicit control use, like suspending to an outer boundary, and does not
create multiple scheduler histories.

Root and Owner are ordinary services, not primitive types or built-in names.
They use Key(Nil) and are portable; outstanding request values remain non-portable.
Root/group registration and observation use internal accounting. Local Ticket
cancel uses v3 cancel on its home worker; MC group cancellation uses the
registered host cancellation actions above and retires the home id once. MC
routes notifications through ordinary request-mode wake services, without new
runtime imports. Generic
request groups also serve batch cancellation, request timeouts, and host plugin
cleanup, independently of tasks. Request operations introduce no new error-row
or environment polymorphism. Ticket
cannot recover A; state only reads through the original Request(A). The
value restriction prevents a fresh request from being generalized at multiple
unrelated answer types. A cell's state transitions are Waiting -> Ready or
Cancelled, and Ready -> Taken or Cancelled; there is no write of another A.

In a task, Await.wait submits a WaitRequest(ticket) command to its Scheduler,
with the task's inner-unwind target registered privately. The loop records the
waiting task and ticket and queues another task. On each switch it scans settled
tickets; a ticket may have several waiters. Await consumes the answer once; multiple
waiters deliberately competing for it may therefore meet request: taken.
The plain async adapter owns its just-started request through resource.ensure,
and cancels it on exit, including while unwinding. A raw starter gives ownership
to the caller, who can use the same pattern. Cancelling one waiter of a manually
shared raw request does not cancel the producer unless its owner's final function
does so; cancelling a ticket wakes all waiters, whose await then stops as above.

Because request.cancel also discards Ready answers, an implicit adapter must
take its A before its final cancels the cell. For resource-creating imports,
IO wrappers additionally transfer resource ownership as section 8 specifies;
discarding an unclaimed Ready answer releases its resource. These are two
separate ownership transfers, not a promise that all raw results are resources.

The ordinary helper for an owned operation that produces no resource is:

```i
await_owned(A) = (r: Request(A)) -> A ! Await {
  resource.ensure(
    () { await(r) }
    () { cancel(ticket(r)) }
  )
}
```

This definition lives in i/request and uses its local function names. The final
runs on success, error and task cancellation; after take, cancel is a no-op.
request.await itself remains a borrowed wait and never takes producer ownership.
File/Stream/Child wrappers and other non-creating IO wrappers use await_owned,
including their close operations; resource creators use resource guards (§9.7).
Cancelling a pending Stream read revokes its read lease before the cancelled
wrapper finishes unwinding, so a following read cannot see busy from that
abandoned request. Already consumed bytes or committed effects are not rolled
back. A cancelled Child wait releases its wait lease without closing the Child;
a cancelled blocking File operation instead closes that File as §9.7 requires.


### 9.12 IO library operations

These ordinary functions use the services described in §9.7; their definitions and exported types are in their library modules.

#### `i/clock`

| Function | What it does |
|---|---|
| `monotonic_ns` | Read the monotonic clock in nanoseconds. |
| `unix_ns` | Read Unix time in nanoseconds. |
| `sleep_ns` | Wait for a duration in nanoseconds. |

#### `i/stdio`

| Function | What it does |
|---|---|
| `read` | Read bytes from standard input. |
| `write` | Write bytes to standard output. |
| `write_error` | Write bytes to standard error. |
| `flush` | Flush standard output. |
| `flush_error` | Flush standard error. |
| `print` | Write text and a newline to standard output. |
| `print_error` | Write text and a newline to standard error. |

#### `i/fs`

| Function | What it does |
|---|---|
| `open` | Open a file in the given mode. |
| `read` | Read bytes from a file. |
| `write` | Write bytes to a file. |
| `flush` | Flush a file. |
| `close` | Close a file. |
| `remove` | Remove a path. |
| `rename` | Rename a path. |
| `mkdir` | Create a directory. |
| `list` | List directory entries. |
| `metadata` | Read file size and kind. |
| `temp_dir` | Find the temporary directory. |
| `read_file` | Read a whole file. |
| `write_file` | Write a whole file. |
| `read_lines` | Read a file as text lines. |

#### `i/process`

| Function | What it does |
|---|---|
| `args` | Read process arguments. |
| `env` | Read an environment variable. |
| `cwd` | Read the working directory. |
| `entropy` | Obtain secure random bytes. |
| `spawn` | Start a child process. |
| `spawn_piped` | Start a child with selected standard streams. |
| `spawn_config` | Start a child with stream, environment and directory settings. |
| `read_stdout` | Read the child’s standard output. |
| `read_stderr` | Read the child’s standard error. |
| `write_stdin` | Write to the child’s standard input. |
| `close_stdin` | Close the child’s standard input. |
| `wait` | Wait for a child to exit. |
| `terminate` | Terminate a child. |
| `close` | Close a child handle. |

#### `i/net`

| Function | What it does |
|---|---|
| `listen` | Listen for TCP connections. |
| `local_port` | Read the listener’s local port. |
| `accept` | Accept a TCP connection. |
| `connect` | Connect to a host and port. |
| `resolve` | Resolve a host name to addresses. |
| `read` | Read bytes from a stream. |
| `write` | Write bytes to a stream. |
| `shutdown_write` | Shut down the stream’s write side. |
| `close_stream` | Close a stream. |
| `close_listener` | Close a listener. |
| `stream_number` | Return the underlying stream number. |
| `byte_stream` | Make a byte stream interface for a connection. |

#### `i/tls`

| Function | What it does |
|---|---|
| `options` | Make default TLS options. |
| `identity` | Make a certificate and key identity. |
| `connect` | Establish client TLS on a connected stream. |
| `accept` | Establish server TLS on a connected stream. |
| `dial` | Connect to a host and establish client TLS. |
| `alpn` | Read the negotiated ALPN protocol. |
| `read` | Read bytes from a TLS stream. |
| `write` | Write bytes to a TLS stream. |
| `shutdown_write` | Shut down the TLS stream’s write side. |
| `close` | Close a TLS stream. |
| `byte_stream` | Make a byte stream interface for a TLS stream. |

## 10. The host boundary

### 10.1 External functions

```
@external("module", "function")
name: (T1, …, Tn) -> R
```

This declares a function that the host gives. It is the Wasm import `function` in the import module `module`, encoded as in Appendix B. The full type must be written. Parameters and the value must be types that the host can take (§10.3), with no errors and no environment. Every call of an external function is done when it is reached, like all calls. An external function should have no side effects that i code may see; work with side effects on the host uses host services (§9.7).

#### 10.1.1 JavaScript value declarations

The form is
`@js("expression") name: T`, where T expands to one closed nominal type with
no constructors. No holes, type variables, unions, functions or constructed
resource types. The annotation is required. It creates no i constructor or
cast between nominal types. `i/js.Value {}` is one ordinary use; packages can
define `Element {}` or other opaque types. JS is specific to this attribute;
it is not a target-neutral foreign language feature.

For example, an opaque type defined by a package needs no library rule:

```i
Element {}
@js("globalThis.document.body")
body: Element
```

Each retained declaration becomes a zero-argument value import returning
`externref`, with a cache in the loader, rather than an imported mutable
global. Its first and only initialization call is emitted in `ilang_start`,
in §2.5 dependency/module/source order alongside ordinary initializers.
The host parses the text as one parenthesized expression in strict ESM scope.
It may name globals but not generated loader locals, other declarations or i
bindings. The compiler does not type-check JS. Syntax failure is a loader
error before startup; an initializer exception stops startup and no entry
runs. Initializers may have host side effects. Use function expressions to defer work to an explicit Js
call. A retained declaration is available only on js and nodejs. On any other target, it is a compile error under §10.1.2.
No initializer or synchronous bridge call may re-enter i. Callback re-entry is not defined.

#### 10.1.2 Foreign linking

This rule defines acceptance and foreign initializer execution. Run it after ordinary type
checking and provider selection, before optimization, with the same result
at O0/O2/O3. There is no per-input specialization, constructor propagation,
constant folding, Boolean evaluation, type-directed branch elimination or
implementation-chosen widening. There is exactly one clause-pruning rule below.

The finite graph has one node for each ordinary binding's initializer, each
source function/clause-block body, each chosen provider body/terminal, and each
foreign declaration. References are resolved lexical definition identities.
Start with all ordinary top-level initializers, emitted entries/exports/tests,
and chosen provider factories. Scanning an initializer preserves §2.5: making
a function literal does not scan its body. A reference to an existing binding,
including an alias or a function passed as data, follows its definition and
activates all function bodies syntactically contained in that definition.
Scanning an activated body visits all expressions and guards, both sides of
conditionals/Boolean operators and every clause except this section's sole pruning case.
A function literal reached in an activated body is itself activated, even if
it is never called. A non-direct initializer scans its nested function bodies
too, since it may execute them. Reads of foreign declarations retain them.

Iterate these fixed reference edges to the least fixed point. Every source
body has one node shared by all callers; a second call adds no specialized
copy. Recursion and mutually recursive aliases terminate by marking nodes
once. References to aggregate bindings activate all function literals in their
definitions, not only a selected field. Higher-order parameters add no inferred
callee set: all supplied function expressions/aliases are already activated
where they are referenced, stored, returned or passed. This closed program
cannot receive an i function from the host (§10.3). Function values supplied
by chosen providers are traversed in the same way. Thus an unknown function
parameter cannot hide an unvisited source closure. No function-value analysis
or optimizer discovery changes this graph.

**The only pruning case.** The provider plan may designate a nominal service
as a build-discriminant service by giving a constructor map for every target.
Its default provider is a generated constructor application from that map,
with independently provisioned payloads. Target is one ordinary instance of
this general mechanism. The compiler uses resolved type/constructor identities
from the map, never the spelling Target, Js, Wasi, or a module name.

A subject is certified only when it is the environment parameter of an actual
emitted entry with that generated default provider, or a lexical alias made
solely by `alias = certified_name` (an annotation/group does not change this).
The entry binding must have **no source value reference anywhere in the
compiled program**, including as a higher-order value or recursive call,
and cannot also be selected as a provider factory. Import/export lists and
entry configuration are declarations, not source value references. Only its
emitted entry designation may invoke it. This syntactic restriction prevents
an ordinary i call from supplying another environment to the same entry body.
An explicit provider override, ordinary parameter, function result, tuple/field
projection or constructor expression is unknown. A helper's own environment
parameter is unknown, even if every apparent caller uses the default. There
is no propagation through actual parameters or return values.

A closure's direct lexical capture of a certified name or alias remains
certified. A newly bound/shadowing name is a different definition and gains
certification only through the alias rule. `! D = value` does not mutate a
previously bound D value; the captured old name remains certified, but later
calls fetching D see the substitution and their own environment parameters
are unknown. `keep_env(f)` activates f by ordinary reference reachability;
it grants no certification to f's environment parameters. A direct capture
of the original certified value remains a capture, including inside keep_env.
These rules use lexical identity, not runtime environment/value analysis.

For the source form `certified_name |> { clauses }` (allowing a one-expression
`{ }` group around the subject), omit a clause's guard and body only
if its pattern selects resolved constructor C and
`C != map[current_target]`, allowing field subpatterns and `name = pattern`.
For a choice, this inequality must hold for **every** alternative, recursively;
any other alternative prevents pruning. Compare resolved constructor identities,
not names or target labels. Maps need not be injective: mapping js and nodejs
to the same Web constructor retains Web on both. A wildcard, type
pattern or any other pattern is retained. A matching-target guarded clause
is always possibly executed, even if its guard is literally False; retain
later clauses too. A constructor unequal to the actual mapped constructor cannot match
regardless of its guard. No first-match or exhaustiveness reasoning removes any other clause.
Calling a named clause block or passing the subject to a function gives no
pruning, even if that function is an alias. Type-check all clauses first.
Suppressing an impossible clause's references does not remove pattern-failure
Assert behavior for a one-clause binding, or any other source semantics.
This rule changes no inferred requirements. The compiler preserves this
source match marker until the analysis is done; optimizer-introduced matches
cannot gain certification.

Every retained foreign declaration is initialized exactly once in §10.1.1 order,
even if an optimizer removes its reads. An unretained declaration is never
initialized. On an unavailable target a retained declaration is a compile
error at its declaration and a requiring reference, never null or a late trap.
Ordinary top-level `forced = foreign` always retains it. Two retained
`@js("({})")` declarations produce distinct objects; reads of one preserve
its identity within its nominal type (§4.1, §11.4).

Passing Target to a helper, fetching it in a helper's environment, or manufacturing the correct
constructor also gives no pruning. An uncalled closure referenced in retained
code still retains its foreign reads. This loss is deliberate and predictable.
A direct entry match remains expressible under the certified-entry rule.
An ordinary environment-bound Target may hold a substituted value. It is not certified by its type or name, and helpers are not specialized.

The generated artifact and loader metadata are specified in §B.6.1. Host exports always use the fixed ABI conversions, never a dynamically replaced Js converter.

### 10.2 Entry points

These functions are **entry points**:

- **The program entry**: the selected top-level function of the compiled package (§2.6). The default is `main` in `src/main.i`; the name `main` in any other module is ordinary. With `i run NAME` or `i build NAME`, use that `[entries]` target instead. It has no value parameters and its result is unused. A missing default means no program entry. Duplicate top-level names and duplicate TOML keys already reject duplicate declarations; there is no package-wide count of attributes.
- **Tests**: the entries of the collection named by `[test] collection` in `i.toml`. `std/test` provides ordinary imported item macros `test`, `skip`, `only`, `fails`, `suite`, and `each`, which contribute private functions and display metadata to `std/test.tests`. For example, `(testlib = (test) = std/test)` makes `test("addition", () { assert(1 + 1 == 2) })` available. These names have no special language meaning. Each selected function takes no value parameters and gives `Nil` or `Never`; its inferred environment is supplied by its provider plan. A test **passes** when it gives `Nil`. It **fails** on a stop (§11.7) or uncaught typed error. `fails` reverses that expectation; `skip` reports an entry without running it, and `only` selects focused entries. `suite` prefixes names and copies enclosing environment statements into its test bodies; `each` contributes one test for each written case. The runner compiles the collection once and runs each selected test in a fresh started instance (§B.1). With no configured collection, no tests run. The removed `@test` attribute is an ordinary unknown-attribute error.
- **Host exports**: the functions named in `[wasm].exports` (§2.6) of the package that is compiled. The `[wasm]` of other packages is not used. They must be in the exports of their module. Their parameters, value, and errors must be types that the host can take (§10.3).

"No parameters" here does not count environment parameters. An entry may also require any ordinary named service supplied by its explicit target provider plan (§2.6). The plan constructs ordinary service values; missing required standard capabilities are build errors, including when all operation errors are caught. Providers do not alter inferred source types. Source-only checking retains the signature rules above without checking target supplies.

An error that comes to the program entry or a test stops the program. An error that comes to a host export is given to the host (Appendix B).

<!-- Provider graph construction, factory/adapter provisioning and synchronous
export validation are implemented; see tests/spec/9.11/providers and
crates/ilang-cli/tests/providers.rs. -->

On targets using the v3 asynchronous entry protocol, an entry is **async** exactly when its complete chosen provider graph (§2.6.1), including both scopes and adapter requirements, contains a request-mode service provider, a scheduler role or a runner role. Normalize an async host declaration to its request-and-await provider first. A service node's declared request/async mode counts even when a factory supplies a completed fake. A pure bundle with no such dependency may be synchronous. Await, Owner and Root alone do not qualify. No source-body reachability or selected-field analysis changes this classification. Metadata and wrapper shape follow the graph before linking and optimization; a requested synchronous export that classifies async is a compile error (§2.6.2). The host waits through Appendix B's protocol. WASI instead keeps blocking imports, synchronous metadata and no async hand-off (§B.7).

Open the dependency-free primitive entry context before chosen factories, enabling async admission exactly for an async entry. It constructs no replaceable service. Construct all services in §2.6.1's one topological order, enter the sole adapter at its place in that order, and invoke the unchanged source entry with the chosen body services. Finish structured cleanup before delivering the result or error; retire the primitive context on every exit. A stop runs no i finals. Synchronous fake-only contexts cannot hand off: a low-level wait needing hand-off or poll stops with `request: no active entry`. An async entry may return status 0 on its first call; pure scheduler switches alone do not yield to the host.

### 10.3 Types the host can take

`Nil`, `Bool`, `Int`, `Float`, `String`, `BitArray`, and custom types with no constructors (§4.3). Values of custom types with no constructors go to the host as opaque references. No other type may go to the host. An error type that the host can take is a union of `Nil`, `Bool`, `Int`, `Float`, `String`, and `BitArray`, with no two the same.

Where i code gives a value to the host, an `Int` or `String` member counts as `Int` or `String`, whatever its value set (§4.2): a literal type, a union of literal types of one type constructor (also through an alias), or all the values but a finite set, such as `Int - 0`. Where the host gives the value (the value of an external function, the value and errors of a host-service function, and the parameters of a host export), an `Int` or `String` member must have all its values, because the host's values are checked only for their form (§B.2).

```i
@external("app", "mode")
mode: () -> "on" | "off"           // compile error: the host gives the value
@external("app", "log")
log: (level: "info" | "warn") -> Nil   // correct: i code gives the value
```

The outer Request and optional Result in a request-mode host declaration describe how work is obtained; they are the declaration forms of [runtime.md §5.2](design/runtime.md), installed by R2, not additional boundary types. Requests, tickets, tasks, scopes, channels and events cannot cross the ABI. IO wrappers over Int and constructed Thrown/Mismatch are not boundary types; their libraries pass Int or opaque Outcome and decode in i. Neither provider plans nor JavaScript value declarations change the value and error lists above. No library-specific exception is added.

Records may neither be given to the host nor received from it through this interface. A binding with label parameters (§4.9.4) cannot be an entry point even when its ordinary parameter types would fit the host interface.

## 11. Run-time behavior

### 11.1 Values

No value may be changed. The values are integers, floats, strings, codepoints, bit arrays, tuples, lists, records, values of custom types, functions, boundaries, continuations, keys, keyed values, requests, tickets, and host references.

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
- Records are the same when they have the same labels and the values with the same label are the same. Field order has no effect. Additional fields count, also when static types hide them. A record is never the same as a value that is not a record.
- A function, a boundary, a continuation, and a host reference have no parts that may be looked at, so each is the same only as itself: the value made by one evaluation of a function expression or a clause block, by one call of `control.boundary` or `control.suspend`, or one reference from the host (§B.4). A top-level function is worked out once (§2.5), so all its uses are the same value. In the same way, a constructor, a built-in function, a library function, and each `@external` declaration are each one value; two declarations of one import are two values. So two function expressions that are written the same way give values that are not the same.

- Each call of `control.keep_env` (§9.4) makes a new function, the same only as itself. A key (§9.8) is the same only as itself. Two keyed values are equal when their keys are the same and their values are equal.
- A request or ticket compares by its underlying request identity, which does not change on answer, take or cancel. Ticket identity is not a fresh value on each call of ticket. Request and Ticket are different type constructors and are not equal to one another.
- A value stays the same value when the program copies it from one worker to another (§11.9).

### 11.5 Comparing

`< <= > >=` compare Ints by number. `<. <=. >. >=.` compare Floats by IEEE 754.

### 11.6 Tail calls

A call in tail position uses no more stack. Tail positions are: the last expression of the body of a function expression (§6.8); the expression of a clause of a clause block that is called as a function, and not in a match, because that expression is the last thing the function works out; the expression of a clause of a match in tail position; the last expression of a block in tail position; and the right side of `&&` or `||` in tail position. An environment statement does not change which calls are in tail position. The clause expressions of a clause block are in tail position only by the two items on clauses above.

So in `loop = { 0 -> Done  n -> loop(n - 1) }`, the call `loop(n - 1)` uses no more stack. A match adds no call frame (§6.12).

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

A JS bridge call that does not expose exceptions uses `panic("js: call threw")` for a thrown exception. Malformed raw outcomes, argument builders or typed host replies use `host: value is not correct`. These stops are not Mismatch errors; explicit checked conversions may return Mismatch. Provider adapter guards use the panic messages of §2.6.1. Memory and stack exhaustion remain stops.

Before the program stops, the host gets a **stop message**, in one call of `panic` (Appendix B): the reason, then one line for each source position, with a line feed between lines. The first position is that of the innermost expression in the program source that was being worked out: a call, an operator, a `<<`, or a binding. Then come the positions of the calls in the program source that were running, the innermost first. Calls that were replaced by tail calls are gone, but the first position is always given, also when that expression is a call in tail position: in `f = (n) { assert(n > 0) }`, the stop shows the position of `assert(n > 0)`. A call of a library function, such as `fail.catch` or `resource.ensure`, counts as a tail call when it is in tail position. For an error at an entry point, there is only the position of the entry point's binding.

A **source position** is written `module path:line:column`, for example `app/users:12:5`, with the module path as its own package names it. For a module of a package with a location, the location, `@`, the major version, and a space come first: `github.com/alice/json@1 json/decode:12:5`. Lines and columns count from 1; a column counts Unicode scalar values. The position of an expression is the position of its first token.

A function with no errors and no environment (`^ Never ! Never`) may still stop for these reasons, and it may never end.

### 11.8 Text form of values

The **text form** of a value is given by `core.inspect` (§13):

| Value | Text form |
|---|---|
| Int | Decimal digits, with `-` before a negative number: `-42` |
| Float | See below |
| String | `"`, then the characters, then `"`. `"` and `\` are written as `\"` and `\\`. Line feed, carriage return, tab, and form feed are `\n`, `\r`, `\t`, `\f`. Other characters from U+0000 to U+001F, U+007F, and U+0080 to U+009F are `\u{XXXX}`, with four upper-case hex digits. All other characters are written as they are. |
| List | `[`, the text forms of the parts with `, ` between them, `]` |
| Tuple | `(`, the text forms of one or more parts with `, ` between them, `)` |
| Record | `()` when empty; otherwise `(label: value, ...)`, with labels in UTF-8 order and non-name labels quoted |
| Value of a custom type | The constructor name. If it has fields: `(`, the text forms of the fields in order with `, ` between them, `)`. Labels are not written. So `True`, `Nil`, `Ok(1)`. |
| BitArray | `<<`, the whole bytes as decimal numbers with `, ` between them, `>>`. If the number of bits is not a multiple of 8, the last k bits come last, written as `value:uk`: `<<82, 2:u2>>` |
| UtfCodepoint | `//utfcodepoint(`, the character, `)` |
| Function | `//fn(`, one name for each parameter of its type with `, ` between them, `) { ... }`. Environment parameters are not counted. Parameter i (counting from 0) is named by the letter `a` + (i mod 26), with the decimal number i div 26 after it when i is 26 or more: `//fn(a, b) { ... }` |
| Boundary, continuation | `//boundary`, `//continuation` |
| Key, keyed value | `//key`, `//keyed` |
| Request, ticket | `//request`, `//ticket` |
| Host reference | `//wasm(opaque)` |

A **Float** is written in this way. Let d1 … dn be the shortest list of decimal digits, with dn not 0, such that 0.d1…dn × 10^k gives back the same float when read. When more than one list of that length does this, take the one nearest to the exact value of the float; when two are equally near, take the one whose last digit is even. `-` comes first for a negative float, including -0.0. Zero is `0.0`. Let p = k − n.

- The **long form** is used when p is at least L and at most U, and not when p ≥ 0 and the size of the float is 2^53 or more. L is −4 when n = 1, and −2 − n when not. U is 2 when n = 1 or k ≥ 11, and 1 when not.
- In the long form, the digits are written as a decimal number, with `.` and at least one digit after it: `123.0`, `12.67`, `0.0123`.
- In the short form, d1 is written, then `.`, then d2 … dn, or `0` when n = 1, then `e`, then k − 1 as a decimal number with `-` when it is negative: `3.0e26`, `1.0e-7`, `1.0e3`.

The text form of a function, a key or a keyed value never depends on which worker made it or holds it. Task handles keep bookkeeping ids under keyed fields, whose text is `//keyed`; only task.path exposes the semantic task path. Requests and tickets show no ids, state or answers.

### 11.9 Workers

A program may run on one or more **workers** at the same time. Which worker does which part of the work cannot be seen. Tasks that run at the same time take their steps (sends and receives, joins, host calls, failures and stops) one after another, in an order that the program does not fix; this order may differ from run to run and with the number of workers. Every run gives a result that some order of the steps gives, and each step does what it would do if the steps were taken one at a time in that order. The number of workers only changes which order happens. So anything that depends on the order of steps may change, for example which sender a receive pairs with, which event of a `choose` commits, whether a step comes before or after the end of a scope, which tasks fail before a cancel reaches them, which of two stops is first, and the order of output. A program whose results do not depend on the order of its steps gives one result. Running out of memory or stack (§11.7) is not covered by this rule. The host chooses the number of workers.

The **captures** of a function made by a function expression are the local names that its body uses and that are bound outside it, after macro expansion; a name counts when it is written in the body, also in a part that never runs. Top-level names are not captures. A function made by `control.keep_env` (§9.4) has the captures of its argument and the services it keeps. Other functions have no captures.

The values that a value **holds** are: the parts of a tuple, a list, a value of a custom type or a record (§11.4); the values of the captures of a function; and the value in a `Keyed` (§9.8). `==` does not look at what a function holds (§11.4). A value is **portable** when it is not a boundary, a continuation, a host reference, a request, a ticket or a `Syntax` value (`meta.md` §8), and every value it holds is portable. A value may hold itself, through a local function that calls itself; only the other values it holds then count.

A value that is not portable never goes to another worker. Values may be copied between workers; a copy is the same value (§11.4).

## 12. Built-in names

These names may be used in every module. Other names may cover them.

| Name | What it is | See |
|---|---|---|
| `Int` `Float` `String` `List` `BitArray` `UtfCodepoint` `Never` `Record` | Types | §4.2, §4.9 |
| `Bool` `Nil` `Result`, and `True` `False` `Nil` `Ok` `Error` | Custom types and their constructors | §4.2 |
| `Assert` `AssertionFailure` | Custom types and their constructors | §9.6 |
| `Boundary` `Continuation` | Built-in types made by `i/control` | §9.4 |
| `Key` `Keyed` | Built-in types made by `i/key` | §9.8 |
| `Request` `Ticket` | Built-in types operated on by `i/request` | §9.11 |
| `panic` | `(message: String) -> Never` | §11.7 |
| `assert` | `(c: Bool) -> Nil ! Assert` | §9.6 |

These are normal functions and types. The modules `i/fail` (§9.3), `i/control` (§9.4), `i/resource` (§9.5), `i/assertion` (§9.6), `i/io` (§9.7), `i/key` (§9.8), `i/task` (§9.9), `i/random` (§9.10), `i/request` (§9.11), `i/core` (§13), and `i/record` (§14) give the other names. The IO module list is extended with its service definitions.

## 13. The module `i/core`

These are normal functions: they are called, given, and compared like any other function. Only their bodies are not written in i. Most of them do things that the rest of the language cannot do, such as looking at how any value is made (`inspect`, `hash`). Some, such as `index_of` and `bit_array_concat`, could be written in i, but the runtime does them much faster. They have no errors and need no environment.

| Function | Type | What it gives |
|---|---|---|
| `portable` | `(value: A) -> Bool` | `True` when the value is portable (§11.9) |
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
| `to_upper` | `(String) -> String` | The string with each character changed to its simple uppercase mapping in Unicode 18.0.0 (UnicodeData.txt). This is one character for one character, and no language changes it. A character with no simple uppercase mapping, such as `ß`, stays the same. |
| `to_lower` | `(String) -> String` | The string with each character changed to its simple lowercase mapping in Unicode 18.0.0, in the same way |
| `index_of` | `(in: String, find: String, from: Int) -> Result(Int, Nil)` | The byte offset (counting from 0) of the first `find` in `in` that starts at `from` or after it. An empty `find` is at `from`. `Error(Nil)` when there is no such `find`, when `from` is less than 0 or greater than the byte size of `in`, or when `from` is inside a character. |
| `bit_size` | `(BitArray) -> Int` | The number of bits |
| `bit_array_to_string` | `(BitArray) -> Result(String, Nil)` | The string, when the bits are whole bytes of correct UTF-8; `Error(Nil)` when not |
| `bit_array_concat` | `(List(BitArray)) -> BitArray` | The bit arrays one after the other. Their sizes may be any number of bits. |
| `inspect` | `(a) -> String` | The text form of the value (§11.8) |
| `hash` | `(a) -> Int` | A number from 0 to 2^32 − 1. Values that are the same (§11.4) give the same number. Which number is not fixed. |

## 14. The module `i/record`

These are ordinary library functions, using the label parameters of §4.9.4. They have no errors and need no environment. Keys for `get`, `put`, and `remove` must be one known string, or an enclosing key parameter passed by its own name. There is no privilege for this module in inference.

| Function | Type | What it gives |
|---|---|---|
| `get(K, V)` | `(r: (K: V), key: K) -> V` | The field `key` of `r` |
| `put(R, K, V)` | `(r: R & Record, key: K, value: V) -> (..R, K: V)` | A record with the field added or replaced |
| `remove(R, K)` | `(r: R & Record, key: K) -> (..R, -K)` | A record without the field, if it has one |
| `has` | `(r: Record, key: String) -> Bool` | Whether the record has the field, including fields hidden by its static type |
| `keys` | `(r: Record) -> List(String)` | The labels, sorted by UTF-8 bytes |
| `size` | `(r: Record) -> Int` | The number of fields |
| `texts` | `(r: Record) -> List((String, String))` | Each label with its value's text form (§11.8), sorted by label bytes |

There is no `empty` function: `()` is the empty record. `has` and `keys` do not change any type. There is no operation returning all fields as values of one type: fields may have different types. Code that knows their types reads them by label; code that shows them uses `texts`.

## Appendix A. Choices

This appendix lists the rules that were chosen without a full discussion, until they are confirmed. Each one says which rule of Gleam or TypeScript it follows, or why it does not. All choices so far have been confirmed, so the list is empty.

## Appendix B. The binary interface

### B.1 The module

A compiled program is one Wasm module. It needs the Wasm features garbage collection, typed function references, tail calls, and exception handling with `exnref`. Exceptions are used only inside the module: its tags are not exported, every export catches them, and an import must not throw one into the module. It exports:

- `memory`: the linear memory used to pass buffers.
- `ilang_alloc(size: i32) -> i32`: gives the address of a new buffer of `size` bytes in `memory`. For size 0 it may give 0.
- `ilang_free(address: i32, size: i32)`: frees a buffer made by `ilang_alloc`.
- `ilang_start()`: starts the program by working out the top-level values (§2.5). It has no parameters and no results. The host calls it one time, after it makes the instance and before it calls any other export but `ilang_alloc` and `ilang_free`. During `ilang_start`, imports may be called as at any other time.

Checked Wasi buffers are separate table-owned allocations (§B.3, §B.7.1); they do not change these host allocation exports.

The module has no Wasm start function, so that imports called while top-level values are worked out may call `ilang_alloc`. The module has one custom section named `ilang.abi` with the metadata of §B.6.

This version runs one instance for each run of the program. Workers (§11.9) come with a later version of this interface.

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

A function `(P1, …, Pn) -> A ^ E` is passed like this. Environment parameters are not passed.

- **Parameters**: the values of `P1`, then `P2`, and so on.
- **Results**, when `E` is `Never`: the values of `A`.
- **Results**, when `E` has members: first an `i32` **tag**, then the values of `A`, then the values of each member of `E` in the **canonical order**: `Nil`, `Bool`, `Int`, `Float`, `String`, `BitArray`. Tag 0 means a normal value; tag i means an error of the i-th member. The values that are not used are 0, or null for an `externref`.

When the host gives a value that is not correct (a `String` that is not UTF-8, an `Int` with a wrong sign or a last part of 0, a `BitArray` whose bits after the end are not 0, a `Float` that is not finite, a `Bool` that is not 0 or 1, a buffer that is not inside `memory`, also an empty one, or a tag that is too big), the program stops (§11.7).

### B.3 Buffers

- A buffer given as a **parameter** belongs to the caller. It may only be used during the call. The callee makes a copy if it needs one.
- A buffer given as a **result** is made with `ilang_alloc` and belongs to the receiver, which frees it with `ilang_free`.

Checked Wasi buffers (§B.7.1) are separate table-owned allocations and are not addresses supplied to `ilang_free` by i code. The allocation exports remain host machinery; checked tokens cannot name ABI buffers.

So for an export, the host makes the parameter buffers with `ilang_alloc`, frees them after the call, and frees the result buffers. For an import, the module makes and frees the parameter buffers, and the host makes the result buffers by calling `ilang_alloc` before it returns. During an import, the host may only call `ilang_alloc` and `ilang_free`.

### B.4 Imports

- **Retained JavaScript value getters** (§10.1.1): zero-argument target imports returning `externref`, described in §B.6.1. They are not new `ilang:host/v1` runtime imports.
- **External functions** (§10.1): the import module and name of the `@external` attribute, encoded as in §B.2.
- **Functions of host services** (§9.7): the import module is the namespace and the name is the field label, encoded as in §B.2.
- **Async functions of host services**: the import has an `i32` **request id** first, then the parameters of §B.2, and no results. The host copies the parameters, keeps the request, and returns at once. The answer comes later through `ilang_resume` (§B.5). Until then the request is **outstanding**.
  - The host calls the service during the import, and waits for its work only after the import has returned. So a service can fix anything that depends on the time of the call, such as the deadline of a sleep, during the import. The parameters are copied then too.
  - The module chooses the id: a number of 0 or more that no other outstanding request of the instance has. After a request is answered, cancelled or dropped (§B.5), its id may be used again.
  - The module calls an async function only during a call of an async entry point, of `ilang_resume:…` or of `ilang_continue`, and not while an import runs (also not from a call such as `ilang_alloc` that the host makes during an import). So it does not call one during `ilang_start` (§B.1) either. If it calls one at another time, or with an id that is below 0 or already outstanding, the import traps and the host does not use the instance again.
  - On the WASI target none of this holds: there, as §B.7 says, an async function is a blocking import, with no request id.
- **Runtime imports**, with the import module `ilang:host/v1`:
  - `panic(address: i32, length: i32)` gives the stop message (§11.7) as UTF-8. Then the module traps.
  - `reference_id(reference: externref) -> i64` gives a number for a host reference: the same number for the same reference, different numbers for different references, and 0 for null. It is used by `==` (§11.4).
  - `cancel(id: i32)` drops the outstanding request `id` (above): the host stops its work if it can and never answers it, and the id may be used again. It may be called at the same times as an async function. With an id that is not outstanding, it traps and the host does not use the instance again.

### B.5 Exports

- **Host exports** (§10.2) have the name written in `[wasm].exports`, for example `app/math.add`, and are encoded as in §B.2.
- **The program entry and tests** have the name `module path.function name`, no parameters, and no results.
- A function that is both a host export and the program entry or a test, or both the program entry and a test, or a function named two times in `[wasm].exports`, is a compile error, because their export names would be the same.
- **Async entry points** (§10.2) have the parameters above and one result, an `i32` **status**. 0 means the entry point has ended. 1 means it is **waiting**: it has one or more outstanding requests (§B.4), and it goes on only when the host answers one. 2 means it is **runnable**: it can go on without an answer. Another status is a fault of the module.
- When the status is 2, the host lets its own work run once without waiting, then answers a request whose work is done, if there is one. If there is none, it calls the export `ilang_continue() -> i32`, with no parameters, whose result is a status again. The host calls `ilang_continue` only then, right after status 2; a call at another time breaks the instance. A module that gives status 2 has this export; if it does not, it is a fault of the module. The names `ilang_continue`, `ilang_start`, `ilang_alloc`, `ilang_free`, `memory` and those that start with `ilang_resume:` or `ilang_result:` are reserved: the metadata may not use them for host exports or entry points.
- The host answers a request by calling the export `ilang_resume:namespace:label` of the function that started it. The first parameter is the request id, and the others are the results of the function as in §B.2. The result is a status again. The host may answer outstanding requests in any order, but one at a time. It makes the result buffers with `ilang_alloc` just before the call that gives them. A module that starts a request of a function and has no such export for it, or one of the wrong type, has a fault, and the host does not use the instance again.
- A call of `ilang_resume:…` with an id that is not outstanding, or that another function started, traps.
- When the status is 1 and no request is outstanding, the program can never go on. This is a fault of the module, and the host does not use the instance again.
- When the status is 0, the host calls the export `ilang_result:name` of the entry point, with no parameters, to get its results as in §B.2. Requests that are still outstanding are dropped: the host stops their work if it can and never answers them. This lets a program end work that it no longer needs; a module made by this compiler has no outstanding request at that point.
- When the work of an async function fails in the host (not an error of its `^ E`, which is an answer), the host drops every outstanding request and does not use the instance again.
- While an async entry point is waiting or runnable, the host may only call `ilang_resume:…`, `ilang_continue`, `ilang_alloc`, and `ilang_free`. So an instance runs one async entry point at a time. The module does not check every export for this; a host that calls another export breaks the instance.
- When the program stops, the export traps after the `panic` import (§B.4). After a trap, the host may not use that instance of the module again. Every outstanding request is dropped, and an answer that comes later is ignored. The host makes a new instance and calls `ilang_start` on it.

### B.6 Metadata

The custom section `ilang.abi` is UTF-8 JSON in this form:

```json
{
  "version": 3,
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

- `version` is 3. A host refuses a module with another version, so a module of an older version must be built again. Version 3 differs from version 2 in these ways: an async function has a request id; status 1 means one or more outstanding requests, not exactly one, and the host may answer them in any order; status 2 and the export `ilang_continue` are new, and `ilang_continue` is a reserved export name; the runtime import `cancel` is new; outstanding requests are dropped when the entry point ends, on a trap, and when the work fails in the host; and an async function may only be called during an async entry point, a resume or `ilang_continue` (§B.4). A runtime import, status or export that a host must handle is added only with a new version.
- Type names are `nil`, `bool`, `int`, `float`, `string`, `bit_array`, and `externref`. `errors` is in the canonical order (§B.2).
- `entry` names the selected function by its module path and function name, not its `[entries]` key. It is `null` when there is no selected entry. Only that entry is emitted as the program entry; unselected declarations do not add ABI exports. The JavaScript SDK uses this metadata entry. The ABI version and shape stay unchanged. `tests` is empty unless tests were compiled.
- `services` lists every required host-registration function left by the chosen provider graph, and `externals` every required external function. A service supplied internally by an ordinary provider is not a required host import. Entry async flags and status/result exports follow §10.2, before linking and optimization.
- The host must give every import in `services`, `externals`, and `ilang:host/v1` before it makes an instance of the module. If not, making the instance fails.
- A module of the WASI target (§B.7) also has `"target": "wasi"`. JS targets have `"target": "js"` or `"target": "nodejs"` (§B.6.1). Host modules have no `target`. A host of this appendix does not run a module with a `target`.

#### B.6.1 JavaScript target artifacts

`name.wasm`, `name.mjs` and `name.d.ts` are
the generated JS artifact. The mjs contains the fixed bridge, retained source
expressions and loader; each instance has its own declaration caches and
entry resource contexts. It starts the module once, then calls main or the
requested host export. JS source may use `import("node:...")` inside a
function and await its opaque promise through the request protocol. There are no static import
declarations inside an expression, implicit Node module variables, or
compiler-generated user callbacks. Standard Node factories may use their
own ESM imports. Their source is fixed support code, not user handwritten JS.

Keep the v3 wire for ordinary service/request calls. Put foreign declaration
text, stable package/module/name identity, generated import name, nominal type
identity, source span and availability in a separate versioned target section
`ilang.js` (initial version 1). A matching generated loader requires that
section and `target: "js"` or `"nodejs"`; old Appendix B hosts reject the
non-host target instead of accidentally loading it. This target metadata does not add a v3 runtime export. Mandatory changes to runtime imports, statuses or host-called exports still require a version bump (§B.6).
The value getters are target imports listed there, not additions to v3's
`ilang:host/v1`. A digest binds the JS manifest to its Wasm artifact; it is
not self-authenticating permission for raw IO (trusted declaration grants still apply).

`bindings/javascript` remains the Host SDK. Generated-target and explicit SDK
usage share encoding and request lifecycle code. Host exports always use the
real SDK conversions and d.ts representations, never a dynamically replaced
Js service; a custom outgoing converter cannot change an export's ABI.

### B.7 The WASI target

`i build --target wasi` makes the module a WASI preview 1 command, which a WASI host such as `wasmtime run` or Node's `node:wasi` runs with no other step. It is the module of §B.1 to §B.6 with these changes:

- It imports `fd_write` and `proc_exit` from `wasi_snapshot_preview1`, and not `panic` from `ilang:host/v1`. The stop message (§11.7) and a line feed are written to file descriptor 2, then the process ends with exit code 1. `reference_id` is imported from `ilang:host/v1` only when a type of an import or export is a custom type with no constructors, because only the host can make such a value.
- The target's ordinary Io provider writes UTF-8 text plus LF to file descriptor 1 through the checked Wasi adapter (§B.7.1). Positive short writes are retried; zero progress or errors give up that write; print gives `Nil`. It attempts the text first, then LF even if writing the text gave up, and frees buffers on every returning path. It is absent from required-import metadata because it is internally provided. Another host service with the namespace `i:io/v1` is an import like any other.
- Because the runtime imports `fd_write` and `proc_exit`, an import (§B.4) with the import module `wasi_snapshot_preview1` and one of these names is a compile error. Such an import is an external function, or a required host-registration field in the entry provider graph (§9.7). Other functions of `wasi_snapshot_preview1` may be imported.
- Every other required host-registration function and external function is an import as in §B.4, so a host that has them can give them. A module that the host makes before the program, such as one given with `wasmtime run --preload`, cannot use the program's `memory` or `ilang_alloc`, so it can only give functions whose parameters, values, and errors are `Nil`, `Bool`, or `Float`. An import with an `Int`, `String`, or `BitArray` needs a host that defines it with the exports of the instance, such as an embedder, or a module linked in statically. An import with a host reference needs such a host too, because only the host makes host references, and the module then imports `reference_id`.
- A function of an async host service is a blocking import, with the results of a function that is not async (§B.2). Nothing is async, and there are no `ilang_resume:…` and `ilang_result:…` exports.
- It exports `_start`, which calls `ilang_start` and then the program entry. The program must have a selected entry; if not, it is a compile error. When `_start` returns, the exit code is 0.
- Tests are not compiled: a test needs a new instance (§10.2), and a command runs one entry.

When the program runs out of memory or stack (§11.7), there is no stop message: the host gives its own error. The host also decides how deep the stack may be, so ordinary recursion that is not in tail position may stop earlier than in another host.

Preview-1 defaults also supply Stdio, Clock, Entropy, Arguments, Environment
and Fs. Required Net, Tls, Process or WorkingDirectory without an explicit
provider is a compile error naming the service, target and entry. Custom
imports retain the blocking ABI above. Standard capability adapters operate
on checked i values: they copy strings/whole-byte bit arrays, validate integer
ranges, construct pointer-bearing layouts privately, check host result extents,
and release scratch allocations on returning paths. No guest-supplied bytes
are interpreted as pointers. Request operations return completed requests.

| Capability | Raw preview-1 imports and checked layouts |
|---|---|
| Stdio | fd_read/fd_write: one private 8-byte iovec and 4-byte count; count must not exceed the requested extent; writes retry positive short progress; zero progress is io |
| Clock | clock_time_get: private u64 timestamp; poll_oneoff: zeroed 48-byte clock subscription, 32-byte event and u32 count; check count/error and recheck the monotonic deadline; zero polls once, negative is invalid, durations above 2^63-1 nanoseconds give range; monotonic origin is command entry |
| Entropy | random_get: private byte allocation, size 0..16777216; return a copy |
| Arguments / Environment | args_sizes_get/args_get and environ_sizes_get/environ_get: private u32 pointer vector and byte buffer, checked NUL-terminated UTF-8 extents; args excludes argv[0]; environment lookup validates its name |
| Fs | fd_prestat_get/dir_name selects the longest matching preopen; path_open uses private UTF-8 path and u32 output; fd_read/write/close (flush validates the live handle; writes are unbuffered); path_create_directory/unlink_file/remove_directory/rename/filestat_get; fd_readdir uses checked 24-byte dirents and cookies, retrying truncated entries |

Fs exposes fresh run-local Int tokens, never raw descriptors or addresses.
Closed tokens never name a reused descriptor. Paths outside all preopens give
denied; the WASI host enforces traversal and symlink confinement. temp_dir
selects the /tmp preopen when present, otherwise the first preopen; no preopens
gives not_found. Preview-1 errno values map to the same standard string error
codes as native IO (not_found, denied, exists, invalid, closed, interrupted,
or io). IO failures do not become runtime traps.

#### B.7.1 Checked Wasi adapter

The ordinary Wasi service's
memory fields below are synchronous. Token is an Int internally naming a live
allocation in this instance's dedicated checked-buffer table. The table never
contains ABI argument/result buffers, runtime scratch, GC data or allocator
metadata. Export no operation giving a token's address. Allocation produces
zeroed bytes and a fresh positive token, never reusing one in that instance.
End of entry/startup failure/stop frees remaining checked buffers. A token
from another instance is not a portable memory reference; no library IO value
wraps it. Root APIs expose operations, not the table.

| Field | Type |
|---|---|
| alloc | `(size: Int) -> Int ^ String` |
| free | `(token: Int) -> Nil ^ String` |
| store | `(token: Int, offset: Int, bytes: BitArray) -> Nil ^ String` |
| load | `(token: Int, offset: Int, size: Int) -> BitArray ^ String` |
| fd_write | `(fd: Int, token: Int, offset: Int, size: Int) -> Int ^ String` |
| proc_exit | `(code: Int) -> Nil ^ String` |

`Wasi` is an ordinary service type with this checked signature. Its implementation is a checked adapter, not generic Int ABI lowering. Library helpers
may wrap buffer tokens privately and add structured cleanup, as ordinary code.
Possession of the root grants use of its buffer table; tokens are not secrets.

Validation order is fixed: validate scalar ranges/whole-byte inputs, then
look up a live token, then check its extent. Negative sizes/offsets, an fd or
exit code outside unsigned 32 bits, and non-whole-byte store input give
`invalid`. Unknown/nonpositive/freed tokens (including double free) give
`closed`. A valid token with offset or size outside its allocation gives
`bounds`. Use mathematical checked addition, never wrapping pointer arithmetic.
For empty loads/stores require `0 <= offset <= allocation size`; they do not
bypass liveness. An allocation size above memory32's addressable range gives
`invalid`; exhaustion within that range stops for memory exhaustion.
Successful free removes exactly one table entry before freeing its allocation;
there is no size argument or interior-pointer free. Load returns a copy.

Tokens are identifiers, so `token + 1` is **not** a pointer into token. It may
name another allocation only if that token is live in this authorized table;
there is no promise of unforgeability. To access a subrange pass token and
checked offset. Stale tokens cannot become valid after allocator address
reuse. A call pins its allocation until it returns. These synchronous fields
do not yield; no i callback can free a pinned buffer inside a host call.
For future async adapters, retain a lease or copy before returning; they
cannot inherit raw addresses from this synchronous contract.

**Checked layouts.** For fd_write, validate
the inputs above; allocate private scratch for one iovec and a u32 count; translate
the checked slice to the real pointer; call the runtime-owned preview-1 import;
then free scratch on every returning path. On errno != 0, raise String
`errno:<unsigned decimal>`. On success return the number of bytes written,
0..size; a larger count is an invalid host value and stops. No user-controlled
pointer-bearing bytes are ever passed as an iovec. Other preview-1 functions require separately specified pointer/length/output layouts and raw signatures before an adapter is added.

proc_exit validates its unsigned code, runs context teardown, then calls the
existing raw `(i32) -> ()` import. Its i signature is Nil because §10.3 has no
Never result; successful execution never returns an i Nil. If the host returns
unexpectedly, trap and retire the instance. Invalid code raises `invalid`
before teardown or exit. There is no fabricated errno/result from a void
Wasm call. Teardown does not run i finals; explicit process exit is termination,
not a way to detach work. WASI resource cleanup must finish before this call.

The adapter reuses the runtime-owned fd_write/proc_exit imports; it does not declare duplicates. The independent runtime stop path, including startup stops, source positions, partial-write retries, LF and exit code 1, cannot call a replaceable Io. Replacing Io leaves this stop path intact.

## Appendix C. String grammar

The string additions to the surface grammar are summarized here (the full lexical rules are §1.6 and pattern rules are §7.1):

```ebnf
StringExpr = cooked_string | multiline_string | raw_string .
StringInterpolation = "\(" Expr ")" .
StringPattern = string_lit "<>" Binder [ "<>" string_lit ]
              | Binder "<>" string_lit .
```

`raw_string` requires equal opening and closing hash counts and enables escapes/interpolation only with that same count after the backslash. `string_lit` in a pattern or another literal-only position excludes interpolation. Multi-line delimiters and indentation have the line constraints of §1.6.
